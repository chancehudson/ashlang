use std::{str::FromStr, time::Instant};

use anyhow::Result;

use lettuce::*;

use compiler::Compiler;
use lettuce::Oraccle;
use r1cs::arithm::Arithmetizer;

mod cli;
mod compiler;
mod log;
mod parser;
mod provers;
mod r1cs;
mod time;

fn main() -> Result<()> {
    type E = MilliScalarMont;
    let mut config = cli::parse()?;
    config.extension_priorities.push("ar1cs".to_string());
    let input = config
        .input
        .iter()
        .map(|v| E::from(u128::from_str(v).unwrap()))
        .collect();

    let mut compiler: Compiler<E> = Compiler::new(&mut config)?;
    let ar1cs_src = compiler.compile(&config.entry_fn)?;
    println!("{}\n", ar1cs_src);

    let mut arithm = Arithmetizer::new(&ar1cs_src)?;
    println!("{}", arithm.r1cs);
    print!("Building witness...");
    let output_len = arithm.compute_wtns(input)?;
    print!(" ✅\nVerifying witness...");
    println!("{}", arithm.wtns.as_ref().unwrap());
    let wtns = arithm.assert_wtns()?;
    if output_len > 0 {
        println!("Received the following outputs:");
        for v in arithm.outputs()? {
            println!("{v}");
        }
    } else {
        print!("No outputs were generated 🟡");
    }
    print!("\nwitness is consistent ✅\nBuilding argument of knowledge...");

    match config.arg_fn.as_str() {
        "innerprod" => {
            let oraccle = Oraccle::new();
            let start = Instant::now();
            let innerprod_arg = lettuce::InnerProdR1CS::new(wtns.clone(), &arithm.r1cs, &oraccle)?;
            print!(
                " {} (?)\nVerifying transparent inner product argument... ",
                format!("{} ms", start.elapsed().as_millis())
            );
            let oraccle = Oraccle::new();
            let start = Instant::now();
            innerprod_arg.verify(&oraccle)?;
            println!(" {} ✅", format!("{} ms", start.elapsed().as_millis()));
            println!(
                "{} constraints, {} variables",
                arithm.r1cs.height(),
                arithm.r1cs.width()
            )
        }
        v => anyhow::bail!("unknown argument \"{}\". valid options: \"innerprod\"", v),
    }
    println!("🔮 program exists and was executed");

    Ok(())
}
