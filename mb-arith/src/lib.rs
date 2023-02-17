/*
    Mandelbrot arithmetic in Rust
    Rewritten from Javascript by Bill Wood, Jan/Feb 2023
    Based on work by David Eck
*/


// *** low precision *** //
pub fn count_iterations(x: f64, y: f64, max_iterations: i32) -> i32 {
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

pub struct HPData<T> {
    work1: Vec<T>,
    work2: Vec<T>,
    work3: Vec<T>,
    work4: Vec<T>,
    zx: Vec<T>,
    zy: Vec<T>,
}

impl<T> HPData<T> {
    pub fn new(chunks: usize) -> HPData::<T>
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

pub fn u32_to_t<T>(a: &[u32]) -> Vec<T>
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
pub fn count_iterations_hp<T>(hp_data: &mut HPData<T>, x: &[T], y: &[T], max_iterations: i32) -> i32
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
        // let test8 = hp_data.work3[0] & t_8_test;
        let test8 = unsafe { *hp_data.work3.get_unchecked(0) } & t_8_test;
        if test8 != T::zero() && test8 != t_8_what_test {
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
pub fn negate<T>(x: &[T], out: &mut[T])
where T: Zero + One + AddAssign + BitAnd + BitAndAssign + Sub<Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>
{
    let (_, t_low_bits) = t_bit_info!();
    let chunks = out.len();
    for i in 0..chunks {
        // out[i] = t_low_bits - x[i];
        unsafe { *out.get_unchecked_mut(i) = t_low_bits - *x.get_unchecked(i); }
    }

    debug_assert!(chunks > 0);
    let mut i = chunks - 1;
    // out[i] += T::one();
    unsafe { *out.get_unchecked_mut(i) += T::one(); }
    let t_overflow_test = t_low_bits + T::one();
    // while i > 0 && out[i] & t_overflow_test != T::zero() {
    while i > 0 && unsafe { *out.get_unchecked(i) } & t_overflow_test != T::zero() {
        // out[i] &= t_low_bits;
        unsafe { *out.get_unchecked_mut(i) &= t_low_bits };
        // out[i - 1] += T::one();
        unsafe { *out.get_unchecked_mut(i - 1) += T::one() };
        i -= 1;
    }
    // out[0] &= t_low_bits;
    unsafe { *out.get_unchecked_mut(0) &= t_low_bits };
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
pub fn incr<T>(x: &mut [T], dx: &[T])
where T: Zero + AddAssign + Shr<usize, Output = T> + BitAndAssign + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let mut carry = T::zero();
    let mut i = x.len();
    while i > 0 {
        i -= 1;
        // x[i] += dx[i] + carry;
        unsafe { *x.get_unchecked_mut(i) += *dx.get_unchecked(i) + carry };
        // carry = x[i] >> t_size_bits/2;
        carry = unsafe { *x.get_unchecked(i) } >> t_size_bits/2;
        // x[i] &= t_low_bits;
        unsafe { *x.get_unchecked_mut(i) &= t_low_bits };
    }
}

pub fn add<T>(x: &[T], y: &[T], out: &mut[T])
where T: Zero + AddAssign + Shr<usize, Output = T> + BitAndAssign + Copy + 'static,
    u64: AsPrimitive<T>
{
    let (t_size_bits, t_low_bits) = t_bit_info!();
    let mut carry = T::zero();
    let mut i = out.len();
    while i > 0 {
        i -= 1;
        // out[i] = x[i] + y[i] + carry;
        unsafe { *out.get_unchecked_mut(i) = *x.get_unchecked(i) + *y.get_unchecked(i) + carry };
        // carry = out[i] >> t_size_bits/2;
        carry = unsafe { *out.get_unchecked(i) } >> t_size_bits/2;
        // out[i] &= t_low_bits;
        unsafe { *out.get_unchecked_mut(i) &= t_low_bits };
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
pub fn multiply<T>(x: &[T], y: &[T], work1: &mut [T], work2: &mut [T], out: &mut [T])
where T: Zero + One + BitAnd + Shr<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    // negate and multiply_pos requirements
    T: AddAssign + BitAndAssign + Sub<Output = T> + PartialEq,
{
    let (_, t_low_bits) = t_bit_info!();
    let t_neg_test = (t_low_bits + T::one()) >> 1;

    // let negx = (x[0] & t_neg_test) != T::zero();
    let negx = (unsafe { *x.get_unchecked(0) } & t_neg_test) != T::zero();
    // let negy = (y[0] & t_neg_test) != T::zero();
    let negy = (unsafe { *y.get_unchecked(0) } & t_neg_test) != T::zero();
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

    // let x0 = x[0];
    let x0 = unsafe { *x.get_unchecked(0) };
    if x0 == T::zero() {
        // for i in 0..count {
        //    out[i] = T::zero();
        // }
        out.fill(T::zero());
    } else {
        let mut carry = T::zero();
        let mut i = count;
        while i > 0 {
            i -= 1;
            // out[i] = x0*y[i] + carry;
            unsafe { *out.get_unchecked_mut(i) = x0* *y.get_unchecked(i) + carry };
            // carry = out[i] >> t_size_bits/2;
            carry = unsafe { *out.get_unchecked(i) } >> t_size_bits/2;
            // out[i] &= t_low_bits;
            unsafe { *out.get_unchecked_mut(i) &= t_low_bits };
        }
    }

    for j in 1..count {
        let mut i = count - j;
        // let mut carry = (x[j]*y[i]) >> t_size_bits/2;
        let mut carry = unsafe { *x.get_unchecked(j)* *y.get_unchecked(i) } >> t_size_bits/2;
        let mut k = count - 1;
        while i > 0 {
            i -= 1;
            // out[k] += x[j]*y[i] + carry;
            unsafe { *out.get_unchecked_mut(k) += *x.get_unchecked(j)* *y.get_unchecked(i) + carry };
            // carry = out[k] >> t_size_bits/2;
            carry = unsafe { *out.get_unchecked(k) } >> t_size_bits/2;
            // out[k] &= t_low_bits;
            unsafe { *out.get_unchecked_mut(k) &= t_low_bits };
            k -= 1;
        }
        while carry != T::zero() {
            // out[k] += carry;
            unsafe { *out.get_unchecked_mut(k) += carry };
            // carry = out[k] >> t_size_bits/2;
            carry = unsafe { *out.get_unchecked(k) } >> t_size_bits/2;
            // out[k] &= t_low_bits;
            unsafe { *out.get_unchecked_mut(k) &= t_low_bits };
            if k == 0 {
                break;
            }
            k -= 1;
        }
    }
}

pub fn sq<T>(x: &[T], work: &mut [T], out: &mut [T])
where T: Zero + One + BitAnd + Shr<usize, Output = T> + Copy + 'static,
    <T as BitAnd>::Output: PartialEq<T>,
    u64: AsPrimitive<T>,
    // negate and multiply_pos requirements
    T: AddAssign + BitAndAssign + Sub<Output = T> + PartialEq,
{
    let (_, t_low_bits) = t_bit_info!();
    let t_neg_test = (t_low_bits + T::one()) >> 1;
    // let neg = (x[0] & t_neg_test) != T::zero();
    let neg = (unsafe { *x.get_unchecked(0) } & t_neg_test) != T::zero();
    if neg {
        negate(x, work);
        multiply_pos(work, work, out);
    } else {
        multiply_pos(x, x, out);
    }
}
