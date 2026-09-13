---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T14:02:09-07:00
spec: 2026-08-31-silent-success-and-startup-stall/spec.md
implemented: true
implemented_by: claude/default
log: claudine/fixes/2026-08-31-silent-success-and-startup-stall/log.md
description: A **fix** review of `2026-08-31-silent-success-and-startup-stall/spec.md`
fix: 2026-08-31-silent-success-and-startup-stall/review-1.md
next: 2026-08-31-silent-success-and-startup-stall/review-2.md
---

# Review 1: Silent Success and Startup Stall

## Verdict

The fix is **not ready for production**. The repository contains the ratified
specification and an unchecked execution plan, but the production code still
implements both defects described by the specification. The closest reachable
tests preserve the old behavior rather than verify the requested fix.

## Findings

### 1. Critical: `step_timeout` still cannot terminate startup silence

`evaluate_timeout_tick` still requires `last_activity_at` to be populated
before evaluating the silence age
(`claudine/cli/src/commands/wrap/exec/watchdog/evaluate.rs:78-84,174-176`). A
child that successfully spawns and emits no non-whitespace bytes or semantic
events therefore has no silence reference, so `step_timeout` cannot fire.

OpenCode remains even less bounded. Its cold-start branch returns `Ok`
unconditionally while no step has started or finished
(`evaluate.rs:132-168`). The Level 1 test
`opencode_cold_start_suppresses_even_when_both_clocks_stale` explicitly asserts
that this indefinitely silent state must not time out
(`watchdog/tests/opencode.rs:422-462`). That test passed in this review, proving
the contradictory behavior is both compiled and reachable.

The warning path has the same defect: `maybe_emit_step_timeout_warn` returns
when `last_event_at` is absent and does not consult either `started_at` or
`last_byte_at` (`watchdog/spawn.rs:475-487`). Consequently, startup warnings,
startup-specific warning wording, and warning reset after startup activity are
also absent.

**Required change:** implement one effective silence reference using the newest
of child `started_at`, `last_event_at`, and `last_byte_at`; bound the OpenCode
cold-start exception with that same age; and use the same reference and episode
state for `step_timeout_warn`. Preserve wall-clock precedence and the existing
post-activity in-flight suppression.

### 2. Critical: stopped or unresolved Claude tasks still become silent success

The parser still routes `TaskNotification` through `handle_task_progress`, so a
notification with `status: "stopped"` becomes `SemanticEvent::Info` and loses
its task identity and status (`claudine/lib/src/stream/providers/claude.rs:636-641,861-868`).
Only `task_completed` currently becomes a terminal subagent observation.

No authoritative session task ledger exists in the Claude parser, and
`StreamExecutionSummary` has no provider-agnostic task-outcome list
(`claudine/lib/src/stream/summary.rs:57-100`). Accordingly, the implementation
cannot retain more than five outcomes, keep anonymous observations distinct,
reconcile stopped/completed events by ID, poison started-without-terminal
tasks, or preserve unknown raw statuses. `summary_to_event_meta_with_context`
also has no `extra["subagent_outcomes"]` projection
(`claudine/lib/src/stream/reporting.rs:109-150`).

The general semantic-failure boundary remains broken too. `AttemptOutcome`
does not carry `StreamExecutionSummary.is_error`
(`claudine/lib/src/harness/model.rs:68-105`), `build_attempt_outcome` drops it,
and `classify_failure` declares every completed exit-code-0 attempt successful
(`claudine/lib/src/harness/runtime.rs:20-40,43-72`). A Claude result with
`is_error: true` and native exit 0 can therefore still fire the success
lifecycle stack, even independently of task notifications.

**Required change:** implement the unbounded, attempt-scoped task ledger and
serializable outcome facts; finalize unresolved tasks as
`incomplete_subagents` without changing the native exit or termination; carry
the semantic error bit through `AttemptOutcome`; and route completed semantic
errors through `AgentFailure`. The concise 240-character headline, complete
`Prose`/`UnorderedList` diagnostic, JSONL projection, and reporting/dashboard
retention must be wired from that same authoritative data.

### 3. High: the required regression evidence and documentation did not land

The implementation has none of the acceptance fixtures for a no-output child,
startup byte-heartbeat polarity, startup warning/reset, the two-stopped-task
incident replay, more-than-five task retention, anonymous observations,
identity reconciliation, started-without-terminal failure, semantic-error exit
0, or old/new summary JSON compatibility. The nearest timeout process tests
begin only after provider output and are Unix-only
(`claudine/cli/tests/wrap_watchdog_timeout.rs:1-123`), so they do not prove the
platform-neutral startup termination contract.

The required documentation update is likewise absent. For example, both the
authoritative timeout topic and its skill snapshot still promise first-event
grace and state that startup cannot be killed by `step_timeout`
(`claudine/docs/topics/timeouts.md:63-76` and
`.claude/skills/claudine/timeouts.md:85-98`). The source comments repeat the
same obsolete contract (`watchdog/evaluate.rs:35-40,78-84,138-155` and
`exec/timeouts.rs:5-14`). No exact-version-pin operational guidance for the
OpenCode `@latest` startup dependency was added. The specification also names
`claudine/docs/topics/opencode-event-sources.md`, but that authoritative file
does not exist; only the skill snapshot is present, so implementation should
either restore the documented source or correct the required-document list
before synchronizing it.

Per the repository's comment-drift rule, these comments and docs are evidence
that the behavior change was not made, not merely missing polish.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| A no-output structured child is terminated by `step_timeout` from spawn | Level 1 watchdog unit test | **Wrong behavior:** the reachable test asserts indefinite startup suppression. A platform-neutral Level 1 re-exec/process test is required. |
| Non-whitespace startup bytes refresh the clock; whitespace does not | Level 1 unit coverage for byte recording and post-event silence | Partial infrastructure only; no test crosses the startup fallback boundary because that boundary does not exist. |
| `step_timeout_warn` warns once during startup and resets after activity | Level 1 post-event warning behavior | Gap: the production helper exits before the first event. Level 1 is the appropriate tier. |
| OpenCode startup and pre-first-finish stalls are bounded while fresh mid-step activity remains protected | Level 1 watchdog tests | **Wrong behavior:** the cold-start counter-test preserves the defect. |
| Claude terminal notifications retain ID/name/raw status and stopped tasks poison success | Level 1 tests for `task_progress` and `task_completed` only | Gap: no `task_notification` terminal-normalization or incident-replay coverage. |
| Every unresolved task, including more than five and anonymous tasks, appears in the diagnostic and machine facts | None | Gap: no ledger, projection, or process assertion exists. Level 1 parser/process coverage is required. |
| A semantic error with native exit 0 fires failure, not success | None | Gap: Level 1 classifier and lifecycle process coverage are required; current production classification returns success. |
| Startup termination is portable across macOS, Windows, and Linux | Existing related process coverage is Unix-only Level 1 | Gap: add reachable platform-neutral/native `cfg` Level 1 evidence without asserting Unix signal numbers. |

No Level 3 coverage is applicable because the fix does not depend on physical
keyboard or mouse input. No feature-specific Level 2 test is required for the
timeout and lifecycle semantics themselves; Level 1 process tests can prove
exit status, stderr content, and JSONL output. If the implementation makes
claims about terminal-specific layout, styling, glyph widths, or scrolling for
the new full diagnostic, those claims additionally require Level 2 capture in
a real terminal. The package's canonical `just test-l2` remains a required
regression gate under acceptance criterion 13, but it cannot substitute for
the missing Level 1 behavior tests.

## Verification Performed

- `just test-cli opencode_cold_start_suppresses_even_when_both_clocks_stale`:
  **passed** (1 test, 2,470 skipped). This is negative evidence: it proves the
  current reachable suite still enforces the obsolete unbounded OpenCode
  startup grace.
- Source review confirmed the plan's seven phases remain unchecked and the
  only commit for this fix directory added `plan.md` and `spec.md`; it added no
  implementation.
- Full `just test`, `just test-l2`, and `just lint` were not run because the two
  production defects and their contradictory Level 1 test are already
  decisive readiness blockers. No green broader gate could establish the
  missing behavior.

## Closure Criteria

Complete all seven execution-plan phases, replace the contradictory cold-start
test with the specified positive and counter-case regressions, add the full
Level 1 acceptance matrix above, synchronize the authoritative docs and skill
copies, and run the canonical Claudine `just test`, relevant `just test-l2`, and
`just lint` gates. Production readiness requires both original incidents to
fail under the pre-fix condition and pass through the real wrapper/lifecycle
path after the implementation.
