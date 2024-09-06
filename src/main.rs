use std::str::FromStr;

use anyhow::Result;
use cli::Config;
use compiler::Compiler;
use r1cs::witness;
use scalarff::alt_bn128::Bn128FieldElement;
use scalarff::curve_25519::Curve25519FieldElement;
use scalarff::foi::FoiFieldElement;
use scalarff::FieldElement;

mod cli;
mod compiler;
mod log;
mod parser;
mod provers;
mod r1cs;
mod tasm;

fn main() -> Result<()> {
    let mut config = cli::parse()?;
    return match config.target.as_str() {
        "tasm" => match provers::tritonvm::prove(&config) {
            Ok((_stark, _claim, _proof)) => {
                println!("{:?}", _stark);
                println!("{:?}", _claim);
                Ok(())
            }
            Err(e) => {
                println!("Triton VM errored");
                println!("{e}");
                std::process::exit(1);
            }
        },
        "r1cs" => match config.field.as_str() {
            "foi" => {
                compile_r1cs::<FoiFieldElement>(&mut config)?;
                Ok(())
            }
            "curve25519" => {
                let constraints = compile_r1cs::<Curve25519FieldElement>(&mut config)?;
                let t = ashlang_spartan::transform_r1cs(&constraints)?;
                let proof = ashlang_spartan::prove(t);
                if ashlang_spartan::verify(proof) {
                    println!("✅ spartan proof is valid");
                } else {
                    println!("🔴 spartan proof is NOT valid");
                }
                Ok(())
            }
            "alt_bn128" => {
                compile_r1cs::<Bn128FieldElement>(&mut config)?;
                Ok(())
            }
            _ => {
                return log::error!(&format!(
                    "Unsupported field for target r1cs: {}",
                    config.field
                ));
            }
        },
        _ => {
            return log::error!(&format!("Unsupported target: {}", config.target));
        }
    };
}

fn compile_r1cs<T: FieldElement>(config: &mut Config) -> Result<String> {
    config.extension_priorities.push("ar1cs".to_string());
    let mut compiler: Compiler<T> = Compiler::new(config)?;

    let constraints = compiler.compile(&config.entry_fn)?;

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
    Ok(constraints)
}

fn compile_tasm(config: &mut Config) -> Result<String> {
    config.extension_priorities.push("tasm".to_string());

    let mut compiler: Compiler<FoiFieldElement> = Compiler::new(config)?;
    compiler.compile(&config.entry_fn)
}
