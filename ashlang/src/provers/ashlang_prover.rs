use anyhow::Result;

use crate::cli::Config;

// TODO: make the resulting proof serializable here
// e.g. `AshlangProver<T: Debug>`

/// A trait representing an abstract prover implementation.
/// The generic argument indicates the type of the proof that
/// the prover builds.
pub trait AshlangProver<T> {
    /// Generate a proof by compiling source files into an IR
    fn prove(config: &Config) -> Result<T>;
    /// Generate a proof from an existing IR
    fn prove_ir(ir: &str, public_inputs: Vec<String>, secret_inputs: Vec<String>) -> Result<T>;
    /// Verify a proof
    fn verify(proof: T) -> Result<bool>;
}
