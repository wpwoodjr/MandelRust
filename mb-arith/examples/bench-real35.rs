// The 35-digit / maxIter=2000 saved view: why is the server's glitch engine slow
// here? Compare rebasing vs glitch per 4-row strip, u64 (server) and u32 (wasm).
use mb_arith::*;
use std::time::Instant;

const XMIN_D: [u32; 10] = [65534, 9144, 56334, 65399, 61036, 36172, 52659, 28981, 4430, 17519];
const DX_D: [u32; 10] = [0, 0, 0, 0, 0, 0, 1, 57312, 56425, 22742];
const YMAX_D: [u32; 10] = [0, 8, 21677, 19005, 52403, 49030, 53408, 23540, 50755, 35330];
const DY_D: [u32; 10] = [0, 0, 0, 0, 0, 0, 1, 57364, 10543, 8664];
const ROWS: usize = 600;
const COLS: usize = 800;
const MAX_ITER: i32 = 2000;
const STRIP: usize = 4;

fn main() {
    let rows: usize = std::env::var("BENCH_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(ROWS).min(ROWS);

    // u64 (server)
    let xmin = u32_to_limbs64(&XMIN_D);
    let dx = u32_to_limbs64(&DX_D);
    let ymax0 = u32_to_limbs64(&YMAX_D);
    let dy = u32_to_limbs64(&DY_D);
    let chunks64 = 1 + (XMIN_D.len() - 1 + 3) / 4;
    let mut dy_neg = vec![0u64; dy.len()];
    negate64(&dy, &mut dy_neg);

    let run64 = |glitch: bool| -> (f64, Vec<i32>) {
        let mut out = vec![0i32; rows * COLS];
        let mut ymax = ymax0.clone();
        let t = Instant::now();
        let mut r0 = 0;
        while r0 < rows {
            let h = STRIP.min(rows - r0);
            let o = &mut out[r0 * COLS..(r0 + h) * COLS];
            if glitch {
                mandelbrot_perturb_glitch64(&xmin, &dx, &ymax, &dy, chunks64, h, COLS, MAX_ITER, o);
            } else {
                mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks64, h, COLS, MAX_ITER, o);
            }
            for _ in 0..h { incr64(&mut ymax, &dy_neg); }
            r0 += h;
        }
        (t.elapsed().as_secs_f64() * 1e3, out)
    };

    // reference-orbit length at the first strip center (how early does it escape?)
    let (orbit, ..) = perturb_setup64(&xmin, &dx, &ymax0, &dy, chunks64, STRIP, COLS, MAX_ITER);
    println!("view {rows}x{COLS}, maxIter {MAX_ITER}, {} u64 limbs; first-strip ref orbit len {}", chunks64, orbit.len());

    let (t_rebase, out_r) = run64(false);
    let (t_glitch, out_g) = run64(true);

    // BLA per strip
    let mut out_bla = vec![0i32; rows * COLS];
    let t_bla = {
        let mut ymax = ymax0.clone();
        let t = Instant::now();
        let mut r0 = 0;
        while r0 < rows {
            let h = STRIP.min(rows - r0);
            mandelbrot_perturb_bla64(&xmin, &dx, &ymax, &dy, chunks64, h, COLS, MAX_ITER, &mut out_bla[r0 * COLS..(r0 + h) * COLS]);
            for _ in 0..h { incr64(&mut ymax, &dy_neg); }
            r0 += h;
        }
        t.elapsed().as_secs_f64() * 1e3
    };

    let n = rows * COLS;
    let interior = out_g.iter().filter(|&&c| c < 0).count();
    let diff = out_r.iter().zip(&out_g).filter(|(a, b)| a != b).count();
    let diff_bla = out_r.iter().zip(&out_bla).filter(|(a, b)| a != b).count();
    println!("u64 rebasing : {t_rebase:8.1} ms   ({:.1} ms per 4-row strip)", t_rebase / (rows as f64 / STRIP as f64));
    println!("u64 glitch   : {t_glitch:8.1} ms   ({:.1} ms per 4-row strip)  -> {:.2}x vs rebasing", t_glitch / (rows as f64 / STRIP as f64), t_rebase / t_glitch);
    println!("u64 BLA      : {t_bla:8.1} ms   ({:.1} ms per 4-row strip)  -> {:.2}x vs rebasing, {:.2}x vs glitch", t_bla / (rows as f64 / STRIP as f64), t_rebase / t_bla, t_glitch / t_bla);
    println!("interior px  : {interior}/{n}, rebasing-vs-glitch mismatches {diff}, rebasing-vs-BLA {diff_bla}");
}
