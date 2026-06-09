---
ready: false
agent: codex
model: ""
---

# Review 3

## Findings

### High: An unmatched component policy still changes renderer-wide terminal color depth

The width-specific review-2 defect is fixed, but the terminal path still uses
`!self.is_default_layout()` to replace the ambient renderer color depth with the
`DarkmatterPage`'s captured depth (`darkmatter/lib/src/layout/page.rs:833-847`).
`is_default_layout()` becomes false for any component policy
(`darkmatter/lib/src/layout/page.rs:1034`), including a policy whose component is
absent from the document.

This is not limited to the policy's typed node attrs. `TerminalOptions::color_depth`
is renderer-wide: the entry point installs it on the terminal context before the
target fold (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:533-540`). For
example, with ambient no-color detection and a page created from
`Terminal::new_optimistic` (captured `TrueColor`), adding an unmatched Tables
policy can make an unrelated fenced code block emit truecolor SGR where the same
page without the policy emits no color. That violates the Option A requirement
that component policies affect only attrs attached during construction and that
an unmatched policy cannot change page output.

The new parity test uses plain prose (`darkmatter/lib/src/layout/page.rs:1909-1935`),
so it cannot observe this global capability difference. Separate capability
selection from policy presence: only explicit page/renderer capability settings
or content that actually needs the captured capability should change the global
terminal context. Add a discriminating parity test with unrelated colored
content under different ambient and captured color depths. Because SGR color is
terminal-observable, this also needs Level 2 real-terminal coverage; the current
strongest verification is Level 1 and does not exercise the defect.

### Medium: Closeout artifacts still describe an absent completed path and obsolete policy behavior

The feature is active at `renderable/features/2026-06-06-tree-closeout`, but
`plan.md`, `verification-record.md`, all three audit artifacts, and architecture
docs/skills still link to `_completed/2026-06-06-tree-closeout`, which is absent.
The traversal inventory also says policy presence controls `max_width` and calls
that compliant (`traversal-inventory.md:63`, `:131-149`), contradicting the
current geometry/ambient-width implementation and the review-2 fix.

Acceptance criterion 9 requires accurate metadata, links, docs, and skills.
Update references to the active path while review remains open, and rewrite the
page-frame audit to describe the final implementation. Move the directory and
restore completed-path links only after production readiness is established.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Browser HR kind/weight and SVG parsing | Browser, real Chrome | Appropriate |
| Browser HR left/center/right placement and `full` width | Browser computed geometry | Appropriate; review-2 gap closed |
| Terminal HR kind/weight/color/background/alignment/width | Level 2 real-terminal capture, fail-closed | Appropriate |
| Page-frame terminal width independence from unmatched policy | Level 1 parity with differing captured/ambient widths | Appropriate for width; review-2 width gap closed |
| Unmatched-policy independence for terminal color/capabilities | Level 1 plain-prose parity only | Gap; implementation is policy-dependent and Level 2 color verification is missing |
| Browser component colors/layout | Browser computed style/geometry | Appropriate |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `cargo test -p darkmatter --lib terminal_unmatched_policy_does_not_cap_width_to_captured_terminal --color=never`: passed.
- `cargo test -p renderable tree::graphics::tests --color=never`: 17 passed.
- `git diff --check` and `git diff --cached --check`: clean.

The browser full-width HR correction and page-width correction resolve the two
review-2 implementation defects. Renderer-wide color depth remains coupled to
component-policy presence, so the feature is not ready for production.
