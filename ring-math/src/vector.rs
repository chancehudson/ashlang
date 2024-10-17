use scalarff::BigUint;
use scalarff::FieldElement;

#[derive(Clone, PartialEq)]
pub struct Vector<T: FieldElement>(pub Vec<T>);

impl<T: FieldElement> Default for Vector<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FieldElement> Vector<T> {
    pub fn new() -> Self {
        Vector(Vec::new())
    }

    pub fn zero(len: usize) -> Self {
        Self(vec![T::zero(); len])
    }

    /// Compute the inner product (dot product) of two vectors.
    /// Vectors are multiplied element-wise and then summed.
    pub fn dot_product(&self, other: Vector<T>) -> T {
        let mut out = T::zero();
        for (a, b) in std::iter::zip(self.iter(), other.iter()) {
            out += a.clone() * b.clone();
        }
        out
    }

    /// Calculate the l1 norm for this vector. That is
    /// the summation of all coefficients
    pub fn norm_l1(&self) -> u64 {
        let digits = self
            .0
            .iter()
            .fold(BigUint::from(0u32), |acc, x| acc + x.to_biguint())
            .to_u64_digits();
        if digits.len() > 1 {
            panic!("Norm l1 is not a single u64 digit");
        } else if digits.len() == 1 {
            digits[0]
        } else {
            0
        }
    }

    /// Calculate the l2 norm for this vector. That is
    /// the square root of the summation of each coefficient squared
    ///
    /// Specifically, we're calculating the square root in the integer
    /// field, not the prime field
    pub fn norm_l2(&self) -> u64 {
        let v = self.0.iter().fold(BigUint::from(0u32), |acc, x| {
            acc + (x.to_biguint() * x.to_biguint())
        });
        let digits = v.sqrt().to_u64_digits();
        if digits.len() > 1 {
            panic!("Norm l2 is not a single u64 digit");
        } else if digits.len() == 1 {
            digits[0]
        } else {
            0
        }
    }

    /// Calculate the l-infinity norm for this vector. That is
    /// the largest coefficient
    pub fn norm_max(&self) -> u64 {
        let mut max = T::zero().to_biguint();
        for i in &self.0 {
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

    /// Sum the elements of the array and return the result.
    pub fn sum(&self) -> T {
        self.0.iter().fold(T::zero(), |acc, x| acc + x.clone())
    }

    /// Sample a uniform random vector of the specified dimension
    /// from the underlying field.
    pub fn sample_uniform<R: rand::Rng>(len: usize, rng: &mut R) -> Self {
        Self((0..len).map(|_| T::sample_uniform(rng)).collect())
    }

    pub fn from_vec(v: Vec<T>) -> Self {
        Vector(v)
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.0.clone()
    }

    pub fn to_vec_ref(&self) -> &Vec<T> {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, v: T) {
        self.0.push(v);
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.0.iter_mut()
    }
}

impl<T: FieldElement> std::fmt::Display for Vector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for v in &self.0 {
            write!(f, "{}, ", v)?;
        }
        Ok(())
    }
}

impl<T: FieldElement> std::ops::Index<std::ops::Range<usize>> for Vector<T> {
    type Output = [T];

    fn index(&self, index: std::ops::Range<usize>) -> &[T] {
        &self.0[index]
    }
}

impl<T: FieldElement> std::ops::Index<usize> for Vector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.0[index]
    }
}

impl<T: FieldElement> std::ops::Mul<Vector<T>> for Vector<T> {
    type Output = Vector<T>;

    fn mul(self, other: Vector<T>) -> Vector<T> {
        assert_eq!(self.0.len(), other.len(), "vector mul length mismatch");
        let mut out = Vec::new();
        for i in 0..self.len() {
            out.push(self.to_vec_ref()[i].clone() * other.to_vec_ref()[i].clone());
        }
        Vector::from_vec(out)
    }
}

impl<T: FieldElement> std::ops::Add<Vector<T>> for Vector<T> {
    type Output = Vector<T>;

    fn add(self, other: Vector<T>) -> Vector<T> {
        assert_eq!(self.0.len(), other.len(), "vector mul length mismatch");
        let mut out = Vec::new();
        for i in 0..self.len() {
            out.push(self.to_vec_ref()[i].clone() + other.to_vec_ref()[i].clone());
        }
        Vector::from_vec(out)
    }
}

impl<T: FieldElement> std::ops::Mul<T> for Vector<T> {
    type Output = Vector<T>;

    fn mul(self, other: T) -> Vector<T> {
        Vector::from_vec(self.iter().map(|v| v.clone() * other.clone()).collect())
    }
}
