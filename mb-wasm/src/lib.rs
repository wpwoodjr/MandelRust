/*
    Mandelbrot calculations in wasm
    By Bill Wood, Jan/Feb 2023
*/

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}


// *** alloc/dealloc for JavaScript *** //
// https://radu-matei.com/blog/practical-guide-to-wasm-memory/
use std::alloc::{alloc, dealloc, Layout};

#[no_mangle]
pub extern "C" fn malloc(size: u32) -> *mut u8 {
    let align = std::mem::align_of::<usize>();
    unsafe {
        let layout = Layout::from_size_align_unchecked(size as usize, align);
        alloc(layout)
    }
}

#[no_mangle]
pub extern "C"  fn dalloc(ptr: *mut u8, size: u32) {
    let align = std::mem::align_of::<usize>();
    unsafe {
        let layout = Layout::from_size_align_unchecked(size as usize, align);
        dealloc(ptr, layout);
    }
}

// Bumped on each build so the client can confirm which binary is actually loaded
// (an absent export = a stale cached build predating this marker). 3 = shared-index
// + glitch engine, scalar 2-lane kernel (SIMD dropped: measured slower than ILP).
// 4 = scalar 4-lane kernel (more ILP on wide cores, no downside on narrow ones).
// 5 = BLA (rebasing + composed-skip table): skips most iterations at deep zoom.
// 6 = depth hybrid: BLA at >= 16 u32 digits, glitch 4-lane below (wasm JITs ran
//     the then-current BLA loop poorly; superseded).
// 7 = BLA at all depths (loop restructure fixed the wasm JIT penalty).
// 8 = orbit-sharing FFI (build_reference_orbit + compute_strip_with_orbit +
//     malloc_f64/free_f64); classic compute_mandelbrot_hp_perturb still present.
// 9 = compute_strip_with_orbit takes dcy_off so one orbit serves both passes
//     (the second pass's grid is half-pixel-shifted; the reference is not a grid
//     point, so pass-2 samples are just different dc offsets).
// 10 = two-tier BLA radii for low-Lyapunov views (40-digits-slow.xml et al):
//      probe pixels detect starvation (> BLA_RELAX_RUN consecutive exact
//      steps) and flip their strip to BLA_EPS_RELAXED radii. Strips whose
//      probes never starve are bit-identical to v9.
// 11 = floatexp: pixel scales below f64's ~1e-308 floor render via a floatexp
//      (f64 mantissa + i64 exponent) head phase. out_meta grows to 6 entries
//      of mantissa/exponent pairs, and compute_strip_with_orbit takes those
//      pairs plus raw (ox, oy) -- the offset folding that JS used to do in f64
//      now happens inside wasm at full range. Views above the floor are
//      bit-identical to v10.
// 12 = orbit dip side table: reference-orbit points that pass below f64's
//      floor (deep minibrot nuclei) carry their true FloatExp values in a
//      small side buffer ([index, zr_m, zr_e, zi_m, zi_e] per entry), built by
//      build_reference_orbit and consumed by compute_strip_with_orbit --
//      without it, interior minibrot pixels at fe depths falsely escape
//      (non-black, fuzzy minibrots).
// 13 = deep iterations: build_reference_orbit takes an orbit point budget
//      (16 B/point; the client sizes it to worker count and RAM). Pixels that
//      outlive a budget-truncated reference return count -2 (rendered white,
//      distinct from interior black); the BLA table keeps
//      full resolution over a 4M-point prefix and sparse 64+ skips beyond.
//      Orbits over 2M points cache ONE whole-image BLA table per worker per
//      view (per-strip table churn ratcheted the wasm heap until memory.grow
//      failed mid-render on 4M-orbit views at high worker counts).
// 14 = reference selection: a resumable orbit builder (reference_builder_*)
//      lets the worker run the server's escalating-selection ladder -- build
//      the center to a cap, probe rows against the truncated prefix via
//      compute_strip_with_orbit, relocate to the shortest escaping pixel
//      >= 1M pts (escaped refs wrap soundly at any length; per-wrap render
//      cost is proportional to ref length, so shortest wins -- see
//      bench-refsel). Ladder TIMING lives in worker JS (wasm32 has no
//      clock); finish() emits build_reference_orbit's exact output shape so
//      broadcast and strips are unchanged.
#[no_mangle]
pub extern "C" fn mb_wasm_version() -> u32 { 14 }


use mb_arith::*;

// *** low precision *** //
#[no_mangle]
pub extern "C" fn compute_mandelbrot(xmin: f64, dx: f64, columns: u32, y: f64, max_iterations: i32, iteration_counts: *mut i32) {

    let columns = columns as usize;
    let iteration_counts = unsafe { std::slice::from_raw_parts_mut(iteration_counts, columns) };
    for i in 0..columns {
        iteration_counts[i] = count_iterations(xmin + (i as f64*dx), y, max_iterations);
    }
}


// *** high precision *** //
#[no_mangle]
pub extern "C" fn compute_mandelbrot_hp(xmin: *const u32, len: u32, dx: *const u32, columns: u32, y: *const u32, max_iterations: i32, iteration_counts: *mut i32) {

    let len = len as usize;
    let xmin = unsafe { std::slice::from_raw_parts(xmin, len) };
    let dx = unsafe { std::slice::from_raw_parts(dx, len) };
    let y = unsafe { std::slice::from_raw_parts(y, len) };
    let columns = columns as usize;
    let iteration_counts = unsafe { std::slice::from_raw_parts_mut(iteration_counts, columns) };

    // use the full coordinate precision the client sent
    let u32_chunks = len;
    // chunks: 1 for the integral part, plus however many u32 limbs are needed for the fractional part.
    // the u32 limb engine is used because wasm32 has no 64x64 -> 128 bit multiply;
    // 32x32 -> 64 is a single native i64.mul
    let chunks = 1 + (u32_chunks - 1 + 1)/2;

    let x_val = u32_to_limbs32(xmin);
    let dx = u32_to_limbs32(dx);
    let y = u32_to_limbs32(y);
    mandelbrot_row_hp32(&x_val, &dx, &y, chunks, columns, max_iterations, iteration_counts);
}


// *** high precision via perturbation *** //
// Computes a whole strip of `rows` rows x `columns` columns in one call: a single
// full-precision reference orbit (u32 limb engine) at the strip center, then a
// cheap f64 delta orbit per pixel. Much faster at deep zoom; the only HP work is
// the one reference, and the per-pixel loop is native-speed f64.
//
// `iteration_counts` must have room for rows*columns i32 (row-major). Rows advance
// y downward by dy (y_i = ymax - i*dy), matching compute_mandelbrot_hp.
#[no_mangle]
pub extern "C" fn compute_mandelbrot_hp_perturb(
    xmin: *const u32, len: u32, dx: *const u32, columns: u32,
    ymax: *const u32, dy: *const u32, rows: u32,
    max_iterations: i32, iteration_counts: *mut i32,
) {
    let len = len as usize;
    let xmin = unsafe { std::slice::from_raw_parts(xmin, len) };
    let dx = unsafe { std::slice::from_raw_parts(dx, len) };
    let ymax = unsafe { std::slice::from_raw_parts(ymax, len) };
    let dy = unsafe { std::slice::from_raw_parts(dy, len) };
    let columns = columns as usize;
    let rows = rows as usize;
    let iteration_counts = unsafe { std::slice::from_raw_parts_mut(iteration_counts, rows*columns) };

    // use the full coordinate precision the client sent
    let u32_chunks = len;
    let chunks = 1 + (u32_chunks - 1 + 1)/2;

    let xmin = u32_to_limbs32(xmin);
    let dx = u32_to_limbs32(dx);
    let ymax = u32_to_limbs32(ymax);
    let dy = u32_to_limbs32(dy);

    // BLA engine at all depths, same as the native server. (v6 briefly used a
    // depth hybrid because the BLA loop ran at ~1/3 native speed under V8; the
    // fix was carrying |d|^2 across loop iterations instead of recomputing it --
    // the redundant multiply-add chain stalled V8's scheduling where native
    // out-of-order execution hid it. wasm BLA now runs at ~83% of native, the
    // same ratio as every other engine, and beats the glitch engine everywhere.)
    mandelbrot_perturb_bla32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iterations, iteration_counts);
}

// *** orbit sharing (v8): build the reference orbit ONCE for the whole image,
// then grind strips against it. The orbit is stored as interleaved f64 pairs
// (zr,zi) == Vec<(f64,f64)> layout. On the browser one worker calls
// build_reference_orbit, JS broadcasts the f64 buffer, every worker copies it into
// an 8-aligned buffer (malloc_f64) and calls compute_strip_with_orbit per strip.
// See mb-arith::bla_strip and CLAUDE.md (BLA-orbit-sharing). *** //

// 8-byte-aligned allocation for the orbit buffer. The general malloc uses
// align_of::<usize>() = 4 on wasm32, but f64 access and JS Float64Array views both
// require 8-byte alignment, so the orbit needs its own allocator. Freed by free_f64
// with the matching (size, align).
#[no_mangle]
pub extern "C" fn malloc_f64(count: u32) -> *mut f64 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(count as usize * 8, 8);
        alloc(layout) as *mut f64
    }
}

#[no_mangle]
pub extern "C" fn free_f64(ptr: *mut f64, count: u32) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(count as usize * 8, 8);
        dealloc(ptr as *mut u8, layout);
    }
}

// Build the whole-image reference orbit (reference at image center, matching
// perturb_setup_fe32). Returns a pointer to `2*N` interleaved f64 (zr,zi);
// writes the orbit point count N to *out_len and the FloatExp scale meta
// [dx_m, dx_e, dy_m, dy_e, dcx0_m, dcx0_e] to out_meta[0..6] (exponents carried
// as f64 -- exact for any real exponent) so the caller can hand them to
// compute_strip_with_orbit without reconverting limbs. The mantissa/exponent
// split is what lets pixel scales below f64's ~1e-308 floor survive the FFI.
// The buffer is 8-aligned; free it with free_f64(ptr, 2*N).
#[no_mangle]
pub extern "C" fn build_reference_orbit(
    xmin: *const u32, dx: *const u32, ymax: *const u32, dy: *const u32, len: u32,
    columns: u32, image_rows: u32, max_iterations: i32, orbit_budget: i32,
    out_len: *mut u32, out_meta: *mut f64,
    out_dips_ptr: *mut u32, out_dips_len: *mut u32,
) -> *mut f64 {
    let len = len as usize;
    let xmin = unsafe { std::slice::from_raw_parts(xmin, len) };
    let dx = unsafe { std::slice::from_raw_parts(dx, len) };
    let ymax = unsafe { std::slice::from_raw_parts(ymax, len) };
    let dy = unsafe { std::slice::from_raw_parts(dy, len) };
    let chunks = 1 + (len - 1 + 1) / 2;

    let xmin = u32_to_limbs32(xmin);
    let dx = u32_to_limbs32(dx);
    let ymax = u32_to_limbs32(ymax);
    let dy = u32_to_limbs32(dy);

    let (orbit, dips, dx_fe, dy_fe, dcx0, _col_ref, _row_ref) = perturb_setup_fe32(
        &xmin, &dx, &ymax, &dy, chunks, image_rows as usize, columns as usize, max_iterations,
        if orbit_budget > 0 { orbit_budget } else { DEFAULT_ORBIT_BUDGET },
    );

    finish_orbit(orbit, &dips, dx_fe, dy_fe, dcx0, out_len, out_meta, out_dips_ptr, out_dips_len)
}

// dip side table as 5 f64 per entry: [index, zr_m, zr_e, zi_m, zi_e]
fn serialize_dips_into(dips: &[OrbitDip], p: *mut f64) {
    for (i, d) in dips.iter().enumerate() {
        unsafe {
            *p.add(i * 5) = d.index as f64;
            *p.add(i * 5 + 1) = d.zr.m;
            *p.add(i * 5 + 2) = d.zr.e as f64;
            *p.add(i * 5 + 3) = d.zi.m;
            *p.add(i * 5 + 4) = d.zi.e as f64;
        }
    }
}

// shared tail of build_reference_orbit / reference_builder_finish: hand the
// orbit + dips to JS in the v8 buffer convention (see build_reference_orbit)
#[allow(clippy::too_many_arguments)]
fn finish_orbit(
    orbit: Vec<(f64, f64)>, dips: &[OrbitDip],
    dx_fe: FloatExp, dy_fe: FloatExp, dcx0: FloatExp,
    out_len: *mut u32, out_meta: *mut f64,
    out_dips_ptr: *mut u32, out_dips_len: *mut u32,
) -> *mut f64 {
    // Freed by JS with free_f64(ptr, 5*len); len 0 => ptr 0, nothing to free.
    let n_dips = dips.len();
    let dips_ptr = if n_dips == 0 {
        std::ptr::null_mut()
    } else {
        let p = malloc_f64((n_dips * 5) as u32);
        serialize_dips_into(dips, p);
        p
    };

    // into_boxed_slice shrinks capacity to len, so free_f64(ptr, 2*N) matches the
    // allocation exactly (N * (f64,f64) == 2N * f64, align 8).
    let boxed: Box<[(f64, f64)]> = orbit.into_boxed_slice();
    let n = boxed.len();
    let ptr = boxed.as_ptr() as *mut f64;
    std::mem::forget(boxed); // ownership passes to JS; freed via free_f64
    unsafe {
        *out_len = n as u32;
        *out_meta.add(0) = dx_fe.m;
        *out_meta.add(1) = dx_fe.e as f64;
        *out_meta.add(2) = dy_fe.m;
        *out_meta.add(3) = dy_fe.e as f64;
        *out_meta.add(4) = dcx0.m;
        *out_meta.add(5) = dcx0.e as f64;
        *out_dips_ptr = dips_ptr as u32;
        *out_dips_len = n_dips as u32;
    }
    ptr
}

// *** v14: resumable reference builder (worker-side selection ladder) *** //
//
// The worker drives the same escalating ladder the server runs: start the
// builder at the image center, extend it in rounds (TIMED IN JS -- wasm32 has
// no clock, and the ladder's trigger/rungs are sized in measured build time),
// probe rows against the truncated prefix with compute_strip_with_orbit
// (max_iterations = current point count, so probes certify only what the
// prefix can prove; the end-of-orbit wrap returns negative), and on finding
// an escaping pixel start() again there (dropping the center prefix) and
// extend to the budget -- the candidate escapes at its known count.
// finish() emits exactly build_reference_orbit's outputs, so the broadcast
// protocol and every strip path are untouched.
struct RefBuilderState {
    b: OrbitBuilder32,
    dx_fe: FloatExp,
    dy_fe: FloatExp,
    dcx0: FloatExp,
    dips_buf: Vec<f64>, // serialized dips scratch for probe calls
}
thread_local! {
    static REF_BUILDER: std::cell::RefCell<Option<RefBuilderState>> =
        const { std::cell::RefCell::new(None) };
}

/// `capacity_points`: reserve the orbit's FINAL expected size up front (the
/// caller's min(maxIterations, budget), or probed count + slack for a
/// candidate). Incremental extends otherwise realloc the Vec with a 2x
/// transient -- on a ladder that reaches 100M points that transient plus the
/// per-round table churn ratchets the wasm heap until memory.grow is denied
/// (v13's one-shot build never had this: it reserved once).
#[no_mangle]
pub extern "C" fn reference_builder_start(
    xmin: *const u32, dx: *const u32, ymax: *const u32, dy: *const u32, len: u32,
    ref_col: u32, ref_row: u32, capacity_points: i32, out_meta: *mut f64,
) {
    let len = len as usize;
    let xmin = unsafe { std::slice::from_raw_parts(xmin, len) };
    let dx = unsafe { std::slice::from_raw_parts(dx, len) };
    let ymax = unsafe { std::slice::from_raw_parts(ymax, len) };
    let dy = unsafe { std::slice::from_raw_parts(dy, len) };
    let chunks = 1 + (len - 1 + 1) / 2;

    let xmin = u32_to_limbs32(xmin);
    let dx = u32_to_limbs32(dx);
    let ymax = u32_to_limbs32(ymax);
    let dy = u32_to_limbs32(dy);

    let mut b = OrbitBuilder32::at_grid(&xmin, &dx, &ymax, &dy, chunks,
        ref_row as usize, ref_col as usize);
    if capacity_points > 0 {
        // clamp to the REAL wasm32 orbit ceiling: Rust caps any single
        // allocation at isize::MAX (2 GB), so an orbit Vec tops out at
        // ~134M points x 16 B -- 128M leaves headroom. (The old 150M
        // "ceiling" was 2.4 GB and would have trapped v13's one-shot
        // with_capacity identically; it was just never exercised.) A
        // bogus budget-sized request must not trap the whole worker.
        let want = (capacity_points as usize).min(128_000_000) + 1;
        if want > b.orbit.capacity() {
            b.orbit.reserve(want - b.orbit.len());
        }
    }
    let dx_fe = limbs32_to_fe(&dx[..chunks]);
    let dy_fe = limbs32_to_fe(&dy[..chunks]);
    let dcx0 = dx_fe.mul_f64(-(ref_col as f64));
    unsafe {
        *out_meta.add(0) = dx_fe.m;
        *out_meta.add(1) = dx_fe.e as f64;
        *out_meta.add(2) = dy_fe.m;
        *out_meta.add(3) = dy_fe.e as f64;
        *out_meta.add(4) = dcx0.m;
        *out_meta.add(5) = dcx0.e as f64;
    }
    REF_BUILDER.with(|c| {
        *c.borrow_mut() = Some(RefBuilderState { b, dx_fe, dy_fe, dcx0, dips_buf: Vec::new() });
    });
}

/// Extend to `target_points` orbit points; returns the point count reached
/// (orbit entries - 1). Escape ends the build early (see _escaped).
#[no_mangle]
pub extern "C" fn reference_builder_extend(target_points: i32) -> i32 {
    REF_BUILDER.with(|c| {
        let mut s = c.borrow_mut();
        let s = s.as_mut().expect("reference_builder_start not called");
        s.b.extend(target_points, None);
        (s.b.orbit.len() - 1) as i32
    })
}

#[no_mangle]
pub extern "C" fn reference_builder_escaped() -> i32 {
    REF_BUILDER.with(|c| {
        let s = c.borrow();
        orbit_escaped(&s.as_ref().expect("no builder").b.orbit) as i32
    })
}

/// Pointer to the current orbit buffer (interleaved f64 pairs; entries =
/// points + 1). Re-fetch after every extend: the Vec may reallocate.
#[no_mangle]
pub extern "C" fn reference_builder_orbit_ptr() -> *const f64 {
    REF_BUILDER.with(|c| {
        let s = c.borrow();
        s.as_ref().expect("no builder").b.orbit.as_ptr() as *const f64
    })
}

/// Serialize the current dip table (5 f64/entry, same format as
/// build_reference_orbit's); writes the entry count and returns the buffer
/// (owned by the builder -- valid until the next builder call).
#[no_mangle]
pub extern "C" fn reference_builder_dips(out_len: *mut u32) -> *const f64 {
    REF_BUILDER.with(|c| {
        let mut s = c.borrow_mut();
        let s = s.as_mut().expect("no builder");
        let n = s.b.dips.len();
        s.dips_buf.resize(n * 5, 0.0);
        serialize_dips_into(&s.b.dips, s.dips_buf.as_mut_ptr());
        unsafe { *out_len = n as u32 };
        s.dips_buf.as_ptr()
    })
}

/// Consume the builder and hand its orbit to JS in build_reference_orbit's
/// exact output convention (buffer + meta + dips; free with free_f64).
#[no_mangle]
pub extern "C" fn reference_builder_finish(
    out_len: *mut u32, out_meta: *mut f64,
    out_dips_ptr: *mut u32, out_dips_len: *mut u32,
) -> *mut f64 {
    let s = REF_BUILDER.with(|c| c.borrow_mut().take()).expect("no builder");
    finish_orbit(s.b.orbit, &s.b.dips, s.dx_fe, s.dy_fe, s.dcx0,
        out_len, out_meta, out_dips_ptr, out_dips_len)
}

// Grind image rows [strip_row0, strip_row0+strip_rows) against a shared orbit
// (2*orbit_len interleaved f64, as returned/broadcast from build_reference_orbit).
// The scale arguments are the FloatExp mantissa/exponent pairs from out_meta;
// (ox, oy) is this job's sampling-grid offset from the basis grid in pixels
// (pass 2: -0.5, +0.5), folded into dc here -- in FloatExp, since at fe depths
// the f64 fold JS used to do would underflow. image_row_ref = image_rows/2
// fixes the reference row the orbit was built at. Writes strip_rows*columns
// i32 to out.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn compute_strip_with_orbit(
    orbit_ptr: *const f64, orbit_len: u32,
    dips_ptr: *const f64, dips_len: u32,
    dx_m: f64, dx_e: f64, dy_m: f64, dy_e: f64, dcx0_m: f64, dcx0_e: f64,
    ox: f64, oy: f64, image_row_ref: u32,
    strip_row0: u32, strip_rows: u32, columns: u32,
    max_iterations: i32, iteration_counts: *mut i32,
) {
    // The buffer is the byte-image of a [(f64,f64)] slice (built that way, or a
    // JS copy of one), 8-aligned, so this reinterpret is layout-valid.
    let orbit = unsafe {
        std::slice::from_raw_parts(orbit_ptr as *const (f64, f64), orbit_len as usize)
    };
    let strip_rows = strip_rows as usize;
    let columns = columns as usize;
    let out = unsafe { std::slice::from_raw_parts_mut(iteration_counts, strip_rows * columns) };
    let mut dips: Vec<OrbitDip> = Vec::with_capacity(dips_len as usize);
    if dips_len > 0 {
        let raw = unsafe { std::slice::from_raw_parts(dips_ptr, dips_len as usize * 5) };
        for c in raw.chunks_exact(5) {
            dips.push(OrbitDip {
                index: c[0] as u32,
                zr: FloatExp::new(c[1], c[2] as i64),
                zi: FloatExp::new(c[3], c[4] as i64),
            });
        }
    }
    let dx = FloatExp::new(dx_m, dx_e as i64);
    let dy = FloatExp::new(dy_m, dy_e as i64);
    let dcx0 = FloatExp::new(dcx0_m, dcx0_e as i64).add(dx.mul_f64(ox));
    let dcy_off = dy.mul_f64(oy);

    // Big orbits: build the BLA table ONCE per view (whole-image dc_max) and
    // cache it across this worker's strips. Per-strip tables at 160 MB+ are
    // alloc/free churn that RATCHETS the wasm heap (wasm memory never
    // shrinks; fragmentation makes each rebuild land higher) until
    // memory.grow is denied mid-render -- observed as Rust aborts
    // ("unreachable executed") across a 32-worker deep render. One cached
    // table also removes the per-strip build cost. Small orbits keep
    // per-strip tables: tight dc_max (longest skips), tiny churn,
    // bit-identical to previous behavior.
    // Threshold measured on both tiers (22 Jul 2026): at a 1M-pt orbit,
    // per-strip tables cost 0.84 s vs 0.22 s cached for a 60-strip node
    // sequence (3.8x, identical outputs) -- mirroring the server's shared-
    // table A/B (bmarks). Sub-1M orbits keep per-strip tables: ms builds,
    // tightest dc_max, unmeasured but the stakes are sub-0.5 s renders.
    const TABLE_CACHE_MIN_ORBIT: u32 = 1_000_000;
    if orbit_len > TABLE_CACHE_MIN_ORBIT {
        use std::cell::RefCell;
        type TableKey = (usize, u32, u64, i64, u64, i64, u64, i64, u32, u32);
        thread_local! {
            static TABLE_CACHE: RefCell<Option<(TableKey, BlaTable)>> = const { RefCell::new(None) };
        }
        let key: TableKey = (
            orbit_ptr as usize, orbit_len,
            dx.m.to_bits(), dx.e,
            dcx0.m.to_bits(), dcx0.e,
            dcy_off.m.to_bits(), dcy_off.e,
            image_row_ref, columns as u32,
        );
        TABLE_CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            let stale = cache.as_ref().map(|(k, _)| *k != key).unwrap_or(true);
            if stale {
                *cache = None; // drop the old table BEFORE allocating the new one
                let rows_est = image_row_ref as usize * 2 + 1; // >= actual rows
                let table = bla_image_table(
                    orbit, dx, dy, dcx0, dcy_off, image_row_ref as usize, rows_est, columns,
                );
                *cache = Some((key, table));
            }
            let (_, table) = cache.as_ref().unwrap();
            bla_strip_fe_with_table(
                orbit, &dips, table, dx, dy, dcx0, dcy_off, image_row_ref as usize,
                strip_row0 as usize, strip_rows, columns, max_iterations, out,
                None, // no mid-strip abort in workers: termination kills them
            );
        });
        return;
    }
    bla_strip_fe(
        orbit, &dips, dx, dy, dcx0, dcy_off, image_row_ref as usize,
        strip_row0 as usize, strip_rows, columns, max_iterations, out,
    );
}
