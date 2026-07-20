// Reference-length A/B: does render cost scale with wraps (per-wrap cost P
// roughly constant, so short references hurt) or does it not (P ~ eps*L, so
// wraps are free and the shortest legal reference always wins)? Times the
// SAME strip against the shortest / median / longest escaping reference the
// view's probe rows actually offer.
//
// Usage: cargo run --release --example bench-refsel -- <limbs.txt> [timed_rows]
// limbs.txt: "u32_chunks maxIter\n" then xmin/dx/ymax/dy as u32 16-bit digit
// lines (the exact arrays an HP2 request carries).

use mb_arith::*;
use std::io::Read;
use std::time::Instant;

const ROWS: usize = 480;
const COLUMNS: usize = 640;
const PREFIX: i32 = 16_000_000; // deepish's first candidate-bearing prefix
const PROBE_ROWS: usize = 16;

fn main() {
    let path = std::env::args().nth(1).expect("usage: bench-refsel <limbs.txt> [timed_rows]");
    let timed_rows: usize = std::env::args().nth(2).map_or(4, |s| s.parse().unwrap());
    let mut s = String::new();
    std::fs::File::open(path).unwrap().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace().map(|t| t.parse::<i64>().unwrap());
    let u32_chunks = it.next().unwrap() as usize;
    let max_iter = it.next().unwrap() as i32;
    let mut arr = |n: usize| -> Vec<u32> { (0..n).map(|_| it.next().unwrap() as u32).collect() };
    let (xd, dxd, yd, dyd) = (arr(u32_chunks), arr(u32_chunks), arr(u32_chunks), arr(u32_chunks));
    let chunks = 1 + (u32_chunks - 1 + 3) / 4;
    let xmin = u32_to_limbs64(&xd);
    let dx = u32_to_limbs64(&dxd);
    let ymax = u32_to_limbs64(&yd);
    let dy = u32_to_limbs64(&dyd);
    let dx_fe = limbs64_to_fe(&dx[..chunks]);
    let dy_fe = limbs64_to_fe(&dy[..chunks]);
    let center_dcx0 = dx_fe.mul_f64(-((COLUMNS / 2) as f64));
    let row_ref = ROWS / 2;
    println!("{} u64 limbs, maxIter {}, timing {} rows around row {}",
        chunks, max_iter, timed_rows, ROWS / 2 - timed_rows / 2);

    // center prefix (what the ladder holds when candidates first appear)
    let t0 = Instant::now();
    let mut b = OrbitBuilder64::at_grid(&xmin, &dx, &ymax, &dy, chunks, row_ref, COLUMNS / 2);
    b.extend(PREFIX, None);
    let build_s = t0.elapsed().as_secs_f64();
    let len = (b.orbit.len() - 1) as i32;
    println!("center prefix: {} pts in {:.1}s ({:.2} us/pt)",
        len, build_s, build_s * 1e6 / len as f64);
    assert!(!orbit_escaped(&b.orbit), "center escaped; view unsuitable for this bench");

    // probe pass (timed: this is also the in-run estimator of walk cost)
    let t0 = Instant::now();
    let table = bla_image_table(&b.orbit, dx_fe, dy_fe, center_dcx0, FE_ZERO, row_ref, ROWS, COLUMNS);
    println!("probe table: {:.1}s", t0.elapsed().as_secs_f64());
    let t0 = Instant::now();
    let mut cands: Vec<(i32, usize, usize)> = Vec::new();
    for pi in 0..PROBE_ROWS {
        let r = pi * (ROWS - 1) / (PROBE_ROWS - 1);
        let mut out = vec![0i32; COLUMNS];
        bla_strip_fe_with_table(&b.orbit, &b.dips, &table, dx_fe, dy_fe, center_dcx0,
            FE_ZERO, row_ref, r, 1, COLUMNS, len, &mut out, None);
        for (c, &ct) in out.iter().enumerate() {
            if ct >= 1_000_000 {
                cands.push((ct, r, c));
            }
        }
    }
    cands.sort();
    println!("probe pass: {:.1}s; {} candidates >=1M within {}M (min {} median {} max {})",
        t0.elapsed().as_secs_f64(), cands.len(), PREFIX / 1_000_000,
        cands.first().map_or(0, |c| c.0), cands[cands.len() / 2].0,
        cands.last().map_or(0, |c| c.0));
    drop(table);
    drop(b);

    let picks = [cands[0], cands[cands.len() / 2], cands[cands.len() - 1]];
    let r0 = ROWS / 2 - timed_rows / 2;
    for (label, &(ct, pr, pc)) in ["shortest", "median", "longest"].iter().zip(picks.iter()) {
        let t0 = Instant::now();
        let (orbit, dips, dxf, dyf, dcx0, _c, rr) = perturb_setup_fe64_at(
            &xmin, &dx, &ymax, &dy, chunks, pr, pc, max_iter, i32::MAX, None);
        let build = t0.elapsed().as_secs_f64();
        let l = (orbit.len() - 1) as f64;
        let t0 = Instant::now();
        let tab = bla_image_table(&orbit, dxf, dyf, dcx0, FE_ZERO, rr, ROWS, COLUMNS);
        let tab_s = t0.elapsed().as_secs_f64();
        let mut out = vec![0i32; timed_rows * COLUMNS];
        let t0 = Instant::now();
        bla_strip_fe_with_table(&orbit, &dips, &tab, dxf, dyf, dcx0, FE_ZERO, rr,
            r0, timed_rows, COLUMNS, max_iter, &mut out, None);
        let render = t0.elapsed().as_secs_f64();
        let pos: Vec<i64> = out.iter().filter(|&&c| c >= 0).map(|&c| c as i64).collect();
        let interior = out.iter().filter(|&&c| c == -1).count();
        let mean_ct = if pos.is_empty() { max_iter as f64 }
            else { pos.iter().sum::<i64>() as f64 / pos.len() as f64 };
        // effective mean count including -1 px (they iterate the full maxIter)
        let n = out.len() as f64;
        let eff_ct = (mean_ct * pos.len() as f64 + max_iter as f64 * interior as f64) / n;
        let wraps = eff_ct / l;
        println!("{label}: ref ({pc},{pr}) escapes {ct}; build {:.1}s, table {:.1}s, \
            render {} rows in {:.1}s = {:.3} rows/s ({:.2} ms/px, {} interior px)",
            build, tab_s, timed_rows, render, timed_rows as f64 / render,
            render * 1000.0 / n, interior);
        println!("    mean count {:.0} (eff {:.0}); wraps/px {:.1}; per-wrap {:.0} us",
            mean_ct, eff_ct, wraps, render * 1e6 / n / wraps);
    }
}
