let /* boolean */ highPrecision;
let /* int */ maxIterations, jobNumber, workerNumber;

let url = "http://localhost:8081/v1/mandelbrot/compute";
//let url = "https://mandelbrot.stagek8s.gsk.com/v1/mandelbrot/compute";
let retryLimit = 5;

function doIterationCounts(coords, url, taskNumber, retryCount) {
    let iterationCounts;
    try {
        let client = new XMLHttpRequest();
        client.open("POST", url, false);
        client.setRequestHeader("Content-Type", "application/json");
        client.setRequestHeader("Accept", "application/json");
        if (highPrecision) {
            coords.y = array_from(coords.y);
            coords.xmin = array_from(coords.xmin);
            coords.dx = array_from(coords.dx);
        }
        client.send(JSON.stringify(coords));

        if (client.status == 200) {
            iterationCounts =  JSON.parse(client.response);
        } else {
            console.log("Error: " + client);
            iterationCounts = array_fill(new Array(coords.columns), -1);
        }
    } catch(err) {
        if (retryCount < retryLimit) {
            retryCount++;
            console.log("XMLHttpRequest failure, retrying " + retryCount + " of " + retryLimit + "...\n" + err);
            setTimeout(function() {
                    doIterationCounts(coords, url, taskNumber, retryCount);
                },
                retryCount*1000);
            return;
        } else {
            console.log("XMLHttpRequest failure, retry limit exceeded!\n" + err);
            iterationCounts = array_fill(new Array(coords.columns), -1);
        }
    }
    let returnData = [ jobNumber, taskNumber, iterationCounts, workerNumber ];
    postMessage(returnData);
}

onmessage = function(msg) {
    let data = msg.data;
    if ( data[0] == "setup" ) {
        jobNumber = data[1];
        maxIterations = data[2];
        highPrecision = data[3];
        workerNumber = data[4];
    } else if ( data[0] == "task" ) {
        let taskNumber = data[1];
        let columns = data[2];
        let xmin = data[3];
        let dx = data[4];
        let y = data[5];
        //console.log(y,xmin,dx,columns,maxIterations,highPrecision);

        doIterationCounts({
                y: y, xmin: xmin, dx: dx, columns: columns, maxIterations: maxIterations
            },
            highPrecision ? url + "HP" : url,
            taskNumber,
            0);
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
