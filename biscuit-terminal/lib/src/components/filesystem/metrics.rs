use chrono::{DateTime, Utc};

/// The kind of metric that can be displayed alongside filesystem entries.
///
/// This enum defines various metadata attributes that can be computed and
/// displayed for files and directories in tree views.
///
/// ## Variants
///
/// - **FileSize**: Human-readable size (e.g., "1.2 KB", "3.5 MB")
/// - **Tokens**: Estimated LLM token count using file extension heuristics
/// - **Created**: Absolute creation timestamp (e.g., "2024-01-15 10:30:00")
/// - **CreatedSince**: Relative creation time (e.g., "2 days ago")
/// - **Modified**: Absolute modification timestamp
/// - **ModifiedSince**: Relative modification time (e.g., "1 month ago")
/// - **Permissions**: Symbolic permission string (e.g., ".rw-r--r--")
/// - **PermissionsNumeric**: Numeric mode (e.g., "644", "755")
/// - **Owner**: File owner username
/// - **Group**: File group name
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::filesystem::MetricKind;
///
/// // Variants can be compared for equality
/// assert_eq!(MetricKind::FileSize, MetricKind::FileSize);
/// assert_ne!(MetricKind::FileSize, MetricKind::Tokens);
///
/// // Common use case: match on metric kind to get display name
/// let kind = MetricKind::FileSize;
/// let name = match kind {
///     MetricKind::FileSize => "Size",
///     MetricKind::Tokens => "Tokens",
///     MetricKind::Created => "Created",
///     MetricKind::CreatedSince => "Created",
///     MetricKind::Modified => "Modified",
///     MetricKind::ModifiedSince => "Modified",
///     MetricKind::Permissions => "Permissions",
///     MetricKind::PermissionsNumeric => "Mode",
///     MetricKind::Owner => "Owner",
///     MetricKind::Group => "Group",
/// };
/// assert_eq!(name, "Size");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    /// File size in bytes, displayed as human-readable (e.g., "1.2 KB").
    FileSize,
    /// Estimated LLM token count based on file extension heuristics.
    Tokens,
    /// Absolute creation timestamp.
    Created,
    /// Relative creation time (e.g., "2 days ago").
    CreatedSince,
    /// Absolute modification timestamp.
    Modified,
    /// Relative modification time (e.g., "1 month ago").
    ModifiedSince,
    /// Symbolic permission string (e.g., ".rw-r--r--").
    Permissions,
    /// Numeric permission mode (e.g., "644").
    PermissionsNumeric,
    /// File owner username.
    Owner,
    /// File group name.
    Group,
}

impl MetricKind {
    /// Returns all metric kinds in their canonical display order.
    pub(super) fn all_in_order() -> &'static [MetricKind] {
        &[
            MetricKind::FileSize,
            MetricKind::Tokens,
            MetricKind::Permissions,
            MetricKind::PermissionsNumeric,
            MetricKind::Owner,
            MetricKind::Group,
            MetricKind::Created,
            MetricKind::CreatedSince,
            MetricKind::Modified,
            MetricKind::ModifiedSince,
        ]
    }

    /// Returns whether this metric is applicable to directories.
    pub(super) fn is_dir_applicable(self) -> bool {
        !matches!(self, MetricKind::FileSize | MetricKind::Tokens)
    }
}

/// Per-metric configuration (private).
#[derive(Debug, Clone, Default)]
pub(super) struct MetricConfig {
    pub(super) enabled: bool,
    pub(super) filename_patterns: Vec<String>,
    pub(super) highlight_threshold: Option<u64>,
}

/// Collected file metrics including size, token count, timestamps, and permissions.
///
/// This struct aggregates various metadata about a filesystem entry:
/// - File size in bytes
/// - Estimated LLM token count (based on character count / 4)
/// - Creation and modification timestamps
/// - Unix permission mode bits (Unix only)
/// - File owner and group (Unix only)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::filesystem::FileMetrics;
/// use chrono::Utc;
///
/// // Create with all fields
/// let metrics = FileMetrics {
///     file_size: Some(1024),
///     tokens: Some(256),
///     created: Some(Utc::now()),
///     modified: Some(Utc::now()),
///     #[cfg(unix)]
///     permissions_mode: Some(0o644),
///     #[cfg(unix)]
///     owner: Some("user".to_string()),
///     #[cfg(unix)]
///     group: Some("staff".to_string()),
/// };
///
/// // Use default for partial initialization
/// let partial = FileMetrics {
///     file_size: Some(2048),
///     ..FileMetrics::default()
/// };
/// ```
///
/// ```
/// use biscuit_terminal::components::filesystem::FileMetrics;
///
/// // FileMetrics implements Default for easy partial initialization
/// let empty = FileMetrics::default();
/// assert!(empty.file_size.is_none());
/// assert!(empty.tokens.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileMetrics {
    /// File size in bytes.
    pub file_size: Option<u64>,
    /// Estimated LLM token count.
    pub tokens: Option<u64>,
    /// File creation timestamp.
    pub created: Option<DateTime<Utc>>,
    /// File modification timestamp.
    pub modified: Option<DateTime<Utc>>,
    /// Unix permission mode bits (e.g., 0o644).
    #[cfg(unix)]
    pub permissions_mode: Option<u32>,
    /// File owner username.
    #[cfg(unix)]
    pub owner: Option<String>,
    /// File group name.
    #[cfg(unix)]
    pub group: Option<String>,
}
