---
ready: false
agent: codex
model: ""
---

# Review 2 - Darkmatter Cutover

## Findings

### High - Percentage layout is still verified at the wrong target levels

The cutover must preserve v1 percentage lengths and let each renderer resolve
them. The implementation now retains those values correctly, but the tests do
not verify the user-visible result at the required level:

- Component browser behavior for `style.table.max-width: 50%` is still checked
  only as serialized HTML (`darkmatter/cli/tests/cli.rs:4662-4692`). The new
  browser-tier percentage test targets `style.page.max-width` on
  `.darkmatter-page` (`darkmatter/lib/tests/browser_render.rs:575-601`), not a
  component node. It therefore cannot catch a browser-fold regression in the
  component policy this feature moves onto node attrs.
- Terminal percentage margins and max-width are checked in-process
  (`darkmatter/lib/tests/style_frontmatter.rs:230-272` and
  `darkmatter/lib/src/layout/page.rs:3058-3076`). No `level2_*` test captures a
  real terminal for percentage page-frame or component layout.

Add a browser-tier computed-style test for a percentage-width table or
blockquote, asserting its used width relative to its containing block. Add a
Level 2 capture for percentage margin/max-width that asserts the visible offset
and width in a real terminal. Under the review's test-rigor policy, these level
mismatches are production blockers.

### Medium - Public documentation still describes the deleted architecture

The docs acceptance criterion is not fully met. The authoritative renderable
architecture document says `PageMargin`, `PagePadding`, `PageFill`, and
`PageAlignment` remain public deprecated types with conversion bridges
(`renderable/docs/layout-and-style.md:418-434`), but this feature deleted them.
`DarkmatterPage` rustdoc also twice says decorated layouts use a legacy
serializer (`darkmatter/lib/src/layout/page.rs:53-57` and `:762-771`), while the
legacy event-stream serializers have been deleted and rendering routes through
the render tree.

Update or remove these stale statements. This repository's comment policy says
behavior-changing work must fix related drift in the same change.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated page-layout types and allows removed | Met in code | Level 1 mechanical search/build |
| Bespoke component CSS and `LayoutContext` component math removed | Met | Level 1 mechanical search |
| Component policy is the single source of layout/color truth | Met | Level 1 unit tests |
| Component color opacity survives browser rendering | Met | Browser-tier computed style |
| Terminal component layout/color behavior | Met for fixed lengths | Level 2 real-terminal capture |
| Browser component layout/color behavior | Met for fixed lengths and colors | Browser-tier computed style |
| Percentage component/page layout | Verification gap | Level 1 terminal/source tests; browser tier only for page wrapper |
| Slim renderable-typed page frame | Met | Level 1 structural/output tests; browser tier for page max-width |
| Pronounced mode flip preserved | Met | Level 1 plus Level 2 |
| Documentation updated | Not met | Manual review |

## Verification

- `cargo test -p darkmatter --lib --color=never`: 3,495 passed, 1 ignored.
- `BISCUIT_BROWSER_REQUIRED=1 cargo test -p darkmatter --test browser_render --color=never`: 13 passed.
- `cargo test -p darkmatter --test style_frontmatter --color=never`: 14 passed.
- `git diff --cached --check`: passed.

No Level 3 coverage is required because this feature has no keyboard or mouse
interaction requirement.
