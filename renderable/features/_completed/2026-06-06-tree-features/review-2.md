---
ready: false
agent: codex
model: ""
---

# Review: Tree Features

## Findings

### High: Image exact width is applied to the alt text, not the visible placeholder

The spec requires image alt/placeholder exact width, maximum width, alignment,
padding, and truncation, with `width` establishing the exact rendered field
([spec.md](spec.md:368)).

The terminal renderer now applies `TextLayoutHints` to `alt` first and then
adds the placeholder framing
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:1048)).
As a result, a six-column exact width produces an eight-column bracket
placeholder or a fourteen-column block placeholder. The renderer's own test
explicitly approves this overflow by requesting a ten-column cap and asserting
that the final bracket placeholder is twelve columns
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:3117)).

The new Darkmatter regression test only checks that the long tail is absent; it
never checks the resulting visible width
([tree_features_characterization.rs](../../../darkmatter/lib/tests/tree_features_characterization.rs:337)).
The Level 2 suite covers a short image's alignment inside a forty-column alt
field, but has no long exact-width image case
([level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2421)).

Apply layout to the complete fallback projection, or subtract the selected
placeholder's framing width before shaping the alt. Add Level 1 assertions for
the complete visible width in both placeholder modes and a Level 2 real-terminal
capture for a long image alt under exact width.

### High: Page foreground and background are still copied onto links and images

The revised component path correctly removed page-color fallback, but the link
and image paths still append `page_color` and `page_bg_color` when no local or
component value exists
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:104)).
`apply_link_policy` and `apply_image_policy` then write those values as explicit
node styles
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:252)).

This contradicts the spec's root-inheritance contract and the implementation's
own comments: foreground should inherit from the styled root, while background
does not inherit and is painted by the page frame
([spec.md](spec.md:311),
[page.rs](../../../darkmatter/lib/src/layout/page.rs:1398)). It is also visibly
wrong for alpha-bearing page backgrounds: the page wrapper paints the
background once, then every link/image composites the same translucent paint
again.

The new structural test checks only a table, so it does not exercise either
remaining fallback
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:883)).
The browser tests verify foreground inheritance on paragraphs, headings, and a
table, but not link/image background behavior
([browser_render.rs](../../../darkmatter/lib/tests/browser_render.rs:776)).

Remove both page fallbacks from `hyperlink_color` and `image_color`. Add Level 1
tree assertions that page-only colors do not create link/image styles, plus a
real-browser computed-style test showing that a translucent page background is
painted only by the frame.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Browser and MarkdownPlus alpha lowering | Level 1 plus real-browser computed style | Appropriate |
| Terminal alpha degradation | Level 1 plus Level 2 color captures | Appropriate |
| Link exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Image exact/max width and alignment | Level 1; Level 2 only for short alignment | Gap: exact visible width is broken and lacks Level 2 verification |
| List-item placement | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Structured link/image browser attrs and CSS precedence | Level 1 tree/output tests plus real-browser computed style | Appropriate |
| Root foreground inheritance | Level 1 direct-document tests plus real-browser computed style | Appropriate |
| Page background frame-only behavior | Page-frame unit tests only | Gap: links/images still receive copied background paint |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- `cargo test -p renderable --color=never`: passed.
- `cargo test -p biscuit-terminal --color=never`: passed.
- `cargo test -p darkmatter --color=never`: passed, including 18 real-browser
  tests and the cutover reference suite.
- `darkmatter/just test-l2`: passed with real harnesses: 15 library tests and
  52 CLI tests.
- `git diff --cached --check`: passed.

The requested `root` skill is not present in the available skill catalog. This
review used the required `renderable` and `rust-testing` skills plus the
repository-root instructions supplied for the session.

## Readiness

Not ready for production. The prior review's link truncation, structured image
CSS, and root-browser-fold fixes are present and their suites pass, but image
exact width remains functionally incorrect and page paint still leaks onto
link/image nodes.
