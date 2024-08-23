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
        let e_bigint = BigUint::from_str(&e.to_string()).unwrap();
        let a = BigUint::from_str(&self.to_string()).unwrap();
        let l = a.modpow(&e_bigint, &Self::prime());
        if l == neg_one {
            return -1;
        } else if l == one {
            return 1;
        } else {
            panic!("legendre symbol is not 1, -1, or 0");
        }
    }
    // Calculate the largest power t for which
    // p - 1 = 2^t * s
    // returns t
    // fn two_adicity() -> u64 {
    //     let p_1 = -Self::one();
    // let two = Self::one() + Self::one();
    // let mut v = two.clone();

    // let mut adicity = 0;
    // for x in 0..1024 {
    //     // search until v is > p_1
    //     if v > p_1 {
    //         if adicity == 0 {
    //             panic!("did not find adicity after searching field");
    //         }
    //         return adicity;
    //     }
    //     if v == p_1 {
    //         panic!("prime-1 is a power of 2");
    //     }
    //     let s = p_1.clone() * (v.clone().modinv(&Self::prime()).unwrap());
    //     println!("{s}");
    //     if p_1 == (s.clone() * v.clone()) % Self::prime() && s.is_odd() {
    //         adicity = x + 1;
    //     }
    //     v = v * two.clone();
    // }
    // panic!("field larger than 2**1024");
    // }
}

/// Tonelli-Shanks Algorithm
/// https://arxiv.org/pdf/2206.07145
pub fn sqrt<T: FieldElement>(i: T) -> T
where
    <T as FromStr>::Err: Debug,
{
    if i.legendre() != 1 {
        panic!("legendre symbol is not 1: root does not exist or input is 0");
    }
    // find a residue and a non-residue
    let mut x = T::one() + T::one();
    let non_residue;
    loop {
        if x.legendre() == -1 {
            non_residue = x.clone();
            break;
        }
        x = x + T::one();
    }
    let non_residue = BigUint::from_str(&non_residue.to_string()).unwrap();

    let one = BigUint::from(1_u32);
    let p = T::prime();
    let p_1 = T::prime() - 1_u32;
    let e = 32_u32;
    if e == 1 {
        panic!("not implemented: adicity is 1");
    }
    let i_bigint = BigUint::from_str(&i.to_string()).unwrap();
    let m = (p_1.clone()
        * BigUint::from(2_u32)
            .pow(e.try_into().unwrap())
            .modinv(&p)
            .unwrap())
        % &p;
    if (&m * BigUint::from(2_u32).pow(e.try_into().unwrap())) % &p != p_1 {
        panic!("m * 2^e != p-1");
    }

    let z = non_residue.modpow(&m, &p);
    let b = i_bigint.modpow(&m, &p);
    let mut r = BigUint::from(2_u32);
    // skip the 0 case
    // use the field implementation, not BigUint
    let b_t = T::from_str(&b.to_string()).unwrap();
    let z_t = T::from_str(&z.to_string()).unwrap();
    let mut z_r_t = T::from_str(&z.to_string()).unwrap();
    loop {
        // let z_r = z.modpow(&r, &p);
        if b_t != z_r_t {
            if &r % 1000000_u32 == 0_u32.into() {
                println!("{r}: {b_t} {z_r_t}");
            }
            r = r + BigUint::from(2_u32);
            z_r_t *= z_t.clone() * z_t.clone();
            continue;
        }
        let n_m_r = non_residue.modpow(&(m.clone() * r.clone()), &p);
        if b == n_m_r {
            break;
        }
        r = r + BigUint::from(2_u32);
        z_r_t *= z_t.clone() * z_t.clone();
    }
    if BigUint::from_str(&z_r_t.to_string()).unwrap() != z.modpow(&r, &p) {
        panic!("z_r != z^r");
    }
    println!("{r} {}", T::from_str(&r.to_string()).unwrap());
    let r_neg = -T::from_str(&r.to_string()).unwrap();
    if r_neg.clone() + T::from_str(&r.to_string()).unwrap() != T::zero() {
        panic!("r_neg + r != 0");
    }
    println!("{r_neg}");
    println!("{}", &m + 1_u32);
    let z_exp = r_neg / (T::one() + T::one());
    let z_exp = BigUint::from_str(&z_exp.to_string()).unwrap();
    let a_exp = T::from_str(&(m + one).to_string()).unwrap() / (T::one() + T::one());
    let a_exp = BigUint::from_str(&a_exp.to_string()).unwrap();

    let out = i_bigint.modpow(&a_exp, &p) * z.modpow(&z_exp, &p);
    let out = out % p;
    T::from_str(&out.to_string()).unwrap()
}
