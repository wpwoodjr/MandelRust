/*
    Mandelbrot calculations in wasm
    By Bill Wood, Jan/Feb 2023
*/

use mb_arith::*;

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
type UInt = u32;
const ZERO: UInt = 0;
const T_LOW_BITS: UInt = 0xFFFF;
const T_8_TEST: UInt = (T_LOW_BITS >> 3) << 3;
// it's called the "what test" because I haven't figured out what it does :)
const T_8_WHAT_TEST: UInt = (T_LOW_BITS >> 4) << 4;

#[no_mangle]
pub extern "C" fn count_iterations_hp(x: *const u32, y: *const u32, max_iterations: i32, len: u32, mem_offset: *mut u32) -> i32 {

    let len = len as usize;
    let x = unsafe { std::slice::from_raw_parts(x, len - 1) };
    let y = unsafe { std::slice::from_raw_parts(y, len - 1) };
 
    let chunks = len - 1;
    let zx = unsafe { std::slice::from_raw_parts_mut(mem_offset, chunks) };
    let zy = unsafe { std::slice::from_raw_parts_mut(mem_offset.add(chunks), chunks) };
    let work1 = unsafe { std::slice::from_raw_parts_mut(mem_offset.add(2*chunks), chunks) };
    let work2 = unsafe { std::slice::from_raw_parts_mut(mem_offset.add(3*chunks), chunks) };
    let work3 = unsafe { std::slice::from_raw_parts_mut(mem_offset.add(4*chunks), chunks) };
    let work4 = unsafe { std::slice::from_raw_parts_mut(mem_offset.add(5*chunks), chunks) };

    let mut count = 0;
    zx.copy_from_slice(x);
    zy.copy_from_slice(y);

    // while count < max_iterations && zx*zx + zy*zy < 8.0 {
    while count < max_iterations {
        sq(zx, work3, work1);
        sq(zy, work3, work2);
        add(work1, work2, work3);
        if (work3[0] & T_8_TEST) != ZERO && (work3[0] & T_8_TEST) != T_8_WHAT_TEST {
            return count;
        }

        // let new_zx = zx*zx - zy*zy + x;
        negate(work2, work3);
        add(work1, work3, work2);
        add(work2, x, work1);

        // zy = 2.0*zx*zy + y;
        add(zx, zx, work2);
        // zx = new_zx;
        zx.copy_from_slice(work1);
        multiply(work2, zy, work1, work3, work4);
        add(work4, y, zy);

        count += 1;
    }
    -1
}
