use camino::Utf8PathBuf;
use clap::{arg, Arg, Command};
use compiler::Compiler;
use triton_vm::prelude::*;

mod compiler;
mod log;
mod parser;
mod r1cs;
mod tasm;

fn cli() -> Command {
    Command::new("acc")
        .about("ashlang compiler")
        .subcommand_required(false)
        .arg_required_else_help(true)
        .arg(arg!(<ENTRY_FN> "The entrypoint function name"))
        .arg(
            Arg::new("target")
                .short('t')
                .long("target")
                .required(false)
                .help("the output compile target")
                .action(clap::ArgAction::Append),
        )
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
                .long("print")
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
    let entry_fn = matches
        .get_one::<String>("ENTRY_FN")
        .expect("Failed to get ENTRY_FN");
    let target = matches
        .get_many::<String>("target")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let include_paths = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let public_inputs = matches.get_one::<String>("public_inputs");
    let secret_inputs = matches.get_one::<String>("secret_inputs");
    if target.len() > 1 {
        println!("Multiple targets not supported yet");
        std::process::exit(1);
    }
    if target.is_empty() {
        println!("No target specified");
        std::process::exit(1);
    }
    let target = target[0];
    match target {
        "tasm" => {
            let mut compiler = Compiler::new(vec!["ash".to_string(), "tasm".to_string()]);
            for p in include_paths {
                if p.is_empty() {
                    continue;
                }
                if let Err(err) = compiler.include(Utf8PathBuf::from(p)) {
                    println!("Failed to include path: {:?}", err);
                    std::process::exit(1);
                }
            }
            compiler.print_asm = *matches.get_one::<bool>("print_asm").unwrap_or(&false);
            let asm = compiler.compile(entry_fn, target);

            let instructions = triton_vm::parser::parse(&asm).unwrap();
            let l_instructions =
                triton_vm::parser::to_labelled_instructions(instructions.as_slice());
            let program = triton_vm::program::Program::new(l_instructions.as_slice());

            let public_inputs = PublicInput::from(parse_inputs(public_inputs));
            let secret_inputs = NonDeterminism::from(parse_inputs(secret_inputs));
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
        "r1cs" => {
            let mut compiler = Compiler::new(vec!["ash".to_string(), "r1cs".to_string()]);
            for p in include_paths {
                if p.is_empty() {
                    continue;
                }
                if let Err(err) = compiler.include(Utf8PathBuf::from(p)) {
                    println!("Failed to include path: {:?}", err);
                    std::process::exit(1);
                }
            }
            compiler.print_asm = *matches.get_one::<bool>("print_asm").unwrap_or(&false);
            let constraints = compiler.compile(entry_fn, target);
        }
        _ => {
            println!("Unsupported target: {}", target);
            std::process::exit(1);
        }
    }
}
