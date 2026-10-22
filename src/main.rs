//! Minimal command-line frontend for the project snapshot engine.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use clip_context::snapshot::collect_project;

const HELP: &str = "clip-context: prepare project context for sharing\n\nUsage:\n  cpctx --stdout PATH, -s PATH  Print project source code and a directory tree\n  cpctx --tree PATH, -t PATH    Print the filtered directory tree\n  cpctx --help                  Show this message\n  cpctx --version               Show the version\n\nClipboard, SSH, custom selection and secret scanning are not implemented yet.";

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
        [mode, path] if mode == "--stdout" || mode == "-s" || mode == "--tree" || mode == "-t" => {
            match collect_project(Path::new(path)) {
                Ok(snapshot) => {
                    if mode == "--tree" || mode == "-t" {
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
            eprintln!("Unsupported arguments. Run cpctx --help.");
            ExitCode::from(2)
        }
    }
}
