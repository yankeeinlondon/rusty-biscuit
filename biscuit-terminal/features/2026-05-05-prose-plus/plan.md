---
phases: 5
created: "2026-05-05"
start_phase: 1
source_files_during_phase_1:
  - biscuit-terminal/lib/src/components/prose/tokens.rs
  - biscuit-terminal/lib/src/components/prose/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - biscuit-terminal
---

# Prose+ Execution Plan

Derived from: `biscuit-terminal/features/_unscheduled/prose-plus/spec.md`

## Overview

Extend the `Prose` component with Markdown syntax support (`**bold**`, `_italics_`, `[desc](ref)` links) while maintaining
full backward compatibility with existing tag-based formatting. Implement `BrowserRenderable` for web-environment rendering.

---

## Phase 1: Foundation — Escape Handling Extension

**Goal:** Extend the existing escape mechanism to cover Markdown syntax characters.

**Dependencies:** None. This is foundational work that all subsequent phases depend on.

### Step 1.1 — Extend escape character set in `tokens.rs`

**File:** `biscuit-terminal/lib/src/components/prose/tokens.rs`

**Work:**
- Extend the backslash escape handler (line 98–106) to recognize `*`, `_`, `[`, `]`, `(`, `)` in addition to the existing `<`,
`>`, `{`, `\`.
- The pattern match on `chars.peek()` must include all Markdown syntax characters.

**Validation:**
- Unit test: `\*` renders as literal `*`
- Unit test: `\_` renders as literal `_`
- Unit test: `\[` renders as literal `[`
- Unit test: `\]` renders as literal `]`
- Unit test: `\(` renders as literal `(`
- Unit test: `\)` renders as literal `)`
- Unit test: `\\` still renders as single `\`

### Checkpoint 1-A

Run `cargo test -p biscuit-terminal` and confirm all existing tests pass plus new escape tests pass.

---

## Phase 2: Markdown Pre-processing Engine

**Goal:** Build a pre-processing step that converts Markdown syntax into existing Prose `<tag>` syntax, leveraging the current
parser and layer-tracking logic.

**Dependencies:** Phase 1 (escape handling must be in place to prevent false positives during pre-processing).

### Step 2.1 — Create `markdown.rs` module

**File:** `biscuit-terminal/lib/src/components/prose/markdown.rs`

**Work:**
- Create new module with a `preprocess_markdown(input: &str) -> String` entry point.
- Export only the entry point; internal helpers stay private.

**Validation:**
- Module compiles and is included in `mod.rs`.

### Step 2.2 — Implement atomic link conversion `[desc](ref)` -> `<a href="ref">desc</a>`

**Work:**
- Implement a regex-free (or minimal-regex) scanner that finds `[desc](ref)` patterns.
- Must respect escaped characters: `\[`, `\]`, `\(`, `\)` should not trigger link detection.
- The conversion must be **atomic**: the `ref` portion is locked and protected from subsequent bold/italics processing. Any
`*` or `_` inside the URL must be preserved literally.
- Href resolution: use the existing `resolve_href` helper from `styles.rs` for the `ref` value.

**Validation:**
- Unit test: `[click here](https://example.com)` -> `<a href="https://example.com">click here</a>`
- Unit test: `[link](https://example.com/path_with_underscores)` preserves underscores in URL
- Unit test: `\[not a link\](url)` renders as literal `[not a link](url)`
- Unit test: `[desc\]](url)` handles escaped bracket in description

### Step 2.3 — Implement bold conversion `**text**` -> `<b>text</b>`

**Work:**
- Implement scanner for `**text**` patterns.
- Must respect escaped asterisks: `\*` should not trigger bold detection.
- Must NOT match `__text__` (explicitly unsupported per spec).
- Nested bold within other styles should work naturally since the output feeds into the existing tag parser.

**Validation:**
- Unit test: `**bold text**` -> `<b>bold text</b>`
- Unit test: `\*\*not bold\*\*` renders as literal `**not bold**`
- Unit test: `**bold _and italics_**` correctly nests (converted to `<b>bold _and italics_</b>`, then italics phase handles
the inner `_..._`)

### Step 2.4 — Implement italics conversion `_text_` -> `<i>text</i>`

**Work:**
- Implement scanner for `_text_` patterns.
- Must respect escaped underscores: `\_` should not trigger italics detection.
- Must NOT match `*text*` (explicitly unsupported per spec).
- Must handle underscores inside already-converted link URLs (protected by atomic link conversion in Step 2.2).

**Validation:**
- Unit test: `_italic text_` -> `<i>italic text</i>`
- Unit test: `\_not italic\_` renders as literal `_not italic_`
- Unit test: `**_bold italics_**` correctly nests

### Step 2.5 — Wire pre-processing into `Prose::parse_tokens`

**File:** `biscuit-terminal/lib/src/components/prose/prose.rs`

**Work:**
- Call `preprocess_markdown` at the top of `parse_tokens` before delegating to `parse_tokens_inner`.
- The conversion order is enforced by the pipeline: links → bold → italics, each phase operating on the output of the previous.

**Validation:**
- Unit test: Mixed markdown input `**bold** and _italics_ and [link](url)` renders correctly.

### Step 2.6 — Unit tests for pre-processing module

**File:** `biscuit-terminal/lib/src/components/prose/markdown.rs` (inline `#[cfg(test)]` module)

**Work:**
- Comprehensive tests covering all acceptance criteria related to markdown parsing.
- Tests for edge cases: empty link description, empty link URL, nested structures, adjacent markdown constructs.

**Parallelizable:** Steps 2.2, 2.3, and 2.4 can be developed in parallel by different agents since they are independent
conversions that share the same pattern (scanner + replacement). The agent must coordinate on the module structure (Step 2.1)
first.

### Checkpoint 2-A

Run `cargo test -p biscuit-terminal` and confirm all markdown pre-processing tests pass.

### Checkpoint 2-B

Verify backward compatibility: run the existing Prose unit tests and confirm zero regressions.

---

## Phase 3: Terminal Rendering Integration & Tests

**Goal:** Ensure markdown-derived links, bold, and italics render correctly in terminal output with proper OSC8 support and
graceful degradation.

**Dependencies:** Phase 2 (markdown pre-processing must be wired in).

### Step 3.1 — Verify OSC8 link rendering for markdown-derived links

**Work:**
- Since markdown links are converted to `<a href="...">...</a>` tags in pre-processing, the existing `block_tag_to_escape`
logic in `styles.rs` handles OSC8 emission automatically.
- Confirm via test that `[desc](https://example.com)` with `osc_link_support == true` emits OSC8 escapes.

**Validation:**
- Unit test: `Prose::new("[click here](https://example.com)")` with optimistic terminal emits OSC8 sequence.

### Step 3.2 — Verify markdown fallback for non-OSC8 terminals

**Work:**
- The existing markdown fallback path in `block_tag_to_escape` (line 247–258) already handles non-OSC8 terminals by emitting
`[description](url)`.
- Confirm via test that markdown-derived links degrade correctly.

**Validation:**
- Unit test: `Prose::new("[click here](https://example.com)")` with `osc_link_support == false` emits `[click
here](https://example.com)`.

### Step 3.3 — Add Level-1 unit tests for markdown syntax

**File:** `biscuit-terminal/lib/src/components/prose/mod.rs` (existing test module)

**Work:**
- Add tests for `**bold**` rendering to ANSI bold.
- Add tests for `_italics_` rendering to ANSI italics.
- Add tests for nested `**_bold italics_**`.
- Add tests for `[link](url)` with OSC8 and fallback.
- Add tests for unsupported `__bold__` and `*italics*` being treated as literal text.
- Add tests for escaped markdown characters.

### Step 3.4 — Add Level-2 integration tests

**Files:**
- `biscuit-terminal/cli/tests/level2_prose_styling.rs`
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`

**Work:**
- Add WezTerm/Kitty tests for markdown bold and italics SGR emission.
- Add WezTerm/Kitty test for markdown link OSC8 rendering.
- Add Apple Terminal test for markdown link fallback visibility.

**Parallelizable:** Level-2 test development can proceed in parallel with BrowserRenderable implementation (Phase 4) since
they test independent render paths.

### Checkpoint 3-A

Run `cargo test -p biscuit-terminal` (unit tests) — all pass.

### Checkpoint 3-B

Run `cargo test -p biscuit-terminal-cli --test level2_prose_styling` (Level-2 terminal tests) — all pass or skip cleanly.

---

## Phase 4: BrowserRenderable Implementation

**Goal:** Implement `BrowserRenderable for Prose` to produce HTML output with high fidelity to terminal layouts.

**Dependencies:** None (this is an independent render path). Can execute in parallel with Phase 3 after Phase 2 completes.

### Step 4.1 — Create `browser.rs` module

**File:** `biscuit-terminal/lib/src/components/prose/browser.rs`

**Work:**
- Create new module.
- Add to `mod.rs`.

### Step 4.2 — Implement `BrowserRenderable for Prose`

**Work:**
- Implement `render_to_browser(&self) -> String`.
- **Outer wrapper:** Emit a `<div>` with inline CSS replicating terminal layout:
    - `margin-left`, `margin-right` from `Layout` margins (use `Margin::to_css_value` pattern from `horizontal_rule/browser.rs`).
    - `text-align` from `Layout::alignment` (`left`/`center`/`right`).
- **Content rendering:** The browser renderer needs its own parsing pass that converts Prose tags to HTML instead of ANSI:
    - `<b>...</b>` -> `<span style="font-weight: bold;">...</span>`
    - `<i>...</i>` -> `<span style="font-style: italic;">...</span>`
    - `<a href="...">...</a>` -> `<a href="...">...</a>`
    - Colors -> `<span style="color: rgb(...);">...</span>` or `<span style="background-color: rgb(...);">...</span>`
    - Strip atomic tokens or handle them as equivalent inline styles.
- **Approach:** Rather than reusing the ANSI-producing `parse_tokens_inner`, create a `parse_tokens_to_html_inner` that
mirrors the structure but emits HTML tags. Alternatively, parse the already-preprocessed string (with markdown converted to
tags) and emit HTML.

**Validation:**
- Unit test: `Prose::new("**bold**")` renders as `<div ...><span style="font-weight: bold;">bold</span></div>`.
- Unit test: `Prose::new("_italics_")` renders as `<div ...><span style="font-style: italic;">italics</span></div>`.
- Unit test: `Prose::new("[link](https://example.com)")` renders as `<div ...><a href="https://example.com">link</a></div>`.
- Unit test: Layout with `Alignment::Center` emits `text-align: center`.

### Step 4.3 — Implement `render_to_browser_with_inline_variables`

**Work:**
- Provide default behavior or custom variable substitution if needed.
- The default implementation from the trait may suffice; evaluate during implementation.

### Step 4.4 — Browser rendering unit tests

**File:** `biscuit-terminal/lib/src/components/prose/browser.rs` (inline `#[cfg(test)]` module)

**Work:**
- Tests for styled text segments (`<span>` tags).
- Tests for link rendering (`<a>` tags).
- Tests for layout CSS (margins, alignment).
- Tests for nested styles.

### Checkpoint 4-A

Run `cargo test -p biscuit-terminal` and confirm all BrowserRenderable tests pass.

---

## Phase 5: Integration Validation & Documentation

**Goal:** Verify all acceptance criteria and update documentation.

**Dependencies:** Phases 3 and 4 both complete.

### Step 5.1 — Acceptance criteria verification

**Work:** Go through each acceptance criterion in the spec and verify:

- [ ] `Prose` correctly parses `**bold**`, `_italics_`, and `[desc](ref)` syntax.
- [ ] `Prose` strictly ignores `__bold__` and `*italics*`.
- [ ] `Prose` correctly handles the escaping mechanism (`\`, `\*`, `\_`, `\[`, etc.).
- [ ] Pre-processing follows the mandated order (Links -> Bold -> Italics).
- [ ] Link target conversion is atomic and protects internal Markdown-like characters.
- [ ] Terminal rendering emits OSC8 for links on supported terminals.
- [ ] Terminal rendering falls back to `[desc](ref)` for links on non-OSC8 terminals.
- [ ] `BrowserRenderable` implementation produces a `<div>` "High-Fidelity Block" with inline CSS.
- [ ] `BrowserRenderable` uses `<span>` tags for styled segments within the output block.
- [ ] Nested styles (e.g., `**_bold italics_**`) are correctly preserved across all rendering targets.

### Step 5.2 — Run full test suite

**Commands:**
```bash
cargo test -p biscuit-terminal
cargo test -p biscuit-terminal-cli --test level2_prose_styling
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose
```

### Step 5.3 — Update documentation

**File:** `biscuit-terminal/docs/components/prose.md`

**Work:**
- Add Markdown syntax examples alongside existing tag examples.
- Document the supported Markdown subset (bold, italics, links).
- Document the escaping mechanism for Markdown characters.
- Document the `BrowserRenderable` output format.

### Step 5.4 — Update `biscuit-terminal` skill

**File:** `.opencode/skill/biscuit-terminal/SKILL.md` or related skill docs

**Work:**
- Update skill documentation to reflect Markdown support in Prose.

### Checkpoint 5-A

All acceptance criteria checked off. All tests pass. Documentation updated.

---

## Parallelization Summary

| Phase | Parallelizable With | Notes |
|-------|---------------------|-------|
| 1 | — | Foundation; must complete first |
| 2 | — | Depends on Phase 1 |
| 3 | Phase 4 | Terminal and browser render paths are independent once markdown preprocessing is done |
| 4 | Phase 3 | BrowserRenderable is a separate trait implementation |
| 5 | — | Requires both Phase 3 and Phase 4 |

**Maximum parallelization:** Phase 3 and Phase 4 can execute concurrently after Phase 2 completes. Within Phase 2, Steps 2.2
(links), 2.3 (bold), and 2.4 (italics) can be developed in parallel if module structure (Step 2.1) is established first.

## Risk Mitigation

1. **Parser collision risk:** The markdown pre-processing emits tags that the existing parser already understands (`<b>`,
`<i>`, `<a>`). This minimizes risk by reusing battle-tested parser logic.
2. **Escape handling risk:** Extending escape handling to cover Markdown characters could theoretically affect existing
content, but since `*`, `_`, `[`, `]`, `(`, `)` were previously unescaped literals, adding escapes for them is purely additive
and safe.
3. **Performance risk:** Pre-processing adds a pass over the input string. Mitigation: the conversion is a single linear scan
per syntax type; acceptable for terminal rendering where inputs are typically small.

## Files to Modify

- `biscuit-terminal/lib/src/components/prose/tokens.rs` — extend escape handling
- `biscuit-terminal/lib/src/components/prose/prose.rs` — wire in pre-processing
- `biscuit-terminal/lib/src/components/prose/mod.rs` — add markdown module, add tests
- `biscuit-terminal/lib/src/components/prose/markdown.rs` — **new** markdown pre-processing
- `biscuit-terminal/lib/src/components/prose/browser.rs` — **new** BrowserRenderable impl
- `biscuit-terminal/cli/tests/level2_prose_styling.rs` — add Level-2 markdown tests
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs` — add Apple Terminal markdown tests
- `biscuit-terminal/docs/components/prose.md` — update documentation
- `.opencode/skill/biscuit-terminal/SKILL.md` — update skill

## Estimated Effort

- Phase 1: Small (1–2 hours)
- Phase 2: Medium (4–6 hours)
- Phase 3: Medium (3–4 hours)
- Phase 4: Medium (4–6 hours)
- Phase 5: Small (1–2 hours)

**Total:** ~15–20 hours of focused development time.
)
