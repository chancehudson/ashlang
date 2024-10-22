use std::str::FromStr;

use anyhow::Result;
use triton_vm::prelude::BFieldElement;
use triton_vm::program::NonDeterminism;
use triton_vm::program::PublicInput;
use triton_vm::proof::Claim;
use triton_vm::proof::Proof;
use triton_vm::stark::Stark;

use super::ashlang_prover::AshlangProver;
use crate::cli::Config;
use crate::compiler::Compiler;
use crate::log;
use crate::rings::OxfoiPolynomialRing;

/// Bindings for executing ashlang programs in the [tritonVM/triton-vm](https://github.com/TritonVM/triton-vm/) prover.
pub struct TritonVMProver {}

impl AshlangProver<(Stark, Claim, Proof)> for TritonVMProver {
    fn prove_ir(
        asm: &str,
        public_inputs: Vec<String>,
        secret_inputs: Vec<String>,
    ) -> Result<(Stark, Claim, Proof)> {
        // then attempt to prove the assembly in TritonVM
        let instructions = triton_vm::parser::parse(asm);
        if let Err(e) = instructions {
            return log::error!(&format!("Failed to parse compiled tasm: {:?}", e));
        }
        let instructions = instructions.unwrap();
        let l_instructions = triton_vm::parser::to_labelled_instructions(instructions.as_slice());
        let program = triton_vm::program::Program::new(l_instructions.as_slice());
        let public_inputs = PublicInput::from(
            public_inputs
                .clone()
                .into_iter()
                .map(|v| BFieldElement::from_str(&v).unwrap())
                .collect::<Vec<_>>(),
        );
        let secret_inputs = NonDeterminism::from(
            secret_inputs
                .clone()
                .into_iter()
                .map(|v| BFieldElement::from_str(&v).unwrap())
                .collect::<Vec<_>>(),
        );

        Ok(triton_vm::prove_program(
            &program,
            public_inputs,
            secret_inputs,
        )?)
    }

    fn prove(config: &Config) -> Result<(Stark, Claim, Proof)> {
        let mut config = config.clone();
        if config.field != "foi" && config.field != "goldilocks" {
            return log::error!(
                &format!("Unsupported field for target tasm: {}", config.field),
                "tasm only support execution in the foi (goldilocks) field"
            );
        }
        // adjust the extension priorities on the config for TritonVM
        config.extension_priorities.push("tasm".to_string());
        // get a compiler instance in the oxfoi field
        let mut compiler: Compiler<OxfoiPolynomialRing> = Compiler::new(&config)?;

        // compile as needed
        //
        let asm = compiler.compile(&config.entry_fn)?;
        // generate the proof
        Self::prove_ir(&asm, config.inputs, config.secret_inputs)
    }

    fn verify(_program: &str, _proof: (Stark, Claim, Proof)) -> Result<bool> {
        panic!("tritonvm verification not implemented");
    }
}
