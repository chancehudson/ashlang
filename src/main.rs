use std::str::FromStr;

use cli::Config;
use compiler::Compiler;
use r1cs::witness;
use scalarff::alt_bn128::Bn128FieldElement;
use scalarff::curve_25519::Curve25519FieldElement;
use scalarff::foi::FoiFieldElement;
use scalarff::FieldElement;
use triton_vm::prelude::*;

mod cli;
mod compiler;
mod log;
mod parser;
mod r1cs;
mod tasm;

fn main() {
    let mut config = cli::parse();
    match config.target.as_str() {
        "tasm" => {
            if config.field != "foi" && config.field != "goldilocks" {
                log::error!(
                    &format!("Unsupported field for target tasm: {}", config.field),
                    "tasm only support execution in the foi (goldilocks) field"
                );
            }
            compile_tasm(&mut config);
        }
        "r1cs" => match config.field.as_str() {
            "foi" => {
                compile_r1cs::<FoiFieldElement>(&mut config);
            }
            "curve25519" => {
                compile_r1cs::<Curve25519FieldElement>(&mut config);
            }
            "alt_bn128" => {
                compile_r1cs::<Bn128FieldElement>(&mut config);
            }
            _ => {
                log::error!(&format!(
                    "Unsupported field for target r1cs: {}",
                    config.field
                ));
            }
        },
        _ => {
            println!("Unsupported target: {}", config.target);
            std::process::exit(1);
        }
    }
}

fn compile_r1cs<T: FieldElement>(config: &mut Config) {
    config.extension_priorities.push("ar1cs".to_string());
    let mut compiler: Compiler<T> = Compiler::new(config);

    let constraints = compiler.compile(&config.entry_fn, &config.target);

    let witness = witness::build::<T>(&constraints);
    if let Err(e) = witness {
        println!("Failed to build witness: {:?}", e);
        std::process::exit(1);
    }
    let witness = witness.unwrap();

    if let Err(e) = witness::verify::<T>(&constraints, witness) {
        println!("Failed to solve r1cs: {:?}", e);
        std::process::exit(1);
    } else {
        println!();
        println!("R1CS: built and validated witness ✅");
    }
}

fn compile_tasm(config: &mut Config) {
    config.extension_priorities.push("tasm".to_string());

    let mut compiler: Compiler<FoiFieldElement> = Compiler::new(config);
    let asm = compiler.compile(&config.entry_fn, &config.target);

    let instructions = triton_vm::parser::parse(&asm);
    if let Err(e) = instructions {
        println!("Failed to parse tasm: {:?}", e);
        std::process::exit(1);
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

    match triton_vm::prove_program(&program, public_inputs, secret_inputs) {
        Ok((_stark, _claim, _proof)) => {
            println!("{:?}", _stark);
            println!("{:?}", _claim);
        }
        Err(e) => {
            println!("Triton VM errored");
            println!("{e}");
            std::process::exit(1);
        }
    }
}
