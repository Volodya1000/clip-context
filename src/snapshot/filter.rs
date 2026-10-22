//! Built-in source selection rules.
//!
//! Explicit path globs augment the selection model without overriding ignores.

use std::ffi::OsStr;
use std::io;
use std::path::Path;

use ignore::overrides::{Override, OverrideBuilder};

/// Additional file-selection rules applied on top of the safe built-in defaults.
///
/// With no explicit extensions, the default source extension list applies.
/// Once an extension is specified, only explicitly listed extensions qualify.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionOptions {
    extensions: Vec<String>,
    excluded_directories: Vec<String>,
    includes: Vec<String>,
    excludes: Vec<String>,
}

impl SelectionOptions {
    /// Restrict selected files to the given extension, without a leading dot.
    pub fn include_extension(&mut self, extension: &str) {
        self.extensions.push(extension.to_owned());
    }

    /// Replace an existing extension selection with a new set of CLI options.
    pub fn clear_extensions(&mut self) {
        self.extensions.clear();
    }

    /// Return whether an extension is safe to use in a selection rule.
    #[must_use]
    pub fn valid_extension(extension: &str) -> bool {
        !extension.is_empty()
            && extension
                .bytes()
                .all(|character| character.is_ascii_alphanumeric())
    }

    /// Return whether a directory exclusion contains only a single name.
    #[must_use]
    pub fn valid_directory_name(name: &str) -> bool {
        !name.is_empty()
            && name != "."
            && name != ".."
            && !name.contains('/')
            && !name.contains('\\')
    }

    /// Exclude a directory by name, at any depth within the project root.
    pub fn exclude_directory(&mut self, name: &str) {
        self.excluded_directories.push(name.to_owned());
    }

    /// Select paths by a project-relative glob instead of default extensions.
    /// Explicit extensions, if present, further restrict the matching files.
    ///
    /// # Errors
    /// Returns an error for invalid or unsafe glob syntax.
    pub fn include_pattern(&mut self, pattern: &str) -> io::Result<()> {
        validate_pattern(pattern)?;
        self.includes.push(pattern.to_owned());
        Ok(())
    }

    /// Exclude project-relative paths matching a glob; exclusions always win.
    ///
    /// # Errors
    /// Returns an error for invalid or unsafe glob syntax.
    pub fn exclude_pattern(&mut self, pattern: &str) -> io::Result<()> {
        validate_pattern(pattern)?;
        self.excludes.push(pattern.to_owned());
        Ok(())
    }
}

fn validate_pattern(pattern: &str) -> io::Result<()> {
    if pattern.is_empty()
        || pattern.starts_with(['!', '#', '/'])
        || pattern.contains(['\\', ':', '\n', '\r', '\0'])
        || pattern.split('/').any(|part| part == "..")
        || pattern.trim() != pattern
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "glob must be a nonempty project-relative pattern without '..', negation or control characters",
        ));
    }
    OverrideBuilder::new(".")
        .add(pattern)
        .map_err(io::Error::other)?;
    Ok(())
}

pub(super) struct SelectionRules {
    patterns: Override,
    options: SelectionOptions,
}

impl SelectionRules {
    pub(super) fn new(root: &Path, options: &SelectionOptions) -> io::Result<Self> {
        let mut builder = OverrideBuilder::new(root);
        for pattern in &options.includes {
            builder.add(pattern).map_err(io::Error::other)?;
        }
        // Append exclusions last so an include can never undo an exclusion.
        for pattern in &options.excludes {
            builder
                .add(&format!("!{pattern}"))
                .map_err(io::Error::other)?;
        }
        Ok(Self {
            patterns: builder.build().map_err(io::Error::other)?,
            options: options.clone(),
        })
    }

    pub(super) fn exclusion(
        &self,
        path: &Path,
        relative: &Path,
        is_dir: bool,
    ) -> Option<&'static str> {
        let name = path.file_name()?;
        let name_text = name.to_string_lossy();
        if [
            ".git",
            ".ssh",
            ".aws",
            ".kube",
            ".env",
            "id_rsa",
            "id_dsa",
            "id_ecdsa",
            "id_ed25519",
        ]
        .iter()
        .any(|blocked| name.eq_ignore_ascii_case(OsStr::new(blocked)))
            || name_text.to_ascii_lowercase().starts_with(".env.")
        {
            return Some("protected credential or VCS path");
        }
        if is_dir && !includes_directory(name, &self.options) {
            return Some("excluded directory");
        }
        if is_hidden(name) && !self.explicit_hidden_path(relative, is_dir) {
            return Some("hidden path (requires an explicit include prefix)");
        }
        if self.patterns.matched(path, is_dir).is_ignore() {
            return Some("path glob selection");
        }
        if !is_dir
            && (self.options.includes.is_empty() || !self.options.extensions.is_empty())
            && !has_supported_extension(path, &self.options)
        {
            return Some("extension selection");
        }
        None
    }

    fn explicit_hidden_path(&self, relative: &Path, is_dir: bool) -> bool {
        let relative = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        self.options.includes.iter().any(|pattern| {
            pattern == &relative || (is_dir && pattern.starts_with(&format!("{relative}/")))
        })
    }
}

const DEFAULT_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cs", "css", "go", "h", "hpp", "html", "java", "js", "json", "kt", "md", "py",
    "rs", "sh", "sql", "toml", "ts", "tsx", "txt", "yaml", "yml",
];

/// Known generated directory names excluded by the local collector.
///
/// Remote SSH prefiltering may omit these to reduce tar archive size.
pub const BUILTIN_EXCLUDED_DIRECTORIES: &[&str] = &[
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
    "__pycache__",
    ".venv",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
];

pub(super) fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

pub(super) fn includes_directory(name: &OsStr, options: &SelectionOptions) -> bool {
    !BUILTIN_EXCLUDED_DIRECTORIES
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
        assert!(has_supported_extension(
            Path::new("src/module.py"),
            &options
        ));
        assert!(!has_supported_extension(Path::new("src/module"), &options));
        assert!(!has_supported_extension(
            Path::new("assets/icon.png"),
            &options
        ));
    }
}
