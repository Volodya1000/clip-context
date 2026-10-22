//! Built-in source selection rules.
//!
//! The fixed extension and directory lists are only the first-stage defaults;
//! custom inclusion and exclusion rules will be introduced in later PRs.

use std::ffi::OsStr;
use std::path::Path;

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

pub(super) fn includes_directory(name: &OsStr) -> bool {
    !EXCLUDED_DIRECTORIES
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(OsStr::new(excluded)))
}

pub(super) fn has_supported_extension(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    DEFAULT_EXTENSIONS
        .iter()
        .any(|expected| extension.eq_ignore_ascii_case(OsStr::new(expected)))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::{has_supported_extension, includes_directory, is_hidden};

    #[test]
    fn hidden_names_are_excluded_by_name_not_underscore() {
        assert!(is_hidden(OsStr::new(".env")));
        assert!(is_hidden(OsStr::new(".git")));
        assert!(!is_hidden(OsStr::new("_domain")));
        assert!(!is_hidden(OsStr::new("source")));
    }

    #[test]
    fn generated_directories_are_skipped_case_insensitively() {
        for directory in [
            "build",
            "target",
            "TARGET",
            "node_modules",
            "__pycache__",
            "venv",
            "dist",
        ] {
            assert!(!includes_directory(OsStr::new(directory)));
        }
        assert!(includes_directory(OsStr::new("src")));
        assert!(includes_directory(OsStr::new("_domain")));
    }

    #[test]
    fn supported_extensions_are_case_insensitive() {
        assert!(has_supported_extension(Path::new("src/main.RS")));
        assert!(has_supported_extension(Path::new("src/module.py")));
        assert!(!has_supported_extension(Path::new("src/module")));
        assert!(!has_supported_extension(Path::new("assets/icon.png")));
    }
}
