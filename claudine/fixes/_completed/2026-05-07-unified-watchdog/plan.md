---
phases: 6
created: 2026-05-07
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/stream/progress.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/stream/progress.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
---

# Execution Plan: Unified Watchdog `step_timeout` Stuck Detection

## Overview

Distinguish **active** in-flight work from **stuck** in-flight work using progress timestamps, so `step_timeout` fires correctly when tools or subagents hang forever.

---

## Phase 1 — Data Model Foundation

**Goal:** Add stuck-detection primitives to `LiveMetricsState`.

**Files:** `claudine/lib/src/stream/progress.rs`

| Step | Description | Observable |
|------|-------------|------------|
| 1.1 | Read current `progress.rs` to understand `InFlightTool`, `InFlightSubagent`, `record_tool_start`, and `record_activity`. | Can locate all four items and their fields. |
| 1.2 | Add `last_progress_at: Instant` to `InFlightTool`. | Field exists in struct definition. |
| 1.3 | On `record_tool_start`, set `last_progress_at = now`. | Unit test in Phase 4 verifies this. |
| 1.4 | On `record_activity`, update `last_progress_at` for every item in `in_flight`. | Unit test in Phase 4 verifies this. |
| 1.5 | Add `stuck_tools(&self, now: Instant, threshold: Duration) -> Vec<&InFlightTool>` returning tools where `now - last_progress_at >= threshold`. | Returns correct stuck tools; passes `stuck_tools_returns_empty_when_all_fresh` and `stuck_tools_returns_stuck_ones`. |
| 1.6 | Add `stuck_subagents(&self, now: Instant, threshold: Duration) -> Vec<&InFlightSubagent>` symmetric to 1.5. | Returns correct stuck subagents; passes `stuck_subagents_returns_stuck_ones`. |

**Validation checkpoint:** `cargo test -p claudine-lib stuck_tools stuck_subagents` passes.

**Parallelizable:** Steps 2.1 and 3.1 can start immediately (they only read).

---

## Phase 2 — Watchdog Core Logic

**Goal:** Replace unconditional in-flight suppression with stuck-aware evaluation.

**Files:** `claudine/cli/src/commands/wrap/exec/watchdog.rs`

| Step | Description | Observable |
|------|-------------|------------|
| 2.1 | Read current `watchdog.rs` to locate `evaluate_timeout_tick` and understand existing suppression logic. | Can quote the `has_in_flight` block. |
| 2.2 | Replace the boolean suppression block with stuck-aware logic: compute `stuck_tools`, `stuck_subagents`, `any_stuck`, `any_active`; return `WatchdogTickResult::Ok` only when `any_active && !any_stuck`. | New logic matches spec exactly; compiles. |
| 2.3 | Update `format_step_timeout_breach_message` to append stuck tools (id + name) when `stuck_tools` is non-empty. | Breach message includes stuck tools when present. |

**Validation checkpoint:** `cargo check -p claudine-cli` compiles after changes.

**Depends on:** Phase 1 (types must exist to call `stuck_tools` / `stuck_subagents`).

---

## Phase 3 — Legacy Timeout Helper

**Goal:** Apply identical stuck-aware logic to the standalone `detect_step_timeout`.

**Files:** `claudine/cli/src/commands/wrap/exec/timeouts.rs`

| Step | Description | Observable |
|------|-------------|------------|
| 3.1 | Read current `timeouts.rs` to locate `detect_step_timeout` and its suppression logic. | Can quote the equivalent `has_in_flight` block. |
| 3.2 | Replace suppression with the same stuck-aware logic used in Phase 2.2. | New logic matches spec; compiles. |

**Validation checkpoint:** `cargo check -p claudine-cli` compiles after changes.

**Depends on:** Phase 1.
**Parallelizable with:** Phase 2 (both depend on Phase 1, neither depends on the other).

---

## Phase 4 — Unit Test Updates

**Goal:** Verify data model and watchdog behavior for all stuck/active combinations.

**Files:** `claudine/lib/src/stream/progress.rs`, `claudine/cli/src/commands/wrap/exec/watchdog.rs`

| Step | Description | Observable |
|------|-------------|------------|
| 4.1 | Add `stuck_tools_returns_empty_when_all_fresh` — all tools have recent `last_progress_at`, expect empty vec. | Test passes. |
| 4.2 | Add `stuck_tools_returns_stuck_ones` — one fresh, one stale, expect only the stale. | Test passes. |
| 4.3 | Add `stuck_subagents_returns_stuck_ones` — one fresh, one stale, expect only the stale. | Test passes. |
| 4.4 | Update `evaluate_timeout_tick_silence_suppressed_by_in_flight_tool` — set `started_at = now - budget - 1s` so tool is stuck, assert it **fires**. | Previously `#[ignore]`-like behavior now asserts `Breached`. |
| 4.5 | Update `evaluate_timeout_tick_silence_suppressed_by_in_flight_subagent` — set `started_at` older than `budget`, assert it **fires**. | Test asserts `Breached`. |
| 4.6 | Add `evaluate_timeout_tick_silence_suppressed_when_tool_is_active` — fresh tool, does not fire. | Test asserts `Ok`. |
| 4.7 | Add `evaluate_timeout_tick_silence_suppressed_when_subagent_is_active` — fresh subagent, does not fire. | Test asserts `Ok`. |
| 4.8 | Add `evaluate_timeout_tick_silence_fires_when_tool_is_stuck` — stuck tool triggers `step_timeout`. | Test asserts `Breached`. |
| 4.9 | Add `evaluate_timeout_tick_silence_fires_when_subagent_is_stuck` — stuck subagent triggers `step_timeout`. | Test asserts `Breached`. |
| 4.10 | Add `evaluate_timeout_tick_mixed_active_and_stuck_fires` — one active + one stuck still fires. | Test asserts `Breached`. |

**Validation checkpoint:** `cargo test -p claudine-cli evaluate_timeout_tick` and `cargo test -p claudine-lib stuck` both pass.

**Depends on:** Phases 1, 2, 3.

---

## Phase 5 — Integration Test Activation

**Goal:** Re-enable the two ignored integration tests.

**Files:** `claudine/cli/tests/wrap_commands.rs`

| Step | Description | Observable |
|------|-------------|------------|
| 5.1 | Remove `#[ignore = "..."]` from `watchdog_stream_idle_timeout_after_tool_call_hang`. | Attribute removed; test compiles and runs. |
| 5.2 | Remove `#[ignore = "..."]` from `watchdog_subagent_hang_terminates_and_names_stuck_ids`. | Attribute removed; test compiles and runs. |

**Validation checkpoint:** Both integration tests pass when run individually.

**Depends on:** Phases 1–4.

---

## Phase 6 — Final Validation

**Goal:** Ensure the entire package area passes.

| Step | Description | Observable |
|------|-------------|------------|
| 6.1 | Run full unit-test suite for `claudine-lib` and `claudine-cli`. | `cargo test -p claudine-lib` passes. `cargo test -p claudine-cli` passes. |
| 6.2 | Run integration tests for `claudine-cli`. | `cargo test -p claudine-cli --test wrap_commands` passes. |
| 6.3 | Run `cargo clippy -p claudine-lib -p claudine-cli` and resolve any new warnings. | Zero new warnings. |

**Depends on:** Phases 1–5.

---

## Parallelization Summary

- **Phase 1 + 2.1 + 3.1** can start simultaneously (all are read/analysis tasks).
- **Phase 2 + Phase 3** can proceed in parallel once Phase 1 completes.
- **Phase 4** can begin as soon as Phases 2 and 3 are done.
- **Phase 5** is trivial and follows Phase 4.
- **Phase 6** is strictly last.

## Risk Flags

- If `record_activity` is called in multiple code paths, Step 1.4 must update `last_progress_at` in every path. Verify with `grep -n record_activity`.
- If `InFlightSubagent` already has a `last_progress_at`-equivalent field, Step 1.6 may reuse it rather than add a new field.
- Integration tests may fail if the simulated hang duration in the test is shorter than the new stuck-detection threshold; confirm threshold values before un-ignoring.
