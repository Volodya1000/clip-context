//! Minimal CLI entry point while the project is being bootstrapped.

use std::env;
use std::process::ExitCode;

const HELP: &str = "clip-context: prepare project context for sharing\n\nUsage: clip-context [--help | --version]\n\nThis is an initial CLI skeleton; copying is not implemented yet.";

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().skip(1).collect();

    if arguments.is_empty() {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }

    if arguments.len() == 1 {
        if arguments[0] == "--help" || arguments[0] == "-h" {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }

        if arguments[0] == "--version" || arguments[0] == "-V" {
            println!(
                "{} {}",
                clip_context::APPLICATION_NAME,
                env!("CARGO_PKG_VERSION")
            );
            return ExitCode::SUCCESS;
        }
    }

    eprintln!("Unsupported arguments. Run clip-context --help.");
    ExitCode::from(2)
}
