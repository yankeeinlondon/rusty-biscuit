---
ready: false
agent: codex
model: ""
---

# Review: Disclosure Blocks

## Findings

### High: Terminal disclosure body still does not emit the required dim + italic styling

The spec requires terminal disclosure output to render the body as a block quote whose text is dim and italic. The implementation in `biscuit-terminal/lib/src/render_tree/render.rs:698` builds a dim/italic style and applies it before wrapping the body in `BlockQuote`, and the focused Level 1 test in `darkmatter/lib/tests/disclosure_render_targets.rs:110` asserts those SGR attributes are present.

That test currently fails:

```text
cargo test -p darkmatter --test disclosure_render_targets --color=never

test terminal_target_renders_summary_and_dim_italic_body ... FAILED

body must contain dim escape: License Agreement

│  Keep your hands off.
```

The body is visible and block-quoted, but the emitted terminal string does not contain the dim (`SGR 2`) or italic (`SGR 3`) styling required by the target contract. This is a functional gap, not just a test-rigor gap.

Verification level present: Level 1 failing. There is also a Level 2 WezTerm test for the same user-observable terminal styling requirement in `darkmatter/cli/tests/level2_layout.rs:2857`, which is the right tier for real-terminal rendering, but production readiness cannot be claimed while the direct render-target regression test fails. After fixing the style emission, rerun both the Level 1 render-target test and `just test-l2` for the disclosure case.

## Test Rigor Notes

- Terminal rendering: intended minimum is Level 2 for user-observable block quote glyph and dim/italic styling. A Level 2 WezTerm test exists, and the Level 1 test currently catches a failing raw-output path.
- Markdown and MarkdownPlus rendering: Level 1 structural tests cover DSL preservation and `<details>/<summary>` lowering; that is appropriate for non-terminal string output.
- Browser rendering: current Level 1 HTML assertions verify native `<details>/<summary>` and no JavaScript. A browser computed-style test is not required by the spec because native element behavior is delegated to the browser and no disclosure-specific CSS behavior is specified beyond ordinary style policy.
- Transclusion unification: Level 1 integration tests cover `::file`, `::code`, explicit summaries, and `disclosure=true` default summary. That is appropriate because compose output is deterministic text.

## Checks Run

```text
cargo test -p darkmatter --test disclosure_render_targets --color=never
# failed: terminal_target_renders_summary_and_dim_italic_body

cargo test -p darkmatter-cli --test cli markdown_plus_renders_disclosure --color=never
# passed: 2 tests

cargo test -p darkmatter --test disclosure_transclusion_integration --color=never
# passed: 4 tests
```

## Production Readiness

Not ready for production. The prior MarkdownPlus compose issue and transclusion unification appear fixed, but the terminal target still fails the specified dim + italic body styling behavior.
