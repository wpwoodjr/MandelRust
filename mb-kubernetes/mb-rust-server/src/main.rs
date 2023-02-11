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
static mut U_TYPE: usize = 128;

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
  --u32          Use 32 bit unsigned integers for high precision calculations (slowest)
  --u64          Use 64 bit unsigned integers for high precision calculations
  --u128         Use 128 bit unsigned integers for high precision calculations (default)"#;

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

    println!("Mandelbrot server running on URL {url} with {} Rayon thread(s), image quality {}, and {} bit unsigned integers for high precision calculations.",
        unsafe { NUM_THREADS },
        unsafe { 2 - IMAGE_QUALITY },
        unsafe { U_TYPE },
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
        for j in 0..columns {
            iteration_counts[i][j] = count_iterations(xmin + j as f64*dx, ymax - (first_row + i) as f64*dy, max_iterations);
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

    let iteration_counts = match unsafe { U_TYPE } {
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
