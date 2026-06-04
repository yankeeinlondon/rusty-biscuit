---
ready: false
agent: codex
model: ""
---

# Review 2 — Tree Cutover

## Verdict

Not ready for production.

Iteration 2 closes several review-1 gaps: `Markdown::as_html` now calls
`render_tree_html`, the main render-tree entry points are exported, docs are
mostly aligned, and Level 2 coverage was added for the public zero-config
terminal entry points. The remaining issues are still production-facing: one
required terminal path remains on legacy, and the flipped browser path regresses
structured-link metadata and malformed code-directive error behavior.

## Findings

### High — `DarkmatterPage::render` is still only partially on the tree

Spec AC1 requires `DarkmatterPage::render` to route through the render-tree
document renderers (`renderable/features/2026-06-02-tree-cutover/spec.md:130`).
The implementation still sends decorated pages through the legacy terminal
serializer:

- `darkmatter/lib/src/markdown/mod.rs:667` switches on `layout_ctx`.
- `darkmatter/lib/src/markdown/mod.rs:669` calls
  `output::terminal::for_terminal_with_layout(self, options, Some(ctx))`.
- `darkmatter/lib/src/layout/page.rs:869` passes `Some(&ctx)` whenever the page
  is not the zero-config/default-layout page.

The docs correctly name three remaining missing tree capabilities: hyperlink
label width/alignment/truncation, image placeholder width/alignment, and
right-aligned list-item body. That is useful triage, but it means AC1 is still
not met. The feature cannot be marked production-ready under this spec while a
production `DarkmatterPage::render` branch remains legacy.

Verification level: there is Level 2 coverage for zero-config public terminal
entry points, and there are Level 1/Level 2 guards for pieces of decorated
layout. The strongest verification of the full decorated public entry point is
still legacy-path coverage because the cutover is not implemented.

### High — Tree-backed `Markdown::as_html` drops structured link metadata

Legacy HTML parses structured link titles such as
`class='btn' target='_blank' prompt='Read docs'` into HTML attributes
(`darkmatter/lib/src/markdown/output/html.rs:409`). The tree fold stores only
the URL and raw title:

- `darkmatter/lib/src/markdown/render_tree/fold.rs:539` builds
  `ContainerKind::Link { url, title }`.
- `darkmatter/lib/src/markdown/render_tree/fold.rs:737` lowers that directly to
  `RenderNode::link(url, title, children)`.
- `renderable/src/tree/render/browser.rs:1915` then emits generic node attrs,
  `href`, and the raw `title` attribute.

So a public `Markdown::as_html` call now emits the structured directive as a
literal `title="class='btn' ..."` instead of preserving `class`, `target`,
`data-prompt`, and `data-*` attributes. When frontmatter hyperlink style is also
present, `inject_inline_styles` can recover only CSS from the directive
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:253`) and still loses
the non-style attributes.

This violates AC3's no-fidelity-regression requirement. Existing tests still
cover structured links through the retained legacy `output::as_html` surface,
but there is no Level 1 test for the flipped public `Markdown::as_html` path
with structured link metadata.

Verification level: Level 1 is sufficient for this string-level HTML contract;
it is missing for the public tree-backed browser entry point.

### High — Malformed code-block directives no longer error on the browser path

Legacy `output::as_html` parsed fenced code metadata with `parse_code_info` and
propagated `InvalidLineRange` errors (`darkmatter/lib/src/markdown/output/html.rs:257`).
The tree browser path now deliberately degrades malformed directives to
language-only metadata:

- `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:72` swallows
  `parse_code_info` errors with `unwrap_or_else`.
- `darkmatter/lib/src/layout/page.rs:2506` changed the browser API test to
  expect success for `highlight=1-2-3`.

That is a user-visible behavior change on the newly flipped `Markdown::as_html`
/ `DarkmatterPage::render_to_browser` path. The spec allows documented
improvements such as `<mark>` recovery, but this is not called out as an
accepted improvement in the cutover spec and it weakens validation of malformed
author input. Either keep the legacy error semantics in the tree code renderer
for fatal DSL parse errors, or update the spec and CLI/user-facing error
contract explicitly.

Verification level: Level 1 is appropriate for this API error contract. There
is coverage, but it asserts the regression rather than the legacy-compatible
behavior required by AC3.

### Medium — `render_tree` module docs still contain visibility drift

`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:19` says all entry
points are `pub(crate)`, but `render_tree_html`, `render_tree_terminal`, and the
markdown renderers are now `pub` and re-exported publicly from
`darkmatter/lib/src/markdown/render_tree/mod.rs:88`.

This is comment drift introduced while fixing review-1 finding 4. Per repo
convention, assume the code is correct and update the module docs.

## Requirement Status

| Requirement | Status | Strongest relevant verification |
|---|---:|---|
| `Markdown::as_html` on tree | Implemented, but with fidelity regressions | Level 1/browser tests for some features; missing Level 1 for structured-link metadata |
| default `Markdown::as_terminal` on tree | Implemented | Level 2 public-entry real-terminal coverage |
| zero-config `DarkmatterPage::render` on tree | Implemented | Level 2 public-entry real-terminal coverage |
| decorated `DarkmatterPage::render` on tree | Not implemented | Legacy path only |
| no browser functional/fidelity regressions | Not met | Malformed code directives now degrade; structured link metadata lost |
| `YamlBlock` tree render only | Implemented | Level 1 unit/parity coverage |
| deletion of bespoke renderers | Blocked | Legacy decorated terminal path remains production-reachable |

## Notes

I did not re-run the full test suite during this review. The findings above are
from code inspection of the staged implementation and the recorded validation
notes.
