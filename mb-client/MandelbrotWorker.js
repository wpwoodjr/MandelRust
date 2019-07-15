function fetchIterationCounts(coords) {
    //console.log(JSON.stringify(coords));

    var client = new XMLHttpRequest();
    client.open("POST", "http://localhost:8080/v1/mandelbrot/compute", false);
    client.setRequestHeader("Content-Type", "application/json");
    client.setRequestHeader("Accept", "application/json");
    client.send(JSON.stringify(coords));
/*
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
*/
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
    // -1.1899791231732677, -2.2, 0.005008347245409015, 600, 100
    let job = msg.data;
    let counts = fetchIterationCounts({
        y: job.y, xmin: job.xmin, dx: job.dx, columns: job.columns, maxIterations: job.maxIterations
    });
    //console.log(job.jobNum, job.workerNum, job.row, counts);
    postMessage({
        workerNum: job.workerNum,
        jobNum: job.jobNum,
        row: job.row,
        iterationCounts: counts
    });
}
