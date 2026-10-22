//! Deterministic directory tree formatting for selected files.

use std::collections::BTreeMap;
use std::path::Path;

use super::Snapshot;

impl Snapshot {
    /// Box-drawing directory tree containing only included files.
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
