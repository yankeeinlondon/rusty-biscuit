# Darkmatter Type Safety, DRY & Test Coverage Review

**Date:** 2026-04-03
**Scope:** `darkmatter/lib/` + `darkmatter/cli/`
**Focus:** Type safety improvements, DRY opportunities, test coverage gaps

---

## Summary

The darkmatter library and CLI are well-structured with strong foundational types (`ComposeOperation` enum, `HeadingLevel` newtype, `ComposeOperationSet` bitset, builder-pattern `ComposeOptions`). The review found **9 type-safety improvements**, **8 DRY opportunities**, and **15 test coverage gaps**. Items are ordered by impact -- the first few in each category would meaningfully improve correctness and maintainability.

---

## 1. Type Safety Improvements

### 1.1 [High] Unify `HeadingLevel` across the codebase

`normalize/types.rs` defines an excellent `HeadingLevel` newtype with validated construction (1-6), `deeper()`, `shallower()`, `delta_to()`, and const variants `H1..H6`. However, heading levels elsewhere are raw `u8` or `HashSet<u8>`:

| File | Current | Should Be |
|------|---------|-----------|
| `toc/types.rs` | `level: u8` on `MarkdownTocNode` | `HeadingLevel` |
| `toc_linking/types.rs` | heading level fields | `HeadingLevel` |
| `delta/types.rs` | `level: u8` on section types | `HeadingLevel` |
| `reference/types.rs` | `section_heading_level: Option<u8>` | `Option<HeadingLevel>` |
| `compose/page_blocks/types.rs` | likely `u8` | `HeadingLevel` |

**Impact:** Eliminates an entire class of bugs where invalid heading levels (0, 7, 255) silently propagate. The newtype is zero-cost at runtime. The ergonomics cost is minor -- `.as_u8()` where raw integers are needed.

**Recommendation:** Add `HeadingLevel` to a shared location (already in `normalize/types.rs` which is reasonable) and migrate all `u8` heading-level fields. The `From<pulldown_cmark::HeadingLevel>` impl can be added alongside.

### 1.2 [High] Replace `TransclusionRefOptions` stringly-typed fields

`reference/types.rs:497-508`:

```rust
pub struct TransclusionRefOptions {
    pub when_expr: Option<String>,
    pub replace: Option<String>,       // "parent-wins" or JSON map
    pub quotation: Option<String>,
    pub disclosure: Option<String>,
    pub exclude: Vec<String>,
}
```

The `replace` field holds either the literal string `"parent-wins"` or a JSON-serialized map. This should be an enum:

```rust
pub enum ReplacePolicy {
    InheritDefault,
    ParentWins,
    OneOff(serde_json::Map<String, serde_json::Value>),
}
```

**Impact:** The current `replace` field requires every consumer to parse and interpret the string. An enum makes invalid states unrepresentable. The conversion already exists in `block_options_to_ref_options()` (reference/mod.rs:454-466) which does exactly this match -- it just throws the result away into a `String`.

### 1.3 [High] Use `PathBuf` for local file path fields

`reference/types.rs:89`:

```rust
pub enum ReferenceTarget {
    LocalPath { raw: String },  // Should be PathBuf
    // ...
}
```

Similarly, `TransclusionRef.raw_target: String` and `resolved_target: Option<String>` should be `PathBuf` / `Option<PathBuf>` for local paths.

**Impact:** Makes the intent clear, enables use of `Path` methods (`extension()`, `file_name()`, `parent()`) without conversion, and prevents accidentally mixing URLs and file paths.

**Trade-off:** Requires matching on `ReferenceTarget` variants to know which is a path vs URL, but the enum already provides that discrimination.

### 1.4 [Medium] Extract `ComposePerfMetric.name` to an enum

`compose/types.rs:1228`:

```rust
pub struct ComposePerfMetric {
    pub name: String,  // "text replacement", "interpolation", etc.
    pub elapsed: Duration,
    pub calls: usize,
}
```

The `name` field is a free-form string but always matches a known set of pipeline stage names. A `ComposeStage` enum would:

1. Prevent typos in metric aggregation (`merge()` matches by string equality)
2. Allow exhaustive matching in the CLI perf report formatter
3. Enable compile-time verification that all stages are reported

**Recommendation:** Derive `ComposeStage` from `ComposeOperation` (or make it a separate small enum for the subset that reports perf). Add `Display` for human-readable names.

### 1.5 [Medium] Node ID newtype for reference graph

Throughout `reference/graph.rs` and `reference/types.rs`, node identifiers are bare `String`:

```rust
pub struct ReferenceGraphNode {
    pub node_id: String,
    // ...
}
pub struct ReferenceInsertion {
    pub child_node_id: String,
    // ...
}
```

A newtype would prevent accidental confusion with other strings:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);
```

**Impact:** Lightweight, zero-cost. Makes graph traversal code self-documenting.

### 1.6 [Medium] CLI `Set.value` dual-interpretation

`cli/src/args.rs` -- The `Set.value: String` is parsed as "JSON if valid, otherwise treated as string". This means `true` becomes a boolean, `42` becomes a number, but `"hello"` stays a string. A `--type` flag (or `--as-string` flag to force string interpretation) would eliminate ambiguity.

**Impact:** Low effort, prevents user surprise. The current behavior is documented but implicit.

### 1.7 [Medium] CLI `Edit.file` should be `PathBuf` not `String`

`cli/src/args.rs` -- `Edit.file: String` supports `@`, `!`, `vault:` prefixes via `FileReference`, but clap treats it as a raw string. Using `PathBuf` with a custom parser that handles the prefixes would provide better validation and tab-completion.

### 1.8 [Low] Reference record `attributes` as typed structs

All `ReferenceRecord` instances store attributes in `serde_json::Map<String, serde_json::Value>`. This means attribute access is stringly-typed:

```rust
record.attributes.get("display").and_then(|v| v.as_str())
```

A typed attribute enum per `ReferenceKind` would be more robust, but the ergonomic cost is significant -- it would require refactoring the entire extraction pipeline. **Defer unless attribute access becomes a bug source.**

### 1.9 [Low] `ComposeOptions` has ~40 public fields

`compose/types.rs:268-442` -- `ComposeOptions` exposes all fields as `pub` with builder-pattern methods. The builder methods are the intended API, but nothing prevents direct field mutation that skips validation (e.g., `indent_size` can be set to 0 via direct assignment, but `with_indent_size()` enforces `.max(1)`).

**Recommendation:** Make fields `pub(crate)` and rely solely on the builder methods for external consumers. This is a breaking change but strengthens the invariant that options are always valid.

---

## 2. DRY Opportunities

### 2.1 [High] Duplicated `build_transclusion_graph` / `build_reference_graph`

`reference/graph.rs:59-110` -- These two functions are **structurally identical**. They differ in exactly one argument: `false` vs `true` passed to `build_node()`:

```rust
// build_transclusion_graph (line 75):
let (root, all_nodes) = build_node(md, &source, options, &mut runtime, false)?;

// build_reference_graph (line 102):
let (root, all_nodes) = build_node(md, &source, options, &mut runtime, true)?;
```

The 12-line preamble (runtime construction, source extraction, root seeding) and 6-line epilogue (transclusion exit, graph construction) are copy-pasted.

**Recommendation:** Extract a shared inner function with an `extract_references: bool` parameter:

```rust
fn build_graph_inner(md: &Markdown, options: &ReferenceGraphOptions, extract_references: bool) 
    -> MarkdownResult<ReferenceGraph> { ... }
```

### 2.2 [High] Duplicated `wrap_to_width` and `filter_with_context` in diff renderers

`diff/visual/side_by_side.rs:478-527` and `diff/visual/unified.rs:323-372` contain **byte-for-byte identical** implementations of `wrap_to_width()`. The `filter_with_context()` function is also duplicated (side_by_side.rs:134 and unified.rs:115).

**Recommendation:** Extract both functions to `diff/visual/mod.rs` or a new `diff/visual/utils.rs` module.

### 2.3 [High] Duplicated link/image extraction in `local.rs`

`reference/local.rs` -- `extract_markdown_links()` (lines 11-84) and `extract_markdown_images()` (lines 87-160) are **structurally identical**. Same parser setup, same accumulation pattern, same attribute construction. The only differences are the tag type (`Tag::Link` vs `Tag::Image`) and attribute key names (`display` vs `alt`).

**Recommendation:** Create a single generic extractor that parameterizes on tag type, or a combined pass that extracts both in one iteration:

```rust
fn extract_inline_references(content: &str, source: &ComposeSource) -> Vec<ReferenceRecord> {
    // Single pass extracting both links and images
}
```

This halves the parsing cost for documents with both link and image references.

### 2.4 [High] Repeated graph filter methods in `mod.rs`

`reference/mod.rs:253-319` -- Five methods follow the exact same pattern:

```rust
pub fn inline_css_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<InlineCssBlock>> {
    let refs = self.composed_references(options)?;
    Ok(refs.records.into_iter()
        .filter(|r| r.kind == ReferenceKind::InlineCss)
        .map(InlineCssBlock::from)
        .collect())
}
```

Each of `inline_css_graph`, `css_import_graph`, `inline_script_graph`, `script_import_graph`, `font_import_graph` is the same filter-map-collect with a different `ReferenceKind` and target type.

**Recommendation:** Add a generic helper to `ReferenceSet`:

```rust
impl ReferenceSet {
    pub fn filter_map<T: From<ReferenceRecord>>(self, kind: ReferenceKind) -> Vec<T> {
        self.records.into_iter()
            .filter(|r| r.kind == kind)
            .map(T::from)
            .collect()
    }
}
```

Then each method becomes one line:

```rust
pub fn inline_css_graph(&self, options: ReferenceGraphOptions) -> MarkdownResult<Vec<InlineCssBlock>> {
    Ok(self.composed_references(options)?.filter_map(ReferenceKind::InlineCss))
}
```

### 2.5 [Medium] Theme detection duplication in CLI

`cli/src/commands.rs:282-286` (in `run_render`) and `commands.rs:584-588` (in `run_compose`):

```rust
let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
let code_theme = cli.code_theme.unwrap_or_else(|| detect_code_theme(prose_theme));
let color_mode = detect_color_mode();
```

This identical 3-line block appears whenever output is rendered.

**Recommendation:** Extract to a `ResolvedTheme` struct with a `from_cli(cli: &Cli)` method.

### 2.6 [Medium] Repeated ANSI escape code constants in diff renderers

`side_by_side.rs:17-28` and `unified.rs:18-32` both define:

```rust
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const BG_REMOVED: &str = "\x1b[48;5;52m";
const BG_ADDED: &str = "\x1b[48;5;22m";
const BG_CHANGED_DEL: &str = "\x1b[48;5;88m";
const BG_CHANGED_ADD: &str = "\x1b[48;5;28m";
```

**Recommendation:** Move to `diff/visual/mod.rs` as shared constants.

### 2.7 [Medium] Path resolution pattern duplicated 3 times

The pattern `FileReference::new() + add_magic_path() + resolve_relative()` appears in:
- `reference/graph.rs` (node resolution)
- `reference/validate.rs` (validation targets)
- `reference/mod.rs` (local extraction)

**Recommendation:** Extract a `resolve_local_reference(target: &str, source: &ComposeSource, options: &ComposeOptions) -> PathBuf` helper.

### 2.8 [Low] CLI test helper repetition

`cli/tests/cli.rs` -- 20+ tests create temp files with the same pattern:

```rust
let mut tmp = tempfile::NamedTempFile::new().unwrap();
writeln!(tmp, "...").unwrap();
```

**Recommendation:** A `fn md_file(content: &str) -> NamedTempFile` helper would reduce boilerplate.

---

## 3. Test Coverage Gaps

### 3.1 Untested Subcommands

| Subcommand | CLI Integration Tests | Risk |
|------------|----------------------|------|
| `edit` | **0 tests** | High -- ~100 lines of editor resolution, file creation, wait-flag injection, post-edit validation |
| `rm` | **0 tests** | Medium -- frontmatter deletion with verbose/JSON output |

### 3.2 Untested Top-Level Flags

| Flag | Tests | Impact |
|------|-------|--------|
| `--mermaid` | 0 | Mermaid image rendering never tested in CLI |
| `--line-numbers` | 0 | Line number injection in code blocks untested |
| `--theme` / `--code-theme` | 0 | Theme selection affects output but never verified |
| `--list-themes` | 0 | No test that theme listing produces output |
| `--completions` | 0 | Shell completion output untested |

### 3.3 Library Coverage Gaps

| Module | Tests | Notes |
|--------|-------|-------|
| `cache/manifest.rs` | **0 tests** | `DocumentSnapshotManifest`, `ComposedDocumentManifest`, `OperationResultManifest` with `is_fresh()`, `touch()`, `is_expired()` methods have zero coverage |
| `render_terminal.rs` | 3 tests (fallback/error only) | The main `render_for_terminal()` path uses stdout printing, making it hard to test. Consider making it return a `String` for testability |
| `output/html.rs` | 0 unit tests | 1353 lines of HTML rendering with no isolated unit tests |

### 3.4 Missing Integration Scenarios

1. **`run_render` TTY path** -- CLI integration tests use piped stdout (non-TTY), so `OutputFormat::Auto` never exercises the terminal-rendering branch. A pseudo-TTY test or a `--force-tty` test flag would cover this.

2. **`validate --remote`** -- Remote URL validation is untested. A `wiremock` test (already used elsewhere in the monorepo) would be appropriate.

3. **`validate --timeout`** -- The timeout parameter passthrough is untested.

4. **`compose --compact` / `--loose`** -- No CLI test for compose with list spacing flags.

5. **`validate --show-all`** -- The show-all flag is untested.

6. **`graph --json`** -- JSON output for the graph command has no integration test.

7. **`--save` + `--verbose`** -- Combined save-and-verbose output is untested.

### 3.5 Test Quality Observations

**Good:** The existing tests are well-structured:
- 78 CLI integration tests via `assert_cmd`
- 31 library integration tests
- 2000+ total `#[test]` functions across the crate
- Inline tests in `local.rs`, `types.rs`, `normalize/types.rs` co-located with implementation
- Comprehensive cache tests (hashing: 17, operation: 10, runtime: 13, store: 8)

**Gaps in test isolation:** Some tests depend on system state (terminal detection, environment variables). The `serial_test` crate is available but not consistently used where tests modify env vars.

---

## 4. Priority Matrix

### Quick Wins (1-2 hours each, high impact)

| Item | Category | Effort | Impact |
|------|----------|--------|--------|
| 2.1 Merge graph builders | DRY | Low | Removes 40 lines of duplication |
| 2.6 Shared ANSI constants | DRY | Low | Removes drift risk |
| 2.2 Shared diff utilities | DRY | Low | Removes 100 lines of duplication |
| 1.5 Node ID newtype | Type Safety | Low | Self-documenting graph code |
| 1.2 `ReplacePolicy` enum | Type Safety | Low | Eliminates stringly-typed matching |

### Medium Effort (half day each)

| Item | Category | Effort | Impact |
|------|----------|--------|--------|
| 1.1 Unify `HeadingLevel` | Type Safety | Medium | Cross-cutting correctness |
| 2.3 Combined link/image extraction | DRY | Medium | Halves parsing cost |
| 2.4 Generic `filter_map` on `ReferenceSet` | DRY | Medium | Simplifies 5 methods |
| 2.5 Theme detection extraction | DRY | Medium | CLI maintainability |
| 3.1 Tests for `edit`/`rm` subcommands | Tests | Medium | Coverage for untested paths |

### Larger Effort (1+ days each)

| Item | Category | Effort | Impact |
|------|----------|--------|--------|
| 1.3 `PathBuf` for local paths | Type Safety | High | Cross-module refactor |
| 1.4 `ComposeStage` enum | Type Safety | Medium | Metric safety |
| 1.9 Private `ComposeOptions` fields | Type Safety | High | Breaking API change |
| 3.3 HTML renderer unit tests | Tests | High | 1353-line file untested |
| 3.4 TTY render path testing | Tests | High | Requires test infrastructure |

---

## 5. Files Reviewed

| File | Lines | Key Findings |
|------|-------|--------------|
| `lib/src/lib.rs` | -- | Module structure |
| `lib/src/markdown/types.rs` | 78 | `MarkdownError` enum (good), `FrontmatterMap` alias |
| `lib/src/markdown/normalize/types.rs` | 596 | `HeadingLevel` newtype (excellent), `NormalizationReport` |
| `lib/src/markdown/compose/types.rs` | 1500+ | `ComposeOperation` enum, `ComposeOperationSet` bitset (excellent), `ComposeOptions` with 40 pub fields |
| `lib/src/markdown/reference/types.rs` | 770 | `ReferenceKind`, `ReferenceTarget`, `ReferenceSyntax` enums (good), stringly-typed `TransclusionRefOptions` |
| `lib/src/markdown/reference/graph.rs` | 1141 | Duplicated `build_transclusion_graph`/`build_reference_graph` |
| `lib/src/markdown/reference/local.rs` | 289 | Duplicated link/image extraction |
| `lib/src/markdown/reference/mod.rs` | 467 | Repeated graph filter methods |
| `lib/src/diff/visual/side_by_side.rs` | 762 | Duplicated `wrap_to_width`, `filter_with_context`, ANSI constants |
| `lib/src/diff/visual/unified.rs` | 566 | Duplicated `wrap_to_width`, `filter_with_context`, ANSI constants |
| `cli/src/commands.rs` | 2111 | Theme detection duplication, 2 untested subcommands |
| `cli/src/args.rs` | 653 | Strong `ValueEnum` usage, `Set.value` dual-interpretation |
| `cli/tests/cli.rs` | 1801 | 78 tests, no `edit`/`rm`/`--mermaid`/`--theme` coverage |
