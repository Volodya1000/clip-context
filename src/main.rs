//! Minimal command-line frontend for the project snapshot engine.

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::ExitCode;

use clip_context::snapshot::{SelectionOptions, collect_project_with_options};

const HELP: &str = "clip-context: prepare project context for sharing\n\nUsage:\n  cpctx -s PATH [OPTIONS]   Print source files and the directory tree\n  cpctx -t PATH [OPTIONS]   Print the filtered directory tree\n  cpctx --help              Show this message\n  cpctx --version           Show the version\n\nOptions:\n  --ext EXT                 Only include files of this extension (repeatable)\n  --exclude-dir NAME        Exclude a directory named NAME at any depth (repeatable)\n\nLong forms: --stdout, --tree. Clipboard and secret scanning are not implemented yet.";

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
            run_snapshot(mode, path, flags)
        }
        _ => {
            eprintln!("Unsupported arguments. Run cpctx --help.");
            ExitCode::from(2)
        }
    }
}

fn run_snapshot(mode: &OsStr, path: &OsStr, flags: &[OsString]) -> ExitCode {
    let options: SelectionOptions = match parse_selection(flags) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message} Run cpctx --help.");
            return ExitCode::from(2);
        }
    };

    match collect_project_with_options(Path::new(path), &options) {
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

fn parse_selection(flags: &[OsString]) -> Result<SelectionOptions, &'static str> {
    let mut options = SelectionOptions::default();
    let mut pairs = flags.chunks_exact(2);

    for pair in &mut pairs {
        let flag: &str = pair[0].to_str().ok_or("Option names must be UTF-8.")?;
        let value: &str = pair[1].to_str().ok_or("Option values must be UTF-8.")?;

        match flag {
            "--ext" if !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric()) => {
                options.include_extension(value);
            }
            "--exclude-dir"
                if !value.is_empty()
                    && value != "."
                    && value != ".."
                    && !value.contains('/')
                    && !value.contains('\\') =>
            {
                options.exclude_directory(value);
            }
            _ => return Err("Invalid file selection option."),
        }
    }

    if !pairs.remainder().is_empty() {
        return Err("File selection options require a value.");
    }

    Ok(options)
}
