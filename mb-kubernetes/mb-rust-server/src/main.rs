use actix_rt::System;
use actix_web::{web, App, HttpResponse, HttpServer, HttpRequest, Result};
use actix_files::NamedFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::exit;

use std::env;
static mut IMAGE_QUALITY: usize = 1;
static mut NUM_SLICES: usize = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut url = String::from("localhost:8000");
    let help = r#"Run the Rust Mandelbrot server

Usage: mb-rust [OPTIONS] [args]

Arguments:
  URL            URL to serve Mandelbrot on; defaults to localhost:8000

Options:
  -h, --help     Show this help message and exit
  -r, --rayon    Number of slices for Rayon to parallelize over on each Javascript "worker";
                 only affects high precision images; defaults to 2
  -q, --quality  Set image quality from 0 (best) to 5 (worst); only affects high precision images;
                 defaults to 1"#;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--quality" => {
                if i + 1 < args.len() {
                    i += 1;
                    unsafe { IMAGE_QUALITY = args[i].parse().unwrap() };
                } else {
                    println!("missing value for --quality!");
                    exit(1);
                }
            }
            "-r" | "--rayon" => {
                if i + 1 < args.len() {
                    i += 1;
                    unsafe { NUM_SLICES = args[i].parse().unwrap() };
                    if unsafe { NUM_SLICES } == 0 {
                        println!("number of Rayon slices must be > 0!");
                        exit(1);
                    }
                } else {
                    println!("missing value for --rayon!");
                    exit(1);
                }
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

    println!("Mandelbrot server running on URL {url} with image quality {} and {} Rayon parallel slice(s)",
        unsafe { IMAGE_QUALITY },
        unsafe { NUM_SLICES },
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

fn web_server(url: &str) {
    let sys = System::new();
    let server = HttpServer::new(|| {
        App::new()
            // .route("/static", web::get().to(static_html))
            .route("/mb-compute", web::post().to(compute_mandelbrot))
            .route("/mb-computeHP", web::post().to(compute_mandelbrot_hp))
            .route("/", web::get().to(redirect))
            .route("/{filename:.*}", web::get().to(file))
    })
    .bind(url)
    .unwrap();

    sys.block_on(server.run()).unwrap();
}

#[derive(Deserialize, Serialize, Debug)]
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

fn count_iterations(x: f64, y: f64, max_iterations: i32) -> i32 {
    let mut count = 0;
    let mut zx = x;
    let mut zy = y;

    while count < max_iterations && zx*zx + zy*zy < 8.0 {
        let new_zx = zx*zx - zy*zy + x;
        zy = 2.0*zx*zy + y;
        zx = new_zx;
        count += 1;
    }

    if count < max_iterations {
        count
    } else {
        -1
    }
}

// high precision
#[derive(Deserialize, Serialize, Debug)]
#[allow(non_snake_case)]
struct MandelbrotCoordsHP {
    columns: usize,
    // firstRow: usize,
    // rows: usize,
    xmin: Vec<u32>,
    dx: Vec<u32>,
    ymax: Vec<u32>,
    // dy: Vec<u32>,
    maxIterations: i32,
}

struct HPData {
    work1: Vec<u32>,
    work2: Vec<u32>,
    work3: Vec<u32>,
    work4: Vec<u32>,
    zx: Vec<u32>,
    zy: Vec<u32>,
}

impl HPData {
    fn new(chunks: usize) -> HPData {
        HPData {
            work1: vec![0; chunks],
            work2: vec![0; chunks],
            work3: vec![0; chunks],
            work4: vec![0; chunks],
            zx: vec![0; chunks],
            zy: vec![0; chunks],
        }
    }
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
    let iteration_counts =
        compute_mandelbrot_hp_rayon(&mandelbrot_coords_hp.xmin, &mandelbrot_coords_hp.dx, &mandelbrot_coords_hp.ymax, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations);
    HttpResponse::Ok().json(vec![iteration_counts; 1])
}

use rayon::prelude::*;
fn compute_mandelbrot_hp_rayon(xmin: &[u32], dx: &[u32], y: &[u32], columns: usize, max_iter: i32) -> Vec<i32> {
    let num_size = xmin.len();

    // ignoring the last u32 chunk seems to be a small speed optimization which reduces precision but doesn't affect image quality
    let chunks = num_size - unsafe { IMAGE_QUALITY };

    let num_slices = unsafe { NUM_SLICES };
    let slice_size = columns/num_slices;

    let mut x_vals = vec![vec![0; num_size]; columns];
    x_vals[0].copy_from_slice(xmin);
    for i in 1..columns {
        for j in 0..num_size {
            x_vals[i][j] = x_vals[i - 1][j];
        }
        incr(&mut x_vals[i], dx);
    }

    x_vals
        .par_chunks(slice_size)
        .map(| x_vals | {
            let mut hp_data = HPData::new(chunks);
            let mut iteration_counts = vec![0; x_vals.len()];
            for i in 0..x_vals.len() {
                iteration_counts[i] = count_iterations_hp(&mut hp_data, &x_vals[i][0..chunks], &y[0..chunks], max_iter);
            }
            iteration_counts
        })
        .flatten()
        .collect()
}

/*
function countIterationsHP( /* Uint32Array */ x, /* Uint32Array */ y, maxIterations) {
    arraycopy(x,0,zx,0,chunks);
    arraycopy(y,0,zy,0,chunks);
    let count = 0;
    while (count < maxIterations) {
        arraycopy(zx, 0, work2, 0, chunks);
        multiply(work2,zx,chunks);  // work2 = zx*zx
        arraycopy(zy, 0, work1, 0, chunks);
        multiply(work1,zy,chunks);  // work1 = zy*zy
        arraycopy(work1,0,work3,0,chunks);   // work3 = zy*zy, save a copy.  (Note: multiplication uses work3.)
        add(work1,work2,chunks);  // work1 = zx*zx + zy*zy
        if ((work1[0] & 0xFFF8) != 0 && (work1[0] & 0xFFF8) != 0xFFF0)
            break;
        negate(work3,chunks);  // work3 = -work3 = -zy*zy
        add(work2,work3,chunks);  // work2 = zx*zx - zy*zy
        add(work2,x,chunks); // work2 = zx*zx - zy*zy + x, the next value for zx
        arraycopy(zx,0,work1,0,chunks);  // work1 = zx
        add(work1,zx,chunks);  // work1 = 2*zx
        multiply(work1,zy,chunks);  // work1 = 2*zx*zy
        add(work1,y,chunks);  // work1 = 2*zx*zy + y, the next value for zy
        arraycopy(work1,0,zy,0,chunks);  // zy = work1
        arraycopy(work2,0,zx,0,chunks);  // zx = work2
        count++;
    }
    return (count < maxIterations)? count : -1 ;
}

function arraycopy( sourceArray, sourceStart, destArray, destStart, count ) {
   for (let i = 0; i < count; i++) {
       destArray[destStart + i] = sourceArray[sourceStart + i];
   }
}
*/

fn count_iterations_hp(hp_data: &mut HPData, x: &[u32], y: &[u32], max_iterations: i32) -> i32 {
    let mut count = 0;
    hp_data.zx.copy_from_slice(x);
    hp_data.zy.copy_from_slice(y);

    // while count < max_iterations && zx*zx + zy*zy < 8.0 {
    while count < max_iterations {
        sq(&hp_data.zx, &mut hp_data.work3, &mut hp_data.work1);
        sq(&hp_data.zy, &mut hp_data.work3, &mut hp_data.work2);
        add(&hp_data.work1, &hp_data.work2, &mut hp_data.work3);
        if (hp_data.work3[0] & 0xFFF8) != 0 && (hp_data.work3[0] & 0xFFF8) != 0xFFF0 {
            return count;
        }

        // let new_zx = zx*zx - zy*zy + x;
        negate(&hp_data.work2, &mut hp_data.work3);
        add(&hp_data.work1, &hp_data.work3, &mut hp_data.work2);
        add(&hp_data.work2, x, &mut hp_data.work1);

        // zy = 2.0*zx*zy + y;
        add(&hp_data.zx, &hp_data.zx, &mut hp_data.work2);
        // zx = new_zx;
        hp_data.zx.copy_from_slice(&hp_data.work1);
        multiply(&hp_data.work2, &hp_data.zy, &mut hp_data.work1, &mut hp_data.work3, &mut hp_data.work4);
        add(&hp_data.work4, y, &mut hp_data.zy);

        count += 1;
    }
    -1
}

/*
function add( /* int[] */ x, /* int[] */ dx, /* int */ count) {
    let carry = 0;
    for (let i = count - 1; i >= 0; i--) {
        x[i] += dx[i];
        x[i] += carry;
        carry = x[i] >>> 16;
        x[i] &= 0xFFFF;
    }
}
*/
fn incr(x: &mut [u32], dx: &[u32]) {
    let mut carry = 0;
    let mut i = x.len();
    while i > 0 {
        i -= 1;
        x[i] += dx[i];
        x[i] += carry;
        carry = x[i] >> 16;
        x[i] &= 0xFFFF;
    }
}

fn add(x: &[u32], y: &[u32], out: &mut[u32]) {
    let mut carry = 0;
    let mut i = out.len();
    while i > 0 {
        i -= 1;
        out[i] = x[i] + y[i];
        out[i] += carry;
        carry = out[i] >> 16;
        out[i] &= 0xFFFF;
    }
}

/*
function multiply( /* int[] */ x, /* int[] */ y, /* int */ count){  // Can't allow x == y !
    let neg1 = (x[0] & 0x8000) != 0;
    if (neg1)
        negate(x,count);
    let neg2 = (y[0] & 0x8000) != 0;
    if (neg2)
        negate(y,count);
    if (x[0] == 0) {
        for (let i = 0; i < count; i++)
            work3[i] = 0;
    }
    else {
        let carry = 0;
        for (let i = count-1; i >= 0; i--) {
            work3[i] = x[0]*y[i] + carry;
            carry = work3[i] >>> 16;
            work3[i] &= 0xFFFF;
        }
    }
    for (let j = 1; j < count; j++) {
        let i = count - j;
        let carry = (x[j]*y[i]) >>> 16;
        i--;
        let k = count - 1;
        while (i >= 0) {
            work3[k] += x[j]*y[i] + carry;
            carry = work3[k] >>> 16;
            work3[k] &= 0xFFFF;
            i--;
            k--;
        }
        while (carry != 0 && k >= 0) {
            work3[k] += carry;
            carry = work3[k] >>> 16;
            work3[k] &= 0xFFFF;
            k--;
        }
    }
    arraycopy(work3,0,x,0,count);
    if (neg2)
        negate(y,count);
    if (neg1 != neg2)
        negate(x,count);
}
*/
fn multiply(x: &[u32], y: &[u32], work1: &mut [u32], work2: &mut [u32], out: &mut [u32]) {
    let negx = (x[0] & 0x8000) != 0;
    let negy = (y[0] & 0x8000) != 0;
    if negx != negy {
        if negx {
            negate(x, work1);
            multiply_pos(work1, y, work2);
        } else {
            negate(y, work1);
            multiply_pos(x, work1, work2);
        }
        negate(work2, out);
    } else if negx && negy {
        negate(x, work1);
        negate(y, work2);
        multiply_pos(work1, work2, out);
    } else {
        multiply_pos(x, y, out);
    }
}

fn multiply_pos(x: &[u32], y: &[u32], out: &mut [u32]) {
    let count = out.len();
    if x[0] == 0 {
        for i in 0..count {
            out[i] = 0;
        }
    } else {
        let mut carry = 0;
        let mut i = count;
        while i > 0 {
            i -= 1;
            out[i] = x[0]*y[i] + carry;
            carry = out[i] >> 16;
            out[i] &= 0xFFFF;
        }
    }
    for j in 1..count {
        let mut i = count - j;
        let mut carry = (x[j]*y[i]) >> 16;
        let mut k = count - 1;
        while i > 0 {
            i -= 1;
            out[k] += x[j]*y[i] + carry;
            carry = out[k] >> 16;
            out[k] &= 0xFFFF;
            k -= 1;
        }
        while carry != 0 {
            out[k] += carry;
            carry = out[k] >> 16;
            out[k] &= 0xFFFF;
            if k == 0 {
                break;
            }
            k -= 1;
        }
    }
}

fn sq(x: &[u32], work: &mut [u32], out: &mut [u32]) {
    let neg = (x[0] & 0x8000) != 0;
    if neg {
        negate(x, work);
        multiply_pos(work, work, out);
    } else {
        multiply_pos(x, x, out);
    }
}

/*
function negate( /* int[] */ x, /* int */ chunks) {
    for (let i = 0; i < chunks; i++)
        x[i] = 0xFFFF-x[i];
    ++x[chunks-1];
    for (let i = chunks-1; i > 0 && (x[i] & 0x10000) != 0; i--) {
        x[i] &= 0xFFFF;
        ++x[i-1];
    }
    x[0] &= 0xFFFF;
}
*/
fn negate(x: &[u32], out: &mut[u32]) {
    let chunks = out.len();
    for i in 0..chunks {
        out[i] = 0xFFFF - x[i];
    }
    debug_assert!(chunks > 0);
    let mut i = chunks - 1;
    out[i] += 1;
    while i > 0 && out[i] & 0x10000 != 0 {
        out[i] &= 0xFFFF;
        out[i - 1] += 1;
        i -= 1;
    }
    out[0] &= 0xFFFF;
}
