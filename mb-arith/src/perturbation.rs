// *** perturbation-theory Mandelbrot for deep zoom *** //
//
// Brute force computes a full-precision orbit for *every* pixel: cost is
// O(pixels * iterations * limbs^2). Perturbation computes a single full-precision
// "reference" orbit Z_n for the whole image, then expresses every other pixel as
// a small delta d_n = z_n - Z_n that evolves in cheap f64:
//
//     d_{n+1} = 2*Z_n*d_n + d_n^2 + dc          (dc = c - C)
//
// so the limbs^2 factor collapses to one HP orbit + pixels * f64 orbits.
//
// Conventions match the rest of the crate (see count_iterations / *_hp64):
//   z_0 = c, z_{n+1} = z_n^2 + c, escape when |z|^2 >= 8, returning the index of
//   the first escaping z (or -1 if it never escapes within max_iterations).
// Internally we iterate the textbook z_0 = 0 orbit (w_k), which relates to the
// crate's z_0 = c orbit by w_{k+1} = z_k(crate); the loop below returns the
// crate-equivalent count directly, so this is a drop-in for count_iterations_hp*.
//
// Glitches (a pixel whose delta grows until it has lost all precision relative to
// the reference) are handled by Zhuoran's rebasing: whenever the true value z_n
// becomes smaller than the delta d_n, or we reach the end of the stored reference,
// we reset d_n := z_n and restart the reference index at 0. Because Z_0 = 0 and
// Z_1 = C, rebasing to index 0 reconstructs the true orbit exactly, so a single
// reference orbit suffices with no multi-reference glitch pass.
//
// The reference orbit is the only high-precision work; the per-pixel delta loop
// (`perturb_point` / `perturb_row`) is pure f64 and shared by both limb widths.
// The width-specific reference/setup code is generated for u64 (native server)
// and u32 (wasm, whose f64 delta loop runs at native speed with no 64x64->128).

use crate::{
    negate64, add64, sub64, sq64, multiply64, incr64,
    negate32, add32, sub32, sq32, multiply32, incr32,
};

const ESCAPE_R2: f64 = 8.0;

/// Iterate one pixel by perturbation against the reference `orbit`. `dcx`/`dcy`
/// are the pixel's offset from the reference coordinate (dc = c - C), in f64.
/// Returns the crate-standard iteration count (index of the first escaping z with
/// the z_0 = c convention), or -1 if it never escapes within `max_iterations`.
/// Width-agnostic: the reference is stored as f64 regardless of the limb engine.
#[inline]
pub fn perturb_point(orbit: &[(f64, f64)], dcx: f64, dcy: f64, max_iterations: i32) -> i32 {
    let ref_len = orbit.len();
    if ref_len < 2 {
        return -1;
    }
    let last = ref_len - 1;

    let mut dr = 0.0f64; // delta, d_0 = 0 (z_0 = 0 convention)
    let mut di = 0.0f64;
    let mut m = 0usize; // reference index

    for n in 0..max_iterations {
        // d_{k+1} = 2*Z_m*d_k + d_k^2 + dc
        let (zr, zi) = unsafe { *orbit.get_unchecked(m) };
        let new_dr = 2.0 * (zr * dr - zi * di) + (dr * dr - di * di) + dcx;
        let new_di = 2.0 * (zr * di + zi * dr) + 2.0 * dr * di + dcy;
        dr = new_dr;
        di = new_di;
        m += 1;

        // true value w_{k+1} = Z_m + d_{k+1}; this equals the crate's z_n, so an
        // escape here is reported at crate-count n.
        let (zmr, zmi) = unsafe { *orbit.get_unchecked(m) };
        let wr = zmr + dr;
        let wi = zmi + di;
        let w2 = wr * wr + wi * wi;
        if w2 >= ESCAPE_R2 {
            return n;
        }

        // rebase: keep the delta small, and wrap when the reference runs out.
        if w2 < dr * dr + di * di || m == last {
            dr = wr;
            di = wi;
            m = 0;
        }
    }
    -1
}

/// Compute one pixel row by perturbation. `dcx0` is dc.x of column 0, `dcx_step`
/// is the per-column increment (the f64 pixel width), `dcy` is dc.y for the row.
pub fn perturb_row(
    orbit: &[(f64, f64)],
    dcx0: f64,
    dcx_step: f64,
    dcy: f64,
    columns: usize,
    max_iterations: i32,
    out: &mut [i32],
) {
    for j in 0..columns {
        let dcx = dcx0 + j as f64 * dcx_step;
        out[j] = perturb_point(orbit, dcx, dcy, max_iterations);
    }
}

// Generate the width-specific reference/setup/conversion code for one limb type.
macro_rules! perturb_engine {
    ($limb:ty,
     $negate:ident, $add:ident, $sub:ident, $sq:ident, $multiply:ident, $incr:ident,
     $mag_to_f64:ident, $limbs_to_f64_scratch:ident, $limbs_to_f64:ident,
     $reference_orbit:ident, $perturb_setup:ident, $mandelbrot_perturb:ident) => {

/// Convert a magnitude (non-negative limb value: limb0 = integral part, limbs 1..
/// = fractional limbs, most significant first) to f64.
#[inline]
fn $mag_to_f64(m: &[$limb]) -> f64 {
    let step = 2.0f64.powi(-(<$limb>::BITS as i32));
    let mut v = m[0] as f64;
    let mut scale = step;
    for &limb in &m[1..] {
        if scale == 0.0 {
            break; // remaining limbs are below f64's smallest subnormal
        }
        v += limb as f64 * scale;
        scale *= step;
    }
    v
}

/// Convert a two's-complement limb value (limb0 = signed integral part, limbs 1..
/// = fractional limbs) to f64. `scratch` (>= x.len()) is used only when negative.
#[inline]
fn $limbs_to_f64_scratch(x: &[$limb], scratch: &mut [$limb]) -> f64 {
    if (x[0] >> (<$limb>::BITS - 1)) != 0 {
        $negate(x, scratch);
        -$mag_to_f64(&scratch[..x.len()])
    } else {
        $mag_to_f64(x)
    }
}

/// Convert a limb value to f64 (allocating). Handy for one-off conversions such as
/// the pixel step dx/dy.
pub fn $limbs_to_f64(x: &[$limb]) -> f64 {
    let mut scratch = vec![0 as $limb; x.len()];
    $limbs_to_f64_scratch(x, &mut scratch)
}

/// Compute the reference orbit Z_0 = 0, Z_{k+1} = Z_k^2 + C at high precision,
/// storing each Z_k as an f64 pair. Iterates until the reference escapes
/// (|Z|^2 >= 8) or `max_iterations` is reached. The returned vector always has at
/// least two entries (Z_0 and Z_1) for max_iterations >= 1.
///
/// `cx` / `cy` are the reference coordinate as limbs, each the same length.
pub fn $reference_orbit(cx: &[$limb], cy: &[$limb], max_iterations: i32) -> Vec<(f64, f64)> {
    let n = cx.len();
    let mut zx = vec![0 as $limb; n];
    let mut zy = vec![0 as $limb; n];
    let mut w1 = vec![0 as $limb; n];
    let mut w2 = vec![0 as $limb; n];
    let mut w3 = vec![0 as $limb; n];
    let mut w4 = vec![0 as $limb; n];
    let mut new_zx = vec![0 as $limb; n];
    let mut new_zy = vec![0 as $limb; n];
    let mut scratch = vec![0 as $limb; n];

    let mut orbit = Vec::with_capacity(max_iterations.max(0) as usize + 1);
    orbit.push((0.0, 0.0)); // Z_0 = 0

    for _ in 0..max_iterations {
        $sq(&zx, &mut w3, &mut w1); // w1 = zx^2
        $sq(&zy, &mut w3, &mut w2); // w2 = zy^2
        $add(&zx, &zx, &mut w4); // w4 = 2*zx  (capture before zx is overwritten)

        // zx' = zx^2 - zy^2 + Cx
        $sub(&w1, &w2, &mut w3);
        $add(&w3, cx, &mut new_zx);
        // zy' = 2*zx*zy + Cy
        $multiply(&w4, &zy, &mut w1, &mut w3, &mut w2); // w2 = 2*zx*zy
        $add(&w2, cy, &mut new_zy);

        zx.copy_from_slice(&new_zx);
        zy.copy_from_slice(&new_zy);

        let zr = $limbs_to_f64_scratch(&zx, &mut scratch);
        let zi = $limbs_to_f64_scratch(&zy, &mut scratch);
        orbit.push((zr, zi));
        if zr * zr + zi * zi >= ESCAPE_R2 {
            break;
        }
    }
    orbit
}

/// Build everything needed to perturb a block: the reference orbit (taken at the
/// block center), the f64 pixel steps, and column-0 dc.x. Exposed so callers can
/// compute the reference once and then drive `perturb_row` in parallel.
///
/// Returns `(orbit, dx_f, dy_f, dcx0, col_ref, row_ref)`. For row i, use
/// `dcy = (row_ref - i) * dy_f`; for the row call pass `dcx0`, `dx_f`.
/// Coordinates are limb arrays; only the first `chunks` limbs of each are used.
/// Rows advance y downward by `dy` (y_i = ymax - i*dy), matching the brute engine.
pub fn $perturb_setup(
    xmin: &[$limb],
    dx: &[$limb],
    ymax: &[$limb],
    dy: &[$limb],
    chunks: usize,
    rows: usize,
    columns: usize,
    max_iterations: i32,
) -> (Vec<(f64, f64)>, f64, f64, f64, usize, usize) {
    let col_ref = columns / 2;
    let row_ref = rows / 2;

    // Cx = xmin + col_ref*dx  (at full precision)
    let mut cx = xmin[..chunks].to_vec();
    for _ in 0..col_ref {
        $incr(&mut cx, &dx[..chunks]);
    }
    // Cy = ymax - row_ref*dy  (y advances downward)
    let mut dy_neg = vec![0 as $limb; chunks];
    $negate(&dy[..chunks], &mut dy_neg);
    let mut cy = ymax[..chunks].to_vec();
    for _ in 0..row_ref {
        $incr(&mut cy, &dy_neg);
    }

    let orbit = $reference_orbit(&cx, &cy, max_iterations);

    let dx_f = $limbs_to_f64(&dx[..chunks]);
    let dy_f = $limbs_to_f64(&dy[..chunks]);
    let dcx0 = -(col_ref as f64) * dx_f;

    (orbit, dx_f, dy_f, dcx0, col_ref, row_ref)
}

/// Compute a `rows` x `columns` block by perturbation, reference at block center.
/// Serial driver (used by tests and the wasm strip export); the server computes
/// the reference once and parallelizes `perturb_row` across rows itself.
pub fn $mandelbrot_perturb(
    xmin: &[$limb],
    dx: &[$limb],
    ymax: &[$limb],
    dy: &[$limb],
    chunks: usize,
    rows: usize,
    columns: usize,
    max_iterations: i32,
    out: &mut [i32],
) {
    let (orbit, dx_f, dy_f, dcx0, _col_ref, row_ref) =
        $perturb_setup(xmin, dx, ymax, dy, chunks, rows, columns, max_iterations);

    for i in 0..rows {
        let dcy = (row_ref as f64 - i as f64) * dy_f;
        let row = &mut out[i * columns..(i + 1) * columns];
        perturb_row(&orbit, dcx0, dx_f, dcy, columns, max_iterations, row);
    }
}

    };
}

perturb_engine!(u64,
    negate64, add64, sub64, sq64, multiply64, incr64,
    mag_to_f64_64, limbs_to_f64_scratch_64, limbs64_to_f64,
    reference_orbit64, perturb_setup64, mandelbrot_perturb64);

perturb_engine!(u32,
    negate32, add32, sub32, sq32, multiply32, incr32,
    mag_to_f64_32, limbs_to_f64_scratch_32, limbs32_to_f64,
    reference_orbit32, perturb_setup32, mandelbrot_perturb32);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        u32_to_limbs64, u32_to_limbs32, negate64, incr64, negate32, incr32,
        mandelbrot_row_hp64, mandelbrot_row_hp32,
    };

    // build a 16-bit-digit coordinate: signed integral part + fraction digits
    fn coord(int_part: i32, frac: &[u32]) -> Vec<u32> {
        let mut a = vec![(int_part as i16 as u16) as u32];
        a.extend_from_slice(frac);
        a
    }

    fn chunks64_for(n_digits: usize) -> usize {
        1 + (n_digits - 1 + 3) / 4
    }
    fn chunks32_for(n_digits: usize) -> usize {
        1 + (n_digits - 1 + 1) / 2
    }

    // brute-force reference image via the u64 HP engine (y_i = ymax - i*dy)
    fn brute_image64(
        xmin: &[u64], dx: &[u64], ymax: &[u64], dy: &[u64],
        chunks: usize, rows: usize, columns: usize, max_iter: i32,
    ) -> Vec<Vec<i32>> {
        let mut dy_neg = vec![0u64; xmin.len()];
        negate64(dy, &mut dy_neg);
        let mut y = ymax.to_vec();
        let mut out = vec![vec![0i32; columns]; rows];
        for i in 0..rows {
            mandelbrot_row_hp64(xmin, dx, &y, chunks, columns, max_iter, &mut out[i]);
            incr64(&mut y, &dy_neg);
        }
        out
    }

    fn brute_image32(
        xmin: &[u32], dx: &[u32], ymax: &[u32], dy: &[u32],
        chunks: usize, rows: usize, columns: usize, max_iter: i32,
    ) -> Vec<Vec<i32>> {
        let mut dy_neg = vec![0u32; xmin.len()];
        negate32(dy, &mut dy_neg);
        let mut y = ymax.to_vec();
        let mut out = vec![vec![0i32; columns]; rows];
        for i in 0..rows {
            mandelbrot_row_hp32(xmin, dx, &y, chunks, columns, max_iter, &mut out[i]);
            incr32(&mut y, &dy_neg);
        }
        out
    }

    #[test]
    fn interior_all_neg1() {
        // tiny window around the origin: everything is deep inside the main cardioid
        let xd = coord(0, &[0, 0, 0, 0]);
        let yd = coord(0, &[0, 0, 0, 0]);
        let dxd = coord(0, &[0, 0, 1, 0]); // 2^-48
        let dyd = coord(0, &[0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (xmin, dx, ymax, dy) =
            (u32_to_limbs64(&xd), u32_to_limbs64(&dxd), u32_to_limbs64(&yd), u32_to_limbs64(&dyd));
        let (rows, columns, max_iter) = (24, 24, 500);

        let mut pert = vec![0i32; rows * columns];
        mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
        for &v in &pert {
            assert_eq!(v, -1, "interior pixel should never escape");
        }
    }

    #[test]
    fn exterior_fast_escape_exact() {
        // window near (0.5, 0.5): everything escapes in a handful of iterations, so
        // f64 rounding cannot flip any count -> perturbation must match brute exactly.
        let xd = coord(0, &[0x8000, 0, 0, 0]); // 0.5
        let yd = coord(0, &[0x8000, 0, 0, 0]); // 0.5
        let dxd = coord(0, &[0, 0, 1, 0]); // 2^-48
        let dyd = coord(0, &[0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (xmin, dx, ymax, dy) =
            (u32_to_limbs64(&xd), u32_to_limbs64(&dxd), u32_to_limbs64(&yd), u32_to_limbs64(&dyd));
        let (rows, columns, max_iter) = (24, 24, 500);

        let mut pert = vec![0i32; rows * columns];
        mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
        let brute = brute_image64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);
        for i in 0..rows {
            for j in 0..columns {
                assert_eq!(pert[i * columns + j], brute[i][j], "mismatch at ({i},{j})");
            }
        }
    }

    // deep window (step 2^-64) centered on the Misiurewicz point c = (0, 1), which
    // lies exactly on the set boundary -> the window straddles the fractal boundary
    // (rich near-boundary structure, too deep for f64 to resolve directly). Runs the
    // same assertions against both limb engines.
    fn deep_view_body(use32: bool) {
        let xd = coord(0, &[0, 0, 0, 0, 0]); // 0.0
        let yd = coord(1, &[0, 0, 0, 0, 0]); // 1.0
        let dxd = coord(0, &[0, 0, 0, 1, 0]); // 2^-64
        let dyd = coord(0, &[0, 0, 0, 1, 0]);
        let (rows, columns, max_iter) = (48, 48, 8000);

        let (mismatches, total, max_c, distinct) = if use32 {
            let chunks = chunks32_for(xd.len());
            let (dx, dy) = (u32_to_limbs32(&dxd), u32_to_limbs32(&dyd));
            let mut xmin = u32_to_limbs32(&xd);
            let mut dx_neg = vec![0u32; xmin.len()];
            negate32(&dx, &mut dx_neg);
            for _ in 0..columns / 2 { incr32(&mut xmin, &dx_neg); }
            let mut ymax = u32_to_limbs32(&yd);
            for _ in 0..rows / 2 { incr32(&mut ymax, &dy); }

            let mut pert = vec![0i32; rows * columns];
            mandelbrot_perturb32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
            let brute = brute_image32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);
            count_stats(&pert, &brute, rows, columns)
        } else {
            let chunks = chunks64_for(xd.len());
            let (dx, dy) = (u32_to_limbs64(&dxd), u32_to_limbs64(&dyd));
            let mut xmin = u32_to_limbs64(&xd);
            let mut dx_neg = vec![0u64; xmin.len()];
            negate64(&dx, &mut dx_neg);
            for _ in 0..columns / 2 { incr64(&mut xmin, &dx_neg); }
            let mut ymax = u32_to_limbs64(&yd);
            for _ in 0..rows / 2 { incr64(&mut ymax, &dy); }

            let mut pert = vec![0i32; rows * columns];
            mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
            let brute = brute_image64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);
            count_stats(&pert, &brute, rows, columns)
        };

        assert!(
            max_c >= 50 && distinct >= 20,
            "view not a meaningful near-boundary test: max_count={max_c} distinct={distinct}"
        );
        // perturbation is f64-approximate; near the boundary a few pixels may differ
        // from exact HP, but the overall rate must be tiny (<1%).
        assert!(mismatches * 100 <= total, "too many mismatches: {mismatches}/{total}");
    }

    fn count_stats(pert: &[i32], brute: &[Vec<i32>], rows: usize, columns: usize) -> (usize, usize, i32, usize) {
        let mut mismatches = 0usize;
        let mut vals: Vec<i32> = Vec::with_capacity(rows * columns);
        for i in 0..rows {
            for j in 0..columns {
                let (p, b) = (pert[i * columns + j], brute[i][j]);
                vals.push(b);
                if p != b {
                    mismatches += 1;
                    // fast escapes are robust: a disagreement there is a real bug
                    assert!(b < 0 || b > 60, "fast-escape pixel disagrees: perturb={p} brute={b}");
                }
            }
        }
        let max_c = vals.iter().copied().max().unwrap();
        vals.sort_unstable();
        vals.dedup();
        (mismatches, rows * columns, max_c, vals.len())
    }

    #[test]
    fn deep_view_matches_brute_u64() {
        deep_view_body(false);
    }

    #[test]
    fn deep_view_matches_brute_u32() {
        deep_view_body(true);
    }
}
