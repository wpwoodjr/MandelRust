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
// 6 = depth hybrid: BLA at >= 16 u32 digits, glitch 4-lane below (wasm JITs run
//     the branchy BLA loop poorly; the crossover is ~60-70 decimal digits).
#[no_mangle]
pub extern "C" fn mb_wasm_version() -> u32 { 6 }


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

    // Engine choice (wasm-specific): BLA's scalar probe-per-iteration loop runs
    // at only ~1/3 native speed under wasm JITs, while the 4-lane glitch engine
    // runs at ~native speed. BLA still wins decisively at deep zoom (~3.4x at
    // 126 decimal digits, node-measured) but loses at shallow depth (~0.6x at
    // 35 digits), so pick by zoom depth; the crossover is around 60-70 decimal
    // digits = ~16 u32 16-bit digits. The native server uses BLA at all depths.
    if u32_chunks >= 16 {
        mandelbrot_perturb_bla32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iterations, iteration_counts);
    } else {
        mandelbrot_perturb_glitch32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iterations, iteration_counts);
    }
}
