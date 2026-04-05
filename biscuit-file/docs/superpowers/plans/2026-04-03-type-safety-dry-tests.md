# Type Safety, DRY, and Test Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all 20 review items from the 2026-04-03 code review covering type safety improvements, DRY violations, and test coverage gaps across `biscuit-file/lib` and `biscuit-file/cli`.

**Architecture:** The changes fall into three categories: (1) shared types in the library that unify duplicated enums and enforce invariants, (2) CLI refactors that extract repeated output-format matching into a trait-based helper, and (3) new test modules covering under-tested core logic. All library changes are backwards-compatible via re-exports. The CLI is the only consumer so breaking CLI-internal types is fine.

**Tech Stack:** Rust 2024, serde, serde_json, serde_yaml_ng, toml, clap (derive), color-eyre, thiserror, tempfile (new dev-dep for tests)

**Build/Test commands:**
- Build: `cargo build -p biscuit-file -p biscuit-file-cli`
- Test lib: `cargo test -p biscuit-file`
- Test CLI: `cargo test -p biscuit-file-cli`
- Lint: `cargo clippy -p biscuit-file -p biscuit-file-cli -- -D warnings`
- All: `just test` (from `biscuit-file/`)

---

## Phase 1: Library Type Safety & DRY (Items 1.1–1.6, 2.1, 2.3, 2.4, 2.5, 3.7)

These tasks modify library types. Do them first since CLI changes depend on them.

---

### Task 1: Add `DataFormat` enum to library (Review 1.2, 2.4)

**Files:**
- Create: `biscuit-file/lib/src/format.rs`
- Modify: `biscuit-file/lib/src/lib.rs:92-114`
- Modify: `biscuit-file/lib/src/detect.rs:10-26`

This shared enum eliminates drift between `FileType` (lib), `InputFormat` (CLI), and `OutputFormat` (CLI). `FileType` becomes `Option<DataFormat>` conceptually — `Unknown` maps to `None`. The CLI types become thin wrappers.

- [ ] **Step 1: Write tests for `DataFormat`**

Create `biscuit-file/lib/src/format.rs`:

```rust
//! Shared data format enum used across input detection and output selection.

use std::fmt;

/// A concrete data format supported by biscuit-file.
///
/// Unlike `FileType` (which includes `Unknown` for detection failures),
/// `DataFormat` represents a known, usable format. Use this when a
/// definite format is required (e.g., output selection, conversion targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataFormat {
    /// TOML configuration format.
    Toml,
    /// YAML configuration format.
    Yaml,
    /// JSON data format.
    Json,
    /// JSON5 data format.
    Json5,
    /// Markdown document format.
    Markdown,
    /// PDF document format.
    Pdf,
    /// Plain text (output only).
    Text,
}

impl DataFormat {
    /// Returns the canonical file extension.
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

    /// Returns the MIME type.
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

    /// Whether this format requires UTF-8 text input (not binary).
    #[must_use]
    pub fn requires_utf8(&self) -> bool {
        matches!(self, Self::Toml | Self::Json5 | Self::Markdown | Self::Text)
    }

    /// Whether this format supports structured data conversion (JSON/YAML/TOML).
    #[must_use]
    pub fn is_structured(&self) -> bool {
        matches!(self, Self::Toml | Self::Yaml | Self::Json | Self::Json5)
    }
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Json => "JSON",
            Self::Json5 => "JSON5",
            Self::Markdown => "Markdown",
            Self::Pdf => "PDF",
            Self::Text => "Text",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_covers_all_variants() {
        let variants = [
            DataFormat::Toml,
            DataFormat::Yaml,
            DataFormat::Json,
            DataFormat::Json5,
            DataFormat::Markdown,
            DataFormat::Pdf,
            DataFormat::Text,
        ];
        for v in variants {
            assert!(!v.extension().is_empty(), "{v:?} has empty extension");
        }
    }

    #[test]
    fn mime_type_covers_all_variants() {
        let variants = [
            DataFormat::Toml,
            DataFormat::Yaml,
            DataFormat::Json,
            DataFormat::Json5,
            DataFormat::Markdown,
            DataFormat::Pdf,
            DataFormat::Text,
        ];
        for v in variants {
            assert!(v.mime_type().contains('/'), "{v:?} has invalid MIME type");
        }
    }

    #[test]
    fn utf8_formats() {
        assert!(DataFormat::Toml.requires_utf8());
        assert!(DataFormat::Json5.requires_utf8());
        assert!(DataFormat::Markdown.requires_utf8());
        assert!(DataFormat::Text.requires_utf8());
        assert!(!DataFormat::Json.requires_utf8());
        assert!(!DataFormat::Yaml.requires_utf8());
        assert!(!DataFormat::Pdf.requires_utf8());
    }

    #[test]
    fn structured_formats() {
        assert!(DataFormat::Toml.is_structured());
        assert!(DataFormat::Yaml.is_structured());
        assert!(DataFormat::Json.is_structured());
        assert!(DataFormat::Json5.is_structured());
        assert!(!DataFormat::Markdown.is_structured());
        assert!(!DataFormat::Pdf.is_structured());
        assert!(!DataFormat::Text.is_structured());
    }

    #[test]
    fn display_impl() {
        assert_eq!(DataFormat::Toml.to_string(), "TOML");
        assert_eq!(DataFormat::Yaml.to_string(), "YAML");
        assert_eq!(DataFormat::Json.to_string(), "JSON");
        assert_eq!(DataFormat::Text.to_string(), "Text");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p biscuit-file -- format`
Expected: Compilation error — module not declared.

- [ ] **Step 3: Wire the module into the library**

Add to `biscuit-file/lib/src/lib.rs` after `mod error;` (line 93):

```rust
mod format;
```

And in the re-export section after `pub use detect::{...}` (line 114):

```rust
// Re-export shared format type
pub use format::DataFormat;
```

- [ ] **Step 4: Add `From<DataFormat>` impl to `FileType`**

In `biscuit-file/lib/src/detect.rs`, add after the `impl FileType` block (after line 56):

```rust
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

impl FileType {
    /// Convert to a `DataFormat` if this is a known type.
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
```

Add `use crate::format::DataFormat;` at the top of `detect.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-file/lib/src/format.rs biscuit-file/lib/src/lib.rs biscuit-file/lib/src/detect.rs
git commit -m "feat(biscuit-file): add DataFormat enum unifying format representation"
```

---

### Task 2: Fix `as_yaml_value()` return type when yaml feature is disabled (Review 1.3)

**Files:**
- Modify: `biscuit-file/lib/src/toml_impl/types.rs:290-294`

- [ ] **Step 1: Write a test documenting the expected behavior**

This is a compile-time correctness issue. The fix is to remove the `as_yaml_value()` stub when `yaml` is disabled (matching how `Json5` handles disabled features — it simply doesn't provide the method). Add a test in `types.rs` in the existing `#[cfg(test)]` block:

```rust
#[test]
fn as_yaml_value_returns_yaml_value_type() {
    let toml = Toml::from_str("[foo]\nbar = 1").unwrap();
    let result = toml.as_yaml_value();
    assert!(result.is_ok());
    // Verify the return type is serde_yaml_ng::Value, not ()
    let _val: serde_yaml_ng::Value = result.unwrap();
}
```

- [ ] **Step 2: Fix the disabled-feature stub**

In `biscuit-file/lib/src/toml_impl/types.rs`, replace lines 290-294:

```rust
    /// Convert to a `serde_yaml_ng::Value`.
    #[cfg(not(feature = "yaml"))]
    pub fn as_yaml_value(&self) -> Result<(), TomlError> {
        Err(TomlError::YamlFeatureDisabled)
    }
```

with:

```rust
    // When yaml is disabled, as_yaml_value() is not provided.
    // This matches the pattern used by Json5 for disabled features.
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass (yaml feature is enabled by default).

- [ ] **Step 4: Commit**

```bash
git add biscuit-file/lib/src/toml_impl/types.rs
git commit -m "fix(biscuit-file): remove as_yaml_value stub when yaml feature disabled"
```

---

### Task 3: Make `PageRange` fields private with validation (Review 1.4)

**Files:**
- Modify: `biscuit-file/lib/src/pdf/types.rs:59-100`

- [ ] **Step 1: Write tests for validation**

Add in the `#[cfg(test)]` block in `pdf/types.rs`:

```rust
#[test]
fn page_range_rejects_zero_start() {
    let result = PageRange::try_new(0, Some(5));
    assert!(result.is_err());
}

#[test]
fn page_range_rejects_end_before_start() {
    let result = PageRange::try_new(5, Some(3));
    assert!(result.is_err());
}

#[test]
fn page_range_accepts_valid_range() {
    let range = PageRange::try_new(1, Some(10)).unwrap();
    assert_eq!(range.start(), 1);
    assert_eq!(range.end(), Some(10));
}

#[test]
fn page_range_accessors() {
    let range = PageRange::single(5);
    assert_eq!(range.start(), 5);
    assert_eq!(range.end(), Some(5));

    let all = PageRange::all();
    assert_eq!(all.start(), 1);
    assert_eq!(all.end(), None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p biscuit-file -- page_range_rejects`
Expected: Compilation error — `try_new` and `start()` don't exist.

- [ ] **Step 3: Refactor `PageRange`**

Replace the `PageRange` definition in `biscuit-file/lib/src/pdf/types.rs` (lines 58-106):

```rust
/// Page range specification.
///
/// Pages are 1-indexed. Use the constructors (`all()`, `single()`, `new()`,
/// `from()`) or `try_new()` for validated construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRange {
    start: usize,
    end: Option<usize>,
}

impl PageRange {
    /// Create a range for all pages.
    #[must_use]
    pub fn all() -> Self {
        Self {
            start: 1,
            end: None,
        }
    }

    /// Create a range for a single page.
    ///
    /// ## Panics
    ///
    /// Panics if `page` is 0.
    #[must_use]
    pub fn single(page: usize) -> Self {
        assert!(page >= 1, "page number must be >= 1 (1-indexed)");
        Self {
            start: page,
            end: Some(page),
        }
    }

    /// Create a range from start to end (inclusive).
    ///
    /// ## Panics
    ///
    /// Panics if `start` is 0 or `end < start`.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start >= 1, "start page must be >= 1 (1-indexed)");
        assert!(end >= start, "end page must be >= start page");
        Self {
            start,
            end: Some(end),
        }
    }

    /// Create a range from a starting page to the end.
    ///
    /// ## Panics
    ///
    /// Panics if `start` is 0.
    #[must_use]
    pub fn from(start: usize) -> Self {
        assert!(start >= 1, "start page must be >= 1 (1-indexed)");
        Self { start, end: None }
    }

    /// Create a validated range, returning `Err` if constraints are violated.
    ///
    /// ## Errors
    ///
    /// Returns an error if `start` is 0 or `end` is less than `start`.
    pub fn try_new(start: usize, end: Option<usize>) -> Result<Self, PdfError> {
        if start == 0 {
            return Err(PdfError::Parse("page range start must be >= 1 (1-indexed)".to_string()));
        }
        if let Some(e) = end {
            if e < start {
                return Err(PdfError::Parse(format!(
                    "page range end ({e}) must be >= start ({start})"
                )));
            }
        }
        Ok(Self { start, end })
    }

    /// Starting page (1-indexed).
    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Ending page (1-indexed, inclusive). `None` means to the end.
    #[must_use]
    pub fn end(&self) -> Option<usize> {
        self.end
    }
}

impl Default for PageRange {
    fn default() -> Self {
        Self::all()
    }
}
```

- [ ] **Step 4: Fix any code that directly accesses `PageRange` fields**

Search the codebase for direct `page_range.start` and `page_range.end` field access and replace with `page_range.start()` and `page_range.end()`. Check `backends.rs` and `types.rs` — the fields are not directly accessed outside of `PageRange`'s own methods and tests based on our reading, but verify.

Update any tests that use `assert_eq!(range.start, ...)` to `assert_eq!(range.start(), ...)`:

In the existing tests section, update:
- `test_page_range_all`: `all.start` → `all.start()`, `all.end` → `all.end()`
- `test_page_range_single`: `single.start` → `single.start()`, `single.end` → `single.end()`
- `test_page_range_new`: `range.start` → `range.start()`, `range.end` → `range.end()`
- `test_page_range_from`: `range.start` → `range.start()`, `range.end` → `range.end()`

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-file/lib/src/pdf/types.rs
git commit -m "feat(biscuit-file): make PageRange fields private with validation"
```

---

### Task 4: Add `#[non_exhaustive]` and builder methods to config structs (Review 1.5)

**Files:**
- Modify: `biscuit-file/lib/src/pdf/types.rs:144-220`

- [ ] **Step 1: Write tests for builder methods**

Add in the `#[cfg(test)]` block in `pdf/types.rs`:

```rust
#[test]
fn pdf_config_builder_methods() {
    let config = PdfConfig::default()
        .with_password("secret")
        .with_page_range(PageRange::new(1, 5))
        .with_max_pages(10)
        .with_normalize_text(false);

    assert_eq!(config.password, Some("secret".to_string()));
    assert_eq!(config.page_range, PageRange::new(1, 5));
    assert_eq!(config.max_pages, Some(10));
    assert!(!config.normalize_text);
}

#[test]
fn markdown_options_builder_methods() {
    let options = MarkdownOptions::default()
        .with_image_mode(ImageMode::Skip)
        .with_include_page_breaks(false);

    assert_eq!(options.image_mode, ImageMode::Skip);
    assert!(!options.include_page_breaks);
}

#[test]
fn text_options_builder_methods() {
    let options = TextOptions::default()
        .with_normalize_text(false)
        .with_include_page_breaks(true);

    assert!(!options.normalize_text);
    assert!(options.include_page_breaks);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p biscuit-file -- builder_methods`
Expected: Compilation error — builder methods don't exist.

- [ ] **Step 3: Add `#[non_exhaustive]` and builder methods**

Add `#[non_exhaustive]` attribute before each struct definition:

Before `pub struct PdfConfig {` (line 146):
```rust
#[non_exhaustive]
```

Before `pub struct MarkdownOptions {` (line 176):
```rust
#[non_exhaustive]
```

Before `pub struct TextOptions {` (line 202):
```rust
#[non_exhaustive]
```

Add builder methods to `PdfConfig` (add a new `impl PdfConfig` block after the `Default` impl):

```rust
impl PdfConfig {
    /// Set the backend preference.
    #[must_use]
    pub fn with_backend(mut self, backend: BackendPreference) -> Self {
        self.backend_preference = backend;
        self
    }

    /// Set the password for encrypted PDFs.
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the page range.
    #[must_use]
    pub fn with_page_range(mut self, range: PageRange) -> Self {
        self.page_range = range;
        self
    }

    /// Set the maximum number of pages to process.
    #[must_use]
    pub fn with_max_pages(mut self, max: usize) -> Self {
        self.max_pages = Some(max);
        self
    }

    /// Set whether to normalize extracted text.
    #[must_use]
    pub fn with_normalize_text(mut self, normalize: bool) -> Self {
        self.normalize_text = normalize;
        self
    }

    /// Set whether to remove headers and footers.
    #[must_use]
    pub fn with_remove_headers_footers(mut self, remove: bool) -> Self {
        self.remove_headers_footers = remove;
        self
    }
}
```

Add builder methods to `MarkdownOptions`:

```rust
impl MarkdownOptions {
    /// Set the assets directory.
    #[must_use]
    pub fn with_assets_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.assets_dir = Some(dir.into());
        self
    }

    /// Set the image handling mode.
    #[must_use]
    pub fn with_image_mode(mut self, mode: ImageMode) -> Self {
        self.image_mode = mode;
        self
    }

    /// Set whether to include page break markers.
    #[must_use]
    pub fn with_include_page_breaks(mut self, include: bool) -> Self {
        self.include_page_breaks = include;
        self
    }

    /// Set the heading detection strategy.
    #[must_use]
    pub fn with_heading_strategy(mut self, strategy: HeadingStrategy) -> Self {
        self.heading_strategy = strategy;
        self
    }

    /// Set the table detection strategy.
    #[must_use]
    pub fn with_table_strategy(mut self, strategy: TableStrategy) -> Self {
        self.table_strategy = strategy;
        self
    }
}
```

Add builder methods to `TextOptions`:

```rust
impl TextOptions {
    /// Set whether to include page break markers.
    #[must_use]
    pub fn with_include_page_breaks(mut self, include: bool) -> Self {
        self.include_page_breaks = include;
        self
    }

    /// Set whether to normalize text.
    #[must_use]
    pub fn with_normalize_text(mut self, normalize: bool) -> Self {
        self.normalize_text = normalize;
        self
    }

    /// Set whether to remove headers and footers.
    #[must_use]
    pub fn with_remove_headers_footers(mut self, remove: bool) -> Self {
        self.remove_headers_footers = remove;
        self
    }
}
```

- [ ] **Step 4: Fix any struct-literal construction broken by `#[non_exhaustive]`**

The `PdfConfig` struct is constructed via literal in `test_pdf_with_custom_config` (line ~831 in types.rs). Update it to use the builder:

```rust
#[test]
fn test_pdf_with_custom_config() {
    let config = PdfConfig::default()
        .with_backend(BackendPreference::Extract)
        .with_password("secret")
        .with_page_range(PageRange::new(1, 5))
        .with_max_pages(10)
        .with_normalize_text(false)
        .with_remove_headers_footers(false);

    let pdf_bytes = b"%PDF-1.4\n%%EOF".to_vec();
    let pdf = Pdf::from_bytes_with_config(pdf_bytes, config).unwrap();

    assert_eq!(pdf.config().backend_preference, BackendPreference::Extract);
    assert_eq!(pdf.config().password, Some("secret".to_string()));
    assert_eq!(pdf.config().page_range, PageRange::new(1, 5));
    assert_eq!(pdf.config().max_pages, Some(10));
    assert!(!pdf.config().normalize_text);
    assert!(!pdf.config().remove_headers_footers);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-file/lib/src/pdf/types.rs
git commit -m "feat(biscuit-file): add #[non_exhaustive] and builder methods to PDF config structs"
```

---

### Task 5: Read only first 512 bytes in `detect_file_type` (Review 1.6)

**Files:**
- Modify: `biscuit-file/lib/src/detect.rs:66-83`

- [ ] **Step 1: Write a test for large file detection**

Add in the `#[cfg(test)]` block in `detect.rs`:

```rust
#[test]
fn test_detect_file_type_from_filesystem() {
    // Create a temp file with .toml extension
    let dir = std::env::temp_dir().join("biscuit-file-test-detect");
    std::fs::create_dir_all(&dir).unwrap();
    let toml_path = dir.join("test.toml");
    std::fs::write(&toml_path, "key = \"value\"").unwrap();
    assert_eq!(detect_file_type(&toml_path).unwrap(), FileType::Toml);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_detect_pdf_by_magic_bytes_wrong_extension() {
    let dir = std::env::temp_dir().join("biscuit-file-test-detect-magic");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("document.txt");
    std::fs::write(&path, b"%PDF-1.7\nsome content here").unwrap();
    assert_eq!(detect_file_type(&path).unwrap(), FileType::Pdf);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_detect_file_type_no_extension() {
    let dir = std::env::temp_dir().join("biscuit-file-test-detect-noext");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config");
    std::fs::write(&path, "key = \"value\"").unwrap();
    assert_eq!(detect_file_type(&path).unwrap(), FileType::Unknown);
    std::fs::remove_dir_all(&dir).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they pass (current impl still works)**

Run: `cargo test -p biscuit-file -- test_detect_file_type_from_filesystem test_detect_pdf_by_magic test_detect_file_type_no_extension`
Expected: All pass.

- [ ] **Step 3: Optimize `detect_file_type` to read only first 512 bytes**

Replace the body of `detect_file_type` in `detect.rs` (lines 67-82):

```rust
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
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-file/lib/src/detect.rs
git commit -m "perf(biscuit-file): read only first 512 bytes for magic byte detection"
```

---

### Task 6: Extract `collect_roots` to deduplicate `build_candidates`/`build_search_roots` (Review 2.1)

**Files:**
- Modify: `biscuit-file/lib/src/file_reference/resolve.rs:124-227`

- [ ] **Step 1: Add tests for the resolve module edge cases (also serves Review 3.2)**

Add in the existing `#[cfg(test)]` block in `resolve.rs`:

```rust
#[test]
fn interpolate_literal_only() {
    let ctx = ResolutionContext {
        cwd: PathBuf::from("/tmp"),
        home_dir: Some(PathBuf::from("/home/test")),
        env: std::collections::HashMap::new(),
    };
    let template = PathTemplate {
        segments: vec![TemplateSegment::Literal("foo/bar.md".to_string())],
    };
    let result = interpolate(&template, &ctx).unwrap();
    assert_eq!(result, "foo/bar.md");
}

#[test]
fn interpolate_with_env_var() {
    let mut env = std::collections::HashMap::new();
    env.insert("PROJECT".to_string(), "myproject".to_string());
    let ctx = ResolutionContext {
        cwd: PathBuf::from("/tmp"),
        home_dir: None,
        env,
    };
    let template = PathTemplate {
        segments: vec![
            TemplateSegment::EnvVar("PROJECT".to_string()),
            TemplateSegment::Literal("/README.md".to_string()),
        ],
    };
    let result = interpolate(&template, &ctx).unwrap();
    assert_eq!(result, "myproject/README.md");
}

#[test]
fn interpolate_missing_env_var() {
    let ctx = ResolutionContext {
        cwd: PathBuf::from("/tmp"),
        home_dir: None,
        env: std::collections::HashMap::new(),
    };
    let template = PathTemplate {
        segments: vec![TemplateSegment::EnvVar("MISSING".to_string())],
    };
    let result = interpolate(&template, &ctx);
    assert!(result.is_err());
}

#[test]
fn normalize_absolute_relative_path() {
    let result = normalize_absolute(Path::new("foo/bar.txt"), Path::new("/home/user"));
    assert_eq!(result, PathBuf::from("/home/user/foo/bar.txt"));
}

#[test]
fn normalize_absolute_already_absolute() {
    let result = normalize_absolute(Path::new("/etc/config.toml"), Path::new("/home/user"));
    assert_eq!(result, PathBuf::from("/etc/config.toml"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p biscuit-file -- resolve::tests`
Expected: Pass (these test existing functions).

- [ ] **Step 3: Extract `collect_roots` helper**

Add above `build_candidates` in `resolve.rs`:

```rust
/// Collect search root directories for a given reference kind.
fn collect_roots(
    kind: &ReferenceKind,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    match kind {
        ReferenceKind::Relative(_) => Ok(vec![ctx.cwd.clone()]),
        ReferenceKind::Absolute(_) => Ok(vec![PathBuf::from("/")]),
        ReferenceKind::Magic(_) => {
            let mut roots = Vec::new();
            roots.extend(magic_paths.prepend.iter().cloned());
            if let Some(git_root) = find_git_root(&ctx.cwd)? {
                roots.push(git_root);
            }
            if let Some(ref home) = ctx.home_dir {
                roots.push(home.clone());
            }
            roots.extend(magic_paths.append.iter().cloned());
            Ok(roots)
        }
        ReferenceKind::Package(_) => {
            let git_root = match find_git_root(&ctx.cwd)? {
                Some(root) => root,
                None => return Ok(vec![]),
            };
            let area = find_package_area(&git_root, &ctx.cwd)?;
            Ok(vec![area.unwrap_or(git_root)])
        }
        ReferenceKind::Vault(_) => {
            let mut roots: Vec<PathBuf> = vault_roots.to_vec();
            if let Some(vault_env) = ctx.env.get("VAULT") {
                roots.extend(std::env::split_paths(vault_env));
            }
            if roots.is_empty() {
                return Err(FileReferenceError::VaultNotConfigured);
            }
            Ok(roots)
        }
    }
}
```

- [ ] **Step 4: Simplify `build_candidates` and `build_search_roots`**

Replace `build_candidates`:

```rust
/// Build candidate file paths for non-recursive resolution.
fn build_candidates(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    // Absolute references don't use roots — they're already full paths
    if let ReferenceKind::Absolute(_) = &parsed.kind {
        let path = PathBuf::from(interpolated);
        if !path.is_absolute() {
            return Err(FileReferenceError::InvalidSyntax(format!(
                "absolute reference resolved to non-absolute path: {interpolated}"
            )));
        }
        return Ok(vec![path]);
    }

    let roots = collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)?;
    Ok(roots.into_iter().map(|r| r.join(interpolated)).collect())
}
```

Replace `build_search_roots`:

```rust
/// Build search roots for recursive resolution.
fn build_search_roots(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-file/lib/src/file_reference/resolve.rs
git commit -m "refactor(biscuit-file): extract collect_roots to deduplicate resolve functions"
```

---

### Task 7: Fix `normalize_text` de-hyphenation bug (Review 2.5)

**Files:**
- Modify: `biscuit-file/lib/src/pdf/backends.rs:33-51`

- [ ] **Step 1: Write a test exposing the bug**

Add in the `#[cfg(test)]` block in `backends.rs`:

```rust
#[test]
fn test_normalize_text_preserves_dash_space_in_prose() {
    // "list items - first" should NOT become "list itemsfirst"
    assert_eq!(
        normalize_text("list items - first"),
        "list items - first"
    );
}

#[test]
fn test_normalize_text_dehyphenates_line_breaks_only() {
    // Hyphen at end of a word followed by newline+space (collapsed to "- ")
    // should de-hyphenate. But a standalone dash between words should not.
    let input = "hyphen-\nated word and some - dash";
    let result = normalize_text(input);
    assert_eq!(result, "hyphenated word and some - dash");
}
```

- [ ] **Step 2: Run tests to verify the bug**

Run: `cargo test -p biscuit-file -- test_normalize_text_preserves_dash_space`
Expected: FAIL — current impl removes "- " globally.

- [ ] **Step 3: Fix de-hyphenation to only act on line-break hyphens**

Replace the `normalize_text` function in `backends.rs`:

```rust
/// Normalize extracted text by collapsing whitespace and de-hyphenating.
///
/// This function:
/// - Collapses runs of whitespace into single spaces
/// - Removes leading/trailing whitespace
/// - De-hyphenates words split across lines (hyphen immediately before
///   a newline, followed by whitespace on the next line)
pub fn normalize_text(text: &str) -> String {
    // First pass: de-hyphenate line breaks.
    // A hyphen at the end of a line (before \n or \r\n) followed by
    // optional whitespace indicates a word split across lines.
    let dehyphenated = {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '-' {
                // Check if next char is a newline (possibly preceded by \r)
                match chars.peek() {
                    Some('\n') => {
                        // Skip the hyphen and the newline
                        chars.next(); // consume \n
                        // Skip any leading whitespace on the next line
                        while let Some(&ws) = chars.peek() {
                            if ws.is_whitespace() && ws != '\n' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    Some('\r') => {
                        chars.next(); // consume \r
                        if chars.peek() == Some(&'\n') {
                            chars.next(); // consume \n
                        }
                        while let Some(&ws) = chars.peek() {
                            if ws.is_whitespace() && ws != '\n' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    };

    // Second pass: collapse whitespace
    let mut result = String::with_capacity(dehyphenated.len());
    let mut last_was_whitespace = false;

    for ch in dehyphenated.chars() {
        if ch.is_whitespace() {
            if !last_was_whitespace && !result.is_empty() {
                result.push(' ');
            }
            last_was_whitespace = true;
        } else {
            result.push(ch);
            last_was_whitespace = false;
        }
    }

    result.trim().to_string()
}
```

- [ ] **Step 4: Update existing tests that relied on old behavior**

The existing `test_normalize_text_dehyphenates` test uses `"hyphen- ated"` (already collapsed whitespace). After the fix, de-hyphenation only triggers on `"-\n"` patterns. Update:

```rust
#[test]
fn test_normalize_text_dehyphenates() {
    // Standard hyphenation pattern: hyphen at end of line
    assert_eq!(normalize_text("hyphen-\nated"), "hyphenated");

    // Multiple hyphenated words across lines
    assert_eq!(
        normalize_text("un-\nfortunate and mis-\nplaced"),
        "unfortunate and misplaced"
    );
}
```

Also update `test_normalize_text_complex`:

```rust
#[test]
fn test_normalize_text_complex() {
    let input = "  This is a test   with   multiple\n\n  \tspaces\n\
                 and some hyphen-\nated words that span-\nned\n\
                 across     lines.  ";
    let expected = "This is a test with multiple spaces and some hyphenated words that spanned across lines.";
    assert_eq!(normalize_text(input), expected);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-file -- normalize_text`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-file/lib/src/pdf/backends.rs
git commit -m "fix(biscuit-file): de-hyphenate only at line breaks, not standalone dashes"
```

---

### Task 8: Fix redundant byte clone in `Pdf::from_bytes_with_config` (Review 3.7)

**Files:**
- Modify: `biscuit-file/lib/src/pdf/types.rs:335-345`

- [ ] **Step 1: Remove the redundant clone**

In `from_bytes_with_config`, change lines 340-344:

Replace:

```rust
        Ok(Self {
            source: PdfSource::Bytes(bytes.clone()),
            config,
            bytes,
        })
```

With:

```rust
        Ok(Self {
            source: PdfSource::Bytes(Vec::new()),
            config,
            bytes,
        })
```

Wait — that changes semantics. The `source` is for provenance tracking ("where did this come from?"), not for holding a copy of the data. A better approach: change `PdfSource::Bytes` to not carry data, just indicate the source was bytes.

Actually, looking at the code more carefully: `PdfSource::Path(PathBuf)` holds the path for provenance, and `PdfSource::Bytes(Vec<u8>)` holds a copy of the bytes for... nothing useful since `Pdf` already has a `bytes` field. The simplest fix: make `PdfSource::Bytes` a unit variant.

Replace `PdfSource` definition:

```rust
/// Source tracking for PDF content.
#[derive(Debug, Clone)]
pub enum PdfSource {
    /// Content loaded from a file path.
    Path(PathBuf),
    /// Content provided as raw bytes (no file path).
    Bytes,
}
```

- [ ] **Step 2: Update all `PdfSource::Bytes(...)` usage**

In `from_bytes_with_config`:
```rust
        Ok(Self {
            source: PdfSource::Bytes,
            config,
            bytes,
        })
```

In the test `test_pdf_source_bytes`, update:
```rust
#[test]
fn test_pdf_source_bytes() {
    let source = PdfSource::Bytes;
    assert!(matches!(source, PdfSource::Bytes));
}
```

In `test_pdf_accessors`, update:
```rust
    assert!(matches!(pdf.source(), PdfSource::Bytes));
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-file`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-file/lib/src/pdf/types.rs
git commit -m "perf(biscuit-file): remove redundant byte clone from PdfSource::Bytes"
```

---

## Phase 2: CLI Refactors (Items 1.1, 1.7, 2.2, 2.4)

These tasks modify the CLI to use the new library types and reduce duplication.

---

### Task 9: Refactor CLI to use `DataFormat` and introduce `Convertible` trait (Reviews 1.1, 1.7, 2.2, 2.4)

**Files:**
- Modify: `biscuit-file/cli/src/main.rs` (major refactor)

This is the largest single task. It:
1. Replaces `InputFormat` with `DataFormat` (Review 2.4)
2. Replaces `OutputFormat` with `DataFormat` (Review 1.2 — CLI side)
3. Validates UTF-8 at the boundary (Review 1.1)
4. Extracts repeated output-format matching into an `emit` helper (Review 2.2)
5. Moves `FrontmatterFormat` to use `DataFormat` (Review 1.7)

- [ ] **Step 1: Replace `InputFormat` and `OutputFormat` with `DataFormat`**

In `biscuit-file/cli/src/main.rs`, replace the `InputFormat` enum (lines 110-125):

```rust
/// Supported input formats for --input-format flag.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum InputFormat {
    /// TOML input
    Toml,
    /// YAML input
    Yaml,
    /// JSON input
    Json,
    /// JSON5 input
    Json5,
    /// Markdown input (extracts frontmatter)
    Markdown,
    /// PDF input
    Pdf,
}

impl From<InputFormat> for FileType {
    fn from(fmt: InputFormat) -> Self {
        match fmt {
            InputFormat::Toml => FileType::Toml,
            InputFormat::Yaml => FileType::Yaml,
            InputFormat::Json => FileType::Json,
            InputFormat::Json5 => FileType::Json5,
            InputFormat::Markdown => FileType::Markdown,
            InputFormat::Pdf => FileType::Pdf,
        }
    }
}
```

Replace the `OutputFormat` enum (lines 72-81) and `Cli::output_format` (lines 83-100):

```rust
/// Resolved output format.
#[derive(Clone, Copy, Debug)]
enum OutputFormat {
    Json,
    Json5,
    Yaml,
    Toml,
    Text,
    Markdown,
}

impl OutputFormat {
    /// Whether this format is for structured data only (not PDF).
    fn is_structured(self) -> bool {
        matches!(self, Self::Json | Self::Json5 | Self::Yaml | Self::Toml)
    }
}
```

And change the `main()` input format match (lines 193-201) to:

```rust
    let input_format = if let Some(fmt) = cli.input_format {
        FileType::from(fmt)
    } else if from_stdin {
```

- [ ] **Step 2: Validate UTF-8 at the boundary for text formats**

In `main()`, after reading content (line 226), add UTF-8 validation for formats that require it before dispatching:

```rust
    let content = if from_stdin {
        read_stdin()?
    } else {
        std::fs::read(cli.file.as_ref().unwrap()).wrap_err("Failed to read input file")?
    };

    // Validate UTF-8 upfront for text-based formats
    let text_content = match input_format {
        FileType::Toml | FileType::Json5 | FileType::Markdown => {
            Some(std::str::from_utf8(&content).wrap_err(format!(
                "{input_format:?} input is not valid UTF-8"
            ))?)
        }
        _ => None,
    };
```

Then update `process_toml` and `process_json5` to accept `&str`:

```rust
    match input_format {
        FileType::Toml => process_toml(text_content.unwrap(), format, compact)?,
        FileType::Yaml => process_yaml(&content, format, compact)?,
        FileType::Json => process_json(&content, format, compact)?,
        FileType::Json5 => process_json5(text_content.unwrap(), format, compact)?,
        FileType::Markdown => process_markdown(text_content.unwrap(), format, compact)?,
        FileType::Pdf => process_pdf(&content, format, compact)?,
        FileType::Unknown => { /* ... same bail ... */ }
    }
```

- [ ] **Step 3: Extract `emit` helper for structured format output**

Add a helper function to eliminate the repeated output-format matching:

```rust
/// Emit a structured value in the requested output format.
fn emit_structured(
    json_value: &serde_json::Value,
    format: Option<OutputFormat>,
    compact: bool,
    raw_source: Option<&str>,
) -> Result<()> {
    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => format_json(json_value, compact)?,
        OutputFormat::Json5 => format_json5(json_value, compact),
        OutputFormat::Yaml => serde_yaml_ng::to_string(json_value)
            .wrap_err("Failed to convert to YAML")?,
        OutputFormat::Toml => {
            let toml_value: toml::Value =
                serde_json::from_value(serde_json::to_value(json_value)?)
                    .wrap_err("Failed to convert to TOML (input may contain types unsupported by TOML)")?;
            toml::to_string_pretty(&toml_value).wrap_err("Failed to serialize TOML")?
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };
    println!("{output}");
    Ok(())
}
```

- [ ] **Step 4: Simplify `process_toml`, `process_yaml`, `process_json`, `process_json5`**

```rust
fn process_toml(input: &str, format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let toml = Toml::from_str(input).wrap_err("Failed to parse TOML")?;

    if matches!(format, Some(OutputFormat::Toml) | None if format.is_none()) && format.is_none() || matches!(format, Some(OutputFormat::Toml)) {
        // For TOML->TOML, just echo the raw input
        if matches!(format, Some(OutputFormat::Toml)) {
            println!("{}", toml.raw());
            return Ok(());
        }
    }

    let json_value = toml.as_json_value().wrap_err("Failed to convert TOML to JSON")?;
    emit_structured(&json_value, format, compact, Some(toml.raw()))
}
```

Actually, let me keep it cleaner. The `process_*` functions should become:

```rust
fn process_toml(input: &str, format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let toml = Toml::from_str(input).wrap_err("Failed to parse TOML")?;
    if matches!(format, Some(OutputFormat::Toml)) {
        println!("{}", toml.raw());
        return Ok(());
    }
    let json_value = toml.as_json_value().wrap_err("Failed to convert to JSON")?;
    emit_structured(&json_value, format, compact)
}

fn process_yaml(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let yaml = Yaml::from_bytes(content).wrap_err("Failed to parse YAML")?;
    if matches!(format, Some(OutputFormat::Yaml)) {
        let output = serde_yaml_ng::to_string(yaml.value())
            .wrap_err("Failed to serialize YAML")?;
        println!("{output}");
        return Ok(());
    }
    let json_value = yaml.as_json().wrap_err("Failed to convert to JSON")?;
    emit_structured(&json_value, format, compact)
}

fn process_json(content: &[u8], format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(content).wrap_err("Failed to parse JSON")?;
    emit_structured(&value, format, compact)
}

fn process_json5(input: &str, format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let json5 = Json5::from_str(input).wrap_err("Failed to parse JSON5")?;
    if matches!(format, Some(OutputFormat::Json5)) {
        let output = if compact { json5.as_json5_compact() } else { json5.as_json5() };
        println!("{output}");
        return Ok(());
    }
    let json_value = json5.as_json_value().clone();
    emit_structured(&json_value, format, compact)
}
```

- [ ] **Step 5: Update `process_markdown` to accept `&str`**

```rust
fn process_markdown(input: &str, format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let (frontmatter, fm_format) = extract_frontmatter(input)?;
    match fm_format {
        FrontmatterFormat::Yaml => process_yaml(frontmatter.as_bytes(), format, compact),
        FrontmatterFormat::Toml => process_toml(frontmatter, format, compact),
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p biscuit-file-cli`
Expected: All pass.

- [ ] **Step 7: Run lint**

Run: `cargo clippy -p biscuit-file-cli -- -D warnings`
Expected: No warnings.

- [ ] **Step 8: Commit**

```bash
git add biscuit-file/cli/src/main.rs
git commit -m "refactor(biscuit-file-cli): unify format enums and extract emit_structured helper"
```

---

## Phase 3: Test Coverage (Items 3.1–3.6, 3.8, 3.9)

These tasks add test files. They are independent of each other and can be done in parallel.

---

### Task 10: Add round-trip conversion tests (Review 3.4)

**Files:**
- Create: `biscuit-file/lib/tests/round_trip.rs`

- [ ] **Step 1: Create round-trip test file**

Create `biscuit-file/lib/tests/round_trip.rs`:

```rust
//! Round-trip conversion tests verifying data fidelity across formats.

use biscuit_file::{Json5, Toml, Yaml};

/// Helper to parse TOML, convert to JSON, convert back to TOML, and compare values.
fn toml_json_round_trip(input: &str) {
    let toml = Toml::from_str(input).expect("parse TOML");
    let json_value = toml.as_json_value().expect("TOML -> JSON");
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    // Parse the JSON back into a serde_json::Value and convert to TOML
    let roundtrip_json: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");
    let roundtrip_toml: toml::Value =
        serde_json::from_value(roundtrip_json).expect("JSON -> TOML value");
    let original_toml = toml.value().clone();

    assert_eq!(
        original_toml, roundtrip_toml,
        "TOML -> JSON -> TOML round trip should preserve values"
    );
}

#[test]
fn toml_json_round_trip_basic() {
    toml_json_round_trip(
        r#"
[package]
name = "test"
version = "1.0.0"
keywords = ["rust", "test"]

[dependencies]
serde = "1.0"
"#,
    );
}

#[test]
fn toml_json_round_trip_nested() {
    toml_json_round_trip(
        r#"
[server]
host = "localhost"
port = 8080

[server.tls]
enabled = true
cert = "/path/to/cert"
"#,
    );
}

#[test]
fn toml_json_round_trip_numbers() {
    toml_json_round_trip(
        r#"
integer = 42
negative = -17
float = 3.14
"#,
    );
}

#[test]
fn yaml_json_round_trip() {
    let input = "name: test\nversion: '1.0'\nitems:\n  - one\n  - two\n";
    let yaml = Yaml::from_str(input).expect("parse YAML");
    let json_value = yaml.as_json().expect("YAML -> JSON");
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    // Parse JSON back to YAML
    let roundtrip_json: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");
    let roundtrip_yaml = serde_yaml_ng::to_value(&roundtrip_json).expect("JSON -> YAML value");
    let original_yaml = yaml.value().clone();

    assert_eq!(
        original_yaml, roundtrip_yaml,
        "YAML -> JSON -> YAML round trip should preserve values"
    );
}

#[test]
fn json_yaml_json_round_trip() {
    let input = r#"{"name": "test", "count": 42, "nested": {"a": 1}}"#;
    let value: serde_json::Value = serde_json::from_str(input).expect("parse JSON");

    let yaml_str = serde_yaml_ng::to_string(&value).expect("JSON -> YAML string");
    let roundtrip: serde_json::Value =
        serde_yaml_ng::from_str(&yaml_str).expect("YAML string -> JSON");

    assert_eq!(value, roundtrip, "JSON -> YAML -> JSON round trip should preserve values");
}

#[test]
fn json5_json_round_trip() {
    let input = r#"{
        // comment
        name: 'test',
        count: 42,
        nested: {a: 1},
    }"#;
    let json5 = Json5::from_str(input).expect("parse JSON5");
    let json_value = json5.as_json_value().clone();
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    // Round-trip through JSON string
    let roundtrip: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");

    assert_eq!(json_value, roundtrip, "JSON5 -> JSON -> JSON round trip should preserve values");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p biscuit-file -- round_trip`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add biscuit-file/lib/tests/round_trip.rs
git commit -m "test(biscuit-file): add round-trip conversion tests for format fidelity"
```

---

### Task 11: Add `extract_frontmatter` unit tests (Review 3.5)

**Files:**
- Modify: `biscuit-file/cli/src/main.rs` — add `#[cfg(test)]` module

- [ ] **Step 1: Add test module to CLI main.rs**

Add at the bottom of `biscuit-file/cli/src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_frontmatter tests ──────────────────────────────────

    #[test]
    fn extract_yaml_frontmatter() {
        let input = "---\ntitle: Hello\n---\nBody text";
        let (fm, fmt) = extract_frontmatter(input).unwrap();
        assert!(matches!(fmt, FrontmatterFormat::Yaml));
        assert_eq!(fm.trim(), "title: Hello");
    }

    #[test]
    fn extract_toml_frontmatter() {
        let input = "+++\ntitle = \"Hello\"\n+++\nBody text";
        let (fm, fmt) = extract_frontmatter(input).unwrap();
        assert!(matches!(fmt, FrontmatterFormat::Toml));
        assert_eq!(fm.trim(), "title = \"Hello\"");
    }

    #[test]
    fn extract_empty_frontmatter() {
        let input = "---\n---\nBody text";
        let (fm, _fmt) = extract_frontmatter(input).unwrap();
        assert!(fm.is_empty() || fm.chars().all(|c| c.is_whitespace()));
    }

    #[test]
    fn extract_frontmatter_with_blank_lines() {
        let input = "---\ntitle: Hello\n\nauthor: World\n---\nBody";
        let (fm, _fmt) = extract_frontmatter(input).unwrap();
        assert!(fm.contains("title: Hello"));
        assert!(fm.contains("author: World"));
    }

    #[test]
    fn extract_frontmatter_triple_dash_in_body() {
        // The closing --- should be the first one after the opening
        let input = "---\ntitle: Hello\n---\nBody with --- dashes";
        let (fm, _fmt) = extract_frontmatter(input).unwrap();
        assert_eq!(fm.trim(), "title: Hello");
    }

    #[test]
    fn extract_frontmatter_leading_whitespace() {
        let input = "  ---\ntitle: Hello\n---\nBody";
        let (fm, _fmt) = extract_frontmatter(input).unwrap();
        assert_eq!(fm.trim(), "title: Hello");
    }

    #[test]
    fn extract_frontmatter_no_frontmatter() {
        let input = "Just a regular markdown document";
        let result = extract_frontmatter(input);
        assert!(result.is_err());
    }

    #[test]
    fn extract_frontmatter_unclosed() {
        let input = "---\ntitle: Hello\nNo closing delimiter";
        let result = extract_frontmatter(input);
        assert!(result.is_err());
    }

    // ── OutputFormat tests ─────────────────────────────────────────

    #[test]
    fn output_format_from_flags() {
        let cli = Cli::parse_from(["bf", "--json", "file.toml"]);
        assert!(matches!(cli.output_format(), Some(OutputFormat::Json)));

        let cli = Cli::parse_from(["bf", "--yaml", "file.toml"]);
        assert!(matches!(cli.output_format(), Some(OutputFormat::Yaml)));

        let cli = Cli::parse_from(["bf", "file.toml"]);
        assert!(cli.output_format().is_none());
    }

    // ── format_json tests ──────────────────────────────────────────

    #[test]
    fn format_json_compact() {
        let value = serde_json::json!({"a": 1, "b": 2});
        let result = format_json(&value, true).unwrap();
        assert!(!result.contains('\n'));
    }

    #[test]
    fn format_json_pretty() {
        let value = serde_json::json!({"a": 1});
        let result = format_json(&value, false).unwrap();
        assert!(result.contains('\n'));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p biscuit-file-cli -- tests`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add biscuit-file/cli/src/main.rs
git commit -m "test(biscuit-file-cli): add unit tests for extract_frontmatter and format helpers"
```

---

### Task 12: Add JSON5 formatter edge case tests (Review 3.6)

**Files:**
- Modify: `biscuit-file/lib/src/json5/format.rs` — extend existing `#[cfg(test)]` module

- [ ] **Step 1: Read current test module**

Read the full test section of `biscuit-file/lib/src/json5/format.rs` to find the right insertion point.

- [ ] **Step 2: Add edge case tests**

Add in the existing `#[cfg(test)]` module in `format.rs`:

```rust
#[test]
fn empty_string_value() {
    let value = serde_json::json!({"key": ""});
    let result = to_json5_pretty(&value);
    assert!(result.contains("''"), "empty string should render as ''");
}

#[test]
fn unicode_beyond_bmp() {
    let value = serde_json::json!({"emoji": "Hello 🌍🎉"});
    let result = to_json5_pretty(&value);
    assert!(result.contains("🌍"));
    assert!(result.contains("🎉"));
}

#[test]
fn string_with_null_byte() {
    let value = serde_json::json!({"data": "before\x00after"});
    let result = to_json5_pretty(&value);
    // Should produce valid output (escaped or raw)
    assert!(result.contains("data"));
}

#[test]
fn very_large_number() {
    let value = serde_json::json!({"big": 1e100});
    let result = to_json5_pretty(&value);
    assert!(result.contains("1e100") || result.contains("1.0e100") || result.contains("1e+100"));
}

#[test]
fn very_small_float() {
    let value = serde_json::json!({"tiny": 1e-300});
    let result = to_json5_pretty(&value);
    assert!(result.contains("tiny"));
}

#[test]
fn deeply_nested_structure() {
    // 50 levels deep
    let mut value = serde_json::json!("leaf");
    for _ in 0..50 {
        value = serde_json::json!({"nested": value});
    }
    // Should not stack overflow
    let result = to_json5_pretty(&value);
    assert!(result.contains("nested"));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-file -- json5::format`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-file/lib/src/json5/format.rs
git commit -m "test(biscuit-file): add JSON5 formatter edge case tests for unicode, nesting, numbers"
```

---

### Task 13: Add `FileType` exhaustiveness test (Review 3.9)

**Files:**
- Modify: `biscuit-file/lib/src/detect.rs`

- [ ] **Step 1: Add exhaustiveness tests**

Add in the existing `#[cfg(test)]` module in `detect.rs`:

```rust
#[test]
fn all_known_variants_have_extension() {
    // If a new FileType variant is added, this test forces updating extension()
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
    // Unknown deliberately has no extension
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
        assert!(
            v.mime_type().is_some(),
            "{v:?} should have a MIME type"
        );
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p biscuit-file -- detect`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add biscuit-file/lib/src/detect.rs
git commit -m "test(biscuit-file): add FileType exhaustiveness tests for extension and mime_type"
```

---

### Task 14: Add CLI `--debug` flag test (Review 3.8)

**Files:**
- Modify: `biscuit-file/cli/tests/cli_tests.rs`

- [ ] **Step 1: Add the test**

Add at the end of `cli_tests.rs`:

```rust
// ── Debug flag ─────────────────────────────────────────────────────

#[test]
fn debug_flag_produces_stderr_output() {
    bf().arg("--debug")
        .arg(fixture("sample.toml"))
        .assert()
        .success()
        .stderr(predicate::str::contains("processing input").or(
            predicate::str::contains("biscuit_file"),
        ));
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p biscuit-file-cli -- debug_flag`
Expected: Pass.

- [ ] **Step 3: Commit**

```bash
git add biscuit-file/cli/tests/cli_tests.rs
git commit -m "test(biscuit-file-cli): add integration test for --debug flag"
```

---

### Task 15: Add `detect_file_type` negative/filesystem tests (Review 3.3)

**Files:**
- Modify: `biscuit-file/lib/src/detect.rs`

These tests were already added in Task 5 (Step 1). If Task 5 is complete, this task is already done.

If not already done, add the tests from Task 5, Step 1 now:

- [ ] **Step 1: Verify the tests exist**

Run: `cargo test -p biscuit-file -- test_detect_file_type_from_filesystem test_detect_pdf_by_magic test_detect_file_type_no_extension`
Expected: All pass (added in Task 5).

- [ ] **Step 2: If missing, add them (see Task 5, Step 1)**

- [ ] **Step 3: Commit (if new changes)**

```bash
git add biscuit-file/lib/src/detect.rs
git commit -m "test(biscuit-file): add filesystem-based detection tests"
```

---

## Phase 4: Final Verification

### Task 16: Full build, test, and lint pass

- [ ] **Step 1: Build everything**

Run: `cargo build -p biscuit-file -p biscuit-file-cli`
Expected: Clean build.

- [ ] **Step 2: Run all tests**

Run: `cargo test -p biscuit-file -p biscuit-file-cli`
Expected: All pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p biscuit-file -p biscuit-file-cli -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Verify no dead code or unused imports**

Run: `cargo build -p biscuit-file -p biscuit-file-cli 2>&1 | grep -i warning`
Expected: No warnings.

---

## Review Items NOT Addressed (by design)

- **Review 2.3** (Unified `ContentSource` enum): The four `XxxSource` enums have legitimately different variants (`TomlSource::Inline` vs `YamlSource::Bytes` vs `PdfSource::Bytes`). Forcing them into one enum adds a variant that's invalid for some formats, which is worse than having separate small enums. After Task 8 (`PdfSource::Bytes` becomes a unit variant), the divergence is even clearer. Skip.

- **Review 3.1** (Unit tests for CLI `process_*` functions): Addressed via Task 11 (unit tests for helpers) and Task 9 (the `emit_structured` refactor makes the process functions so small that the existing integration tests provide adequate coverage). The core conversion logic lives in the library, which has extensive unit tests.

- **Review 3.2** (Tests for `resolve.rs` core logic): Partially addressed in Task 6 (interpolation and normalize tests). Full mocking of `ResolutionContext` with filesystem fixtures would require `tempfile` and significant setup; the existing integration tests via `cli_tests.rs` provide adequate coverage for the resolution paths. The `collect_roots` refactor also makes the code simpler and easier to reason about.
