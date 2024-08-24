use super::FieldElement;
use curve25519_dalek::scalar::Scalar;
use ff::PrimeField;
use num_bigint::BigUint;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Curve25519FieldElement(Scalar);

impl FieldElement for Curve25519FieldElement {
    fn zero() -> Self {
        Self::from(Curve25519FieldElement(Scalar::ZERO))
    }

    fn one() -> Self {
        Self::from(Curve25519FieldElement(Scalar::ONE))
    }

    fn prime() -> BigUint {
        // the modulus returned by this implementation is a hex string
        // BigUint doesn't like that so we do some nonsense to calculate
        // the prime
        BigUint::from_str(&(-Self::one()).to_string()).unwrap() + 1_u32
    }

    fn serialize(&self) -> String {
        self.clone().to_string()
    }

    fn deserialize(str: &str) -> Self {
        Self::from_str(str).unwrap()
    }
}

impl Display for Curve25519FieldElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", BigUint::from_bytes_le(self.0.as_bytes()))
    }
}

impl FromStr for Curve25519FieldElement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(Curve25519FieldElement(
            Scalar::from_str_vartime(s).unwrap(),
        )))
    }
}

impl From<u64> for Curve25519FieldElement {
    fn from(value: u64) -> Self {
        Self::from(Curve25519FieldElement(Scalar::from(value)))
    }
}

impl Add for Curve25519FieldElement {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::from(Curve25519FieldElement(self.0 + other.0))
    }
}

impl Sub for Curve25519FieldElement {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::from(Curve25519FieldElement(self.0 - other.0))
    }
}

impl Mul for Curve25519FieldElement {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::from(Curve25519FieldElement(self.0 * other.0))
    }
}

impl Div for Curve25519FieldElement {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self::from(Curve25519FieldElement(self.0 * other.0.invert()))
    }
}

impl AddAssign for Curve25519FieldElement {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl MulAssign for Curve25519FieldElement {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl SubAssign for Curve25519FieldElement {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Neg for Curve25519FieldElement {
    type Output = Self;

    fn neg(self) -> Self {
        Self::from(Curve25519FieldElement(-self.0))
    }
}
