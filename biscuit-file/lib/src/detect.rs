//! File type detection using magic bytes and extensions.
//!
//! This module provides utilities to detect whether a file is TOML, YAML, or PDF
//! based on its content (magic bytes) and file extension.

use std::path::Path;
use tracing::{debug, trace};

use crate::format::DataFormat;

/// Detected file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// TOML configuration file.
    Toml,
    /// YAML configuration file.
    Yaml,
    /// JSON data file.
    Json,
    /// JSON5 data file.
    Json5,
    /// Markdown document.
    Markdown,
    /// PDF document.
    Pdf,
    /// Unknown or unsupported file type.
    Unknown,
}

impl FileType {
    /// Returns the canonical file extension for this type.
    #[must_use]
    pub fn extension(&self) -> Option<&'static str> {
        match self {
            Self::Toml => Some("toml"),
            Self::Yaml => Some("yaml"),
            Self::Json => Some("json"),
            Self::Json5 => Some("json5"),
            Self::Markdown => Some("md"),
            Self::Pdf => Some("pdf"),
            Self::Unknown => None,
        }
    }

    /// Returns the MIME type for this file type.
    #[must_use]
    pub fn mime_type(&self) -> Option<&'static str> {
        match self {
            Self::Toml => Some("application/toml"),
            Self::Yaml => Some("application/yaml"),
            Self::Json => Some("application/json"),
            Self::Json5 => Some("application/json5"),
            Self::Markdown => Some("text/markdown"),
            Self::Pdf => Some("application/pdf"),
            Self::Unknown => None,
        }
    }

    /// Convert to a `DataFormat` if this is a known type.
    ///
    /// ## Returns
    ///
    /// Returns `Some(DataFormat)` for all known types, or `None` for `Unknown`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::{FileType, DataFormat};
    ///
    /// assert_eq!(FileType::Toml.as_data_format(), Some(DataFormat::Toml));
    /// assert_eq!(FileType::Unknown.as_data_format(), None);
    /// ```
    #[must_use]
    pub fn as_data_format(&self) -> Option<DataFormat> {
        match self {
            Self::Toml => Some(DataFormat::Toml),
            Self::Yaml => Some(DataFormat::Yaml),
            Self::Json => Some(DataFormat::Json),
            Self::Json5 => Some(DataFormat::Json5),
            Self::Markdown => Some(DataFormat::Markdown),
            Self::Pdf => Some(DataFormat::Pdf),
            Self::Unknown => None,
        }
    }
}

impl From<DataFormat> for FileType {
    fn from(fmt: DataFormat) -> Self {
        match fmt {
            DataFormat::Toml => Self::Toml,
            DataFormat::Yaml => Self::Yaml,
            DataFormat::Json => Self::Json,
            DataFormat::Json5 => Self::Json5,
            DataFormat::Markdown => Self::Markdown,
            DataFormat::Pdf => Self::Pdf,
            DataFormat::Text => Self::Unknown,
        }
    }
}

/// Detect file type from a file path using extension and magic bytes.
///
/// This function first attempts to detect the type from file content (magic bytes),
/// then falls back to extension-based detection.
///
/// ## Errors
///
/// Returns an IO error if the file cannot be read.
pub fn detect_file_type(path: impl AsRef<Path>) -> std::io::Result<FileType> {
    use std::io::Read;

    let path = path.as_ref();
    trace!(?path, "detecting file type");

    // Read only first 512 bytes for magic byte detection
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 512];
    let n = file.read(&mut buf)?;
    let from_bytes = detect_file_type_from_bytes(&buf[..n]);

    if from_bytes != FileType::Unknown {
        debug!(?from_bytes, "detected via magic bytes");
        return Ok(from_bytes);
    }

    // Fall back to extension-based detection
    let from_ext = detect_from_extension(path);
    debug!(?from_ext, "detected via extension");
    Ok(from_ext)
}

/// Detect file type from raw bytes using magic bytes.
///
/// PDF files start with `%PDF-`. TOML and YAML cannot be reliably detected
/// from magic bytes alone, so this function returns `Unknown` for text formats.
#[must_use]
pub fn detect_file_type_from_bytes(bytes: &[u8]) -> FileType {
    // PDF magic bytes: %PDF-
    if bytes.starts_with(b"%PDF-") {
        return FileType::Pdf;
    }

    // Use infer crate for additional magic byte detection
    if let Some(kind) = infer::get(bytes)
        && kind.mime_type() == "application/pdf"
    {
        return FileType::Pdf;
    }

    // TOML and YAML cannot be reliably detected from magic bytes
    FileType::Unknown
}

/// Detect file type from file extension.
fn detect_from_extension(path: &Path) -> FileType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("toml") => FileType::Toml,
        Some("yaml" | "yml") => FileType::Yaml,
        Some("json") => FileType::Json,
        Some("json5") => FileType::Json5,
        Some("md" | "markdown" | "mdx") => FileType::Markdown,
        Some("pdf") => FileType::Pdf,
        _ => FileType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_magic_bytes() {
        let pdf_bytes = b"%PDF-1.7\n";
        assert_eq!(detect_file_type_from_bytes(pdf_bytes), FileType::Pdf);
    }

    #[test]
    fn test_unknown_from_text() {
        let text = b"key = \"value\"";
        assert_eq!(detect_file_type_from_bytes(text), FileType::Unknown);
    }

    #[test]
    fn test_file_type_extension() {
        assert_eq!(FileType::Toml.extension(), Some("toml"));
        assert_eq!(FileType::Yaml.extension(), Some("yaml"));
        assert_eq!(FileType::Json.extension(), Some("json"));
        assert_eq!(FileType::Json5.extension(), Some("json5"));
        assert_eq!(FileType::Markdown.extension(), Some("md"));
        assert_eq!(FileType::Pdf.extension(), Some("pdf"));
        assert_eq!(FileType::Unknown.extension(), None);
    }

    #[test]
    fn test_file_type_mime() {
        assert_eq!(FileType::Toml.mime_type(), Some("application/toml"));
        assert_eq!(FileType::Yaml.mime_type(), Some("application/yaml"));
        assert_eq!(FileType::Json.mime_type(), Some("application/json"));
        assert_eq!(FileType::Json5.mime_type(), Some("application/json5"));
        assert_eq!(FileType::Markdown.mime_type(), Some("text/markdown"));
        assert_eq!(FileType::Pdf.mime_type(), Some("application/pdf"));
        assert_eq!(FileType::Unknown.mime_type(), None);
    }

    #[test]
    fn test_extension_detection() {
        assert_eq!(
            detect_from_extension(Path::new("config.toml")),
            FileType::Toml
        );
        assert_eq!(
            detect_from_extension(Path::new("config.yaml")),
            FileType::Yaml
        );
        assert_eq!(
            detect_from_extension(Path::new("config.yml")),
            FileType::Yaml
        );
        assert_eq!(
            detect_from_extension(Path::new("data.json")),
            FileType::Json
        );
        assert_eq!(
            detect_from_extension(Path::new("config.json5")),
            FileType::Json5
        );
        assert_eq!(
            detect_from_extension(Path::new("readme.md")),
            FileType::Markdown
        );
        assert_eq!(
            detect_from_extension(Path::new("readme.markdown")),
            FileType::Markdown
        );
        assert_eq!(
            detect_from_extension(Path::new("page.mdx")),
            FileType::Markdown
        );
        assert_eq!(detect_from_extension(Path::new("doc.pdf")), FileType::Pdf);
        assert_eq!(
            detect_from_extension(Path::new("file.txt")),
            FileType::Unknown
        );
    }

    #[test]
    fn test_detect_file_type_from_filesystem() {
        let dir = std::env::temp_dir().join("biscuit-file-test-detect");
        std::fs::create_dir_all(&dir).unwrap();
        let toml_path = dir.join("test.toml");
        std::fs::write(&toml_path, "key = \"value\"").unwrap();
        assert_eq!(detect_file_type(&toml_path).unwrap(), FileType::Toml);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_pdf_by_magic_bytes_wrong_extension() {
        let dir = std::env::temp_dir().join("biscuit-file-test-detect-magic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("document.txt");
        std::fs::write(&path, b"%PDF-1.7\nsome content here").unwrap();
        assert_eq!(detect_file_type(&path).unwrap(), FileType::Pdf);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_file_type_no_extension() {
        let dir = std::env::temp_dir().join("biscuit-file-test-detect-noext");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "key = \"value\"").unwrap();
        assert_eq!(detect_file_type(&path).unwrap(), FileType::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_known_variants_have_extension() {
        let variants = [
            FileType::Toml,
            FileType::Yaml,
            FileType::Json,
            FileType::Json5,
            FileType::Markdown,
            FileType::Pdf,
        ];
        for v in variants {
            assert!(
                v.extension().is_some(),
                "{v:?} should have a file extension"
            );
        }
        assert!(FileType::Unknown.extension().is_none());
    }

    #[test]
    fn all_known_variants_have_mime_type() {
        let variants = [
            FileType::Toml,
            FileType::Yaml,
            FileType::Json,
            FileType::Json5,
            FileType::Markdown,
            FileType::Pdf,
        ];
        for v in variants {
            assert!(v.mime_type().is_some(), "{v:?} should have a MIME type");
        }
        assert!(FileType::Unknown.mime_type().is_none());
    }

    #[test]
    fn all_known_variants_have_data_format() {
        let variants = [
            FileType::Toml,
            FileType::Yaml,
            FileType::Json,
            FileType::Json5,
            FileType::Markdown,
            FileType::Pdf,
        ];
        for v in variants {
            assert!(
                v.as_data_format().is_some(),
                "{v:?} should convert to DataFormat"
            );
        }
        assert!(FileType::Unknown.as_data_format().is_none());
    }
}
