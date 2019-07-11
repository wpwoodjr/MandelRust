
async function fetchIterationCounts(coords) {
    let response = await fetch("url", {
        method: 'POST', // *GET, POST, PUT, DELETE, etc.
        mode: 'no-cors', // no-cors, cors, *same-origin
        cache: 'no-cache', // *default, no-cache, reload, force-cache, only-if-cached
        credentials: 'omit', // include, *same-origin, omit
        headers: {
            'Content-Type': 'application/json',
            // 'Content-Type': 'application/x-www-form-urlencoded',
        },
        redirect: 'follow', // manual, *follow, error
        referrer: 'no-referrer', // no-referrer, *client
        body: JSON.stringify(coords), // body data type must match "Content-Type" header
    });
    return await response.json();
}

function compute(y, xmin, dx, columns, maxIterations) {
//    console.log(y + ", " + xmin + ", " + dx + ", " + columns + ", " + maxIterations);
// -1.1899791231732677, -2.2, 0.005008347245409015, 600, 100

    return fetchIterationCounts({
        y: y, xmin: xmin, dx: dx, columns: columns, maxIterations: maxIterations
    })
}

onmessage = function(msg) {
    let job = msg.data;
    counts = compute(job.y,job.xmin,job.dx,job.columns,job.maxIterations);
    postMessage({
        workerNum: job.workerNum,
        jobNum: job.jobNum,
        row: job.row,
        iterationCounts: counts
    });
}
