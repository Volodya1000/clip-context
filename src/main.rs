//! Entry point for the project snapshot command line application.

mod cli;
mod clipboard;
mod stats;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

const HELP: &str = include_str!("help.txt");

fn main() -> ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();

    match arguments.as_slice() {
        [] => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        [argument] if argument == "--help" || argument == "-h" => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        [argument] if argument == "--version" || argument == "-V" => {
            println!(
                "{} {}",
                clip_context::APPLICATION_NAME,
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::SUCCESS
        }
        [mode, path, flags @ ..]
            if mode == "--stdout"
                || mode == "-s"
                || mode == "--tree"
                || mode == "-t"
                || mode == "--clipboard"
                || mode == "-c" =>
        {
            cli::run_snapshot(mode, path, flags)
        }
        _ => {
            eprintln!("Unsupported arguments. Run cpctx --help.");
            ExitCode::from(2)
        }
    }
}
