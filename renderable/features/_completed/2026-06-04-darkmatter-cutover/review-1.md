---
ready: false
agent: codex
model: ""
---

# Review 1 - Darkmatter Cutover

## Verdict

Not ready for production.

The deprecated `Page*` vocabulary and bespoke component CSS are removed, and
the terminal test inventory is strong. Two production blockers remain: the new
component-style path drops color opacity while retaining the side-channel the
spec says to delete, and browser-observable component layout/style behavior is
not verified in a real browser.

## Findings

### High - Component color opacity is lost and the retired side-channel remains

The spec requires component `color` / `bg-color` to live in
`ComponentPolicy.style` and be written directly onto node attrs
(`spec.md:88-123`). The implementation creates that `Style`, but strips the
`StyleColor.opacity` field when doing so:

- `darkmatter/lib/src/style/apply.rs:540-545` copies only `color.color` into
  renderable `Style`.
- `darkmatter/lib/src/layout/page.rs:523-539` then converts the policy back into
  `component_colors` / `component_bg_colors`; that conversion explicitly
  constructs `StyleColor { opacity: None }` at `page.rs:1280-1297`.
- `darkmatter/lib/src/markdown/render_tree/decorate.rs:151-160` ignores
  `ComponentPolicy.style` and reads those duplicate maps through
  `LayoutContext::component_color` / `component_bg_color` instead.
- The duplicate maps are still stored on `DarkmatterPage` (`page.rs:82-86`) and
  `LayoutContext` (`context.rs:42-45`), despite the design requiring the
  per-component side-channel to be deleted.

Consequently valid v1 values such as `style.table.color: red-500/50` and
`style.block-quote.bg-color: '#ff000080'` become fully opaque on the cutover
path. This is a user-visible regression and also leaves two sources of truth
that can drift. Preserve opacity in the renderable style representation (or
define an explicit supported lowering), write `policy.style` directly to the
node, and remove the duplicate component color maps.

Verification level: Level 1 coverage is missing for opacity-bearing component
colors. The real browser effect additionally needs browser-tier computed-style
coverage; terminal color application needs Level 2 where visible SGR behavior
is asserted.

### High - Browser component layout/style is only source-tested

The cutover moves table, blockquote, list, image, and component color behavior
from `build_component_css` to inline styles emitted by the browser fold. That is
user-observable browser behavior, but the new tests assert HTML substrings and
snapshots only:

- `darkmatter/cli/tests/cli.rs:4641-4658` checks `max-width` and auto-margin
  source text.
- `darkmatter/cli/tests/cli.rs:4681-4691` checks that `max-width:50%` appears.
- `darkmatter/lib/tests/cutover_reference.rs:52-153` snapshots serialized HTML.

The existing real-browser suite in `darkmatter/lib/tests/browser_render.rs`
checks code-block and HR behavior, but has no computed-style test for the
component policies changed by this feature. Source assertions cannot prove the
browser accepts the declarations, computes percentage widths against the
intended containing block, centers/right-aligns the element, or applies
foreground/background colors and opacity.

Add browser-tier tests through `ChromeHarness` for at least centered/right
aligned tables or blockquotes, percentage max-width, list left margin, and
component foreground/background color including opacity. Under the required
test-rigor policy, this level mismatch is a high-severity readiness gap.

Verification level: strongest current coverage is Level 1; browser-tier
computed-style verification is required.

### Medium - The page frame stores renderable types but immediately collapses them to cells

The design says the slim page frame stores `Edges` and
`TargetValue<Length>`, with terminal and browser frame code reading those
renderable values (`spec.md:132-147`). The fields have those types, but all
builders still accept `u16` (`darkmatter/lib/src/layout/page.rs:400-550`), and
both `LayoutContext::tv_cells` (`context.rs:107-115`) and the page renderer's
`tv_cells` (`page.rs:1103-1114`) treat every non-`Ch` value as zero.
`apply_page_style` also resolves percentage margin/padding/max-width to `u16`
before storing it (`darkmatter/lib/src/style/apply.rs:257-327`).

This leaves the page frame renderable-typed in representation only; it cannot
carry a percentage through to the browser and would silently erase a
non-`Ch` value if one reaches the frame. Either retain validated `Length`
values and resolve per target, or document and encode the page-frame contract
as cell-only rather than presenting a general `TargetValue<Length>` surface.

Verification level: Level 1 structural tests currently confirm only `Ch`
values. Add tests for each accepted `Length` variant and browser computed
behavior if percentages remain supported.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated `Page*` types and allows removed | Met | Level 1 mechanical search |
| Bespoke component CSS and width helpers removed | Met | Level 1 mechanical search |
| Component policy lowers directly to node `Layout` / `Style` | Not met for `Style` | Level 1 layout tests; color still uses duplicate maps |
| `style:` v1 input remains accepted | Partially verified | Level 1 parser/CLI tests; opacity semantics regress |
| Terminal component layout remains visible correctly | Broadly covered | Level 2 tests for table, blockquote, lists, images, HR, and colors |
| Browser component layout/style remains visible correctly | Verification gap | Level 1 source/snapshot tests only |
| Slim renderable-typed page frame | Partial | Level 1 `Ch`-only tests and Level 2 page-frame captures |
| Pronounced mode flip preserved | Covered | Level 1 snapshot plus Level 2 terminal tests |

## Verification Notes

`cargo test -p renderable fold_does_zero --color=never` passed. Full
`darkmatter` and `darkmatter-cli` runs could not be completed in this review:
the shared target directory was locked by concurrent repository sessions and
the review-owned Cargo processes exceeded the non-interactive time limit, so
they were terminated. No Level 3 coverage is required because this feature has
no keyboard or mouse interaction requirement.
