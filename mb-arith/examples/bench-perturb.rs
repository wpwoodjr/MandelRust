// Isolate where the perturbation delta-loop speedup actually comes from:
//   1. single-pixel scalar  (old per-pixel path, early-exit)
//   2. 2-lane scalar        (branchless lock-step lane kernel -> ILP)
//   3. NEON f64x2           (explicit SIMD on the same 2-lane structure)
// The 1->2 gap is instruction-level parallelism; the 2->3 gap is SIMD proper.
use mb_arith::*;
use std::time::Instant;

fn coord(int_part: i32, frac: &[u32]) -> Vec<u32> {
    let mut a = vec![(int_part as i16 as u16) as u32];
    a.extend_from_slice(frac);
    a
}

fn main() {
    // deep interior window at the origin: every pixel runs to max_iter (no escape),
    // so all three do identical work -> a clean isolation of ILP vs SIMD.
    let xd = coord(0, &[0, 0, 0, 0, 0, 0, 0, 0]);
    let yd = coord(0, &[0, 0, 0, 0, 0, 0, 0, 0]);
    let dxd = coord(0, &[0, 0, 0, 0, 0, 0, 1, 0]);
    let dyd = coord(0, &[0, 0, 0, 0, 0, 0, 1, 0]);
    let chunks = 1 + (xd.len() - 1 + 3) / 4;
    let (dx, dy) = (u32_to_limbs64(&dxd), u32_to_limbs64(&dyd));
    let mut xmin = u32_to_limbs64(&xd);
    let mut dx_neg = vec![0u64; xmin.len()];
    negate64(&dx, &mut dx_neg);
    let (rows, columns, max_iter) = (48usize, 256usize, 50000i32);
    for _ in 0..columns / 2 { incr64(&mut xmin, &dx_neg); }
    let mut ymax = u32_to_limbs64(&yd);
    for _ in 0..rows / 2 { incr64(&mut ymax, &dy); }

    let (orbit, dx_f, dy_f, dcx0, _cr, row_ref) =
        perturb_setup64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);
    println!("view {rows}x{columns}, max_iter {max_iter}, ref orbit len {}", orbit.len());

    let dcx = |j: usize| dcx0 + j as f64 * dx_f;
    let dcy = |i: usize| (row_ref as f64 - i as f64) * dy_f;

    // 1. single-pixel scalar
    let mut s1 = 0i64;
    let t = Instant::now();
    for i in 0..rows { for j in 0..columns {
        if let PtResult::Escaped(c) = perturb_point_shared(&orbit, dcx(j), dcy(i), max_iter) { s1 += c as i64; }
    }}
    let t1 = t.elapsed().as_secs_f64() * 1e3;

    // 2. 2-lane scalar (branchless lock-step)
    let mut s2 = 0i64;
    let t = Instant::now();
    for i in 0..rows { for j in (0..columns).step_by(2) {
        for r in perturb_lanes_shared::<2>(&orbit, &[dcx(j), dcx(j+1)], &[dcy(i), dcy(i)], max_iter) {
            if let PtResult::Escaped(c) = r { s2 += c as i64; }
        }
    }}
    let t2 = t.elapsed().as_secs_f64() * 1e3;

    // 3. NEON f64x2
    #[cfg(target_arch = "aarch64")]
    {
        let mut s3 = 0i64;
        let t = Instant::now();
        for i in 0..rows { for j in (0..columns).step_by(2) {
            for r in perturb_pair_shared_neon(&orbit, &[dcx(j), dcx(j+1)], &[dcy(i), dcy(i)], max_iter) {
                if let PtResult::Escaped(c) = r { s3 += c as i64; }
            }
        }}
        let t3 = t.elapsed().as_secs_f64() * 1e3;
        println!("1. single-pixel scalar : {t1:8.1} ms");
        println!("2. 2-lane scalar (ILP) : {t2:8.1} ms   -> {:.2}x vs single", t1/t2);
        println!("3. NEON f64x2 (SIMD)   : {t3:8.1} ms   -> {:.2}x vs 2-lane, {:.2}x vs single", t2/t3, t1/t3);
        assert!(s1 == s2 && s2 == s3, "kernels disagree: {s1} {s2} {s3}");
    }
    #[cfg(not(target_arch = "aarch64"))]
    { println!("1. single: {t1:.1} ms   2. 2-lane: {t2:.1} ms ({:.2}x)", t1/t2); let _ = (s1, s2); }
}
