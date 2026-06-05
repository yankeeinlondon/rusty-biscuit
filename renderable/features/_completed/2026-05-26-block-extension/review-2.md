---
ready: false
agent: codex
model: ""
---

# Review: Block Extension - HR-Attribute Lift, Iteration 2

## Findings

### High - HR `kind` is parsed and attached, but the tree renderers still read `style`

The block-extension path now lowers both canonical `kind` and legacy `style` to the `darkmatter.hr.kind` hint, which matches the spec's precedence rule and hint namespace. The terminal and browser renderers still look for `darkmatter.hr.style`, so the authored visual rule kind is lost on the tree rendering path:

- `darkmatter/lib/src/markdown/render_tree/fold.rs:350` sets only `darkmatter.hr.kind`.
- `biscuit-terminal/lib/src/render_tree/render.rs:1432` reads only `darkmatter.hr.style` before choosing `RuleStyle::Waves`, `Dots`, etc.
- `renderable/src/tree/render/browser.rs:431` exports only `style`, `alignment`, `weight`, `width`, and `color` as `data-hr-*`, so `kind` never reaches HTML.

The new snapshots accidentally pin this broken behavior. `render_tree_hr_snapshots__terminal_waves.snap` and `render_tree_hr_snapshots__terminal_kind_waves.snap` both show the default dashed rule glyph (`╌`), not the waves glyph the fixture asks for. The HTML snapshots for `style: waves` and `kind: waves` both render a bare `<hr>` with no `data-hr-kind` or `data-hr-style`.

This violates the spec's preserved behavior for HR attributes and the documented "same render-side behavior" expectation. It also means canonical `kind: waves` and legacy `style: waves` parse successfully but do not produce the requested user-visible style in tree-rendered terminal or HTML output.

Recommended fix: make renderers consume `darkmatter.hr.kind` as the visual selector, optionally falling back to `style` for older nodes. Update the HR snapshots so `waves` / `kind_waves` visibly differ from a plain rule, and assert the browser output includes `data-hr-kind="waves"` or an equivalent style-bearing surface.

## Test Rigor Notes

- HR paragraph recognition, malformed attributes, source body ranges, blockquote handling, list-item non-rewrite, fenced-code defense, and warning preflight are covered at Level 1.
- Byte-stable tree-pipeline snapshots now cover the named HR fixtures, including `mark_dim_hr`, at Level 1.
- Terminal HR styling has a Level 2 test, but its assertion is too broad: `level2_tree_hr_attributes_render_styled_rule_in_real_terminal` passes if the captured pane contains any `~`, including shell prompt/path text, and it does not isolate the rule line. It did not catch the renderer reading the wrong hint key.
- Browser/HTML HR kind visibility is only Level 1 string/snapshot coverage, and the current snapshots encode the missing kind.

## Verdict

Not ready for production. The block-extension parser lift itself is largely in place, but the feature loses the HR visual selector at render time, and the current Level 2 terminal coverage is not strong enough to prove the user-observable styling requirement.

## Verification Run

- `cargo test -p darkmatter --lib block_extension --color=never` - passed, 17 tests.
- `cargo test -p darkmatter --test render_tree_hr_snapshots --color=never` - passed, 3 tests, but snapshots pin the missing HR kind rendering.
- `cargo test -p darkmatter --lib scan_inline_hr_warnings --color=never` - passed, 4 tests.
- `cargo test -p darkmatter --test render_tree_parity render_tree_parity_hr_attributes --color=never` - passed, 2 tests.
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_hr_attributes_render_styled_rule_in_real_terminal --color=never` - passed, 1 test, but the assertion is a false-positive risk as described above.
