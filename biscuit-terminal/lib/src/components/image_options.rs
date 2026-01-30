//! Configuration options for terminal image rendering.
//!
//! This module provides a unified `TerminalImageOptions` type that controls
//! how images are loaded, validated, and rendered in terminal environments.
//!
//! ## Examples
//!
//! ```rust
//! use std::path::PathBuf;
//! use biscuit_terminal::components::image_options::{TerminalImageOptions, ImageWidth};
//!
//! // Using the builder pattern
//! let options = TerminalImageOptions::builder()
//!     .base_path(PathBuf::from("/safe/directory"))
//!     .max_file_size(5 * 1024 * 1024) // 5MB
//!     .allow_remote(false)
//!     .width(ImageWidth::Percent(0.75))
//!     .use_viuer(true)
//!     .build();
//!
//! // Using defaults
//! let default_options = TerminalImageOptions::default();
//! ```

use std::path::PathBuf;

// Re-export ImageWidth for convenience
pub use super::terminal_image::ImageWidth;

/// Default maximum file size: 10MB
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Configuration options for terminal image rendering.
///
/// This struct provides a unified way to configure image rendering behavior
/// across different terminal image implementations. It controls security
/// boundaries, size limits, and rendering preferences.
///
/// ## Security
///
/// - `base_path`: When set, restricts image loading to files within this directory
/// - `max_file_size`: Prevents loading excessively large files
/// - `allow_remote`: Controls whether remote URLs can be fetched
///
/// ## Rendering
///
/// - `width`: How the image should be sized (fill, percentage, or fixed characters)
/// - `use_viuer`: Whether to use the viuer crate for rendering or raw protocols
#[derive(Debug, Clone)]
pub struct TerminalImageOptions {
    /// Base path for resolving relative paths (security boundary).
    ///
    /// When set, all relative image paths will be resolved against this directory,
    /// and attempts to escape via `..` will be rejected.
    pub base_path: Option<PathBuf>,

    /// Maximum file size in bytes.
    ///
    /// Files larger than this limit will be rejected to prevent memory exhaustion.
    /// Default: 10MB (10 * 1024 * 1024 bytes)
    pub max_file_size: u64,

    /// Whether to allow remote URLs.
    ///
    /// When `false`, only local file paths are permitted.
    /// Default: `false`
    pub allow_remote: bool,

    /// Width specification for image rendering.
    ///
    /// Controls how the image width is calculated:
    /// - `Fill`: Use available terminal width
    /// - `Percent(f32)`: Use a percentage of available width
    /// - `Characters(u32)`: Use a fixed number of character columns
    ///
    /// Default: `ImageWidth::Percent(0.5)` (50% of terminal width)
    pub width: ImageWidth,

    /// Whether to use viuer for rendering.
    ///
    /// When `true`, the viuer crate handles protocol selection and rendering.
    /// When `false`, raw Kitty/iTerm2 protocols are used directly.
    ///
    /// Default: `true`
    pub use_viuer: bool,
}

impl Default for TerminalImageOptions {
    fn default() -> Self {
        Self {
            base_path: None,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            allow_remote: false,
            width: ImageWidth::default(),
            use_viuer: true,
        }
    }
}

impl TerminalImageOptions {
    /// Create a new builder for `TerminalImageOptions`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .max_file_size(5 * 1024 * 1024)
    ///     .build();
    /// ```
    pub fn builder() -> TerminalImageOptionsBuilder {
        TerminalImageOptionsBuilder::default()
    }

    /// Check if a file size is within the allowed limit.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::default();
    /// assert!(options.is_size_allowed(1024)); // 1KB is fine
    /// assert!(!options.is_size_allowed(20 * 1024 * 1024)); // 20MB exceeds default
    /// ```
    pub fn is_size_allowed(&self, size: u64) -> bool {
        size <= self.max_file_size
    }

    /// Check if a path is within the allowed base path.
    ///
    /// Returns `true` if:
    /// - No base path is set (all paths allowed)
    /// - The path is within the base path after canonicalization
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .base_path(PathBuf::from("/safe"))
    ///     .build();
    ///
    /// // Note: These checks require actual filesystem access
    /// // In practice, paths are canonicalized before comparison
    /// ```
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        match &self.base_path {
            None => true,
            Some(base) => {
                // Try to canonicalize both paths for comparison
                let canonical_path = match std::fs::canonicalize(path) {
                    Ok(p) => p,
                    Err(_) => return false, // Can't verify, deny
                };
                let canonical_base = match std::fs::canonicalize(base) {
                    Ok(p) => p,
                    Err(_) => return false, // Invalid base, deny
                };
                canonical_path.starts_with(&canonical_base)
            }
        }
    }
}

/// Builder for `TerminalImageOptions`.
///
/// Provides a fluent API for constructing `TerminalImageOptions` instances.
#[derive(Debug, Clone, Default)]
pub struct TerminalImageOptionsBuilder {
    base_path: Option<PathBuf>,
    max_file_size: Option<u64>,
    allow_remote: Option<bool>,
    width: Option<ImageWidth>,
    use_viuer: Option<bool>,
}

impl TerminalImageOptionsBuilder {
    /// Set the base path for resolving relative paths.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .base_path(PathBuf::from("/safe/images"))
    ///     .build();
    /// ```
    pub fn base_path(mut self, path: PathBuf) -> Self {
        self.base_path = Some(path);
        self
    }

    /// Set the maximum allowed file size in bytes.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .max_file_size(5 * 1024 * 1024) // 5MB
    ///     .build();
    /// ```
    pub fn max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }

    /// Set whether remote URLs are allowed.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .allow_remote(true)
    ///     .build();
    /// ```
    pub fn allow_remote(mut self, allow: bool) -> Self {
        self.allow_remote = Some(allow);
        self
    }

    /// Set the width specification for image rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::{TerminalImageOptions, ImageWidth};
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .width(ImageWidth::Percent(0.75))
    ///     .build();
    /// ```
    pub fn width(mut self, width: ImageWidth) -> Self {
        self.width = Some(width);
        self
    }

    /// Set whether to use viuer for rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::image_options::TerminalImageOptions;
    ///
    /// let options = TerminalImageOptions::builder()
    ///     .use_viuer(false) // Use raw protocols
    ///     .build();
    /// ```
    pub fn use_viuer(mut self, use_viuer: bool) -> Self {
        self.use_viuer = Some(use_viuer);
        self
    }

    /// Build the `TerminalImageOptions` instance.
    ///
    /// Any unset fields will use their default values.
    pub fn build(self) -> TerminalImageOptions {
        TerminalImageOptions {
            base_path: self.base_path,
            max_file_size: self.max_file_size.unwrap_or(DEFAULT_MAX_FILE_SIZE),
            allow_remote: self.allow_remote.unwrap_or(false),
            width: self.width.unwrap_or_default(),
            use_viuer: self.use_viuer.unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = TerminalImageOptions::default();

        assert!(options.base_path.is_none());
        assert_eq!(options.max_file_size, 10 * 1024 * 1024);
        assert!(!options.allow_remote);
        assert!(matches!(options.width, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
        assert!(options.use_viuer);
    }

    #[test]
    fn test_builder_all_fields() {
        let base = PathBuf::from("/test/path");
        let options = TerminalImageOptions::builder()
            .base_path(base.clone())
            .max_file_size(5 * 1024 * 1024)
            .allow_remote(true)
            .width(ImageWidth::Characters(80))
            .use_viuer(false)
            .build();

        assert_eq!(options.base_path, Some(base));
        assert_eq!(options.max_file_size, 5 * 1024 * 1024);
        assert!(options.allow_remote);
        assert!(matches!(options.width, ImageWidth::Characters(80)));
        assert!(!options.use_viuer);
    }

    #[test]
    fn test_builder_partial_fields() {
        let options = TerminalImageOptions::builder()
            .max_file_size(1024)
            .build();

        // Set field
        assert_eq!(options.max_file_size, 1024);

        // Default fields
        assert!(options.base_path.is_none());
        assert!(!options.allow_remote);
        assert!(matches!(options.width, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
        assert!(options.use_viuer);
    }

    #[test]
    fn test_is_size_allowed() {
        let options = TerminalImageOptions::builder()
            .max_file_size(1000)
            .build();

        assert!(options.is_size_allowed(0));
        assert!(options.is_size_allowed(500));
        assert!(options.is_size_allowed(1000));
        assert!(!options.is_size_allowed(1001));
        assert!(!options.is_size_allowed(u64::MAX));
    }

    #[test]
    fn test_is_path_allowed_no_base() {
        let options = TerminalImageOptions::default();

        // With no base path, all paths should be allowed
        assert!(options.is_path_allowed(std::path::Path::new("/any/path")));
        assert!(options.is_path_allowed(std::path::Path::new("relative/path")));
    }

    #[test]
    fn test_is_path_allowed_with_base() {
        // Create a temp directory for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create a subdirectory
        let sub_dir = base_path.join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();

        // Create a test file
        let test_file = sub_dir.join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        let options = TerminalImageOptions::builder()
            .base_path(base_path.clone())
            .build();

        // File within base should be allowed
        assert!(options.is_path_allowed(&test_file));

        // Nonexistent paths should be denied (can't canonicalize)
        assert!(!options.is_path_allowed(std::path::Path::new("/nonexistent/path")));
    }

    #[test]
    fn test_builder_width_variants() {
        // Fill
        let options = TerminalImageOptions::builder()
            .width(ImageWidth::Fill)
            .build();
        assert!(matches!(options.width, ImageWidth::Fill));

        // Percent
        let options = TerminalImageOptions::builder()
            .width(ImageWidth::Percent(0.25))
            .build();
        assert!(matches!(options.width, ImageWidth::Percent(p) if (p - 0.25).abs() < 0.001));

        // Characters
        let options = TerminalImageOptions::builder()
            .width(ImageWidth::Characters(120))
            .build();
        assert!(matches!(options.width, ImageWidth::Characters(120)));
    }

    #[test]
    fn test_builder_is_clone() {
        let builder = TerminalImageOptions::builder()
            .max_file_size(2048);

        let builder2 = builder.clone();
        let options1 = builder.build();
        let options2 = builder2.build();

        assert_eq!(options1.max_file_size, options2.max_file_size);
    }

    #[test]
    fn test_options_is_clone() {
        let options = TerminalImageOptions::builder()
            .base_path(PathBuf::from("/test"))
            .max_file_size(4096)
            .allow_remote(true)
            .width(ImageWidth::Fill)
            .use_viuer(false)
            .build();

        let cloned = options.clone();

        assert_eq!(options.base_path, cloned.base_path);
        assert_eq!(options.max_file_size, cloned.max_file_size);
        assert_eq!(options.allow_remote, cloned.allow_remote);
        assert_eq!(options.use_viuer, cloned.use_viuer);
    }

    #[test]
    fn test_options_debug() {
        let options = TerminalImageOptions::default();
        let debug_str = format!("{:?}", options);

        assert!(debug_str.contains("TerminalImageOptions"));
        assert!(debug_str.contains("max_file_size"));
    }
}
