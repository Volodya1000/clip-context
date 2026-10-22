//! Deterministic project snapshots without hidden files or filesystem side effects.
//!
//! This initial collector only reads common UTF-8 source files. Secret scanning,
//! custom exclusions, and `.gitignore` support are separate future features.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DEFAULT_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cs", "css", "go", "h", "hpp", "html", "java", "js", "json", "kt", "md", "py", "rs",
    "sh", "sql", "toml", "ts", "tsx", "txt", "yaml", "yml",
];

const EXCLUDED_DIRECTORIES: &[&str] = &[
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
    "__pycache__",
];

/// Source text together with its path relative to the project root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    relative_path: PathBuf,
    content: String,
}

impl SourceFile {
    /// The source file path relative to the requested project directory.
    #[must_use]
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    /// The original UTF-8 text, including trailing newlines.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// A reproducibly ordered, complete snapshot of matching project files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    project_name: String,
    files: Vec<SourceFile>,
}

impl Snapshot {
    /// Files in lexicographic relative-path order.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// ASCII-style directory tree containing only included files.
    #[must_use]
    pub fn tree(&self) -> String {
        let mut root = TreeNode::default();
        for file in &self.files {
            root.insert(&file.relative_path);
        }

        let mut output = format!("{}/\n", self.project_name);
        root.append_children(&mut output, "");
        output
    }

    /// Markdown with the project tree and source text for every selected file.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut output = format!("Project: {}\n\n## File tree\n\n```text\n{}```\n", self.project_name, self.tree());

        for file in &self.files {
            let path = file
                .relative_path
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let fence = "`".repeat(longest_backtick_run(&file.content).saturating_add(1).max(3));
            output.push_str(&format!("\n### File: {path}\n\n{fence}\n"));
            output.push_str(&file.content);
            if !file.content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&fence);
            output.push('\n');
        }

        output
    }
}

#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, Self>,
}

impl TreeNode {
    fn insert(&mut self, relative_path: &Path) {
        let mut node = self;
        for part in relative_path.components() {
            let name = part.as_os_str().to_string_lossy().into_owned();
            node = node.children.entry(name).or_default();
        }
    }

    fn append_children(&self, output: &mut String, indentation: &str) {
        let count = self.children.len();
        for (index, (name, child)) in self.children.iter().enumerate() {
            let is_last = index + 1 == count;
            output.push_str(indentation);
            output.push_str(if is_last { "└── " } else { "├── " });
            output.push_str(name);
            if !child.children.is_empty() {
                output.push('/');
            }
            output.push('\n');
            let next_indent = format!("{indentation}{}", if is_last { "    " } else { "│   " });
            child.append_children(output, &next_indent);
        }
    }
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default()
}

/// Read eligible project files, rejecting unreadable and non-UTF-8 source files.
///
/// Hidden paths, common generated directories and symbolic links are excluded.
/// File selection uses a fixed list of familiar source-code extensions. No
/// secret-content scanner or `.gitignore` interpreter is included in this stage.
///
/// # Errors
///
/// Fails if the path is not a directory or an included file cannot be read.
/// No partial result is returned on failure.
pub fn collect_project(root: &Path) -> io::Result<Snapshot> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a project directory: {}", root.display()),
        ));
    }

    let project_name = root
        .file_name()
        .unwrap_or_else(|| OsStr::new("project"))
        .to_string_lossy()
        .into_owned();

    let mut files = Vec::new();
    visit_directory(root, root, &mut files)?;
    files.sort_unstable_by(|left: &SourceFile, right: &SourceFile| {
        left.relative_path.cmp(&right.relative_path)
    });
    Ok(Snapshot {
        project_name,
        files,
    })
}

fn visit_directory(root: &Path, directory: &Path, files: &mut Vec<SourceFile>) -> io::Result<()> {
    for result in fs::read_dir(directory)? {
        let entry = result?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }

        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }

        if kind.is_dir() {
            if !EXCLUDED_DIRECTORIES
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(OsStr::new(excluded)))
            {
                visit_directory(root, &entry.path(), files)?;
            }
        } else if kind.is_file() && has_supported_extension(&entry.path()) {
            let path = entry.path();
            let content = fs::read_to_string(&path).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("failed to read {}: {error}", path.display()),
                )
            })?;
            let relative_path = path
                .strip_prefix(root)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
                .to_path_buf();
            files.push(SourceFile {
                relative_path,
                content,
            });
        }
    }

    Ok(())
}

fn has_supported_extension(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    DEFAULT_EXTENSIONS
        .iter()
        .any(|expected| extension.eq_ignore_ascii_case(OsStr::new(expected)))
}
