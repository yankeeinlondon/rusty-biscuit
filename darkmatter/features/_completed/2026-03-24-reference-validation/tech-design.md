# Reference Validation Tech Design

This document defines the implementation-ready technical design for the `reference-validation` feature in Darkmatter. It is derived from:

- `darkmatter/features/2026-03-24. reference-validation/spec.md`
- the current `Markdown` API in `darkmatter/lib/src/markdown/mod.rs`
- the current inline HTML extraction in `darkmatter/lib/src/markdown/inline_html.rs`
- the compose pipeline and caching runtime in `darkmatter/lib/src/markdown/compose/`
- existing transclusion resolution behavior and `biscuit-file::FileReference` path semantics

The design goal is to add graph-aware reference analysis and validation without regressing the current fast local extraction APIs or duplicating compose logic.

## Purpose

Darkmatter already has two useful but narrow capabilities:

1. local Markdown link/image extraction via `links()` and `image_references()`
2. local inline HTML link/image extraction via `inline_html_links()` and `inline_html_image_references()`

Those APIs are intentionally cheap and local, but they are not sufficient for the problem the spec is actually asking to solve:

1. references in the effective composed document
2. references introduced or removed by page blocks, interpolation, shell expansion, and transclusion
3. additional reference classes beyond hyperlinks and images
4. validation and diagnostics over those references

The correct abstraction is not “make `links()` secretly compose first”. The correct abstraction is a reference-analysis subsystem with explicit local, graph, and composed views.

## Scope

In scope:

1. transclusion-aware reference discovery
2. validation of local file-like references
3. optional validation of remote URLs
4. first-class querying for transclusions
5. extraction of hyperlinks, image references, CSS/script/font imports, inline CSS/script blocks, and meta tags
6. metadata merge helpers for HTML `<meta>` tags
7. diagnostics that retain source document and line information
8. reuse of existing compose semantics for path resolution, condition evaluation, and recursive traversal

Out of scope for v1:

1. making validation a default compose operation
2. full HTML DOM normalization or rendering-time HTML mutation
3. executing scripts or fetching CSS/fonts as part of validation
4. deep CSS semantic validation beyond import and font-source extraction
5. perfect source maps from composed byte offsets back to every original source byte
6. caching remote validation results across runs

## Current Baseline

Darkmatter currently has:

1. a mature three-phase compose pipeline with operation-level toggles
2. recursive transclusion with depth and cycle detection
3. run-local and persistent compose caching
4. local Markdown link and image extraction
5. local inline HTML `<a>` and `<img>` extraction
6. source-aware resolution through `ComposeSource` and `biscuit-file::FileReference`

Darkmatter does not yet have:

1. a unified “reference” model
2. graph-aware reference APIs
3. extraction for transclusion directives as references
4. extraction for `<style>`, `<script>`, `<link>`, or `<meta>`
5. validation reports distinct from compose reports
6. provenance-aware flattened composed reference ordering

## Primary Recommendations

### 1. Keep existing local APIs local

Do not change `links()`, `inline_html_links()`, `image_references()`, or `inline_html_image_references()` to implicitly compose the document.

Reasons:

1. that would be a silent behavior and performance break
2. current callers likely depend on these methods being cheap and local
3. composed analysis needs more than just flattened `Link` and `ImageRef` values; it needs provenance, diagnostics, and graph structure

Recommendation:

1. keep current methods unchanged
2. add explicit graph/composed APIs for callers that need reference-aware composition

### 2. Add a dedicated `markdown/reference/` subsystem

Reference analysis should be a first-class module, not ad hoc helpers bolted onto `Markdown`.

Recommended module layout:

```txt
darkmatter/lib/src/markdown/
├── inline_html.rs                # keep fast HTML fragment helpers, refactor internals as needed
├── reference/
│   ├── mod.rs
│   ├── types.rs
│   ├── local.rs
│   ├── html.rs
│   ├── css.rs
│   ├── graph.rs
│   ├── validate.rs
│   └── meta.rs
└── ...
```

Responsibilities:

1. `local.rs`: local Markdown-native extraction
2. `html.rs`: raw HTML tag/block extraction from markdown content
3. `css.rs`: CSS import and font-source extraction from `<style>` blocks
4. `graph.rs`: recursive document graph analysis and composed-order flattening
5. `validate.rs`: filesystem/network validation and report building
6. `meta.rs`: `<meta>` parsing and frontmatter merge helpers

### 3. Treat transclusion as a graph, not just a string transform

The composed document should be modeled as a traversal over document nodes and insertion edges, not just as `compose()` followed by reparsing text.

This is the key design choice.

Reasons:

1. composed-order results can still retain source document provenance
2. validation diagnostics can point to originating files and lines
3. frontmatter transclusion and block transclusion already form a natural graph
4. future graph-visualization and dependency-reporting features can reuse the same structure

### 4. Do not add a public `compose_without_report()` in v1

The spec proposes `.compose_without_report()`. The public API does not need it yet.

Reasons:

1. `compose()` and `compose_with()` already let callers ignore the report
2. a public compose-only variant duplicates an API shape that is not otherwise missing
3. reference analysis should not depend on flattening through the public compose API

Recommendation:

1. add a crate-private helper only if the implementation benefits from an internal compose-only path
2. defer a public convenience wrapper until there is broader demand

## Public API Design

### Naming

Use Rust-style noun/verb names instead of `get_*`.

Examples:

1. `transclusions()`, not `get_transclusions()`
2. `meta_tags()`, not `get_meta_tags()`
3. `css_imports()`, not `get_css_imports()`

### `Markdown` additions

Recommended public additions:

```rust
impl Markdown {
    pub fn has_transclusions(&self) -> bool;

    pub fn transclusions(&self) -> MarkdownResult<Vec<TransclusionRef>>;
    pub fn transclusion_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceGraph>;

    pub fn reference_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceGraph>;

    pub fn composed_references(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<ReferenceSet>;

    pub fn composed_links(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<LinkReference>>;

    pub fn composed_image_references(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImageReference>>;

    pub fn inline_css(&self) -> MarkdownResult<Vec<InlineCssBlock>>;
    pub fn inline_css_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineCssBlock>>;

    pub fn css_imports(&self) -> MarkdownResult<Vec<ImportReference>>;
    pub fn css_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>>;

    pub fn inline_scripts(&self) -> MarkdownResult<Vec<InlineScriptBlock>>;
    pub fn inline_script_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineScriptBlock>>;

    pub fn script_imports(&self) -> MarkdownResult<Vec<ImportReference>>;
    pub fn script_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>>;

    pub fn font_imports(&self) -> MarkdownResult<Vec<ImportReference>>;
    pub fn font_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>>;

    pub fn meta_tags(&self) -> MarkdownResult<MetaTagMap>;
    pub fn merge_meta_into_frontmatter(&mut self, overwrite: bool) -> MarkdownResult<usize>;
    pub fn set_meta_tag(&mut self, key: impl Into<String>, value: MetaValue) -> MarkdownResult<()>;

    pub fn validate_references(
        &self,
        options: ReferenceValidationOptions,
    ) -> MarkdownResult<ReferenceValidationReport>;
}
```

Design notes:

1. current `links()` and `image_references()` remain unchanged and local-only
2. “graph” methods preserve source provenance and recursive structure
3. “composed” methods return a flattened source-ordered view of the effective document

## Core Types

Recommended foundational types:

```rust
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
    MarkdownLink,
    HtmlAnchor,
    MarkdownImage,
    HtmlImage,
    DirectiveFile,
    DirectiveUrl,
    DirectiveCode,
    DirectiveTocLinking,
    FrontmatterPrologue,
    FrontmatterEpilogue,
    HtmlLinkTag,
    HtmlScriptTag,
    HtmlStyleTag,
    CssAtImport,
    CssFontFaceSrc,
    HtmlMetaTag,
}

pub enum ReferenceTarget {
    LocalPath { raw: String },
    RemoteUrl { raw: String },
    Fragment { raw: String },
    DataUri { raw: String },
    OtherScheme { raw: String, scheme: String },
    Inline,
}

pub struct ReferenceOrigin {
    pub source: ComposeSource,
    pub line: usize,
    pub span: std::ops::Range<usize>,
    pub syntax: ReferenceSyntax,
}

pub struct ReferenceRecord {
    pub id: String,
    pub kind: ReferenceKind,
    pub target: ReferenceTarget,
    pub origin: ReferenceOrigin,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

pub struct ReferenceSet {
    pub records: Vec<ReferenceRecord>,
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
```

Type-specific wrappers such as `LinkReference`, `ImageReference`, `InlineCssBlock`, and `MetaTagMap` should be thin convenience views over `ReferenceRecord` plus strongly typed payloads where needed.

## Graph Analysis Model

### Execution model

Reference graph analysis should reuse compose semantics but not literally call full compose for every question.

For each markdown node:

1. clone the current `Markdown`
2. apply only inline-pre operations that can change reference visibility or target text:
   - `TextReplacement`
   - `PageBlocks`
   - `Interpolation`
   - `ShellExpansion`
3. do not run inline-post operations; cleanup and normalization do not materially affect reference targets
4. parse local references from the prepared content
5. parse transclusion directives and frontmatter transclusion refs from the prepared node
6. recursively load and analyze child markdown nodes using the same source resolution and option semantics as compose
7. record child insertion order so the final reference list can be flattened in composed order

This gives the feature the same functional truth as compose where it matters, without requiring a lossy reparsing of the final flattened markdown string.

### Why not just analyze the fully composed text?

Because that loses the most important validation information:

1. which source file introduced the broken reference
2. which transclusion edge pulled it into the document
3. which line in the original source should receive the diagnostic

## Transclusion Semantics

### `has_transclusions()`

This should be a cheap local query over the current document and frontmatter:

1. return `true` if the body contains `::file`, `::url`, `::code`, or `::toc-linking`
2. return `true` if frontmatter contains non-empty `prologue` or `epilogue`
3. otherwise return `false`

Implementation should reuse the existing transclusion directive parser and frontmatter parser, not add another directive detector.

### `transclusions()`

This method should return local transclusion references only:

1. block directives in source order
2. frontmatter `prologue` refs in frontmatter order
3. frontmatter `epilogue` refs in frontmatter order

Each entry should include:

1. directive kind
2. raw target
3. resolved target when source context is available
4. options such as `when`, `replace`, `quotation`, `disclosure`, and `exclude`
5. source line/span provenance

### `transclusion_graph()`

This method should recursively resolve markdown-bearing children:

1. `::file`
2. `::url` when enabled
3. frontmatter `prologue`
4. frontmatter `epilogue`

`::code` and `::toc-linking` are still references and dependencies, but they are not recursive markdown nodes in the same way:

1. `::code` contributes a reference record and validation target, but not a child markdown graph node
2. `::toc-linking` contributes a reference/dependency record; any generated links belong to composed reference analysis only if that operation is explicitly included in analysis options

## Local Extraction Details

### Hyperlinks and images

Retain the current pulldown-cmark extraction path for Markdown-native links/images and generalize the current inline HTML helper so it can also emit provenance spans.

Recommended refactor:

1. move the low-level extraction loops into `markdown/reference/local.rs`
2. keep `Markdown::links()` and related existing methods as wrappers
3. extend the extraction results with source span and line data for the new graph-aware APIs

### HTML blocks and tags

The current `inline_html.rs` only extracts `<a>` and `<img>`. It should be generalized to support:

1. `<a>`
2. `<img>`
3. `<style>...</style>`
4. `<script>...</script>`
5. `<link ...>`
6. `<meta ...>`

Important constraint:

1. only raw HTML that appears as markdown HTML nodes should be considered
2. directive-like text inside fenced code blocks must remain ignored

Recommended strategy:

1. keep using MDAST positions to find HTML fragments
2. replace the tag classifier with a generalized fragment collector
3. pair multi-part tags like `<style>` and `<script>` using source-order scanning

### CSS imports and font sources

For v1, parse only what is required for validation:

1. `<link rel="stylesheet" href="...">`
2. CSS `@import` inside `<style>` blocks
3. font-like `<link>` tags where `as="font"` or `rel` implies a font resource
4. `@font-face` `src: url(...)` inside `<style>` blocks

Recommended dependency:

1. add `cssparser` for `<style>` content parsing rather than regex-heavy parsing

Reason:

1. CSS parsing is deceptively tricky once quoting, comments, and nested functions appear

### Scripts

Treat scripts in two buckets:

1. inline script blocks: `<script>...</script>` without `src`
2. script imports: `<script src="...">`

Validation should never execute scripts. It only validates that imported targets are syntactically and optionally physically reachable.

### Meta tags

`meta_tags()` should parse local document `<meta>` tags into a normalized map:

1. key priority: `name`, then `property`, then `http-equiv`, then `charset`
2. value source: `content`, except `charset` which uses the tag value directly
3. duplicate keys should preserve order and be represented as `MetaValue::Many`

Recommended types:

```rust
pub enum MetaValue {
    String(String),
    Many(Vec<String>),
}

pub type MetaTagMap = indexmap::IndexMap<String, MetaValue>;
```

`merge_meta_into_frontmatter(overwrite)` should:

1. normalize meta keys to stable strings
2. insert into frontmatter using existing serialization logic
3. skip existing keys unless `overwrite == true`
4. return the number of keys inserted or updated

`set_meta_tag()` should modify the body, not just frontmatter. It is the only write-path in this feature and should be deferred to phase 2 unless it is immediately needed by a caller.

## Validation Model

### Separate report type

Validation should not write into `ComposeReport`. It should have its own report:

```rust
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
```

Recommended issue codes:

1. `MissingLocalTarget`
2. `InvalidUrl`
3. `RemoteUnreachable`
4. `RemoteDisallowed`
5. `MissingSourceContext`
6. `UnsupportedScheme`
7. `MissingFragmentTarget`
8. `MalformedHtmlTag`
9. `MalformedCssImport`
10. `MalformedMetaTag`

### Validation options

```rust
pub struct ReferenceValidationOptions {
    pub graph: ReferenceGraphOptions,
    pub validate_remote: bool,
    pub remote_timeout: std::time::Duration,
    pub validate_fragments: bool,
    pub fail_fast: bool,
}
```

`ReferenceGraphOptions` should include:

1. `compose: ComposeOptions`
2. `include_generated_toc_links: bool`
3. `follow_remote_transclusions: bool`

### Local path validation

All file-like references should resolve through the same rules compose already uses:

1. source-aware relative resolution
2. `@` repository-root semantics
3. URL-vs-file distinction compatible with current `Link` and `ImageRef` behavior

Recommendation:

1. use `biscuit-file::FileReference` for local-path normalization and resolution
2. never invent separate path logic inside reference validation

### Remote validation

Remote validation must be opt-in.

Default behavior:

1. parse the URL
2. if syntactically valid, treat it as unverified unless `validate_remote == true`

When enabled:

1. use `reqwest`
2. prefer `HEAD`
3. fall back to `GET` only when `HEAD` is clearly unsupported
4. treat 2xx and 3xx as success
5. surface timeouts and 4xx/5xx as validation issues

### Fragment validation

Fragment validation should be intentionally narrow in v1:

1. validate same-document fragments against the relevant heading set
2. validate `path#fragment` only when the target is a local markdown file that can be loaded safely
3. defer remote fragment validation

For composed validation:

1. same-document fragment checks should use the composed heading set
2. local-only checks should use the local document heading set

## Runtime and Caching

### Reuse the compose runtime where it helps

Reference graph analysis should reuse run-local document loading and source normalization behavior. It does not need full persistent caching in v1.

Recommended runtime:

```rust
pub struct ReferenceAnalysisRuntime {
    pub transclusion: compose::transclusion::TransclusionRuntime,
    pub loaded_markdown: dashmap::DashMap<String, Markdown>,
}
```

Future optimization:

1. add persistent cached reference snapshots keyed by source body hash
2. invalidate them using the same closure-hash strategy used by composed document cores

That should be phase 2, not a v1 blocker.

## Error Handling

Recommended error surface:

```rust
pub enum ReferenceError {
    ParseDirective { line: usize, message: String },
    MissingSourceContext { reference: String, line: usize },
    Validation(String),
    Compose(#[from] MarkdownError),
    FileReference(#[from] biscuit_file::FileReferenceError),
    Io(#[from] std::io::Error),
    Url(#[from] url::ParseError),
}
```

Public `Markdown` methods can continue returning `MarkdownResult<T>` by wrapping `ReferenceError` in `MarkdownError::Transform` or by adding a dedicated `MarkdownError::Reference` variant if the maintainers want stronger typing.

Recommendation:

1. add `MarkdownError::Reference(ReferenceError)` for cleaner public diagnostics

## Testing Strategy

### Unit tests

Add focused tests for:

1. local markdown links and images with provenance spans
2. HTML `<a>`, `<img>`, `<style>`, `<script>`, `<link>`, and `<meta>`
3. CSS `@import` and `@font-face src` extraction
4. frontmatter meta merge behavior

### Graph tests

Use `tempfile` fixtures for:

1. `::file` recursion
2. frontmatter `prologue` and `epilogue`
3. page blocks removing references before traversal
4. interpolation mutating href/src targets
5. shell expansion injecting references
6. cycles and depth limits

### Validation tests

1. local file existence checks
2. repo-root `@` path resolution
3. same-document fragment validation
4. remote validation using `wiremock`
5. source-less markdown instances returning `MissingSourceContext` where appropriate

## Rollout Plan

### Phase 1

1. add local transclusion queries and `has_transclusions()`
2. add unified reference types and local extraction with provenance
3. add reference graph traversal for markdown-bearing transclusions
4. add composed flattening
5. add local file validation

### Phase 2

1. add CSS/font/script/meta extraction
2. add remote validation
3. add fragment validation across local markdown targets
4. add `merge_meta_into_frontmatter()`

### Phase 3

1. persistent caching for reference analysis
2. CLI surface such as `md validate refs`
3. optional graph visualization output using the same reference graph model

## Final Recommendation

Implement this feature as a reference-analysis subsystem with explicit graph and validation APIs.

Do not:

1. overload existing local link/image methods with implicit compose behavior
2. add validation as a default compose operation
3. invent new path-resolution semantics outside `biscuit-file` and current compose source rules

Do:

1. keep local extraction cheap
2. build a provenance-preserving reference graph
3. flatten that graph into composed-order results when callers need the effective document view
4. layer validation on top of that graph so diagnostics stay tied to the correct source files
