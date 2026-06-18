#![allow(dead_code)]
//! Render a list of repository-relative paths as a hierarchical tree.
//!
//! Output uses the same box-drawing characters as the biscuit-terminal
//! filesystem component (`├── `, `└── `, `│   `, four-space indent) and emits
//! Prose-flavored markup so the caller can colorize via `Prose::new(...).render(...)`.
//!
//! Source-code files (per `sniff::filesystem::path_kind::is_source_code_path`)
//! are wrapped in `<red>...</red>`; other files in `<yellow>...</yellow>`;
//! directories in `<dim>...</dim>`.
//!
//! ## Why a custom implementation?
//!
//! `darkmatter::markdown::reference::file_tree::FileTree` is purpose-built for
//! Markdown reference graphs: it walks real files on disk, renders
//! transclusion edges, validation overlays, and toc-linking indicators. The
//! `wt remove` command needs the opposite: render an arbitrary `Vec<PathBuf>`
//! (git status output) as a static tree with per-file source-code
//! color-coding. Rather than fighting FileTree's Markdown-centric model,
//! this small module produces exactly the markup we need with no unused
//! features.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::filesystem::tree_chars;

/// Maximum number of files to render in the dirty tree before truncating.
///
/// A worktree that has been wiped or partially cleaned up can report hundreds
/// of dirty paths (e.g. via `D` entries). Dumping the full tree floods the
/// terminal and pushes the confirmation prompt off-screen, so we cap the
/// rendered file count and emit a single overflow line summarising the rest.
pub const MAX_DISPLAYED_FILES: usize = 50;

#[derive(Debug, Default)]
struct Node {
    children: BTreeMap<String, Node>,
    is_file: bool,
}

impl Node {
    fn insert(&mut self, parts: &[String]) {
        let Some((head, tail)) = parts.split_first() else {
            self.is_file = true;
            return;
        };
        self.children.entry(head.clone()).or_default().insert(tail);
    }
}

/// Build the markup for a tree-rendering of `paths`, all interpreted as
/// repository-relative entries.
///
/// When `paths.len()` exceeds [`MAX_DISPLAYED_FILES`], only the leading slice
/// is rendered into the tree and a trailing `...and {N} more files` line is
/// appended so the caller's confirmation prompt stays on-screen.
pub fn render_markup(paths: &[PathBuf]) -> String {
    let total = paths.len();
    let (displayed, overflow) = if total > MAX_DISPLAYED_FILES {
        (&paths[..MAX_DISPLAYED_FILES], total - MAX_DISPLAYED_FILES)
    } else {
        (paths, 0)
    };

    let mut root = Node::default();
    for path in displayed {
        let parts: Vec<String> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            continue;
        }
        root.insert(&parts);
    }

    let mut out = String::new();
    render_children(&root, &mut out, "", &PathBuf::new());
    if overflow > 0 {
        out.push_str(&format!("<dim>...and {overflow} more file(s)</dim>\n"));
    }
    out
}

fn render_children(node: &Node, out: &mut String, prefix: &str, base: &Path) {
    let total = node.children.len();
    for (idx, (name, child)) in node.children.iter().enumerate() {
        let is_last = idx + 1 == total;
        let connector = if is_last {
            tree_chars::LAST_BRANCH
        } else {
            tree_chars::BRANCH
        };
        let child_path = base.join(name);
        let label = format_label(name, child.is_file && child.children.is_empty(), &child_path);
        out.push_str(prefix);
        out.push_str(connector);
        out.push_str(&label);
        out.push('\n');

        let next_prefix = format!(
            "{}{}",
            prefix,
            if is_last { tree_chars::INDENT } else { tree_chars::VERTICAL }
        );
        render_children(child, out, &next_prefix, &child_path);
    }
}

fn format_label(name: &str, is_file: bool, full_path: &Path) -> String {
    if !is_file {
        return format!("<dim>{name}/</dim>");
    }
    if sniff::filesystem::path_kind::is_source_code_path(full_path) {
        format!("<red>{name}</red>")
    } else {
        format!("<yellow>{name}</yellow>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_paths_render_empty() {
        assert_eq!(render_markup(&[]), "");
    }

    #[test]
    fn single_file_at_root() {
        let out = render_markup(&[PathBuf::from("README.md")]);
        assert_eq!(out, "└── <yellow>README.md</yellow>\n");
    }

    #[test]
    fn nested_files_group_by_directory() {
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("docs/intro.md"),
        ];
        let out = render_markup(&paths);
        // docs/ comes before src/ (BTreeMap is alphabetical)
        assert!(out.contains("<dim>docs/</dim>"));
        assert!(out.contains("<dim>src/</dim>"));
        assert!(out.contains("<red>lib.rs</red>"));
        assert!(out.contains("<red>main.rs</red>"));
        assert!(out.contains("<yellow>intro.md</yellow>"));
        // The first directory at root uses BRANCH; the last uses LAST_BRANCH.
        assert!(out.starts_with("├── <dim>docs/</dim>\n"));
        assert!(out.contains("└── <dim>src/</dim>\n"));
    }

    #[test]
    fn deep_nesting_uses_vertical_continuation() {
        let paths = vec![
            PathBuf::from("a/b/c.rs"),
            PathBuf::from("a/d.rs"),
        ];
        let out = render_markup(&paths);
        // Expect a vertical continuation under `a/` for the non-last child line.
        assert!(out.contains("│   "));
    }

    #[test]
    fn at_threshold_does_not_truncate() {
        let paths: Vec<PathBuf> = (0..MAX_DISPLAYED_FILES)
            .map(|i| PathBuf::from(format!("file-{i:03}.txt")))
            .collect();
        let out = render_markup(&paths);
        assert!(
            !out.contains("more file"),
            "should not emit overflow footer at threshold:\n{out}"
        );
        assert!(out.contains("file-000.txt"));
        assert!(out.contains(&format!("file-{:03}.txt", MAX_DISPLAYED_FILES - 1)));
    }

    #[test]
    fn over_threshold_truncates_and_summarises() {
        let extra = 25;
        let total = MAX_DISPLAYED_FILES + extra;
        let paths: Vec<PathBuf> = (0..total)
            .map(|i| PathBuf::from(format!("file-{i:03}.txt")))
            .collect();
        let out = render_markup(&paths);
        assert!(
            out.contains(&format!("<dim>...and {extra} more file(s)</dim>")),
            "expected overflow footer for {extra} extras:\n{out}"
        );
        // First displayed file appears; first truncated file does not.
        assert!(out.contains("file-000.txt"));
        let first_truncated = format!("file-{:03}.txt", MAX_DISPLAYED_FILES);
        assert!(
            !out.contains(&first_truncated),
            "expected file at truncation boundary to be omitted: {first_truncated}\n{out}"
        );
    }

    #[test]
    fn overflow_footer_is_last_line() {
        let total = MAX_DISPLAYED_FILES + 1;
        let paths: Vec<PathBuf> = (0..total)
            .map(|i| PathBuf::from(format!("file-{i:03}.txt")))
            .collect();
        let out = render_markup(&paths);
        let lines: Vec<&str> = out.lines().collect();
        let last = lines.last().expect("at least one line");
        assert!(
            last.starts_with("<dim>...and "),
            "overflow footer must be the final line, got: {last:?}\nfull:\n{out}"
        );
    }
}
