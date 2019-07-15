let /* boolean */ highPrecision;
let /* int */ maxIterations, jobNumber, workerNumber;

//let url = "http://localhost:8081/v1/mandelbrot/compute";
let url = "https://mandelbrot.devk8s.gsk.com/v1/mandelbrot/compute";

function fetchIterationCounts(coords) {
    let client = new XMLHttpRequest();
    client.open("POST", url, false);
    client.setRequestHeader("Content-Type", "application/json");
    client.setRequestHeader("Accept", "application/json");
    client.send(JSON.stringify(coords));

    if (client.status == 200) {
        return JSON.parse(client.response);
    } else {
        console.error(client);
        let iterationCounts = [];
        for (let i = 0; i < coords.columns; i++) {
            iterationCounts[i] = -1;
        }
        return iterationCounts;
    }
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

        let iterationCounts = fetchIterationCounts({
            highPrecision: highPrecision, y: y, xmin: xmin, dx: dx, columns: columns, maxIterations: maxIterations
        });
        let returnData = [ jobNumber, taskNumber, iterationCounts, workerNumber ];
        postMessage(returnData);
    }
}
