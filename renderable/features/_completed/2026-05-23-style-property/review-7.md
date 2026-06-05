---
ready: false
agent: codex
model: ""
---

# Review 7

## Findings

### High: `style.hyperlinks.{width,max-width,alignment}` is never applied in terminal output

The spec requires hyperlink `width`, `max-width`, and `alignment` to affect the terminal fallback/display text before OSC 8 wrapping. The implementation parses and stores the bucket in `apply_bespoke_style`, but the terminal renderer only consumes hyperlink foreground/background color. In the non-table path, `Start(Link)` calls `ctx.hyperlink_color(is_local)` and pushes a color scope; in the table path, `End(Link)` again resolves only `hyperlink_color` before `push_table_link` (`darkmatter/lib/src/markdown/output/terminal.rs:1307`, `darkmatter/lib/src/markdown/output/terminal.rs:1330`). There is no corresponding use of `CommonStyle.width`, `max_width`, or `alignment`, and `LayoutContext::hyperlink_color` explicitly returns only colors (`darkmatter/lib/src/layout/context.rs:288`).

This also means `Length::Css` values for hyperlink width/max-width are accepted silently instead of producing the documented terminal layout error when they would affect terminal layout.

Verification level: strongest coverage appears to be Level 1 for hyperlink colors plus one Level 2 color/OSC8 table test. There is no Level 1 assertion for hyperlink width/alignment bytes and no Level 2 real-terminal capture for visible hyperlink layout. Because this is user-observable terminal layout, the implementation and verification are not production-ready.

### High: `style.images.local-style.{width,max-width,alignment}` is ignored for terminal fallback text

The spec requires local image style to apply width/max-width/alignment to the rendered terminal fallback text. The renderer computes local image colors (`image_fg`, `image_bg`) and then calls `apply_component_layout(..., PageComponent::Images, ctx)` (`darkmatter/lib/src/markdown/output/terminal.rs:1769`, `darkmatter/lib/src/markdown/output/terminal.rs:1785`, `darkmatter/lib/src/markdown/output/terminal.rs:1798`, `darkmatter/lib/src/markdown/output/terminal.rs:1807`). `apply_component_layout` resolves only the regular `PageComponent::Images` component fill/alignment, not `local_image_style` (`darkmatter/lib/src/markdown/output/terminal.rs:3260`). `LayoutContext::image_color` likewise merges only the color fields for local images (`darkmatter/lib/src/layout/context.rs:320`).

So a document with only `style.images.local-style.alignment: right` or `width: 20ch` will not change local terminal fallback layout. This is a direct functionality gap, and it also leaves the CSS-length rejection rule unenforced for local image terminal layout.

Verification level: the new tests cover local image color in Level 1 HTML/terminal paths, but I did not find Level 1 or Level 2 coverage for local image terminal width/alignment. The spec explicitly calls for terminal fallback behavior, so this needs at least Level 2 capture for visible layout, plus Level 1 error tests for width/max-width/CSS-length cases.

### High: Global hyperlink HTML style overwrites existing per-link inline CSS

The spec says existing per-link inline CSS from `Link::with_style` wins over global frontmatter style for the same CSS property, with frontmatter filling only missing declarations. In `as_html`, the link is parsed from the Markdown title metadata, then frontmatter CSS is applied with `with_style(css)` (`darkmatter/lib/src/markdown/output/html.rs:380`, `darkmatter/lib/src/markdown/output/html.rs:398`). `Link::with_style` replaces the entire existing style object, so a link like `[x](https://e "style='color: green; width: 10ch'")` under `style.hyperlinks.color: red-500` loses its per-link style instead of preserving `color: green` and filling only missing properties.

Images have a merge helper where existing CSS wins (`merge_css_style`), but links do not use an equivalent helper. Add a link-side merge and tests where existing `color` wins while frontmatter adds a missing property such as `background-color` or `max-width`.

Verification level: current coverage asserts global hyperlink color appears in HTML, but does not cover the per-link precedence rule. This is an in-process HTML output requirement, so a focused Level 1 integration test is sufficient.

### High: Level 2 coverage does not match several sub-spec #7 user-visible requirements

Sub-spec #7 introduces user-visible terminal behavior for page code theme, hyperlink colors/layout, and local image fallback styling. The Level 2 additions currently verify hyperlink color inside a table and prior HR/color behavior, but I did not find Level 2 coverage for:

- `style.page.code.theme` changing terminal code-block rendering.
- CLI `--code-theme` overriding `style.page.code.theme` in terminal rendering.
- local hyperlink override making local links visually different from remote links in a real terminal.
- local image fallback color/background rendering in a real terminal.
- hyperlink width/alignment and local image width/alignment in real terminal output.

Per the requested rigor policy, user-observable SGR styling and visible layout need Level 2 capture. The existing Level 1 tests are useful, but they are not enough to mark those terminal behaviors ready.

## Notes

I attempted targeted Cargo tests for `darkmatter` and `darkmatter-cli`, but the commands spent over a minute in dependency compilation / Cargo lock contention, so I stopped them and did not record a passing test run.

## Verdict

Not ready for production. The parser/applicator surface is mostly present, but terminal width/alignment behavior for hyperlinks and local images is not implemented, per-link CSS precedence is broken for HTML hyperlinks, and the Level 2 coverage does not yet match the spec's user-visible terminal requirements.
