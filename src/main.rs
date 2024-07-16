use clap::{arg, Command};
use compiler::Compiler;
use std::path::PathBuf;
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
    // .arg(arg!(<INCLUDE_PATH> "The include path (recursive)"))
}

fn main() {
    let matches = cli().get_matches();

    let source_path = matches
        .get_one::<String>("ASM_PATH")
        .expect("Failed to get ASM_PATH");
    // let include_path = matches.get_one::<String>("INCLUDE_PATH").expect("Failed to get INCLUDE_PATH");
    let mut compiler = Compiler::new();
    compiler.include(PathBuf::from(source_path));

    let asm = compiler.compile(&PathBuf::from(source_path));

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
