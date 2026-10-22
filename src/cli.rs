//! Parsing CLI file-selection flags and resolving their precedence.

use std::ffi::{OsStr, OsString};
use std::io;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clip_context::profile::{DEFAULT_CONFIG_FILE, load_profile};
use clip_context::remote::collect_remote_project_with_selection;
use clip_context::snapshot::{
    SelectionOptions, SnapshotLimits, collect_project_with_options_and_limits,
};

#[derive(Default)]
struct SelectionArguments {
    profile: Option<String>,
    config_file: Option<PathBuf>,
    extensions: Vec<String>,
    excluded_directories: Vec<String>,
    limits: SnapshotLimits,
    ssh_destination: Option<String>,
}

pub fn run_snapshot(mode: &OsStr, path: &OsStr, flags: &[OsString]) -> ExitCode {
    let arguments: SelectionArguments = match parse_selection(flags) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message} Run cpctx --help.");
            return ExitCode::from(2);
        }
    };

    let root: &Path = Path::new(path);
    let snapshot_result = if let Some(destination) = arguments.ssh_destination.as_deref() {
        collect_remote_project_with_selection(destination, root, &arguments.limits, |local_root| {
            if arguments
                .config_file
                .as_deref()
                .is_some_and(Path::is_absolute)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "remote profiles require a configuration path relative to the project",
                ));
            }
            resolve_selection(local_root, &arguments)
        })
    } else {
        resolve_selection(root, &arguments).and_then(|options| {
            collect_project_with_options_and_limits(root, &options, &arguments.limits)
        })
    };
    match snapshot_result {
        Ok(snapshot) => {
            if mode == "--clipboard" || mode == "-c" {
                if let Err(error) = crate::clipboard::copy_text(&snapshot.to_markdown()) {
                    eprintln!("Clipboard error: {error}");
                    return ExitCode::from(1);
                }
                eprintln!("Project context copied to clipboard.");
            } else if mode == "--tree" || mode == "-t" {
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
            "--ssh" if arguments.ssh_destination.is_none() && !value.is_empty() => {
                arguments.ssh_destination = Some(value.to_owned());
            }
            "--max-bytes" => {
                let limit: NonZeroUsize = value
                    .parse()
                    .map_err(|_error| "--max-bytes must be a positive integer.")?;
                arguments.limits.set_max_source_bytes(limit);
            }
            "--max-estimated-tokens" => {
                let limit: NonZeroUsize = value
                    .parse()
                    .map_err(|_error| "--max-estimated-tokens must be a positive integer.")?;
                arguments.limits.set_max_estimated_tokens(limit);
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
