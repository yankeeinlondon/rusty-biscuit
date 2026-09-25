#![allow(dead_code)]
//! Render a list of repository-relative paths as a hierarchical tree.
//!
//! Output uses the same box-drawing characters as the biscuit-terminal
//! filesystem component (`├── `, `└── `, `│   `, four-space indent) and emits
//! Prose-flavored markup so the caller can colorize via `Prose::new(...).render(...)`.
//!
//! Files are colored by kind with the `wt list` dirty-dot palette, split by
//! `sniff::filesystem::path_kind::is_source_code_path`. Callers cap how many
//! paths they pass; `wt remove` shows a count instead of a tree above its
//! `LIST_LIMIT`.
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
pub fn render_markup(paths: &[PathBuf]) -> String {
    let mut root = Node::default();
    for path in paths {
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
    let name = biscuit_terminal::components::prose::Prose::escape_text(name);
    if !is_file {
        return format!("<dim>{name}/</dim>");
    }
    if sniff::filesystem::path_kind::is_source_code_path(full_path) {
        format!("<orange>{name}</orange>")
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
        assert!(out.contains("<orange>lib.rs</orange>"));
        assert!(out.contains("<orange>main.rs</orange>"));
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
    fn every_path_is_rendered_and_markup_in_names_is_escaped() {
        let paths: Vec<PathBuf> = (0..60)
            .map(|i| PathBuf::from(format!("file-{i:03}.txt")))
            .collect();
        let out = render_markup(&paths);
        assert_eq!(out.lines().count(), 60);
        assert!(!out.contains("more file"));

        let out = render_markup(&[PathBuf::from("<b>odd.md")]);
        assert!(out.contains(r"<yellow>\<b\>odd.md</yellow>"), "{out}");
    }
}
