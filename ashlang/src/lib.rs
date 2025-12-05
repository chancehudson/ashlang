mod cli;
mod compiler;
mod log;
/// Ashlang source code parser.
mod parser;
mod provers;
/// Core logic for the r1cs target.
mod r1cs;
mod time;

// Expose provers at the top level export here
pub use compiler::*;
pub use parser::*;
pub use provers::*;
pub use r1cs::*;

use zkpo::ZKArg;
use zkpo::ZKProgram;

use anyhow::Result;
use lettuce::*;

pub struct AshlangInnerProdArg<E: FieldScalar> {
    pub arg: InnerProdR1CS<E>,
}

impl<E: FieldScalar> AshlangInnerProdArg<E> {}

impl<E: FieldScalar> ZKArg<E> for AshlangInnerProdArg<E> {
    fn new(program: impl ZKProgram<E>, input: Vector<E>) -> Result<Self> {
        let oraccle = Oraccle::new();
        // let start = Instant::now();
        let input_len = input.len();
        let wtns = program.compute_wtns(input)?;
        // print!(
        //     " {} (?)\nVerifying transparent inner product argument... ",
        //     format!("{} ms", start.elapsed().as_millis())
        // );
        Ok(Self {
            arg: lettuce::InnerProdR1CS::new(wtns, program.r1cs(input_len)?, &oraccle)?,
        })
    }

    fn verify(self) -> Result<()> {
        let oraccle = Oraccle::new();
        self.arg.verify(&oraccle)?;
        Ok(())
    }

    fn outputs(&self) -> impl Iterator<Item = E> {
        // TODO: outputs
        vec![].into_iter()
    }
}

pub fn cli_main() -> Result<()> {
    type E = MilliScalarMont;
    let mut config = cli::parse()?;
    config.extension_priorities.push("ar1cs".to_string());

    // we'll take an entry and compile to a single string ashlang file.
    let compiler: Compiler<E> = Compiler::new(&mut config)?;
    let ashlang_program = compiler.combine_src(&config.entry_fn)?;

    println!("ashlang source program: \n{}", ashlang_program.src);

    println!(
        "compiled to ar1cs: \n{}",
        ashlang_program.ar1cs_src(config.input.len())?
    );
    println!("{}", ashlang_program.r1cs(config.input.len())?);

    let output = match config.arg_fn.as_str() {
        "innerprod" => {
            print!("\nBuilding argument of knowledge...");
            let arg = AshlangInnerProdArg::new(ashlang_program, config.input)?;
            println!("\nVerifying transparent inner prod argument...");
            let outputs = arg.outputs().collect::<Vector<_>>();
            arg.verify()?;
            outputs
        }
        v => anyhow::bail!("unknown argument \"{}\". valid options: \"innerprod\"", v),
    };
    if output.len() > 0 {
        println!("Received the following outputs: {output}");
    } else {
        println!("No outputs were generated 🟡");
    }
    println!("🔮 program exists and was executed");

    Ok(())
}
