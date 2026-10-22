//! Built-in source selection rules.
//!
//! The fixed extension and directory lists are only the first-stage defaults;
//! custom inclusion and exclusion rules will be introduced in later PRs.

use std::ffi::OsStr;
use std::path::Path;

/// Additional file-selection rules applied on top of the safe built-in defaults.
///
/// With no explicit extensions, the default source extension list applies.
/// Once an extension is specified, only explicitly listed extensions qualify.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionOptions {
    extensions: Vec<String>,
    excluded_directories: Vec<String>,
}

impl SelectionOptions {
    /// Restrict selected files to the given extension, without a leading dot.
    pub fn include_extension(&mut self, extension: &str) {
        self.extensions.push(extension.to_owned());
    }

    /// Exclude a directory by name, at any depth within the project root.
    pub fn exclude_directory(&mut self, name: &str) {
        self.excluded_directories.push(name.to_owned());
    }
}

const DEFAULT_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cs", "css", "go", "h", "hpp", "html", "java", "js", "json", "kt", "md", "py",
    "rs", "sh", "sql", "toml", "ts", "tsx", "txt", "yaml", "yml",
];

const EXCLUDED_DIRECTORIES: &[&str] = &[
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
    "__pycache__",
];

pub(super) fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

pub(super) fn includes_directory(name: &OsStr, options: &SelectionOptions) -> bool {
    !EXCLUDED_DIRECTORIES
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(OsStr::new(excluded)))
        && !options
            .excluded_directories
            .iter()
            .any(|excluded| name.eq_ignore_ascii_case(OsStr::new(excluded)))
}

pub(super) fn has_supported_extension(path: &Path, options: &SelectionOptions) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    if options.extensions.is_empty() {
        DEFAULT_EXTENSIONS
            .iter()
            .any(|expected| extension.eq_ignore_ascii_case(OsStr::new(expected)))
    } else {
        options
            .extensions
            .iter()
            .any(|expected| extension.eq_ignore_ascii_case(OsStr::new(expected)))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::{SelectionOptions, has_supported_extension, includes_directory, is_hidden};

    #[test]
    fn hidden_names_are_excluded_by_name_not_underscore() {
        assert!(is_hidden(OsStr::new(".env")));
        assert!(is_hidden(OsStr::new(".git")));
        assert!(!is_hidden(OsStr::new("_domain")));
        assert!(!is_hidden(OsStr::new("source")));
    }

    #[test]
    fn extra_directory_exclusion_preserves_default_safety() {
        let mut options = SelectionOptions::default();
        options.exclude_directory("generated");
        assert!(!includes_directory(OsStr::new("generated"), &options));
        assert!(!includes_directory(OsStr::new("GENERATED"), &options));
        assert!(!includes_directory(OsStr::new("target"), &options));
        assert!(includes_directory(OsStr::new("src"), &options));
    }

    #[test]
    fn specified_extensions_replace_the_default_allowlist() {
        let mut options = SelectionOptions::default();
        options.include_extension("rs");
        options.include_extension("MD");
        assert!(has_supported_extension(Path::new("main.rs"), &options));
        assert!(has_supported_extension(Path::new("README.md"), &options));
        assert!(!has_supported_extension(Path::new("script.py"), &options));
        assert!(!has_supported_extension(
            Path::new("no_extension"),
            &options
        ));
    }

    #[test]
    fn generated_directories_are_skipped_case_insensitively() {
        let options = SelectionOptions::default();
        for directory in [
            "build",
            "target",
            "TARGET",
            "node_modules",
            "__pycache__",
            "venv",
            "dist",
        ] {
            assert!(!includes_directory(OsStr::new(directory), &options));
        }
        assert!(includes_directory(OsStr::new("src"), &options));
        assert!(includes_directory(OsStr::new("_domain"), &options));
    }

    #[test]
    fn supported_extensions_are_case_insensitive() {
        let options = SelectionOptions::default();
        assert!(has_supported_extension(Path::new("src/main.RS"), &options));
        assert!(has_supported_extension(Path::new("src/module.py"), &options));
        assert!(!has_supported_extension(Path::new("src/module"), &options));
        assert!(!has_supported_extension(Path::new("assets/icon.png"), &options));
    }
}
