use colored::Colorize;

macro_rules! error {
    ($msg:expr) => {
        crate::log::compile_error($msg, None);
        unreachable!();
    };
    ($msg:expr, $details:expr) => {
        crate::log::compile_error($msg, Some($details));
        unreachable!();
    };
}
pub(crate) use error;

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
