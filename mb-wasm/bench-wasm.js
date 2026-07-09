// Drive the deployed mb-wasm.wasm directly: time compute_mandelbrot_hp_perturb
// on the 35-digit and 126-digit views, 32-row strips, no browser pipeline.
const fs = require("fs");

const views = {
  "35-digit ": {
    xmin: [65534, 9144, 56334, 65399, 61036, 36172, 52659, 28981, 4430, 17519],
    dx: [0, 0, 0, 0, 0, 0, 1, 57312, 56425, 22742],
    ymax: [0, 8, 21677, 19005, 52403, 49030, 53408, 23540, 50755, 35330],
    dy: [0, 0, 0, 0, 0, 0, 1, 57364, 10543, 8664],
    maxIter: 2000,
  },
  "126-digit": {
    xmin: [65534, 9144, 56334, 65399, 61036, 36181, 257, 1520, 31979, 726, 42080, 47440, 30505, 25996, 33085, 56186, 41268, 2729, 37082, 20510, 29642, 54243, 494, 6659, 11729, 50848, 21020, 21004, 57914],
    dx: [...Array(23).fill(0), 10, 65312, 38968, 51516, 23555, 17685],
    ymax: [0, 8, 21677, 19005, 52403, 48987, 16498, 46084, 63934, 24332, 32560, 53815, 2636, 12957, 1511, 18787, 58966, 62864, 15526, 44179, 39935, 52063, 47770, 5526, 27208, 5500, 6279, 824, 59403],
    dy: [...Array(23).fill(0), 11, 77, 24652, 30750, 20182, 15135],
    maxIter: 50000,
  },
};

// 16-bit digit helpers (from the worker)
function incr(x, dx) {
  let carry = 0;
  for (let i = x.length - 1; i >= 0; i--) {
    x[i] += dx[i] + carry;
    carry = x[i] >>> 16;
    x[i] &= 0xffff;
  }
}
function negate(x) {
  const len = x.length;
  for (let i = 0; i < len; i++) x[i] = 0xffff - x[i];
  ++x[len - 1];
  for (let i = len - 1; i > 0 && (x[i] & 0x10000) != 0; i--) {
    x[i] &= 0xffff;
    ++x[i - 1];
  }
  x[0] &= 0xffff;
}

async function main() {
  const bytes = fs.readFileSync(require("path").join(__dirname, "..", "client", "mb-wasm.wasm"));
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  console.log("wasm version:", e.mb_wasm_version ? e.mb_wasm_version() : "absent");

  const ROWS = 600, COLS = 800, STRIP = 32;
  for (const [name, v] of Object.entries(views)) {
    const len = v.xmin.length;
    // ymax walks down per strip
    const ymax = v.ymax.slice();
    const dyNeg = v.dy.slice();
    negate(dyNeg);

    const t0 = performance.now();
    let computeMs = 0;
    for (let r0 = 0; r0 < ROWS; r0 += STRIP) {
      const h = Math.min(STRIP, ROWS - r0);
      // growth-safe: capture pointers as numbers
      const alloc = (arr) => {
        const p = e.malloc(arr.length * 4);
        new Uint32Array(e.memory.buffer, p, arr.length).set(arr);
        return p;
      };
      const pXmin = alloc(v.xmin), pDx = alloc(v.dx), pYmax = alloc(ymax), pDy = alloc(v.dy);
      const outLen = h * COLS;
      const pOut = e.malloc(outLen * 4);
      const t = performance.now();
      e.compute_mandelbrot_hp_perturb(pXmin, len, pDx, COLS, pYmax, pDy, h, v.maxIter, pOut);
      computeMs += performance.now() - t;
      e.dalloc(pOut, outLen * 4);
      e.dalloc(pDy, len * 4); e.dalloc(pYmax, len * 4); e.dalloc(pDx, len * 4); e.dalloc(pXmin, len * 4);
      for (let i = 0; i < h; i++) incr(ymax, dyNeg);
    }
    const total = performance.now() - t0;
    console.log(`${name}: compute ${computeMs.toFixed(0)} ms for ${ROWS} rows -> ${(ROWS / (computeMs / 1e3)).toFixed(0)} rows/s (total incl. JS ${total.toFixed(0)} ms)`);
  }
}
main();
