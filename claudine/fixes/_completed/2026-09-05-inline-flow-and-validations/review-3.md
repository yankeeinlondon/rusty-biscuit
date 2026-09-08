---
$schema: feature-review.yaml
ready: true
agent: codex/gpt-5.6-sol
created: 2026-09-07T15:54:11-07:00
spec: 2026-09-05-inline-flow-and-validations/spec.md
implemented: false
description: A **fix** review of `2026-09-05-inline-flow-and-validations/spec.md`
fix: 2026-09-05-inline-flow-and-validations/review-3.md
previous: 2026-09-05-inline-flow-and-validations/review-2.md
---

# Review 3: Inline Flow and Completion Validation

## Verdict

**Ready for production.**

No blocking or non-blocking implementation findings remain. The high-severity
review-2 blocker is closed: the interrupted inline-run subprocess test now
asserts the operator-facing restore notice, summary label, and agent summary,
while the implementation uses prospective wording because rollback occurs
after that report. The other review-2 findings are also resolved: the scratch
test was replaced by an asserted regression case, ambient `MODEL` is scrubbed
from the relevant fixture paths, the nested-property collision is pinned, and
the inline prompt's completion column treats `eager` and `required` as
independent axes.

The implementation matches the specification's central architecture. Both
composition modes reach `complete_active_document` through the shared
production orchestration path; inline mode reconciles and writes the artifact
before passively validating the effective instance, while direct compose
performs no source-file write. Schema problems, body-change rejection,
rollback, recovery, provider-summary accumulation, and write-grant posture all
retain typed and independently tested contracts.

## Findings

None.

## Review-2 resolution verification

| Review-2 finding | Resolution | Verification |
|---|---|---|
| High — interrupt messaging had no test | **Resolved** | `a_provider_exit_130_restores_the_captured_baseline` asserts Ctrl+C attribution, the prospective restore notice, `Agent summary:`, and the summary sentinel; the document remains byte-identical. |
| Low — assertion-free scratch test | **Resolved** | The scratch dump is gone. `provided_file_array_multi_element_partial_reports_first_element` asserts the typed first-element result for the multi-element case. |
| Low — ambient `MODEL` broke AC9 L2 equivalence | **Resolved** | Fixture subprocesses remove `MODEL`; staged L2 commands default it to empty; target-launch unit helpers hold an environment guard. The formerly failing L2 test passes with this review session's ambient `MODEL`. |
| Low — no nested-name collision regression | **Resolved** | `nested_conversion_error_does_not_anchor_on_a_same_named_top_level_property` confines all diagnostics to the invalid nested `meta.prompt`, leaving the valid top-level `prompt` clean. |
| Note — independent-axis follow-through was uncommitted | **Resolved** | The prompt header marks only `required` properties as required at completion, with eager-only, required-only, combined, union, and both array-placement cases pinned. |

## Test-rigor classification

| User-facing requirement | Strongest evidence | Level | Assessment |
|---|---|---|---|
| AC1/AC2 — universal `eager`, independent diagnostics, ranges, and hover isolation | Darkmatter conversion and in-process DMLS/LSP sessions, including the original voip schema and the nested-name collision | Level 1 | Appropriate: these are schema and editor-protocol semantics, not terminal-emulator behavior. |
| AC3/AC9a/AC13 — launch collection and inline deferral | Unit matrix plus PTY prompt tests | Level 1 / Level 2 PTY | Appropriate: the collection interaction is exercised through a PTY; no OS keyboard encoder behavior is claimed. |
| AC4 — completion failure and complete per-property status report | Lifecycle subprocess tests plus `level2_completion_schema_failure_renders_the_full_plain_status_report` in tmux | Level 2 | Appropriate: real-terminal capture verifies the visible status block, including satisfied and missing rows. |
| AC5 — native path, file-aware guardrails, summary routing | Provider-stub subprocess test with positive stdout and negative document assertions | Level 1 | Appropriate: this is file/stdout routing with manufactured provider output. |
| AC6/AC7/AC8/AC17 — owned-node warnings, unchanged body, interrupts, and rollback | Provider-stub subprocess tests with byte-for-byte file assertions; interrupt stderr is now asserted | Level 1 | Appropriate: no terminal rendering or keyboard encoding is required to verify rollback and messages. |
| AC9/AC10/AC18 — direct completion, shared path, sequence/loop verdicts, and recovery | Pure evaluator tests, subprocess lifecycle tests, and tmux lifecycle-equivalence coverage | Level 1 / Level 2 | Appropriate: orchestration semantics are covered in-process/subprocess; terminal-visible equivalence is captured in a real terminal. |
| AC11 — final-response accumulator across adapters | Per-adapter replay fixtures, including the saved OpenCode voip transcript | Level 1 | Appropriate: this is a deterministic parser/event contract. |
| Completion error presentation for both typed codes | `level2_completion_schema_failure_renders_the_full_plain_status_report` and `level2_unchanged_inline_body_renders_the_plain_typed_error` | Level 2 | Appropriate: both errors are captured after rendering through real tmux. |
| AC19 — minimum provider write capability | Cross-provider/all-OS launch-plan matrix plus opt-in real Claude/Codex write smoke | Level 1 / real-provider | Appropriate: deterministic argv/environment construction covers portability; live smoke covers actual provider acceptance where enabled. |

No requirement in this fix depends on what bytes a terminal emits for a real
keyboard event, so Level 3 testing is neither required nor appropriate.

## Verification performed

| Command | Result |
|---|---|
| `just test` in `claudine/` | 6,810 passed, 11 skipped, 0 failed. |
| `just test` in `darkmatter/` | 7,709 passed, 51 skipped, 0 failed. |
| `just lint` in `claudine/` | Passed for `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`; the diagnostic guard tests also passed 18/18. |
| `just lint` in `darkmatter/` | Passed for `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, and the `wasm32-wasip2` Zed extension check. |
| Focused `just _test_l2` for the two completion error captures | 2 passed in real tmux, 0 failed. |
| Focused `just _test_l2` for `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run` | 1 passed in real tmux with the review session's ambient `MODEL`, 0 failed. |

The macOS linker emitted its known compact-unwind table-size warning while
linking the large `claudine` test binary. It did not affect compilation,
clippy, test execution, or this fix's behavior.

## Recommendation

Close the fix cycle as production-ready. The review found no remaining
functional, ergonomic, performance, or test-tier gap within the specification.
