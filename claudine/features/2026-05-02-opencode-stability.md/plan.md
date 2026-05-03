---
phases: 6
created: 2026-05-03
start_phase: 1
spec: claudine/features/2026-05-02-opencode-stability.md/spec.md
owner: claudine
packages:
  - claudine
  - claudine-cli
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - claudine
  - claudine-cli
source_files_during_phase_2:
  - claudine/cli/src/commands/wrap/live_semantic_sink.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/lib/src/stream/logs/opencode.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - claudine
  - claudine-cli
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/section.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/composition.md
  - claudine/cli/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/SKILL.md
---

# Plan - OpenCode Stability Watchdogs

Source spec: [`spec.md`](./spec.md).

## Summary

OpenCode can hang during non-interactive `compose` / `inline-compose` /
`sequence` runs when parallel `task` subagents go silent without a matching
completion event. Claudine currently waits only for child process exit and
stdout EOF, so it can deadlock with no user-visible diagnostic.

The fix adds provider-agnostic wrapper state for outstanding subagents, a
watchdog termination path that cooperates with the existing process-group
signal handling, a broader stream-idle fallback, and idle diagnostics on the
existing heartbeat. The work should preserve normal long-running tool calls:
only silent subagents and complete stream silence receive bounded ceilings.

## Dependency Map

| Phase | Outcome | Depends on |
|---|---|---|
| 1 | Pure watchdog state model exists and is unit-tested | none |
| 2 | `LiveSemanticSink` updates and exposes shared watchdog state | 1 |
| 3 | Exec wait loop can be terminated by watchdogs and can mark exit reasons | 1 |
| 4 | Subagent and stream-idle watchdog rules are wired end to end | 2, 3 |
| 5 | Idle diagnostics and fixture replay acceptance tests are covered | 2, 4 |
| 6 | Documentation, skill updates, and final regression sweep land | 1-5 |

## Phase 1 - Watchdog State Model

**Goal.** Add a small, testable state component that tracks active subagents
without touching child-process termination yet.

**Files touched.**

- `claudine/cli/src/commands/wrap/subagent_watchdog.rs` *(new)*
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

**Steps.**

1. Create `subagent_watchdog.rs` under `claudine/cli/src/commands/wrap/`.
2. Define `SubagentId`, `ActiveSubagentInfo`, `ActiveSubagentSnapshot`,
   `SubagentDiagnosticLine`, and `WatchdogState`.
3. Store, at minimum, subagent id, optional name/title, `started_at`,
   `last_progress_at`, and the last diagnostic emission time.
4. Add methods:
   - `subagent_started(id, name, now)`
   - `subagent_stopped(id, now)`
   - `observe_subagent_progress(id, now)`
   - `active_subagents(now) -> Vec<ActiveSubagentSnapshot>`
   - `stuck_subagents(now, threshold) -> Vec<ActiveSubagentSnapshot>`
   - `diagnostic_lines(now, silence_window) -> Vec<SubagentDiagnosticLine>`
5. Keep the state generic over `Instant` passed by callers; do not call
   `Instant::now()` inside mutation methods except in thin convenience wrappers.
6. Register the module in `wrap/mod.rs` only after the new file compiles.

**Parallelizable.**

- Unit tests for `WatchdogState` can be written in parallel with the model
  implementation because they do not touch existing wrapper code.

**Validation checkpoint.**

- `cargo test -p claudine-cli subagent_watchdog`
- Tests cover N starts and M stops, duplicate starts updating metadata without
  losing timestamps unexpectedly, progress resetting only the matching
  subagent, and diagnostic lines being emitted at most once per silence window.

## Phase 2 - Sink Integration

**Goal.** Feed the state model from the same semantic event stream already used
for live rendering and metrics.

**Files touched.**

- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

**Steps.**

1. Add `watchdog_state: Arc<Mutex<WatchdogState>>` to `LiveSemanticSink`.
2. Add `LiveSemanticSink::watchdog_state(&self) -> Arc<Mutex<WatchdogState>>`
   so `exec.rs` can share the same state with ticker threads.
3. In `SemanticEventSink::on_semantic_event`, update `LiveMetrics` first as
   today, then update `watchdog_state` before rendering:
   - `SubagentStart { id, name, .. }` inserts or refreshes an active entry.
   - `SubagentStop { id, name, .. }` drains the matching entry.
   - Any event carrying a recognized subagent id updates `last_progress_at`.
4. Implement a narrow helper to extract subagent ids from semantic events:
   start/stop ids directly, plus OpenCode `task_progress` style data from
   `Info.payload` or `extra` when present. Leave unknown events untouched.
5. Add a read-only sink accessor returning a snapshot suitable for diagnostics
   without holding the sink mutex during writes.
6. Keep existing `SubagentStart` / `SubagentStop` visible rendering unchanged.

**Parallelizable.**

- Event-id extraction tests can be built independently from the sink field
  plumbing once the helper signature is decided.

**Validation checkpoint.**

- `cargo test -p claudine-cli live_semantic_sink::tests::subagent`
- Existing `SubagentStart` / `SubagentStop` render tests still pass.
- New tests assert active entries are inserted and removed through
  `on_semantic_event`, and that OpenCode progress payloads reset
  `last_progress_at` for the matching id.

## Phase 3 - Termination Plumbing

**Goal.** Give watchdogs a single, deterministic way to request child-process
termination and to mark the synthesized summary with a distinct exit reason.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`

**Steps.**

1. Extend the existing early-termination model in `exec.rs` with:
   - `SubagentsUnresponsive { message }`
   - `StreamIdleTimeout { message }`
2. Add a `WatchdogTermination` request type carrying the reason, rendered
   message, and optional stuck-subagent snapshots.
3. Thread a termination channel into
   `wait_with_signal_and_early_termination`; when a watchdog request arrives,
   reuse the existing SIGTERM to process group, grace wait, and SIGKILL
   escalation logic instead of adding a second signal path.
4. Ensure the non-advanced wait path is promoted to the advanced wait path
   whenever any watchdog is enabled, even if wall-clock timeout, step timeout,
   and OpenCode stderr early-termination are absent.
5. Add `apply_early_termination_to_summary` handling for:
   - exit reason `subagents_unresponsive`
   - exit reason `stream_idle_timeout`
6. Preserve the existing user SIGINT behavior and `exit_code: 130` reporting.

**Parallelizable.**

- Summary override tests can be written while the channel plumbing is in
  progress, as long as they target `apply_early_termination_to_summary`
  directly.

**Validation checkpoint.**

- `cargo test -p claudine-cli wrap::exec`
- Unit tests prove watchdog termination maps to the new exit reasons and user
  interrupt still maps to the existing SIGINT semantics.
- `cargo build -p claudine-cli`

## Phase 4 - Watchdog Rules

**Goal.** Run the two kill rules from a dedicated ticker: subagent silence
first, stream silence second.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`

**Steps.**

1. Add a `WatchdogConfig` parsed from environment:
   - `CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS`, default `180`, `0` disables
   - `CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS`, default `10`
   - `CLAUDINE_SUBAGENT_WATCHDOG_INTERVAL_SECONDS`, default `5`
   - `CLAUDINE_STREAM_IDLE_KILL_SECONDS`, default `300`, `0` disables
2. Add `spawn_subagent_watchdog_ticker` next to
   `spawn_flush_if_idle_ticker`; keep cadences and side effects separate.
3. On each tick, evaluate rules in priority order:
   - If active subagents exist and any exceeded the subagent idle threshold,
     render/send one `SubagentsUnresponsive` request.
   - If active subagents exist but none is over threshold, suppress stream-idle
     evaluation for that tick.
   - If no subagents are active, evaluate stream-level silence using
     `LiveMetrics.last_event_at`, only after at least one activity event beyond
     session start.
4. Render the first breach as a `SemanticEvent::Error` equivalent with
   `SemanticErrorKind::AgentNative` so the live sink's coloured `BlockQuote`
   contract is preserved.
5. Stop and join the watchdog ticker after the wait loop returns, mirroring
   `stop_timing_ticker(flush_ticker)`.
6. Guard against double-fire with an atomic or shared fired flag.

**Parallelizable.**

- Env parsing tests and rule-priority tests can run in parallel with ticker
  wiring because they live on pure helpers.

**Validation checkpoint.**

- `cargo test -p claudine-cli subagent_watchdog`
- `cargo test -p claudine-cli wrap::exec::tests::watchdog`
- Tests cover disabled thresholds, subagent breach, stream-idle breach, stream
  idle suppressed while subagents are outstanding, and one-shot firing.

## Phase 5 - Diagnostics and Fixture Acceptance

**Goal.** Make stuck sessions diagnosable before termination and prove the
reference hang class terminates with the expected stderr and summary.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- `claudine/cli/tests/wrap_commands.rs`

**Steps.**

1. Extend the existing `flush_if_idle` ticker callback to consult the shared
   watchdog snapshot after flushing buffered prose.
2. Emit at most one diagnostic line per active subagent per silence window:
   ` ⏳ Awaiting subagent: <name-or-id> (<elapsed-since-start>)`.
3. Route diagnostic lines through `SectionTracker` and the Tool Use & Events
   section so spacing stays consistent with the current live surface.
4. Gate diagnostics on `CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS != 0`; disabling
   the subagent watchdog also disables the idle diagnostic.
5. Add an end-to-end fixture replay for the reference shape:
   9 `task_started`, 7 `task_completed`, then no further stream lines.
6. Assert the run terminates after the configured threshold plus interval and
   grace, stderr names the 2 stuck ids, and the synthesized summary reports
   `subagents_unresponsive`.
7. Add a stream-idle fixture with a tool call and no result; assert
   `stream_idle_timeout`.

**Parallelizable.**

- The stream-idle fixture can be authored independently from the subagent
  fixture after Phase 4 exposes deterministic low-threshold config.

**Validation checkpoint.**

- `cargo test -p claudine-cli wrap_commands -- --nocapture`
- New fixture tests pass with low test-only thresholds.
- Existing compose / inline-compose / sequence wrapper tests still pass.

## Phase 6 - Docs and Final Regression

**Goal.** Document the new behavior and run the highest-signal regression set
for the affected packages.

**Files touched.**

- `claudine/docs/topics/composition.md`
- `claudine/cli/README.md`
- `.claude/skills/claudine/SKILL.md`

**Steps.**

1. Document the new environment knobs, defaults, and `0` disable behavior in
   the composition/wrapper docs.
2. Mention the OpenCode stability behavior in the CLI README where wrapper
   non-interactive execution is described.
3. Update the Claudine skill with the new watchdog contract, exit reasons, and
   troubleshooting guidance.
4. Run targeted tests:
   - `cargo test -p claudine-cli subagent_watchdog`
   - `cargo test -p claudine-cli wrap::exec`
   - `cargo test -p claudine-cli live_semantic_sink`
   - `cargo test -p claudine-cli wrap_commands`
5. Run package-level checks:
   - `cargo test -p claudine`
   - `cargo test -p claudine-cli`
   - `cargo clippy -p claudine-cli -- -D warnings`
6. Manual smoke with low thresholds:
   - Launch a controlled fake OpenCode stream that emits an outstanding
     subagent and then sleeps.
   - Confirm stderr shows the awaiting diagnostic first, then the Agent Error
     block, then a final trailer with `exit reason: subagents_unresponsive`.

**Validation checkpoint.**

- All targeted and package-level tests pass.
- Docs and skill updates describe exactly the shipped env vars and exit
  reasons.
- No unrelated workspace files are modified.
