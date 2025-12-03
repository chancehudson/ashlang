use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Arg;
use clap::Command;
use clap::arg;

use crate::log;

/// Compiler configuration. Contains all fields necessary to compile an ashlang program.
#[derive(Clone, Debug)]
pub struct Config {
    pub include_paths: Vec<Utf8PathBuf>,
    pub verbosity: u8,
    pub input: Vec<String>,
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

    // hmmm
    // TODO
    let include_paths = vec![".".into()];

    let input = matches.get_one::<String>("input");
    let verbosity = 0_u8;
    Ok(Config {
        include_paths,
        verbosity,
        input: parse_input(input),
        extension_priorities: vec!["ash".to_string()],
        entry_fn,
        arg_fn,
    })
}

fn parse_input(input: Option<&String>) -> Vec<String> {
    if let Some(i) = input {
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
                .help("private input to the program"),
        )
}
