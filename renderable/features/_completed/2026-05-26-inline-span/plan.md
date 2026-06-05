---
phases: 6
created: 2026-06-01
start_phase: 1
packages:
  - renderable
  - darkmatter
  - biscuit-terminal
depends_on:
  - ../2026-05-26-block-extension/plan.md
source_files_during_phase_1:
  - darkmatter/lib/tests/inline_envelope_prototype.rs
docs_updated_during_phase_1:
  - renderable/features/2026-05-26-inline-span/spec.md
docs_created_during_phase_1:
  - renderable/features/2026-05-26-inline-span/phase-1-prototype-notes.md
skills_files_updated_during_phase_1: []
packages_during_phase_1:
  - darkmatter
source_files_during_phase_2:
  - renderable/src/tree/node.rs
  - renderable/src/tree/validate.rs
  - renderable/src/tree/render/markdown.rs
  - renderable/src/tree/render/browser.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/renderable/tree.md
packages_during_phase_2:
  - renderable
  - biscuit-terminal
source_files_during_phase_3:
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/renderable/tree.md
packages_during_phase_3:
  - renderable
  - biscuit-terminal
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/render_tree/inline_extension.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - darkmatter
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/render_tree/fold.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/tests/render_tree_parity.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/render_tree/span.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/render_tree/inline_extension.rs
  - darkmatter/lib/src/markdown/render_tree/block_extension.rs
  - darkmatter/lib/src/markdown/render_tree/fold.rs
  - darkmatter/lib/benches/migration_parity.rs
docs_updated_during_phase_6:
  - renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - darkmatter
---

# Execution Plan: Inline Span Extensions

Spec: `spec.md` (this directory)

## Overview

Replace Darkmatter's span-aware inline transport with a source rewrite into
GFM strikethrough envelopes, then lower those envelopes through the plain
pulldown-cmark fold into `NodeKind::Extended` nodes.

**Success criteria:** `SpannedInlineStyleProcessor` and its support transport
are deleted after the HR block-extension prerequisite lands, mark and dim
continue to roundtrip across Markdown, Browser, and Terminal targets, Browser
HTML recovers semantic `<mark>` output, source provenance remains available for
diagnostics, and `migration_parity` records the new performance numbers.

---

## Phase 1 — Prototype Envelope Decisions

*Lock the implementation-level decisions before changing public IR or renderer
behavior.*

### Tasks

- [ ] Create a focused prototype test or throwaway internal test that feeds
  `~~<<mark>>\u{FDD0}text<<mark>>\u{FDD0}~~` through pulldown-cmark with
  `ENABLE_STRIKETHROUGH`.

- [ ] Verify pulldown-cmark emits a `Tag::Strikethrough` container and preserves
  both `<<mark>>` markers plus U+FDD0 as literal text children.

- [ ] Decide and document the multi-field separator for future tooltip payloads.
  Use U+FDD1 unless the prototype exposes a concrete reason to choose a visible
  separator.

- [ ] Decide and document the provenance translation-table shape for the
  rewriter. Prefer `Vec<(usize, usize)>` sorted by rewritten offset unless the
  prototype shows a simpler fit with the fold code.

- [ ] Confirm escape semantics for `\==`, `\⌄`, `\:`, and `\{{` match the
  current span-aware processor where behavior already exists.

- [ ] Add module-level notes for the future rewriter stating that all inline
  token matching must happen in one shared scan, not one pass per token.

### Validation

- [ ] Prototype test proves the `<<NAME>>\u{FDD0}` marker is not parsed as HTML
  or an autolink.

- [ ] Prototype notes identify the exact marker sentinel, field separator, and
  provenance table shape the implementation will use.

- [ ] No production behavior changes have landed yet.

---

## Phase 2 — Add Extended IR Scaffolding

*Introduce the target-agnostic tree shape before any fold emits it.*

### Tasks

- [ ] Add `NodeKind::Extended { token: Cow<'static, str>, children:
  Vec<RenderNode>, payload: Option<String> }` to `renderable::tree`.

- [ ] Update any `NodeKind` pattern matches, validators, walkers, debug output,
  serializers, or helper constructors so `Extended` is handled deliberately.

- [ ] Add a small constructor or helper for built-in extension nodes if it
  matches existing render-tree style; skip the helper if the enum literal is
  clearer at call sites.

- [ ] Add default fallback handling in Browser, Markdown, and Terminal tree
  renderers for unknown `Extended` tokens.

- [ ] Add focused renderable tests that manually construct an unknown
  `Extended` node and verify fallback output is stable in all three renderers.

### Validation

- [ ] `cargo test -p renderable --lib` passes.

- [ ] `cargo test -p renderable --test '*'` passes if the package has
  integration tests.

- [ ] `cargo test -p biscuit-terminal --lib -- render_tree` passes if terminal
  render-tree tests exist.

### Parallelizable Work

- [ ] Browser, Markdown, and Terminal fallback arms can be implemented in
  parallel after the enum compiles.

---

## Phase 3 — Implement Built-In Token Lowering

*Teach renderers what `mark` and `dim` mean before the Darkmatter fold starts
producing them.*

### Tasks

- [ ] Implement Browser lowering for `Extended { token: "mark", .. }` as
  semantic `<mark>children</mark>`.

- [ ] Implement Browser lowering for `Extended { token: "dim", .. }` as a span
  with the existing dim visual policy.

- [ ] Implement Markdown lowering for `mark` as `==children==` and `dim` as
  `⌄children⌄`.

- [ ] Implement Terminal lowering for `mark` using reverse video and `dim`
  using the existing Prose/terminal dim style path.

- [ ] Add renderer tests that construct nested `mark` and `dim` `Extended`
  nodes directly and assert target output, including Browser `<mark>` fidelity.

- [ ] Add unknown-token tests that prove fallback behavior still works after
  built-in token dispatch is added.

### Validation

- [ ] `cargo test -p renderable --lib -- extended` or the closest focused
  render-tree test filter passes.

- [ ] `cargo test -p biscuit-terminal --lib -- extended` or the closest focused
  terminal renderer filter passes.

- [ ] Browser tests assert `<mark>` is emitted, not `<span class="mark">`.

### Parallelizable Work

- [ ] Browser, Markdown, and Terminal built-in lowering can be developed in
  parallel once Phase 2 is complete.

---

## Phase 4 — Build Source Rewriter And Token Registry

*Add the Darkmatter source-layer rewrite without wiring it into the public
span-aware fold yet.*

### Tasks

- [ ] Create the Darkmatter inline rewrite module, for example
  `darkmatter/lib/src/markdown/render_tree/inline_extension.rs` or
  `darkmatter/lib/src/markdown/rewrite.rs`, following the module placement that
  best matches nearby render-tree code.

- [ ] Define the central inline token registry shape with token name, source
  pattern, fold handler metadata, and roundtrip metadata as needed by the fold
  and Markdown renderer.

- [ ] Implement a one-pass scanner over the source text that recognizes built-in
  paired patterns `==...==` and `⌄...⌄`.

- [ ] Emit canonical envelopes in the rewritten source:
  `~~<<TOKEN>>\u{FDD0}payload<<TOKEN>>\u{FDD0}~~`.

- [ ] Preserve user escapes so escaped delimiters stay literal and do not
  rewrite.

- [ ] Preserve unmatched delimiter behavior so unclosed or orphan delimiters
  remain literal text.

- [ ] Produce the chosen provenance translation table from rewritten byte
  offsets back to original byte offsets.

- [ ] Add unit tests for mark, dim, nested Markdown payloads, escaped
  delimiters, unclosed delimiters, adjacent spans, UTF-8 payloads, and
  no-extension documents.

### Validation

- [ ] Rewriter tests prove no-extension input returns either a borrowed
  unchanged source or an observable no-rewrite result, depending on the chosen
  API.

- [ ] Rewriter tests prove all emitted envelopes include both opener and closer
  token markers with U+FDD0.

- [ ] Rewriter tests prove translated offsets point back to the original source
  byte positions for rewritten spans.

### Parallelizable Work

- [ ] Registry type design, scanner implementation, and provenance translation
  tests can be split between implementers after Phase 1 decisions are locked.

---

## Phase 5 — Add Fold-Side Dispatcher And Transitional Wiring

*Consume the rewritten strikethrough envelopes in the plain fold and preserve the
existing public entry point signature.*

### Tasks

- [ ] Add fold logic for `Tag::Strikethrough` that peeks at the first child text
  for `<<NAME>>\u{FDD0}`.

- [ ] Preserve standard GFM strikethrough behavior by emitting
  `NodeKind::Delete` when no extension marker is present.

- [ ] For registered `mark` and `dim` tokens, strip opener and closer markers,
  parse the payload as inline Markdown, and emit `NodeKind::Extended` with
  nested children and `payload: None`.

- [ ] For unknown token markers, emit a diagnostic and fall back to standard
  `NodeKind::Delete`.

- [ ] Apply the provenance translation table when constructing source spans for
  rewritten inline content and diagnostics.

- [ ] Wire `fold_markdown_spanned_with_frontmatter` internally to run the
  block-extension processor, then the inline source rewriter, then the plain
  fold dispatcher, while keeping its public signature unchanged.

- [ ] Add a temporary feature flag or internal switch only if needed to diff the
  old and new paths during implementation; remove it before Phase 6 completes.

- [ ] Add fold tests for mark, dim, mixed mark/dim nesting, ordinary
  strikethrough, escaped delimiters, malformed envelopes, unknown tokens, and
  source-span diagnostics.

### Validation

- [ ] `cargo test -p darkmatter --lib -- render_tree` passes.

- [ ] `cargo test -p darkmatter --test render_tree_parity` passes with expected
  diff limited to Browser `<mark>` fidelity updates.

- [ ] `cargo test -p darkmatter --test render_tree_roundtrip` passes after
  Markdown roundtrip expectations are updated for `Extended` nodes.

- [ ] `cargo test -p darkmatter --test horizontal_rule_integration` still
  passes, proving the block-extension prerequisite remains intact.

### Dependency Checkpoint

- [ ] Confirm `../2026-05-26-block-extension/plan.md` has completed through its
  active-path removal of `SpannedRuleProcessor` before deleting any remaining
  span-aware inline transport.

---

## Phase 6 — Big-Bang Cutover, Cleanup, And Measurement

*Remove the old span-aware inline processor and record final behavior and
performance.*

### Tasks

- [ ] Delete `SpannedInlineStyleProcessor`, `SpannedInlineEvent`,
  `SpanningAdapter`, and any support types from
  `darkmatter/lib/src/markdown/render_tree/span.rs` once no active HR or inline
  path depends on them.

- [ ] Remove `span.rs` module exports if the file becomes empty; otherwise audit
  the remaining contents and rename the module if it no longer owns inline
  spans.

- [ ] Remove any temporary feature flag or internal switch used to compare old
  and new inline paths.

- [ ] Update Browser parity fixtures for the deliberate `<mark>` recovery and
  document that this is an accepted fidelity improvement, not an accidental
  regression.

- [ ] Update relevant README or render-tree docs if public behavior or extension
  architecture is described there.

- [ ] Update `.claude/skills/renderable` only if the extension architecture or
  workflow guidance changes in a way future agents need to know.

- [ ] Run the full Darkmatter, Renderable, and Biscuit Terminal test gates.

- [ ] Run `cargo bench -p darkmatter --bench migration_parity` and record the
  new `mark_dim_hr` numbers in the appropriate baseline document.

- [ ] Inspect `git diff` for stale comments around inline spans, HR processing,
  and `<mark>` rendering; update or delete any comments that describe the old
  processor.

### Validation

- [ ] `cargo test -p renderable` passes.

- [ ] `cargo test -p darkmatter` passes.

- [ ] `cargo test -p biscuit-terminal` passes for affected terminal rendering
  tests.

- [ ] `cargo bench -p darkmatter --bench migration_parity` completes and the
  recorded numbers include the new inline-span path.

- [ ] `rg "SpannedInlineStyleProcessor|SpannedInlineEvent|SpanningAdapter"
  darkmatter/lib/src/markdown/render_tree` returns no active production uses.

- [ ] Final parity review confirms the only intended output change is Browser
  `<mark>` recovery.

---

## Dependency & Parallelism Notes

- **Phase 1 blocks all production code.** It locks the envelope, separator,
  provenance, and escape decisions.

- **Phase 2 and the block-extension plan can proceed in parallel.** Phase 2 is
  in `renderable`; the block-extension prerequisite is in `darkmatter`.

- **Phase 3 depends on Phase 2** but can be split by target renderer.

- **Phase 4 depends on Phase 1** and can proceed while Phase 3 renderer
  lowering is underway.

- **Phase 5 depends on Phases 3 and 4** because the fold will emit nodes that
  renderers must already understand.

- **Phase 6 depends on Phase 5 and the sibling block-extension plan.** Do not
  delete the span-aware transport until HR attributes are already off that path.
