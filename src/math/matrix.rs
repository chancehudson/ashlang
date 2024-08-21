use super::FieldElement;
use crate::log;
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct Matrix<T: FieldElement> {
    pub dimensions: Vec<usize>,
    pub values: Vec<T>,
}

impl<T: FieldElement> Matrix<T> {
    pub fn len(&self) -> usize {
        self.values.len()
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

    pub fn _assert_internal_consistency(&self) {
        assert_eq!(self.values.len(), self.dimensions.iter().product::<usize>());
    }

    pub fn assert_eq_shape(&self, m: &Matrix<T>) {
        if self.dimensions.len() != m.dimensions.len() {
            log::error!(&format!(
                "lhs and rhs dimensions are not equal: {:?} {:?}",
                self, m
            ));
        }
        for x in 0..m.dimensions.len() {
            if self.dimensions[x] != m.dimensions[x] {
                log::error!(&format!(
                    "lhs and rhs inner dimensions are not equal: {:?} {:?}",
                    self, m
                ));
            }
        }
    }
}

impl<T: FieldElement> Add for Matrix<T> {
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

impl<T: FieldElement> AddAssign for Matrix<T> {
    fn add_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] += other.values[i].clone();
        }
    }
}

impl<T: FieldElement> Sub for Matrix<T> {
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

impl<T: FieldElement> SubAssign for Matrix<T> {
    fn sub_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] -= other.values[i].clone();
        }
    }
}

impl<T: FieldElement> Mul for Matrix<T> {
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

impl<T: FieldElement> MulAssign for Matrix<T> {
    fn mul_assign(&mut self, other: Self) {
        self.assert_eq_shape(&other);
        for i in 0..self.values.len() {
            self.values[i] *= other.values[i].clone();
        }
    }
}

impl<T: FieldElement> Div for Matrix<T> {
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

impl<T: FieldElement> Neg for Matrix<T> {
    type Output = Self;

    fn neg(self) -> Self {
        let values = self.values.iter().map(|x| -x.clone()).collect();
        Matrix {
            dimensions: self.dimensions,
            values,
        }
    }
}

impl<T: FieldElement> From<T> for Matrix<T> {
    fn from(v: T) -> Self {
        Matrix {
            dimensions: vec![1],
            values: vec![v],
        }
    }
}

impl<T: FieldElement> From<u64> for Matrix<T> {
    fn from(v: u64) -> Self {
        Matrix::from(T::from(v))
    }
}

impl<T: FieldElement> FromStr for Matrix<T> {
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Matrix::from(T::from_str(s)?))
    }
}

impl<T: FieldElement> Display for Matrix<T> {
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
