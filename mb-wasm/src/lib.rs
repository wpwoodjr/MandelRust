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
#[no_mangle]
pub extern "C" fn mb_wasm_version() -> u32 { 8 }


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
// perturb_setup32). Returns a pointer to `2*N` interleaved f64 (zr,zi); writes the
// orbit point count N to *out_len and the f64 pixel steps [dx_f, dy_f, dcx0] to
// out_meta[0..3] so the caller can hand them to compute_strip_with_orbit without
// reconverting limbs. The buffer is 8-aligned; free it with free_f64(ptr, 2*N).
#[no_mangle]
pub extern "C" fn build_reference_orbit(
    xmin: *const u32, dx: *const u32, ymax: *const u32, dy: *const u32, len: u32,
    columns: u32, image_rows: u32, max_iterations: i32,
    out_len: *mut u32, out_meta: *mut f64,
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

    let (orbit, dx_f, dy_f, dcx0, _col_ref, _row_ref) = perturb_setup32(
        &xmin, &dx, &ymax, &dy, chunks, image_rows as usize, columns as usize, max_iterations,
    );

    // into_boxed_slice shrinks capacity to len, so free_f64(ptr, 2*N) matches the
    // allocation exactly (N * (f64,f64) == 2N * f64, align 8).
    let boxed: Box<[(f64, f64)]> = orbit.into_boxed_slice();
    let n = boxed.len();
    let ptr = boxed.as_ptr() as *mut f64;
    std::mem::forget(boxed); // ownership passes to JS; freed via free_f64
    unsafe {
        *out_len = n as u32;
        *out_meta.add(0) = dx_f;
        *out_meta.add(1) = dy_f;
        *out_meta.add(2) = dcx0;
    }
    ptr
}

// Grind image rows [strip_row0, strip_row0+strip_rows) against a shared orbit
// (2*orbit_len interleaved f64, as returned/broadcast from build_reference_orbit).
// dx_f/dy_f/dcx0 come from out_meta; image_row_ref = image_rows/2 fixes the
// reference the orbit was built at. Writes strip_rows*columns i32 to out.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn compute_strip_with_orbit(
    orbit_ptr: *const f64, orbit_len: u32,
    dx_f: f64, dy_f: f64, dcx0: f64, image_row_ref: u32,
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
    bla_strip(
        orbit, dx_f, dy_f, dcx0, image_row_ref as usize,
        strip_row0 as usize, strip_rows, columns, max_iterations, out,
    );
}
