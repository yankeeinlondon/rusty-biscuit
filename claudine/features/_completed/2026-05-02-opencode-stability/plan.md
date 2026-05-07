---
phases: 6
created: 2026-05-03
updated: 2026-05-03
start_phase: 1
spec: claudine/features/2026-05-02-opencode-stability/spec.md
owner: claudine
packages:
  - claudine
  - claudine-cli
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages_during_phase_1:
  - claudine-cli
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/lib/src/stream/logs/opencode.rs
  - claudine/lib/src/stream/logs/mod.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - claudine
  - claudine-cli
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/composition.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4:
  - claudine/docs/topics/composition.md
  - claudine/cli/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/SKILL.md
packages_during_phase_4:
  - claudine-cli
source_files_during_phase_5:
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine-cli
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/composition.md
  - claudine/cli/README.md
docs_created_during_phase_6:
  - claudine/docs/topics/timeouts.md
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/SKILL.md
packages_during_phase_6:
  - claudine
  - claudine-cli
---

# Plan - Unified Timeout Enforcement

Source spec: [`spec.md`](./spec.md).

## Summary

OpenCode can hang during non-interactive `compose` / `inline-compose` /
`sequence` runs when parallel `task` subagents go silent without a matching
completion event. Claudine currently waits only for child process exit and
stdout EOF, so it can deadlock with no user-visible diagnostic.

The fix introduces a single, unified timeout contract: two timeouts
(`timeout` and `step_timeout`) sourced from CLI flags, markdown frontmatter,
or env-var defaults — with **no parallel watchdog-only thresholds**. The
watchdog tracks active subagents only for diagnostic enrichment in error
reports; all silence kills resolve to a single `step_timeout` exit reason.
The work should preserve normal long-running tool calls: only complete
parent-stream silence past the (large) default ceiling triggers a kill.

## Vocabulary

The single source of truth for timeout names is the markdown frontmatter
contract on `HarnessPlan`:

| Name | What it measures | Built-in default |
|---|---|---|
| `timeout` | Wall-clock from child spawn | none (opt-in) |
| `step_timeout` | Silence since last parent-stream event | `30m` |

The same names apply to:

- markdown frontmatter (`timeout:`, `step_timeout:`)
- CLI flags (`--timeout`, `--step-timeout`)
- env-var defaults (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`)

Resolution precedence: CLI flag > frontmatter > env-var default >
built-in default. All values use the existing
`claudine::harness::parse_timeout` grammar (`30s`, `5m`, `2h`).
Setting an env var to `0s` disables the rule.

Supporting knobs (not user-facing timeouts):

| Env var | Default | Purpose |
|---|---|---|
| `CLAUDINE_KILL_GRACE` | `10s` | SIGTERM → SIGKILL grace |
| `CLAUDINE_WATCHDOG_INTERVAL` | `5s` | internal ticker cadence |

## Dependency Map

| Phase | Outcome | Depends on |
|---|---|---|
| 1 | Pure `WatchdogState` and `TimeoutConfig` exist and are unit-tested | none |
| 2 | `LiveSemanticSink` updates and exposes shared subagent state for diagnostics | 1 |
| 3 | Exec wait loop can be terminated by the timeout watchdog and can mark exit reasons | 1 |
| 4 | `timeout` and `step_timeout` rules are wired end to end with full source-precedence resolution | 2, 3 |
| 5 | Idle diagnostics and fixture replay acceptance tests are covered | 2, 4 |
| 6 | Documentation (including new `docs/topics/timeouts.md`), skill updates, and final regression sweep land | 1-5 |

## Phase 1 - State Model and Config

**Goal.** Add small, testable components: a `WatchdogState` that tracks
active subagents (for diagnostics), and a `TimeoutConfig` that resolves
`timeout` and `step_timeout` from the precedence chain.

**Files touched.**

- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

**Steps.**

1. In `subagent_watchdog.rs`, define `SubagentId`, `ActiveSubagentInfo`,
   `ActiveSubagentSnapshot`, `SubagentDiagnosticLine`, and `WatchdogState`.
2. Store, at minimum, subagent id, optional name/title, `started_at`,
   `last_progress_at`, and the last diagnostic emission time.
3. Add methods:
   - `subagent_started(id, name, now)`
   - `subagent_stopped(id, now)`
   - `observe_subagent_progress(id, now)`
   - `active_subagents(now) -> Vec<ActiveSubagentSnapshot>`
   - `outstanding_at_breach(now) -> Vec<ActiveSubagentSnapshot>` (snapshot
     for diagnostic enrichment when the timeout fires)
   - `diagnostic_lines(now, silence_window) -> Vec<SubagentDiagnosticLine>`
4. Replace the previous two-threshold `WatchdogConfig` with a new
   `TimeoutConfig`:
   ```rust
   pub(crate) struct TimeoutConfig {
       pub(crate) timeout: Option<Duration>,        // None = no wall-clock kill
       pub(crate) step_timeout: Option<Duration>,   // None = no silence kill
       pub(crate) kill_grace: Duration,             // default 10s
       pub(crate) interval: Duration,               // default 5s
   }
   ```
5. Add `TimeoutConfig::resolve` taking the resolved values for `timeout`
   and `step_timeout` (already chosen by the composition layer in Phase 4)
   plus parsed env-var overrides for `CLAUDINE_KILL_GRACE` and
   `CLAUDINE_WATCHDOG_INTERVAL`. The function MUST NOT itself consult env
   vars for `timeout` / `step_timeout` — those are resolved in
   `composition.rs` so all three sources (CLI, frontmatter, env) flow
   through one path.
6. Keep `WatchdogState` generic over `Instant` passed by callers; do not
   call `Instant::now()` inside mutation methods except in thin
   convenience wrappers.
7. Register the module in `wrap/mod.rs` only after the new file compiles.

**Parallelizable.**

- Unit tests for `WatchdogState` and `TimeoutConfig` can be written in
  parallel with the model implementation.

**Validation checkpoint.**

- `cargo test -p claudine-cli subagent_watchdog`
- Tests cover N starts and M stops, duplicate starts updating metadata
  without losing timestamps unexpectedly, progress resetting only the
  matching subagent, and `TimeoutConfig` honouring its inputs.

## Phase 2 - Sink Integration

**Goal.** Feed `WatchdogState` from the same semantic event stream already
used for live rendering and metrics.

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
   - Any event carrying a recognized subagent id updates
     `last_progress_at`.
4. Implement a narrow helper to extract subagent ids from semantic events:
   start/stop ids directly, plus OpenCode `task_progress` style data from
   `Info.payload` or `extra` when present. Leave unknown events untouched.
5. Add a read-only sink accessor returning a snapshot suitable for
   diagnostics without holding the sink mutex during writes.
6. Keep existing `SubagentStart` / `SubagentStop` visible rendering
   unchanged.

**Parallelizable.**

- Event-id extraction tests can be built independently from the sink
  field plumbing once the helper signature is decided.

**Validation checkpoint.**

- `cargo test -p claudine-cli live_semantic_sink::tests::subagent`
- Existing `SubagentStart` / `SubagentStop` render tests still pass.
- New tests assert active entries are inserted and removed through
  `on_semantic_event`, and that OpenCode progress payloads reset
  `last_progress_at` for the matching id.

## Phase 3 - Termination Plumbing

**Goal.** Give the watchdog a single, deterministic way to request
child-process termination and to mark the synthesized summary with a
distinct exit reason.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`

**Steps.**

1. Extend the existing early-termination model in `exec.rs` with two
   reasons (and only these two):
   - `Timeout { message }` — wall-clock breach
   - `StepTimeout { message, outstanding: Vec<ActiveSubagentSnapshot> }`
     — stream-silence breach; the `outstanding` field carries the
     subagent diagnostic enrichment described in the spec.
2. Add a `WatchdogTermination` request type carrying the reason and the
   rendered message.
3. Thread a termination channel into
   `wait_with_signal_and_early_termination`; when a watchdog request
   arrives, reuse the existing SIGTERM-to-process-group, grace wait, and
   SIGKILL escalation logic instead of adding a second signal path. The
   grace duration is `TimeoutConfig.kill_grace`.
4. Ensure the non-advanced wait path is promoted to the advanced wait path
   whenever any timeout rule is enabled, even if the existing OpenCode
   stderr early-termination is absent.
5. Add `apply_early_termination_to_summary` handling for:
   - exit reason `timeout`
   - exit reason `step_timeout`
6. Delete the previous `subagents_unresponsive` and `stream_idle_timeout`
   exit reasons and any code paths that emit them; the unified design
   uses `step_timeout` for any silence kill, with stuck-subagent detail
   surfaced in the rendered error block, not in the exit reason.
7. Preserve the existing user SIGINT behavior and `exit_code: 130`
   reporting.

**Parallelizable.**

- Summary override tests can be written while the channel plumbing is in
  progress, as long as they target `apply_early_termination_to_summary`
  directly.

**Validation checkpoint.**

- `cargo test -p claudine-cli wrap::exec`
- Unit tests prove watchdog termination maps to the new exit reasons and
  user interrupt still maps to the existing SIGINT semantics.
- `cargo build -p claudine-cli`

## Phase 4 - Watchdog Rules and Source-Precedence Resolution

**Goal.** Run the two kill rules (`timeout`, `step_timeout`) from a
dedicated ticker, and resolve their values through the full precedence
chain before launch.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

**Steps.**

1. In `composition.rs`, add a single `resolve_timeouts` helper that takes:
   - the parsed `--timeout` / `--step-timeout` CLI flag values (already
     present on `SharedComposeArgs::step_timeout_secs`; add `timeout_secs`
     symmetrically),
   - the `HarnessPlan.timeout` / `HarnessPlan.step_timeout` from
     frontmatter,
   - the `CLAUDINE_TIMEOUT` / `CLAUDINE_STEP_TIMEOUT` env vars,
   - and the built-in defaults (`timeout = None`, `step_timeout = 30m`).

   Resolution: CLI > frontmatter > env > built-in. Env values use the
   `parse_timeout` grammar; an env value of `0s` MUST disable the rule
   for this run (resolved value `None`).
2. Parse `CLAUDINE_KILL_GRACE` (default `10s`) and
   `CLAUDINE_WATCHDOG_INTERVAL` (default `5s`) into `TimeoutConfig` at the
   same point.
3. Pass the resolved `TimeoutConfig` into the wrapper (`run_child_stream_semantic`
   call site) instead of having the watchdog read env vars on its own.
4. Add `spawn_timeout_watchdog_ticker` next to `spawn_flush_if_idle_ticker`;
   keep cadences and side effects separate.
5. On each tick, evaluate:
   - **Wall-clock rule.** If `timeout` is set and
     `now - started_at >= timeout`, fire `Timeout`.
   - **Silence rule.** If `step_timeout` is set and at least one activity
     event has been observed and `now - last_event_at >= step_timeout`,
     fire `StepTimeout` with `outstanding =
     watchdog_state.active_subagents(now)` for diagnostic enrichment.
6. Render the first breach as a `SemanticEvent::Error` with
   `SemanticErrorKind::AgentNative` so the live sink's coloured
   `BlockQuote` contract is preserved. The error block for `step_timeout`
   MUST list any outstanding subagents from `outstanding` (id, name,
   elapsed since last progress).
7. Stop and join the watchdog ticker after the wait loop returns, mirroring
   `stop_timing_ticker(flush_ticker)`.
8. Guard against double-fire with an atomic `fired` flag covering both
   rules.
9. **Remove** the previous `CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS`,
   `CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS`, `CLAUDINE_SUBAGENT_WATCHDOG_INTERVAL_SECONDS`,
   and `CLAUDINE_STREAM_IDLE_KILL_SECONDS` env vars and any code that
   reads them. Search the workspace and delete every reference, including
   tests and docs strings; replace with the new vocabulary throughout.

**Parallelizable.**

- Env parsing tests, `resolve_timeouts` precedence tests, and rule-priority
  tests can run in parallel with ticker wiring because they live on pure
  helpers.

**Validation checkpoint.**

- `cargo test -p claudine-cli subagent_watchdog`
- `cargo test -p claudine-cli wrap::exec::tests::watchdog`
- `cargo test -p claudine-cli composition::tests::resolve_timeouts`
- Tests cover: each precedence rung; disabled (`None`) rules; wall-clock
  breach; silence breach; one-shot firing; `0s` env override disables.
- `rg "SUBAGENT_IDLE_KILL|SUBAGENT_KILL_GRACE|SUBAGENT_WATCHDOG_INTERVAL|STREAM_IDLE_KILL"`
  returns no matches anywhere in the workspace.

## Phase 5 - Diagnostics and Fixture Acceptance

**Goal.** Make stuck sessions diagnosable before termination and prove the
reference hang class terminates with the expected stderr and summary.

**Files touched.**

- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/subagent_watchdog.rs`
- `claudine/cli/src/commands/wrap/section.rs`
- `claudine/cli/tests/wrap_commands.rs`

**Steps.**

1. Extend the existing `flush_if_idle` ticker callback to consult the
   shared watchdog snapshot after flushing buffered prose.
2. Emit at most one diagnostic line per active subagent per silence
   window:
   ` ⏳ Awaiting subagent: <name-or-id> (<elapsed-since-start>)`.
3. Route diagnostic lines through `SectionTracker` and the Tool Use &
   Events section so spacing stays consistent with the current live
   surface.
4. Gate diagnostics on `step_timeout.is_some()` — if the user disabled
   the silence rule, the idle diagnostic is also suppressed.
5. Add an end-to-end fixture replay for the reference shape: 9
   `task_started`, 7 `task_completed`, then no further stream lines.
6. Assert the run terminates after the configured `step_timeout` plus
   interval and grace, stderr names the 2 stuck ids in the error block,
   and the synthesized summary reports `step_timeout`.
7. Add a wall-clock fixture: a stream that keeps emitting events but runs
   past `timeout`; assert termination with exit reason `timeout`.

**Parallelizable.**

- The wall-clock fixture can be authored independently from the silence
  fixture after Phase 4 exposes deterministic low-threshold config.

**Validation checkpoint.**

- `cargo test -p claudine-cli wrap_commands -- --nocapture`
- New fixture tests pass with low test-only thresholds.
- Existing compose / inline-compose / sequence wrapper tests still pass.

## Phase 6 - Docs, Topic Page, Skill Update, and Final Regression

**Goal.** Document the new behavior in a single authoritative topic page,
update adjacent docs that mention timeouts, refresh the Claudine skill,
and run the highest-signal regression set.

**Files touched.**

- `claudine/docs/topics/timeouts.md` *(new)*
- `claudine/docs/topics/composition.md`
- `claudine/cli/README.md`
- `.claude/skills/claudine/SKILL.md`

**Steps.**

1. **Create `claudine/docs/topics/timeouts.md`** as the comprehensive
   reference. The page MUST cover, in this order:
   - **Overview.** Two timeouts, one termination path. Why we have only
     two types.
   - **`timeout` (wall-clock).** Definition, what resets it (nothing),
     the formula `now - started_at >= timeout`, a worked example with
     timestamps, exit reason `timeout`.
   - **`step_timeout` (stream-silence).** Definition, what resets it
     (every parent-stream event including subagent progress), the
     formula `now - last_event_at >= step_timeout`, a worked example
     with timestamps, exit reason `step_timeout`.
   - **Configuration sources.** Precedence table (CLI > frontmatter >
     env > built-in default). Worked example showing the same prompt
     overridden at each layer.
   - **Configuration knobs.** Table of `CLAUDINE_TIMEOUT`,
     `CLAUDINE_STEP_TIMEOUT`, `CLAUDINE_KILL_GRACE`,
     `CLAUDINE_WATCHDOG_INTERVAL` with defaults and grammar reference.
   - **Defaults and rationale.** Why `timeout` has no default and
     `step_timeout` defaults to `30m`.
   - **Frontmatter syntax.** Examples of `timeout: 2h` /
     `step_timeout: 30m` in markdown frontmatter; the warning variants
     (`timeout_warn`, `step_timeout_warn`) and how they relate.
   - **Termination path.** SIGTERM → `kill_grace` → SIGKILL; how
     watchdog termination cooperates with `wait_with_signal_handling`;
     the synthesised `session_end` exit reasons.
   - **Subagent diagnostics in error reports.** What the operator sees
     in the rendered error block when `step_timeout` fires with
     outstanding subagents.
   - **Disabling.** How to disable each rule (omit from frontmatter,
     export `CLAUDINE_TIMEOUT=0s` / `CLAUDINE_STEP_TIMEOUT=0s`).
   - **Worked example: the OpenCode hang class.** How the unified
     `step_timeout` catches the reference incident, with example stderr.

   The topic page MUST be the **single canonical reference** for
   timeouts; other docs link here rather than duplicating definitions.
2. Update `claudine/docs/topics/composition.md` to reference
   `topics/timeouts.md` for full details and remove any duplicated env-var
   tables.
3. Mention the unified timeout behavior in `claudine/cli/README.md` where
   wrapper non-interactive execution is described, with a one-paragraph
   summary and a link to `topics/timeouts.md`.
4. Update `.claude/skills/claudine/SKILL.md` with the unified timeout
   contract:
   - exactly two timeouts (`timeout`, `step_timeout`),
   - the four env vars (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`,
     `CLAUDINE_KILL_GRACE`, `CLAUDINE_WATCHDOG_INTERVAL`),
   - exit reasons `timeout` and `step_timeout`,
   - the precedence chain CLI > frontmatter > env > built-in,
   - removal of any reference to the old `CLAUDINE_SUBAGENT_*` /
     `CLAUDINE_STREAM_IDLE_*` vocabulary.
5. Run targeted tests:
   - `cargo test -p claudine-cli subagent_watchdog`
   - `cargo test -p claudine-cli wrap::exec`
   - `cargo test -p claudine-cli live_semantic_sink`
   - `cargo test -p claudine-cli wrap_commands`
   - `cargo test -p claudine-cli composition`
6. Run package-level checks:
   - `cargo test -p claudine`
   - `cargo test -p claudine-cli`
   - `cargo clippy -p claudine-cli -- -D warnings`
7. Manual smoke with low thresholds:
   - Launch a controlled fake OpenCode stream that emits an outstanding
     subagent and then sleeps.
   - Confirm stderr shows the awaiting diagnostic first, then the
     `Step Timeout` block (naming the stuck subagent), then a final
     trailer with `exit reason: step_timeout`.

**Validation checkpoint.**

- All targeted and package-level tests pass.
- `claudine/docs/topics/timeouts.md` exists and contains the sections
  enumerated above.
- Docs and skill updates describe exactly the shipped env vars and exit
  reasons; no stale references to the old vocabulary remain.
- No unrelated workspace files are modified.
