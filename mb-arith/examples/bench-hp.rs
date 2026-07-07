/*
    Benchmark of the high-precision engines on a grid over the full Mandelbrot set:
    legacy generic u128 half-limb (previous server default), the full-width u64
    limb engine (dynamic limb count), and the row entry point which dispatches to
    monomorphized kernels for <= 8 limbs.

    Run with: cargo run --release --example bench-hp
*/

use std::time::Instant;
use mb_arith::*;

// convert an f64 in (-32768, 32768) to the 16-bit digit format used by the client
fn f64_to_digits(v: f64, n_digits: usize) -> Vec<u32> {
    let mut t = if v < 0.0 { v + 65536.0 } else { v };
    let mut a = Vec::with_capacity(n_digits);
    for _ in 0..n_digits {
        let d = t.floor();
        a.push(d as u32);
        t = (t - d) * 65536.0;
    }
    a
}

fn main() {
    let grid = 48;
    let max_iter = 2000;

    println!("{}x{} grid, max_iter {}\n", grid, grid, max_iter);
    println!("{:>10} {:>6} {:>13} {:>13} {:>13} {:>8} {:>8} {:>9}",
        "frac bits", "limbs", "legacy u128", "u64 dynamic", "u64 rows", "dyn/leg", "row/leg", "mismatch");

    for n_digits in [5usize, 9, 13, 21, 33] {
        let chunks = 1 + (n_digits - 1 + 3)/4;

        let xmin_d = f64_to_digits(-2.2, n_digits);
        let dx_d = f64_to_digits(3.0/grid as f64, n_digits);
        let rows_d: Vec<Vec<u32>> = (0..grid)
            .map(|j| f64_to_digits(-1.5 + 3.0 * (j as f64 + 0.5) / grid as f64, n_digits))
            .collect();

        // legacy u128 half-limb, per pixel with incr
        let t = Instant::now();
        let mut counts_legacy = vec![vec![0i32; grid]; grid];
        {
            let xmin = u32_to_t::<u128>(&xmin_d);
            let dx = u32_to_t::<u128>(&dx_d);
            let mut hp = HPData::<u128>::new(chunks);
            for (j, yd) in rows_d.iter().enumerate() {
                let y = u32_to_t::<u128>(yd);
                let mut x_val = xmin.clone();
                for i in 0..grid {
                    counts_legacy[j][i] = count_iterations_hp(&mut hp, &x_val[0..chunks], &y[0..chunks], max_iter);
                    incr(&mut x_val, &dx);
                }
            }
        }
        let t_legacy = t.elapsed();

        // full-width u64, dynamic limb count, per pixel with incr
        let t = Instant::now();
        let mut counts_dyn = vec![vec![0i32; grid]; grid];
        {
            let xmin = u32_to_limbs64(&xmin_d);
            let dx = u32_to_limbs64(&dx_d);
            let mut hp = HPData64::new(chunks);
            for (j, yd) in rows_d.iter().enumerate() {
                let y = u32_to_limbs64(yd);
                let mut x_val = xmin.clone();
                for i in 0..grid {
                    counts_dyn[j][i] = count_iterations_hp64(&mut hp, &x_val[0..chunks], &y[0..chunks], max_iter);
                    incr64(&mut x_val, &dx);
                }
            }
        }
        let t_dyn = t.elapsed();

        // full-width u64 via the row dispatcher (monomorphized for <= 8 limbs)
        let t = Instant::now();
        let mut counts_row = vec![vec![0i32; grid]; grid];
        {
            let xmin = u32_to_limbs64(&xmin_d);
            let dx = u32_to_limbs64(&dx_d);
            for (j, yd) in rows_d.iter().enumerate() {
                let y = u32_to_limbs64(yd);
                mandelbrot_row_hp64(&xmin, &dx, &y, chunks, grid, max_iter, &mut counts_row[j]);
            }
        }
        let t_row = t.elapsed();

        let mut mismatches = 0;
        for j in 0..grid {
            for i in 0..grid {
                if counts_legacy[j][i] != counts_dyn[j][i] || counts_legacy[j][i] != counts_row[j][i] {
                    mismatches += 1;
                }
            }
        }

        println!("{:>10} {:>6} {:>13} {:>13} {:>13} {:>7.2}x {:>7.2}x {:>9}",
            (n_digits - 1)*16, chunks,
            format!("{:.1?}", t_legacy), format!("{:.1?}", t_dyn), format!("{:.1?}", t_row),
            t_legacy.as_secs_f64() / t_dyn.as_secs_f64(),
            t_legacy.as_secs_f64() / t_row.as_secs_f64(),
            mismatches);
    }
}
