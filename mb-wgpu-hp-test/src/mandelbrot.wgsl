// Change this value to adjust the precision = (MAX_NUM_SIZE - 2)/3.325*16
const MAX_NUM_SIZE: u32 = 11u;

/// use an override-expression?
type HPArray = array<u32, MAX_NUM_SIZE>;
struct HPNumber {
    a: HPArray,
};
type HPNumberPtr = ptr<private, HPNumber>;

var<private> offset: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> x: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> y: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> zx: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> zy: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> work1: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> work2: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> work3: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));
var<private> work4: HPNumber = HPNumber(HPArray(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u));

struct Output {
    data: array<u32>,
};

@binding(0) @group(0)
var<storage, read_write> output: Output;

struct MandelbrotCoordsHP {
    xmin: HPNumber,
    dx: HPNumber,
    ymax: HPNumber,
    dy: HPNumber,
    num_size: u32,
    max_iterations: u32,
    rows: u32,
    columns: u32,
};

@binding(1) @group(0)
var<storage, read> mandelbrot_coords_hp: MandelbrotCoordsHP;

/// column outside of bounds; don't use MAX_NUM_SIZE; check arith routines
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let column = global_id.x;
    let row = global_id.y;
//    if (column >= mandelbrot_coords_hp.columns || row >= mandelbrot_coords_hp.rows) {
//        return;
//    }

    // compute x coord
    offset.a[0] = column;
    work1 = mandelbrot_coords_hp.dx;
    multiply_pos(&work1, &offset, &work2);
    x = mandelbrot_coords_hp.xmin;
    add(&x, &work2, &zx);

    // compute y coord
    offset.a[0] = row;
    work3 = mandelbrot_coords_hp.dy;
    multiply_pos(&work3, &offset, &work2);
    negate(&work2, &work3);
    y = mandelbrot_coords_hp.ymax;
    add(&y, &work3, &zy);

    let iterations: i32 = count_iterations_hp(mandelbrot_coords_hp.max_iterations);
    let index = column + row*mandelbrot_coords_hp.columns;
    var color: u32 = 0x0u;
    if (iterations >= 0) {
        color = u32(iterations);
    }
    output.data[index] = color | (color << 8u) | (color << 16u) | (0xFFu << 24u);
}

fn count_iterations_hp(max_iterations: u32) -> i32 {

    // while count < max_iterations && zx*zx + zy*zy < 8.0
    for (var count = 0u; count < max_iterations; count++) {
        sq(&zx, &work3, &work1);
        sq(&zy, &work3, &work2);
        add(&work1, &work2, &work3);
        let test = work3.a[0] & 0xFFF8u;
        if (test != 0u && test != 0xFFF0u) {
            return i32(count);
        }

        add(&zx, &zx, &work4);

        // zx = zx*zx - zy*zy + x;
        negate(&work2, &work3);
        add(&work1, &work3, &work2);
        add(&work2, &x, &zx);

        // zy = 2.0*zx*zy + y;
        multiply(&work4, &zy, &work1, &work3, &work2);
        add(&work2, &y, &zy);
    }

    return -1;
}


// HP arithmetic functions

fn add(x: HPNumberPtr, y: HPNumberPtr, result: HPNumberPtr) {
    var carry: u32 = 0u;
    var i = MAX_NUM_SIZE;
    while i > 0u {
        i--;
        (*result).a[i] = (*x).a[i] + (*y).a[i] + carry;
        carry = (*result).a[i] >> 16u;
        (*result).a[i] &= 0xFFFFu;
    }
}

fn multiply(x: HPNumberPtr, y: HPNumberPtr, work1: HPNumberPtr, work2: HPNumberPtr, result: HPNumberPtr) {
    let negx = ((*x).a[0] & 0x8000u) != 0u;
    let negy = ((*y).a[0] & 0x8000u) != 0u;

    if negx != negy {
        if negx {
            negate(x, work1);
            multiply_pos(work1, y, work2);
        } else {
            negate(y, work1);
            multiply_pos(x, work1, work2);
        }
        negate(work2, result);
    } else if negx && negy {
        negate(x, work1);
        negate(y, work2);
        multiply_pos(work1, work2, result);
    } else {
        multiply_pos(x, y, result);
    }
}

fn multiply_pos(x: HPNumberPtr, y: HPNumberPtr, result: HPNumberPtr) {
    let count = MAX_NUM_SIZE;
    if (*x).a[0] == 0u {
        for (var i = 0u; i < count; i++) {
            (*result).a[i] = 0u;
        }
    } else {
        var carry = 0u;
        var i = count;
        while i > 0u {
            i--;
            (*result).a[i] = (*x).a[0]*(*y).a[i] + carry;
            carry = (*result).a[i] >> 16u;
            (*result).a[i] &= 0xFFFFu;
        }
    }
    for (var j = 1u; j < count; j++) {
        var i = count - j;
        var carry = ((*x).a[j]*(*y).a[i]) >> 16u;
        var k = count - 1u;
        while i > 0u {
            i--;
            (*result).a[k] += (*x).a[j]*(*y).a[i] + carry;
            carry = (*result).a[k] >> 16u;
            (*result).a[k] &= 0xFFFFu;
            k--;
        }
        while carry != 0u {
            (*result).a[k] += carry;
            carry = (*result).a[k] >> 16u;
            (*result).a[k] &= 0xFFFFu;
            if k == 0u {
                break;
            }
            k--;
        }
    }
}

fn sq(x: HPNumberPtr, work: HPNumberPtr, result: HPNumberPtr) {
    let neg = ((*x).a[0] & 0x8000u) != 0u;
    if (neg) {
        negate(x, work);
        multiply_pos(work, work, result);
    } else {
        multiply_pos(x, x, result);
    }
}

fn negate(x: HPNumberPtr, result: HPNumberPtr) {
    for (var i = 0u; i < MAX_NUM_SIZE; i++) {
        (*result).a[i] = 0xFFFFu - (*x).a[i];
    }

    var i = MAX_NUM_SIZE - 1u;
    (*result).a[i]++;
    while i > 0u && ((*result).a[i] & 0x10000u) != 0u {
        (*result).a[i] &= 0xFFFFu;
        (*result).a[i - 1u]++;
        i--;
    }
    (*result).a[0] &= 0xFFFFu;
}
