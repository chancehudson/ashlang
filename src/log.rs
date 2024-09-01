use anyhow::anyhow;
use anyhow::Error;
use colored::Colorize;

macro_rules! error {
    ($msg:expr) => {
        Err(anyhow::anyhow!(crate::log::compile_error($msg, None)))
    };
    ($msg:expr, $details:expr) => {
        Err(anyhow::anyhow!(crate::log::compile_error(
            $msg,
            Some($details)
        )))
    };
}
pub(crate) use error;

// compiler errors always halt the program
pub fn compile_error(msg: &str, details: Option<&str>) -> String {
    let mut out_strs = vec![];
    out_strs.push(format!("{}", "Compile error".red().bold()));
    out_strs.push(format!("{msg}"));
    if let Some(details) = details {
        out_strs.push(format!("{}", "Explanation".green().bold()));
        out_strs.push(format!("{details}"));
    }
    out_strs.join("\n")
}

pub fn parse_error<T: pest::RuleType>(err: pest::error::Error<T>, filename: &str) -> String {
    let mut out_strs = vec![];
    out_strs.push(format!("{}", "Parse error".red().bold()));
    out_strs.push(format!("In function {filename}"));
    out_strs.push("".to_string());
    out_strs.push(format!("{err}"));
    out_strs.join("\n")
}
