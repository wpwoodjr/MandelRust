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
use core::mem::size_of;
type UInt = u64;

#[no_mangle]
pub extern "C" fn compute_mandelbrot_hp(xmin: *const u32, len: u32, dx: *const u32, columns: u32, y: *const u32, max_iterations: i32, iteration_counts: *mut i32) {

    let len = len as usize;
    let xmin = unsafe { std::slice::from_raw_parts(xmin, len) };
    let dx = unsafe { std::slice::from_raw_parts(dx, len) };
    let y = unsafe { std::slice::from_raw_parts(y, len) };
    let columns = columns as usize;
    let iteration_counts = unsafe { std::slice::from_raw_parts_mut(iteration_counts, columns) };

    // ignore lowest 16 bits for efficiency during Mandelbrot calculation, has no impact on image quality
    // use all bits for incrementing x_val though
    let u32_chunks = len - 1;
    // chunks: 1 for the integral part, plus however many T elements are needed for the fractional part
    let chunks = 1 + {
        let t_to_u32_size_ratio = size_of::<UInt>()/size_of::<u32>();
        (u32_chunks - 1 + t_to_u32_size_ratio - 1)/t_to_u32_size_ratio
    };

    let mut x_val = u32_to_t::<UInt>(xmin);
    let dx = u32_to_t::<UInt>(dx);
    let y = u32_to_t::<UInt>(y);
    let mut hp_data = HPData::new(chunks);

    for i in 0..columns {
        iteration_counts[i] = count_iterations_hp(&mut hp_data, &x_val[0..chunks], &y[0..chunks], max_iterations);
        incr(&mut x_val, &dx);
    }
}
