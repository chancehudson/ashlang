use camino::Utf8PathBuf;
use clap::{arg, Arg, Command};
use compiler::Compiler;
use triton_vm::prelude::*;

mod compiler;
mod parser;
mod vm;

fn cli() -> Command {
    Command::new("acc")
        .about("ashlang compiler")
        .subcommand_required(false)
        .arg_required_else_help(true)
        .arg(arg!(<ASM_PATH> "The source entrypoint"))
        .arg(
            Arg::new("include")
                .short('i')
                .long("include")
                .required(false)
                .help("specify a path to be recursively included")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("print_asm")
                .short('v')
                .long("asm")
                .required(false)
                .num_args(0)
                .help("print the compiled asm before proving"),
        )
}

fn main() {
    let matches = cli().get_matches();
    let source_path = matches
        .get_one::<String>("ASM_PATH")
        .expect("Failed to get ASM_PATH");
    let include_paths = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let mut compiler = Compiler::new();
    compiler.include(Utf8PathBuf::from(source_path));
    for p in include_paths {
        if p.is_empty() {
            continue;
        }
        compiler.include(Utf8PathBuf::from(p));
    }

    compiler.print_asm = *matches.get_one::<bool>("print_asm").unwrap_or(&false);
    let asm = compiler.compile(&Utf8PathBuf::from(source_path));

    let instructions = triton_vm::parser::parse(&asm).unwrap();
    let l_instructions = triton_vm::parser::to_labelled_instructions(instructions.as_slice());
    let program = triton_vm::program::Program::new(l_instructions.as_slice());

    let public_input = PublicInput::from([]);
    let non_determinism = NonDeterminism::default();
    let (_stark, _claim, _proof) =
        triton_vm::prove_program(&program, public_input, non_determinism).unwrap();
    println!("{:?}", _stark);
    println!("{:?}", _claim);
}
