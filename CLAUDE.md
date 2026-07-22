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
  -v, --verbose     # Log one line per HP request: grid, basis, offsets, orbit
                    # cache hit/miss, cache occupancy (diagnostic for "why isn't
                    # the cache hitting" -- e.g. a stale client sending no basis)
  --orbit-cache N   # Reference-orbit cache budget in MB (default 128, 0 = off).
                    # Keyed by view coords; pass 2 / re-renders skip the orbit
                    # build. Real bound: max(N, largest single orbit)
  --orbit-points N  # Reference-orbit point budget in MILLIONS. Default is
                    # RAM-aware: RAM/4 as orbit bytes (16 B/pt), clamped
                    # [4M, 256M] pts; containers sized by their cgroup limit.
                    # maxIterations above the budget renders the pixels that
                    # outlive the orbit white (unresolved) instead of wrong;
                    # with reference selection, only views with NO escaper
                    # inside the budget ever build all of it
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
  TWO-TIER ADAPTIVE EPS (wasm v10, server): LOW-LYAPUNOV views (reference
  |2Z| ~= 1, e.g. 40-digits-slow.xml at lambda ~5e-4/iter) starve BLA -- |d|
  outgrows the strict ceiling at ~iter 8k of 58k and the rest single-steps, 83%
  of that view's work, under ANY reference (reference choice was tested and
  refuted first, see bmarks). Fix: each strip probes 8 pixels; a probe taking
  >BLA_RELAX_RUN (1024) consecutive exact steps flips the strip to
  BLA_EPS_RELAXED (2^-16) radii, built lazily per table. Measured single-thread
  wasm (Beast): 40-digits-slow 4.1x, 54-digits 2.4x, 360-boundary 1.7x -- count
  changes are confined to the already-ill-conditioned boundary speckle (adjacent-pixel
  count spacing ~1/(320*lambda); image-validated, structure intact); strips
  whose probes never starve (126/270-digit) are BIT-IDENTICAL and at speed
  parity. Lambda alone cannot pick the tier (270-digit has LOWER lambda but
  dc ~1e-270 never reaches the ceiling) -- hence probes, not a formula.
  FLOATEXP (wasm v11, server): pixel scales below ~2^-1000 (which f64 cannot
  represent -- dc underflows) run a FloatExp head phase (`floatexp.rs`: f64
  mantissa + i64 exponent; `bla_drive_fe`): the BLA loop on FloatExp deltas
  until |d| climbs past ~2^-800, then a mid-pixel BlaState handoff to the
  UNTOUCHED f64 engine carrying the f64-converted dc. The handoff happens ONLY
  when dc converts to a NORMAL f64 (`fe_handoff_ok`); otherwise the pixel runs
  floatexp end to end. DO NOT drop dc at handoff, and do not assume |d| only
  grows: that shipped first and is UNSOUND in near-neutral (low-lambda)
  regions -- lambda is an average, |2Z| < 1 stretches meander |d| back DOWN,
  it can retrace the ~190 doublings to dc scale, and the missing dc showed as
  a systematic ~25-55-count bias (palette-band crawl exactly at the fe/f64
  cutover, found via a forced-cutover A/B; with dc carried, forced-fe vs f64
  is unbiased and brute checks went from |delta| <= 80 to EXACT). While |d| is
  fe-tiny every skip validates, so the head phase mega-skips; deep views cost
  ~32 rows/s single-thread wasm on the handoff path and ~13-15 rows/s on the
  full-fe path (2.5M-iter 360-digit views). Dispatch is by dx exponent
  (`bla_strip_fe`, cutover 2^-1000): views above it run the identical f64
  path -- validated bit-identical (126/270/40-slow) at speed parity. The scale
  plumbing (perturb_setup_fe*, server orbit cache, wasm meta = 6
  mantissa/exponent entries, (ox,oy) folded inside wasm) carries FloatExp end
  to end because a plain-f64 FFI would underflow the values in transit.
  Brute-validated EXACT at spot checks on 2.5M-count pixels at 2^-1002,
  2^-1075, and 2^-1081 pixel scales, and <1% mismatch at 2^-1280 in
  `fe_deep_view_matches_brute_*`.
  ORBIT DIP SIDE TABLE (wasm v12, server): at a deep minibrot nucleus the
  reference passes below f64's ~1e-308 floor every period, and the STORED
  orbit points go subnormal/zero -- at fe pixel scales the dropped 2*Z*d term
  at such a dip can be the LARGEST term in the recurrence, so interior
  minibrot pixels falsely escaped with the reference's count (non-black fuzzy
  minibrot at a 1077-digit KF location, mb-rust-server/1077-digit-minibrot.xml
  = the acceptance test; deltas were wrong by hundreds of orders). The orbit
  builder records the true FloatExp value of every degraded point (OrbitDip;
  ~a dozen entries) and the fe engine consults the table in exact steps and
  the escape/rebase check (fe_orbit_at). BLA skips never need it: dip-spanning
  blocks have validity radius 0. Benign at f64 depths (the window where the
  dropped term dominates is empty when dc > 1e-308) -- shallow views
  bit-identical. Fixed px vs brute: interior -1, outliving 1,790,922, edges
  exact; 36 interior px/row where there were 0.
  PERF NOTE: |d|^2 is carried across loop iterations, NOT recomputed — a
  redundant multiply-add in the serial dependency chain cost wasm ~2.7x (native
  OoO hid it, V8 didn't) and native ~1.2x. Same law, second sighting: ANY
  addition to the hot loop is suspect under V8 — a per-iteration starvation
  counter cost 6-21%, and letting the relaxed loop copy inline beside the
  strict one cost the same again (I-cache); hence detection lives in cold probe
  pixels and each tier is its own monomorphized detection-free loop.
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

Measured on the Beast (800x600, 126-digit view, rows/sec, see bmarks.txt):
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
1. Merge `BLA-orbit-sharing` -> `BLA` -> `perturbation` -> `master` once
   soaked (plus the `deep-iterations` branch on top).
2. Revisit within-pixel SIMD only on x86 hardware (AVX2 shuffles are cheaper —
   measure, don't assume).

SHIPPED from this list (wasm v13, `deep-iterations` branch): **deep iteration
counts** — the 4M cap is gone. reference_orbit takes a runtime point budget
(16 B/pt; server --orbit-points in millions, browser = a 4 GB RAM allowance /
workerCount clamped to a 128M-pt wasm32 ceiling (Rust caps a single
allocation at isize::MAX = 2 GB, so the orbit Vec tops out at ~134M pts;
the original 150M constant was never allocatable) — fewer workers render
deeper); pixels that outlive a budget-truncated non-escaped reference return
count -2, rendered WHITE (distinct from interior black), never wrong (the
flat-blob unsoundness is structurally impossible now),
while rebasing still resolves counts far past the budget where close
approaches allow (measured: the 51-digit minibrot view at maxIter 16M renders
IDENTICALLY with an 8M budget; needs-750M-iters.xml at a 128M budget resolves
counts to 749.8M with 17.8% unresolved, full 750M budget = 7% black). The BLA
table keeps full level resolution over a 4M-point prefix (bit-identical to
the old whole-orbit tables within it) and only 64+-point skips beyond
(~1.25 B/pt); the scan's start level is a TAIL const generic because even a
branchless runtime check cost ~2-3.5% under V8 (hot-loop law, third
sighting). Big orbits (> 1M pts; was 8M until measured A/Bs on the
51-digit view, 32 threads, pixel-identical: 4M orbit 3.9 s -> 0.7 s warm,
1M orbit 0.4 s -> 0.1 s -- concurrent per-strip table builds contend for
the memory bus; sub-1M stays per-strip, unmeasured) share ONE
whole-image table across a
server request's threads — per-strip tables multiply by thread count and
OOM-killed a 32-thread 128M-orbit run at ~475 MB each. UI maxIterations cap
is now 1e9 (i32 counts wall at ~2.1e9 is the next ceiling). first-render
latency at extreme budgets is the orbit build (~30 s at 128M native, ~112 s
at 750M; ~17.5 us/pt at 2600 digits — cost/pt goes with limbs^2).

SHIPPED on `deep-iterations` (server): **escalating reference selection +
orbit-build cancellation**. A center reference that escapes (or reaches
maxIterations) within the 4M-pt cap builds exactly as before — zero overhead
on the common case. A cap-TRUNCATED center now probes 16 full-width rows in
f64 perturbation against the truncated prefix (probe counts that resolve
within the prefix are EXACT: escapes and rebases are sound, only the
end-of-orbit wrap is not and it returns negative) and relocates the reference
to the SHORTEST escaping pixel >= 1M pts (fallback: longest >= 100k).
Shortest-wins is MEASURED, not assumed: bench-refsel (mb-arith example; run
it on any view's HP2 digit arrays) timed identical strips on the 2600-digit
view against 4.6M/4.8M/15.6M refs — per-wrap render cost is PROPORTIONAL to
ref length (106/111/372 us), so wraps are free, render is count-determined
(2.14 vs 2.23 ms/px), and a shorter ref is a pure build-time win (74 s vs
249 s). The 1M floor guards the untested regime where per-wrap fixed costs
must surface. Escaped refs wrap soundly at ANY length — the reverted
reference-selection experiment is the proof — so a minibrot-under-the-
crosshair view costs ~2 prefix builds instead of a maxIterations-long one
(measured, 1077-digit interior-centered view at maxIter 8M: 8M-pt center
build -> 4M prefix + 1.2M relocated build; interior px IDENTICAL, 0.032% of
px off <= 44 counts = cross-reference BLA speckle, which grows mildly as the
ref shortens; mb-arith::relocated_reference_matches_center covers the engine
fact). No
escaper: the cap doubles, with each step capped at ~4x the measured
probe-round time (floor 8 s) of MEASURED build time -- the step is sized in
seconds, not points, so it self-scales with depth (~64M/round at 116 digits,
~500k at 2600; a fixed point step is 10 s at one depth and 17 min at the
other) and bounds center overshoot past the frame's shallowest escaper at
one time-bounded step. The TRIGGER (first probe point) is time-scaled the
same way: a 256k measuring chunk learns the build rate, then the first
probe fires at min(4M, max(1.5M, 20 s of build)) -- deepish probes at 1.5M
(24 s in) instead of 4M; cheap-build depths evaluate to 4M exactly, and
escaping centers never probe at all, so fast views are untouched by
construction. The center build
EXTENDS -- OrbitBuilder64/32
keeps the full-precision z limbs alive between rounds, so no prefix is ever
recomputed (bit-for-bit vs one-shot, mb-arith::resumed_build_matches_one_shot;
the one-shot fn is now a thin wrapper over the builder, hot loop unmoved at
16.9 s/1M pts on the 2600-digit view) -- up to the budget
(whole-frame-deep KF views like the 2600-digit location have NO pixel under
4M, a fixed trigger would never rescue them); an escaping
center short-circuits any round (1b-view: escapes at 5.6M in round 2, full
render, 0 unresolved); nothing at the budget = the old truncated-center
behavior bit for bit. The relocated candidate build is one-shot (its length
is known from the probe); only the center ladder resumes. CANCELLATION: reference_orbit takes an optional
per-65536-pt control hook (OrbitBuildCtl; cold outer-batch check, hot loop
untouched). The server hook must STREAM A HEARTBEAT newline per batch
(clients skip empty NDJSON lines): actix only notices a dead client on
WRITE, so without it is_closed() stays false forever and an abandoned deep
build pegs a core for hours (measured both ways: 100% CPU forever before,
exit within ~3 s after). The hook also logs verbose build progress every
10M pts. BROWSER TIER (wasm v14): the same ladder runs in the worker --
reference_builder_* FFI (resumable OrbitBuilder32 behind thread_local state)
driven by ladder logic in mandelbrot-worker-local-wasm.js (TIMING lives in
JS: wasm32 has no clock); probes reuse compute_strip_with_orbit against the
partial orbit (max_iterations = prefix points), and finish() emits
build_reference_orbit's exact output shape so broadcast + strips are
untouched (the broadcast already carried rowRef/meta). The candidate build
targets its PROBED COUNT + 1M slack, never the budget (the server got the
same cap for hygiene -- identical output, no 9.6 GB virtual reservation),
and the CENTER pre-reserves min(maxIter, budget, 128M) ONCE at start: the
ladder's incremental extends otherwise realloc the orbit Vec with a 2x
transient which, plus per-round probe-table churn, ratcheted a 100M-budget
ladder's heap until memory.grow was denied (user repro: 2-worker
750M-iters trap; node repro of the same ladder now finishes clean in 32 s).
Browser testing surfaced two LATENT pre-v14 bugs the ladder exercised:
the 150M worker ceiling exceeded Rust's 2 GB single-allocation cap (now
128M everywhere), and wasm pointers cross the FFI as SIGNED i32 -- above
the 2 GB heap line they arrive negative and break JS view offsets, so every
pointer source in the worker now masks with >>> 0 (this plausibly explains
some historical big-heap "out of memory" fatals). The broadcast buildOrbit
path also gained the try/catch the task path had -- a trap there was
swallowed by the promise and hung the page. Validated (node, deployed
wasm, deepish): identical ladder trace and relocation target as the server
(px (0,0) escapes 4,619,098; trigger 1.5M, ~370k rungs at 22.5 us/pt), and
strip counts vs the server grid show only the known cross-tier u32/u64
speckle -- calm rows 0.039% <= 5 counts; near-interior chaotic px can shift
by a fraction of the local 100M+ count gradient (0.6% of the worst rows),
0 interior flips, 0 white. Cancellation stays worker-termination.

SHIPPED from this list (wasm v11): **floatexp deltas** — perturbation past
f64's ~1e-308 pixel-scale floor (see the engine section above for mechanism
and numbers). The old wall: full precision ended at 2.2e-308 pixel scale
(~308 digits), subnormal degradation to the 4.9e-324 quantum, and
mb-rust-server/360-digits-boundary.xml sat AT the floor (pixel step below one
quantum -> rendered at ~1.9x wrong scale, could not zoom deeper). That view is
now the shipped acceptance test: renders at TRUE scale, brute-verified, and
zooming past it works (validated to 64x deeper / dx = 2^-1081; nothing
depth-specific remains -- the client's digit pipeline is uncapped and the fe
engine's exponents are i64). floatexp is also the prerequisite the GPU path
was waiting on (f32 + exponent rescaling needs the same machinery).

SHIPPED from this list (wasm v10): **two-tier adaptive BLA_EPS** for
low-Lyapunov views — the fix for `mb-rust-server/40-digits-slow.xml` (see the
engine section above for the mechanism and numbers). Two hard-won findings
from its investigation, both measured (details in bmarks.txt, 16 Jul 2026):
- Dead references HEAL: reference SELECTION (probe + relocate when the center
  escapes early) was fully implemented first, produced NO speedup anywhere
  (the Zhuoran rebase re-engages skips after a wrap; a dead center ref even
  BEAT a full-coverage selected ref — shorter orbit, nearer reference), and
  was reverted (diff: reference-selection.patch, 2026-07 session scratchpad).
  "Pan so the center sits on high-count structure" is a placebo. The coverage
  LAW still holds for item 1: a CAP-truncated non-escaped ref is unsound;
  escaped refs wrap soundly and cheaply.
- Eps tolerance is scale-free (pixel spacing and deltas shrink together, so
  the sub-pixel criterion is dx/|dc| ~ 1/320 at every depth) and lambda alone
  cannot pick the tier (270-digit has lower lambda than 40-digits-slow yet
  never starves — its dc ~1e-270 keeps |d| under the ceiling for a pixel's
  whole life). Starvation must be MEASURED, hence probe pixels. Validate any
  eps change by image comparison, not exact counts.

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
  headers Pages can't set), deferred. WORKER-COUNT GUIDANCE (measured): on
  many-thread x86, deep HP views peak below hardwareConcurrency — Beast: 8w 312
  beats 32w 294 on the 270-digit view (bigger strips → fewer per-strip BLA
  tables, less bandwidth churn). On the 8-core Chromebook, clean 8w wins (a
  session suggesting otherwise was contaminated by a busy Crostini VM — on
  big.LITTLE, quiesce background VMs and trust repeated runs, not single ones).
  An early "8GB machines page on 4M-maxIter views (halve workers)" finding was
  RETRACTED for the CB 17 Jul 2026: it actually has 16GB (~3GB of tables can't
  page that), and the max-memory configuration (4M-pt orbit, ~305 MB table per
  worker strip) later ran a clean 131 rows/s there at 8w -- the 1.25 rows/s
  measurement was a transient. The concern still stands for true 8GB devices
  (phones/tablets: fewer GB, tighter mobile wasm heaps), where the prefix-table
  item keeps its memory-relief value. Halve workers only if a slowdown shows.

## x86 Evaluation Playbook (BLA branch)

Machine attribution: the perturbation engines and the initial BLA
implementation were developed on an aarch64 big.LITTLE machine (1x Cortex-X925
+ 3x X4 + 4x A720 — heterogeneity repeatedly produced misleading unpinned
benchmarks); that machine supplied the NEON-kernel and lane-width findings
below. Development moved to the x86 Beast (i9-13900KF under WSL2) at the start
of the orbit-rebuild investigation and has stayed there (orbit sharing, HP2,
adaptive eps). "CB" numbers are the 8-core ARM Chromebook (MediaTek Kompanio),
used for cross-arch validation throughout. Items worth re-measuring natively
on x86:

1. `cd mb-arith && cargo run --release --example bench-real` (and
   `bench-real35`) — native engine comparison on the two saved views:
   A = rebasing, B = glitch 4-lane, E/F = BLA per-strip / one-reference.
   Expect BLA (E) to dominate; homogeneous cores should give stable numbers
   without `taskset`.
2. `cargo run --release --example bench-perturb` — single-pixel vs 2-lane vs
   4-lane scalar kernels (the NEON section auto-skips on x86). Answers how much
   lane ILP x86 extracts; on ARM this ranged 1.2x (A720) to 2.2x (X925).
3. **The AVX2 question — RESOLVED (21 Jul 2026, bench-strip-ab)**:
   `target-cpu=native` changes NOTHING. The real finding: LLVM's SLP
   vectorizer packs the latency-bound f64 BLA delta chain into
   shuffle-laden <2 x double> (unpckhpd dances lengthen the serial critical
   path -- the same pathology this project measured for hand-written SIMD
   kernels, inflicted automatically), making V8's plain-scalar wasm codegen
   BEAT native by 1.57x on identical strips (3.18 s vs 2.02 s). Root-caused
   by disassembly diff, fixed by `.cargo/config.toml` in mb-rust-server and
   mb-arith: `-C llvm-args=-slp-threshold=999999`. With SLP off: f64 strips
   1.71x faster (native 1.87 s, beating wasm's 2.02), fe path unchanged,
   orbit builds unchanged, outputs identical; server-level 2-thread
   750M-iters run 112-118 s -> 62.6 s (1.83x). mb-wasm has NO such config:
   LLVM's wasm backend leaves the loop scalar on its own. Builds stay
   native-dominated (0.119 vs 0.37 us/pt). Fourth hot-loop-law sighting,
   first compiler-inflicted one: any addition to the serial chain is
   suspect, including additions made by the optimizer.
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
