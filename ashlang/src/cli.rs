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
    pub extension_priorities: Vec<String>,
    pub entry_fn: String,
    pub arg_fn: String,
}

#[allow(dead_code)]
pub fn parse() -> Result<Config> {
    let matches = cli().get_matches();
    let entry_fn = matches
        .get_one::<String>("ENTRY_FN")
        .expect("Failed to get ENTRY_FN")
        .to_string();
    let arg_fn = matches
        .get_one::<String>("ARG_FN")
        .expect("Failed to get ARG_FN")
        .to_string();

    // TODO: unfuck this up
    let mut include_paths = matches
        .get_many::<String>("include")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(Utf8PathBuf::from)
        .collect::<Vec<_>>();
    include_paths.push(".".into());

    let inputs = matches.get_one::<String>("inputs");
    let mut verbosity = 0_u8;
    if *matches.get_one::<bool>("print_asm").unwrap_or(&false) {
        verbosity = 1;
    }
    Ok(Config {
        include_paths,
        verbosity,
        inputs: parse_inputs(inputs),
        extension_priorities: vec!["ash".to_string()],
        entry_fn,
        arg_fn,
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
        .about("ashlang pq cryptography kit")
        .subcommand_required(false)
        .arg_required_else_help(true)
        .arg(arg!(<ENTRY_FN> "name of function to run"))
        .arg(arg!(<ARG_FN> "name of argument to create"))
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .required(false)
                .help("private inputs to the program"),
        )
}
