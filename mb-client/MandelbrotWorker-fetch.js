async function fetchIterationCounts(coords) {
    //console.log(JSON.stringify(coords));
    let response = await fetch("http://localhost:8081/v1/mandelbrot/compute", {
        method: 'POST', // *GET, POST, PUT, DELETE, etc.
        mode: 'cors', // no-cors, cors, *same-origin
        //cache: 'no-cache', // *default, no-cache, reload, force-cache, only-if-cached
        //credentials: 'omit', // include, *same-origin, omit
        headers: {
            'Content-Type': 'application/json',
            'Accept': 'application/json'
            // 'Content-Type': 'application/x-www-form-urlencoded',
        },
        //redirect: 'follow', // manual, *follow, error
        //referrer: 'no-referrer', // no-referrer, *client
        body: JSON.stringify(coords), // body data type must match "Content-Type" header
    });

    let retval = await response;
    if (retval.status == 200) {
        console.log(retval);
        return retval.json();
    } else {
        console.log(retval);
        let iterationCounts = [];
        for (let i = 0; i < coords.columns; i++) {
            iterationCounts[i] = -1;
        }
        return iterationCounts;
    }
}

function compute(y, xmin, dx, columns, maxIterations) {
//    console.log(y + ", " + xmin + ", " + dx + ", " + columns + ", " + maxIterations);
// -1.1899791231732677, -2.2, 0.005008347245409015, 600, 100

    let retval = fetchIterationCounts({
        y: y, xmin: xmin, dx: dx, columns: columns, maxIterations: maxIterations
    });
    return retval;
}

onmessage = function(msg) {
    let job = msg.data;
    let counts = compute(job.y,job.xmin,job.dx,job.columns,job.maxIterations);
    console.log(counts);
    postMessage({
        workerNum: job.workerNum,
        jobNum: job.jobNum,
        row: job.row,
        iterationCounts: counts
    });
}
