---
phases: 5
created: 2026-06-05
start_phase: 2
spec: "features/2026-06-05-pid/spec.md"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - "claudine/features/2026-06-05-pid/plan.md"
docs_created_during_phase_1:
  - "claudine/features/2026-06-05-pid/phase-1-implementation-note.md"
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - "claudine/cli/src/commands/wrap/env.rs"
  - "claudine/cli/src/commands/wrap/exec/mod.rs"
  - "claudine/cli/src/commands/wrap/exec/spawn.rs"
  - "claudine/cli/src/commands/wrap/exec/wiring.rs"
docs_updated_during_phase_2:
  - "claudine/features/2026-06-05-pid/plan.md"
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - "claudine/lib/src/events/environment.rs"
  - "claudine/lib/src/events/event_meta.rs"
  - "claudine/lib/src/dispatch/mod.rs"
  - "claudine/lib/src/stream/reporting.rs"
  - "claudine/lib/src/dispatch/expression.rs"
  - "claudine/lib/src/dispatch/loader.rs"
  - "claudine/lib/src/dispatch/matcher.rs"
  - "claudine/lib/src/dispatch/runner/mod.rs"
  - "claudine/lib/src/dispatch/runner/report.rs"
  - "claudine/lib/src/dispatch/template.rs"
  - "claudine/lib/src/reporting/mod.rs"
  - "claudine/cli/src/commands/wrap/policy.rs"
  - "claudine/cli/src/commands/wrap/composition/mod.rs"
  - "claudine/cli/src/commands/wrap/composition/structured.rs"
  - "claudine/cli/src/commands/wrap/composition/summary.rs"
  - "claudine/cli/src/commands/wrap/composition/inline_guards.rs"
  - "claudine/cli/src/commands/wrap/harness_orch.rs"
  - "claudine/cli/src/commands/wrap/mod.rs"
  - "claudine/cli/src/perf.rs"
  - "claudine/cli/src/commands/wrap/repo_home.rs"
docs_updated_during_phase_3:
  - "claudine/features/2026-06-05-pid/plan.md"
docs_created_during_phase_3:
  - "claudine/features/2026-06-05-pid/phase-3-implementation-note.md"
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - "claudine/lib/src/reporting/schema.rs"
  - "claudine/lib/src/reporting/ingest.rs"
  - "claudine/lib/src/reporting/types.rs"
  - "claudine/lib/src/reporting/queries/common.rs"
  - "claudine/lib/src/reporting/queries/sessions.rs"
  - "claudine/lib/src/reporting/metrics.rs"
  - "claudine/lib/src/reporting/mod.rs"
  - "claudine/cli/src/commands/logs/sessions.rs"
  - "claudine/cli/tests/wrap_commands.rs"
  - "claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap"
docs_updated_during_phase_4:
  - "claudine/features/2026-06-05-pid/plan.md"
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - "claudine/cli/tests/wrap_commands.rs"
  - "claudine/lib/src/dispatch/template.rs"
docs_updated_during_phase_5:
  - "claudine/features/2026-06-05-pid/plan.md"
  - "claudine/docs/topics/traces-and-logging.md"
  - "claudine/docs/topics/log-reporting.md"
  - "claudine/docs/topics/wrapped-execution-switches.md"
  - "claudine/cli/README.md"
  - "claudine/docs/topics/repo-isolation.md"
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - ".claude/skills/claudine/cli-reference.md"
source_code:
  - "claudine/cli/src/commands/wrap/env.rs"
  - "claudine/cli/src/commands/wrap/exec/mod.rs"
  - "claudine/cli/src/commands/wrap/exec/spawn.rs"
  - "claudine/cli/src/commands/wrap/exec/wiring.rs"
  - "claudine/lib/src/events/environment.rs"
  - "claudine/lib/src/events/event_meta.rs"
  - "claudine/lib/src/dispatch/mod.rs"
  - "claudine/lib/src/stream/reporting.rs"
  - "claudine/lib/src/dispatch/expression.rs"
  - "claudine/lib/src/dispatch/loader.rs"
  - "claudine/lib/src/dispatch/matcher.rs"
  - "claudine/lib/src/dispatch/runner/mod.rs"
  - "claudine/lib/src/dispatch/runner/report.rs"
  - "claudine/lib/src/dispatch/template.rs"
  - "claudine/lib/src/reporting/mod.rs"
  - "claudine/cli/src/commands/wrap/policy.rs"
  - "claudine/cli/src/commands/wrap/composition/mod.rs"
  - "claudine/cli/src/commands/wrap/composition/structured.rs"
  - "claudine/cli/src/commands/wrap/composition/summary.rs"
  - "claudine/cli/src/commands/wrap/composition/inline_guards.rs"
  - "claudine/cli/src/commands/wrap/harness_orch.rs"
  - "claudine/cli/src/commands/wrap/mod.rs"
  - "claudine/cli/src/perf.rs"
  - "claudine/cli/src/commands/wrap/repo_home.rs"
  - "claudine/lib/src/reporting/schema.rs"
  - "claudine/lib/src/reporting/ingest.rs"
  - "claudine/lib/src/reporting/types.rs"
  - "claudine/lib/src/reporting/queries/common.rs"
  - "claudine/lib/src/reporting/queries/sessions.rs"
  - "claudine/lib/src/reporting/metrics.rs"
  - "claudine/cli/src/commands/logs/sessions.rs"
  - "claudine/cli/tests/wrap_commands.rs"
  - "claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap"
documentation:
  - "claudine/features/2026-06-05-pid/plan.md"
  - "claudine/features/2026-06-05-pid/phase-1-implementation-note.md"
  - "claudine/features/2026-06-05-pid/phase-3-implementation-note.md"
  - "claudine/docs/topics/traces-and-logging.md"
  - "claudine/docs/topics/log-reporting.md"
  - "claudine/docs/topics/wrapped-execution-switches.md"
  - "claudine/cli/README.md"
  - "claudine/docs/topics/repo-isolation.md"
packages:
  - claudine
---

# PID Capture for Wrapped Agentic CLIs - Execution Plan

## Assumptions

- The implementation target is the Claudine wrapper path in `claudine-cli` and the shared event/reporting model in `claudine`.
- `CLAUDINE_PID` is known at wrapper startup and must be injected before every wrapped provider spawn.
- `AGENT_PID` is the immediate child PID returned by Rust's spawn operation; no provider-specific descendant lookup is required.
- Raw JSONL records should omit `agent_pid` when unavailable, while report/query DTOs and database outputs should expose nullable `agent_pid`.
- The requested closure path appears to duplicate `claudine`; this plan is saved beside `spec.md` as `claudine/features/_unscheduled/pid/plan.md`.

## Phase 1 - Current-State Mapping and PID Model

- [x] Inventory every wrapped-provider spawn path and record the exact owner function, including `claudine/cli/src/commands/wrap/exec/spawn.rs`, `claudine/cli/src/commands/wrap/exec/wiring.rs`, and any harness or composition wrapper path that can call them.
- [x] Inventory every Claudine-controlled context surface that serializes `EventMeta` or context extras, including dispatch JSONL logging, terminal reports, stream summary events, semantic event logging, hook/action contexts, and reporting ingest.
- [x] Decide the canonical in-memory representation for PIDs, preferring typed numeric fields over string-only context where the existing model supports it.
- [x] Define the propagation boundary between `EnvironmentContext`, `EventMeta`, `DispatchRuntimeContext`, and wrapper-only context extras so `claudine_pid` and `agent_pid` are available without duplicating provider stream records unnecessarily.
- [x] Document the expected unavailable-state behavior: `claudine_pid` should be present once captured; `agent_pid` should be absent from raw JSONL until a successful spawn and nullable in query/report outputs.
- [x] Validation checkpoint: produce a short implementation note, in code comments or the PR description, listing all spawn paths and all serialization/reporting surfaces that will be touched.

## Phase 2 - Wrapper Environment and Spawn Capture

- [x] Add `CLAUDINE_PID` to the child environment in `claudine/cli/src/commands/wrap/env.rs` before provider-specific env overrides are finalized, using the current Claudine process ID.
- [x] Add focused env-plan tests proving `CLAUDINE_PID` is present for interactive and non-interactive wrapper env construction.
- [x] Capture `child.id()` immediately after each successful provider `spawn()` and store it as `AGENT_PID` in the wrapper execution state for that run.
- [x] Thread the captured `agent_pid` through structured, legacy, Kimi wire, harness retry, and no-harness execution paths without making failed-spawn paths fabricate a value.
- [x] Ensure retry/harness attempts update `agent_pid` per successful child spawn rather than reusing a stale PID from a previous attempt.
- [x] Validation checkpoint: add spawn-path unit tests or a lightweight fake-provider integration test proving `CLAUDINE_PID` reaches the child process and the immediate child PID is captured after spawn.

## Phase 3 - Claudine-Controlled Context Propagation

- [x] Extend the shared event/context model with `claudine_pid` and optional `agent_pid` where Claudine-controlled records are built, likely in `EventMeta` and/or a dedicated wrapper PID context that is merged into `EventMeta`.
- [x] Update dispatch preparation so hook, action, log, and report contexts include `claudine_pid` when available and `agent_pid` only after spawn.
- [x] Update template, expression, and terminal report context generation so `CLAUDINE_PID`/`AGENT_PID` and `claudine_pid`/`agent_pid` resolve consistently where existing context variables are exposed.
- [x] Update stream summary event creation in `claudine/lib/src/stream/reporting.rs` so wrapper session lifecycle records include PID fields while ordinary provider stream records do not receive duplicate PID fields solely because they occurred during a wrapped session.
- [x] Update composition dispatch context builders to pass PID context through `compose`, `inline-compose`, `sequence`, harness attempts, and direct wrapper runs.
- [x] Parallelizable after the shared PID model is defined: update docs/comments for the modified context surfaces, deleting any drifted comments that describe the old context shape.
- [x] Validation checkpoint: add serialization tests showing raw JSONL omits unavailable `agent_pid`, includes `claudine_pid`, and includes `agent_pid` after a simulated successful spawn.

## Phase 4 - Reporting Schema, Ingest, Queries, and CLI Output

- [x] Bump the reporting schema version in `claudine/lib/src/reporting/schema.rs` and add nullable `agent_pid` and non-null-or-nullable `claudine_pid` columns according to the chosen backfill behavior.
- [x] Add migration logic that preserves derived-cache semantics by clearing ingestion state and indexed rows when PID columns are introduced.
- [x] Update `claudine/lib/src/reporting/ingest.rs` to read PID fields from top-level `EventMeta` fields or the agreed fallback location and write them into `events` and `sessions`.
- [x] Update session aggregation so `sessions.agent_pid` is nullable and reflects the spawned child PID when any lifecycle record in the session has one.
- [x] Update reporting DTOs in `claudine/lib/src/reporting/types.rs` and query mappers under `claudine/lib/src/reporting/queries/` to expose stable nullable `agent_pid` fields or columns.
- [x] Update CLI report renderers under `claudine/cli/src/commands/logs/` so text and JSON outputs handle `agent_pid: null` without hiding or mislabeling the field.
- [x] Parallelizable after schema columns are decided: update query-specific tests for sessions, errors, today/week/month, and sync reports that project event/session rows.
- [x] Validation checkpoint: run reporting schema migration tests and ingest/query tests against synthetic JSONL containing both missing and present `agent_pid`.

## Phase 5 - End-to-End Verification and Documentation

- [x] Add an end-to-end wrapper test with a fake provider command that prints its environment, proving the provider receives `CLAUDINE_PID` and is not required to receive `AGENT_PID`.
- [x] Add an end-to-end wrapped-session logging test proving lifecycle summary records include `claudine_pid` and include `agent_pid` after spawn.
- [x] Add a negative test for failed provider spawn proving no raw JSONL record fabricates `agent_pid`.
- [x] Add or update documentation for wrapper environment variables and reporting fields in the relevant Claudine docs, including the distinction between `AGENT_PID` and descendant provider PIDs.
- [x] Review comments on every touched symbol whose behavior changed, removing stale HOW-narration and updating only comments that carry real invariants.
- [x] Run targeted tests first: wrapper env/spawn tests, dispatch serialization tests, stream reporting tests, reporting schema tests, ingest tests, and affected logs query tests.
- [x] Run the package-area validation command for Claudine, preferring the existing package-area `just test` recipe or the narrowest `cargo test -p claudine -p claudine-cli` equivalent if the recipe is unavailable.
- [x] Final validation checkpoint: confirm all acceptance criteria from `spec.md` are covered by tests or documented manual verification, with explicit evidence for interactive and non-interactive wrapper modes.
