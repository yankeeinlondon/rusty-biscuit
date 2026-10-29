---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T12:31:58-07:00
spec: 2026-10-27-horizontal-rule-browser-parity/spec.md
implemented: false
description: "A **fix** review of `2026-10-27-horizontal-rule-browser-parity/spec.md`"
fix: 2026-10-27-horizontal-rule-browser-parity/review-2.md
previous: 2026-10-27-horizontal-rule-browser-parity/review-1.md
---

# Review 2: Horizontal Rule Browser Parity

The fix is production ready within the specification's scope. The previous finding is verified fixed across all seven required instances. No new findings or human decisions remain.

Reviewed the specification, plan and implementation logs, review 1, the repair's Finding Ledger, the implementation diff from `d7ea96a1f`, and current rendering and test paths. Changes since review 1 strengthen L1 assertions only. Review 1 contains no Human rulings section; its accepted implementation decisions remain accepted. The explicitly excluded terminal/browser difference for Full alignment remains outside this fix.

## Previous Finding Verification

The parent finding concerns independent checks that authored outer layout reaches the public browser fragment. Every child from review 1 and the repair ledger was checked; rows previously clean were checked again rather than treated as additional defects.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Outer bottom margin has no independent fragment assertion | R1-F1.1–R1-F1.7 | verified fixed | All seven rows below checked against implementation; repaired test detects deliberately discarded bottom margin | Nothing |
| R1-F1.1: Bottom margin | Bottom `ch(1)` becomes `margin-bottom:1lh`, once on the wrapper | verified fixed | [Outer-layout test](../../lib/src/components/horizontal_rule/mod.rs#L2847) counts the exact declaration on the wrapper and in the complete fragment. Reviewer mutation substituting zero for the projected bottom margin fails at this assertion | Nothing |
| R1-F1.2: Top and side margins | Top `ch(2)`; Terminal left `ch(2)` versus Browser left `ch(5)`; unauthored right edge | verified fixed | Same test requires top and browser-left declarations once on the wrapper and in the fragment, rejects the terminal value, checks right zero, and requires one declaration for each margin edge | Nothing |
| R1-F1.3: Sizing and outer alignment | Width `50%`, maximum `40ch`, centered outer box | verified fixed | [Sizing assertions](../../lib/src/components/horizontal_rule/mod.rs#L2882) count exact wrapper declarations for width, maximum width, and both automatic horizontal margins; SVG variables cannot satisfy them | Nothing |
| R1-F1.4: Percentage top margin | Authored top `2%` | verified fixed | [Percentage-margin test](../../lib/src/components/horizontal_rule/mod.rs#L2731) requires `margin-top:2%` exactly once in the outer wrapper's declarations | Nothing |
| R1-F1.5: Routing and default layout | Nondefault layout routes through adapter; default layout has no wrapper | verified fixed | [Routing test](../../lib/src/components/horizontal_rule/mod.rs#L2771) compares rendered HTML with the adapter; [default-layout test](../../lib/src/components/horizontal_rule/mod.rs#L2893) requires SVG and rejects any div. Independent margin behavior is checked above | Nothing |
| R1-F1.6: Style, weight, and custom snapshots | Default outer layouts across seven styles and three weights, plus custom attributes | verified fixed | Public-fragment snapshot tests pass. These fixtures author no outer bottom margin, so their scope does not substitute for the bottom assertion | Nothing |
| R1-F1.7: Projection and shared lowering | Complete layout forwarded through adapter; all four edges lowered for Browser | verified fixed | [Projection](../../lib/src/components/horizontal_rule/mod.rs#L269) forwards `self.layout`; [adapter](../../lib/src/render_tree/browser_adapter.rs#L111) renders that tree; [outer-box lowering](../../../renderable/src/tree/render/browser.rs#L3317) resolves all four margins. Public-fragment assertions confirm the wrapper | Nothing |

## Unblocked Findings

None.

## Blocked Findings

None. No repair waits on a human decision.

## Requirement coverage

| Requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| One browser path, rich SVG by default, original downcast identity | L1 routing and identity tests; implementation delegates to the adapter; deleted helpers absent | Satisfied |
| Left, Right, and Centered preserve width; Full forces `100%` | L1 independent public-fragment assertions for all four alignments | Satisfied |
| Outer margins, browser-specific values, sizing, and alignment applied once; no default wrapper | L1 exact wrapper declarations, fragment-wide margin counts, placement and default-wrapper assertions; negative mutation check | Satisfied |
| Unsafe width/color fall back; safe values survive | L1 public-fragment tests independently exercise quote/attribute and semicolon/declaration payloads, with positive width and color controls | Satisfied |
| CSS variables, seven styles, three weights, page composition | L1 variable assertions, style/weight snapshots, and declared page-composition integration tests | Satisfied |
| Terminal behavior and moved color helpers unchanged | L1 terminal snapshots, parity and helper tests, relocated fallback test; helper bodies moved without behavior changes | Satisfied |
| Serialization selected in ordinary L1; opt-in for consumers | Four serialization tests selected and passed; metadata and recipes enable `image,serde`; both build configurations compile | Satisfied |
| Current documentation and comments describe delegation | Component documentation, source comments, snapshots, and relevant skill guidance checked against implementation | Satisfied |

L1 is appropriate: the spec explicitly requests HTML-string contract checks and changes no terminal drawing, browser interaction, or keyboard handling. Existing higher-tier harness tests were unchanged and were not launched. No new L2/L3 or browser evidence is required here. The input robustness matrix does not apply to this renderer consolidation.

The unsafe-color test uses default Full alignment, an accepted choice from review 1: shared color validation runs independently of alignment, so forced width cannot conceal a color validation failure. No accepted decision is reopened.

## Verification performed

- After restoring the mutation, `just test`: 3,931 passed, 55 excluded by tier selection, including both HTML-page integration tests. All four serialization tests and the relocated fallback test were selected and passed.
- `just test horizontal_rule`: 147 passed, including all four serialization tests, the relocated fallback test, fragment tests, and terminal/browser snapshots.
- Negative mutation: temporarily replaced the projected bottom margin with zero and ran `just test test_fragment_outer_layout_wraps_rule`. The test failed specifically because `margin-bottom:1lh` was absent from the wrapper. Restored source bytes in a `finally` block, preserving the existing repair.
- `cargo check -p biscuit-terminal --tests --all-features`: passed.
- `cargo check -p biscuit-terminal --no-default-features`: passed.
- `just check-tier-coverage biscuit-terminal`: zero stranded tests. Modified tests remain library L1 tests; page-composition tests are declared by the L1 integration target.
- Current library, CLI, and component docs contain neither `render_browser_svg` nor `MarginToCss`.

The earlier review and repair log record passing lint and doctests. This review did not rerun formatting or unchanged doctests. Cross-OS proof remains with CI and is not a readiness condition. No production source or snapshot change was retained by this review.

## Recurrence

None. R1-F1 is closed; no unchanged instance, repair regression, or missed sibling was found.
