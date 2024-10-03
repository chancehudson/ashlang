mod cli;
pub mod compiler;
pub mod log;
pub mod parser;
mod provers;
pub mod r1cs;
pub mod rings;
pub mod tasm;

pub use cli::Config;

// Expose provers at the top level export here
// e.g. use ashlang::SpartanProver;
pub use provers::AshlangProver;
#[cfg(feature = "spartan-prover")]
pub use provers::SpartanProver;
#[cfg(feature = "tritonvm-prover")]
pub use provers::TritonVMProver;
