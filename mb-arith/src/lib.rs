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

        add(&hp_data.zx, &hp_data.zx, &mut hp_data.work4);

        // zx = zx*zx - zy*zy + x;
        negate(&hp_data.work2, &mut hp_data.work3);
        add(&hp_data.work1, &hp_data.work3, &mut hp_data.work2);
        add(&hp_data.work2, x, &mut hp_data.zx);

        // zy = 2.0*zx*zy + y;
        multiply(&hp_data.work4, &hp_data.zy, &mut hp_data.work1, &mut hp_data.work3, &mut hp_data.work2);
        add(&hp_data.work2, y, &mut hp_data.zy);

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


// *** high precision, full-width limbs *** //
//
// Same fixed point format as the generic engine above (limb 0 = integral part,
// limbs 1.. = fraction, two's complement, most significant first), but every limb
// uses all of its bits. Limb products are widening multiplies to a double-width
// type and carries use overflowing-add chains, instead of masking half-empty limbs.
// Squaring computes each cross product once and doubles it. Truncation and
// carry-estimation semantics are bit-identical to the generic path (see tests).
//
// Two instantiations of the same macro:
//   u64 limbs, u128 products - native targets, where 64x64 -> 128 is a single
//     MULX on x86-64 / MUL+UMULH on aarch64, and carries compile to ADC chains
//   u32 limbs, u64 products - wasm32, which has no 64x64 -> 128 multiply but
//     computes 32x64 -> 64 with a single native i64.mul

macro_rules! fw_engine {
    ($limb:ty, $wide:ty,
     $hpdata:ident, $u32_to_limbs:ident, $negate:ident, $incr:ident, $add:ident, $sub:ident,
     $mul_wide:ident, $multiply_pos:ident, $square_pos:ident, $multiply:ident, $sq:ident,
     $count_iterations:ident) => {

pub struct $hpdata {
    work1: Vec<$limb>,
    work2: Vec<$limb>,
    work3: Vec<$limb>,
    work4: Vec<$limb>,
    zx: Vec<$limb>,
    zy: Vec<$limb>,
}

impl $hpdata {
    pub fn new(chunks: usize) -> $hpdata {
        $hpdata {
            work1: vec![0; chunks],
            work2: vec![0; chunks],
            work3: vec![0; chunks],
            work4: vec![0; chunks],
            zx: vec![0; chunks],
            zy: vec![0; chunks],
        }
    }
}

// input format: a[0] = 16 bit signed integral part, a[1..] = 16 bit fraction digits
pub fn $u32_to_limbs(a: &[u32]) -> Vec<$limb> {
    const BITS: usize = <$limb>::BITS as usize;
    const DPL: usize = BITS/16; // fraction digits per limb
    let n_frac = a.len() - 1;
    let mut r = vec![0 as $limb; 1 + (n_frac + DPL - 1)/DPL];
    r[0] = a[0] as $limb;
    if a[0] & 0x8000 != 0 {
        r[0] |= !(0xFFFF as $limb);
    }
    for (k, &d) in a[1..].iter().enumerate() {
        r[1 + k/DPL] |= (d as $limb) << (BITS - 16 - 16*(k % DPL));
    }
    r
}

pub fn $negate(x: &[$limb], out: &mut [$limb]) {
    let mut carry = 1 as $limb;
    for i in (0..out.len()).rev() {
        let (s, c) = (!unsafe { *x.get_unchecked(i) }).overflowing_add(carry);
        unsafe { *out.get_unchecked_mut(i) = s };
        carry = c as $limb;
    }
}

pub fn $incr(x: &mut [$limb], dx: &[$limb]) {
    let mut carry = 0 as $limb;
    for i in (0..x.len()).rev() {
        let (s1, c1) = unsafe { *x.get_unchecked(i) }.overflowing_add(unsafe { *dx.get_unchecked(i) });
        let (s2, c2) = s1.overflowing_add(carry);
        unsafe { *x.get_unchecked_mut(i) = s2 };
        carry = (c1 | c2) as $limb;
    }
}

pub fn $add(x: &[$limb], y: &[$limb], out: &mut [$limb]) {
    let mut carry = 0 as $limb;
    for i in (0..out.len()).rev() {
        let (s1, c1) = unsafe { *x.get_unchecked(i) }.overflowing_add(unsafe { *y.get_unchecked(i) });
        let (s2, c2) = s1.overflowing_add(carry);
        unsafe { *out.get_unchecked_mut(i) = s2 };
        carry = (c1 | c2) as $limb;
    }
}

pub fn $sub(x: &[$limb], y: &[$limb], out: &mut [$limb]) {
    let mut borrow = 0 as $limb;
    for i in (0..out.len()).rev() {
        let (s1, b1) = unsafe { *x.get_unchecked(i) }.overflowing_sub(unsafe { *y.get_unchecked(i) });
        let (s2, b2) = s1.overflowing_sub(borrow);
        unsafe { *out.get_unchecked_mut(i) = s2 };
        borrow = (b1 | b2) as $limb;
    }
}

#[inline(always)]
fn $mul_wide(a: $limb, b: $limb) -> $wide {
    a as $wide * b as $wide
}

// truncated column-wise (Comba) multiply of non-negative values; keeps the top
// `out.len()` limbs of the full product. Column `n` contributes only the high
// halves of its products as carry, matching the generic multiply_pos exactly.
//
// The two accumulation strategies below compute identical column sums. On native
// targets, overflowing_add compiles to ADC chains and wins; wasm has no carry
// flag, so there products are accumulated as separate hi/lo halves instead,
// which cannot overflow for any realistic limb count and needs no flag emulation.
#[cfg(not(target_arch = "wasm32"))]
fn $multiply_pos(x: &[$limb], y: &[$limb], out: &mut [$limb]) {
    const BITS: usize = <$limb>::BITS as usize;
    let n = out.len();

    let mut carry = 0 as $wide;
    for i in 1..n {
        carry += $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *y.get_unchecked(n - i) }) >> BITS;
    }

    for m in (0..n).rev() {
        let mut acc = carry;
        let mut acc_hi = 0 as $limb;
        for i in 0..=m {
            let p = $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *y.get_unchecked(m - i) });
            let (s, o) = acc.overflowing_add(p);
            acc = s;
            acc_hi += o as $limb;
        }
        unsafe { *out.get_unchecked_mut(m) = acc as $limb };
        carry = (acc >> BITS) | ((acc_hi as $wide) << BITS);
    }
}

#[cfg(target_arch = "wasm32")]
fn $multiply_pos(x: &[$limb], y: &[$limb], out: &mut [$limb]) {
    const BITS: usize = <$limb>::BITS as usize;
    const LOW: $wide = (1 as $wide << BITS) - 1;
    let n = out.len();

    let mut carry = 0 as $wide;
    for i in 1..n {
        carry += $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *y.get_unchecked(n - i) }) >> BITS;
    }

    for m in (0..n).rev() {
        let mut acc_lo = carry;
        let mut acc_hi = 0 as $wide;
        for i in 0..=m {
            let p = $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *y.get_unchecked(m - i) });
            acc_lo += p & LOW;
            acc_hi += p >> BITS;
        }
        unsafe { *out.get_unchecked_mut(m) = acc_lo as $limb };
        carry = (acc_lo >> BITS) + acc_hi;
    }
}

// truncated squaring: each cross product x[i]*x[j] (i < j) is computed once and
// added twice, roughly halving the multiplies vs $multiply_pos(x, x, out).
// Accumulation strategies per target as in $multiply_pos.
#[cfg(not(target_arch = "wasm32"))]
fn $square_pos(x: &[$limb], out: &mut [$limb]) {
    const BITS: usize = <$limb>::BITS as usize;
    let n = out.len();

    let mut carry = 0 as $wide;
    for i in 1..(n + 1)/2 {
        carry += ($mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *x.get_unchecked(n - i) }) >> BITS) * 2;
    }
    if n % 2 == 0 && n > 0 {
        let h = unsafe { *x.get_unchecked(n/2) };
        carry += $mul_wide(h, h) >> BITS;
    }

    for m in (0..n).rev() {
        let mut acc = carry;
        let mut acc_hi = 0 as $limb;
        for i in 0..(m + 1)/2 {
            let p = $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *x.get_unchecked(m - i) });
            let (s, o) = acc.overflowing_add(p);
            acc = s;
            acc_hi += o as $limb;
            let (s, o) = acc.overflowing_add(p);
            acc = s;
            acc_hi += o as $limb;
        }
        if m % 2 == 0 {
            let h = unsafe { *x.get_unchecked(m/2) };
            let (s, o) = acc.overflowing_add($mul_wide(h, h));
            acc = s;
            acc_hi += o as $limb;
        }
        unsafe { *out.get_unchecked_mut(m) = acc as $limb };
        carry = (acc >> BITS) | ((acc_hi as $wide) << BITS);
    }
}

#[cfg(target_arch = "wasm32")]
fn $square_pos(x: &[$limb], out: &mut [$limb]) {
    const BITS: usize = <$limb>::BITS as usize;
    const LOW: $wide = (1 as $wide << BITS) - 1;
    let n = out.len();

    let mut carry = 0 as $wide;
    for i in 1..(n + 1)/2 {
        carry += ($mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *x.get_unchecked(n - i) }) >> BITS) * 2;
    }
    if n % 2 == 0 && n > 0 {
        let h = unsafe { *x.get_unchecked(n/2) };
        carry += $mul_wide(h, h) >> BITS;
    }

    for m in (0..n).rev() {
        let mut acc_lo = carry;
        let mut acc_hi = 0 as $wide;
        for i in 0..(m + 1)/2 {
            let p = $mul_wide(unsafe { *x.get_unchecked(i) }, unsafe { *x.get_unchecked(m - i) });
            acc_lo += (p & LOW) << 1;
            acc_hi += (p >> BITS) << 1;
        }
        if m % 2 == 0 {
            let h = unsafe { *x.get_unchecked(m/2) };
            let p = $mul_wide(h, h);
            acc_lo += p & LOW;
            acc_hi += p >> BITS;
        }
        unsafe { *out.get_unchecked_mut(m) = acc_lo as $limb };
        carry = (acc_lo >> BITS) + acc_hi;
    }
}

pub fn $multiply(x: &[$limb], y: &[$limb], work1: &mut [$limb], work2: &mut [$limb], out: &mut [$limb]) {
    const SIGN: usize = <$limb>::BITS as usize - 1;
    let negx = (x[0] >> SIGN) != 0;
    let negy = (y[0] >> SIGN) != 0;
    if negx != negy {
        if negx {
            $negate(x, work1);
            $multiply_pos(work1, y, work2);
        } else {
            $negate(y, work1);
            $multiply_pos(x, work1, work2);
        }
        $negate(work2, out);
    } else if negx {
        $negate(x, work1);
        $negate(y, work2);
        $multiply_pos(work1, work2, out);
    } else {
        $multiply_pos(x, y, out);
    }
}

pub fn $sq(x: &[$limb], work: &mut [$limb], out: &mut [$limb]) {
    const SIGN: usize = <$limb>::BITS as usize - 1;
    if (x[0] >> SIGN) != 0 {
        $negate(x, work);
        $square_pos(work, out);
    } else {
        $square_pos(x, out);
    }
}

pub fn $count_iterations(hp_data: &mut $hpdata, x: &[$limb], y: &[$limb], max_iterations: i32) -> i32 {
    let mut count = 0;
    hp_data.zx.copy_from_slice(x);
    hp_data.zy.copy_from_slice(y);

    while count < max_iterations {
        $sq(&hp_data.zx, &mut hp_data.work3, &mut hp_data.work1);      // work1 = zx*zx
        $sq(&hp_data.zy, &mut hp_data.work3, &mut hp_data.work2);      // work2 = zy*zy
        $add(&hp_data.work1, &hp_data.work2, &mut hp_data.work3);      // work3 = zx*zx + zy*zy
        let test8 = unsafe { *hp_data.work3.get_unchecked(0) } & !(7 as $limb);
        if test8 != 0 && test8 != !(15 as $limb) {
            return count;
        }

        $add(&hp_data.zx, &hp_data.zx, &mut hp_data.work4);            // work4 = 2*zx

        // zx = zx*zx - zy*zy + x;
        $sub(&hp_data.work1, &hp_data.work2, &mut hp_data.work3);
        $add(&hp_data.work3, x, &mut hp_data.zx);

        // zy = 2*zx*zy + y;
        $multiply(&hp_data.work4, &hp_data.zy, &mut hp_data.work1, &mut hp_data.work3, &mut hp_data.work2);
        $add(&hp_data.work2, y, &mut hp_data.zy);

        count += 1;
    }
    -1
}

    };
}

fw_engine!(u64, u128,
    HPData64, u32_to_limbs64, negate64, incr64, add64, sub64,
    mul_wide64, multiply_pos64, square_pos64, multiply64, sq64,
    count_iterations_hp64);

fw_engine!(u32, u64,
    HPData32, u32_to_limbs32, negate32, incr32, add32, sub32,
    mul_wide32, multiply_pos32, square_pos32, multiply32, sq32,
    count_iterations_hp32);



#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        // random 16 bit fraction digit
        fn digit(&mut self) -> u32 {
            ((self.next() >> 40) & 0xFFFF) as u32
        }
    }

    // random coordinate in roughly [-3, 3): 16 bit signed integral digit + random fraction digits
    fn random_coord(rng: &mut Lcg, n_digits: usize) -> Vec<u32> {
        let int_part = (rng.next() % 6) as i64 - 3;
        let mut a = vec![(int_part as i16 as u16) as u32];
        for _ in 1..n_digits {
            a.push(rng.digit());
        }
        a
    }

    fn count_old(x: &[u32], y: &[u32], chunks: usize, max_iter: i32) -> i32 {
        let xo = u32_to_t::<u128>(x);
        let yo = u32_to_t::<u128>(y);
        let mut hp = HPData::<u128>::new(chunks);
        count_iterations_hp(&mut hp, &xo[0..chunks], &yo[0..chunks], max_iter)
    }

    fn count_new(x: &[u32], y: &[u32], chunks: usize, max_iter: i32) -> i32 {
        let xn = u32_to_limbs64(x);
        let yn = u32_to_limbs64(y);
        let mut hp = HPData64::new(chunks);
        count_iterations_hp64(&mut hp, &xn[0..chunks], &yn[0..chunks], max_iter)
    }

    #[test]
    fn conversion_matches_generic_u128() {
        let mut rng = Lcg(42);
        for n_digits in 1..=21 {
            let a = random_coord(&mut rng, n_digits);
            let old: Vec<u64> = u32_to_t::<u128>(&a).iter().map(|&v| v as u64).collect();
            let new = u32_to_limbs64(&a);
            assert_eq!(old, new, "digits: {:x?}", a);
        }
    }

    #[test]
    fn counts_match_generic_u128() {
        let mut rng = Lcg(12345);
        let max_iter = 500;
        for n_digits in 2..=17 {
            let chunks = 1 + (n_digits - 1 + 3)/4;
            for _ in 0..300 {
                let x = random_coord(&mut rng, n_digits);
                let y = random_coord(&mut rng, n_digits);
                let old = count_old(&x, &y, chunks, max_iter);
                let new = count_new(&x, &y, chunks, max_iter);
                assert_eq!(old, new, "x: {:x?} y: {:x?} chunks: {}", x, y, chunks);
            }
        }
    }

    fn count_new32(x: &[u32], y: &[u32], chunks: usize, max_iter: i32) -> i32 {
        let xn = u32_to_limbs32(x);
        let yn = u32_to_limbs32(y);
        let mut hp = HPData32::new(chunks);
        count_iterations_hp32(&mut hp, &xn[0..chunks], &yn[0..chunks], max_iter)
    }

    // the u32 limb engine truncates at 32 bit granularity, so it is not bit-identical
    // to the u128/u64 paths; iteration counts should still almost never differ
    #[test]
    fn u32_engine_matches_u64_engine() {
        let mut rng = Lcg(999);
        let max_iter = 500;
        let mut mismatches = 0;
        let mut total = 0;
        for n_digits in 2..=17 {
            let chunks64 = 1 + (n_digits - 1 + 3)/4;
            let chunks32 = 1 + (n_digits - 1 + 1)/2;
            for _ in 0..300 {
                let x = random_coord(&mut rng, n_digits);
                let y = random_coord(&mut rng, n_digits);
                let a = count_new(&x, &y, chunks64, max_iter);
                let b = count_new32(&x, &y, chunks32, max_iter);
                if a != b {
                    mismatches += 1;
                }
                total += 1;
            }
        }
        assert!(mismatches <= total/1000, "{} of {} points differ", mismatches, total);
    }

    #[test]
    fn special_points_match() {
        let max_iter = 1000;
        // (0, 0) is in the set; (-2, 0) is on the boundary and in the set; (2, 2) escapes immediately
        let cases: [(u32, u32); 3] = [(0, 0), (0xFFFE, 0), (2, 2)];
        for n_digits in [2usize, 5, 9, 13] {
            let chunks = 1 + (n_digits - 1 + 3)/4;
            for &(xi, yi) in &cases {
                let mut x = vec![0u32; n_digits];
                let mut y = vec![0u32; n_digits];
                x[0] = xi;
                y[0] = yi;
                let old = count_old(&x, &y, chunks, max_iter);
                let new = count_new(&x, &y, chunks, max_iter);
                assert_eq!(old, new, "x0: {:x} y0: {:x}", xi, yi);
            }
        }
        assert_eq!(count_new(&[0, 0], &[0, 0], 2, max_iter), -1);
        assert_eq!(count_new(&[2, 0], &[2, 0], 2, max_iter), 0);
    }

    #[test]
    fn matches_f64_at_low_zoom() {
        // sanity check against the f64 path away from the set boundary, where
        // truncation differences can't flip the result
        let mut rng = Lcg(777);
        let n_digits = 13;
        let chunks = 1 + (n_digits - 1 + 3)/4;
        let mut checked = 0;
        for _ in 0..2000 {
            let x = random_coord(&mut rng, n_digits);
            let y = random_coord(&mut rng, n_digits);
            let xf = digits_to_f64(&x);
            let yf = digits_to_f64(&y);
            let f = count_iterations(xf, yf, 100);
            // only compare fast escapes; borderline points can legitimately differ
            if f >= 0 && f <= 20 {
                let hp = count_new(&x, &y, chunks, 100);
                assert!((hp - f).abs() <= 1, "x: {} y: {} f64: {} hp: {}", xf, yf, f, hp);
                checked += 1;
            }
        }
        assert!(checked > 500);
    }

    fn digits_to_f64(a: &[u32]) -> f64 {
        let mut v = (a[0] as i16) as f64;
        let mut scale = 1.0f64;
        for &d in &a[1..] {
            scale /= 65536.0;
            v += d as f64 * scale;
        }
        v
    }
}
