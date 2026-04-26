---
phases: 6
created: 2026-04-26
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: [darkmatter]
---

# YAML Component Execution Plan

Derived from:
- `darkmatter/features/_unscheduled/yaml-component/spec.md`
- `darkmatter/features/_unscheduled/yaml-component/tech-design.md`

## Overview

Add `YamlBlock` to `darkmatter::markdown` as a typed, validated YAML payload that renders through the same terminal and browser code-block highlighting paths used by normal Markdown `yaml` fences. The work is mostly additive, but it includes a small private refactor of existing code-block helpers so Markdown rendering and `YamlBlock` share the same `syntect` / `two-face` behavior.

## Phase 1: Baseline Inspection and API Confirmation

**Goal:** Confirm current implementation seams before editing shared renderer code.

**Dependencies:** None.

### Step 1.1: Confirm package and dependency state

**Files / commands:**
- `cargo metadata --no-deps --format-version 1`
- `darkmatter/lib/Cargo.toml`

**Work:**
- Confirm `darkmatter` package name and workspace membership from `cargo metadata`.
- Confirm `serde_yaml_ng`, `thiserror`, `syntect`, `two-face`, and `biscuit-terminal` are already available to `darkmatter/lib`.
- Record that no `docs/dependencies.md` update is needed unless a dependency is added during implementation.

**Validation checkpoint:** Implementation can proceed without Cargo dependency edits.

### Step 1.2: Verify existing frontmatter and rendering APIs

**Files:**
- `darkmatter/lib/src/markdown/mod.rs`
- `darkmatter/lib/src/markdown/frontmatter.rs`
- `darkmatter/lib/src/markdown/types.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/markdown/output/html.rs`
- `biscuit-terminal/lib/src/components/renderable.rs`

**Work:**
- Confirm `Markdown::try_from_content` is the fail-fast frontmatter parser to use.
- Confirm malformed frontmatter maps to `MarkdownError::FrontmatterParse`.
- Confirm current private terminal helper is `highlight_code`.
- Confirm current private browser helper is `highlight_code_block`.
- Confirm `Renderable` requires `render`, `layout`, `layout_mut`, `as_any`, and optional `is_block_level`.
- Confirm `BrowserRenderable` requires `render_to_browser` and `as_any`.

**Validation checkpoint:** Notes identify exact functions to move or wrap in Phase 2.

## Phase 2: Shared Code-Block Rendering Helpers

**Goal:** Extract reusable private helpers for terminal and browser code-block rendering without changing Markdown output.

**Dependencies:** Phase 1.

### Step 2.1: Add `output::code_block` module

**Files:**
- `darkmatter/lib/src/markdown/output/code_block.rs`
- `darkmatter/lib/src/markdown/output/mod.rs`

**Work:**
- Create `code_block.rs` as `pub(crate)`.
- Move or wrap the reusable body of terminal `highlight_code` into `render_terminal_code_block`.
- Move or wrap the reusable body of HTML `highlight_code_block` into `render_html_code_block`.
- Keep helper APIs private to `darkmatter/lib`; do not expose a public general code-block API.
- Prefer small options structs only if that reduces coupling; otherwise pass existing `TerminalOptions` / `HtmlOptions` to minimize churn.

**Validation checkpoint:** `cargo check -p darkmatter` compiles after adding the module.

### Step 2.2: Route Markdown terminal code fences through the shared helper

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/src/markdown/output/code_block.rs`

**Work:**
- Replace private `highlight_code` call sites with `code_block::render_terminal_code_block`.
- Preserve existing behavior for:
  - syntax lookup fallback to plain text
  - `LinesWithEndings`
  - line numbers
  - highlighted lines from `CodeBlockMeta`
  - ANSI reset behavior
  - theme background and padding rows
- Keep Mermaid and non-code rendering paths unchanged.

**Validation checkpoint:** Existing terminal code-block tests pass unchanged.

### Step 2.3: Route Markdown HTML code fences through the shared helper

**Files:**
- `darkmatter/lib/src/markdown/output/html.rs`
- `darkmatter/lib/src/markdown/output/code_block.rs`

**Work:**
- Replace private `highlight_code_block` call sites with `code_block::render_html_code_block`.
- Preserve existing behavior for:
  - code-block title
  - optional line-number table
  - `<div class="code-block">` wrapper
  - `<pre><code class="language-...">` output when line numbers are disabled
  - HTML escaping and highlighted spans

**Parallelizable with:** Step 2.2 after Step 2.1 lands.

**Validation checkpoint:** Existing HTML code-block tests pass unchanged.

### Step 2.4: Add parity coverage for the refactor

**Files:**
- Existing test modules in `terminal.rs`, `html.rs`, or new focused tests in `code_block.rs`.

**Work:**
- Add a terminal helper test comparing helper output to the previous Markdown fence output shape for a simple YAML snippet.
- Add an HTML helper test asserting a YAML block includes `language-yaml` and escaped scalar text.
- Add explicit light and dark `ColorMode` helper tests by constructing `CodeHighlighter::new(ThemePair::Github, ColorMode::{Light,Dark})`.

**Validation checkpoint:** `cargo test -p darkmatter code_block` or the nearest module-scoped test target passes.

## Phase 3: `YamlBlock` Data Type and Constructors

**Goal:** Add the public component type, fail-fast constructors, and error mapping.

**Dependencies:** Phase 1. Phase 2 is not required for constructors, so this phase can start in parallel after API confirmation.

### Step 3.1: Define and export `YamlBlock`

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`
- `darkmatter/lib/src/markdown/mod.rs`

**Work:**
- Add `mod yaml_block;`.
- Re-export `YamlBlock` and `YamlBlockError` from `darkmatter::markdown`.
- Define:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct YamlBlock`
  - private `yaml: String`
  - private `layout: biscuit_terminal::utils::layout::Layout`
- Add `yaml(&self) -> &str` and `into_yaml(self) -> String`.

**Validation checkpoint:** `cargo check -p darkmatter` sees the public re-export.

### Step 3.2: Add `YamlBlockError`

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`

**Work:**
- Add `#[derive(Debug, thiserror::Error)]`.
- Add variants:
  - `Io(#[from] std::io::Error)`
  - `YamlParse(#[from] serde_yaml_ng::Error)`
  - `MarkdownParse(#[from] crate::markdown::MarkdownError)`
- Use error messages aligned with the technical design.

**Validation checkpoint:** Pattern matching on each error variant compiles in unit tests.

### Step 3.3: Implement raw YAML and file constructors

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`

**Work:**
- Add private `validate_yaml(&str) -> Result<(), serde_yaml_ng::Error>` using `serde_yaml_ng::from_str::<serde_yaml_ng::Value>`.
- Implement `YamlBlock::new<T: Into<String>>`:
  - convert to `String`
  - validate
  - store original bytes unchanged
- Implement `YamlBlock::from_yaml_file<P: AsRef<Path>>`:
  - `std::fs::read_to_string`
  - delegate to `YamlBlock::new`
  - preserve `Io` vs `YamlParse` mapping.

**Validation checkpoint:** Constructor tests cover valid YAML, malformed YAML, missing YAML file, and malformed YAML file.

### Step 3.4: Implement Markdown frontmatter constructors

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`

**Work:**
- Implement `YamlBlock::from_markdown_content<T: Into<String>>`:
  - call `Markdown::try_from_content`, not infallible `From`.
  - if frontmatter is empty, use literal `{}`.
  - otherwise serialize the `Frontmatter` / `FrontmatterMap` back to YAML with `serde_yaml_ng::to_string`.
  - validate serialized YAML with `validate_yaml`.
  - store serialized YAML text.
- Implement `YamlBlock::from_markdown_file<P: AsRef<Path>>`:
  - read file
  - delegate to `from_markdown_content`
  - preserve `Io` vs parse error mapping.
- Do not scan Markdown body fences.

**Validation checkpoint:** Tests cover no frontmatter, valid frontmatter, malformed frontmatter, body ignored, and missing Markdown file.

**Parallelizable work:** Steps 3.2 and 3.3 can proceed once Step 3.1 creates the module. Step 3.4 can proceed once Step 3.2 is available.

## Phase 4: `Renderable` and `BrowserRenderable` Implementations

**Goal:** Render `YamlBlock` through the shared code-block helpers using the same `yaml` language path as Markdown fences.

**Dependencies:** Phase 2 and Phase 3.

### Step 4.1: Implement terminal rendering

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`
- `darkmatter/lib/src/markdown/output/code_block.rs`

**Work:**
- Implement `biscuit_terminal::components::renderable::Renderable for YamlBlock`.
- In `render(&self, term: &Terminal)`:
  - build the same default code highlighter path used by Markdown terminal output, preferably via `TerminalOptions::default()`.
  - call `render_terminal_code_block(self.yaml(), "yaml", ...)`.
  - set line numbers off and `CodeBlockMeta::default()`.
  - keep the `term` argument accepted for trait compatibility and future width/capability use.
  - use a plain escaped fallback only if the helper returns an error.
- Implement `layout`, `layout_mut`, `as_any`, and `is_block_level() -> true`.

**Validation checkpoint:** Rendering `YamlBlock::new("foo: 1")` returns ANSI code-block output rather than raw unstyled YAML when color support is available.

### Step 4.2: Implement browser rendering

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`
- `darkmatter/lib/src/markdown/output/code_block.rs`

**Work:**
- Implement `biscuit_terminal::components::renderable::BrowserRenderable for YamlBlock`.
- In `render_to_browser(&self)`:
  - build the same default highlighter path used by Markdown HTML output.
  - call `render_html_code_block(self.yaml(), "yaml", ...)`.
  - set line numbers off and `CodeBlockMeta::default()`.
  - ensure fallback output is `<pre><code class="language-yaml">...</code></pre>` with HTML-escaped payload.
- Do not add `YamlBlock`-specific theme fields or frontmatter knobs.

**Parallelizable with:** Step 4.1 after shared helpers exist.

**Validation checkpoint:** Browser output contains `<pre><code class="language-yaml">` and escapes `<`, `>`, and `&` in YAML scalar content.

### Step 4.3: Add rendering parity tests

**Files:**
- `darkmatter/lib/src/markdown/yaml_block.rs`
- Existing output test modules if helper-level assertions are clearer.

**Work:**
- Compare terminal render of `YamlBlock::new(X)` with terminal render of a `Markdown` document containing only:
  ```text
  ```yaml
  X
  ```
  ```
- Normalize only newline differences caused by `Renderable::render` composition semantics.
- Compare browser render against Markdown YAML fence expectations by checking required code element, language class, escaping, and highlighted span structure where stable.
- Exercise both light and dark mode through explicit helper tests if environment-driven `detect_color_mode()` is not deterministic in unit tests.

**Validation checkpoint:** Acceptance criteria 6, 7, and 8 have direct automated coverage.

## Phase 5: Documentation and Local Knowledge Updates

**Goal:** Document the new component and update repo-maintenance artifacts required by AGENTS.md.

**Dependencies:** Phase 4 behavior should be stable.

### Step 5.1: Update library README documentation

**Files:**
- `darkmatter/lib/README.md`

**Work:**
- Add a short `YamlBlock` example showing:
  - construction from raw YAML
  - construction from Markdown frontmatter
  - terminal or browser rendering via the implemented traits.
- State that `YamlBlock` validates YAML but stores and renders raw YAML text.

**Validation checkpoint:** README example imports compile as doctest if README examples are tested, or are manually checked against actual exports.

### Step 5.2: Update package-level README if component lists exist

**Files:**
- `darkmatter/README.md`

**Work:**
- If reusable Markdown/rendering components are listed, add `YamlBlock`.
- If no such list exists, do not force a broad README rewrite.

**Validation checkpoint:** Documentation remains scoped to public behavior added by this feature.

### Step 5.3: Update the local darkmatter skill

**Files:**
- `.claude/skills/darkmatter/SKILL.md`

**Work:**
- Add a concise `YamlBlock` section describing:
  - constructors
  - validation via `serde_yaml_ng`
  - frontmatter-only Markdown ingestion
  - shared terminal/browser syntax highlighting
  - no tree view or custom YAML renderer.

**Validation checkpoint:** Skill reflects the architecture and workflow changes required by AGENTS.md drift-maintenance rules.

**Parallelizable work:** Steps 5.1, 5.2, and 5.3 can be drafted in parallel after public API shape is fixed.

## Phase 6: Final Validation and Regression Sweep

**Goal:** Prove the feature is complete and existing Markdown code fences did not regress.

**Dependencies:** Phases 2 through 5.

### Step 6.1: Run focused test suite

**Commands:**
- `cargo test -p darkmatter yaml_block`
- `cargo test -p darkmatter code_block`
- `cargo test -p darkmatter output::terminal`
- `cargo test -p darkmatter output::html`

**Work:**
- Run focused tests for constructors, helper refactor, terminal output, and HTML output.
- Fix failures before broader validation.

**Validation checkpoint:** All focused tests pass.

### Step 6.2: Run package checks

**Commands:**
- `cargo check -p darkmatter`
- `cargo test -p darkmatter`

**Work:**
- Run full package compilation and test suite.
- Confirm no behavior changed for existing Markdown YAML fences except through the shared helper refactor preserving output.

**Validation checkpoint:** `darkmatter` package checks pass.

### Step 6.3: Acceptance criteria audit

**Work:**
- Verify each acceptance criterion from `spec.md` maps to at least one test:
  1. `new` valid and invalid YAML.
  2. `from_yaml_file` missing and malformed file.
  3. `from_markdown_content` without frontmatter yields `{}`.
  4. `from_markdown_content` stores only frontmatter.
  5. `from_markdown_file` missing file maps to `Io`.
  6. Terminal parity with Markdown YAML fence.
  7. Browser output contains YAML code block and uses existing classes/variables.
  8. Light and dark color-mode paths are exercised.
- Confirm docs and skill updates match actual API names.

**Validation checkpoint:** No unchecked acceptance criteria remain.

### Step 6.4: Optional root-level validation

**Commands:**
- `just test darkmatter` if supported by the current justfile shape.
- Otherwise keep direct `cargo test -p darkmatter` as the authoritative package validation.

**Work:**
- Use root `just` only if it targets this package area correctly.
- Do not rely on root `just` coverage for unrelated workspace members.

**Validation checkpoint:** Final implementation status is reproducible from documented commands.

## Parallel Work Summary

- Phase 3 can begin in parallel with Phase 2 after Phase 1 confirms APIs.
- Phase 2 Step 2.2 and Step 2.3 can proceed in parallel after `output::code_block` exists.
- Phase 4 Step 4.1 and Step 4.2 can proceed in parallel once shared helpers and constructors are available.
- Phase 5 documentation updates can proceed in parallel after the public API is stable.
- Test writing can be split by area: constructor tests, terminal parity tests, browser parity tests, and docs/skill verification.
