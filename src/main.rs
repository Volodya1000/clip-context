//! Entry point for the project snapshot command line application.

mod cli;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

const HELP: &str = "clip-context: prepare project context for sharing\n\nUsage:\n  cpctx -s PATH [OPTIONS]   Print selected project source and tree\n  cpctx -t PATH [OPTIONS]   Print selected file tree\n  cpctx --help              Show this message\n  cpctx --version           Show the version\n\nOptions:\n  --ext EXT                 Restrict extensions, repeatable\n  --exclude-dir NAME        Exclude directory names, repeatable\n  --profile NAME            Use a named profile from .cpctx.toml\n  --config FILE             Alternative TOML file (requires --profile)\n\nLong forms: --stdout and --tree. Clipboard and secret scanning are not implemented yet.";

fn main() -> ExitCode {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();

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
        [mode, path, flags @ ..]
            if mode == "--stdout" || mode == "-s" || mode == "--tree" || mode == "-t" =>
        {
            cli::run_snapshot(mode, path, flags)
        }
        _ => {
            eprintln!("Unsupported arguments. Run cpctx --help.");
            ExitCode::from(2)
        }
    }
}
