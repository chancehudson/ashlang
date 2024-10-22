use anyhow::Result;
use ashlang::rings::Curve25519PolynomialRing;
use cli::Config;
use compiler::Compiler;
use r1cs::witness;
use ring_math::PolynomialRingElement;
use scalarff::Curve25519FieldElement;
use scalarff::FieldElement;

use crate::provers::AshlangProver;
use crate::rings::Bn128PolynomialRing;
use crate::rings::DilithiumPolynomialRingElement;
use crate::rings::OxfoiPolynomialRing;

mod cli;
mod compiler;
mod log;
mod parser;
mod provers;
mod r1cs;
mod rings;
mod tasm;
mod time;

fn main() -> Result<()> {
    let mut config = cli::parse()?;
    return match config.target.as_str() {
        "tasm" => match provers::TritonVMProver::prove(&config) {
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
                println!("⚠️ Field specifier \"foi\" is deprecated and will be removed. Instead use \"oxfoi\"");
                compile_r1cs::<OxfoiPolynomialRing>(&mut config)?;
                Ok(())
            }
            "oxfoi" => {
                compile_r1cs::<OxfoiPolynomialRing>(&mut config)?;
                Ok(())
            }
            "curve25519" => {
                let r1cs = compile_r1cs::<Curve25519PolynomialRing>(&mut config)?;
                let proof =
                    provers::SpartanProver::prove_ir(&r1cs, config.inputs, config.secret_inputs)?;
                if provers::SpartanProver::verify(&r1cs, proof)? {
                    println!("✅ spartan proof is valid");
                } else {
                    println!("🔴 spartan proof is NOT valid");
                }
                Ok(())
            }
            "alt_bn128" => {
                compile_r1cs::<Bn128PolynomialRing>(&mut config)?;
                Ok(())
            }
            "dilithium" => {
                compile_r1cs::<DilithiumPolynomialRingElement>(&mut config)?;
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

/// Used to compile and verify r1cs that does not yet have a default prover
fn compile_r1cs<T: PolynomialRingElement>(config: &mut Config) -> Result<String> {
    config.extension_priorities.push("ar1cs".to_string());
    let mut compiler: Compiler<T> = Compiler::new(config)?;

    let constraints = compiler.compile(&config.entry_fn)?;

    let witness = witness::build::<T>(
        &constraints,
        config
            .secret_inputs
            .iter()
            .map(|v| T::from_str(v))
            .collect::<Result<Vec<_>>>()?,
    );
    if let Err(e) = witness {
        println!("Failed to build witness: {:?}", e);
        std::process::exit(1);
    }
    let witness = witness.unwrap();

    let solved = witness::verify::<T>(&constraints, witness);
    if let Err(e) = solved {
        println!("Failed to solve r1cs: {:?}", e);
        std::process::exit(1);
    }
    println!();
    println!("R1CS: built and validated witness ✅");
    let outputs = solved?;
    if !outputs.is_empty() {
        println!("Received the following outputs:");
        for v in outputs {
            println!("{v}");
        }
    } else {
        println!("No outputs were generated");
    }
    Ok(constraints)
}
