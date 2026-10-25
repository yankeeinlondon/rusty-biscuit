use super::TreeIndex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Default shared budget for one query.
pub const DEFAULT_DEADLINE: Duration = Duration::from_secs(2);

/// Scope and budget for [`query_path_usage`](super::query_path_usage).
///
/// The deadline is a shared scan budget: when it expires the query stops
/// scheduling new work and reports what it collected. A native call already
/// running when the budget expires can overrun it, so the deadline is not a
/// guarantee of when the function returns.
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::query::PathUsageOptions;
/// use std::time::Duration;
///
/// let options = PathUsageOptions::default()
///     .target_only()
///     .with_deadline(Duration::from_millis(500));
/// assert!(!options.recursive());
/// ```
#[derive(Debug, Clone)]
pub struct PathUsageOptions {
    recursive: bool,
    deadline: Duration,
    tree_index: Option<Arc<TreeIndex>>,
}

impl Default for PathUsageOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            deadline: DEFAULT_DEADLINE,
            tree_index: None,
        }
    }
}

/// Two options are equal when they would run the same query: the same
/// supplied index (by identity, never by comparing its entries), or none.
impl PartialEq for PathUsageOptions {
    fn eq(&self, other: &Self) -> bool {
        self.recursive == other.recursive
            && self.deadline == other.deadline
            && match (&self.tree_index, &other.tree_index) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for PathUsageOptions {}

#[allow(missing_docs)]
impl PathUsageOptions {
    /// Query only the target itself, not a directory's descendants.
    ///
    /// A nonrecursive watch of the target directory is still target evidence.
    pub fn target_only(mut self) -> Self {
        self.recursive = false;
        self
    }

    /// Replace the shared budget. Zero, or a duration too large to form a
    /// deadline, is rejected by the query as [`PathUsageError::InvalidOption`].
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Match against `index` instead of walking the target directory.
    ///
    /// For a caller that has already walked the tree, so the query's budget
    /// goes to processes rather than a second walk. The query still resolves
    /// and verifies the root itself, and rejects with
    /// [`PathUsageError::InvalidOption`] an index whose root identity is not
    /// the target's, a file target, and [`target_only`](Self::target_only).
    /// The index is shared, not copied, so one index can serve several
    /// queries of the same tree; it describes the tree as the caller captured
    /// it, so entries created since are not matched.
    pub fn with_tree_index(mut self, index: Arc<TreeIndex>) -> Self {
        self.tree_index = Some(index);
        self
    }

    pub fn tree_index(&self) -> Option<&Arc<TreeIndex>> {
        self.tree_index.as_ref()
    }

    pub fn recursive(&self) -> bool {
        self.recursive
    }

    pub fn deadline(&self) -> Duration {
        self.deadline
    }
}

/// A query that could not produce a report about a verified target.
///
/// Failures after the root identity is captured never use this type: they
/// become coverage statuses and limitations in the report.
#[derive(Debug, thiserror::Error)]
pub enum PathUsageError {
    #[error("invalid option: {message}")]
    InvalidOption { message: String },

    #[error("target does not exist: {}", path.display())]
    MissingTarget { path: PathBuf },

    /// Sockets, devices, and FIFOs are rejected from `symlink_metadata`
    /// before any operation that could open or wait on them.
    #[error("unsupported target kind ({file_type}): {}", path.display())]
    UnsupportedTargetKind {
        path: PathBuf,
        file_type: &'static str,
    },

    /// The shared budget expired before the root identity was captured.
    #[error(
        "the {}ms budget expired before the target's identity was verified: {}",
        budget.as_millis(),
        path.display()
    )]
    RootValidationTimeout { path: PathBuf, budget: Duration },

    #[error("cannot establish the target's identity: {}: {source}", path.display())]
    RootIdentity {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl PathUsageError {
    /// Stable machine-readable kind, used as the CLI's JSON `error.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidOption { .. } => "invalid_option",
            Self::MissingTarget { .. } => "missing_target",
            Self::UnsupportedTargetKind { .. } => "unsupported_target_kind",
            Self::RootValidationTimeout { .. } => "root_validation_timeout",
            Self::RootIdentity { .. } => "root_identity",
        }
    }
}
