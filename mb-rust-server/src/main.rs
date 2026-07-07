/*
    Backend Mandelbrot web server in Rust
    By Bill Wood, Jan/Feb 2023
*/

// *** web server *** //
use actix_rt::System;
use actix_web::{web, App, HttpResponse, HttpServer, HttpRequest, Result};
use actix_files::NamedFile;
use serde::{Deserialize};
use std::path::PathBuf;
use std::process::exit;

use std::env;
static mut IMAGE_QUALITY: usize = 1;
static mut NUM_THREADS: usize = 2;
// 0 = full-width u64 limb engine (default); 32/64/128 = legacy half-limb engines
static mut U_TYPE: usize = 0;
// perturbation engine: one HP reference orbit per request, f64 deltas per pixel
static mut PERTURB: bool = false;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut url = String::from("localhost:8000");
    let help = r#"Run the Rust Mandelbrot server

Usage: mb-rust [OPTIONS] [args]

Arguments:
  URL            URL to serve Mandelbrot on; defaults to localhost:8000

Options:
  -h, --help     Show this help message and exit
  -r, --rayon    Number of Rayon threads for each Javascript "worker"; only affects high precision images;
                 defaults to 2
  -q, --quality  Set image quality from 2 (best) to 0 (worst); only affects high precision images;
                 lower quality may be faster in certain situations; defaults to 1
  --u32          Use legacy 32 bit engine for high precision calculations (slowest)
  --u64          Use legacy 64 bit engine for high precision calculations
  --u128         Use legacy 128 bit engine for high precision calculations
                 default is a full-width 64 bit limb engine, faster than all of the above
  --perturb      Use perturbation theory for high precision images: one full-precision
                 reference orbit per request, cheap f64 deltas per pixel. Much faster at
                 deep zoom. f64 deltas are usable down to ~1e-300 pixel scale."#;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--quality" => {
                if i + 1 < args.len() {
                    i += 1;
                    let quality = args[i].parse::<usize>().unwrap();
                    if quality > 2 {
                        println!("quality must be a number between 0 and 2!");
                        exit(1);
                    }
                    unsafe { IMAGE_QUALITY = 2 - quality };
                } else {
                    println!("missing value for --quality!");
                    exit(1);
                }
            }
            "-r" | "--rayon" => {
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
            "--u32" => unsafe {
                U_TYPE = 32;
            }
            "--u64" => unsafe {
                U_TYPE = 64;
            }
            "--u128" => unsafe {
                U_TYPE = 128;
            }
            "--perturb" => unsafe {
                PERTURB = true;
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

    println!("Mandelbrot server running on URL {url} with {} Rayon thread(s), image quality {}, and the {} engine for high precision calculations.",
        unsafe { NUM_THREADS },
        unsafe { 2 - IMAGE_QUALITY },
        if unsafe { PERTURB } {
            "perturbation (full-width u64 reference)".to_string()
        } else {
            match unsafe { U_TYPE } {
                0 => "full-width u64".to_string(),
                u => format!("legacy u{u}"),
            }
        },
    );
    web_server(&url);
}

async fn file(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    Ok(NamedFile::open(path)?)
}

async fn redirect() -> Result<HttpResponse> {
    Ok(HttpResponse::MovedPermanently().append_header(("Location", "/MB.html")).finish())
}

async fn ping() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().into())
}

fn web_server(url: &str) {
    let sys = System::new();
    let server = HttpServer::new(|| {
        App::new()
            .route("/mb-compute", web::post().to(compute_mandelbrot))
            .route("/mb-computeHP", web::post().to(compute_mandelbrot_hp))
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

    // ignoring the last u32 chunk seems to be a small speed optimization which reduces precision but doesn't affect image quality
    let u32_chunks = mandelbrot_coords_hp.xmin.len() - unsafe { IMAGE_QUALITY };

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

// perturbation engine: one full-precision reference orbit at the block center,
// then cheap f64 delta orbits per pixel (with rebasing to stay glitch-free).
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

    // one high-precision reference orbit for the whole block; deltas are f64
    let (orbit, dx_f, dy_f, dcx0, _col_ref, row_ref) =
        perturb_setup64(&xmin, &dx, &ymax, &dy, chunks, rows, columns, max_iter);

    // parallelize the (embarrassingly parallel) per-pixel delta orbits across rows
    let slice_size = core::cmp::max(1, rows/num_threads);
    let row_indices: Vec<usize> = (0..rows).collect();
    row_indices
        .par_chunks(slice_size)
        .map(| idxs | {
            idxs.iter().map(| &i | {
                let dcy = (row_ref as f64 - i as f64) * dy_f;
                let mut row = vec![0i32; columns];
                perturb_row(&orbit, dcx0, dx_f, dcy, columns, max_iter, &mut row);
                row
            }).collect::<Vec<Vec<i32>>>()
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
    // println!("{} {} {}", u32_chunks - 1 + unsafe { IMAGE_QUALITY }, u32_chunks - 1, chunks - 1 );

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
