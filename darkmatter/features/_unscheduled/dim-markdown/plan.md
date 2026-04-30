---
phases: 6
created: 2025-01-27
start_phase: 1
source_files_during_phase_1:
  - biscuit-terminal/lib/src/discovery/detection.rs
  - darkmatter/lib/src/terminal/supports.rs
  - darkmatter/lib/src/terminal/mod.rs
  - darkmatter/lib/src/terminal/tests.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/output/code_block.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/inline/types.rs
  - darkmatter/lib/src/markdown/inline/mod.rs
  - darkmatter/lib/src/markdown/highlighting/scope_cache.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/output/html.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/darkmatter/SKILL.md
packages:
  - biscuit-terminal
  - darkmatter
---

# Execution Plan: Dim Markdown Syntax

Derived from:
- `darkmatter/features/_unscheduled/dim-markdown/spec.md`
- `darkmatter/features/_unscheduled/dim-markdown/tech-design.md`

## Overview

Implement the `⌄text⌄` dim markdown syntax. This is a terminal-first inline formatting extension that maps to ANSI SGR `2` (faint/decreased intensity) when the terminal supports it, and falls back to plain text otherwise. HTML output preserves the `⌄` delimiters as literal characters.

---

## Phase 1: Foundation — Capability Detection & Options

**Goal:** Add terminal capability detection for dim support and the `DimMode` / `TerminalOptions` configuration.

**Dependencies:** None.

### Step 1.1 — Add `dim_support()` to `biscuit-terminal`

**Files:**
- `biscuit-terminal/lib/src/discovery/detection.rs`

**Work:**
- Add `pub fn dim_support() -> bool` after `italics_support()`.
- Strategy (per tech design):
  1. Return `false` when `TERM=dumb`.
  2. Return `false` when `ColorDepth::None`.
  3. Return `true` for known modern terminal apps: Kitty, WezTerm, Ghostty, iTerm2, Warp, VS Code, Alacritty, Konsole, Foot, Contour, GNOME Terminal.
  4. Return `true` for common ANSI `TERM` values: `xterm`, `xterm-256color`, `screen`, `screen-256color`, `tmux`, `tmux-256color`, `rxvt`, `linux`.
  5. Return `false` otherwise.
- Add scoped environment-guard unit tests (follow existing `italics_support()` test pattern):
  - `TERM=dumb` → `false`
  - `TERM_PROGRAM=WezTerm` → `true`
  - `TERM=xterm-256color` → `true`
  - no known signals + no color → `false`

**Checkpoint:** `cargo test -p biscuit-terminal` passes.

### Step 1.2 — Expose `supports_dim()` in `darkmatter`

**Files:**
- `darkmatter/lib/src/terminal/supports.rs`
- `darkmatter/lib/src/terminal/mod.rs`

**Work:**
- Add `pub fn supports_dim() -> bool` in `supports.rs` that delegates to `biscuit_terminal::discovery::detection::dim_support()`.
- Re-export from `terminal/mod.rs`.
- Add rustdoc and a minimal unit test (follow `supports_italics` pattern).

**Checkpoint:** `cargo check -p darkmatter` compiles.

### Step 1.3 — Add `DimMode` enum and integrate into `TerminalOptions`

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs`

**Work:**
- Define `DimMode` enum (`Auto`, `Always`, `Never`) with `Default` = `Auto`, mirroring `ItalicMode`.
- Add `dim_mode: DimMode` field to `TerminalOptions`.
- Set default in `TerminalOptions::default()`.
- Because `TerminalOptions` is `#[non_exhaustive]`, update any in-repo struct-literal tests to use `..Default::default()` or include the new field.

**Parallelizable with:** Step 1.1 (no cross-package dependency at compile time).

**Checkpoint:** `cargo check -p darkmatter` compiles.

---

## Phase 2: Event Model & Inline Processor

**Goal:** Add the `Dim` inline tag and evolve the inline processor to handle both `==mark==` and `⌄dim⌄`.

**Dependencies:** Phase 1 (for compilation context; technically only Step 1.3 is needed if tests reference `DimMode`, but the processor itself is independent).

### Step 2.1 — Add `InlineTag::Dim`

**Files:**
- `darkmatter/lib/src/markdown/inline/types.rs`

**Work:**
- Add `Dim` variant to `InlineTag` enum.
- Update rustdoc: `Mark` → custom highlight syntax; `Dim` → terminal-first dim syntax; HTML preserves delimiters.
- Update `InlineEvent` docs to no longer describe `MarkProcessor` as the only producer.
- Add unit tests for `InlineTag::Dim` equality, debug, clone.
- Add unit tests for `InlineEvent::Start(Dim)` / `End(Dim)`.

**Checkpoint:** `cargo test -p darkmatter inline::types` passes.

### Step 2.2 — Rename `MarkProcessor` → `InlineStyleProcessor`

**Files:**
- `darkmatter/lib/src/markdown/inline/mod.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/markdown/output/html.rs`

**Work:**
- Rename struct and `impl` blocks in `inline/mod.rs`.
- Add `pub type MarkProcessor<'a, I> = InlineStyleProcessor<'a, I>;` compatibility alias.
- Update module-level rustdoc, doc examples, and comments.
- Update imports in `terminal.rs` and `html.rs` to use `InlineStyleProcessor` (or keep `MarkProcessor` alias temporarily; preference: migrate to new name).

**Checkpoint:** `cargo check -p darkmatter` compiles.

### Step 2.3 — Implement U+2304 delimiter detection and pairing

**Files:**
- `darkmatter/lib/src/markdown/inline/mod.rs`

**Work:**
- Define internal types: `InlineDelimiterKind { Mark, Dim }`, `InlineDelimiter { kind, byte_start, byte_end, can_open, can_close }`.
- Implement `find_delimiters(text: &str) -> Vec<InlineDelimiter>` that scans for both `==` and `⌄` in a single pass, classifying each by byte order.
- For `==`: keep existing simple toggle behavior (no `can_open`/`can_close` classification needed).
- For `⌄` (U+2304):
  - `can_open` = not followed by Unicode whitespace and next char exists.
  - `can_close` = not preceded by Unicode whitespace and previous char exists.
  - Intra-word rule: if both prev and next are alphanumeric, disallow both sides forming a pair (treat like `_` in CommonMark).
  - Escaped: if raw backslash immediately before `⌄`, force literal.
- Implement pairing with a small stack (per tech design pseudo-code).
- Convert unclosed opening delimiters back to literal text.
- Update `process_text` to call the unified delimiter scanner and emit `InlineEvent::Start/End(Dim)` and `Start/End(Mark)` as appropriate.
- Update code-block tracking: still pass `Event::Code` through unchanged; still skip processing inside fenced/indented code blocks.
- Update fast path: check for both `==` and `⌄` (or `\u{2304}`).

**Checkpoint:** `cargo test -p darkmatter inline::mod` passes.

### Step 2.4 — Add `markup.faint.markdown` scope to `ScopeCache`

**Files:**
- `darkmatter/lib/src/markdown/highlighting/scope_cache.rs`

**Work:**
- Add `pub faint: Scope` field to `ScopeCache`.
- Initialize it in `ScopeCache::new()` with `parse_scope("markup.faint.markdown")`.
- Update `scope_for_inline_tag` to map `InlineTag::Dim => self.faint`.
- Add unit test for `scope_for_inline_tag(Dim)`.

**Parallelizable with:** Steps 2.1–2.3 (no direct dependency).

**Checkpoint:** `cargo test -p darkmatter highlighting::scope_cache` passes.

---

## Phase 3: Terminal Rendering

**Goal:** Render `Dim` spans with SGR `2` when supported, plain text when not.

**Dependencies:** Phase 2 (needs `InlineTag::Dim`, `InlineStyleProcessor`, `DimMode`).

### Step 3.1 — Add `in_dim` state tracking to terminal renderer event loop

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs`

**Work:**
- In the main `for event in events` loop, add `let mut in_dim = false;` alongside `in_emphasis`, `in_strong`, etc.
- Handle `InlineEvent::Start(InlineTag::Dim)` and `InlineEvent::End(InlineTag::Dim)`:
  - Set `in_dim = true/false`.
  - Push/pop scope via `ScopeCache::global().scope_for_inline_tag(InlineTag::Dim)`.
- Compute `let emit_dim = options.dim_mode.should_emit_dim();` near where `emit_italic` is resolved.

**Checkpoint:** `cargo check -p darkmatter` compiles.

### Step 3.2 — Thread `in_dim` / `emit_dim` through prose text emission

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs`

**Work:**
- Update `emit_prose_text` signature to accept `in_dim: bool` and `emit_dim: bool`.
- Emit `\x1b[2m` before foreground color when `emit_dim && in_dim`.
- Use SGR `22` to clear dim where practical; note that `22` also clears bold, so ensure nested strong+dim tests verify no style leakage.
- Update `resolve_prose_text_style` to account for `in_dim`.
- Update `LineWrapper::emit_styled` signature to accept `in_dim`.
- Update `LineWrapper::emit_word` signature to accept `in_dim`.
- Update all call sites in the terminal renderer to pass `in_dim` through.

**Checkpoint:** `cargo check -p darkmatter` compiles.

### Step 3.3 — Add `in_dim` to `TableCellInlineState` and table cell rendering

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs`

**Work:**
- Add `in_dim: bool` to `TableCellInlineState`.
- Update table cell inline rendering to pass `in_dim` into `emit_prose_text`.
- When serializing table cell content through `Prose`:
  - If `in_dim && emit_dim`, wrap in `<dim>...</dim>`.
  - Maintain stable nesting order: `<bold><italic><strikethrough><dim>text</dim></strikethrough></italic></bold>`.
- Update all `TableCellInlineState` construction sites.

**Checkpoint:** `cargo test -p darkmatter` for terminal output module passes.

---

## Phase 4: HTML Rendering

**Goal:** Preserve `⌄` delimiters as literal characters in HTML output.

**Dependencies:** Phase 2 (needs `InlineTag::Dim`).

**Parallelizable with:** Phase 3.

### Step 4.1 — Handle `Dim` events as literal delimiters in HTML

**Files:**
- `darkmatter/lib/src/markdown/output/html.rs`

**Work:**
- In the main event loop, add arms for `InlineEvent::Start(InlineTag::Dim)` and `InlineEvent::End(InlineTag::Dim)`.
- Each arm pushes a literal `'⌄'` character to `output`.
- Because normal text goes through `html_escape::encode_text`, literal U+2304 is safe to write directly.

**Checkpoint:** `cargo check -p darkmatter` compiles.

---

## Phase 5: Testing

**Goal:** Add focused unit and integration tests for all changed modules.

**Dependencies:** Phases 3 and 4.

### Step 5.1 — Inline processor unit tests

**File:** `darkmatter/lib/src/markdown/inline/mod.rs` (test module)

**Test cases:**
- `⌄dimmed⌄` emits balanced `Dim` start/end events.
- `This is ⌄unclosed` renders literal `⌄`.
- `⌄⌄` emits an empty balanced span.
- `\⌄literal\⌄` renders literal delimiters and no `Dim` events.
- `` `⌄code⌄` `` emits no `Dim` events.
- Fenced code containing `⌄code⌄` emits no `Dim` events.
- `foo⌄bar⌄baz` follows the intended intraword rule.
- `*⌄dim italic⌄*` preserves both `Emphasis` and `Dim` events.
- `==⌄dim mark⌄==` preserves both `Mark` and `Dim` events.
- Mixed `==` and `⌄` in same text event produce correct event ordering.

**Checkpoint:** `cargo test -p darkmatter inline::mod` passes.

### Step 5.2 — Terminal renderer unit tests

**File:** `darkmatter/lib/src/markdown/output/terminal.rs` (test module)

**Test cases:**
- `DimMode::Always` produces `\x1b[2m` for `⌄dim⌄`.
- `DimMode::Never` strips the delimiters and produces no `\x1b[2m`.
- `DimMode::Always` with `**⌄bold dim⌄**` includes bold and dim and does not leak either style into following text.
- Unclosed delimiters remain literal in terminal output.
- Inline code and fenced code retain literal delimiters.
- Dim works inside list items and blockquotes.
- Dim table cells keep correct visible text and do not expose delimiters.
- `DimMode::Auto` with a supporting terminal produces `\x1b[2m`.
- `DimMode::Auto` with `TERM=dumb` produces no `\x1b[2m`.

**Checkpoint:** `cargo test -p darkmatter output::terminal` passes.

### Step 5.3 — HTML renderer unit tests

**File:** `darkmatter/lib/src/markdown/output/html.rs` (test module)

**Test cases:**
- `⌄dim⌄` renders as literal `⌄dim⌄`.
- `⌄dim and **strong**⌄` preserves delimiters around nested HTML (`<p>⌄dim and <strong>strong</strong>⌄</p>`).
- Inline code and fenced code preserve literal delimiters.
- HTML escaping remains correct for content inside dim spans.

**Checkpoint:** `cargo test -p darkmatter output::html` passes.

### Step 5.4 — biscuit-terminal detection tests

**File:** `biscuit-terminal/lib/src/discovery/detection.rs` (test module)

**Test cases:**
- `TERM=dumb` returns `false`.
- `TERM_PROGRAM=WezTerm` returns `true`.
- `TERM=xterm-256color` returns `true`.
- No known signals + `ColorDepth::None` returns `false`.

**Checkpoint:** `cargo test -p biscuit-terminal discovery::detection` passes.

### Step 5.5 — Integration / end-to-end tests

**File:** `darkmatter/lib/src/markdown/mod.rs` or existing integration test location

**Test cases:**
- Full pipeline: Markdown source `⌄dim⌄` → terminal output contains `\x1b[2m`.
- Full pipeline: Markdown source `⌄dim⌄` → HTML output contains literal `⌄dim⌄`.
- Cross-format consistency: `Prose::new("<dim>text</dim>").render(None)` and darkmatter terminal-rendered `⌄text⌄` produce identical visible output.

**Checkpoint:** `cargo test -p darkmatter` (full suite) passes.

---

## Phase 6: Documentation

**Goal:** Update all user-facing and internal docs.

**Dependencies:** Phase 5 (tests must pass first).

### Step 6.1 — Update darkmatter docs

**Files:**
- `darkmatter/docs/topics/output-formats.md`
  - Add a note that Terminal output supports `⌄dim⌄` syntax.
- `darkmatter/docs/topics/html.md`
  - Document that `⌄text⌄` is preserved as literal `⌄` characters in HTML (no semantic `<dim>` tag).
- `darkmatter/lib/README.md`
  - Add `⌄dim⌄` to inline syntax features if there's an inline formatting list.

### Step 6.2 — Update rustdoc in modified modules

**Files:**
- `darkmatter/lib/src/markdown/inline/types.rs`
- `darkmatter/lib/src/markdown/inline/mod.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/terminal/supports.rs`
- `biscuit-terminal/lib/src/discovery/detection.rs`

**Work:**
- Update doc comments to reflect `Dim` support.
- Update module-level docs to mention `InlineStyleProcessor` instead of `MarkProcessor`.
- Update `TerminalOptions` docs to describe `dim_mode`.

### Step 6.3 — Update `.claude/skills/darkmatter/SKILL.md`

**File:** `.opencode/skill/darkmatter/SKILL.md`

**Work:**
- Add `⌄dim⌄` to the list of supported inline syntax extensions.
- Note terminal-first behavior and HTML literal preservation.

### Step 6.4 — Update `.claude/skills/biscuit-terminal/SKILL.md`

**File:** `.opencode/skill/biscuit-terminal/SKILL.md`

**Work:**
- Add `dim_support()` to the detection capability list.

**Checkpoint:** Review all modified docs for consistency and completeness.

---

## Parallelization Summary

| Phase | Parallel With | Notes |
|-------|--------------|-------|
| Phase 1 | — | Steps 1.1 and 1.3 can be done in parallel; 1.2 depends on 1.1 |
| Phase 2 | Phase 1 (after Step 1.3) | Event model changes are independent of detection |
| Phase 3 | Phase 4 | Terminal and HTML rendering both depend on Phase 2 but not on each other |
| Phase 5 | — | Must wait for Phases 3 and 4 |
| Phase 6 | — | Must wait for Phase 5 |

---

## Validation Checkpoints

| Checkpoint | Command | Expected Result |
|-----------|---------|----------------|
| After Phase 1 | `cargo test -p biscuit-terminal` | Passes |
| After Phase 2 | `cargo test -p darkmatter inline` | Passes |
| After Phase 3 | `cargo test -p darkmatter output::terminal` | Passes |
| After Phase 4 | `cargo test -p darkmatter output::html` | Passes |
| After Phase 5 | `cargo test -p darkmatter` | Full suite passes |
| After Phase 5 | `cargo test -p biscuit-terminal` | Full suite passes |
| Final | `cargo test` (workspace) | All relevant packages pass |
| Final | `cargo clippy -p darkmatter -p biscuit-terminal` | Clean |

---

## Risk Mitigation

1. **Parser fidelity risk:** The `⌄` delimiter classifier is intentionally simpler than full CommonMark emphasis parsing. If edge cases arise, document them in tests and defer exact CommonMark parity to a future dedicated inline extension parser.
2. **SGR 22 bold/dim collision:** The terminal renderer recomputes active styles per text fragment. Mandatory tests for nested `**⌄bold dim⌄**` verify no leakage.
3. **Breaking change:** No mitigation — `⌄` becomes a delimiter immediately. Document this in release notes.

---

## Files to Modify

### biscuit-terminal
- `biscuit-terminal/lib/src/discovery/detection.rs`

### darkmatter
- `darkmatter/lib/src/terminal/supports.rs`
- `darkmatter/lib/src/terminal/mod.rs`
- `darkmatter/lib/src/markdown/inline/types.rs`
- `darkmatter/lib/src/markdown/inline/mod.rs`
- `darkmatter/lib/src/markdown/highlighting/scope_cache.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/markdown/output/html.rs`
- `darkmatter/docs/topics/output-formats.md`
- `darkmatter/docs/topics/html.md`
- `darkmatter/lib/README.md`

### Skills
- `.opencode/skill/darkmatter/SKILL.md`
- `.opencode/skill/biscuit-terminal/SKILL.md`
