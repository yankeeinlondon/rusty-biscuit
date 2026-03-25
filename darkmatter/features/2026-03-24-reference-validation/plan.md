# Reference Validation — Implementation Plan

> **Source documents:**
> - [spec.md](./spec.md) — feature requirements
> - [tech-design.md](./tech-design.md) — implementation-ready technical design
>
> **Branch:** `feat/darkmatter-graph-features`
> **Date:** 2026-03-25

---

## Codebase Baseline

The plan is grounded in the following verified codebase state:

| Aspect | Current State |
|--------|--------------|
| **Markdown struct** | `darkmatter/lib/src/markdown/mod.rs` — ~30 public methods including `links()`, `image_references()`, `inline_html_links()`, `inline_html_image_references()`, `compose()`, `compose_with()`, `compose_mut()` |
| **Link / ImageRef types** | `darkmatter/lib/src/render/link.rs` (`Link`, `LinkType`) and `render/image_ref.rs` (`ImageRef`) |
| **Inline HTML extraction** | `markdown/inline_html.rs` — MDAST-based extraction for `<a>` and `<img>` only |
| **Compose pipeline** | `markdown/compose/` — 10 operations across 3 phases (InlinePre, Transclusion, InlinePost) |
| **ComposeSource** | `Unknown`, `File(PathBuf)`, `Url(Url)` |
| **TransclusionRuntime** | `compose/transclusion/types.rs` — per-branch cycle detection, depth limiting, child cloning |
| **Directive parser** | `compose/transclusion/parser.rs` — `parse_directives()` for `::file`, `::code`, `::url`; `parse_frontmatter_refs()` for prologue/epilogue |
| **Path resolution** | `compose/transclusion/resolver.rs` — uses `biscuit_file::FileReference` for `@`, `!`, `vault:`, `%`, `{{ENV}}` |
| **Error types** | `MarkdownError` in `markdown/types.rs` — 12 variants including `Transclusion(TransclusionError)` |
| **Existing deps** | `dashmap` 6.1, `reqwest` 0.13, `url` 2.5, `indexmap` 2, `pulldown-cmark` 0.13, `biscuit-file`, `rayon` 1.10, `thiserror` 2.0 |
| **Missing deps** | `cssparser` (needed Phase 2), `wiremock` (test dep, already in workspace) |
| **Reference module** | Does not exist yet |

---

## Architecture Decisions

These decisions are locked for this plan. They come directly from the tech design and are validated against the codebase.

1. **Existing local APIs are unchanged.** `links()`, `image_references()`, `inline_html_links()`, `inline_html_image_references()` keep their current signatures, return types, and local-only semantics.

2. **New `markdown/reference/` module.** All reference analysis lives here. The `Markdown` struct gains new methods via an `impl` block in `reference/mod.rs`.

3. **Graph model, not string reparsing.** Reference graph analysis traverses documents as nodes with insertion edges. It does not call full `compose()` and reparse the flattened text.

4. **Reuse compose internals without coupling.** Reference graph analysis reuses `TransclusionRuntime` (cycle/depth detection), `resolve_target()` (path resolution), `parse_directives()` / `parse_frontmatter_refs()` (directive parsing), and InlinePre operations (TextReplacement, PageBlocks, Interpolation, ShellExpansion) — but does not run InlinePost or produce a `ComposeReport`.

5. **Separate validation report.** `ReferenceValidationReport` is distinct from `ComposeReport`.

6. **`MarkdownError::Reference(ReferenceError)`** added as a new variant for cleaner public diagnostics.

7. **Rust naming conventions.** `transclusions()` not `get_transclusions()`, `meta_tags()` not `get_meta_tags()`.

---

## Phase 1 — Core Graph Infrastructure

> Local transclusion queries, unified reference types, local extraction with provenance, reference graph traversal, composed flattening, local file validation.

### 1.1 Create `markdown/reference/types.rs` — Core Reference Types

**Create:** `darkmatter/lib/src/markdown/reference/types.rs`

Define the foundational type system for reference analysis:

```rust
// --- Reference classification ---

pub enum ReferenceKind {
    Hyperlink,
    Image,
    Transclusion,
    CssImport,
    InlineCss,
    ScriptImport,
    InlineScript,
    FontImport,
    MetaTag,
}

pub enum ReferenceSyntax {
    MarkdownLink,          // [text](url)
    HtmlAnchor,            // <a href="...">
    MarkdownImage,         // ![alt](src)
    HtmlImage,             // <img src="...">
    DirectiveFile,         // ::file path
    DirectiveUrl,          // ::url https://...
    DirectiveCode,         // ::code path
    DirectiveTocLinking,   // ::toc-linking path
    FrontmatterPrologue,   // prologue: path
    FrontmatterEpilogue,   // epilogue: path
    HtmlLinkTag,           // <link href="...">
    HtmlScriptTag,         // <script src="...">
    HtmlStyleTag,          // <style>...</style>
    CssAtImport,           // @import url(...)
    CssFontFaceSrc,        // @font-face { src: url(...) }
    HtmlMetaTag,           // <meta ...>
}

pub enum ReferenceTarget {
    LocalPath { raw: String },
    RemoteUrl { raw: String },
    Fragment { raw: String },
    DataUri { raw: String },
    OtherScheme { raw: String, scheme: String },
    Inline,  // inline <style>/<script> blocks
}

// --- Provenance ---

pub struct ReferenceOrigin {
    pub source: ComposeSource,
    pub line: usize,
    pub span: std::ops::Range<usize>,
    pub syntax: ReferenceSyntax,
}

// --- Record ---

pub struct ReferenceRecord {
    pub id: String,  // stable unique id (e.g., "{source_hash}:{line}:{col}")
    pub kind: ReferenceKind,
    pub target: ReferenceTarget,
    pub origin: ReferenceOrigin,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

pub struct ReferenceSet {
    pub records: Vec<ReferenceRecord>,
}

// --- Graph ---

pub struct ReferenceInsertion {
    pub child_node_id: String,
    pub directive_line: usize,
    pub insertion_order: usize,
}

pub struct ReferenceGraphNode {
    pub node_id: String,
    pub source: ComposeSource,
    pub local_references: ReferenceSet,
    pub child_insertions: Vec<ReferenceInsertion>,
}

pub struct ReferenceGraph {
    pub root: ReferenceGraphNode,
    pub nodes: Vec<ReferenceGraphNode>,
}

// --- Options ---

pub struct ReferenceGraphOptions {
    pub compose: ComposeOptions,
    pub include_generated_toc_links: bool,
    pub follow_remote_transclusions: bool,
}

// --- Transclusion ref (local query) ---

pub struct TransclusionRef {
    pub kind: TransclusionRefKind,
    pub raw_target: String,
    pub resolved_target: Option<String>,
    pub options: TransclusionRefOptions,
    pub origin: ReferenceOrigin,
}

pub enum TransclusionRefKind {
    File,
    Code,
    Url,
    TocLinking,
    Prologue,
    Epilogue,
}

pub struct TransclusionRefOptions {
    pub when_expr: Option<String>,
    pub replace: Option<String>,
    pub quotation: Option<String>,
    pub disclosure: Option<String>,
    pub exclude: Vec<String>,
}
```

**Convenience wrappers** (thin views over `ReferenceRecord`):

```rust
pub struct LinkReference {
    pub record: ReferenceRecord,
    pub display: String,
    pub title: Option<String>,
}

pub struct ImageReference {
    pub record: ReferenceRecord,
    pub alt: String,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
```

**Design notes:**
- `ReferenceRecord.id` uses format `{source_id_hash:016x}:{line}:{span_start}` for stable identity
- `ReferenceOrigin.source` reuses existing `ComposeSource` enum unchanged
- `attributes` captures extra data-* attributes, CSS classes, etc. without type explosion
- All types derive `Debug, Clone`; enums also derive `PartialEq, Eq`
- `ReferenceSet` implements `IntoIterator`, filter-by-kind helpers, and `len()`/`is_empty()`

**Tests:** Unit tests for `ReferenceTarget` classification (URL vs path vs fragment detection), `ReferenceRecord` construction, `ReferenceSet` filtering.

---

### 1.2 Create `markdown/reference/errors.rs` — Error Types

**Create:** `darkmatter/lib/src/markdown/reference/errors.rs`

```rust
#[derive(Debug, Error)]
pub enum ReferenceError {
    #[error("Failed to parse directive at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    #[error("Missing source context for reference '{reference}' at line {line}")]
    MissingSourceContext { reference: String, line: usize },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error(transparent)]
    Compose(#[from] MarkdownError),

    #[error(transparent)]
    FileReference(#[from] biscuit_file::FileReferenceError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}
```

**Modify:** `darkmatter/lib/src/markdown/types.rs`

Add a new `MarkdownError` variant:

```rust
/// Reference analysis error.
#[error("Reference error: {0}")]
Reference(#[from] crate::markdown::reference::ReferenceError),
```

**Tests:** Verify `ReferenceError` converts into `MarkdownError::Reference` via `From`.

---

### 1.3 Create `markdown/reference/mod.rs` — Module Entry Point

**Create:** `darkmatter/lib/src/markdown/reference/mod.rs`

```rust
//! Reference analysis subsystem for graph-aware reference discovery and validation.

pub mod types;
pub mod errors;
mod local;
mod html;
mod graph;
mod validate;

// Phase 2 modules (added later):
// mod css;
// mod meta;

pub use errors::ReferenceError;
pub use types::*;
```

Add `impl Markdown` block with Phase 1 public methods:

```rust
impl Markdown {
    pub fn has_transclusions(&self) -> bool { ... }
    pub fn transclusions(&self) -> MarkdownResult<Vec<TransclusionRef>> { ... }
    pub fn transclusion_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceGraph> { ... }
    pub fn reference_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceGraph> { ... }
    pub fn composed_references(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceSet> { ... }
    pub fn composed_links(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<LinkReference>> { ... }
    pub fn composed_image_references(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<ImageReference>> { ... }
    pub fn validate_references(&self, options: ReferenceValidationOptions) -> MarkdownResult<ReferenceValidationReport> { ... }
}
```

**Modify:** `darkmatter/lib/src/markdown/mod.rs`

Add `pub mod reference;` and re-export key types:

```rust
pub use reference::{
    ReferenceError, ReferenceGraph, ReferenceGraphOptions, ReferenceKind,
    ReferenceRecord, ReferenceSet, TransclusionRef,
};
```

**Tests:** Smoke test that `Markdown::from("# Hello").has_transclusions()` returns `false`.

---

### 1.4 Implement `has_transclusions()` and `transclusions()`

**File:** `darkmatter/lib/src/markdown/reference/mod.rs`

**`has_transclusions()`** — cheap local query:

1. Call existing `transclusion::parser::parse_directives(self.content())` — if any directives found, return `true`
2. Call existing `transclusion::parser::parse_frontmatter_refs(self.frontmatter().as_map())` — if prologue or epilogue non-empty, return `true`
3. Otherwise return `false`

No new parsing logic needed. Reuses existing parsers directly.

**`transclusions()`** — local transclusion references with provenance:

1. Parse block directives via `parse_directives(self.content())`
2. Convert each `BlockDirective` → `TransclusionRef` with:
   - `kind` mapped from `DirectiveKind` → `TransclusionRefKind`
   - `raw_target` from directive
   - `resolved_target`: attempt resolution via `resolver::resolve_target()` if source context available, `None` if `ComposeSource::Unknown`
   - `options` mapped from `BlockOptions`
   - `origin` with `ComposeSource` from `self` (needs a way to access source — see note below)
3. Parse `::toc-linking` directives via `toc_linking::parser::parse_toc_directives(self.content())`
4. Convert each → `TransclusionRef` with kind `TocLinking`
5. Parse frontmatter refs via `parse_frontmatter_refs()`
6. Convert prologue entries → `TransclusionRef` with kind `Prologue`
7. Convert epilogue entries → `TransclusionRef` with kind `Epilogue`
8. Return all in source order (block directives by line, then frontmatter refs)

**Source context consideration:** The `Markdown` struct does not currently store its `ComposeSource`. For `transclusions()` to resolve targets, the caller must provide source context. Two options:

- **Option A:** Add an optional `source: Option<ComposeSource>` field to `Markdown` (set when loaded from file via `TryFrom<&Path>`)
- **Option B:** `transclusions()` takes a `source: &ComposeSource` parameter

**Recommendation:** Option A — add `source: Option<ComposeSource>` to `Markdown`. Set it in the `TryFrom<&Path>` impl (already converts path to `Markdown`). This is a non-breaking additive change. The field defaults to `None` for string-constructed instances. Methods that need source context return `MissingSourceContext` when `None`.

**Changes to `Markdown` struct:**

```rust
// In markdown/mod.rs
pub struct Markdown {
    frontmatter: Frontmatter,
    content: String,
    source: Option<ComposeSource>,  // NEW
}

// New accessor
pub fn source(&self) -> &Option<ComposeSource> { &self.source }
pub fn with_source(mut self, source: ComposeSource) -> Self {
    self.source = Some(source);
    self
}
```

**Tests:**
- Document with `::file ./child.md` → `has_transclusions() == true`, `transclusions()` returns 1 entry
- Document with `prologue: header.md` → `has_transclusions() == true`, `transclusions()` returns 1 Prologue entry
- Document with `::code main.rs` → `has_transclusions() == true`, returns Code kind
- Document with `::toc-linking ./doc.md` → returns TocLinking kind
- Document with no directives → `has_transclusions() == false`, empty vec
- Directives inside fenced code blocks → excluded

---

### 1.5 Create `markdown/reference/local.rs` — Local Extraction with Provenance

**Create:** `darkmatter/lib/src/markdown/reference/local.rs`

This module extracts `ReferenceRecord` values from a single document's content with full provenance (source, line, span, syntax).

**Functions:**

```rust
/// Extract Markdown-native links as ReferenceRecords with provenance.
pub(crate) fn extract_markdown_links(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord>

/// Extract Markdown-native images as ReferenceRecords with provenance.
pub(crate) fn extract_markdown_images(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord>
```

**Implementation strategy:**

Reuse the same `pulldown_cmark` event stream that `Markdown::links()` and `Markdown::image_references()` currently use, but:
1. Capture `Range<usize>` spans from the parser events
2. Compute line numbers from byte offsets
3. Classify targets as `ReferenceTarget::LocalPath`, `RemoteUrl`, `Fragment`, `DataUri`, or `OtherScheme`
4. Build `ReferenceRecord` with `ReferenceOrigin`

**Do NOT modify** the existing `links()` and `image_references()` implementations. `local.rs` is a parallel extraction path that produces the richer `ReferenceRecord` type.

**Target classification logic:**

```rust
fn classify_target(raw: &str) -> ReferenceTarget {
    if raw.starts_with('#') {
        ReferenceTarget::Fragment { raw: raw.into() }
    } else if raw.starts_with("data:") {
        ReferenceTarget::DataUri { raw: raw.into() }
    } else if raw.starts_with("http://") || raw.starts_with("https://") {
        ReferenceTarget::RemoteUrl { raw: raw.into() }
    } else if raw.starts_with("mailto:") || raw.starts_with("tel:") || raw.contains("://") {
        let scheme = raw.split("://").next().unwrap_or(raw.split(':').next().unwrap_or(""));
        ReferenceTarget::OtherScheme { raw: raw.into(), scheme: scheme.into() }
    } else {
        ReferenceTarget::LocalPath { raw: raw.into() }
    }
}
```

**Tests:**
- Markdown link `[text](./file.md)` → `Hyperlink`, `MarkdownLink`, `LocalPath`
- Markdown link `[text](https://example.com)` → `Hyperlink`, `MarkdownLink`, `RemoteUrl`
- Markdown link `[text](#section)` → `Hyperlink`, `MarkdownLink`, `Fragment`
- Markdown image `![alt](./img.png)` → `Image`, `MarkdownImage`, `LocalPath`
- Data URI image → `DataUri`
- `mailto:` link → `OtherScheme`
- Provenance: line number and span match source positions

---

### 1.6 Create `markdown/reference/html.rs` — HTML Tag Extraction with Provenance

**Create:** `darkmatter/lib/src/markdown/reference/html.rs`

Generalizes the current `inline_html.rs` approach to extract provenance-rich records from all relevant HTML tags.

**Phase 1 scope:** `<a>` and `<img>` tags only (matching current `inline_html.rs` capability but producing `ReferenceRecord` output).

**Phase 2 additions:** `<style>`, `<script>`, `<link>`, `<meta>` (added in tasks 2.1–2.3).

**Functions:**

```rust
/// Extract HTML <a> tags as ReferenceRecords with provenance.
pub(crate) fn extract_html_links(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord>

/// Extract HTML <img> tags as ReferenceRecords with provenance.
pub(crate) fn extract_html_images(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord>
```

**Implementation strategy:**

1. Reuse the MDAST-based HTML fragment detection from `inline_html.rs` (find HTML nodes via MDAST positions)
2. Parse `<a>` and `<img>` tags using the same attribute extraction logic
3. Add `ReferenceOrigin` with computed line numbers and byte spans
4. Exclude fragments inside fenced code blocks (same gate as current impl)

**Important:** This does NOT replace `inline_html.rs`. The existing `extract_inline_html_links()` and `extract_inline_html_images()` functions remain untouched. `html.rs` is a parallel extraction path for the reference subsystem.

**Tests:**
- HTML `<a href="./doc.md">link</a>` → `Hyperlink`, `HtmlAnchor`, `LocalPath`
- HTML `<img src="photo.jpg">` → `Image`, `HtmlImage`, `LocalPath`
- HTML inside fenced code → excluded
- Nested/malformed tags → handled gracefully

---

### 1.7 Create `markdown/reference/graph.rs` — Graph Traversal

**Create:** `darkmatter/lib/src/markdown/reference/graph.rs`

This is the core of the feature: recursive document graph analysis with provenance preservation.

**Public functions:**

```rust
/// Build a transclusion-only graph (no link/image extraction at leaf nodes).
pub(crate) fn build_transclusion_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph>

/// Build a full reference graph (transclusions + all reference types at each node).
pub(crate) fn build_reference_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph>

/// Flatten a reference graph into composed-order ReferenceSet.
pub(crate) fn flatten_graph(graph: &ReferenceGraph) -> ReferenceSet
```

**Execution model per node** (from tech design §Graph Analysis Model):

1. Clone the current `Markdown`
2. Apply only InlinePre operations that affect reference visibility:
   - `TextReplacement` — may change href/src text
   - `PageBlocks` — may remove entire sections containing references
   - `Interpolation` — may expand `{{var}}` in href/src
   - `ShellExpansion` — may inject references
3. Do NOT run InlinePost (Cleanup, Normalization) — these don't affect reference targets
4. Extract local references from the prepared content:
   - `local::extract_markdown_links()` + `local::extract_markdown_images()`
   - `html::extract_html_links()` + `html::extract_html_images()`
   - Transclusion directives as reference records
5. For markdown-bearing children (`::file`, `::url`, frontmatter prologue/epilogue):
   - Resolve target using existing `resolver::resolve_target()`
   - Load child `Markdown` document
   - Recurse (with `TransclusionRuntime` for cycle/depth safety)
6. For non-recursive children (`::code`, `::toc-linking`):
   - Record as reference/dependency but do NOT recurse into a child graph node
7. Record child insertion order for composed-order flattening

**Runtime struct:**

```rust
pub(crate) struct ReferenceAnalysisRuntime {
    pub transclusion: TransclusionRuntime,
    pub loaded_markdown: dashmap::DashMap<String, Markdown>,
}
```

Reuses `TransclusionRuntime` directly for cycle/depth detection. `loaded_markdown` provides document-level dedup (load each file once).

**InlinePre execution path:**

To run only InlinePre operations without running the full compose pipeline, build a `ComposeOptions` with:
```rust
ComposeOptions::new()
    .only(&[
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::ShellExpansion,
    ])
    .with_source(source)
    // ... carry forward relevant options from ReferenceGraphOptions.compose
```

Then call `clone.compose_with(inline_pre_options)` and discard the report. This reuses the existing pipeline machinery without inventing a separate execution path.

**Flattening algorithm:**

Depth-first traversal of the graph:
1. For each node, yield prologue child references (recursed)
2. Yield the node's own local references in source order
3. At each directive insertion point, yield the child's flattened references (recursed)
4. Yield epilogue child references (recursed)

This produces composed-order output where references appear in the order they would exist in the final composed document.

**Tests:**
- Single document with links → graph has 1 node, references match local extraction
- Document with `::file child.md` → graph has 2 nodes, child links appear in flattened set
- Document with prologue → prologue references appear first in flattened order
- Document with epilogue → epilogue references appear last
- `::code` directive → recorded as reference but no child graph node
- `::toc-linking` → recorded as reference but no child graph node
- Cycle detection → error propagated
- Depth limit → error propagated
- `ComposeSource::Unknown` with relative `::file` → `MissingSourceContext` error
- Page blocks removing a section with links → those links absent from graph

---

### 1.8 Create `markdown/reference/validate.rs` — Local File Validation

**Create:** `darkmatter/lib/src/markdown/reference/validate.rs`

**Types:**

```rust
pub struct ReferenceValidationOptions {
    pub graph: ReferenceGraphOptions,
    pub validate_remote: bool,
    pub remote_timeout: std::time::Duration,
    pub validate_fragments: bool,
    pub fail_fast: bool,
}

pub struct ReferenceValidationReport {
    pub references_scanned: usize,
    pub references_valid: usize,
    pub issues: Vec<ReferenceIssue>,
    pub warnings: Vec<String>,
}

pub struct ReferenceIssue {
    pub code: ReferenceIssueCode,
    pub message: String,
    pub severity: ReferenceSeverity,
    pub reference_id: String,
    pub origin: ReferenceOrigin,
}

pub enum ReferenceIssueCode {
    MissingLocalTarget,
    InvalidUrl,
    RemoteUnreachable,
    RemoteDisallowed,
    MissingSourceContext,
    UnsupportedScheme,
    MissingFragmentTarget,
    MalformedHtmlTag,
    MalformedCssImport,
    MalformedMetaTag,
}

pub enum ReferenceSeverity {
    Error,
    Warning,
    Info,
}
```

**Phase 1 validation scope:**

1. **Local path validation** — For each `ReferenceTarget::LocalPath`:
   - Resolve through `biscuit_file::FileReference` (same rules as compose)
   - Check `path.exists()`
   - Report `MissingLocalTarget` if not found
2. **URL syntax validation** — For each `ReferenceTarget::RemoteUrl`:
   - Parse with `url::Url::parse()`
   - Report `InvalidUrl` if malformed
   - If `validate_remote == false`: mark as unverified (no issue)
3. **Missing source context** — If source is `ComposeSource::Unknown` and validation requires path resolution, report `MissingSourceContext` as warning
4. **Unsupported schemes** — `OtherScheme` targets get `UnsupportedScheme` at Info severity

**Phase 1 does NOT include:** remote HTTP validation, fragment validation. These are Phase 2.

**`fail_fast` behavior:** If true, return immediately on first Error-severity issue.

**Tests:**
- Local path that exists → valid, no issues
- Local path that doesn't exist → `MissingLocalTarget` error
- `@/path/file.md` resolution → uses `FileReference`
- Valid URL → no issue (unverified)
- Malformed URL → `InvalidUrl` error
- Source-less markdown with relative ref → `MissingSourceContext` warning
- `fail_fast` stops on first error
- Mix of valid and invalid → report counts correct

---

### 1.9 Wire Up Public API Methods

**File:** `darkmatter/lib/src/markdown/reference/mod.rs`

Implement the `impl Markdown` methods declared in 1.3 by delegating to internal modules:

```rust
impl Markdown {
    pub fn has_transclusions(&self) -> bool {
        // Delegate to transclusion parser (1.4)
    }

    pub fn transclusions(&self) -> MarkdownResult<Vec<TransclusionRef>> {
        // Delegate to transclusion parser + resolver (1.4)
    }

    pub fn transclusion_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceGraph> {
        graph::build_transclusion_graph(self, &options).map_err(Into::into)
    }

    pub fn reference_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceGraph> {
        graph::build_reference_graph(self, &options).map_err(Into::into)
    }

    pub fn composed_references(&self, options: ReferenceGraphOptions) -> MarkdownResult<ReferenceSet> {
        let graph = graph::build_reference_graph(self, &options)?;
        Ok(graph::flatten_graph(&graph))
    }

    pub fn composed_links(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<LinkReference>> {
        let refs = self.composed_references(options)?;
        Ok(refs.records.into_iter()
            .filter(|r| r.kind == ReferenceKind::Hyperlink)
            .map(LinkReference::from)
            .collect())
    }

    pub fn composed_image_references(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<ImageReference>> {
        let refs = self.composed_references(options)?;
        Ok(refs.records.into_iter()
            .filter(|r| r.kind == ReferenceKind::Image)
            .map(ImageReference::from)
            .collect())
    }

    pub fn validate_references(&self, options: ReferenceValidationOptions) -> MarkdownResult<ReferenceValidationReport> {
        validate::validate(self, &options).map_err(Into::into)
    }
}
```

**Tests:** Integration tests using `tempfile` fixtures:
- Create a multi-file document tree, verify `composed_links()` returns all links in composed order
- Verify `validate_references()` catches broken local paths
- Verify `reference_graph()` structure matches expected node/edge layout

---

### Phase 1 Task Dependency Order

```
1.1 types.rs ──┐
1.2 errors.rs ─┤
               ├── 1.3 mod.rs (module scaffold)
               │      │
1.4 has_transclusions/transclusions ◄── 1.3
1.5 local.rs ◄── 1.1
1.6 html.rs  ◄── 1.1
               │
1.7 graph.rs ◄── 1.4, 1.5, 1.6
1.8 validate.rs ◄── 1.7
1.9 wire up API ◄── 1.7, 1.8
```

**Tasks 1.1 + 1.2** can be done in parallel.
**Tasks 1.5 + 1.6** can be done in parallel (both depend only on 1.1).
**Task 1.7** is the critical path — it depends on extraction (1.5, 1.6) and transclusion queries (1.4).

---

## Phase 2 — Extended Extraction and Validation

> CSS/font/script/meta extraction, remote validation, fragment validation, merge helpers.

### 2.1 Generalize `html.rs` — Extract `<style>`, `<script>`, `<link>`, `<meta>`

**Modify:** `darkmatter/lib/src/markdown/reference/html.rs`

Add extraction functions:

```rust
pub(crate) fn extract_html_style_blocks(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord>
pub(crate) fn extract_html_script_blocks(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord>
pub(crate) fn extract_html_link_tags(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord>
pub(crate) fn extract_html_meta_tags(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord>
```

**Implementation strategy:**

Replace the tag-specific extraction in the MDAST walker with a generalized HTML fragment collector:

1. Walk MDAST `Html` nodes (same entry point as Phase 1)
2. For each HTML fragment, classify the tag:
   - `<a>` → existing link extraction
   - `<img>` → existing image extraction
   - `<style>...</style>` → `InlineCss` reference (target: `Inline`)
   - `<script src="...">` → `ScriptImport` reference
   - `<script>...</script>` (no src) → `InlineScript` reference (target: `Inline`)
   - `<link rel="stylesheet" href="...">` → `CssImport` reference
   - `<link ... as="font">` or font-indicating rel → `FontImport` reference
   - `<meta ...>` → `MetaTag` reference
3. Multi-part tags (`<style>` content split across adjacent MDAST Html nodes) must be paired using source-order scanning

**Constraint:** Only raw HTML that appears as MDAST `Html` nodes is considered. Directive-like text inside fenced code blocks remains ignored (same gate as existing).

**Tests:**
- `<style>body { color: red; }</style>` → `InlineCss`, `HtmlStyleTag`, `Inline`
- `<script src="app.js"></script>` → `ScriptImport`, `HtmlScriptTag`, `LocalPath`
- `<script>console.log("hi")</script>` → `InlineScript`, `HtmlScriptTag`, `Inline`
- `<link rel="stylesheet" href="style.css">` → `CssImport`, `HtmlLinkTag`, `LocalPath`
- `<link rel="preload" href="font.woff2" as="font">` → `FontImport`, `HtmlLinkTag`, `LocalPath`
- `<meta name="description" content="Hello">` → `MetaTag`, `HtmlMetaTag`
- Tags inside code blocks → excluded

---

### 2.2 Create `markdown/reference/css.rs` — CSS Import and Font Extraction

**Create:** `darkmatter/lib/src/markdown/reference/css.rs`

**Add dependency:** `cssparser` to `darkmatter/lib/Cargo.toml`

**Functions:**

```rust
/// Extract @import URLs from a CSS string (inside <style> blocks).
pub(crate) fn extract_css_imports(css_content: &str, source: &ComposeSource, base_line: usize) -> Vec<ReferenceRecord>

/// Extract @font-face src: url(...) references from a CSS string.
pub(crate) fn extract_font_face_sources(css_content: &str, source: &ComposeSource, base_line: usize) -> Vec<ReferenceRecord>
```

**Implementation:**

Use `cssparser` tokenizer to parse:
1. `@import url("...")` and `@import "..."` → `CssImport`, `CssAtImport`
2. `@font-face { src: url("...") }` → `FontImport`, `CssFontFaceSrc`

The `base_line` parameter offsets line numbers so provenance maps back to the original markdown document's `<style>` block position.

**Tests:**
- `@import url("reset.css")` → `CssImport` with `LocalPath`
- `@import "https://fonts.googleapis.com/..."` → `CssImport` with `RemoteUrl`
- `@font-face { src: url("font.woff2") }` → `FontImport` with `LocalPath`
- Malformed CSS → graceful skip with warning, not crash
- Comments and nested functions handled correctly

---

### 2.3 Create `markdown/reference/meta.rs` — Meta Tag Parsing

**Create:** `darkmatter/lib/src/markdown/reference/meta.rs`

**Types:**

```rust
pub enum MetaValue {
    String(String),
    Many(Vec<String>),
}

pub type MetaTagMap = indexmap::IndexMap<String, MetaValue>;
```

**Functions:**

```rust
/// Parse <meta> tags from content into a normalized map.
pub(crate) fn parse_meta_tags(content: &str) -> MetaTagMap

/// Merge meta tags into frontmatter.
pub(crate) fn merge_meta_into_frontmatter(
    meta: &MetaTagMap,
    frontmatter: &mut Frontmatter,
    overwrite: bool,
) -> usize
```

**Key priority for meta tag map key** (from tech design):
1. `name` attribute
2. `property` attribute (Open Graph)
3. `http-equiv` attribute
4. `charset` attribute

**Value source:** `content` attribute, except `charset` which uses the tag value directly.

**Duplicate handling:** First occurrence creates `MetaValue::String`. Subsequent occurrences with same key promote to `MetaValue::Many`.

**`merge_meta_into_frontmatter`:**
1. Normalize meta keys to stable strings
2. Insert into frontmatter using existing `fm_insert()` method
3. Skip existing keys unless `overwrite == true`
4. Return count of keys inserted or updated

**Markdown public API:**

```rust
impl Markdown {
    pub fn meta_tags(&self) -> MarkdownResult<MetaTagMap> { ... }
    pub fn merge_meta_into_frontmatter(&mut self, overwrite: bool) -> MarkdownResult<usize> { ... }
}
```

**Defer `set_meta_tag()`** to Phase 3 per tech design recommendation (write-path, not immediately needed).

**Tests:**
- `<meta name="author" content="Ken">` → `{"author": String("Ken")}`
- `<meta property="og:title" content="Hello">` → `{"og:title": String("Hello")}`
- `<meta charset="utf-8">` → `{"charset": String("utf-8")}`
- Duplicate names → `Many(["first", "second"])`
- Merge into empty frontmatter → all inserted, returns count
- Merge with existing keys, `overwrite=false` → skipped
- Merge with existing keys, `overwrite=true` → overwritten

---

### 2.4 Wire CSS/Script/Font/Meta into Graph and Public API

**Modify:** `darkmatter/lib/src/markdown/reference/local.rs` and `html.rs`

Integrate the new extractors into the graph node extraction pipeline so that `build_reference_graph()` picks up CSS, script, font, and meta references at each node.

**Add public API methods:**

```rust
impl Markdown {
    // Local (single document) queries
    pub fn inline_css(&self) -> MarkdownResult<Vec<InlineCssBlock>> { ... }
    pub fn css_imports(&self) -> MarkdownResult<Vec<ImportReference>> { ... }
    pub fn inline_scripts(&self) -> MarkdownResult<Vec<InlineScriptBlock>> { ... }
    pub fn script_imports(&self) -> MarkdownResult<Vec<ImportReference>> { ... }
    pub fn font_imports(&self) -> MarkdownResult<Vec<ImportReference>> { ... }

    // Graph queries (recursive through transclusions)
    pub fn inline_css_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<InlineCssBlock>> { ... }
    pub fn css_import_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<ImportReference>> { ... }
    pub fn inline_script_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<InlineScriptBlock>> { ... }
    pub fn script_import_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<ImportReference>> { ... }
    pub fn font_import_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<ImportReference>> { ... }
}
```

**Convenience wrapper types:**

```rust
pub struct InlineCssBlock {
    pub record: ReferenceRecord,
    pub css_content: String,
}

pub struct InlineScriptBlock {
    pub record: ReferenceRecord,
    pub script_content: String,
}

pub struct ImportReference {
    pub record: ReferenceRecord,
    pub href: String,
}
```

**Tests:**
- Document with `<style>@import "reset.css"</style>` → `css_imports()` returns 1 entry
- Document `::file`-including a child with `<script src="app.js">` → `script_import_graph()` includes child's script
- `font_import_graph()` traverses through transclusions

---

### 2.5 Remote URL Validation

**Modify:** `darkmatter/lib/src/markdown/reference/validate.rs`

When `options.validate_remote == true`:

1. For each `ReferenceTarget::RemoteUrl`:
   - Parse URL (already done in Phase 1)
   - Send `HEAD` request via `reqwest::Client` with configurable timeout
   - If `HEAD` returns `405 Method Not Allowed`, fall back to `GET`
   - 2xx/3xx → valid
   - 4xx/5xx → `RemoteUnreachable` with status code in message
   - Timeout → `RemoteUnreachable` with timeout message
2. Use a shared `reqwest::Client` across all validation calls (connection pooling)
3. Limit concurrent remote checks (bounded via `tokio::sync::Semaphore`, default 10)

**Note:** `reqwest` is already a dependency. No new crate needed.

**Tests:** Use `wiremock` (already in workspace test deps):
- Mock server returns 200 → valid
- Mock server returns 404 → `RemoteUnreachable`
- Mock server returns 405 on HEAD, 200 on GET → valid (fallback works)
- Timeout → `RemoteUnreachable`
- `validate_remote == false` → URL left unverified

---

### 2.6 Fragment Validation

**Modify:** `darkmatter/lib/src/markdown/reference/validate.rs`

When `options.validate_fragments == true`:

1. **Same-document fragments** (`#section-name`):
   - Build heading set from the document (or composed document if using graph mode)
   - Check fragment against slug-ified heading list
   - Report `MissingFragmentTarget` if not found
2. **Cross-document fragments** (`path.md#section`):
   - Only validate when target is a local markdown file that can be loaded
   - Load target, extract headings, check fragment
   - Skip remote fragments (deferred)

**Heading slug generation:** Reuse the same slugification logic that `toc/` module uses for anchor generation.

**Tests:**
- `#existing-heading` in document → valid
- `#nonexistent` in document → `MissingFragmentTarget`
- `./other.md#heading` where heading exists → valid
- `./other.md#missing` → `MissingFragmentTarget`
- Remote URL with fragment → skipped (not validated)

---

### Phase 2 Task Dependency Order

```
2.1 html.rs generalization ──┐
2.2 css.rs ◄── 2.1           ├── 2.4 wire into graph + API
2.3 meta.rs                  ─┘
2.5 remote validation (independent)
2.6 fragment validation (independent)
```

**Tasks 2.1, 2.3, 2.5, 2.6** can begin in parallel.
**Task 2.2** depends on 2.1 (needs `<style>` content extraction).
**Task 2.4** depends on 2.1, 2.2, 2.3.

---

## Phase 3 — Polish and Tooling

> Persistent caching, CLI surface, graph visualization.

### 3.1 Persistent Caching for Reference Analysis

**Modify:** `darkmatter/lib/src/markdown/reference/graph.rs`

Extend `ReferenceAnalysisRuntime` to optionally use the existing compose cache infrastructure:

```rust
pub(crate) struct ReferenceAnalysisRuntime {
    pub transclusion: TransclusionRuntime,
    pub loaded_markdown: dashmap::DashMap<String, Markdown>,
    pub reference_cache: Option<dashmap::DashMap<String, ReferenceSet>>,  // NEW
}
```

**Cache key:** `ref:{source_id_hash:016x}:{content_hash:016x}`

**Invalidation:** Same closure-hash strategy used by compose cache (`cache/hashing.rs`). A document's reference snapshot is valid as long as its content hash and all transitive dependency hashes match.

**Implementation:** This is optimization, not correctness. The uncached path must remain correct and be the fallback.

**Tests:**
- First call computes references, second call hits cache
- Content change invalidates cache
- Child document change invalidates parent's cache

---

### 3.2 CLI Surface — `md validate refs`

**Modify:** `darkmatter/cli/src/main.rs` (or appropriate subcommand module)

Add a `validate` subcommand with a `refs` sub-subcommand:

```
md validate refs doc.md [options]
```

**Options:**
- `--remote` — enable remote URL validation
- `--fragments` — enable fragment validation
- `--timeout <seconds>` — remote validation timeout (default: 10)
- `--fail-fast` — stop on first error
- `--format <text|json>` — output format (default: text)
- `--verbose` / `-v` — show all references, not just issues

**Output (text format):**

```
References scanned: 42
Valid: 38
Issues: 4

ERROR  doc.md:15  Missing local target: ./missing-file.md
ERROR  doc.md:23  Missing local target: @/docs/removed.md
WARN   child.md:8  Remote URL not verified: https://example.com/page
INFO   doc.md:31  Unsupported scheme: mailto:test@example.com
```

**Output (JSON format):** Serialize `ReferenceValidationReport` directly.

**Tests:** CLI integration tests using `assert_cmd` + `predicates`:
- `md validate refs valid-doc.md` → exit 0, "0 issues"
- `md validate refs broken-doc.md` → exit 1, lists issues
- `md validate refs doc.md --format json` → valid JSON output

---

### 3.3 Graph Visualization Output

**Modify:** `darkmatter/lib/src/markdown/reference/graph.rs`

Add methods to `ReferenceGraph`:

```rust
impl ReferenceGraph {
    /// Render as Mermaid flowchart.
    pub fn to_mermaid(&self) -> String { ... }

    /// Render as DOT (Graphviz) graph.
    pub fn to_dot(&self) -> String { ... }
}
```

**CLI integration:** Add `--graph` flag to `md validate refs`:

```
md validate refs doc.md --graph        # Print Mermaid diagram
md validate refs doc.md --graph=dot    # Print DOT diagram
```

**Tests:**
- Single node graph → valid Mermaid/DOT
- Multi-node graph with edges → correct structure
- Cycle (detected as error) → graph still renderable up to cycle point

---

### 3.4 Deferred: `set_meta_tag()`

Per tech design recommendation, `set_meta_tag()` is deferred unless a caller immediately needs it. It is the only write-path in this feature and modifies the markdown body (not just frontmatter).

```rust
impl Markdown {
    pub fn set_meta_tag(&mut self, key: impl Into<String>, value: MetaValue) -> MarkdownResult<()> { ... }
}
```

Implementation would:
1. Find existing `<meta>` tag with matching key
2. If found: update the `content` attribute in-place
3. If not found: append a new `<meta>` tag after the last existing one (or at document start)

---

## File Change Summary

### New Files

| File | Phase | Description |
|------|-------|-------------|
| `markdown/reference/mod.rs` | 1.3 | Module entry point, `impl Markdown` block |
| `markdown/reference/types.rs` | 1.1 | All reference types, enums, graph structures |
| `markdown/reference/errors.rs` | 1.2 | `ReferenceError` enum |
| `markdown/reference/local.rs` | 1.5 | Markdown-native extraction with provenance |
| `markdown/reference/html.rs` | 1.6 / 2.1 | HTML tag extraction with provenance |
| `markdown/reference/graph.rs` | 1.7 | Graph traversal and flattening |
| `markdown/reference/validate.rs` | 1.8 / 2.5–2.6 | Validation engine and report |
| `markdown/reference/css.rs` | 2.2 | CSS `@import` and `@font-face` parsing |
| `markdown/reference/meta.rs` | 2.3 | `<meta>` tag parsing and frontmatter merge |

### Modified Files

| File | Phase | Change |
|------|-------|--------|
| `markdown/mod.rs` | 1.3 | Add `pub mod reference;`, re-exports, `source` field on `Markdown` |
| `markdown/types.rs` | 1.2 | Add `MarkdownError::Reference` variant |
| `lib/Cargo.toml` | 2.2 | Add `cssparser` dependency |
| CLI subcommand module | 3.2 | Add `validate refs` subcommand |

### Unchanged Files

| File | Reason |
|------|--------|
| `markdown/inline_html.rs` | Existing extraction preserved as-is |
| `render/link.rs` | `Link` type unchanged |
| `render/image_ref.rs` | `ImageRef` type unchanged |
| `compose/transclusion/` | Reused, not modified |
| `compose/types.rs` | `ComposeOptions`, `ComposeSource` unchanged |

---

## Testing Strategy Summary

| Layer | Approach | Location |
|-------|----------|----------|
| Type construction | Unit tests | `reference/types.rs` `#[cfg(test)]` |
| Local extraction | Unit tests with inline markdown | `reference/local.rs`, `reference/html.rs` `#[cfg(test)]` |
| CSS/meta parsing | Unit tests with inline content | `reference/css.rs`, `reference/meta.rs` `#[cfg(test)]` |
| Graph traversal | Integration tests with `tempfile` fixtures | `reference/graph.rs` `#[cfg(test)]` |
| Validation | Integration tests with `tempfile` + `wiremock` | `reference/validate.rs` `#[cfg(test)]` |
| Public API | Integration tests | `reference/mod.rs` `#[cfg(test)]` |
| CLI | `assert_cmd` + `predicates` | `darkmatter/cli/tests/` |

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| **InlinePre execution for graph nodes adds overhead** | Only run when `has_transclusions() == true`; leaf documents skip InlinePre entirely if no directives that affect references |
| **`cssparser` adds a new dependency** | It's a well-maintained Mozilla crate (used by Servo). Only needed in Phase 2. |
| **Source field on Markdown is a structural change** | It's additive (`Option<ComposeSource>`, defaults to `None`), non-breaking for existing callers |
| **Graph traversal may be slow for deep trees** | Reuse `TransclusionRuntime` depth limits (default 16). Add `loaded_markdown` dedup to avoid re-parsing. |
| **Remote validation may be flaky in CI** | Remote validation is opt-in (`validate_remote: false` by default). Tests use `wiremock` for deterministic mocking. |
