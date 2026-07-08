// Does the SIMD win survive the app's tiny strips? Tile a deep image into strips
// of various heights and compare the scalar rebasing engine (mandelbrot_perturb64)
// to the SIMD glitch engine (mandelbrot_perturb_glitch64, NEON on aarch64), the
// way the browser does (one reference orbit per strip, HP, not SIMD-accelerated).
use mb_arith::*;
use std::time::Instant;

fn coord(int_part: i32, frac: &[u32]) -> Vec<u32> {
    let mut a = vec![(int_part as i16 as u16) as u32];
    a.extend_from_slice(frac);
    a
}

fn main() {
    // deep interior-ish window near the origin (every pixel runs to max_iter -> the
    // most pixel work, best case for the delta-loop SIMD), ~9 u64 limbs
    let xd = coord(0, &[0, 0, 0, 0, 0, 0, 0, 0]);
    let yd = coord(0, &[0, 0, 0, 0, 0, 0, 0, 0]);
    let dxd = coord(0, &[0, 0, 0, 0, 0, 0, 1, 0]);
    let dyd = coord(0, &[0, 0, 0, 0, 0, 0, 1, 0]);
    let chunks = 1 + (xd.len() - 1 + 3) / 4;
    let (dx, dy) = (u32_to_limbs64(&dxd), u32_to_limbs64(&dyd));
    let xmin = u32_to_limbs64(&xd);
    let ymax0 = u32_to_limbs64(&yd);
    let mut dy_neg = vec![0u64; ymax0.len()];
    negate64(&dy, &mut dy_neg);

    let (rows, columns, max_iter) = (256usize, 512usize, 3000i32);
    println!("image {rows}x{columns}, max_iter {max_iter}, chunks {chunks}");
    println!("{:>6}  {:>10}  {:>10}  {:>8}", "strip", "scalar ms", "SIMD ms", "speedup");

    for &sh in &[4usize, 16, 64, 256] {
        let run = |glitch: bool| -> f64 {
            let mut out = vec![0i32; sh * columns];
            let mut ymax = ymax0.clone();
            let t = Instant::now();
            let mut r0 = 0;
            while r0 < rows {
                let h = sh.min(rows - r0);
                if glitch {
                    mandelbrot_perturb_glitch64(&xmin, &dx, &ymax, &dy, chunks, h, columns, max_iter, &mut out[..h * columns]);
                } else {
                    mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks, h, columns, max_iter, &mut out[..h * columns]);
                }
                for _ in 0..h { incr64(&mut ymax, &dy_neg); }
                r0 += h;
            }
            t.elapsed().as_secs_f64() * 1e3
        };
        let scalar = run(false);
        let simd = run(true);
        println!("{:>6}  {:>10.1}  {:>10.1}  {:>7.2}x", sh, scalar, simd, scalar / simd);
    }
}
