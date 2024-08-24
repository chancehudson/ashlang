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

use super::FieldElement;
use num_bigint::BigUint;
use twenty_first::math::b_field_element::BFieldElement;
use twenty_first::math::traits::Inverse;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
pub struct FoiFieldElement(BFieldElement);

impl FieldElement for FoiFieldElement {
    fn zero() -> Self {
        Self(BFieldElement::from(0))
    }

    fn one() -> Self {
        Self(BFieldElement::from(1))
    }

    fn prime() -> num_bigint::BigUint {
        num_bigint::BigUint::from(BFieldElement::P)
    }

    fn serialize(&self) -> String {
        self.0.value().to_string()
    }

    fn deserialize(str: &str) -> Self {
        Self(BFieldElement::from_str(str).unwrap())
    }
}

impl PartialOrd for FoiFieldElement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a = BigUint::from_str(&self.serialize()).unwrap();
        let b = BigUint::from_str(&other.serialize()).unwrap();
        if a == b {
            Some(std::cmp::Ordering::Equal)
        } else if a < b {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Greater)
        }
    }
}

impl Display for FoiFieldElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for FoiFieldElement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(FoiFieldElement(
            BFieldElement::from_str(s).unwrap(),
        )))
    }
}

impl From<u64> for FoiFieldElement {
    fn from(value: u64) -> Self {
        Self::from(FoiFieldElement(BFieldElement::from(value)))
    }
}

impl Add for FoiFieldElement {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::from(FoiFieldElement(self.0 + other.0))
    }
}

impl Sub for FoiFieldElement {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::from(FoiFieldElement(self.0 - other.0))
    }
}

impl Mul for FoiFieldElement {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::from(FoiFieldElement(self.0 * other.0))
    }
}

impl Div for FoiFieldElement {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self::from(FoiFieldElement(self.0 * other.0.inverse()))
    }
}

impl AddAssign for FoiFieldElement {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl MulAssign for FoiFieldElement {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl SubAssign for FoiFieldElement {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Neg for FoiFieldElement {
    type Output = Self;

    fn neg(self) -> Self {
        Self::from(FoiFieldElement(-self.0))
    }
}
