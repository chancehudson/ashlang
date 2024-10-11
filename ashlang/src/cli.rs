use anyhow::Result;
use camino::Utf8PathBuf;
use clap::arg;
use clap::Arg;
use clap::Command;

use crate::log;

/// Compiler configuration. Contains all fields necessary to compile an ashlang program.
#[derive(Clone, Debug)]
pub struct Config {
    pub include_paths: Vec<Utf8PathBuf>,
    pub verbosity: u8,
    pub inputs: Vec<String>,
    pub secret_inputs: Vec<String>,
    pub target: String,
    pub extension_priorities: Vec<String>,
    pub entry_fn: String,
    pub field: String,
}

#[allow(dead_code)]
pub fn parse() -> Result<Config> {
    let matches = cli().get_matches();
    let entry_fn = matches
        .get_one::<String>("ENTRY_FN")
        .expect("Failed to get ENTRY_FN");
    let target = matches.get_one::<String>("target");
    let field = matches.get_one::<String>("field");
    let include_paths = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(Utf8PathBuf::from)
        .collect::<Vec<_>>();
    let inputs = matches.get_one::<String>("public_inputs");
    let secret_inputs = matches.get_one::<String>("secret_inputs");
    let mut verbosity = 0_u8;
    if *matches.get_one::<bool>("print_asm").unwrap_or(&false) {
        verbosity = 1;
    }
    if target.is_none() {
        return log::error!(
            "No target specified",
            "specify a target using -t [r1cs | tasm]"
        );
    }
    if field.is_none() {
        return log::error!(
            "No field specified",
            "specify a field using -f [foi | alt_bn128 | curve25519]"
        );
    }
    let target = target.unwrap().clone();
    let field = field.unwrap().clone();
    Ok(Config {
        include_paths,
        target,
        field,
        verbosity,
        inputs: parse_inputs(inputs),
        secret_inputs: parse_inputs(secret_inputs),
        extension_priorities: vec!["ash".to_string()],
        entry_fn: entry_fn.to_string(),
    })
}

fn parse_inputs(inputs: Option<&String>) -> Vec<String> {
    if let Some(i) = inputs {
        i.split(',')
            .filter(|v| !v.is_empty())
            .map(|v| v.parse().unwrap())
            .collect()
    } else {
        vec![]
    }
}

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
            Arg::new("field")
                .short('f')
                .long("scalar field to execute in")
                .required(false)
                .help("the name of the scalar field that should be used for proving: foi (goldilocks), alt_bn128, curve25519"),
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
