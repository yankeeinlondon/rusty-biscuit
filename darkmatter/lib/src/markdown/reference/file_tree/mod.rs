//! FileTree component for visualizing a Markdown file's dependency graph.
//!
//! Shows non-transclusion references above the file line and transclusions
//! below, with optional recursive expansion and validation overlays.
//!
//! ## Examples
//!
//! ```no_run
//! use darkmatter::markdown::reference::file_tree::FileTree;
//! use biscuit_terminal::components::renderable::TerminalRenderable;
//! use biscuit_terminal::terminal::Terminal;
//!
//! let mut tree = FileTree::new("doc.md").unwrap();
//! tree.ensure_built().unwrap();
//! let term = Terminal::default();
//! println!("{}", tree.render(&term));
//! ```

pub mod icons;
pub mod model;
mod render;

use std::any::Any;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, LayoutTerminalExt};

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeContext, ComposeOptions};
use crate::markdown::reference::ReferenceError;
use crate::markdown::reference::types::{ReferenceGraph, ReferenceGraphOptions};
use crate::markdown::reference::validate::{
    self, ReferenceValidationOptions, ReferenceValidationReport,
};
use crate::markdown::types::MarkdownError;

pub use model::{
    FileTreeIconKind, FileTreeInlineSummary, FileTreeModel, FileTreeNode, FileTreeNodeValidation,
    FileTreeReferenceGroup, FileTreeReferenceGroupKind, FileTreeReferenceRow,
    FileTreeReferenceValidation, FileTreeTransclusionEdge, FileTreeTransclusionKind,
};

/// Error type for FileTree operations.
#[derive(Debug, thiserror::Error)]
pub enum FileTreeError {
    #[error("path not found: {}", .0.display())]
    PathNotFound(PathBuf),

    #[error("not a file: {}", .0.display())]
    NotAFile(PathBuf),

    #[error("failed to load markdown: {0}")]
    Markdown(#[from] MarkdownError),

    #[error("failed to analyze references: {0}")]
    Reference(#[from] ReferenceError),
}

impl biscuit_terminal::errors::BlockError for FileTreeError {
    fn status_block(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            FileTreeError::PathNotFound(path) => {
                let absolute = path
                    .canonicalize()
                    .ok()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("FileTreeError", "path not found"))
                    .body(format!(
                        "<dim>Path:</dim> <cyan>{}</cyan>\n<dim>Resolved:</dim> {absolute}",
                        path.display()
                    ))
                    .hint("Invoke with a path that exists, e.g. <cyan>md graph ./docs/root.md</cyan>.")
            }

            FileTreeError::NotAFile(path) => {
                let absolute = path
                    .canonicalize()
                    .ok()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("FileTreeError", "not a file"))
                    .body(format!(
                        "<dim>Path:</dim> <cyan>{}</cyan>\n<dim>Resolved:</dim> {absolute}",
                        path.display()
                    ))
                    .hint("Pass a regular file (markdown document) rather than a directory or symlink target.")
            }

            FileTreeError::Markdown(inner) => inner.status_block(term),
            FileTreeError::Reference(inner) => inner.status_block(term),
        }
    }

    fn block_source(&self) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        None
    }
}

/// A terminal component that renders a Markdown file's dependency graph.
///
/// Shows references above the file line and transclusions below,
/// with optional recursive expansion and validation overlays.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::reference::file_tree::FileTree;
///
/// let tree = FileTree::new("doc.md")
///     .unwrap()
///     .follow_transclusions()
///     .validate();
/// ```
#[derive(Clone)]
pub struct FileTree {
    md: Markdown,
    follow: bool,
    do_validate: bool,
    show_root: bool,
    graph_options: ReferenceGraphOptions,
    validation_options: ReferenceValidationOptions,
    layout: Layout,
    model: Option<model::FileTreeModel>,
    graph: Option<ReferenceGraph>,
    validation_report: Option<ReferenceValidationReport>,
}

impl std::fmt::Debug for FileTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileTree")
            .field("follow", &self.follow)
            .field("do_validate", &self.do_validate)
            .field("show_root", &self.show_root)
            .field("model_built", &self.model.is_some())
            .finish()
    }
}

impl FileTree {
    /// Creates a FileTree from a file path.
    ///
    /// ## Errors
    ///
    /// Returns [`FileTreeError::PathNotFound`] if the path does not exist,
    /// [`FileTreeError::NotAFile`] if the path is not a file.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, FileTreeError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(FileTreeError::PathNotFound(path.to_path_buf()));
        }
        if !path.is_file() {
            return Err(FileTreeError::NotAFile(path.to_path_buf()));
        }
        let md = Markdown::try_from(path).map_err(FileTreeError::Markdown)?;
        Ok(Self::from_markdown(md))
    }

    /// Creates a FileTree from an already-loaded Markdown document.
    ///
    /// ## Notes
    ///
    /// The graph and validation options share one runtime context, captured
    /// from `md` rather than eagerly. Taking `ReferenceGraphOptions::default()`
    /// and `ReferenceValidationOptions::default()` instead would run the
    /// repo-wide sniff scan (git, repo, file changes, languages, docs, OS,
    /// hardware, GPU) *twice* — 2.8s measured on this working tree — before the
    /// tree knows whether the document reads any `ctx.*` at all.
    pub fn from_markdown(md: Markdown) -> Self {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let compose =
            ComposeOptions::new_with_context(ComposeContext::capture_for_document(&base_dir, &md));
        let graph_options = ReferenceGraphOptions::with_compose(compose);
        let validation_options = ReferenceValidationOptions::with_graph(graph_options.clone());
        Self {
            md,
            follow: false,
            do_validate: false,
            show_root: true,
            graph_options,
            validation_options,
            layout: Layout::default(),
            model: None,
            graph: None,
            validation_report: None,
        }
    }

    /// Enable recursive expansion of followable transclusions.
    pub fn follow_transclusions(mut self) -> Self {
        self.follow = true;
        self.model = None;
        self
    }

    /// Enable validation overlays.
    pub fn validate(mut self) -> Self {
        self.do_validate = true;
        self.model = None;
        self
    }

    /// Set graph options for reference extraction.
    pub fn graph_options(mut self, options: ReferenceGraphOptions) -> Self {
        self.graph_options = options;
        self.model = None;
        self
    }

    /// Set validation options.
    pub fn validation_options(mut self, options: ReferenceValidationOptions) -> Self {
        self.validation_options = options;
        self.model = None;
        self
    }

    /// Set whether to show the root file label.
    pub fn show_root(mut self, show: bool) -> Self {
        self.show_root = show;
        self
    }

    /// Lazily build the view model, graph, and optional validation report.
    ///
    /// This is idempotent — calling it multiple times without changing
    /// builder options is a no-op.
    ///
    /// ## Errors
    ///
    /// Returns [`FileTreeError`] if graph building or validation fails.
    pub fn ensure_built(&mut self) -> Result<(), FileTreeError> {
        if self.model.is_some() {
            return Ok(());
        }

        let graph = self
            .md
            .reference_graph(self.graph_options.clone())
            .map_err(|e| match e {
                MarkdownError::Reference(re) => FileTreeError::Reference(*re),
                other => FileTreeError::Markdown(other),
            })?;

        let report = if self.do_validate {
            let mut val_opts = self.validation_options.clone();
            val_opts.graph = self.graph_options.clone();
            // Eligible for the fresh seam: the graph was built a few lines
            // above from this FileTree's own `md` and `graph_options` in this
            // same method, with no caller handoff in between — descendant
            // re-verification would only re-confirm identities captured there.
            Some(
                validate::validate_fresh_graph(&self.md, &val_opts, &graph)
                    .map_err(FileTreeError::Reference)?,
            )
        } else {
            None
        };

        let built_model = model::build_file_tree_model(&graph, report.as_ref(), self.follow);

        self.graph = Some(graph);
        self.validation_report = report;
        self.model = Some(built_model);
        Ok(())
    }

    /// Access the built reference graph.
    pub fn graph(&self) -> Option<&ReferenceGraph> {
        self.graph.as_ref()
    }

    /// Access the built validation report.
    pub fn validation_report(&self) -> Option<&ReferenceValidationReport> {
        self.validation_report.as_ref()
    }
}

impl TerminalRenderable for FileTree {
    fn render(&self, term: &Terminal) -> String {
        match &self.model {
            Some(model) => {
                let raw = render::render_model(model, term, self.show_root);
                self.layout.apply_layout(&raw, term.width())
            }
            None => {
                if cfg!(debug_assertions) {
                    "[FileTree: call ensure_built() before render()]".to_string()
                } else {
                    String::new()
                }
            }
        }
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        match &self.model {
            Some(model) => {
                let raw = render::render_model_optimistic(model, width as usize, self.show_root);
                self.layout.apply_layout(&raw, width)
            }
            None => {
                if cfg!(debug_assertions) {
                    "[FileTree: call ensure_built() before render()]".to_string()
                } else {
                    String::new()
                }
            }
        }
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_validates_path_exists() {
        let err = FileTree::new("/nonexistent/file.md").unwrap_err();
        assert!(matches!(err, FileTreeError::PathNotFound(_)));
    }

    #[test]
    fn new_validates_is_file() {
        let dir = TempDir::new().unwrap();
        let err = FileTree::new(dir.path()).unwrap_err();
        assert!(matches!(err, FileTreeError::NotAFile(_)));
    }

    #[test]
    fn from_markdown_builds_and_renders() {
        let md = Markdown::new("# Hello\n\n[link](https://example.com)");
        let mut tree = FileTree::from_markdown(md);
        tree.ensure_built().unwrap();

        let output = tree.render_optimistic(Some(120));
        assert!(output.contains("example.com"));
    }

    #[test]
    fn ensure_built_is_idempotent() {
        let md = Markdown::new("# Hello");
        let mut tree = FileTree::from_markdown(md);
        tree.ensure_built().unwrap();
        tree.ensure_built().unwrap(); // second call should be no-op
        assert!(tree.graph().is_some());
    }

    #[test]
    fn follow_transclusions_invalidates_model() {
        let md = Markdown::new("# Hello");
        let mut tree = FileTree::from_markdown(md);
        tree.ensure_built().unwrap();
        assert!(tree.model.is_some());
        tree = tree.follow_transclusions();
        assert!(tree.model.is_none());
    }

    #[test]
    fn validation_report_none_when_not_enabled() {
        let md = Markdown::new("# Hello");
        let mut tree = FileTree::from_markdown(md);
        tree.ensure_built().unwrap();
        assert!(tree.validation_report().is_none());
    }

    #[test]
    fn file_tree_from_real_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(
            &file_path,
            "# Test\n\n[link](https://example.com)\n\n![img](./logo.png)",
        )
        .unwrap();

        let mut tree = FileTree::new(&file_path).unwrap();
        tree.ensure_built().unwrap();

        let output = tree.render_optimistic(Some(120));
        assert!(output.contains("test.md"));
        assert!(output.contains("example.com"));
        assert!(output.contains("logo.png"));
    }
}
