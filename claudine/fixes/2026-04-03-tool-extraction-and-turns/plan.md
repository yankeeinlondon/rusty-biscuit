# OpenCode Tool Extraction And Turn Semantics Plan

## Context

`claudine compose --opencode` is correctly wrapping OpenCode and streaming its NDJSON output, but the current coarse-event mapping creates two problems in long non-interactive sessions:

1. OpenCode tool metadata is often missing from Claudine dispatch because the parser only looks for top-level tool names.
2. OpenCode `step_start` / `step_finish` are currently mapped to full `before_prompt` / `turn_complete` lifecycle events, which causes repeated hook dispatch after the user-perceived task is already "done".

With `--debug trace`, this becomes especially obvious:

- repeated `before_prompt`, `before_tool`, and `turn_complete` dispatch cycles
- repeated config reloads
- repeated protect evaluations
- empty `tool_name=""` in logs even when a tool call happened

The timings in the trace suggest Claudine is not spinning in a CPU-heavy loop. Instead, OpenCode keeps emitting more streamed steps, and Claudine keeps treating those step boundaries as high-level turn boundaries.

## Problem Statement

### What the user observed

In a non-interactive compose run used for commit generation:

- the staged-files commit was already created
- the provider appeared functionally done from the user's perspective
- Claudine continued printing many trace lines for several more seconds before exiting

### Why the current behavior is confusing

Even if OpenCode is still technically running, Claudine currently amplifies those remaining provider steps into repeated high-level lifecycle traffic:

- every OpenCode `step_start` becomes `BeforePrompt`
- every OpenCode `step_finish` becomes `TurnComplete`
- every coarse event reloads runtime config and re-runs protect evaluation

That means a single OpenCode session can generate many "turn complete" moments, which is not what users expect from a one-shot non-interactive wrapper command.

### Required invariants

1. OpenCode tool events must include tool names whenever the provider emits them anywhere in the structured payload.
2. A non-interactive OpenCode session should not emit repeated high-level "turn complete" hook cycles for every internal step unless that is intentionally modeled and documented.
3. Claudine should avoid repeated config reload work within a single wrapped session when the runtime config inputs are stable.
4. Trace logs should make it obvious whether the provider is still actively producing work vs. Claudine performing local cleanup.

## Goals

1. Fix OpenCode tool-name extraction so dispatch and logging have real tool identities.
2. Reduce or eliminate false high-level turn boundaries caused by per-step mapping.
3. Reduce repetitive per-event config load overhead inside a single wrapper session.
4. Lock the behavior down with parser and wrapper regression tests.

## Non-Goals

- Redesigning the entire normalized event model
- Changing how non-OpenCode providers map their stream events
- Removing protect evaluation entirely
- Hiding all trace output
- Reworking commit prompt logic itself

## Root Cause Analysis

## 1. Tool name extraction is too shallow

In `claudine/lib/src/stream/opencode.rs`, `tool_use` / `tool_start` only read:

- `obj["name"]`
- `obj["tool_name"]`

If OpenCode places the tool identity under `part.name`, `part.tool_name`, or another nested field, Claudine emits `before_tool` with no `tool_name`.

Observed symptom:

- trace spans show `tool_name=""`

Impact:

- weak logs
- weak protect context
- matcher rules that depend on tool name may not work
- summary tool-name reporting is incomplete

## 2. Step boundaries are being treated as turn boundaries

In the current OpenCode parser:

- `step_start` calls `sink.on_turn_start(...)`
- `step_finish` calls `sink.on_turn_complete(...)`

In the live wrapper sink:

- `on_turn_start` maps to `AgenticEvent::BeforePrompt`
- `on_turn_complete` maps to `AgenticEvent::TurnComplete`

That means each streamed OpenCode step becomes a synthetic full turn lifecycle in Claudine, even for a single non-interactive prompt.

Observed symptom:

- many repeated `before_prompt` and `turn_complete` dispatch cycles after the user thought the job was finished

Impact:

- noisy traces
- repeated hook execution
- repeated protect evaluation
- user confusion about whether Claudine is stuck

## 3. Runtime config is reloaded on every dispatch

`dispatch_preparsed()` calls `loader::load_runtime_config(...)` for every incoming event.

This is individually cheap, but with repeated synthetic turn events it multiplies:

- config loads
- merges
- tracing noise

This is likely not the main wall-clock cost in the observed run, but it is part of the perceived "endless tail".

## Refactor Strategy

Treat OpenCode stream parsing as having two layers:

1. provider-step events
2. high-level session/turn lifecycle events

The wrapper should only emit high-level lifecycle events when they are semantically justified, not at every internal provider step.

Also, make OpenCode tool extraction robust to nested payload shape and cache resolved runtime config for the duration of a single wrapper session.

## Detailed Implementation Plan

## Phase 1: Fix OpenCode Tool Extraction

**Goal:** Always populate `tool_name` when OpenCode provides one.

### File

`claudine/lib/src/stream/opencode.rs`

### Changes

1. Add a helper:

```rust
fn opencode_tool_name(obj: &Value) -> Option<&str>
```

Search in this order:

- top-level `name`
- top-level `tool_name`
- `part.name`
- `part.tool_name`
- any other currently observed OpenCode field carrying the tool identifier

2. Update `tool_use` / `tool_start` handling to use that helper for:

- trace logging
- emitted `EventMeta.extra["tool_name"]`

3. If tool name is still missing, optionally trace a provider-specific diagnostic such as:

- `"OpenCode tool event missing tool name"`

### Tests

Add parser tests that cover:

- top-level tool name
- nested `part.name`
- nested `part.tool_name`
- missing tool name leaves behavior safe but visible

## Phase 2: Separate OpenCode Step Events From High-Level Turn Events

**Goal:** Stop mapping every OpenCode step to a full Claudine turn lifecycle.

### Files

- `claudine/lib/src/stream/parser.rs`
- `claudine/lib/src/stream/opencode.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

### Preferred design

Extend the stream sink contract with step-aware callbacks, for example:

```rust
fn on_step_start(&mut self, _meta: &EventMeta) {}
fn on_step_finish(&mut self, _meta: &EventMeta) {}
```

Then:

- OpenCode `step_start` maps to `on_step_start`, not `on_turn_start`
- OpenCode `step_finish` maps to `on_step_finish`, not `on_turn_complete`

The live wrapper sink can decide whether step events:

- are ignored for dispatch
- are summarized only
- or are mapped to a lower-noise provider-specific notification

### Scope decision

For this fix, the safest behavior is:

- keep `SessionStart`
- keep `BeforeTool` / `AfterTool`
- keep `TurnError`
- emit `TurnComplete` only once per actual session completion signal, not on every `step_finish`

### Key implementation options

#### Option A: Minimal-risk fix

- stop emitting `on_turn_start` from `step_start`
- stop emitting `on_turn_complete` from `step_finish`
- rely on `step_complete` / `turn_complete` if OpenCode emits a true session-end completion

This is recommended if current streams provide a reliable terminal completion event.

#### Option B: Add explicit step callbacks

- richer design
- more future-proof
- slightly wider refactor

Recommended answer:

- implement Option B if the code stays small and localized
- otherwise do Option A first and document follow-up work

### Tests

Add OpenCode parser and wrapper tests proving:

1. multiple `step_start` / `step_finish` pairs do not produce multiple `TurnComplete` dispatches
2. multiple internal steps still preserve text/tool accumulation
3. the final session summary still reports correct turns/tool counts/cost

## Phase 3: Cache Runtime Config For A Wrapped Session

**Goal:** Avoid reloading and merging user/repo config on every streamed event.

### Files

- `claudine/lib/src/dispatch/mod.rs`
- `claudine/lib/src/dispatch/loader.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

### Changes

Introduce a wrapper-session-scoped runtime config cache keyed by:

- provider
- repo root
- explicit config override path if any

Possible shapes:

```rust
pub struct DispatchRuntimeContext {
    pub config: Arc<RuntimeConfig>,
}
```

or a small memoized loader closure owned by the wrapper session.

### Important constraint

Do not change CLI semantics for separate invocations. The cache should live only for one wrapper process lifetime.

### Tests

Add focused tests to ensure:

- repeated dispatches within one wrapper session reuse cached config
- user/repo config precedence remains unchanged

If direct unit verification is awkward, at minimum add tracing or counters in tests proving the loader is invoked once for repeated events.

## Phase 4: Improve End-Of-Run Visibility

**Goal:** Make traces clearly show whether the provider is still active.

### Files

- `claudine/lib/src/stream/opencode.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

### Changes

1. Add a session-end or parser-finish trace that states:

- parser finished
- final tool count
- final turn count
- final provider status
- child exit code

2. Consider adding a trace for ignored step events if step callbacks are introduced.

3. Consider logging an explicit distinction between:

- provider emitted new stream line
- Claudine dispatch reused cached config

This is mainly diagnostics, but it will make future reports much easier to interpret.

## Acceptance Criteria

The fix is complete when all of the following are true:

1. OpenCode tool events populate `tool_name` whenever the provider emits one in supported payload shapes.
2. A non-interactive OpenCode compose run with many internal steps does not emit repeated high-level `turn_complete` hook cycles for each step.
3. Repeated streamed events in a single wrapper session do not reload runtime config every time.
4. Existing OpenCode wrapper behavior still works for:
   - prompt delivery
   - summary reporting
   - tool counting
   - protect evaluation
5. Trace output makes it clear when the child is still producing stream events vs. Claudine just finishing locally.

## Suggested Commit Sequence

### Commit 1

`fix(claudine): extract OpenCode tool names from nested stream payloads`

- add nested tool-name extraction
- add parser tests

### Commit 2

`fix(claudine): stop treating OpenCode step boundaries as full turns`

- refactor OpenCode stream event mapping
- add regression coverage for repeated steps

### Commit 3

`perf(claudine): cache runtime dispatch config within wrapper sessions`

- add session-scoped runtime config cache
- reduce repeated config load noise

### Commit 4

`test(claudine): lock OpenCode stream lifecycle semantics`

- add wrapper/integration regressions for multi-step OpenCode runs

## Verification Commands

Use focused checks while iterating:

```bash
cargo test -p claudine-cli compose_opencode_non_interactive_passes_prompt_as_positional_arg -- --nocapture
cargo test -p claudine-cli opencode_non_interactive -- --nocapture
```

Add new targeted tests for this fix, for example:

```bash
cargo test -p claudine open_code_tool_name_extraction -- --nocapture
cargo test -p claudine open_code_step_finish_does_not_emit_turn_complete_per_step -- --nocapture
cargo test -p claudine-cli compose_opencode_multi_step_run_emits_single_turn_complete -- --nocapture
```

If broader validation is needed after focused tests pass:

```bash
just test
```

## Open Questions

1. Does OpenCode always emit a true final completion event distinct from `step_finish`?

Recommended answer:

- verify from captured streams
- if yes, map only that event to `TurnComplete`
- if not, emit `TurnComplete` once at parser finish when exit is successful

2. Should step-level lifecycle become first-class in the generic stream sink API?

Recommended answer:

- yes if the refactor stays localized
- otherwise do the smaller OpenCode-only fix first

3. Should config caching live in dispatch or in the wrapper session layer?

Recommended answer:

- prefer wrapper-session scope first
- avoid making global dispatch stateful across unrelated invocations
