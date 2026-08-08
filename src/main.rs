mod analysis;
mod cli;
mod exit_code;
mod graph;
mod input;
mod model;
mod output;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            exit_code::failure()
        }
    }
}
