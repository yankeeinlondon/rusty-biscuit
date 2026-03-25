# FileTree Implementation Plan

## Context

Darkmatter has a mature reference graph subsystem (`darkmatter/lib/src/markdown/reference/`) that handles graph-aware reference extraction, recursive transclusion traversal, and validation. What it lacks is a **terminal presentation layer** centered on a single Markdown file's dependency surface. This plan implements:

1. A `FileTree` component implementing `Renderable` — shows references above and transclusions below the file line
2. An `md graph <file> [--follow] [--validate]` CLI command
3. The graph metadata extension (section-aware insertion context) needed by the view

The design treats `FileTree` as a thin view-model and renderer over `ReferenceGraph`, not a second analyzer.

---

## Phase 1: Graph Metadata Extension

**Goal**: Extend `ReferenceInsertion` with section-aware context so transclusion captions can say "inserted into the '## Intro' section". Add followability helper.

### 1A. New type: `ReferenceInsertionContext`

**File**: `darkmatter/lib/src/markdown/reference/types.rs` (after line 248)

```rust
#[derive(Debug, Clone, Default)]
pub struct ReferenceInsertionContext {
    pub directive_kind: Option<ReferenceSyntax>,
    pub section_heading_text: Option<String>,
    pub section_heading_level: Option<u8>,
}
```

Extend `ReferenceInsertion` (line 241) with a new field:

```rust
pub struct ReferenceInsertion {
    pub child_node_id: String,
    pub directive_line: usize,
    pub insertion_order: usize,
    pub context: ReferenceInsertionContext,  // NEW
}
```

### 1B. Followability helper on `ReferenceSyntax`

**File**: `darkmatter/lib/src/markdown/reference/types.rs`

```rust
impl ReferenceSyntax {
    pub fn is_followable_transclusion(&self) -> bool {
        matches!(self,
            ReferenceSyntax::DirectiveFile
            | ReferenceSyntax::DirectiveTocLinking
            | ReferenceSyntax::FrontmatterPrologue
            | ReferenceSyntax::FrontmatterEpilogue
        )
    }
}
```

### 1C. Derive section context in `build_node()`

**File**: `darkmatter/lib/src/markdown/reference/graph.rs`

Add two helpers:

- `build_heading_index(prepared_content: &str) -> Vec<(usize, String, u8)>` — parses `prepared_content` via `Markdown::new().toc().all_headings()`, returns `(start_line, title, level)` tuples sorted by line
- `section_at_line(index, target_line) -> Option<(&str, u8)>` — binary/reverse search for the active heading at a given line

In `build_node()` (line 202), after `prepare_content()` returns, build the heading index once:

```rust
let heading_index = build_heading_index(&prepared_content);
```

Then update all **three** `ReferenceInsertion` construction sites:

| Site | Line | directive_kind | Notes |
|------|------|---------------|-------|
| `::file` directives | 269 | `syntax` variable (already in scope at line 233) | Section from `section_at_line(&heading_index, directive.line)` |
| Prologue | 370 | `FrontmatterPrologue` | `directive_line: 0` → section context is `None` |
| Epilogue | 415 | `FrontmatterEpilogue` | `directive_line: usize::MAX` → section context is last heading or `None` |

**Note**: `::toc-linking` directives do not produce `ReferenceInsertion` entries (they produce `ReferenceRecord` entries instead at line 296), so they are unaffected. But transclusion records for toc-linking already carry `ReferenceSyntax::DirectiveTocLinking` in their `origin.syntax`, which the view model will use.

### 1D. Fix existing test construction sites

All manual `ReferenceInsertion` constructions in `graph.rs` tests (around line 816) need the new `context: ReferenceInsertionContext::default()` field.

---

## Phase 2: FileTree Component

**Goal**: Create the `file_tree/` submodule with view model, lazy build, Renderable implementation, and icons.

### Module Layout

```
darkmatter/lib/src/markdown/reference/file_tree/
├── mod.rs    — public API: FileTree struct, FileTreeError, lazy build, Renderable delegation
├── model.rs  — view model types + graph-to-model transformation
├── render.rs — terminal string rendering (Renderable impl)
└── icons.rs  — icon selection with nerd font / unicode fallback
```

### 2A. `file_tree/mod.rs` — Public API

**Key types**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FileTreeError {
    #[error("path not found: {}", .0.display())]
    PathNotFound(PathBuf),
    #[error("not a file: {}", .0.display())]
    NotAFile(PathBuf),
    #[error("failed to load markdown: {0}")]
    Markdown(#[from] MarkdownError),
    #[error("failed to analyze references: {0}")]
    Reference(#[from] ReferenceError),
}
```

**`FileTree` struct fields**:

| Field | Type | Purpose |
|-------|------|---------|
| `md` | `Markdown` | Source document |
| `follow` | `bool` | Recursive transclusion expansion |
| `validate` | `bool` | Run validation |
| `show_root` | `bool` | Show root file label (default true) |
| `graph_options` | `ReferenceGraphOptions` | Graph extraction options |
| `validation_options` | `ReferenceValidationOptions` | Validation options |
| `layout` | `Layout` | Renderable layout config |
| `model` | `Option<FileTreeModel>` | Lazily-built view model |
| `graph` | `Option<ReferenceGraph>` | Retained graph |
| `validation_report` | `Option<ReferenceValidationReport>` | Retained report |

**Public methods**:

| Method | Signature | Notes |
|--------|-----------|-------|
| `new` | `(path: impl AsRef<Path>) -> Result<Self, FileTreeError>` | Validates path exists and is file, loads Markdown |
| `from_markdown` | `(md: Markdown) -> Self` | From already-loaded doc |
| `follow_transclusions` | `(mut self) -> Self` | Builder — sets `follow = true`, invalidates model |
| `validate` | `(mut self) -> Self` | Builder — sets `validate = true`, invalidates model |
| `graph_options` | `(mut self, opts) -> Self` | Builder |
| `validation_options` | `(mut self, opts) -> Self` | Builder |
| `show_root` | `(mut self, show: bool) -> Self` | Builder |
| `ensure_built` | `(&mut self) -> Result<(), FileTreeError>` | Lazy build: graph → optional validation → model |
| `graph` | `(&self) -> Option<&ReferenceGraph>` | Access built graph |
| `validation_report` | `(&self) -> Option<&ReferenceValidationReport>` | Access built report |

**`ensure_built()` orchestration**:
1. If `model.is_some()` → return Ok
2. Build `ReferenceGraph` via `md.reference_graph(graph_options)`
3. If `validate` → build `ReferenceValidationReport` via `md.validate_references(validation_options)`
4. Call `model::build_file_tree_model(&graph, report.as_ref(), follow)` → `FileTreeModel`
5. Store all three

**Renderable impl** delegates to `render::render_model()` / `render::render_model_optimistic()`.

### 2B. `file_tree/model.rs` — View Model

**Core types**:

| Type | Key Fields |
|------|------------|
| `FileTreeModel` | `root: FileTreeNode` |
| `FileTreeNode` | `source`, `file_label`, `file_icon_kind`, `inline_summary`, `reference_groups`, `transclusions`, `children`, `validation` |
| `FileTreeInlineSummary` | `inline_css_count`, `inline_script_count`, `meta_tag_count` — plus `to_display_string()` for "(2 inline scripts, 3 meta tags)" |
| `FileTreeReferenceGroup` | `kind: FileTreeReferenceGroupKind`, `rows: Vec<FileTreeReferenceRow>` |
| `FileTreeReferenceGroupKind` | `RemoteHyperlinks`, `LocalHyperlinks`, `Images`, `CssImports`, `ScriptImports`, `FontImports`, `OtherLocalDependencies` — with `sort_order()` method |
| `FileTreeReferenceRow` | `kind`, `display_target`, `raw_reference_id`, `validation: Option<FileTreeReferenceValidation>` |
| `FileTreeTransclusionEdge` | `kind`, `display_target`, `caption`, `directive_line`, `followable`, `child_node_id`, `validation` |
| `FileTreeReferenceValidation` | `is_valid`, `suffix: Option<String>`, `severity` |
| `FileTreeNodeValidation` | `issues_count`, `has_errors` |

**Key transformation functions**:

- `build_file_tree_model(graph, report, follow) -> FileTreeModel` — entry point
- `build_node_model(node, graph, report, follow) -> FileTreeNode` — recursive per-node transform
- `classify_reference_group(record) -> Option<FileTreeReferenceGroupKind>` — maps `ReferenceKind` + `ReferenceTarget` to group kind; returns `None` for inline types (InlineCss, InlineScript, MetaTag) and Transclusion
- `transclusion_caption(context: &ReferenceInsertionContext) -> String` — generates captions based on `directive_kind` and `section_heading_text`

**Reference group classification**:

| ReferenceKind | ReferenceTarget | Group |
|---------------|----------------|-------|
| Hyperlink | RemoteUrl | RemoteHyperlinks |
| Hyperlink | LocalPath / Fragment | LocalHyperlinks |
| Image | any | Images |
| CssImport | any | CssImports |
| ScriptImport | any | ScriptImports |
| FontImport | any | FontImports |
| InlineCss / InlineScript / MetaTag | — | None (→ inline_summary) |
| Transclusion | — | None (→ transclusions) |

**Group ordering**: Fixed by `sort_order()`: Remote hyperlinks → Local hyperlinks → Images → CSS → Scripts → Fonts → Other. Source order preserved within each group. Blank line between non-empty groups.

**Caption generation**:

| directive_kind | Caption |
|---------------|---------|
| `DirectiveFile` / `FrontmatterPrologue` / `FrontmatterEpilogue` | `inserted into the '## Heading' section` |
| `DirectiveTocLinking` | `inserted TOC links into the '## Heading' section` |
| `DirectiveCode` | `inserted code into the '## Heading' section` |
| `DirectiveUrl` | `transcluded from URL into the '## Heading' section` |

**Followability**: A transclusion edge is followable when its corresponding `ReferenceRecord.origin.syntax.is_followable_transclusion()` is true **and** the target is `ReferenceTarget::LocalPath`. When `follow = true` and edge is followable, look up the child node in `graph.node_by_id(child_node_id)` and recursively build a `FileTreeNode` for it. If not found (cycle-pruned), mark as non-followable leaf.

**Validation projection**: When `report` is `Some`, iterate `report.issues` and build a `HashMap<String, Vec<&ReferenceIssue>>` keyed by `reference_id`. When building each row, look up issues by `raw_reference_id` and project:

| IssueCode | Suffix | Severity |
|-----------|--------|----------|
| MissingLocalTarget | `[missing]` | Error |
| InvalidUrl | `[invalid url]` | Error |
| UnsupportedScheme | `[unsupported]` | Warning |
| RemoteUnreachable | `[unreachable]` | Error |
| Other | `[issue]` | from issue |

### 2C. `file_tree/render.rs` — Terminal Rendering

**Three vertical zones per node**:

```
    │ ̅ ̅ ̅ ̅ 🔗 https://example.com       ← reference groups (above)
    │ ̅ ̅ ̅ ̅ 🖼 ./image.png
    │
📄 root.md (2 inline scripts, 3 meta tags)  ← file head line
    │
    │<--- 📄 @docs/a.md inserted into the '## Intro' section  ← transclusion edges (below)
```

**Rendering approach**:

1. Each node renders to a `Vec<String>` of lines
2. Reference group rows: `"    │ ̅ ̅ ̅ ̅ {icon} {display_target}{validation_suffix}"`
3. Blank separator line between non-empty groups: `"    │"`
4. File head: `"{icon} {file_label}{summary}"`
5. Transclusion edge: `"    │<--- {icon} {display_target} {caption}{validation_suffix}"` for incoming, `"    │---> {icon} {display_target} {caption}"` for TOC-linking outgoing
6. Follow-mode children: indent the child's entire rendering by the node's connector width

**Connector strings**: Reuse `biscuit_terminal::components::filesystem::tree_chars` constants where applicable. Keep connectors and content separate until final assembly so truncation never mangles graph prefixes.

**ANSI styling** (only when `is_tty`):
- File label: bold
- Remote URLs: blue
- Local paths: default
- Validation error rows: red
- Validation warning rows: yellow
- Inline summary: dim

**Width tracking**: Each nesting level consumes ~8 columns of indent. Content (not connectors) truncates with ellipsis when width is exhausted.

### 2D. `file_tree/icons.rs` — Icon Mapping

| Row Kind | Nerd Font | Unicode Fallback |
|----------|-----------|-----------------|
| Hyperlink | `\u{f0c1}` (link) | `🔗` |
| Image | `\u{f03e}` (image) | `🖼` |
| CSS | `\u{e749}` (css3) | `🎨` |
| Script | `\u{e60c}` (code) | `📜` |
| Font | `\u{f031}` (font) | `🔤` |
| File (markdown) | `\u{f0354}` (markdown) | `📄` |
| File (generic) | `\u{ea7b}` (file) | `📄` |
| URL transclusion | `\u{f0362}` (brain) | `🧠` |

Uses `term.is_nerd_font == Some(true)` for nerd font, unicode fallback otherwise. Follows same pattern as `FileSystem::get_icon()`.

### 2E. Wire into public API

**File**: `darkmatter/lib/src/markdown/reference/mod.rs` — add `pub mod file_tree;`

**File**: `darkmatter/lib/src/markdown/mod.rs` (line 55) — add to re-exports:
```rust
pub use reference::file_tree::{FileTree, FileTreeError};
```

---

## Phase 3: CLI Integration

**Goal**: Add `md graph <file> [--follow] [--validate]` command.

### 3A. Add `Graph` variant to `Command` enum

**File**: `darkmatter/cli/src/args.rs` (after `Hash` variant, before line 252 closing brace)

```rust
/// Visualize a markdown file's dependency graph.
Graph {
    /// Input file path or file reference
    #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
    input: PathBuf,

    /// Recursively expand followable transclusions
    #[arg(long)]
    follow: bool,

    /// Validate references and show inline status
    #[arg(long)]
    validate: bool,
},
```

### 3B. Add command handler

**File**: `darkmatter/cli/src/commands.rs`

Add dispatch in `run_subcommand()`:

```rust
CliCommand::Graph { input, follow, validate } => {
    run_graph(&input, follow, validate)?;
}
```

Handler implementation:

```rust
fn run_graph(input: &PathBuf, follow: bool, validate: bool) -> Result<()> {
    let resolved = resolve_file_path(input)?;
    let mut tree = FileTree::new(&resolved)
        .map_err(|e| eyre!("{e}"))?;

    if follow {
        tree = tree.follow_transclusions();
    }
    if validate {
        tree = tree.validate();
    }

    tree.ensure_built().map_err(|e| eyre!("{e}"))?;

    let term = Terminal::default();
    print!("{}", tree.display(&term));

    // Validation summary footer
    if validate {
        if let Some(report) = tree.validation_report() {
            println!(
                "{} references scanned, {} valid, {} issues",
                report.references_scanned,
                report.references_valid,
                report.issues.len()
            );
        }
    }

    // Exit code 2 for validation errors
    if validate {
        if let Some(report) = tree.validation_report() {
            if !report.is_valid() {
                std::process::exit(2);
            }
        }
    }

    Ok(())
}
```

### 3C. Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success or validation passed |
| 1 | Runtime/command errors (existing `main()` error path) |
| 2 | Validation found error-severity issues |

---

## Phase 4: Testing

### 4A. Unit tests — model transformation

**File**: `darkmatter/lib/src/markdown/reference/file_tree/model.rs` (`#[cfg(test)] mod tests`)

| Test | Verifies |
|------|----------|
| `test_reference_grouping_by_kind` | References classified into correct `FileTreeReferenceGroupKind` |
| `test_inline_summary_counts` | InlineCss/InlineScript/MetaTag → summary, not rows |
| `test_inline_summary_display_string` | Formatted output with singular/plural |
| `test_transclusion_caption_file` | `::file` + section → "inserted into the '## X' section" |
| `test_transclusion_caption_toc_linking` | `::toc-linking` → "inserted TOC links..." |
| `test_transclusion_caption_no_section` | No heading context → just "inserted" |
| `test_followability_file` | `::file` returns followable = true |
| `test_followability_url` | `::url` returns followable = false |
| `test_followability_code` | `::code` returns followable = false |
| `test_validation_projection` | Issues mapped to correct rows by reference_id |
| `test_group_sort_order` | Groups appear in documented fixed order |
| `test_file_label_extraction` | Filename extracted from ComposeSource::File |

### 4B. Unit tests — graph metadata

**File**: `darkmatter/lib/src/markdown/reference/graph.rs` (existing `#[cfg(test)] mod tests`)

| Test | Verifies |
|------|----------|
| `test_section_context_populated` | Document with headings + `::file` → context.section_heading_text set |
| `test_section_context_prologue_none` | Prologue at line 0 → section context is None |
| `test_is_followable_transclusion` | `ReferenceSyntax` helper returns correct values |

### 4C. Rendering tests

**File**: `darkmatter/lib/src/markdown/reference/file_tree/render.rs` (`#[cfg(test)] mod tests`)

Uses `Terminal::new_optimistic(80)` and `strip_ansi_codes()` from `darkmatter::testing`.

| Test | Verifies |
|------|----------|
| `test_render_no_references` | Just file head line |
| `test_render_references_above` | Groups above with correct icons |
| `test_render_transclusions_below` | Edges with captions |
| `test_render_blank_line_between_groups` | Separator between different group kinds |
| `test_render_follow_nested` | Recursive child indentation |
| `test_render_validation_suffix` | `[missing]` suffix on broken ref |
| `test_render_unicode_fallback` | Emoji icons when is_nerd_font is None |
| `test_render_nerd_font` | Nerd font icons when is_nerd_font is Some(true) |
| `test_render_narrow_width` | Content truncation, connectors preserved |

### 4D. Integration tests — graph metadata

**File**: `darkmatter/lib/tests/reference_integration.rs` (alongside existing tests)

| Test | Verifies |
|------|----------|
| `test_section_context_in_real_graph` | Multi-section doc with `::file` in different sections |
| `test_file_tree_from_real_filesystem` | FileTree built from tempdir document, model populated |

### 4E. CLI integration tests

**File**: `darkmatter/cli/tests/cli.rs`

| Test | Verifies |
|------|----------|
| `test_graph_basic` | `md graph <tempfile>` succeeds, output contains filename |
| `test_graph_follow` | `--follow` succeeds, output contains child filename |
| `test_graph_validate_valid` | `--validate` exits 0 for valid doc |
| `test_graph_validate_invalid` | `--validate` exits non-zero for broken refs |
| `test_graph_follow_validate` | Both flags together |
| `test_graph_file_not_found` | Fails with appropriate error message |

Uses `assert_cmd::cargo_bin_cmd!("md")`, `tempfile::NamedTempFile`, `predicates`.

---

## Phase 5: Documentation

### 5A. CLI README

**File**: `darkmatter/cli/README.md` — add `md graph` section with synopsis, examples, exit codes.

### 5B. Library README

**File**: `darkmatter/lib/README.md` — add `FileTree` to the public API listing alongside `ReferenceGraph`.

### 5C. Inline rustdoc

All new public types follow the rustdoc conventions from CLAUDE.md (no H1, H2 sections, Examples/Errors/Notes order).

### 5D. Skill update

**File**: `.claude/skills/darkmatter/SKILL.md` — mention FileTree and `md graph`.

---

## Phase Dependencies

```
Phase 1 (Graph Metadata)
    │
    ▼
Phase 2 (FileTree Component)
    │
    ├──▶ Phase 4A/4B (model + graph unit tests) — can start immediately
    │
    ▼
Phase 3 (CLI Integration)
    │
    ├──▶ Phase 4C (rendering tests) — can start after Phase 2
    ├──▶ Phase 4D (integration tests) — can start after Phase 2
    ▼
Phase 4E (CLI tests) — requires Phase 3
    │
    ▼
Phase 5 (Documentation)
```

---

## Critical Files Summary

| File | Action | Phase |
|------|--------|-------|
| `darkmatter/lib/src/markdown/reference/types.rs` | Extend `ReferenceInsertion`, add `ReferenceInsertionContext`, add `is_followable_transclusion()` | 1 |
| `darkmatter/lib/src/markdown/reference/graph.rs` | Add heading index helpers, populate context in `build_node()` at 3 insertion sites | 1 |
| `darkmatter/lib/src/markdown/reference/mod.rs` | Add `pub mod file_tree;` | 2 |
| `darkmatter/lib/src/markdown/mod.rs` | Add `FileTree`, `FileTreeError` to re-exports | 2 |
| `darkmatter/lib/src/markdown/reference/file_tree/mod.rs` | **NEW** — FileTree struct, FileTreeError, lazy build | 2 |
| `darkmatter/lib/src/markdown/reference/file_tree/model.rs` | **NEW** — view model types, graph-to-model transform | 2 |
| `darkmatter/lib/src/markdown/reference/file_tree/render.rs` | **NEW** — Renderable impl, terminal rendering | 2 |
| `darkmatter/lib/src/markdown/reference/file_tree/icons.rs` | **NEW** — icon mapping with nerd font fallback | 2 |
| `darkmatter/cli/src/args.rs` | Add `Graph` variant to `Command` enum | 3 |
| `darkmatter/cli/src/commands.rs` | Add `run_graph()` handler, dispatch | 3 |
| `darkmatter/lib/tests/reference_integration.rs` | Add section context + FileTree integration tests | 4 |
| `darkmatter/cli/tests/cli.rs` | Add `md graph` CLI tests | 4 |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `ReferenceInsertion` breaking change | Only constructed in `graph.rs` (3 sites) + tests; `Default` on context makes migration trivial |
| Line numbering mismatch between `toc().all_headings()` and `directive.line` | Both are 1-indexed from pulldown-cmark; verify in Phase 4B tests |
| Circular transclusion in follow mode | `ReferenceGraph` already enforces depth/cycle limits; `node_by_id()` returns `None` for pruned nodes → leaf edge |
| Nested rendering width exhaustion | Track available width per nesting level; truncate content, never connectors |
| Exit code 2 bypasses eyre error handling | Acceptable — `std::process::exit(2)` is called after successful rendering, only for distinct validation exit code |

---

## Existing Code to Reuse

| What | Where | How |
|------|-------|-----|
| Reference graph building | `Markdown::reference_graph()` | Direct call in `ensure_built()` |
| Validation pipeline | `Markdown::validate_references()` | Direct call in `ensure_built()` |
| Heading extraction | `Markdown::toc().all_headings()` | In `build_heading_index()` for section context |
| Tree drawing constants | `biscuit_terminal::components::filesystem::tree_chars` | Import in render.rs |
| Nerd font detection | `Terminal::is_nerd_font` | Passed to icon selection |
| File resolution | `biscuit_file::FileReference` via `resolve_file_path()` | In CLI handler |
| Markdown file completion | `complete_markdown_files()` | On `Graph` input arg |
| Test helpers | `darkmatter::testing::strip_ansi_codes()`, `Terminal::new_optimistic()` | In render tests |
| Error formatting | `Prose::new()` + `Terminal::default()` in `main.rs` | Existing error path handles exit code 1 |
