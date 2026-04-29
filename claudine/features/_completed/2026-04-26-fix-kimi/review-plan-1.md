---
created: 2026-04-26
phases: 4
source_review: claudine/features/2026-04-26-fix-kimi/review-1.md
source_spec: claudine/features/2026-04-26-fix-kimi/spec.md
parent_plan: claudine/features/2026-04-26-fix-kimi/plan.md
packages:
    - claudine
    - claudine-cli
---

# Fix-Kimi Review Follow-Up Plan

This plan addresses every recommendation in
[`review-1.md`](./review-1.md) for the already-shipped Kimi wire-mode
implementation. The review confirms the feature is production-ready;
the items below close gaps in **secondary telemetry** (timing
headers), **edge-case stderr classification**, and **logic
duplication** between the wire-mode and stream-mode launch branches.
A final phase guarantees no Clippy/rustc warnings remain in the
`claudine` package area.

## Risk Surface (read first)

- **Refactoring risk in Phase 3 (unification).** The review's third
  item is explicitly framed as a "consider" suggestion, not a
  blocker. The two launch branches in
  [`cli/src/commands/wrap/mod.rs`](../../cli/src/commands/wrap/mod.rs)
  (around lines 1660 and 2167) currently use a structured
  `if let Some(wire_prompt) = …` split. A poorly-scoped refactor
  here can regress every structured provider, not just Kimi. The
  plan therefore takes a **narrow, mechanical** approach: extract a
  single private helper that owns the branch, called from both call
  sites. No semantic changes to the non-Kimi path.
- **Timing monitor and stderr bridge interactions in wire mode.**
  Wire mode currently spawns its own stderr thread (`wire_io.rs:638`)
  and discards the `StderrBridgeHandle` (`let _ = stderr_bridge;` at
  `mod.rs:1670` and `mod.rs:2178`). Wire mode also ignores
  `prompt_timing` (`let _ = prompt_timing;` at `mod.rs:2179`). Both
  must be wired in **without** breaking the existing single-stderr-
  reader contract: only one consumer may own `child.stderr`. The
  plan resolves this by routing every captured stderr line through
  the bridge **inside** the existing `wire_io` stderr thread, and
  by making `spawn_prompt_timing_monitor` / `stop_timing_ticker`
  available as `pub(crate)` helpers that `wire_io` can call directly
  (no thread-ownership change to the timing monitor itself).
- **Clippy warning is trivial** but the acceptance criterion is
  zero warnings in the whole `claudine` area, not just lines this
  plan touches. Phase 4 therefore runs Clippy across the
  `claudine` and `claudine-cli` crates as a gate, not just the
  files modified in earlier phases.

## Phase Index

| Phase | Outcome | Depends on |
| --- | --- | --- |
| 1 | Wire mode emits prompt-timing headers and two-stage timeout warnings | none |
| 2 | Wire mode integrates the `StderrBridgeHandle` so kimi stderr is classified and merged into the summary | 1 (sequencing only — no code dependency) |
| 3 | Launch dispatch in `mod.rs` is unified through a single private helper to remove drift risk between transports | 1, 2 |
| 4 | Zero Clippy/rustc warnings in `claudine` package area; full test suite green | 1, 2, 3 |

---

## Phase 1: Timing Monitor Parity for Wire Mode

### Scope

**In:**

- Make `exec::spawn_prompt_timing_monitor` and
  `exec::stop_timing_ticker` callable from `wire_io.rs` by changing
  their visibility from private to `pub(crate)`.
- Thread `prompt_timing: Option<PromptTimingContext>` into
  `WireSessionConfig` (or `WireSessionWiring`) so
  `run_kimi_wire_session` receives the context that today is
  silenced via `let _ = prompt_timing;` at
  [`mod.rs:2179`](../../cli/src/commands/wrap/mod.rs).
- Inside `run_kimi_wire_session`, spawn the timing monitor when the
  caller provided context **and** the live sink is rendering output
  (mirroring `run_child_stream_semantic`'s `show_timing_output`
  predicate). Call `stop_timing_ticker` after the child exits and
  before joining the stdout reader thread.

**Out:**

- Composition-side `build_prompt_timing_context` plumbing at
  `mod.rs:3124` is unchanged. The context is already constructed
  there; only the **consumption** side in `run_kimi_wire_session`
  changes.
- Stall-detection wiring (`live_metrics` is already passed in via
  `WireSessionWiring`, the comment "reserved for Phase 4 wiring of
  stall detection" can stay — that is the spec's Phase 4, not this
  plan's Phase 4).
- The direct-wrapper branch at `mod.rs:1660` does **not** carry a
  `prompt_timing` context today (wrapper passthrough runs skip the
  monitor entirely per `exec.rs:1372` doc comment), so that call
  site continues to pass `prompt_timing: None`.

### Files to modify

- `claudine/cli/src/commands/wrap/exec.rs` — change visibility of
  `spawn_prompt_timing_monitor` (line 1377) and `stop_timing_ticker`
  (line 1705) from private to `pub(crate)`. The functions
  themselves are unchanged.
- `claudine/cli/src/commands/wrap/wire_io.rs` —
  - Add `pub prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>`
    to `WireSessionWiring` (preferred over `WireSessionConfig`
    because it logically belongs with the other live-wiring fields
    `build_parser`, `stream_output`, `live_metrics`).
  - In `run_kimi_wire_session`, after the child is spawned and
    before stdout/stderr reader threads start, optionally call
    `crate::commands::wrap::exec::spawn_prompt_timing_monitor` —
    only when `wiring.prompt_timing.is_some()` AND
    `crate::log::should_render_timing()` (or whatever the existing
    `show_timing_output` predicate resolves to — locate the same
    condition `exec.rs:1849` uses and extract it to a
    `pub(crate)` helper if it isn't already).
  - Capture the `(Arc<AtomicBool>, JoinHandle)` pair returned and
    call `stop_timing_ticker(Some(...))` in the cleanup path
    immediately after `wait_for_child_exit` returns and before
    joining `stdout_handle`.
  - Pass `wiring.live_metrics.clone()` and
    `wiring.stream_output.clone()` to the monitor — both are
    already available at that scope.
  - Keep all existing tracing spans intact; the timing monitor
    spawns its own thread and emits timing headers via
    `stream_output`, no span entry needed at the wire-mode call
    site.
- `claudine/cli/src/commands/wrap/mod.rs` —
  - At line 2167 (composition / harness call site), pass
    `prompt_timing` into `WireSessionWiring` instead of discarding
    it. Remove the `let _ = prompt_timing;` line.
  - At line 1660 (direct wrapper call site), pass `prompt_timing:
    None` (the variable is not in scope there, so this is just an
    explicit `None` literal).

### Test additions

- `claudine/cli/src/commands/wrap/wire_io.rs` (in the existing
  `#[cfg(test)] mod tests` block):
  - `wire_session_threads_prompt_timing_into_monitor` — drives a
    fake child that takes 11 seconds to exit (use the existing
    fake-child fixture pattern), sets a `PromptTimingContext` with
    a known `prompt_path`, and asserts that the captured
    `stream_output` saw at least one `t=0` timing header line
    written. The exact line shape is owned by
    `prompt_timing_mod::HEADER_CADENCE` in `exec.rs`, so the
    assertion targets the substring `t=0` rather than re-modeling
    the line.
  - `wire_session_skips_timing_monitor_when_context_absent` —
    same fake child, `prompt_timing: None`, asserts no `t=0` line
    was emitted.
  - `wire_session_emits_timeout_warn_when_threshold_crosses` —
    use a `PromptTimingContext` with `timeout_warn = Some(1s)` and
    a fake child that runs for 3 seconds, assert the captured
    `stream_output` contains the existing two-stage warning
    string. Look up the canonical warning text in
    `exec.rs::maybe_emit_timeout_warn` (or equivalent) so the
    assertion stays a substring match, not a hard-coded full
    line.
- `claudine/cli/tests/wrap_commands.rs` — extend the existing
  `kimi_non_interactive_uses_wire_protocol_and_wire_rpc_delivery`
  test (or add a sibling test) that runs the Kimi wrapper through
  `compose` against the fake-child harness and asserts a `t=0`
  line appears on the captured live-sink output. This is the
  end-to-end proof; the unit tests above are the unit proof.

### Verification commands

```bash
cargo test -p claudine-cli --test wrap_commands kimi
cargo test -p claudine-cli wire_io
cargo check -p claudine-cli
```

### Acceptance criteria

- `run_kimi_wire_session` spawns and tears down the timing monitor
  on every composition-mode Kimi run that has a `PromptTimingContext`.
- `cargo test -p claudine-cli wire_io` passes including the three
  new tests above.
- `cargo test -p claudine-cli --test wrap_commands kimi` passes
  including the extended composition timing test.
- No `let _ = prompt_timing;` remains anywhere in `mod.rs`.

---

## Phase 2: Stderr Bridge Integration for Wire Mode

### Scope

**In:**

- Thread `Option<StderrBridgeHandle>` into `WireSessionWiring` so
  `run_kimi_wire_session` can route kimi's stderr lines through
  the same classification pipeline as every other structured
  provider.
- Inside `wire_io.rs`, replace the existing trivial stderr reader
  thread (line 638) with a bridge-aware reader that mirrors the
  shape used in `exec.rs::run_child_stream_semantic` (lines
  1959-1966): when the bridge handle is present, every captured
  line goes through `bridge.ingest_line(...)`; when absent, retain
  the current behavior of writing the line to stderr verbatim.
- Run the bridge `finalize` closure on the main thread after the
  reader thread joins, before the parser's `finish` call so the
  classified stderr state lands in the final
  `StreamExecutionSummary`.
- Honor the `early_terminate` receiver: poll it from
  `wait_for_child_exit` alongside the existing cancel flag so a
  rate-limit-style early termination kills the child the same way
  a Ctrl+C does.

**Out:**

- The stderr bridge protocol itself (`StderrLogBridge` trait) is
  not modified.
- OpenCode-specific bridge behavior is unchanged. Wire mode is
  Kimi-only today; the bridge handle for Kimi is currently always
  `None` in `build_structured_plumbing` (`exec.rs:399-468` doc
  shows OpenCode is the only provider that returns `Some`). This
  phase wires up the **path** so a future Kimi-specific bridge can
  be plumbed without touching the IO loop.
- Stderr noise prefix filtering (`stderr_noise_prefixes`) is **not**
  added to wire mode in this phase. Wire mode does not use the
  noise-prefix mechanism today and the review did not flag it.

### Files to modify

- `claudine/cli/src/commands/wrap/wire_io.rs` —
  - Add `pub stderr_bridge: Option<StderrBridgeHandle>` to
    `WireSessionWiring`. Import
    `claudine::stream::logs::StderrBridgeHandle`.
  - Refactor the stderr reader thread (current body at line 638):
    - Take the optional bridge before spawning, decompose into
      `(bridge, finalize, early_terminate)` exactly as
      `exec.rs:1959-1966` does.
    - In the reader thread, for each line: if `bridge.is_some()`,
      call `bridge.ingest_line(&line)` and respect its
      `StderrIngestOutcome` (drop, forward, classify-and-forward);
      else retain today's behavior (forward verbatim to
      `std::io::stderr()`).
    - Continue accumulating the captured stderr into the existing
      `String` so the `summary.stderr_text` fallback at line 779
      still works.
  - In `wait_for_child_exit`, accept an
    `Option<&Receiver<EarlyTermination>>` parameter and poll it
    alongside the existing `cancel_requested` and timeout polls.
    On early-termination signal: kill the child, return `-1` (or
    the existing interrupted exit code).
  - After joining stderr/stdout threads and before
    `parser.finish(...)`, run the bridge `finalize` closure
    against the not-yet-finalized summary placeholder. Because
    wire mode finalizes via `parser.finish(exit_code)` rather
    than a pre-built summary, run `finalize` **after** `finish`
    against the returned `StreamExecutionSummary` instead. Mirror
    the order in `exec.rs:2067-2068`.
- `claudine/cli/src/commands/wrap/mod.rs` —
  - At line 2167, pass `stderr_bridge` into `WireSessionWiring`
    instead of discarding it. Remove the `let _ = stderr_bridge;`
    line.
  - At line 1660 (direct wrapper call site), the
    `build_structured_plumbing` call already returned
    `stderr_bridge`; pass it through.

### Test additions

- `claudine/cli/src/commands/wrap/wire_io.rs` (test module):
  - `wire_session_routes_stderr_through_bridge` — construct a
    fake `StderrLogBridge` impl that records every `ingest_line`
    call, run a fake child that emits three stderr lines, assert
    the bridge saw all three lines in order.
  - `wire_session_invokes_finalizer_on_summary` — fake bridge
    finalizer mutates `summary.stderr_text` to a sentinel value;
    assert the returned `ProcessResult.data.stderr_text` equals
    the sentinel.
  - `wire_session_honors_early_terminate_signal` — bridge
    finalizer is irrelevant; what matters is that an
    `EarlyTermination` sent on the receiver causes the child to
    be killed and `wait_for_child_exit` to return promptly. Use a
    fake child that loops indefinitely on stdout; assert total
    runtime is < 2s.
  - `wire_session_no_bridge_falls_back_to_stderr_capture` —
    `stderr_bridge: None`, assert `summary.stderr_text` still
    contains the verbatim child stderr.

### Verification commands

```bash
cargo test -p claudine-cli wire_io
cargo test -p claudine-cli --test wrap_commands kimi
cargo check -p claudine-cli
```

### Acceptance criteria

- `run_kimi_wire_session` accepts and consumes a
  `StderrBridgeHandle` when one is provided.
- The four new unit tests pass.
- No `let _ = stderr_bridge;` remains anywhere in `mod.rs`.
- Existing `kimi_wrapper_non_interactive_appends_wire` and
  related Kimi composition tests continue to pass unchanged
  (this proves we did not regress the bridge-absent path).

---

## Phase 3: Unify Launch Dispatch (Reduce Drift Risk)

### Scope

**In:**

- Extract a single private helper —
  `fn launch_structured_attempt(...)` — in
  `cli/src/commands/wrap/mod.rs` that owns the
  `if let Some(wire_prompt) = …` branch currently duplicated at
  lines 1661-1711 and 2169-2220. Both call sites invoke the helper
  with the resources they have already prepared
  (`build_parser`, `stream_output`, `live_metrics`,
  `section_stream`, `stderr_bridge`, optional `prompt_timing`,
  optional `wire_prompt`).
- The helper returns the same `ProcessResult<StreamExecutionSummary>`
  shape both sites already consume.

**Out:**

- No behavioral changes. Wire-mode and stream-mode call paths
  must continue to hand exactly the same arguments to
  `run_kimi_wire_session` and `run_child_stream_semantic`
  respectively. This is a pure code-motion refactor.
- No changes to any other function signatures (Phase 1 and 2
  already settled on the wire-IO API surface; Phase 3 only
  consumes that API).
- The non-structured `Legacy path: forward I/O to terminal` branch
  at `mod.rs:1753` is **not** touched.

### Files to modify

- `claudine/cli/src/commands/wrap/mod.rs` —
  - Define `launch_structured_attempt` as a private function
    (or `pub(crate)` if any test wants to drive it directly)
    accepting one struct argument, e.g.
    `LaunchStructuredAttemptArgs<'_>`, holding all the borrowed
    references currently passed positionally. Using a struct
    avoids a 14-positional-argument function and makes the
    parallel call sites trivially diff-able.
  - Replace the bodies at line 1661-1711 and line 2169-2220 with
    calls to the helper.
  - Both call sites continue to handle the post-call summary
    application (`structured_codex_output.apply_to_summary`,
    `provider == Provider::Codex` Codex stdout flush, etc.) — the
    helper returns the raw `ProcessResult` and does not touch
    summary post-processing.

### Test additions

- No new tests are required — this is pure code motion. The
  existing `wrap_commands` tests, plus the new tests added in
  Phases 1 and 2, are the regression net. If extracting the helper
  reveals a difference between the two call sites that **wasn't**
  obvious before extraction, surface it as an open question
  instead of patching it silently.

### Verification commands

```bash
cargo test -p claudine-cli
cargo test -p claudine
cargo check -p claudine-cli
```

### Acceptance criteria

- `cli/src/commands/wrap/mod.rs` no longer contains two textually
  near-identical `if let Some(wire_prompt) = …` blocks.
- `launch_structured_attempt` (or equivalently named helper)
  exists and is the single point that branches on
  `wire_prompt.is_some()`.
- All `claudine` and `claudine-cli` tests pass without changes
  outside the helper plumbing.
- `git diff --stat` for this phase shows net **negative** lines
  in `mod.rs` (the helper is shorter than the two duplicates it
  replaces).

---

## Phase 4: Zero Warnings + Full Test Sweep

### Scope

**In:**

- Fix the one currently-known Clippy warning in
  `claudine/cli/tests/wrap_commands.rs:947` (`manual_contains`).
- Run the full Clippy sweep across `claudine` and `claudine-cli`
  with `--all-targets` and zero new warnings.
- Run the full test suite for both crates.

**Out:**

- Suppressing warnings via `#[allow(...)]` is **not** acceptable
  unless the warning is a known false positive that the
  surrounding code has already documented. Default action is to
  fix the underlying issue.

### Files to modify

- `claudine/cli/tests/wrap_commands.rs` line 947 — replace
  `args.iter().any(|a| *a == "--output-format")` with
  `args.contains(&"--output-format".to_string())` or
  `args.iter().any(|a| a == "--output-format")` — pick whichever
  matches the existing style in adjacent assertions in the same
  test.
- Any **new** warnings introduced by Phases 1-3 (the four
  modified files: `exec.rs`, `wire_io.rs`, `mod.rs`,
  `wrap_commands.rs`). Address them inline as they appear; do not
  defer to a separate pass.

### Test additions

- None. Phase 4 is a hygiene gate, not a feature phase.

### Verification commands

```bash
# Lint gate: must complete with zero warnings.
cargo clippy -p claudine -p claudine-cli --all-targets --no-deps -- -D warnings

# Full test gate.
cargo test -p claudine
cargo test -p claudine-cli
cargo test -p claudine --test kimi_wire
cargo test -p claudine --test semantic_fidelity
cargo test -p claudine-cli --test wrap_commands

# Build gate (catches anything --no-deps clippy missed).
cargo build -p claudine -p claudine-cli
```

### Acceptance criteria

- `cargo clippy -p claudine -p claudine-cli --all-targets --no-deps
  -- -D warnings` exits zero.
- `cargo test -p claudine` and `cargo test -p claudine-cli` both
  exit zero.
- All tests added in Phases 1 and 2 pass.
- No `#[allow(...)]` was added to suppress warnings except where
  documented in commit message with rationale.

---

## Cross-Phase Notes

### Why this ordering

Phases 1 and 2 are independent feature additions; they could in
principle run in parallel, but ordering them sequentially keeps
the `WireSessionWiring` struct churn linear and the wire-mode test
file diffs reviewable. Phase 3 has to come after both because it
unifies a control-flow site whose surface area changes in 1 and 2.
Phase 4 is the gate.

### Mapping to review items

| Review § | Item | Plan phase |
| --- | --- | --- |
| 1 | Missing timing monitor (feature parity) | Phase 1 |
| 2 | Stderr bridge ignored | Phase 2 |
| 3 | Logic duplication in launch paths | Phase 3 |
| (cross-cutting) | Zero warnings, all tests green | Phase 4 |

### Open questions

- **Parallel timing-monitor and stderr-bridge ownership.** The
  timing monitor and stderr bridge both want to write to
  `stream_output`. The existing `exec.rs::run_child_stream_semantic`
  already runs them concurrently without conflict, so wire mode
  inherits the same guarantee for free. This is called out only
  as a thing to **verify**, not a thing to design.
- **Should Kimi ever return a non-`None` `StderrBridgeHandle`?**
  Today `build_structured_plumbing` returns `None` for Kimi. The
  review item is about the **path**, not about activating a Kimi
  bridge — that is a separate discussion. This plan deliberately
  keeps the activation question out of scope; it just makes the
  path correct so a future "yes, classify Kimi rate-limit
  warnings via the bridge" change is a one-liner in
  `build_structured_plumbing` rather than a fresh wire-mode
  refactor.
