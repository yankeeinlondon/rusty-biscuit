---
phases: 7
created: 2026-06-02
start_phase: 1
packages:
  - renderable
  - biscuit-terminal
source_spec: spec.md
source_files_during_phase_1:
  - renderable/src/style.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
  - biscuit-terminal/lib/src/render_tree/style.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/components/prose/styles.rs
  - biscuit-terminal/lib/src/components/text_block.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/renderable/style.md
source_files_during_phase_2:
  - renderable/src/tree/render/markdown.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - renderable
source_files_during_phase_3:
  - biscuit-terminal/lib/src/components/prose/parity.rs
  - biscuit-terminal/lib/src/components/prose/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - biscuit-terminal
---

# Execution Plan: Prose to Shared Render Tree

Spec: `spec.md` (this directory)

## Overview

Collapse `biscuit-terminal::components::Prose` onto the shared
`renderable::tree::RenderNode` pipeline. The implementation lands shared tree
fidelity prerequisites first, captures the current bespoke Prose output as a
parity oracle, rewrites Prose parsing to emit tree nodes directly, delegates
all Prose target rendering to the shared renderers, then deletes the private
`ProseDocument` IR and bespoke emitters.

**Success criteria:** Prose has no component-local rendering IR, all Prose
targets render through the shared tree renderers, `inverse` and MarkdownPlus
inline `Style` are first-class shared capabilities, Prose output is byte-stable
except for the approved `<hidden>` removal and browser inverse CSS whitespace,
and Prose performance is neutral or better unless a documented cutover
performance gate accepts the regression.

---

## Phase 1 - Add Shared Inverse Emphasis

*Add `inverse` to the shared style model before Prose depends on it.*

### Tasks

- [ ] Add `inverse: bool` to `renderable::style::TextEmphasis` with serde
  backward compatibility so older serialized render trees deserialize with
  `inverse == false`.

- [ ] Add `EmphasisLayer::Inverse` with SGR set/reset operations `7` and `27`.

- [ ] Update `TextEmphasis::is_empty`, inheritance handling, and SGR operation
  generation so inverse behaves as a peer of `dim`, `blink`, `underline`, and
  the other existing emphasis fields.

- [ ] Update the `biscuit-terminal` terminal style-layer bridge so nested
  inverse spans restore the parent inverse state after a child span ends.

- [ ] Update the shared browser tree renderer to lower inverse to
  `filter: invert(1)` on the styled element.

- [ ] Confirm the shared markdown renderer degrades inverse to inner text for
  both Markdown and MarkdownPlus.

- [ ] Remove or update any comment or rustdoc that enumerates `TextEmphasis`
  fields and would drift after adding inverse.

### Validation

- [ ] Add focused tests for SGR `7` output and SGR `27` reset behavior.

- [ ] Add nested inverse tests proving a child reset restores an inverse parent
  instead of clearing all emphasis state.

- [ ] Add a serde test proving missing `inverse` data deserializes as `false`.

- [ ] Add browser renderer coverage asserting `filter: invert(1)` is emitted.

- [ ] Add markdown renderer coverage asserting inverse produces no Markdown
  sigil and preserves child text.

- [ ] Run `cargo test -p renderable --lib` and the focused
  `biscuit-terminal` render-tree/style tests touched by this phase.

### Parallelizable Work

- [ ] Browser, markdown, and terminal lowering tests can be implemented in
  parallel after the shared `TextEmphasis` field compiles.

---

## Phase 2 - Add MarkdownPlus Inline Style Lowering

*Teach the shared markdown renderer to preserve inline style in MarkdownPlus
before Prose routes Markdown output through it.*

### Tasks

- [x] Extend `renderable/src/tree/render/markdown.rs::render_span` so
  `MarkdownDialect::MarkdownPlus` emits inline HTML for nodes carrying a
  concrete `Style`.

- [x] Convert supported inline `Style` fields to concrete CSS declarations:
  foreground color, background color, and underline variants needed for Prose
  parity.

- [x] Preserve existing class-based span behavior, and emit one `<span>` with
  both `class` and `style` attributes when both are present.

- [x] Escape literal `<`, `>`, and `&` in MarkdownPlus inline-HTML bodies while
  preserving the renderer's existing Markdown sigil escaping rules.

- [x] Keep plain Markdown behavior unchanged: strict mode rejects classed or
  styled spans where strictness already applies, warn mode records a lossy
  diagnostic and emits inner text, and lossy mode emits inner text.

- [x] Audit Markdown link destination escaping in the shared renderer against
  Prose's current bespoke behavior before Prose delegates to this path.

### Validation

- [x] Add MarkdownPlus renderer tests for foreground color, background color,
  underline variants, HTML body escaping, class+style coalescing, and Markdown
  sigil escaping inside styled spans.

- [x] Add plain Markdown strict/warn/lossy tests for styled spans with no
  classes.

- [x] Add or update link destination tests covering parentheses, backslashes,
  whitespace angle-bracket wrapping, and line-ending degradation to spaces.

- [x] Run `cargo test -p renderable --lib -- markdown` or the closest focused
  markdown renderer filter.

### Parallelizable Work

- [x] CSS declaration serialization and Markdown escaping tests can be developed
  in parallel after the target `render_span` branch is identified.

---
## Phase 3 - Capture Current Prose Parity Snapshots

*Pin today's bespoke Prose output before changing parsing or deleting emitters.*

### Tasks

- [x] Inventory existing Prose tests and identify which target policies are
  already covered for terminal, browser, Markdown, and MarkdownPlus.

- [x] Add a representative Prose fixture corpus covering nested style
  restoration, colors, background colors, dim, blink, underline variants,
  `<inverse>`, `<hidden>`, links, path-like links, OSC8 and non-OSC8 terminal
  behavior, unknown tags, escaped literal markup, code fences, layout, margin,
  and word wrapping.

- [x] Capture current bespoke terminal output snapshots, including capability
  variants for OSC8 support and underline degradation.

- [x] Capture current bespoke browser output snapshots, including HTML escaping,
  href attribute escaping, code block escaping, and inverse filter output.

- [x] Capture current bespoke Markdown and MarkdownPlus snapshots, including
  Markdown link destination escaping and MarkdownPlus color spans.

- [x] Label `<hidden>` expectations as intentionally removed after the cutover
  so future diffs are reviewed as an approved behavior drop, not a regression.

- [x] Keep these tests pinned to the current bespoke emitters until Phase 5
  flips Prose rendering to the tree.

### Validation

- [x] Run the focused Prose test filters and confirm the new snapshots pass on
  the pre-migration bespoke path.

- [x] Run `cargo test -p biscuit-terminal --lib -- prose` or the closest
  existing Prose-focused filter.

- [x] Record any existing unrelated failures before changing production Prose
  code.

### Parallelizable Work

- [x] Terminal, browser, and markdown snapshot fixtures can be authored in
  parallel once the shared fixture corpus shape is agreed.

---

## Phase 4 - Parse Prose Directly Into RenderNode

*Replace the private `ProseDocument` build path with direct shared tree nodes
while keeping public embedding APIs stable.*

### Tasks

- [ ] Rewrite the Prose parser so bracket-tag tokens build `RenderNode` values
  directly instead of `ProseDocument` / `ProseNode` / `ProseStyle`.

- [ ] Reuse the existing `prose/tree.rs::node_to_render_node` mapping shapes:
  text to `Text`, bold to `Strong`, italic to `Emphasis`, strikethrough to
  `Delete`, styled spans to `Span` with `Style`, links to `Link`, and code
  blocks to `Code`.

- [ ] Map `<inverse>` and `<reverse>` to `TextEmphasis::inverse`.

- [ ] Remove `<hidden>` from the parser's semantic style handling while keeping
  unknown or unsupported tags visible as inert text according to current Prose
  unknown-tag behavior.

- [ ] Preserve Prose link resolution semantics for `http`, `https`, `file`,
  `mailto`, absolute paths, `./` paths, and package/repo-root fallback
  relative paths.

- [ ] Keep `Prose::to_render_nodes()` as the container embedding API returning
  the parsed inline/mixed node sequence.

- [ ] Implement the document-shaped wrapper for `TreeRenderable::render_tree()`
  so contiguous top-level inline nodes become `Paragraph` blocks and top-level
  `Code` nodes remain block children under `RenderNode::root`.

- [ ] Preserve `Prose::with_layout`, margin helpers, and word-wrap settings via
  `TreeRenderable::tree_layout()` and renderer options.

- [ ] Update behavior-adjacent rustdoc and inline comments that still describe
  `ProseDocument`, `ProseNode`, `ProseStyle`, hidden style handling, or
  bespoke parsing-to-rendering ownership.

### Validation

- [ ] Add parser/unit tests that assert direct tree output for text, nested
  styles, links, code blocks, inverse, unknown tags, and escaped literal
  markup.

- [ ] Add tree validation tests for the `TreeRenderable::render_tree()` root
  wrapper, including mixed inline content and block code.

- [ ] Add layout tests proving Prose layout and wrapping still affect rendered
  output through the tree path.

- [ ] Run `cargo test -p biscuit-terminal --lib -- prose` and
  `cargo test -p renderable --lib` after the parser compiles.

### Parallelizable Work

- [ ] Link resolution preservation and root/paragraph wrapper tests can be
  implemented in parallel after the direct parser output type is in place.

---

## Phase 5 - Delegate Prose Rendering To Shared Renderers

*Flip Prose target traits to the shared tree renderers and compare against the
Phase 3 oracle.*

### Tasks

- [ ] Change `TerminalRenderable` for `Prose` to render the parsed tree through
  the shared terminal tree renderer.

- [ ] Change `BrowserRenderable` for `Prose` to render the parsed tree through
  the shared browser tree renderer and return the same ready fragment shape as
  callers expect today.

- [ ] Change `MarkdownRenderable` for `Prose` to render the parsed tree through
  the shared markdown renderer for both Markdown and MarkdownPlus dialects.

- [ ] Ensure terminal links still render OSC8 only when supported and degrade to
  `[description](href)` otherwise.

- [ ] Ensure browser text, code blocks, and href attributes are escaped through
  the shared path with no double escaping.

- [ ] Ensure Markdown link descriptions are not double-escaped and destinations
  retain the bespoke escaping rules pinned in Phase 3.

- [ ] Diff every Phase 3 snapshot against the tree-rendered output.

- [ ] Classify any diff as approved (`<hidden>` removal or browser inverse CSS
  whitespace normalization) or blocking.

- [ ] Fix every blocking diff before proceeding to deletion.

### Validation

- [ ] All Phase 3 Prose snapshots pass on the shared tree path with only
  approved snapshot updates.

- [ ] Run focused terminal capability tests for OSC8 and underline degradation.

- [ ] Run focused BrowserRenderable and MarkdownRenderable Prose tests.

- [ ] Run `cargo test -p biscuit-terminal --lib -- prose` and any integration
  tests that cover Prose in containers such as block quotes, status blocks,
  lists, tables, and file-system output.

### Parallelizable Work

- [ ] Terminal, browser, and markdown trait delegation can be implemented in
  parallel after Phase 4 exposes the stable parsed tree output.

---

## Phase 6 - Delete Prose Local IR And Bespoke Emitters

*Remove the compatibility code after the shared tree path is proven equivalent.*

### Tasks

- [ ] Delete `biscuit-terminal/lib/src/components/prose/ir.rs` and all
  remaining `ProseDocument` and `ProseNode` references. Retain `ProseStyle`,
  moved to `prose/styles.rs` as a parser-local tag-intent helper (not a
  rendering IR).

- [ ] Delete `biscuit-terminal/lib/src/components/prose/terminal.rs`,
  `browser.rs`, and `to_markdown.rs` after their behavior is covered by shared
  renderers and snapshots.

- [ ] Fold any still-useful helper from `prose/tree.rs` into the direct parser
  output path, then delete dead projection-only helpers.

- [ ] Remove module declarations, imports, tests, and fixture glue that only
  existed for the deleted bespoke emitters.

- [ ] Search the workspace for `ProseDocument`, `ProseNode`, `ProseStyle`,
  `to_markdown.rs`, `prose::terminal`, and `prose::browser`; resolve every
  remaining reference deliberately.

- [ ] Update `renderable/docs/components.md` so Prose's IR state flips from
  component-local IR to tree render only.

- [ ] Update Prose docs and changelog-level notes to document the `<hidden>`
  removal.

- [ ] Update `.claude/skills/renderable/SKILL.md` or linked topic docs if they
  still describe Prose as a component-local IR holdout.

### Validation

- [ ] `rg "ProseDocument|ProseNode|ProseStyle|render_bespoke|fallback_render"
  biscuit-terminal renderable darkmatter` returns no Prose-local IR or deleted
  emitter references except historical feature docs.

- [ ] Run `cargo test -p biscuit-terminal --lib -- prose`.

- [ ] Run `cargo test -p biscuit-terminal --lib -- render_tree` or the closest
  available render-tree-focused filter.

- [ ] Run `cargo test -p renderable --lib`.

### Parallelizable Work

- [ ] Documentation updates and dead-reference cleanup can proceed in parallel
  after the production files are deleted.

---

## Phase 7 - Benchmark And Final Cutover Validation

*Verify the hot-path performance claim and run the broader gates needed before
the parent tree cutover consumes this work.*

### Tasks

- [ ] Add or update a Prose render benchmark covering terminal, browser,
  Markdown, and MarkdownPlus over small, medium, and tag-dense corpora.

- [ ] Capture a pre-delete or pre-flip baseline if Phase 3 did not already add
  one; otherwise compare the new tree-only benchmark results against the saved
  bespoke baseline.

- [ ] Run the Prose benchmark and record the middle estimates and any material
  regression in the feature directory or the existing renderable baseline doc
  used by the package.

- [ ] If a material regression appears, either optimize before closure or
  document why it satisfies the parent cutover's accepted mild-regression gate.

- [ ] Run the focused renderable and biscuit-terminal tests touched by this
  feature.

- [ ] Run the broader package-area validation command from the renderable or
  biscuit-terminal justfile when feasible in the implementation environment.

- [ ] Review READMEs, docs, and skills for drift caused by public behavior
  changes, especially the hidden-tag removal and Prose's new tree-only status.

### Validation

- [ ] Prose benchmarks show neutral-or-better performance, or any accepted
  regression is explicitly documented with comparison numbers.

- [ ] All focused Prose parity snapshots remain green after deletion.

- [ ] `cargo test -p renderable --lib` passes.

- [ ] `cargo test -p biscuit-terminal --lib -- prose` passes.

- [ ] The implementation team records any skipped broad validation command with
  the exact reason it could not be run.

### Parallelizable Work

- [ ] Benchmark analysis, documentation drift checks, and final reference
  searches can be done in parallel after Phase 6 deletion is complete.
