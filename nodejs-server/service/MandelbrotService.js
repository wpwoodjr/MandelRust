'use strict';


/**
 * Iterate over pixels to determine if each is in the Mandelbrot set
 *
 *
 * mandelbrotCoords MandelbrotCoords Mandelbrot coordinates to compute
 * returns MandelbrotResults
 **/
exports.computeMandelbrot = function(mandelbrotCoords) {
    return new Promise(function(resolve, reject) {

        let y = mandelbrotCoords.y;
        let xmin = mandelbrotCoords.xmin;
        let dx = mandelbrotCoords.dx;
        let columns = mandelbrotCoords.columns;
        let maxIterations = mandelbrotCoords.maxIterations;

        let iterationCounts = [];
        let x0 = xmin;
        for (let i = 0; i < columns; i++) {
            let y0 = y;
            let a = x0;
            let b = y0;
            let ct = 0;
            while (a*a + b*b < 4.0) {
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
        resolve(iterationCounts);
    });
}
