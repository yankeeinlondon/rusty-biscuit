---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-27-union-partial-file-completion/spec.md`
fix: 2026-09-27-union-partial-file-completion/review-3.md
spec: 2026-09-27-union-partial-file-completion/spec.md
previous: 2026-09-27-union-partial-file-completion/review-2.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T18:02:42-07:00
implemented: false
ready: false
findings:
    - "[high] Root-union file patterns are enforced only when the arms disagree"
human_review: true
human_review_items:
    - |-
        Review the recurring file-pattern finding before another review/fix cycle. The specification says an existing file outside an arm's `match(...)` pattern rules that arm out. The current code and documentation apply that rule only when two simplified arms declare different patterns. Please confirm that the specification's broader rule remains intended, and check that the repair covers the identical-pattern, single-declaration, and mixed-schema cases listed below. No terminal interaction or OS-specific judgment is needed.
has_blocked_findings: false
blocked: false
recurrence: true
---

# Review 3: Union Partial File Completion

## Assessment

**Not production ready.** Review 2's one unblocked finding is fixed for the two-arm, different-pattern example: a picked or explicitly supplied file selects the matching arm, and a settled arm rejects a file in the other tree. Review 2 had no blocked findings. The same defect class remains in other root-union shapes, so the review loop should stop for a human check of the full sweep.

## Verification and requirement coverage

| User-visible requirement | Strongest evidence | Assessment |
| --- | --- | --- |
| Partial-file chooser, confirmation, cancellation, and prompt order | Level 1 PTY tests in `claudine-cli` | Appropriate for program decisions. The new chooser tests also prove which arm coerced a sibling value. |
| Caller-relative file resolution and late error wording | Level 1 shipped-CLI tests; fresh shipped-CLI controls in this review | Passing for the tested single, settled, and undecided shapes. |
| Inline provider picker and retained terminal history | Level 2 tmux and WezTerm captures reported in review 2; the current change keeps those tests declared | Appropriate for actual terminal rendering. No keyboard-encoder claim requires Level 3. |
| Focused frontmatter excerpts | Level 1 excerpt tests and Level 2 captures reported in review 2 | No new change to the excerpt path. |
| A selected file outside a root-union arm's pattern rules that arm out | Level 1 CLI and library tests for different patterns; fresh public-CLI matrix below | Incomplete across the other union shapes. |

`just test-cli contested_file_match` passed its one selected Claudine CLI test. `just test contested_file_match` passed its one selected Darkmatter CLI test. `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter` each reported zero stranded tests. I did not rerun the full suites or the Level 2 captures. Cross-OS evidence is outside this readiness decision. The input robustness matrix does not apply: this change converts and validates schema declarations, but does not add a file-format or configuration reader.

For the public-CLI sweep, I copied one real fixture with existing `features/x/spec.md`, `fixes/x/spec.md`, and `other/x/spec.md`. Each row changed only the root schema shape or the `spec` value. I ran `md schema validate --no-trigger-schemas --format json` against every row. For the different-pattern and identical-pattern rows, I also ran `claudine compose --goose` with a stub provider and checked whether it launched.

## Unblocked Findings

### 1. Root-union file patterns are enforced only when the arms disagree (high)

**Defect class:** Root-union validation drops a declared `file(match(...))` constraint when the same property has identical patterns in other arms, occurs in only one arm, or is combined with a referenced JSON Schema arm.

The specification says that a selected path failing an arm's `match` rules that arm out. [`contested_match_patterns`](../../../darkmatter/lib/src/markdown/schemas/file_match.rs) returns no pattern for identical declarations or a sole declaration. [`attach_contested_match`](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs) therefore emits no validation keyword for those arms. The schema resolver also skips attachment when a root union mixes simplified and raw JSON Schema arms. Claudine's early arm selector uses the same narrowed helper in [`supplied_file_arms`](../../lib/src/composition/schema/supplied.rs), so early and final decisions share the gap. The current Claudine and Darkmatter topic pages describe this narrowed behavior; that departure from the specification needs correction alongside the code.

| Site and shape tested | Observed result | Expected result |
| --- | --- | --- |
| Darkmatter, single schema with a `fixes` pattern; existing `features` file | Accepted. | Accepted: the existing single-schema rule makes `match` a suggestion; clean comparator. |
| Darkmatter and Claudine, two inline arms with different `features`/`fixes` patterns; settled `fix` arm and `features` file | Rejected; Claudine did not launch the provider. | Rejected; clean implementation of the review 2 example. |
| Darkmatter, two inline arms with different patterns on `file[]`; settled `fix` arm and a `features` element | Rejected. | Rejected; clean array comparator. |
| Darkmatter and Claudine, two inline arms with the **same** `fixes` pattern; settled `fix` arm and an existing `features` file | Both accepted; Claudine launched the provider. | Reject the file outside the selected arm's declared pattern. |
| Darkmatter, only the settled `feature` arm declares `spec: file(match(**/fixes/**/spec.md))`; existing `features` file | Accepted. | Reject the file outside the selected arm's declared pattern. |
| Darkmatter, settled simplified `fix` arm with a `fixes` pattern plus a referenced raw JSON Schema arm; existing `features` file | Accepted. | Reject the file outside the selected simplified arm's declared pattern. |

The matching `fixes` file passed in every applicable variant, so the failures are not caused by an unusable fixture. The new public tests cover the different-pattern shape but omit the three failing sibling shapes. Enforce the arm's declared pattern after arm selection for every root-union shape, including when the union cannot retain an all-simplified projection. Keep the documented single-schema suggestion rule. Add public-result tests for every row above, with an arm-specific outcome or provider-launch assertion where selection matters.

## Blocked Findings

None. The specification already states the expected behavior; the human review item is a check on this recurring class, not an implementation prerequisite.

## Recurrence

This is the same defect class as **review 2, finding 1, “Union arm selection ignores file match patterns.”** That fix swept the two-arm, different-pattern case and an array variant, but it should also have swept identical patterns, a pattern declared by only one arm, and a simplified arm beside a referenced JSON Schema arm. The table above includes the clean single-schema and different-pattern controls as well as every failing sibling found in this sweep. Review 1's caller-origin and terminal-verification findings are distinct classes and did not recur.
