---
ready: false
agent: codex
model: ""
---

# Review: Disclosure Blocks

## Findings

### High: Terminal disclosure body still does not emit the required dim + italic styling

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

### High: Terminal disclosure layout/color style is parsed but not rendered

The spec requires `style.disclosure.width`, `style.disclosure.max-width`, `style.disclosure.alignment`, `style.disclosure.color`, and `style.disclosure.bg-color` to parse, lower through `ComponentPolicy`, and render visibly where supported; it also explicitly requires terminal layout overrides to be honored. The implementation merges frontmatter and opener-level disclosure style into the disclosure node attributes in `darkmatter/lib/src/markdown/render_tree/build_context.rs:267`, setting layout at `:290` and colors at `:303`.

The terminal renderer never receives or reads the disclosure node, though. `biscuit-terminal/lib/src/render_tree/render.rs:686` defines `render_disclosure(&mut self, summary, children)` with no `node: &RenderNode`, so the layout/style attrs computed by the Darkmatter fold are unavailable. As a result, both frontmatter style and instance-level opener styles such as `::disclosure max-width=60ch color=red-500 ...` are ignored on the terminal target.

Verification level present: mostly Level 1 parser/apply coverage. The tests prove that the style bucket is accepted and that policy can be attached, but I did not find render-target tests proving terminal output changes for disclosure width/max-width/alignment/color/bg-color, nor Level 2 coverage for real-terminal styling/layout. Because this is user-observable terminal behavior, at least one Level 1 render-output regression and one Level 2 real-terminal capture should cover the supported visible properties, especially color and width/alignment.

## Test Rigor Notes

- Terminal dim+italic body: Level 1 is present and failing; Level 2 exists but cannot compensate for a failing direct render-target test.
- Terminal block quote glyph: Level 2 coverage exists in `darkmatter/cli/tests/level2_layout.rs:2857`, which is the appropriate tier for real-terminal glyph rendering.
- Terminal disclosure style bucket and instance-level style: strongest coverage appears to be Level 1 parse/apply tests, not render behavior. This is below the required level for user-visible terminal layout/color.
- Markdown, MarkdownPlus, Browser, JSON, nested disclosure rendering, malformed disclosure parsing, near-miss keywords, fenced-code ignores, compose invariance, and transclusion unification have appropriate Level 1 structural coverage for their deterministic string/tree behavior.

## Production Readiness

Not ready for production. The terminal target still fails a required render-target test, and disclosure-specific terminal style policy is not wired into rendering.
