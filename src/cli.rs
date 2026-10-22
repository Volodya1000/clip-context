//! Parsing CLI file-selection flags and resolving their precedence.

use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clip_context::profile::{DEFAULT_CONFIG_FILE, load_profile};
use clip_context::remote::{
    collect_remote_project_with_selection, collect_remote_tree_with_selection,
};
use clip_context::snapshot::{
    ContextFormat, SelectionOptions, SnapshotLimits, collect_project_tree,
    collect_project_with_options_and_limits, explain_project,
};

#[derive(Default)]
struct SelectionArguments {
    profile: Option<String>,
    config_file: Option<PathBuf>,
    extensions: Vec<String>,
    excluded_directories: Vec<String>,
    includes: Vec<String>,
    excludes: Vec<String>,
    limits: SnapshotLimits,
    ssh_destination: Option<String>,
    format: ContextFormat,
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
    if mode == "--explain" || mode == "--dry-run" {
        return run_explanation(root, &arguments);
    }
    let tree_only = mode == "--tree" || mode == "-t";
    let snapshot_result = if let Some(destination) = arguments.ssh_destination.as_deref() {
        let resolve_remote = |local_root: &Path| {
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
        };
        if tree_only {
            collect_remote_tree_with_selection(destination, root, &arguments.limits, resolve_remote)
        } else {
            collect_remote_project_with_selection(
                destination,
                root,
                &arguments.limits,
                resolve_remote,
            )
        }
    } else {
        resolve_selection(root, &arguments).and_then(|options| {
            if tree_only {
                collect_project_tree(root, &options)
            } else {
                collect_project_with_options_and_limits(root, &options, &arguments.limits)
            }
        })
    };
    match snapshot_result {
        Ok(snapshot) => {
            let rendered = if mode == "--tree" || mode == "-t" {
                snapshot.tree()
            } else {
                snapshot.to_markdown_with_format(arguments.format)
            };
            if let Err(error) = arguments.limits.validate_rendered(&rendered) {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
            if mode == "--clipboard" || mode == "-c" {
                if let Err(error) = crate::clipboard::copy_text(&rendered) {
                    eprintln!("Clipboard error: {error}");
                    return ExitCode::from(1);
                }
                eprintln!("Project context copied to clipboard.");
            } else {
                let mut stdout = io::stdout().lock();
                if let Err(error) = stdout
                    .write_all(rendered.as_bytes())
                    .and_then(|()| stdout.flush())
                {
                    eprintln!("Output error: {error}");
                    return ExitCode::from(1);
                }
            }
            eprintln!("{}", crate::stats::OutputStats::from_utf8(&rendered));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run_explanation(root: &Path, arguments: &SelectionArguments) -> ExitCode {
    if arguments.ssh_destination.is_some() {
        eprintln!(
            "--explain/--dry-run currently require a local project; SSH would transfer source contents."
        );
        return ExitCode::from(2);
    }
    let result = resolve_selection(root, arguments)
        .and_then(|options| explain_project(root, &options, &arguments.limits))
        .and_then(|report| io::stdout().lock().write_all(report.as_bytes()));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn parse_selection(flags: &[OsString]) -> Result<SelectionArguments, String> {
    let mut arguments = SelectionArguments::default();
    let mut pairs = flags.chunks_exact(2);

    for pair in &mut pairs {
        let flag: &str = pair[0].to_str().ok_or("Option names must be UTF-8.")?;
        let value: &str = pair[1].to_str().ok_or("Option values must be UTF-8.")?;

        match flag {
            "--include" | "--exclude" => {
                let mut validation = SelectionOptions::default();
                validation
                    .include_pattern(value)
                    .map_err(|error| error.to_string())?;
                if flag == "--include" {
                    arguments.includes.push(value.to_owned());
                } else {
                    arguments.excludes.push(value.to_owned());
                }
            }
            "--ext" => {
                if !SelectionOptions::valid_extension(value) {
                    return Err(
                        "--ext requires letters and digits only, without a leading dot.".into(),
                    );
                }
                arguments.extensions.push(value.to_owned());
            }
            "--exclude-dir" => {
                if !SelectionOptions::valid_directory_name(value) {
                    return Err(
                        "--exclude-dir requires a single directory name, not a path.".into(),
                    );
                }
                arguments.excluded_directories.push(value.to_owned());
            }
            "--profile" => {
                if arguments.profile.is_some() {
                    return Err("--profile may be specified only once.".into());
                }
                if value.is_empty()
                    || !value.bytes().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_')
                    })
                {
                    return Err(
                        "--profile requires a name using letters, digits, '-' or '_'.".into(),
                    );
                }
                arguments.profile = Some(value.to_owned());
            }
            "--config" => {
                if arguments.config_file.is_some() {
                    return Err("--config may be specified only once.".into());
                }
                if value.is_empty() {
                    return Err("--config requires a nonempty file path.".into());
                }
                arguments.config_file = Some(PathBuf::from(value));
            }
            "--ssh" => {
                if arguments.ssh_destination.is_some() {
                    return Err("--ssh may be specified only once.".into());
                }
                if value.is_empty() {
                    return Err("--ssh requires an SSH host or user@host.".into());
                }
                arguments.ssh_destination = Some(value.to_owned());
            }
            "--format" => {
                arguments.format = match value {
                    "full" => ContextFormat::Full,
                    "compact" => ContextFormat::Compact,
                    _ => return Err("--format must be full or compact.".into()),
                };
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
            _ => return Err(format!("Unknown option {flag:?}.")),
        }
    }

    if let Some(missing_value) = pairs.remainder().first() {
        return Err(format!(
            "Missing value for {}.",
            missing_value.to_string_lossy()
        ));
    }
    if arguments.config_file.is_some() && arguments.profile.is_none() {
        return Err("--config requires --profile.".into());
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
    for pattern in &args.includes {
        options.include_pattern(pattern)?;
    }
    for pattern in &args.excludes {
        options.exclude_pattern(pattern)?;
    }

    Ok(options)
}
