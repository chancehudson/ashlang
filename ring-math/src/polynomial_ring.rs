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

use scalarff::FieldElement;

use super::polynomial::Polynomial;
use super::Matrix2D;
use super::Vector;

/// A trait representing a polynomial ring
/// defined as `T[X]/<Self::modulus()>`
/// where T is a FieldElement trait
/// and modulus is a function implemented by the
/// struct implementing PolynomialRingElement
pub trait PolynomialRingElement:
    FieldElement
    + Add<Output = Self>
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
    + From<Polynomial<Self::F>>
    + Display
{
    type F: FieldElement;

    /// Modulus used in remainder division to form
    /// the polynomial ring.
    ///
    /// Operations are done modulo this polynomial,
    /// in a way similar to scalar fields.
    ///
    /// See the division implementation for more info.
    fn modulus() -> Polynomial<Self::F>;

    /// Return the Polynomial representation of the current value
    /// Used to automatically implement norms and other functions.
    fn polynomial(&self) -> &Polynomial<Self::F>;

    /// Attempt to get a scalar representation of the polynomial.
    /// If the polynomial degree is > 0 this method will error.
    fn to_scalar(&self) -> anyhow::Result<Self::F> {
        if self.polynomial().degree() == 0 {
            Ok(self.polynomial().coefficients[0].clone())
        } else {
            anyhow::bail!("Cannot convert polynomial of degree > 0 to scalar")
        }
    }

    /// Calculate the l1 norm for this polynomial. That is
    /// the summation of all coefficients
    fn norm_l1(&self) -> u64 {
        let digits = self
            .polynomial()
            .coefficients
            .iter()
            .fold(Self::F::zero(), |acc, x| acc + x.clone())
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
            .polynomial()
            .coefficients
            .iter()
            .fold(Self::F::zero(), |acc, x| acc + (x.clone() * x.clone()));
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
        let mut max = Self::F::zero().to_biguint();
        for i in &self.polynomial().coefficients {
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

    /// Returns a coefficient vector of length equal
    /// to the ring modulus degree.
    fn coef(&self) -> Vector<Self::F> {
        let modulus = Self::modulus();
        let target_degree = modulus.degree();
        let poly_coefs = self.polynomial().coef_vec().to_vec();
        let poly_coefs_len = poly_coefs.len();
        Vector::from_vec(
            [
                poly_coefs,
                vec![Self::F::zero(); target_degree - poly_coefs_len],
            ]
            .concat(),
        )
    }

    /// Create a rotated matrix of polynomial coefficients
    ///
    /// from LatticeFold page 9
    /// https://eprint.iacr.org/2024/257.pdf
    fn rot(&self) -> Matrix2D<Self::F> {
        let modulus = Self::modulus();
        let degree = modulus.degree();
        let mut values = vec![Self::F::zero(); degree * degree];
        // TODO: check if this logic is correct
        // technically in each row we're multiplying by X
        // and then reducing by the modulus. In practice this
        // results in coefficients being rotated and inverted.
        //
        // Test in with various coefficients and moduluses or
        // mathematically verify
        for i in 0..degree {
            let mut coefs = self.coef().to_vec();
            coefs.rotate_right(i);
            for j in 0..i {
                coefs[j] = -coefs[j].clone();
            }
            for j in 0..degree {
                values[j * degree + i] = coefs[j].clone();
            }
        }
        Matrix2D {
            dimensions: (degree, degree),
            values,
        }
    }
}

/// Use this to build a concrete instance of a polynomial ring.
///
/// e.g.
/// ```
/// polynomial_ring!(
/// Poly64,
/// FoiFieldElement,
/// {
///     let mut p = Polynomial::new(vec![FoiFieldElement::one()]);
///     p.term(&FoiFieldElement::one(), 64);
///     p
/// },
/// "Poly64"
/// );
/// ```
#[macro_export]
macro_rules! polynomial_ring {
    ( $name: ident, $field_element: ident, $modulus: expr, $name_str: expr ) => {
        #[derive(Clone, Debug, PartialEq, Eq, std::hash::Hash)]
        pub struct $name(pub Polynomial<$field_element>);

        impl PolynomialRingElement for $name {
            type F = $field_element;

            fn modulus() -> Polynomial<$field_element> {
                $modulus
            }

            fn polynomial(&self) -> &Polynomial<$field_element> {
                &self.0
            }
        }

        impl From<Polynomial<$field_element>> for $name {
            fn from(p: Polynomial<$field_element>) -> Self {
                $name(p.div(&Self::modulus()).1)
            }
        }

        impl FieldElement for $name {
            fn zero() -> Self {
                $name(Polynomial {
                    coefficients: vec![$field_element::zero()],
                })
            }

            fn one() -> Self {
                $name(Polynomial::identity())
            }

            fn byte_len() -> usize {
                Self::modulus().degree() * $field_element::byte_len()
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
                $name(Polynomial {
                    coefficients: str
                        .split(',')
                        .map(|v| $field_element::deserialize(v))
                        .collect::<Vec<_>>(),
                })
            }

            fn prime() -> scalarff::BigUint {
                panic!("cannot retrieve a scalar prime for a polynomial field");
            }

            fn name_str() -> &'static str {
                $name_str
            }

            /// Return a constant polynomial with the provided
            /// value
            fn from_usize(value: usize) -> Self {
                $name(Polynomial {
                    coefficients: vec![$field_element::from_usize(value)],
                })
            }

            fn to_biguint(&self) -> scalarff::BigUint {
                panic!("cannot retrieve a scalar representation for a polynomial field element");
            }

            fn from_biguint(_v: &scalarff::BigUint) -> Self {
                panic!();
            }

            fn from_bytes_le(bytes: &[u8]) -> Self {
                $name(Polynomial {
                    coefficients: bytes
                        .chunks($field_element::byte_len())
                        .map(|chunk| $field_element::from_bytes_le(chunk))
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

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ();

            fn from_str(_s: &str) -> Result<Self, Self::Err> {
                Err(())
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self::from(Polynomial {
                    coefficients: vec![$field_element::from(value)],
                })
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                Self::from(self.0 + other.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                Self::from(self.0 - other.0)
            }
        }

        impl std::ops::Mul for $name {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                Self::from(self.0 * other.0)
            }
        }

        impl std::ops::Div for $name {
            type Output = Self;

            fn div(self, other: Self) -> Self {
                // this implementation implies floored division, so discard the remainder
                Self::from(self.0.div(&other.0).0)
            }
        }

        impl std::ops::AddAssign for $name {
            fn add_assign(&mut self, other: Self) {
                *self = self.clone() + other;
            }
        }

        impl std::ops::MulAssign for $name {
            fn mul_assign(&mut self, other: Self) {
                *self = self.clone() * other;
            }
        }

        impl std::ops::SubAssign for $name {
            fn sub_assign(&mut self, other: Self) {
                *self = self.clone() - other;
            }
        }

        impl std::ops::Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                $name(-self.0.clone())
            }
        }
    };
}

#[cfg(test)]
mod test {
    use scalarff::FieldElement;
    use scalarff::FoiFieldElement;

    use super::Polynomial;
    use super::PolynomialRingElement;

    polynomial_ring!(
        Poly64,
        FoiFieldElement,
        {
            let mut p = Polynomial::new(vec![FoiFieldElement::one()]);
            p.term(&FoiFieldElement::one(), 64);
            p
        },
        "Poly64"
    );

    #[test]
    fn scalar_math_in_ring() {
        for x in 100..500 {
            for y in 200..600 {
                let z_scalar = FoiFieldElement::from(x) * FoiFieldElement::from(y);
                let z_poly = Poly64::from(x) * Poly64::from(y);
                assert_eq!(z_poly.polynomial().degree(), 0);
                assert_eq!(z_poly.polynomial().coefficients[0], z_scalar);
            }
        }
    }

    #[test]
    fn poly_coefs() {
        let poly = Poly64::one();
        // the coefficient vector should always be equal in length
        // to the degree of the polynomial ring modulus
        let c = poly.coef();
        assert_eq!(c.len(), Poly64::modulus().degree());
    }

    #[test]
    fn poly_rot() {
        // Testing the relationship as defined in the LatticeFold paper
        // coef(a * b) = rot(a) * coef(b)
        // we end up doing multiplication without a division
        // reduction step (still O(n^2))
        //
        // sample two random polynomials
        let a = Poly64::sample_rand(&mut rand::thread_rng());
        let b = Poly64::sample_rand(&mut rand::thread_rng());
        // create a rotated matrix of polynomial coefficients
        let rot_mat = a.rot();
        let b_coef = b.coef();

        // check the above
        let expected_coef = (a * b).coef();
        let actual_coef = rot_mat * b_coef.clone();
        for i in 0..b_coef.len() {
            assert_eq!(expected_coef[i], actual_coef[i]);
        }
    }
}
