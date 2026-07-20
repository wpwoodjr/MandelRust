/*
    Backend Mandelbrot web server in Rust
    By Bill Wood, Jan/Feb 2023
*/

// *** web server *** //
use actix_rt::System;
use actix_web::{web, App, HttpResponse, HttpServer, HttpRequest, Responder, Result};
use actix_files::NamedFile;
use serde::{Deserialize};
use std::path::PathBuf;
use std::process::exit;

use std::env;
static mut NUM_THREADS: usize = 2;
// 0 = full-width u64 limb engine (default); 32/64/128 = legacy half-limb engines
static mut U_TYPE: usize = 0;
// perturbation engine (default): one HP reference orbit per band of rows,
// f64 deltas per pixel
static mut PERTURB: bool = true;
// reference-orbit cache budget for /mb-computeHP2 (--orbit-cache, in MB)
static mut ORBIT_CACHE_BYTES: usize = 128 * 1024 * 1024;
// log one line per HP2 request (--verbose)
static mut VERBOSE: bool = false;
// reference-orbit point budget (--orbit-points, in millions of points).
// 16 bytes/point: the deep-iterations lever -- maxIterations above the budget
// resolves pixels up to it; the rest return count -2, rendered white by the
// client (see reference_orbit).
static mut ORBIT_BUDGET_POINTS: i32 = mb_arith::DEFAULT_ORBIT_BUDGET;

fn orbit_budget() -> i32 {
    unsafe { ORBIT_BUDGET_POINTS }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut url = String::from("localhost:8000");
    let help = r#"Run the Rust Mandelbrot server

Usage: mb-rust [OPTIONS] [args]

Arguments:
  URL              URL to serve Mandelbrot on; defaults to localhost:8000

Options:
  -h, --help       Show this help message and exit
  -v, --verbose    Log one line per high-precision request: grid, basis,
                   offsets, orbit cache hit/miss, and cache occupancy
  --orbit-cache N  Reference-orbit cache budget in MB (default 128). Orbits are
                   cached by view coordinates so a repeated view -- notably the
                   second pass of a two-pass render -- skips the orbit build.
                   The newest orbit is always cached, so the real bound is
                   max(N, largest single orbit). 0 disables caching.
  --orbit-points N Reference-orbit point budget in MILLIONS (default 4).
                   16 bytes/point of RAM while a request is in flight (plus
                   cache retention): 100 = 1.6 GB, 500 = 8 GB. maxIterations
                   above the budget renders the pixels that outlive the orbit
                   white (unresolved) instead of wrong -- raise this to resolve
                   them.

Legacy options (apply only to the old per-job /mb-computeHP endpoint, used by
old clients; the current client sends one streaming /mb-computeHP2 request per
image and picks its own thread count):
  -r, --rayon      Rayon threads per /mb-computeHP request; defaults to 2
  --no-perturb     Disable the perturbation engine and compute every pixel at
                   full precision with the full-width 64 bit limb engine
                   (slower, exact)
  --u32            Use legacy 32 bit engine for high precision (slowest)
  --u64            Use legacy 64 bit engine for high precision
  --u128           Use legacy 128 bit engine for high precision
                   these also disable the perturbation engine

  High precision uses perturbation theory with BLA acceleration: one
  full-precision reference orbit per image (shared across strips, cached across
  requests), cheap f64 deltas per pixel, and a composed-skip table that skips
  most iterations at deep zoom. f64 deltas are usable down to ~1e-300 pixel
  scale."#;

    let mut i = 1;
    let mut legacy_configured = false;
    while i < args.len() {
        match args[i].as_str() {
            "-r" | "--rayon" => {
                legacy_configured = true;
                if i + 1 < args.len() {
                    i += 1;
                    unsafe { NUM_THREADS = args[i].parse().unwrap() };
                    if unsafe { NUM_THREADS } == 0 {
                        println!("number of Rayon threads must be > 0!");
                        exit(1);
                    }
                } else {
                    println!("missing value for --rayon!");
                    exit(1);
                }
            }
            "--orbit-cache" => {
                if i + 1 < args.len() {
                    i += 1;
                    match args[i].parse::<usize>() {
                        Ok(mb) => unsafe { ORBIT_CACHE_BYTES = mb * 1024 * 1024 },
                        Err(_) => {
                            println!("--orbit-cache expects a size in MB!");
                            exit(1);
                        }
                    }
                } else {
                    println!("missing value for --orbit-cache!");
                    exit(1);
                }
            }
            "--orbit-points" => {
                if i + 1 < args.len() {
                    i += 1;
                    match args[i].parse::<i32>() {
                        Ok(m) if m >= 1 => unsafe { ORBIT_BUDGET_POINTS = m.saturating_mul(1_000_000) },
                        _ => {
                            println!("--orbit-points expects a positive size in millions of points!");
                            exit(1);
                        }
                    }
                } else {
                    println!("missing value for --orbit-points!");
                    exit(1);
                }
            }
            "-v" | "--verbose" => unsafe {
                VERBOSE = true;
            }
            "--u32" => unsafe {
                legacy_configured = true;
                U_TYPE = 32;
                PERTURB = false;
            }
            "--u64" => unsafe {
                legacy_configured = true;
                U_TYPE = 64;
                PERTURB = false;
            }
            "--u128" => unsafe {
                legacy_configured = true;
                U_TYPE = 128;
                PERTURB = false;
            }
            "--perturb" => unsafe {
                legacy_configured = true;
                PERTURB = true;
            }
            "--no-perturb" => unsafe {
                legacy_configured = true;
                PERTURB = false;
            }
            "-h" | "--help" => {
                println!("{help}");
                exit(0);
            }
            arg => {
                url = arg.to_string();
            }
        }
        i += 1;
    }

    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    println!("Mandelbrot server running on URL {url}");
    println!("  high precision: perturbation + BLA, shared reference orbit, streamed strips");
    println!("                  ({cores} cores available; thread count chosen per request by the client)");
    println!("  orbit cache:    {} MB (--orbit-cache to change; 0 disables)",
        unsafe { ORBIT_CACHE_BYTES } / (1024*1024));
    if legacy_configured { println!("  legacy /mb-computeHP: {} Rayon thread(s) per request, {} engine",
        unsafe { NUM_THREADS },
        if unsafe { PERTURB } {
            "perturbation + BLA (full-width u64 reference)".to_string()
        } else {
            match unsafe { U_TYPE } {
                0 => "full-width u64".to_string(),
                u => format!("legacy u{u}"),
            }
        },
    ); }
    web_server(&url);
}

async fn file(req: HttpRequest) -> Result<HttpResponse> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    let file = NamedFile::open(path)?;
    // Match what GitHub Pages serves, so the local server behaves like the live
    // demo. Sending NO Cache-Control would not be equivalent: the browser would
    // then invent a heuristic freshness window (~10% of the file's age, unbounded),
    // which is how a 2023-era worker once outlived the wasm it drives.
    // Subresources carry ?v=<content hash> (see stamp-assets.sh), so 10 minutes of
    // staleness cannot pair a new worker with an old binary. MB.html cannot be
    // versioned -- it carries the hash -- so hard-reload (ctrl-shift-R) before
    // trusting a browser benchmark.
    Ok(file
        .customize()
        .insert_header(("cache-control", "max-age=600"))
        .respond_to(&req)
        .map_into_boxed_body())
}

async fn redirect() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently().append_header(("Location", "/MB.html")).finish())
}

async fn ping() -> Result<HttpResponse> {
    // advertise the orbit point budget so the client can flag budget-limited
    // renders in remote mode (old clients ignore the header)
    Ok(HttpResponse::Ok()
        .insert_header(("X-MB-Orbit-Points", orbit_budget().to_string()))
        .finish())
}

fn web_server(url: &str) {
    let sys = System::new();
    let server = HttpServer::new(|| {
        App::new()
            .route("/mb-compute", web::post().to(compute_mandelbrot))
            .route("/mb-computeHP", web::post().to(compute_mandelbrot_hp))
            .route("/mb-computeHP2", web::post().to(compute_mandelbrot_hp2))
            .route("/remoteCanComputeMB", web::get().to(ping))
            .route("/", web::get().to(redirect))
            .route("/{filename:.*}", web::get().to(file))
    })
    .bind(url)
    .unwrap();

    sys.block_on(server.run()).unwrap();
}


use mb_arith::*;

// *** low precision *** //
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct MandelbrotCoords {
    columns: usize,
    firstRow: usize,
    rows: usize,
    xmin: f64,
    dx: f64,
    ymax: f64,
    dy: f64,
    maxIterations: i32,
}

async fn compute_mandelbrot(mandelbrot_coords: web::Json<MandelbrotCoords>) -> HttpResponse {
    let xmin = mandelbrot_coords.xmin;
    let dx = mandelbrot_coords.dx;
    let columns = mandelbrot_coords.columns;
    let ymax = mandelbrot_coords.ymax;
    let dy = mandelbrot_coords.dy;
    let first_row = mandelbrot_coords.firstRow;
    let rows = mandelbrot_coords.rows;
    let max_iterations = mandelbrot_coords.maxIterations;

    let mut iteration_counts = vec![vec![0; columns]; rows];
    for i in 0..rows {
        let y = ymax - (first_row + i) as f64*dy;
        for j in 0..columns {
            iteration_counts[i][j] = count_iterations(xmin + j as f64*dx, y, max_iterations);
        }
    }

    HttpResponse::Ok().json(iteration_counts)
}


// *** high precision *** //
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct MandelbrotCoordsHP {
    columns: usize,
    // firstRow: usize,
    rows: usize,
    xmin: Vec<u32>,
    dx: Vec<u32>,
    ymax: Vec<u32>,
    dy: Vec<u32>,
    maxIterations: i32,
}

/*
exports.computeMandelbrotHP = function(mandelbrotCoords) {
    return new Promise(function(resolve, reject) {
        let xmin = new Uint32Array(mandelbrotCoords.xmin);
        let dx = new Uint32Array(mandelbrotCoords.dx);
        let columns = mandelbrotCoords.columns;
        let ymax = new Uint32Array(mandelbrotCoords.ymax);
        let maxIterations = mandelbrotCoords.maxIterations;

        //console.log(xmin, dx, columns, ymax, maxIterations, ArrayType);
        let iterationCounts = new Array(columns);
        createHPData(xmin, dx, columns);
        for (let i = 0; i < columns; i++) {
            iterationCounts[i] = countIterationsHP(xs[i], ymax, maxIterations);
        }

        resolve([iterationCounts]);
    });
};
*/
async fn compute_mandelbrot_hp(mandelbrot_coords_hp: web::Json<MandelbrotCoordsHP>) -> HttpResponse {

    // use the full coordinate precision the client sent
    let u32_chunks = mandelbrot_coords_hp.xmin.len();

    if unsafe { PERTURB } {
        let iteration_counts = compute_mandelbrot_perturb64(&mandelbrot_coords_hp, u32_chunks, unsafe { NUM_THREADS });
        return HttpResponse::Ok().json(iteration_counts);
    }

    let iteration_counts = match unsafe { U_TYPE } {
        0 => {
            compute_mandelbrot_hp64(&mandelbrot_coords_hp, u32_chunks, unsafe { NUM_THREADS })
        }
        32 => {
            let xmin = u32_to_t::<u32>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u32>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u32>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u32>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks, unsafe { NUM_THREADS })
        }
        64 => {
            let xmin = u32_to_t::<u64>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u64>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u64>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u64>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks, unsafe { NUM_THREADS })
        }
        128 => {
            let xmin = u32_to_t::<u128>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u128>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u128>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u128>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks, unsafe { NUM_THREADS })
        }
        _ => panic!("illegal size!")
    };
    HttpResponse::Ok().json(iteration_counts)
}


// *** high precision v2: whole image per request, orbit shared, streamed *** //
//
// xmin/dx/ymax/dy are the BASIS grid coordinates -- the grid the reference
// orbit is built on. Normally that's the request's own sampling grid; for a
// second pass the client sends PASS 1's coords here plus the half-pixel (ox,
// oy) offset of its sampling grid, so both passes key to (and reuse) one
// cached orbit. basisRows/basisColumns are the basis grid's dimensions when
// they differ from the sampling grid's (pass 2 is one row/column bigger);
// 0 = same as rows/columns. All new fields default so old-format requests
// (plain pass-1 renders) parse unchanged.
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct MandelbrotCoordsHP2 {
    columns: usize,
    rows: usize,
    xmin: Vec<u32>,
    dx: Vec<u32>,
    ymax: Vec<u32>,
    dy: Vec<u32>,
    maxIterations: i32,
    threads: usize,
    #[serde(default)]
    basisRows: usize,
    #[serde(default)]
    basisColumns: usize,
    #[serde(default)]
    ox: f64,
    #[serde(default)]
    oy: f64,
}

// Reference-orbit cache: a byte-budgeted LRU keyed by the basis coordinates
// (which fully determine the orbit, so entries can never be stale). Entries are
// Arc'd so eviction can't free an orbit an in-flight request is still grinding
// against: eviction drops the cache's reference, the request keeps its own, and
// the memory dies with the last holder. The newest orbit is always admitted
// (it was just built for a live request and pass 2 is the most predictable
// upcoming request), so the true memory bound is max(budget, largest orbit).
struct CachedOrbit {
    orbit: Vec<(f64, f64)>,
    // FloatExp side table for orbit points below f64's floor (deep minibrot
    // nuclei); without it, fe-depth interior pixels falsely escape
    dips: Vec<OrbitDip>,
    // FloatExp scale meta: pixel scales below f64's ~1e-308 floor (360+ digit
    // views) are unrepresentable as plain f64; bla_strip_fe dispatches back to
    // the f64 engine when the scale allows (bit-identical to the old path)
    dx_fe: FloatExp,
    dy_fe: FloatExp,
    dcx0: FloatExp,
    row_ref: usize,
}

type OrbitKey = (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>, usize, usize, i32, i32);

// most-recently-used last; sizes tracked per entry
static ORBIT_CACHE: std::sync::Mutex<Vec<(OrbitKey, std::sync::Arc<CachedOrbit>, usize)>> =
    std::sync::Mutex::new(Vec::new());

fn orbit_cache_lookup(key: &OrbitKey) -> Option<std::sync::Arc<CachedOrbit>> {
    let mut cache = ORBIT_CACHE.lock().unwrap();
    if let Some(pos) = cache.iter().position(|(k, _, _)| k == key) {
        let entry = cache.remove(pos);
        let arc = entry.1.clone();
        cache.push(entry);   // move to most-recently-used
        Some(arc)
    } else {
        None
    }
}

fn orbit_cache_insert(key: OrbitKey, orbit: std::sync::Arc<CachedOrbit>) {
    let budget = unsafe { ORBIT_CACHE_BYTES };
    if budget == 0 {
        return;
    }
    let bytes = orbit.orbit.len() * std::mem::size_of::<(f64, f64)>();
    let mut cache = ORBIT_CACHE.lock().unwrap();
    cache.retain(|(k, _, _)| k != &key);   // replace, don't duplicate
    let mut total: usize = cache.iter().map(|(_, _, b)| b).sum();
    // evict least-recently-used until the newcomer fits; always admit it
    while !cache.is_empty() && total + bytes > budget {
        total -= cache.remove(0).2;
    }
    cache.push((key, orbit, bytes));
}

// One request = one whole image (or one pass). The reference orbit is built ONCE
// (image-center reference, same scheme as the browser's orbit sharing), then
// small strips are computed in parallel on a per-request rayon pool sized by the
// client's `threads` (clamped to the machine). Each strip is streamed back the
// moment it finishes as one NDJSON line -- out of order; the client paints by
// firstRow -- so the display stays progressive with no batch barrier. If the
// client disconnects, the next strip's send fails and remaining strips
// early-out. This replaces the old 32-row-jobs protocol for HP, which rebuilt
// the orbit per band and idled through per-request round trips; legacy flags
// (--no-perturb, --u32/64/128) do not apply here.
// Reference selection: a center reference that hasn't escaped by the cap
// would otherwise build to maxIterations -- hours at extreme depths when a
// minibrot sits under the crosshair. Instead, probe PROBE_ROWS evenly-spaced
// full-width rows against the truncated prefix and relocate the reference to
// the longest-lived ESCAPING pixel found (escaped references wrap soundly at
// any length -- the reverted reference-selection experiment proved short
// escaped refs run at least as fast as full-coverage ones). The cap starts at
// SELECT_TRIGGER_POINTS and escalates x4 up to the budget for views whose
// every pixel outlives the first prefix (whole-frame-deep KF locations).
// MIN_CAND_POINTS rejects runt references whose tiny BLA skip ladder
// (max skip ~ orbit length) would slow every pixel.
const SELECT_TRIGGER_POINTS: i32 = 4_000_000;
const PROBE_ROWS: usize = 16;
const MIN_CAND_POINTS: i32 = 100_000;

async fn compute_mandelbrot_hp2(coords: web::Json<MandelbrotCoordsHP2>) -> HttpResponse {
    let coords = coords.into_inner();
    let (tx, rx) = tokio::sync::mpsc::channel::<std::result::Result<web::Bytes, std::convert::Infallible>>(64);

    std::thread::spawn(move || {
        let rows = coords.rows;
        let columns = coords.columns;
        let max_iter = coords.maxIterations;
        // the grid the orbit is built on; defaults to the sampling grid
        let basis_rows = if coords.basisRows > 0 { coords.basisRows } else { rows };
        let basis_columns = if coords.basisColumns > 0 { coords.basisColumns } else { columns };

        let max_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let threads = coords.threads.clamp(1, max_threads);
        // Builds are decoupled from strips, so strips are sized for balance alone
        // (same reasoning as the browser's adaptive formula).
        let strip = rows.div_ceil(4*threads).clamp(4, 32);

        let pool = match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
            Ok(p) => p,
            Err(_) => return,
        };

        let budget = orbit_budget();
        let key: OrbitKey = (coords.xmin.clone(), coords.dx.clone(), coords.ymax.clone(),
            coords.dy.clone(), basis_rows, basis_columns, max_iter, budget);
        let (cached, hit) = match orbit_cache_lookup(&key) {
            Some(c) => (c, true),
            None => {
                let u32_chunks = coords.xmin.len();
                let chunks = 1 + (u32_chunks - 1 + 3)/4;
                let xmin = u32_to_limbs64(&coords.xmin);
                let dx = u32_to_limbs64(&coords.dx);
                let ymax = u32_to_limbs64(&coords.ymax);
                let dy = u32_to_limbs64(&coords.dy);

                // Build control hook: cancel when the client disconnects, and
                // in verbose mode log progress -- deep builds are multi-minute
                // and were silent. Runs once per 65536-point batch: free.
                // DISCONNECT DETECTION NEEDS THE HEARTBEAT: actix only notices
                // a dead connection when it WRITES, and nothing is written
                // during the build -- an idle stream left is_closed() false
                // forever (measured: a killed client's build thread stayed at
                // 100% CPU). The empty NDJSON line is skipped by every client
                // (worker: `if (line.length == 0) continue`); try_send never
                // blocks (a full channel just skips a beat).
                let progress_last = std::sync::atomic::AtomicUsize::new(0);
                let ctl_fn = |pts: usize| -> bool {
                    if unsafe { VERBOSE } {
                        let last = progress_last.load(std::sync::atomic::Ordering::Relaxed);
                        if pts >= last + 10_000_000 {
                            progress_last.store(pts, std::sync::atomic::Ordering::Relaxed);
                            println!("  orbit build: {} M pts...", pts / 1_000_000);
                        }
                    }
                    let _ = tx.try_send(Ok(web::Bytes::from_static(b"\n")));
                    !tx.is_closed()
                };
                let ctl: OrbitBuildCtl = Some(&ctl_fn);

                // Center build with escalating reference selection. Round N
                // EXTENDS the center reference to t points (the builder keeps
                // the full-precision z alive, so each round appends -- no
                // prefix is ever recomputed); a center that escapes (or runs
                // the full maxIterations) within the cap is a finished
                // reference -- the common case, zero overhead. A cap-truncated
                // center means every pixel would outlive it, so probe the
                // frame against the truncated prefix: any probe that escapes
                // within it yields its TRUE count (escapes and rebases are
                // exact; only the end-of-orbit wrap is not, and that comes
                // back negative), and ANY escaping pixel works as the
                // reference (escaped orbits wrap soundly at any length). No
                // escaper yet: quadruple the cap and extend, up to the budget.
                let dx_fe = limbs64_to_fe(&dx[..chunks]);
                let dy_fe = limbs64_to_fe(&dy[..chunks]);
                let center_row = basis_rows / 2;
                let center_dcx0 = dx_fe.mul_f64(-((basis_columns / 2) as f64));
                let mut builder = OrbitBuilder64::at_grid(
                    &xmin, &dx, &ymax, &dy, chunks, center_row, basis_columns / 2);
                let mut t = budget.min(SELECT_TRIGGER_POINTS);
                let c = loop {
                    builder.extend(max_iter.min(t.max(2)), ctl);
                    if tx.is_closed() {
                        return; // cancelled: discard the partial orbit, never cache it
                    }
                    let len = builder.orbit.len() as i32 - 1;
                    if orbit_escaped(&builder.orbit) || len >= max_iter {
                        // finished reference: escaped, or full-length interior
                        break CachedOrbit { orbit: std::mem::take(&mut builder.orbit),
                            dips: std::mem::take(&mut builder.dips),
                            dx_fe, dy_fe, dcx0: center_dcx0, row_ref: center_row };
                    }

                    let best = {
                        let table = bla_image_table(&builder.orbit, dx_fe, dy_fe,
                            center_dcx0, FE_ZERO, center_row, basis_rows, basis_columns);
                        let mut probe_rows: Vec<usize> = (0..PROBE_ROWS)
                            .map(|i| i * basis_rows.saturating_sub(1) / (PROBE_ROWS - 1).max(1))
                            .collect();
                        probe_rows.dedup();
                        pool.install(|| {
                            probe_rows.par_iter().map(|&r| {
                                let mut out = vec![0i32; basis_columns];
                                bla_strip_fe_with_table(&builder.orbit, &builder.dips, &table,
                                    dx_fe, dy_fe, center_dcx0, FE_ZERO, center_row,
                                    r, 1, basis_columns, len, &mut out);
                                out.iter().enumerate()
                                    .map(|(col, &ct)| (ct, r, col))
                                    .max()
                                    .unwrap_or((-1, r, 0))
                            }).max()
                        }).filter(|&(ct, _, _)| ct >= MIN_CAND_POINTS)
                    };
                    if tx.is_closed() {
                        return;
                    }
                    if let Some((count, pr, pc)) = best {
                        if unsafe { VERBOSE } {
                            println!("  center reference unresolved at {} pts: relocating to px ({}, {}), escapes at {}",
                                len, pc, pr, count);
                        }
                        // free the center prefix before the candidate build:
                        // a deep rescue holds two near-budget orbits otherwise
                        // (256M center + 256M candidate = ~8 GB peak)
                        drop(std::mem::take(&mut builder.orbit));
                        drop(std::mem::take(&mut builder.dips));
                        let (orbit, dips, dx_fe, dy_fe, dcx0, _c2, row_ref) = perturb_setup_fe64_at(
                            &xmin, &dx, &ymax, &dy, chunks, pr, pc, max_iter, budget, ctl);
                        if tx.is_closed() {
                            return;
                        }
                        break CachedOrbit { orbit, dips, dx_fe, dy_fe, dcx0, row_ref };
                    }
                    if t >= budget {
                        // no pixel escapes within the whole budget: no rescue
                        // exists; the truncated center is the best sound
                        // answer (outliving pixels come back -2, white)
                        break CachedOrbit { orbit: std::mem::take(&mut builder.orbit),
                            dips: std::mem::take(&mut builder.dips),
                            dx_fe, dy_fe, dcx0: center_dcx0, row_ref: center_row };
                    }
                    t = t.saturating_mul(4).min(budget);
                    if unsafe { VERBOSE } {
                        println!("  no escaping probe within {} pts: extending center build to {} M pts",
                            len, t / 1_000_000);
                    }
                };
                let c = std::sync::Arc::new(c);
                orbit_cache_insert(key, c.clone());
                (c, false)
            }
        };
        if unsafe { VERBOSE } {
            // one line per HP2 request: enough to see at a glance whether the
            // client is sending basis+offsets and whether the cache is hitting
            let cache = ORBIT_CACHE.lock().unwrap();
            let total_mb = cache.iter().map(|(_, _, b)| b).sum::<usize>() as f64 / (1024.0*1024.0);
            println!("HP2 {}x{} basis {}x{} off ({},{}) threads {}: orbit {} ({} pts) | cache {} entries, {:.0} MB",
                columns, rows, basis_columns, basis_rows, coords.ox, coords.oy, threads,
                if hit { "cache HIT" } else { "built" }, cached.orbit.len(),
                cache.len(), total_mb);
        }
        // this request's sampling grid, offset (ox, oy) pixels from the basis
        // (folded in FloatExp: at fe depths the f64 fold would underflow)
        let dcx0_eff = cached.dcx0.add(cached.dx_fe.mul_f64(coords.ox));
        let dcy_off = cached.dy_fe.mul_f64(coords.oy);

        // Big orbits share ONE whole-image BLA table across the request's
        // threads: per-strip tables (smaller dc_max, slightly longer skips)
        // multiply memory by the thread count -- ~475 MB each at a 128M-point
        // orbit OOM-killed a 32-thread run. Below the threshold, per-strip
        // tables keep today's behavior bit for bit.
        const SHARED_TABLE_MIN_ORBIT: usize = 8_000_000;
        let shared_table = if cached.orbit.len() > SHARED_TABLE_MIN_ORBIT {
            Some(bla_image_table(&cached.orbit, cached.dx_fe, cached.dy_fe,
                dcx0_eff, dcy_off, cached.row_ref, rows, columns))
        } else {
            None
        };

        let dead = std::sync::atomic::AtomicBool::new(false);
        pool.install(|| {
            let starts: Vec<usize> = (0..rows).step_by(strip).collect();
            starts.par_iter().for_each(|&r0| {
                if dead.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let h = strip.min(rows - r0);
                let mut out = vec![0i32; h*columns];
                match &shared_table {
                    Some(t) => bla_strip_fe_with_table(&cached.orbit, &cached.dips, t,
                        cached.dx_fe, cached.dy_fe, dcx0_eff, dcy_off,
                        cached.row_ref, r0, h, columns, max_iter, &mut out),
                    None => bla_strip_fe(&cached.orbit, &cached.dips, cached.dx_fe, cached.dy_fe,
                        dcx0_eff, dcy_off, cached.row_ref, r0, h, columns, max_iter, &mut out),
                }
                let counts: Vec<&[i32]> = out.chunks(columns).collect();
                let line = format!(
                    "{{\"firstRow\":{},\"nrows\":{},\"iterationCounts\":{}}}\n",
                    r0, h, serde_json::to_string(&counts).unwrap()
                );
                if tx.blocking_send(Ok(web::Bytes::from(line))).is_err() {
                    dead.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            });
        });
    });

    HttpResponse::Ok()
        .content_type("application/x-ndjson")
        .streaming(tokio_stream::wrappers::ReceiverStream::new(rx))
}

use std::ops::{ BitAnd, BitAndAssign, Shl, Shr, AddAssign, Sub };
use num::traits::{ Zero, One, AsPrimitive };
use core::cmp::PartialEq;
use core::mem::size_of;
use rayon::prelude::*;

// full-width u64 limb engine (default)
fn compute_mandelbrot_hp64(coords: &MandelbrotCoordsHP, u32_chunks: usize, num_threads: usize) -> Vec<Vec<i32>> {
    let xmin = u32_to_limbs64(&coords.xmin);
    let dx = u32_to_limbs64(&coords.dx);
    let yval = u32_to_limbs64(&coords.ymax);
    let dy = u32_to_limbs64(&coords.dy);
    let rows = coords.rows;
    let columns = coords.columns;
    let max_iter = coords.maxIterations;

    // chunks: 1 for the integral part, plus however many u64 limbs are needed for the fractional part
    let chunks = 1 + (u32_chunks - 1 + 3)/4;

    let mut dy_neg = vec![0u64; dy.len()];
    negate64(&dy, &mut dy_neg);
    let mut y = yval;
    let mut y_vals = Vec::with_capacity(rows);
    for _ in 0..rows {
        y_vals.push(y.clone());
        incr64(&mut y, &dy_neg);
    }

    let slice_size = core::cmp::max(1, rows/num_threads);
    y_vals
        .par_chunks(slice_size)
        .map(| y_vals | {
            let mut iteration_counts = vec![vec![0; columns]; y_vals.len()];
            for i in 0..y_vals.len() {
                mandelbrot_row_hp64(&xmin, &dx, &y_vals[i], chunks, columns, max_iter, &mut iteration_counts[i]);
            }
            iteration_counts
        })
        .flatten()
        .collect()
}

// perturbation engine: BLA (rebasing + composed-skip table, see mb-arith).
// The request's rows are split into one contiguous band per Rayon thread; each
// band computes its own full-precision reference orbit (band center), builds the
// BLA table for it once, and runs cheap f64 delta orbits per pixel that skip
// most iterations at deep zoom.
fn compute_mandelbrot_perturb64(coords: &MandelbrotCoordsHP, u32_chunks: usize, num_threads: usize) -> Vec<Vec<i32>> {
    let xmin = u32_to_limbs64(&coords.xmin);
    let dx = u32_to_limbs64(&coords.dx);
    let ymax = u32_to_limbs64(&coords.ymax);
    let dy = u32_to_limbs64(&coords.dy);
    let rows = coords.rows;
    let columns = coords.columns;
    let max_iter = coords.maxIterations;

    // chunks: 1 for the integral part, plus however many u64 limbs the fraction needs
    let chunks = 1 + (u32_chunks - 1 + 3)/4;

    let band = core::cmp::max(1, rows.div_ceil(num_threads));
    let mut dy_neg = vec![0u64; dy.len()];
    negate64(&dy, &mut dy_neg);
    let mut bands = Vec::new();
    let mut y = ymax;
    let mut r0 = 0;
    while r0 < rows {
        let h = core::cmp::min(band, rows - r0);
        bands.push((y.clone(), h));
        for _ in 0..h {
            incr64(&mut y, &dy_neg);
        }
        r0 += h;
    }

    bands
        .par_iter()
        .map(| (band_ymax, h) | {
            let mut out = vec![0i32; h * columns];
            mandelbrot_perturb_bla64(&xmin, &dx, band_ymax, &dy, chunks, *h, columns, max_iter, &mut out);
            out.chunks(columns).map(| r | r.to_vec()).collect::<Vec<Vec<i32>>>()
        })
        .flatten()
        .collect()
}

pub fn compute_mandelbrot_hp_t<T>(xmin: &[T], dx: &[T], yval: &[T], dy: &[T], rows: usize, columns: usize, max_iter: i32, u32_chunks: usize, num_threads: usize) -> Vec<Vec<i32>>
where T: Sync + Zero + Copy,
    // add, sq, multiply, negate, incr, count_iterations requirements
    T: One + AddAssign + BitAndAssign + Sub<Output = T> + PartialEq +
        BitAnd + Shr<usize, Output = T> + Shl<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    T: std::fmt::LowerHex,
{
    // chunks: 1 for the integral part, plus however many T elements are needed for the fractional part
    let chunks = 1 + {
        let t_to_u32_size_ratio = size_of::<T>()/size_of::<u32>();
        (u32_chunks - 1 + t_to_u32_size_ratio - 1)/t_to_u32_size_ratio
    };

    let mut dy_neg = vec![T::zero(); xmin.len()];
    negate(&dy, &mut dy_neg);
    let mut y_vals = vec![vec![T::zero(); xmin.len()]; rows];
    y_vals[0] = yval.to_vec();
    for i in 1..rows {
        (0..xmin.len()).for_each(| j | y_vals[i][j] = y_vals[i - 1][j]);
        incr(&mut y_vals[i], &dy_neg);
    }

    let slice_size = core::cmp::max(1, rows/num_threads);
    y_vals
        .par_chunks(slice_size)
        .map(| y_vals | {
            let mut x_val = xmin.to_vec();
            let rows = y_vals.len();
            let mut hp_data = HPData::new(chunks);
            let mut iteration_counts = vec![vec![0; columns]; rows];
            for i in 0..rows {
                for j in 0..columns {
                    iteration_counts[i][j] = count_iterations_hp(&mut hp_data, &x_val[0..chunks], &y_vals[i][0..chunks], max_iter);
                    incr(&mut x_val, &dx);
                }
                x_val.copy_from_slice(xmin);
            }
            iteration_counts
        })
        .flatten()
        .collect()
}
