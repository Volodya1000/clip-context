//! Strict, typed TOML configuration of named file-selection profiles.
//!
//! Profiles only narrow file selection. Built-in exclusions and `.gitignore`
//! rules remain active when a named profile is selected.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::snapshot::SelectionOptions;

/// The default configuration filename, relative to the project directory.
pub const DEFAULT_CONFIG_FILE: &str = ".cpctx.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    exclude_directories: Vec<String>,
}

/// Read a named profile from a TOML document into the shared selection model.
///
/// # Errors
///
/// Returns an error for an unreadable file, invalid TOML, unsupported keys,
/// unknown profile names, or unsafe directory and extension values.
pub fn load_profile(config_file: &Path, profile_name: &str) -> io::Result<SelectionOptions> {
    let contents = fs::read_to_string(config_file).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "cannot read profile file {}: {error}",
                config_file.display()
            ),
        )
    })?;

    let configuration: Configuration = toml::from_str(&contents).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid profile file {}: {error}", config_file.display()),
        )
    })?;

    let profile = configuration.profiles.get(profile_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown profile {profile_name:?} in {}",
                config_file.display()
            ),
        )
    })?;

    let mut options = SelectionOptions::default();
    for extension in &profile.extensions {
        if !SelectionOptions::valid_extension(extension) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid extension {extension:?} in profile {profile_name:?}"),
            ));
        }
        options.include_extension(extension);
    }
    for name in &profile.exclude_directories {
        if !SelectionOptions::valid_directory_name(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid directory exclusion {name:?} in profile {profile_name:?}"),
            ));
        }
        options.exclude_directory(name);
    }
    Ok(options)
}
