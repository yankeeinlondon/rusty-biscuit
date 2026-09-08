---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T18:27:10-07:00
spec: 2026-08-31-silent-success-and-startup-stall/spec.md
implemented: false
description: A **fix** review of `2026-08-31-silent-success-and-startup-stall/spec.md`
fix: 2026-08-31-silent-success-and-startup-stall/review-3.md
previous: 2026-08-31-silent-success-and-startup-stall/review-2.md
---

# Review 3: Silent Success and Startup Stall

## Verdict

**Not production ready.** The startup watchdog and failure classification now satisfy the core incident contracts, and the prior missing Level 2 diagnostic coverage has landed and passed. Three remaining correctness issues affect the promised diagnostic and machine-data behavior. None of these findings claims that the original startup hang or stopped-task success classification remains unfixed.

This review assesses the current working tree, including the uncommitted implementation of review 2, against the current specification. Its clarified notification vocabulary, lifecycle catalog identity, and detection-versus-reap deadline are the review baseline.

## Findings

### 1. Medium: finalization drops a provider error message when no error kind exists

At `claudine/lib/src/stream/task_ledger.rs:337`, `apply_to_summary` constructs the displaced-error clause only through `summary.error_kind.map(...)`. An existing `error_message` without an `error_kind` is overwritten without preservation. This is a reachable Claude parser state: `handle_result` records the text of `result.is_error=true` without requiring or assigning an error kind (`claudine/lib/src/stream/providers/claude.rs:393`).

A standalone Level 1 probe linked against the built library fed a stopped/unknown task notification followed by:

```json
{"type":"result","is_error":true,"result":"Provider rejected the request"}
```

The final message contained only `1 sub-agent task did not complete: alpha (Evaporated)`. The provider failure text disappeared. The session still fails, but its concise diagnostic loses the actionable cause, contrary to required behavior §4's preservation contract.

Preserve the prior clause when either a meaningful kind or message exists, retaining the existing reserved character budget. Extend the Level 1 parser and ledger coverage with message-only failures; the current displacement tests always supply a kind. This is a missing input case at the appropriate verification level, not a need for keyboard or terminal-emulator testing.

### 2. Medium: `raw_status` is normalized before storage despite the verbatim contract

`TaskLedger::record_terminal` trims the stored status at `claudine/lib/src/stream/task_ledger.rs:286`. Normalizing the comparison is correct, but modifying the raw field violates required behavior §2 and the `SubagentOutcome.raw_status` field documentation. The emitted `SubagentStop.status` preserves the original string, so live semantic data and final machine data can disagree.

The same parser probe supplied `"status":"  Evaporated  "`. The final fact was `raw_status=Some("Evaporated")`, rather than the authored string. Existing parser and serialization tests use already-trimmed strings and cannot expose the loss.

Store the original status independently of the normalized classification value. Add Level 1 assertions for padded unknown and unsuccessful statuses through parser finalization and summary serialization. Keep trimming in display formatting if desired. Review the affected field docs and implementation together so the verbatim claim becomes true.

### 3. Medium: the full diagnostic incorrectly claims normal exit after a watchdog kill

`IncompleteSubagents::render` always says the provider “exited normally” and that failure occurred “despite its exit code” (`claudine/lib/src/render/incomplete_subagents.rs:47`). Its caller renders this component for every nonempty outcome list, without checking termination or exit code (`claudine/cli/src/commands/wrap/policy.rs:379`). A task started before a timeout remains unfinished in the ledger; timeout projection retains that list.

A Level 1 CLI reproduction used a fixture Claude executable that emitted initialization and `task_started`, then remained silent. With a 2-second silence budget, the resulting diagnostic still claimed normal exit, while its synthetic session row recorded `exit_reason: "step_timeout"`, `exit_code: 1`, and an unfinished task. This makes the report contradict the termination that the watchdog just performed. The same unconditional sentence also reaches nonzero native exits.

Use termination-neutral wording for the shared task list, or provide the component enough outcome context to select accurate wording. Add a spawned-process regression for a started task followed by timeout, plus a native nonzero-exit case. The existing Level 2 capture proves wrapping and styling for the exit-zero incident; it does not test these termination combinations. Retain that capture when changing the wording.

## Requirement verification

| User-facing requirement | Strongest relevant evidence | Assessment |
|---|---|---|
| Silence is bounded from spawn, without a wall-clock timeout | Level 1 watchdog tests and `wrap_watchdog_startup_stall` process tests | Passed; reap duration and outer elapsed time are asserted with documented scheduling allowances. |
| Non-whitespace bytes refresh the budget; whitespace does not | Level 1 metric and spawned-process tests | Passed; both directions are exercised. |
| Startup warning uses the same clock, fires once, and resets after activity | Level 1 progress and warning-render tests | Appropriate for timing, state, and wording; passed. No new keyboard or styling contract applies. |
| OpenCode cold start is bounded; either fresh clock suppresses a live step; wall-clock timeout has precedence | Level 1 watchdog and process tests | Passed, including both stale/absent clock permutations added after review 2. |
| Capture and interactive modes remain outside the silence rule | Source inspection of the separate wall-clock ticker paths; existing Level 1 timeout coverage | No regression found. |
| Terminal notifications fail closed and preserve identity/status | Level 1 parser and ledger tests | Vocabulary routing passes; verbatim raw-status preservation remains defective (finding 2). |
| Ledger retains more than five tasks, anonymous facts, duplicate-ID reconciliation, restart state, and unfinished tasks | Level 1 ledger/parser tests; representative process replay | Passed for the covered states. |
| Native exit zero with incomplete tasks fires failure, suppresses success, and exposes `provider.incomplete_subagents` | Level 1 `wrap_incomplete_subagents` lifecycle replay and diagnostic registry tests | Passed; the prior identity mismatch is resolved. |
| Semantic errors outside the task ledger also classify as `AgentFailure` | Level 1 harness runtime tests | Passed; clean success and interrupt/timeout precedence have companion cases. |
| Concise diagnostic retains the displaced failure and stays within 240 characters | Level 1 ledger/harness tests and review probe | Budget tests pass; message-only prior failures are lost (finding 1). |
| Full diagnostic renders every incomplete task with correct wrapping and styling | Level 2 detached tmux capture at 70 columns, seven long task names, plus Level 1 content tests | Feature-specific capture passed. Incorrect termination wording remains (finding 3). The 200-row fixture verifies horizontal wrapping, not scrollback overflow; the spec introduces no separate scrolling UX contract. |
| Summary compatibility and machine facts survive reporting | Level 1 legacy/empty/populated JSON tests, synthetic-row tests, and SQLite ingestion/query round trip | Passed for supplied values; ingestion preserves extras rather than repairing finding 2. |
| Documentation and skill copies reflect the corrected contracts | Source review of topic/skill changes and implementation log | Review 2's omitted summary is now updated through its durable research source, including the regeneration prompt. |
| Cross-platform operation and focus preservation | Portable L1 fixtures with Unix/Windows scripts; local macOS execution; detached tmux L2 | Windows and Linux execution were not independently repeated in this review. No Level 3 input-encoder requirement applies. |

No remaining requirement was found whose existing test needs promotion from Level 1 to Level 2 or Level 3. The findings concern incorrect values and missing cases, rather than a substitute of a lower verification level for a required higher one.

## Verification performed

- `just test`: passed, 6,844 tests; 11 skipped. Skips are not counted as verified behavior.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 incomplete_subagent`: passed, including the feature-specific tmux capture (1 selected CLI test). The generator invocation selected no matching tests; it supplies no additional feature evidence.
- `just lint`: passed, including transport guards, lifecycle documentation guards, and package lint checks.
- `git diff --check`: passed after review metadata edits; frontmatter was parsed with Biscuit File to verify the requested values and boolean types.
- Standalone parser probe: reproduced findings 1 and 2 without modifying repository Rust sources.
- Fixture-provider CLI probe: reproduced finding 3 without launching a real provider or requesting desktop focus.
- The implementation log records non-vacuity mutations from the implementation cycles. This review did not repeat those source mutations.

## Review metadata

The requested `@prompts/_reviews/claudine/fixes/2026-08-31-silent-success-and-startup-stall/review-2.md` reference does not resolve. The existing previous review is beside this specification at `claudine/fixes/2026-08-31-silent-success-and-startup-stall/review-2.md`; its `implemented` and `next` properties were updated there. The spec's `review_iterations` is now `3`. Implementation files were left unchanged.
