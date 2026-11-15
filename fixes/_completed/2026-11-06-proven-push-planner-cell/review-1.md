---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (medium): Malformed producer records escape the shared validator as exceptions'
human_review: true
human_review_items:
  - Keep exact workflow-condition assertions rather than weaker prefix checks; no action is needed unless the author prefers looser tests.
  - Keep the additional sweep of contract-named uploads, excluding diagnostics that require a later step to fail; no action is needed unless the author prefers only the seven fixed cases.
  - Keep the CLI test's real package mapping and corrupt only its check cell so it reaches the intended validator; no action is needed.
  - Keep the explanations above the four status-upload guards; the author may remove them later without changing behavior.
  - Keep dependency facts in proven plans and correct their schema comment; this preserves the specified reporting behavior.
  - Keep the implementation-run identity claude/opus despite the plan naming claude/sonnet; this is metadata only.
  - Accept workflow contract assertions as verification of uploads after later failures; hosted execution was deliberately avoided as the spec requests.
  - Accept direct Clippy as the lint substitute and the temporary-fixture signing override as test setup; neither changes production policy.
  - Leave the index staged by another process untouched; no author action is required for this review.
  - actionlint remains unperformed because it is unavailable; run it when available, without adding a hosted CI run solely for this review.
  - Additional OS runs were deferred to CI; the unrelated coverage-audit investigation and Rust port remain outside this fix.
  - Repair also guarded 15 sibling identity lookups beyond the two named; the sweep found free-form fields (runner, build settings, flags) that accept lists or objects without raising, left as is since they are outside this class.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-08T17:54:44-08:00
spec: 2026-11-06-proven-push-planner-cell/spec.md
implemented: true
description: 'A **fix** review of `2026-11-06-proven-push-planner-cell/spec.md`'
fix: 2026-11-06-proven-push-planner-cell/review-1.md
log: /Volumes/coding/rusty-biscuit/fixes/2026-11-06-proven-push-planner-cell/log.md
implemented_by: claude/opus
next: 2026-11-06-proven-push-planner-cell/review-2.md
---

# Review 1

The scheduling repair meets the intended event rules, but the shared validator has one malformed-input finding. No finding requires an author ruling before repair.

## Findings

### R1-F1 (medium): Malformed producer records escape the shared validator as exceptions

**Authority — regression and defect:** In the repo-deps package, [the new producer lookup](../../scripts/ci/schema.py:1904) rereads `entry["for"]` even after the preceding validation has identified that required key as missing. A developer inspecting or editing a carried nightly plan can remove its producer's `for` field: the base version returns `malformed-receipt`, while the implementation raises `KeyError: 'for'`. This loses the validator's documented coded-error result and the evidence overlay's contextual refusal. The adjacent producer-name validation also crashes for a list or object instead of reporting its invalid type; that sibling predates this fix.

**Defect class:** producer metadata is used for indexing or set membership before its required keys and scalar types have been established.

The reproduction starts with the new `DependentCheckSchedulingTests` nightly fixture, confirms it validates, and changes one field per case. Calls go through `schema.validate_resolved_plan`, `apply_accepted_cells`, and the shipped CLI. The following table includes the clean sibling paths and downstream decisions.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Producer lookup, `schema.py:1904` | Producer `for` absent | `KeyError`; base returns `malformed-receipt` | Return the existing missing-key problem; do not reread an invalid entry |
| R1-F1.2 | Producer membership check, `schema.py:1887` | Producer `name` is `[]` or `{}` | `TypeError` after recording an invalid-name problem | Return an invalid-name problem without using the value as a set key |
| R1-F1.3 | Producer field validation | `name` absent, null, number, or empty string; `for` null, number, object, empty string, empty list, mixed string/number list, or all-number list | Coded problems returned | Same; clean controls |
| R1-F1.4 | Optional producer collection validation | Collection absent, null, number, object, empty list, or non-record elements | Absence allowed; invalid present shapes refused | Same; clean controls |
| R1-F1.5 | Cell/environment relationship check | Valid nightly producer; cells on absent or producer-only environments | Valid producer builds accepted; invalid cells refused with package/environment/gate | Same; clean control and required relationship cases |
| R1-F1.6 | Evidence overlay, `affected_scope.py:4863` | Missing producer `for`; list/object producer `name` | Raw `KeyError`/`TypeError` propagates | Contextual `RuntimeError` containing the validator's coded problem |
| R1-F1.7 | CLI carried-plan path, `affected_scope.py:6020` | Missing producer `for`; list producer `name` | Exit 1 with traceback; empty stdout; no output file | Exit nonzero with the coded validation reason; preserve no-publication behavior |

The unedited nightly document is accepted through the validator and overlay. The producer collection's absence is an explicit allowed default; every malformed present shape above should produce a validation problem. A valid producer name and guest list preserve the producer-only distinction. Scalar `name` has no element-shape cases; the guest-list element cases are included above.

**Recommended repair:** use only producer entries that passed required-key and name-type checks when building the lookup, and avoid set membership for a non-string producer name. Preserve the existing validation problems rather than adding another parsing layer or changing the document format. Add focused in-process regressions for both failing shapes, plus one CLI case proving the coded reason and no publication. This is a small repair; the defect does not schedule work or publish an invalid plan.

## Verification

All requirements here concern planning, structured validation, or workflow conditions. Level 1 is appropriate; there is no terminal rendering or keyboard behavior requiring Levels 2 or 3.

| Requirement | Strongest verification reviewed |
| --- | --- |
| Proven push has no cells, builds, rows, scheduled areas, or preflight runners; proof and dependency facts remain | Level 1 planner fixture and projection assertions |
| Pull request and unproven push retain one non-reusable Linux dependent check | Level 1 event fixture for both events |
| Example/benchmark and dependent checks combine | Level 1 example fixture; both kinds use the same existing uncovered-target branch |
| Nightly retains archive producers and Windows/WSL2 work without Linux cells | Level 1 nightly fixture, including build consumers |
| Accepted local L1 evidence cannot reuse the dependent check | Level 1 calculated-plan and carried-plan overlay cases |
| Absent/producer-only environments are refused, including reused cells | Level 1 validator and overlay cases; malformed producer handling is R1-F1 |
| Invalid calculated plan reaches neither plan file nor stdout | Level 1 CLI entry-point test with the exact validator diagnostic |
| Failed resolution skips unnamed uploads and completion; later failures retain named status uploads | Level 1 workflow contract assertions plus existing cell-contract refusal tests; conditions inspected across check, native test, lint, and WSL2 jobs |

Review reruns: `test_schema.py` passed 203 tests; `test_resolved_plan.py` passed 103 tests through `suite_runner.py`. `just test test-toolkit` passed 484 tests with 4 skips; the new upload-guard test took 0.027 seconds. Four unchanged tests exceeded five seconds in this concurrent run; none was introduced or modified by this fix. `suite_runner.py test_affected_scope.py` passed all 488 tests in 526.774 seconds, including all 10 new regressions.

The carried-plan JSON loader is unchanged and shared: `schema.loads_strict`. Direct checks refused duplicate keys, trailing garbage, a second JSON document, and an empty document. Existing CLI tests also cover duplicate keys and trailing content. No new format or loader was introduced. The changed relationship check guards cell identity types before using them; the producer-record shape failures above are the actionable gap. No permissive default was introduced for a missing producer guest list.

`actionlint` could not run (`command not found`), matching the implementation log's limitation. No hosted workflow or additional OS run was launched. Cross-OS evidence is outside this review's readiness decision.

## Implementation decisions (informational)

Two independent agents read the implementation log and specification in parallel. None of the collected decisions meets the test for stopping for a human.

- Updating three existing tests to pin the exact guarded workflow conditions preserves their original strength; recommended.
- Adding an upload sweep beyond the plan's seven named steps is a cheap extension of the spec's requested upload audit; recommended. Excluding WSL2 diagnostics is sound because it requires a later step's failure, and that step is skipped after failed resolution.
- Keeping the CLI fixture's actual package mapping and corrupting only the check cell ensures the failure reaches the new validator rather than an earlier build lookup; recommended.
- Adding comments above four status-upload guards explains why a failed job sometimes has no upload identity; acceptable and reversible.
- Correcting the `dependent_seam` comment preserves dependency facts on proven plans, as the spec permits. The detected comment drift was resolved without changing behavior.
- Recording `claude/opus` follows the implementation run's instruction rather than the plan's differing identity; this is metadata only.
- Verifying later-failure uploads by their condition and artifact name is appropriate for this spec, which explicitly rejects launching hosted CI to learn expression behavior.
- Running Clippy directly substitutes for a nonexistent lint recipe. Disabling signing only for temporary test repositories allowed fixture setup after an expired host signing key; it did not alter repository signing policy.
- Leaving staging performed by another process untouched respects the shared working tree.
- `actionlint` was deferred because the tool was unavailable; the plan explicitly allowed recording this limitation. Additional OS evidence was left to CI and is not a readiness finding.
- The unrelated October coverage-audit investigation and the Rust planner port were not performed because the spec excludes them. No implementation scope was moved to another spec.

This is the first implementation review; there are no previous findings to reconcile.
