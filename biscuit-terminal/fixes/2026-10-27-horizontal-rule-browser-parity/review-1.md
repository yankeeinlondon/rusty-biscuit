---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Outer bottom margin has no independent fragment assertion"
human_review: true
human_review_items:
  - "For awareness: retain the earlier migration of browser tests and snapshots so deleting the old method leaves each phase compiling."
  - "For awareness: retain the 22 changed snapshots; the 23rd snapshot has unchanged output and needs no header-only refresh."
  - "For awareness: retain the ten test renames to match the public fragment API; their assertions and snapshot names are unchanged."
  - "For awareness: retain the appended implementation log rather than recreating an existing file."
  - "For awareness: retain the spec's implementation metadata, which records completion without changing its requirements."
  - "For awareness: no blast_radius frontmatter was added because the documentation had none to preserve."
  - "For awareness: retain the existing skill guidance, which describes the unchanged terminal image path accurately."
  - "For awareness: obsolete-name searches were scoped to current code and guidance; historical specs and reviews were left unchanged."
  - "For awareness: retain the narrowed full-width assertion, which checks the width attribute without rejecting valid SVG coordinates."
  - "For awareness: retain the scoped verification choices; unchanged terminal/browser harness suites and cross-OS runs were omitted as the spec permits, with cross-OS evidence left to CI."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T12:23:39-07:00
spec: 2026-10-27-horizontal-rule-browser-parity/spec.md
implemented: true
description: "A **fix** review of `2026-10-27-horizontal-rule-browser-parity/spec.md`"
fix: 2026-10-27-horizontal-rule-browser-parity/review-1.md
next: 2026-10-27-horizontal-rule-browser-parity/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/biscuit-terminal/fixes/2026-10-27-horizontal-rule-browser-parity/log.md
implemented_by: claude/opus
---

# Review 1: Horizontal Rule Browser Parity

The implementation follows the intended single browser rendering path. One explicit L1 verification requirement remains unprotected, so the fix is not ready yet. No production rendering defect was found, and no repair needs a human decision.

Reviewed the spec, plan, implementation log, and changes from `d7ea96a1f` through `ec1c19a51`. Two agents independently extracted the implementation decisions before the code review. This is the first review; there are no previous findings to carry forward.

## Findings

### R1-F1 (high): Outer bottom margin has no independent fragment assertion

**Authority — spec, Fix item 4:** “check outer top/bottom and side margins, width, maximum width, and alignment through the existing tree layout contract.” In biscuit-terminal, [the outer-layout fragment test](../../lib/src/components/horizontal_rule/mod.rs#L2810) sets a bottom margin of one character cell but never checks its rendered value. This leaves an expressly requested behavior untested even though the current code produces the correct output.

The defect class is incomplete independent assertions for the outer layout carried through the public browser fragment.

Reproduction: start with the existing test's realistic layout: top `ch(2)`, bottom `ch(1)`, and browser-specific left `ch(5)`. Temporarily change biscuit-terminal's tree projection to clone the layout, replace its bottom margin with zero, and pass that altered layout to `set_layout`. Run `just test test_fragment_outer_layout_wraps_rule` in the package area. It still passes (1/1), although the authored bottom margin has been discarded. The mutation was removed after this check; implementation source is unchanged by the review.

The bounded sweep covered each placement/sizing assertion in the changed outer-layout test, the other changed tests that supply layouts, and the shared rendering decisions reached by them:

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Outer-layout fragment test: bottom | Bottom `ch(1)`; projection deliberately substitutes zero | Test passes without `margin-bottom:1lh` | Test must fail when the authored bottom margin is lost |
| R1-F1.2 | Same test: top and side margins | Top `ch(2)`; left Terminal `ch(2)` / Browser `ch(5)` | Clean: explicit assertions require `margin-top:2lh`, exactly one `margin-left:5ch`, and no terminal left value | Browser values on the outer wrapper |
| R1-F1.3 | Same test: width, maximum width, outer alignment | Width `50%`, maximum `40ch`, centered outer box | Clean: explicit assertions require sizing and both automatic horizontal margins on one wrapper | One sized, centered wrapper |
| R1-F1.4 | Migrated percent-margin test | Top `2%` | Clean: independently asserts `margin-top:2%` | Authored percentage reaches browser output |
| R1-F1.5 | Routing test and default-layout test | Nondefault top margin; default layout | Clean for their contracts: routing equality and no default wrapper; neither claims independent bottom coverage | Routing preserved and default output bare |
| R1-F1.6 | Migrated style/weight and custom-attribute snapshots | Default outer layouts | Clean: these fixtures have no authored outer bottom margin to preserve | Style/weight geometry retained; no substitute for R1-F1.1 |
| R1-F1.7 | Tree projection, browser adapter, shared thematic-break renderer | Layout forwarded to the tree; SVG intrinsic vertical margins fixed at zero | Code inspection: projection forwards the complete layout, adapter renders that tree, and shared outer-box lowering processes all four edges in one loop | Bottom `ch(1)` becomes `1lh` on the wrapper, once |

**Recommended repair:** extend the existing test to assert that `margin-bottom:1lh` occurs exactly once on the outer wrapper. Keep the existing SVG-inside-wrapper and intrinsic-margin checks. Confirm that dropping the bottom margin now fails the test. No new test binary, browser suite, or production abstraction is needed. This finding is high because an explicit spec verification requirement is unprotected, not because a runtime rendering failure was observed.

## Requirement coverage and verification

| Requirement | Strongest relevant verification | Review result |
| --- | --- | --- |
| Single browser path; default rich SVG; original downcast identity | L1 routing, fragment, and `as_any` tests; source inspection | Satisfied |
| Left, Right, Centered authored widths; Full forced width | L1 independent HTML assertions | Satisfied |
| Outer margins, target-specific layout, sizing, alignment, wrapper count | L1 public-fragment assertions and shared lowering inspection | Bottom assertion gap reported as R1-F1 |
| Unsafe width/color fallback, positive values, CSS variables | L1 public-fragment tests with quote and declaration payloads | Satisfied; Full does not suppress color validation, so its use in the color test does not weaken that check |
| Seven styles, three weights, page composition | L1 snapshots and HTML-page integration tests | Satisfied |
| Terminal behavior and moved color helpers | L1 terminal tests and unchanged terminal snapshots; helpers moved verbatim | Satisfied |
| Serialization selected in ordinary L1; opt-in production feature | L1 results, Cargo metadata, just recipes, both build configurations | Satisfied |
| Current docs and comments describe delegated browser behavior | Diff and source comparison | Required browser behavior documented; accurate terminal-only skill guidance retained |

L1 is the appropriate level here: the spec explicitly requires HTML-string contract checks and leaves terminal drawing unchanged. No keyboard, focus, or new emulator behavior needs L2/L3 evidence. The input robustness matrix does not apply to this renderer change.

Reviewer checks on the unmodified implementation:

- `just test`: **3,931 passed**, 55 excluded by tier selection. All four serialization tests and the relocated terminal fallback test appeared and passed.
- `just test horizontal_rule`: **147 passed**, including the eight new fragment tests and terminal/browser snapshots.
- `just test html_page_example`: **2 passed** through the declared L1 integration target.
- After removing the temporary mutation, `just test test_fragment_outer_layout_wraps_rule`: **1 passed**; no implementation-source diff remains.
- `cargo check -p biscuit-terminal --tests --all-features`: passed.
- `cargo check -p biscuit-terminal --no-default-features`: passed.
- `just doctest`: **204 passed**, 1 ignored; CLI has no library doctests.
- `just check-tier-coverage biscuit-terminal`: **0 stranded tests**.
- Implementation log records passing `just lint`; this review did not rerun lint or formatting.

All 22 browser snapshot changes match the intended alignment change and updated source headers. No terminal snapshot changed. Searches of current library, CLI, and component docs found neither deleted helper nor obsolete browser method names. Higher-tier harness tests were unchanged and were not launched. Cross-OS proof remains a CI concern, not a readiness finding.

## Implementation decisions (informational)

Retain the following choices. None changes the agreed scope, fixes a new public contract, introduces a safety default, or requires a human before repair. The frontmatter lists them for optional author awareness after the review/fix cycle; no response is required to continue.

- **Move test migration earlier:** deleting the legacy method required migrating its callers in the same compiling phase. This changes sequencing only.
- **Regenerate 22 snapshots:** the Centered 75% custom snapshot already matches the shared output. Retaining its historical source header avoids an unnecessary content-free update.
- **Rename ten tests:** the new names describe the fragment API and satisfy the obsolete-name search without changing coverage or snapshot identities.
- **Append to the existing log:** earlier phases had already created it. No separate replacement was needed.
- **Update implementation metadata:** marking the spec implemented follows the recorded final-phase pipeline instruction despite the plan's request not to modify the spec. Requirements remained unchanged.
- **Do not invent missing frontmatter:** the plan asked to preserve `blast_radius`, but the documentation had no such property.
- **Retain accurate skill guidance:** both skill copies mention HorizontalRule only in unchanged terminal image guidance. The spec's conditional documentation update does not require editing accurate text.
- **Scope the drift search to current material:** the proposed exclusion globs did not exclude nested historical documents. Leaving past specs/reviews intact is correct.
- **Narrow the full-width assertion:** checking the SVG width attribute avoids mistaking valid `50%` geometry coordinates for an authored half-width rule.
- **Use scoped verification:** no implementation requirement was deferred. Unchanged terminal/browser harness tests and cross-OS checks were omitted; the spec allows L1 here. Dependent tests were omitted for test-only phases, with relevant darkmatter and tree-hugger checks recorded elsewhere. Test-toolkit had no applicable L1 tier and was selected only for a lint companion. The terminal/browser Full-width difference remains explicitly outside scope.

The four entries explicitly flagged in `decisions_for_review`—phase sequencing, unchanged snapshot, test renames, and log/spec bookkeeping—are all covered above. None requires a human ruling. R1-F1 is unblocked and can be repaired directly.
