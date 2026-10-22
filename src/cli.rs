//! Parsing CLI file-selection flags and resolving their precedence.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clip_context::profile::{DEFAULT_CONFIG_FILE, load_profile};
use clip_context::snapshot::{SelectionOptions, collect_project_with_options};

#[derive(Default)]
struct SelectionArguments {
    profile: Option<String>,
    config_file: Option<PathBuf>,
    extensions: Vec<String>,
    excluded_directories: Vec<String>,
}

pub(super) fn run_snapshot(mode: &OsStr, path: &OsStr, flags: &[OsString]) -> ExitCode {
    let arguments: SelectionArguments = match parse_selection(flags) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message} Run cpctx --help.");
            return ExitCode::from(2);
        }
    };

    let root: &Path = Path::new(path);
    let options: SelectionOptions = match resolve_selection(root, &arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    match collect_project_with_options(root, &options) {
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

fn parse_selection(flags: &[OsString]) -> Result<SelectionArguments, &'static str> {
    let mut arguments = SelectionArguments::default();
    let mut pairs = flags.chunks_exact(2);

    for pair in &mut pairs {
        let flag: &str = pair[0].to_str().ok_or("Option names must be UTF-8.")?;
        let value: &str = pair[1].to_str().ok_or("Option values must be UTF-8.")?;

        match flag {
            "--ext" if SelectionOptions::valid_extension(value) => {
                arguments.extensions.push(value.to_owned());
            }
            "--exclude-dir" if SelectionOptions::valid_directory_name(value) => {
                arguments.excluded_directories.push(value.to_owned());
            }
            "--profile"
                if arguments.profile.is_none()
                    && !value.is_empty()
                    && value.bytes().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_')
                    }) =>
            {
                arguments.profile = Some(value.to_owned());
            }
            "--config" if arguments.config_file.is_none() && !value.is_empty() => {
                arguments.config_file = Some(PathBuf::from(value));
            }
            _ => return Err("Invalid file selection option."),
        }
    }

    if !pairs.remainder().is_empty() {
        return Err("File selection options require a value.");
    }
    if arguments.config_file.is_some() && arguments.profile.is_none() {
        return Err("--config requires --profile.");
    }

    Ok(arguments)
}

fn resolve_selection(root: &Path, args: &SelectionArguments) -> io::Result<SelectionOptions> {
    let mut options = if let Some(profile_name) = args.profile.as_deref() {
        let config_file = root.join(
            args.config_file
                .as_deref()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE)),
        );
        load_profile(&config_file, profile_name)?
    } else {
        SelectionOptions::default()
    };

    if !args.extensions.is_empty() {
        options.clear_extensions();
    }
    for extension in &args.extensions {
        options.include_extension(extension);
    }
    for name in &args.excluded_directories {
        options.exclude_directory(name);
    }

    Ok(options)
}
