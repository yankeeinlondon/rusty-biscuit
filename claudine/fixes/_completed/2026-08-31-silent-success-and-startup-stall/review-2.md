---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T16:01:20-07:00
spec: 2026-08-31-silent-success-and-startup-stall/spec.md
implemented: true
implemented_by: claude/default
log: claudine/fixes/2026-08-31-silent-success-and-startup-stall/log.md
description: A **fix** review of `2026-08-31-silent-success-and-startup-stall/spec.md`
fix: 2026-08-31-silent-success-and-startup-stall/review-2.md
previous: 2026-08-31-silent-success-and-startup-stall/review-1.md
next: 2026-08-31-silent-success-and-startup-stall/review-3.md
---

# Review 2: Silent Success and Startup Stall

## Verdict

The fix is **not ready for production**. The second implementation closes the
original startup-silence hole and adds a session-scoped task ledger, semantic
failure routing, diagnostics, and broad Level 1 coverage. The canonical unit,
integration, Level 2, and lint recipes all pass. However, several ratified
contracts are either implemented differently from the specification or are not
verified at the level needed to support the user-facing claim.

## Findings

### 1. Critical: OpenCode can still suppress a timed-out in-flight step forever when only one activity clock exists

`evaluate_timeout_tick` defines `both_clocks_stale` as `false` whenever either
`last_event_at` or `last_byte_at` is absent, then treats `!both_clocks_stale` as
recent activity (`claudine/cli/src/commands/wrap/exec/watchdog/evaluate.rs:156-167`).
Consequently, an OpenCode step with one populated but stale clock and one absent
clock remains suppressed indefinitely. This contradicts the requirement that
mid-step suppression applies only while **at least one** clock is fresh, and
that timeout evaluation resumes when no clock is fresh.

The Level 1 tests cover one fresh clock and two populated stale clocks, but not
the one-populated-stale/one-absent state. GitNexus rates the upstream blast
radius of `evaluate_timeout_tick` as **critical**: 33 direct callers, 36 impacted
symbols, and the `execute_harness_attempt` flow. Replace the double-staleness
inference with an explicit `any clock is fresh` predicate and add both
one-clock-stale permutations as regression cases.

### 2. High: unknown `task_notification` terminal statuses are downgraded and their raw status is lost

`handle_task_notification` routes a notification to terminal handling only when
`terminal_outcome_for_status` already recognizes the status; every unknown
status falls through to `handle_task_progress`
(`claudine/lib/src/stream/providers/claude.rs:655-663`). The test for
`status: "thinking"` explicitly expects an `Info` event and a clean summary
(`claudine/lib/src/stream/providers/claude/tests.rs:850-860`). The separate
unknown-status test uses `task_completed`, so it does not cover this path.

The specification requires terminal notifications to preserve ID, name, and
raw status and requires unknown terminal statuses to remain unresolved. As
implemented, an unknown future terminal status may be silently accepted, and a
previously started task becomes merely `Unfinished` with the provider status
discarded. Preserve unknown notification statuses in the ledger and fail closed,
or ratify and specify an explicit nonterminal notification vocabulary before
downgrading statuses.

### 3. High: incomplete-subagent failures do not expose the specified machine identity

`TaskLedger::apply_to_summary` sets `error_kind` to
`incomplete_subagents` only when no earlier provider error kind exists
(`claudine/lib/src/stream/task_ledger.rs:271-284`). The specification instead
requires this value whenever unresolved stopped or unfinished tasks remain.
This precedence changes the stable machine-facing exit reason for sessions that
have both a provider error and incomplete tasks.

The end-to-end lifecycle test also asserts `err.variant ==
"incomplete_subagents"` rather than the specified `err.kind`
(`claudine/cli/tests/wrap_incomplete_subagents.rs:161-169,230-234`). The runtime
keeps `err.kind == "LifecycleAction"`. The implementation log and documentation
describe these as deliberate deviations, but the ratified specification was
not updated. Implement the specified identity, or revise and ratify the
contract and its compatibility implications before treating the fix as ready.

### 4. High: the startup termination deadline is not asserted by the process-level test

The no-output Level 1 integration test configures a 2-second `step_timeout`, a
200-millisecond watchdog interval, and a 500-millisecond kill grace, but only
wraps the command in a 60-second outer timeout
(`claudine/cli/tests/wrap_watchdog_startup_stall.rs:145-173`). It never measures
or asserts the specification's `step_timeout + one watchdog interval` bound.
In this review the focused test completed in 2.678 seconds, beyond the literal
2.2-second acceptance bound, although the extra time is plausibly the configured
kill grace.

Level 1 is the correct verification level for this process deadline, but the
current assertion is too weak: a large regression could pass for almost a
minute. Clarify whether the acceptance bound applies to timeout detection,
termination request, or final child reap; then assert the appropriate elapsed
bound, including kill grace only if the ratified contract permits it.

### 5. High: the full terminal diagnostic has no feature-specific Level 2 verification

Level 1 tests verify that rendered stderr contains every stopped task name,
including more than five tasks. They do not run this diagnostic in a real
terminal emulator or capture its pane. The canonical Level 2 suite passes, but
none of its tests exercises the incomplete-subagent diagnostic. Therefore it
does not verify wrapping, widths, list rendering, or scrollback behavior for the
required full terminal diagnostic.

Add a Level 2 test that runs the fixture in a supported real terminal at a
constrained width and captures the pane or scrollback, asserting that the
heading, remediation, and every task entry survive real-terminal rendering.
Level 3 is not required because this fix has no physical-keyboard encoding
contract.

### 6. Medium: the required Claudine skill summary was not updated

The implementation updates `claudine/docs/topics/non-interactive-sessions.md`,
but not the specifically required
`.claude/skills/claudine/summaries/non-interactive-sessions.md`. The omission is
called intentional in the implementation log, while the specification still
lists the skill summary as a required documentation surface. Update the summary,
or revise the specification if the generated research artifact is no longer a
maintained source. The topic documentation should also avoid presenting the
unratified `error_kind` precedence deviation as the settled contract.

## Requirement Verification Matrix

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Startup silence is bounded from successful spawn | Level 1 unit and spawned-process tests | Core wiring is present, but the required elapsed deadline is not asserted. |
| Byte activity updates the watchdog, including filtered whitespace | Level 1 integration tests | Appropriate level and covered in both positive and negative directions. |
| Startup warning uses the startup reference and remains deduplicated | Level 1 unit tests | Appropriate for the timing and state contract. |
| OpenCode cold start is bounded and mid-step suppression requires fresh activity | Level 1 unit/integration tests | Cold start is covered; one-clock-stale mid-step state is missing and broken. |
| Terminal task notifications preserve identity and status | Level 1 parser tests | Known statuses are covered; unknown notification statuses violate the contract. |
| Incomplete Claude tasks prevent silent success and fire failure lifecycle actions | Level 1 process test | Failure routing works, but the specified `err.kind` identity does not. |
| Full diagnostic enumerates all incomplete tasks | Level 1 renderer/process tests | Content is checked; Level 2 real-terminal rendering and overflow behavior are unverified. |
| Task ledger retains all tasks, including anonymous and duplicate-name tasks | Level 1 unit/parser tests | Appropriate level and covered. |
| General semantic errors with native exit zero classify as agent failure | Level 1 unit test | Appropriate level and covered. |
| JSON, SQLite, and retry compatibility | Level 1 serialization/reporting tests | Appropriate level, subject to the `error_kind` contract mismatch above. |
| macOS, Windows, and Linux support | Cross-platform source paths; macOS execution in this review | No platform-specific defect found, but this review did not execute Windows or Linux CI. |
| Full terminal behavior | Level 1 only for this feature | Level 2 gap; no Level 3 requirement applies. |

## Verification Performed

- `just test` — passed: 6,829 tests, 11 skipped.
- `just test-l2` — passed: 236 CLI Level 2 tests and 3 generator Level 2 tests; none target this fix's terminal diagnostic.
- `just lint` — passed.
- Focused startup-stall, incomplete-subagent lifecycle, notification
  normalization, unknown terminal-status, and semantic-error classification
  tests — passed.
- `git diff --check` — passed before review metadata was written and was rerun
  after the review edits.

The implementation log records the required non-vacuity mutations. This review
did not repeat those temporary source mutations, so that evidence remains a
review artifact rather than a durable automated check.
