struct MandelbrotCoordsHP {
    xmin: Vec<u32>,
    dx: Vec<u32>,
    ymax: Vec<u32>,
    dy: Vec<u32>,
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

fn compute_mandelbrot_hp(x: &[u32], dx: &[u32], y: &[u32], dy: &[u32], max_iterations: i32) -> Vec<i32> {

    let num_size = x.len();
    // ignoring the last u32 chunk seems to be a small speed optimization which reduces precision but doesn't affect image quality
    let chunks = num_size;// - 1;
    let mut hp_data = HPData::new(chunks);

    // get current column from wsgl runtime
    let column = wsgl column;
    // compute x coord
    hp_data.work1[0] = column;
    multiply_pos(&mandelbrot_coords_hp.dx, &hp_data.work1, &mut hp_data.zx);
    incr(&hp_data.zx, &mandelbrot_coords_hp.xmin);

    // get current row from wsgl runtime
    let row = wsgl row;
    // compute y coord
    hp_data.work1[0] = row;
    multiply_pos(&mandelbrot_coords_hp.dy, &hp_data.work1, &mut hp_data.zy);
    incr(&hp_data.zy, &mandelbrot_coords_hp.ymax);

    let iter = count_iterations_hp(&mut hp_data, max_iterations);

    // assign iter to wsgl output buffer
    output_buffer[row*wsgl row_size + col] = iter;
}

fn count_iterations_hp(hp_data: &mut HPData, max_iterations: i32) -> i32 {
    let mut count = 0;

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
