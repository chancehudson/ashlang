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
    input_len: usize,
    static_args: Vec<Vector<E>>,
}

impl<E: FieldScalar> AshlangInnerProdArg<E> {}

impl<E: FieldScalar> ZKArg<E> for AshlangInnerProdArg<E> {
    type Program = AshlangProgram<E>;

    fn new(
        program: AshlangProgram<E>,
        input: Vector<E>,
        static_args: Vec<Vector<E>>,
    ) -> Result<Self> {
        let oraccle = Oraccle::new();
        // let start = Instant::now();
        let input_len = input.len();
        let wtns = program.compute_wtns(input, static_args.clone())?;
        // print!(
        //     " {} (?)\nVerifying transparent inner product argument... ",
        //     format!("{} ms", start.elapsed().as_millis())
        // );
        Ok(Self {
            arg: lettuce::InnerProdR1CS::new(
                wtns,
                program.r1cs(input_len, static_args.clone())?,
                &oraccle,
            )?,
            input_len,
            static_args,
        })
    }

    fn verify(self, program: AshlangProgram<E>) -> Result<impl Iterator<Item = E>> {
        let oraccle = Oraccle::new();
        Ok(self
            .arg
            .verify(
                program.r1cs(self.input_len, self.static_args.clone())?,
                oraccle,
            )?
            .into_iter())
    }

    fn outputs(&self, program: AshlangProgram<E>) -> Result<impl Iterator<Item = E>> {
        let r1cs = program.r1cs(self.input_len, self.static_args.clone())?;
        Ok(self.arg.outputs(r1cs.output_mask))
    }
}

pub fn cli_main() -> Result<()> {
    type E = MilliScalarMont;
    let mut config = cli::parse()?;
    let static_args = config
        .statics
        .iter()
        .map(|v| vec![(*v as u128).into()].into())
        .collect::<Vec<Vector<E>>>();

    // we'll take an entry and compile to a single string ashlang file.
    let compiler: Compiler<E> = Compiler::<E>::new(&mut config)?;
    let ashlang_program = compiler.combine_src(&config.entry_fn)?;

    println!("ashlang source program: \n{}", ashlang_program.src);

    // println!(
    //     "compiled to ar1cs: \n{}",
    //     ashlang_program.ar1cs_src(config.input.len(), &config.statics)?
    // );
    println!(
        "{}",
        ashlang_program.r1cs(config.input.len(), static_args.clone())?
    );
    println!(
        "\n\nWitness computation script: \n{}",
        ashlang_program
            .as_r1cs(config.input.len(), static_args.clone())?
            .1
            .src
    );

    let output = match config.arg_fn.as_str() {
        "innerprod" => {
            print!("\nBuilding argument of knowledge...");
            let arg = AshlangInnerProdArg::new(
                ashlang_program.clone(),
                config.input,
                static_args.clone(),
            )?;
            println!("\nVerifying transparent inner prod argument...");
            let outputs = arg.verify(ashlang_program)?;
            outputs.collect::<Vector<_>>()
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
