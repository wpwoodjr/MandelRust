# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MandelRust is a high-performance Mandelbrot Set viewer web application. It uses Rust for computationally intensive calculations, deployed via WebAssembly for in-browser computation or as a backend server for maximum performance.

Live demo: https://wpwoodjr.github.io/MandelRust

## Build Commands

```bash
# Local build (builds Rust server + WASM)
./local-build.sh

# Local run (serves on localhost:8000)
./local-run.sh

# Docker build and run (serves on localhost:8001)
./docker-build.sh
./docker-run.sh

# Build only WASM module
cd mb-wasm && ./build.sh

# Build only server
cd mb-rust-server && cargo build --release
```

## Server CLI Options

```bash
mb-rust-server [URL] [OPTIONS]
  URL               # Defaults to localhost:8000
  --orbit-cache N   # Reference-orbit cache budget in MB (default 128, 0 = off).
                    # Keyed by view coords; pass 2 / re-renders skip the orbit
                    # build. Real bound: max(N, largest single orbit)
  # Legacy options -- apply only to the old per-job /mb-computeHP endpoint
  # (current clients use /mb-computeHP2 and pick their own thread count):
  -r, --rayon N     # Rayon threads per legacy request (default: 2)
  --no-perturb      # Disable the perturbation engine (legacy endpoint)
  --u32/--u64/--u128 # Legacy half-limb engines; also disable perturbation
```

## Architecture

**Three-tier computation model:**
1. **Browser JS** (slowest) - JavaScript fallback for compatibility
2. **Browser WASM** (5-7x faster) - Rust compiled to WebAssembly
3. **Backend Server** (10-15x faster) - Rust server with Rayon parallelization

**Key directories:**
- `mb-arith/` - Core Mandelbrot arithmetic library (low and high precision)
- `mb-rust-server/` - Actix-web backend server
- `mb-wasm/` - WebAssembly compilation target (FFI exports)
- `client/` - Web frontend (MB.html is main entry point)

**Server API endpoints:**
- `POST /mb-compute` - Low precision (f64) calculation
- `POST /mb-computeHP` - High precision, legacy: one 32-row job per request,
  banded by `-r`, orbit rebuilt per band. Kept for old clients
- `POST /mb-computeHP2` - High precision v2: ONE request per image/pass
  (`threads` in the body, clamped server-side; `-r` does not apply). The body's
  coords are the BASIS grid the orbit is built on, plus optional
  `basisRows/basisColumns/ox/oy` when the sampling grid differs (pass 2 sends
  pass 1's coords with offsets (-0.5, +0.5)). Orbits are cached by basis coords
  (byte-budgeted LRU, `--orbit-cache`, Arc'd so eviction can't free an orbit an
  in-flight request is grinding against), so pass 2 and repeated views skip the
  ~0.5s build. Strips stream as NDJSON lines the moment they finish (out of
  order; client paints by firstRow). A dropped connection aborts remaining
  strips
- `GET /remoteCanComputeMB` - Health check

**Worker scripts in client/:**
- `mandelbrot-worker-local-js.js` - JavaScript compute worker
- `mandelbrot-worker-local-wasm.js` - WASM compute worker
- `mandelbrot-worker-remote.js` - Remote server worker

## High Precision Calculations

Coordinates are passed as `Vec<u32>` arrays of 16-bit digits (element 0 = signed integral part, rest = fraction, two's complement). Key functions in `mb-arith/src/lib.rs`:
- `count_iterations()` - Standard f64 Mandelbrot iteration
- `count_iterations_hp64()` - High precision iteration, full-width u64 limbs with 64x64->128 widening multiplies and ADC carry chains; truncated Comba multiply and symmetric squaring. Server default; bit-identical results to the legacy engine (see tests in `mb-arith`)
- `count_iterations_hp32()` - Same engine with u32 limbs and 32x32->64 products; used by mb-wasm because wasm32 has no 64x64->128 multiply. The `fw_engine!` macro generates both widths, with accumulation strategy selected per target (ADC chains on native, branchless hi/lo split on wasm). With the `simd128` cargo feature (enabled by mb-wasm), columns of 11+ limbs use SIMD128 `extmul` kernels (`simd32` module) for up to ~1.4x more at deep zooms; requires a SIMD-capable wasm engine (all major browsers since 2021, Safari 16.4+)
- `count_iterations_hp()` - Legacy generic engine (`u32`/`u64`/`u128` half-limb); only used by the server's `--u32/--u64/--u128` flags now
- `mandelbrot_row_hp64()` / `mandelbrot_row_hp32()` - Compute a whole pixel row (x advances by dx at full precision). Dispatch to const-generic kernels (`count_iterations_hp64_n::<N>`, stack-array workspaces, fully unrolled) for 1-8 limbs, dynamic fallback beyond. This is what the server and mb-wasm call
- `HPData64` / `HPData32` / `HPData<T>` - Workspace structs for HP calculations

Benchmark the two engines with `cargo run --release --example bench-hp` in `mb-arith/`.

## Perturbation Engine (mb-arith/src/perturbation.rs)

HP images default to perturbation theory on both tiers: one full-precision
reference orbit per block (stored as f64 pairs), then a cheap f64 delta orbit per
pixel — `d' = 2*Z_n*d + d^2 + dc`. Depth-independent per-pixel cost; f64 deltas
are usable down to ~1e-300 pixel scale. Engines, generated for u64/u32 limbs by
the `perturb_engine!` macro (BLA drivers are width-thin wrappers over shared f64
code):

- `mandelbrot_perturb_bla64/32()` - **current default at all depths** (server +
  wasm v7). BLA (bivariate linear approximation) over the Zhuoran rebasing
  engine: while |d| << |Z_n| the delta step is effectively linear (d' ~= A*d +
  B*dc) and linear steps compose, so `build_bla_table()` precomputes merged
  skips of 2^k iterations along the reference orbit (flat array, power-of-two
  aligned, per-level offsets), each with a validity radius guaranteeing a skip
  can miss neither an escape nor a rebase. `perturb_point_bla()` takes the
  longest valid skip per iteration (radii are monotone up the levels: scan
  upward, stop at first failure) and falls back to exact steps near escapes and
  reference zeros. `BLA_EPS` (2^-40) holds results at the f64 noise floor of
  the method itself. Skips ~90-99% of iterations at deep zoom: measured 7-11x
  over the glitch engine at 126 decimal digits, 1.1-2x at 35 digits.
  PERF NOTE: |d|^2 is carried across loop iterations, NOT recomputed — a
  redundant multiply-add in the serial dependency chain cost wasm ~2.7x (native
  OoO hid it, V8 didn't) and native ~1.2x.
- `mandelbrot_perturb_glitch64/32()` - previous default. Shared-index engine:
  all pixels walk the reference at the same index, batched 4 per loop through
  `perturb_lanes_shared::<4>` (branchless lock-step; ~1.3-4x from
  instruction-level parallelism, core-dependent). Glitches detected by the
  Pauldelbrot criterion, corrected by re-referencing passes, stragglers fall
  back to brute HP. Superseded by BLA everywhere, kept as the non-BLA fallback.
- `mandelbrot_perturb64/32()` - plain rebasing engine (`perturb_point`: reset
  d:=z, index:=0 when |z|<|d|); glitch-free single reference, the base BLA
  builds on.
- Explicit SIMD kernels (`perturb_pair_shared_neon`, `perturb_pair_shared_wasm`)
  exist but are UNSHIPPED: benched slower than scalar lanes on Cortex-X925/A720
  (mask/select overhead exceeds what ILP already provides); NEON wins only on
  Cortex-X4. Note BLA's per-pixel control flow (variable skips, no gather loads
  in NEON/SIMD128) makes cross-pixel SIMD moot now. Any single-kernel choice is
  a compromise on big.LITTLE — benchmark per core type (`taskset -c N`) before
  believing any perf number on this machine.

Job sizing (client, MB.html): local HP jobs target ~8 jobs/worker (clamped 4..32
rows; a lone worker gets 32s), then `splitHPTailJobs()` re-splits the
last-dispatched jobs into 4-row strips after interlace reordering (workers pop()
from the array end, so the array FRONT dispatches last). Remote jobs are fixed 32
rows — each costs an HTTP round trip the server idles through; the server splits
each request into one band per Rayon thread (`compute_mandelbrot_perturb64`).

Benchmarks in `mb-arith/examples/`: `bench-real.rs` (126-digit saved view, exact
client digit pipeline, BENCH_START/BENCH_ROWS slicing, engines A/B/D/E/F),
`bench-real35.rs` (35-digit view), `bench-perturb.rs` (single vs 2/4-lane vs NEON
kernel isolation), `bench-strips.rs` (strip-height sweep). For wasm numbers, run
`node mb-wasm/bench-wasm.js` against the deployed `client/mb-wasm.wasm` — it
predicts browser single-worker rows/sec within ~1%. Use it before trusting any
wasm perf theory (browser pipeline noise is not the wasm engine).

Measured on the dev machine (800x600, 126-digit view, rows/sec, see bmarks.txt):
browser 8-worker 101 -> 1440 and single-worker 17 -> 281 across the perturbation
+ BLA work; server 8-worker ~1600-1900. A 4-row strip that takes exact HP ~2.1 s
computes in ~9 ms, bit-identical on a server strip test (BLA differs from the
exact engines on ~0.002% of pixels by <= 6 counts on the worst view tested).

## Build Configuration

Release builds use aggressive optimizations (`Cargo.toml`):
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

WASM builds additionally strip symbols and abort on panic for minimal binary size.

## Client Asset Versioning

`MB.html` is the main document (always revalidated); the worker scripts and
`mb-wasm.wasm` are subresources it loads by URL. `stamp-assets.sh` (run by
`mb-wasm/build.sh`, hence by `local-build.sh`) hashes the 8 cache-sensitive
assets and stamps that hash into `MB.html` twice: as `ASSET_VERSION` (used by JS
to fetch the wasm and spawn workers) and as `?v=<hash>` on the four `<script
src>` tags, which are markup and cannot read a JS constant. Changed content
therefore always means a changed cache key. `stamp-assets.sh --check` exits 1 if
either stamp is stale (compares content, never mtimes — `git checkout` rewrites
mtimes on every branch switch); run it before committing, since editing a worker
or page script by hand triggers no build and so no re-stamp.

The server sends `Cache-Control: max-age=600`, matching what GitHub Pages serves
so the local server behaves like the live demo. Sending *no* header is NOT
equivalent and must not be "simplified" to that: with no `Cache-Control` the
browser invents a heuristic freshness window (~10% of the file's age, unbounded),
which is exactly how a stale worker once outlived its wasm.

`MB.html` cannot be versioned — it is the entry point and carries the hash — so
it can be up to 10 min stale, and a stale `MB.html` hands out an old
`ASSET_VERSION`, quietly loading a *coherent* old build (old worker + old wasm
agree, so the version gate never fires). BENCH NOTE: hard-reload
(ctrl-shift-R) before trusting any browser benchmark; pasting a saved-view URL
from `bmarks.txt` is a navigation, not a reload, and is cache-eligible.

Independently, the wasm worker checks `mb_wasm_version()` against
`EXPECTED_WASM_VERSION` and fails loudly on mismatch, which catches the reverse
skew (new worker, old wasm).
CACHE NOTE: without this, a browser-cached worker silently outlived the binary it
drove. The perturbation branch's `bmarks.txt` browser row was measured with a
speedup-era worker calling `compute_mandelbrot_hp` on a v4 binary — brute HP, ~9x
and ~60x understated, no error shown. Any two of {worker, wasm} from different
builds can produce plausible-but-wrong numbers, so never trust a browser
benchmark taken on a reused origin (port) without a cache-disabled reload.

## Current Development

Main stable branch is `master`. The `perturbation` branch holds the perturbation
engines; the `BLA` branch (this work) adds BLA on top — both shipped as defaults
(wasm v7, server). The `gpu` branch contains experimental WebGPU acceleration
work.

Next steps (after the `BLA` branch):
1. **floatexp deltas** (f64 mantissa + i64 exponent) to push perturbation past
   the ~1e-300 pixel-scale f64 underflow floor (~300 decimal digits) — now the
   only depth limit.
2. Merge `BLA` -> `perturbation` -> `master` once soaked.
3. **Orbit sharing** — build the reference orbit once and reuse it across a
   worker's strips (see below). NOTE: an earlier note here called this "a few %
   of HP setup per job" — that is WRONG at depth. On a 270-digit view the
   per-strip orbit rebuild is 90%+ of the work; this is the dominant HP lever,
   not smaller fry. Being prototyped on the `BLA-orbit-sharing` branch.
4. Revisit within-pixel SIMD only on x86 hardware (AVX2 shuffles are cheaper —
   measure, don't assume).

## Orbit-Rebuild Bottleneck / Orbit Sharing (BLA-orbit-sharing branch)

The single biggest HP lever at depth, found by benchmarking a 270-digit view.
Both tiers rebuild the reference orbit + BLA table **per strip** (per local job,
per server band). The orbit is ~max_iter HP iterations at N limbs and dominates:
on the 270-digit view it is 90%+ of a small strip's time. So the whole
throughput story reduces to ONE quantity — **builds per row** — and every knob
(browser strip size, server `-r`, worker count, band size, image height) is that
quantity wearing a disguise.

Consequences measured (Beast i9-13900KF, see bmarks.txt):
- Big bands win by rebuilding fewer orbits per row: browser 80 -> 472 rows/sec on
  the 270-digit view purely by strip sizing. band-128 wasm beat band-64 native —
  fewer builds beat the wasm penalty.
- Native compute ceiling (`bench-server270`) ~500 rows/sec at 32 threads; the
  480-row image caps at ~305 only because it can't feed 32 threads with big
  bands (geometry, not the box).
- Big bands have a cost: a tall band gives its BLA table a large `dc_max`, which
  shortens the linear skips (slower per pixel). So big-band tuning trades build
  savings for lookup slowdown — a compromise, not the fix.

The fix — **share the orbit, keep per-strip BLA tables**: build the expensive HP
orbit once, reuse it across strips (kills the rebuilds), but rebuild the CHEAP
f64 BLA table per strip so each strip's `dc_max` stays small (keeps skips long).
Few builds AND fast lookups AND all-core scaling — what big bands only half-do.
- Correctness: one orbit serves the whole image because at deep zoom the entire
  image spans ~1e-(digits) of the plane, so every pixel is an infinitesimal `dc`
  from the reference; the rebasing engine (`mandelbrot_perturb64`) makes a single
  reference glitch-free. The "BLA one-reference" engine already validates this
  (matches exact on all but ~0.002% of pixels, <=6 counts off).
- **Server**: SHIPPED via a protocol change (`/mb-computeHP2`): the client sends
  ONE whole-image request (thread count in the body) instead of 32-row jobs, so
  "once per request" = once per image. The orbit is built once, strips
  (`rows/(4*threads)` clamped [4,32]) run on a per-request rayon pool reading it
  in place, and each strip streams back as an NDJSON line when it finishes —
  progressive paint survives, out-of-order, no batch barrier; an aborted fetch
  kills the remaining strips at the next send. The remote worker
  (mandelbrot-worker-remote.js) streams via fetch, dedupes strips on retry, and
  falls back to the legacy endpoint on 404 (old servers). Validated vs the old
  endpoint and vs the wasm shared engine (u64 vs u32 limb orbits round to
  identical f64s): <=6/25600 px, max delta <=4. Orbits are cached ACROSS requests
  (byte-budgeted Arc'd LRU keyed by basis coords, `--orbit-cache`, default 128MB):
  the client sends the pass-1 basis + (ox, oy) offsets with every HP job on both
  tiers, so the server's pass 2, the deferred second pass, and re-renders of the
  same view all skip the build (measured: first strip 384ms -> 61ms on a hit).
  The 404 fallback to the legacy endpoint shifts the basis coords by (ox, oy) in
  16-bit digit arithmetic to recover the pass's own sampling grid.
- **Browser**: SHIPPED on this branch (wasm v9). Web workers have isolated
  memory, so worker 0 builds the orbit and the main thread relays the ~16MB f64
  buffer to the others (~3ms/worker; the ~40MB BLA table is NOT moved — rebuilt
  per strip). SAB-free, works on GitHub Pages. `USE_ORBIT_SHARING` +
  `USE_ORBIT_BROADCAST` in MB.html (both default true; broadcast=false is the
  per-worker-build variant, which measured 12-35% slower — N simultaneous
  million-iteration builds throttle each other). The engine is
  `bla_strip`/`compute_strip_with_orbit` with per-strip dc_max; jobs carry the
  pass-1 BASIS grid coords + per-job (ox, oy) pixel offsets, so BOTH passes share
  one orbit (pass 2 = offsets (-0.5, +0.5); validated to max-delta-1 vs the
  shipping engine on the shifted grid). Adaptive strips under sharing:
  rows/(4*workers) clamped [8,32]. Results (270-digit view): 640x480 230 -> 292
  rows/sec; ties the 640x2048 no-share record (466). Known cost: a far reference
  shortens BLA skips (~10% on a 2048-row span, isolated in bmarks.txt); known
  ceiling: per-worker BLA tables still contend for memory bandwidth at high
  worker counts — a shared read-only table needs SharedArrayBuffer (COOP/COEP
  headers Pages can't set), deferred.

## x86 Evaluation Playbook (BLA branch)

All perf numbers above are from an aarch64 big.LITTLE dev machine
(1x Cortex-X925 + 3x X4 + 4x A720 — heterogeneity repeatedly produced
misleading unpinned benchmarks). On an x86 box, measure in this order:

1. `cd mb-arith && cargo run --release --example bench-real` (and
   `bench-real35`) — native engine comparison on the two saved views:
   A = rebasing, B = glitch 4-lane, E/F = BLA per-strip / one-reference.
   Expect BLA (E) to dominate; homogeneous cores should give stable numbers
   without `taskset`.
2. `cargo run --release --example bench-perturb` — single-pixel vs 2-lane vs
   4-lane scalar kernels (the NEON section auto-skips on x86). Answers how much
   lane ILP x86 extracts; on ARM this ranged 1.2x (A720) to 2.2x (X925).
3. **The AVX2 question**: rerun 1-2 with `RUSTFLAGS="-C target-cpu=native"`
   (default x86-64 assumes only SSE2). The branchless `[f64; 4]` lane kernel in
   `perturb_lanes_shared::<4>` is autovectorizer-friendly — if the glitch
   engine (B) jumps, that's free 256-bit SIMD for an x86 server build (would
   need the flag added to the build to actually ship). BLA (E) is a scalar
   latency chain and should move little.
4. `node mb-wasm/bench-wasm.js` — wasm-vs-native ratio under x86 V8. On ARM,
   wasm runs ~83-86% of native for every engine; wasm SIMD is capped at 128-bit
   regardless of host, so the gap may widen on x86 wherever native got AVX2.
5. Browser + server tests as usual (rows/sec readout, saved views in
   bmarks.txt). Note whether the chip has SMT: "8 workers" vs physical core
   count is a variable the ARM machine didn't have; also Chrome caps 6
   concurrent connections per host for the remote (server) engine.

Known ARM-derived conclusions to RE-TEST rather than assume on x86: explicit
SIMD kernels lost to scalar ILP (AVX2's cheap shuffles may flip this for a
within-pixel complex-mul kernel); the 4-lane width choice; BLA winning at all
depths in wasm (V8 x86 codegen may differ). The |d|^2-carry rule in
perturb_point_bla is architectural, not ARM-specific — keep it.
