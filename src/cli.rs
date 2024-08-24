use camino::Utf8PathBuf;
use clap::arg;
use clap::Arg;
use clap::Command;

pub struct Config {
    pub include_paths: Vec<Utf8PathBuf>,
    pub verbosity: u8,
    pub inputs: Vec<String>,
    pub secret_inputs: Vec<String>,
    pub target: String,
    pub extension_priorities: Vec<String>,
    pub entry_fn: String,
}
impl Config {}

pub fn parse() -> Config {
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
        .filter(|v| !v.is_empty())
        .map(|v| Utf8PathBuf::from(v))
        .collect::<Vec<_>>();
    let inputs = matches.get_one::<String>("public_inputs");
    let secret_inputs = matches.get_one::<String>("secret_inputs");
    let mut verbosity = 0_u8;
    if *matches.get_one::<bool>("print_asm").unwrap_or(&false) {
        verbosity = 1;
    }
    if target.len() > 1 {
        println!("Multiple targets not supported yet");
        std::process::exit(1);
    }
    if target.is_empty() {
        println!("No target specified");
        std::process::exit(1);
    }
    Config {
        include_paths,
        target: target[0].to_string(),
        verbosity,
        inputs: parse_inputs(inputs),
        secret_inputs: parse_inputs(secret_inputs),
        extension_priorities: vec!["ash".to_string()],
        entry_fn: entry_fn.to_string(),
    }
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
