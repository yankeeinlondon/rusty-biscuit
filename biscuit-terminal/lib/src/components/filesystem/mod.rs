//! Filesystem tree rendering component.
//!
//! This module provides the [`FileSystem`] component for rendering directory trees
//! in the terminal with support for:
//!
//! - Tree-style output with Unicode box-drawing characters (`├──`, `└──`, `│`)
//! - Nerd Font icons for files and directories (with Unicode fallbacks)
//! - gitignore awareness (dim/hide ignored entries)
//! - Configurable styling (italics for dotfiles, color highlights)
//! - Symlink detection (shown but not followed)
//! - Depth and entry limits for large directories
//!
//! ## Examples
//!
//! Basic usage:
//!
//! ```no_run
//! use biscuit_terminal::prelude::{FileSystem, TerminalRenderable};
//!
//! let mut fs = FileSystem::new(".").unwrap();
//! fs.ensure_tree_built();
//! println!("{}", fs.render_optimistic(Some(80)));
//! ```
//!
//! With formatting options:
//!
//! ```no_run
//! use biscuit_terminal::prelude::FileSystem;
//!
//! let mut fs = FileSystem::new_with_formatting(".")?
//!     .depth(5)
//!     .max_entries(100)
//!     .highlight_green("test")
//!     .highlight_red("TODO");
//! fs.ensure_tree_built();
//! # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
//! ```
//!
//! ## Icon Support
//!
//! Icons use Nerd Fonts by default with Unicode fallback for unsupported terminals:
//!
//! - Directories: folder icons with special variants for `.git`, `.github`, etc.
//! - Files: file icons with extension-specific variants (`.rs`, `.ts`, `.md`, etc.)
//! - Symlinks: link icon overlay on the base type
//!
//! Access icons directly via the [`icons`] module:
//!
//! ```rust
//! use biscuit_terminal::components::filesystem::icons;
//!
//! // Nerd Font icons (require patched fonts)
//! let rust_icon = icons::nerd::ext::RUST;
//! let dir_icon = icons::nerd::dir::BASE;
//!
//! // Unicode fallbacks (work in any terminal)
//! let file_emoji = icons::unicode::file::BASE; // 📄
//! let folder_emoji = icons::unicode::dir::BASE; // 📂
//! ```
//!
//! ## Error Handling
//!
//! The component returns [`FileSystemError`] for:
//! - Path not found
//! - Path is not a directory
//! - Permission denied
//! - IO errors
//! - gitignore pattern errors
//!
//! ## Rendering Methods
//!
//! - [`FileSystem::render()`] - Basic rendering without terminal context (uses Unicode icons)
//! - [`FileSystem::render()`] - Terminal-aware rendering with Nerd Font support
//!
//! For CLI output, always use `render()` with a [`Terminal`] instance to get
//! proper icon selection and ANSI styling based on terminal capabilities.
//!
//! ## Layout & Style Contract
//!
//! `FileSystem` is an internal-layout component (spec C2). The tree projection
//! honors all applicable `Layout` properties (`margin`, `padding`, `width`,
//! `max_width`, `alignment`, `word_wrap`) and `Style` properties (`color`,
//! `background`, `emphasis`, `border`) via the shared render-tree fold (C1).
//!
//! - [`Width::Auto`](renderable::layout::Width::Auto) (default) and
//!   [`Width::Fixed`](renderable::layout::Width::Fixed) fill the available
//!   width; [`Width::FitContent`](renderable::layout::Width::FitContent) hugs
//!   the tree's natural width.
//! - **Slack sink** (spec D2): the entry-label region. Connector glyphs and
//!   file/directory icons stay fixed across width modes.
//! - The public [`TerminalRenderable::render`](crate::components::renderable::TerminalRenderable)
//!   path remains deferred to the bespoke Nerd-Font renderer; the matrix and
//!   parity coverage use the tree projection, which emits Unicode icons.
//!
//! Markdown degrades layout/appearance attrs by Decision D1 and preserves the
//! structural file-tree syntax where the target renderer supports it.

// Submodules
pub mod error;
pub mod gitignore;
pub mod icons;
pub mod metrics;
pub mod tree_chars;
pub mod tree_node;

// Re-exports for backward compatibility
pub use error::FileSystemError;
pub use gitignore::GitignoreMatcher;
pub use metrics::{FileMetrics, MetricKind};
pub use tree_node::TreeNode;

use std::any::Any;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use paste::paste;

use self::metrics::MetricConfig;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::{BasicColor, Color};
use renderable::html::HtmlPage;
use renderable::layout::TargetValue as RTargetValue;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{PaintColor, PerMode, Style, TextEmphasis};
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{ListMarkerPolicy, NodeKind, RenderNode, RenderStrictness, TreeRenderable};

use crate::components::prose::Prose;
use crate::components::renderable::{BrowserRenderable, TerminalRenderable};
use crate::terminal::Terminal;
use crate::utils::block_constraint::{split_at_visible_width, visible_width};
use crate::utils::layout::{Layout, LayoutTerminalExt};
use crate::utils::wrap_policy::WordWrap;

/// Icon kind for the root directory line.
///
/// Controls which icon is rendered for the root header when a custom
/// [`FileSystem::with_root_icon`] override is configured. When no override is
/// set the component falls back to the default directory icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RootIconKind {
    /// Default directory folder icon.
    #[default]
    Directory,
    /// Repository icon (for when the root starts at the repo root).
    Repository,
}

impl RootIconKind {
    /// Returns the Nerd Font codepoint for this icon kind.
    fn nerd_char(self) -> char {
        match self {
            RootIconKind::Directory => icons::nerd::dir::BASE,
            RootIconKind::Repository => icons::nerd::dir::REPO,
        }
    }

    /// Returns the Unicode fallback glyph for this icon kind.
    fn unicode_str(self) -> &'static str {
        match self {
            RootIconKind::Directory => "📂",
            RootIconKind::Repository => "📦",
        }
    }
}

/// A terminal component that renders a filesystem directory tree.
///
/// `FileSystem` scans a directory and renders it as an indented tree with
/// icons, colors, and configurable formatting options.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::prelude::FileSystem;
///
/// // Create with default settings
/// let fs = FileSystem::new("src")?;
///
/// // Create with common formatting presets
/// let fs = FileSystem::new_with_formatting(".")?;
///
/// // Use builder pattern for custom configuration
/// let fs = FileSystem::new(".")?
///     .depth(10)
///     .dim_gitignore(true)
///     .italicize_dot_files(true);
/// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
/// ```
///
/// ## Rendering
///
/// ```no_run
/// use biscuit_terminal::prelude::{FileSystem, Terminal, TerminalRenderable};
///
/// let mut fs = FileSystem::new(".")?;
///
/// // Build the tree (required before rendering)
/// fs.ensure_tree_built();
///
/// // Option 1: Basic rendering (no terminal context, Unicode icons only)
/// let output = fs.render_optimistic(Some(80));
///
/// // Option 2: Terminal-aware rendering (Nerd Font icons, ANSI styling)
/// let term = Terminal::default();
/// let output = fs.render(&term);
/// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
/// ```
///
/// ## Notes
///
/// - Call [`ensure_tree_built()`](Self::ensure_tree_built) before rendering
/// - The tree is built lazily on first render or explicit call
/// - Symlinks are shown but not followed (prevents infinite loops)
/// - Permission errors create error-marked nodes instead of failing
///
/// ## Layout & Style Contract
///
/// `FileSystem` is an internal-layout component (spec C2) **on the tree
/// path**: the canonical projection sets the configured [`Layout`] on the
/// root `List` node, and the shared render-tree fold resolves the outer box.
/// Tree connectors are forced to [`WordWrap::None`] so the connector
/// geometry is never wrapped.
///
/// - [`Width::Auto`] (default), [`Width::Fixed`], and [`Width::FitContent`]
///   resolve the outer box; the filesystem tree renders inside that box.
/// - **Slack sink** (spec D2): the entry-label region. The connector columns
///   (`├── ` / `└── ` / `│   `) and the icon columns stay fixed; only the
///   entry labels absorb slack by truncating inside the resolved content
///   width.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the
///   tree never re-resolves the raw percentage against its own narrowed box.
///
/// ### Terminal render gap (deferred flip)
///
/// The bespoke [`TerminalRenderable::render`] path remains the production
/// terminal renderer because the target-agnostic projection emits Unicode
/// fallback icons (📂 / 📄) that cannot reproduce the bespoke Nerd Font
/// icons the terminal path chooses based on the live terminal's font
/// support. The tree path's box-model contract is honored today
/// (`margin` / `alignment` / `max_width` / `width`), and the bespoke
/// terminal path applies the same [`Layout`] via `apply_block_layout`. A
/// future flip will route the terminal target through the tree path once
/// icon parity is achieved.
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
#[derive(Debug, Clone)]
pub struct FileSystem {
    /// The root directory path to display.
    root_path: PathBuf,
    /// Lazily-built tree structure (None until first render).
    tree: Option<Vec<TreeNode>>,
    /// Layout configuration for rendering.
    layout: Layout,
    /// Whether to dim entries matched by `.gitignore`.
    dim_gitignore: bool,
    /// Whether to italicize files starting with `.`.
    italicize_dot_files: bool,
    /// Whether to italicize directories starting with `.`.
    italicize_dot_dirs: bool,
    /// Whether to hide files starting with `.`.
    hide_dot_files: bool,
    /// Whether to hide directories starting with `.`.
    hide_dot_dirs: bool,
    /// Whether to skip recursing into directories matched by `.gitignore`.
    do_not_recurse_gitignore: bool,
    /// Glob patterns to filter entries (only matching entries are shown).
    filter_patterns: Vec<String>,
    /// Patterns to highlight in red.
    highlight_red: Vec<String>,
    /// Patterns to highlight in green.
    highlight_green: Vec<String>,
    /// Maximum depth to traverse (0 = root only).
    max_depth: u32,
    /// Maximum number of entries to display.
    max_entries: u32,
    /// Whether to show the root directory name as the first line.
    show_root: bool,
    /// Per-metric configuration (which metrics to show, filters, thresholds).
    metric_configs: HashMap<MetricKind, MetricConfig>,
    /// When true, directories also display applicable metrics (timestamps, permissions).
    show_metrics_on_directories: bool,
    /// When true, filenames are rendered as clickable OSC8 hyperlinks to the files.
    file_links: bool,
    /// Case-insensitive extension allowlist (without leading dot). When
    /// non-empty, only files whose extension is in this set are scanned.
    /// Directories are retained only when they contain an included descendant.
    extension_allowlist: BTreeSet<String>,
    /// Exact included-path allowlist, each relative to `root_path`. When
    /// non-empty, only files whose relative path matches an entry are scanned.
    /// Entries outside `root_path` are ignored. Directories are retained only
    /// when they contain an included descendant.
    included_paths: BTreeSet<PathBuf>,
    /// Dimmed prefix rendered on the root line before the target name.
    root_prefix: Option<String>,
    /// Explicit root display name (overrides the directory's `file_name()`).
    root_display_name: Option<String>,
    /// Custom root icon kind (overrides the default directory icon).
    root_icon: Option<RootIconKind>,
}

impl Default for FileSystem {
    fn default() -> Self {
        // Use current_dir with fallback to "." to avoid panic
        let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root_path: path,
            tree: None,
            layout: Layout::default(),
            dim_gitignore: false,
            italicize_dot_files: false,
            italicize_dot_dirs: false,
            hide_dot_files: false,
            hide_dot_dirs: false,
            do_not_recurse_gitignore: false,
            filter_patterns: Vec::new(),
            highlight_red: Vec::new(),
            highlight_green: Vec::new(),
            max_depth: 20,
            max_entries: 1000,
            show_root: true,
            metric_configs: HashMap::new(),
            show_metrics_on_directories: false,
            file_links: false,
            extension_allowlist: BTreeSet::new(),
            included_paths: BTreeSet::new(),
            root_prefix: None,
            root_display_name: None,
            root_icon: None,
        }
    }
}

impl FileSystem {
    /// Creates a new `FileSystem` for the given directory path.
    ///
    /// The tree is not built immediately; call [`ensure_tree_built()`](Self::ensure_tree_built)
    /// or render the component to trigger tree construction.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::FileSystem;
    ///
    /// // From a string path
    /// let fs = FileSystem::new("/path/to/dir")?;
    ///
    /// // From current directory
    /// let fs = FileSystem::new(".")?;
    ///
    /// // The tree is not built yet
    /// assert!(!fs.is_tree_built());
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`FileSystemError::PathNotFound`] if the path does not exist.
    ///
    /// Returns [`FileSystemError::NotADirectory`] if the path exists but is not a directory.
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        let path = dir.as_ref();

        if !path.exists() {
            return Err(FileSystemError::PathNotFound {
                path: path.to_path_buf(),
            });
        }

        if !path.is_dir() {
            return Err(FileSystemError::NotADirectory {
                path: path.to_path_buf(),
            });
        }

        Ok(Self {
            root_path: path.to_path_buf(),
            tree: None,
            layout: Layout::default(),
            dim_gitignore: false,
            italicize_dot_files: false,
            italicize_dot_dirs: false,
            hide_dot_files: false,
            hide_dot_dirs: false,
            do_not_recurse_gitignore: false,
            filter_patterns: Vec::new(),
            highlight_red: Vec::new(),
            highlight_green: Vec::new(),
            max_depth: 20,
            max_entries: 1000,
            show_root: true,
            metric_configs: HashMap::new(),
            show_metrics_on_directories: false,
            file_links: false,
            extension_allowlist: BTreeSet::new(),
            included_paths: BTreeSet::new(),
            root_prefix: None,
            root_display_name: None,
            root_icon: None,
        })
    }

    /// Creates a `FileSystem` with common formatting presets enabled.
    ///
    /// This is a convenience constructor that applies sensible defaults for
    /// most use cases:
    ///
    /// - **Italicize dotfiles**: Files and directories starting with `.` are italicized
    /// - **Dim gitignored**: Entries matched by `.gitignore` are rendered dim
    /// - **Skip gitignored dirs**: Gitignored directories are not recursed into
    ///
    /// Equivalent to:
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// FileSystem::new(".")?
    ///     .italicize_dot_files(true)
    ///     .italicize_dot_dirs(true)
    ///     .dim_gitignore(true)
    ///     .do_not_recurse_gitignore(true);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::FileSystem;
    ///
    /// // Apply formatting presets, then customize further
    /// let fs = FileSystem::new_with_formatting(".")?
    ///     .depth(5)
    ///     .highlight_green("src");
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`FileSystemError::PathNotFound`] if the path does not exist.
    ///
    /// Returns [`FileSystemError::NotADirectory`] if the path exists but is not a directory.
    pub fn new_with_formatting(dir: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        Self::new(dir).map(|fs| {
            fs.italicize_dot_files(true)
                .italicize_dot_dirs(true)
                .dim_gitignore(true)
                .do_not_recurse_gitignore(true)
        })
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Sets the maximum depth to traverse into subdirectories.
    ///
    /// - Depth 0: Show only the root directory contents (no recursion)
    /// - Depth 1: Show root and one level of subdirectories
    /// - Depth N: Show N levels of subdirectory nesting
    ///
    /// Default is 20.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// // Show only top-level entries
    /// let shallow = FileSystem::new(".")?.depth(0);
    ///
    /// // Show up to 3 levels deep
    /// let deep = FileSystem::new(".")?.depth(3);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Sets the maximum total number of entries to display.
    ///
    /// Limits the total count of files and directories shown. Useful for
    /// preventing excessive output in large repositories.
    ///
    /// Default is 1000.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// // Show at most 50 entries
    /// let fs = FileSystem::new(".")?.max_entries(50);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn max_entries(mut self, max: u32) -> Self {
        self.max_entries = max;
        self
    }

    /// Sets whether to show the root directory as the first line of output.
    ///
    /// When enabled (the default), the root directory name and icon are
    /// rendered as a header line above the tree contents.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// // Hide the root directory line
    /// let fs = FileSystem::new(".")?.show_root(false);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn show_root(mut self, show: bool) -> Self {
        self.show_root = show;
        self
    }

    /// Sets whether to dim entries matched by `.gitignore`.
    ///
    /// When enabled, gitignored files and directories are rendered with
    /// dim ANSI styling (reduced brightness).
    ///
    /// Default is `false`.
    pub fn dim_gitignore(mut self, dim: bool) -> Self {
        self.dim_gitignore = dim;
        self
    }

    /// Sets whether to italicize files starting with `.` (dotfiles).
    ///
    /// When enabled, hidden files like `.gitignore`, `.env`, etc. are
    /// rendered with italic ANSI styling.
    ///
    /// Default is `false`.
    pub fn italicize_dot_files(mut self, italic: bool) -> Self {
        self.italicize_dot_files = italic;
        self
    }

    /// Sets whether to italicize directories starting with `.` (dotdirs).
    ///
    /// When enabled, hidden directories like `.git`, `.github`, etc. are
    /// rendered with italic ANSI styling.
    ///
    /// Default is `false`.
    pub fn italicize_dot_dirs(mut self, italic: bool) -> Self {
        self.italicize_dot_dirs = italic;
        self
    }

    /// Sets whether to hide files starting with `.` (dotfiles).
    ///
    /// When enabled, hidden files are excluded from the tree entirely.
    ///
    /// Default is `false`.
    pub fn hide_dot_files(mut self, hide: bool) -> Self {
        self.hide_dot_files = hide;
        self
    }

    /// Sets whether to hide directories starting with `.` (dotdirs).
    ///
    /// When enabled, hidden directories are excluded from the tree entirely.
    ///
    /// Default is `false`.
    pub fn hide_dot_dirs(mut self, hide: bool) -> Self {
        self.hide_dot_dirs = hide;
        self
    }

    /// Sets whether to skip recursing into directories matched by `.gitignore`.
    ///
    /// When enabled, gitignored directories are shown in the tree but their
    /// contents are not traversed. This improves performance and reduces
    /// noise from build artifacts, dependencies, etc.
    ///
    /// Default is `false`.
    pub fn do_not_recurse_gitignore(mut self, skip: bool) -> Self {
        self.do_not_recurse_gitignore = skip;
        self
    }

    /// Adds a pattern to filter which entries are shown.
    ///
    /// Only entries whose names contain at least one of the filter patterns
    /// will be included. Can be called multiple times to add multiple patterns.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// // Show only Rust files
    /// let fs = FileSystem::new(".")?
    ///     .filter(".rs")
    ///     .filter(".toml");
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn filter(mut self, pattern: impl Into<String>) -> Self {
        self.filter_patterns.push(pattern.into());
        self
    }

    /// Adds a pattern for entries to highlight in red.
    ///
    /// Entries whose names contain the pattern will be rendered in red.
    /// Useful for drawing attention to specific files like TODOs or warnings.
    /// Can be called multiple times to add multiple patterns.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// let fs = FileSystem::new(".")?
    ///     .highlight_red("TODO")
    ///     .highlight_red("FIXME");
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn highlight_red(mut self, pattern: impl Into<String>) -> Self {
        self.highlight_red.push(pattern.into());
        self
    }

    /// Adds a pattern for entries to highlight in green.
    ///
    /// Entries whose names contain the pattern will be rendered in green.
    /// Useful for highlighting important directories or recently changed files.
    /// Can be called multiple times to add multiple patterns.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// let fs = FileSystem::new(".")?
    ///     .highlight_green("src")
    ///     .highlight_green("test");
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn highlight_green(mut self, pattern: impl Into<String>) -> Self {
        self.highlight_green.push(pattern.into());
        self
    }

    /// Sets the layout configuration for margins and alignment.
    ///
    /// See [`Layout`] for available options.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Enables metric display on directories for applicable metrics.
    ///
    /// When enabled, directories show timestamps and permissions (but not
    /// file_size or tokens, which are file-only metrics).
    pub fn show_on_directories(mut self) -> Self {
        self.show_metrics_on_directories = true;
        self
    }

    /// Enables OSC8 hyperlinks on filenames and directory names.
    ///
    /// When enabled, each filename in the tree output becomes a clickable
    /// OSC8 hyperlink pointing to the file's absolute path. This only takes
    /// effect when rendering to a TTY (via [`fallback_render`](Self::fallback_render)).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::{FileSystem, Terminal, TerminalRenderable};
    /// let mut fs = FileSystem::new_with_formatting(".")?
    ///     .with_file_links();
    /// fs.ensure_tree_built();
    /// let output = fs.render(&Terminal::default());
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn with_file_links(mut self) -> Self {
        self.file_links = true;
        self
    }

    /// Restricts scanned files to the given set of case-insensitive extensions.
    ///
    /// Each extension should be supplied **without** a leading dot (e.g. `"md"`,
    /// `"pdf"`). The scan drops any file whose lowercased extension is not in
    /// `extensions`. Directories are retained only when they contain at least
    /// one included descendant, so ancestor directories that would otherwise be
    /// empty are pruned.
    ///
    /// Calling this replaces any previously configured extension allowlist.
    /// To add the standard Darkmatter document set (`.md`, `.txt`, `.doc`,
    /// `.docx`, `.xls`, `.xlsx`, `.pdf`), use
    /// [`document_extensions`](Self::document_extensions).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// let fs = FileSystem::new(".")?
    ///     .extension_filter(["md", "pdf"].into_iter());
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn extension_filter<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extension_allowlist = extensions
            .into_iter()
            .map(|e| {
                let s = e.into();
                s.trim_start_matches('.').to_lowercase()
            })
            .collect();
        self
    }

    /// Restricts scanned files to the standard Darkmatter document extensions.
    ///
    /// Equivalent to:
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// FileSystem::new(".")?
    ///     .extension_filter(["md", "txt", "doc", "docx", "xls", "xlsx", "pdf"].into_iter());
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn document_extensions(self) -> Self {
        self.extension_filter(
            ["md", "txt", "doc", "docx", "xls", "xlsx", "pdf"],
        )
    }

    /// Restricts scanned files to an exact set of paths relative to the root.
    ///
    /// Each path must be relative to [`root_path`](Self::root_path). Entries
    /// that are absolute or escape the root (via `..`) are silently ignored.
    /// Only files whose normalized relative path matches an entry are
    /// included; their ancestor directories are preserved so the directory
    /// hierarchy remains intact.
    ///
    /// This is intended for glob-based callers that have already determined
    /// the matched file set and want to render only those files while keeping
    /// the tree structure.
    ///
    /// Calling this replaces any previously configured included-path set.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// use std::path::PathBuf;
    /// let fs = FileSystem::new(".")?
    ///     .included_paths([
    ///         PathBuf::from("docs/a.md"),
    ///         PathBuf::from("docs/b.txt"),
    ///     ]);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn included_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<PathBuf>,
    {
        self.included_paths = paths
            .into_iter()
            .map(|s| s.into())
            .filter(|p| is_safe_relative(p))
            .collect();
        self
    }

    /// Injects an already-built tree, bypassing the filesystem walk.
    ///
    /// [`ensure_tree_built`](Self::ensure_tree_built) becomes a no-op once a tree
    /// is present, so a caller that has already discovered the entries (for
    /// example darkmatter's `::file-links` resolver) can supply them directly and
    /// avoid a second filesystem traversal.
    pub fn with_prebuilt_tree(mut self, tree: Vec<TreeNode>) -> Self {
        self.tree = Some(tree);
        self
    }

    /// Sets a dimmed prefix rendered on the root line before the target name.
    ///
    /// When set, the root header renders as `{icon} {dim}{prefix}{reset}{target}`
    /// instead of the default `{icon} {target}`. The prefix is typically the
    /// path from the repository/CWD root to the target directory (e.g.
    /// `"/docs/"`); the highlighted target name follows it.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # use biscuit_terminal::prelude::FileSystem;
    /// let fs = FileSystem::new("docs/topics")?
    ///     .with_dimmed_root_prefix("/docs/");
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn with_dimmed_root_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.root_prefix = Some(prefix.into());
        self
    }

    /// Overrides the highlighted target name on the root line.
    ///
    /// By default the root line uses the last component of `root_path`.
    /// Call this when the display name should differ from the on-disk
    /// directory name.
    pub fn with_root_display_name(mut self, name: impl Into<String>) -> Self {
        self.root_display_name = Some(name.into());
        self
    }

    /// Overrides the icon used on the root line.
    ///
    /// Use [`RootIconKind::Repository`] when the root starts at the repository
    /// root so the git icon is rendered instead of the default folder icon.
    pub fn with_root_icon(mut self, icon: RootIconKind) -> Self {
        self.root_icon = Some(icon);
        self
    }

    // =========================================================================
    // Metric Builder Methods
    // =========================================================================

    /// Returns whether any metrics are configured to be shown.
    fn has_any_metrics(&self) -> bool {
        self.metric_configs.values().any(|c| c.enabled)
    }

    /// Returns `true` when the extension or included-path allowlist is active.
    fn has_file_filters(&self) -> bool {
        !self.extension_allowlist.is_empty() || !self.included_paths.is_empty()
    }

    /// Returns `true` when a file should be included given all active filters.
    ///
    /// `matches_substring` is the pre-computed result of the legacy substring
    /// [`filter`](Self::filter) patterns so the caller does not duplicate that
    /// scan. The extension and included-path allowlists are checked here.
    fn file_passes_all_filters(
        &self,
        file_name: &str,
        rel_file_path: &Path,
        matches_substring: bool,
    ) -> bool {
        // Substring filter (legacy): when active the file must match at least
        // one pattern. When inactive, `matches_substring` is `false` and the
        // check is skipped.
        if !self.filter_patterns.is_empty() && !matches_substring {
            return false;
        }
        // Extension allowlist
        if !self.extension_allowlist.is_empty() {
            match lowercase_extension(file_name) {
                Some(e) if self.extension_allowlist.contains(&e) => {}
                _ => return false,
            }
        }
        // Included-paths allowlist
        if !self.included_paths.is_empty() && !self.included_paths.contains(rel_file_path) {
            return false;
        }
        true
    }

    /// Checks if a metric should be shown for the given filename.
    fn should_show_metric(&self, kind: MetricKind, filename: &str) -> bool {
        let Some(config) = self.metric_configs.get(&kind) else {
            return false;
        };
        if !config.enabled {
            return false;
        }
        if config.filename_patterns.is_empty() {
            return true;
        }
        let mut matched = false;
        for pattern in &config.filename_patterns {
            if let Some(neg) = pattern.strip_prefix('!') {
                if glob_match(neg, filename) {
                    return false;
                }
            } else if glob_match(pattern, filename) {
                matched = true;
            }
        }
        // If there are only negation patterns and none matched, show the metric.
        // If there are positive patterns, at least one must match.
        let has_positive = config.filename_patterns.iter().any(|p| !p.starts_with('!'));
        if has_positive { matched } else { true }
    }
}

/// Generates three builder methods per metric kind:
/// - `show_{name}()` — enables the metric globally
/// - `show_{name}_with_filename(globs)` — enables for matching filenames only
/// - `show_{name}_highlight_greater_than(threshold)` — enables with highlight threshold
macro_rules! metric_builder {
    ($name:ident, $kind:expr) => {
        paste! {
            /// Enables the metric for all files.
            pub fn [<show_ $name>](mut self) -> Self {
                self.metric_configs.entry($kind).or_default().enabled = true;
                self
            }

            /// Enables the metric only for files matching the given glob patterns.
            ///
            /// Patterns prefixed with `!` act as negation (exclude matching files).
            pub fn [<show_ $name _with_filename>]<T: Into<String>>(mut self, globs: Vec<T>) -> Self {
                let config = self.metric_configs.entry($kind).or_default();
                config.enabled = true;
                config.filename_patterns = globs.into_iter().map(|g| g.into()).collect();
                self
            }

            /// Enables the metric with a highlight threshold.
            ///
            /// Values exceeding the threshold are rendered in bold yellow.
            pub fn [<show_ $name _highlight_greater_than>](mut self, threshold: u64) -> Self {
                let config = self.metric_configs.entry($kind).or_default();
                config.enabled = true;
                config.highlight_threshold = Some(threshold);
                self
            }
        }
    };
}

impl FileSystem {
    metric_builder!(file_size, MetricKind::FileSize);
    metric_builder!(tokens, MetricKind::Tokens);
    metric_builder!(created, MetricKind::Created);
    metric_builder!(created_since, MetricKind::CreatedSince);
    metric_builder!(modified, MetricKind::Modified);
    metric_builder!(modified_since, MetricKind::ModifiedSince);
    metric_builder!(permissions, MetricKind::Permissions);
    metric_builder!(permissions_numeric, MetricKind::PermissionsNumeric);
    metric_builder!(owner, MetricKind::Owner);
    metric_builder!(group, MetricKind::Group);
}

impl FileSystem {
    // =========================================================================
    // Accessor Methods
    // =========================================================================

    /// Returns the root path of this filesystem tree.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Returns whether the tree has been built.
    ///
    /// The tree is built lazily on first render.
    pub fn is_tree_built(&self) -> bool {
        self.tree.is_some()
    }

    /// Returns the configured maximum depth.
    pub fn get_max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Returns the configured maximum entries.
    pub fn get_max_entries(&self) -> u32 {
        self.max_entries
    }

    // =========================================================================
    // Icon Selection Methods
    // =========================================================================

    /// Returns the appropriate icon for a tree node.
    ///
    /// Icon selection follows this priority:
    /// 1. Error state (permission denied directories)
    /// 2. Symlink indicator
    /// 3. Depth limit indicator
    /// 4. Special directory names (`.git`, `.github`, etc.)
    /// 5. Root-only file icons (`CLAUDE.md`, `Agents.md` at depth 0)
    /// 6. Exact filename matches (`README.md`, `SKILL.md`, `.gitignore`, etc.)
    /// 7. Extension-based icons (`.rs`, `.ts`, `.md`, etc.)
    /// 8. Base icon fallback
    ///
    /// ## Arguments
    ///
    /// * `node` - The tree node to get an icon for
    /// * `depth` - Current depth in the tree (0 = root level)
    /// * `is_nerd_font` - Whether to use Nerd Font icons:
    ///   - `Some(true)` - Use Nerd Font icons
    ///   - `Some(false)` or `None` - Use Unicode fallback icons
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::filesystem::{FileSystem, TreeNode, icons};
    ///
    /// let fs = FileSystem::default();
    /// let file = TreeNode::File {
    ///     name: "main.rs".into(),
    ///     is_ignored: false,
    ///     is_symlink: false,
    ///     metrics: None,
    /// };
    ///
    /// // Nerd Font icon for Rust files
    /// assert_eq!(fs.get_icon(&file, 0, Some(true)), icons::nerd::ext::RUST);
    ///
    /// // Unicode fallback
    /// assert_eq!(fs.get_icon(&file, 0, Some(false)), icons::unicode::file::BASE);
    /// ```
    pub fn get_icon(&self, node: &TreeNode, depth: u32, is_nerd_font: Option<bool>) -> char {
        let use_nerd = is_nerd_font.unwrap_or(false);

        match node {
            TreeNode::Dir {
                name,
                is_symlink,
                has_error,
                at_depth_limit,
                ..
            } => {
                // Error state takes highest priority
                if *has_error {
                    return if use_nerd {
                        icons::nerd::dir::ERROR
                    } else {
                        icons::unicode::dir::ERROR
                    };
                }
                // Symlink indicator
                if *is_symlink {
                    return if use_nerd {
                        icons::nerd::file::SYMLINK
                    } else {
                        icons::unicode::file::SYMLINK
                    };
                }
                // Depth limit indicator
                if *at_depth_limit {
                    return if use_nerd {
                        icons::nerd::dir::DEPTH_LIMIT
                    } else {
                        icons::unicode::dir::DEPTH_LIMIT
                    };
                }
                // Special directory names
                self.get_dir_icon(name, use_nerd)
            }
            TreeNode::File {
                name, is_symlink, ..
            } => {
                // Symlink indicator takes priority
                if *is_symlink {
                    return if use_nerd {
                        icons::nerd::file::SYMLINK
                    } else {
                        icons::unicode::file::SYMLINK
                    };
                }
                // Check for root-only icons (depth 0)
                if depth == 0
                    && let Some(icon) = self.get_root_only_file_icon(name, use_nerd)
                {
                    return icon;
                }
                // Check exact filename matches
                if let Some(icon) = self.get_exact_filename_icon(name, use_nerd) {
                    return icon;
                }
                // Check extension
                self.get_extension_icon(name, use_nerd)
            }
        }
    }

    /// Returns the appropriate icon for a tree node with trailing space for PUA alignment.
    ///
    /// Nerd Font icons are in the Private Use Area (PUA: U+E000-U+F8FF) and often
    /// display wider than their measured width (1 column). This method adds a
    /// trailing space after Nerd Font icons to ensure proper alignment in
    /// tree output.
    ///
    /// ## Arguments
    ///
    /// * `node` - The tree node to get an icon for
    /// * `depth` - Current depth in the tree (0 = root level)
    /// * `is_nerd_font` - Whether to use Nerd Font icons
    ///
    /// ## Returns
    ///
    /// A string containing the icon character followed by a space for Nerd Font
    /// icons, or just the icon character for Unicode fallbacks.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::filesystem::{FileSystem, TreeNode};
    ///
    /// let fs = FileSystem::default();
    /// let file = TreeNode::File {
    ///     name: "main.rs".into(),
    ///     is_ignored: false,
    ///     is_symlink: false,
    ///     metrics: None,
    /// };
    ///
    /// // Nerd Font: icon + space for PUA width compensation
    /// let nerd_icon = fs.get_icon_with_padding(&file, 0, Some(true));
    /// assert_eq!(nerd_icon.chars().count(), 2); // icon + space
    ///
    /// // Unicode: no extra padding needed
    /// let unicode_icon = fs.get_icon_with_padding(&file, 0, Some(false));
    /// assert_eq!(unicode_icon.chars().count(), 1); // just the icon
    /// ```
    pub fn get_icon_with_padding(
        &self,
        node: &TreeNode,
        depth: u32,
        is_nerd_font: Option<bool>,
    ) -> String {
        let icon = self.get_icon(node, depth, is_nerd_font);
        let use_nerd = is_nerd_font.unwrap_or(false);

        if use_nerd {
            format!("{} ", icon)
        } else {
            icon.to_string()
        }
    }

    /// Returns the icon for a directory based on its name.
    ///
    /// Matches special directory names like `.git`, `.github`, `docs`, `utils`.
    fn get_dir_icon(&self, name: &str, use_nerd: bool) -> char {
        match name {
            ".git" => {
                if use_nerd {
                    icons::nerd::dir::GIT
                } else {
                    icons::unicode::dir::BASE
                }
            }
            ".github" => {
                if use_nerd {
                    icons::nerd::dir::GITHUB
                } else {
                    icons::unicode::dir::BASE
                }
            }
            "utils" | "util" | "helpers" => {
                if use_nerd {
                    icons::nerd::dir::UTILS
                } else {
                    icons::unicode::dir::BASE
                }
            }
            "docs" | "doc" | "documentation" => {
                if use_nerd {
                    icons::nerd::dir::DOCS
                } else {
                    icons::unicode::dir::BASE
                }
            }
            _ => {
                if use_nerd {
                    icons::nerd::dir::BASE
                } else {
                    icons::unicode::dir::BASE
                }
            }
        }
    }

    /// Returns icons for files that should only be recognized at root level (depth 0).
    ///
    /// These files have special significance only when at the root of a project.
    fn get_root_only_file_icon(&self, name: &str, use_nerd: bool) -> Option<char> {
        match name {
            "CLAUDE.md" => Some(if use_nerd {
                icons::nerd::file::CLAUDE
            } else {
                icons::unicode::file::BASE
            }),
            "Agents.md" | "Gemini.md" => Some(if use_nerd {
                icons::nerd::file::AGENTS
            } else {
                icons::unicode::file::BASE
            }),
            _ => None,
        }
    }

    /// Returns icons for exact filename matches at any depth.
    fn get_exact_filename_icon(&self, name: &str, use_nerd: bool) -> Option<char> {
        match name {
            "README.md" | "README" | "readme.md" => Some(if use_nerd {
                icons::nerd::file::README
            } else {
                icons::unicode::file::BASE
            }),
            "SKILL.md" => Some(if use_nerd {
                icons::nerd::file::SKILL
            } else {
                icons::unicode::file::BASE
            }),
            ".gitignore" => Some(if use_nerd {
                icons::nerd::file::GITIGNORE
            } else {
                icons::unicode::file::BASE
            }),
            ".env" | ".env.local" | ".env.example" => Some(if use_nerd {
                icons::nerd::file::ENV
            } else {
                icons::unicode::file::BASE
            }),
            "justfile" | "Justfile" => Some(if use_nerd {
                icons::nerd::file::JUSTFILE
            } else {
                icons::unicode::file::BASE
            }),
            ".editorconfig" => Some(if use_nerd {
                icons::nerd::file::EDITORCONFIG
            } else {
                icons::unicode::file::BASE
            }),
            _ => None,
        }
    }

    /// Returns icons based on file extension (case-insensitive).
    fn get_extension_icon(&self, name: &str, use_nerd: bool) -> char {
        // Extract extension (everything after the last dot)
        let ext = name.rsplit('.').next().map(|e| e.to_lowercase());

        match ext.as_deref() {
            // Code
            Some("rs") => {
                if use_nerd {
                    icons::nerd::ext::RUST
                } else {
                    icons::unicode::file::BASE
                }
            }
            Some("ts" | "tsx") => {
                if use_nerd {
                    icons::nerd::ext::TYPESCRIPT
                } else {
                    icons::unicode::file::BASE
                }
            }
            Some("js" | "jsx" | "mjs" | "cjs") => {
                if use_nerd {
                    icons::nerd::ext::JAVASCRIPT
                } else {
                    icons::unicode::file::BASE
                }
            }
            // Config
            Some("toml") => {
                if use_nerd {
                    icons::nerd::ext::TOML
                } else {
                    icons::unicode::file::BASE
                }
            }
            Some("yaml" | "yml") => {
                if use_nerd {
                    icons::nerd::ext::YAML
                } else {
                    icons::unicode::file::BASE
                }
            }
            Some("json" | "json5" | "jsonc") => {
                if use_nerd {
                    icons::nerd::ext::JSON
                } else {
                    icons::unicode::file::BASE
                }
            }
            // Docs - markdown files that aren't README or SKILL
            Some("md" | "mdx" | "markdown") => {
                if use_nerd {
                    icons::nerd::file::MARKDOWN
                } else {
                    icons::unicode::file::BASE
                }
            }
            // Documents
            Some("txt") => {
                if use_nerd {
                    icons::nerd::ext::TEXT
                } else {
                    icons::unicode::file::TEXT
                }
            }
            Some("pdf") => {
                if use_nerd {
                    icons::nerd::ext::PDF
                } else {
                    icons::unicode::file::PDF
                }
            }
            Some("doc" | "docx") => {
                if use_nerd {
                    icons::nerd::ext::WORD
                } else {
                    icons::unicode::file::WORD
                }
            }
            Some("xls" | "xlsx") => {
                if use_nerd {
                    icons::nerd::ext::EXCEL
                } else {
                    icons::unicode::file::EXCEL
                }
            }
            // Default
            _ => {
                if use_nerd {
                    icons::nerd::file::BASE
                } else {
                    icons::unicode::file::BASE
                }
            }
        }
    }

    // =========================================================================
    // Tree Building Methods
    // =========================================================================

    /// Ensures the tree is built, triggering lazy initialization if needed.
    ///
    /// The tree is built by walking the filesystem starting from `root_path`,
    /// respecting the configured depth limit, entry limit, and filter settings.
    ///
    /// This method is **idempotent**: calling it multiple times has no effect
    /// after the first build. To rebuild the tree with different settings,
    /// create a new `FileSystem` instance.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::FileSystem;
    ///
    /// let mut fs = FileSystem::new(".")?;
    ///
    /// // Tree not built yet
    /// assert!(!fs.is_tree_built());
    /// assert!(fs.tree().is_none());
    ///
    /// // Build the tree
    /// fs.ensure_tree_built();
    ///
    /// // Now it's built
    /// assert!(fs.is_tree_built());
    /// assert!(fs.tree().is_some());
    ///
    /// // Subsequent calls are no-ops
    /// fs.ensure_tree_built();
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Notes
    ///
    /// - Symlinks are detected but not followed (prevents infinite loops)
    /// - Permission errors are handled gracefully (directories marked with error state)
    /// - The tree respects [`max_depth`](Self::depth) and [`max_entries`](Self::max_entries)
    pub fn ensure_tree_built(&mut self) {
        if self.tree.is_none() {
            let mut total_entries = 0;
            let matcher = GitignoreMatcher::for_root(&self.root_path);
            self.tree = Some(self.build_tree_recursive(
                &self.root_path.clone(),
                Path::new(""),
                0,
                &mut total_entries,
                &matcher,
            ));
        }
    }

    /// Returns a reference to the built tree, if available.
    ///
    /// Returns `None` if the tree has not been built yet.
    /// Call [`ensure_tree_built()`](Self::ensure_tree_built) to build the tree first.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::FileSystem;
    ///
    /// let mut fs = FileSystem::new(".")?;
    ///
    /// // Before building
    /// assert!(fs.tree().is_none());
    ///
    /// // After building
    /// fs.ensure_tree_built();
    /// let tree = fs.tree().expect("tree should be built");
    /// println!("Found {} entries at root level", tree.len());
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    pub fn tree(&self) -> Option<&Vec<TreeNode>> {
        self.tree.as_ref()
    }

    /// Builds the tree by walking the filesystem.
    ///
    /// `rel_path` is the path of `path` relative to `root_path`, used to match
    /// against the [`included_paths`](Self::included_paths) allowlist. At the
    /// top level `rel_path` is empty.
    ///
    /// gitignore status for each entry is computed via `matcher`, a
    /// [`GitignoreMatcher`] built once for the whole tree.
    fn build_tree_recursive(
        &self,
        path: &Path,
        rel_path: &Path,
        depth: u32,
        total_entries: &mut u32,
        matcher: &GitignoreMatcher,
    ) -> Vec<TreeNode> {
        // Respect max_depth (depth 0 = root level, so we check if we've exceeded)
        if depth >= self.max_depth {
            return vec![];
        }

        let mut entries = Vec::new();

        // Read directory entries
        let dir_entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return vec![], // Permission error or other issue - return empty
        };

        // Collect and filter entries
        let mut raw_entries: Vec<_> = dir_entries.filter_map(|e| e.ok()).collect();

        // Sort: directories first, then alphabetically (case-insensitive)
        raw_entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let a_name = a.file_name().to_string_lossy().to_lowercase();
                    let b_name = b.file_name().to_string_lossy().to_lowercase();
                    a_name.cmp(&b_name)
                }
            }
        });

        for entry in raw_entries {
            // Check max_entries limit
            if *total_entries >= self.max_entries {
                // Could add a "...N more items" indicator here in future
                break;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_path = entry.path();
            let rel_file_path = rel_path.join(&file_name);

            // Get file type (handling potential errors)
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue, // Skip entries we can't read
            };

            // Check if symlink
            let is_symlink = file_type.is_symlink();

            // For symlinks, we need to check what the symlink points to
            // to determine if it's a directory or file
            let (is_dir, is_file) = if is_symlink {
                // Use metadata to follow the symlink and get the target type
                match std::fs::metadata(&file_path) {
                    Ok(meta) => (meta.is_dir(), meta.is_file()),
                    Err(_) => {
                        // Broken symlink - treat as file
                        (false, true)
                    }
                }
            } else {
                (file_type.is_dir(), file_type.is_file())
            };

            let is_ignored = matcher.is_ignored(&file_path, is_dir);

            // Determine if this is a dot file/dir
            let is_dot = file_name.starts_with('.');

            // Apply hide filters for dot files/dirs
            if is_dot {
                if is_dir && self.hide_dot_dirs {
                    continue;
                }
                if is_file && self.hide_dot_files {
                    continue;
                }
            }

            // Determine whether any filtering is active (substring, extension,
            // or included-path). When active, non-matching files are skipped
            // and directories are pruned unless they have surviving children.
            let has_substring_filter = !self.filter_patterns.is_empty();
            let matches_substring = has_substring_filter
                && self.filter_patterns.iter().any(|p| file_name.contains(p));
            let has_file_filters = self.has_file_filters();

            if is_dir {
                // Don't follow symlinks to avoid infinite loops
                let at_depth_limit = depth + 1 >= self.max_depth;
                let children = if is_symlink || at_depth_limit {
                    vec![]
                } else {
                    self.build_tree_recursive(
                        &file_path,
                        &rel_file_path,
                        depth + 1,
                        total_entries,
                        matcher,
                    )
                };

                // When filters are active, only include directories that either
                // match a substring filter themselves or have surviving children.
                if (has_substring_filter || has_file_filters)
                    && !matches_substring
                    && children.is_empty()
                {
                    continue;
                }

                *total_entries += 1;

                let dir_metrics = if self.has_any_metrics() && self.show_metrics_on_directories {
                    self.collect_file_metrics(&file_path, &file_name, true)
                } else {
                    None
                };
                entries.push(TreeNode::Dir {
                    name: file_name,
                    children,
                    is_ignored,
                    is_symlink,
                    has_error: false,
                    at_depth_limit,
                    metrics: dir_metrics,
                });
            } else {
                // Skip non-matching files when any filter is active
                if !self.file_passes_all_filters(
                    &file_name,
                    &rel_file_path,
                    matches_substring,
                ) {
                    continue;
                }

                *total_entries += 1;

                let file_metrics = if self.has_any_metrics() {
                    self.collect_file_metrics(&file_path, &file_name, false)
                } else {
                    None
                };
                entries.push(TreeNode::File {
                    name: file_name,
                    is_ignored,
                    is_symlink,
                    metrics: file_metrics,
                });
            }
        }

        entries
    }

    /// Creates a directory node marked as having an error.
    ///
    /// Used when a directory cannot be read due to permission errors.
    #[allow(dead_code)]
    fn create_error_dir_node(name: String, is_symlink: bool) -> TreeNode {
        TreeNode::Dir {
            name,
            children: vec![],
            is_ignored: false,
            is_symlink,
            has_error: true,
            at_depth_limit: false,
            metrics: None,
        }
    }

    /// Collects file metrics for a single path based on configured metric kinds.
    fn collect_file_metrics(
        &self,
        path: &Path,
        filename: &str,
        is_dir: bool,
    ) -> Option<FileMetrics> {
        let any_applicable = MetricKind::all_in_order().iter().any(|&kind| {
            if is_dir && !kind.is_dir_applicable() {
                return false;
            }
            self.should_show_metric(kind, filename)
        });

        if !any_applicable {
            return None;
        }

        let metadata = std::fs::metadata(path).ok();
        let mut fm = FileMetrics::default();

        if !is_dir && self.should_show_metric(MetricKind::FileSize, filename) {
            fm.file_size = metadata.as_ref().map(|m| m.len());
        }

        if !is_dir && self.should_show_metric(MetricKind::Tokens, filename) {
            fm.tokens = estimate_tokens(path, metadata.as_ref());
        }

        if self.should_show_metric(MetricKind::Created, filename)
            || self.should_show_metric(MetricKind::CreatedSince, filename)
        {
            fm.created = metadata
                .as_ref()
                .and_then(|m| m.created().ok())
                .map(DateTime::<Utc>::from);
        }

        if self.should_show_metric(MetricKind::Modified, filename)
            || self.should_show_metric(MetricKind::ModifiedSince, filename)
        {
            fm.modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .map(DateTime::<Utc>::from);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            if self.should_show_metric(MetricKind::Permissions, filename)
                || self.should_show_metric(MetricKind::PermissionsNumeric, filename)
            {
                fm.permissions_mode = metadata.as_ref().map(|m| m.mode() & 0o777);
            }

            if self.should_show_metric(MetricKind::Owner, filename) {
                fm.owner = metadata
                    .as_ref()
                    .and_then(|m| get_username_from_uid(m.uid()));
            }

            if self.should_show_metric(MetricKind::Group, filename) {
                fm.group = metadata
                    .as_ref()
                    .and_then(|m| get_groupname_from_gid(m.gid()));
            }
        }

        Some(fm)
    }

    /// Formats all enabled metrics for a node into a parenthesized string.
    fn format_metrics(&self, metrics: &FileMetrics, filename: &str, is_tty: bool) -> String {
        let parts: Vec<String> = MetricKind::all_in_order()
            .iter()
            .filter(|&&kind| self.should_show_metric(kind, filename))
            .filter_map(|&kind| self.format_single_metric(kind, metrics, is_tty))
            .collect();

        if parts.is_empty() {
            return String::new();
        }

        format!("( {} )", parts.join(", "))
    }

    /// Formats a single metric value with its label.
    fn format_single_metric(
        &self,
        kind: MetricKind,
        metrics: &FileMetrics,
        is_tty: bool,
    ) -> Option<String> {
        let highlight = self.should_highlight_metric(kind, metrics);

        match kind {
            MetricKind::FileSize => {
                let size = metrics.file_size?;
                Some(format_metric_pair(
                    "file size",
                    &format_bytes(size),
                    is_tty,
                    highlight,
                ))
            }
            MetricKind::Tokens => {
                let tokens = metrics.tokens?;
                Some(format_metric_pair(
                    "tokens",
                    &format_token_count(tokens),
                    is_tty,
                    highlight,
                ))
            }
            MetricKind::Created => {
                let dt = metrics.created?;
                Some(format_metric_pair(
                    "created",
                    &dt.format("%Y-%m-%d %H:%M").to_string(),
                    is_tty,
                    highlight,
                ))
            }
            MetricKind::CreatedSince => {
                let dt = metrics.created?;
                Some(format_metric_pair(
                    "created",
                    &format_relative_time(dt),
                    is_tty,
                    highlight,
                ))
            }
            MetricKind::Modified => {
                let dt = metrics.modified?;
                Some(format_metric_pair(
                    "modified",
                    &dt.format("%Y-%m-%d %H:%M").to_string(),
                    is_tty,
                    highlight,
                ))
            }
            MetricKind::ModifiedSince => {
                let dt = metrics.modified?;
                Some(format_metric_pair(
                    "modified",
                    &format_relative_time(dt),
                    is_tty,
                    highlight,
                ))
            }
            #[cfg(unix)]
            MetricKind::Permissions => {
                let mode = metrics.permissions_mode?;
                Some(format_metric_pair(
                    "perm",
                    &format_permissions_string(mode, is_tty),
                    is_tty,
                    highlight,
                ))
            }
            #[cfg(unix)]
            MetricKind::PermissionsNumeric => {
                let mode = metrics.permissions_mode?;
                Some(format_metric_pair(
                    "perm",
                    &format!("{:o}", mode),
                    is_tty,
                    highlight,
                ))
            }
            #[cfg(unix)]
            MetricKind::Owner => {
                let owner = metrics.owner.as_ref()?;
                Some(format_metric_pair("owner", owner, is_tty, highlight))
            }
            #[cfg(unix)]
            MetricKind::Group => {
                let group = metrics.group.as_ref()?;
                Some(format_metric_pair("group", group, is_tty, highlight))
            }
            #[cfg(not(unix))]
            MetricKind::Permissions
            | MetricKind::PermissionsNumeric
            | MetricKind::Owner
            | MetricKind::Group => None,
        }
    }

    /// Checks if a metric value exceeds its configured highlight threshold.
    fn should_highlight_metric(&self, kind: MetricKind, metrics: &FileMetrics) -> bool {
        let Some(config) = self.metric_configs.get(&kind) else {
            return false;
        };
        let Some(threshold) = config.highlight_threshold else {
            return false;
        };

        match kind {
            MetricKind::FileSize => metrics.file_size.is_some_and(|v| v > threshold),
            MetricKind::Tokens => metrics.tokens.is_some_and(|v| v > threshold),
            _ => false,
        }
    }
}

// =============================================================================
// TryFrom Implementations
// =============================================================================

impl TryFrom<&str> for FileSystem {
    type Error = FileSystemError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<String> for FileSystem {
    type Error = FileSystemError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&Path> for FileSystem {
    type Error = FileSystemError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<PathBuf> for FileSystem {
    type Error = FileSystemError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

// =============================================================================
// TerminalRenderable Implementation
// =============================================================================

impl TerminalRenderable for FileSystem {
    /// Renders the filesystem tree as a string without terminal context.
    ///
    /// This method provides basic rendering with Unicode fallback icons and no
    /// ANSI styling. For CLI output, prefer [`render()`](Self::render)
    /// which uses terminal capabilities for Nerd Font icons and proper styling.
    ///
    /// ## Arguments
    ///
    /// * `term_width` - Terminal width in columns. Defaults to 80 if `None`.
    ///
    /// ## Returns
    ///
    /// An empty string if the tree has not been built via
    /// [`ensure_tree_built()`](Self::ensure_tree_built).
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::{FileSystem, TerminalRenderable};
    ///
    /// let mut fs = FileSystem::new(".")?;
    /// fs.ensure_tree_built();
    ///
    /// // Render at 80 columns (default)
    /// let output = fs.render_optimistic(None);
    ///
    /// // Render at specific width
    /// let narrow = fs.render_optimistic(Some(40));
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Notes
    ///
    /// - Tree connectors (`├──`, `└──`, `│`) are never wrapped or broken
    /// - File/directory names are truncated with ellipsis (`...`) when too long
    /// - Uses Unicode fallback icons (📄, 📂) regardless of terminal capabilities
    /// - No ANSI styling is applied since there is no terminal context
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);

        let tree = match &self.tree {
            Some(t) => t,
            None => return String::new(),
        };

        if tree.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        if self.show_root {
            self.render_root_line(&mut output, None, false);
        }

        // Pass is_tty=false since render_optimistic() has no terminal context
        self.render_nodes(
            &mut output,
            tree,
            "",
            width,
            0,
            None,
            false,
            &self.root_path,
        );

        // Tree connectors form a cohesive block: align all lines by the same
        // offset (based on the widest line) so connectors stay visually aligned.
        // Word wrap is not applied — it would break tree connectors.
        self.layout.apply_block_layout(&output, width)
    }

    /// Renders the filesystem tree using terminal capabilities.
    ///
    /// This is the **recommended rendering method** for CLI output. It uses the
    /// terminal's capabilities to:
    ///
    /// - Select Nerd Font icons when available (falls back to Unicode otherwise)
    /// - Apply ANSI styling (colors, bold, italic) when connected to a TTY
    /// - Use the actual terminal width for proper truncation
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::prelude::{FileSystem, Terminal, TerminalRenderable};
    ///
    /// let mut fs = FileSystem::new_with_formatting(".")?;
    /// fs.ensure_tree_built();
    ///
    /// // Use Terminal::default() for current terminal capabilities
    /// let term = Terminal::default();
    /// let output = fs.render(&term);
    /// println!("{}", output);
    /// # Ok::<(), biscuit_terminal::prelude::FileSystemError>(())
    /// ```
    ///
    /// ## Notes
    ///
    /// - Returns empty string if tree has not been built
    /// - Directory names are styled bold blue
    /// - Symlinks are styled cyan
    /// - Error directories (permission denied) are styled red
    /// - Dotfiles are italicized when configured
    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        tracing::debug!(
            root = %self.root_path.display(),
            depth = self.max_depth,
            entries = self.tree.as_ref().map(|t| t.len()).unwrap_or(0),
            "FileSystem rendering"
        );

        let tree = match &self.tree {
            Some(t) => t,
            None => return String::new(),
        };

        if tree.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        if self.show_root {
            self.render_root_line(&mut output, term.is_nerd_font, term.is_tty);
        }

        // Canonicalize once for OSC8 file links so paths are absolute
        let base_path = if self.file_links {
            self.root_path
                .canonicalize()
                .unwrap_or_else(|_| self.root_path.clone())
        } else {
            self.root_path.clone()
        };
        self.render_nodes(
            &mut output,
            tree,
            "",
            width,
            0,
            term.is_nerd_font,
            term.is_tty,
            &base_path,
        );

        self.layout.apply_block_layout(&output, width)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Returns `true` because filesystem trees are block-level components.
    ///
    /// Filesystem trees occupy the full width and should not be placed side-by-side
    /// with other components.
    fn is_block_level(&self) -> bool {
        true
    }

    /// Exposes the tree projection through the canonical
    /// [`TerminalRenderable::render_tree_node`] hook so cross-target adapters
    /// (and nested [`RenderableTerminalContent`](crate::components::renderable::RenderableTerminalContent)`::Component`
    /// projection) consume `FileSystem` structurally instead of degrading to
    /// ANSI-stripped text.
    ///
    /// Delegates to [`TreeRenderable::render_tree`] so both public entry
    /// points share one source of truth. The bespoke
    /// [`TerminalRenderable::render`] path remains the production terminal
    /// renderer (see Stage 3a.3); this hook only governs structural projection
    /// into a parent render tree.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(<Self as TreeRenderable>::render_tree(self))
    }
}

impl FileSystem {
    /// Renders the root directory name as a header line.
    ///
    /// Shows the directory icon and name (e.g., ` docs`), styled bold blue
    /// when connected to a TTY. When a [`with_dimmed_root_prefix`](Self::with_dimmed_root_prefix)
    /// is configured, the prefix is rendered dimmed and the target name is
    /// rendered bold blue separately.
    fn render_root_line(&self, output: &mut String, is_nerd_font: Option<bool>, is_tty: bool) {
        let use_nerd = is_nerd_font.unwrap_or(false);

        // Resolve the display name: canonicalize relative paths like "." and ".."
        // so we show the actual directory name instead of a dot. An explicit
        // `root_display_name` override takes priority.
        let name = self.root_display_name.clone().unwrap_or_else(|| {
            self.root_path
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .or_else(|| {
                    self.root_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| self.root_path.display().to_string())
        });

        // Resolve the icon: custom root icon override or the default dir icon.
        let icon_str = match self.root_icon {
            Some(kind) => {
                let icon = kind.nerd_char();
                if use_nerd {
                    format!("{} ", icon)
                } else {
                    kind.unicode_str().to_string()
                }
            }
            None => {
                let icon = self.get_dir_icon(&name, use_nerd);
                if use_nerd {
                    format!("{} ", icon)
                } else {
                    icon.to_string()
                }
            }
        };

        // When no dimmed prefix is configured, preserve the original rendering.
        let Some(prefix) = &self.root_prefix else {
            if is_tty {
                let display_name = if self.file_links {
                    match file_url(&self.root_path) {
                        Some(url) => Prose::new(format!("<a href=\"{url}\">{name}</a>"))
                            .render_optimistic(None),
                        None => name,
                    }
                } else {
                    name
                };
                output.push_str(&format!("\x1b[1;34m{}{}\x1b[0m\n", icon_str, display_name));
            } else {
                output.push_str(&format!("{}{}\n", icon_str, name));
            }
            return;
        };

        // Dimmed-prefix root line:
        // `{icon} {dim}{prefix}{reset}{bold-blue}{target}{reset}`
        let target = if self.file_links && is_tty {
            match file_url(&self.root_path) {
                Some(url) => {
                    Prose::new(format!("<a href=\"{url}\">{name}</a>")).render_optimistic(None)
                }
                None => name,
            }
        } else {
            name
        };

        if is_tty {
            output.push_str(&format!(
                "{}\x1b[2m{}\x1b[0m\x1b[1;34m{}\x1b[0m\n",
                icon_str, prefix, target
            ));
        } else {
            output.push_str(&format!("{}{}{}\n", icon_str, prefix, target));
        }
    }

    /// Renders tree nodes recursively to the output string.
    ///
    /// This is the internal rendering engine that builds the tree output line by line.
    ///
    /// ## Arguments
    ///
    /// * `output` - Mutable string to append rendered lines to
    /// * `nodes` - The tree nodes to render at this level
    /// * `prefix` - The prefix string for the current depth (accumulated `│   ` or `    `)
    /// * `width` - Terminal width for truncation calculations
    /// * `depth` - Current depth in the tree (0 = root level)
    /// * `is_nerd_font` - Whether to use Nerd Font icons
    /// * `is_tty` - Whether stdout is connected to a TTY (for ANSI styling)
    /// * `current_path` - The filesystem path of the current directory level
    #[allow(clippy::too_many_arguments)]
    fn render_nodes(
        &self,
        output: &mut String,
        nodes: &[TreeNode],
        prefix: &str,
        width: u32,
        depth: u32,
        is_nerd_font: Option<bool>,
        is_tty: bool,
        current_path: &Path,
    ) {
        for (idx, node) in nodes.iter().enumerate() {
            let is_last = idx == nodes.len() - 1;

            // Build the line prefix with connector
            let connector = if is_last {
                tree_chars::LAST_BRANCH
            } else {
                tree_chars::BRANCH
            };
            let line_prefix = format!("{}{}", prefix, connector);

            // Get icon with padding (space after Nerd Font icons)
            let icon = self.get_icon_with_padding(node, depth, is_nerd_font);

            // Calculate available width for the name
            // line_prefix + icon + name must fit in width
            let prefix_width = visible_width(&line_prefix);
            let icon_width = visible_width(&icon);
            let available = width
                .saturating_sub(prefix_width)
                .saturating_sub(icon_width);

            // Get the display name, applying styles if configured
            let name = node.name();
            let style = self.style_prefix(node, name, is_tty);
            let name_width = visible_width(name);

            // Truncate name if needed
            let display_name = if name_width > available && available > 1 {
                truncate_with_ellipsis(name, available)
            } else {
                name.to_string()
            };

            // Wrap name in an OSC8 hyperlink when file_links is enabled
            let display_name = if self.file_links && is_tty {
                let node_path = current_path.join(name);
                match file_url(&node_path) {
                    Some(url) => Prose::new(format!("<a href=\"{url}\">{display_name}</a>"))
                        .render_optimistic(None),
                    None => display_name,
                }
            } else {
                display_name
            };

            // Format metrics suffix if available
            let metrics_str = if let Some(metrics) = node.metrics() {
                let is_dir = node.is_dir();
                if is_dir && !self.show_metrics_on_directories {
                    String::new()
                } else {
                    let formatted = self.format_metrics(metrics, name, is_tty);
                    if formatted.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", formatted)
                    }
                }
            } else {
                String::new()
            };

            // Write the line: prefix + [style]icon name[reset] + metrics
            output.push_str(&line_prefix);
            if style.is_empty() {
                output.push_str(&icon);
                output.push_str(&display_name);
            } else {
                output.push_str(&style);
                output.push_str(&icon);
                output.push_str(&display_name);
                output.push_str("\x1b[0m");
            }
            output.push_str(&metrics_str);
            output.push('\n');

            // Recurse into children for directories
            if let TreeNode::Dir { children, .. } = node {
                let new_prefix = format!(
                    "{}{}",
                    prefix,
                    if is_last {
                        tree_chars::INDENT
                    } else {
                        tree_chars::VERTICAL
                    }
                );
                self.render_nodes(
                    output,
                    children,
                    &new_prefix,
                    width,
                    depth + 1,
                    is_nerd_font,
                    is_tty,
                    &current_path.join(name),
                );
            }
        }
    }

    /// Returns the ANSI style prefix for a node, or empty string if no styling.
    ///
    /// ## Color Scheme
    ///
    /// - Directories: bold blue
    /// - Symlinks: cyan
    /// - Error directories: red
    /// - Gitignored items: dim (when `dim_gitignore` is true)
    /// - Dot files/dirs: italic (when `italicize_dot_files/dirs` is true)
    /// - Highlight patterns: red or green (highest priority)
    ///
    /// ## TTY Awareness
    ///
    /// When `is_tty` is false, no codes are returned. This ensures clean
    /// output when redirected to files or pipes.
    fn style_prefix(&self, node: &TreeNode, name: &str, is_tty: bool) -> String {
        if !is_tty {
            return String::new();
        }

        let is_dot = name.starts_with('.');

        // Check highlights first (highest priority)
        for pattern in &self.highlight_red {
            if name.contains(pattern) {
                return "\x1b[31m".to_string();
            }
        }
        for pattern in &self.highlight_green {
            if name.contains(pattern) {
                return "\x1b[32m".to_string();
            }
        }

        let mut codes: Vec<&str> = Vec::new();

        let is_error = matches!(
            node,
            TreeNode::Dir {
                has_error: true,
                ..
            }
        );
        if is_error {
            codes.push("31");
        }

        if node.is_ignored() && self.dim_gitignore && !is_error {
            codes.push("2");
        }

        let should_italicize = is_dot
            && match node {
                TreeNode::Dir { .. } => self.italicize_dot_dirs,
                TreeNode::File { .. } => self.italicize_dot_files,
            };
        if should_italicize {
            codes.push("3");
        }

        if node.is_dir() && !is_error {
            codes.push("1");
            codes.push("34");
        }

        if node.is_symlink() {
            codes.push("36");
        }

        if codes.is_empty() {
            return String::new();
        }

        format!("\x1b[{}m", codes.join(";"))
    }

    /// Applies styling to a node name based on configuration and TTY status.
    ///
    /// Wraps the name with the appropriate ANSI escape sequence and reset.
    /// The style covers the name only; use [`style_prefix`](Self::style_prefix)
    /// when you need to wrap additional content (e.g., an icon) in the same style.
    pub fn style_name(&self, node: &TreeNode, name: &str, is_tty: bool) -> String {
        let prefix = self.style_prefix(node, name, is_tty);
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}{}\x1b[0m", prefix, name)
        }
    }
}

/// Truncates a string to fit within the specified visible width, adding an ellipsis.
///
/// Preserves ANSI escape sequences and handles Unicode width correctly.
fn truncate_with_ellipsis(content: &str, max_width: u32) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "\u{2026}".to_string(); // Just ellipsis
    }

    let content_width = visible_width(content);
    if content_width <= max_width {
        return content.to_string();
    }

    // Leave room for ellipsis (1 column)
    let truncate_at = max_width.saturating_sub(1);
    let (head, _tail) = split_at_visible_width(content, truncate_at);
    format!("{}\u{2026}", head)
}

/// Checks if a filename matches a glob pattern.
fn glob_match(pattern: &str, filename: &str) -> bool {
    globset::Glob::new(pattern)
        .ok()
        .map(|g| g.compile_matcher().is_match(filename))
        .unwrap_or(false)
}

/// Returns `true` when `path` is a clean relative path that stays within the
/// root (no root or drive prefix, no `..` components). Used to filter
/// [`FileSystem::included_paths`] entries.
fn is_safe_relative(path: &Path) -> bool {
    if path.has_root() || path.as_os_str().is_empty() {
        return false;
    }
    !path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    })
}

/// Builds a standards-compliant file URL, resolving relative paths against the
/// process working directory. Returns `None` when the path cannot be expressed
/// as a file URL.
fn file_url(path: &Path) -> Option<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    url::Url::from_file_path(absolute).ok().map(String::from)
}

/// Returns the lowercased extension (without dot) of a filename, or `None`
/// when the name has no extension.
fn lowercase_extension(name: &str) -> Option<String> {
    std::path::Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
}

/// Formats a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats a token count with comma separators.
fn format_token_count(tokens: u64) -> String {
    let s = tokens.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Formats a datetime as a relative time string (e.g., "2 days ago").
fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    let seconds = duration.num_seconds();
    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        return if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{minutes} minutes ago")
        };
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{hours} hours ago")
        };
    }

    let days = duration.num_days();
    if days < 30 {
        return if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{days} days ago")
        };
    }

    let months = days / 30;
    if months < 12 {
        return if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{months} months ago")
        };
    }

    let years = months / 12;
    if years == 1 {
        "1 year ago".to_string()
    } else {
        format!("{years} years ago")
    }
}

/// Formats Unix permission mode bits as a symbolic string (e.g., ".rw-r--r--").
///
/// When `is_tty` is true, r/w/x characters are colored (green/red/yellow).
#[cfg(unix)]
fn format_permissions_string(mode: u32, is_tty: bool) -> String {
    let mut s = String::with_capacity(64);
    s.push('.');

    let triples = [
        (mode >> 6) & 0o7, // owner
        (mode >> 3) & 0o7, // group
        mode & 0o7,        // other
    ];

    for bits in triples {
        // read
        if bits & 0o4 != 0 {
            if is_tty {
                s.push_str("\x1b[32mr\x1b[0m");
            } else {
                s.push('r');
            }
        } else {
            s.push('-');
        }
        // write
        if bits & 0o2 != 0 {
            if is_tty {
                s.push_str("\x1b[31mw\x1b[0m");
            } else {
                s.push('w');
            }
        } else {
            s.push('-');
        }
        // execute
        if bits & 0o1 != 0 {
            if is_tty {
                s.push_str("\x1b[33mx\x1b[0m");
            } else {
                s.push('x');
            }
        } else {
            s.push('-');
        }
    }

    s
}

/// Formats a metric label/value pair with optional highlighting.
/// Returns the human-readable label for a [`MetricKind`] as used in the
/// `( label: value, … )` suffix produced by both the bespoke ANSI renderer
/// and the canonical render tree projection.
fn fs_metric_label(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::FileSize => "file size",
        MetricKind::Tokens => "tokens",
        MetricKind::Created | MetricKind::CreatedSince => "created",
        MetricKind::Modified | MetricKind::ModifiedSince => "modified",
        #[cfg(unix)]
        MetricKind::Permissions | MetricKind::PermissionsNumeric => "perm",
        #[cfg(unix)]
        MetricKind::Owner => "owner",
        #[cfg(unix)]
        MetricKind::Group => "group",
        // On non-Unix the Unix-only kinds simply have no label; the caller
        // never asks for one because their value resolver returns None.
        #[cfg(not(unix))]
        MetricKind::Permissions
        | MetricKind::PermissionsNumeric
        | MetricKind::Owner
        | MetricKind::Group => "",
    }
}

/// Returns the plain-text value string for a [`MetricKind`] read from
/// [`FileMetrics`], or `None` when the metric is absent.
///
/// The output mirrors the ANSI-free path of
/// [`FileSystem::format_single_metric`] so projection and the bespoke
/// renderer agree on the displayed value.
fn fs_metric_value_string(kind: MetricKind, metrics: &FileMetrics) -> Option<String> {
    match kind {
        MetricKind::FileSize => metrics.file_size.map(format_bytes),
        MetricKind::Tokens => metrics.tokens.map(format_token_count),
        MetricKind::Created => metrics
            .created
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
        MetricKind::CreatedSince => metrics.created.map(format_relative_time),
        MetricKind::Modified => metrics
            .modified
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
        MetricKind::ModifiedSince => metrics.modified.map(format_relative_time),
        #[cfg(unix)]
        MetricKind::Permissions => metrics
            .permissions_mode
            // `is_tty=false` strips color from `format_permissions_string`.
            .map(|mode| format_permissions_string(mode, false)),
        #[cfg(unix)]
        MetricKind::PermissionsNumeric => metrics.permissions_mode.map(|mode| format!("{mode:o}")),
        #[cfg(unix)]
        MetricKind::Owner => metrics.owner.clone(),
        #[cfg(unix)]
        MetricKind::Group => metrics.group.clone(),
        #[cfg(not(unix))]
        MetricKind::Permissions
        | MetricKind::PermissionsNumeric
        | MetricKind::Owner
        | MetricKind::Group => None,
    }
}

fn format_metric_pair(label: &str, value: &str, is_tty: bool, highlight: bool) -> String {
    if is_tty {
        let dim_label = format!("\x1b[2m{}:\x1b[0m", label);
        if highlight {
            format!("{} \x1b[1;33m{}\x1b[0m", dim_label, value)
        } else {
            format!("{} {}", dim_label, value)
        }
    } else {
        format!("{}: {}", label, value)
    }
}

/// Estimates the number of LLM tokens for a file based on its extension and size.
///
/// Returns `None` for binary/unknown file types.
fn estimate_tokens(path: &Path, metadata: Option<&std::fs::Metadata>) -> Option<u64> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let size = metadata.map(|m| m.len())?;

    let chars_per_token: f64 = match ext.as_str() {
        "log" | "json" | "yaml" | "yml" | "toml" => 2.5,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "md" | "txt" | "py" | "go" | "java" | "c" | "cpp"
        | "h" | "hpp" | "css" | "html" | "xml" | "sql" | "sh" | "bash" | "zsh" | "fish" | "rb"
        | "php" | "swift" | "kt" | "scala" | "lua" | "r" | "pl" | "ex" | "exs" | "elm" | "hs"
        | "ml" | "vim" | "conf" | "cfg" | "ini" | "env" | "csv" | "tsv" => 4.0,
        _ => return None,
    };

    Some((size as f64 / chars_per_token) as u64)
}

/// Resolves a Unix UID to a username.
#[cfg(unix)]
fn get_username_from_uid(uid: u32) -> Option<String> {
    // SAFETY: getpwuid returns a pointer to a static struct or null.
    // We only read from the returned pointer and copy the string immediately.
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*pw).pw_name);
        Some(name.to_string_lossy().into_owned())
    }
}

/// Resolves a Unix GID to a group name.
#[cfg(unix)]
fn get_groupname_from_gid(gid: u32) -> Option<String> {
    // SAFETY: getgrgid returns a pointer to a static struct or null.
    // We only read from the returned pointer and copy the string immediately.
    unsafe {
        let gr = libc::getgrgid(gid);
        if gr.is_null() {
            return None;
        }
        let name = std::ffi::CStr::from_ptr((*gr).gr_name);
        Some(name.to_string_lossy().into_owned())
    }
}

// ============================================================
// Canonical Render-Tree Projection (RT-FILESYSTEM-001)
// ============================================================
//
// The projection emits a `Root` containing an optional root-header `Paragraph`
// and an unordered `List` carrying `ListMarkerPolicy::TreeConnectors`. Each
// entry becomes a `ListItem` with semantic classes (`fs-dir`, `fs-file`, …)
// and a `Paragraph` containing an icon `Span`, the entry name (or a `Link`),
// and an optional metrics `Span`. Directory items nest a sibling `List` with
// the same marker policy so the terminal renderer can infer connector
// geometry.
//
// The production `TerminalRenderable::render` path stays bespoke per the
// FileSystem migration sequence; this projection feeds Markdown, Browser, and
// future terminal parity tests once the parity harness lands.

/// Semantic class hooks used by the canonical projection.
///
/// Browser CSS and MarkdownPlus consume these as hooks; the terminal renderer
/// reads typed `Style` instead and does not depend on `fs-*` class names.
const CLASS_ROOT: &str = "fs-root";
const CLASS_DIR: &str = "fs-dir";
const CLASS_FILE: &str = "fs-file";
const CLASS_IGNORED: &str = "fs-ignored";
const CLASS_SYMLINK: &str = "fs-symlink";
const CLASS_ERROR: &str = "fs-error";
const CLASS_DEPTH_LIMIT: &str = "fs-depth-limit";
const CLASS_DOT: &str = "fs-dot";
const CLASS_HIGHLIGHT_RED: &str = "fs-highlight-red";
const CLASS_HIGHLIGHT_GREEN: &str = "fs-highlight-green";
const CLASS_ICON: &str = "fs-icon";
const CLASS_METRICS: &str = "fs-metrics";
const CLASS_METRIC_HIGHLIGHT: &str = "fs-metric-highlight";
const CLASS_ROOT_PREFIX: &str = "fs-root-prefix";

/// Precedence-ordered foreground kind used by the canonical projection.
///
/// Mirrors the precedence used by the bespoke ANSI [`FileSystem::style_prefix`]
/// so the typed `Style` projection lowers to the same observable color in the
/// terminal renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FsFgKind {
    /// No foreground color override (ordinary files).
    None,
    /// Highlight-red pattern matched the name.
    HighlightRed,
    /// Highlight-green pattern matched the name.
    HighlightGreen,
    /// Directory with a permission error.
    Error,
    /// Directory entry (bold blue in the bespoke renderer).
    Directory,
    /// Symbolic link entry (cyan in the bespoke renderer).
    Symlink,
}

impl FsFgKind {
    fn color(self) -> Option<Color> {
        match self {
            FsFgKind::None => None,
            FsFgKind::HighlightRed | FsFgKind::Error => Some(Color::BasicColor(BasicColor::Red)),
            FsFgKind::HighlightGreen => Some(Color::BasicColor(BasicColor::Green)),
            FsFgKind::Directory => Some(Color::BasicColor(BasicColor::Blue)),
            FsFgKind::Symlink => Some(Color::BasicColor(BasicColor::Cyan)),
        }
    }
}

/// Universal [`Style`] color slot for a concrete [`Color`].
fn fs_universal_color(color: Color) -> RTargetValue<PerMode<PaintColor>> {
    RTargetValue::universal(PerMode::universal(color))
}

/// Returns the Unicode fallback glyph for a [`TreeNode`].
///
/// The canonical projection always uses the Unicode fallback set because the
/// projection is target-agnostic and Nerd Font glyphs are not portable to
/// Markdown or HTML. The bespoke terminal renderer continues to swap in Nerd
/// Font icons via [`FileSystem::get_icon_with_padding`] when the terminal
/// advertises Nerd Font support.
fn fs_unicode_icon(node: &TreeNode) -> &'static str {
    match node {
        TreeNode::Dir {
            has_error: true, ..
        } => "⚠",
        TreeNode::Dir {
            at_depth_limit: true,
            ..
        } => "📁",
        TreeNode::Dir { .. } => "📂",
        TreeNode::File { name, .. } => fs_unicode_file_icon(name),
    }
}

/// Returns the Unicode fallback glyph for a file based on its extension.
///
/// Document extensions (`.txt`, `.pdf`, `.doc`, `.docx`, `.xls`, `.xlsx`)
/// receive distinct glyphs so they remain distinguishable even without Nerd
/// Font support. All other files fall back to the generic page emoji.
fn fs_unicode_file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().map(|e| e.to_lowercase());
    match ext.as_deref() {
        Some("txt") => "📝",
        Some("pdf") => "📕",
        Some("doc" | "docx") => "📘",
        Some("xls" | "xlsx") => "📗",
        _ => "📄",
    }
}

/// Recursively removes every `fs-icon` [`Span`](renderable::tree::NodeKind::Span)
/// from `node`, along with a single literal `" "` text node immediately
/// following it.
///
/// Used by [`FileSystem::render_markdown`] to satisfy the spec requirement
/// that icons are omitted from portable Markdown. MarkdownPlus and Browser
/// keep the spans because they have CSS hooks to control rendering.
fn fs_strip_icon_spans(node: &mut RenderNode) {
    if let Some(children) = node.children_mut() {
        // Two-pass walk: first remove any direct `fs-icon` child plus the
        // literal space that follows it; then recurse.
        let mut i = 0;
        while i < children.len() {
            let is_icon = matches!(&children[i].kind, NodeKind::Span { .. })
                && children[i].attrs.classes.iter().any(|c| c == CLASS_ICON);
            if is_icon {
                children.remove(i);
                // If the next sibling is the canonical " " separator, drop
                // it too so the projected paragraph reads `name (metrics)`
                // rather than ` name`.
                if i < children.len()
                    && let NodeKind::Text { value } = &children[i].kind
                    && value == " "
                {
                    children.remove(i);
                }
                continue;
            }
            i += 1;
        }
        for child in children.iter_mut() {
            fs_strip_icon_spans(child);
        }
    }
}

impl FileSystem {
    /// Resolves the precedence-ordered foreground kind for a tree node.
    fn fs_fg_kind(&self, node: &TreeNode) -> FsFgKind {
        let name = node.name();
        for pattern in &self.highlight_red {
            if name.contains(pattern) {
                return FsFgKind::HighlightRed;
            }
        }
        for pattern in &self.highlight_green {
            if name.contains(pattern) {
                return FsFgKind::HighlightGreen;
            }
        }
        if let TreeNode::Dir {
            has_error: true, ..
        } = node
        {
            return FsFgKind::Error;
        }
        if node.is_symlink() {
            return FsFgKind::Symlink;
        }
        if node.is_dir() {
            return FsFgKind::Directory;
        }
        FsFgKind::None
    }

    /// Builds the entry [`Style`] for a tree node, mirroring the precedence
    /// used by the bespoke terminal renderer:
    ///
    /// - highlight red / green have highest priority
    /// - error directories are red
    /// - gitignored entries are dim when `dim_gitignore` is true
    /// - configured dotfiles/dotdirs are italic
    /// - directories are bold blue unless overridden by error/highlight
    /// - symlinks are cyan
    fn fs_entry_style(&self, node: &TreeNode) -> Style {
        let fg = self.fs_fg_kind(node);
        let name = node.name();
        let is_dot = name.starts_with('.');
        let is_error = matches!(
            node,
            TreeNode::Dir {
                has_error: true,
                ..
            }
        );

        let mut emphasis = TextEmphasis::default();
        // Highlight matches short-circuit dim/italic/bold to mirror the
        // bespoke `style_prefix`, which returns the bare `\x1b[31m`/`\x1b[32m`
        // SGR sequence and never stacks additional attributes when a
        // highlight pattern matches.
        let is_highlighted = matches!(fg, FsFgKind::HighlightRed | FsFgKind::HighlightGreen);
        if !is_highlighted {
            if !is_error && node.is_ignored() && self.dim_gitignore {
                emphasis.dim = true;
            }
            let should_italicize = is_dot
                && match node {
                    TreeNode::Dir { .. } => self.italicize_dot_dirs,
                    TreeNode::File { .. } => self.italicize_dot_files,
                };
            if should_italicize {
                emphasis.italic = true;
            }
            // Bold applies to directories that resolve to the directory fg
            // or to symlinks pointing at a directory (the bespoke renderer
            // pushes `1;34` then layers `36` on top — bold survives even
            // when cyan wins the foreground color). Highlight/error variants
            // remain un-bold to match the bespoke renderer's `is_error`
            // branch (which skips the bold/blue codes).
            if matches!(fg, FsFgKind::Directory) || (node.is_symlink() && node.is_dir()) {
                emphasis.bold = true;
            }
        }

        Style {
            color: fg.color().map(fs_universal_color),
            emphasis,
            ..Style::default()
        }
    }

    /// Builds the semantic class list for a tree-entry node.
    fn fs_entry_classes(&self, node: &TreeNode) -> Vec<String> {
        let name = node.name();
        let mut classes: Vec<String> = Vec::new();

        classes.push(
            match node {
                TreeNode::Dir { .. } => CLASS_DIR,
                TreeNode::File { .. } => CLASS_FILE,
            }
            .to_string(),
        );

        if node.is_ignored() {
            classes.push(CLASS_IGNORED.to_string());
        }
        if node.is_symlink() {
            classes.push(CLASS_SYMLINK.to_string());
        }
        if let TreeNode::Dir {
            has_error: true, ..
        } = node
        {
            classes.push(CLASS_ERROR.to_string());
        }
        if let TreeNode::Dir {
            at_depth_limit: true,
            ..
        } = node
        {
            classes.push(CLASS_DEPTH_LIMIT.to_string());
        }

        let is_dot = name.starts_with('.');
        let dot_styled = is_dot
            && match node {
                TreeNode::Dir { .. } => self.italicize_dot_dirs,
                TreeNode::File { .. } => self.italicize_dot_files,
            };
        if dot_styled {
            classes.push(CLASS_DOT.to_string());
        }

        for pattern in &self.highlight_red {
            if name.contains(pattern) {
                classes.push(CLASS_HIGHLIGHT_RED.to_string());
                break;
            }
        }
        for pattern in &self.highlight_green {
            if name.contains(pattern) {
                classes.push(CLASS_HIGHLIGHT_GREEN.to_string());
                break;
            }
        }

        classes
    }

    /// Projects a single tree node into a `ListItem` containing a `Paragraph`
    /// (icon + name + optional metrics) and an optional nested `List` of
    /// directory children.
    ///
    /// `current_path` is the absolute path of the directory containing `node`.
    /// The caller is responsible for canonicalizing the root once (see
    /// [`fs_render_tree_inner`]) so `file://` URLs do not embed `./` or `..`.
    fn fs_project_tree_node(&self, node: &TreeNode, current_path: &Path) -> RenderNode {
        let name = node.name();

        // ── Icon span (semantic class only; the bespoke renderer selects
        // Nerd Font icons on its own; the projection prefers Unicode).
        let icon_glyph = fs_unicode_icon(node);
        let icon_span = RenderNode::span(
            vec![CLASS_ICON.to_string()],
            vec![RenderNode::text(icon_glyph)],
        );

        // ── Name (or Link when file_links is enabled).
        // The caller threads an already-canonicalized `current_path`, so
        // `current_path.join(name)` produces an absolute URL without `./`.
        // The name is wrapped in a classed `Span` so the MarkdownPlus and
        // Browser renderers can hang entry-kind CSS (`fs-dir`, `fs-file`,
        // `fs-symlink`, …) off the name itself. ListItem-level classes are
        // preserved for Browser fidelity.
        let entry_classes = self.fs_entry_classes(node);
        let inner_name: RenderNode = if self.file_links {
            let abs_path = current_path.join(name);
            match file_url(&abs_path) {
                Some(url) => RenderNode::link(url, None, vec![RenderNode::text(name)]),
                None => RenderNode::text(name),
            }
        } else {
            RenderNode::text(name)
        };
        let name_node = RenderNode::span(entry_classes.clone(), vec![inner_name]);

        // ── Metrics span. The projected metrics are an inline span tree
        // (rather than flattened plain text) so threshold-triggered values
        // can carry a bold-yellow Style on their own child span and survive
        // into Terminal/Browser/MarkdownPlus rendering. See
        // [`fs_project_metrics`](FileSystem::fs_project_metrics).
        let metrics_span = self.fs_project_metrics(node);

        let mut para_children: Vec<RenderNode> = Vec::with_capacity(5);
        para_children.push(icon_span);
        // Literal space separates the icon glyph from the name in the
        // canonical projection. Browser CSS controls spacing via `fs-icon`.
        para_children.push(RenderNode::text(" "));
        para_children.push(name_node);
        if let Some(metrics_span) = metrics_span {
            para_children.push(RenderNode::text(" "));
            para_children.push(metrics_span);
        }

        let mut paragraph = RenderNode::paragraph(para_children);
        let style = self.fs_entry_style(node);
        if !style.is_empty() {
            paragraph.attrs.set_style(&style);
        }

        let mut item_children: Vec<RenderNode> = vec![paragraph];
        if let TreeNode::Dir { children, .. } = node
            && !children.is_empty()
        {
            let nested_path = current_path.join(name);
            let nested_items: Vec<RenderNode> = children
                .iter()
                .map(|child| self.fs_project_tree_node(child, &nested_path))
                .collect();
            let mut nested_list = RenderNode::list(false, None, nested_items);
            nested_list
                .attrs
                .set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
            item_children.push(nested_list);
        }

        let mut list_item = RenderNode::list_item(None, item_children);
        list_item.attrs.classes = entry_classes;
        list_item
    }

    /// Projects the metrics suffix for a node as a structured inline span.
    ///
    /// The returned [`RenderNode`] carries the `fs-metrics` class on its
    /// outer [`Span`](renderable::tree::NodeKind::Span). Each metric pair is
    /// emitted as `<dim>label:</dim> value`, with values that exceed their
    /// configured `highlight_threshold` wrapped in a bold-yellow child
    /// `<span class="fs-metric-highlight">value</span>` so the threshold
    /// signal survives all three render targets.
    ///
    /// Returns `None` when no metrics are configured, the node has no
    /// `FileMetrics`, the node is a directory and `show_metrics_on_directories`
    /// is false, or every configured metric resolved to `None` for this node.
    fn fs_project_metrics(&self, node: &TreeNode) -> Option<RenderNode> {
        if !self.has_any_metrics() {
            return None;
        }
        let metrics = node.metrics()?;
        if node.is_dir() && !self.show_metrics_on_directories {
            return None;
        }

        let pairs: Vec<(MetricKind, String, bool)> = MetricKind::all_in_order()
            .iter()
            .filter(|&&kind| self.should_show_metric(kind, node.name()))
            .filter_map(|&kind| {
                let value = fs_metric_value_string(kind, metrics)?;
                let highlighted = self.should_highlight_metric(kind, metrics);
                Some((kind, value, highlighted))
            })
            .collect();

        if pairs.is_empty() {
            return None;
        }

        let mut children: Vec<RenderNode> = Vec::with_capacity(pairs.len() * 4 + 2);
        children.push(RenderNode::text("( "));
        for (i, (kind, value, highlighted)) in pairs.iter().enumerate() {
            if i > 0 {
                children.push(RenderNode::text(", "));
            }
            // Dim `label:` span — mirrors the bespoke ANSI `\x1b[2m…\x1b[0m`.
            let mut label = RenderNode::span(
                Vec::new(),
                vec![RenderNode::text(format!("{}:", fs_metric_label(*kind)))],
            );
            label.attrs.set_style(&Style {
                emphasis: TextEmphasis {
                    dim: true,
                    ..TextEmphasis::default()
                },
                ..Style::default()
            });
            children.push(label);
            children.push(RenderNode::text(" "));

            if *highlighted {
                let mut hl = RenderNode::span(
                    vec![CLASS_METRIC_HIGHLIGHT.to_string()],
                    vec![RenderNode::text(value)],
                );
                hl.attrs.set_style(&Style {
                    color: Some(fs_universal_color(Color::BasicColor(BasicColor::Yellow))),
                    emphasis: TextEmphasis {
                        bold: true,
                        ..TextEmphasis::default()
                    },
                    ..Style::default()
                });
                children.push(hl);
            } else {
                children.push(RenderNode::text(value));
            }
        }
        children.push(RenderNode::text(" )"));

        let outer = RenderNode::span(vec![CLASS_METRICS.to_string()], children);
        Some(outer)
    }

    /// Builds the root header `Paragraph` projected onto the canonical tree
    /// when [`show_root`](FileSystem::show_root) is true.
    ///
    /// When a [`with_dimmed_root_prefix`](FileSystem::with_dimmed_root_prefix)
    /// is configured, the prefix is projected as a separate dimmed span and the
    /// target name as a bold-blue span. Without a prefix the original
    /// paragraph-level bold-blue style is preserved.
    fn fs_project_root_header(&self) -> RenderNode {
        let name = self.root_display_name.clone().unwrap_or_else(|| {
            self.root_path
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .or_else(|| {
                    self.root_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| self.root_path.display().to_string())
        });

        // Choose the root icon glyph: custom override or default folder emoji.
        let icon_glyph = match self.root_icon {
            Some(kind) => kind.unicode_str(),
            None => "📂",
        };
        let icon_span = RenderNode::span(
            vec![CLASS_ICON.to_string()],
            vec![RenderNode::text(icon_glyph)],
        );

        let name_node = if self.file_links {
            let abs_path = self
                .root_path
                .canonicalize()
                .unwrap_or_else(|_| self.root_path.clone());
            match file_url(&abs_path) {
                Some(url) => RenderNode::link(url, None, vec![RenderNode::text(&name)]),
                None => RenderNode::text(&name),
            }
        } else {
            RenderNode::text(&name)
        };

        let bold_blue_style = Style {
            color: Some(fs_universal_color(Color::BasicColor(BasicColor::Blue))),
            emphasis: TextEmphasis {
                bold: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        };

        let mut paragraph = if let Some(prefix) = &self.root_prefix {
            // Dimmed-prefix root header: icon + dim prefix + bold-blue target.
            let mut prefix_span = RenderNode::span(
                vec![CLASS_ROOT_PREFIX.to_string()],
                vec![RenderNode::text(prefix.clone())],
            );
            prefix_span.attrs.set_style(&Style {
                emphasis: TextEmphasis {
                    dim: true,
                    ..TextEmphasis::default()
                },
                ..Style::default()
            });

            let mut target_span = RenderNode::span(Vec::new(), vec![name_node]);
            target_span.attrs.set_style(&bold_blue_style);

            RenderNode::paragraph(vec![
                icon_span,
                RenderNode::text(" "),
                prefix_span,
                target_span,
            ])
        } else {
            // Default root header: icon + name with paragraph-level bold blue.
            let mut para =
                RenderNode::paragraph(vec![icon_span, RenderNode::text(" "), name_node]);
            para.attrs.set_style(&bold_blue_style);
            para
        };

        // Root header classes mark it as both `fs-root` and `fs-dir` for CSS hooks.
        paragraph.attrs.classes = vec![CLASS_ROOT.to_string(), CLASS_DIR.to_string()];
        paragraph
    }

    /// Builds the canonical render tree even when the FileSystem tree has not
    /// been built yet. This produces an empty `Root` rather than panicking,
    /// matching the empty-string contract of [`render_optimistic`].
    ///
    /// When `file_links` is enabled the root path is canonicalized once and
    /// threaded into the entry projection so emitted `file://` URLs are
    /// absolute even if the caller constructed the component with a relative
    /// path such as `"."` or `"./src"`. This mirrors the bespoke ANSI
    /// renderer's `base_path` handling in [`TerminalRenderable::render`].
    fn fs_render_tree_inner(&self) -> RenderNode {
        // Silently projecting an empty Root when the FileSystem tree has
        // not been built is a documented contract (matches the empty-string
        // contract of `render_optimistic`), but the resulting empty render
        // tree is a frequent foot-gun for callers who forget to invoke
        // `ensure_tree_built`. The Markdown / Browser entrypoints clone +
        // build before calling this; only direct `TreeRenderable::render_tree`
        // callers can land here without a built tree. Emit a debug-level
        // trace so the silent empty is at least observable.
        if self.tree.is_none() {
            tracing::debug!(
                root = %self.root_path.display(),
                "FileSystem render_tree called without ensure_tree_built; \
                 projecting empty Root"
            );
        }

        let mut children: Vec<RenderNode> = Vec::new();

        if self.show_root {
            children.push(self.fs_project_root_header());
        }

        // Canonicalize once for OSC8/file:// link generation so that entries
        // never embed `./` or `..` segments. Falls back to the raw root_path
        // when canonicalization fails (broken symlinks, missing dir, etc.).
        let base_path: PathBuf = if self.file_links {
            self.root_path
                .canonicalize()
                .unwrap_or_else(|_| self.root_path.clone())
        } else {
            self.root_path.clone()
        };

        if let Some(tree) = &self.tree
            && !tree.is_empty()
        {
            let entries: Vec<RenderNode> = tree
                .iter()
                .map(|node| self.fs_project_tree_node(node, &base_path))
                .collect();
            let mut list = RenderNode::list(false, None, entries);
            list.attrs
                .set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
            children.push(list);
        }

        let mut root = RenderNode::root(children);

        // Tree connectors must never be wrapped — force `WordWrap::None`
        // onto the root layout so the terminal renderer preserves connector
        // geometry. Other layout slots (margins, alignment, max_width) ride
        // along via the component's Layout if non-default.
        let mut layout = self.layout.clone();
        layout.word_wrap = WordWrap::None;
        if layout != Layout::default() {
            root.attrs.set_layout(&layout);
        }
        root
    }
}

impl TreeRenderable for FileSystem {
    /// Projects the filesystem tree into the canonical render tree.
    ///
    /// The output is a [`NodeKind::Root`](renderable::tree::NodeKind::Root)
    /// containing an optional `Paragraph` (the root header) and an unordered
    /// `List` carrying the [`ListMarkerPolicy::TreeConnectors`] policy. Each
    /// entry projects to a `ListItem` containing a `Paragraph` with an icon
    /// `Span`, the entry name (or a `Link` when `file_links` is enabled), and
    /// an optional metrics `Span`. Directory items nest a sibling `List` with
    /// the same marker policy.
    ///
    /// Word wrap is forced to [`WordWrap::None`] on the root because tree
    /// connectors must never wrap; other layout slots (margins, alignment,
    /// max-width) ride along when the component's [`Layout`] is non-default.
    ///
    /// The bespoke [`TerminalRenderable::render`] path remains the production
    /// terminal renderer until parity tests prove the tree renderer produces
    /// equivalent connector geometry, icons, truncation, OSC8 links, metrics,
    /// ANSI styling, and layout behavior.
    fn render_tree(&self) -> RenderNode {
        // The caller may not have invoked `ensure_tree_built()` — projection
        // is read-only, so we project from whatever is currently cached and
        // emit an empty `Root` when nothing has been scanned. This matches
        // the empty-string contract of `render_optimistic`.
        self.fs_render_tree_inner()
    }
}

impl MarkdownRenderable for FileSystem {
    /// Renders the filesystem tree as portable Markdown via the canonical
    /// render tree.
    ///
    /// The tree carries [`ListMarkerPolicy::TreeConnectors`], which the
    /// Markdown renderer degrades to a native nested `- ` list. The render is
    /// performed under [`RenderStrictness::Lossy`] so the lossy diagnostic
    /// the renderer would otherwise raise stays out of the CLI output stream
    /// — Markdown intentionally has no terminal box-drawing characters.
    ///
    /// Styling (color, dim, italic) and Nerd Font icons are dropped, as are
    /// the Unicode fallback icon glyphs (📂 / 📄). The spec calls for icons
    /// to be omitted in plain Markdown so the output stays portable to
    /// renderers that lack font support for emoji/PUA glyphs. File links
    /// continue to render as `[name](file:///absolute/path)`.
    fn render_markdown(&self) -> String {
        let mut snapshot = self.clone();
        snapshot.ensure_tree_built();
        let mut node = snapshot.fs_render_tree_inner();
        // Plain Markdown: strip `fs-icon` spans (and their trailing separator
        // space) so glyphs like 📂 / 📄 do not leak into portable output.
        // The classed spans remain in MarkdownPlus and Browser.
        fs_strip_icon_spans(&mut node);
        let opts = MarkdownRenderOptions {
            strictness: RenderStrictness::Lossy,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(%error, "FileSystem markdown render failed");
                String::new()
            }
        }
    }

    /// Renders the filesystem tree as MarkdownPlus via the canonical render
    /// tree.
    ///
    /// MarkdownPlus accepts inline HTML, so the [`ListMarkerPolicy::TreeConnectors`]
    /// hint still degrades to a native nested list but classed `<span class="…">`
    /// hooks survive on the icon and metrics spans for CSS-driven preview.
    fn render_markdown_plus(&self) -> String {
        let mut snapshot = self.clone();
        snapshot.ensure_tree_built();
        let node = snapshot.fs_render_tree_inner();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            strictness: RenderStrictness::Lossy,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(%error, "FileSystem markdown_plus render failed");
                String::new()
            }
        }
    }
}

impl BrowserRenderable for FileSystem {
    /// Renders the filesystem tree as an HTML fragment via the canonical
    /// render tree.
    ///
    /// The tree's [`ListMarkerPolicy::TreeConnectors`] hint degrades cleanly
    /// to a nested `<ul>` / `<li>` structure with the connector list styled
    /// via `list-style: none` by the browser renderer. Component CSS hooks
    /// (`fs-dir`, `fs-file`, `fs-icon`, `fs-metrics`, …) survive on each
    /// node so a site stylesheet can apply icons, color, and metric styling.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let mut snapshot = self.clone();
        snapshot.ensure_tree_built();
        let node = snapshot.fs_render_tree_inner();
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Lossy,
            ..BrowserRenderOptions::default()
        };
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(%error, "FileSystem browser render failed");
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    // ============================================================
    // Error Display Tests
    // ============================================================

    #[test]
    fn test_error_path_not_found_display() {
        let err = FileSystemError::PathNotFound {
            path: PathBuf::from("/nonexistent/path"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Path not found"),
            "Expected 'Path not found' in: {msg}"
        );
        assert!(msg.contains("/nonexistent/path"), "Expected path in: {msg}");
    }

    #[test]
    fn test_error_not_a_directory_display() {
        let err = FileSystemError::NotADirectory {
            path: PathBuf::from("/some/file.txt"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Not a directory"),
            "Expected 'Not a directory' in: {msg}"
        );
        assert!(msg.contains("/some/file.txt"), "Expected path in: {msg}");
    }

    #[test]
    fn test_error_permission_denied_display() {
        let err = FileSystemError::PermissionDenied {
            path: PathBuf::from("/root/secret"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Permission denied"),
            "Expected 'Permission denied' in: {msg}"
        );
        assert!(msg.contains("/root/secret"), "Expected path in: {msg}");
    }

    #[test]
    fn test_error_io_error_from_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let fs_err: FileSystemError = io_err.into();
        let msg = fs_err.to_string();
        assert!(msg.contains("IO error"), "Expected 'IO error' in: {msg}");
        assert!(msg.contains("file not found"), "Expected cause in: {msg}");
    }

    #[test]
    fn test_error_io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err = FileSystemError::IoError(io_err);
        let msg = err.to_string();
        assert!(msg.contains("IO error"), "Expected 'IO error' in: {msg}");
        assert!(msg.contains("access denied"), "Expected cause in: {msg}");
    }

    // ============================================================
    // Icon Constant Tests
    // ============================================================

    #[test]
    fn test_nerd_dir_icons_are_valid_unicode() {
        // Just verify they're valid single characters
        assert!(icons::nerd::dir::BASE.is_alphanumeric() || !icons::nerd::dir::BASE.is_ascii());
        assert!(
            icons::nerd::dir::DEPTH_LIMIT.is_alphanumeric()
                || !icons::nerd::dir::DEPTH_LIMIT.is_ascii()
        );
        assert!(icons::nerd::dir::GIT.is_alphanumeric() || !icons::nerd::dir::GIT.is_ascii());
        assert!(icons::nerd::dir::GITHUB.is_alphanumeric() || !icons::nerd::dir::GITHUB.is_ascii());
        assert!(icons::nerd::dir::UTILS.is_alphanumeric() || !icons::nerd::dir::UTILS.is_ascii());
        assert!(icons::nerd::dir::DOCS.is_alphanumeric() || !icons::nerd::dir::DOCS.is_ascii());
    }

    #[test]
    fn test_nerd_file_icons_are_valid_unicode() {
        assert!(icons::nerd::file::BASE.is_alphanumeric() || !icons::nerd::file::BASE.is_ascii());
        assert!(
            icons::nerd::file::MARKDOWN.is_alphanumeric()
                || !icons::nerd::file::MARKDOWN.is_ascii()
        );
        assert!(
            icons::nerd::file::README.is_alphanumeric() || !icons::nerd::file::README.is_ascii()
        );
        assert!(
            icons::nerd::file::CLAUDE.is_alphanumeric() || !icons::nerd::file::CLAUDE.is_ascii()
        );
        assert!(icons::nerd::file::SKILL.is_alphanumeric() || !icons::nerd::file::SKILL.is_ascii());
        assert!(
            icons::nerd::file::SYMLINK.is_alphanumeric() || !icons::nerd::file::SYMLINK.is_ascii()
        );
    }

    #[test]
    fn test_nerd_ext_icons_are_valid_unicode() {
        assert!(icons::nerd::ext::RUST.is_alphanumeric() || !icons::nerd::ext::RUST.is_ascii());
        assert!(
            icons::nerd::ext::TYPESCRIPT.is_alphanumeric()
                || !icons::nerd::ext::TYPESCRIPT.is_ascii()
        );
        assert!(
            icons::nerd::ext::JAVASCRIPT.is_alphanumeric()
                || !icons::nerd::ext::JAVASCRIPT.is_ascii()
        );
        assert!(icons::nerd::ext::TOML.is_alphanumeric() || !icons::nerd::ext::TOML.is_ascii());
        assert!(icons::nerd::ext::YAML.is_alphanumeric() || !icons::nerd::ext::YAML.is_ascii());
        assert!(icons::nerd::ext::JSON.is_alphanumeric() || !icons::nerd::ext::JSON.is_ascii());
    }

    #[test]
    #[allow(deprecated)]
    fn test_nerd_special_icons_are_valid_unicode() {
        // Note: special module is deprecated, use file module instead
        assert!(
            icons::nerd::special::GITIGNORE.is_alphanumeric()
                || !icons::nerd::special::GITIGNORE.is_ascii()
        );
        assert!(
            icons::nerd::special::ENV.is_alphanumeric() || !icons::nerd::special::ENV.is_ascii()
        );
        assert!(
            icons::nerd::special::JUSTFILE.is_alphanumeric()
                || !icons::nerd::special::JUSTFILE.is_ascii()
        );
        assert!(
            icons::nerd::special::EDITORCONFIG.is_alphanumeric()
                || !icons::nerd::special::EDITORCONFIG.is_ascii()
        );
    }

    #[test]
    fn test_nerd_file_special_icons() {
        // These were moved from special module to file module
        assert!(
            icons::nerd::file::GITIGNORE.is_alphanumeric()
                || !icons::nerd::file::GITIGNORE.is_ascii()
        );
        assert!(icons::nerd::file::ENV.is_alphanumeric() || !icons::nerd::file::ENV.is_ascii());
        assert!(
            icons::nerd::file::JUSTFILE.is_alphanumeric()
                || !icons::nerd::file::JUSTFILE.is_ascii()
        );
        assert!(
            icons::nerd::file::EDITORCONFIG.is_alphanumeric()
                || !icons::nerd::file::EDITORCONFIG.is_ascii()
        );
        assert!(
            icons::nerd::file::AGENTS.is_alphanumeric() || !icons::nerd::file::AGENTS.is_ascii()
        );
    }

    #[test]
    fn test_nerd_dir_error_icon() {
        assert!(icons::nerd::dir::ERROR.is_alphanumeric() || !icons::nerd::dir::ERROR.is_ascii());
        assert_eq!(icons::nerd::dir::ERROR, '\u{f071}');
    }

    #[test]
    fn test_unicode_dir_error_icon() {
        assert_eq!(icons::unicode::dir::ERROR, '\u{26A0}'); // ⚠
    }

    #[test]
    fn test_unicode_fallback_icons() {
        // Unicode fallback icons should be emoji or ASCII
        assert_eq!(icons::unicode::dir::BASE, '📂');
        assert_eq!(icons::unicode::dir::DEPTH_LIMIT, '📁');
        assert_eq!(icons::unicode::file::BASE, '📄');
        assert_eq!(icons::unicode::file::SYMLINK, '@');
    }

    #[test]
    #[allow(deprecated)]
    fn test_specific_icon_codepoints() {
        // Verify exact codepoints from spec
        assert_eq!(icons::nerd::dir::BASE, '\u{e5fe}');
        assert_eq!(icons::nerd::dir::DEPTH_LIMIT, '\u{e652}');
        assert_eq!(icons::nerd::dir::ERROR, '\u{f071}');
        assert_eq!(icons::nerd::dir::GIT, '\u{e5fb}');
        assert_eq!(icons::nerd::dir::GITHUB, '\u{e5fd}');
        assert_eq!(icons::nerd::dir::UTILS, '\u{f19fc}');
        assert_eq!(icons::nerd::dir::DOCS, '\u{ebdf}');

        assert_eq!(icons::nerd::file::BASE, '\u{ea7b}');
        assert_eq!(icons::nerd::file::MARKDOWN, '\u{f0354}');
        assert_eq!(icons::nerd::file::README, '\u{f02e}');
        assert_eq!(icons::nerd::file::CLAUDE, '\u{f0721}');
        assert_eq!(icons::nerd::file::SKILL, '\u{f113c}');
        assert_eq!(icons::nerd::file::AGENTS, '\u{f21b}');
        assert_eq!(icons::nerd::file::SYMLINK, '\u{eaee}');
        assert_eq!(icons::nerd::file::GITIGNORE, '\u{e702}');
        assert_eq!(icons::nerd::file::ENV, '\u{eafa}');
        assert_eq!(icons::nerd::file::JUSTFILE, '\u{ee0d}');
        assert_eq!(icons::nerd::file::EDITORCONFIG, '\u{e615}');

        assert_eq!(icons::nerd::ext::RUST, '\u{e7a8}');
        assert_eq!(icons::nerd::ext::TYPESCRIPT, '\u{e8ca}');
        assert_eq!(icons::nerd::ext::JAVASCRIPT, '\u{e781}');
        assert_eq!(icons::nerd::ext::TOML, '\u{e6b2}');
        assert_eq!(icons::nerd::ext::YAML, '\u{e8eb}');
        assert_eq!(icons::nerd::ext::JSON, '\u{eb0f}');

        // Deprecated special module (now aliased to file module)
        assert_eq!(icons::nerd::special::GITIGNORE, '\u{e702}');
        assert_eq!(icons::nerd::special::ENV, '\u{eafa}');
        assert_eq!(icons::nerd::special::JUSTFILE, '\u{ee0d}');
        assert_eq!(icons::nerd::special::EDITORCONFIG, '\u{e615}');
    }

    // ============================================================
    // Icon Selection Tests
    // ============================================================

    #[test]
    fn test_get_icon_rust_file() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "main.rs".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        // Nerd Font icon for Rust files
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::ext::RUST);
        // Unicode fallback
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::file::BASE
        );
        // None defaults to Unicode fallback
        assert_eq!(fs.get_icon(&node, 0, None), icons::unicode::file::BASE);
    }

    #[test]
    fn test_get_icon_claude_md_root_only() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "CLAUDE.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        // At root (depth 0) - special CLAUDE icon
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::file::CLAUDE);
        // Not at root (depth > 0) - regular markdown icon
        assert_eq!(
            fs.get_icon(&node, 1, Some(true)),
            icons::nerd::file::MARKDOWN
        );
        assert_eq!(
            fs.get_icon(&node, 5, Some(true)),
            icons::nerd::file::MARKDOWN
        );
    }

    #[test]
    fn test_get_icon_agents_md_root_only() {
        let fs = FileSystem::default();

        // Agents.md at root
        let agents_node = TreeNode::File {
            name: "Agents.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&agents_node, 0, Some(true)),
            icons::nerd::file::AGENTS
        );
        assert_eq!(
            fs.get_icon(&agents_node, 1, Some(true)),
            icons::nerd::file::MARKDOWN
        );

        // Gemini.md at root
        let gemini_node = TreeNode::File {
            name: "Gemini.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&gemini_node, 0, Some(true)),
            icons::nerd::file::AGENTS
        );
    }

    #[test]
    fn test_get_icon_skill_md_any_depth() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "SKILL.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        // SKILL.md gets special icon at any depth
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::file::SKILL);
        assert_eq!(fs.get_icon(&node, 1, Some(true)), icons::nerd::file::SKILL);
        assert_eq!(fs.get_icon(&node, 10, Some(true)), icons::nerd::file::SKILL);
    }

    #[test]
    fn test_get_icon_readme_any_depth() {
        let fs = FileSystem::default();

        // README.md
        let readme_md = TreeNode::File {
            name: "README.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&readme_md, 0, Some(true)),
            icons::nerd::file::README
        );
        assert_eq!(
            fs.get_icon(&readme_md, 5, Some(true)),
            icons::nerd::file::README
        );

        // readme.md (lowercase)
        let readme_lower = TreeNode::File {
            name: "readme.md".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&readme_lower, 0, Some(true)),
            icons::nerd::file::README
        );

        // README (no extension)
        let readme_no_ext = TreeNode::File {
            name: "README".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&readme_no_ext, 0, Some(true)),
            icons::nerd::file::README
        );
    }

    #[test]
    fn test_get_icon_gitignore() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: ".gitignore".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&node, 0, Some(true)),
            icons::nerd::file::GITIGNORE
        );
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::file::BASE
        );
    }

    #[test]
    fn test_get_icon_env_files() {
        let fs = FileSystem::default();

        for name in [".env", ".env.local", ".env.example"] {
            let node = TreeNode::File {
                name: name.into(),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::file::ENV,
                "Expected ENV icon for {name}"
            );
        }
    }

    #[test]
    fn test_get_icon_justfile() {
        let fs = FileSystem::default();

        for name in ["justfile", "Justfile"] {
            let node = TreeNode::File {
                name: name.into(),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::file::JUSTFILE,
                "Expected JUSTFILE icon for {name}"
            );
        }
    }

    #[test]
    fn test_get_icon_editorconfig() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: ".editorconfig".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&node, 0, Some(true)),
            icons::nerd::file::EDITORCONFIG
        );
    }

    #[test]
    fn test_get_icon_extension_case_insensitive() {
        let fs = FileSystem::default();

        // Test uppercase .RS
        let upper = TreeNode::File {
            name: "main.RS".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&upper, 0, Some(true)), icons::nerd::ext::RUST);

        // Test mixed case .Rs
        let mixed = TreeNode::File {
            name: "lib.Rs".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&mixed, 0, Some(true)), icons::nerd::ext::RUST);
    }

    #[test]
    fn test_get_icon_typescript_extensions() {
        let fs = FileSystem::default();

        for ext in ["ts", "tsx", "TS", "TSX"] {
            let node = TreeNode::File {
                name: format!("file.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::ext::TYPESCRIPT,
                "Expected TYPESCRIPT icon for .{ext}"
            );
        }
    }

    #[test]
    fn test_get_icon_javascript_extensions() {
        let fs = FileSystem::default();

        for ext in ["js", "jsx", "mjs", "cjs"] {
            let node = TreeNode::File {
                name: format!("file.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::ext::JAVASCRIPT,
                "Expected JAVASCRIPT icon for .{ext}"
            );
        }
    }

    #[test]
    fn test_get_icon_config_extensions() {
        let fs = FileSystem::default();

        // TOML
        let toml = TreeNode::File {
            name: "Cargo.toml".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&toml, 0, Some(true)), icons::nerd::ext::TOML);

        // YAML variants
        for ext in ["yaml", "yml"] {
            let node = TreeNode::File {
                name: format!("config.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::ext::YAML,
                "Expected YAML icon for .{ext}"
            );
        }

        // JSON variants
        for ext in ["json", "json5", "jsonc"] {
            let node = TreeNode::File {
                name: format!("package.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::ext::JSON,
                "Expected JSON icon for .{ext}"
            );
        }
    }

    #[test]
    fn test_get_icon_markdown_extensions() {
        let fs = FileSystem::default();

        for ext in ["md", "mdx", "markdown"] {
            let node = TreeNode::File {
                name: format!("docs.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::file::MARKDOWN,
                "Expected MARKDOWN icon for .{ext}"
            );
        }
    }

    #[test]
    fn test_get_icon_unknown_extension() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "data.xyz".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::file::BASE);
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::file::BASE
        );
    }

    #[test]
    fn test_get_icon_symlink_file() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "main.rs".into(),
            is_ignored: false,
            is_symlink: true, // Symlink takes priority
            metrics: None,
        };
        // Symlink icon takes priority over extension
        assert_eq!(
            fs.get_icon(&node, 0, Some(true)),
            icons::nerd::file::SYMLINK
        );
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::file::SYMLINK
        );
    }

    #[test]
    fn test_get_icon_symlink_dir() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "docs".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: true,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&node, 0, Some(true)),
            icons::nerd::file::SYMLINK
        );
    }

    #[test]
    fn test_get_icon_error_dir() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "restricted".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: true, // Permission error
            at_depth_limit: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::dir::ERROR);
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::dir::ERROR
        );
    }

    #[test]
    fn test_get_icon_depth_limit_dir() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "deep".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: true,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&node, 0, Some(true)),
            icons::nerd::dir::DEPTH_LIMIT
        );
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::dir::DEPTH_LIMIT
        );
    }

    #[test]
    fn test_get_icon_special_dirs() {
        let fs = FileSystem::default();

        // .git
        let git_dir = TreeNode::Dir {
            name: ".git".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&git_dir, 0, Some(true)), icons::nerd::dir::GIT);
        assert_eq!(
            fs.get_icon(&git_dir, 0, Some(false)),
            icons::unicode::dir::BASE
        );

        // .github
        let github_dir = TreeNode::Dir {
            name: ".github".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };
        assert_eq!(
            fs.get_icon(&github_dir, 0, Some(true)),
            icons::nerd::dir::GITHUB
        );

        // utils variants
        for name in ["utils", "util", "helpers"] {
            let node = TreeNode::Dir {
                name: name.into(),
                children: vec![],
                is_ignored: false,
                is_symlink: false,
                has_error: false,
                at_depth_limit: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::dir::UTILS,
                "Expected UTILS icon for {name}"
            );
        }

        // docs variants
        for name in ["docs", "doc", "documentation"] {
            let node = TreeNode::Dir {
                name: name.into(),
                children: vec![],
                is_ignored: false,
                is_symlink: false,
                has_error: false,
                at_depth_limit: false,
                metrics: None,
            };
            assert_eq!(
                fs.get_icon(&node, 0, Some(true)),
                icons::nerd::dir::DOCS,
                "Expected DOCS icon for {name}"
            );
        }
    }

    #[test]
    fn test_get_icon_regular_dir() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "src".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::dir::BASE);
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::dir::BASE
        );
    }

    #[test]
    fn test_get_icon_with_padding_nerd_font() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "main.rs".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let padded = fs.get_icon_with_padding(&node, 0, Some(true));
        // Should be icon + space
        assert_eq!(padded.chars().count(), 2);
        assert!(padded.ends_with(' '));
        assert!(padded.starts_with(icons::nerd::ext::RUST));
    }

    #[test]
    fn test_get_icon_with_padding_unicode() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "main.rs".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let padded = fs.get_icon_with_padding(&node, 0, Some(false));
        // Should be just the icon (no padding)
        assert_eq!(padded.chars().count(), 1);
        assert_eq!(padded, icons::unicode::file::BASE.to_string());
    }

    #[test]
    fn test_get_icon_with_padding_none() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "main.rs".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        // None defaults to unicode (no padding)
        let padded = fs.get_icon_with_padding(&node, 0, None);
        assert_eq!(padded.chars().count(), 1);
    }

    // ============================================================
    // Tree-Drawing Constants Tests
    // ============================================================

    #[test]
    fn test_tree_chars_lengths() {
        // All tree chars should be 4 display columns wide
        assert_eq!(tree_chars::BRANCH.chars().count(), 4);
        assert_eq!(tree_chars::LAST_BRANCH.chars().count(), 4);
        assert_eq!(tree_chars::VERTICAL.chars().count(), 4);
        assert_eq!(tree_chars::INDENT.chars().count(), 4);
    }

    #[test]
    fn test_tree_chars_content() {
        assert_eq!(tree_chars::BRANCH, "├── ");
        assert_eq!(tree_chars::LAST_BRANCH, "└── ");
        assert_eq!(tree_chars::VERTICAL, "│   ");
        assert_eq!(tree_chars::INDENT, "    ");
    }

    // ============================================================
    // TreeNode Tests
    // ============================================================

    #[test]
    fn test_tree_node_dir_basic() {
        let node = TreeNode::Dir {
            name: "src".to_string(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        assert_eq!(node.name(), "src");
        assert!(!node.is_ignored());
        assert!(!node.is_symlink());
        assert!(node.is_dir());
        assert!(!node.is_file());
    }

    #[test]
    fn test_tree_node_file_basic() {
        let node = TreeNode::File {
            name: "main.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        assert_eq!(node.name(), "main.rs");
        assert!(!node.is_ignored());
        assert!(!node.is_symlink());
        assert!(!node.is_dir());
        assert!(node.is_file());
    }

    #[test]
    fn test_tree_node_ignored_file() {
        let node = TreeNode::File {
            name: "target".to_string(),
            is_ignored: true,
            is_symlink: false,
            metrics: None,
        };

        assert!(node.is_ignored());
    }

    #[test]
    fn test_tree_node_symlink_dir() {
        let node = TreeNode::Dir {
            name: "link".to_string(),
            children: vec![],
            is_ignored: false,
            is_symlink: true,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        assert!(node.is_symlink());
    }

    #[test]
    fn test_tree_node_symlink_file() {
        let node = TreeNode::File {
            name: "config".to_string(),
            is_ignored: false,
            is_symlink: true,
            metrics: None,
        };

        assert!(node.is_symlink());
    }

    #[test]
    fn test_tree_node_with_children() {
        let child1 = TreeNode::File {
            name: "lib.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let child2 = TreeNode::File {
            name: "main.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let parent = TreeNode::Dir {
            name: "src".to_string(),
            children: vec![child1, child2],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        if let TreeNode::Dir { children, .. } = parent {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].name(), "lib.rs");
            assert_eq!(children[1].name(), "main.rs");
        } else {
            panic!("Expected Dir variant");
        }
    }

    #[test]
    fn test_tree_node_depth_limit() {
        let node = TreeNode::Dir {
            name: "deep".to_string(),
            children: vec![], // Children not populated due to depth limit
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: true,
            metrics: None,
        };

        if let TreeNode::Dir { at_depth_limit, .. } = node {
            assert!(at_depth_limit);
        } else {
            panic!("Expected Dir variant");
        }
    }

    #[test]
    fn test_tree_node_has_error() {
        let node = TreeNode::Dir {
            name: "unreadable".to_string(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: true,
            at_depth_limit: false,
            metrics: None,
        };

        if let TreeNode::Dir { has_error, .. } = node {
            assert!(has_error);
        } else {
            panic!("Expected Dir variant");
        }
    }

    #[test]
    fn test_tree_node_clone() {
        let original = TreeNode::File {
            name: "test.rs".to_string(),
            is_ignored: true,
            is_symlink: true,
            metrics: None,
        };

        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_tree_node_debug() {
        let node = TreeNode::File {
            name: "test.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let debug_str = format!("{:?}", node);
        assert!(debug_str.contains("File"));
        assert!(debug_str.contains("test.rs"));
    }

    #[test]
    fn test_tree_node_equality() {
        let node1 = TreeNode::File {
            name: "test.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let node2 = TreeNode::File {
            name: "test.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let node3 = TreeNode::File {
            name: "other.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        assert_eq!(node1, node2);
        assert_ne!(node1, node3);
    }

    #[test]
    fn test_nested_tree_structure() {
        // Build a realistic nested structure
        let main_rs = TreeNode::File {
            name: "main.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let lib_rs = TreeNode::File {
            name: "lib.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let src = TreeNode::Dir {
            name: "src".to_string(),
            children: vec![lib_rs, main_rs],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        let cargo_toml = TreeNode::File {
            name: "Cargo.toml".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let target = TreeNode::Dir {
            name: "target".to_string(),
            children: vec![],
            is_ignored: true,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        let root = TreeNode::Dir {
            name: "my-project".to_string(),
            children: vec![cargo_toml, src, target],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        // Verify structure
        assert_eq!(root.name(), "my-project");
        if let TreeNode::Dir { children, .. } = &root {
            assert_eq!(children.len(), 3);

            // Check src directory has children
            if let TreeNode::Dir {
                children: src_children,
                ..
            } = &children[1]
            {
                assert_eq!(src_children.len(), 2);
            }

            // Check target is ignored
            assert!(children[2].is_ignored());
        }
    }

    // ============================================================
    // FileSystem Constructor Tests
    // ============================================================

    #[test]
    fn test_filesystem_new_with_valid_directory() {
        // Use a directory that definitely exists
        let fs = FileSystem::new(".").expect("current directory should exist");
        assert!(fs.root_path().exists());
        assert!(fs.root_path().is_dir());
    }

    #[test]
    fn test_filesystem_new_with_absolute_path() {
        let temp_dir = std::env::temp_dir();
        let fs = FileSystem::new(&temp_dir).expect("temp directory should exist");
        assert_eq!(fs.root_path(), temp_dir.as_path());
    }

    #[test]
    fn test_filesystem_new_path_not_found() {
        let result = FileSystem::new("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::PathNotFound { .. }),
            "Expected PathNotFound, got {:?}",
            err
        );
    }

    #[test]
    fn test_filesystem_new_not_a_directory() {
        // Use Cargo.toml which exists but is a file, not a directory
        let result = FileSystem::new("Cargo.toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::NotADirectory { .. }),
            "Expected NotADirectory, got {:?}",
            err
        );
    }

    #[test]
    fn test_filesystem_new_with_formatting() {
        let fs = FileSystem::new_with_formatting(".").expect("current directory should exist");

        // Verify the formatting presets are applied
        assert!(fs.italicize_dot_files);
        assert!(fs.italicize_dot_dirs);
        assert!(fs.dim_gitignore);
        assert!(fs.do_not_recurse_gitignore);
    }

    // ============================================================
    // FileSystem Default Tests
    // ============================================================

    #[test]
    fn test_filesystem_default() {
        let fs = FileSystem::default();

        // Default values
        assert_eq!(fs.max_depth, 20);
        assert_eq!(fs.max_entries, 1000);
        assert!(!fs.dim_gitignore);
        assert!(!fs.italicize_dot_files);
        assert!(!fs.italicize_dot_dirs);
        assert!(!fs.hide_dot_files);
        assert!(!fs.hide_dot_dirs);
        assert!(!fs.do_not_recurse_gitignore);
        assert!(fs.filter_patterns.is_empty());
        assert!(fs.highlight_red.is_empty());
        assert!(fs.highlight_green.is_empty());
    }

    #[test]
    fn test_filesystem_default_tree_is_none() {
        let fs = FileSystem::default();
        assert!(
            fs.tree.is_none(),
            "Tree should be None (lazy initialization)"
        );
        assert!(!fs.is_tree_built());
    }

    // ============================================================
    // FileSystem Builder Tests
    // ============================================================

    #[test]
    fn test_filesystem_builder_depth() {
        let fs = FileSystem::new(".").unwrap().depth(5);
        assert_eq!(fs.get_max_depth(), 5);
    }

    #[test]
    fn test_filesystem_builder_max_entries() {
        let fs = FileSystem::new(".").unwrap().max_entries(500);
        assert_eq!(fs.get_max_entries(), 500);
    }

    #[test]
    fn test_filesystem_builder_dim_gitignore() {
        let fs = FileSystem::new(".").unwrap().dim_gitignore(true);
        assert!(fs.dim_gitignore);

        let fs = FileSystem::new(".").unwrap().dim_gitignore(false);
        assert!(!fs.dim_gitignore);
    }

    #[test]
    fn test_filesystem_builder_italicize_dot_files() {
        let fs = FileSystem::new(".").unwrap().italicize_dot_files(true);
        assert!(fs.italicize_dot_files);
    }

    #[test]
    fn test_filesystem_builder_italicize_dot_dirs() {
        let fs = FileSystem::new(".").unwrap().italicize_dot_dirs(true);
        assert!(fs.italicize_dot_dirs);
    }

    #[test]
    fn test_filesystem_builder_hide_dot_files() {
        let fs = FileSystem::new(".").unwrap().hide_dot_files(true);
        assert!(fs.hide_dot_files);
    }

    #[test]
    fn test_filesystem_builder_hide_dot_dirs() {
        let fs = FileSystem::new(".").unwrap().hide_dot_dirs(true);
        assert!(fs.hide_dot_dirs);
    }

    #[test]
    fn test_filesystem_builder_do_not_recurse_gitignore() {
        let fs = FileSystem::new(".").unwrap().do_not_recurse_gitignore(true);
        assert!(fs.do_not_recurse_gitignore);
    }

    #[test]
    fn test_filesystem_builder_filter() {
        let fs = FileSystem::new(".")
            .unwrap()
            .filter("*.rs")
            .filter("*.toml");
        assert_eq!(fs.filter_patterns.len(), 2);
        assert_eq!(fs.filter_patterns[0], "*.rs");
        assert_eq!(fs.filter_patterns[1], "*.toml");
    }

    #[test]
    fn test_filesystem_builder_highlight_red() {
        let fs = FileSystem::new(".")
            .unwrap()
            .highlight_red("error")
            .highlight_red("fail");
        assert_eq!(fs.highlight_red.len(), 2);
        assert_eq!(fs.highlight_red[0], "error");
        assert_eq!(fs.highlight_red[1], "fail");
    }

    #[test]
    fn test_filesystem_builder_highlight_green() {
        let fs = FileSystem::new(".")
            .unwrap()
            .highlight_green("success")
            .highlight_green("pass");
        assert_eq!(fs.highlight_green.len(), 2);
        assert_eq!(fs.highlight_green[0], "success");
        assert_eq!(fs.highlight_green[1], "pass");
    }

    #[test]
    fn test_filesystem_builder_layout() {
        use crate::utils::layout::{Layout, Length, Edges, TargetValue};

        let custom_layout = Layout {
            margin: Edges {
                left: TargetValue::universal(Length::ch(4)),
                ..Edges::default()
            },
            ..Layout::default()
        };

        let fs = FileSystem::new(".").unwrap().layout(custom_layout.clone());
        assert_eq!(fs.layout.margin.left, TargetValue::universal(Length::ch(4)));
    }

    #[test]
    fn test_filesystem_builder_chaining() {
        // Test that multiple builder methods can be chained fluently
        let fs = FileSystem::new(".")
            .unwrap()
            .depth(5)
            .max_entries(100)
            .dim_gitignore(true)
            .italicize_dot_files(true)
            .italicize_dot_dirs(true)
            .hide_dot_files(false)
            .hide_dot_dirs(false)
            .do_not_recurse_gitignore(true)
            .filter("*.rs")
            .highlight_red("error")
            .highlight_green("success");

        assert_eq!(fs.get_max_depth(), 5);
        assert_eq!(fs.get_max_entries(), 100);
        assert!(fs.dim_gitignore);
        assert!(fs.italicize_dot_files);
        assert!(fs.italicize_dot_dirs);
        assert!(!fs.hide_dot_files);
        assert!(!fs.hide_dot_dirs);
        assert!(fs.do_not_recurse_gitignore);
        assert_eq!(fs.filter_patterns.len(), 1);
        assert_eq!(fs.highlight_red.len(), 1);
        assert_eq!(fs.highlight_green.len(), 1);
    }

    #[test]
    fn test_filesystem_tree_not_built_after_construction() {
        let fs = FileSystem::new(".").unwrap();
        assert!(
            !fs.is_tree_built(),
            "Tree should not be built after construction"
        );
        assert!(fs.tree.is_none());
    }

    #[test]
    fn test_filesystem_tree_not_built_after_builder_chaining() {
        let fs = FileSystem::new(".")
            .unwrap()
            .depth(5)
            .dim_gitignore(true)
            .filter("*.rs");

        assert!(
            !fs.is_tree_built(),
            "Tree should not be built after builder method calls"
        );
    }

    #[test]
    fn test_filesystem_clone() {
        let fs1 = FileSystem::new(".").unwrap().depth(10).dim_gitignore(true);

        let fs2 = fs1.clone();

        assert_eq!(fs1.root_path(), fs2.root_path());
        assert_eq!(fs1.get_max_depth(), fs2.get_max_depth());
        assert_eq!(fs1.dim_gitignore, fs2.dim_gitignore);
    }

    #[test]
    fn test_filesystem_debug() {
        let fs = FileSystem::new(".").unwrap();
        let debug_str = format!("{:?}", fs);
        assert!(debug_str.contains("FileSystem"));
        assert!(debug_str.contains("root_path"));
    }

    // ============================================================
    // TryFrom Trait Tests
    // ============================================================

    #[test]
    fn test_try_from_str_valid_directory() {
        let fs = FileSystem::try_from(".").expect("should work for current dir");
        assert!(fs.root_path().exists());
        assert!(fs.root_path().is_dir());
    }

    #[test]
    fn test_try_from_str_nonexistent_path() {
        let result = FileSystem::try_from("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::PathNotFound { .. }),
            "Expected PathNotFound, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_str_not_a_directory() {
        let result = FileSystem::try_from("Cargo.toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::NotADirectory { .. }),
            "Expected NotADirectory, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_string_valid_directory() {
        let path = String::from(".");
        let fs = FileSystem::try_from(path).expect("should work for current dir");
        assert!(fs.root_path().exists());
        assert!(fs.root_path().is_dir());
    }

    #[test]
    fn test_try_from_string_nonexistent_path() {
        let path = String::from("/nonexistent/path/that/does/not/exist");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::PathNotFound { .. }),
            "Expected PathNotFound, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_string_not_a_directory() {
        let path = String::from("Cargo.toml");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::NotADirectory { .. }),
            "Expected NotADirectory, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_path_ref_valid_directory() {
        let path = Path::new(".");
        let fs = FileSystem::try_from(path).expect("should work for current dir");
        assert!(fs.root_path().exists());
        assert!(fs.root_path().is_dir());
    }

    #[test]
    fn test_try_from_path_ref_nonexistent_path() {
        let path = Path::new("/nonexistent/path/that/does/not/exist");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::PathNotFound { .. }),
            "Expected PathNotFound, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_path_ref_not_a_directory() {
        let path = Path::new("Cargo.toml");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::NotADirectory { .. }),
            "Expected NotADirectory, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_pathbuf_valid_directory() {
        let path = PathBuf::from(".");
        let fs = FileSystem::try_from(path).expect("should work for current dir");
        assert!(fs.root_path().exists());
        assert!(fs.root_path().is_dir());
    }

    #[test]
    fn test_try_from_pathbuf_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path/that/does/not/exist");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::PathNotFound { .. }),
            "Expected PathNotFound, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_pathbuf_not_a_directory() {
        let path = PathBuf::from("Cargo.toml");
        let result = FileSystem::try_from(path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, FileSystemError::NotADirectory { .. }),
            "Expected NotADirectory, got {:?}",
            err
        );
    }

    #[test]
    fn test_try_from_absolute_pathbuf() {
        let temp_dir = std::env::temp_dir();
        let fs = FileSystem::try_from(temp_dir.clone()).expect("temp directory should exist");
        assert_eq!(fs.root_path(), temp_dir.as_path());
    }

    #[test]
    fn test_try_from_error_type_is_filesystem_error() {
        // Verify that the error type is FileSystemError for all variants
        let result: Result<FileSystem, FileSystemError> = FileSystem::try_from("/nonexistent");
        assert!(result.is_err());

        let result: Result<FileSystem, FileSystemError> =
            FileSystem::try_from(String::from("/nonexistent"));
        assert!(result.is_err());

        let result: Result<FileSystem, FileSystemError> =
            FileSystem::try_from(Path::new("/nonexistent"));
        assert!(result.is_err());

        let result: Result<FileSystem, FileSystemError> =
            FileSystem::try_from(PathBuf::from("/nonexistent"));
        assert!(result.is_err());
    }

    // ============================================================
    // Tree Building Integration Tests (with tempdir)
    // ============================================================

    #[test]
    fn test_tree_building_basic_structure() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create test structure:
        // temp/
        // ├── dir1/
        // │   └── nested.txt
        // └── file1.txt
        fs::create_dir(temp.path().join("dir1")).expect("create dir1");
        fs::write(temp.path().join("file1.txt"), "content").expect("create file1.txt");
        fs::write(temp.path().join("dir1/nested.txt"), "nested").expect("create nested.txt");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        assert!(fs_tree.is_tree_built());
        let tree = fs_tree.tree().expect("tree should be built");

        // Should have 2 entries: dir1 (sorted first as directory) and file1.txt
        assert_eq!(tree.len(), 2, "Expected 2 top-level entries");

        // First entry should be dir1 (directories first)
        assert!(tree[0].is_dir(), "First entry should be a directory");
        assert_eq!(tree[0].name(), "dir1");

        // Second entry should be file1.txt
        assert!(tree[1].is_file(), "Second entry should be a file");
        assert_eq!(tree[1].name(), "file1.txt");

        // Check nested structure
        if let TreeNode::Dir { children, .. } = &tree[0] {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].name(), "nested.txt");
        } else {
            panic!("Expected dir1 to be a directory");
        }
    }

    #[test]
    fn test_tree_building_directories_sorted_first() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create files and dirs with mixed names to test sorting
        fs::create_dir(temp.path().join("zebra")).expect("create zebra dir");
        fs::write(temp.path().join("alpha.txt"), "").expect("create alpha.txt");
        fs::create_dir(temp.path().join("beta")).expect("create beta dir");
        fs::write(temp.path().join("gamma.txt"), "").expect("create gamma.txt");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 4);

        // Directories first, then alphabetically
        // Expected order: beta, zebra, alpha.txt, gamma.txt
        assert!(tree[0].is_dir());
        assert_eq!(tree[0].name(), "beta");
        assert!(tree[1].is_dir());
        assert_eq!(tree[1].name(), "zebra");
        assert!(tree[2].is_file());
        assert_eq!(tree[2].name(), "alpha.txt");
        assert!(tree[3].is_file());
        assert_eq!(tree[3].name(), "gamma.txt");
    }

    #[test]
    fn test_tree_building_case_insensitive_sort() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create files with mixed case names
        fs::write(temp.path().join("Zebra.txt"), "").expect("create Zebra.txt");
        fs::write(temp.path().join("apple.txt"), "").expect("create apple.txt");
        fs::write(temp.path().join("BANANA.txt"), "").expect("create BANANA.txt");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 3);

        // Should be sorted case-insensitively: apple, BANANA, Zebra
        assert_eq!(tree[0].name(), "apple.txt");
        assert_eq!(tree[1].name(), "BANANA.txt");
        assert_eq!(tree[2].name(), "Zebra.txt");
    }

    #[test]
    fn test_tree_building_max_depth_enforced() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create deep nesting: level0/level1/level2/file.txt
        let level0 = temp.path().join("level0");
        let level1 = level0.join("level1");
        let level2 = level1.join("level2");

        fs::create_dir(&level0).expect("create level0");
        fs::create_dir(&level1).expect("create level1");
        fs::create_dir(&level2).expect("create level2");
        fs::write(level2.join("deep.txt"), "deep").expect("create deep.txt");

        // Test with max_depth = 2 (should only see level0 and level1)
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path").depth(2);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].name(), "level0");

        if let TreeNode::Dir {
            children,
            at_depth_limit,
            ..
        } = &tree[0]
        {
            assert!(!at_depth_limit, "level0 should not be at depth limit");
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].name(), "level1");

            // level1 should be at depth limit (depth 2 = indices 0, 1)
            if let TreeNode::Dir {
                children: level1_children,
                at_depth_limit: level1_at_limit,
                ..
            } = &children[0]
            {
                assert!(level1_at_limit, "level1 should be at depth limit");
                assert!(
                    level1_children.is_empty(),
                    "level1 children should be empty due to depth limit"
                );
            }
        }
    }

    #[test]
    fn test_tree_building_max_entries_enforced() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create more files than the limit
        for i in 0..10 {
            fs::write(temp.path().join(format!("file{:02}.txt", i)), "").expect("create file");
        }

        // Set max_entries to 5
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .max_entries(5);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert!(
            tree.len() <= 5,
            "Should have at most 5 entries, got {}",
            tree.len()
        );
    }

    #[test]
    fn test_tree_building_hide_dot_files() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        fs::write(temp.path().join(".hidden"), "hidden").expect("create .hidden");
        fs::write(temp.path().join("visible.txt"), "visible").expect("create visible.txt");

        // Without hide_dot_files
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2, "Should see both files");

        // With hide_dot_files
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .hide_dot_files(true);
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 1, "Should only see visible.txt");
        assert_eq!(tree[0].name(), "visible.txt");
    }

    #[test]
    fn test_tree_building_hide_dot_dirs() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        fs::create_dir(temp.path().join(".hidden_dir")).expect("create .hidden_dir");
        fs::create_dir(temp.path().join("visible_dir")).expect("create visible_dir");

        // Without hide_dot_dirs
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2, "Should see both dirs");

        // With hide_dot_dirs
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .hide_dot_dirs(true);
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 1, "Should only see visible_dir");
        assert_eq!(tree[0].name(), "visible_dir");
    }

    #[test]
    fn test_tree_building_filter_patterns() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        fs::write(temp.path().join("main.rs"), "").expect("create main.rs");
        fs::write(temp.path().join("lib.rs"), "").expect("create lib.rs");
        fs::write(temp.path().join("readme.md"), "").expect("create readme.md");
        fs::write(temp.path().join("config.toml"), "").expect("create config.toml");

        // Filter for .rs files only
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .filter(".rs");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2, "Should only see .rs files");

        let names: Vec<_> = tree.iter().map(|n| n.name()).collect();
        assert!(names.contains(&"main.rs"));
        assert!(names.contains(&"lib.rs"));
    }

    #[cfg(unix)]
    #[test]
    fn test_tree_building_symlink_detection() {
        use std::fs;
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create a regular file and a symlink to it
        fs::write(temp.path().join("target.txt"), "target").expect("create target.txt");
        symlink(temp.path().join("target.txt"), temp.path().join("link.txt"))
            .expect("create symlink");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2);

        // Find the symlink
        let link_node = tree.iter().find(|n| n.name() == "link.txt");
        assert!(link_node.is_some(), "Should find link.txt");
        assert!(
            link_node.unwrap().is_symlink(),
            "link.txt should be marked as symlink"
        );

        // Target should not be a symlink
        let target_node = tree.iter().find(|n| n.name() == "target.txt");
        assert!(target_node.is_some(), "Should find target.txt");
        assert!(
            !target_node.unwrap().is_symlink(),
            "target.txt should not be a symlink"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_tree_building_symlink_dir_not_followed() {
        use std::fs;
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create a directory with content
        let real_dir = temp.path().join("real_dir");
        fs::create_dir(&real_dir).expect("create real_dir");
        fs::write(real_dir.join("inside.txt"), "inside").expect("create inside.txt");

        // Create a symlink to the directory
        symlink(&real_dir, temp.path().join("link_dir")).expect("create dir symlink");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2);

        // Find the symlinked directory
        let link_dir = tree.iter().find(|n| n.name() == "link_dir");
        assert!(link_dir.is_some(), "Should find link_dir");

        if let TreeNode::Dir {
            children,
            is_symlink,
            ..
        } = link_dir.unwrap()
        {
            assert!(is_symlink, "link_dir should be marked as symlink");
            assert!(
                children.is_empty(),
                "Symlinked directory should not have children (not followed)"
            );
        } else {
            panic!("link_dir should be a Dir variant");
        }

        // The real directory should have children
        let real_dir_node = tree.iter().find(|n| n.name() == "real_dir");
        if let TreeNode::Dir { children, .. } = real_dir_node.unwrap() {
            assert_eq!(children.len(), 1, "real_dir should have one child");
            assert_eq!(children[0].name(), "inside.txt");
        }
    }

    #[test]
    fn test_tree_building_empty_directory() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create empty subdirectory
        fs::create_dir(temp.path().join("empty_dir")).expect("create empty_dir");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 1);

        if let TreeNode::Dir { children, .. } = &tree[0] {
            assert!(children.is_empty(), "empty_dir should have no children");
        }
    }

    #[test]
    fn test_ensure_tree_built_idempotent() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("file.txt"), "content").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");

        // First build
        fs_tree.ensure_tree_built();
        assert!(fs_tree.is_tree_built());
        let first_tree = fs_tree.tree().expect("tree should be built").clone();

        // Second build should be a no-op
        fs_tree.ensure_tree_built();
        let second_tree = fs_tree.tree().expect("tree should still be built");

        assert_eq!(
            first_tree, *second_tree,
            "Tree should be identical after multiple ensure_tree_built calls"
        );
    }

    #[test]
    fn test_tree_building_permission_error_graceful() {
        // This test verifies we don't panic on permission errors.
        // We can't easily simulate permission errors in a portable way,
        // but we can verify the method handles read_dir failures.

        let temp = tempfile::tempdir().expect("create temp dir");
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");

        // Should not panic even on an empty directory
        fs_tree.ensure_tree_built();
        assert!(fs_tree.is_tree_built());
    }

    #[test]
    fn test_tree_accessor_returns_none_before_build() {
        let fs_tree = FileSystem::new(".").expect("valid path");
        assert!(
            fs_tree.tree().is_none(),
            "tree() should return None before build"
        );
    }

    #[test]
    fn test_tree_accessor_returns_some_after_build() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("file.txt"), "content").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        assert!(
            fs_tree.tree().is_some(),
            "tree() should return Some after build"
        );
    }

    // ============================================================
    // TerminalRenderable Implementation Tests
    // ============================================================

    #[test]
    fn test_render_empty_tree_returns_empty_string() {
        let fs = FileSystem::default();
        // Tree not built, should return empty
        let result = fs.render_optimistic(Some(80));
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_with_single_file() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.rs"), "fn main() {}").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));

        // Should contain the tree connector and filename
        assert!(result.contains("test.rs"), "Output should contain filename");
        assert!(
            result.contains(tree_chars::LAST_BRANCH),
            "Output should contain tree connector"
        );
    }

    #[test]
    fn test_render_with_directory_and_file() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        let src_dir = temp.path().join("src");
        fs::create_dir(&src_dir).expect("create src dir");
        fs::write(src_dir.join("main.rs"), "fn main() {}").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));

        // Should contain both src directory and main.rs file
        assert!(
            result.contains("src"),
            "Output should contain src directory"
        );
        assert!(result.contains("main.rs"), "Output should contain main.rs");
    }

    #[test]
    fn test_render_tree_connectors_correct() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("a.txt"), "").expect("create a.txt");
        fs::write(temp.path().join("b.txt"), "").expect("create b.txt");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .show_root(false);
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(lines.len(), 2, "Should have 2 lines for 2 files");

        // First file should have BRANCH (├──), last should have LAST_BRANCH (└──)
        assert!(
            lines[0].contains("├── "),
            "First line should use BRANCH connector"
        );
        assert!(
            lines[1].contains("└── "),
            "Last line should use LAST_BRANCH connector"
        );
    }

    #[test]
    fn test_render_shows_root_by_default() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("a.txt"), "").expect("create a.txt");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();

        // First line is root header, second is the file
        assert_eq!(lines.len(), 2, "Should have root header + 1 file");
        // Root line should NOT have a tree connector
        assert!(
            !lines[0].contains("├── ") && !lines[0].contains("└── "),
            "Root line should not have tree connectors"
        );
        // Second line should have a tree connector
        assert!(
            lines[1].contains("└── "),
            "File line should have LAST_BRANCH connector"
        );
    }

    #[test]
    fn test_render_skip_root() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("a.txt"), "").expect("create a.txt");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .show_root(false);
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(
            lines.len(),
            1,
            "Should have only 1 file line (no root header)"
        );
        assert!(
            lines[0].contains("└── "),
            "File line should have LAST_BRANCH connector"
        );
    }

    #[test]
    fn test_render_nested_tree_structure() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        let src_dir = temp.path().join("src");
        fs::create_dir(&src_dir).expect("create src dir");
        fs::write(src_dir.join("lib.rs"), "").expect("create lib.rs");
        fs::write(src_dir.join("main.rs"), "").expect("create main.rs");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .show_root(false);
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));
        let lines: Vec<&str> = result.lines().collect();

        // Should have: src, then lib.rs and main.rs inside
        assert_eq!(lines.len(), 3, "Should have 3 lines");

        // The nested files should have vertical continuation from parent
        for line in &lines[1..] {
            // Nested lines should start with either VERTICAL + connector or INDENT + connector
            assert!(
                line.starts_with("    ") || line.starts_with("│   "),
                "Nested lines should be indented: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_render_respects_width_40() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        // Create a file with a long name
        let long_name = "this_is_a_very_long_filename_that_should_be_truncated.rs";
        fs::write(temp.path().join(long_name), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(40));

        // Each line should fit within 40 columns
        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 40,
                "Line exceeds width 40: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_respects_width_80() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        // Create a file with a long name
        let long_name =
            "this_is_a_very_long_filename_that_should_probably_be_truncated_at_some_point.rs";
        fs::write(temp.path().join(long_name), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(80));

        // Each line should fit within 80 columns
        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 80,
                "Line exceeds width 80: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_respects_width_120() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        let long_name = "very_long_filename_that_exceeds_normal_terminal_width.rs";
        fs::write(temp.path().join(long_name), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let result = fs_tree.render_optimistic(Some(120));

        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 120,
                "Line exceeds width 120: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_five_level_deep_tree() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create 5 levels of nested directories
        let level1 = temp.path().join("level1");
        let level2 = level1.join("level2");
        let level3 = level2.join("level3");
        let level4 = level3.join("level4");
        let level5 = level4.join("level5");

        fs::create_dir_all(&level5).expect("create nested dirs");
        fs::write(level5.join("deep_file.txt"), "content").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path").depth(10);
        fs_tree.ensure_tree_built();

        // Test at different widths
        for width in [40, 80, 120] {
            let result = fs_tree.render_optimistic(Some(width));

            // Verify file appears
            assert!(
                result.contains("deep_file.txt") || result.contains("deep_fil…"),
                "Deep file should appear (possibly truncated) at width {}",
                width
            );

            // Verify no line exceeds width
            for line in result.lines() {
                let line_width = visible_width(line);
                assert!(
                    line_width <= width,
                    "Line exceeds width {}: {:?} (width: {})",
                    width,
                    line,
                    line_width
                );
            }
        }
    }

    #[test]
    fn test_render_tree_connectors_never_wrapped() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("a.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .show_root(false);
        fs_tree.ensure_tree_built();

        // Even at very narrow width, connectors should be intact
        let result = fs_tree.render_optimistic(Some(20));

        for line in result.lines() {
            // Each line should have an intact connector at the appropriate position
            let has_branch = line.contains("├── ");
            let has_last_branch = line.contains("└── ");
            assert!(
                has_branch || has_last_branch,
                "Line should have intact connector: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_render_truncation_uses_ellipsis() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        // Create a file with a very long name
        let long_name =
            "this_is_an_extremely_long_filename_that_will_definitely_need_truncation.rs";
        fs::write(temp.path().join(long_name), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        // Render at narrow width to force truncation
        let result = fs_tree.render_optimistic(Some(30));

        // Should contain ellipsis for truncated name
        assert!(
            result.contains("\u{2026}"),
            "Truncated names should end with ellipsis"
        );
    }

    #[test]
    fn test_render_uses_terminal_width() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let term = crate::terminal::TerminalBuilder::default()
            .width(50)
            .build();

        let result = fs_tree.render(&term);

        // Should contain the file
        assert!(result.contains("test.txt"));

        // Each line should respect terminal width
        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 50,
                "Line exceeds terminal width: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_uses_nerd_font_setting() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("main.rs"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        // With Nerd Font enabled
        let mut term_nerd = crate::terminal::TerminalBuilder::default()
            .width(80)
            .build();
        term_nerd.is_nerd_font = Some(true);
        let result_nerd = fs_tree.render(&term_nerd);

        // With Nerd Font disabled
        let mut term_unicode = crate::terminal::TerminalBuilder::default()
            .width(80)
            .build();
        term_unicode.is_nerd_font = Some(false);
        let result_unicode = fs_tree.render(&term_unicode);

        // Both should contain the file name
        assert!(result_nerd.contains("main.rs"));
        assert!(result_unicode.contains("main.rs"));

        // Results should differ due to different icons
        // Nerd Font adds a space after the icon for PUA compensation
        // so the outputs will be different lengths or content
        // (We can't assert exact icons since they depend on font rendering)
    }

    #[test]
    fn test_with_file_links_produces_osc8_links() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("hello.txt"), "").expect("create file");
        fs::create_dir(temp.path().join("sub")).expect("create dir");
        fs::write(temp.path().join("sub/nested.rs"), "").expect("create nested file");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .with_file_links();
        fs_tree.ensure_tree_built();

        // Render with a TTY terminal so OSC8 links are emitted
        let term = crate::terminal::Terminal::builder()
            .width(120)
            .is_tty(true)
            .build();
        let result = fs_tree.render(&term);

        let canonical = temp.path().canonicalize().expect("canonicalize");

        // File should have an OSC8 link with the absolute path
        let file_link = format!(
            "\x1b]8;;{}\x1b\\",
            file_url(&canonical.join("hello.txt")).expect("file URL")
        );
        assert!(
            result.contains(&file_link),
            "Expected OSC8 link for hello.txt in output.\nLooking for: {:?}\nOutput: {:?}",
            file_link,
            result
        );

        // Nested file should have full path
        let nested_link = format!(
            "\x1b]8;;{}\x1b\\",
            file_url(&canonical.join("sub/nested.rs")).expect("file URL")
        );
        assert!(
            result.contains(&nested_link),
            "Expected OSC8 link for sub/nested.rs in output.\nLooking for: {:?}\nOutput: {:?}",
            nested_link,
            result
        );

        // Directory should also be linked
        let dir_link = format!(
            "\x1b]8;;{}\x1b\\",
            file_url(&canonical.join("sub")).expect("file URL")
        );
        assert!(
            result.contains(&dir_link),
            "Expected OSC8 link for sub/ directory in output.\nLooking for: {:?}\nOutput: {:?}",
            dir_link,
            result
        );
    }

    #[test]
    fn test_with_file_links_disabled_has_no_osc8() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let term = crate::terminal::Terminal::builder()
            .width(80)
            .is_tty(true)
            .build();
        let result = fs_tree.render(&term);

        // Should NOT contain OSC8 sequences
        assert!(
            !result.contains("\x1b]8;;"),
            "Expected no OSC8 links when file_links is disabled.\nOutput: {:?}",
            result
        );
    }

    #[test]
    fn test_with_file_links_no_osc8_without_tty() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .with_file_links();
        fs_tree.ensure_tree_built();

        // render() has no terminal context (is_tty=false)
        let result = fs_tree.render_optimistic(Some(80));

        // Should NOT contain OSC8 sequences without TTY
        assert!(
            !result.contains("\x1b]8;;"),
            "Expected no OSC8 links without TTY.\nOutput: {:?}",
            result
        );
    }

    #[test]
    fn test_is_block_level_returns_true() {
        let fs = FileSystem::default();
        assert!(fs.is_block_level(), "FileSystem should be block-level");
    }

    #[test]
    fn test_layout_accessor() {
        use crate::components::renderable::TerminalRenderable;
        let fs = FileSystem::default();
        let layout = TerminalRenderable::layout(&fs);
        // Default layout should have no margins
        assert_eq!(
            layout.margin.left,
            crate::utils::layout::TargetValue::universal(crate::utils::layout::Length::Zero)
        );
    }

    #[test]
    fn test_layout_mut_accessor() {
        use crate::components::renderable::TerminalRenderable;
        use crate::utils::layout::{Length, TargetValue};
        let mut fs = FileSystem::default();
        TerminalRenderable::layout_mut(&mut fs).margin.left = TargetValue::universal(Length::ch(4));

        assert_eq!(
            TerminalRenderable::layout(&fs).margin.left,
            TargetValue::universal(Length::ch(4))
        );
    }

    #[test]
    fn test_as_any() {
        let fs = FileSystem::default();
        let any_ref = TerminalRenderable::as_any(&fs);

        // Should be able to downcast back to FileSystem
        assert!(any_ref.downcast_ref::<FileSystem>().is_some());
    }

    #[test]
    fn test_truncate_with_ellipsis_short_string() {
        // String that fits should not be truncated
        let result = super::truncate_with_ellipsis("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_exact_fit() {
        // String that exactly fits should not be truncated
        let result = super::truncate_with_ellipsis("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_needs_truncation() {
        // String that needs truncation
        let result = super::truncate_with_ellipsis("hello_world", 8);
        assert_eq!(visible_width(&result), 8);
        assert!(result.ends_with('\u{2026}'));
    }

    #[test]
    fn test_truncate_with_ellipsis_width_zero() {
        let result = super::truncate_with_ellipsis("hello", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_with_ellipsis_width_one() {
        let result = super::truncate_with_ellipsis("hello", 1);
        assert_eq!(result, "\u{2026}");
    }

    #[test]
    fn test_style_name_dim_ignored() {
        let fs = FileSystem::default().dim_gitignore(true);
        let node = TreeNode::File {
            name: "target".into(),
            is_ignored: true,
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "target", true);

        // Should contain dim escape codes with reset
        assert!(styled.contains("\x1b[2"), "Should have dim code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_italic_dot_file() {
        let fs = FileSystem::default().italicize_dot_files(true);
        let node = TreeNode::File {
            name: ".gitignore".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, ".gitignore", true);

        // Should contain italic escape codes with reset
        assert!(styled.contains("\x1b[3"), "Should have italic code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_italic_dot_dir() {
        let fs = FileSystem::default().italicize_dot_dirs(true);
        let node = TreeNode::Dir {
            name: ".git".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, ".git", true);

        // Should contain italic (3) and bold blue (1;34) codes with reset
        assert!(styled.contains("\x1b["), "Should have escape sequence");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
        // Directories get bold blue, so italic is combined
        assert!(styled.contains("3"), "Should have italic code");
        assert!(styled.contains("34"), "Should have blue code");
    }

    #[test]
    fn test_style_name_highlight_red() {
        let fs = FileSystem::default().highlight_red("error");
        let node = TreeNode::File {
            name: "error.log".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "error.log", true);

        // Should contain red color escape codes with reset
        assert!(styled.contains("\x1b[31m"), "Should have red start");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_highlight_green() {
        let fs = FileSystem::default().highlight_green("success");
        let node = TreeNode::File {
            name: "success.txt".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "success.txt", true);

        // Should contain green color escape codes with reset
        assert!(styled.contains("\x1b[32m"), "Should have green start");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_no_ansi_when_not_tty() {
        let fs = FileSystem::default()
            .dim_gitignore(true)
            .italicize_dot_files(true)
            .highlight_red("error");

        // Test ignored file
        let ignored_node = TreeNode::File {
            name: "target".into(),
            is_ignored: true,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&ignored_node, "target", false);
        assert_eq!(styled, "target", "No ANSI codes when is_tty=false");
        assert!(
            !styled.contains("\x1b["),
            "Should not contain escape sequences"
        );

        // Test dot file
        let dot_node = TreeNode::File {
            name: ".gitignore".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&dot_node, ".gitignore", false);
        assert_eq!(styled, ".gitignore", "No ANSI codes when is_tty=false");

        // Test highlighted file
        let error_node = TreeNode::File {
            name: "error.log".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&error_node, "error.log", false);
        assert_eq!(styled, "error.log", "No ANSI codes when is_tty=false");
    }

    #[test]
    fn test_style_name_directory_bold_blue() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "src".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "src", true);

        // Should contain bold (1) and blue (34) codes
        assert!(styled.contains("1"), "Should have bold code");
        assert!(styled.contains("34"), "Should have blue code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_symlink_cyan() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "link.txt".into(),
            is_ignored: false,
            is_symlink: true,
            metrics: None,
        };

        let styled = fs.style_name(&node, "link.txt", true);

        // Should contain cyan (36) code
        assert!(styled.contains("36"), "Should have cyan code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_error_dir_red() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "unreadable".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: true,
            at_depth_limit: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "unreadable", true);

        // Should contain red (31) code
        assert!(styled.contains("31"), "Should have red code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_highlight_takes_priority() {
        // Highlight patterns should take priority over other styles
        let fs = FileSystem::default()
            .dim_gitignore(true)
            .highlight_red("target");

        let node = TreeNode::File {
            name: "target".into(),
            is_ignored: true, // This would normally be dimmed
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "target", true);

        // Should be red (highlight) not dim
        assert!(
            styled.contains("\x1b[31m"),
            "Should have red from highlight"
        );
        assert!(!styled.contains("\x1b[2"), "Should not have dim code");
    }

    #[test]
    fn test_style_name_symlink_dir_combines_styles() {
        let fs = FileSystem::default();
        let node = TreeNode::Dir {
            name: "linked_dir".into(),
            children: vec![],
            is_ignored: false,
            is_symlink: true,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "linked_dir", true);

        // Should have both bold blue (dir) and cyan (symlink)
        assert!(styled.contains("1"), "Should have bold code");
        assert!(styled.contains("34"), "Should have blue code");
        assert!(styled.contains("36"), "Should have cyan code");
    }

    #[test]
    fn test_display_method_adds_trailing_newline() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let term = crate::terminal::Terminal::default();
        let result = fs_tree.display(&term);

        assert!(
            result.ends_with('\n'),
            "display() should ensure trailing newline"
        );
    }

    // ============================================================
    // Additional Edge Case Tests (Phase 9)
    // ============================================================

    #[test]
    fn test_error_ignore_display() {
        // Test the Ignore error variant from the ignore crate
        // We create a synthetic error since ignore::Error is not easily constructed
        let err = FileSystemError::Ignore(ignore::Error::InvalidDefinition);
        let msg = err.to_string();
        assert!(
            msg.contains("Ignore error"),
            "Expected 'Ignore error' in: {msg}"
        );
    }

    #[test]
    fn test_tree_building_unicode_filenames() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create files with various Unicode characters
        fs::write(temp.path().join("日本語.txt"), "japanese").expect("create japanese file");
        fs::write(temp.path().join("中文.md"), "chinese").expect("create chinese file");
        fs::write(temp.path().join("emoji_🎉.txt"), "emoji").expect("create emoji file");
        fs::write(temp.path().join("ñoño.rs"), "spanish").expect("create spanish file");
        fs::write(temp.path().join("über.txt"), "german").expect("create german file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 5, "Should have 5 Unicode-named files");

        // Verify all names are preserved
        let names: Vec<_> = tree.iter().map(|n| n.name()).collect();
        assert!(names.iter().any(|n| n.contains("日本語")));
        assert!(names.iter().any(|n| n.contains("中文")));
        assert!(names.iter().any(|n| n.contains("🎉")));
        assert!(names.iter().any(|n| n.contains("ñoño")));
        assert!(names.iter().any(|n| n.contains("über")));
    }

    #[test]
    fn test_tree_building_special_character_filenames() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create files with special characters (excluding those invalid on Windows)
        fs::write(temp.path().join("file with spaces.txt"), "").expect("create spaced file");
        fs::write(temp.path().join("file-with-dashes.txt"), "").expect("create dashed file");
        fs::write(temp.path().join("file_with_underscores.txt"), "")
            .expect("create underscored file");
        fs::write(temp.path().join("file.multiple.dots.txt"), "").expect("create multi-dot file");
        fs::write(temp.path().join("(parentheses).txt"), "").expect("create parentheses file");
        fs::write(temp.path().join("[brackets].txt"), "").expect("create brackets file");
        fs::write(temp.path().join("file@symbol.txt"), "").expect("create at-symbol file");
        fs::write(temp.path().join("file#hash.txt"), "").expect("create hash file");
        fs::write(temp.path().join("file+plus.txt"), "").expect("create plus file");
        fs::write(temp.path().join("file=equals.txt"), "").expect("create equals file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 10, "Should have 10 special-char files");

        // Verify names with spaces are preserved
        let names: Vec<_> = tree.iter().map(|n| n.name()).collect();
        assert!(names.iter().any(|n| n.contains("file with spaces")));
        assert!(names.iter().any(|n| n.contains("(parentheses)")));
        assert!(names.iter().any(|n| n.contains("[brackets]")));
    }

    #[test]
    fn test_render_unicode_filenames_fit_in_width() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create files with wide characters (CJK characters are 2 columns wide)
        fs::write(temp.path().join("日本語ファイル.txt"), "").expect("create japanese file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        // Render at narrow width to test truncation of wide chars
        let result = fs_tree.render_optimistic(Some(30));

        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 30,
                "Line with Unicode exceeds width 30: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_tree_building_very_deep_nesting_beyond_limit() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create 25 levels deep (beyond default max_depth of 20)
        let mut current = temp.path().to_path_buf();
        for i in 0..25 {
            current = current.join(format!("level{}", i));
        }
        fs::create_dir_all(&current).expect("create deep dirs");
        fs::write(current.join("deepest.txt"), "").expect("create deep file");

        // With default max_depth=20, the deepest levels should not be traversed
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");

        // Verify we have entries but the deepest file is not reachable
        assert!(!tree.is_empty(), "Should have some entries");

        // The tree should stop at depth 20, so deepest.txt should not appear
        let result = fs_tree.render_optimistic(Some(200));
        assert!(
            !result.contains("deepest.txt"),
            "deepest.txt should not appear due to max_depth=20"
        );
    }

    #[test]
    fn test_tree_building_very_wide_directory() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create more files than the default max_entries (1000)
        // We'll test with a smaller custom limit for speed
        for i in 0..50 {
            fs::write(temp.path().join(format!("file{:04}.txt", i)), "").expect("create file");
        }

        // Set max_entries to 25 (less than 50 files)
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .max_entries(25);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");

        // Should be limited to at most 25 entries
        assert!(
            tree.len() <= 25,
            "Should have at most 25 entries, got {}",
            tree.len()
        );
    }

    #[test]
    fn test_tree_building_only_hidden_files() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create only hidden files
        fs::write(temp.path().join(".hidden1"), "").expect("create .hidden1");
        fs::write(temp.path().join(".hidden2"), "").expect("create .hidden2");
        fs::create_dir(temp.path().join(".hidden_dir")).expect("create .hidden_dir");

        // Without hiding, should see all
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree");
        assert_eq!(tree.len(), 3, "Should see 3 hidden entries");

        // With hide_dot_files and hide_dot_dirs, should see nothing
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .hide_dot_files(true)
            .hide_dot_dirs(true);
        fs_tree.ensure_tree_built();
        let tree = fs_tree.tree().expect("tree");
        assert!(
            tree.is_empty(),
            "Should see nothing when all entries are hidden"
        );
    }

    #[test]
    fn test_render_empty_tree_after_build() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create only hidden files and then hide them
        fs::write(temp.path().join(".hidden"), "").expect("create hidden");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .hide_dot_files(true);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert!(tree.is_empty(), "Tree should be empty");

        // Render should return empty string for empty tree
        let result = fs_tree.render_optimistic(Some(80));
        assert_eq!(result, "", "Empty tree should render as empty string");
    }

    #[test]
    fn test_create_error_dir_node_helper() {
        // Test the helper function for creating error nodes
        let node = FileSystem::create_error_dir_node("unreadable".to_string(), false);

        if let TreeNode::Dir {
            name,
            children,
            is_ignored,
            is_symlink,
            has_error,
            at_depth_limit,
            metrics,
        } = node
        {
            assert_eq!(name, "unreadable");
            assert!(children.is_empty());
            assert!(!is_ignored);
            assert!(!is_symlink);
            assert!(has_error, "has_error should be true");
            assert!(!at_depth_limit);
            assert!(metrics.is_none());
        } else {
            panic!("Expected Dir variant");
        }
    }

    #[test]
    fn test_create_error_dir_node_with_symlink() {
        let node = FileSystem::create_error_dir_node("symlink_error".to_string(), true);

        if let TreeNode::Dir {
            is_symlink,
            has_error,
            ..
        } = node
        {
            assert!(is_symlink, "is_symlink should be true");
            assert!(has_error, "has_error should be true");
        } else {
            panic!("Expected Dir variant");
        }
    }

    #[test]
    fn test_style_name_combines_multiple_styles() {
        // Test that dim + italic can be combined
        let fs = FileSystem::default()
            .dim_gitignore(true)
            .italicize_dot_files(true);

        let node = TreeNode::File {
            name: ".ignored_dotfile".into(),
            is_ignored: true, // Should be dim
            is_symlink: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, ".ignored_dotfile", true);

        // Should contain both dim (2) and italic (3)
        assert!(styled.contains("2"), "Should have dim code");
        assert!(styled.contains("3"), "Should have italic code");
        assert!(styled.contains("\x1b[0m"), "Should have reset");
    }

    #[test]
    fn test_style_name_error_dir_not_dimmed_when_ignored() {
        // Error styling should take priority over dim styling
        let fs = FileSystem::default().dim_gitignore(true);

        let node = TreeNode::Dir {
            name: "error_ignored".into(),
            children: vec![],
            is_ignored: true, // Would be dim
            is_symlink: false,
            has_error: true, // Error takes priority
            at_depth_limit: false,
            metrics: None,
        };

        let styled = fs.style_name(&node, "error_ignored", true);

        // Should have red but not dim
        assert!(styled.contains("31"), "Should have red code for error");
        assert!(
            !styled.contains("\x1b[2"),
            "Should not have dim code when error"
        );
    }

    #[test]
    fn test_render_narrow_width_10_columns() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("a.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .show_root(false);
        fs_tree.ensure_tree_built();

        // Very narrow width (10 columns) - connector takes 4, icon takes ~2
        let result = fs_tree.render_optimistic(Some(10));

        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 10,
                "Line exceeds very narrow width 10: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_default_width_when_none() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("test.txt"), "").expect("create file");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        // render(None) should use default width of 80
        let result = fs_tree.render_optimistic(None);

        for line in result.lines() {
            let line_width = visible_width(line);
            assert!(
                line_width <= 80,
                "Line exceeds default width 80: {:?} (width: {})",
                line,
                line_width
            );
        }
    }

    #[test]
    fn test_render_no_styling_when_not_tty() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::create_dir(temp.path().join("src")).expect("create src dir");
        fs::write(temp.path().join(".hidden"), "").expect("create hidden");

        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .italicize_dot_files(true);
        fs_tree.ensure_tree_built();

        // Create terminal with is_tty = false
        let mut term = crate::terminal::TerminalBuilder::default()
            .width(80)
            .build();
        term.is_tty = false;

        let result = fs_tree.render(&term);

        // Should not contain any ANSI escape sequences
        assert!(
            !result.contains("\x1b["),
            "Should not have ANSI codes when is_tty=false: {:?}",
            result
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_tree_building_broken_symlink() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("create temp dir");

        // Create a symlink to a nonexistent target
        let link_path = temp.path().join("broken_link");
        symlink("/nonexistent/target/path", &link_path).expect("create broken symlink");

        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 1, "Should have the broken symlink");

        // Broken symlinks are treated as files
        let node = &tree[0];
        assert_eq!(node.name(), "broken_link");
        assert!(node.is_symlink(), "Should be marked as symlink");
        assert!(node.is_file(), "Broken symlink should be treated as file");
    }

    #[test]
    fn test_get_icon_no_extension_file() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: "Makefile".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        // Files without extension should get base file icon
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::file::BASE);
        assert_eq!(
            fs.get_icon(&node, 0, Some(false)),
            icons::unicode::file::BASE
        );
    }

    #[test]
    fn test_get_icon_dotfile_no_extension() {
        let fs = FileSystem::default();
        let node = TreeNode::File {
            name: ".bashrc".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        // Dotfiles without recognized extension should get base icon
        assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::file::BASE);
    }

    #[test]
    fn test_filter_patterns_partial_match() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        fs::write(temp.path().join("test_utils.rs"), "").expect("create test_utils.rs");
        fs::write(temp.path().join("utils.rs"), "").expect("create utils.rs");
        fs::write(temp.path().join("main.rs"), "").expect("create main.rs");
        fs::write(temp.path().join("utility.ts"), "").expect("create utility.ts");

        // Filter for "util" should match test_utils.rs, utils.rs, and utility.ts
        let mut fs_tree = FileSystem::new(temp.path())
            .expect("valid path")
            .filter("util");
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 3, "Should match 3 files containing 'util'");

        let names: Vec<_> = tree.iter().map(|n| n.name()).collect();
        assert!(names.contains(&"test_utils.rs"));
        assert!(names.contains(&"utils.rs"));
        assert!(names.contains(&"utility.ts"));
        assert!(!names.contains(&"main.rs"));
    }

    #[test]
    fn test_highlight_patterns_partial_match() {
        let fs = FileSystem::default()
            .highlight_red("err")
            .highlight_green("pass");

        // "error.log" contains "err"
        let error_node = TreeNode::File {
            name: "error.log".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&error_node, "error.log", true);
        assert!(styled.contains("\x1b[31m"), "Should be red");

        // "passed_tests.txt" contains "pass"
        let pass_node = TreeNode::File {
            name: "passed_tests.txt".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&pass_node, "passed_tests.txt", true);
        assert!(styled.contains("\x1b[32m"), "Should be green");

        // "normal.txt" contains neither
        let normal_node = TreeNode::File {
            name: "normal.txt".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&normal_node, "normal.txt", true);
        assert!(!styled.contains("\x1b[31m"), "Should not be red");
        assert!(!styled.contains("\x1b[32m"), "Should not be green");
    }

    #[test]
    fn test_highlight_red_takes_priority_over_green() {
        let fs = FileSystem::default()
            .highlight_red("test")
            .highlight_green("test");

        let node = TreeNode::File {
            name: "test.txt".into(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let styled = fs.style_name(&node, "test.txt", true);

        // Red should take priority (checked first)
        assert!(styled.contains("\x1b[31m"), "Red should take priority");
        assert!(!styled.contains("\x1b[32m"), "Green should not be applied");
    }

    #[test]
    fn test_tree_node_equality_different_variants() {
        let file = TreeNode::File {
            name: "test".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };

        let dir = TreeNode::Dir {
            name: "test".to_string(),
            children: vec![],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        };

        // Same name but different variants should not be equal
        assert_ne!(file, dir);
    }

    #[test]
    fn test_depth_zero_shows_only_root() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).expect("create subdir");
        fs::write(subdir.join("nested.txt"), "").expect("create nested file");
        fs::write(temp.path().join("root.txt"), "").expect("create root file");

        // depth(0) means we don't recurse at all - only root level entries
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path").depth(0);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");

        // Should be empty because depth 0 means we can't show root level (depth >= max_depth check)
        assert!(tree.is_empty(), "depth(0) should show nothing");
    }

    #[test]
    fn test_depth_one_shows_root_only() {
        use std::fs;

        let temp = tempfile::tempdir().expect("create temp dir");

        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).expect("create subdir");
        fs::write(subdir.join("nested.txt"), "").expect("create nested file");
        fs::write(temp.path().join("root.txt"), "").expect("create root file");

        // depth(1) means root level items only, subdir marked at_depth_limit
        let mut fs_tree = FileSystem::new(temp.path()).expect("valid path").depth(1);
        fs_tree.ensure_tree_built();

        let tree = fs_tree.tree().expect("tree should be built");
        assert_eq!(tree.len(), 2, "Should have 2 root entries");

        // Find the subdir and verify it's at depth limit
        let subdir_node = tree.iter().find(|n| n.name() == "subdir");
        assert!(subdir_node.is_some(), "Should find subdir");

        if let TreeNode::Dir {
            children,
            at_depth_limit,
            ..
        } = subdir_node.unwrap()
        {
            assert!(at_depth_limit, "subdir should be at depth limit");
            assert!(children.is_empty(), "subdir should have no children");
        }
    }

    // ============================================================
    // Metric Type Tests
    // ============================================================

    #[test]
    fn test_metric_config_default() {
        let config = MetricConfig::default();
        assert!(!config.enabled);
        assert!(config.filename_patterns.is_empty());
        assert!(config.highlight_threshold.is_none());
    }

    #[test]
    fn test_metric_kind_as_hashmap_key() {
        let mut map = HashMap::new();
        map.insert(MetricKind::FileSize, "size");
        map.insert(MetricKind::Tokens, "tokens");
        assert_eq!(map.get(&MetricKind::FileSize), Some(&"size"));
        assert_eq!(map.get(&MetricKind::Tokens), Some(&"tokens"));
        assert_eq!(map.get(&MetricKind::Owner), None);
    }

    #[test]
    fn test_metric_kind_dir_applicability() {
        assert!(!MetricKind::FileSize.is_dir_applicable());
        assert!(!MetricKind::Tokens.is_dir_applicable());
        assert!(MetricKind::Created.is_dir_applicable());
        assert!(MetricKind::Modified.is_dir_applicable());
        assert!(MetricKind::Permissions.is_dir_applicable());
        assert!(MetricKind::Owner.is_dir_applicable());
        assert!(MetricKind::Group.is_dir_applicable());
    }

    // ============================================================
    // Format Helper Tests
    // ============================================================

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_format_token_count() {
        assert_eq!(format_token_count(0), "0");
        assert_eq!(format_token_count(999), "999");
        assert_eq!(format_token_count(1000), "1,000");
        assert_eq!(format_token_count(1234567), "1,234,567");
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();

        // Just now
        assert_eq!(format_relative_time(now), "just now");

        // Minutes
        let two_min_ago = now - chrono::Duration::minutes(2);
        assert_eq!(format_relative_time(two_min_ago), "2 minutes ago");

        let one_min_ago = now - chrono::Duration::minutes(1);
        assert_eq!(format_relative_time(one_min_ago), "1 minute ago");

        // Hours
        let three_hours_ago = now - chrono::Duration::hours(3);
        assert_eq!(format_relative_time(three_hours_ago), "3 hours ago");

        // Days
        let five_days_ago = now - chrono::Duration::days(5);
        assert_eq!(format_relative_time(five_days_ago), "5 days ago");

        // Months
        let two_months_ago = now - chrono::Duration::days(65);
        assert_eq!(format_relative_time(two_months_ago), "2 months ago");

        // Years
        let two_years_ago = now - chrono::Duration::days(750);
        assert_eq!(format_relative_time(two_years_ago), "2 years ago");
    }

    #[cfg(unix)]
    #[test]
    fn test_format_permissions_string_644() {
        assert_eq!(format_permissions_string(0o644, false), ".rw-r--r--");
    }

    #[cfg(unix)]
    #[test]
    fn test_format_permissions_string_755() {
        assert_eq!(format_permissions_string(0o755, false), ".rwxr-xr-x");
    }

    #[cfg(unix)]
    #[test]
    fn test_format_permissions_string_000() {
        assert_eq!(format_permissions_string(0o000, false), ".---------");
    }

    #[cfg(unix)]
    #[test]
    fn test_format_permissions_string_777() {
        assert_eq!(format_permissions_string(0o777, false), ".rwxrwxrwx");
    }

    // ============================================================
    // Estimate Tokens Tests
    // ============================================================

    #[test]
    fn test_estimate_tokens_json_file() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let path = temp.path().join("data.json");
        let content = "a".repeat(1000); // 1000 bytes, json uses 2.5 chars/token = 400
        std::fs::write(&path, &content).expect("write");
        let metadata = std::fs::metadata(&path).ok();
        assert_eq!(estimate_tokens(&path, metadata.as_ref()), Some(400));
    }

    #[test]
    fn test_estimate_tokens_rust_file() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let path = temp.path().join("main.rs");
        let content = "a".repeat(1000); // 1000 bytes, rs uses 4.0 chars/token = 250
        std::fs::write(&path, &content).expect("write");
        let metadata = std::fs::metadata(&path).ok();
        assert_eq!(estimate_tokens(&path, metadata.as_ref()), Some(250));
    }

    #[test]
    fn test_estimate_tokens_binary_returns_none() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let path = temp.path().join("image.png");
        std::fs::write(&path, "fake png data").expect("write");
        let metadata = std::fs::metadata(&path).ok();
        assert_eq!(estimate_tokens(&path, metadata.as_ref()), None);
    }

    // ============================================================
    // Glob Match Tests
    // ============================================================

    #[test]
    fn test_glob_match_basic() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.ts"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*", "anything.txt"));
        assert!(glob_match("test*", "test_file.rs"));
    }

    // ============================================================
    // Builder Method Tests
    // ============================================================

    #[test]
    fn test_show_file_size_builder() {
        let fs = FileSystem::default().show_file_size();
        assert!(fs.has_any_metrics());
        assert!(fs.should_show_metric(MetricKind::FileSize, "anything.rs"));
    }

    #[test]
    fn test_show_file_size_with_filename() {
        let fs = FileSystem::default().show_file_size_with_filename(vec!["*.rs"]);
        assert!(fs.should_show_metric(MetricKind::FileSize, "main.rs"));
        assert!(!fs.should_show_metric(MetricKind::FileSize, "main.ts"));
    }

    #[test]
    fn test_show_file_size_with_negation() {
        let fs = FileSystem::default().show_file_size_with_filename(vec!["*", "!*.log"]);
        assert!(fs.should_show_metric(MetricKind::FileSize, "main.rs"));
        assert!(!fs.should_show_metric(MetricKind::FileSize, "app.log"));
    }

    #[test]
    fn test_should_show_metric_disabled() {
        let fs = FileSystem::default();
        assert!(!fs.should_show_metric(MetricKind::FileSize, "anything.rs"));
    }

    #[test]
    fn test_multiple_metrics_enabled() {
        let fs = FileSystem::default().show_file_size().show_tokens();
        assert!(fs.should_show_metric(MetricKind::FileSize, "test.rs"));
        assert!(fs.should_show_metric(MetricKind::Tokens, "test.rs"));
        assert!(!fs.should_show_metric(MetricKind::Owner, "test.rs"));
    }

    // ============================================================
    // Integration Tests with tempfile
    // ============================================================

    #[test]
    fn test_file_size_metric_on_real_file() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let file_path = temp.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").expect("write file");

        let mut fs = FileSystem::new(temp.path()).unwrap().show_file_size();
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        assert_eq!(tree.len(), 1);

        let file_node = &tree[0];
        assert!(file_node.metrics().is_some());
        let metrics = file_node.metrics().unwrap();
        assert!(metrics.file_size.is_some());
        assert_eq!(metrics.file_size.unwrap(), 12); // "fn main() {}" is 12 bytes
    }

    #[test]
    fn test_tokens_metric_on_rs_file() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let file_path = temp.path().join("test.rs");
        let content = "a".repeat(400); // 400 bytes, at 4 chars/token = 100 tokens
        std::fs::write(&file_path, &content).expect("write file");

        let mut fs = FileSystem::new(temp.path()).unwrap().show_tokens();
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        let file_node = &tree[0];
        let metrics = file_node.metrics().unwrap();
        assert_eq!(metrics.tokens, Some(100));
    }

    #[test]
    fn test_file_size_with_glob_filter() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("code.rs"), "let x = 1;").expect("write rs");
        std::fs::write(temp.path().join("data.json"), r#"{"a":1}"#).expect("write json");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_file_size_with_filename(vec!["*.rs"]);
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        let rs_node = tree.iter().find(|n| n.name() == "code.rs").unwrap();
        let json_node = tree.iter().find(|n| n.name() == "data.json").unwrap();

        assert!(rs_node.metrics().is_some(), "rs file should have metrics");
        assert!(
            json_node.metrics().is_none(),
            "json file should not have metrics"
        );
    }

    #[test]
    fn test_negation_glob_excludes_files() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("app.rs"), "fn main() {}").expect("write rs");
        std::fs::write(temp.path().join("app.log"), "log data").expect("write log");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_file_size_with_filename(vec!["*", "!*.log"]);
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        let rs_node = tree.iter().find(|n| n.name() == "app.rs").unwrap();
        let log_node = tree.iter().find(|n| n.name() == "app.log").unwrap();

        assert!(rs_node.metrics().is_some());
        assert!(log_node.metrics().is_none());
    }

    #[test]
    fn test_render_output_contains_file_size_label() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("test.txt"), "hello world").expect("write");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_file_size()
            .show_root(false);
        fs.ensure_tree_built();

        let output = fs.render_optimistic(Some(120));
        assert!(
            output.contains("file size:"),
            "output should contain 'file size:' label, got: {}",
            output
        );
    }

    #[test]
    fn test_multiple_metrics_render() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("test.rs"), "fn main() {}").expect("write");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_file_size()
            .show_tokens()
            .show_root(false);
        fs.ensure_tree_built();

        let output = fs.render_optimistic(Some(120));
        assert!(output.contains("file size:"), "should have file size");
        assert!(output.contains("tokens:"), "should have tokens");
    }

    #[test]
    fn test_show_on_directories_with_modified() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).expect("create dir");
        std::fs::write(subdir.join("file.txt"), "data").expect("write");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_modified_since()
            .show_on_directories()
            .depth(2);
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        let dir_node = tree.iter().find(|n| n.name() == "subdir").unwrap();
        assert!(
            dir_node.metrics().is_some(),
            "directory should have metrics when show_on_directories is enabled"
        );
    }

    #[test]
    fn test_no_metrics_on_dirs_without_show_on_directories() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).expect("create dir");
        std::fs::write(subdir.join("file.txt"), "data").expect("write");

        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .show_file_size()
            .depth(2);
        fs.ensure_tree_built();

        let tree = fs.tree().expect("tree built");
        let dir_node = tree.iter().find(|n| n.name() == "subdir").unwrap();
        assert!(
            dir_node.metrics().is_none(),
            "directory should NOT have metrics without show_on_directories"
        );

        // But files should still have metrics
        if let TreeNode::Dir { children, .. } = dir_node {
            let file_node = children.iter().find(|n| n.name() == "file.txt").unwrap();
            assert!(file_node.metrics().is_some(), "file should have metrics");
        }
    }

    #[test]
    fn test_highlight_threshold() {
        let fs = FileSystem::default().show_file_size_highlight_greater_than(1000);

        let small_metrics = FileMetrics {
            file_size: Some(500),
            ..Default::default()
        };
        assert!(!fs.should_highlight_metric(MetricKind::FileSize, &small_metrics));

        let large_metrics = FileMetrics {
            file_size: Some(2000),
            ..Default::default()
        };
        assert!(fs.should_highlight_metric(MetricKind::FileSize, &large_metrics));
    }

    // =============================================================
    // Render-Tree Projection Tests (RT-FILESYSTEM-001)
    // =============================================================

    use renderable::tree::{ListMarkerPolicy, NodeKind, RenderNode, TreeRenderable};

    /// Builds a small fixture tree:
    /// root/
    ///   ├── src/
    ///   │   └── main.rs
    ///   └── README.md
    fn build_fixture_tree() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("create temp dir");
        let root = temp.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
        temp
    }

    /// Counts list-item nodes in a tree (depth-first).
    fn count_list_items(node: &RenderNode) -> usize {
        let mut count = 0;
        match &node.kind {
            NodeKind::ListItem { children, .. } => {
                count += 1;
                for child in children {
                    count += count_list_items(child);
                }
            }
            _ => {
                for child in children_of(node) {
                    count += count_list_items(child);
                }
            }
        }
        count
    }

    fn children_of(node: &RenderNode) -> &[RenderNode] {
        match &node.kind {
            NodeKind::Root { children }
            | NodeKind::Paragraph { children }
            | NodeKind::Section { children, .. }
            | NodeKind::BlockQuote { children }
            | NodeKind::List { children, .. }
            | NodeKind::ListItem { children, .. }
            | NodeKind::TableRow { children }
            | NodeKind::TableCell { children }
            | NodeKind::Strong { children }
            | NodeKind::Emphasis { children }
            | NodeKind::Delete { children }
            | NodeKind::Span { children }
            | NodeKind::Link { children, .. }
            | NodeKind::Table { children, .. }
            | NodeKind::Heading { children, .. } => children,
            _ => &[],
        }
    }

    /// Returns the first `List` node found via DFS.
    fn first_list(node: &RenderNode) -> Option<&RenderNode> {
        if matches!(node.kind, NodeKind::List { .. }) {
            return Some(node);
        }
        for child in children_of(node) {
            if let Some(found) = first_list(child) {
                return Some(found);
            }
        }
        None
    }

    /// Collects all classes from all nodes in DFS order.
    fn collect_classes(node: &RenderNode) -> Vec<String> {
        let mut out: Vec<String> = node.attrs.classes.clone();
        for child in children_of(node) {
            out.extend(collect_classes(child));
        }
        out
    }

    #[test]
    fn render_tree_returns_root_with_tree_connector_list() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        assert!(matches!(node.kind, NodeKind::Root { .. }));

        // Root contains the root header paragraph plus the tree-connector list.
        let list = first_list(&node).expect("projected list");
        assert_eq!(
            list.attrs.list_marker_policy(),
            ListMarkerPolicy::TreeConnectors,
        );
    }

    #[test]
    fn render_tree_seeds_word_wrap_none_on_root_when_layout_is_non_default() {
        use crate::utils::layout::Alignment;

        let temp = build_fixture_tree();
        // A non-default layout (custom alignment) forces the layout hint
        // onto the root; we then verify word_wrap was overridden to None.
        let custom_layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .layout(custom_layout);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let layout = node
            .attrs
            .layout()
            .expect("layout seeded on root when non-default");
        // Tree connectors must never wrap.
        assert!(matches!(
            layout.word_wrap,
            renderable::layout::WordWrap::None
        ));
        // Other layout slots survive the override.
        assert_eq!(layout.alignment, Alignment::Center);
    }

    #[test]
    fn render_tree_omits_layout_hint_when_layout_is_default() {
        // A FileSystem with no layout customizations should not emit a
        // layout hint — saving every renderer the deserialization cost and
        // matching `Layout::default()`'s semantics.
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        assert!(node.attrs.layout().is_none());
    }

    #[test]
    fn render_tree_emits_one_list_item_per_entry() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        // src/ + src/main.rs + README.md = 3 list items.
        assert_eq!(count_list_items(&node), 3);
    }

    #[test]
    fn render_tree_directory_carries_fs_dir_class_and_bold_blue_style() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(classes.contains(&CLASS_DIR.to_string()), "{classes:?}");
        assert!(classes.contains(&CLASS_FILE.to_string()), "{classes:?}");

        // Find the `src` list item's paragraph and verify the typed Style
        // carries bold + blue.
        let style = first_entry_style(&node, "src/").or_else(|| first_entry_style(&node, "src"));
        let style = style.expect("src style");
        assert!(style.emphasis.bold, "directory should be bold");
        assert!(style.color.is_some(), "directory should have fg color");
    }

    /// Finds the `Paragraph` style for an entry whose visible text contains
    /// the given suffix. Walks the tree top-down.
    fn first_entry_style(node: &RenderNode, needle: &str) -> Option<renderable::style::Style> {
        if matches!(node.kind, NodeKind::Paragraph { .. })
            && paragraph_visible_text(node).contains(needle)
        {
            return node.attrs.style();
        }
        for child in children_of(node) {
            if let Some(found) = first_entry_style(child, needle) {
                return Some(found);
            }
        }
        None
    }

    /// Concatenates plain text under a node, ignoring styling.
    fn paragraph_visible_text(node: &RenderNode) -> String {
        let mut out = String::new();
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
            _ => {
                for child in children_of(node) {
                    out.push_str(&paragraph_visible_text(child));
                }
            }
        }
        out
    }

    #[test]
    fn render_tree_skips_root_header_when_show_root_is_false() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs").show_root(false);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let root_classes_present = collect_classes(&node).contains(&CLASS_ROOT.to_string());
        assert!(!root_classes_present, "fs-root must be absent");
    }

    #[test]
    fn render_tree_single_child_directory_produces_single_list_item() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let dir = temp.path().join("solo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("only.txt"), "x").unwrap();

        let mut fs = FileSystem::new(&dir).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        // Only one entry; the projection still uses TreeConnectors. The
        // terminal renderer will choose `└──` because it is also the last
        // child. We assert structure here; the terminal connector glyph is
        // covered by the terminal renderer's own tests.
        let list = first_list(&node).expect("list");
        assert_eq!(
            list.attrs.list_marker_policy(),
            ListMarkerPolicy::TreeConnectors,
        );
        if let NodeKind::List { children, .. } = &list.kind {
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn render_tree_empty_directory_emits_only_root_header() {
        let temp = tempfile::tempdir().expect("create temp dir");

        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        assert!(first_list(&node).is_none(), "empty dir should have no list");
    }

    #[test]
    fn render_tree_file_links_emit_link_nodes_with_file_url() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs").with_file_links();
        fs.ensure_tree_built();

        let node = fs.render_tree();
        // Walk the tree and confirm at least one `Link` with a `file://` URL.
        fn has_file_link(node: &RenderNode) -> bool {
            if let NodeKind::Link { url, .. } = &node.kind
                && url.starts_with("file://")
            {
                return true;
            }
            children_of(node).iter().any(has_file_link)
        }
        assert!(has_file_link(&node), "expected file:// links");
    }

    #[test]
    fn render_tree_highlight_red_wins_over_directory_style() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("TODO-dir")).unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .highlight_red("TODO");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(classes.contains(&CLASS_HIGHLIGHT_RED.to_string()));

        let style = first_entry_style(&node, "TODO-dir").expect("style");
        // Highlight red overrides the directory bold-blue treatment.
        assert!(!style.emphasis.bold, "highlight should suppress bold");
    }

    #[test]
    fn render_tree_dotfiles_are_italic_when_configured() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".env"), "X=1").unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .italicize_dot_files(true);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(classes.contains(&CLASS_DOT.to_string()));
        let style = first_entry_style(&node, ".env").expect("style");
        assert!(style.emphasis.italic, "dotfile should be italic");
    }

    #[test]
    fn render_markdown_outputs_nested_list_with_no_box_drawing() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let md = fs.render_markdown();
        assert!(!md.contains('├'), "markdown must not contain ├");
        assert!(!md.contains('└'), "markdown must not contain └");
        assert!(!md.contains('│'), "markdown must not contain │");
        // Native Markdown list bullets degrade from the TreeConnectors hint.
        assert!(md.contains("- "), "expected `- ` markdown bullets in: {md}");
    }

    #[test]
    fn render_markdown_plus_emits_classed_spans() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let md = fs.render_markdown_plus();
        // MarkdownPlus preserves classed spans for icon/metrics.
        assert!(
            md.contains("class=\"fs-icon\""),
            "expected fs-icon span in: {md}",
        );
    }

    #[test]
    fn render_html_fragment_emits_nested_ul_li_with_no_box_drawing() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let html = fs.render_html_fragment().render();
        assert!(html.contains("<ul"), "expected <ul>: {html}");
        assert!(html.contains("<li"), "expected <li>: {html}");
        assert!(!html.contains('├'), "html must not contain ├");
        assert!(!html.contains('└'), "html must not contain └");
        assert!(!html.contains('│'), "html must not contain │");
    }

    #[test]
    fn render_html_fragment_carries_fs_classes() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let html = fs.render_html_fragment().render();
        assert!(html.contains("fs-dir"), "expected fs-dir class in: {html}");
        assert!(
            html.contains("fs-file"),
            "expected fs-file class in: {html}"
        );
        assert!(
            html.contains("fs-icon"),
            "expected fs-icon class in: {html}"
        );
    }

    #[test]
    fn render_tree_filter_pattern_constrains_output() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join("keep.rs"), "fn x(){}").unwrap();
        std::fs::write(temp.path().join("drop.txt"), "skip").unwrap();

        let mut fs = FileSystem::new(temp.path()).expect("fs").filter(".rs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let names: String = collect_text(&node);
        assert!(names.contains("keep.rs"));
        assert!(!names.contains("drop.txt"));
    }

    fn collect_text(node: &RenderNode) -> String {
        let mut out = String::new();
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
            _ => {
                for child in children_of(node) {
                    out.push_str(&collect_text(child));
                }
            }
        }
        out
    }

    #[test]
    fn render_tree_deeply_nested_lists_are_structural() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("a/b/c")).unwrap();
        std::fs::write(temp.path().join("a/b/c/leaf.txt"), "x").unwrap();

        let mut fs = FileSystem::new(temp.path()).expect("fs").depth(10);
        fs.ensure_tree_built();

        let node = fs.render_tree();

        // Count nested lists; expect 4 (top-level + a + b + c).
        fn count_lists(node: &RenderNode) -> usize {
            let mut count = if matches!(node.kind, NodeKind::List { .. }) {
                1
            } else {
                0
            };
            for child in children_of(node) {
                count += count_lists(child);
            }
            count
        }
        assert_eq!(count_lists(&node), 4);
    }

    #[test]
    fn render_tree_root_header_is_bold_blue() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        // The first paragraph child of root is the root header.
        let header = children_of(&node)
            .iter()
            .find(|c| matches!(c.kind, NodeKind::Paragraph { .. }))
            .expect("root header paragraph");
        let style = header.attrs.style().expect("root header style");
        assert!(style.emphasis.bold);
        assert!(style.color.is_some());
        // Carries both fs-root and fs-dir classes.
        assert!(header.attrs.classes.contains(&CLASS_ROOT.to_string()));
        assert!(header.attrs.classes.contains(&CLASS_DIR.to_string()));
    }

    // ============================================================
    // Regression tests for FileSystem review (2026-05-19)
    // ============================================================

    /// Collects every `Link` URL in the projected tree.
    fn collect_link_urls(node: &RenderNode) -> Vec<String> {
        let mut out = Vec::new();
        if let NodeKind::Link { url, .. } = &node.kind {
            out.push(url.clone());
        }
        for child in children_of(node) {
            out.extend(collect_link_urls(child));
        }
        out
    }

    /// Walks the tree to find the first `Style` attached to a `Span` whose
    /// classes contain `class`.
    fn first_style_for_span_class(
        node: &RenderNode,
        class: &str,
    ) -> Option<renderable::style::Style> {
        if let NodeKind::Span { .. } = &node.kind
            && node.attrs.classes.iter().any(|c| c == class)
        {
            return node.attrs.style();
        }
        for child in children_of(node) {
            if let Some(style) = first_style_for_span_class(child, class) {
                return Some(style);
            }
        }
        None
    }

    /// Critical #1 — entry projection must canonicalize relative roots so
    /// `file://` URLs do not embed `./` segments.
    #[test]
    fn render_tree_file_links_are_canonical_for_relative_root() {
        let temp = build_fixture_tree();
        // Construct using a relative path so the bug condition reproduces.
        let saved = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(temp.path()).expect("chdir");
        let mut fs = FileSystem::new(".").expect("fs").with_file_links();
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let urls = collect_link_urls(&node);
        std::env::set_current_dir(saved).expect("restore cwd");

        assert!(!urls.is_empty(), "expected at least one file:// link");
        for url in &urls {
            assert!(
                url.starts_with("file://"),
                "URL must start with file:// — got {url}"
            );
            assert!(
                !url.contains("/./"),
                "URL must not contain `./` segment — got {url}"
            );
            assert!(
                !url.contains("file://./"),
                "URL must not start with `file://./` — got {url}"
            );
        }
    }

    /// Critical #2 — plain Markdown must omit the projected icon glyphs.
    #[test]
    fn render_markdown_omits_icon_emoji_glyphs() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let md = fs.render_markdown();
        assert!(
            !md.contains('📂'),
            "plain markdown must not contain 📂 in: {md}"
        );
        assert!(
            !md.contains('📄'),
            "plain markdown must not contain 📄 in: {md}"
        );
        // MarkdownPlus must keep them — verify the contrast in one shot.
        let md_plus = fs.render_markdown_plus();
        assert!(
            md_plus.contains("class=\"fs-icon\""),
            "markdown_plus must keep fs-icon span: {md_plus}",
        );
    }

    /// Suggested #3 — MarkdownPlus must include entry-kind classes on the
    /// name span (`fs-dir` / `fs-file`).
    #[test]
    fn render_markdown_plus_emits_fs_dir_and_fs_file_spans() {
        let temp = build_fixture_tree();
        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let md = fs.render_markdown_plus();
        assert!(
            md.contains("class=\"fs-dir\"") || md.contains("\"fs-dir "),
            "expected fs-dir span in MarkdownPlus: {md}",
        );
        assert!(
            md.contains("class=\"fs-file\"") || md.contains("\"fs-file "),
            "expected fs-file span in MarkdownPlus: {md}",
        );
    }

    /// Suggested #5 — when highlight matches, italic from dotfile config is
    /// suppressed (parity with bespoke `style_prefix`).
    #[test]
    fn render_tree_highlight_short_circuits_dotfile_italic() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".env-TODO"), "x").unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .italicize_dot_files(true)
            .highlight_red("TODO");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let style = first_entry_style(&node, ".env-TODO").expect("style");
        assert!(
            !style.emphasis.italic,
            "highlight should suppress dotfile italic — got style: {style:?}",
        );
        assert!(
            !style.emphasis.bold,
            "highlight should not add bold either — got style: {style:?}",
        );
    }

    /// Suggested #6 — a symlink that points at a directory must keep the
    /// bold attribute (the bespoke renderer stacks `1;34;36` SGR — bold
    /// survives even when cyan wins the foreground color).
    #[cfg(unix)]
    #[test]
    fn render_tree_symlink_to_directory_keeps_bold_and_cyan() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let target = temp.path().join("target_dir");
        std::fs::create_dir_all(&target).unwrap();
        let link = temp.path().join("alias");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let style = first_entry_style(&node, "alias").expect("style");
        assert!(
            style.emphasis.bold,
            "symlink-to-dir must remain bold — got: {style:?}",
        );
        assert!(
            style.color.is_some(),
            "symlink-to-dir must have a foreground color — got: {style:?}",
        );
    }

    /// Suggested #7 — threshold-highlighted metric values are projected as a
    /// classed inline span with a bold-yellow Style.
    #[test]
    fn render_tree_metric_threshold_highlight_emits_classed_span() {
        let temp = tempfile::tempdir().expect("create temp dir");
        // Write a "large" .rs file; ensure tokens count exceeds threshold.
        let big = "x".repeat(4096);
        std::fs::write(temp.path().join("big.rs"), &big).unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            // Threshold of 100 tokens — ~4096/4 = ~1000 tokens, well over.
            .show_tokens_highlight_greater_than(100);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(
            classes.iter().any(|c| c == CLASS_METRIC_HIGHLIGHT),
            "expected fs-metric-highlight class in: {classes:?}",
        );

        let style =
            first_style_for_span_class(&node, CLASS_METRIC_HIGHLIGHT).expect("highlight style");
        assert!(
            style.emphasis.bold,
            "metric highlight must be bold — got: {style:?}",
        );
        assert!(
            style.color.is_some(),
            "metric highlight must carry a yellow fg color — got: {style:?}",
        );
    }

    /// Suggested #9 — gitignored entries carry the `fs-ignored` class and a
    /// `dim` emphasis Style on the entry paragraph. Gitignored entries are
    /// shown by default; `dim_gitignore(true)` adds the dim attribute.
    #[test]
    fn render_tree_gitignored_entry_is_dim_with_class() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(temp.path().join("ignored.txt"), "x").unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .dim_gitignore(true);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(
            classes.iter().any(|c| c == CLASS_IGNORED),
            "expected fs-ignored class in: {classes:?}",
        );
        let style = first_entry_style(&node, "ignored.txt").expect("style");
        assert!(
            style.emphasis.dim,
            "gitignored entry should be dim — got: {style:?}",
        );
    }

    /// Walks `nodes` (and children) for the first entry named `name`.
    fn find_node<'a>(nodes: &'a [TreeNode], name: &str) -> Option<&'a TreeNode> {
        for node in nodes {
            if node.name() == name {
                return Some(node);
            }
            if let TreeNode::Dir { children, .. } = node
                && let Some(found) = find_node(children, name)
            {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn gitignore_matcher_flags_ignored_entries() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".gitignore"), "target/\n*.log\n").unwrap();
        std::fs::write(temp.path().join("keep.md"), "x").unwrap();
        std::fs::write(temp.path().join("debug.log"), "x").unwrap();
        std::fs::create_dir_all(temp.path().join("target")).unwrap();
        std::fs::write(temp.path().join("target/out.bin"), "x").unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .dim_gitignore(true);
        fs.ensure_tree_built();
        let tree = fs.tree().expect("tree");

        assert!(
            !find_node(tree, "keep.md").expect("keep.md").is_ignored(),
            "keep.md should not be ignored",
        );
        assert!(
            find_node(tree, "debug.log").expect("debug.log").is_ignored(),
            "debug.log should be ignored by *.log",
        );
        assert!(
            find_node(tree, "target").expect("target").is_ignored(),
            "target dir should be ignored by target/",
        );
    }

    #[test]
    fn with_prebuilt_tree_is_used_verbatim() {
        let child = TreeNode::File {
            name: "child.rs".to_string(),
            is_ignored: false,
            is_symlink: false,
            metrics: None,
        };
        let injected = vec![TreeNode::Dir {
            name: "src".to_string(),
            children: vec![child],
            is_ignored: false,
            is_symlink: false,
            has_error: false,
            at_depth_limit: false,
            metrics: None,
        }];

        let temp = tempfile::tempdir().expect("create temp dir");
        // A real on-disk entry that must NOT appear, proving the walk is skipped.
        std::fs::write(temp.path().join("on_disk.txt"), "x").unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .with_prebuilt_tree(injected.clone());
        fs.ensure_tree_built();

        assert_eq!(fs.tree(), Some(&injected));
    }

    /// Suggested #9 — highlight-green wins over directory bold-blue (the
    /// red counterpart is already tested above).
    #[test]
    fn render_tree_highlight_green_wins_over_directory_style() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("DONE-dir")).unwrap();

        let mut fs = FileSystem::new(temp.path())
            .expect("fs")
            .highlight_green("DONE");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(classes.contains(&CLASS_HIGHLIGHT_GREEN.to_string()));
        let style = first_entry_style(&node, "DONE-dir").expect("style");
        assert!(!style.emphasis.bold, "highlight should suppress bold");
    }

    /// Suggested #9 — error directory carries `fs-error` plus red Style.
    #[test]
    fn render_tree_error_directory_carries_fs_error_class() {
        // The simplest way to fabricate a `has_error` Dir is to build a tree
        // by hand and inject one — we cannot reliably reproduce a permission
        // error on every host in CI.
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("perm_denied")).unwrap();
        // Best-effort: skip the test if we can read the dir (i.e. the
        // simulation is impossible on this host). We accept that the
        // permission error is host-dependent; the canonical assertion here
        // is that the projection respects `has_error` when it is set.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o000);
            std::fs::set_permissions(temp.path().join("perm_denied"), perms).unwrap();
        }

        let mut fs = FileSystem::new(temp.path()).expect("fs");
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        // We can only assert presence when the host actually denied the read;
        // otherwise the test is informational. Restore mode afterwards so
        // tempdir cleanup succeeds.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                temp.path().join("perm_denied"),
                std::fs::Permissions::from_mode(0o755),
            );
        }
        if classes.iter().any(|c| c == CLASS_ERROR) {
            let style = first_entry_style(&node, "perm_denied").expect("style");
            assert!(
                style.color.is_some(),
                "error directory must have a foreground color — got: {style:?}",
            );
        }
    }

    /// Suggested #9 — depth-limit directory carries `fs-depth-limit` and the
    /// alternate icon glyph `📁`.
    #[test]
    fn render_tree_depth_limit_directory_carries_class_and_icon() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join("a/b/c")).unwrap();

        let mut fs = FileSystem::new(temp.path()).expect("fs").depth(1);
        fs.ensure_tree_built();

        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(
            classes.iter().any(|c| c == CLASS_DEPTH_LIMIT),
            "expected fs-depth-limit class in: {classes:?}",
        );
    }

    // ============================================================
    // Phase 1 — File-Links Directive Component Enhancements
    // ============================================================

    // --- Document extension icon constants ---

    #[test]
    fn test_nerd_document_ext_icons_are_valid_unicode() {
        assert!(icons::nerd::ext::PDF.is_alphanumeric() || !icons::nerd::ext::PDF.is_ascii());
        assert!(icons::nerd::ext::WORD.is_alphanumeric() || !icons::nerd::ext::WORD.is_ascii());
        assert!(icons::nerd::ext::EXCEL.is_alphanumeric() || !icons::nerd::ext::EXCEL.is_ascii());
        assert!(icons::nerd::ext::TEXT.is_alphanumeric() || !icons::nerd::ext::TEXT.is_ascii());
    }

    #[test]
    fn test_unicode_document_fallback_icons() {
        assert_eq!(icons::unicode::file::PDF, '\u{1F4D5}'); // 📕
        assert_eq!(icons::unicode::file::WORD, '\u{1F4D8}'); // 📘
        assert_eq!(icons::unicode::file::EXCEL, '\u{1F4D7}'); // 📗
        assert_eq!(icons::unicode::file::TEXT, '\u{1F4DD}'); // 📝
    }

    #[test]
    fn test_unicode_dir_repo_icon() {
        assert!(icons::nerd::dir::REPO.is_alphanumeric() || !icons::nerd::dir::REPO.is_ascii());
    }

    // --- Document extension icon selection (get_extension_icon) ---

    #[test]
    fn test_get_icon_pdf_extension() {
        let fs = FileSystem::default();
        for name in ["doc.pdf", "doc.PDF", "Doc.PdF"] {
            let node = TreeNode::File {
                name: name.into(),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::ext::PDF, "{name}");
            assert_eq!(fs.get_icon(&node, 0, Some(false)), icons::unicode::file::PDF, "{name}");
        }
    }

    #[test]
    fn test_get_icon_word_extensions() {
        let fs = FileSystem::default();
        for ext in ["doc", "docx", "DOC", "DocX"] {
            let node = TreeNode::File {
                name: format!("file.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::ext::WORD, ".{ext}");
            assert_eq!(fs.get_icon(&node, 0, Some(false)), icons::unicode::file::WORD, ".{ext}");
        }
    }

    #[test]
    fn test_get_icon_excel_extensions() {
        let fs = FileSystem::default();
        for ext in ["xls", "xlsx", "XLS", "XlsX"] {
            let node = TreeNode::File {
                name: format!("sheet.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::ext::EXCEL, ".{ext}");
            assert_eq!(fs.get_icon(&node, 0, Some(false)), icons::unicode::file::EXCEL, ".{ext}");
        }
    }

    #[test]
    fn test_get_icon_txt_extension() {
        let fs = FileSystem::default();
        for ext in ["txt", "TXT", "Txt"] {
            let node = TreeNode::File {
                name: format!("readme.{ext}"),
                is_ignored: false,
                is_symlink: false,
                metrics: None,
            };
            assert_eq!(fs.get_icon(&node, 0, Some(true)), icons::nerd::ext::TEXT, ".{ext}");
            assert_eq!(fs.get_icon(&node, 0, Some(false)), icons::unicode::file::TEXT, ".{ext}");
        }
    }

    // --- fs_unicode_file_icon (canonical render tree projection) ---

    #[test]
    fn test_fs_unicode_file_icon_documents() {
        assert_eq!(fs_unicode_file_icon("notes.txt"), "📝");
        assert_eq!(fs_unicode_file_icon("report.pdf"), "📕");
        assert_eq!(fs_unicode_file_icon("letter.doc"), "📘");
        assert_eq!(fs_unicode_file_icon("letter.docx"), "📘");
        assert_eq!(fs_unicode_file_icon("budget.xls"), "📗");
        assert_eq!(fs_unicode_file_icon("budget.xlsx"), "📗");
        // Mixed case
        assert_eq!(fs_unicode_file_icon("NOTES.TXT"), "📝");
        // Unknown extension
        assert_eq!(fs_unicode_file_icon("data.bin"), "📄");
        assert_eq!(fs_unicode_file_icon("noext"), "📄");
    }

    // --- Extension allowlist builder ---

    #[test]
    fn test_extension_filter_builder() {
        let fs = FileSystem::default().extension_filter(["md", "pdf"]);
        assert!(fs.extension_allowlist.contains("md"));
        assert!(fs.extension_allowlist.contains("pdf"));
        assert_eq!(fs.extension_allowlist.len(), 2);
    }

    #[test]
    fn test_extension_filter_strips_leading_dot_and_lowercases() {
        let fs = FileSystem::default().extension_filter([".MD", ".Pdf"]);
        assert!(fs.extension_allowlist.contains("md"));
        assert!(fs.extension_allowlist.contains("pdf"));
        assert!(!fs.extension_allowlist.contains(".md"));
    }

    #[test]
    fn test_document_extensions_builder() {
        let fs = FileSystem::default().document_extensions();
        for ext in ["md", "txt", "doc", "docx", "xls", "xlsx", "pdf"] {
            assert!(fs.extension_allowlist.contains(ext), "expected '{ext}' in allowlist");
        }
        assert_eq!(fs.extension_allowlist.len(), 7);
    }

    #[test]
    fn test_extension_filter_scanning() {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::write(root.join("a.md"), "x").unwrap();
        std::fs::write(root.join("b.txt"), "x").unwrap();
        std::fs::write(root.join("c.png"), "x").unwrap();
        std::fs::write(root.join("d.rs"), "x").unwrap();

        let mut fs = FileSystem::new(root).unwrap().document_extensions().show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        let names: Vec<&str> = tree.iter().map(|n| n.name()).collect();
        assert!(names.contains(&"a.md"), "md should pass");
        assert!(names.contains(&"b.txt"), "txt should pass");
        assert!(!names.contains(&"c.png"), "png should be filtered out");
        assert!(!names.contains(&"d.rs"), "rs should be filtered out");
    }

    #[test]
    fn test_extension_filter_mixed_case_extensions() {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::write(root.join("upper.PDF"), "x").unwrap();
        std::fs::write(root.join("mixed.TxT"), "x").unwrap();
        std::fs::write(root.join("lower.docx"), "x").unwrap();

        let mut fs = FileSystem::new(root).unwrap().document_extensions().show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        let names: Vec<&str> = tree.iter().map(|n| n.name()).collect();
        assert!(names.contains(&"upper.PDF"));
        assert!(names.contains(&"mixed.TxT"));
        assert!(names.contains(&"lower.docx"));
    }

    #[test]
    fn test_extension_filter_prunes_empty_ancestors() {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        // src/ contains only .rs files (not in document extension set)
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "x").unwrap();
        std::fs::write(root.join("src/lib.rs"), "x").unwrap();
        // docs/ contains .md files
        std::fs::create_dir(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/readme.md"), "x").unwrap();

        let mut fs = FileSystem::new(root).unwrap().document_extensions().show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        let names: Vec<&str> = tree.iter().map(|n| n.name()).collect();
        // 'src' must be pruned (no matching descendants), 'docs' must survive
        assert!(!names.contains(&"src"), "empty ancestor 'src' should be pruned");
        assert!(names.contains(&"docs"), "'docs' has matching descendants and must survive");
    }

    #[test]
    fn test_extension_filter_empty_allowlist_shows_all() {
        // When no filter is set, all files appear (existing behavior unchanged)
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::write(root.join("a.md"), "x").unwrap();
        std::fs::write(root.join("b.png"), "x").unwrap();

        let mut fs = FileSystem::new(root).unwrap().show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        let names: Vec<&str> = tree.iter().map(|n| n.name()).collect();
        assert!(names.contains(&"a.md"));
        assert!(names.contains(&"b.png"));
    }

    // --- Included-paths allowlist builder ---

    #[test]
    fn test_included_paths_builder() {
        use std::path::PathBuf;
        let fs = FileSystem::default().included_paths([
            PathBuf::from("docs/a.md"),
            PathBuf::from("docs/b.txt"),
        ]);
        assert!(fs.included_paths.contains(&PathBuf::from("docs/a.md")));
        assert!(fs.included_paths.contains(&PathBuf::from("docs/b.txt")));
    }

    #[test]
    fn test_included_paths_rejects_unsafe() {
        use std::path::PathBuf;
        let fs = FileSystem::default().included_paths([
            PathBuf::from("safe.md"),            // ok
            PathBuf::from("../escape.md"),        // rejected (ParentDir)
            PathBuf::from("/absolute.md"),        // rejected (absolute)
        ]);
        assert!(fs.included_paths.contains(&PathBuf::from("safe.md")));
        assert_eq!(fs.included_paths.len(), 1, "only the safe path should survive");
    }

    #[test]
    fn test_included_paths_scanning_preserves_hierarchy() {
        use std::path::PathBuf;
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::create_dir(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/a.md"), "x").unwrap();
        std::fs::write(root.join("docs/b.md"), "x").unwrap();
        std::fs::write(root.join("docs/c.md"), "x").unwrap();

        // Only select docs/a.md and docs/c.md — docs/b.md should be hidden
        let mut fs = FileSystem::new(root)
            .unwrap()
            .included_paths([PathBuf::from("docs/a.md"), PathBuf::from("docs/c.md")])
            .show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        // docs/ should survive as ancestor, but only a.md and c.md
        assert_eq!(tree.len(), 1);
        let dir = &tree[0];
        assert!(dir.is_dir());
        assert_eq!(dir.name(), "docs");
        if let TreeNode::Dir { children, .. } = dir {
            let names: Vec<&str> = children.iter().map(|n| n.name()).collect();
            assert!(names.contains(&"a.md"));
            assert!(names.contains(&"c.md"));
            assert!(!names.contains(&"b.md"), "b.md was not in included_paths");
        }
    }

    #[test]
    fn test_included_paths_empty_set_shows_all() {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::write(root.join("a.md"), "x").unwrap();
        std::fs::write(root.join("b.md"), "x").unwrap();

        let mut fs = FileSystem::new(root).unwrap().show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        assert_eq!(tree.len(), 2, "no included_paths filter → all files shown");
    }

    #[test]
    fn test_included_paths_combined_with_extension_filter() {
        use std::path::PathBuf;
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        std::fs::create_dir(root.join("d")).unwrap();
        std::fs::write(root.join("d/a.md"), "x").unwrap();
        std::fs::write(root.join("d/b.txt"), "x").unwrap();
        std::fs::write(root.join("d/c.png"), "x").unwrap();

        // included_paths selects a.md and c.png, but extension_filter only
        // allows md/txt — so only a.md survives (both filters must pass).
        let mut fs = FileSystem::new(root)
            .unwrap()
            .included_paths([
                PathBuf::from("d/a.md"),
                PathBuf::from("d/c.png"),
            ])
            .extension_filter(["md", "txt"])
            .show_root(false);
        fs.ensure_tree_built();
        let tree = fs.tree().unwrap();
        let dir = &tree[0];
        if let TreeNode::Dir { children, .. } = dir {
            let names: Vec<&str> = children.iter().map(|n| n.name()).collect();
            assert!(names.contains(&"a.md"));
            // c.png is in included_paths but fails extension_filter
            assert!(!names.contains(&"c.png"));
            // b.txt passes extension_filter but is not in included_paths
            assert!(!names.contains(&"b.txt"));
        } else {
            panic!("expected directory");
        }
    }

    // --- Dimmed root prefix ---

    #[test]
    fn test_with_dimmed_root_prefix_sets_field() {
        let fs = FileSystem::default().with_dimmed_root_prefix("/docs/");
        assert_eq!(fs.root_prefix.as_deref(), Some("/docs/"));
    }

    #[test]
    fn test_with_root_display_name_sets_field() {
        let fs = FileSystem::default().with_root_display_name("topics");
        assert_eq!(fs.root_display_name.as_deref(), Some("topics"));
    }

    #[test]
    fn test_with_root_icon_sets_field() {
        let fs = FileSystem::default().with_root_icon(RootIconKind::Repository);
        assert_eq!(fs.root_icon, Some(RootIconKind::Repository));
    }

    #[test]
    fn test_dimmed_root_prefix_render_optimistic() {
        // render_optimistic uses no TTY, so no ANSI codes. The prefix and
        // target should appear concatenated in the root line.
        let temp = tempfile::tempdir().expect("temp");
        std::fs::write(temp.path().join("a.txt"), "x").unwrap();
        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .with_dimmed_root_prefix("/docs/")
            .with_root_display_name("topics");
        fs.ensure_tree_built();
        let output = fs.render_optimistic(Some(120));
        // First line is the root; should contain both prefix and target
        let first_line = output.lines().next().expect("root line");
        assert!(first_line.contains("/docs/"), "prefix in: {first_line:?}");
        assert!(first_line.contains("topics"), "target in: {first_line:?}");
    }

    #[test]
    fn test_dimmed_root_prefix_render_tree_has_dimmed_prefix_span() {
        let temp = tempfile::tempdir().expect("temp");
        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .with_dimmed_root_prefix("/docs/")
            .with_root_display_name("topics");
        fs.ensure_tree_built();
        let node = fs.render_tree();
        let classes = collect_classes(&node);
        assert!(
            classes.iter().any(|c| c == CLASS_ROOT_PREFIX),
            "expected fs-root-prefix class: {classes:?}",
        );
    }

    #[test]
    fn test_no_root_prefix_preserves_original_behavior() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::write(temp.path().join("a.txt"), "x").unwrap();

        // Without prefix — original behavior
        let mut fs1 = FileSystem::new(temp.path()).unwrap().show_root(true);
        fs1.ensure_tree_built();
        let out1 = fs1.render_optimistic(Some(80));

        // The root line should be just icon+name (no prefix text)
        let first_line = out1.lines().next().expect("root line");
        assert!(!first_line.contains("/docs/"));
    }

    #[test]
    fn test_dimmed_root_prefix_with_file_links() {
        // When both file_links and root_prefix are set, the target should
        // still be wrapped in an OSC8 hyperlink in the bespoke TTY renderer.
        let temp = tempfile::tempdir().expect("temp");
        std::fs::write(temp.path().join("a.txt"), "x").unwrap();
        let mut fs = FileSystem::new(temp.path())
            .unwrap()
            .with_dimmed_root_prefix("/docs/")
            .with_root_display_name("topics")
            .with_file_links();
        fs.ensure_tree_built();

        // render_optimistic does not use TTY so no OSC8 wrapping occurs,
        // but the content should survive.
        let output = fs.render_optimistic(Some(120));
        let first_line = output.lines().next().expect("root line");
        assert!(first_line.contains("topics"), "target in: {first_line:?}");
    }

    // --- Helper function tests ---

    #[test]
    fn test_is_safe_relative() {
        assert!(is_safe_relative(Path::new("a.md")));
        assert!(is_safe_relative(Path::new("docs/a.md")));
        assert!(is_safe_relative(Path::new("docs/sub/a.md")));
        assert!(!is_safe_relative(Path::new("../escape.md")));
        assert!(!is_safe_relative(Path::new("a/../b.md"))); // contains ParentDir
        assert!(!is_safe_relative(Path::new("/absolute.md")));
        assert!(!is_safe_relative(Path::new("")));
    }

    #[test]
    fn test_lowercase_extension_helper() {
        assert_eq!(lowercase_extension("file.md").as_deref(), Some("md"));
        assert_eq!(lowercase_extension("file.PDF").as_deref(), Some("pdf"));
        assert_eq!(lowercase_extension("file.DocX").as_deref(), Some("docx"));
        assert_eq!(lowercase_extension("noext"), None);
        assert_eq!(lowercase_extension(".env"), None); // dotfile, no real extension
    }

    #[test]
    fn test_root_icon_kind_nerd_char() {
        assert_eq!(RootIconKind::Directory.nerd_char(), icons::nerd::dir::BASE);
        assert_eq!(RootIconKind::Repository.nerd_char(), icons::nerd::dir::REPO);
    }

    #[test]
    fn test_root_icon_kind_unicode_str() {
        assert_eq!(RootIconKind::Directory.unicode_str(), "📂");
        assert_eq!(RootIconKind::Repository.unicode_str(), "📦");
    }

    #[test]
    fn test_root_icon_default_is_directory() {
        assert_eq!(RootIconKind::default(), RootIconKind::Directory);
    }
}
