---
total_phases: 7
created: 2026-08-31
phase: 1
agent: codex/default
yolo: true
spec: claudine/fixes/2026-08-31-silent-success-and-startup-stall/spec.md
---

# Silent Success and Startup Stall — Execution Plan

This plan implements the reviewed fix specification in
[`spec.md`](claudine/fixes/2026-08-31-silent-success-and-startup-stall/spec.md).
It adopts the specification's fail-closed ruling: every unresolved stopped,
unknown-status, or unterminated task makes a structured provider attempt fail.
Capture and interactive/passthrough modes remain outside `step_timeout`, and
the repository change must not edit the operator's OpenCode configuration.

## Dependency and Parallelization Overview

- Phase 1 establishes the current behavior, blast radius, fixtures, and
  regression-test seams required by all later phases.
- Phases 2 and 4 are independent after Phase 1 and are explicitly
  parallelizable: Phase 2 owns startup timeout/warning behavior, while Phase 4
  owns terminal-task normalization and the library ledger.
- Phase 3 depends on Phase 2 and proves startup behavior through the real
  platform-neutral wrapper harness.
- Phase 5 depends on Phase 4. It may run in parallel with Phase 3 and carries
  semantic failure through attempt classification, lifecycle dispatch,
  diagnostics, JSONL, reporting, and dashboard projections.
- Phase 6 begins once the behavior and public data contract from Phases 2, 4,
  and 5 are stable.
- Phase 7 closes only after all implementation and documentation phases pass.
- Primary code areas are `claudine/lib/src/stream/`,
  `claudine/lib/src/harness/`, `claudine/cli/src/commands/wrap/exec/`,
  `claudine/cli/src/commands/wrap/harness_orch/`, and
  `claudine/lib/src/reporting/`.

## Phase 1 — Baseline, Impact Gates, and Regression Fixtures

*Goal: freeze the two defects in focused tests and establish the exact edit
surface before changing behavior.*

- [ ] Record a clean baseline with `git status --short`, run the focused
  existing timeout/parser/runtime tests through nextest, and note any
  pre-existing failures without changing unrelated files.
- [ ] Run GitNexus upstream impact analysis before editing each affected
  symbol, including at minimum `evaluate_timeout_tick`,
  `maybe_emit_step_timeout_warn`, `should_warn_stall`, the Claude task-event
  handlers and parser finalizer, `StreamExecutionSummary`, `AttemptOutcome`,
  `build_attempt_outcome`, `classify_failure`, and
  `summary_to_event_meta_with_context`; stop and warn before implementation if
  any result is HIGH or CRITICAL.
- [ ] Map and record all structured spawn/finalization call sites so the change
  covers normal semantic execution and any structured wire-mode path without
  extending `step_timeout` to capture or interactive/passthrough execution.
- [ ] Add or identify deterministic re-exec fixtures for: no output until
  killed; periodic non-whitespace startup bytes; whitespace-only startup
  bytes; OpenCode activity without a completed step; and the Claude incident
  stream with two stopped task notifications followed by a successful native
  result and exit 0.
- [ ] Add failing characterization tests for the current defects before the
  implementation changes: startup silence currently survives
  `step_timeout`, Claude `task_notification status:"stopped"` currently
  becomes `Info`, and completed exit 0 with `summary.is_error == true`
  currently classifies as success.
- [ ] Keep shared integration assertions platform-neutral: assert typed
  termination and error fields rather than Unix signal numbers, use the
  existing re-exec/test-harness process controls, and do not open or focus a
  terminal or browser window.

**Validation checkpoint:**

- [ ] Run the new focused tests through `cargo nextest run` filters and confirm
  they fail for the intended missing contracts rather than fixture setup,
  timing variance, or platform assumptions.
- [ ] Confirm the fixture inventory covers all acceptance-criteria branches:
  startup timeout, byte heartbeat polarity, warning/reset, OpenCode guard,
  task normalization, ledger reconciliation, semantic exit-0 failure, and
  machine-data compatibility.

## Phase 2 — Spawn-Anchored `step_timeout` and Warning Semantics

*Goal: make the existing silence rule and warning clock begin at successful
child spawn while preserving wall-clock precedence and established in-flight
suppression.*

- [ ] Refactor the timeout evaluation seam to compute one effective silence
  reference as the newest of `started_at`, `last_event_at`, and
  `last_byte_at`; use the spawn instant only until real activity exists and do
  not add a timeout key, duration, or termination reason.
- [ ] Preserve the existing non-whitespace byte contract by exercising
  `LiveMetricsState::record_byte_activity`: real bytes refresh startup silence,
  while empty and whitespace-only chunks leave the spawn-based budget
  advancing.
- [ ] Bound the OpenCode cold-start exception by the same effective silence
  age so missing `provider_status` cannot suppress forever; retain mid-step
  suppression while either event or byte activity is fresh and permit a
  breach once both clocks are stale for the full budget.
- [ ] Preserve rule ordering so a wall-clock `timeout` breach wins when both
  budgets expire on the same watchdog tick, and preserve the current
  active-versus-stuck tool/subagent gate after activity begins.
- [ ] Extend the step-timeout breach context/formatter to distinguish “no
  activity since launch” from “OpenCode activity followed by a stall before
  the first completed step,” while retaining `ProcessTermination::TimedOut`,
  `error_kind: "step_timeout"`, and the existing cross-platform termination
  ladder.
- [ ] Make `step_timeout_warn` use the same effective spawn/event/byte clock;
  emit startup-specific wording before first activity, fire once per stall
  episode, and preserve the existing reset rule after activity advances past
  the prior warning.
- [ ] Update unit tests in the watchdog timeout, OpenCode, breach-message, and
  live-progress test modules to cover no-activity startup, byte polarity,
  warning dedup/reset, cold-start release, fresh single-clock suppression,
  both-clocks-stale breach, one-shot firing, and wall-clock precedence.

**Validation checkpoint:**

- [ ] Run the focused watchdog/progress test binaries with nextest and confirm
  every timing assertion uses deterministic `Instant` offsets rather than
  sleeping where a pure evaluation test is sufficient.
- [ ] Confirm the changed tests retain explicit companion cases for healthy
  startup, active in-flight work, OpenCode mid-step activity, and clean
  wall-clock behavior so the fix cannot collapse into “always time out.”

## Phase 3 — Cross-Platform Startup-Stall Integration Coverage

*Goal: prove the spawn-anchored rule terminates real structured children and
does not regress heartbeat or platform behavior.*

- [ ] Extend `claudine/cli/tests/wrap_watchdog_timeout.rs` (or the existing
  equivalent wrapper integration target) with a no-output re-exec child and no
  wall-clock timeout; assert termination within `step_timeout` plus one
  watchdog interval, `TimedOut`, `error_kind: "step_timeout"`, and the
  startup-specific diagnostic.
- [ ] Add companion re-exec cases proving periodic non-whitespace output moves
  the deadline and whitespace-only output does not; keep total fixture
  duration short enough for the ordinary L1 suite.
- [ ] Add an OpenCode-shaped integration case with no completed step to prove
  absent `provider_status` no longer exempts startup, plus a healthy-progress
  companion that survives while an activity clock remains fresh.
- [ ] Assert only platform-neutral termination semantics in shared tests and
  reuse the established Unix/Windows process-group teardown path instead of
  adding test-only signal handling.
- [ ] Verify capture and passthrough call paths neither start the structured
  watchdog nor acquire the new spawn-based `step_timeout` behavior.

**Validation checkpoint:**

- [ ] From `claudine/`, run the focused integration target through nextest,
  then `just test-cli` with the relevant filters; confirm child processes are
  reaped and nextest reports no leaks.
- [ ] Run the relevant `just test-l2` filter only if the existing tiering puts
  the changed coverage at L2; ensure the harness remains headless and no
  terminal or browser window gains focus.

## Phase 4 — Terminal Task Normalization and Authoritative Session Ledger

*Goal: preserve Claude terminal task facts and finalize a complete,
provider-agnostic task-outcome contract in the library stream layer.*

- [ ] Add a provider-agnostic serializable normalized task outcome and fact
  type under `claudine/lib/src/stream/`; each fact carries optional provider
  task ID, optional name, normalized outcome, and optional raw provider status.
  Use serde defaults and omit an empty summary list for backward-compatible
  JSON.
- [ ] Add an unbounded, attempt-scoped task ledger in the library stream layer
  and keep `WatchdogState::recent_subagents` unchanged as a five-entry,
  diagnostic-only ring.
- [ ] Reconcile ledger entries strictly by nonempty provider task ID: start or
  resume makes the ID in flight; terminal stopped/unknown state remains
  incomplete; and a later recognized-success terminal event for the same ID
  clears the earlier failure. Never reconcile by name or description.
- [ ] Give every anonymous start/terminal observation a distinct internal
  identity so missing IDs are not converted to `""` and one anonymous event
  cannot clear another; keep these synthetic identities internal rather than
  falsifying the provider task ID in machine output.
- [ ] Normalize successful raw statuses with an explicit allowlist containing
  at least `completed`, `success`, and `succeeded`; normalize `stopped` as
  unsuccessful; preserve every unknown status string and finalize it as
  unresolved until a tested vocabulary extension is added.
- [ ] Keep Claude `task_progress` as `SemanticEvent::Info`; route terminal
  `task_notification` records through the same `SubagentStop`/ledger path as
  `task_completed`, preserving ID, name, raw status, and relevant raw-kind
  metadata. A notification lacking terminal status must not invent success.
- [ ] Finalize every started-without-terminal, stopped, or unknown-status entry
  into `StreamExecutionSummary.subagent_outcomes`; when any remain incomplete,
  set `is_error = true`, `error_kind = "incomplete_subagents"`, and a complete
  operator-facing source message while preserving the native `exit_code`.
- [ ] Add library tests for progress-versus-terminal normalization, more than
  five stopped tasks, duplicate events for one ID, distinct anonymous facts,
  stopped-then-success for one ID, same-name/different-ID non-reconciliation,
  started-without-terminal, unknown status preservation, and clean completion.
- [ ] Add summary serde tests proving legacy JSON without
  `subagent_outcomes` defaults to an empty list, empty lists are omitted, and a
  populated list round-trips without losing raw status or anonymous facts.

**Validation checkpoint:**

- [ ] Run focused `claudine` library parser, ledger, semantic-fidelity, and
  summary serde tests through nextest.
- [ ] Replay the incident fixture directly through the Claude parser and
  confirm native exit 0 is retained while the summary reports two complete
  machine facts and `incomplete_subagents`.

## Phase 5 — Semantic Failure Classification, Lifecycle, and Observability

*Goal: make parser-semantic errors authoritative throughout execution without
falsifying native process termination or dropping task facts.*

- [ ] Add `is_error` to `AttemptOutcome`, populate it in every structured
  outcome-construction path from `StreamExecutionSummary`, and explicitly set
  it for non-stream attempt constructors so clean capture/passthrough behavior
  remains unchanged.
- [ ] Change `classify_failure` so `ProcessTermination::Completed` becomes
  `AgentFailure` when either `exit_code != 0` or `is_error` is true; retain the
  existing handling for `TimedOut`, `Interrupted`, `LaunchFailed`, and
  `Aborted`.
- [ ] Add focused runtime tests for a non-task semantic error with native exit
  0, an incomplete-subagent semantic error with native exit 0, and a clean
  completed exit-0 summary; this makes the provider-agnostic behavior explicit.
- [ ] Carry `error_kind: "incomplete_subagents"` and the concise failure
  headline through the lifecycle `err` snapshot, honoring the existing
  240-character hygiene limit with the incomplete count and as many task names
  as fit.
- [ ] Add an end-to-end lifecycle replay for the two-stopped-task incident:
  termination remains `Completed`, native exit remains 0, the failure stack
  fires with `err.kind == "incomplete_subagents"`, the success stack and its
  side effects do not run, and recovery occurs only if the ordinary failure
  stack explicitly requests it.
- [ ] Render the full incomplete-task diagnostic with
  `TerminalRenderable` components using `Prose` and `UnorderedList`; enumerate
  every incomplete fact independently of the concise headline and keep quiet/
  silent output behavior consistent with existing policy.
- [ ] Project a nonempty summary list to the synthetic SessionEnd row as the
  stable top-level `extra["subagent_outcomes"]` field in
  `summary_to_event_meta_with_context`; do not place it under
  `provider_summary` or Claude `raw_summary`.
- [ ] Audit JSONL writing, SQLite `extra_json` ingestion, `claudine logs`
  machine records, and rendezvous/dashboard session projections. Add explicit
  retention tests, and update only fixed-schema consumers that currently drop
  the populated list.
- [ ] Add integration assertions that the complete machine list survives even
  when terminal headline/display text truncates task names.

**Validation checkpoint:**

- [ ] Run focused harness runtime, lifecycle dispatch/control, stream
  reporting, reporting-ingest/query, and dashboard/rendezvous tests through
  nextest.
- [ ] Inspect the synthesized SessionEnd JSON from the incident replay and
  verify `extra.subagent_outcomes` is top-level and complete, while
  `exit_code == 0`, termination is `Completed`, and error classification is
  `AgentFailure`.

## Phase 6 — Contract Documentation and Operational Guidance

*Goal: make the new timeout and outcome contracts authoritative and keep all
materialized skill snapshots synchronized.*

- [ ] Update `claudine/docs/topics/timeouts.md` and
  `.claude/skills/claudine/timeouts.md` together: spawn is the initial silence
  anchor, non-whitespace bytes refresh it, startup warnings share the clock,
  OpenCode cold start is bounded, and wall-clock timeout retains precedence.
- [ ] Add the required startup-stall operational section explaining that
  OpenCode plugin specs using `@latest` depend on npm reachability and
  recommending exact version pins; state that Claudine does not mutate user
  configuration.
- [ ] Update `claudine/docs/topics/signal-handling.md` and
  `.claude/skills/claudine/signal-handling.md` together to preserve the native
  exit/`Completed` distinction for semantic failures and the existing
  platform-specific termination ladder for real timeout kills.
- [ ] Update `claudine/docs/topics/non-interactive-sessions.md` and
  `.claude/skills/claudine/summaries/non-interactive-sessions.md` together with
  the authoritative `is_error` success contract and incomplete-subagent
  SessionEnd machine data.
- [ ] Update `claudine/docs/topics/opencode-event-sources.md` and
  `.claude/skills/claudine/opencode-event-sources.md` together with the bounded
  cold-start guard and unchanged `stall_timeout` semantics.
- [ ] Reconcile behavior-bearing rustdocs/module docs and inline comments in
  `watchdog/evaluate.rs`, `exec/timeouts.rs`, `LiveMetrics` progress state,
  `HarnessPlan::step_timeout`, summary/outcome types, and runtime
  classification; remove stale claims about unbounded first-event grace or
  exit 0 being sufficient for success.
- [ ] Verify authoritative topic files and materialized skill copies are
  byte-equivalent where the repository expects mirrored content, or differ
  only by established snapshot framing.

**Validation checkpoint:**

- [ ] Search the changed documentation and code comments for stale phrases
  such as “first-event grace,” “suppress unconditionally,” and “exit code 0 =
  success,” and confirm any remaining occurrence accurately describes a
  historical or negated statement.
- [ ] Confirm the operational guidance does not include editing
  `~/.config/opencode/config.json` as an implementation or acceptance step.

## Phase 7 — Non-Vacuity, Canonical Verification, and Scope Audit

*Goal: prove every new guard detects its intended regression and close with the
package area's canonical gates and expected blast radius.*

- [ ] Perform the specification's non-vacuity check for each new guard by
  temporarily neutralizing one condition at a time—spawn fallback, byte
  heartbeat polarity, bounded OpenCode guard, terminal notification routing,
  ledger poisoning, `is_error` classification, and SessionEnd projection—run
  the focused test to observe failure, then restore the implementation.
- [ ] Run `just test` from `claudine/` and confirm the canonical L1 suite uses
  nextest and leaves no child processes behind.
- [ ] Run the relevant headless `just test-l2` coverage from `claudine/` with
  no terminal or browser focus, then run `just lint`; do not use `cargo test`
  and do not run `cargo fmt`.
- [ ] Run targeted serde/compatibility and reporting/dashboard filters again
  after lint fixes so machine-contract coverage cannot be masked by the broad
  suite.
- [ ] Review macOS-tested behavior for Windows and Linux portability: no Unix
  signal constants in shared assertions, no shell-only fixture assumptions,
  and all production termination continues through the established
  cross-platform abstraction.
- [ ] Run `git diff --check`, review `git diff --stat` and the full diff for
  surgical scope, and verify no user OpenCode configuration, unrelated source,
  or generated artifact was changed.
- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` and
  confirm only the expected timeout, stream parsing/summary, harness lifecycle,
  reporting/dashboard, test, documentation, and skill-snapshot flows are
  affected; investigate any unexpected symbol or execution flow before
  closure.
- [ ] Record final evidence against all 14 acceptance criteria, including
  exact commands, test names, platform limitations, and the native-exit-0
  incident replay result. Do not commit unless separately instructed.

**Final validation checkpoint:**

- [ ] `just test`, relevant `just test-l2`, and `just lint` are green in the
  `claudine/` package area; focused non-vacuity checks have been restored and
  rerun green.
- [ ] The final diff contains synchronized docs/skill snapshots, complete
  machine facts, bounded terminal text, no false native exit code, and no
  expansion of `step_timeout` into capture or interactive/passthrough modes.
