---
kind: log
implementation_1: "2026-10-27T12:27:43-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-27T12:27:43-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/biscuit-terminal/fixes/2026-10-27-horizontal-rule-browser-parity/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Outer bottom margin has no independent fragment assertion | fixed | All children verified; mutation that zeroes the projected bottom margin now fails `test_fragment_outer_layout_wraps_rule` |
| R1-F1.1 | Outer-layout fragment test: bottom `ch(1)` must appear as `margin-bottom:1lh` exactly once on the outer wrapper; dropping it must fail the test | fixed | New `wrapper_decls` helper parses the wrapper's `style`; asserts `margin-bottom:1lh` exactly once there and once in the whole HTML. Mutation (projection replaces bottom with `ch(0)`) → test FAILED at the bottom assertion; mutation reverted (`set_layout(&self.layout)` restored, diff is test-only) |
| R1-F1.2 | Same test: top and side margins asserted independently | fixed | Sibling of the class: `margin-top:2lh` was a whole-HTML `contains`; now exactly once on the wrapper, as is `margin-left:5ch`; unauthored right edge asserted as `margin-right:0`; each edge declared once on the wrapper; terminal `2ch` still asserted absent |
| R1-F1.3 | Same test: width, maximum width, outer alignment asserted | fixed | Sibling: `width:50%`, `max-width:40ch`, `margin-left:auto`, `margin-right:auto` were whole-HTML substring checks (`width:50%` could match the SVG's `--hr-width: 50%` style if spacing changed); now each exactly once in the wrapper's declarations |
| R1-F1.4 | Percent-margin test: authored top `2%` asserted | fixed | Sibling: `test_render_html_fragment_percent_margin` now requires `margin-top:2%` exactly once on the wrapper rather than anywhere in the HTML |
| R1-F1.5 | Routing and default-layout tests | clean | Routing test's contract is byte equality with the adapter (any lost edge changes both equally; independent behavior is owned by F1.1–F1.4); default-layout test asserts no `<div` at all, which no wrapper-edge loss can satisfy wrongly |
| R1-F1.6 | Style/weight and custom-attribute snapshots (default layouts) | clean | Default `Layout` authors no outer margins, so there is no edge to lose; snapshots unchanged (147/147 `horizontal_rule` tests pass) |
| R1-F1.7 | Tree projection, browser adapter, shared outer-box lowering forward all four edges | clean | `mod.rs` projection calls `node.attrs.set_layout(&self.layout)` with the full layout; `renderable/src/tree/render/browser.rs:3329-3332` iterates top/bottom/left/right in one table; rendered wrapper observed as `margin-top:2lh;margin-bottom:1lh;margin-left:5ch;margin-right:0` |

### Progress

- starting the work on 'R1-F1 Outer bottom margin has no independent fragment assertion' at 12:28:10
        - defect class: outer-layout declarations were checked by substring presence anywhere in the fragment, rather than as exact declarations on the outer wrapper, so a lost or duplicated edge could go unnoticed
        - sweep: every outer-layout assertion in `horizontal_rule/mod.rs` tests (`test_render_html_fragment_percent_margin`, `test_fragment_routes_through_tree_adapter`, `test_fragment_outer_layout_wraps_rule` margins and sizing halves, `test_fragment_default_layout_adds_no_wrapper`) plus the projection and shared lowering
        - added `wrapper_style`/`wrapper_decls` test helpers; converted the margin, sizing, and percent-margin assertions to exact-once declarations on the wrapper
        - discovered: the shared renderer emits all four edges, so an unauthored edge renders `margin-right:0`; the test now asserts that rather than absence
        - mutation check: projection temporarily zeroed the bottom margin → test failed; source restored, then rerun green
        - implemented directly by the orchestrator: the repair is a single test-function edit, so delegating it would only have added handoff loss
- work completed for 'R1-F1 Outer bottom margin has no independent fragment assertion' at 12:30:55

### Completion Report

- **outcome:** complete
- **elapsed:** about 4 minutes (12:27:43 → 12:31)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 7 (4 fixed, 3 clean)
- **open finding IDs:** none
- **changed files:**
        - `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` (tests only: R1-F1.1–F1.4)
        - this log
- **verification:**
        - `just test horizontal_rule` (biscuit-terminal): 147 passed
        - mutation of the projected bottom margin: `just test test_fragment_outer_layout_wraps_rule` failed as required (R1-F1.1); reverted
        - `just test` (biscuit-terminal): 3,931 passed, 55 skipped by tier
        - `just lint` (biscuit-terminal): exit 0
        - `cargo check -p biscuit-terminal --tests --all-features`: passed
        - `just check-tier-coverage biscuit-terminal`: 0 stranded
- **cross-OS:** not run; the change only alters string assertions in an L1 test, with no path, process, or OS-specific code
