---
phases: 5
starting_phase: 1
---

# Implementation Plan: Review 2 — Tracing/Instrumentation Gaps

## Current State Assessment

The review identifies 9 findings. Re-examination of the codebase reveals that several findings are **already resolved or partially resolved** since the review was written:

| # | Finding | Status | Detail |
|---|---------|--------|--------|
| 1 | Protect evaluation has no spans | **Resolved** | `service.rs` has `protect_evaluate`, `protect_bash`, `protect_write`, `protect_mcp` spans with structured fields (`surface`, `enabled`, `outcome`, `finding_count`, `group`, `rule_id`, `matched_text`). |
| 2 | Harness module is nearly silent | **Partially resolved** | `runtime.rs`, `handlers.rs`, `validate.rs`, `parse.rs` all have spans. `shell.rs` and `audit.rs` remain uninstrumented. |
| 3 | Silent modules (linking, permissions, system_prompt, compose, init) | **Partially resolved** | `linking/mod.rs` has `link_skills` span. `permissions/engine.rs` has `permissions_configured` and `permissions_effective` spans. `system_prompt/prepare.rs` has a span. `compose.rs` has `compose`/`inline_compose` spans. `sequence.rs` has `sequence` span. `init` command has zero tracing. |
| 4 | Stream instrumentation is partial | **Partially resolved** | `stream/mod.rs` has 6 structured trace helpers creating spans. `stream/logs/opencode.rs` has debug/warn sites. `stream/logs/codex.rs` has no tracing. |
| 5 | No `#[instrument]` usage | Open | Lower priority — deferred. |
| 6 | Wrapper session span missing fields | **Open** | Span lacks `provider`, `session_id`, `child_pid`, `structured_mode`. No `field::Empty` declarations currently exist — fields simply aren't declared. |
| 7 | No dispatch span for non-canonical events | **Open** | `dispatch()` function (line 94) has no span wrapper. Only a `debug!` for unknown events. |
| 8 | Composition/sequence have no tracing | **Partially resolved** | Root spans exist (`compose`, `inline_compose`, `sequence`). Per-step spans and phase timing are missing. |
| 9 | Telemetry formatter missing span names | **Resolved** | `collect_span_names` (telemetry.rs:311) renders `[span1>span2]` prefix in output. |

## Actual Remaining Work

Based on the assessment, the real gaps are:

1. **Wrapper session span fields** (Finding 6) — add `provider`, `session_id`, `child_pid`, `structured_mode`
2. **Dispatch span for non-canonical events** (Finding 7) — wrap `dispatch()` in a span
3. **Harness gaps** (Finding 2) — `shell.rs`, `audit.rs`
4. **Silent submodule gaps** (Finding 3) — `init` command, individual linking/permissions/system_prompt submodules
5. **Stream gaps** (Finding 4) — `codex.rs` tracing, error/fallback tracing
6. **Composition/sequence step spans** (Finding 8) — per-step spans with timing
7. **`#[instrument]` migration** (Finding 5) — **DEFERRED** (lower priority consistency improvement)

---

## Phase 1: Wrapper Session Span Fields + Dispatch Non-Canonical Span

**Findings addressed:** 6, 7
**Priority:** Highest — these affect every wrapper session and every non-canonical event path.

### 1a. Add high-value fields to `wrapper_session` span

**File:** `claudine/cli/src/commands/wrap/mod.rs`

The wrapper session span is created at line 915. Currently:

```rust
let wrapper_span = info_span!(
    "wrapper_session",
    binary_path = %binary_path.display(),
    has_prompt,
    interactive_requested,
    edit_requested,
    yolo_requested,
    model_override = %args.model.as_deref().unwrap_or(""),
);
```

Changes:
- Add `provider` field — available from the `provider` variable in the enclosing scope
- Add `session_id` as `tracing::field::Empty` — record it when the stream parser discovers the session ID
- Add `child_pid` as `tracing::field::Empty` — record it when the child process is spawned
- Add `structured_mode` — record it when structured stream mode is determined (or `false` for non-structured providers)

The span is stored as `wrapper_span` and entered via `wrapper_span.enter()` returning `_wrapper_guard`. To support late field population:
1. Change the span to use `.entered()` to get a guard
2. Store the `Span` in a way that downstream code can call `.record("session_id", &value)` on it
3. This likely requires making the span available to the stream processing loop

**Specific approach:**
- Declare `session_id` and `child_pid` as `tracing::field::Empty` on the span
- After child spawn (~line where `Command::new().spawn()` is called), record `child_pid`
- In the structured stream processing path, record `session_id` when it becomes known
- Record `structured_mode` at the point where stream protocol is selected

### 1b. Add dispatch span for non-canonical events

**File:** `claudine/lib/src/dispatch/mod.rs`

The `dispatch()` function (line 94) handles all event types but only has a `debug!` for unknown events. The `dispatch_canonical()` function already has spans.

Changes:
- Wrap the entire `dispatch()` function body in an `info_span!("dispatch_event", %provider)`
- On the unknown-event path, add the `reason` field showing why the event was skipped
- On the error path, include the error in the span

```rust
pub async fn dispatch(...) -> Result<DispatchOutcome> {
    let _span = info_span!("dispatch_event", %provider).entered();
    let adapter = adapters::adapter_for(provider);
    let (event, mut meta) = match adapter.parse_event(raw) {
        Ok(parsed) => parsed,
        Err(AdapterError::UnknownEvent(reason)) => {
            debug!(%provider, %reason, "adapter returned unknown event, skipping dispatch");
            return Ok(DispatchOutcome::default());
        }
        Err(error) => return Err(error.into()),
    };
    // ... rest unchanged
}
```

### Testing — Phase 1

- Manual: Run `claudine codex --debug trace ...` and verify wrapper_session span now shows `provider` and eventually `child_pid`/`session_id`
- Manual: Feed a non-canonical JSON event and verify the `dispatch_event` span appears in trace output
- Existing tests should pass unchanged (no behavioral changes)

---

## Phase 2: Harness Gaps (shell.rs, audit.rs)

**Findings addressed:** 2
**Priority:** High — harness internals are where retry/redirect decisions happen.

### 2a. Instrument `harness/shell.rs`

**File:** `claudine/lib/src/harness/shell.rs`

This module handles shell command execution and approval. No tracing exists.

Changes:
- Add `info_span!("harness_shell_execute", command, timeout_secs)` around `execute_approved_command` and `execute_approved_command_parts`
- Add `debug!` for approval decisions (approved/denied) in `validate_and_approve_command`
- Add `debug!` for execution outcomes (exit_code, duration)

### 2b. Instrument `harness/audit.rs`

**File:** `claudine/lib/src/harness/audit.rs`

This module handles shell audit collection and command approval. No tracing exists.

Changes:
- Add `info_span!("harness_audit", source, command_count)` around `audit_shell_commands`
- Add `debug!` for individual command approval/denial outcomes
- Add `debug!` for the final audit summary

### Testing — Phase 2

- Existing `shell.rs` and `audit.rs` tests pass unchanged
- Manual: Run a harness-enabled composition with `--debug debug` and verify shell execution and audit spans appear

---

## Phase 3: Silent Submodule Gaps

**Findings addressed:** 3
**Priority:** Medium — these modules handle important logic but are less frequently debugged.

### 3a. Instrument `init` command

**File:** `claudine/cli/src/commands/init/mod.rs`, `claudine/cli/src/commands/init_wizard.rs`

The init wizard has zero tracing.

Changes:
- Add `info_span!("init_wizard")` at the top of the wizard entry point
- Add `debug!` for each wizard step (provider selection, config write, etc.)
- Add `debug!` for the final config file path written

### 3b. Add tracing to `linking/` submodules

**Files:** Key files in `claudine/lib/src/linking/`

The module root (`mod.rs`) has `link_skills` span and a `debug!` for skill count. Submodules have zero tracing.

Priority submodules to instrument:
- `linking/discovery.rs` — skill/command discovery: `info_span!("linking_discovery")` with paths_scanned, items_found
- `linking/symlink.rs` — symlink operations: `debug!` for each symlink created/verified
- `linking/agents.rs` — agent linking: `info_span!("linking_agents")` with count
- `linking/commands.rs` — command linking: `debug!` for command registration
- `linking/capabilities.rs` — capability detection: `debug!` for detected capabilities

Lower priority (pure data/logic with few side effects):
- `linking/filter.rs`, `linking/hashing.rs`, `linking/paths.rs`, `linking/compatibility.rs`, `linking/conflict.rs`, `linking/canonical.rs`

### 3c. Add tracing to `permissions/` submodules

**Files:** Key files in `claudine/lib/src/permissions/`

`permissions/engine.rs` has two spans. Submodules have zero tracing.

Priority submodules to instrument:
- `permissions/query.rs` — permission queries: `debug!` for query evaluation outcomes
- `permissions/canonical.rs` — canonical event mapping: `debug!` for mapping results
- `permissions/matchers.rs` — pattern matching: `debug!` for match/no-match on tool patterns
- `permissions/change.rs` — change tracking: `debug!` for detected file changes
- `permissions/explain.rs` — explanation generation: `debug!` for explanation requests

### 3d. Add tracing to `system_prompt/` submodules

**Files:** Key files in `claudine/lib/src/system_prompt/`

`system_prompt/prepare.rs` has a span. Other submodules have zero tracing.

Priority submodules to instrument:
- `system_prompt/resolve.rs` — prompt resolution: `debug!` for resolved sources
- `system_prompt/context.rs` — context assembly: `debug!` for context size/composition
- `system_prompt/types.rs` — pure types, no tracing needed

### Testing — Phase 3

- All existing tests pass unchanged
- Manual: Run `claudine init --debug debug` and verify wizard steps appear in trace
- Manual: Run `claudine claude --debug trace ...` and verify linking/permission resolution appears

---

## Phase 4: Stream Instrumentation Completion

**Findings addressed:** 4
**Priority:** Medium — stream parsing already has good coverage; this fills the remaining gaps.

### 4a. Instrument `stream/logs/codex.rs`

**File:** `claudine/lib/src/stream/logs/codex.rs`

This file has no tracing of its own (only parsing logic for Codex's tracing-format output).

Changes:
- Add `trace!` for each successfully parsed Codex event
- Add `debug!` for unrecognized Codex output lines (fallback/skip paths)
- Add `debug!` for the final parse summary (events parsed, lines skipped)

### 4b. Add error/fallback tracing

**Files:** Stream parser files across `claudine/lib/src/stream/`

Changes:
- In each parser's fallback/error paths, add `debug!` with the reason for fallback
- In the main stream processing loop, add `debug!` when the parser finishes in an error state

### Testing — Phase 4

- Existing stream tests pass unchanged
- Manual: Run with `--debug trace` against each provider and verify all parser events appear

---

## Phase 5: Composition/Sequence Step-Level Tracing

**Findings addressed:** 8
**Priority:** Medium — root spans exist; this adds the per-step visibility the review requested.

### 5a. Add per-step spans to composition execution

**File:** `claudine/cli/src/commands/wrap/composition.rs`

The `execute_composition_request` function orchestrates prompt generation, file writes, and provider invocation. Currently only the root `compose`/`inline_compose` span exists.

Changes:
- Add `info_span!("composition_preflight")` around pre-flight checks and shell approvals
- Add `info_span!("composition_prepare")` around prompt preparation
- Add `info_span!("composition_execute")` around provider invocation
- Add `info_span!("composition_postprocess")` around output processing
- Each span should include timing data (leveraging `FmtSpan::CLOSE` which is already enabled at debug level)

### 5b. Add per-step spans to sequence execution

**File:** `claudine/cli/src/commands/wrap/sequence.rs`

The `execute_sequence` function runs a serial sequence of composition steps.

Changes:
- Add `info_span!("sequence_step", step_index, step_file)` around each step's execution
- Add `info_span!("sequence_preflight")` around the overall plan resolution
- Add `debug!` for step outcomes (pass/fail/skip)
- Add `debug!` for fail-fast decisions

### Testing — Phase 5

- Existing composition and sequence tests pass unchanged
- Manual: Run `claudine compose <file> --debug debug` and verify per-phase spans with timing
- Manual: Run `claudine sequence <file> --debug debug` and verify per-step spans

---

## Deferred: `#[instrument]` Migration (Finding 5)

The codebase uses manual `info_span!().entered()` / `.in_scope()` consistently. Migrating to `#[instrument]` is a style consistency improvement that:

- Reduces boilerplate
- Keeps span names in sync with function names
- Enables `skip` and `fields(...)` attributes for cleaner field declarations

This is deferred because:
1. It requires `tracing` attributes feature and adds a proc-macro dependency
2. It's a large mechanical change across many files
3. It doesn't add new observability — just improves consistency
4. Risk of introducing subtle behavioral changes (span guard lifetime differences)

**If pursued later**, the migration should be done module-by-module, starting with the highest-value spans (dispatch, wrapper session, harness) and working outward.

---

## Implementation Notes

### Conventions to follow

1. **Span naming:** Use `{module}_{action}` pattern (e.g., `harness_shell_execute`, `linking_discovery`)
2. **Field naming:** Use snake_case, avoid `Debug` format (`?`) in span fields — prefer `Display` or explicit formatting
3. **Level selection:**
   - `info_span!` for operations that represent meaningful work boundaries
   - `debug!` for intermediate decisions and data flow
   - `trace!` for high-frequency/low-value per-event data
4. **Guard management:** Follow existing pattern: `let _span = info_span!(...).entered();` for synchronous scopes
5. **Async spans:** For async functions, use `info_span!(...).entered()` only if the span covers the full function body. For partial coverage, use `span.in_scope(|| ...)` for sync blocks or `tracing::instrument::Instrument` for futures.

### Lint hygiene

- Every file touched must pass `cargo clippy --all-targets` without new warnings
- Unused `tracing` imports from adding/removing macros must be cleaned up
- `#[allow(unused_imports)]` must not be used to suppress legitimate warnings

### Verification checklist (after each phase)

1. `cargo check -p claudine` passes
2. `cargo check -p claudine-cli` passes
3. `cargo clippy -p claudine --all-targets` passes
4. `cargo clippy -p claudine-cli --all-targets` passes
5. `cargo test -p claudine` passes
6. `cargo test -p claudine-cli` passes
7. Manual smoke test with `--debug trace` shows new spans
