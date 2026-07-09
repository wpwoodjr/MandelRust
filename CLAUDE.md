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
  -r, --rayon N     # Rayon threads per request (default: 2)
  --no-perturb      # Disable the perturbation engine (default: ON — one
                    # full-precision reference orbit per band of rows, cheap f64
                    # deltas per pixel via the glitch engine; deep-zoom fast)
  --u32/--u64/--u128 # Legacy half-limb engines for HP calculations; these also
                     # disable perturbation (full-precision default: u64 limbs)
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
- `POST /mb-computeHP` - High precision (arbitrary precision) calculation
- `GET /remoteCanComputeMB` - Health check

**Worker scripts in client/:**
- `mandelbrot-worker-local-js.js` - JavaScript compute worker
- `mandelbrot-worker-local-wasm.js` - WASM compute worker
- `mandelbrot-worker-remote-v2.0.js` - Remote server worker

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
kernel isolation), `bench-strips.rs` (strip-height sweep). For wasm numbers, a
node driver calling the deployed `client/mb-wasm.wasm` predicts browser
single-worker rows/sec within ~1% — write one before trusting any wasm perf
theory (browser pipeline noise is not the wasm engine).

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
3. Smaller fry: share the reference orbit + BLA table across a worker's jobs
   (saves the few % of HP setup per job); revisit within-pixel SIMD only on x86
   hardware (AVX2 shuffles are cheaper — measure, don't assume).
