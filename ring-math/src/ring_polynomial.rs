use scalarff::BigUint;
use scalarff::FieldElement;

use super::norms::Norm;
use super::polynomial::Polynomial;
use super::RING_DEGREE;

/// A concrete implementation of the polynomial ring
/// defined as `R_q[X]/<X^64 + 1>`
/// where q is the prime defined by `T::prime()`
///
/// This polynomial ring can be considered a field as long as it's
/// irreducible over the chosen base field T
#[derive(Clone, Debug, PartialEq, Eq, std::hash::Hash)]
pub struct RingPolynomial<T: FieldElement>(pub Polynomial<T>);

impl<T: FieldElement> RingPolynomial<T> {
    pub fn modulus() -> Self {
        let mut p = Polynomial::identity();
        p.term(&T::one(), RING_DEGREE);
        RingPolynomial(p)
    }

    pub fn degree() -> usize {
        RING_DEGREE
    }
}

impl<T: FieldElement> Norm for RingPolynomial<T> {
    /// Calculate the l1 norm for this polynomial. That is
    /// the summation of all coefficients
    fn norm_l1(&self) -> u64 {
        let digits = self
            .0
            .coefficients
            .iter()
            .fold(T::zero(), |acc, x| acc + x.clone())
            .to_biguint()
            .to_u64_digits();
        if digits.len() > 1 {
            panic!("Norm l1 is not a single u64 digit");
        } else if digits.len() == 1 {
            digits[0]
        } else {
            0
        }
    }

    /// Calculate the l2 norm for this polynomial. That is
    /// the square root of the summation of each coefficient squared
    ///
    /// Specifically, we're calculating the square root in the integer
    /// field, not the prime field
    fn norm_l2(&self) -> u64 {
        let v = self
            .0
            .coefficients
            .iter()
            .fold(T::zero(), |acc, x| acc + (x.clone() * x.clone()));
        let digits = v.to_biguint().sqrt().to_u64_digits();
        if digits.len() > 1 {
            panic!("Norm l2 is not a single u64 digit");
        } else if digits.len() == 1 {
            digits[0]
        } else {
            0
        }
    }

    /// Calculate the l-infinity norm for this polynomial. That is
    /// the largest coefficient
    fn norm_max(&self) -> u64 {
        let mut max = T::zero().to_biguint();
        for i in &self.0.coefficients {
            if i.to_biguint() > max {
                max = i.to_biguint();
            }
        }
        let digits = max.to_u64_digits();
        if digits.len() > 1 {
            panic!("Norm max is not a single u64 digit");
        } else if digits.len() == 1 {
            digits[0]
        } else {
            0
        }
    }
}

impl<T: FieldElement> From<Polynomial<T>> for RingPolynomial<T> {
    fn from(p: Polynomial<T>) -> Self {
        RingPolynomial(p.div(&Self::modulus().0).1)
    }
}

impl<T: FieldElement> FieldElement for RingPolynomial<T> {
    fn zero() -> Self {
        RingPolynomial(Polynomial {
            coefficients: vec![T::zero()],
        })
    }

    fn one() -> Self {
        RingPolynomial(Polynomial::identity())
    }

    fn byte_len() -> usize {
        RING_DEGREE * T::byte_len()
    }

    fn serialize(&self) -> String {
        self.0
            .coefficients
            .iter()
            .map(|v| v.serialize())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn deserialize(str: &str) -> Self {
        Self(Polynomial {
            coefficients: str
                .split(',')
                .map(|v| T::deserialize(v))
                .collect::<Vec<_>>(),
        })
    }

    fn prime() -> BigUint {
        panic!("cannot retrieve a scalar prime for a polynomial field");
    }

    fn name_str() -> &'static str {
        "polynomial_ring"
    }

    /// Return a constant polynomial with the provided
    /// value
    fn from_usize(value: usize) -> Self {
        RingPolynomial(Polynomial {
            coefficients: vec![T::from_usize(value)],
        })
    }

    fn to_biguint(&self) -> BigUint {
        panic!("cannot retrieve a scalar representation for a polynomial field element");
    }

    fn from_biguint(v: &BigUint) -> Self {
        panic!();
    }

    fn from_bytes_le(bytes: &[u8]) -> Self {
        Self(Polynomial {
            coefficients: bytes
                .chunks(T::byte_len())
                .map(|chunk| T::from_bytes_le(chunk))
                .collect::<Vec<_>>(),
        })
    }

    fn to_bytes_le(&self) -> Vec<u8> {
        self.0
            .coefficients
            .iter()
            .flat_map(|v| v.to_bytes_le())
            .collect()
    }
}

impl<T: FieldElement> std::fmt::Display for RingPolynomial<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: FieldElement> std::str::FromStr for RingPolynomial<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
    }
}

impl<T: FieldElement> From<u64> for RingPolynomial<T> {
    fn from(value: u64) -> Self {
        RingPolynomial::from(Polynomial {
            coefficients: vec![T::from(value)],
        })
    }
}

impl<T: FieldElement> std::ops::Add for RingPolynomial<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        RingPolynomial::from(self.0 + other.0)
    }
}

impl<T: FieldElement> std::ops::Sub for RingPolynomial<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        RingPolynomial::from(self.0 - other.0)
    }
}

impl<T: FieldElement> std::ops::Mul for RingPolynomial<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        RingPolynomial::from(self.0 * other.0)
    }
}

impl<T: FieldElement> std::ops::Div for RingPolynomial<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        // this implementation implies floored division, so discard the remainder
        RingPolynomial::from(self.0.div(&other.0).0)
    }
}

impl<T: FieldElement> std::ops::AddAssign for RingPolynomial<T> {
    fn add_assign(&mut self, other: Self) {
        *self = self.clone() + other;
    }
}

impl<T: FieldElement> std::ops::MulAssign for RingPolynomial<T> {
    fn mul_assign(&mut self, other: Self) {
        *self = self.clone() * other;
    }
}

impl<T: FieldElement> std::ops::SubAssign for RingPolynomial<T> {
    fn sub_assign(&mut self, other: Self) {
        *self = self.clone() - other;
    }
}

impl<T: FieldElement> std::ops::Neg for RingPolynomial<T> {
    type Output = Self;

    fn neg(self) -> Self {
        RingPolynomial(-self.0.clone())
    }
}
