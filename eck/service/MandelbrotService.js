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
};

function countIterations( /* double */ x, /* double */ y, maxIterations) {
    let count = 0;
    let zx = x;
    let zy = y;
    while (count < maxIterations
            && zx*zx + zy*zy < 8) {        // < 8 to match orig version?  Maybe HP uses 8?
        let new_zx = zx*zx - zy*zy + x;
        zy = 2*zx*zy + y;
        zx = new_zx;
        count++;
    }
    return (count < maxIterations)? count : -1 ;
}

exports.computeMandelbrotHP = function(mandelbrotCoords) {
    return new Promise(function(resolve, reject) {
        let y = new Uint32Array(mandelbrotCoords.y);
        let x = new Uint32Array(mandelbrotCoords.xmin);
        let dx = new Uint32Array(mandelbrotCoords.dx);
        let columns = mandelbrotCoords.columns;
        let maxIterations = mandelbrotCoords.maxIterations;

        //console.log(y, x, dx, columns, maxIterations, ArrayType);
        let iterationCounts = new Array(columns);
        createHPData(x, dx, columns);
        for (let i = 0; i < columns; i++) {
            iterationCounts[i] = countIterationsHP(xs[i], y, maxIterations);
        }

        resolve(iterationCounts);
    });
};

function countIterationsHP( /* Uint32Array */ x, /* Uint32Array */ y, maxIterations) {
    arraycopy(x,0,zx,0,chunks);
    arraycopy(y,0,zy,0,chunks);
    let count = 0;
    while (count < maxIterations) {
        arraycopy(zx, 0, work2, 0, chunks);
        multiply(work2,zx,chunks);  // work2 = zx*zx
        arraycopy(zy, 0, work1, 0, chunks);
        multiply(work1,zy,chunks);  // work1 = zy*zy
        arraycopy(work1,0,work3,0,chunks);   // work3 = zy*zy, save a copy.  (Note: multiplication uses work3.)
        add(work1,work2,chunks);  // work1 = zx*zx + zy*zy
        if ((work1[0] & 0xFFF8) != 0 && (work1[0] & 0xFFF8) != 0xFFF0)
            break;
        negate(work3,chunks);  // work3 = -work3 = -zy*zy
        add(work2,work3,chunks);  // work2 = zx*zx - zy*zy
        add(work2,x,chunks); // work2 = zx*zx - zy*zy + x, the next value for zx
        arraycopy(zx,0,work1,0,chunks);  // work1 = zx
        add(work1,zx,chunks);  // work1 = 2*zx
        multiply(work1,zy,chunks);  // work1 = 2*zx*zy
        add(work1,y,chunks);  // work1 = 2*zx*zy + y, the next value for zy
        arraycopy(work1,0,zy,0,chunks);  // zy = work1
        arraycopy(work2,0,zx,0,chunks);  // zx = work2
        count++;
    }
    return (count < maxIterations)? count : -1 ;
}

function arraycopy( sourceArray, sourceStart, destArray, destStart, count ) {
   for (let i = 0; i < count; i++) {
       destArray[destStart + i] = sourceArray[sourceStart + i];
   }
}


// ------- support for high-precision calculation ------------

var  /* int[][] */ xs;
var /* int[] */ y;

var /* int[] */ work1,work2,work3;
var /* int[] */ zx, zy;

var /* int */ chunks;

var /* double */ log2of10 = Math.log(10)/Math.log(2);

let ArrayType = Uint32Array;

function createHPData(xmin, dx, columnCount) {
    chunks = xmin.length - 1;
    y = new ArrayType(chunks+1);
    xs = new Array(columnCount);
    xs[0] = xmin; // must have xs.length = chunks+1
    for (let i = 1; i < columnCount; i++) {
        xs[i] = new ArrayType(chunks+1);
    }
    if (columnCount > 1) {
        for (let i = 1; i < columnCount; i++) {
            for (let j = 0; j <= chunks; j++) {
                xs[i][j] = xs[i-1][j];
            }
            add(xs[i],dx,chunks+1);
        }
    }
    zx = new ArrayType(chunks);
    zy = new ArrayType(chunks);
    work1 = new ArrayType(chunks);
    work2 = new ArrayType(chunks);
    work3 = new ArrayType(chunks);
}

function add( /* int[] */ x, /* int[] */ dx, /* int */ count) {
    let carry = 0;
    for (let i = count - 1; i >= 0; i--) {
        x[i] += dx[i];
        x[i] += carry;
        carry = x[i] >>> 16;
        x[i] &= 0xFFFF;
    }
}

function multiply( /* int[] */ x, /* int[] */ y, /* int */ count){  // Can't allow x == y !
    let neg1 = (x[0] & 0x8000) != 0;
    if (neg1)
        negate(x,count);
    let neg2 = (y[0] & 0x8000) != 0;
    if (neg2)
        negate(y,count);
    if (x[0] == 0) {
        for (let i = 0; i < count; i++)
            work3[i] = 0;
    }
    else {
        let carry = 0;
        for (let i = count-1; i >= 0; i--) {
            work3[i] = x[0]*y[i] + carry;
            carry = work3[i] >>> 16;
            work3[i] &= 0xFFFF;
        }
    }
    for (let j = 1; j < count; j++) {
        let i = count - j;
        let carry = (x[j]*y[i]) >>> 16;
        i--;
        let k = count - 1;
        while (i >= 0) {
            work3[k] += x[j]*y[i] + carry;
            carry = work3[k] >>> 16;
            work3[k] &= 0xFFFF;
            i--;
            k--;
        }
        while (carry != 0 && k >= 0) {
            work3[k] += carry;
            carry = work3[k] >>> 16;
            work3[k] &= 0xFFFF;
            k--;
        }
    }
    arraycopy(work3,0,x,0,count);
    if (neg2)
        negate(y,count);
    if (neg1 != neg2)
        negate(x,count);
}

function negate( /* int[] */ x, /* int */ chunks) {
    for (let i = 0; i < chunks; i++)
        x[i] = 0xFFFF-x[i];
    ++x[chunks-1];
    for (let i = chunks-1; i > 0 && (x[i] & 0x10000) != 0; i--) {
        x[i] &= 0xFFFF;
        ++x[i-1];
    }
    x[0] &= 0xFFFF;
}
