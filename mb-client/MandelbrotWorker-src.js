var Mandelbrot = require('mandelbrot');

var api = new Mandelbrot.MandelbrotApi()

var mandelbrotCoords = new Mandelbrot.MandelbrotCoords(); // {MandelbrotCoords} Mandelbrot coordinates to compute

/*
api.computeMandelbrot(mandelbrotCoords).then(function(data) {
  console.log('API called successfully. Returned data: ' + data);
}, function(error) {
  console.error(error);
});
*/

function compute(y, xmin, dx, columns, maxIterations) {
//    console.log(y + ", " + xmin + ", " + dx + ", " + columns + ", " + maxIterations);
// -1.1899791231732677, -2.2, 0.005008347245409015, 600, 100
    console.log(mandelbrotCoords);
    iterationCounts = [];
    let x0 = xmin;
    for (let i = 0; i < columns; i++) {
        let y0 = y;
        let a = x0;
        let b = y0;
        let ct = 0;
        while (a*a + b*b < 4.1) {
            ct++;
            if (ct > maxIterations) {
                ct = -1;
                break;
            }
            let newa = a*a - b*b + x0;
            b = 2*a*b + y0;
            a = newa;
        }
        iterationCounts[i] = ct;
        x0 += dx;
    }
    return iterationCounts;
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
