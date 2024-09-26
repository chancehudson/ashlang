use scalarff::FieldElement;

use super::vector::Vector;

/// A two dimensional matrix implementation
#[derive(Clone, PartialEq)]
pub struct Matrix2D<T: FieldElement> {
    pub dimensions: (usize, usize), // (rows, cols)
    pub values: Vec<T>,
}

impl<T: FieldElement> Matrix2D<T> {
    /// Return an identity matrix of size `n`
    pub fn identity(n: usize) -> Self {
        let mut values: Vec<T> = Vec::new();
        for x in 0..n {
            let mut row = vec![T::zero(); n];
            row[x] = T::one();
            values.append(&mut row);
        }
        Matrix2D {
            dimensions: (n, n),
            values,
        }
    }

    /// Return a zero matrix of the specified dimensions
    pub fn zero(rows: usize, cols: usize) -> Self {
        Matrix2D {
            dimensions: (rows, cols),
            values: vec![T::zero(); rows * cols],
        }
    }

    /// Take the matrix and split it into 2 matrices vertically.
    /// e.g. take the first m1_height rows and return them as a matrix,
    /// and return the remaining rows as the m2 matrix.
    pub fn split_vertical(&self, m1_height: usize, m2_height: usize) -> (Matrix2D<T>, Matrix2D<T>) {
        assert_eq!(
            self.dimensions.0,
            m1_height + m2_height,
            "matrix vertical split height mismatch"
        );
        let (_, cols) = self.dimensions;
        let mid_offset = m1_height * cols;
        (
            Matrix2D {
                dimensions: (m1_height, cols),
                values: self.values[..mid_offset].to_vec(),
            },
            Matrix2D {
                dimensions: (m2_height, cols),
                values: self.values[mid_offset..].to_vec(),
            },
        )
    }

    /// Compose the matrix self with another matrix vertically.
    pub fn compose_vertical(&self, other: Self) -> Self {
        assert_eq!(
            self.dimensions.1, other.dimensions.1,
            "horizontal size mismatch in vertical composition"
        );
        Self {
            dimensions: (self.dimensions.0 + other.dimensions.0, self.dimensions.1),
            values: self
                .values
                .iter()
                .chain(other.values.iter())
                .cloned()
                .collect(),
        }
    }

    /// Compose the matrix self with another matrix horizontally.
    pub fn compose_horizontal(&self, other: Self) -> Self {
        let mut values = vec![];
        let (m1_rows, m1_cols) = self.dimensions;
        let (m2_rows, m2_cols) = other.dimensions;
        assert_eq!(
            m1_rows, m2_rows,
            "vertical size mismatch in horizontal composition"
        );
        for i in 0..m1_rows {
            values.append(&mut self.values[i * m1_cols..(i + 1) * m1_cols].to_vec());
            values.append(&mut other.values[i * m2_cols..(i + 1) * m2_cols].to_vec());
        }
        Self {
            dimensions: (self.dimensions.0, self.dimensions.1 + other.dimensions.1),
            values,
        }
    }

    pub fn rand_uniform<R: rand::Rng>(rows: usize, columns: usize, rng: &mut R) -> Self {
        Self {
            dimensions: (rows, columns),
            values: Vector::rand_uniform(rows * columns, rng).to_vec(),
        }
    }
}

impl<T: FieldElement> std::fmt::Display for Matrix2D<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (rows, cols) = self.dimensions;
        writeln!(f, "[")?;
        for i in 0..rows {
            write!(f, "  [ ")?;
            for j in 0..cols {
                write!(f, "{}, ", self.values[i * cols + j])?;
            }
            writeln!(f, "],")?;
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

impl<T: FieldElement> std::ops::Add for Matrix2D<T> {
    type Output = Matrix2D<T>;

    fn add(self, other: Matrix2D<T>) -> Matrix2D<T> {
        assert_eq!(
            self.dimensions, other.dimensions,
            "matrix addition dimensions mismatch"
        );
        Matrix2D {
            dimensions: self.dimensions,
            values: self
                .values
                .iter()
                .zip(other.values.iter())
                .map(|(a, b)| a.clone() + b.clone())
                .collect(),
        }
    }
}

impl<T: FieldElement> std::ops::Mul<Vector<T>> for Matrix2D<T> {
    type Output = Vector<T>;

    fn mul(self, other: Vector<T>) -> Vector<T> {
        let mut out = Vec::new();
        let (m_rows, m_cols) = self.dimensions;
        for i in 0..m_rows {
            let row = self.values[i * m_cols..(i + 1) * m_cols].to_vec();

            out.push(
                // TODO: determine if summing the vector here is correct
                (other.clone() * Vector::from_vec(row))
                    .iter()
                    .fold(T::zero(), |acc, v| acc + v.clone()),
            );
        }
        Vector::from_vec(out)
    }
}
