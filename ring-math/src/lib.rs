//! [![Build](https://img.shields.io/circleci/build/github/chancehudson/ashlang/main)](https://dl.circleci.com/status-badge/redirect/gh/chancehudson/ashlang/tree/main) [![Docs](https://img.shields.io/docsrs/ring-math)](https://docs.rs/ring-math) [![Version](https://img.shields.io/crates/v/ring-math)](https://crates.io/crates/ring-math)
//!
//! Polynomial ring math with variables in [`scalarff::FieldElement`](https://docs.rs/scalarff/latest/scalarff/trait.FieldElement.html). Includes structures for vectors and matrices of variable dimension and overloads for mathematical operations.

mod matrix;
mod matrix2d;
mod polynomial;
mod polynomial_ring;
mod vector;

pub use matrix::Matrix;
pub use matrix2d::Matrix2D;
pub use polynomial::Polynomial;
pub use polynomial_ring::PolynomialRingElement;
pub use vector::Vector;

pub use scalarff::custom_ring;
pub use scalarff::FieldElement;
