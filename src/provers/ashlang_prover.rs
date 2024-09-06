use anyhow::Result;

use crate::cli::Config;

// TODO: make the resulting proof serializable here
// e.g. `AshlangProver<T: Debug>`
pub trait AshlangProver<T> {
    fn prove(config: &Config) -> Result<T>;
    fn verify(proof: T) -> Result<bool>;
}
