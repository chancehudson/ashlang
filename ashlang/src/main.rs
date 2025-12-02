use std::str::FromStr;

use anyhow::Result;

use lettuce::FieldScalar;
use lettuce::MilliScalarMont;

use cli::Config;
use compiler::Compiler;
use lettuce::Oraccle;
use r1cs::parser::R1csParser;
use r1cs::witness;

mod cli;
mod compiler;
mod log;
mod parser;
mod provers;
mod r1cs;
mod time;

fn main() -> Result<()> {
    let mut config = cli::parse()?;
    compile_r1cs::<MilliScalarMont>(&mut config)?;
    Ok(())
}

/// Used to compile and verify r1cs that does not yet have a default prover
fn compile_r1cs<E: FieldScalar>(config: &mut Config) -> Result<String> {
    config.extension_priorities.push("ar1cs".to_string());
    let mut compiler: Compiler<E> = Compiler::new(config)?;

    let ar1cs = compiler.compile(&config.entry_fn)?;
    let compiled_str = ar1cs.clone();
    let r1cs_parser = R1csParser::new(&ar1cs)?;

    let witness = witness::build::<E>(
        r1cs_parser.clone(),
        config
            .inputs
            .iter()
            .map(|v| Ok(E::from(u128::from_str(v)?)))
            .collect::<Result<Vec<_>>>()?,
    )?;
    let r1cs = r1cs_parser.into_r1cs();

    let solved = witness::verify::<E>(&r1cs_parser, witness.clone());
    if let Err(e) = solved {
        println!("Failed to solve r1cs: {:?}", e);
        std::process::exit(1);
    }
    let outputs = solved?;
    println!("{}", compiled_str);
    println!("R1CS: built and validated witness ✅");
    if !outputs.is_empty() {
        println!("Received the following outputs:");
        for v in outputs {
            println!("{v}");
        }
    } else {
        println!("No outputs were generated");
    }

    match config.arg_fn.as_str() {
        "innerprod" => {
            let oraccle = Oraccle::new();
            let innerprod_arg =
                lettuce::InnerProdR1CS::new(witness.variables.into(), &r1cs, &oraccle)?;
            println!("Generated transparent argument of knowledge");
            let oraccle = Oraccle::new();
            innerprod_arg.verify(&oraccle)?;
            println!("Verified transparent argument of knowledge");
        }
        v => anyhow::bail!("unknown argument \"{}\". valid options: \"innerprod\"", v),
    }
    Ok(compiled_str)
}
