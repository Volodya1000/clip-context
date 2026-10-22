//! Markdown snapshot rendering with content-safe code fences.

use super::Snapshot;

/// How selected source files are presented in Markdown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContextFormat {
    /// Existing detailed format with a directory tree and file headers.
    #[default]
    Full,
    /// Token-lean format without the repeated directory tree.
    Compact,
}

impl Snapshot {
    /// Markdown with the project tree and source text for every selected file.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        self.to_markdown_with_format(ContextFormat::Full)
    }

    /// Render the complete source snapshot in the requested Markdown format.
    ///
    /// Compact output omits the redundant directory tree and shortens per-file
    /// headers. Both formats retain every selected file and its original text.
    #[must_use]
    pub fn to_markdown_with_format(&self, format: ContextFormat) -> String {
        let mut output = match format {
            ContextFormat::Full => format!(
                "Project: {}\n\n## File tree\n\n```text\n{}```\n",
                self.project_name,
                self.tree()
            ),
            ContextFormat::Compact => format!("Project: {}\n", self.project_name),
        };

        for file in &self.files {
            let path = file
                .relative_path
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let fence = "`".repeat(longest_backtick_run(&file.content).saturating_add(1).max(3));
            match format {
                ContextFormat::Full => output.push_str("\n### File: "),
                ContextFormat::Compact => output.push_str("\n## "),
            }
            output.push_str(&path);
            output.push_str("\n\n");
            output.push_str(&fence);
            output.push('\n');
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

fn longest_backtick_run(text: &str) -> usize {
    text.split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::longest_backtick_run;

    #[test]
    fn counts_longest_backtick_sequence() {
        assert_eq!(longest_backtick_run("before ``` and ```` after"), 4);
        assert_eq!(longest_backtick_run("no backtick"), 0);
    }

    #[test]
    fn a_single_backtick_is_a_run() {
        assert_eq!(longest_backtick_run("x`y"), 1);
    }
}
