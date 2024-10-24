use std::fmt::Debug;

use scalarff::FieldElement;

use crate::Vector;

/// A univariate polynomial with coefficients in a field
///
/// The base field may be finite or infinite depending
/// on T
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq)]
pub struct Polynomial<T>
where
    T: FieldElement,
{
    pub coefficients: Vec<T>, // len() <= degree, non-existent elements assumed to be zero
}

impl<T: FieldElement> Polynomial<T> {
    pub fn new(coefficients: Vec<T>) -> Self {
        Self { coefficients }
    }

    /// Return the zero polynomial
    pub fn zero() -> Self {
        Self {
            coefficients: vec![],
        }
    }

    /// Return the identity polynomial
    pub fn identity() -> Self {
        Self {
            coefficients: vec![T::one()],
        }
    }

    /// Returns true if `self` is the zero polynomial
    pub fn is_zero(&self) -> bool {
        if self.coefficients.is_empty() {
            return true;
        }
        for v in &self.coefficients {
            if *v != T::zero() {
                return false;
            }
        }
        true
    }

    /// Do a scalar multiplication in place
    pub fn mul_scalar(&mut self, v: &T) {
        for i in 0..self.coefficients.len() {
            self.coefficients[i] *= v.clone();
        }
    }

    /// Return the constant term of the polynomial.
    /// This is the coefficient of the x^0 term.
    pub fn constant_term(&self) -> T {
        if self.coefficients.is_empty() {
            T::zero()
        } else {
            self.coefficients[0].clone()
        }
    }

    /// Return a coefficient vector with trailing zero
    /// coefficients removed.
    ///
    /// out.len() == self.degree() + 1
    pub fn coef_vec(&self) -> Vector<T> {
        let mut out = Vector::new();
        for i in 0..(self.degree() + 1) {
            out.push(self.coefficients[i].clone());
        }
        out
    }

    /// Add a term to the polynomial
    pub fn term(&mut self, coef: &T, exp: usize) {
        if self.coefficients.len() < exp + 1 {
            self.coefficients.resize(exp + 1, T::zero());
        }
        self.coefficients[exp] += coef.clone();
    }

    /// Remove the highest degree term from the polynomial and return it
    pub fn pop_term(&mut self) -> (T, usize) {
        for i in 0..self.coefficients.len() {
            let index = self.coefficients.len() - (i + 1);
            let v = self.coefficients[index].clone();
            if v != T::zero() {
                self.coefficients[index] = T::zero();
                return (v, index);
            }
        }
        (T::zero(), 0)
    }

    /// Return the degree of the polynomial. e.g. the degree of the largest
    /// non-zero term
    pub fn degree(&self) -> usize {
        for i in 0..self.coefficients.len() {
            let index = self.coefficients.len() - (i + 1);
            if self.coefficients[index] != T::zero() {
                return index;
            }
        }
        0
    }

    /// a fast method for multiplying by a single term polynomial
    /// with a coefficient of 1
    /// e.g. multiplying by x^5
    pub fn shift_and_clone(&self, degree: usize) -> Self {
        let mut shifted_coefs = vec![T::zero(); degree];
        shifted_coefs.extend(self.coefficients.clone());
        Self {
            coefficients: shifted_coefs,
        }
    }

    /// return q, r such that self = q * divisor + r
    /// divisor must not be the zero polynomial
    pub fn div(&self, divisor: &Self) -> (Self, Self) {
        if divisor.is_zero() {
            panic!("divide by zero");
        }
        if self.degree() < divisor.degree() {
            return (Self::zero(), self.clone());
        }
        let mut dclone = divisor.clone();
        let mut quotient = Self::zero();
        let (divisor_term, divisor_term_exp) = dclone.pop_term();
        let divisor_term_inv = T::one() / divisor_term.clone();
        let mut remainder = self.clone();
        while !remainder.is_zero() && remainder.degree() >= divisor.degree() {
            let (largest_term, largest_term_exp) = remainder.clone().pop_term();
            let new_coef = largest_term * divisor_term_inv.clone();
            let new_exp = largest_term_exp - divisor_term_exp;
            quotient.term(&new_coef, new_exp);
            let mut t = divisor.shift_and_clone(new_exp);
            t.mul_scalar(&new_coef);
            remainder = remainder - t;
        }
        (quotient, remainder)
    }
}

impl<T: FieldElement> std::fmt::Display for Polynomial<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_zero() {
            write!(f, "0")
        } else {
            write!(
                f,
                "{}",
                self.coefficients
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        if *v == T::zero() && i > 0 {
                            return "".to_string();
                        }
                        if i > 0 {
                            format!("{}x^{i}", v.to_string())
                        } else {
                            v.to_string()
                        }
                    })
                    .filter(|x| x.len() > 0)
                    .collect::<Vec<_>>()
                    .join(" + ")
            )
        }
    }
}

impl<T: FieldElement> std::ops::Add for Polynomial<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let max_len = if self.coefficients.len() > other.coefficients.len() {
            self.coefficients.len()
        } else {
            other.coefficients.len()
        };
        let mut coefficients = vec![T::zero(); max_len];
        for x in 0..max_len {
            if x < self.coefficients.len() {
                coefficients[x] += self.coefficients[x].clone();
            }
            if x < other.coefficients.len() {
                coefficients[x] += other.coefficients[x].clone();
            }
        }
        Polynomial { coefficients }
    }
}

impl<T: FieldElement> std::ops::Sub for Polynomial<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let max_len = if self.coefficients.len() > other.coefficients.len() {
            self.coefficients.len()
        } else {
            other.coefficients.len()
        };
        let mut coefficients = vec![T::zero(); max_len];
        for x in 0..max_len {
            if x < self.coefficients.len() {
                coefficients[x] += self.coefficients[x].clone();
            }
            if x < other.coefficients.len() {
                coefficients[x] -= other.coefficients[x].clone();
            }
        }
        Polynomial { coefficients }
    }
}

impl<T: FieldElement> std::ops::Mul for Polynomial<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let mut coefficients = vec![T::zero(); self.coefficients.len() + other.coefficients.len()];
        for i in 0..other.coefficients.len() {
            for j in 0..self.coefficients.len() {
                // combine the exponents
                let e = j + i;
                // combine with existing coefficients
                coefficients[e] += self.coefficients[j].clone() * other.coefficients[i].clone();
            }
        }
        // TODO: remove this trimming, it's only necessary
        // because of hacky compatiblity between ashlang serialization
        // and PolynomialRingElement serialization
        let mut max = coefficients.len();
        for i in (0..coefficients.len()).rev() {
            if coefficients[i] != T::zero() {
                break;
            }
            max = i;
        }
        Polynomial {
            coefficients: coefficients[0..max].to_vec(),
        }
    }
}

impl<T: FieldElement> std::ops::Neg for Polynomial<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Polynomial {
            coefficients: self.coefficients.iter().map(|v| -v.clone()).collect(),
        }
    }
}

impl<T: FieldElement> std::cmp::PartialEq for Polynomial<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.degree() != other.degree() {
            return false;
        }
        for i in 0..self.degree() {
            if self.coefficients[i] != other.coefficients[i] {
                return false;
            }
        }
        true
    }
}

impl<T: FieldElement> std::hash::Hash for Polynomial<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // hash only the non-zero coefficients
        for i in 0..self.degree() {
            self.coefficients[i].hash(state);
        }
    }
}

#[cfg(test)]
mod test {
    use super::Polynomial;
    use scalarff::FieldElement;
    use scalarff::OxfoiFieldElement;

    #[test]
    #[cfg(feature = "rand")]
    fn mul_div() {
        for _ in 0..100 {
            let mut r = rand::thread_rng();
            let p1 = Polynomial {
                coefficients: vec![
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                ],
            };
            let p2 = Polynomial {
                coefficients: vec![
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                    OxfoiFieldElement::sample_uniform(&mut r),
                ],
            };
            let (q, r) = p1.div(&p2);
            assert!(!q.is_zero());
            assert!(!p1.is_zero());
            assert!(!p2.is_zero());
            assert_eq!(q * p2 + r, p1);
        }
    }
}
