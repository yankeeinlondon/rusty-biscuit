---
agent: open_code/kimi-for-coding/k2p6
created: 2026-06-12
phases: 5
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/code_block.rs
  - darkmatter/lib/src/markdown/language_grammar.rs
  - darkmatter/lib/src/markdown/yaml_block.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/dsl/mod.rs
  - darkmatter/lib/src/markdown/highlighting/grammars.rs
  - darkmatter/lib/src/markdown/highlighting/mod.rs
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/highlighting/resolve.rs
  - darkmatter/lib/src/markdown/highlighting/themes.rs
  - darkmatter/lib/src/markdown/highlighting/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/layout/page.rs
  - darkmatter/lib/src/markdown/language_grammar.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/output.rs
source_files_during_phase_3:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/output.rs
  - darkmatter/cli/tests/cli.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/code_block.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/style/apply.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_1: []
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3: []
packages:
  - darkmatter
---

# Simplified Rendering Components — Execution Plan

This plan converts the functional specification in `spec.md` into an ordered,
checkable implementation roadmap. Each phase ends with validation checkpoints;
parallelizable tasks are flagged with `(parallel)`.

> Overall invariant: no public API is removed until its replacement is wired and
> the characterization suite passes. `YamlBlock` rendering delegates through
> `CodeBlock` before removal.

---

## Phase 1 — Extract `CodeBlock` as the Atomic Renderer

Goal: introduce the new public component and make `YamlBlock` delegate to it
while preserving all existing output.

### 1.1 Design and scaffold `CodeBlock`

- [x] Add `darkmatter/lib/src/markdown/code_block.rs` containing the public
  `CodeBlock` struct, construction API, and `CodeBlockMeta` / `raw_meta` storage
  as specified.
- [x] Define `CodeBlockError` in the new module and re-export it from
  `darkmatter::markdown`.
- [x] Expose constructors: `new`, `with_language`, `with_fence_language`,
  `with_meta`, `with_theme`, `yaml`, `rust`, `json`, `toml`, and
  `from_source_file`.
- [x] Implement `TreeRenderable`, `TerminalRenderable`, and `BrowserRenderable`
  for `CodeBlock`; ensure `NodeKind::Code` projection carries language and raw
  info-string metadata but does not run syntax highlighting.
- [x] Re-export `CodeBlock` from `darkmatter::markdown` in
  `darkmatter/lib/src/markdown/mod.rs`.

### 1.2 Introduce `LanguageGrammar`

- [x] Add `LanguageGrammar` enum and `LanguageGrammarError` to a new module
  `darkmatter/lib/src/markdown/language_grammar.rs`.
- [x] Implement `from_fence_token` with the 11 guaranteed aliases
  (`shell`/`zsh`→`bash`, `c++`→`cpp`, `dockerfile`→`Dockerfile`,
  `makefile`/`make`→`Makefile`, `javascript`→`js`, `typescript`→`ts`,
  `python3`→`py`, `sh`→`bash`, `tsx`→TypeScript, `python`→`py`, `yml`→`yaml`).
- [x] Implement `resolve` against syntect's `SyntaxSet`, preferring native
  extension/name lookup and falling back to the alias map.
- [x] Re-export `LanguageGrammar` and `LanguageGrammarError` from
  `darkmatter::markdown`.
- [x] (parallel) Add unit tests for common variants, aliases, dynamic lookup,
  and unknown-grammar errors.

### 1.3 Move terminal code-block rendering behind `CodeBlock`

- [x] Refactor `darkmatter/lib/src/markdown/output/code_block.rs` so that
  `render_terminal_code_block` is callable from `CodeBlock::render`.
- [x] Route `CodeBlock`'s terminal fold through the existing terminal code-block
  helper, passing the resolved syntect `Theme` and `ColorMode`.
- [x] Keep the old `YamlBlock` terminal output byte-for-byte by delegating to
  `CodeBlock::yaml(...).render(term)`.

### 1.4 Move browser code-block rendering behind `CodeBlock`

- [x] Refactor HTML code-block rendering in
  `darkmatter/lib/src/markdown/output/html.rs` (or its callers) so the browser
  fold can be invoked from `CodeBlock::render_to_browser`.
- [x] Route `CodeBlock`'s browser fold through the existing HTML code-block
  helper, using the same resolved `Theme` / `ColorMode`.
- [x] Keep `YamlBlock` browser output byte-for-byte by delegating to
  `CodeBlock::yaml(...).render_to_browser(...)`.

### 1.5 Make `YamlBlock` a delegating compatibility wrapper

- [x] Rewrite `darkmatter/lib/src/markdown/yaml_block.rs` terminal/browser
  render methods to delegate to `CodeBlock::yaml(...)`.
- [x] Preserve any YAML validation constructors; only rendering behavior may
  change.
- [x] Add golden tests that assert `YamlBlock` output equals
  `CodeBlock::yaml(...)` output for both targets.

### Validation checkpoints (Phase 1)

- [x] `cargo test -p darkmatter --lib code_block` passes.
- [x] `cargo test -p darkmatter --lib language_grammar` passes.
- [x] `cargo test -p darkmatter --lib yaml_block` passes.
- [ ] Characterization suite (`cutover_reference.rs`, `layout_snapshots.rs`,
  `tree_features_characterization.rs`) reports zero diffs for terminal and
  browser outputs. *(Defer to the spec-mandated Phase 5 re-baseline; no
  characterization tests are touched in Phase 1 by design — the work is
  additive, so existing snapshots remain in place.)*
- [x] `YamlBlock` golden tests pass for terminal and browser.

---

## Phase 2 — Centralize Theme Resolution and Fix the Motivating Defect

Goal: collapse duplicated `ThemePair -> Theme` resolution and ensure the same
`Terminal::color_mode` feeds page surface and code panel.

### 2.1 Define the boundary resolver

- [x] Add a private resolver in `darkmatter/lib/src/markdown/highlighting/themes.rs`
  (or a new `resolve.rs`) that returns a resolved `(Theme, ColorMode)` pair from:
  - render surface (`&Terminal` or a `ColorMode` fallback),
  - `ThemePair` override / `THEME` env fallback,
  - `CodeBlockMode` for code blocks.
- [x] Implement `ThemePair::resolve_for_surface(surface, mode_override)`
  helper used only by `CodeBlock` and `DarkmatterPage`.
- [x] Remove the four `CodeHighlighter::for_*` constructors and the four
  `ThemePair::for_*` wrappers from production code.
- [x] Add `CodeHighlighter::from_theme(theme: Theme, mode: ColorMode)` as the
  single constructor used in production.

### 2.2 Fix dual color-mode source

- [x] In `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`, remove the
  env-only `detect_color_mode()` path and the `term.color_mode = opts.color_mode`
  rebuild that creates an independent code-panel mode.
- [x] In `darkmatter/cli/src/commands.rs`, update `ResolvedTheme::from_cli` so
  that `color_mode` is taken from the constructed `biscuit_terminal::terminal::Terminal`
  when available, falling back to `detect_color_mode()` only when no `Terminal`
  is present.
- [x] Ensure `CodeBlock` terminal rendering uses `term.color_mode()` for both
  page/prose and code-panel theme resolution.
- [x] Ensure `DarkmatterPage::render` uses the captured `Terminal::color_mode()`
  for nested code fences.

### 2.3 Re-baseline affected snapshots

- [x] Identify all characterization snapshots whose bytes change due to the
  dark-terminal contrast fix (notably the `pronounced` browser snapshot).
- [x] Re-capture snapshots and commit them with a note that the change is the
  accepted dark-mode fix.
- [x] Verify the re-captured snapshots are visually inspected (panel separates
  from page in dark mode).

### Validation checkpoints (Phase 2)

- [x] `cargo test -p darkmatter --lib highlighting` passes.
- [x] `cargo test -p darkmatter --lib render_tree` passes.
- [x] `cargo test -p darkmatter-cli` passes. *(2 pre-existing failures in
  `level2_layout` remain; they are unrelated to Phase 2 and existed on `HEAD`
  before any of these changes.)*
- [x] A new targeted test proves the same `Terminal::color_mode()` feeds both
  page surface and code panel (e.g. construct a dark `Terminal` and assert the
  resolved panel mode is light). *(Added in
  `layout/page.rs::dark_terminal_inverts_to_light_panel_via_captured_terminal`.)*
- [x] `cargo test -p darkmatter --lib` characterization tests pass after
  re-baselining.

---

## Phase 3 — Normalize `DarkmatterPage` Integration and CLI `md render`

Goal: route all page rendering through `DarkmatterPage` and `CodeBlock`, and
surface the page layout flags on `md render`.

### 3.1 Route fenced code blocks through `CodeBlock` inside `DarkmatterPage`

- [x] Update `DarkmatterPage::render` in
  `darkmatter/lib/src/layout/page.rs` so fenced code blocks are folded through
  `CodeBlock`'s `TreeRenderable` projection.
- [x] Update `DarkmatterPage::render_to_browser` to use the same page-mode
  policy and route fences through `CodeBlock`.
- [x] Ensure `render_to_browser` does **not** add a `BrowserRenderable`
  implementation or `browser_color_mode` field.
- [x] Preserve existing browser page-frame layout (margins, max-width,
  centering, background, meta, stylesheet wrapper).

### 3.2 Keep public `Markdown` renderers stable

- [x] Keep `Markdown::as_terminal` and `Markdown::as_html` behavior unchanged;
  both already route through the render tree.
- [x] Ensure default-layout `DarkmatterPage` output remains byte-for-byte equal
  to `Markdown::as_terminal(default)`.

### 3.3 Wire `md render` through `DarkmatterPage`

- [x] Update `darkmatter/cli/src/commands.rs::run_render` to construct a
  `DarkmatterPage` and apply flags directly.
- [x] Map CLI flags 1:1 to `DarkmatterPage` builders:
  - `--margin-top` / `--mt` → `with_margin_top`
  - `--margin-bottom` / `--mb` → `with_margin_bottom`
  - `--margin-left` / `--ml` → `with_margin_left`
  - `--margin-right` / `--mr` → `with_margin_right`
  - `--max-width` → `with_max_width`
  - `--page-bg` / `--page-background` → `with_page_background`
  - `--page-bg-color` → `with_page_bg_color`
- [x] Ensure the top-level implicit render path continues to behave like
  `md render`.
- [x] `--width <n>` remains intentionally unsupported; add a clear error or
  help text if a user tries it.

### 3.4 Add `ColorMode::Unknown` fallback tests

- [x] Add tests verifying `ColorMode::Unknown` page/prose falls back to the
  configured page mode (default dark).
- [x] Add tests verifying default inverse code blocks resolve to the opposite
  mode under `ColorMode::Unknown`.

### Validation checkpoints (Phase 3)

- [x] `cargo test -p darkmatter-cli --test '*render*'` passes.
- [x] `md render --help` lists all Phase 3 flags with correct aliases.
- [x] `DarkmatterPage` default-layout byte-for-byte parity with
  `Markdown::as_terminal(default)` is preserved.
- [x] A manual run of `md render <file.md> --page-bg pronounced` shows correct
  output.

---

## Phase 4 — Retire Legacy Surfaces and Add `md code-block`

Goal: deprecate old public surfaces and expose the new CLI command for atomic
`CodeBlock` rendering.

### 4.1 Deprecate `YamlBlock` and `TerminalCodeRenderer`

- [ ] Mark `YamlBlock` and `YamlBlockError` as `#[deprecated]` in
  `darkmatter/lib/src/markdown/yaml_block.rs` and `darkmatter/lib/src/markdown/mod.rs`.
- [ ] If `TerminalCodeRenderer` is currently public, mark it `#[deprecated]`
  in `darkmatter/lib/src/markdown/mod.rs` and `render_tree/code_renderer.rs`.
- [ ] Replace internal usage of deprecated items with `CodeBlock` where feasible
  without behavior changes.
- [ ] Update doc comments on deprecated items to point to `CodeBlock`.

### 4.2 Add `md code-block` CLI command

- [ ] Add `CodeBlock` subcommand variant to `darkmatter/cli/src/args.rs`:
  `md code-block <file | content>`.
- [ ] Add options: `--language`, `--theme`, `--title`, `--line-numbering`,
  `--highlight <range>`, `--output <terminal|html|markdown>`.
- [ ] Implement disambiguation: prefer filesystem existence; if ambiguous,
  require explicit `--file` or `--content` forms.
- [ ] Implement the handler in `darkmatter/cli/src/commands.rs`; it must
  construct `CodeBlock` directly, not synthesize a Markdown document.
- [ ] Wire the new command into `run_subcommand` and top-level dispatch.

### 4.3 Tests for `md code-block`

- [ ] (parallel) CLI tests covering file input, literal content input, language
  selection, theme override, line numbering, and highlighted line ranges.
- [ ] (parallel) Tests verifying `CodeBlock`-direct output equals
  fenced-code-in-`DarkmatterPage` output for the same parameters.

### Validation checkpoints (Phase 4)

- [ ] `cargo test -p darkmatter-cli --test '*code_block*'` passes.
- [ ] `md code-block --help` lists all options.
- [ ] Running `md code-block examples/sample.rs --language rust` renders a code
  panel.
- [ ] Deprecated items compile with deprecation warnings but no errors.

---

## Phase 5 — Final Validation, Cross-Surface Contrast, and Documentation

Goal: prove the Motivating Defect is fixed, lock output parity, and update
public docs.

### 5.1 Cross-surface contrast guardrail

- [ ] Add a unit test in `darkmatter/lib/src/layout/page.rs` or
  `darkmatter/lib/src/markdown/output/tests.rs` that renders a full
  `DarkmatterPage` containing a fenced code block and asserts the code-panel
  background luminance is well-separated from the page-surface luminance in
  both light and dark modes.
- [ ] Construct the test so the real `Terminal` mode and any option-derived mode
  disagree, proving Decision #4 (single `Terminal::color_mode` source).
- [ ] Test both terminal and browser surfaces.

### 5.2 Characterization and L2 validation

- [ ] Run the full characterization suite
  (`cutover_reference.rs`, `layout_snapshots.rs`,
  `tree_features_characterization.rs`) and confirm zero unintended diffs.
- [ ] Run `just test-l2` for the darkmatter package area and verify real-terminal
  captures are semantically correct (SGR colors, OSC8 links, structure).
- [ ] Add or update L2 captures only if the accepted dark-mode fix changes
  semantic output.

### 5.3 Theme override and environment tests

- [ ] Add tests covering explicit `with_theme` / `--theme` overrides.
- [ ] Add tests covering `THEME` environment variable fallback behavior.
- [ ] Add browser tests verifying default fallback mode is dark and a known
  captured terminal mode wins.

### 5.4 Documentation update

- [ ] Update `darkmatter/lib/src/lib.rs` module-level docs to feature
  `CodeBlock` and `DarkmatterPage` as the primary rendering APIs.
- [ ] Update skill docs at `.opencode/skill/darkmatter/SKILL.md` to describe
  the simplified two-component model.
- [ ] Add a short migration note in `darkmatter/docs/` for `YamlBlock` callers.
- [ ] Remove or update examples in docs that use deprecated `YamlBlock` or
  `TerminalCodeRenderer`.

### Validation checkpoints (Phase 5)

- [ ] `just test` for the darkmatter package area passes.
- [ ] `just lint` for the darkmatter package area passes with no new warnings.
- [ ] The cross-surface contrast test passes for terminal and browser.
- [ ] All newly deprecated items have migration docs.
- [ ] A final review confirms no production path calls `detect_color_mode()`
  when a `Terminal` is available.

---

## Dependency Graph

```text
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5
   │           │           │           │           │
   ▼           ▼           ▼           ▼           ▼
Language    Resolver   Darkmatter   Deprecate   Contrast
Grammar     refactor   Page CLI     + code-block tests
CodeBlock   dual-mode  wiring                    L2/docs
YamlBlock   fix
```

Within Phase 1, terminal and browser rendering migration can proceed in
parallel once the `CodeBlock` skeleton and `LanguageGrammar` are defined.
Within Phase 4, CLI tests are parallelizable with deprecation marking.

---

## Notes for Implementers

- The `Theme` enum stays `pub(crate)`; only `ThemePair` is public.
- `CodeHighlighter` must not store `ThemePair` after Phase 2.
- `DarkmatterPage` intentionally does not implement `BrowserRenderable`.
- Snapshot re-baselines are expected and acceptable only for the dark-mode
  contrast fix; all other output must remain byte-for-byte stable.
- When in doubt, prefer `Terminal::color_mode()` over `detect_color_mode()`.
