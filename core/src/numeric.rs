use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use num_bigint::BigInt as BigInteger;
use num_rational::Ratio as NumRatio;
use num_traits::{One, ToPrimitive, Zero};

// ============================================================================
// Numeric Type System
// ============================================================================

#[derive(Debug, Clone)]
pub enum NumericType {
    /// Primary integer type - promotes to BigInt on overflow
    Int(i64),

    /// Arbitrary precision integer
    BigInt(Arc<BigInteger>),

    /// Exact rational number (numerator, denominator in reduced form)
    Ratio(i64, i64),

    /// Arbitrary precision rational
    BigRatio(Arc<NumRatio<BigInteger>>),

    /// IEEE 754 double precision floating point
    Float(f64),
}

// ============================================================================
// Display Implementation
// ============================================================================

impl fmt::Display for NumericType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NumericType::Int(n) => write!(f, "{n}"),
            NumericType::BigInt(n) => write!(f, "{n}"),
            NumericType::Ratio(num, denom) => write!(f, "{num}/{denom}"),
            NumericType::BigRatio(r) => {
                let numer = r.numer();
                let denom = r.denom();
                write!(f, "{numer}/{denom}")
            }
            NumericType::Float(x) => {
                if x.is_nan() {
                    write!(f, "NaN")
                } else if x.is_infinite() {
                    let sign = if *x > 0.0 { "+Inf" } else { "-Inf" };
                    write!(f, "{sign}")
                } else {
                    write!(f, "{x}")
                }
            }
        }
    }
}

// ============================================================================
// Equality and Comparison
// ============================================================================

impl PartialEq for NumericType {
    fn eq(&self, other: &Self) -> bool {
        use NumericType::*;

        match (self, other) {
            // Same types
            (Int(a), Int(b)) => a == b,
            (BigInt(a), BigInt(b)) => a == b,
            (Ratio(an, ad), Ratio(bn, bd)) => an == bn && ad == bd,
            (BigRatio(a), BigRatio(b)) => a == b,
            (Float(a), Float(b)) => a == b,

            // Cross-type comparisons
            (Int(a), BigInt(b)) => &BigInteger::from(*a) == b.as_ref(),
            (BigInt(a), Int(b)) => a.as_ref() == &BigInteger::from(*b),

            (Int(a), Ratio(bn, bd)) => a * bd == *bn,
            (Ratio(an, ad), Int(b)) => *an == b * ad,

            (Int(a), Float(b)) => (*a as f64) == *b,
            (Float(a), Int(b)) => *a == (*b as f64),

            (Ratio(an, ad), Float(b)) => (*an as f64) / (*ad as f64) == *b,
            (Float(a), Ratio(bn, bd)) => *a == (*bn as f64) / (*bd as f64),

            // Add more cross-type comparisons as needed
            _ => false,
        }
    }
}

impl Eq for NumericType {}

impl Hash for NumericType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use NumericType::*;
        std::mem::discriminant(self).hash(state);
        match self {
            Int(n) => n.hash(state),
            BigInt(n) => n.to_string().hash(state),
            Ratio(num, denom) => {
                num.hash(state);
                denom.hash(state);
            }
            BigRatio(r) => {
                r.numer().to_string().hash(state);
                r.denom().to_string().hash(state);
            }
            Float(x) => {
                // Use bit representation for deterministic hashing
                // NaN values all hash to the same value
                if x.is_nan() {
                    u64::MAX.hash(state);
                } else {
                    x.to_bits().hash(state);
                }
            }
        }
    }
}

impl PartialOrd for NumericType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use NumericType::*;

        match (self, other) {
            // Same types
            (Int(a), Int(b)) => a.partial_cmp(b),
            (BigInt(a), BigInt(b)) => a.partial_cmp(b),
            (Float(a), Float(b)) => a.partial_cmp(b),

            // Ratios: a/b < c/d iff ad < bc (assuming positive denominators)
            (Ratio(an, ad), Ratio(bn, bd)) => (an * bd).partial_cmp(&(bn * ad)),

            // Cross-type comparisons
            (Int(a), Float(b)) => (*a as f64).partial_cmp(b),
            (Float(a), Int(b)) => a.partial_cmp(&(*b as f64)),

            (Int(a), Ratio(bn, bd)) => (a * bd).partial_cmp(bn),
            (Ratio(an, ad), Int(b)) => an.partial_cmp(&(b * ad)),

            (Ratio(an, ad), Float(b)) => ((*an as f64) / (*ad as f64)).partial_cmp(b),
            (Float(a), Ratio(bn, bd)) => a.partial_cmp(&((*bn as f64) / (*bd as f64))),

            // BigInt comparisons
            (Int(a), BigInt(b)) => BigInteger::from(*a).partial_cmp(b),
            (BigInt(a), Int(b)) => a.as_ref().partial_cmp(&BigInteger::from(*b)),

            _ => None,
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

impl NumericType {
    /// Create a ratio in reduced form
    pub fn make_ratio(num: i64, denom: i64) -> Result<NumericType, String> {
        if denom == 0 {
            return Err("Division by zero".to_string());
        }

        let g = gcd(num, denom);
        let (mut num, mut denom) = (num / g, denom / g);

        // Normalize: denominator always positive
        if denom < 0 {
            num = -num;
            denom = -denom;
        }

        // Reduce to integer if possible
        if denom == 1 {
            Ok(NumericType::Int(num))
        } else {
            Ok(NumericType::Ratio(num, denom))
        }
    }

    /// Convert to float (may lose precision)
    pub fn to_float(&self) -> f64 {
        match self {
            NumericType::Int(n) => *n as f64,
            NumericType::BigInt(n) => n.to_f64().unwrap_or(f64::INFINITY),
            NumericType::Ratio(num, denom) => (*num as f64) / (*denom as f64),
            NumericType::BigRatio(r) => {
                r.numer().to_f64().unwrap_or(0.0) / r.denom().to_f64().unwrap_or(1.0)
            }
            NumericType::Float(x) => *x,
        }
    }

    /// Check if number is zero
    pub fn is_zero(&self) -> bool {
        match self {
            NumericType::Int(n) => *n == 0,
            NumericType::BigInt(n) => n.is_zero(),
            NumericType::Ratio(num, _) => *num == 0,
            NumericType::BigRatio(r) => r.is_zero(),
            NumericType::Float(x) => *x == 0.0,
        }
    }
}

// ============================================================================
// Arithmetic Operations
// ============================================================================

impl NumericType {
    /// Addition with automatic type promotion
    pub fn add(&self, other: &NumericType) -> Result<NumericType, String> {
        use NumericType::*;

        match (self, other) {
            // Int + Int with overflow check
            (Int(a), Int(b)) => match a.checked_add(*b) {
                Some(result) => Ok(Int(result)),
                None => {
                    // Promote to BigInt on overflow
                    let big_a = BigInteger::from(*a);
                    let big_b = BigInteger::from(*b);
                    Ok(BigInt(Arc::new(big_a + big_b)))
                }
            },

            // Int + BigInt
            (Int(a), BigInt(b)) => Ok(BigInt(Arc::new(BigInteger::from(*a) + b.as_ref()))),
            (BigInt(a), Int(b)) => Ok(BigInt(Arc::new(a.as_ref() + BigInteger::from(*b)))),

            // BigInt + BigInt
            (BigInt(a), BigInt(b)) => Ok(BigInt(Arc::new(a.as_ref() + b.as_ref()))),

            // Ratio + Ratio: a/b + c/d = (ad + bc) / bd
            (Ratio(an, ad), Ratio(bn, bd)) => {
                let num = match an
                    .checked_mul(*bd)
                    .and_then(|x| bn.checked_mul(*ad).and_then(|y| x.checked_add(y)))
                {
                    Some(n) => n,
                    None => {
                        // Overflow: promote to BigRatio
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big + b_big)));
                    }
                };

                let denom = match ad.checked_mul(*bd) {
                    Some(d) => d,
                    None => {
                        // Overflow: promote to BigRatio
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big + b_big)));
                    }
                };

                Self::make_ratio(num, denom)
            }

            // Int + Ratio: a + c/d = (ad + c) / d
            (Int(a), Ratio(bn, bd)) => {
                let num = match a.checked_mul(*bd).and_then(|x| x.checked_add(*bn)) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*a), BigInteger::one());
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big + b_big)));
                    }
                };
                Self::make_ratio(num, *bd)
            }
            (Ratio(an, ad), Int(b)) => {
                let num = match b.checked_mul(*ad).and_then(|x| x.checked_add(*an)) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*b), BigInteger::one());
                        return Ok(BigRatio(Arc::new(a_big + b_big)));
                    }
                };
                Self::make_ratio(num, *ad)
            }

            // Float operations
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Int(a), Float(b)) => Ok(Float(*a as f64 + b)),
            (Float(a), Int(b)) => Ok(Float(a + *b as f64)),
            (Ratio(an, ad), Float(b)) => Ok(Float((*an as f64) / (*ad as f64) + b)),
            (Float(a), Ratio(bn, bd)) => Ok(Float(a + (*bn as f64) / (*bd as f64))),

            // BigRatio operations
            (BigRatio(a), BigRatio(b)) => Ok(BigRatio(Arc::new(a.as_ref() + b.as_ref()))),

            _ => Err(format!("Unsupported addition: {self} + {other}")),
        }
    }

    /// Subtraction with automatic type promotion
    pub fn sub(&self, other: &NumericType) -> Result<NumericType, String> {
        use NumericType::*;

        match (self, other) {
            // Int - Int with overflow check
            (Int(a), Int(b)) => match a.checked_sub(*b) {
                Some(result) => Ok(Int(result)),
                None => {
                    let big_a = BigInteger::from(*a);
                    let big_b = BigInteger::from(*b);
                    Ok(BigInt(Arc::new(big_a - big_b)))
                }
            },

            // Int - BigInt
            (Int(a), BigInt(b)) => Ok(BigInt(Arc::new(BigInteger::from(*a) - b.as_ref()))),
            (BigInt(a), Int(b)) => Ok(BigInt(Arc::new(a.as_ref() - BigInteger::from(*b)))),

            // BigInt - BigInt
            (BigInt(a), BigInt(b)) => Ok(BigInt(Arc::new(a.as_ref() - b.as_ref()))),

            // Ratio - Ratio: a/b - c/d = (ad - bc) / bd
            (Ratio(an, ad), Ratio(bn, bd)) => {
                let num = match an
                    .checked_mul(*bd)
                    .and_then(|x| bn.checked_mul(*ad).and_then(|y| x.checked_sub(y)))
                {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big - b_big)));
                    }
                };

                let denom = match ad.checked_mul(*bd) {
                    Some(d) => d,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big - b_big)));
                    }
                };

                Self::make_ratio(num, denom)
            }

            // Int - Ratio
            (Int(a), Ratio(bn, bd)) => {
                let num = match a.checked_mul(*bd).and_then(|x| x.checked_sub(*bn)) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*a), BigInteger::one());
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big - b_big)));
                    }
                };
                Self::make_ratio(num, *bd)
            }
            (Ratio(an, ad), Int(b)) => {
                let num = match b.checked_mul(*ad).and_then(|x| an.checked_sub(x)) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*b), BigInteger::one());
                        return Ok(BigRatio(Arc::new(a_big - b_big)));
                    }
                };
                Self::make_ratio(num, *ad)
            }

            // Float operations
            (Float(a), Float(b)) => Ok(Float(a - b)),
            (Int(a), Float(b)) => Ok(Float(*a as f64 - b)),
            (Float(a), Int(b)) => Ok(Float(a - *b as f64)),
            (Ratio(an, ad), Float(b)) => Ok(Float((*an as f64) / (*ad as f64) - b)),
            (Float(a), Ratio(bn, bd)) => Ok(Float(a - (*bn as f64) / (*bd as f64))),

            // BigRatio operations
            (BigRatio(a), BigRatio(b)) => Ok(BigRatio(Arc::new(a.as_ref() - b.as_ref()))),

            _ => Err(format!("Unsupported subtraction: {self} - {other}")),
        }
    }

    /// Multiplication with automatic type promotion
    pub fn mul(&self, other: &NumericType) -> Result<NumericType, String> {
        use NumericType::*;

        match (self, other) {
            // Int * Int with overflow check
            (Int(a), Int(b)) => match a.checked_mul(*b) {
                Some(result) => Ok(Int(result)),
                None => {
                    let big_a = BigInteger::from(*a);
                    let big_b = BigInteger::from(*b);
                    Ok(BigInt(Arc::new(big_a * big_b)))
                }
            },

            // Int * BigInt
            (Int(a), BigInt(b)) => Ok(BigInt(Arc::new(BigInteger::from(*a) * b.as_ref()))),
            (BigInt(a), Int(b)) => Ok(BigInt(Arc::new(a.as_ref() * BigInteger::from(*b)))),

            // BigInt * BigInt
            (BigInt(a), BigInt(b)) => Ok(BigInt(Arc::new(a.as_ref() * b.as_ref()))),

            // Ratio * Ratio: a/b * c/d = ac / bd
            (Ratio(an, ad), Ratio(bn, bd)) => {
                let num = match an.checked_mul(*bn) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big * b_big)));
                    }
                };

                let denom = match ad.checked_mul(*bd) {
                    Some(d) => d,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big * b_big)));
                    }
                };

                Self::make_ratio(num, denom)
            }

            // Int * Ratio
            (Int(a), Ratio(bn, bd)) => {
                let num = match a.checked_mul(*bn) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*a), BigInteger::one());
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big * b_big)));
                    }
                };
                Self::make_ratio(num, *bd)
            }
            (Ratio(an, ad), Int(b)) => {
                let num = match an.checked_mul(*b) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*b), BigInteger::one());
                        return Ok(BigRatio(Arc::new(a_big * b_big)));
                    }
                };
                Self::make_ratio(num, *ad)
            }

            // Float operations
            (Float(a), Float(b)) => Ok(Float(a * b)),
            (Int(a), Float(b)) => Ok(Float(*a as f64 * b)),
            (Float(a), Int(b)) => Ok(Float(a * *b as f64)),
            (Ratio(an, ad), Float(b)) => Ok(Float((*an as f64) / (*ad as f64) * b)),
            (Float(a), Ratio(bn, bd)) => Ok(Float(a * (*bn as f64) / (*bd as f64))),

            // BigRatio operations
            (BigRatio(a), BigRatio(b)) => Ok(BigRatio(Arc::new(a.as_ref() * b.as_ref()))),

            _ => Err(format!("Unsupported multiplication: {self} * {other}")),
        }
    }

    /// Division - returns exact ratio when possible
    pub fn div(&self, other: &NumericType) -> Result<NumericType, String> {
        if other.is_zero() {
            return Err("Division by zero".to_string());
        }

        use NumericType::*;

        match (self, other) {
            // Int / Int - return exact ratio if not evenly divisible
            (Int(a), Int(b)) => {
                if a % b == 0 {
                    Ok(Int(a / b))
                } else {
                    Self::make_ratio(*a, *b)
                }
            }

            // Ratio / Ratio: (a/b) / (c/d) = (ad) / (bc)
            (Ratio(an, ad), Ratio(bn, bd)) => {
                let num = match an.checked_mul(*bd) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big / b_big)));
                    }
                };

                let denom = match ad.checked_mul(*bn) {
                    Some(d) => d,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big / b_big)));
                    }
                };

                Self::make_ratio(num, denom)
            }

            // Int / Ratio: a / (c/d) = ad / c
            (Int(a), Ratio(bn, bd)) => {
                let num = match a.checked_mul(*bd) {
                    Some(n) => n,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*a), BigInteger::one());
                        let b_big = NumRatio::new(BigInteger::from(*bn), BigInteger::from(*bd));
                        return Ok(BigRatio(Arc::new(a_big / b_big)));
                    }
                };
                Self::make_ratio(num, *bn)
            }

            // Ratio / Int: (a/b) / c = a / (bc)
            (Ratio(an, ad), Int(b)) => {
                let denom = match ad.checked_mul(*b) {
                    Some(d) => d,
                    None => {
                        let a_big = NumRatio::new(BigInteger::from(*an), BigInteger::from(*ad));
                        let b_big = NumRatio::new(BigInteger::from(*b), BigInteger::one());
                        return Ok(BigRatio(Arc::new(a_big / b_big)));
                    }
                };
                Self::make_ratio(*an, denom)
            }

            // Float operations
            (Float(a), Float(b)) => Ok(Float(a / b)),
            (Int(a), Float(b)) => Ok(Float(*a as f64 / b)),
            (Float(a), Int(b)) => Ok(Float(a / *b as f64)),
            (Ratio(an, ad), Float(b)) => Ok(Float((*an as f64) / (*ad as f64) / b)),
            (Float(a), Ratio(bn, bd)) => Ok(Float(a / ((*bn as f64) / (*bd as f64)))),

            // BigInt operations
            (BigInt(a), BigInt(b)) => {
                let ratio = NumRatio::new(a.as_ref().clone(), b.as_ref().clone());
                if ratio.is_integer() {
                    Ok(BigInt(Arc::new(ratio.numer().clone())))
                } else {
                    Ok(BigRatio(Arc::new(ratio)))
                }
            }

            (Int(a), BigInt(b)) => Ok(BigRatio(Arc::new(NumRatio::new(
                BigInteger::from(*a),
                b.as_ref().clone(),
            )))),

            (BigInt(a), Int(b)) => Ok(BigRatio(Arc::new(NumRatio::new(
                a.as_ref().clone(),
                BigInteger::from(*b),
            )))),

            // BigRatio operations
            (BigRatio(a), BigRatio(b)) => Ok(BigRatio(Arc::new(a.as_ref() / b.as_ref()))),

            _ => Err(format!("Unsupported division: {self} / {other}")),
        }
    }

    /// Negation
    pub fn neg(&self) -> Result<NumericType, String> {
        use NumericType::*;

        match self {
            Int(n) => match n.checked_neg() {
                Some(result) => Ok(Int(result)),
                None => Ok(BigInt(Arc::new(-BigInteger::from(*n)))),
            },
            BigInt(n) => Ok(BigInt(Arc::new(-n.as_ref()))),
            Ratio(num, denom) => Ok(Ratio(-num, *denom)),
            BigRatio(r) => Ok(BigRatio(Arc::new(-r.as_ref()))),
            Float(x) => Ok(Float(-x)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_arithmetic() {
        let a = NumericType::Int(5);
        let b = NumericType::Int(3);

        assert_eq!(a.add(&b).unwrap(), NumericType::Int(8));
        assert_eq!(a.sub(&b).unwrap(), NumericType::Int(2));
        assert_eq!(a.mul(&b).unwrap(), NumericType::Int(15));
    }

    #[test]
    fn test_int_overflow() {
        let a = NumericType::Int(i64::MAX);
        let b = NumericType::Int(1);

        // Should promote to BigInt on overflow
        match a.add(&b).unwrap() {
            NumericType::BigInt(_) => {}
            _ => panic!("Expected BigInt promotion on overflow"),
        }
    }

    #[test]
    fn test_exact_division() {
        let a = NumericType::Int(5);
        let b = NumericType::Int(2);

        // 5/2 should return Ratio(5, 2), not 2
        assert_eq!(a.div(&b).unwrap(), NumericType::Ratio(5, 2));

        // 6/2 should return Int(3)
        let c = NumericType::Int(6);
        assert_eq!(c.div(&b).unwrap(), NumericType::Int(3));
    }

    #[test]
    fn test_ratio_reduction() {
        // 6/9 should reduce to 2/3
        let r = NumericType::make_ratio(6, 9).unwrap();
        assert_eq!(r, NumericType::Ratio(2, 3));

        // 10/5 should reduce to Int(2)
        let r2 = NumericType::make_ratio(10, 5).unwrap();
        assert_eq!(r2, NumericType::Int(2));
    }

    #[test]
    fn test_ratio_arithmetic() {
        let a = NumericType::Ratio(1, 2); // 1/2
        let b = NumericType::Ratio(1, 3); // 1/3

        // 1/2 + 1/3 = 5/6
        assert_eq!(a.add(&b).unwrap(), NumericType::Ratio(5, 6));

        // 1/2 * 1/3 = 1/6
        assert_eq!(a.mul(&b).unwrap(), NumericType::Ratio(1, 6));
    }

    #[test]
    fn test_division_by_zero() {
        let a = NumericType::Int(5);
        let zero = NumericType::Int(0);

        assert!(a.div(&zero).is_err());
    }

    #[test]
    fn test_float_operations() {
        let a = NumericType::Float(3.15);
        let b = NumericType::Float(2.0);

        if let NumericType::Float(result) = a.add(&b).unwrap() {
            assert!((result - 5.15).abs() < 1e-10);
        } else {
            panic!("Expected Float result");
        }
    }

    #[test]
    fn test_cross_type_comparison() {
        let int_five = NumericType::Int(5);
        let ratio_five = NumericType::Ratio(10, 2);
        let float_five = NumericType::Float(5.0);

        assert_eq!(int_five, ratio_five);
        assert_eq!(int_five, float_five);
    }

    #[test]
    fn test_ratio_minus_int_overflow() {
        // Test the specific overflow case in Ratio - Int subtraction
        // When b * ad overflows, we should promote to BigRatio

        // Use large values that will cause overflow when multiplied
        let large_num = i64::MAX / 2;
        let large_denom = 3i64;
        let large_int = i64::MAX / 2;

        let ratio = NumericType::Ratio(large_num, large_denom);
        let int = NumericType::Int(large_int);

        // This should not panic and should return BigRatio
        let result = ratio.sub(&int);
        assert!(result.is_ok());

        // Verify the result is BigRatio (since multiplication would overflow)
        match result.unwrap() {
            NumericType::BigRatio(_) => {
                // Correct - promoted to BigRatio
            }
            other => panic!("Expected BigRatio for overflow case, got {other:?}"),
        }
    }

    #[test]
    fn test_ratio_minus_int_no_overflow() {
        // Test case where Ratio - Int doesn't overflow
        let ratio = NumericType::Ratio(10, 3);
        let int = NumericType::Int(2);

        // 10/3 - 2 = 10/3 - 6/3 = 4/3
        let result = ratio.sub(&int).unwrap();
        assert_eq!(result, NumericType::Ratio(4, 3));
    }

    #[test]
    fn test_ratio_arithmetic_overflow_consistency() {
        // Ensure all overflow paths work consistently
        let large = i64::MAX / 2;

        // Test Ratio + Int overflow
        let r1 = NumericType::Ratio(large, 2);
        let i1 = NumericType::Int(large);
        assert!(r1.add(&i1).is_ok());

        // Test Ratio - Int overflow
        let r2 = NumericType::Ratio(large, 3);
        let i2 = NumericType::Int(large);
        assert!(r2.sub(&i2).is_ok());

        // Both should produce BigRatio
        match r1.add(&i1).unwrap() {
            NumericType::BigRatio(_) => {}
            other => panic!("Expected BigRatio, got {other:?}"),
        }

        match r2.sub(&i2).unwrap() {
            NumericType::BigRatio(_) => {}
            other => panic!("Expected BigRatio, got {other:?}"),
        }
    }
}
