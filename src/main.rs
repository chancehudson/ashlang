use camino::Utf8PathBuf;
use clap::{arg, Arg, Command};
use compiler::Compiler;
use triton_vm::prelude::*;

mod compiler;
mod log;
mod parser;
mod vm;

fn cli() -> Command {
    Command::new("acc")
        .about("ashlang compiler")
        .subcommand_required(false)
        .arg_required_else_help(true)
        .arg(arg!(<SRC_PATH> "The source entrypoint"))
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
        .arg(
            Arg::new("public_inputs")
                .short('p')
                .long("public")
                .required(false)
                .help("public inputs to the program"),
        )
        .arg(
            Arg::new("secret_inputs")
                .short('s')
                .long("secret")
                .required(false)
                .help("secret inputs to the program"),
        )
}

fn parse_inputs(inputs: Option<&String>) -> Vec<BFieldElement> {
    if let Some(i) = inputs {
        i.split(',')
            .filter(|v| !v.is_empty())
            .map(|v| v.parse().unwrap())
            .collect()
    } else {
        vec![]
    }
}

fn main() {
    let matches = cli().get_matches();
    let source_path = matches
        .get_one::<String>("SRC_PATH")
        .expect("Failed to get SRC_PATH");
    let include_paths = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let public_inputs = matches.get_one::<String>("public_inputs");
    let secret_inputs = matches.get_one::<String>("secret_inputs");
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

    let public_inputs = PublicInput::from(parse_inputs(public_inputs));
    let secret_inputs = NonDeterminism::from(parse_inputs(secret_inputs));
    let (_stark, _claim, _proof) =
        triton_vm::prove_program(&program, public_inputs, secret_inputs).unwrap();
    println!("{:?}", _stark);
    println!("{:?}", _claim);
}
