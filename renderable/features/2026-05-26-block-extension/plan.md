---
phases: 4
created: 2026-05-26
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/block/rule_processor.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/block/mod.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/render_tree/block_extension.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/render_tree/fold.rs
  - darkmatter/lib/src/markdown/render_tree/span.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/render_tree/block_extension.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - darkmatter
---

# Execution Plan: Block Extension — HR-Attribute Lift

Spec: `spec.md` (this directory)

## Overview

Lift HR-attribute detection out of the inline-span transport
(`span.rs` / `SpannedInlineEvent`) into a dedicated offset-aware block
post-processor that sits between `pulldown-cmark` and the inline-span
dispatcher. This unblocks the sibling inline-span spec's deletion of
`SpannedInlineStyleProcessor`.

**Success criteria:** byte-identical output on every existing HR fixture,
same source-span policy, same warnings, `SpannedRuleProcessor` removed from
the active chain.

---

## Phase 1 — Extract Shared HR Parser

*Consolidate `RuleProcessor::parse_attributes` and `try_parse_hr_attrs` into
one canonical helper so both the legacy path and the new block-extension
processor use identical parsing and warning behavior.*

### Tasks

- [ ] Create `parse_hr_attribute_block()` in `darkmatter/lib/src/markdown/block/rule_processor.rs`
  - Returns a structured result type carrying `Option<HorizontalRuleAttrs>` plus
    any warning data (unknown keys, non-scalar fields, legacy `style`
    deprecation flag).
  - Internally calls the existing YAML flow-mapping parse, falls back to the
    legacy comma splitter on malformed YAML, and collects the same
    `tracing::warn!` messages.
  - Reuse the existing `matches_horizontal_rule_pattern` predicate (already
    pub(crate)).

- [ ] Refactor `RuleProcessor::process_paragraph_buffer` to call
  `parse_hr_attribute_block()` instead of its private `parse_attributes`.
  - Delete the private `parse_attributes`, `parse_attributes_legacy`,
    `attrs_from_mapping` methods from `RuleProcessor` (move logic into the
    shared helper).

- [ ] Refactor `try_parse_hr_attrs` to delegate to `parse_hr_attribute_block()`.
  - The free function becomes a thin wrapper: match predicate → shared parser →
    return `Option<HorizontalRuleAttrs>`.

- [ ] Refactor `scan_inline_hr_warnings` to use `parse_hr_attribute_block()`
  instead of spinning up a full `RuleProcessor` pipeline for warning collection
  (if the shared helper already captures warning data, the preflight scan can
  call it directly on candidate paragraphs).

### Validation

- [ ] Run `cargo test -p darkmatter --lib -- rule_processor` — all existing
  `rule_processor.rs` unit tests pass unchanged (no test edits).
- [ ] Run `cargo test -p darkmatter --test horizontal_rule_integration` — green.
- [ ] Run `cargo test -p darkmatter --test horizontal_rule_snapshots` — no
  snapshot diffs.
- [ ] `cargo test -p darkmatter --test render_tree_parity` — HR parity tests
  green.

---

## Phase 2 — Create Block-Extension Processor Module

*Introduce the new offset-aware processor in `render_tree/block_extension.rs`.*

### Tasks

- [ ] Add `pub(crate) mod block_extension;` to
  `darkmatter/lib/src/markdown/render_tree/mod.rs`.

- [ ] Define `BlockExtensionEvent<'a>` in the new module:
  ```rust
  pub(crate) enum BlockExtensionEvent<'a> {
      Standard(Event<'a>, Range<usize>),
      HorizontalRule { attrs: HorizontalRuleAttrs, body_range: Range<usize> },
  }
  ```
  Add module-level docs stating only HR attributes are implemented today.

- [ ] Implement `BlockExtensionProcessor<I>` as a small state-machine iterator
  adapter over `I: Iterator<Item = (Event<'a>, Range<usize>)>`:
  - States: `Idle`, `BufferingParagraph { buffer, paragraph_start_range }`.
  - On `Start(Paragraph)` → enter buffering.
  - Buffer all events until matching `End(Paragraph)`.
  - On paragraph close: check simple-paragraph policy (exactly one `Text`
    child, no other inline events). If eligible, run
    `matches_horizontal_rule_pattern` then `parse_hr_attribute_block`.
  - Matched: emit `BlockExtensionEvent::HorizontalRule { attrs, body_range }`
    where `body_range` is the buffered text event's range (not the paragraph
    end range).
  - Unmatched: flush buffered `Standard(..)` events.
  - `Event::Rule` and all non-paragraph events pass through as `Standard(..)`.

- [ ] Add focused unit tests in `block_extension.rs` (or a companion test
  module):
  - Matched: `--- { kind: waves }` → `HorizontalRule` event.
  - Matched with multiple attrs: `*** { kind: dots, weight: thick, color: blue }`.
  - Unmatched: bare `---` stays as `Standard(Rule, ..)`.
  - Unmatched: regular paragraph text passes through.
  - Unmatched: paragraph with bold/italic/code is not rewritten.
  - Unmatched: `-- { kind: waves }` (insufficient markers).
  - Unmatched: mixed markers (`-** { }`).
  - Fenced code block containing `--- { kind: waves }` stays as code block.
  - Blockquote-wrapped HR attribute is matched.
  - List-item-wrapped HR attribute is NOT matched.
  - Malformed attribute block falls back gracefully.
  - Provenance: body_range points at text-event bytes, not paragraph end.

### Validation

- [ ] `cargo test -p darkmatter --lib -- block_extension` — all new tests green.
- [ ] No changes to existing tests yet — legacy path still active.

---

## Phase 3 — Wire Block-Extension Into Fold & Retire Old Path

*Replace the inline-span HR path with the block-extension processor in
`fold_markdown_spanned_with_frontmatter` and remove `SpannedRuleProcessor`.*

### Tasks

- [ ] Extract HR lowering helper in `fold.rs`:
  - Move the `InlineEvent::HorizontalRule` match-arm body (lines ~443-480) into
    a free function like `lower_hr_attrs_to_node(attrs, body_range) ->
    RenderNode`.
  - This helper builds `RenderNode::thematic_break()`, sets
    `Provenance::Generated` with `body_range`, and attaches
    `darkmatter.hr.*` hints.

- [ ] Add a `BlockExtensionEvent` match arm to the fold dispatch in
  `fold_markdown_spanned_with_frontmatter`:
  - `BlockExtensionEvent::Standard(event, range)` → existing `feed_event` /
    mark/dim dispatch.
  - `BlockExtensionEvent::HorizontalRule { attrs, body_range }` → call
    `lower_hr_attrs_to_node(attrs, body_range)`.

- [ ] Rewire the pipeline in `fold_markdown_spanned_with_frontmatter`:
  ```
  Parser::new_ext(..).into_offset_iter()
      → BlockExtensionProcessor::new(..)
      → SpanningAdapter + SpannedInlineStyleProcessor  (mark/dim only)
      → fold
  ```
  Remove `SpannedRuleProcessor` from the chain.

- [ ] Remove `SpannedRuleProcessor` from `span.rs`:
  - Delete the struct, its `Iterator` impl, and the private `parse_hr_paragraph`
    helper.
  - Keep `SpannedInlineEvent`, `SpanningAdapter`, and
    `SpannedInlineStyleProcessor` (they remain for mark/dim until the sibling
    spec replaces them).

- [ ] Remove the `InlineEvent::HorizontalRule` match arm from the fold dispatch
  (now dead code since no path produces it anymore).

- [ ] Update `render_tree/mod.rs` visibility if needed — the block-extension
  module is `pub(crate)`; no public API surface changes.

### Validation

- [ ] `cargo test -p darkmatter --lib -- fold` — span-aware fold HR tests pass.
- [ ] `cargo test -p darkmatter --test horizontal_rule_integration` — all green.
- [ ] `cargo test -p darkmatter --test horizontal_rule_snapshots` — no diffs.
- [ ] `cargo test -p darkmatter --test render_tree_parity` — HR parity tests
  green (byte-identical output, same source spans).
- [ ] `cargo test -p darkmatter` — full crate green.
- [ ] Run `just lint` and `just test` from the darkmatter area — no regressions.

---

## Phase 4 — Final Validation & Cleanup

*Confirm behavioral parity, clean up dead code, and verify the hand-off
surface for the sibling inline-span spec.*

### Tasks

- [ ] Audit `span.rs` — confirm it no longer contains any HR-related logic.
  Only `SpannedInlineEvent`, `SpanningAdapter`, and
  `SpannedInlineStyleProcessor` should remain.

- [ ] Audit `block/rule_processor.rs` — confirm `RuleProcessor` still works
  for the legacy event-stream path (it must remain until the legacy renderers
  are retired). The shared `parse_hr_attribute_block` helper is the single
  source of truth for parsing.

- [ ] Add a regression test in `block_extension` tests for the fenced-code
  defense (code block containing `--- { kind: waves }` is not rewritten).

- [ ] Run the full test suite one final time:
  ```
  cargo nextest run -p darkmatter
  ```

- [ ] Verify `cargo doc -p darkmatter --no-deps` builds without warnings in
  the new module.

- [ ] Confirm the sibling inline-span spec's prerequisite is met:
  `SpannedInlineStyleProcessor` no longer depends on any HR path, and
  `SpannedRuleProcessor` is deleted from `span.rs`.

### Validation

- [ ] All darkmatter tests green under `nextest`.
- [ ] No new compiler warnings in `block_extension.rs` or `fold.rs`.
- [ ] Snapshot tests unchanged (`insta review` confirms no diffs).
- [ ] Ready hand-off: the inline-span spec can now safely replace
  `SpannedInlineStyleProcessor` without losing HR-attribute support.

---

## Dependency & Parallelism Notes

- **Phase 1 and Phase 2 are independent** and can be developed in parallel on
  separate branches. Phase 1 touches `block/rule_processor.rs`; Phase 2 adds a
  new file `render_tree/block_extension.rs`. They merge without conflict.
- **Phase 3 depends on both Phase 1 and Phase 2** — it wires the shared parser
  into the new processor and integrates both into the fold.
- **Phase 4 depends on Phase 3** — final validation only after the new path is
  active.

```
Phase 1 ──┐
           ├──► Phase 3 ──► Phase 4
Phase 2 ──┘
```

## Risk Notes

- The simple-paragraph policy must be preserved exactly. The new state machine
  must not accept paragraphs with inline code, emphasis, links, or any nested
  formatting — the existing `paragraph_is_simple` flag logic in
  `SpannedRuleProcessor` is the reference.
- `parse_hr_attribute_block` must preserve the `kind` over `style` precedence
  and the legacy comma-splitter fallback. Test coverage already exists for
  these edge cases — no new tests needed for parsing, just re-use.
- `try_parse_hr_attrs` remains the public API for callers outside the
  render-tree path. It must continue to work after the internal refactor.
