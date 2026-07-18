let /* boolean */ highPrecision;
let /* int */ maxIterations, jobNumber, workerNumber;
let compute_mandelbrot = null, compute_mandelbrot_hp = null, compute_mandelbrot_hp_perturb = null;
let malloc, dalloc;
// orbit sharing (v8): build one reference orbit per image, reuse across strips
let build_reference_orbit = null, compute_strip_with_orbit = null, malloc_f64 = null, free_f64 = null;
// cached orbit for the image this worker is currently rendering
let cachedImageId = null, cachedOrbit = null; // {ptr, len, meta: Float64Array(6) copy, rowRef}
// meta = [dx_m, dx_e, dy_m, dy_e, dcx0_m, dcx0_e]: FloatExp mantissa/exponent
// pairs -- pixel scales below f64's ~1e-308 floor can't ride through plain f64
// values, so the scale crosses the FFI split and (ox, oy) folding happens
// inside wasm (v11).

// Set to true to log HP compute time per strip and the loaded .wasm size
// (the SIMD build is ~57 KB; the pre-SIMD build was ~43 KB -> a quick cache check).
// This lives in the WORKER scope, not the page: edit it here and hard-reload, or in
// DevTools switch the console's context dropdown to the worker and run `DEBUG = true`.
let DEBUG = false;
let hpMsAccum = 0, hpStripAccum = 0;

// Must match mb_wasm_version() in mb-wasm/src/lib.rs. Bump both together, and
// bump ASSET_VERSION in MB.html so caches can't pair a new worker with an old
// binary (or vice versa).
const EXPECTED_WASM_VERSION = 13;

function wasmReady() {
    return compute_mandelbrot && compute_mandelbrot_hp && compute_mandelbrot_hp_perturb
        && build_reference_orbit && compute_strip_with_orbit && malloc_f64 && free_f64;
}

async function waitForWasm(workerNumber, jobNumber) {
    if (wasmReady()) {
        return;
    } else {
        await new Promise(resolve => {
            // console.log(`worker ${workerNumber} job ${jobNumber} waiting for WASM`);
            const intervalId = setInterval(() => {
                if (wasmReady()) {
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

// Build the whole-image reference orbit from the basis grid coords and cache it.
// Pointers are captured as numbers: the build grows wasm memory (detaching views).
function buildOrbit(imageId, xmin, dx, ymax, dy, basisCols, basisRows, orbitBudget) {
    if (cachedOrbit) {
        free_f64(cachedOrbit.ptr, cachedOrbit.len * 2);
        if (cachedOrbit.dipsLen > 0) free_f64(cachedOrbit.dipsPtr, cachedOrbit.dipsLen * 5);
        cachedOrbit = null;
    }
    let len = xmin.length;
    let xminPtr = wasmMemory.copyFromArrayU32(xmin).byteOffset;
    let dxPtr = wasmMemory.copyFromArrayU32(dx).byteOffset;
    let ymaxPtr = wasmMemory.copyFromArrayU32(ymax).byteOffset;
    let dyPtr = wasmMemory.copyFromArrayU32(dy).byteOffset;
    let lenPtr = malloc(4);
    let metaPtr = malloc_f64(6);
    let dipsPtrPtr = malloc(4), dipsLenPtr = malloc(4);
    let _tb = performance.now();
    let orbitPtr = build_reference_orbit(xminPtr, dxPtr, ymaxPtr, dyPtr, len,
        basisCols, basisRows, maxIterations, orbitBudget || 0, lenPtr, metaPtr, dipsPtrPtr, dipsLenPtr);
    let orbitLen = new Uint32Array(wasmMemory.memory.buffer, lenPtr, 1)[0];
    // copy the meta out of wasm memory (later allocations may grow/detach it)
    let meta = Float64Array.from(new Float64Array(wasmMemory.memory.buffer, metaPtr, 6));
    // dip side table: FloatExp values for orbit points below f64's floor
    // (deep minibrot nuclei); stays in wasm memory alongside the orbit
    let dipsPtr = new Uint32Array(wasmMemory.memory.buffer, dipsPtrPtr, 1)[0];
    let dipsLen = new Uint32Array(wasmMemory.memory.buffer, dipsLenPtr, 1)[0];
    cachedOrbit = { ptr: orbitPtr, len: orbitLen, meta: meta,
        dipsPtr: dipsPtr, dipsLen: dipsLen, rowRef: basisRows >>> 1 };
    cachedImageId = imageId;
    // always logged: builds are rare and expensive, and a build where a cache
    // hit was expected is the first thing to look for when a view is slow
    console.log(`[mb w${workerNumber}] built reference orbit: ${orbitLen} pts, ${dipsLen} dips, ${(performance.now()-_tb).toFixed(0)} ms (image ${imageId})`);
    dalloc(lenPtr, 4); free_f64(metaPtr, 6); dalloc(dipsPtrPtr, 4); dalloc(dipsLenPtr, 4);
    dalloc(dyPtr, len*4); dalloc(ymaxPtr, len*4); dalloc(dxPtr, len*4); dalloc(xminPtr, len*4);
}

onmessage = function(msg) {
    let data = msg.data;
    if ( data[0] == "setup" ) {
        // console.log("setup worker", data[4], "job", data[1], data[2], data[3]);
        jobNumber = data[1];
        maxIterations = data[2];
        highPrecision = data[3];
        workerNumber = data[4];
        if (DEBUG) { hpMsAccum = 0; hpStripAccum = 0; }
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
                // orbit sharing only (all undefined on the classic path):
                let imageRows = data[8];   // BASIS grid rows (pass 1) -- build param
                let imageId = data[9];     // identifies the view's orbit
                let imageCols = data[10];  // BASIS grid columns -- build param
                let ox = data[11], oy = data[12]; // this job's grid offset from the
                                                  // basis grid, in pixels (pass 2: -0.5, +0.5)
                let orbitBudget = data[13];       // reference-orbit point budget
                if (highPrecision && imageId !== undefined) {
                    // ORBIT SHARING: build the reference orbit ONCE per view
                    // (cached by imageId), then grind this strip against it.
                    // xmin/ymax/etc here are the BASIS (pass-1) image coords shared
                    // by every job of both passes; firstRow/columnCount describe
                    // THIS job's sampling grid, offset from the basis by (ox, oy).
                    try {
                        if (cachedImageId !== imageId) {
                            buildOrbit(imageId, xmin, dx, ymax, dy, imageCols, imageRows, orbitBudget);
                        }
                    } catch (err) {
                        // a wasm trap here is almost always memory.grow being
                        // denied (the orbit + table exceed what the browser
                        // grants this worker) -- say so instead of dying mute
                        postMessage(["fatal", `Worker ${workerNumber}: high-precision compute failed (${err}). ` +
                            `Likely out of memory: reduce the number of workers or Max Iterations.`]);
                        return;
                    }
                    let o = cachedOrbit;
                    let outLen = nrows*columnCount;
                    let outPtr = wasmMemory.newArrayI32(outLen).byteOffset;
                    let _t0 = DEBUG ? performance.now() : 0;
                    try {
                        compute_strip_with_orbit(o.ptr, o.len, o.dipsPtr, o.dipsLen,
                            o.meta[0], o.meta[1], o.meta[2], o.meta[3], o.meta[4], o.meta[5],
                            ox, oy, o.rowRef,
                            firstRow, nrows, columnCount, maxIterations, outPtr);
                    } catch (err) {
                        postMessage(["fatal", `Worker ${workerNumber}: high-precision compute failed (${err}). ` +
                            `Likely out of memory: reduce the number of workers or Max Iterations.`]);
                        return;
                    }
                    if (DEBUG) { hpMsAccum += performance.now()-_t0; hpStripAccum += 1; }
                    // fresh view: grinding (BLA table alloc) may have grown memory
                    let counts = new Int32Array(wasmMemory.memory.buffer, outPtr, outLen);
                    let returnIterations = new Array(nrows);
                    for (let i = 0; i < nrows; i++) {
                        returnIterations[i] = Array.from(counts.subarray(i*columnCount, (i + 1)*columnCount));
                    }
                    dalloc(outPtr, outLen*4);
                    postMessage([ jobNumber, firstRow, returnIterations, workerNumber, nrows ]);
                } else if (highPrecision) {
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

                    let _t0 = DEBUG ? performance.now() : 0;
                    compute_mandelbrot_hp_perturb(xminPtr, len, dxPtr, columnCount, ymaxPtr, dyPtr, nrows, maxIterations, outPtr);
                    if (DEBUG) {
                        let ms = performance.now() - _t0;
                        hpMsAccum += ms; hpStripAccum += 1;
                        console.log(`[w${workerNumber}] HP strip ${nrows}x${columnCount} maxIter=${maxIterations}: ${ms.toFixed(2)} ms  (job total ${hpMsAccum.toFixed(1)} ms over ${hpStripAccum} strips)`);
                    }

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
    } else if (data[0] == "buildOrbit") {
        // BROADCAST mode: this worker is the designated builder. Build the orbit
        // from the basis coords, cache it locally, and post a copy of the f64
        // buffer back to the main thread for relay to the other workers.
        // ["buildOrbit", imageId, xmin, dx, ymax, dy, basisCols, basisRows]
        waitForWasm(workerNumber, jobNumber).then(() => {
            let imageId = data[1];
            if (cachedImageId !== imageId) {
                buildOrbit(imageId, data[2], data[3], data[4], data[5], data[6], data[7], data[8]);
            }
            let o = cachedOrbit;
            // copy out of wasm memory (the view may not be transferred directly)
            let copy = new Float64Array(o.len * 2);
            copy.set(new Float64Array(wasmMemory.memory.buffer, o.ptr, o.len * 2));
            let dipsCopy = o.dipsLen > 0
                ? Array.from(new Float64Array(wasmMemory.memory.buffer, o.dipsPtr, o.dipsLen * 5))
                : [];
            postMessage([ "orbit", imageId,
                { len: o.len, meta: Array.from(o.meta), dips: dipsCopy, rowRef: o.rowRef },
                copy ], [ copy.buffer ]);
        });
    } else if (data[0] == "orbit") {
        // BROADCAST mode: receive the relayed orbit and adopt it as the cache.
        // ["orbit", imageId, meta, Float64Array]  (arrives before any task for
        // this image -- main dispatches tasks only after relaying, and per-pair
        // postMessage ordering is FIFO.)
        waitForWasm(workerNumber, jobNumber).then(() => {
            let imageId = data[1], meta = data[2], arr = data[3];
            if (cachedImageId === imageId) {
                return; // the builder itself: already cached in wasm memory
            }
            if (cachedOrbit) {
                free_f64(cachedOrbit.ptr, cachedOrbit.len * 2);
                if (cachedOrbit.dipsLen > 0) free_f64(cachedOrbit.dipsPtr, cachedOrbit.dipsLen * 5);
                cachedOrbit = null;
            }
            let ptr = malloc_f64(arr.length);
            new Float64Array(wasmMemory.memory.buffer, ptr, arr.length).set(arr);
            let dipsLen = meta.dips ? meta.dips.length / 5 : 0;
            let dipsPtr = 0;
            if (dipsLen > 0) {
                dipsPtr = malloc_f64(meta.dips.length);
                new Float64Array(wasmMemory.memory.buffer, dipsPtr, meta.dips.length).set(meta.dips);
            }
            cachedOrbit = { ptr: ptr, len: meta.len,
                meta: Float64Array.from(meta.meta),
                dipsPtr: dipsPtr, dipsLen: dipsLen, rowRef: meta.rowRef };
            cachedImageId = imageId;
            if (DEBUG) console.log(`[w${workerNumber}] adopted broadcast orbit ${meta.len} pts for image ${imageId}`);
        });
    } else if (data[0] == "wasm") {
        // console.log("wasm worker", data[1]);
        WebAssembly
            .instantiate(data[2], { } )
            .then(instance => {
                // console.log("loading wasm");
                // A mismatch here means this worker and mb-wasm.wasm came from
                // different builds; the engine we'd select would be whatever the
                // older half happens to export. Fail loudly rather than silently
                // benchmarking the wrong engine.
                let v = instance.exports.mb_wasm_version;
                let version = v ? v() : 0;
                if (version !== EXPECTED_WASM_VERSION) {
                    let msg = `worker ${data[1]}: expected wasm version ${EXPECTED_WASM_VERSION}, `
                        + `loaded ${version || "none (pre-v4 build)"} -- stale cached mb-wasm.wasm or worker script`;
                    console.error(msg);
                    postMessage([ "fatal", msg ]);
                    throw new Error(msg);
                }
                if (DEBUG) {
                    console.log(`[w${data[1]}] wasm loaded: version ${version}`);
                }
                wasmMemory = new WasmMemory(instance.exports.memory);
                // Hold onto the module's exports so that we can reuse them
                compute_mandelbrot = instance.exports.compute_mandelbrot;
                compute_mandelbrot_hp = instance.exports.compute_mandelbrot_hp;
                compute_mandelbrot_hp_perturb = instance.exports.compute_mandelbrot_hp_perturb;
                malloc = instance.exports.malloc;
                dalloc = instance.exports.dalloc;
                build_reference_orbit = instance.exports.build_reference_orbit;
                compute_strip_with_orbit = instance.exports.compute_strip_with_orbit;
                malloc_f64 = instance.exports.malloc_f64;
                free_f64 = instance.exports.free_f64;
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
