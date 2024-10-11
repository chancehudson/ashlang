//! This module contains bindings to various prover implementations.
//!
mod ashlang_prover;
#[cfg(feature = "spartan-prover")]
mod spartan;
#[cfg(feature = "tritonvm-prover")]
mod tritonvm;

pub use ashlang_prover::AshlangProver;
#[cfg(feature = "spartan-prover")]
pub use spartan::SpartanProver;
#[cfg(feature = "tritonvm-prover")]
pub use tritonvm::TritonVMProver;
