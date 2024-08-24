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

use num_bigint::BigUint;
use num_integer::Integer;

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
    + PartialOrd
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
    fn prime() -> BigUint;

    /// calculate the legendre symbol for a field element
    /// https://en.wikipedia.org/wiki/Legendre_symbol#Definition
    fn legendre(&self) -> i32 {
        if self == &Self::zero() {
            return 0;
        }
        let neg_one = Self::prime() - 1_u32;
        let one = BigUint::from(1_u32);
        let e = (-Self::one()) / (Self::one() + Self::one());
        let e_bigint = BigUint::from_str(&e.serialize()).unwrap();
        let a = BigUint::from_str(&self.serialize()).unwrap();
        let l = a.modpow(&e_bigint, &Self::prime());
        if l == neg_one {
            -1
        } else if l == one {
            return 1;
        } else {
            panic!("legendre symbol is not 1, -1, or 0");
        }
    }

    /// Kumar 08
    /// https://arxiv.org/pdf/2008.11814v4
    /// always returns the smaller root
    /// e.g. the positive root
    fn sqrt(&self) -> Self {
        if self == &Self::zero() {
            return Self::zero();
        }
        if self.legendre() != 1 {
            panic!("legendre symbol is not 1: root does not exist or input is 0");
        }
        // find a non-residue
        let mut x = Self::one() + Self::one();
        let non_residue;
        loop {
            if x.legendre() == -1 {
                non_residue = x.clone();
                break;
            }
            x += Self::one();
        }
        let b = BigUint::from_str(&non_residue.serialize()).unwrap();

        let a = BigUint::from_str(&self.serialize()).unwrap();
        let two = Self::one() + Self::one();
        let m = (-Self::one()) / two.clone();
        let mut apow = -Self::one();
        let mut bpow = Self::zero();
        while BigUint::from_str(&apow.serialize()).unwrap().is_even() {
            apow = apow / two.clone();
            bpow = bpow / two.clone();
            let a_ = a.modpow(
                &BigUint::from_str(&apow.serialize()).unwrap(),
                &Self::prime(),
            );
            let b_ = b.modpow(
                &BigUint::from_str(&bpow.serialize()).unwrap(),
                &Self::prime(),
            );
            if (a_ * b_) % Self::prime() == Self::prime() - 1_u32 {
                bpow += m.clone();
            }
        }
        apow = (apow + Self::one()) / two.clone();
        bpow = bpow / two;
        let a_ = a.modpow(
            &BigUint::from_str(&apow.serialize()).unwrap(),
            &Self::prime(),
        );
        let b_ = b.modpow(
            &BigUint::from_str(&bpow.serialize()).unwrap(),
            &Self::prime(),
        );
        let o = (a_ * b_) % Self::prime();
        let root = Self::deserialize(&o.to_string());
        let other_root = -root.clone();
        if root > other_root {
            other_root
        } else {
            root
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alt_bn128::Bn128FieldElement;
    use curve_25519::Curve25519FieldElement;
    use foi::FoiFieldElement;

    fn test_sqrt<T: FieldElement>() {
        let mut x = T::one();
        for _ in 0..1000 {
            let square = x.clone() * x.clone();
            let root = square.sqrt();
            assert_eq!(square, root.clone() * root.clone());
            x += T::one();
        }
    }

    #[test]
    fn sqrt_foi() {
        test_sqrt::<FoiFieldElement>();
    }

    #[test]
    fn sqrt_bn128() {
        test_sqrt::<Bn128FieldElement>();
    }

    #[test]
    fn sqrt_curve25519() {
        test_sqrt::<Curve25519FieldElement>();
    }
}
