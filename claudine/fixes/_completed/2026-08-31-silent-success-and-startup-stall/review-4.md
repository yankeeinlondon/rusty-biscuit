---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-07T20:12:46-07:00
spec: 2026-08-31-silent-success-and-startup-stall/spec.md
implemented: false
description: A **fix** review of `2026-08-31-silent-success-and-startup-stall/spec.md`
fix: 2026-08-31-silent-success-and-startup-stall/review-4.md
previous: 2026-08-31-silent-success-and-startup-stall/review-3.md
---

# Review 4: Silent Success and Startup Stall

## Verdict

**Production ready.** All three review-3 findings are resolved, the canonical L1 and lint checks pass, and the feature-specific real-terminal diagnostic test passes. No functional blocker or test-level mismatch remains. One low-severity documentation correction is recommended below; it does not affect runtime behavior.

This review assesses the current working tree, including the uncommitted review-3 corrections, against the specification's ratified notification vocabulary, lifecycle identity, and detection-versus-reap deadline. Implementation files were not changed during this review.

## Findings

### 1. Low, nonblocking: the watchdog function documentation overstates in-flight suppression

`claudine/cli/src/commands/wrap/exec/watchdog/evaluate.rs:36` says the silence rule requires that no tools or subagents are in flight. The implementation at lines 170–173 instead suppresses only when work is in flight **and none is stuck**. Stuck work can therefore time out, as intended by the adjacent implementation comments, the existing tests, and the specification's requirement to retain the existing in-flight gate.

Correct the function documentation to describe the stuck-aware condition. The code is the source of truth here; changing it to match the stale comment would reintroduce an indefinite-wait hazard. This is documentation drift, not a behavioral defect. No new test or higher verification level is needed for that prose correction. It is recorded here rather than changing implementation files in a review-only task.

## Previous findings resolved

1. **Message-only failures:** `TaskLedger::apply_to_summary` now calls `prior_failure_clause` with both optional fields. A nonblank message survives even without a kind, including a whitespace-only kind. Parser replay, ledger assertions, and the long-message reservation case cover the original failure and the 240-character bound.
2. **Verbatim raw statuses:** `record_terminal` stores the supplied string independently of classification. Parser tests compare live terminal-event statuses with finalized outcomes; ledger, summary serialization, and synthetic-row tests preserve padded unknown and stopped statuses. Whitespace-only inherently terminal statuses also remain present rather than becoming `None`.
3. **Accurate termination wording:** `IncompleteSubagents::with_exit` receives termination and exit code from both summary-emission callers. The component distinguishes completed-zero, completed-nonzero, timeout, abort, and interrupt; without context it uses neutral wording. Real-CLI regressions verify the started-task timeout and native exit-2 cases against stderr and session records. Existing L2 coverage retains the exit-zero diagnostic's rendering contract.

## Requirement verification

L1 below includes hermetic spawned-binary tests; those remain L1 even when their filenames or documentation call them integration tests. L2 means an actual terminal capture, not manufactured PTY input.

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Silence is bounded from spawn without wall-clock timeout | L1 `watchdog/tests/timeout_evaluation.rs`, `watchdog/tests/opencode.rs`, and `wrap_watchdog_startup_stall.rs` | Correct boundary: pure deadline predicates plus actual wrapper termination and synthetic-row duration assertions. Reap assertions include the stated tick, grace, and scheduling allowances; the outer timeout is only a hang backstop. |
| Non-whitespace startup bytes refresh silence; whitespace does not | L1 progress tests and chatty/whitespace fixture providers | Both directions covered through the byte-to-watchdog wiring. |
| Startup warning fires once, identifies startup, and resets after activity | L1 `stream/progress/tests.rs` and warning-render tests | Appropriate for the clock, one-shot state, and wording. No new color, layout, or input-encoder requirement. |
| OpenCode startup has no indefinite exemption; fresh activity suppresses a live step; wall-clock rule wins | L1 watchdog tests and real wrapper fixtures | Includes stale/absent clock permutations, both absent, fresh-clock countercases, and simultaneous-breach precedence. |
| Capture and interactive modes stay outside step silence | Source inspection of distinct execution paths and existing L1 timeout tests | No regression found; this fix does not introduce keyboard behavior. |
| Claude notification routing and identity/status preservation | L1 Claude parser and task-ledger tests | Absent/blank, progress, recognized terminal, and unknown terminal branches covered. Raw status preservation now has regression assertions. |
| Ledger completeness and reconciliation | L1 ledger/parser tests and `wrap_incomplete_subagents.rs` | More than five tasks, anonymous observations, duplicate IDs, same-ID completion, distinct-ID same-name tasks, restart, and started-without-terminal states covered. |
| Native zero plus incomplete work invokes failure and suppresses success | L1 incident replay through `compose --claude` | Checks lifecycle side effects, native code, `Completed` semantics, complete task facts, and the locked `provider.incomplete_subagents` lifecycle identity. |
| Non-task semantic failure also becomes `AgentFailure` | L1 `harness/runtime/tests.rs` | Includes semantic-error exit zero, clean success, and termination precedence. |
| Concise failure preserves displaced provider error within 240 characters | L1 ledger/parser/harness tests | Message-only regression now complements kind-bearing and long-message cases; full machine list is not truncated. |
| Full diagnostic enumerates all tasks, wraps correctly, and preserves styling | L2 `level2_incomplete_subagent_diagnostic_renders_in_tmux` | Detached 70-column tmux capture checks seven long entries, hanging indents, heading word boundaries, bold SGR, and failure exit. This is the appropriate terminal boundary. Its 200-row pane does not test scrollback; the spec introduces no scrolling interaction requirement. |
| Full diagnostic reports termination accurately | L1 component cases and spawned timeout/nonzero-exit regressions, with the shared renderer exercised at L2 | Branch wording and actual termination wiring are checked at L1; the terminal layout mechanism is checked at L2. No level mismatch. |
| Old/empty summary compatibility and durable machine facts | L1 summary JSON, synthetic session-row, and SQLite ingestion/query tests | Empty omission, legacy deserialization, populated extras, and verbatim padded values covered. Dashboard/reporting retention uses the preserved extra data; no new dashboard UI is promised. |
| Documentation and operational guidance | Source inspection of topics, skill snapshots, and durable research-summary source | Startup semantics, semantic failure, and plugin-version guidance are present. The watchdog comment exception is finding 1. |
| Cross-platform termination and focus preservation | Portable L1 fixtures with Unix/Windows scripts; detached tmux L2 | Shared assertions use typed facts rather than Unix signal numbers. This review runs on macOS; Linux and Windows execution are not independently verified here. |

No requirement was found whose strongest existing test is at an inappropriately low verification level. No L3 keyboard injection is needed for this fix.

## Verification performed

- `just test`: passed; **6,861 tests passed, 11 skipped**, 38.835 seconds of test execution after compilation. Skips are not counted as verified behavior.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 incomplete_subagent`: passed; the feature-specific tmux capture actually ran and passed (1.115 seconds). The generator invocation selected zero matching tests and adds no evidence.
- `just lint`: passed, including transport guards, lifecycle documentation guards, and Clippy checks across the five claudine crates.
- `git diff --check`: passed. Biscuit File parsed all three review/spec frontmatters; the requested values and boolean types were checked, including final readiness metadata.

The review did not run `cargo fmt`, commit changes, launch real provider sessions, or inject OS keyboard events. Linux and Windows runtime results remain outside this macOS verification run.

The implementation log records guard-neutralization failures and restoration for the original guards and all three review-3 corrections. This review inspected those records and the resulting regression assertions but did not repeat source mutations.

GitNexus reported medium impact for ledger finalization (five indexed direct references, no indexed execution flows). Its line numbers and test names lag the current sources, so this is navigation evidence, not an exhaustive impact proof. Direct source inspection confirms Claude parser finalization and both CLI summary callers; sniff identifies `claudine-cli` and `claudine-contract` as library consumers. Verification is scoped to the claudine package area's canonical recipes.

## Review metadata

The requested `@prompts/_reviews/claudine/fixes/2026-08-31-silent-success-and-startup-stall/review-3.md` did not resolve through Biscuit File. The existing previous review resolves beside the specification at `claudine/fixes/2026-08-31-silent-success-and-startup-stall/review-3.md`; its `implemented: true` is preserved and `next` points to this review. The spec's `review_iterations` is now `4`.
