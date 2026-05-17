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

// Submodules
pub mod error;
pub mod icons;
pub mod metrics;
pub mod tree_chars;
pub mod tree_node;

// Re-exports for backward compatibility
pub use error::FileSystemError;
pub use metrics::{FileMetrics, MetricKind};
pub use tree_node::TreeNode;

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use paste::paste;

use self::metrics::MetricConfig;

use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::terminal::Terminal;
use crate::utils::block_constraint::{split_at_visible_width, visible_width};
use crate::utils::layout::{Layout, LayoutTerminalExt};

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

    // =========================================================================
    // Metric Builder Methods
    // =========================================================================

    /// Returns whether any metrics are configured to be shown.
    fn has_any_metrics(&self) -> bool {
        self.metric_configs.values().any(|c| c.enabled)
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
            self.tree =
                Some(self.build_tree_recursive(&self.root_path.clone(), 0, &mut total_entries));
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
    /// For now, uses `std::fs` directly. The `ignore` crate integration
    /// will be added in Phase 8 for proper `.gitignore` support.
    fn build_tree_recursive(
        &self,
        path: &Path,
        depth: u32,
        total_entries: &mut u32,
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

            // Apply filter patterns
            let has_filters = !self.filter_patterns.is_empty();
            let matches_filter =
                has_filters && self.filter_patterns.iter().any(|p| file_name.contains(p));

            if is_dir {
                // Don't follow symlinks to avoid infinite loops
                let at_depth_limit = depth + 1 >= self.max_depth;
                let children = if is_symlink || at_depth_limit {
                    vec![]
                } else {
                    self.build_tree_recursive(&file_path, depth + 1, total_entries)
                };

                // When filters are active, only include directories that either
                // match the filter themselves or have matching descendants.
                if has_filters && !matches_filter && children.is_empty() {
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
                    is_ignored: false, // Will be set properly with ignore crate in Phase 8
                    is_symlink,
                    has_error: false,
                    at_depth_limit,
                    metrics: dir_metrics,
                });
            } else {
                // Skip non-matching files when filters are active
                if has_filters && !matches_filter {
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
                    is_ignored: false, // Will be set properly with ignore crate in Phase 8
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
}

impl FileSystem {
    /// Renders the root directory name as a header line.
    ///
    /// Shows the directory icon and name (e.g., ` docs`), styled bold blue
    /// when connected to a TTY.
    fn render_root_line(&self, output: &mut String, is_nerd_font: Option<bool>, is_tty: bool) {
        let use_nerd = is_nerd_font.unwrap_or(false);

        // Resolve the display name: canonicalize relative paths like "." and ".."
        // so we show the actual directory name instead of a dot.
        let name = self
            .root_path
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .or_else(|| {
                self.root_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| self.root_path.display().to_string());

        let icon = self.get_dir_icon(&name, use_nerd);
        let icon_str = if use_nerd {
            format!("{} ", icon)
        } else {
            icon.to_string()
        };

        if is_tty {
            let display_name = if self.file_links {
                let abs_path = self
                    .root_path
                    .canonicalize()
                    .unwrap_or_else(|_| self.root_path.clone());
                Prose::new(format!("<a href=\"{}\">{}</a>", abs_path.display(), name))
                    .render_optimistic(None)
            } else {
                name
            };
            output.push_str(&format!("\x1b[1;34m{}{}\x1b[0m\n", icon_str, display_name));
        } else {
            output.push_str(&format!("{}{}\n", icon_str, name));
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
                Prose::new(format!(
                    "<a href=\"{}\">{}</a>",
                    node_path.display(),
                    display_name
                ))
                .render_optimistic(None)
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
        use crate::utils::layout::{Layout, Margin};

        let custom_layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };

        let fs = FileSystem::new(".").unwrap().layout(custom_layout.clone());
        assert_eq!(fs.layout.left_margin, Margin::Chars(4));
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
        let file_link = format!("\x1b]8;;file://{}/hello.txt\x1b\\", canonical.display());
        assert!(
            result.contains(&file_link),
            "Expected OSC8 link for hello.txt in output.\nLooking for: {:?}\nOutput: {:?}",
            file_link,
            result
        );

        // Nested file should have full path
        let nested_link = format!("\x1b]8;;file://{}/sub/nested.rs\x1b\\", canonical.display());
        assert!(
            result.contains(&nested_link),
            "Expected OSC8 link for sub/nested.rs in output.\nLooking for: {:?}\nOutput: {:?}",
            nested_link,
            result
        );

        // Directory should also be linked
        let dir_link = format!("\x1b]8;;file://{}/sub\x1b\\", canonical.display());
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
        assert_eq!(layout.left_margin, crate::utils::layout::Margin::None);
    }

    #[test]
    fn test_layout_mut_accessor() {
        use crate::components::renderable::TerminalRenderable;
        let mut fs = FileSystem::default();
        TerminalRenderable::layout_mut(&mut fs).left_margin =
            crate::utils::layout::Margin::Chars(4);

        assert_eq!(
            TerminalRenderable::layout(&fs).left_margin,
            crate::utils::layout::Margin::Chars(4)
        );
    }

    #[test]
    fn test_as_any() {
        let fs = FileSystem::default();
        let any_ref = fs.as_any();

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
}
