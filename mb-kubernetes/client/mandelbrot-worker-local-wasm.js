let /* boolean */ highPrecision;
let /* int */ maxIterations, jobNumber, workerNumber;
let compute_mandelbrot = null, compute_mandelbrot_hp = null;
let malloc, dalloc;

async function waitForWasm(workerNumber, jobNumber) {
    if (compute_mandelbrot && compute_mandelbrot_hp) {
        return;
    } else {
        await new Promise(resolve => {
            // console.log(`worker ${workerNumber} job ${jobNumber} waiting for WASM`);
            const intervalId = setInterval(() => {
                if (compute_mandelbrot && compute_mandelbrot_hp) {
                    clearInterval(intervalId);
                    resolve();
                }
            }, 25);
        });
    }
}

class WasmMemory {
    constructor(wasmMemory) {
        this.memory = wasmMemory;
        this.capacity = this.memory.buffer.byteLength/Uint32Array.BYTES_PER_ELEMENT;
        // console.log(`creating new WasmMemory32 with capacity ${this.capacity}`)
    }
    newArrayU32(length,log) {
        let ptr = malloc(length*Uint32Array.BYTES_PER_ELEMENT);
        if (log) console.log(ptr, length, this.memory.buffer.byteLength);
        let array = new Uint32Array(this.memory.buffer, ptr, length);
        return array;
    }
    newArrayI32(length) {
        let ptr = malloc(length*Int32Array.BYTES_PER_ELEMENT);
        let array = new Int32Array(this.memory.buffer, ptr, length);
        return array;
    }
    copyFromArrayU32(source,log) {
        let dest = this.newArrayU32(source.length,log);
        dest.set(source);
        return dest;
    }
    free(array) {
        dalloc(array.byteOffset, array.byteLength);
    }
}

let wasmMemory;

onmessage = function(msg) {
    let data = msg.data;
    if ( data[0] == "setup" ) {
        // console.log("setup worker", data[4], "job", data[1], data[2], data[3]);
        jobNumber = data[1];
        maxIterations = data[2];
        highPrecision = data[3];
        workerNumber = data[4];
    } else if ( data[0] == "task" ) {
        // console.log("task job", jobNumber);
        let myJobNumber = jobNumber;
        waitForWasm(workerNumber, jobNumber)
            .then( () => {
                // check that we're still working on this job after waiting for WASM
                if (myJobNumber != jobNumber) {
                    // console.log(`cancelling worker ${workerNumber} job ${myJobNumber}`);
                    return;
                }
                let firstRow = data[1];
                let columnCount = data[2];
                let xmin = data[3];
                let dx = data[4];
                let ymax = data[5];
                let dy = data[6];
                let nrows = data[7];
                if (highPrecision) {
                    // console.log(jobNumber,workerNumber,xmin,dx,columnCount,ymax,maxIterations,highPrecision);
                    xmin = wasmMemory.copyFromArrayU32(xmin);
                    y = wasmMemory.copyFromArrayU32(ymax);
                    createHPData(xmin, dx, dy, columnCount);
                    let returnIterations = new Array(nrows);
                    for (let i = 0; i < nrows; i++) {
                        let iterationCounts = new Array(columnCount);
                        for (let j = 0; j < columnCount; j++) {
                            iterationCounts[j] = compute_mandelbrot_hp(xs[j].byteOffset, y.byteOffset, maxIterations, xmin.length);
                        }
                        returnIterations[i] = iterationCounts;
                        add(y, dy_neg, y.length);
                    }
                    wasmMemory.free(y);
                    wasmMemory.free(xmin);
                    for (i = 0; i < xs.length; i++) {
                        wasmMemory.free(xs[i]);
                    }
                    postMessage([ jobNumber, firstRow, returnIterations, workerNumber, nrows ]);
                } else {
                    let returnIterations = new Array(nrows);
                    let iterationCounts = wasmMemory.newArrayI32(columnCount);
                    for (i = 0; i < nrows; i++) {
                        let y = ymax - (firstRow + i)*dy;
                        compute_mandelbrot(xmin, dx, columnCount, y, maxIterations, iterationCounts.byteOffset);
                        returnIterations[i] = Array.from(iterationCounts);
                    }
                    wasmMemory.free(iterationCounts);
                    postMessage([ jobNumber, firstRow, returnIterations, workerNumber, nrows ]);
                }
            });
    } else if (data[0] == "wasm") {
        // console.log("wasm worker", data[1]);
        WebAssembly
            .instantiate(data[2], { } )
            .then(instance => {
                console.log("loading wasm");
                wasmMemory = new WasmMemory(instance.exports.memory);
                // Hold onto the module's exports so that we can reuse them
                compute_mandelbrot = instance.exports.compute_mandelbrot;
                compute_mandelbrot_hp = instance.exports.count_iterations_hp;
                malloc = instance.exports.malloc;
                dalloc = instance.exports.dalloc;
            });
    }
}

// ------- support for high-precision calculation ------------
var  /* int[][] */ xs;
var /* int[] */ dy_neg;

function createHPData(xmin, dx, dy, columnCount) {
    let chunks = xmin.length - 1;

    dy_neg = new Uint32Array(chunks+1);
    dy_neg.set(dy);
    negate(dy_neg, chunks+1);

    xs = new Array(columnCount);
    xs[0] = wasmMemory.copyFromArrayU32(xmin); // must have xs.length = chunks+1
    for (var i = 1; i < columnCount; i++) {
        xs[i] = wasmMemory.newArrayU32(chunks+1);
        for (var j = 0; j <= chunks; j++) {
            xs[i][j] = xs[i-1][j];
        }
        add(xs[i],dx,chunks+1);
    }
}

function add( /* int[] */ x, /* int[] */ dx, /* int */ count) {
    var carry = 0;
    for (var i = count - 1; i >= 0; i--) {
        x[i] += dx[i];
        x[i] += carry;
        carry = x[i] >>> 16;
        x[i] &= 0xFFFF;
    }
}

function negate( /* int[] */ x, /* int */ chunks) {
    for (var i = 0; i < chunks; i++)
        x[i] = 0xFFFF-x[i];
    ++x[chunks-1];
    for (var i = chunks-1; i > 0 && (x[i] & 0x10000) != 0; i--) {
        x[i] &= 0xFFFF;
        ++x[i-1];
    }
    x[0] &= 0xFFFF;
}
