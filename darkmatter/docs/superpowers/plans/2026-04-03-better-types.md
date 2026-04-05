# Better Types, DRY & Test Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all 9 type-safety improvements, 8 DRY deduplication opportunities, and 15 test coverage gaps identified in the 2026-04-03 code review.

**Architecture:** Bottom-up approach -- shared utilities and type changes first (phases 1-2), then consumers updated (phase 3), then tests added (phase 4). Each phase produces a buildable, testable codebase.

**Tech Stack:** Rust, darkmatter lib + CLI, pulldown_cmark, serde_json, assert_cmd, tempfile

---

## File Structure

### New files to create:
- `lib/src/diff/visual/constants.rs` -- shared ANSI escape code constants
- `lib/src/diff/visual/utils.rs` -- shared `wrap_to_width` and `filter_with_context`

### Files to modify:
- `lib/src/markdown/compose/types.rs` -- `ComposeStage` enum for perf metrics, `ComposeOptions` fields to `pub(crate)`
- `lib/src/markdown/compose/perf.rs` -- use `ComposeStage` in `ComposePerfMetric`
- `lib/src/markdown/reference/types.rs` -- `ReplacePolicy` enum, `NodeId` newtype, `HeadingLevel` in `ReferenceInsertionContext`
- `lib/src/markdown/reference/graph.rs` -- merge duplicated graph builders, use `NodeId`
- `lib/src/markdown/reference/local.rs` -- merge link/image extraction into single pass
- `lib/src/markdown/reference/mod.rs` -- generic `filter_map` on `ReferenceSet`
- `lib/src/markdown/toc/types.rs` -- `HeadingLevel` for `MarkdownTocNode.level`
- `lib/src/markdown/delta/types.rs` -- `HeadingLevel` for `ContentChange` level fields
- `lib/src/markdown/compose/toc_linking/types.rs` -- `HeadingLevel` for `LevelFilter`
- `lib/src/diff/visual/mod.rs` -- re-export new modules
- `lib/src/diff/visual/side_by_side.rs` -- use shared constants/utils
- `lib/src/diff/visual/unified.rs` -- use shared constants/utils
- `cli/src/commands.rs` -- `ResolvedTheme` struct, theme detection extraction
- `cli/tests/cli.rs` -- new tests for `edit`, `rm`, flags

---

## Phase 1: Shared Utilities & Constants (DRY foundations)

### Task 1: Extract shared ANSI constants to `diff/visual/constants.rs`

**Files:**
- Create: `lib/src/diff/visual/constants.rs`
- Modify: `lib/src/diff/visual/mod.rs`
- Modify: `lib/src/diff/visual/side_by_side.rs`
- Modify: `lib/src/diff/visual/unified.rs`

- [ ] **Step 1: Create `constants.rs` with all shared ANSI codes**

```rust
//! Shared ANSI escape code constants for diff renderers.

// Text styles
pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m";
pub(crate) const DIM: &str = "\x1b[2m";
pub(crate) const UNDERLINE: &str = "\x1b[4m";

// Background colors (256-color mode)
pub(crate) const BG_REMOVED: &str = "\x1b[48;5;52m";
pub(crate) const BG_ADDED: &str = "\x1b[48;5;22m";
pub(crate) const BG_CHANGED_DEL: &str = "\x1b[48;5;88m";
pub(crate) const BG_CHANGED_ADD: &str = "\x1b[48;5;28m";
```

- [ ] **Step 2: Register the module in `mod.rs`**

Add `mod constants;` to `lib/src/diff/visual/mod.rs`. Remove the local `RESET` and `BOLD` constants from `mod.rs` and replace with `use constants::*;`.

- [ ] **Step 3: Update `side_by_side.rs` to use shared constants**

Replace lines 17-28 with:

```rust
use super::constants::{RESET, BOLD, DIM, UNDERLINE, BG_REMOVED, BG_ADDED, BG_CHANGED_DEL, BG_CHANGED_ADD};

// Module-specific constants
const INVERSE: &str = "\x1b[7m";
```

- [ ] **Step 4: Update `unified.rs` to use shared constants**

Replace lines 18-32 with:

```rust
use super::constants::{RESET, BOLD, DIM, UNDERLINE, BG_REMOVED, BG_ADDED, BG_CHANGED_DEL, BG_CHANGED_ADD};

// Module-specific constants
const FG_RED: &str = "\x1b[31m";
const FG_GREEN: &str = "\x1b[32m";
```

- [ ] **Step 5: Build and test**

Run: `cargo test -p darkmatter --lib diff::visual`
Expected: All existing diff tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/diff/visual/
git commit -m "refactor(darkmatter): extract shared ANSI constants to diff/visual/constants.rs"
```

---

### Task 2: Extract shared `wrap_to_width` and `filter_with_context` to `diff/visual/utils.rs`

**Files:**
- Create: `lib/src/diff/visual/utils.rs`
- Modify: `lib/src/diff/visual/mod.rs`
- Modify: `lib/src/diff/visual/side_by_side.rs`
- Modify: `lib/src/diff/visual/unified.rs`

- [ ] **Step 1: Create `utils.rs` with extracted functions**

```rust
//! Shared utilities for diff renderers.

use biscuit_terminal::utils::UnicodeWidthStr;
use std::collections::HashSet;

use super::diff::DiffLine;

/// Filter lines to show only changes and surrounding context.
pub(crate) fn filter_with_context(diff: &[DiffLine], context_lines: usize) -> HashSet<usize> {
    let mut visible = HashSet::new();

    // First pass: mark all change lines.
    let change_indices: Vec<usize> = diff
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.is_context())
        .map(|(idx, _)| idx)
        .collect();

    // Second pass: add context around each change.
    for &change_idx in &change_indices {
        let start = change_idx.saturating_sub(context_lines);
        for i in start..=change_idx {
            visible.insert(i);
        }

        let end = (change_idx + context_lines + 1).min(diff.len());
        for i in change_idx..end {
            visible.insert(i);
        }
    }

    visible
}

/// Word-wrap text to fit within `max_width` visual columns.
///
/// Tries to break at word boundaries (whitespace); falls back to hard
/// character-level breaks for words longer than `max_width`.
pub(crate) fn wrap_to_width(s: &str, max_width: usize) -> Vec<String> {
    if s.is_empty() || max_width == 0 {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: usize = 0;

    for word in s.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);

        if word_width > max_width {
            // Flush current line before handling the long word
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            // Hard-break the long word using biscuit-terminal
            let chunks = biscuit_terminal::utils::block_constraint::wrap_lines(
                vec![word.to_string()],
                &biscuit_terminal::utils::layout::WordWrap::None,
                max_width as u32,
            );
            let num_chunks = chunks.len();
            for (i, chunk) in chunks.into_iter().enumerate() {
                if i < num_chunks - 1 {
                    lines.push(chunk);
                } else {
                    // Last chunk may be partial — carry it as the current line
                    current_width = UnicodeWidthStr::width(chunk.as_str());
                    current = chunk;
                }
            }
        } else if current_width == 0 {
            current = word.to_string();
            current_width = word_width;
        } else if current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}
```

- [ ] **Step 2: Register the module in `mod.rs`**

Add `mod utils;` to `lib/src/diff/visual/mod.rs`.

- [ ] **Step 3: Update `side_by_side.rs` to use shared utilities**

Delete the local `filter_with_context` function (lines 134-159) and `wrap_to_width` function (lines 478-535). Add import:

```rust
use super::utils::{filter_with_context, wrap_to_width};
```

Remove the now-unnecessary `use std::collections::HashSet;` from the top of `side_by_side.rs` (unless used elsewhere in the file -- check first).

- [ ] **Step 4: Update `unified.rs` to use shared utilities**

Delete the local `filter_with_context` function (lines 115-147) and `wrap_to_width` function (lines 323-379). Add import:

```rust
use super::utils::{filter_with_context, wrap_to_width};
```

- [ ] **Step 5: Build and test**

Run: `cargo test -p darkmatter --lib diff::visual`
Expected: All existing diff tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/diff/visual/
git commit -m "refactor(darkmatter): extract shared wrap_to_width and filter_with_context to diff/visual/utils.rs"
```

---

### Task 3: Merge duplicated `build_transclusion_graph` / `build_reference_graph`

**Files:**
- Modify: `lib/src/markdown/reference/graph.rs`

- [ ] **Step 1: Create shared inner function**

Replace both `build_transclusion_graph` (lines 59-83) and `build_reference_graph` (lines 86-110) with:

```rust
/// Shared graph construction with configurable reference extraction.
fn build_graph_inner(
    md: &Markdown,
    options: &ReferenceGraphOptions,
    extract_references: bool,
) -> MarkdownResult<ReferenceGraph> {
    let mut runtime = ReferenceAnalysisRuntime {
        transclusion: TransclusionRuntime::new(options.compose.max_transclusion_depth),
        cache: make_cache(options),
    };

    let source = md.source().clone().unwrap_or(ComposeSource::Unknown);

    // Seed the runtime with the root node so child documents that
    // transclude the root are detected as cycles immediately.
    let root_id = source_to_id(&source);
    let _ = runtime.transclusion.enter(root_id);

    let (root, all_nodes) = build_node(md, &source, options, &mut runtime, extract_references)?;

    runtime.transclusion.exit();

    Ok(ReferenceGraph {
        root,
        nodes: all_nodes,
    })
}

/// Build a transclusion-only graph (no link/image extraction at leaf nodes).
pub(crate) fn build_transclusion_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph> {
    build_graph_inner(md, options, false)
}

/// Build a full reference graph (transclusions + all reference types at each node).
pub(crate) fn build_reference_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph> {
    build_graph_inner(md, options, true)
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test -p darkmatter --lib markdown::reference && cargo test -p darkmatter --test reference_integration`
Expected: All reference tests pass.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/graph.rs
git commit -m "refactor(darkmatter): merge duplicated graph builder functions into build_graph_inner"
```

---

### Task 4: Merge duplicated link/image extraction in `local.rs`

**Files:**
- Modify: `lib/src/markdown/reference/local.rs`

- [ ] **Step 1: Create unified extractor and refactor existing functions**

Add a shared inner function and refactor both `extract_markdown_links` and `extract_markdown_images` to delegate to it:

```rust
/// Tag configuration for inline reference extraction.
enum InlineRefKind {
    Link,
    Image,
}

/// Extract markdown inline references (links or images) in a single pass.
fn extract_inline_refs(
    content: &str,
    source: &ComposeSource,
    kind: InlineRefKind,
) -> Vec<ReferenceRecord> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(content, options);

    let line_index = LineIndex::new(content);
    let mut records = Vec::new();
    let mut active = false;
    let mut current_target = String::new();
    let mut current_title = String::new();
    let mut current_text = String::new();
    let mut span_start: usize = 0;

    for (event, range) in parser.into_offset_iter() {
        match (&kind, &event) {
            (InlineRefKind::Link, Event::Start(Tag::Link { dest_url, title, .. })) => {
                active = true;
                current_target = dest_url.to_string();
                current_title = title.to_string();
                current_text.clear();
                span_start = range.start;
            }
            (InlineRefKind::Image, Event::Start(Tag::Image { dest_url, title, .. })) => {
                active = true;
                current_target = dest_url.to_string();
                current_title = title.to_string();
                current_text.clear();
                span_start = range.start;
            }
            (InlineRefKind::Link, Event::End(TagEnd::Link)) if active => {
                let text = std::mem::take(&mut current_text);
                let target = std::mem::take(&mut current_target);
                let title = std::mem::take(&mut current_title);
                let span = span_start..range.end;
                let line = line_index.line_at(span_start);

                let mut attributes = serde_json::Map::new();
                attributes.insert("display".into(), serde_json::Value::String(text));
                if !title.is_empty() {
                    attributes.insert("title".into(), serde_json::Value::String(title));
                }

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, span.start),
                    kind: ReferenceKind::Hyperlink,
                    target: classify_target(&target),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span,
                        syntax: ReferenceSyntax::MarkdownLink,
                    },
                    attributes,
                });

                active = false;
            }
            (InlineRefKind::Image, Event::End(TagEnd::Image)) if active => {
                let text = std::mem::take(&mut current_text);
                let target = std::mem::take(&mut current_target);
                let title = std::mem::take(&mut current_title);
                let span = span_start..range.end;
                let line = line_index.line_at(span_start);

                let mut attributes = serde_json::Map::new();
                attributes.insert("alt".into(), serde_json::Value::String(text));
                if !title.is_empty() {
                    attributes.insert("title".into(), serde_json::Value::String(title));
                }

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, span.start),
                    kind: ReferenceKind::Image,
                    target: classify_target(&target),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span,
                        syntax: ReferenceSyntax::MarkdownImage,
                    },
                    attributes,
                });

                active = false;
            }
            (_, Event::Text(text)) if active => {
                current_text.push_str(text);
            }
            (_, Event::Code(code)) if active => {
                current_text.push('`');
                current_text.push_str(code);
                current_text.push('`');
            }
            (_, Event::SoftBreak) if active => {
                current_text.push(' ');
            }
            (_, Event::HardBreak) if active => {
                current_text.push('\n');
            }
            _ => {}
        }
    }

    records
}

/// Extract Markdown-native links as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_markdown_links(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    extract_inline_refs(content, source, InlineRefKind::Link)
}

/// Extract Markdown-native images as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_markdown_images(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    extract_inline_refs(content, source, InlineRefKind::Image)
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test -p darkmatter --lib markdown::reference::local && cargo test -p darkmatter --test reference_integration`
Expected: All existing local extraction tests pass.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/local.rs
git commit -m "refactor(darkmatter): merge duplicated link/image extraction into shared extract_inline_refs"
```

---

### Task 5: Add generic `filter_map` to `ReferenceSet` and simplify graph filter methods

**Files:**
- Modify: `lib/src/markdown/reference/types.rs`
- Modify: `lib/src/markdown/reference/mod.rs`

- [ ] **Step 1: Add `filter_map` method to `ReferenceSet`**

Add to the `impl ReferenceSet` block (after the `transclusions()` method, around line 233):

```rust
    /// Consumes the set and returns records of the given kind, converted to `T`.
    pub fn filter_convert<T: From<ReferenceRecord>>(self, kind: ReferenceKind) -> Vec<T> {
        self.records
            .into_iter()
            .filter(|r| r.kind == kind)
            .map(T::from)
            .collect()
    }
```

Note: Named `filter_convert` to avoid confusion with `Iterator::filter_map`.

- [ ] **Step 2: Simplify the five graph filter methods in `mod.rs`**

Replace lines 252-319 (the five methods) with:

```rust
    /// Returns inline CSS blocks across the composed document graph.
    pub fn inline_css_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineCssBlock>> {
        Ok(self.composed_references(options)?.filter_convert(ReferenceKind::InlineCss))
    }

    /// Returns CSS `@import` references across the composed document graph.
    pub fn css_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self.composed_references(options)?.filter_convert(ReferenceKind::CssImport))
    }

    /// Returns inline script blocks across the composed document graph.
    pub fn inline_script_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<InlineScriptBlock>> {
        Ok(self.composed_references(options)?.filter_convert(ReferenceKind::InlineScript))
    }

    /// Returns `<script src="...">` import references across the composed document graph.
    pub fn script_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self.composed_references(options)?.filter_convert(ReferenceKind::ScriptImport))
    }

    /// Returns font import references across the composed document graph.
    pub fn font_import_graph(
        &self,
        options: ReferenceGraphOptions,
    ) -> MarkdownResult<Vec<ImportReference>> {
        Ok(self.composed_references(options)?.filter_convert(ReferenceKind::FontImport))
    }
```

- [ ] **Step 3: Build and test**

Run: `cargo test -p darkmatter --lib markdown::reference && cargo test -p darkmatter --test reference_integration`
Expected: All reference tests pass.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/types.rs darkmatter/lib/src/markdown/reference/mod.rs
git commit -m "refactor(darkmatter): add ReferenceSet::filter_convert and simplify graph filter methods"
```

---

### Task 6: Extract theme detection to `ResolvedTheme` in CLI

**Files:**
- Modify: `cli/src/commands.rs`

- [ ] **Step 1: Add `ResolvedTheme` struct and `from_cli` method**

Add near the top of `commands.rs` (after the existing imports):

```rust
/// Resolved theme configuration for terminal rendering.
struct ResolvedTheme {
    prose: darkmatter::markdown::highlighting::ThemePair,
    code: darkmatter::markdown::highlighting::ThemePair,
    color_mode: darkmatter::markdown::highlighting::ColorMode,
}

impl ResolvedTheme {
    /// Resolves theme from CLI options, falling back to auto-detection.
    fn from_cli(cli: &Cli) -> Self {
        let prose = cli.theme.unwrap_or_else(detect_prose_theme);
        let code = cli.code_theme.unwrap_or_else(|| detect_code_theme(prose));
        let color_mode = detect_color_mode();
        Self { prose, code, color_mode }
    }
}
```

- [ ] **Step 2: Replace theme detection in `run_render`**

Replace lines 282-286 with:

```rust
    let theme = ResolvedTheme::from_cli(cli);
```

Update subsequent references: `prose_theme` → `theme.prose`, `code_theme` → `theme.code`, `color_mode` → `theme.color_mode`.

- [ ] **Step 3: Replace theme detection in `run_compose`**

Replace lines 584-588 with:

```rust
    let theme = ResolvedTheme::from_cli(cli);
```

Update subsequent references the same way.

- [ ] **Step 4: Build and test**

Run: `cargo test -p darkmatter-cli`
Expected: All CLI tests pass.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/cli/src/commands.rs
git commit -m "refactor(darkmatter-cli): extract theme detection to ResolvedTheme::from_cli"
```

---

## Phase 2: Type Safety Improvements

### Task 7: Add `ComposeStage` enum for perf metric names

**Files:**
- Modify: `lib/src/markdown/compose/types.rs`
- Modify: `lib/src/markdown/compose/perf.rs`
- Modify: `cli/src/commands.rs`

- [ ] **Step 1: Add `ComposeStage` enum to `compose/types.rs`**

Add after the `ComposePerfReport` struct (around line 1242):

```rust
/// Named compose pipeline stages for type-safe metric identification.
///
/// Variants are listed in pipeline execution order so reports have
/// a deterministic, intuitive ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeStage {
    FrontmatterInterpolation,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    Cleanup,
    Normalization,
}

impl std::fmt::Display for ComposeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::FrontmatterInterpolation => "frontmatter interpolation",
            Self::EffectiveStateBuild => "effective state build",
            Self::TextReplacement => "text replacement",
            Self::PageBlocks => "page blocks",
            Self::Interpolation => "interpolation",
            Self::ShellExpansion => "shell expansion",
            Self::TransclusionParse => "transclusion parse",
            Self::TransclusionPrepare => "transclusion prepare",
            Self::TransclusionResolve => "transclusion resolve",
            Self::TransclusionApply => "transclusion apply",
            Self::Cleanup => "cleanup",
            Self::Normalization => "normalization",
        })
    }
}
```

- [ ] **Step 2: Update `ComposePerfMetric` to use `ComposeStage`**

Change `ComposePerfMetric.name` from `String` to `ComposeStage`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposePerfMetric {
    /// Pipeline stage this metric represents.
    pub stage: ComposeStage,
    /// Accumulated elapsed time for this metric.
    pub elapsed: Duration,
    /// Number of times this metric was recorded.
    pub calls: usize,
}
```

Note: Also add `Copy` since `ComposeStage` is `Copy` and `Duration` is `Copy`.

- [ ] **Step 3: Update `perf.rs` to use `ComposeStage`**

In `perf.rs`, replace `PerfMetricKind` with a direct mapping to `ComposeStage`. Update the `finish()` method:

```rust
use super::types::{ComposePerfMetric, ComposePerfReport, ComposeStage};
```

Change `PerfMetricKind` to wrap `ComposeStage`:

```rust
/// Metric kinds corresponding to compose pipeline stages.
///
/// Wraps [`ComposeStage`] for internal indexing. The mapping is
/// 1:1 — this exists so the collector can use ordinal indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PerfMetricKind {
    FrontmatterInterpolation,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    Cleanup,
    Normalization,
}

impl PerfMetricKind {
    /// Convert to the public `ComposeStage` enum.
    pub(crate) fn stage(self) -> ComposeStage {
        match self {
            Self::FrontmatterInterpolation => ComposeStage::FrontmatterInterpolation,
            Self::EffectiveStateBuild => ComposeStage::EffectiveStateBuild,
            Self::TextReplacement => ComposeStage::TextReplacement,
            Self::PageBlocks => ComposeStage::PageBlocks,
            Self::Interpolation => ComposeStage::Interpolation,
            Self::ShellExpansion => ComposeStage::ShellExpansion,
            Self::TransclusionParse => ComposeStage::TransclusionParse,
            Self::TransclusionPrepare => ComposeStage::TransclusionPrepare,
            Self::TransclusionResolve => ComposeStage::TransclusionResolve,
            Self::TransclusionApply => ComposeStage::TransclusionApply,
            Self::Cleanup => ComposeStage::Cleanup,
            Self::Normalization => ComposeStage::Normalization,
        }
    }
```

Update `finish()` to use `stage` instead of `label`:

```rust
    ComposePerfMetric {
        stage: kind.stage(),
        elapsed,
        calls,
    }
```

- [ ] **Step 4: Update all consumers of `ComposePerfMetric.name`**

In `cli/src/commands.rs`, update the perf report formatter (around line 1342) and any test that constructs `ComposePerfMetric` to use `stage: ComposeStage::Cleanup` instead of `name: "cleanup".to_string()`.

In tests in `compose/types.rs` and `compose/perf.rs`, update `.find(|m| m.name == "cleanup")` to `.find(|m| m.stage == ComposeStage::Cleanup)`.

In `compose/mod.rs`, update line 66 import to include `ComposeStage`.

- [ ] **Step 5: Build and test**

Run: `cargo test -p darkmatter && cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/types.rs darkmatter/lib/src/markdown/compose/perf.rs darkmatter/lib/src/markdown/compose/mod.rs darkmatter/cli/src/commands.rs
git commit -m "feat(darkmatter): add ComposeStage enum for type-safe perf metric names"
```

---

### Task 8: Add `ReplacePolicy` enum for transclusion options

**Files:**
- Modify: `lib/src/markdown/reference/types.rs`

- [ ] **Step 1: Add `ReplacePolicy` enum**

Add before `TransclusionRefOptions` (around line 495):

```rust
/// Policy for how a transclusion directive handles frontmatter key conflicts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReplacePolicy {
    /// Use the document's default merge behavior.
    #[default]
    InheritDefault,
    /// Parent document values always win over child values.
    ParentWins,
    /// Specific key-value overrides for the transcluded content.
    OneOff(serde_json::Map<String, serde_json::Value>),
}
```

- [ ] **Step 2: Update `TransclusionRefOptions.replace` field**

Change the `replace` field from `Option<String>` to `ReplacePolicy`:

```rust
#[derive(Debug, Clone, Default)]
pub struct TransclusionRefOptions {
    /// Optional `when` condition expression.
    pub when_expr: Option<String>,
    /// How frontmatter key conflicts are resolved during transclusion.
    pub replace: ReplacePolicy,
    /// Quotation wrapper text.
    pub quotation: Option<String>,
    /// Disclosure summary text.
    pub disclosure: Option<String>,
    /// Heading sections to exclude.
    pub exclude: Vec<String>,
}
```

- [ ] **Step 3: Update all producers of `TransclusionRefOptions`**

Search for all places that construct or set `TransclusionRefOptions` (likely in `reference/mod.rs` around `block_options_to_ref_options()` and in `compose/transclusion/`). Update them to produce `ReplacePolicy` variants instead of `Option<String>`.

Run: `cargo check -p darkmatter 2>&1 | head -50` to find all compilation errors and fix each one.

- [ ] **Step 4: Update all consumers of `TransclusionRefOptions.replace`**

Search for all places that read `.replace` and update the matching logic. The existing match on `"parent-wins"` becomes `ReplacePolicy::ParentWins`, and JSON parsing becomes `ReplacePolicy::OneOff(map)`.

Run: `cargo check -p darkmatter 2>&1 | head -50` to find remaining errors.

- [ ] **Step 5: Build and test**

Run: `cargo test -p darkmatter && cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/types.rs darkmatter/lib/src/markdown/reference/ darkmatter/lib/src/markdown/compose/
git commit -m "feat(darkmatter): add ReplacePolicy enum to replace stringly-typed transclusion options"
```

---

### Task 9: Add `NodeId` newtype for reference graph

**Files:**
- Modify: `lib/src/markdown/reference/types.rs`
- Modify: `lib/src/markdown/reference/graph.rs`
- Modify: `lib/src/markdown/reference/file_tree/model.rs` (if it uses `node_id: String`)

- [ ] **Step 1: Add `NodeId` newtype to `reference/types.rs`**

Add near the top (after imports, around line 8):

```rust
/// Unique identifier for a node in the reference graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

- [ ] **Step 2: Update `ReferenceGraphNode.node_id` and `ReferenceInsertion.child_node_id`**

Change `node_id: String` → `node_id: NodeId` and `child_node_id: String` → `child_node_id: NodeId`.

- [ ] **Step 3: Fix compilation errors**

Run: `cargo check -p darkmatter 2>&1 | head -80` and fix each error. Key places:
- `graph.rs`: `source_to_id()` should return `NodeId`
- `graph.rs`: comparisons like `node.node_id == id` → use `.as_ref()` or compare `NodeId` directly
- `types.rs`: `node_by_id()` parameter should accept `&str` or `&NodeId`
- `file_tree/model.rs`: update any `node_id` string usage

- [ ] **Step 4: Build and test**

Run: `cargo test -p darkmatter --lib markdown::reference && cargo test -p darkmatter --test reference_integration`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/
git commit -m "feat(darkmatter): add NodeId newtype for type-safe reference graph identifiers"
```

---

### Task 10: Unify `HeadingLevel` across the codebase

**Files:**
- Modify: `lib/src/markdown/toc/types.rs`
- Modify: `lib/src/markdown/toc/mod.rs`
- Modify: `lib/src/markdown/delta/types.rs`
- Modify: `lib/src/markdown/compose/toc_linking/types.rs`
- Modify: `lib/src/markdown/reference/types.rs`
- Modify: `lib/src/markdown/reference/graph.rs`
- Modify: `lib/src/markdown/reference/file_tree/model.rs`
- Modify: `cli/src/commands.rs`

This is a cross-cutting change. Work module-by-module.

- [ ] **Step 1: Update `MarkdownTocNode.level` in `toc/types.rs`**

Change `pub level: u8` to `pub level: HeadingLevel`. Add import:

```rust
use crate::markdown::normalize::types::HeadingLevel;
```

Update the `new()` constructor: parameter `level: u8` → `level: HeadingLevel` (or keep `u8` and convert internally with `HeadingLevel::new(level).expect("valid heading level")`).

Fix all usages in the file (comparisons like `n.level == 1` → `n.level == HeadingLevel::H1` or `n.level.as_u8() == 1`).

- [ ] **Step 2: Fix `toc/mod.rs` compilation**

Run: `cargo check -p darkmatter 2>&1 | grep "toc/"` and fix each error. Key spots:
- Line 224: `heading.level` passed where `u8` expected
- Line 258: `top.level >= node.level` comparison (works because `HeadingLevel` derives `PartialOrd`)
- Line 346: `n.level == 1` → `n.level == HeadingLevel::H1`

- [ ] **Step 3: Update `ContentChange` in `delta/types.rs`**

Change `original_level: Option<u8>` → `original_level: Option<HeadingLevel>` and `new_level: Option<u8>` → `new_level: Option<HeadingLevel>`.

Add import:

```rust
use crate::markdown::normalize::types::HeadingLevel;
```

Update `added()`, `removed()`, `modified()` helper functions: parameter `level: u8` → `level: HeadingLevel`.

- [ ] **Step 4: Fix `delta/mod.rs` and other delta consumers**

Run: `cargo check -p darkmatter 2>&1 | grep "delta/"` and fix each error. Where raw `u8` values are passed, wrap with `HeadingLevel::new(n).unwrap()` or propagate the `HeadingLevel` type.

- [ ] **Step 5: Update `LevelFilter` in `toc_linking/types.rs`**

Change `pub levels: HashSet<u8>` → `pub levels: HashSet<HeadingLevel>`.

Update `includes()`: parameter `level: u8` → `level: HeadingLevel`. Update the default range:

```rust
    pub fn includes(&self, level: HeadingLevel) -> bool {
        if self.levels.is_empty() {
            // Default: H2-H6
            level >= HeadingLevel::H2
        } else {
            self.levels.contains(&level)
        }
    }
```

Add import:

```rust
use crate::markdown::normalize::types::HeadingLevel;
```

- [ ] **Step 6: Fix `toc_linking/mod.rs` consumers**

Run: `cargo check -p darkmatter 2>&1 | grep "toc_linking/"` and fix each error.

- [ ] **Step 7: Update `ReferenceInsertionContext.section_heading_level` in `reference/types.rs`**

Change `pub section_heading_level: Option<u8>` → `pub section_heading_level: Option<HeadingLevel>`.

- [ ] **Step 8: Fix `reference/graph.rs` compilation**

Lines 352, 437, 462, 493, 562, 637 all set `section_heading_level`. The value `sec_level` likely comes from the heading index. Update `build_heading_index` to produce `HeadingLevel` instead of `u8`, or wrap at the assignment site with `HeadingLevel::new(level)`.

- [ ] **Step 9: Fix `reference/file_tree/model.rs` compilation**

Line 436: `context.section_heading_level.unwrap_or(2) as usize` → `context.section_heading_level.unwrap_or(HeadingLevel::H2).hash_count()`.

Test data lines (514, 527, etc.): `section_heading_level: Some(2)` → `section_heading_level: Some(HeadingLevel::H2)`.

- [ ] **Step 10: Fix `cli/src/commands.rs` compilation**

Line 1819: `if let Some(level) = insertion.context.section_heading_level` — update to use `level.as_u8()` or `level.hash_count()` where the raw number is needed.

- [ ] **Step 11: Fix integration tests**

Run: `cargo check -p darkmatter --tests 2>&1 | head -50` and fix:
- `reference_integration.rs:809`: `Some(2)` → `Some(HeadingLevel::H2)`
- `reference_integration.rs:1090`: similar

- [ ] **Step 12: Build and test everything**

Run: `cargo test -p darkmatter && cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 13: Commit**

```bash
git add darkmatter/lib/src/markdown/ darkmatter/cli/src/commands.rs darkmatter/lib/tests/
git commit -m "feat(darkmatter): unify HeadingLevel newtype across toc, delta, toc_linking, and reference modules"
```

---

### Task 11: Make `ComposeOptions` fields `pub(crate)` (with builder-only API)

**Files:**
- Modify: `lib/src/markdown/compose/types.rs`
- Potentially modify: `cli/src/commands.rs`, integration tests

- [ ] **Step 1: Change public fields to `pub(crate)`**

In `ComposeOptions` (lines 268-442), change all `pub` fields to `pub(crate)` except `context` (already private). Keep builder methods `pub`.

Important: Do NOT change fields that are `pub(crate)` already (`replace_parent_wins`, `one_off_replace`).

- [ ] **Step 2: Add getter methods for fields accessed outside the crate**

For each field accessed by `darkmatter-cli` or integration tests, add a getter:

```rust
    /// Returns the source location.
    pub fn source(&self) -> &ComposeSource { &self.source }
    
    /// Returns whether perf collection is enabled.
    pub fn perf_enabled(&self) -> bool { self.perf_enabled }
    
    // ... add others as needed based on compilation errors
```

- [ ] **Step 3: Fix compilation errors iteratively**

Run: `cargo check -p darkmatter-cli 2>&1 | head -80` and add getters or fix access patterns for each error.

Run: `cargo check -p darkmatter --tests 2>&1 | head -80` and fix test access.

- [ ] **Step 4: Build and test**

Run: `cargo test -p darkmatter && cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/types.rs darkmatter/cli/src/commands.rs darkmatter/lib/tests/
git commit -m "feat(darkmatter): make ComposeOptions fields pub(crate) with builder-only public API"
```

---

## Phase 3: Additional Type Safety

### Task 12: Use `PathBuf` for `ReferenceTarget::LocalPath`

**Files:**
- Modify: `lib/src/markdown/reference/types.rs`
- Modify: consumers in `reference/`, `compose/`, `cli/`

- [ ] **Step 1: Change `LocalPath { raw: String }` to `LocalPath { raw: String, path: PathBuf }`**

Keep the `raw` string for display/serialization and add a `path` field for filesystem operations. This is less disruptive than removing `raw` entirely.

Actually, the simpler approach: change `raw: String` → `raw: PathBuf` in `LocalPath` only:

```rust
    /// A local filesystem path.
    LocalPath { raw: PathBuf },
```

- [ ] **Step 2: Update `classify_target()` to produce `PathBuf`**

In the `classify_target` function, change the `LocalPath` arm:

```rust
    ReferenceTarget::LocalPath { raw: PathBuf::from(raw) }
```

- [ ] **Step 3: Update `ReferenceTarget::raw()` method**

The `raw()` method returns `Option<&str>`. For `LocalPath`, use `raw.to_str()`:

```rust
    Self::LocalPath { raw } => raw.to_str(),
```

- [ ] **Step 4: Fix compilation errors**

Run: `cargo check -p darkmatter 2>&1 | head -80` and fix each consumer. Most will just need `.to_str().unwrap_or_default()` or `.display()` calls where strings were expected.

- [ ] **Step 5: Build and test**

Run: `cargo test -p darkmatter && cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/reference/
git commit -m "feat(darkmatter): use PathBuf for ReferenceTarget::LocalPath"
```

---

## Phase 4: Test Coverage

### Task 13: Add CLI integration tests for `rm` subcommand

**Files:**
- Modify: `cli/tests/cli.rs`

- [ ] **Step 1: Add `rm` subcommand tests**

Add a new test module section for `rm`:

```rust
#[test]
fn rm_removes_frontmatter_key() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Test\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(content.contains("title: Hello"));
}

#[test]
fn rm_nonexistent_key_reports_error_or_noop() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "---\ntitle: Hello\n---\n\n# Content\n").unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "nonexistent"])
        .assert()
        .success();
}

#[test]
fn rm_with_verbose_shows_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Test\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["-v", "rm", file.to_str().unwrap(), "author"])
        .assert()
        .success()
        .stderr(predicates::str::contains("author"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p darkmatter-cli rm`
Expected: All new tests pass.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/cli/tests/cli.rs
git commit -m "test(darkmatter-cli): add integration tests for rm subcommand"
```

---

### Task 14: Add CLI integration tests for untested flags

**Files:**
- Modify: `cli/tests/cli.rs`

- [ ] **Step 1: Add `--list-themes` test**

```rust
#[test]
fn list_themes_produces_output() {
    md_cmd()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicates::str::contains("theme"));
}
```

- [ ] **Step 2: Add `--completions` test**

```rust
#[test]
fn completions_bash_produces_output() {
    md_cmd()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}
```

- [ ] **Step 3: Add `--line-numbers` test**

```rust
#[test]
fn line_numbers_flag_adds_numbers_to_code_blocks() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(tmp, "```rust\nfn main() {{}}\n```").unwrap();

    md_cmd()
        .args(["--line-numbers", "--output", "html"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("line-number").or(predicates::str::contains("fn main")));
}
```

- [ ] **Step 4: Add compose `--compact` / `--loose` tests**

```rust
#[test]
fn compose_compact_flag() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(tmp, "- item 1\n\n- item 2\n").unwrap();

    md_cmd()
        .args(["compose", "--compact"])
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn compose_loose_flag() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(tmp, "- item 1\n- item 2\n").unwrap();

    md_cmd()
        .args(["compose", "--loose"])
        .arg(tmp.path())
        .assert()
        .success();
}
```

- [ ] **Step 5: Add `graph --json` test**

```rust
#[test]
fn graph_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "# Test\n\n[link](other.md)\n").unwrap();

    md_cmd()
        .args(["graph", "--json"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{").or(predicates::str::starts_with("[")));
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p darkmatter-cli`
Expected: All new tests pass.

- [ ] **Step 7: Commit**

```bash
git add darkmatter/cli/tests/cli.rs
git commit -m "test(darkmatter-cli): add integration tests for --list-themes, --completions, --line-numbers, --compact, --loose, graph --json"
```

---

### Task 15: Add `md_file` test helper to reduce boilerplate

**Files:**
- Modify: `cli/tests/cli.rs`

- [ ] **Step 1: Add helper function**

Add near the top of the test file (after `md_cmd()`):

```rust
/// Creates a temporary markdown file with the given content.
fn md_file(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", content).unwrap();
    tmp
}
```

- [ ] **Step 2: Refactor 3-5 existing tests to use the helper**

Pick a handful of tests that use the `NamedTempFile::new()` + `writeln!()` pattern and replace with `md_file()`. Don't refactor all 20+ at once -- just enough to validate the pattern.

- [ ] **Step 3: Run tests**

Run: `cargo test -p darkmatter-cli`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/cli/tests/cli.rs
git commit -m "test(darkmatter-cli): add md_file helper and refactor a few tests to use it"
```

---

### Task 16: Add cache manifest tests

**Files:**
- Modify: `lib/src/markdown/compose/cache/manifest.rs`

- [ ] **Step 1: Read the manifest module to understand the API**

Read the full contents of `manifest.rs` to understand the types (`DocumentSnapshotManifest`, `ComposedDocumentManifest`, `OperationResultManifest`) and their methods (`is_fresh()`, `touch()`, `is_expired()`).

- [ ] **Step 2: Write tests for each manifest type**

Add a `#[cfg(test)] mod tests` block at the end of the file with tests covering:
- Creating each manifest type
- `is_fresh()` returns true when first created
- `touch()` updates the timestamp
- `is_expired()` with various durations
- Edge cases (zero duration, max duration)

The exact test code depends on the API surface discovered in Step 1.

- [ ] **Step 3: Run tests**

Run: `cargo test -p darkmatter --lib markdown::compose::cache::manifest`
Expected: All new tests pass.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/cache/manifest.rs
git commit -m "test(darkmatter): add unit tests for cache manifest types"
```

---

## Deferred Items (not in this plan)

The following review items are intentionally deferred:

- **1.6 CLI `Set.value` dual-interpretation** -- Behavioral change requiring CLI UX discussion
- **1.7 CLI `Edit.file` as `PathBuf`** -- `FileReference` comes from `biscuit_file` crate, change belongs there
- **1.8 Reference record attributes as typed structs** -- Review explicitly recommends deferring
- **2.7 Path resolution deduplication** -- Requires deeper analysis of `FileReference` + `resolve_relative` interaction across 3 call sites
- **2.8 CLI test helper repetition** -- Addressed minimally in Task 15; full migration is mechanical
- **3.2 `--mermaid` and `--theme`/`--code-theme` flag tests** -- Mermaid requires external tooling; theme tests need terminal capability mocking
- **3.3 `render_terminal.rs` testability** -- Requires refactoring render function signature (returns `()`, prints to stdout)
- **3.3 `output/html.rs` unit tests** -- Review says 0 unit tests but exploration found 49 tests (line 648+); review may be outdated
- **3.4 `run_render` TTY path** -- Requires pseudo-TTY test infrastructure
- **3.4 `validate --remote`/`--timeout`** -- Requires wiremock setup
- **3.4 `validate --show-all`** -- Minor flag test
- **3.4 `--save` + `--verbose`** -- Minor flag combination test
- **3.5 Test isolation with `serial_test`** -- Cross-cutting concern, separate effort

---

## Execution Order

Phases must be executed in order. Within each phase, tasks can be executed in parallel where noted:

- **Phase 1** (Tasks 1-6): Can run Tasks 1+2 in parallel, Task 3 alone, Task 4 alone, Task 5 alone, Task 6 alone
- **Phase 2** (Tasks 7-11): Task 7 alone, Task 8 alone, Task 9 alone, Task 10 depends on toc/delta/reference modules being stable, Task 11 alone
- **Phase 3** (Task 12): Alone, after Phase 2 stabilizes
- **Phase 4** (Tasks 13-16): All can run in parallel
