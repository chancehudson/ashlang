//! A vector/matrix structure for doing arithmetic on
//! sets of `PolynomialRingElements`. Matrices can be 1 dimensional
//! for representing vectors.
//!
//! This matrix implementation is designed to represent matrices
//! of variable dimension.
//!
use std::fmt::Display;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;
use std::str::FromStr;

use super::PolynomialRingElement;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Matrix<T: PolynomialRingElement> {
    // scalars should be represented as dimensions: vec![1]
    pub dimensions: Vec<usize>,
    pub values: Vec<T>,
}

impl<T: PolynomialRingElement> Matrix<T> {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn mul_scalar(&self, v: T) -> Self {
        let values = self.values.iter().map(|x| x.clone() * v.clone()).collect();
        Matrix {
            dimensions: self.dimensions.clone(),
            values,
        }
    }

    pub fn invert(&self) -> Self {
        let values = self.values.iter().map(|x| T::one() / x.clone()).collect();
        Matrix {
            dimensions: self.dimensions.clone(),
            values,
        }
    }

    /// Retrieve a scalar or sub-matrix from the matrix using
    /// index notation. e.g. v[3][2]
    pub fn retrieve_indices(&self, indices: &[usize]) -> (Self, usize) {
        let mul_sum = |vec: &Vec<usize>, start: usize| -> usize {
            let mut out = 1;
            for v in &vec[start..] {
                out *= v;
            }
            out
        };
        let mut offset = 0;
        for x in 0..indices.len() {
            // for each index we sum the deeper dimensions
            // to determine how far to move in the array storage
            if x == indices.len() - 1 && indices.len() == self.dimensions.len() {
                offset += indices[x];
            } else {
                offset += indices[x] * mul_sum(&self.dimensions, x + 1);
            }
        }

        let mut new_dimensions = vec![];
        for x in indices.len()..self.dimensions.len() {
            new_dimensions.push(self.dimensions[x]);
        }
        // add a dimension to mark as scalar
        if new_dimensions.is_empty() {
            new_dimensions.push(1);
        }
        let offset_end = if indices.len() == self.dimensions.len() {
            offset + 1
        } else {
            offset + mul_sum(&self.dimensions, indices.len())
        };
        (
            Self {
                dimensions: new_dimensions,
                values: self.values[offset..offset_end].to_vec(),
            },
            offset,
        )
    }

    pub fn _assert_internal_consistency(&self) {
        assert_eq!(self.values.len(), self.dimensions.iter().product::<usize>());
    }

    pub fn assert_eq_shape(&self, m: &Matrix<T>) {
        if self.dimensions.len() != m.dimensions.len() {
            panic!("lhs and rhs dimensions are not equal: {:?} {:?}", self, m);
        }
        for x in 0..m.dimensions.len() {
            if self.dimensions[x] != m.dimensions[x] {
                panic!(
                    "lhs and rhs inner dimensions are not equal: {:?} {:?}",
                    self, m
                );
            }
        }
    }
}

impl<T: PolynomialRingElement> Add for Matrix<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.assert_eq_shape(&other);
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a.clone() + b.clone())
            .collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: PolynomialRingElement> AddAssign for Matrix<T> {
    fn add_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] += other.values[i].clone();
        }
    }
}

impl<T: PolynomialRingElement> Sub for Matrix<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self.assert_eq_shape(&other);
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a.clone() - b.clone())
            .collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: PolynomialRingElement> SubAssign for Matrix<T> {
    fn sub_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] -= other.values[i].clone();
        }
    }
}

impl<T: PolynomialRingElement> Mul for Matrix<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        self.assert_eq_shape(&other);
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a.clone() * b.clone())
            .collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: PolynomialRingElement> MulAssign for Matrix<T> {
    fn mul_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] *= other.values[i].clone();
        }
    }
}

impl<T: PolynomialRingElement> Div for Matrix<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        self.assert_eq_shape(&other);
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a.clone() / b.clone())
            .collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: PolynomialRingElement> Neg for Matrix<T> {
    type Output = Self;

    fn neg(self) -> Self {
        let values = self.values.iter().map(|x| -x.clone()).collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: PolynomialRingElement> From<T> for Matrix<T> {
    fn from(v: T) -> Self {
        Matrix {
            dimensions: vec![1],
            values: vec![v],
        }
    }
}

impl<T: PolynomialRingElement> From<u64> for Matrix<T> {
    fn from(v: u64) -> Self {
        Matrix::from(T::from(v))
    }
}

impl<T: PolynomialRingElement> FromStr for Matrix<T> {
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Matrix::from(T::from_str(s)?))
    }
}

impl<T: PolynomialRingElement> Display for Matrix<T> {
    // TODO: pretty print the matrix
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut s = String::new();
        s.push_str(&format!(
            "dimensions: {}\n",
            self.dimensions
                .clone()
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("x")
        ));
        for i in 0..self.values.len() {
            s.push_str(&format!("{}, ", self.values[i]));
        }
        write!(f, "{}", s)
    }
}
