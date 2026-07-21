// Native side of the strip A/B: build the center reference to a point cap
// (truncated, like a no-rescue ladder), then time strip rows against it --
// the exact work the server's strip phase does, single-threaded, no HTTP, no
// passes. Compare with the wasm twin (test-v14.js timing the same rows via
// compute_strip_with_orbit) to get the raw native-vs-wasm strip ratio.
//
// Usage: bench-strip-ab <limbs.txt> <rows> <columns> <cap_points> <row0> <nrows>

use mb_arith::*;
use std::io::Read;
use std::time::Instant;

fn main() {
    let mut a = std::env::args().skip(1);
    let path = a.next().expect("limbs.txt");
    let rows: usize = a.next().expect("rows").parse().unwrap();
    let columns: usize = a.next().expect("columns").parse().unwrap();
    let cap: i32 = a.next().expect("cap_points").parse().unwrap();
    let row0: usize = a.next().expect("row0").parse().unwrap();
    let nrows: usize = a.next().expect("nrows").parse().unwrap();

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
    let dcx0 = dx_fe.mul_f64(-((columns / 2) as f64));
    let row_ref = rows / 2;

    let t0 = Instant::now();
    let mut b = OrbitBuilder64::at_grid(&xmin, &dx, &ymax, &dy, chunks, row_ref, columns / 2);
    b.extend(max_iter.min(cap), None);
    let pts = b.orbit.len() - 1;
    println!("orbit: {} pts in {:.1}s ({:.3} us/pt), escaped={}",
        pts, t0.elapsed().as_secs_f64(),
        t0.elapsed().as_secs_f64() * 1e6 / pts as f64, orbit_escaped(&b.orbit));

    let t0 = Instant::now();
    let table = bla_image_table(&b.orbit, dx_fe, dy_fe, dcx0, FE_ZERO, row_ref, rows, columns);
    println!("table: {:.1}s", t0.elapsed().as_secs_f64());

    let mut out = vec![0i32; nrows * columns];
    let t0 = Instant::now();
    bla_strip_fe_with_table(&b.orbit, &b.dips, &table, dx_fe, dy_fe, dcx0, FE_ZERO,
        row_ref, row0, nrows, columns, max_iter, &mut out, None);
    let el = t0.elapsed().as_secs_f64();
    let white = out.iter().filter(|&&c| c == -2).count();
    let pos: Vec<i64> = out.iter().filter(|&&c| c >= 0).map(|&c| c as i64).collect();
    let mean = if pos.is_empty() { 0.0 } else { pos.iter().sum::<i64>() as f64 / pos.len() as f64 };
    println!("strips: {} rows in {:.2}s = {:.3} rows/s ({:.2} ms/px); white {} mean count {:.0}",
        nrows, el, nrows as f64 / el, el * 1000.0 / out.len() as f64, white, mean);
}
