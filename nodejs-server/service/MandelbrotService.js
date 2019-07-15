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
        let x = mandelbrotCoords.xmin;
        let dx = mandelbrotCoords.dx;
        let columns = mandelbrotCoords.columns;
        let maxIterations = mandelbrotCoords.maxIterations;

        let iterationCounts = new Array(columns);
        for (let i = 0; i < columns; i++) {
            iterationCounts[i] = countIterations(x, y, maxIterations);
            x += dx;
        }

        resolve(iterationCounts);
    });
}

function countIterations( /* double */ x, /* double */ y, maxIterations) {
    let count = 0;
    let zx = x;
    let zy = y;
    while (count < maxIterations
            && zx*zx + zy*zy <= 4) {
        let new_zx = zx*zx - zy*zy + x;
        zy = 2*zx*zy + y;
        zx = new_zx;
        count++;
    }
    return (count < maxIterations)? count : -1 ;
}
