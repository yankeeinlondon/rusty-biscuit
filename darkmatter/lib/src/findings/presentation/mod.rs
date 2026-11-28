//! Render-tree presentations of findings, and the layout helpers they share.
//!
//! - [`ReferencePresentation`]: the reference report of `md validate refs`,
//!   `md graph --validate`, and `md compose`'s reference check, grouped by
//!   the document that holds each problem with a per-reason summary.
//!   `ValidationReportView` is its command-output adapter.
//! - [`FindingPresentation`]: any [`FindingReport`](super::FindingReport),
//!   each subject's findings under its heading.
//!
//! Both build one [`Document`](renderable::tree::Document) per link style and
//! fold it to the requested output; authored tokens enter the tree as text or
//! inline code, never as markup.

mod generic;
mod reference;

use std::path::{Path, PathBuf};

use biscuit_file::{PathIdentity, canonicalize_simplified, to_forward_slash_string};
use biscuit_terminal::components::prose::InlineProse;
use biscuit_terminal::components::table::{Table, TableColumn};
use biscuit_terminal::utils::layout::{Layout, WordWrap};
use renderable::layout::{Edges, Length, TargetValue, Width as LayoutWidth};
use renderable::tree::{RenderNode, SequenceJoin, TreeRenderable};

use super::Severity;

pub use generic::FindingPresentation;
pub use reference::{ReferencePresentation, ReferenceReportVisibility};

/// How far a reference table or a summary tally sits under its heading in a
/// terminal.
const INDENT: u32 = 2;

/// Which outputs a built document serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkStyle {
    /// Markdown, MarkdownPlus, and HTML: absolute paths, section fragments.
    Portable,
    /// `terminal` and `plain`: bare `file://` URIs, section as a parameter.
    Terminal,
}

impl LinkStyle {
    /// The document root. Each section's blocks sit one line apart in a
    /// terminal and sections one blank line apart; elsewhere everything is an
    /// ordinary separate block, where a heading must stay clear of a table.
    fn root(self, sections: Vec<Vec<RenderNode>>) -> RenderNode {
        match self {
            Self::Portable => RenderNode::root(sections.into_iter().flatten().collect()),
            Self::Terminal => {
                let mut children = Vec::new();
                for section in sections {
                    if !children.is_empty() {
                        children.push(RenderNode::text("\n\n"));
                    }
                    for (index, block) in section.into_iter().enumerate() {
                        if index > 0 {
                            children.push(RenderNode::text("\n"));
                        }
                        children.push(block);
                    }
                }
                let mut root = RenderNode::root(children);
                root.attrs.set_sequence_join(SequenceJoin::None);
                root
            }
        }
    }
}

/// How the paths a presentation names are shown and linked, from the launch
/// directory its caller captured; the process's current directory is never
/// consulted.
#[derive(Debug, Clone, Default)]
struct PathDisplay {
    launch_dir: Option<PathBuf>,
}

impl PathDisplay {
    fn absolute_path(&self, path: &Path) -> PathBuf {
        match &self.launch_dir {
            Some(launch_dir) if !path.is_absolute() => launch_dir.join(path),
            _ => path.to_path_buf(),
        }
    }

    /// `path` relative to the launch directory, `/`-separated on every OS; its
    /// absolute form without a launch directory or a route to it.
    ///
    /// A path outside the launch directory by spelling alone (`/var` for
    /// `/private/var`) is routed again through both canonical forms, so an
    /// alias does not turn a file beside the reader into a climb to the root.
    fn relative_display(&self, path: &Path) -> String {
        let absolute = self.absolute_path(path);
        let Some(launch_dir) = self.launch_dir.as_deref() else {
            return to_forward_slash_string(&absolute);
        };
        let route_between = |file: &Path, dir: &Path| PathIdentity::new(file).relative_from(&PathIdentity::new(dir));
        let canonical = |path: &Path| canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
        let route = route_between(&absolute, launch_dir)
            .filter(|route| route.parent_hops() == 0)
            .or_else(|| route_between(&canonical(&absolute), &canonical(launch_dir)));
        match route {
            Some(route) => to_forward_slash_string(&route.to_path_buf()),
            None => to_forward_slash_string(&absolute),
        }
    }

    /// The absolute path as a link destination whose `#` and `?` still belong
    /// to the path. Nothing else is rewritten.
    fn portable_destination(&self, path: &Path) -> String {
        to_forward_slash_string(&self.absolute_path(path))
            .replace('#', "%23")
            .replace('?', "%3F")
    }

    /// The path's `file://` URI, or its portable text when no URI can name it.
    fn file_uri(&self, path: &Path) -> String {
        let absolute = self.absolute_path(path);
        url::Url::from_file_path(&absolute).map_or_else(|()| to_forward_slash_string(&absolute), String::from)
    }

    fn path_link(&self, path: &Path, links: LinkStyle, children: Vec<RenderNode>) -> RenderNode {
        let destination = match links {
            LinkStyle::Portable => self.portable_destination(path),
            LinkStyle::Terminal => self.file_uri(path),
        };
        RenderNode::link(destination, None, children)
    }
}

/// The Prose markup of a finding's severity glyph, shared by every
/// presentation and the string-built human report.
pub(super) fn glyph_markup(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "<red>✗</red>",
        Severity::Warning => "<yellow>⚠</yellow>",
        Severity::Info => "<blue>ℹ</blue>",
    }
}

/// A borderless three-column table of `rows`, whose empty header a terminal
/// omits and a Markdown table requires.
fn rows_table(rows: Vec<RenderNode>, links: LinkStyle) -> RenderNode {
    let mut table = Table::new()
        // A reference and a location are copied as written, so a width that
        // cannot hold them stacks the row rather than split them. A hint in
        // the first column breaks only between words, keeping `::file-links`
        // whole.
        .with_columns(vec![
            TableColumn::new("").with_word_wrap(WordWrap::BespokeProse(None, vec![' '], None)),
            TableColumn::new("").with_word_wrap(WordWrap::None),
            TableColumn::new("").with_word_wrap(WordWrap::None),
        ])
        .borderless()
        .render_tree();
    if links == LinkStyle::Terminal {
        table.attrs.set_layout(&indented(INDENT));
    }
    if let Some(children) = table.children_mut() {
        children.extend(rows);
    }
    table
}

/// A fit-content block `columns` character cells in from the left.
fn indented(columns: u32) -> Layout {
    Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(columns)),
            ..Edges::default()
        },
        width: LayoutWidth::FitContent,
        ..Layout::default()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| to_forward_slash_string(path), |name| name.to_string_lossy().into_owned())
}

/// Phrasing nodes for Prose `text`, whose styling tags take effect.
fn markup(text: &str) -> Vec<RenderNode> {
    InlineProse::new(text).to_render_nodes()
}

/// Phrasing nodes for the literal `text`, dimmed.
fn dim(text: &str) -> Vec<RenderNode> {
    markup(&format!("<dim>{}</dim>", InlineProse::escape_text(text)))
}

fn noun(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
