---
ready: true
agent: codex
model: ""
---

# Review: Disclosure Blocks

## Resolution

Both findings are addressed.

1. **Dim + italic body** — already fixed in commit `367a85041`. The Level 1 test
   `terminal_target_renders_summary_and_dim_italic_body` and the Level 2
   `level2_disclosure_body_renders_as_dim_italic_block_quote` both pass.

2. **Disclosure layout/color style** — the renderer was never the problem: the
   generic terminal `render()` path already reads `node.attrs.layout`/`style` and
   applies width, max-width, alignment, and color around any node, disclosures
   included. The real defect was upstream in the fold:
   `split_disclosure_directives` (`darkmatter/lib/src/markdown/render_tree/fold.rs`)
   split the opener at the bare `::disclosure` keyword, stranding inline
   `key=value` style tokens in a following text event where they were mistaken
   for summary content (`parse_disclosure_opener_style` saw an empty tail, so the
   node's `style` was `None`). The fix keeps the remainder of the opener line
   attached to the directive event so the existing opener handler parses the
   inline style. Frontmatter `style.disclosure.*` already lowered and rendered
   correctly through the CLI's `apply_disclosure_style` + `apply_color_style`
   sequence.

   New regression coverage:
   - Level 1 (`darkmatter/lib/tests/disclosure_render_targets.rs`):
     `inline_opener_style_is_parsed_off_the_summary` (parse),
     `terminal_target_honors_inline_opener_style` (terminal color + alignment +
     max-width wrap), and `terminal_target_honors_frontmatter_disclosure_style`
     (frontmatter color + max-width wrap).
   - Level 2 (`darkmatter/cli/tests/level2_layout.rs`):
     `level2_disclosure_honors_inline_opener_color_and_width` (real-terminal
     truecolor + width wrapping).

## Findings

### High (RESOLVED): Terminal disclosure body still does not emit the required dim + italic styling

The spec requires terminal disclosure output to render the body as a block quote whose text is dim and italic. The current terminal renderer builds a dim+italic style in `biscuit-terminal/lib/src/render_tree/render.rs:698`, but the focused Level 1 test still fails: `darkmatter/lib/tests/disclosure_render_targets.rs:121` expects SGR 2 and `:122` expects SGR 3, and the actual output is only:

```text
License Agreement

│  Keep your hands off. 
```

Check run:

```text
cargo test -p darkmatter --test disclosure_render_targets --color=never
# failed: terminal_target_renders_summary_and_dim_italic_body
# 10 passed, 1 failed
```

Verification level present: Level 1 failing. The requirement is also user-observable terminal styling, so Level 2 is the right production-confidence tier after the raw-output bug is fixed. A Level 2 test exists in `darkmatter/cli/tests/level2_layout.rs:2857`, but production readiness cannot be claimed while the direct render-target regression test fails.

### High (RESOLVED): Terminal disclosure layout/color style is parsed but not rendered

The spec requires `style.disclosure.width`, `style.disclosure.max-width`, `style.disclosure.alignment`, `style.disclosure.color`, and `style.disclosure.bg-color` to parse, lower through `ComponentPolicy`, and render visibly where supported; it also explicitly requires terminal layout overrides to be honored. The implementation merges frontmatter and opener-level disclosure style into the disclosure node attributes in `darkmatter/lib/src/markdown/render_tree/build_context.rs:267`, setting layout at `:290` and colors at `:303`.

The terminal renderer never receives or reads the disclosure node, though. `biscuit-terminal/lib/src/render_tree/render.rs:686` defines `render_disclosure(&mut self, summary, children)` with no `node: &RenderNode`, so the layout/style attrs computed by the Darkmatter fold are unavailable. As a result, both frontmatter style and instance-level opener styles such as `::disclosure max-width=60ch color=red-500 ...` are ignored on the terminal target.

Verification level present: mostly Level 1 parser/apply coverage. The tests prove that the style bucket is accepted and that policy can be attached, but I did not find render-target tests proving terminal output changes for disclosure width/max-width/alignment/color/bg-color, nor Level 2 coverage for real-terminal styling/layout. Because this is user-observable terminal behavior, at least one Level 1 render-output regression and one Level 2 real-terminal capture should cover the supported visible properties, especially color and width/alignment.

## Test Rigor Notes

- Terminal dim+italic body: Level 1 is present and failing; Level 2 exists but cannot compensate for a failing direct render-target test.
- Terminal block quote glyph: Level 2 coverage exists in `darkmatter/cli/tests/level2_layout.rs:2857`, which is the appropriate tier for real-terminal glyph rendering.
- Terminal disclosure style bucket and instance-level style: strongest coverage appears to be Level 1 parse/apply tests, not render behavior. This is below the required level for user-visible terminal layout/color.
- Markdown, MarkdownPlus, Browser, JSON, nested disclosure rendering, malformed disclosure parsing, near-miss keywords, fenced-code ignores, compose invariance, and transclusion unification have appropriate Level 1 structural coverage for their deterministic string/tree behavior.

## Production Readiness

Ready for production. The terminal target emits the required dim + italic body
styling, and disclosure-specific layout/color style — both inline opener tokens
and `style.disclosure.*` frontmatter — parses and renders, covered by new Level 1
render-output regressions and a Level 2 real-terminal capture.
