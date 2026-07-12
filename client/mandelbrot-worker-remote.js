let jobNumber, workerNumber;
const retryLimit = 7;

function doIterationCounts(coords, url, retryCount, thisJobNum) {
    let iterationCounts;
    let error = "";

    if (thisJobNum != jobNumber) {
        // console.log("doIterationCounts current job number", jobNumber, ", stale job number", thisJobNum);
        return;
    }

    try {
        let client = new XMLHttpRequest();
        client.open("POST", url, false);
        client.setRequestHeader("Content-Type", "application/json");
        client.setRequestHeader("Accept", "application/json");
        client.send(JSON.stringify(coords));

        if (client.status == 200) {
            iterationCounts =  JSON.parse(client.response);
        } else {
            error = "XMLHttpRequest status: " + client.status;
        }
    } catch(err) {
        error = err;
    }

    if (error !== "") {
        if (retryCount < retryLimit) {
            retryCount++;
            // console.log("XMLHttpRequest failure, retrying " + retryCount + " of " + retryLimit + "...\n" + error);
            setTimeout(function() {
                    doIterationCounts(coords, url, retryCount, thisJobNum);
                },
                retryCount*2000);
            return;
        } else {
            console.error("XMLHttpRequest failure, retry limit exceeded!\n" + error);
            iterationCounts = [];
            for (let i = 0; i < coords.rows; i++) {
                iterationCounts[i] = array_fill(new Array(coords.columns), -1);
            }
        }
    }

    let returnData = [ thisJobNum, coords.firstRow, iterationCounts, workerNumber, coords.rows ];
    postMessage(returnData);
}

// *** HP v2: one streaming request for the whole image *** //
// The server builds the reference orbit ONCE, computes small strips in parallel
// on `threads` rayon threads, and streams each strip as an NDJSON line the
// moment it finishes (out of order). Every strip is posted to the page as if it
// were a completed job, so progressive painting and the progress readout work
// unchanged. On a mid-stream failure the whole request is retried and rowsDone
// dedupes already-posted strips (strip boundaries are deterministic for the same
// coords + threads).
let streamAbort = null;   // AbortController of the in-flight HP2 stream
let rowsDone = null;      // per-row posted flags for the current task

function doIterationCountsHP2(coords, thisJobNum, retryCount) {
    if (thisJobNum != jobNumber) {
        return;
    }
    const controller = new AbortController();
    streamAbort = controller;
    fetch(url + "HP2", {
        method: "POST",
        headers: { "Content-Type": "application/json", "Accept": "application/x-ndjson" },
        body: JSON.stringify(coords),
        signal: controller.signal,
    }).then(async (response) => {
        if (response.status == 404) {
            // old server without HP2: whole image in one blocking request
            doIterationCounts(coords, url + "HP", 0, thisJobNum);
            return;
        }
        if (!response.ok) {
            throw new Error("HTTP status " + response.status);
        }
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buf = "";
        for (;;) {
            const { done, value } = await reader.read();
            if (thisJobNum != jobNumber) {
                reader.cancel();
                return;
            }
            if (done) {
                break;
            }
            buf += decoder.decode(value, { stream: true });
            let nl;
            while ((nl = buf.indexOf("\n")) >= 0) {
                const line = buf.slice(0, nl);
                buf = buf.slice(nl + 1);
                if (line.length == 0) continue;
                const strip = JSON.parse(line);
                if (rowsDone[strip.firstRow]) continue;   // retry duplicate
                for (let i = 0; i < strip.nrows; i++) rowsDone[strip.firstRow + i] = 1;
                postMessage([ thisJobNum, coords.firstRow + strip.firstRow,
                    strip.iterationCounts, workerNumber, strip.nrows ]);
            }
        }
    }).catch((err) => {
        if (controller.signal.aborted || thisJobNum != jobNumber) {
            return;
        }
        if (retryCount < retryLimit) {
            retryCount++;
            setTimeout(function() {
                    doIterationCountsHP2(coords, thisJobNum, retryCount);
                },
                retryCount*2000);
        } else {
            console.error("mb-computeHP2 failure, retry limit exceeded!\n" + err);
            // post the rows that never arrived as -1 so the page can finish
            let r = 0;
            while (r < coords.rows) {
                if (rowsDone[r]) { r++; continue; }
                let r0 = r;
                while (r < coords.rows && !rowsDone[r]) r++;
                let counts = [];
                for (let i = 0; i < r - r0; i++) {
                    counts[i] = array_fill(new Array(coords.columns), -1);
                }
                postMessage([ thisJobNum, coords.firstRow + r0, counts, workerNumber, r - r0 ]);
            }
        }
    });
}

let highPrecision;
let maxIterations;
let threadCount = 2;
const url = "mb-compute";
onmessage = function(msg) {
    let data = msg.data;
    if ( data[0] == "stop" ) {
        // Stop pressed: kill the in-flight HP2 stream so the server abandons the
        // remaining strips instead of computing the whole image for nobody.
        if (streamAbort) {
            streamAbort.abort();
            streamAbort = null;
        }
        jobNumber = -1;   // drop any strips already decoded but not yet posted
    } else if ( data[0] == "setup" ) {
        if (jobNumber !== data[1] && streamAbort) {
            streamAbort.abort();   // new job: abandon any in-flight HP2 stream
            streamAbort = null;
        }
        jobNumber = data[1];
        maxIterations = data[2];
        highPrecision = data[3];
        workerNumber = data[4];
        threadCount = data[5] || threadCount;
    } else if ( data[0] == "task" ) {
        let firstRow = data[1];
        let columns = data[2];
        let xmin = highPrecision ? array_from(data[3]) : data[3];
        let dx = highPrecision ? array_from(data[4]) : data[4];
        let ymax = highPrecision ? array_from(data[5]) : data[5];
        let dy = highPrecision ? array_from(data[6]) : data[6];
        let nrows = data[7];

        if (highPrecision) {
            rowsDone = new Uint8Array(nrows);
            doIterationCountsHP2({
                    xmin: xmin, dx: dx, columns: columns, ymax: ymax, dy: dy,
                    firstRow: firstRow, rows: nrows, maxIterations: maxIterations,
                    threads: threadCount
                },
                jobNumber, 0);
        } else {
            doIterationCounts({
                    xmin: xmin, dx: dx, columns: columns, ymax: ymax, dy: dy, firstRow: firstRow, rows: nrows, maxIterations: maxIterations
                },
                url,
                0, jobNumber);
        }
    }
}

function array_from(a) {
    let len = a.length;
    let r = Array(len);
    for (let i = 0; i < len; i++)
        r[i] = a[i];
    return r;
}

function array_fill(a, f) {
    let len = a.length;
    for (let i = 0; i < len; i++)
        a[i] = f;
    return a;
}
