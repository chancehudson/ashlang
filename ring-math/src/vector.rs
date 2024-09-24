use scalarff::FieldElement;

#[derive(Clone, PartialEq)]
pub struct Vector<T: FieldElement>(pub Vec<T>);

impl<T: FieldElement> Vector<T> {
    pub fn new() -> Self {
        Vector(Vec::new())
    }

    pub fn zero(len: usize) -> Self {
        Self(vec![T::zero(); len])
    }

    pub fn rand_uniform<R: rand::Rng>(len: usize, rng: &mut R) -> Self {
        Self((0..len).map(|_| T::sample_rand(rng)).collect())
    }

    /// Generate a random vector using a gaussian distribution around the zero point
    /// define a l_infinite norm and l_1 norm for the resulting polynomial
    pub fn rand_gaussian<R: rand::Rng>(len: usize, l_inf: usize, l_1: usize, rng: &mut R) -> Self {
        // l_inf = max coefficient
        // l_1 = max sum of coefficients

        // rng.gen_range(0..=l_inf)
        // Self((0..len).map(|_| T::sample_rand(rng)).collect())
        panic!();
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
