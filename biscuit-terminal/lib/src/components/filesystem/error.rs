use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur when working with filesystem trees.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::{FileSystem, FileSystemError};
///
/// // Path not found
/// let result = FileSystem::new("/nonexistent/path");
/// assert!(matches!(result, Err(FileSystemError::PathNotFound { .. })));
///
/// // Check error message
/// if let Err(e) = result {
///     assert!(e.to_string().contains("Path not found"));
/// }
/// ```
///
/// ```
/// use biscuit_terminal::prelude::{FileSystem, FileSystemError};
///
/// // Using pattern matching for error handling
/// fn handle_fs_error(err: FileSystemError) -> String {
///     match err {
///         FileSystemError::PathNotFound { path } => {
///             format!("Path does not exist: {}", path.display())
///         }
///         FileSystemError::NotADirectory { path } => {
///             format!("Expected directory, got file: {}", path.display())
///         }
///         FileSystemError::PermissionDenied { path } => {
///             format!("Access denied: {}", path.display())
///         }
///         FileSystemError::IoError(e) => {
///             format!("IO error: {}", e)
///         }
///         FileSystemError::Ignore(e) => {
///             format!("Gitignore error: {}", e)
///         }
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum FileSystemError {
    /// The specified path does not exist.
    #[error("Path not found: {}", path.display())]
    PathNotFound {
        /// The path that was not found.
        path: PathBuf,
    },

    /// The specified path exists but is not a directory.
    #[error("Not a directory: {}", path.display())]
    NotADirectory {
        /// The path that is not a directory.
        path: PathBuf,
    },

    /// Permission was denied when accessing the path.
    #[error("Permission denied: {}", path.display())]
    PermissionDenied {
        /// The path that could not be accessed.
        path: PathBuf,
    },

    /// An I/O error occurred during filesystem operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// An error from the ignore crate (gitignore pattern handling).
    #[error("Ignore error: {0}")]
    Ignore(#[from] ignore::Error),
}
