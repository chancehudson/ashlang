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

pub mod alt_bn128;
pub mod curve_25519;
pub mod foi;
pub mod matrix;

pub trait FieldElement:
    Add<Output = Self>
    + AddAssign
    + Div<Output = Self>
    + Mul<Output = Self>
    + MulAssign
    + Neg<Output = Self>
    + Sub<Output = Self>
    + SubAssign
    + FromStr
    + PartialEq
    + Clone
    + Hash
    + Debug
    + From<u64>
    + Display
{
    fn one() -> Self;
    fn zero() -> Self;
    fn serialize(&self) -> String;
    fn deserialize(str: &str) -> Self;
}
