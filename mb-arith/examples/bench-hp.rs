/*
    Benchmark: legacy generic HP engine (u128 half-limb, the previous server default)
    vs the full-width u64 limb engine, on a grid over the full Mandelbrot set.

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

    println!("{} points, max_iter {}\n", grid*grid, max_iter);
    println!("{:>10} {:>6} {:>14} {:>14} {:>9} {:>10}", "frac bits", "limbs", "legacy u128", "new u64", "speedup", "mismatch");

    for n_digits in [5usize, 9, 13, 21, 33] {
        let chunks = 1 + (n_digits - 1 + 3)/4;

        let mut coords = vec![];
        for i in 0..grid {
            for j in 0..grid {
                let x = -2.2 + 3.0 * (i as f64 + 0.5) / grid as f64;
                let y = -1.5 + 3.0 * (j as f64 + 0.5) / grid as f64;
                coords.push((f64_to_digits(x, n_digits), f64_to_digits(y, n_digits)));
            }
        }

        let t = Instant::now();
        let mut hp = HPData::<u128>::new(chunks);
        for (x, y) in &coords {
            let xo = u32_to_t::<u128>(x);
            let yo = u32_to_t::<u128>(y);
            count_iterations_hp(&mut hp, &xo[0..chunks], &yo[0..chunks], max_iter);
        }
        let t_old = t.elapsed();

        let t = Instant::now();
        let mut hp = HPData64::new(chunks);
        for (x, y) in &coords {
            let xn = u32_to_limbs64(x);
            let yn = u32_to_limbs64(y);
            count_iterations_hp64(&mut hp, &xn[0..chunks], &yn[0..chunks], max_iter);
        }
        let t_new = t.elapsed();

        // untimed: verify per-pixel counts match the legacy u128 engine exactly
        let mut mismatches = 0;
        let mut hp_new = HPData64::new(chunks);
        let mut hp_old = HPData::<u128>::new(chunks);
        for (x, y) in &coords {
            let new = count_iterations_hp64(&mut hp_new,
                &u32_to_limbs64(x)[0..chunks], &u32_to_limbs64(y)[0..chunks], max_iter);
            let old = count_iterations_hp(&mut hp_old,
                &u32_to_t::<u128>(x)[0..chunks], &u32_to_t::<u128>(y)[0..chunks], max_iter);
            if new != old {
                mismatches += 1;
            }
        }

        println!("{:>10} {:>6} {:>14} {:>14} {:>8.2}x {:>10}",
            (n_digits - 1)*16, chunks,
            format!("{:?}", t_old), format!("{:?}", t_new),
            t_old.as_secs_f64() / t_new.as_secs_f64(), mismatches);
    }
}
