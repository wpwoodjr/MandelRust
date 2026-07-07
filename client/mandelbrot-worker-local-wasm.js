let /* boolean */ highPrecision;
let /* int */ maxIterations, jobNumber, workerNumber;
let compute_mandelbrot = null, compute_mandelbrot_hp = null, compute_mandelbrot_hp_perturb = null;
let malloc, dalloc;

async function waitForWasm(workerNumber, jobNumber) {
    if (compute_mandelbrot && compute_mandelbrot_hp && compute_mandelbrot_hp_perturb) {
        return;
    } else {
        await new Promise(resolve => {
            // console.log(`worker ${workerNumber} job ${jobNumber} waiting for WASM`);
            const intervalId = setInterval(() => {
                if (compute_mandelbrot && compute_mandelbrot_hp && compute_mandelbrot_hp_perturb) {
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
        // if (log) console.log(ptr, length, this.memory.buffer.byteLength);
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
                    // Perturbation: one full-precision reference orbit for the whole
                    // strip, then a cheap f64 delta orbit per pixel. The reference
                    // orbit's Vec inside wasm (up to maxIterations entries) can grow
                    // wasm memory mid-call, detaching JS views, so we capture pointers
                    // as numbers up front and rebuild the output view after the call.
                    let len = xmin.length;
                    let xminPtr = wasmMemory.copyFromArrayU32(xmin).byteOffset;
                    let dxPtr = wasmMemory.copyFromArrayU32(dx).byteOffset;
                    let ymaxPtr = wasmMemory.copyFromArrayU32(ymax).byteOffset;
                    let dyPtr = wasmMemory.copyFromArrayU32(dy).byteOffset;
                    let outLen = nrows*columnCount;
                    let outPtr = wasmMemory.newArrayI32(outLen).byteOffset;

                    compute_mandelbrot_hp_perturb(xminPtr, len, dxPtr, columnCount, ymaxPtr, dyPtr, nrows, maxIterations, outPtr);

                    // fresh view: wasm memory may have grown (and detached old views)
                    let counts = new Int32Array(wasmMemory.memory.buffer, outPtr, outLen);
                    let returnIterations = new Array(nrows);
                    for (let i = 0; i < nrows; i++) {
                        returnIterations[i] = Array.from(counts.subarray(i*columnCount, (i + 1)*columnCount));
                    }
                    let U32 = Uint32Array.BYTES_PER_ELEMENT, I32 = Int32Array.BYTES_PER_ELEMENT;
                    dalloc(outPtr, outLen*I32);
                    dalloc(dyPtr, len*U32);
                    dalloc(ymaxPtr, len*U32);
                    dalloc(dxPtr, len*U32);
                    dalloc(xminPtr, len*U32);
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
                // console.log("loading wasm");
                wasmMemory = new WasmMemory(instance.exports.memory);
                // Hold onto the module's exports so that we can reuse them
                compute_mandelbrot = instance.exports.compute_mandelbrot;
                compute_mandelbrot_hp = instance.exports.compute_mandelbrot_hp;
                compute_mandelbrot_hp_perturb = instance.exports.compute_mandelbrot_hp_perturb;
                malloc = instance.exports.malloc;
                dalloc = instance.exports.dalloc;
            });
    }
}


// ------- support for high-precision calculation ------------

function incr( /* int[] */ x, /* int[] */ dx) {
    let len = x.length;
    var carry = 0;
    for (var i = len - 1; i >= 0; i--) {
        x[i] += dx[i];
        x[i] += carry;
        carry = x[i] >>> 16;
        x[i] &= 0xFFFF;
    }
}

function negate( /* int[] */ x) {
    let len = x.length;
    for (var i = 0; i < len; i++)
        x[i] = 0xFFFF-x[i];
    ++x[len-1];
    for (var i = len-1; i > 0 && (x[i] & 0x10000) != 0; i--) {
        x[i] &= 0xFFFF;
        ++x[i-1];
    }
    x[0] &= 0xFFFF;
}
