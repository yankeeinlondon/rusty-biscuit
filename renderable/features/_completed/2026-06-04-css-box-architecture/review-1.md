---
ready: false
agent: codex
date: 2026-06-06
---

# Review 1 - CSS Box Architecture

## Verdict

Not ready for the parent-spec cutover.

The four child implementations are individually mature and heavily tested, but
the integrated production path does not satisfy the parent's defining
architecture: policy is not baked into attrs during tree construction, and the
targets are not single folds over those attrs.

## Findings

### High - The committed cutover reference suite is red

A fresh Level 1 run failed five tests in
`darkmatter/lib/tests/cutover_reference.rs`:

- `reference_block_quote_width_and_left`
- `reference_list_left_margin`
- `reference_page_background_pronounced`
- `reference_centered_table`
- `reference_table_max_width`

All failures are stale browser snapshots. The committed snapshots expect the
page wrapper's default horizontal margins as `0ch`; the current browser
centering implementation emits `auto`. This appears to be the intended
max-width centering change from the final darkmatter review, but it was not
accepted into the cutover reference corpus.

Review the five diffs and re-baseline them with an explicit rationale if
automatic centering is correct. Then rerun the complete Level 1 suite. A
cutover cannot be declared ready while its dedicated reference suite fails.

### High - The `LayoutContext` side-channel and second decorate traversal remain

The parent requires `style:` to lower once at tree-build time, with no
`LayoutContext` component side-channel, `component_for` lookup, or second
decorate pass (`spec.md:44-59`, `:127-136`, `:175-195`). The active paths still
implement exactly that pre-cutover shape:

- `LayoutContext` carries `component_policies` plus hyperlink/image policy
  (`darkmatter/lib/src/layout/context.rs:41-50`).
- Both terminal and browser fold the document first, then call
  `decorate_document` (`entrypoints.rs:330-344`, `:886-897`).
- `decorate_document` performs a second recursive walk and maps
  `NodeKind -> PageComponent -> HashMap` (`decorate.rs:25-68`, `:94-123`).
- `DarkmatterPage` clones the policy map into each render-time context
  (`layout/page.rs:784-800`, `:883-898`).

This fails parent AC4 and AC6 and means the requested full cutover has not
happened. Move component policy into the Markdown-to-tree build context and set
`Layout` / `Style` as each node is created. Reduce `LayoutContext` to the
explicitly retained page-frame residue, then delete the component policy fields,
`component_for`, `decorate_document`, and the `*_with_layout` split.

The same change should use the shared `InheritedStyle` contract rather than
copying page-color fallback onto component nodes through
`LayoutContext::component_color` / `component_bg_color`.

### High - Render-time policy math and content mutation remain outside the renderer fold

The parent says render-time re-derivation is forbidden, and the darkmatter child
spec says `decorate.rs` performs no width/offset math. The active decorate pass
still:

- pre-renders and replaces hyperlink labels/image alt text through
  `bespoke::apply_inline_text_layout` using `ctx.effective_width`
  (`decorate.rs:232-288`);
- resolves list-item width and alignment padding itself, then stores the result
  in `darkmatter.li` JSON hints (`decorate.rs:309-335` and following);
- special-cases lone-image paragraph layout instead of projecting the intended
  block policy when the node is built (`decorate.rs:126-149`).

These are additional policy representations, not target degradation rules.
They can drift from `Layout` resolution and make the tree depend on the
terminal-sized `LayoutContext` before either target folds it. Represent the
required semantics as typed tree attrs/hints and teach the shared renderer
folds to resolve them, or explicitly narrow the parent architecture before
claiming completion.

Add structural tests that construct a styled document and assert the completed
tree already contains all required attrs with no decorate call and no
target-width-derived text replacement.

### High - Browser opacity bypasses typed attrs and performs a bespoke post-render rewrite

`ComponentPolicy` retains `StyleColor` because renderable `Style` cannot carry
alpha (`layout/page.rs:31-44`). The decorate pass serializes opacity-bearing
colors into `darkmatter.style` JSON hints (`decorate.rs:151-201`). The browser
path then performs another tree traversal, adds sentinel classes, renders HTML,
and mutates the HTML string to splice `rgba(...)` declarations
(`entrypoints.rs:338-344`, `:348-421`).

That directly conflicts with “no bespoke per-target CSS” and “every target is a
single fold over attrs.” It also reintroduces the exact hot-path costs the
parent's performance rule is intended to prevent: `get_hint` formats a lookup
key when `data` is non-empty (`renderable/src/tree/attrs.rs:869-880`), while the
perf gate explicitly ignores all extension-namespace accesses
(`attrs.rs:782-827`, `:2408-2456`).

Extend the shared color/style vocabulary with typed alpha support, lower it into
`Style`, and emit browser `rgba()` directly from the browser fold while terminal
degrades alpha intentionally. Delete `darkmatter.style`, sentinel classes, and
`apply_style_merges`.

The structural perf gate must exercise the actual styled darkmatter production
entry points and fail on extension-bag accesses used for first-class style
behavior. The current Criterion page cases use only page `max_width`
(`darkmatter/lib/benches/render_pipeline_steps.rs:160-173`), so they do not
trend component-policy decoration or opacity rewriting.

### Medium - The parent spec's completion metadata and links are stale

The parent still says `status: ready for planning and implementation`, lists
four `2026-06-05-*` child IDs, labels every child “designed; ready for
planning,” and links to active directories that no longer exist
(`spec.md:1-12`, `:101-106`). The actual child specs are under
`features/_completed/2026-06-04-*`; the parent link to the completed tree-cutover
spec is also broken.

Update the metadata, status table, parent/child links, and acceptance checklist
only after resolving the architecture findings above. Until then, do not mark
the parent completed merely because the child directories are completed.

## Coverage Assessment

Coverage is strong for the behavior that currently exists:

- Fresh `renderable` run: 412 unit tests, 22 integration tests, and 81 doctests
  passed.
- Fresh biscuit-terminal Level 1 run: 2,785 tests passed; one leaked-handle
  retry passed on its second attempt.
- Fresh darkmatter/darkmatter-cli Level 1 run stopped after 3,579 passes and
  five failing cutover-reference snapshots; 665 tests were not run because
  nextest was fail-fast.
- Prior child reviews record passing real-terminal box-model coverage and
  browser computed-style/geometry coverage.

The missing coverage is architectural, not another output snapshot: no test
proves that a styled tree is complete when built, that production rendering
uses no component side-channel/decorate pass, or that browser style output is
produced without JSON hints and post-render string mutation.

## Readiness

The CSS box vocabulary, typed attrs, terminal/browser box rendering, and v1
frontmatter compatibility are ready. The parent program is not 100% ready for
cutover until the remaining policy side-channel, decorate-time math, and
browser opacity rewrite are removed or the parent spec is explicitly revised
to permit them.
