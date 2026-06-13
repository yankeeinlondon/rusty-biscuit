---
agent: open_code/kimi-for-coding/k2p7
phases: 5
created: 2026-06-13
start_phase: 1
yolo: true
---

# Disclosure Blocks — Implementation Plan

Execution plan for the `::disclosure / ::details / ::end-disclosure` render-time
block extension. Derived from
[`spec.md`](./spec.md).

## Phase 1 — Render-Time Recognition

Implement render-time recognition of the disclosure triple in the block-extension
processor, add typed errors, and ensure compose invariance.

- [ ] Add `BlockExtensionEvent::Disclosure { summary_events, body_events, range }`
  variant to `lib/src/markdown/render_tree/block_extension.rs`.
- [ ] Extend `BlockExtensionProcessor` to detect the `::disclosure` opener,
  buffer events until `::details` and `::end-disclosure`, split the buffer into
  summary and body event sub-ranges, and emit a synthetic disclosure event over
  the original byte range.
- [ ] Enforce keyword boundary rules (`::disclosure`, `::details`,
  `::end-disclosure` must be followed by ASCII whitespace or EOL) so near-miss
  prose remains literal.
- [ ] Preserve fence-state immunity: directive lines inside fenced code blocks
  are never interpreted by the existing processor.
- [ ] Add `MarkdownError::MalformedDisclosure { reason, range }` variant and map
  page rendering failures through `PageRenderError::Render`.
- [ ] Reject malformed disclosures with a fatal error: missing `::details`,
  missing `::end-disclosure`, `::details` without a matching closer, empty
  summary region, hard line break in summary, or any block-level element in the
  summary region.
- [ ] Add unit tests for:
  - valid disclosure parsing,
  - empty summary rejection,
  - hard line break in summary rejection,
  - block element in summary rejection,
  - missing closer(s) rejection,
  - near-miss keywords treated as literal text,
  - directives inside fenced code blocks ignored.

### Validation Checkpoint 1

- `cargo test -p darkmatter` passes for block-extension processor tests.
- Compose invariance test verifies a disclosure document is byte-identical in
  the disclosure region after `darkmatter compose`.

## Phase 2 — Transclusion Unification

Unify the existing `::file` / `::code` `disclosure="..."` transclusion option
with the new render-time DSL.

- [ ] Update transclusion parser/type wiring
  (`lib/src/markdown/compose/transclusion/types.rs::BlockOptions::disclosure`) to
  retain the summary string without invoking HTML wrapping.
- [ ] Modify `::file` and `::code` transclusion emission so a configured
  `disclosure` option wraps transcluded content in
  `::disclosure {summary}\n…\n::details\n…\n::end-disclosure` instead of inline
  HTML.
- [ ] Normalize `disclosure=true` (empty string summary) to the default summary
  `"Details"`.
- [ ] Remove the compose-time `wrap_disclosure` helper and its call sites from
  `lib/src/markdown/compose/transclusion/wrappers.rs`.
- [ ] Delete or rewrite the `wrap_disclosure` unit test to assert DSL wrapping
  behavior instead of HTML wrapping.
- [ ] Add integration tests proving `::file ... disclosure="…"` and
  `::code ... disclosure="…"` compose to the DSL block, not `<details>`.

### Validation Checkpoint 2

- Transclusion tests confirm no `<details>` HTML is emitted at compose time.
- `disclosure=true` produces a `::disclosure` block whose summary is `"Details"`.

## Phase 3 — Render-Tree Node and Target Folds

Land the `NodeKind::Disclosure` render-tree node and lower it across all
supported targets.

- [ ] Add `NodeKind::Disclosure { summary, children, layout, style }` to the
  `renderable` node model.
- [ ] Update `render_tree::fold_markdown_to_document` to lower the synthetic
  disclosure event into `NodeKind::Disclosure`.
- [ ] Implement Terminal fold: render summary normally, render body as a block
  quote whose text is dim and italic.
- [ ] Implement Markdown fold: emit the DSL
  `::disclosure / ::details / ::end-disclosure` verbatim.
- [ ] Implement MarkdownPlus fold: render summary and body to Markdown, then
  wrap with `<details><summary>…</summary>…</details>`.
- [ ] Implement Browser fold: render summary and body to HTML, then wrap with
  native `<details>`/`<summary>` elements; no JavaScript.
- [ ] Implement JSON export: represent disclosure content without loss,
  preferably using native `NodeKind::Disclosure` if the renderable-IR JSON
  migration is active; otherwise preserve the current JSON contract with an
  explicit disclosure representation.
- [ ] Add fallback behavior: any target that does not recognize the node renders
  summary followed by body so content is never dropped.
- [ ] Add tests for each target (Terminal, Markdown, MarkdownPlus, Browser,
  JSON), including nested disclosures.

### Validation Checkpoint 3

- Target-specific snapshot or assertion tests pass for all five outputs.
- Nested disclosures render recursively on every target.

## Phase 4 — Style Frontmatter and CLI

Add the `style.disclosure` bucket, wire `OutputFormat::MarkdownPlus`, and expose
`--output markdown-plus`.

- [ ] Add optional `disclosure` bucket to `StyleFrontmatter`
  (`lib/src/style/schema/mod.rs`) using the `CommonStyle` shape
  (`width`, `max-width`, `alignment`, `color`, `bg-color`).
- [ ] Support kebab-case keys as canonical and snake_case keys as deprecated
  aliases (`max_width`, `bg_color`).
- [ ] Add `PageComponent::Disclosure` and include it in `PageComponent::ALL`.
- [ ] Implement `apply_disclosure_style` that lowers the bucket into
  `ComponentPolicy` via the existing `apply_component_bucket` helper.
- [ ] Enforce mutual exclusivity of `width` and `max-width` with the same
  conflict behavior as other component buckets.
- [ ] Reject CSS lengths that affect terminal layout with existing
  `style-apply` errors.
- [ ] Add `OutputFormat::MarkdownPlus` to the CLI `OutputFormat` enum in
  `darkmatter/cli/src/args.rs`.
- [ ] Wire `--output markdown-plus` to route through the MarkdownPlus fold.
- [ ] Add `browser` as an alias for `html` if not already present.
- [ ] Implement instance-level `param=value` style parsing on `::disclosure`,
  limited to the disclosure style key set.
- [ ] Implement style precedence: instance-level params > `style.disclosure`
  frontmatter > existing all-components CLI broadcast (if any) > built-in
  default.
- [ ] Update descriptor, walker, strict-style, and schema coverage tests so
  `style.disclosure.*` is neither unknown nor silently inactive.
- [ ] Add `--strict-style` tests rejecting unknown and deprecated keys inside
  `style.disclosure`.

### Validation Checkpoint 4

- `cargo test -p darkmatter` and `cargo test -p darkmatter-cli` pass.
- `md render README.md --output markdown-plus` emits well-formed inline HTML.
- `md render README.md --output auto` on a TTY uses the Terminal fold.
- `--strict-style` rejects `style.disclosure.unknown_key`.

## Phase 5 — Documentation and Final Verification

Promote the disclosure feature from planned to documented and run final
verification.

- [ ] Update `darkmatter/docs/rendering/disclosure.md` to describe shipped
  behavior, including syntax, targets, terminal presentation, MarkdownPlus
  output, and style frontmatter.
- [ ] Cross-link the disclosure doc from the rendering-pipeline and style docs.
- [ ] Update the `::file` / `::code` transclusion docs to note that
  `disclosure="…"` now emits the render-time DSL.
- [ ] Run the full darkmatter test suite: `just test` or `cargo test -p
  darkmatter -p darkmatter-cli`.
- [ ] Run `just lint` / `cargo clippy -p darkmatter -p darkmatter-cli` and fix
  warnings.
- [ ] Run `just build` or `cargo build -p darkmatter -p darkmatter-cli` to
  confirm a clean workspace build.

### Validation Checkpoint 5

- All tests pass.
- No clippy warnings in touched crates.
- Documentation reflects shipped behavior.
