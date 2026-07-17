// *** floatexp: f64 mantissa + i64 exponent *** //
//
// Extends the perturbation engine's dynamic range past f64's ~1e-308 floor.
// Deep-zoom pixel offsets (dc ~ 1e-360 at 360 digits) are unrepresentable in
// f64, but a pixel's delta only GROWS from dc, so only the head of each delta
// orbit needs the extended range -- once |d| climbs into normal f64 range the
// existing f64 engine finishes the pixel (see bla_drive_fe). FloatExp is the
// scalar for that head phase and for the dx/dy/dc plumbing that feeds it.
//
// Representation: value = m * 2^e with m normalized to +/-[1, 2) (or exactly
// 0.0 with e = i64::MIN). Normalization is pure exponent redistribution --
// exact, no rounding -- so a chain of FloatExp ops rounds exactly like the
// same chain of f64 ops on values scaled into normal range. That property is
// what keeps the shallow dispatch path bit-identical to the old all-f64 code
// after its arguments round-trip through FloatExp.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatExp {
    pub m: f64,
    pub e: i64,
}

pub const FE_ZERO: FloatExp = FloatExp { m: 0.0, e: i64::MIN };

const EXP_MASK: u64 = 0x7ff_u64 << 52;
const EXP_BIAS: i64 = 1023;

#[inline]
fn renorm(m: f64, e: i64) -> FloatExp {
    if m == 0.0 {
        return FE_ZERO;
    }
    let bits = m.to_bits();
    let be = ((bits >> 52) & 0x7ff) as i64;
    debug_assert!(be != 0 && be != 0x7ff, "renorm input must be normal: {m:e}");
    FloatExp { m: f64::from_bits((bits & !EXP_MASK) | ((EXP_BIAS as u64) << 52)), e: e + be - EXP_BIAS }
}

// 2^i for |i| <= 53: exact in f64 (used only for exponent alignment in add)
#[inline]
fn pow2(i: i64) -> f64 {
    f64::from_bits(((EXP_BIAS + i) as u64) << 52)
}

impl FloatExp {
    /// From an f64 (any finite value, including subnormals).
    #[inline]
    pub fn from_f64(v: f64) -> FloatExp {
        if v == 0.0 {
            return FE_ZERO;
        }
        if (v.to_bits() & EXP_MASK) == 0 {
            // subnormal: lift into normal range first (exact scaling)
            return renorm(v * pow2(200) * pow2(200), -400);
        }
        renorm(v, 0)
    }

    /// From a mantissa-and-exponent pair in any scale (m need not be normalized,
    /// but must be a normal f64 or zero).
    #[inline]
    pub fn new(m: f64, e: i64) -> FloatExp {
        if m == 0.0 {
            FE_ZERO
        } else {
            renorm(m, e)
        }
    }

    /// To f64, rounding into (or through) the subnormal range exactly like a
    /// direct `mant * 2^e` scaling; overflows saturate to +/-inf.
    #[inline]
    pub fn to_f64(self) -> f64 {
        if self.m == 0.0 || self.e < -1200 {
            return 0.0;
        }
        if self.e > 1023 {
            return if self.m > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
        }
        if self.e >= -1022 {
            self.m * f64::from_bits(((EXP_BIAS + self.e) as u64) << 52)
        } else {
            // split the scaling so the intermediate stays normal; the final
            // multiply correctly rounds into the subnormal range (same shape
            // as mag_to_f64's two-step scaling)
            (self.m * f64::from_bits(1_u64 << 52))
                * f64::from_bits(((EXP_BIAS + self.e + 1022) as u64) << 52)
        }
    }

    #[inline]
    pub fn mul(self, b: FloatExp) -> FloatExp {
        if self.m == 0.0 || b.m == 0.0 {
            return FE_ZERO;
        }
        renorm(self.m * b.m, self.e + b.e)
    }

    /// Multiply by a plain f64 (any finite value).
    #[inline]
    pub fn mul_f64(self, s: f64) -> FloatExp {
        self.mul(FloatExp::from_f64(s))
    }

    #[inline]
    pub fn add(self, b: FloatExp) -> FloatExp {
        if self.m == 0.0 {
            return b;
        }
        if b.m == 0.0 {
            return self;
        }
        let de = self.e - b.e;
        if de >= 54 {
            self
        } else if de <= -54 {
            b
        } else if de >= 0 {
            renorm(self.m + b.m * pow2(-de), self.e)
        } else {
            renorm(b.m + self.m * pow2(de), b.e)
        }
    }

    #[inline]
    pub fn sub(self, b: FloatExp) -> FloatExp {
        self.add(FloatExp { m: -b.m, e: b.e })
    }

    #[inline]
    pub fn neg(self) -> FloatExp {
        FloatExp { m: -self.m, e: self.e }
    }

    /// |self| < |b| (magnitude compare; exact)
    #[inline]
    pub fn mag_lt(self, b: FloatExp) -> bool {
        if self.m == 0.0 {
            return b.m != 0.0;
        }
        if b.m == 0.0 {
            return false;
        }
        if self.e != b.e {
            self.e < b.e
        } else {
            self.m.abs() < b.m.abs()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fe(v: f64) -> FloatExp {
        FloatExp::from_f64(v)
    }

    #[test]
    fn roundtrip_and_ops_match_f64_in_range() {
        let vals = [1.5, -0.0625, 3.7e-105, -2.9e200, 1.0e-310, 4.9e-324, 0.0, 7.25];
        for &a in &vals {
            assert_eq!(fe(a).to_f64(), a, "roundtrip {a:e}");
            for &b in &vals {
                let s = fe(a).add(fe(b)).to_f64();
                assert_eq!(s, a + b, "add {a:e} + {b:e}");
                let p = fe(a).mul(fe(b)).to_f64();
                // product may over/underflow f64; only compare when a*b is normal-ish
                let direct = a * b;
                if direct != 0.0 && direct.is_finite() && direct.abs() > 1e-300 {
                    assert_eq!(p, direct, "mul {a:e} * {b:e}");
                }
                assert_eq!(fe(a).mag_lt(fe(b)), a.abs() < b.abs(), "cmp {a:e} {b:e}");
            }
        }
    }

    #[test]
    fn extended_range() {
        // values far below f64's floor survive and compare correctly
        let tiny = FloatExp::new(1.25, -2000);
        let tinier = FloatExp::new(1.25, -2100);
        assert!(tinier.mag_lt(tiny));
        assert_eq!(tiny.mul(tiny).e, -4000); // 1.5625 * 2^-4000: m stays in [1,2)
        assert_eq!(tiny.to_f64(), 0.0);
        // growth back into range is exact
        let back = tiny.mul(FloatExp::new(1.0, 1900));
        assert_eq!(back.to_f64(), 1.25 * 2.0f64.powi(-100));
        // addition drops a negligible addend, keeps a comparable one
        let sum = tiny.add(tinier);
        assert_eq!(sum.e, -2000);
        assert!((sum.m - (1.25 + 1.25 * 2.0f64.powi(-100))).abs() < 1e-15);
        let kept = FloatExp::new(1.0, -2000).add(FloatExp::new(1.0, -2010));
        assert_eq!(kept.to_f64(), 0.0);
        assert_eq!(kept.e, -2000);
        assert!((kept.m - (1.0 + 2.0f64.powi(-10))).abs() < 1e-15);
    }

    #[test]
    fn subnormal_to_f64_rounds_like_direct_scaling() {
        // mirror of mag_to_f64's two-step subnormal scaling
        let v = FloatExp::new(1.7654321, -1050);
        let direct = (1.7654321 * 2.0f64.powi(-1022)) * 2.0f64.powi(-1050 + 1022);
        assert_eq!(v.to_f64(), direct);
    }
}
