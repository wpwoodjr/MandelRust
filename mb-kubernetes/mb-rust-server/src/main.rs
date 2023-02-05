/*
    Backend Mandelbrot web server in Rust
    Rewritten from Javascript by Bill Wood, Jan/Feb 2023
    Based on work by David Eck
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
                        println!("number of Rayon slices must be > 0!");
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

// *** high precision *** //
use std::ops::{ BitAnd, BitXor, BitAndAssign, BitOrAssign, Shl, Shr, AddAssign, Sub, Mul };
use num::traits::{ Zero, One, AsPrimitive };
use core::cmp::PartialEq;
use core::mem::size_of;

macro_rules! t_bit_info {
    () => {
        {
            let t_size_bits = size_of::<T>()*8;
            let t_low_bits = u64::MAX >> (64 - t_size_bits/2);
            (t_size_bits, t_low_bits.as_())
        }
    };
}

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

struct HPData<T> {
    work1: Vec<T>,
    work2: Vec<T>,
    work3: Vec<T>,
    work4: Vec<T>,
    zx: Vec<T>,
    zy: Vec<T>,
}

impl<T> HPData<T> {
    fn new(chunks: usize) -> HPData::<T>
    where T: Zero + Copy,
    {
        HPData::<T> {
            work1: vec![T::zero(); chunks],
            work2: vec![T::zero(); chunks],
            work3: vec![T::zero(); chunks],
            work4: vec![T::zero(); chunks],
            zx: vec![T::zero(); chunks],
            zy: vec![T::zero(); chunks],
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

    // ignoring the last u32 chunk seems to be a small speed optimization which reduces precision but doesn't affect image quality
    let u32_chunks = mandelbrot_coords_hp.xmin.len() - unsafe { IMAGE_QUALITY };

    let iteration_counts = match unsafe { U_TYPE } {
        32 => {
            let xmin = u32_to_t::<u32>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u32>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u32>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u32>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks)
        }
        64 => {
            let xmin = u32_to_t::<u64>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u64>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u64>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u64>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks)
        }
        128 => {
            let xmin = u32_to_t::<u128>(&mandelbrot_coords_hp.xmin);
            let dx = u32_to_t::<u128>(&mandelbrot_coords_hp.dx);
            let yval = u32_to_t::<u128>(&mandelbrot_coords_hp.ymax);
            let dy = u32_to_t::<u128>(&mandelbrot_coords_hp.dy);
            compute_mandelbrot_hp_t(&xmin, &dx, &yval, &dy, mandelbrot_coords_hp.rows, mandelbrot_coords_hp.columns, mandelbrot_coords_hp.maxIterations, u32_chunks)
        }
        _ => panic!("illegal size!")
    };
    HttpResponse::Ok().json(iteration_counts)
}

use rayon::prelude::*;
fn compute_mandelbrot_hp_t<T>(xmin: &[T], dx: &[T], yval: &[T], dy: &[T], rows: usize, columns: usize, max_iter: i32, u32_chunks: usize) -> Vec<Vec<i32>>
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

    let slice_size = core::cmp::max(1, rows/unsafe { NUM_THREADS });
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

fn u32_to_t<T>(a: &[u32]) -> Vec<T>
where T: BitOrAssign + BitXor<Output = T> + Shl<usize, Output = T> + From<u32> + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let mut r = vec![];

    r.push(a[0].into());
    if a[0] & 0x8000 != 0 {
        let neg_mask = t_low_bits ^ 0xFFFF.into();
        r[0] |= neg_mask;
    }

    let mut i = 1;
    while i < a.len() {
        let mut k = 1;
        r.push(T::from(a[i]) << (t_size_bits/2 - k*16));
        i += 1;
        let rlast = r.len() - 1;
        while k < t_size_bits/32 && i < a.len() {
            k += 1;
            r[rlast] |= T::from(a[i]) << (t_size_bits/2 - k*16);
            i += 1;
        }
    }
    r
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
fn count_iterations_hp<T>(hp_data: &mut HPData<T>, x: &[T], y: &[T], max_iterations: i32) -> i32
where T: Zero + BitAnd + Shr<usize, Output = T> + Shl<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    T: std::fmt::LowerHex,
    // add, sq, multiply, negate requirements
    T: One + AddAssign + BitAndAssign + Sub<Output = T> + PartialEq,
{
    let mut count = 0;
    hp_data.zx.copy_from_slice(x);
    hp_data.zy.copy_from_slice(y);

    let (_, t_low_bits) = t_bit_info!();
    let t_8_test = (t_low_bits >> 3) << 3;
    // it's called the "what test" because I haven't figured out what it does :)
    let t_8_what_test = (t_low_bits >> 4) << 4;

    // while count < max_iterations && zx*zx + zy*zy < 8.0 {
    while count < max_iterations {
        sq(&hp_data.zx, &mut hp_data.work3, &mut hp_data.work1);
        sq(&hp_data.zy, &mut hp_data.work3, &mut hp_data.work2);
        add(&hp_data.work1, &hp_data.work2, &mut hp_data.work3);
        if (hp_data.work3[0] & t_8_test) != T::zero() && (hp_data.work3[0] & t_8_test) != t_8_what_test {
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
fn negate<T>(x: &[T], out: &mut[T])
where T: Zero + One + AddAssign + BitAnd + BitAndAssign + Sub<Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>
{
    let (_, t_low_bits) = t_bit_info!();
    let chunks = out.len();
    for i in 0..chunks {
        out[i] = t_low_bits - x[i];
    }

    debug_assert!(chunks > 0);
    let mut i = chunks - 1;
    out[i] += T::one();
    let t_overflow_test = t_low_bits + T::one();
    while i > 0 && out[i] & t_overflow_test != T::zero() {
        out[i] &= t_low_bits;
        out[i - 1] += T::one();
        i -= 1;
    }
    out[0] &= t_low_bits;
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
fn incr<T>(x: &mut [T], dx: &[T])
where T: Zero + AddAssign + Shr<usize, Output = T> + BitAndAssign + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let mut carry = T::zero();
    let mut i = x.len();
    while i > 0 {
        i -= 1;
        x[i] += dx[i] + carry;
        carry = x[i] >> t_size_bits/2;
        x[i] &= t_low_bits;
    }
}

fn add<T>(x: &[T], y: &[T], out: &mut[T])
where T: Zero + AddAssign + Shr<usize, Output = T> + BitAndAssign + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let mut carry = T::zero();
    let mut i = out.len();
    while i > 0 {
        i -= 1;
        out[i] = x[i] + y[i] + carry;
        carry = out[i] >> t_size_bits/2;
        out[i] &= t_low_bits;
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
fn multiply<T>(x: &[T], y: &[T], work1: &mut [T], work2: &mut [T], out: &mut [T])
where T: Zero + One + BitAnd + Shr<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    // negate and multiply_pos requirements
    T: AddAssign + BitAndAssign + Sub<Output = T> + PartialEq,
{
    let (_, t_low_bits) = t_bit_info!();
    let t_neg_test = (t_low_bits + T::one()) >> 1;

    let negx = (x[0] & t_neg_test) != T::zero();
    let negy = (y[0] & t_neg_test) != T::zero();
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

fn multiply_pos<T>(x: &[T], y: &[T], out: &mut [T])
where T: Zero + AddAssign + Mul<Output = T> + Shr<usize, Output = T> + BitAndAssign + PartialEq + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let count = out.len();

    if x[0] == T::zero() {
        // for i in 0..count {
        //    out[i] = T::zero();
        // }
        out.fill(T::zero());
    } else {
        let mut carry = T::zero();
        let mut i = count;
        while i > 0 {
            i -= 1;
            out[i] = x[0]*y[i] + carry;
            carry = out[i] >> t_size_bits/2;
            out[i] &= t_low_bits;
        }
    }

    for j in 1..count {
        let mut i = count - j;
        let mut carry = (x[j]*y[i]) >> t_size_bits/2;
        let mut k = count - 1;
        while i > 0 {
            i -= 1;
            out[k] += x[j]*y[i] + carry;
            carry = out[k] >> t_size_bits/2;
            out[k] &= t_low_bits;
            k -= 1;
        }
        while carry != T::zero() {
            out[k] += carry;
            carry = out[k] >> t_size_bits/2;
            out[k] &= t_low_bits;
            if k == 0 {
                break;
            }
            k -= 1;
        }
    }
}

fn sq<T>(x: &[T], work: &mut [T], out: &mut [T])
where T: Zero + One + BitAnd + Shr<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    // negate and multiply_pos requirements
    T: AddAssign + BitAndAssign + Sub<Output = T> + PartialEq,
{
    let (_, t_low_bits) = t_bit_info!();
    let t_neg_test = (t_low_bits + T::one()) >> 1;
    let neg = (x[0] & t_neg_test) != T::zero();
    if neg {
        negate(x, work);
        multiply_pos(work, work, out);
    } else {
        multiply_pos(x, x, out);
    }
}
