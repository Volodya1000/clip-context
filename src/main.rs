//! Minimal command-line frontend for the project snapshot engine.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use clip_context::snapshot::collect_project;

const HELP: &str = "clip-context: prepare project context for sharing\n\nUsage:\n  clip-context --stdout PATH  Print project source code and a directory tree\n  clip-context --tree PATH    Print the filtered directory tree\n  clip-context --help         Show this message\n  clip-context --version      Show the version\n\nClipboard, SSH, custom selection and secret scanning are not implemented yet.";

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args_os().skip(1).collect();

    match arguments.as_slice() {
        [] => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [argument] if argument == "--help" || argument == "-h" => {
            println!("{HELP}");
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
        [mode, path] if mode == "--stdout" || mode == "--tree" => {
            match collect_project(Path::new(path)) {
                Ok(snapshot) => {
                    if mode == "--tree" {
                        print!("{}", snapshot.tree());
                    } else {
                        print!("{}", snapshot.to_markdown());
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("Unsupported arguments. Run clip-context --help.");
            ExitCode::from(2)
        }
    }
}
