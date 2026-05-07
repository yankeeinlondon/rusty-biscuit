use super::metrics::FileMetrics;

/// Represents a node in a filesystem tree.
///
/// Each node is either a directory (with children) or a file.
/// Both variants track whether the entry is ignored (by `.gitignore`)
/// and whether it is a symbolic link.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::filesystem::TreeNode;
///
/// // Create a file node
/// let file = TreeNode::File {
///     name: "main.rs".to_string(),
///     is_ignored: false,
///     is_symlink: false,
///     metrics: None,
/// };
/// assert!(file.is_file());
/// assert_eq!(file.name(), "main.rs");
///
/// // Create a directory node
/// let dir = TreeNode::Dir {
///     name: "src".to_string(),
///     children: vec![file],
///     is_ignored: false,
///     is_symlink: false,
///     has_error: false,
///     at_depth_limit: false,
///     metrics: None,
/// };
/// assert!(dir.is_dir());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    /// A directory entry.
    Dir {
        /// The directory name (not the full path).
        name: String,
        /// Child entries within this directory.
        children: Vec<TreeNode>,
        /// Whether this directory is ignored by `.gitignore`.
        is_ignored: bool,
        /// Whether this directory is a symbolic link.
        is_symlink: bool,
        /// Whether an error occurred reading this directory.
        has_error: bool,
        /// Whether this directory is at the configured depth limit.
        ///
        /// When true, children are not populated and the directory
        /// is rendered with a special "depth limit" icon.
        at_depth_limit: bool,
        /// Collected metrics for this directory (when metrics are configured).
        metrics: Option<FileMetrics>,
    },
    /// A file entry.
    File {
        /// The filename (not the full path).
        name: String,
        /// Whether this file is ignored by `.gitignore`.
        is_ignored: bool,
        /// Whether this file is a symbolic link.
        is_symlink: bool,
        /// Collected metrics for this file (when metrics are configured).
        metrics: Option<FileMetrics>,
    },
}

impl TreeNode {
    /// Returns the name of this node.
    pub fn name(&self) -> &str {
        match self {
            TreeNode::Dir { name, .. } => name,
            TreeNode::File { name, .. } => name,
        }
    }

    /// Returns whether this node is ignored.
    pub fn is_ignored(&self) -> bool {
        match self {
            TreeNode::Dir { is_ignored, .. } => *is_ignored,
            TreeNode::File { is_ignored, .. } => *is_ignored,
        }
    }

    /// Returns whether this node is a symlink.
    pub fn is_symlink(&self) -> bool {
        match self {
            TreeNode::Dir { is_symlink, .. } => *is_symlink,
            TreeNode::File { is_symlink, .. } => *is_symlink,
        }
    }

    /// Returns whether this node is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, TreeNode::Dir { .. })
    }

    /// Returns whether this node is a file.
    pub fn is_file(&self) -> bool {
        matches!(self, TreeNode::File { .. })
    }

    /// Returns a reference to the metrics attached to this node, if any.
    pub fn metrics(&self) -> Option<&FileMetrics> {
        match self {
            TreeNode::Dir { metrics, .. } | TreeNode::File { metrics, .. } => metrics.as_ref(),
        }
    }
}
