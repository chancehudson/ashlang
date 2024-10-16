//! [![Build](https://img.shields.io/circleci/build/github/chancehudson/ashlang/main)](https://dl.circleci.com/status-badge/redirect/gh/chancehudson/ashlang/tree/main) [![Docs](https://img.shields.io/docsrs/ashlang)](https://docs.rs/ashlang) [![Version](https://img.shields.io/crates/v/ashlang)](https://crates.io/crates/ashlang)
//!
//! A language designed to compile and execute on mathematical virtual machines.
//!
//! Simplicity is the philosophy of ashlang. The language is simple to learn and expresses relationships very close to the arithmetization. Functions are globally available to encourage the development of a single, well audited, well maintained standard library of logic that can be re-used in many proving systems.
//!
//! ## Targets
//! ashlang currently supports two targets:
//!
//! - [`ar1cs`](https://github.com/chancehudson/ashlang/tree/main/ashlang/src/r1cs#readme) - an extended rank 1 constraint system that includes witness calculation instructions
//! - [`tasm`](https://triton-vm.org/spec/instructions.html) - a novel assembly language used to express instructions for the [Triton VM](https://github.com/tritonvm/triton-vm)
//! ## Provers
//!
//! ashlang supprts proving on the following systems:
//!
//! - [`TritonVM/triton-vm`](https://github.com/tritonvm/triton-vm) - using `tasm` target in this crate
//! - [`microsoft/spartan`](https://github.com/microsoft/spartan) - using `ar1cs` target in [chancehudson/ashlang-spartan](https://github.com/chancehudson/ashlang-spartan)
//!
//! ## Language
//!
//! ashlang is a scripting language for expressing mathematical relations between scalars and vectors in a finite field.
//!
//! The language is untyped, with each variable being one of the following:
//!
//! - scalar
//! - vector
//! - matrix (of any dimension)
//!
//! ashlang is designed to be written in conjunction with a lower level language. Each file is a single function, it may be invoked using its filename. Directories are recursively imported and functions become globally available.
//!
//! ### Features
//!
//! - element-wise vector operations
//! - throws if vectors of mismatched size are used in an operation e.g. `val[0..10] * lav[0..5]`
//! - functions cannot be declared, each file is a single function
//! - files are not imported, function calls match the filename and tell the compiler what files are needed
//! - r1cs witnesses can be computed without specialized code

mod cli;
pub mod compiler;
pub mod log;
/// Ashlang source code parser.
pub mod parser;
mod provers;
/// Core logic for the r1cs target.
pub mod r1cs;
/// Concrete ring instances used by ashlang compile targets.
pub mod rings;
/// Core logic for the tasm target.
pub mod tasm;
mod time;

pub use cli::Config;

// Expose provers at the top level export here
// e.g. use ashlang::SpartanProver;
pub use provers::AshlangProver;
#[cfg(feature = "spartan-prover")]
pub use provers::SpartanProver;
#[cfg(feature = "tritonvm-prover")]
pub use provers::TritonVMProver;
