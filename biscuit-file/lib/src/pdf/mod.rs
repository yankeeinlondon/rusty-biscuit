//! PDF parsing, text extraction, and conversion.
//!
//! This module provides the [`Pdf`] struct for working with PDF files,
//! including text extraction, Markdown conversion, and table of contents extraction.
//!
//! ## Backend Architecture
//!
//! The PDF module supports multiple backends with different capabilities:
//!
//! - **`extract`** (default): Fast text extraction via pdf-extract
//! - **`lopdf`** (default): TOC and metadata extraction
//! - **`pdfium`** (optional): High-fidelity layout and image extraction
//!
//! ## Examples
//!
//! ```rust,ignore
//! use biscuit_file::Pdf;
//!
//! // Parse from file
//! let pdf = Pdf::new("document.pdf")?;
//!
//! // Extract text
//! let text = pdf.as_text()?;
//!
//! // Convert to Markdown
//! let markdown = pdf.as_markdown(Default::default())?;
//!
//! // Extract table of contents
//! let toc = pdf.toc()?;
//! ```

mod types;

#[cfg(any(feature = "extract", feature = "lopdf"))]
mod backends;

pub use types::{
    BackendPreference, HeadingStrategy, ImageMode, MarkdownOptions, PageRange, Pdf, PdfConfig,
    PdfError, PdfMarkdown, PdfSource, PdfToc, PdfTocItem, TableStrategy, TextOptions,
};
