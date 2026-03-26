# FileTree Tech Design

This document defines the implementation-ready technical design for the `FileTree` feature in Darkmatter. It is derived from:

- `darkmatter/features/2026-03-25-file-tree-component/spec.md`
- the current Darkmatter reference graph subsystem in `darkmatter/lib/src/markdown/reference/`
- the existing reference validation pipeline in `darkmatter/lib/src/markdown/reference/validate.rs`
- the current Darkmatter CLI shape in `darkmatter/cli/src/args.rs` and `darkmatter/cli/src/commands.rs`
- the `FileSystem` component in `biscuit-terminal/lib/src/components/filesystem.rs`

The design goal is to add a production-grade terminal component and CLI experience for visualizing a Markdown document's dependency surface without duplicating the reference graph or validation logic that Darkmatter already has.

## Purpose

Darkmatter already has most of the hard backend work needed for this feature:

1. graph-aware reference extraction
2. recursive transclusion traversal
3. provenance-aware reference records
4. validation of local and remote reference targets

What it does not yet have is the terminal-facing presentation layer described by the spec:

1. a `Renderable` component centered on a single Markdown file
2. a visual split between "references above" and "transclusions below"
3. optional recursive transclusion expansion as a nested file tree
4. inline validation overlays in that tree view
5. a dedicated `md graph <file-ref>` CLI entry point

The correct design is to treat `FileTree` as a view-model and renderer on top of the existing reference graph, not as a second graph builder.

## Goals

1. Add a Darkmatter `FileTree` component that implements `biscuit_terminal::components::renderable::Renderable`.
2. Make `FileTree` operate on a single Markdown file root, not a directory.
3. Show non-transclusion dependencies above the file line.
4. Show transclusions below the file line.
5. Support `.follow_transclusions()` so local Markdown transclusions expand into nested `FileTree` subtrees.
6. Support optional validation overlays without changing default extraction behavior.
7. Reuse biscuit-terminal Nerd Font detection and Unicode fallback behavior.
8. Add `md graph <file-ref>` with `--follow` and `--validate`.
9. Preserve source-aware semantics from the existing reference graph and validation engines.

## Non-Goals

1. Replacing `ReferenceGraph` with a `FileTree`-specific traversal engine.
2. Turning every hyperlink or image reference into a recursive graph node.
3. Adding remote network validation to `md graph --validate` in v1.
4. Making stdin a first-class input mode for `md graph`.
5. Replacing existing `md validate refs` detailed validation output.
6. Replacing existing Mermaid or DOT graph exports.

## Current Baseline

Darkmatter already provides:

1. `Markdown::reference_graph(options)` for recursive reference discovery
2. `Markdown::validate_references(options)` for graph-aware validation
3. `ReferenceRecord`, `ReferenceSet`, `ReferenceGraph`, and `ReferenceGraphNode`
4. extraction for hyperlinks, images, CSS imports, script imports, inline CSS, inline scripts, meta tags, and transclusions
5. transclusion provenance including source file and directive line

Darkmatter does not yet provide:

1. a terminal component for these graphs
2. a file-centric render model
3. section-aware transclusion captions such as "inserted into `## Some Section`"
4. an inline validation presentation layer
5. a `graph` CLI subcommand

## Primary Recommendation

Build `FileTree` as a thin presentation layer over `ReferenceGraph` plus optional `ReferenceValidationReport`.

That means:

1. keep graph construction in `markdown/reference/graph.rs`
2. keep validation in `markdown/reference/validate.rs`
3. add a new renderer-specific view-model that groups reference records for terminal output
4. extend graph metadata only where the view genuinely needs more context

This is the lowest-risk design because it preserves one source of truth for:

1. path resolution
2. compose preparation behavior
3. transclusion recursion
4. validation semantics

## Public API

### Component API

Add a new public type exported through `darkmatter::markdown::reference` and re-exported from `darkmatter::markdown`:

```rust
use biscuit_terminal::components::renderable::Renderable;

#[derive(Debug, Clone)]
pub struct FileTree { ... }

impl FileTree {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self, FileTreeError>;
    pub fn from_markdown(md: Markdown) -> Result<Self, FileTreeError>;

    pub fn follow_transclusions(mut self) -> Self;
    pub fn validate(mut self) -> Self;
    pub fn graph_options(mut self, options: ReferenceGraphOptions) -> Self;
    pub fn validation_options(mut self, options: ReferenceValidationOptions) -> Self;
    pub fn show_root(mut self, show: bool) -> Self;

    pub fn ensure_built(&mut self) -> Result<(), FileTreeError>;
    pub fn graph(&self) -> Option<&ReferenceGraph>;
    pub fn validation_report(&self) -> Option<&ReferenceValidationReport>;
}

impl Renderable for FileTree { ... }
```

### Error Type

Add a component-specific error wrapper:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FileTreeError {
    #[error("path not found: {0}")]
    PathNotFound(std::path::PathBuf),

    #[error("not a file: {0}")]
    NotAFile(std::path::PathBuf),

    #[error("failed to load markdown: {0}")]
    Markdown(#[from] MarkdownError),

    #[error("failed to analyze references: {0}")]
    Reference(#[from] ReferenceError),
}
```

This matches the ergonomics of `FileSystemError` while still delegating actual parsing and analysis failures to Darkmatter's existing error types.

### Builder Behavior

`FileTree` should follow the same usage pattern as `FileSystem`:

1. construction is cheap
2. graph/model building is lazy
3. rendering requires prior build, or triggers build internally
4. `render_optimistic()` uses Unicode fallback icons
5. `render()` uses terminal-aware icon and ANSI decisions

## Module Layout

Recommended additions:

```txt
darkmatter/lib/src/markdown/reference/
├── mod.rs
├── types.rs
├── graph.rs
├── validate.rs
└── file_tree/
    ├── mod.rs
    ├── model.rs
    ├── render.rs
    └── icons.rs
```

Responsibilities:

1. `file_tree/mod.rs`: public component API, lazy build orchestration, error type
2. `file_tree/model.rs`: graph-to-view-model transformation
3. `file_tree/render.rs`: terminal string rendering
4. `file_tree/icons.rs`: icon selection and Unicode fallback mapping

The `file_tree/` submodule is justified because this feature has distinct concerns:

1. build-time transformation
2. render-time formatting
3. icon policy
4. validation overlays

## Core Design

### Two-Layer Architecture

The design has two layers:

1. analysis layer: existing `ReferenceGraph` and `ReferenceValidationReport`
2. presentation layer: new `FileTreeModel`

`FileTree` never walks Markdown syntax directly. It always consumes the graph APIs.

### View Model

Recommended internal types:

```rust
struct FileTreeModel {
    root: FileTreeNode,
}

struct FileTreeNode {
    source: ComposeSource,
    file_label: String,
    file_icon_kind: FileTreeIconKind,
    inline_summary: FileTreeInlineSummary,
    reference_groups: Vec<FileTreeReferenceGroup>,
    transclusions: Vec<FileTreeTransclusionEdge>,
    children: Vec<FileTreeNode>,
    validation: FileTreeNodeValidation,
}

struct FileTreeReferenceGroup {
    kind: FileTreeReferenceGroupKind,
    rows: Vec<FileTreeReferenceRow>,
}

struct FileTreeReferenceRow {
    kind: ReferenceKind,
    display_target: String,
    raw_reference_id: String,
    validation: Option<FileTreeReferenceValidation>,
}

struct FileTreeTransclusionEdge {
    kind: FileTreeTransclusionKind,
    display_target: String,
    caption: String,
    directive_line: usize,
    followable: bool,
    child_node_id: Option<String>,
    validation: Option<FileTreeReferenceValidation>,
}
```

This gives the renderer a stable, presentation-ready structure without polluting `ReferenceGraph` with terminal-only concerns.

## Graph Reuse Strategy

### Reuse As-Is

The following behavior should be reused exactly as it exists today:

1. compose-aware reference extraction
2. path resolution through `biscuit_file::FileReference`
3. shared run-local cache behavior
4. transclusion depth and cycle protection
5. validation rules and issue classification

### Extend the Graph Metadata

`FileTree` needs one important piece of information the current graph does not retain cleanly:

1. insertion context for a transclusion directive

The view needs captions like:

- `inserted into the '## Some Section' section`
- `inserted TOC links into the '## Another Section' section`

To support that, extend `ReferenceInsertion` or introduce a companion metadata struct:

```rust
pub struct ReferenceInsertionContext {
    pub directive_kind: ReferenceSyntax,
    pub section_heading_text: Option<String>,
    pub section_heading_level: Option<u8>,
}
```

Updated type:

```rust
pub struct ReferenceInsertion {
    pub child_node_id: String,
    pub directive_line: usize,
    pub insertion_order: usize,
    pub context: ReferenceInsertionContext,
}
```

### How Section Context Is Derived

When building a node in `graph.rs`:

1. parse prepared content into a heading index keyed by line range
2. for each directive line, look up the active section at that line
3. attach the closest active heading to the insertion metadata

This should be based on prepared content, not raw source, so page blocks and interpolation cannot drift the reported section.

## Follow Semantics

### What `.follow_transclusions()` Means

`follow_transclusions()` changes only the rendering of transclusion edges. It does not change the extraction of non-transclusion references.

Without follow:

1. transclusions render as leaf rows below the file line
2. existence can still be validated
3. no nested subtrees are shown

With follow:

1. followable transclusions render both as an edge and as a nested child `FileTreeNode`
2. the child node gets the same "references above / file / transclusions below" treatment
3. recursion stops at the same transclusion depth and cycle rules already enforced by `ReferenceGraph`

### What Counts As Followable

In v1, only local Markdown document transclusions are followable:

1. `::file`
2. `::toc-linking`
3. frontmatter `prologue`
4. frontmatter `epilogue`

Not followable in v1:

1. `::url`
2. `::code`
3. any remote URL target
4. any local target known to be non-Markdown

This avoids misleading nested trees for assets or remote content.

## Rendering Rules

### Visual Structure

Each rendered node has three vertical zones:

1. reference groups above
2. file head line
3. transclusion edges below

Example shape:

```txt
    | ̅ ̅ ̅ ̅ 🔗 https://example.com
    | ̅ ̅ ̅ ̅ 🖼 ./image.png
    |
📄 root.md (2 inline scripts, 3 meta tags)
    |
    |<--- 📄 @docs/a.md inserted into the '## Intro' section
    |<--- 📄 @docs/b.md inserted into the '## Details' section
```

### Group Ordering Above the File

Use a fixed semantic group order, with source order preserved inside each group:

1. remote hyperlinks
2. local document/file hyperlinks
3. image references
4. CSS imports
5. script imports
6. font imports
7. other local file-like dependencies

Blank lines are inserted between non-empty groups.

Inline CSS, inline scripts, and meta tags are not rendered as rows above the file. They contribute to the file-head summary instead.

### File Head Summary

The file line renders:

1. an icon
2. the display file label
3. optional summary counts in parentheses

Summary counts include only non-zero categories:

1. inline CSS blocks
2. inline script blocks
3. meta tags

Example:

```txt
📄 foobar.md (2 inline scripts, 3 meta tags)
```

### Transclusion Edge Captions

Render captions from directive kind:

1. `::file`, `prologue`, `epilogue`: `inserted into the '## Heading' section`
2. `::toc-linking`: `inserted TOC links into the '## Heading' section`
3. `::url`: `summarize <url> into the '## Heading' section` when the directive is a summarization form, otherwise `transcluded from <url>`
4. `::code`: `inserted code into the '## Heading' section`

The view should use the raw authored target when available. Resolved paths are for validation and follow decisions, not primary display.

### Tree Connectors

Use the same connector philosophy as `FileSystem`:

1. box-drawing characters in terminal-aware and optimistic renders
2. no line wrapping across connector prefixes
3. layout margins apply after line construction, not during connector generation

The renderer should keep connector strings and content strings separate until the last stage so truncation never mangles the graph prefix.

## Icon Policy

### Nerd Font Detection

Use the provided `Terminal.is_nerd_font` signal during `render()` and Unicode fallback in `render_optimistic()`, exactly like `FileSystem`.

### Icon Reuse

Reuse `biscuit-terminal` `filesystem::icons` for file and Markdown-document icons where possible.

Add a small `FileTree` icon mapping for non-file dependency rows:

1. hyperlink: Nerd Font link/globe icon, fallback `🔗`
2. image: Nerd Font image/media icon, fallback `🖼`
3. CSS: Nerd Font palette/stylesheet icon, fallback `🎨`
4. script: Nerd Font code icon, fallback `📜`
5. font: Nerd Font typography icon, fallback `🔤`
6. summarize/url transclusion: Nerd Font brain/agent icon if available, fallback `🧠`

The exact Nerd Font code points can be finalized during implementation, but the fallback set should be explicit in tests.

## Validation Design

### Validation Scope

`FileTree::validate()` and `md graph --validate` should use `ReferenceValidationOptions` with these defaults:

1. `validate_remote = false`
2. `validate_fragments = false`
3. `fail_fast = false`

This matches the spec's intent:

1. validate immediate links and transclusion targets
2. do not turn the graph command into a network checker
3. when `--follow` is also set, validate the entire followed tree recursively

### Presentation of Validation Results

Validation issues should be projected back into the view model by `reference_id`.

Recommended presentation:

1. valid local references: no suffix
2. missing local target: red row plus concise suffix such as `[missing]`
3. invalid URL syntax: red row plus `[invalid url]`
4. unsupported scheme or unverified remote: yellow/info styling when shown

The graph command should also print a one-line summary after the tree when validation is enabled:

```txt
18 references scanned, 16 valid, 2 issues
```

### Exit Codes

`md graph --validate` should exit non-zero when error-severity validation issues exist.

Recommended:

1. `0` when no error issues exist
2. `2` when validation found one or more error issues
3. `1` for command/runtime failures

## CLI Design

### New Subcommand

Add to `darkmatter/cli/src/args.rs`:

```rust
Graph {
    #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
    input: PathBuf,

    #[arg(long)]
    follow: bool,

    #[arg(long)]
    validate: bool,
}
```

### CLI Semantics

`md graph <file-ref>`:

1. resolves `<file-ref>` through the same file-resolution path used by `compose` and `edit`
2. builds `FileTree`
3. renders to terminal using `Terminal::default()`

`md graph <file-ref> --follow`:

1. enables `.follow_transclusions()`
2. renders recursively expanded file-tree children

`md graph <file-ref> --validate`:

1. validates the non-follow tree surface
2. checks transclusion target existence even when not following
3. annotates invalid rows inline
4. prints a summary footer

`md graph <file-ref> --follow --validate`:

1. validates the full recursively expanded tree
2. surfaces validation state inline at every nested node

### Why a New Command Instead of Extending `validate refs`

`md validate refs` remains the audit-style command.

`md graph` is the structural visualization command.

They should share backend analysis, but keep distinct UX:

1. `validate refs`: issue-first textual report
2. `graph`: topology-first tree view with optional status overlay

## Input Resolution

`md graph` should require a file path or file reference, not stdin.

Reason:

1. relative dependency resolution depends on a real source file
2. follow mode depends on recursive file loading
3. validation of local targets depends on source context

If stdin support is added later, it should require an explicit `--source-file`.

## Implementation Plan

### Phase 1: Graph Metadata

1. add section-aware insertion context to graph nodes
2. add any small helpers needed to identify followable Markdown transclusions
3. keep `ReferenceGraph` as the canonical backend

### Phase 2: FileTree Component

1. add `file_tree/` module
2. implement lazy build orchestration
3. transform graph plus optional validation into `FileTreeModel`
4. implement `Renderable`

### Phase 3: CLI

1. add `graph` subcommand
2. wire file-resolution logic
3. render through terminal-aware path
4. return validation-based exit codes when applicable

### Phase 4: Docs

Update:

1. `darkmatter/cli/README.md`
2. `darkmatter/docs/cli/graph.md`
3. `darkmatter/lib/README.md`
4. `darkmatter/docs/structs/Markdown.md` or a new `FileTree` struct doc if it is publicly exported

## Testing Strategy

### Library Unit Tests

Add unit tests for:

1. grouping reference rows by semantic kind
2. file-head inline summary generation
3. transclusion caption generation by directive kind
4. followability decisions
5. validation annotation projection by `reference_id`
6. section heading lookup for directive lines

### Rendering Tests

Add deterministic render tests for:

1. Unicode fallback render
2. Nerd Font render when `Terminal.is_nerd_font = Some(true)`
3. blank-line separation between groups
4. nested follow-mode rendering
5. validation suffixes and styling markers
6. truncation behavior with narrow widths

Use the existing terminal testing helpers in `darkmatter/lib/src/testing/terminal.rs` where appropriate.

### CLI Tests

Add integration tests for:

1. `md graph <file>`
2. `md graph <file> --follow`
3. `md graph <file> --validate`
4. `md graph <file> --follow --validate`
5. non-zero exit code on invalid references
6. file-reference resolution for `@...` style inputs

## Key Design Decisions

1. `FileTree` is a renderer over `ReferenceGraph`, not a second analyzer.
2. Only Markdown document transclusions are followable in v1.
3. Inline HTML resource blocks contribute to node summaries rather than cluttering the dependency rows.
4. `md graph` is topology-first and complements, rather than replaces, `md validate refs`.
5. Validation remains local-only by default for this command.

## Future Extensions

These are reasonable follow-ups, but not part of v1:

1. `--remote` validation for `md graph`
2. machine-readable `md graph --json`
3. graph render export to Mermaid or DOT directly from `md graph`
4. richer per-row metadata such as line numbers in verbose mode
5. hover/open interactions in terminals that support OSC8 beyond the file head line
