//! Core types for PDF handling.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Source tracking for PDF content.
#[derive(Debug, Clone)]
pub enum PdfSource {
    /// Content loaded from a file path.
    Path(PathBuf),
    /// Content provided as bytes.
    Bytes(Vec<u8>),
}

/// Error type for PDF operations.
#[derive(Debug, Error)]
pub enum PdfError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Backend not available.
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),

    /// PDF parsing error.
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// PDF is encrypted and requires a password.
    #[error("Encrypted PDF requires a password")]
    Encrypted,

    /// Image extraction error.
    #[error("Image extraction error: {0}")]
    Image(String),

    /// Unsupported operation.
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

/// Backend preference for PDF operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BackendPreference {
    /// Automatically select the best available backend.
    #[default]
    Auto,
    /// Use pdf-extract for text extraction.
    Extract,
    /// Use lopdf for metadata and TOC.
    Lopdf,
    /// Use pdfium for full-fidelity extraction.
    Pdfium,
}

/// Page range specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    /// Starting page (1-indexed).
    pub start: usize,
    /// Ending page (1-indexed, inclusive).
    pub end: Option<usize>,
}

impl PageRange {
    /// Create a range for all pages.
    #[must_use]
    pub fn all() -> Self {
        Self { start: 1, end: None }
    }

    /// Create a range for a single page.
    #[must_use]
    pub fn single(page: usize) -> Self {
        Self {
            start: page,
            end: Some(page),
        }
    }

    /// Create a range from start to end (inclusive).
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }

    /// Create a range from a starting page to the end.
    #[must_use]
    pub fn from(start: usize) -> Self {
        Self { start, end: None }
    }
}

impl Default for PageRange {
    fn default() -> Self {
        Self::all()
    }
}

/// Image handling mode for Markdown output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageMode {
    /// Write images to external files.
    #[default]
    ExternalFiles,
    /// Embed images as data URIs.
    InlineDataUri,
    /// Skip images entirely.
    Skip,
}

/// Strategy for detecting headings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeadingStrategy {
    /// Use font size histogram to detect headings.
    #[default]
    FontSize,
    /// Detect short all-caps lines.
    AllCaps,
    /// Combine multiple heuristics.
    Combined,
}

/// Strategy for detecting tables.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TableStrategy {
    /// Use column alignment detection.
    #[default]
    ColumnAlignment,
    /// Use whitespace gap analysis.
    WhitespaceGaps,
    /// Skip table detection.
    Skip,
}

/// Configuration for PDF operations.
#[derive(Debug, Clone)]
pub struct PdfConfig {
    /// Preferred backend for operations.
    pub backend_preference: BackendPreference,
    /// Password for encrypted PDFs.
    pub password: Option<String>,
    /// Page range to process.
    pub page_range: PageRange,
    /// Maximum number of pages to process.
    pub max_pages: Option<usize>,
    /// Normalize text (collapse whitespace, de-hyphenate).
    pub normalize_text: bool,
    /// Remove detected headers and footers.
    pub remove_headers_footers: bool,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            backend_preference: BackendPreference::Auto,
            password: None,
            page_range: PageRange::all(),
            max_pages: None,
            normalize_text: true,
            remove_headers_footers: true,
        }
    }
}

/// Options for Markdown conversion.
#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    /// Directory to write extracted assets.
    pub assets_dir: Option<PathBuf>,
    /// How to handle images.
    pub image_mode: ImageMode,
    /// Whether to include page break markers.
    pub include_page_breaks: bool,
    /// Strategy for heading detection.
    pub heading_strategy: HeadingStrategy,
    /// Strategy for table detection.
    pub table_strategy: TableStrategy,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            assets_dir: None,
            image_mode: ImageMode::ExternalFiles,
            include_page_breaks: true,
            heading_strategy: HeadingStrategy::FontSize,
            table_strategy: TableStrategy::ColumnAlignment,
        }
    }
}

/// Options for text extraction.
#[derive(Debug, Clone)]
pub struct TextOptions {
    /// Whether to include page break markers.
    pub include_page_breaks: bool,
    /// Normalize text (collapse whitespace, de-hyphenate).
    pub normalize_text: bool,
    /// Remove detected headers and footers.
    pub remove_headers_footers: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            include_page_breaks: false,
            normalize_text: true,
            remove_headers_footers: true,
        }
    }
}

/// Markdown output with associated assets and warnings.
#[derive(Debug, Clone)]
pub struct PdfMarkdown {
    /// The Markdown content.
    pub content: String,
    /// Paths to extracted assets (images).
    pub assets: Vec<PathBuf>,
    /// Warnings generated during conversion.
    pub warnings: Vec<String>,
}

impl PdfMarkdown {
    /// Create a new PdfMarkdown with just content.
    #[must_use]
    pub fn new(content: String) -> Self {
        Self {
            content,
            assets: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Table of contents entry.
#[derive(Debug, Clone)]
pub struct PdfTocItem {
    /// Title of the entry.
    pub title: String,
    /// Page number (1-indexed).
    pub page_index: usize,
    /// Nested children.
    pub children: Vec<PdfTocItem>,
}

/// Table of contents for a PDF.
#[derive(Debug, Clone, Default)]
pub struct PdfToc {
    /// Top-level entries.
    pub items: Vec<PdfTocItem>,
}

impl PdfToc {
    /// Returns true if the TOC is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the total number of entries (including nested).
    #[must_use]
    pub fn len(&self) -> usize {
        fn count(items: &[PdfTocItem]) -> usize {
            items.iter().map(|i| 1 + count(&i.children)).sum()
        }
        count(&self.items)
    }
}

/// PDF document with extraction and conversion capabilities.
#[derive(Debug)]
pub struct Pdf {
    source: PdfSource,
    config: PdfConfig,
    bytes: Vec<u8>,
}

impl Pdf {
    /// Open a PDF file from the given path.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or is invalid.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, PdfError> {
        Self::with_config(path, PdfConfig::default())
    }

    /// Open a PDF file with custom configuration.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or is invalid.
    pub fn with_config(path: impl AsRef<Path>, config: PdfConfig) -> Result<Self, PdfError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;

        // Basic validation - check for PDF magic bytes
        if !bytes.starts_with(b"%PDF-") {
            return Err(PdfError::Parse("Not a valid PDF file".to_string()));
        }

        Ok(Self {
            source: PdfSource::Path(path.to_path_buf()),
            config,
            bytes,
        })
    }

    /// Create from raw bytes.
    ///
    /// ## Errors
    ///
    /// Returns an error if the bytes are not a valid PDF.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, PdfError> {
        Self::from_bytes_with_config(bytes, PdfConfig::default())
    }

    /// Create from raw bytes with custom configuration.
    ///
    /// ## Errors
    ///
    /// Returns an error if the bytes are not a valid PDF.
    pub fn from_bytes_with_config(bytes: Vec<u8>, config: PdfConfig) -> Result<Self, PdfError> {
        if !bytes.starts_with(b"%PDF-") {
            return Err(PdfError::Parse("Not a valid PDF file".to_string()));
        }

        Ok(Self {
            source: PdfSource::Bytes(bytes.clone()),
            config,
            bytes,
        })
    }

    /// Returns the source of this PDF document.
    #[must_use]
    pub fn source(&self) -> &PdfSource {
        &self.source
    }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &PdfConfig {
        &self.config
    }

    /// Returns the raw PDF bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Extract text from the PDF.
    ///
    /// ## Errors
    ///
    /// Returns an error if text extraction fails.
    #[cfg(feature = "extract")]
    pub fn as_text(&self) -> Result<String, PdfError> {
        use super::backends::extract_text;
        extract_text(&self.bytes, &self.config)
    }

    /// Extract text from the PDF.
    #[cfg(not(feature = "extract"))]
    pub fn as_text(&self) -> Result<String, PdfError> {
        Err(PdfError::BackendUnavailable(
            "Text extraction requires the 'extract' feature".to_string(),
        ))
    }

    /// Convert the PDF to Markdown.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails.
    pub fn as_markdown(&self, _options: MarkdownOptions) -> Result<PdfMarkdown, PdfError> {
        // For Phase 1, just wrap the text output in markdown
        let text = self.as_text()?;
        Ok(PdfMarkdown::new(text))
    }

    /// Extract the table of contents.
    ///
    /// ## Errors
    ///
    /// Returns an error if TOC extraction fails.
    #[cfg(feature = "lopdf")]
    pub fn toc(&self) -> Result<PdfToc, PdfError> {
        use super::backends::extract_toc;
        extract_toc(&self.bytes)
    }

    /// Extract the table of contents.
    #[cfg(not(feature = "lopdf"))]
    pub fn toc(&self) -> Result<PdfToc, PdfError> {
        Err(PdfError::BackendUnavailable(
            "TOC extraction requires the 'lopdf' feature".to_string(),
        ))
    }
}

// TryFrom implementations

impl TryFrom<&str> for Pdf {
    type Error = PdfError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<String> for Pdf {
    type Error = PdfError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<PathBuf> for Pdf {
    type Error = PdfError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // PdfConfig default tests - verify alignment with design doc
    // ==========================================================================

    #[test]
    fn test_pdf_config_defaults_match_design() {
        let config = PdfConfig::default();

        // From design doc: backend_preference: Auto (best available)
        assert_eq!(
            config.backend_preference,
            BackendPreference::Auto,
            "backend_preference should default to Auto"
        );

        // From design doc: normalize_text: true (collapse whitespace, de-hyphenate)
        assert!(
            config.normalize_text,
            "normalize_text should default to true"
        );

        // From design doc: remove_headers_footers: true (detect repeated lines across pages)
        assert!(
            config.remove_headers_footers,
            "remove_headers_footers should default to true"
        );

        // password should be None by default (no encrypted PDF)
        assert!(config.password.is_none(), "password should default to None");

        // page_range should default to all pages
        assert_eq!(
            config.page_range,
            PageRange::all(),
            "page_range should default to all pages"
        );

        // max_pages should be None (no limit) by default
        assert!(
            config.max_pages.is_none(),
            "max_pages should default to None"
        );
    }

    #[test]
    fn test_markdown_options_defaults_match_design() {
        let options = MarkdownOptions::default();

        // From design doc: image_mode: ExternalFiles (safer, smaller markdown)
        assert_eq!(
            options.image_mode,
            ImageMode::ExternalFiles,
            "image_mode should default to ExternalFiles"
        );

        // assets_dir should be None by default
        assert!(
            options.assets_dir.is_none(),
            "assets_dir should default to None"
        );

        // include_page_breaks should be true by default
        assert!(
            options.include_page_breaks,
            "include_page_breaks should default to true"
        );

        // heading_strategy defaults to FontSize
        assert_eq!(
            options.heading_strategy,
            HeadingStrategy::FontSize,
            "heading_strategy should default to FontSize"
        );

        // table_strategy defaults to ColumnAlignment
        assert_eq!(
            options.table_strategy,
            TableStrategy::ColumnAlignment,
            "table_strategy should default to ColumnAlignment"
        );
    }

    #[test]
    fn test_text_options_defaults() {
        let options = TextOptions::default();

        // Text extraction should not include page breaks by default
        assert!(
            !options.include_page_breaks,
            "include_page_breaks should default to false for text"
        );

        // normalize_text should be true (matches PdfConfig)
        assert!(
            options.normalize_text,
            "normalize_text should default to true"
        );

        // remove_headers_footers should be true (matches PdfConfig)
        assert!(
            options.remove_headers_footers,
            "remove_headers_footers should default to true"
        );
    }

    // ==========================================================================
    // PageRange tests
    // ==========================================================================

    #[test]
    fn test_page_range_all() {
        let all = PageRange::all();
        assert_eq!(all.start, 1);
        assert_eq!(all.end, None);
    }

    #[test]
    fn test_page_range_single() {
        let single = PageRange::single(5);
        assert_eq!(single.start, 5);
        assert_eq!(single.end, Some(5));
    }

    #[test]
    fn test_page_range_new() {
        let range = PageRange::new(2, 10);
        assert_eq!(range.start, 2);
        assert_eq!(range.end, Some(10));
    }

    #[test]
    fn test_page_range_from() {
        let range = PageRange::from(5);
        assert_eq!(range.start, 5);
        assert_eq!(range.end, None);
    }

    #[test]
    fn test_page_range_default() {
        let range = PageRange::default();
        assert_eq!(range, PageRange::all());
    }

    // ==========================================================================
    // BackendPreference tests
    // ==========================================================================

    #[test]
    fn test_backend_preference_default() {
        assert_eq!(BackendPreference::default(), BackendPreference::Auto);
    }

    #[test]
    fn test_backend_preference_variants() {
        // Ensure all variants are accessible and distinct
        let auto = BackendPreference::Auto;
        let extract = BackendPreference::Extract;
        let lopdf = BackendPreference::Lopdf;
        let pdfium = BackendPreference::Pdfium;

        assert_ne!(auto, extract);
        assert_ne!(extract, lopdf);
        assert_ne!(lopdf, pdfium);
    }

    // ==========================================================================
    // ImageMode tests
    // ==========================================================================

    #[test]
    fn test_image_mode_default() {
        assert_eq!(ImageMode::default(), ImageMode::ExternalFiles);
    }

    #[test]
    fn test_image_mode_variants() {
        let external = ImageMode::ExternalFiles;
        let inline = ImageMode::InlineDataUri;
        let skip = ImageMode::Skip;

        assert_ne!(external, inline);
        assert_ne!(inline, skip);
    }

    // ==========================================================================
    // HeadingStrategy tests
    // ==========================================================================

    #[test]
    fn test_heading_strategy_default() {
        assert_eq!(HeadingStrategy::default(), HeadingStrategy::FontSize);
    }

    // ==========================================================================
    // TableStrategy tests
    // ==========================================================================

    #[test]
    fn test_table_strategy_default() {
        assert_eq!(TableStrategy::default(), TableStrategy::ColumnAlignment);
    }

    // ==========================================================================
    // PdfToc tests
    // ==========================================================================

    #[test]
    fn test_pdf_toc_empty() {
        let toc = PdfToc::default();
        assert!(toc.is_empty());
        assert_eq!(toc.len(), 0);
    }

    #[test]
    fn test_pdf_toc_nested() {
        let toc = PdfToc {
            items: vec![
                PdfTocItem {
                    title: "Chapter 1".to_string(),
                    page_index: 1,
                    children: vec![PdfTocItem {
                        title: "Section 1.1".to_string(),
                        page_index: 2,
                        children: vec![],
                    }],
                },
                PdfTocItem {
                    title: "Chapter 2".to_string(),
                    page_index: 10,
                    children: vec![],
                },
            ],
        };

        assert!(!toc.is_empty());
        assert_eq!(toc.len(), 3); // 2 chapters + 1 section
    }

    #[test]
    fn test_pdf_toc_deeply_nested() {
        let toc = PdfToc {
            items: vec![PdfTocItem {
                title: "Root".to_string(),
                page_index: 1,
                children: vec![PdfTocItem {
                    title: "Level 1".to_string(),
                    page_index: 2,
                    children: vec![PdfTocItem {
                        title: "Level 2".to_string(),
                        page_index: 3,
                        children: vec![PdfTocItem {
                            title: "Level 3".to_string(),
                            page_index: 4,
                            children: vec![],
                        }],
                    }],
                }],
            }],
        };

        assert_eq!(toc.len(), 4);
    }

    // ==========================================================================
    // PdfMarkdown tests
    // ==========================================================================

    #[test]
    fn test_pdf_markdown_new() {
        let md = PdfMarkdown::new("# Title\n\nContent".to_string());
        assert_eq!(md.content, "# Title\n\nContent");
        assert!(md.assets.is_empty());
        assert!(md.warnings.is_empty());
    }

    #[test]
    fn test_pdf_markdown_with_assets() {
        let mut md = PdfMarkdown::new("Content".to_string());
        md.assets.push(PathBuf::from("assets/image1.png"));
        md.assets.push(PathBuf::from("assets/image2.jpg"));

        assert_eq!(md.assets.len(), 2);
    }

    #[test]
    fn test_pdf_markdown_with_warnings() {
        let mut md = PdfMarkdown::new("Content".to_string());
        md.warnings.push("Could not extract image on page 3".to_string());

        assert_eq!(md.warnings.len(), 1);
    }

    // ==========================================================================
    // PdfSource tests
    // ==========================================================================

    #[test]
    fn test_pdf_source_path() {
        let source = PdfSource::Path(PathBuf::from("/tmp/test.pdf"));
        if let PdfSource::Path(p) = source {
            assert_eq!(p, PathBuf::from("/tmp/test.pdf"));
        } else {
            panic!("Expected PdfSource::Path");
        }
    }

    #[test]
    fn test_pdf_source_bytes() {
        let source = PdfSource::Bytes(vec![1, 2, 3]);
        if let PdfSource::Bytes(b) = source {
            assert_eq!(b, vec![1, 2, 3]);
        } else {
            panic!("Expected PdfSource::Bytes");
        }
    }

    // ==========================================================================
    // PdfError tests
    // ==========================================================================

    #[test]
    fn test_pdf_error_display() {
        let io_err = PdfError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("IO error"));

        let backend_err = PdfError::BackendUnavailable("pdfium".to_string());
        assert!(backend_err.to_string().contains("Backend unavailable"));

        let parse_err = PdfError::Parse("invalid structure".to_string());
        assert!(parse_err.to_string().contains("PDF parse error"));

        let encrypted_err = PdfError::Encrypted;
        assert!(encrypted_err.to_string().contains("password"));

        let image_err = PdfError::Image("corrupt image".to_string());
        assert!(image_err.to_string().contains("Image extraction"));

        let unsupported_err = PdfError::Unsupported("feature X".to_string());
        assert!(unsupported_err.to_string().contains("Unsupported"));
    }

    // ==========================================================================
    // Pdf struct tests
    // ==========================================================================

    #[test]
    fn test_invalid_pdf_bytes() {
        let result = Pdf::from_bytes(b"not a pdf".to_vec());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::Parse(_)));
    }

    #[test]
    fn test_valid_pdf_magic_bytes() {
        // Minimal valid PDF header (just the magic bytes)
        let minimal_pdf = b"%PDF-1.4\n%%EOF";
        let result = Pdf::from_bytes(minimal_pdf.to_vec());
        assert!(result.is_ok());
    }

    #[test]
    fn test_pdf_accessors() {
        let pdf_bytes = b"%PDF-1.4\n%%EOF".to_vec();
        let pdf = Pdf::from_bytes(pdf_bytes.clone()).unwrap();

        // Test source accessor
        assert!(matches!(pdf.source(), PdfSource::Bytes(_)));

        // Test config accessor
        assert_eq!(pdf.config().backend_preference, BackendPreference::Auto);

        // Test bytes accessor
        assert_eq!(pdf.bytes(), pdf_bytes.as_slice());
    }

    #[test]
    fn test_pdf_with_custom_config() {
        let config = PdfConfig {
            backend_preference: BackendPreference::Extract,
            password: Some("secret".to_string()),
            page_range: PageRange::new(1, 5),
            max_pages: Some(10),
            normalize_text: false,
            remove_headers_footers: false,
        };

        let pdf_bytes = b"%PDF-1.4\n%%EOF".to_vec();
        let pdf = Pdf::from_bytes_with_config(pdf_bytes, config).unwrap();

        assert_eq!(pdf.config().backend_preference, BackendPreference::Extract);
        assert_eq!(pdf.config().password, Some("secret".to_string()));
        assert_eq!(pdf.config().page_range, PageRange::new(1, 5));
        assert_eq!(pdf.config().max_pages, Some(10));
        assert!(!pdf.config().normalize_text);
        assert!(!pdf.config().remove_headers_footers);
    }

    #[test]
    fn test_pdf_file_not_found() {
        let result = Pdf::new("/nonexistent/path/to/file.pdf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::Io(_)));
    }
}
