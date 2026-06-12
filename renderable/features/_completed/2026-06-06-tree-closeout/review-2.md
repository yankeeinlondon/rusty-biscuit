---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: Browser `full` HR alignment still renders a partial-width rule

`HrAlignment` documents `Full` as stretching the rule across the whole width
(`renderable/src/tree/attrs.rs:1364`), and the terminal renderer preserves that
contract by mapping it to `RuleAlignment::Full`. The browser SVG path only sets
both horizontal margins to zero (`renderable/src/tree/graphics.rs:87`) while
still emitting the authored width unchanged (`renderable/src/tree/graphics.rs:97`).
Consequently, `alignment: full` with `width: 50%` produces a left-anchored 50%
rule rather than a full-width rule, diverging from Terminal and the typed API.

The new real-browser test does not catch this. It deliberately supplies `50%`
for every alignment but asserts only that `Full` has zero margins
(`darkmatter/lib/tests/browser_render.rs:202`); it never measures the SVG width
or bounding rectangle. Make `Full` resolve the outer SVG width to `100%` (or
define and document a different cross-target contract), then add a browser
geometry assertion that its rendered width equals the containing block while
left/center/right remain narrow.

### High: Terminal page framing still branches on component-policy presence

The review-1 fix changed the width-cap condition to `!self.is_default_layout()`
(`darkmatter/lib/src/layout/page.rs:816`), but `is_default_layout()` still
requires `self.component_policies.is_empty()` (`darkmatter/lib/src/layout/page.rs:999`).
An otherwise zero-configuration page with any component policy therefore sets
`options.max_width` to the page's captured terminal width and forces the
context-aware page path, even when the policy matches no node. That is still a
page-frame decision driven by component-policy state, contrary to Option A's
required boundary.

The new L1 parity test uses `Terminal::new_optimistic(80)` and compares output
under the current ambient width (`darkmatter/lib/src/layout/page.rs:1903`). It
can pass because auto-detection also commonly resolves to 80; it does not vary
ambient and captured widths, so it cannot prove the claimed independence.
Separate the decisions for construction-time policy application and page-frame
width/color behavior. Add a deterministic test with different ambient and
captured widths and an unmatched policy, plus Level 2 capture if the retained
behavior intentionally changes real-terminal wrapping.

### Medium: Closeout artifacts still describe paths and behavior that no longer exist

The feature currently lives under `renderable/features/2026-06-06-tree-closeout`,
but `plan.md`, `verification-record.md`, and the audit artifacts still link to
`renderable/features/_completed/2026-06-06-tree-closeout`, which is absent in
this working tree. The traversal inventory also explicitly claims that policy
presence controls the width cap and calls that compliant
(`traversal-inventory.md:63`, `:131-149`), contradicting both review 1 and the
new source comments.

Acceptance criterion 9 requires accurate metadata, links, docs, and skills.
Either finish the move to `_completed` after this review closes or update all
frontmatter and links to the active path. Rewrite the page-frame audit after
the policy-dependent width behavior is actually removed.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Browser HR kind/weight and SVG parsing | Browser, real Chrome | Appropriate |
| Browser HR left/center/right placement | Browser, computed margins | Appropriate |
| Browser HR `full` width | Browser test checks margins only | Gap; browser geometry does not verify the required width |
| Terminal HR color/background/alignment/width | Level 2 real-terminal capture, now fail-closed | Appropriate |
| Page-frame policy independence | Level 1 byte comparison at one coincident width | Gap; implementation remains policy-dependent and the test is non-discriminating |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `cargo test -p renderable tree::graphics::tests --color=never`: 16 passed.
- `git diff --check`: clean.
- Focused darkmatter tests could not be completed: concurrent review-owned
  Cargo processes contended on the shared build lock for over 60 seconds and
  were terminated per the non-interactive session rules.

The HR enum conversion, fail-closed Level 2 assertions, and narrowed
performance claims address substantial parts of review 1, but the two
user-observable high-severity gaps above prevent production readiness.
