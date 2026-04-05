# Code Review: Type Safety, DRY, and Test Coverage

**Date**: 2026-04-03
**Scope**: `biscuit-file/lib` and `biscuit-file/cli`
**Focus**: Type safety improvements, DRY violations, test coverage gaps

---

## 1. Type Safety Improvements

### 1.1 (High) CLI `process_*` functions take `&[u8]` but some formats need UTF-8

**Files**: `cli/src/main.rs:314-425`

`process_toml`, `process_json5`, and `process_markdown` immediately convert `&[u8]` to `&str` via `std::str::from_utf8`. This means the UTF-8 validation is buried inside each function. Meanwhile, `process_json` and `process_yaml` accept bytes directly into their parsers.

**Recommendation**: Introduce a typed input wrapper so the UTF-8 check happens once, at the boundary:

```rust
enum ContentSource {
    Text(String),   // validated UTF-8
    Binary(Vec<u8>), // PDF etc.
}
```

Construct it at the call site in `main()` and pass it down. Each `process_*` function then takes `&ContentSource` or `&str` directly, eliminating repeated `from_utf8` calls and making it impossible to forget the validation.

**Impact**: Minor ergonomic cost (one extra enum match at the call site). Gains compile-time guarantee that text formats never see invalid bytes.

### 1.2 (High) `OutputFormat` in the CLI duplicates `FileType` from the library

**Files**: `cli/src/main.rs:73-81`, `lib/src/detect.rs:10-26`

The CLI defines its own `OutputFormat` enum (`Json`, `Json5`, `Yaml`, `Toml`, `Text`, `Markdown`) that partially overlaps with `FileType` but adds `Text` and has different semantics (output vs. input). These two enums drift independently.

**Recommendation**: Either:

- (a) Extend `FileType` with the additional output-only variants (`Text`) and reuse it, or
- (b) Define a single `DataFormat` enum in the library that covers both input and output concerns, with `FileType` and `OutputFormat` as thin wrappers.

Option (b) is preferred because `FileType::Unknown` makes sense for input but not output, and `OutputFormat::Text` makes sense for output but not input. A shared `DataFormat` could be:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFormat {
    Toml, Yaml, Json, Json5, Markdown, Pdf, Text,
}
```

Then `FileType` becomes `Option<DataFormat>` for detection, and `OutputFormat` becomes a newtype or subset.

**Impact**: Zero ergonomic cost for consumers. Adds one shared type to the library's public API.

### 1.3 (Medium) `as_yaml_value()` returns `Result<(), TomlError>` when feature is disabled

**Files**: `lib/src/toml_impl/types.rs:292-294`

When the `yaml` feature is off, `Toml::as_yaml_value()` returns `Result<(), TomlError>` instead of `Result<serde_yaml_ng::Value, TomlError>`. This means the return type changes based on a feature flag, which can silently break downstream code that conditionally compiles.

```rust
#[cfg(not(feature = "yaml"))]
pub fn as_yaml_value(&self) -> Result<(), TomlError> {  // <- returns ()
    Err(TomlError::YamlFeatureDisabled)
}
```

**Recommendation**: Use the same return type in both configurations. When the feature is off, the function should still return `Result<serde_yaml_ng::Value, TomlError>` -- but since `serde_yaml_ng` is unavailable, consider either:

- Using a sealed `Unavailable` type, or
- Simply not providing the function at all when the feature is off (already done for other types like `Json5`).

The current approach of returning `()` is the worst of both worlds: it compiles but returns a nonsensical type.

### 1.4 (Medium) `PageRange` fields are public with no validation

**Files**: `lib/src/pdf/types.rs:59-100`

`PageRange` has public `start` and `end` fields but `start` is documented as "1-indexed" with no enforcement. A user can construct `PageRange { start: 0, end: Some(5) }` which would be semantically wrong.

**Recommendation**: Make fields private and use the existing constructors (`all()`, `single()`, `new()`, `from()`). Add `TryFrom` for `(usize, usize)` if tuple construction is desired. If keeping fields public is important for ergonomics, add a `fn is_valid(&self) -> bool` and document the 1-indexed contract more prominently.

### 1.5 (Medium) `PdfConfig`, `MarkdownOptions`, `TextOptions` have public fields with no builder

**Files**: `lib/src/pdf/types.rs:146-220`

These config structs have all-public fields and no builder pattern. This works but means:

- There's no single place to enforce invariants (e.g., `max_pages` must be > 0)
- Adding a new field is a breaking change for anyone using struct literal syntax

**Recommendation**: Use `#[non_exhaustive]` on these structs and add builder methods:

```rust
impl PdfConfig {
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }
}
```

This is low-cost and preserves the ability to do `PdfConfig { ..Default::default(), password: Some("x".into()) }`.

### 1.6 (Low) `detect_file_type` reads the entire file into memory

**Files**: `lib/src/detect.rs:66-83`

`detect_file_type` calls `std::fs::read(path)?` which reads the entire file just to check the first few bytes. For large files (especially PDFs), this is wasteful.

**Recommendation**: Read only the first N bytes (e.g., 512) for magic detection, then fall back to extension:

```rust
let mut buf = [0u8; 512];
let n = File::open(path)?.read(&mut buf)?;
let from_bytes = detect_file_type_from_bytes(&buf[..n]);
```

This is both a type-safety and performance issue -- the function signature implies lightweight detection but the implementation is heavyweight.

### 1.7 (Low) `FrontmatterFormat` in the CLI could be a library type

**Files**: `cli/src/main.rs:255-261`

`FrontmatterFormat` is a private enum in the CLI but could be useful for library consumers who want to know what kind of frontmatter was detected. If the library ever provides a `Frontmatter` type, this should move there.

---

## 2. DRY Violations

### 2.1 (High) `build_candidates` and `build_search_roots` are near-identical

**Files**: `lib/src/file_reference/resolve.rs:124-227`

These two functions contain almost identical `match` arms for `ReferenceKind`. The only difference is that `build_candidates` joins the interpolated path to each root (producing file paths), while `build_search_roots` returns bare root directories.

```rust
// build_candidates - Magic arm
ReferenceKind::Magic(_) => {
    let mut roots = Vec::new();
    roots.extend(magic_paths.prepend.iter().cloned());
    if let Some(git_root) = find_git_root(&ctx.cwd)? { roots.push(git_root); }
    if let Some(ref home) = ctx.home_dir { roots.push(home.clone()); }
    roots.extend(magic_paths.append.iter().cloned());
    Ok(roots.into_iter().map(|r| r.join(interpolated)).collect())
}

// build_search_roots - Magic arm
ReferenceKind::Magic(_) => {
    let mut roots = Vec::new();
    roots.extend(magic_paths.prepend.iter().cloned());
    if let Some(git_root) = find_git_root(&ctx.cwd)? { roots.push(git_root); }
    if let Some(ref home) = ctx.home_dir { roots.push(home.clone()); }
    roots.extend(magic_paths.append.iter().cloned());
    Ok(roots)
}
```

**Recommendation**: Extract a single `fn collect_roots(kind, magic_paths, vault_roots, ctx) -> Result<Vec<PathBuf>>` and let `build_candidates` call `collect_roots(...).map(|roots| roots.iter().map(|r| r.join(interpolated)).collect())`.

### 2.2 (High) `process_toml`, `process_yaml`, `process_json`, `process_json5` share a common pattern

**Files**: `cli/src/main.rs:314-425`

All four functions follow the same structure:

1. Parse input bytes into a structured type
2. Match on `OutputFormat`
3. Convert to the requested format
4. Print and return

The output format matching is nearly identical across all of them, with each branch calling the appropriate conversion method. The `Text | Markdown` error arm is repeated verbatim four times.

**Recommendation**: Introduce a trait or a conversion helper:

```rust
trait Convertible {
    fn to_json_value(&self) -> Result<serde_json::Value, color_eyre::eyre::Report>;
    fn to_yaml_string(&self) -> Result<String, color_eyre::eyre::Report>;
    fn to_toml_string(&self) -> Result<String, color_eyre::eyre::Report>;
    fn to_raw(&self) -> String;
}
```

Then the output matching can be a single function:

```rust
fn emit(doc: &dyn Convertible, format: Option<OutputFormat>, compact: bool) -> Result<()> {
    let output = match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => format_json(&doc.to_json_value()?, compact)?,
        OutputFormat::Json5 => format_json5(&doc.to_json_value()?, compact),
        OutputFormat::Yaml => doc.to_yaml_string()?,
        OutputFormat::Toml => doc.to_toml_string()?,
        OutputFormat::Text | OutputFormat::Markdown => {
            bail!("--text and --md are only supported for PDF files");
        }
    };
    println!("{output}");
    Ok(())
}
```

This eliminates 4 copies of the error message and the repeated format matching.

### 2.3 (Medium) `Source` enum variants are repeated per format

**Files**: `lib/src/toml_impl/types.rs:10-16`, `lib/src/yaml/types.rs:8-16`, `lib/src/json5/types.rs:8-14`, `lib/src/pdf/types.rs:8-14`

Each format module defines its own `XxxSource` enum with near-identical variants:

```rust
pub enum TomlSource { Path(PathBuf), Inline }
pub enum YamlSource { Path(PathBuf), Text(String), Bytes(Vec<u8>) }
pub enum Json5Source { Path(PathBuf), Inline }
pub enum PdfSource { Path(PathBuf), Bytes(Vec<u8>) }
```

**Recommendation**: Define a single `ContentSource` enum in the library root:

```rust
pub enum ContentSource {
    Path(PathBuf),
    Text(String),
    Bytes(Vec<u8>),
    Inline,
}
```

Each format type can then hold a `ContentSource` instead of its own enum. This also enables writing generic utilities that work with any format's source tracking.

### 2.4 (Medium) `InputFormat` in CLI maps 1:1 to `FileType`

**Files**: `cli/src/main.rs:111-125`, `lib/src/detect.rs:10-26`

`InputFormat` is a clap `ValueEnum` that maps directly to `FileType` variants. The match in `main()` (lines 194-201) is a manual 1:1 mapping that must be kept in sync.

```rust
InputFormat::Toml => FileType::Toml,
InputFormat::Yaml => FileType::Yaml,
InputFormat::Json => FileType::Json,
InputFormat::Json5 => FileType::Json5,
InputFormat::Markdown => FileType::Markdown,
InputFormat::Pdf => FileType::Pdf,
```

**Recommendation**: Derive `ValueEnum` on `FileType` directly (or a new `DataFormat` enum as suggested in 1.2), eliminating the intermediate type entirely. If `FileType` can't derive `ValueEnum` due to library constraints, add a `From<InputFormat>` impl so the match can be replaced with `input_format.into()`.

### 2.5 (Low) `normalize_text` in `backends.rs` has an edge case bug that also represents DRY

**Files**: `lib/src/pdf/backends.rs:49-50`

The de-hyphenation is implemented as `result.replace("- ", "")`, which is overly aggressive. It will incorrectly remove legitimate hyphen+space patterns like `list items - first` (becomes `list itemsfirst`). A proper implementation would only remove hyphens that appear at the end of a line (i.e., followed by `\n` and then whitespace).

This is both a correctness issue and a DRY issue -- a text normalization module could be shared between PDF and future format backends.

---

## 3. Test Coverage Gaps

### 3.1 (High) No unit tests for CLI `process_*` functions

**Files**: `cli/src/main.rs`

The CLI integration tests (`cli/tests/cli_tests.rs`) only test the binary end-to-end. The `process_toml`, `process_yaml`, `process_json`, `process_json5`, `process_markdown`, and `process_pdf` functions are private and have zero direct unit test coverage. If any of these functions had a regression in error handling or output formatting, it would only be caught by the integration tests.

**Recommendation**: Either:

- (a) Make these functions `pub(crate)` and add unit tests, or
- (b) Extract the core logic into a testable module and test that.

Option (b) is better for long-term maintenance. The functions are already small and pure (they don't access global state), so testing is straightforward.

### 3.2 (High) No tests for `resolve.rs` core logic

**Files**: `lib/src/file_reference/resolve.rs`

The `resolve` function and its helpers (`build_candidates`, `build_search_roots`, `resolve_recursive`) have only 3 tests (`diff_paths_*` and `normalize_dotdot`). There are no tests for:

- `resolve_direct` with actual `ResolutionContext` mocking
- `resolve_recursive` matching logic
- `build_candidates` for each `ReferenceKind`
- `interpolate` function
- The `normalize_absolute` function

**Recommendation**: Add tests using a constructed `ResolutionContext` (make fields public or add a `fn new_test(cwd, home, env) -> Self` method behind a `#[cfg(test)]` feature). Test at least:

- Interpolation of env vars present / missing
- Each `ReferenceKind` producing correct candidates
- Recursive search with multiple matches returning lexicographically first

### 3.3 (High) No negative tests for `detect_file_type` (filesystem-based)

**Files**: `lib/src/detect.rs:66-83`

`detect_file_type` reads from the filesystem but there are no tests exercising the filesystem path -- only the extension-based and magic-byte helpers are tested. Missing:

- Test with a real file that has wrong extension but valid magic bytes (e.g., a PDF named `file.txt`)
- Test with an unreadable file (permission denied)
- Test with a file that has no extension

### 3.4 (Medium) No round-trip tests for TOML/JSON/YAML conversion fidelity

**Files**: `lib/src/toml_impl/types.rs`, `lib/src/yaml/types.rs`

There are tests that TOML converts to JSON correctly and YAML converts to JSON correctly, but no tests verifying:

- TOML -> JSON -> round-trip back to TOML preserves values
- YAML -> JSON -> round-trip back to YAML preserves values
- JSON -> YAML -> JSON round-trip

**Recommendation**: Add at least one round-trip test per format pair. These catch subtle bugs like float precision loss, datetime serialization drift, and key ordering changes.

### 3.5 (Medium) `extract_frontmatter` has no unit tests

**Files**: `cli/src/main.rs:263-293`

The frontmatter extraction logic has edge cases (whitespace before delimiters, empty frontmatter, nested delimiters in content) but is only tested via integration tests. It's not easily testable because it's a private function.

**Recommendation**: Extract to a `fn extract_frontmatter(input: &str) -> Result<(&str, FrontmatterFormat)>` in the library and test directly. Test cases should include:

- Empty frontmatter body (`---\n---`)
- Frontmatter with blank lines
- Content that contains `---` after the frontmatter
- Leading whitespace before the opening delimiter
- Only opening delimiter (unclosed)

### 3.6 (Medium) No tests for `json5::format` edge cases

**Files**: `lib/src/json5/format.rs`

The JSON5 formatter tests cover basic cases but miss:

- Unicode characters beyond BMP (surrogate pairs)
- Strings containing `\0` in the middle
- Deeply nested structures (stack overflow risk)
- Numbers like `1e100` and very small floats
- Empty string value

### 3.7 (Medium) `Pdf::from_bytes_with_config` clones bytes unnecessarily

**Files**: `lib/src/pdf/types.rs:335-345`

```rust
Ok(Self {
    source: PdfSource::Bytes(bytes.clone()),  // clones
    config,
    bytes,                                    // also owns the bytes
})
```

The bytes are stored in both `source` and the `bytes` field. This is not a test gap per se but it means the `PdfSource::Bytes` variant always holds a redundant copy of the data. Either the source should be `PdfSource::Bytes` (removing the `bytes` field) or the source should just track the provenance without duplicating data.

### 3.8 (Low) No CLI test for `--debug` flag

**Files**: `cli/tests/cli_tests.rs`

The `--debug` flag affects tracing output but there's no test verifying it works. A simple test that runs `bf --debug <file>` and checks stderr for debug-level output would catch regressions.

### 3.9 (Low) No test for `FileType::extension()` and `mime_type()` completeness

**Files**: `lib/src/detect.rs`

If a new `FileType` variant is added, there's no compile-time or test-time check ensuring `extension()` and `mime_type()` handle it. Consider adding a property test or exhaustive match test.

---

## Summary Priority Matrix

| ID | Category | Priority | Effort | Impact |
|----|----------|----------|--------|--------|
| 1.2 | Type Safety | High | Medium | Eliminates enum drift between CLI and lib |
| 2.1 | DRY | High | Low | Removes ~100 lines of near-duplicate code |
| 2.2 | DRY | High | Medium | Removes ~80 lines of repeated output matching |
| 2.3 | DRY | Medium | Low | Unifies 4 near-identical enums |
| 3.1 | Testing | High | Medium | Core CLI logic has zero direct tests |
| 3.2 | Testing | High | Medium | File resolution core logic barely tested |
| 1.1 | Type Safety | High | Low | UTF-8 validation in one place |
| 1.3 | Type Safety | Medium | Low | Prevents silent type change across features |
| 1.4 | Type Safety | Medium | Low | Prevents invalid `PageRange` construction |
| 2.4 | DRY | Medium | Low | Eliminates manual 1:1 mapping |
| 3.4 | Testing | Medium | Low | Catches subtle conversion bugs |
| 3.5 | Testing | Medium | Low | Frontmatter edge cases untested |
| 2.5 | DRY | Low | Low | Shared text normalization + correctness fix |
| 1.6 | Type Safety | Low | Low | Reads whole file for magic detection |
| 3.7 | Perf/Correctness | Medium | Low | Redundant byte clone in PdfSource |
