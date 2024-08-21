use super::FieldElement;
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
use twenty_first::math::b_field_element::BFieldElement;

// Wrapper for BFieldElement because i can't figure
// out how to appliy the FieldElement trait to it
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct FoiFieldElement(BFieldElement);

impl FieldElement for FoiFieldElement {
    fn zero() -> Self {
        Self(BFieldElement::from(0))
    }

    fn one() -> Self {
        Self(BFieldElement::from(1))
    }
}

impl From<u64> for FoiFieldElement {
    fn from(input: u64) -> Self {
        Self(BFieldElement::from(input))
    }
}

impl FromStr for FoiFieldElement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(BFieldElement::from_str(s).unwrap()))
    }
}

impl Display for FoiFieldElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.value())
    }
}

impl Add for FoiFieldElement {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl AddAssign for FoiFieldElement {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Div for FoiFieldElement {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl Mul for FoiFieldElement {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl MulAssign for FoiFieldElement {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl Neg for FoiFieldElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(self.0.neg())
    }
}

impl Sub for FoiFieldElement {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl SubAssign for FoiFieldElement {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}
