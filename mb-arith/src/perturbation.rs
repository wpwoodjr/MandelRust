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
    negate64, add64, sub64, sq64, multiply64, incr64, HPData64, count_iterations_hp64,
    negate32, add32, sub32, sq32, multiply32, incr32, HPData32, count_iterations_hp32,
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

// *** bivariate linear approximation (BLA) *** //
//
// When |d| is small against |Z_n|, the delta step
//     d' = 2*Z_n*d + d^2 + dc
// is dominated by its linear part: d' ~= A*d + B*dc with A = 2*Z_n, B = 1.
// Consecutive linear steps compose into a single linear step (A, B merge), so we
// precompute, for skips of 1, 2, 4, ... iterations along the reference orbit, the
// composed (A, B) plus a validity radius r. Entering a skip with |d| < r keeps
// |d| inside every sub-step's own radius (the merge shrinks r accordingly), so a
// valid skip can neither miss an escape nor a missed-rebase glitch: both require
// |d| ~ |Z|, while validity keeps |d| <= BLA_EPS*|Z| << |Z|. Where the reference
// passes near zero (r -> 0) or |d| has grown (near escape), no skip validates and
// the loop falls back to exact single steps, so the interesting iterations are
// still computed exactly.
//
// BLA_EPS scales the per-step radius (r_1 = BLA_EPS * |Z_n|): smaller is more
// accurate but skips less. Approximation error only shifts pixels sitting exactly
// on an iteration-count boundary; validated against brute HP in tests.
pub const BLA_EPS: f64 = 9.094_947_017_729_282e-13; // 2^-40

#[derive(Clone, Copy)]
pub struct BlaEntry {
    // d' = A*d + B*dc (complex), valid while |d|^2 < r2
    pub ax: f64,
    pub ay: f64,
    pub bx: f64,
    pub by: f64,
    pub r2: f64,
}

/// Composed-skip table over a reference orbit: level k, entry j skips the 2^k
/// iterations starting at orbit index j*2^k (power-of-two aligned). Level 0 is
/// the linearized single step; each level above merges pairs from the one below.
/// Stored flat (one allocation, levels back to back) so a probe is a single
/// computed-index load -- wasm engines don't hoist nested-Vec indirections.
pub struct BlaTable {
    pub entries: Vec<BlaEntry>,
    pub level_off: Vec<u32>, // start of level k within entries
}

// compose "x then y" into one linear step over both spans
fn bla_merge(x: &BlaEntry, y: &BlaEntry, dc_max: f64) -> BlaEntry {
    // A = Ay*Ax, B = Ay*Bx + By  (complex)
    let ax = y.ax * x.ax - y.ay * x.ay;
    let ay = y.ax * x.ay + y.ay * x.ax;
    let bx = y.ax * x.bx - y.ay * x.by + y.bx;
    let by = y.ax * x.by + y.ay * x.bx + y.by;
    // valid while x is valid AND x's output stays inside y's radius:
    // |Ax*d + Bx*dc| <= |Ax|*|d| + |Bx|*dc_max < ry
    let ax_mag = (x.ax * x.ax + x.ay * x.ay).sqrt();
    let bx_mag = (x.bx * x.bx + x.by * x.by).sqrt();
    let rx = x.r2.sqrt();
    let ry = y.r2.sqrt();
    let r = if ax_mag > 0.0 {
        rx.min(((ry - bx_mag * dc_max) / ax_mag).max(0.0))
    } else {
        rx
    };
    BlaEntry { ax, ay, bx, by, r2: r * r }
}

/// Build the BLA table for `orbit`. `dc_max` must bound |dc| over every pixel the
/// table will serve; a bigger bound shrinks merged radii (slower, never wrong).
/// Memory/build cost is ~2x the orbit length; build once per reference orbit.
pub fn build_bla_table(orbit: &[(f64, f64)], dc_max: f64) -> BlaTable {
    // step i maps d_i -> d_{i+1} using Z_i; the last usable step needs Z_{i+1}
    // to exist for the escape check, hence orbit.len()-1 steps.
    let steps = orbit.len().saturating_sub(1);
    let mut entries = Vec::with_capacity(2 * steps + 8);
    for i in 0..steps {
        let (zx, zy) = orbit[i];
        let r = BLA_EPS * (zx * zx + zy * zy).sqrt();
        entries.push(BlaEntry { ax: 2.0 * zx, ay: 2.0 * zy, bx: 1.0, by: 0.0, r2: r * r });
    }
    let mut level_off = vec![0u32];
    let (mut off, mut len) = (0usize, steps);
    while len >= 2 {
        level_off.push(entries.len() as u32);
        for j in 0..len / 2 {
            let merged = bla_merge(&entries[off + 2 * j], &entries[off + 2 * j + 1], dc_max);
            entries.push(merged);
        }
        off = *level_off.last().unwrap() as usize;
        len /= 2;
    }
    BlaTable { entries, level_off }
}

/// Like `perturb_point` (same rebasing, same count conventions -- see that fn),
/// but consults the BLA table each iteration and replaces up to 2^k exact steps
/// with one composed multiply-add when the current |d| is inside a skip's radius.
#[inline]
pub fn perturb_point_bla(
    orbit: &[(f64, f64)],
    bla: &BlaTable,
    dcx: f64,
    dcy: f64,
    max_iterations: i32,
) -> i32 {
    let ref_len = orbit.len();
    if ref_len < 2 {
        return -1;
    }
    let last = ref_len - 1;
    let n_levels = bla.level_off.len();

    let mut dr = 0.0f64;
    let mut di = 0.0f64;
    let mut d2 = 0.0f64; // |d|^2, maintained across iterations
    let mut m = 0usize; // reference index
    let mut n = 0i32; // crate-standard iteration counter

    while n < max_iterations {
        // Longest power-of-two skip aligned at m whose radius admits |d|. Radii
        // are monotone non-increasing up the levels (a merged radius is <= its
        // left child's), and the bounds checks only get harder as skips grow, so
        // scan UPWARD and stop at the first failure: iterations that cannot skip
        // (large |d|, shallow zoom) pay a single probe instead of a full descent.
        // Skips of 1 aren't taken: a linearized step costs the same as an exact one.
        // (m + s <= last also keeps m>>k inside level k: len_k = steps >> k.)
        let k_align = if m == 0 { usize::MAX } else { m.trailing_zeros() as usize };
        let k_max = k_align.min(n_levels - 1);
        let mut best_k = 0usize;
        let mut k = 1usize;
        while k <= k_max {
            let s = 1usize << k;
            if m + s > last || (n as usize + s) > max_iterations as usize {
                break;
            }
            let r2 = unsafe {
                bla.entries
                    .get_unchecked(*bla.level_off.get_unchecked(k) as usize + (m >> k))
                    .r2
            };
            if d2 >= r2 {
                break;
            }
            best_k = k;
            k += 1;
        }
        if best_k > 0 {
            let e = unsafe {
                bla.entries
                    .get_unchecked(*bla.level_off.get_unchecked(best_k) as usize + (m >> best_k))
            };
            let new_dr = e.ax * dr - e.ay * di + e.bx * dcx - e.by * dcy;
            let new_di = e.ax * di + e.ay * dr + e.bx * dcy + e.by * dcx;
            dr = new_dr;
            di = new_di;
            m += 1usize << best_k;
            n += (1usize << best_k) as i32;
        } else {
            // exact step, identical to perturb_point
            let (zr, zi) = unsafe { *orbit.get_unchecked(m) };
            let new_dr = 2.0 * (zr * dr - zi * di) + (dr * dr - di * di) + dcx;
            let new_di = 2.0 * (zr * di + zi * dr) + 2.0 * dr * di + dcy;
            dr = new_dr;
            di = new_di;
            m += 1;
            n += 1;
        }

        // escape / rebase checks at the new position (count n-1, as perturb_point)
        let (zmr, zmi) = unsafe { *orbit.get_unchecked(m) };
        let wr = zmr + dr;
        let wi = zmi + di;
        let w2 = wr * wr + wi * wi;
        if w2 >= ESCAPE_R2 {
            return n - 1;
        }
        d2 = dr * dr + di * di;
        if w2 < d2 || m == last {
            dr = wr;
            di = wi;
            d2 = w2;
            m = 0;
        }
    }
    -1
}

// bound |dc| over a rows x columns block and run the BLA pixel loop for it
fn bla_block(
    orbit: &[(f64, f64)],
    dx_f: f64,
    dy_f: f64,
    dcx0: f64,
    row_ref: usize,
    rows: usize,
    columns: usize,
    max_iterations: i32,
    out: &mut [i32],
) {
    let x1 = dcx0 + columns.saturating_sub(1) as f64 * dx_f;
    let mx = dcx0.abs().max(x1.abs());
    let y1 = (row_ref as f64 - (rows as f64 - 1.0)) * dy_f;
    let my = (row_ref as f64 * dy_f).abs().max(y1.abs());
    let dc_max = (mx * mx + my * my).sqrt();
    let bla = build_bla_table(orbit, dc_max);

    for i in 0..rows {
        let dcy = (row_ref as f64 - i as f64) * dy_f;
        for j in 0..columns {
            let dcx = dcx0 + j as f64 * dx_f;
            out[i * columns + j] = perturb_point_bla(orbit, &bla, dcx, dcy, max_iterations);
        }
    }
}

/// Orbit-sharing strip grinder. Compute image rows `[strip_row0, strip_row0 +
/// strip_rows)` against a reference orbit that was built ONCE for the whole
/// image (reference at the image center, i.e. `perturb_setup(.., image_rows, ..)`
/// so `image_row_ref = image_rows/2`, `dcx0 = -(columns/2)*dx_f`). The orbit is
/// shared read-only; only the CHEAP f64 BLA table is (re)built here, and it is
/// bounded by THIS strip's `dc_max` so the skips stay long -- that is the whole
/// point: pay the expensive HP orbit once, keep per-strip tables tight.
///
/// `dcy_off` shifts every sample by a constant in dc.y: 0.0 for the reference's
/// own grid (bit-identical to `bla_block` on the whole image, since x + 0.0 == x),
/// `0.5*dy_f` for the second pass's half-pixel-shifted grid -- the reference
/// point is not a grid point, so one orbit serves both passes. A matching x
/// shift is folded into `dcx0` by the caller.
/// `out` holds `strip_rows * columns` i32, row-major.
#[allow(clippy::too_many_arguments)]
pub fn bla_strip(
    orbit: &[(f64, f64)],
    dx_f: f64,
    dy_f: f64,
    dcx0: f64,
    dcy_off: f64,
    image_row_ref: usize,
    strip_row0: usize,
    strip_rows: usize,
    columns: usize,
    max_iterations: i32,
    out: &mut [i32],
) {
    let x1 = dcx0 + columns.saturating_sub(1) as f64 * dx_f;
    let mx = dcx0.abs().max(x1.abs());
    // |dcy| over this strip's image rows: extremes are the top and bottom rows.
    let dcy_top = (image_row_ref as f64 - strip_row0 as f64) * dy_f + dcy_off;
    let dcy_bot = (image_row_ref as f64 - (strip_row0 + strip_rows.saturating_sub(1)) as f64) * dy_f + dcy_off;
    let my = dcy_top.abs().max(dcy_bot.abs());
    let dc_max = (mx * mx + my * my).sqrt();
    let bla = build_bla_table(orbit, dc_max);

    for i in 0..strip_rows {
        let img_row = strip_row0 + i;
        let dcy = (image_row_ref as f64 - img_row as f64) * dy_f + dcy_off;
        for j in 0..columns {
            let dcx = dcx0 + j as f64 * dx_f;
            out[i * columns + j] = perturb_point_bla(orbit, &bla, dcx, dcy, max_iterations);
        }
    }
}

// *** shared-index variant (SIMD-friendly) *** //
//
// Unlike perturb_point above, this does NOT rebase per pixel: every pixel walks
// the reference orbit with the same index n (reading Z_n), so a whole SIMD vector
// of pixels shares one broadcast reference read instead of a per-lane gather.
//
// The price is that glitches (catastrophic cancellation, or a pixel outliving the
// reference orbit) are no longer prevented locally; they are *detected* here with
// the Pauldelbrot criterion and corrected by the caller with a fresh reference
// (see mandelbrot_perturb_glitch*). A larger TAU flags more pixels as glitched
// (safer, more re-reference work); smaller flags fewer (faster, riskier).
const GLITCH_TAU: f64 = 1e-3;

// safety cap on re-reference passes; leftover pixels fall back to brute-force HP
const MAX_GLITCH_PASSES: usize = 32;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PtResult {
    Escaped(i32), // crate-standard iteration count
    Interior,     // never escaped within max_iterations
    Glitched(i32), // lost precision / outlived the reference at this iteration
}

/// One pixel by perturbation against a shared reference index (no rebasing).
/// Returns Escaped/Interior, or Glitched(n) when the delta can no longer be
/// trusted (the caller must recompute this pixel against a nearer reference).
#[inline]
pub fn perturb_point_shared(orbit: &[(f64, f64)], dcx: f64, dcy: f64, max_iterations: i32) -> PtResult {
    let ref_len = orbit.len();
    let mut dr = 0.0f64;
    let mut di = 0.0f64;
    for n in 0..max_iterations {
        let nu = n as usize;
        if nu + 1 >= ref_len {
            return PtResult::Glitched(n); // outlived the stored reference orbit
        }
        let (zr, zi) = unsafe { *orbit.get_unchecked(nu) };
        let ndr = 2.0 * (zr * dr - zi * di) + (dr * dr - di * di) + dcx;
        let ndi = 2.0 * (zr * di + zi * dr) + 2.0 * dr * di + dcy;
        dr = ndr;
        di = ndi;

        let (z1r, z1i) = unsafe { *orbit.get_unchecked(nu + 1) };
        let wr = z1r + dr;
        let wi = z1i + di;
        let w2 = wr * wr + wi * wi;
        if w2 >= ESCAPE_R2 {
            return PtResult::Escaped(n);
        }
        // Pauldelbrot: true value shrank far below the reference -> delta unreliable
        let zref2 = z1r * z1r + z1i * z1i;
        if w2 < GLITCH_TAU * zref2 {
            return PtResult::Glitched(n);
        }
    }
    PtResult::Interior
}

/// Process LANES pixels together against a shared reference index. Every lane
/// advances in lock-step reading the same Z_n (a broadcast, not a per-lane
/// gather); finished lanes are frozen by a mask rather than an early exit. This
/// is the branchless structure the SIMD kernels vectorize, and the portable
/// scalar fallback on targets without SIMD. Results match calling
/// perturb_point_shared on each lane individually.
pub fn perturb_lanes_shared<const LANES: usize>(
    orbit: &[(f64, f64)],
    dcx: &[f64; LANES],
    dcy: &[f64; LANES],
    max_iterations: i32,
) -> [PtResult; LANES] {
    let ref_len = orbit.len();
    let mut dr = [0.0f64; LANES];
    let mut di = [0.0f64; LANES];
    let mut done = [false; LANES];
    let mut res = [PtResult::Interior; LANES];

    for n in 0..max_iterations {
        let nu = n as usize;
        if nu + 1 >= ref_len {
            for l in 0..LANES {
                if !done[l] {
                    res[l] = PtResult::Glitched(n);
                }
            }
            break;
        }
        let (zr, zi) = unsafe { *orbit.get_unchecked(nu) };
        let (z1r, z1i) = unsafe { *orbit.get_unchecked(nu + 1) };
        let zref2 = z1r * z1r + z1i * z1i;

        let mut all_done = true;
        for l in 0..LANES {
            // frozen lanes: keep their delta, contribute nothing new
            let active = !done[l];
            let ndr = 2.0 * (zr * dr[l] - zi * di[l]) + (dr[l] * dr[l] - di[l] * di[l]) + dcx[l];
            let ndi = 2.0 * (zr * di[l] + zi * dr[l]) + 2.0 * dr[l] * di[l] + dcy[l];
            if active {
                dr[l] = ndr;
                di[l] = ndi;
            }
            let wr = z1r + dr[l];
            let wi = z1i + di[l];
            let w2 = wr * wr + wi * wi;
            if active {
                if w2 >= ESCAPE_R2 {
                    res[l] = PtResult::Escaped(n);
                    done[l] = true;
                } else if w2 < GLITCH_TAU * zref2 {
                    res[l] = PtResult::Glitched(n);
                    done[l] = true;
                } else {
                    all_done = false;
                }
            }
        }
        if all_done {
            break;
        }
    }
    res
}

// Cross-pixel NEON f64x2 kernel: two pixels per iteration, planar layout
// (dr/di each hold both pixels' real/imag parts), reference values broadcast.
// Same algorithm as perturb_lanes_shared::<2>; finished lanes are frozen by a
// select mask. NEON is baseline on aarch64, so the intrinsics need no runtime
// detection. Bit-identical to the scalar path (verified in tests).
#[cfg(target_arch = "aarch64")]
pub fn perturb_pair_shared_neon(
    orbit: &[(f64, f64)],
    dcx: &[f64; 2],
    dcy: &[f64; 2],
    max_iterations: i32,
) -> [PtResult; 2] {
    use core::arch::aarch64::*;
    let ref_len = orbit.len();
    unsafe {
        let dcx_v = vld1q_f64(dcx.as_ptr());
        let dcy_v = vld1q_f64(dcy.as_ptr());
        let two = vdupq_n_f64(2.0);
        let escape = vdupq_n_f64(ESCAPE_R2);

        let mut dr = vdupq_n_f64(0.0);
        let mut di = vdupq_n_f64(0.0);
        let mut active = vdupq_n_u64(!0u64); // per-lane: still iterating
        let mut esc_mask = vdupq_n_u64(0);
        let mut glitch_mask = vdupq_n_u64(0);
        let mut count = vdupq_n_s64(0);

        for n in 0..max_iterations {
            let nu = n as usize;
            let n_v = vdupq_n_s64(n as i64);
            if nu + 1 >= ref_len {
                // still-active lanes outlived the reference orbit -> glitch here
                count = vbslq_s64(active, n_v, count);
                glitch_mask = vorrq_u64(glitch_mask, active);
                break;
            }
            let (zr, zi) = *orbit.get_unchecked(nu);
            let (z1r, z1i) = *orbit.get_unchecked(nu + 1);
            let zr_v = vdupq_n_f64(zr);
            let zi_v = vdupq_n_f64(zi);
            let z1r_v = vdupq_n_f64(z1r);
            let z1i_v = vdupq_n_f64(z1i);
            let tau_zref2 = vdupq_n_f64(GLITCH_TAU * (z1r * z1r + z1i * z1i));

            // ndr = 2*(zr*dr - zi*di) + (dr*dr - di*di) + dcx
            let t1 = vsubq_f64(vmulq_f64(zr_v, dr), vmulq_f64(zi_v, di));
            let t2 = vsubq_f64(vmulq_f64(dr, dr), vmulq_f64(di, di));
            let ndr = vaddq_f64(vaddq_f64(vmulq_f64(two, t1), t2), dcx_v);
            // ndi = 2*(zr*di + zi*dr + dr*di) + dcy
            let u1 = vaddq_f64(vmulq_f64(zr_v, di), vmulq_f64(zi_v, dr));
            let u2 = vmulq_f64(dr, di);
            let ndi = vaddq_f64(vmulq_f64(two, vaddq_f64(u1, u2)), dcy_v);

            // freeze finished lanes (active ? new : old)
            dr = vbslq_f64(active, ndr, dr);
            di = vbslq_f64(active, ndi, di);

            let wr = vaddq_f64(z1r_v, dr);
            let wi = vaddq_f64(z1i_v, di);
            let w2 = vaddq_f64(vmulq_f64(wr, wr), vmulq_f64(wi, wi));

            let esc_now = vcgeq_f64(w2, escape);
            let gl_now = vcltq_f64(w2, tau_zref2);
            let new_esc = vandq_u64(active, esc_now);
            let new_gl = vandq_u64(active, gl_now); // esc and gl are mutually exclusive
            let new_finish = vorrq_u64(new_esc, new_gl);

            count = vbslq_s64(new_finish, n_v, count);
            esc_mask = vorrq_u64(esc_mask, new_esc);
            glitch_mask = vorrq_u64(glitch_mask, new_gl);
            active = vbicq_u64(active, new_finish); // active &= ~new_finish

            // both lanes finished?
            if vmaxvq_u32(vreinterpretq_u32_u64(active)) == 0 {
                break;
            }
        }

        let decode = |esc: u64, gl: u64, c: i64| {
            if esc != 0 {
                PtResult::Escaped(c as i32)
            } else if gl != 0 {
                PtResult::Glitched(c as i32)
            } else {
                PtResult::Interior
            }
        };
        [
            decode(vgetq_lane_u64::<0>(esc_mask), vgetq_lane_u64::<0>(glitch_mask), vgetq_lane_s64::<0>(count)),
            decode(vgetq_lane_u64::<1>(esc_mask), vgetq_lane_u64::<1>(glitch_mask), vgetq_lane_s64::<1>(count)),
        ]
    }
}

// Cross-pixel wasm SIMD128 f64x2 kernel: the direct port of the NEON kernel
// above (identical algorithm, same 2-wide planar layout), for the browser.
// simd128 is scoped to this function via #[target_feature] as in the simd32
// module. Callers must ensure the deploy engine supports SIMD128 (all major
// browsers since 2021); the mb-wasm build requires it.
#[cfg(all(target_arch = "wasm32", feature = "simd128"))]
#[target_feature(enable = "simd128")]
pub unsafe fn perturb_pair_shared_wasm(
    orbit: &[(f64, f64)],
    dcx: &[f64; 2],
    dcy: &[f64; 2],
    max_iterations: i32,
) -> [PtResult; 2] {
    use core::arch::wasm32::*;
    let ref_len = orbit.len();
    let dcx_v = f64x2(dcx[0], dcx[1]);
    let dcy_v = f64x2(dcy[0], dcy[1]);
    let two = f64x2_splat(2.0);
    let escape = f64x2_splat(ESCAPE_R2);

    let mut dr = f64x2_splat(0.0);
    let mut di = f64x2_splat(0.0);
    let mut active = u64x2_splat(!0u64);
    let mut esc_mask = u64x2_splat(0);
    let mut glitch_mask = u64x2_splat(0);
    let mut count = i64x2_splat(0);

    for n in 0..max_iterations {
        let nu = n as usize;
        let n_v = i64x2_splat(n as i64);
        if nu + 1 >= ref_len {
            count = v128_bitselect(n_v, count, active);
            glitch_mask = v128_or(glitch_mask, active);
            break;
        }
        let (zr, zi) = *orbit.get_unchecked(nu);
        let (z1r, z1i) = *orbit.get_unchecked(nu + 1);
        let zr_v = f64x2_splat(zr);
        let zi_v = f64x2_splat(zi);
        let z1r_v = f64x2_splat(z1r);
        let z1i_v = f64x2_splat(z1i);
        let tau_zref2 = f64x2_splat(GLITCH_TAU * (z1r * z1r + z1i * z1i));

        let t1 = f64x2_sub(f64x2_mul(zr_v, dr), f64x2_mul(zi_v, di));
        let t2 = f64x2_sub(f64x2_mul(dr, dr), f64x2_mul(di, di));
        let ndr = f64x2_add(f64x2_add(f64x2_mul(two, t1), t2), dcx_v);
        let u1 = f64x2_add(f64x2_mul(zr_v, di), f64x2_mul(zi_v, dr));
        let u2 = f64x2_mul(dr, di);
        let ndi = f64x2_add(f64x2_mul(two, f64x2_add(u1, u2)), dcy_v);

        dr = v128_bitselect(ndr, dr, active); // active ? new : old
        di = v128_bitselect(ndi, di, active);

        let wr = f64x2_add(z1r_v, dr);
        let wi = f64x2_add(z1i_v, di);
        let w2 = f64x2_add(f64x2_mul(wr, wr), f64x2_mul(wi, wi));

        let esc_now = f64x2_ge(w2, escape);
        let gl_now = f64x2_lt(w2, tau_zref2);
        let new_esc = v128_and(active, esc_now);
        let new_gl = v128_and(active, gl_now);
        let new_finish = v128_or(new_esc, new_gl);

        count = v128_bitselect(n_v, count, new_finish);
        esc_mask = v128_or(esc_mask, new_esc);
        glitch_mask = v128_or(glitch_mask, new_gl);
        active = v128_andnot(active, new_finish); // active &= ~new_finish

        if !v128_any_true(active) {
            break;
        }
    }

    let decode = |esc: u64, gl: u64, c: i64| {
        if esc != 0 {
            PtResult::Escaped(c as i32)
        } else if gl != 0 {
            PtResult::Glitched(c as i32)
        } else {
            PtResult::Interior
        }
    };
    [
        decode(u64x2_extract_lane::<0>(esc_mask), u64x2_extract_lane::<0>(glitch_mask), i64x2_extract_lane::<0>(count)),
        decode(u64x2_extract_lane::<1>(esc_mask), u64x2_extract_lane::<1>(glitch_mask), i64x2_extract_lane::<1>(count)),
    ]
}

/// Process a pixel pair. The real win is the branchless 2-lane lock-step
/// structure below, which the compiler/JIT already parallelizes via ILP across
/// the two independent lanes. Benchmarks showed the explicit SIMD kernels
/// (perturb_pair_shared_{neon,wasm}) are actually *slower* than this scalar path
/// — the mask/select overhead outweighs the lanes ILP already provides (native
/// NEON ~0.78x, wasm SIMD128 ~0.69x on a warm JIT). So we use scalar here; the
/// SIMD kernels are kept only for reference / possible future mixed-view tuning.
#[inline]
pub fn perturb_pair_shared(orbit: &[(f64, f64)], dcx: &[f64; 2], dcy: &[f64; 2], max_iterations: i32) -> [PtResult; 2] {
    perturb_lanes_shared::<2>(orbit, dcx, dcy, max_iterations)
}

// Generate the width-specific reference/setup/conversion code for one limb type.
macro_rules! perturb_engine {
    ($limb:ty,
     $negate:ident, $add:ident, $sub:ident, $sq:ident, $multiply:ident, $incr:ident,
     $hpdata:ident, $count_iterations:ident,
     $mag_to_f64:ident, $limbs_to_f64_scratch:ident, $limbs_to_f64:ident,
     $reference_orbit:ident, $perturb_setup:ident, $mandelbrot_perturb:ident,
     $mandelbrot_perturb_glitch:ident) => {

/// Convert a magnitude (non-negative limb value: limb0 = integral part, limbs 1..
/// = fractional limbs, most significant first) to f64.
///
/// Scales ONCE from the first nonzero limb rather than walking a running scale
/// down limb by limb: the old loop's running scale underflowed to 0.0 partway
/// down (at limb weight 2^-1088 for u64 limbs), silently dropping every limb
/// past it -- values below 2^-1024 converted to 0.0 (the server rendered deep
/// views as one flat color) and values just above the cliff kept only their top
/// few bits (26% error at 2^-1023.4). The u32 engine had the same flaw ~2^32
/// deeper (browser views degraded to ~16-bit precision below ~1e-294). Three
/// limbs from the first nonzero give >= 96 bits of mantissa, and the two-step
/// scaling stays correct through f64's subnormal range to the true ~2^-1074
/// floor.
#[inline]
fn $mag_to_f64(m: &[$limb]) -> f64 {
    const B: i32 = <$limb>::BITS as i32;
    let k = match m.iter().position(|&l| l != 0) {
        Some(k) => k,
        None => return 0.0,
    };
    let step = 2.0f64.powi(-B);
    let mut mant = m[k] as f64;
    let mut lo = step;
    let end = core::cmp::min(k + 3, m.len());
    for i in (k + 1)..end {
        mant += m[i] as f64 * lo;
        lo *= step;
    }
    let e = -B * (k as i32);
    if e >= -1022 {
        mant * 2.0f64.powi(e)
    } else {
        // split the scaling so the intermediate stays normal; the final multiply
        // correctly rounds into (or below) the subnormal range
        (mant * 2.0f64.powi(-1022)) * 2.0f64.powi(e + 1022)
    }
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
/// The stored orbit is CAPPED at 4M points regardless of max_iterations: memory
/// is 16 bytes/point and the per-strip BLA table another ~80, so an uncapped
/// orbit at huge maxIterations would be a multi-GB allocation (wasm32 dies at
/// 4GB). The UI caps maxIterations at 4M so this is normally not binding and
/// every UI-reachable view computes with a full-coverage orbit.
///
/// KNOWN LIMIT for max_iterations > 4M (reachable only via raw API requests):
/// when the cap truncates a NON-escaped reference, pixels that outlive it wrap
/// (m == last -> rebase to index 0) with an order-1 delta, and at deep zoom the
/// per-pixel dc (e.g. 1e-244) is annihilated against that delta's f64 ulp --
/// adjacent pixels collapse onto one shared trajectory and return IDENTICAL
/// counts (measured: a 289-digit view at maxIter 5e8 returned 2001017 for every
/// center pixel = cap + 1017; renders as a flat blob). Wrapping is only sound
/// for ESCAPED references (pixels die soon after; validated vs the exact
/// engine) and for interior pixels (reference re-converges; wraps measured
/// free). Deep counts beyond the cap need an orbit that covers them: runtime
/// orbit budget (browser: budget/workers, wasm32 ceiling ~150M pts; server:
/// one shared orbit, RAM-bound, threads free) + truncation-returns-black --
/// see CLAUDE.md next steps.
///
/// `cx` / `cy` are the reference coordinate as limbs, each the same length.
pub fn $reference_orbit(cx: &[$limb], cy: &[$limb], max_iterations: i32) -> Vec<(f64, f64)> {
    const MAX_REF_ORBIT_POINTS: i32 = 4_000_000;
    let max_iterations = max_iterations.min(MAX_REF_ORBIT_POINTS);
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

/// Compute a `rows` x `columns` block by shared-index perturbation with glitch
/// correction. Each pass runs every outstanding pixel against one reference orbit
/// (all pixels share the reference index, so this loop is SIMD-ready); pixels the
/// Pauldelbrot criterion flags as glitched are retried next pass against a nearer
/// reference (the longest-surviving glitched pixel, i.e. the most in-set one).
/// Any stragglers after MAX_GLITCH_PASSES are computed exactly by brute-force HP.
///
/// Serial reference implementation used for validation; the server/wasm drivers
/// parallelize the per-pixel loop within a pass.
pub fn $mandelbrot_perturb_glitch(
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
    let dx_f = $limbs_to_f64(&dx[..chunks]);
    let dy_f = $limbs_to_f64(&dy[..chunks]);
    let mut dy_neg = vec![0 as $limb; chunks];
    $negate(&dy[..chunks], &mut dy_neg);

    // coordinate (as `chunks` limbs) of pixel (r, c): xmin + c*dx, ymax - r*dy
    let coord_of = |r: usize, c: usize| -> (Vec<$limb>, Vec<$limb>) {
        let mut cx = xmin[..chunks].to_vec();
        for _ in 0..c { $incr(&mut cx, &dx[..chunks]); }
        let mut cy = ymax[..chunks].to_vec();
        for _ in 0..r { $incr(&mut cy, &dy_neg); }
        (cx, cy)
    };

    let mut todo: Vec<usize> = (0..rows * columns).collect();
    let (mut ref_r, mut ref_c) = (rows / 2, columns / 2);

    for _pass in 0..MAX_GLITCH_PASSES {
        if todo.is_empty() {
            break;
        }
        let (cx, cy) = coord_of(ref_r, ref_c);
        let orbit = $reference_orbit(&cx, &cy, max_iterations);

        // dc offset (from the reference) of pixel p
        let (rr, rc) = (ref_r, ref_c);
        let dc = |p: usize| -> (f64, f64) {
            let r = p / columns;
            let c = p % columns;
            ((c as f64 - rc as f64) * dx_f, (rr as f64 - r as f64) * dy_f)
        };

        // run pixels through the 4-lane kernel (tail: scalar). 4 independent
        // delta orbits per loop give wide cores enough ILP to fill their FP
        // pipes; benched faster than (or equal to) 2 lanes on every core type.
        let mut results: Vec<(usize, PtResult)> = Vec::with_capacity(todo.len());
        let mut k = 0;
        while k + 3 < todo.len() {
            let mut dcx = [0f64; 4];
            let mut dcy = [0f64; 4];
            for i in 0..4 {
                let (x, y) = dc(todo[k + i]);
                dcx[i] = x;
                dcy[i] = y;
            }
            let r = perturb_lanes_shared::<4>(&orbit, &dcx, &dcy, max_iterations);
            for i in 0..4 {
                results.push((todo[k + i], r[i]));
            }
            k += 4;
        }
        while k < todo.len() {
            let p = todo[k];
            let (ax, ay) = dc(p);
            results.push((p, perturb_point_shared(&orbit, ax, ay, max_iterations)));
            k += 1;
        }

        let mut glitched: Vec<usize> = Vec::new();
        let mut best_survived = -1i32;
        let mut best_pixel = todo[0];
        for (p, r) in results {
            match r {
                PtResult::Escaped(cnt) => out[p] = cnt,
                PtResult::Interior => out[p] = -1,
                PtResult::Glitched(surv) => {
                    if surv > best_survived {
                        best_survived = surv;
                        best_pixel = p;
                    }
                    glitched.push(p);
                }
            }
        }
        todo = glitched;
        ref_r = best_pixel / columns;
        ref_c = best_pixel % columns;
    }

    // stragglers (should be rare / none): compute exactly with brute-force HP
    if !todo.is_empty() {
        let mut hp = $hpdata::new(chunks);
        for &p in &todo {
            let (cx, cy) = coord_of(p / columns, p % columns);
            out[p] = $count_iterations(&mut hp, &cx[..chunks], &cy[..chunks], max_iterations);
        }
    }
}

    };
}

perturb_engine!(u64,
    negate64, add64, sub64, sq64, multiply64, incr64,
    HPData64, count_iterations_hp64,
    mag_to_f64_64, limbs_to_f64_scratch_64, limbs64_to_f64,
    reference_orbit64, perturb_setup64, mandelbrot_perturb64, mandelbrot_perturb_glitch64);

perturb_engine!(u32,
    negate32, add32, sub32, sq32, multiply32, incr32,
    HPData32, count_iterations_hp32,
    mag_to_f64_32, limbs_to_f64_scratch_32, limbs32_to_f64,
    reference_orbit32, perturb_setup32, mandelbrot_perturb32, mandelbrot_perturb_glitch32);

/// Compute a block by BLA-accelerated perturbation (rebasing engine + skip
/// table), reference at the block center. Same signature/semantics as
/// mandelbrot_perturb64; the BLA table is built once per reference orbit.
pub fn mandelbrot_perturb_bla64(
    xmin: &[u64], dx: &[u64], ymax: &[u64], dy: &[u64],
    chunks: usize, rows: usize, columns: usize,
    max_iterations: i32, out: &mut [i32],
) {
    let (orbit, dx_f, dy_f, dcx0, _col_ref, row_ref) =
        perturb_setup64(xmin, dx, ymax, dy, chunks, rows, columns, max_iterations);
    bla_block(&orbit, dx_f, dy_f, dcx0, row_ref, rows, columns, max_iterations, out);
}

/// u32-limb variant of mandelbrot_perturb_bla64 (the delta loop and BLA table
/// are width-agnostic f64; only the reference orbit differs).
pub fn mandelbrot_perturb_bla32(
    xmin: &[u32], dx: &[u32], ymax: &[u32], dy: &[u32],
    chunks: usize, rows: usize, columns: usize,
    max_iterations: i32, out: &mut [i32],
) {
    let (orbit, dx_f, dy_f, dcx0, _col_ref, row_ref) =
        perturb_setup32(xmin, dx, ymax, dy, chunks, rows, columns, max_iterations);
    bla_block(&orbit, dx_f, dy_f, dcx0, row_ref, rows, columns, max_iterations, out);
}

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
    #[derive(Clone, Copy)]
    enum Eng {
        Rebase,
        Glitch,
        Bla,
    }

    fn deep_view_body(use32: bool, engine: Eng) {
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
            match engine {
                Eng::Glitch => mandelbrot_perturb_glitch32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
                Eng::Rebase => mandelbrot_perturb32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
                Eng::Bla => mandelbrot_perturb_bla32(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
            }
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
            match engine {
                Eng::Glitch => mandelbrot_perturb_glitch64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
                Eng::Rebase => mandelbrot_perturb64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
                Eng::Bla => mandelbrot_perturb_bla64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert),
            }
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
        deep_view_body(false, Eng::Rebase);
    }

    #[test]
    fn deep_view_matches_brute_u32() {
        deep_view_body(true, Eng::Rebase);
    }

    #[test]
    fn glitch_deep_view_matches_brute_u64() {
        deep_view_body(false, Eng::Glitch);
    }

    #[test]
    fn glitch_deep_view_matches_brute_u32() {
        deep_view_body(true, Eng::Glitch);
    }

    #[test]
    fn bla_deep_view_matches_brute_u64() {
        deep_view_body(false, Eng::Bla);
    }

    #[test]
    fn bla_deep_view_matches_brute_u32() {
        deep_view_body(true, Eng::Bla);
    }

    #[test]
    fn bla_interior_all_neg1() {
        // interior window: every pixel must run to max_iter through the skip path
        let xd = coord(0, &[0, 0, 0, 0]);
        let yd = coord(0, &[0, 0, 0, 0]);
        let dxd = coord(0, &[0, 0, 1, 0]); // 2^-48
        let dyd = coord(0, &[0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (xmin, dx, ymax, dy) =
            (u32_to_limbs64(&xd), u32_to_limbs64(&dxd), u32_to_limbs64(&yd), u32_to_limbs64(&dyd));
        let (rows, columns, max_iter) = (24, 24, 500);

        let mut pert = vec![0i32; rows * columns];
        mandelbrot_perturb_bla64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
        for &v in &pert {
            assert_eq!(v, -1, "interior pixel should never escape");
        }
    }

    #[test]
    fn bla_exterior_fast_escape_exact() {
        // fast escapes have large |d| almost immediately: BLA must not skip there
        // and must match brute exactly, like the plain rebasing engine
        let xd = coord(0, &[0x8000, 0, 0, 0]); // 0.5
        let yd = coord(0, &[0x8000, 0, 0, 0]); // 0.5
        let dxd = coord(0, &[0, 0, 1, 0]); // 2^-48
        let dyd = coord(0, &[0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (xmin, dx, ymax, dy) =
            (u32_to_limbs64(&xd), u32_to_limbs64(&dxd), u32_to_limbs64(&yd), u32_to_limbs64(&dyd));
        let (rows, columns, max_iter) = (24, 24, 500);

        let mut pert = vec![0i32; rows * columns];
        mandelbrot_perturb_bla64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
        let brute = brute_image64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);
        for i in 0..rows {
            for j in 0..columns {
                assert_eq!(pert[i * columns + j], brute[i][j], "mismatch at ({i},{j})");
            }
        }
    }

    #[test]
    fn lanes_match_per_pixel() {
        // the lane-batch kernel (the SIMD template) must give identical results to
        // calling perturb_point_shared on each pixel individually
        let xd = coord(0, &[0, 0, 0, 0, 0]);
        let yd = coord(1, &[0, 0, 0, 0, 0]);
        let dxd = coord(0, &[0, 0, 0, 1, 0]);
        let dyd = coord(0, &[0, 0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (dx, dy) = (u32_to_limbs64(&dxd), u32_to_limbs64(&dyd));
        let mut xmin = u32_to_limbs64(&xd);
        let mut dx_neg = vec![0u64; xmin.len()];
        negate64(&dx, &mut dx_neg);
        let (rows, columns, max_iter) = (48, 48, 8000);
        for _ in 0..columns / 2 { incr64(&mut xmin, &dx_neg); }
        let mut ymax = u32_to_limbs64(&yd);
        for _ in 0..rows / 2 { incr64(&mut ymax, &dy); }

        let (orbit, dx_f, dy_f, dcx0, _cr, row_ref) =
            perturb_setup64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);

        // collect all pixels' (dcx, dcy)
        let mut dcxs = Vec::new();
        let mut dcys = Vec::new();
        for i in 0..rows {
            let dcy = (row_ref as f64 - i as f64) * dy_f;
            for j in 0..columns {
                dcxs.push(dcx0 + j as f64 * dx_f);
                dcys.push(dcy);
            }
        }

        // LANES = 2 and 4 must both match the per-pixel result
        let per_pixel: Vec<PtResult> = (0..dcxs.len())
            .map(|p| perturb_point_shared(&orbit, dcxs[p], dcys[p], max_iter))
            .collect();

        for base in (0..dcxs.len()).step_by(2) {
            let dcx = [dcxs[base], dcxs[base + 1]];
            let dcy = [dcys[base], dcys[base + 1]];
            let r = perturb_lanes_shared::<2>(&orbit, &dcx, &dcy, max_iter);
            assert_eq!(r[0], per_pixel[base], "lane2 pixel {base}");
            assert_eq!(r[1], per_pixel[base + 1], "lane2 pixel {}", base + 1);
        }
        for base in (0..dcxs.len()).step_by(4) {
            let dcx = [dcxs[base], dcxs[base + 1], dcxs[base + 2], dcxs[base + 3]];
            let dcy = [dcys[base], dcys[base + 1], dcys[base + 2], dcys[base + 3]];
            let r = perturb_lanes_shared::<4>(&orbit, &dcx, &dcy, max_iter);
            for l in 0..4 {
                assert_eq!(r[l], per_pixel[base + l], "lane4 pixel {}", base + l);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn neon_matches_scalar() {
        // the NEON f64x2 kernel must be bit-identical to the scalar per-pixel path
        let xd = coord(0, &[0, 0, 0, 0, 0]);
        let yd = coord(1, &[0, 0, 0, 0, 0]);
        let dxd = coord(0, &[0, 0, 0, 1, 0]);
        let dyd = coord(0, &[0, 0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (dx, dy) = (u32_to_limbs64(&dxd), u32_to_limbs64(&dyd));
        let mut xmin = u32_to_limbs64(&xd);
        let mut dx_neg = vec![0u64; xmin.len()];
        negate64(&dx, &mut dx_neg);
        let (rows, columns, max_iter) = (48, 48, 8000);
        for _ in 0..columns / 2 { incr64(&mut xmin, &dx_neg); }
        let mut ymax = u32_to_limbs64(&yd);
        for _ in 0..rows / 2 { incr64(&mut ymax, &dy); }

        let (orbit, dx_f, dy_f, dcx0, _cr, row_ref) =
            perturb_setup64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);

        for i in 0..rows {
            let dcy = (row_ref as f64 - i as f64) * dy_f;
            for j in (0..columns).step_by(2) {
                let dcx = [dcx0 + j as f64 * dx_f, dcx0 + (j + 1) as f64 * dx_f];
                let dcys = [dcy, dcy];
                let neon = perturb_pair_shared_neon(&orbit, &dcx, &dcys, max_iter);
                let s0 = perturb_point_shared(&orbit, dcx[0], dcys[0], max_iter);
                let s1 = perturb_point_shared(&orbit, dcx[1], dcys[1], max_iter);
                assert_eq!(neon[0], s0, "neon lane0 at ({i},{j})");
                assert_eq!(neon[1], s1, "neon lane1 at ({i},{})", j + 1);
            }
        }
    }

    #[test]
    fn glitch_interior_all_neg1() {
        let xd = coord(0, &[0, 0, 0, 0]);
        let yd = coord(0, &[0, 0, 0, 0]);
        let dxd = coord(0, &[0, 0, 1, 0]);
        let dyd = coord(0, &[0, 0, 1, 0]);
        let chunks = chunks64_for(xd.len());
        let (xmin, dx, ymax, dy) =
            (u32_to_limbs64(&xd), u32_to_limbs64(&dxd), u32_to_limbs64(&yd), u32_to_limbs64(&dyd));
        let (rows, columns, max_iter) = (24, 24, 500);
        let mut pert = vec![0i32; rows * columns];
        mandelbrot_perturb_glitch64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter, &mut pert);
        for &v in &pert {
            assert_eq!(v, -1, "interior pixel should never escape");
        }
    }
}
