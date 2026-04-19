//! Data format enumeration and utilities.
//!
//! This module provides a shared `DataFormat` enum that represents known file formats
//! without an "Unknown" variant. This allows CLI and library code to work with
//! a common set of format types.

use std::fmt;

/// Known data format types.
///
/// This enum represents file formats that biscuit-file can work with.
/// Unlike `FileType`, this enum has no "Unknown" variant — it only
/// represents formats we can handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataFormat {
    /// TOML configuration file.
    Toml,
    /// YAML configuration file.
    Yaml,
    /// JSON data file.
    Json,
    /// JSON5 data file (JSON with comments and trailing commas).
    Json5,
    /// Markdown document.
    Markdown,
    /// PDF document.
    Pdf,
    /// Plain text file.
    Text,
}

impl DataFormat {
    /// Returns the canonical file extension for this format.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::DataFormat;
    ///
    /// assert_eq!(DataFormat::Toml.extension(), "toml");
    /// assert_eq!(DataFormat::Yaml.extension(), "yaml");
    /// assert_eq!(DataFormat::Json.extension(), "json");
    /// ```
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Json5 => "json5",
            Self::Markdown => "md",
            Self::Pdf => "pdf",
            Self::Text => "txt",
        }
    }

    /// Returns the MIME type for this format.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::DataFormat;
    ///
    /// assert_eq!(DataFormat::Toml.mime_type(), "application/toml");
    /// assert_eq!(DataFormat::Yaml.mime_type(), "application/yaml");
    /// assert_eq!(DataFormat::Json.mime_type(), "application/json");
    /// ```
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Toml => "application/toml",
            Self::Yaml => "application/yaml",
            Self::Json => "application/json",
            Self::Json5 => "application/json5",
            Self::Markdown => "text/markdown",
            Self::Pdf => "application/pdf",
            Self::Text => "text/plain",
        }
    }

    /// Returns `true` if this format requires UTF-8 encoding.
    ///
    /// Formats that require UTF-8 are text-based and may contain
    /// Unicode characters beyond ASCII.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::DataFormat;
    ///
    /// assert!(DataFormat::Toml.requires_utf8());
    /// assert!(DataFormat::Markdown.requires_utf8());
    /// assert!(!DataFormat::Pdf.requires_utf8());
    /// ```
    #[must_use]
    pub fn requires_utf8(&self) -> bool {
        matches!(self, Self::Toml | Self::Json5 | Self::Markdown | Self::Text)
    }

    /// Returns `true` if this format represents structured data.
    ///
    /// Structured formats can be parsed into a tree or object model.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::DataFormat;
    ///
    /// assert!(DataFormat::Toml.is_structured());
    /// assert!(DataFormat::Json.is_structured());
    /// assert!(!DataFormat::Markdown.is_structured());
    /// ```
    #[must_use]
    pub fn is_structured(&self) -> bool {
        matches!(self, Self::Toml | Self::Yaml | Self::Json | Self::Json5)
    }
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Json => "JSON",
            Self::Json5 => "JSON5",
            Self::Markdown => "Markdown",
            Self::Pdf => "PDF",
            Self::Text => "Text",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_covers_all_variants() {
        // Ensure every variant has an extension
        let formats = [
            DataFormat::Toml,
            DataFormat::Yaml,
            DataFormat::Json,
            DataFormat::Json5,
            DataFormat::Markdown,
            DataFormat::Pdf,
            DataFormat::Text,
        ];

        for format in &formats {
            assert!(
                !format.extension().is_empty(),
                "{format:?} should have an extension"
            );
        }
    }

    #[test]
    fn mime_type_covers_all_variants() {
        // Ensure every variant has a MIME type
        let formats = [
            DataFormat::Toml,
            DataFormat::Yaml,
            DataFormat::Json,
            DataFormat::Json5,
            DataFormat::Markdown,
            DataFormat::Pdf,
            DataFormat::Text,
        ];

        for format in &formats {
            assert!(
                !format.mime_type().is_empty(),
                "{format:?} should have a MIME type"
            );
        }
    }

    #[test]
    fn utf8_formats() {
        // Test UTF-8 requirement
        assert!(DataFormat::Toml.requires_utf8());
        assert!(DataFormat::Json5.requires_utf8());
        assert!(DataFormat::Markdown.requires_utf8());
        assert!(DataFormat::Text.requires_utf8());

        // Non-UTF-8 formats
        assert!(!DataFormat::Yaml.requires_utf8());
        assert!(!DataFormat::Json.requires_utf8());
        assert!(!DataFormat::Pdf.requires_utf8());
    }

    #[test]
    fn structured_formats() {
        // Structured formats
        assert!(DataFormat::Toml.is_structured());
        assert!(DataFormat::Yaml.is_structured());
        assert!(DataFormat::Json.is_structured());
        assert!(DataFormat::Json5.is_structured());

        // Non-structured formats
        assert!(!DataFormat::Markdown.is_structured());
        assert!(!DataFormat::Pdf.is_structured());
        assert!(!DataFormat::Text.is_structured());
    }

    #[test]
    fn display_impl() {
        assert_eq!(DataFormat::Toml.to_string(), "TOML");
        assert_eq!(DataFormat::Yaml.to_string(), "YAML");
        assert_eq!(DataFormat::Json.to_string(), "JSON");
        assert_eq!(DataFormat::Json5.to_string(), "JSON5");
        assert_eq!(DataFormat::Markdown.to_string(), "Markdown");
        assert_eq!(DataFormat::Pdf.to_string(), "PDF");
        assert_eq!(DataFormat::Text.to_string(), "Text");
    }
}
