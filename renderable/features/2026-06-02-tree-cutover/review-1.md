---
ready: false
agent: codex
model: ""
---

# Review 1 — Tree Cutover

## Verdict

Not ready for production.

The implementation makes real progress on the default terminal path and on
`YamlBlock`, but it does not satisfy the spec's cutover acceptance criteria.
Several production-reachable document render paths still use the legacy
serializers, and a couple of new docs now incorrectly claim the browser path is
already backed by the tree.

## Findings

### High — Acceptance Criteria #1 is not met for public browser rendering

Spec AC1 requires `Markdown::as_html` to route through the render-tree document
renderer (`renderable/features/2026-06-02-tree-cutover/spec.md:130`). The
implementation explicitly leaves it on the legacy serializer:

- `darkmatter/lib/src/markdown/mod.rs:606` calls `output::as_html(self, options)`.
- `darkmatter/lib/src/markdown/output/html.rs:153` remains the production HTML
  serializer.

The implementation notes call out two known fidelity blockers: `style:`
hyperlink/image color injection and the `.code-block` / prose stylesheet. That
is the right reason not to flip, but it means the feature is partial, not
production-ready under this spec.

Verification level: current HTML/browser coverage is in-process and browser
harness coverage for the legacy public path. The tree browser path has parity
tests, but the user-observable public `Markdown::as_html` path is not on the
tree, so verification of the cutover requirement is absent rather than merely
low-level.

### High — The public `output::for_terminal` API still routes through legacy

Spec AC1 names the terminal document pipeline as part of the cutover
(`renderable/features/2026-06-02-tree-cutover/spec.md:130`). The inherent
`Markdown::as_terminal` path was flipped, but the public
`darkmatter::markdown::output::for_terminal` function remains exported and still
calls `for_terminal_with_layout(md, options, None)`:

- `darkmatter/lib/src/markdown/output/mod.rs:35` re-exports `for_terminal`.
- `darkmatter/lib/src/markdown/output/terminal.rs:848` is still the public
  entry point.
- `darkmatter/lib/src/markdown/output/terminal.rs:849` still delegates to the
  legacy serializer.

Many tests and docs still call this public API as the "legacy" comparison
surface. If this function is still supported, it is part of the user-observable
terminal document pipeline and should either be flipped to `Markdown::as_terminal`
/ `render_tree_terminal`, or explicitly removed/deprecated outside this feature's
ready claim.

Verification level: `level2_render_tree_terminal.rs` provides Level 2 coverage
for direct tree rendering, but it does not prove this public API uses that tree
path. The strongest verification for the public `for_terminal` entry point is
still legacy-path Level 1/legacy integration coverage.

### High — Decorated `DarkmatterPage::render` remains on legacy terminal rendering

Spec AC1 requires `DarkmatterPage::render` to route through the render-tree
document renderers (`renderable/features/2026-06-02-tree-cutover/spec.md:130`).
The implementation only flips zero-config/default layout pages. Decorated
layouts still call the legacy terminal serializer:

- `darkmatter/lib/src/layout/page.rs:867` uses the tree-backed path only when
  `self.is_default_layout()`.
- `darkmatter/lib/src/layout/page.rs:870` passes `Some(&ctx)`, which reaches
  `darkmatter/lib/src/markdown/mod.rs:655`.
- `darkmatter/lib/src/markdown/mod.rs:655` calls
  `output::terminal::for_terminal_with_layout`.

This is an intentional fidelity deferral because the tree lacks
per-component alignment / fill / list-left-margin layout support. That is a
valid blocker, but it means the production `DarkmatterPage::render` surface has
not cut over.

Verification level: there is Level 2 real-terminal coverage for some page
rendering behavior in `level2_render_tree_terminal.rs`, but the remaining
decorated page path is explicitly legacy. The cutover requirement itself is not
verified for decorated pages because it is not implemented.

### High — `render_tree_*` entry points were not promoted as specified

Phase 2 requires promoting `render_tree_*` entry points from `pub(crate)` to
`pub` (`renderable/features/2026-06-02-tree-cutover/spec.md:203`). They remain
crate-private:

- `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:89`
  `pub(crate) fn render_tree_html`
- `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:115`
  `pub(crate) fn render_tree_terminal`
- `darkmatter/lib/src/markdown/render_tree/mod.rs:96` re-exports them as
  `pub(crate)`.

If the intended design changed to keep them internal, the spec should be
updated. As written, this is an unimplemented Phase 2 requirement.

### Medium — New render-tree module docs falsely claim `as_html` is tree-backed

There is direct comment drift:

- `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:6` says
  `render_tree_html` backs public `Markdown::as_html`.
- `darkmatter/lib/src/markdown/render_tree/mod.rs:88` says the same.
- Actual code at `darkmatter/lib/src/markdown/mod.rs:606` still calls
  `output::as_html`.

This is especially risky because `Markdown::as_html`'s own docs correctly state
the opposite at `darkmatter/lib/src/markdown/mod.rs:594`. Per repo convention,
assume the code is correct and fix the drifted comments.

### Medium — `DarkmatterPage` docs still pin zero-config parity to legacy `for_terminal`

Several `DarkmatterPage` docs/comments still describe zero-config rendering as
byte-for-byte equivalent to `for_terminal(default)`, while the updated tests now
compare against `Markdown::as_terminal(default)`:

- `darkmatter/lib/src/layout/page.rs:48`
- `darkmatter/lib/src/layout/page.rs:75`
- `darkmatter/lib/src/layout/page.rs:796`
- `darkmatter/lib/src/layout/page.rs:847`
- `darkmatter/lib/src/layout/page.rs:865`

That was true before the flip; now it either needs to say `as_terminal(default)`
or explicitly distinguish the still-legacy public `output::for_terminal`
function.

### Medium — Test coverage does not verify the flipped public terminal entry through Level 2

The repo has useful Level 2 tests for the direct tree terminal renderer
(`darkmatter/lib/tests/level2_render_tree_terminal.rs`), and Level 1 tests now
check zero-config `DarkmatterPage::render` against `Markdown::as_terminal`
(`darkmatter/lib/src/layout/page.rs:1842`). However, there is no Level 2 test
that drives the public post-flip entry point (`Markdown::as_terminal` or
zero-config `DarkmatterPage::render`) and captures it in a real terminal.

For this spec's user-observable terminal behavior (code-block headers, SGR
styling, wrapping, HR/image policy), Level 2 is the appropriate minimum. Direct
renderer Level 2 plus public-entry Level 1 leaves an integration gap at the
adapter boundary where options are mapped and the code renderer is wired.

## Requirement Status

| Requirement | Status | Strongest relevant verification |
|---|---:|---|
| `Markdown::as_html` on tree | Not implemented | No cutover verification; public path is legacy |
| public terminal document path on tree | Partial | Level 2 for direct tree renderer; public `output::for_terminal` remains legacy |
| zero-config `DarkmatterPage::render` on tree | Implemented | Level 1 parity against `Markdown::as_terminal`; no public-entry Level 2 |
| decorated `DarkmatterPage::render` on tree | Not implemented | Legacy path only |
| `YamlBlock` tree render only | Implemented | Level 1 unit/parity coverage |
| `FileSystem` terminal flip | Deferred/exempted | Level 1 parity for connector styling; default render remains bespoke |
| bespoke renderer deletion | Blocked | Legacy paths remain production-reachable |

## Notes

The implementation notes accurately identify the major blockers and should not
be treated as a production-ready signoff. The code is closer to a partial Phase
3/4 landing with documented deferrals than to the Phase 5 deletion-ready state
described by the spec.
