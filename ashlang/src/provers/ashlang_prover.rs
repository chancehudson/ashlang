use anyhow::Result;

use crate::cli::Config;

// TODO: make the resulting proof serializable here
// e.g. `AshlangProver<T: Debug>`

/// A trait representing an abstract prover implementation.
/// The generic argument indicates the type of the proof that
/// the prover builds.
pub trait AshlangProver<T> {
    fn prove(config: &Config) -> Result<T>;
    fn verify(proof: T) -> Result<bool>;
}
