use colored::Colorize;

// compiler errors always halt the program
pub fn compile_error(msg: &str, details: Option<&str>) {
    println!("{}", "Compile error".red().bold());
    println!("{msg}");
    if let Some(details) = details {
        println!("{}", "Explanation".green().bold());
        println!("{details}");
    }
    std::process::exit(1);
}

pub fn parse_error<T: pest::RuleType>(err: pest::error::Error<T>) {
    println!("{}", "Parse error".red().bold());
    println!("{err}");
    std::process::exit(1);
}
