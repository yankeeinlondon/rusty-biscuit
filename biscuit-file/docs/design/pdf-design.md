# PDF Library Design

## Context

This design defines the library-side PDF functionality for the shared file utilities. It aligns with the core requirement to provide a `Pdf` struct that can parse a file, expose text and Markdown conversions, and surface a table of contents.

## Goals

- Provide a stable `Pdf` API: `new`, `as_text`, `as_markdown`, and `toc`.
- Support multiple backends with graceful fallback for text, layout, images, and TOC.
- Preserve links and basic formatting when generating Markdown.
- Extract images and emit correct Markdown references.
- Keep default dependencies light while allowing higher-fidelity extraction as opt-in.

## Non-goals

- PDF creation or modification.
- OCR or scanned-document recognition (future extension).
- Perfect visual fidelity; focus on semantic structure and readable output.

## Public API (Proposed)

```rust
pub struct Pdf {
    source: PdfSource,
    config: PdfConfig,
    engine: PdfEngine,
}

impl Pdf {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PdfError>;
    pub fn as_text(&self) -> Result<String, PdfError>;
    pub fn as_markdown(&self, options: MarkdownOptions) -> Result<PdfMarkdown, PdfError>;
    pub fn toc(&self) -> Result<PdfToc, PdfError>;
}

impl TryFrom<&str> for Pdf;
impl TryFrom<String> for Pdf;
impl TryFrom<std::path::PathBuf> for Pdf;
```

Notes:

- `as_markdown` returns a `PdfMarkdown` wrapper so we can include extracted assets and warnings.
- `TryFrom` uses `Pdf::new` and treats the input as a file path.

## Configuration

```rust
pub struct PdfConfig {
    pub backend_preference: BackendPreference,
    pub password: Option<String>,
    pub page_range: Option<PageRange>,
    pub max_pages: Option<usize>,
    pub normalize_text: bool,
    pub remove_headers_footers: bool,
}

pub struct MarkdownOptions {
    pub assets_dir: Option<std::path::PathBuf>,
    pub image_mode: ImageMode,
    pub include_page_breaks: bool,
    pub heading_strategy: HeadingStrategy,
    pub table_strategy: TableStrategy,
}

pub struct TextOptions {
    pub include_page_breaks: bool,
    pub normalize_text: bool,
    pub remove_headers_footers: bool,
}
```

Defaults:

- `backend_preference`: Auto (best available).
- `normalize_text`: true (collapse whitespace, de-hyphenate).
- `remove_headers_footers`: true (detect repeated lines across pages).
- `image_mode`: ExternalFiles (safer, smaller markdown).

## Data Model

```rust
pub struct PdfDocument {
    pub metadata: PdfMetadata,
    pub pages: Vec<PdfPage>,
    pub toc: Option<PdfToc>,
}

pub struct PdfPage {
    pub index: usize,
    pub size: PdfPageSize,
    pub blocks: Vec<PdfBlock>,
    pub images: Vec<PdfImage>,
    pub links: Vec<PdfLink>,
}

pub enum PdfBlock {
    Heading(PdfTextBlock),
    Paragraph(PdfTextBlock),
    List(PdfListBlock),
    Table(PdfTableBlock),
    Image(PdfImageBlock),
    RawText(PdfTextBlock),
}

pub struct PdfTextSpan {
    pub text: String,
    pub bbox: PdfRect,
    pub style: PdfTextStyle,
}
```

The internal model supports both layout-aware extraction and fallback plain text by using `RawText` blocks when layout data is not available.

## Backend Architecture

### Trait Split

```rust
pub trait TextBackend {
    fn extract_text(&self, range: PageRange) -> Result<Vec<String>, PdfError>;
}

pub trait LayoutBackend {
    fn extract_layout(&self, range: PageRange) -> Result<PdfLayoutDoc, PdfError>;
}

pub trait TocBackend {
    fn extract_toc(&self) -> Result<Option<PdfToc>, PdfError>;
}

pub trait ImageBackend {
    fn extract_images(&self, range: PageRange) -> Result<Vec<PdfImage>, PdfError>;
}
```

### Recommended Backends

- `pdfium-render` (feature: `pdfium`): best layout, images, annotations, and TOC.
- `pdf-extract` (feature: `extract`): fast, simple text extraction fallback.
- `lopdf` (feature: `lopdf`): lightweight TOC and metadata extraction fallback.

### Engine Selection

- If `backend_preference` is set, use it or return `PdfError::BackendUnavailable`.
- Otherwise, choose the best available backend for each capability:
  - Layout + images + TOC: `pdfium-render`.
  - Text only: `pdf-extract`.
  - TOC only: `lopdf`.

This yields a composite engine that can mix backends per capability.

## Extraction Pipeline

1. Load source bytes from file path.
2. Initialize available backends based on features and runtime availability.
3. Extract layout if available; otherwise extract text.
4. Normalize text:
   - collapse repeated whitespace
   - de-hyphenate line breaks
   - normalize unicode and control chars
5. Detect headers and footers by repeated lines across pages and remove if enabled.
6. Build `PdfDocument` blocks from layout or plain text.
7. Convert to Markdown or Text as requested.

## Markdown Conversion Details

### Reading Order

- Group spans into lines using Y-axis clustering with tolerance.
- Sort lines top-to-bottom and left-to-right.
- Detect columns by clustering X-start positions; process columns left-to-right.

### Headings

- Compute font-size histogram per page.
- Map largest sizes to `#`/`##`/`###` using a relative threshold to body size.
- Fallback when no font info: detect short all-caps lines or lines surrounded by blank space.

### Lists

- Detect bullet glyphs ("-", "*", "o") and numeric patterns ("1.", "1)").
- Use X-indentation to determine nesting level.

### Tables

- Layout backend: detect column boundaries from aligned text spans.
- Fallback: infer columns from consistent whitespace gaps on multiple lines.
- Emit GitHub Flavored Markdown tables when confidence is high; otherwise emit text blocks.

### Links

- Use link annotations when backend provides them.
- If a link overlaps a text span, emit `[text](url)`; otherwise emit `<url>`.

### Images

- Extract images when backend supports it.
- Write assets to `assets_dir` and emit `![alt](relative/path)`.
- If `assets_dir` is not set and `image_mode` is InlineDataUri, embed images as data URIs.

### Page Breaks

- When enabled, emit `\n\n---\n\n` between pages.

## Text Conversion

- Use the same reading order logic as Markdown.
- Preserve paragraph breaks and optional page breaks.
- Never emit Markdown syntax.

## Table of Contents

- Prefer PDF outlines/bookmarks (pdfium or lopdf).
- Fallback: build a synthetic TOC from detected headings.

```rust
pub struct PdfTocItem {
    pub title: String,
    pub page_index: usize,
    pub children: Vec<PdfTocItem>,
}
```

## Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum PdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("PDF parse error: {0}")]
    Parse(String),
    #[error("Encrypted PDF requires a password")]
    Encrypted,
    #[error("Image extraction error: {0}")]
    Image(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}
```

## Feature Flags and Dependencies

Proposed feature gates to keep defaults light:

- `extract` (default): enables `pdf-extract` text backend.
- `lopdf` (default): enables TOC/metadata fallback.
- `pdfium`: enables `pdfium-render` backend and image extraction.
- `image`: enables `image` crate usage when image extraction is on.
- `full`: enables all backends.

Dependency additions should be checked against the repository dependency policy before implementation.

## Performance and Memory

- Support `page_range` and `max_pages` to cap work.
- Avoid loading all pages when only TOC is requested.
- Cache `PdfDocument` in memory after first build.
- Consider optional rayon usage for page-level extraction in the future.

## Testing Strategy

- Unit tests for normalization and layout heuristics.
- Integration fixtures:
  - text-only PDF
  - multi-column PDF
  - PDF with images
  - PDF with bookmarks/TOC
  - encrypted PDF (error path)
- Golden Markdown outputs for stable fixtures.

## Implementation Plan

1. Define `Pdf`, config types, and error enum.
2. Implement `pdf-extract` backend for text-only paths.
3. Implement TOC fallback via `lopdf`.
4. Add layout model and Markdown emitter with simple heuristics.
5. Add `pdfium-render` backend behind feature gate for richer extraction.
6. Expand heuristics for lists, tables, links, and images.

## Open Questions

- Should `as_markdown` return `String` or `PdfMarkdown` with assets and warnings?
- Do we want a `from_bytes` constructor for in-memory PDFs?
- Is a generic `PdfDocument` export needed, or should it remain internal?
