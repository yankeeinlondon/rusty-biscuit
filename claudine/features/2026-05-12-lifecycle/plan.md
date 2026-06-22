---
agent: "codex/"
phases: 8
created: 2026-06-22
start_phase: 1
yolo: "true"
---

# Lifecycle Formalization Execution Plan

Success means Claudine parses and dispatches all seven composition lifecycle events, supports top-level and stack-based actions with static validation, preserves existing prompt behavior, and proves the new runtime ordering through focused L1 and L2 tests.

## Phase 1 - Baseline Orientation and Contracts

- [ ] Confirm current workspace package names with `cargo metadata --no-deps --format-version 1` and note the relevant crates (`claudine`, `claudine-cli`, and any Darkmatter APIs already exposed).
- [ ] Read the current lifecycle, loop, prepare, harness, and composition CLI integration points: `claudine/lib/src/composition/lifecycle.rs`, `loop_config.rs`, `loop_engine.rs`, `prepare.rs`, `types.rs`, and `claudine/cli/src/commands/wrap/composition/`.
- [ ] Record the existing behavior of top-level `start`/`success`/`blocked`/`failure` notifications with a small test fixture before changing code.
- [ ] Identify existing expression parsing and expression walking APIs in Darkmatter that can parse `when:` clauses, short-form arguments, and static scans without adding a new parser.
- [ ] Identify existing Darkmatter side-effect descriptor and execution APIs under `darkmatter/lib/src/effects/` and decide the minimal adapter Claudine needs for lifecycle stack invocation.
- [ ] Inventory every current shell-audit input surface and add a note for the new lifecycle stack shell command surface.
- [ ] Validation checkpoint: write down the exact touched module list and confirm no planned task requires retiring the harness DSL beyond the spec's narrow definition of lifecycle pre-flight.

Parallelizable after this phase: Phase 2 parser/model work and Phase 6 documentation planning can start once module boundaries are confirmed.

## Phase 2 - Lifecycle Data Model and Parse-Time Validation

- [ ] Extend `LifecycleSignal` with `Initialize`, `Finalize`, and `Loop`, including canonical names, display/status behavior, deterministic ordering, and `LifecycleConfig::get`.
- [ ] Add optional `initialize`, `finalize`, and `loop` lifecycle fields to `LifecycleConfig` while keeping existing `start`, `success`, `blocked`, and `failure` behavior unchanged.
- [ ] Extend `LifecycleNotification` with `info`, `warn`, and `stack`; keep `say` and `say_first` mutually exclusive for every event.
- [ ] Extend `LoopConfig` or add an adjacent parsed type so the `loop` frontmatter block carries both existing iteration controls and standard lifecycle concerns.
- [ ] Add typed action model types: `LifecycleStackItem`, `LifecycleActionRef`, lifecycle control actions, communication actions, shell actions, side-effect actions, and expression-function actions.
- [ ] Implement short-form action parsing as `verb(args)` using Darkmatter expressions for arguments, including parse-time rejection of unquoted multi-word literals.
- [ ] Implement long-form action parsing for communication actions, lifecycle control actions, shell actions, and side effects.
- [ ] Enforce the per-stack-item cardinality rule at parse time: at most one lifecycle control action, and it must be the last action in the item.
- [ ] Enforce the "Where valid" matrix at parse time for `Skip`, `Proxy`, `Retry`, `Resume`, and `Requeue`.
- [ ] Add typed `CompositionError` variants and `BlockError` renderings for invalid short-form syntax, invalid action placement, multiple lifecycle actions, invalid lifecycle action order, invalid lifecycle args, and `err` misuse.
- [ ] Reject accidental `stdout` fields or `stdout(...)` actions as typed lifecycle parse errors with frontmatter excerpts when rendered.
- [ ] Validation checkpoint: L1 parser tests prove all seven event blocks parse, legacy top-level-only prompts still parse, invalid stack shapes fail at parse time, and unknown fields remain rejected.

Parallelizable: error rendering tests can be implemented alongside the parser once the new `CompositionError` variants are defined.

## Phase 3 - Lifecycle Context, Static Scans, and Shell Audit

- [ ] Define the lifecycle-stack-only globals `err`, `timing`, and `current` in the lifecycle execution context without changing body or frontmatter interpolation semantics.
- [ ] Implement `err` value construction for `ClaudineError`, `HarnessError`, and `CompositionError` with `kind`, `variant`, and `msg` fields.
- [ ] Implement `timing` values for document duration, total duration, and sequence-step timing using the best existing timing data without introducing overflow-prone public commitments beyond the spec.
- [ ] Implement `current.ctx` and `current.env` lazy capture at event execution time using existing context/env capture APIs.
- [ ] Extend lifecycle interpolation leak scanning to include `initialize`, `finalize`, `loop`, top-level `info`/`warn`, and all stack expression/message surfaces.
- [ ] Extend undefined lifecycle variable validation to include all new events and stack expression/message surfaces while preserving the `ctx.*`, `env.*`, and `doc.*` exemptions.
- [ ] Add the parse-time static scan that forbids global `err` in `initialize`, `start`, `success`, and `loop`, while allowing `doc.err` everywhere and allowing `err` in `blocked`, `failure`, and `finalize`.
- [ ] Ensure bare `err`, `timing`, and `current` outside lifecycle stacks continue to resolve as ordinary identifiers through existing Darkmatter handling.
- [ ] Collect shell commands from every reachable lifecycle stack and include them in the existing pre-flight shell whitelist audit before any provider invocation.
- [ ] Extend `no_error: true` parsing to every action category and thread it into stack execution metadata.
- [ ] Validation checkpoint: L1 tests cover `err` misuse, `doc.err` exemption, stack leak/undefined scans, no special body/frontmatter behavior for lifecycle globals, and shell-audit collection for stack shell commands.

Parallelizable: timing/current context construction can proceed in parallel with static scan tests after the execution context shape is agreed.

## Phase 4 - Stack Execution Engine

- [ ] Implement ordered stack processing for each event: top-level communication first, then stack items top to bottom.
- [ ] Evaluate each stack item's `when:` expression against the lifecycle context; omitted `when` evaluates as true.
- [ ] Execute scalar and array `action:` forms in order, stopping the current event immediately after a lifecycle control action.
- [ ] Implement communication actions `say`/`speak`, `effect`, `message`, `notify`, `stderr`, `info`, and `warn` through existing emitter routes.
- [ ] Render `stderr`, `info`, and `warn` through `biscuit-terminal` `TerminalRenderable` components and keep lifecycle output off stdout.
- [ ] Invoke Darkmatter side effects by name through the existing side-effect system, preserving file-reference semantics and typed errors.
- [ ] Invoke read-only expression-function actions and log or surface their result according to the existing lifecycle/status style.
- [ ] Invoke shell actions through the approved shell execution path with `command`, `on_error`, and `no_error` support.
- [ ] Implement unintentional action error propagation by event: setup-phase errors route to failure, terminal-phase errors log without changing the composition outcome, and `no_error: true` logs and continues.
- [ ] Implement explicit `Error` lifecycle action semantics separately from unintentional action errors, including success-to-failure conversion at `success` and `finalize`.
- [ ] Validation checkpoint: L1 tests cover top-level-before-stack ordering, `when` true/false behavior, communication rendering paths, `no_error`, action error propagation, and explicit `Error` transitions.

Parallelizable: communication-action tests and side-effect/shell-action tests can run in separate branches after the shared stack executor API exists.

## Phase 5 - Runtime Flow Integration

- [ ] Insert `initialize` after prompt identification/frontmatter parse and CLI/frontmatter override merge, before user `$schema` validation and shell pre-flight.
- [ ] Route `Skip` at `initialize` to a clean whole-document opt-out: no pre-flight, no provider invocation, no `finalize`, no `loop`, and sequence advances to the next step.
- [ ] Route `Proxy` at `initialize`, `blocked`, and `failure` to a fresh target prompt run that enters at the target's `initialize`.
- [ ] Route `Error` at `initialize` and `start` through `failure`, then `finalize`, then optional loop gate.
- [ ] Ensure `start` fires only after `$schema` validation and lifecycle shell audit pass and immediately before provider invocation.
- [ ] Ensure schema-validation failures and lifecycle shell-audit denials produce `blocked`, not provider invocation.
- [ ] Ensure `blocked`, `success`, and `failure` each route to `finalize` exactly once per iteration.
- [ ] Implement `Retry` from `blocked` by rerunning pre-flight/start flow and from `failure` by rerunning the provider invocation path with max attempts, delay, and backoff.
- [ ] Implement `Resume` from `failure` using the existing provider session resume support and validate required message/provider capabilities.
- [ ] Implement `Requeue` from `blocked` and `failure`; if the `rendezvous` queue integration is not present or not ready, add a typed unsupported error rather than a silent no-op.
- [ ] Preserve existing `LifecycleRunGuard` safety behavior or replace it with an equivalent state machine that cannot double-emit terminal/finalize events.
- [ ] Validation checkpoint: L2 tests prove event order for success, failure, blocked, skip, proxy, retry, resume, and requeue paths, including no provider launch for blocked/skip.

Dependency note: this phase depends on Phases 2-4. Do not start runtime control-flow changes before parse-time validation and stack execution semantics are covered by L1 tests.

## Phase 6 - Loop Gate Integration

- [ ] Merge lifecycle concerns into `loop` parsing while preserving existing `while`/`until`/`action`/`actions`/`max`/`fail_fast`/`on_rate_limit` iteration controls.
- [ ] Change loop execution so the first iteration runs `initialize` once, and later iterations re-enter at `start` without rerunning `initialize`, schema validation, or shell pre-flight.
- [ ] Ensure `success`, `failure`, and `finalize` fire once per loop iteration.
- [ ] Move the loop condition check to the post-`finalize` gate required by the spec.
- [ ] At the loop gate, execute lifecycle concerns first against pre-mutation frontmatter, then evaluate `while`/`until`, then apply per-iteration mutations only when continuing.
- [ ] Ensure loop lifecycle concerns fire on every gate pass, including the terminal pass that exits.
- [ ] Preserve `_loop_count`, `_loop_is_first`, `_loop_is_last`, `_loop_last_output`, and `_loop_last_exit_code` semantics in `when:` clauses and loop conditions.
- [ ] Enforce `fail_fast: true` so unrecovered `blocked`/`failure` iterations still emit `finalize` but exit before the loop gate.
- [ ] Enforce `fail_fast: false` so failed iterations can reach the loop gate after terminal handling.
- [ ] Handle blocked first iteration as `blocked` -> `finalize` -> exit for unrecovered `fail_fast: true`, or `blocked` -> `finalize` -> `loop` for recovered or `fail_fast: false`.
- [ ] Validation checkpoint: L1 and L2 loop tests prove re-entry at `start`, no repeated initialize/pre-flight/schema, exact `finalize` counts, concerns-before-condition-before-mutation ordering, and fail-fast behavior.

Parallelizable: loop parser tests can start during Phase 2; loop runtime tests depend on Phase 5.

## Phase 7 - Backward Compatibility, Documentation, and UX Polish

- [ ] Verify existing top-level-only lifecycle prompts behave byte-for-byte or behavior-for-behavior the same for `start`, `success`, `blocked`, and `failure`.
- [ ] Update rustdoc in changed lifecycle, loop, and error types; remove or correct comments that still describe only four lifecycle events.
- [ ] Update Claudine topic docs that describe composition lifecycle behavior, loop behavior, and lifecycle output channels.
- [ ] Update `.claude/skills/claudine/SKILL.md` if implementation changes the architecture, commands, event inventory, or workflow guidance.
- [ ] Update `claudine context --side-effects` references only if the rendered catalog or lifecycle docs need new wording; do not duplicate the Darkmatter side-effect catalog manually.
- [ ] Add examples for `initialize`, `finalize`, `loop` lifecycle concerns, short-form expression arguments, `err` usage, `doc.err` escape hatch, and `no_error`.
- [ ] Ensure all user-facing text uses US English and avoids introducing a lifecycle `stdout` channel.
- [ ] Validation checkpoint: docs and rustdoc match implemented behavior, and no stale comments claim `loop` condition evaluation happens before lifecycle concerns.

Parallelizable: documentation updates can proceed after Phase 2 model names stabilize, but final wording should wait until runtime behavior is verified.

## Phase 8 - Final Verification and Release Readiness

- [ ] Run focused L1 tests for lifecycle parsing, validation, stack execution, loop parsing, loop execution, and error rendering.
- [ ] Run `just test` in the `claudine` package area and address failures without broad formatting churn.
- [ ] Run `just test-l2` in the `claudine` package area if integration fixtures and local environment support it.
- [ ] Run `just lint` in the `claudine` package area and address warnings relevant to the changed code.
- [ ] Run a manual dry-run or fixture-backed `claudine compose` check for a legacy prompt with only top-level lifecycle properties.
- [ ] Run a manual dry-run or fixture-backed `claudine compose` check for a prompt using `initialize`, `start.stack`, `success`, `finalize`, and `loop` concerns.
- [ ] Confirm no lifecycle side effect writes to stdout by piping compose output and checking lifecycle chatter stays on stderr.
- [ ] Confirm frontmatter excerpts still render for lifecycle parse errors in TTY/forced-color paths and remain suppressed in non-TTY/no-color paths.
- [ ] Review `git diff` for accidental unrelated refactors, formatting-only churn, or comment drift.
- [ ] Validation checkpoint: every acceptance criterion in the spec is mapped to a passing L1/L2/manual check or explicitly documented as blocked with a concrete reason.
