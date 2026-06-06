---
ready: false
agent: codex
model: ""
---

# Review 4 - Darkmatter Cutover

## Findings

### High - Browser page max-width no longer centers the page frame

The spec explicitly retains page-frame max-width centering, but
`wrap_browser_html` emits the authored page margins followed by `max-width`
without adding auto side margins
(`darkmatter/lib/src/layout/page.rs:1308-1325,1347-1361`). With the default
zero margins, a constrained `.darkmatter-page` remains left-aligned in the
viewport.

The browser-tier page test does not catch this. It reads the computed
`max-width` declaration and accepts either pixels or the literal percentage
(`darkmatter/lib/tests/browser_render.rs:595-620`); it never checks the
wrapper's used width or horizontal position. The new component percentage test
correctly verifies the table-to-page width ratio
(`darkmatter/lib/tests/browser_render.rs:645-665`), but that ratio remains 50%
even when the page wrapper itself is incorrectly left-aligned.

Restore page-frame centering when max-width constrains the wrapper, while
preserving explicitly authored side margins according to the intended page
margin contract. Add a browser-tier geometry assertion using the wrapper and
viewport/body rectangles, verifying both the used max-width and equal left/right
offsets.

Verification level: terminal page max-width has Level 2 coverage, but the
browser-observable centering requirement has only Level 1 source assertions and
a browser-tier declaration check. The required browser-tier used-geometry
verification is missing.

## Requirement Status

| Requirement | Status | Strongest verification |
|---|---:|---|
| Deprecated page-layout types and allows removed | Met | Level 1 mechanical search/build |
| Bespoke component CSS and `LayoutContext` component math removed | Met | Level 1 mechanical search |
| Component policy is the single source of layout/color truth | Met | Level 1 unit tests |
| Component color opacity survives browser rendering | Met | Browser-tier computed style |
| Terminal component layout/color behavior | Met | Level 2 real-terminal capture |
| Browser component layout/style, including percentage width | Met | Browser-tier used geometry/computed style |
| Terminal page frame, percentage sizing, and pronounced mode | Met | Level 1 plus Level 2 |
| Browser page max-width centering | Not met | Browser tier checks declaration only |
| `style:` v1 parsing and strict warning surface | Met | Level 1 parser/CLI tests |
| Documentation updated | Met | Manual review |

## Verification

- `BISCUIT_BROWSER_REQUIRED=1 cargo test -p darkmatter --test browser_render browser_component_table_width_percent_resolves_against_container --color=never`: passed.
- `cargo test -p darkmatter apply_preserves_component_color_opacity --lib --color=never`: passed.
- `cargo test -p darkmatter page_frame_stores_renderable_types --lib --color=never`: passed.
- `cargo test -p darkmatter --test style_frontmatter --color=never`: 14 passed.
- Mechanical searches found no removed vocabulary or helpers in active darkmatter code.
- No Level 3 coverage is required because this feature has no keyboard or mouse interaction requirement.
