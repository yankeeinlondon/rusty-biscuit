---
agent: open_code/kimi-for-coding/k2p7
phases: 5
created: 2026-06-13
start_phase: 1
yolo: true
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/transclusion/parser.rs
- darkmatter/lib/src/markdown/compose/transclusion/wrappers.rs
- darkmatter/lib/src/markdown/compose/transclusion/mod.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/transform/mod.rs
- darkmatter/lib/tests/disclosure_transclusion_integration.rs
source_files_during_phase_3:
- renderable/src/tree/node.rs
- renderable/src/tree/validate.rs
- renderable/src/tree/render/markdown.rs
- renderable/src/tree/render/browser.rs
- biscuit-terminal/lib/src/render_tree/render.rs
- darkmatter/lib/src/markdown/render_tree/fold.rs
- darkmatter/lib/src/markdown/render_tree/block_extension.rs
- darkmatter/lib/src/markdown/mod.rs
- darkmatter/cli/src/output.rs
- darkmatter/lib/tests/disclosure_render_targets.rs
docs_updated_during_phase_3:
- darkmatter/features/2026-06-12-disclosure/plan.md
source_files_during_phase_4:
- darkmatter/lib/src/style/schema/mod.rs
- darkmatter/lib/src/style/schema/components.rs
- darkmatter/lib/src/layout/types.rs
- darkmatter/lib/src/style/apply.rs
- darkmatter/lib/src/style/descriptor.rs
- darkmatter/lib/src/style/parse.rs
- darkmatter/lib/src/style/coverage_tests.rs
- darkmatter/cli/src/output.rs
- darkmatter/cli/src/args.rs
- darkmatter/cli/src/commands.rs
- darkmatter/cli/tests/cli.rs
- darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
- darkmatter/lib/src/markdown/render_tree/block_extension.rs
- darkmatter/lib/src/markdown/render_tree/fold.rs
- darkmatter/lib/src/markdown/render_tree/build_context.rs
- darkmatter/lib/src/markdown/render_tree/entrypoints.rs
- darkmatter/lib/src/layout/page.rs
- renderable/src/tree/attrs.rs
- renderable/src/tree/node.rs
- renderable/src/tree/mod.rs
- renderable/src/tree/render/browser.rs
- renderable/src/tree/render/markdown.rs
- biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_4:
- darkmatter/features/2026-06-12-disclosure/plan.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
- darkmatter/docs/rendering/disclosure.md
- darkmatter/docs/darkmatter-rendering-pipeline.md
- darkmatter/docs/rendering/style.md
- darkmatter/docs/transclusion/block-transclusion.md
- darkmatter/docs/transclusion/code-transclusion.md
- darkmatter/features/2026-06-12-disclosure/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/SKILL.md
source_code:
- darkmatter/lib/src/markdown/compose/transclusion/parser.rs
- darkmatter/lib/src/markdown/compose/transclusion/wrappers.rs
- darkmatter/lib/src/markdown/compose/transclusion/mod.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/transform/mod.rs
- darkmatter/lib/tests/disclosure_transclusion_integration.rs
- renderable/src/tree/node.rs
- renderable/src/tree/validate.rs
- renderable/src/tree/render/markdown.rs
- renderable/src/tree/render/browser.rs
- biscuit-terminal/lib/src/render_tree/render.rs
- darkmatter/lib/src/markdown/render_tree/fold.rs
- darkmatter/lib/src/markdown/render_tree/block_extension.rs
- darkmatter/lib/src/markdown/mod.rs
- darkmatter/cli/src/output.rs
- darkmatter/lib/tests/disclosure_render_targets.rs
- darkmatter/lib/src/style/schema/mod.rs
- darkmatter/lib/src/style/schema/components.rs
- darkmatter/lib/src/layout/types.rs
- darkmatter/lib/src/style/apply.rs
- darkmatter/lib/src/style/descriptor.rs
- darkmatter/lib/src/style/parse.rs
- darkmatter/lib/src/style/coverage_tests.rs
- darkmatter/cli/src/args.rs
- darkmatter/cli/src/commands.rs
- darkmatter/cli/tests/cli.rs
- darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
- darkmatter/lib/src/markdown/render_tree/build_context.rs
- darkmatter/lib/src/markdown/render_tree/entrypoints.rs
- darkmatter/lib/src/layout/page.rs
- renderable/src/tree/attrs.rs
- renderable/src/tree/mod.rs
documentation:
- darkmatter/docs/rendering/disclosure.md
- darkmatter/docs/darkmatter-rendering-pipeline.md
- darkmatter/docs/rendering/style.md
- darkmatter/docs/transclusion/block-transclusion.md
- darkmatter/docs/transclusion/code-transclusion.md
- darkmatter/features/2026-06-12-disclosure/plan.md
packages:
- darkmatter
hash: f0534c6a72f84522-8110039da9ecb1d6
last_updated: 2026-06-13
---

# Disclosure Blocks — Implementation Plan

Execution plan for the `::disclosure / ::details / ::end-disclosure` render-time
block extension. Derived from
[`spec.md`](./spec.md).

## Phase 1 — Render-Time Recognition

Implement render-time recognition of the disclosure triple in the block-extension
processor, add typed errors, and ensure compose invariance.

- [x] Add `BlockExtensionEvent::Disclosure { summary_events, body_events, range }`
  variant to `lib/src/markdown/render_tree/block_extension.rs`.
- [x] Extend `BlockExtensionProcessor` to detect the `::disclosure` opener,
  buffer events until `::details` and `::end-disclosure`, split the buffer into
  summary and body event sub-ranges, and emit a synthetic disclosure event over
  the original byte range.
- [x] Enforce keyword boundary rules (`::disclosure`, `::details`,
  `::end-disclosure` must be followed by ASCII whitespace or EOL) so near-miss
  prose remains literal.
- [x] Preserve fence-state immunity: directive lines inside fenced code blocks
  are never interpreted by the existing processor.
- [x] Add `MarkdownError::MalformedDisclosure { reason, range }` variant and map
  page rendering failures through `PageRenderError::Render`.
- [x] Reject malformed disclosures with a fatal error: missing `::details`,
  missing `::end-disclosure`, `::details` without a matching closer, empty
  summary region, hard line break in summary, or any block-level element in the
  summary region.
- [x] Add unit tests for:
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

- [x] Update transclusion parser/type wiring
  (`lib/src/markdown/compose/transclusion/types.rs::BlockOptions::disclosure`) to
  retain the summary string without invoking HTML wrapping.
- [x] Modify `::file` and `::code` transclusion emission so a configured
  `disclosure` option wraps transcluded content in
  `::disclosure {summary}\n…\n::details\n…\n::end-disclosure` instead of inline
  HTML.
- [x] Normalize `disclosure=true` (empty string summary) to the default summary
  `"Details"`.
- [x] Remove the compose-time `wrap_disclosure` helper and its call sites from
  `lib/src/markdown/compose/transclusion/wrappers.rs`.
- [x] Delete or rewrite the `wrap_disclosure` unit test to assert DSL wrapping
  behavior instead of HTML wrapping.
- [x] Add integration tests proving `::file ... disclosure="…"` and
  `::code ... disclosure="…"` compose to the DSL block, not `<details>`.

### Validation Checkpoint 2

- Transclusion tests confirm no `<details>` HTML is emitted at compose time.
- `disclosure=true` produces a `::disclosure` block whose summary is `"Details"`.

## Phase 3 — Render-Tree Node and Target Folds

Land the `NodeKind::Disclosure` render-tree node and lower it across all
supported targets.

- [x] Add `NodeKind::Disclosure { summary, children, layout, style }` to the
  `renderable` node model.
- [x] Update `render_tree::fold_markdown_to_document` to lower the synthetic
  disclosure event into `NodeKind::Disclosure`.
- [x] Implement Terminal fold: render summary normally, render body as a block
  quote whose text is dim and italic.
- [x] Implement Markdown fold: emit the DSL
  `::disclosure / ::details / ::end-disclosure` verbatim.
- [x] Implement MarkdownPlus fold: render summary and body to Markdown, then
  wrap with `<details><summary>…</summary>…</details>`.
- [x] Implement Browser fold: render summary and body to HTML, then wrap with
  native `<details>`/`<summary>` elements; no JavaScript.
- [x] Implement JSON export: represent disclosure content without loss,
  preferably using native `NodeKind::Disclosure` if the renderable-IR JSON
  migration is active; otherwise preserve the current JSON contract with an
  explicit disclosure representation.
- [x] Add fallback behavior: any target that does not recognize the node renders
  summary followed by body so content is never dropped.
- [x] Add tests for each target (Terminal, Markdown, MarkdownPlus, Browser,
  JSON), including nested disclosures.

### Validation Checkpoint 3

- [x] Target-specific assertion tests pass for all five outputs.
- [x] Nested disclosures render recursively on every target.

## Phase 4 — Style Frontmatter and CLI

Add the `style.disclosure` bucket, wire `OutputFormat::MarkdownPlus`, and expose
`--output markdown-plus`.

- [x] Add optional `disclosure` bucket to `StyleFrontmatter`
  (`lib/src/style/schema/mod.rs`) using the `CommonStyle` shape
  (`width`, `max-width`, `alignment`, `color`, `bg-color`).
- [x] Support kebab-case keys as canonical and snake_case keys as deprecated
  aliases (`max_width`, `bg_color`).
- [x] Add `PageComponent::Disclosure` and include it in `PageComponent::ALL`.
- [x] Implement `apply_disclosure_style` that lowers the bucket into
  `ComponentPolicy` via the existing `apply_component_bucket` helper.
- [x] Enforce mutual exclusivity of `width` and `max-width` with the same
  conflict behavior as other component buckets.
- [x] Reject CSS lengths that affect terminal layout with existing
  `style-apply` errors.
- [x] Add `OutputFormat::MarkdownPlus` to the CLI `OutputFormat` enum in
  `darkmatter/cli/src/args.rs`.
- [x] Wire `--output markdown-plus` to route through the MarkdownPlus fold.
- [x] Add `browser` as an alias for `html` if not already present.
- [x] Implement instance-level `param=value` style parsing on `::disclosure`,
  limited to the disclosure style key set.
- [x] Implement style precedence: instance-level params > `style.disclosure`
  frontmatter > existing all-components CLI broadcast (if any) > built-in
  default.
- [x] Update descriptor, walker, strict-style, and schema coverage tests so
  `style.disclosure.*` is neither unknown nor silently inactive.
- [x] Add `--strict-style` tests rejecting unknown and deprecated keys inside
  `style.disclosure`.

### Validation Checkpoint 4

- [x] `cargo test -p darkmatter` and `cargo test -p darkmatter-cli` pass
  (modulo pre-existing unrelated snapshot failures in `error_snapshots`).
- [x] `md render README.md --output markdown-plus` emits well-formed inline HTML.
- [x] `md render README.md --output auto` on a TTY uses the Terminal fold.
- [x] `--strict-style` rejects `style.disclosure.unknown_key`.

## Phase 5 — Documentation and Final Verification

Promote the disclosure feature from planned to documented and run final
verification.

- [x] Update `darkmatter/docs/rendering/disclosure.md` to describe shipped
  behavior, including syntax, targets, terminal presentation, MarkdownPlus
  output, and style frontmatter.
- [x] Cross-link the disclosure doc from the rendering-pipeline and style docs.
- [x] Update the `::file` / `::code` transclusion docs to note that
  `disclosure="…"` now emits the render-time DSL.
- [x] Run the full darkmatter test suite: `just test` or `cargo test -p
  darkmatter -p darkmatter-cli`.
- [x] Run `just lint` / `cargo clippy -p darkmatter -p darkmatter-cli` and fix
  warnings.
- [x] Run `just build` or `cargo build -p darkmatter -p darkmatter-cli` to
  confirm a clean workspace build.

### Validation Checkpoint 5

- All tests pass.
- No clippy warnings in touched crates.
- Documentation reflects shipped behavior.