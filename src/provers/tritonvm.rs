use std::str::FromStr;

use anyhow::Result;
use scalarff::foi::FoiFieldElement;
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

pub struct TritonVMProver {}

impl AshlangProver<(Stark, Claim, Proof)> for TritonVMProver {
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
        let mut compiler: Compiler<FoiFieldElement> = Compiler::new(&config)?;

        // compile as needed
        //
        let asm = compiler.compile(&config.entry_fn)?;
        // then attempt to prove the assembly in TritonVM
        let instructions = triton_vm::parser::parse(&asm);
        if let Err(e) = instructions {
            return log::error!(&format!("Failed to parse compiled tasm: {:?}", e));
        }
        let instructions = instructions.unwrap();
        let l_instructions = triton_vm::parser::to_labelled_instructions(instructions.as_slice());
        let program = triton_vm::program::Program::new(l_instructions.as_slice());
        let public_inputs = PublicInput::from(
            config
                .inputs
                .clone()
                .into_iter()
                .map(|v| BFieldElement::from_str(&v).unwrap())
                .collect::<Vec<_>>(),
        );
        let secret_inputs = NonDeterminism::from(
            config
                .secret_inputs
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

    fn verify(_proof: (Stark, Claim, Proof)) -> Result<bool> {
        panic!("tritonvm verification not implemented");
    }
}
