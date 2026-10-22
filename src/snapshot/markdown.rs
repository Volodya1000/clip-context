//! Markdown snapshot rendering with content-safe code fences.

use super::Snapshot;

impl Snapshot {
    /// Markdown with the project tree and source text for every selected file.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "Project: {}\n\n## File tree\n\n```text\n{}```\n",
            self.project_name,
            self.tree()
        );

        for file in &self.files {
            let path = file
                .relative_path
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let fence = "`".repeat(longest_backtick_run(&file.content).saturating_add(1).max(3));
            output.push_str("\n### File: ");
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
