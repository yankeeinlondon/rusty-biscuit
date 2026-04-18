---
phases: 4
created: 2026-04-17
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: [claudine/features/2026-04-18-opencode-reporting-improvements/plan.md]
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/opencode_semantic.rs
  - claudine/lib/src/stream/tool_display.rs
  - claudine/lib/tests/semantic_fidelity.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/live_semantic_sink.rs
  - claudine/lib/src/stream/badges.rs
  - claudine/lib/src/stream/logs/opencode.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/live_semantic_sink.rs
docs_updated_during_phase_4:
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/features/2026-04-18-opencode-reporting-improvements/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
packages: [claudine, claudine-cli]
---

## Phase 1 Baseline — Audit Findings

**Tests encoding the old contracts (must be flipped in later phases):**

- `claudine/lib/src/stream/tool_display.rs:744` — `from_result_uses_status_and_drops_summary_when_status_present` asserts `summary.is_none()` when a status is present ("status wins over summary"). Replace in Phase 2 with a test asserting a successful shell result keeps both `status = Success` and `summary = Some("bash ls -la")`.
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs:1310` — `tool_result_status_wins_over_input_summary` asserts `"ls -la"` is absent from the rendered line. Replace in Phase 3 with an assertion that `successful` and the summary co-render.
- `claudine/lib/src/stream/badges.rs:609` — `stderr_diagnostics_malformed_assets_yields_config_badge` asserts `badges.len() == 1` with category `Config`. Flip in Phase 3 to assert the Config badge is absent even when `malformed_asset_events > 0`.
- `claudine/lib/src/stream/badges.rs:629` — `stderr_diagnostics_single_malformed_asset_uses_singular_noun` asserts singular noun form. Flip in Phase 3 (absence check).
- `claudine/cli/tests/wrap_commands.rs:3890` — `opencode_stderr_malformed_asset_yields_config_badge_and_success` asserts a `config` badge is present in the summary payload. Update in Phase 3 to assert absence while `malformed_asset_events == 1` is preserved.
- `claudine/cli/tests/wrap_commands.rs:4012` — `opencode_structured_summary_merges_stderr_diagnostics_and_badges` asserts a `config` badge is present after merge. Update in Phase 3 to preserve the diagnostics merge assertion and remove the badge expectation.

**Fixtures available (no new fixtures required for Phase 1):**

- `claudine/lib/tests/fixtures/providers/opencode.ndjson` (112 lines) already contains `step_start` / `step_finish` and 41 tool-use events, covering the replay path for spacing and suppression assertions.
- Inline shell fixtures in `wrap_commands.rs` (e.g. `opencode_stderr_malformed_asset_yields_config_badge_and_success` at line 3890, `opencode_structured_summary_merges_stderr_diagnostics_and_badges` at line 4012) already emit `ERROR … service=config … failed to load …` stderr lines and the matching stdout events needed for the malformed-asset trailer regression.
- Unit fixtures inside `live_semantic_sink.rs` (`opencode_stderr_snapshot` at line 2929, `opencode_tool_use_completion_shows_incoming_arrow_only` at line 2951) provide short inline OpenCode event lists for step-phase and tool-result rendering checks. No new fixtures needed for Phase 2–3; the new tests can reuse these.

**Baseline test runs (all passing pre-change):**

- `cargo test -p claudine --test semantic_fidelity` — 34/34
- `cargo test -p claudine --lib stream::badges` — 27/27
- `cargo test -p claudine --lib stream::tool_display` — 37/37
- `cargo test -p claudine-cli --bin claudine live_semantic_sink` — 59/59
- `cargo test -p claudine-cli --test wrap_commands opencode` — 17/17

These baselines encode the user-visible defects listed in `spec.md`: the current output includes a `config` trailer badge, `Bash(successful)` slot-less renders, and `step_start` / `step_finish` Info lines. Phase 2–3 will flip these assertions to the new contract.



# OpenCode Reporting Improvements Execution Plan

## Outcome

Bring OpenCode's non-interactive reporting back into contract by:

1. suppressing `step_start` / `step_finish` from stderr without dropping them from semantic logging
2. restoring useful summaries on successful incoming tool results
3. removing the duplicate malformed-asset trailer badge while preserving diagnostics
4. proving the spacing and rendering contract with focused regression coverage

## Phase Overview

| Phase | Goal | Depends on | Parallelizable |
| --- | --- | --- | --- |
| 1 | Baseline the current behavior and pin the affected fixtures/tests | none | limited |
| 2 | Repair the semantic data path for OpenCode tool results | 1 | no |
| 3 | Update stderr rendering and trailer presentation | 2 | yes, within phase |
| 4 | Lock in coverage, docs, and acceptance validation | 3 | limited |

## Phase 1 - Baseline and Test Scaffolding

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 1.1 | Audit the existing assertions in `claudine/lib/tests/semantic_fidelity.rs`, `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, `claudine/lib/src/stream/badges.rs`, and `claudine/cli/tests/wrap_commands.rs` that encode the old contracts. | none | yes | A short list of the exact tests to update or replace, including current assertion intent. |
| 1.2 | Confirm the fixture inputs needed for this change: an OpenCode `tool_start` + `tool_end` success path, a replay that includes `step_start` / `step_finish`, and malformed-asset warnings that currently produce the trailer badge. Add or isolate fixtures only if coverage is missing. | 1.1 | yes | Fixture paths are identified, or a new fixture file exists with the required event shapes. |
| 1.3 | Run the current focused tests to capture the pre-change surface and verify the starting point. | 1.1, 1.2 | no | Terminal output shows the current rendering behavior and identifies which assertions will flip. |

Validation checkpoint:

- Run `cargo test -p claudine semantic_fidelity -- --nocapture`
- Run `cargo test -p claudine-cli wrap_commands -- --nocapture`
- Run `cargo test -p claudine-cli live_semantic_sink -- --nocapture`
- Exit when the team can point to the specific baseline output for blank-line noise, missing tool summaries, and malformed-asset badge duplication.

## Phase 2 - Semantic Data Path Fixes

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 2.1 | Update `claudine/lib/src/stream/opencode_semantic.rs` so `handle_tool_result(...)` preserves cached tool input on emitted `SemanticEvent::ToolResult`, preferring wire-provided input when present and using cached input as fallback. | 1 | no | `ToolResult.extra["input"]` is populated in the paired OpenCode success path. |
| 2.2 | Update `claudine/lib/src/stream/tool_display.rs` so `ToolCallDisplay::from_result(...)` derives summaries from `extra["input"]` first, then `output`, and no longer drops summaries just because a status exists. | 2.1 | no | A successful shell result can carry both `status` and `summary`; unknown tools still degrade to summary-less rendering. |
| 2.3 | Add or replace library regressions for the new contract in `tool_display.rs` and `semantic_fidelity.rs`. | 2.1, 2.2 | yes | Tests assert cached OpenCode input survives into `ToolResult`, and successful non-file results retain summaries. |

Validation checkpoint:

- Run `cargo test -p claudine semantic_fidelity -- --nocapture`
- Run `cargo test -p claudine tool_display -- --nocapture`
- Exit when a fixture-driven OpenCode success case proves `extra["input"]` survives into `ToolResult` and a `Bash` result keeps a summary alongside `success`.

## Phase 3 - Stderr Rendering and Badge Cleanup

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 3.1 | Add an OpenCode-specific sink guard in `claudine/cli/src/commands/wrap/live_semantic_sink.rs` that suppresses rendering of `SemanticEvent::Info` only when `extra["step_phase"]` is present. Keep the event flowing to metrics and JSONL. | 2 | yes | `step_start` and `step_finish` disappear from stderr replays while metrics/logging behavior is unchanged. |
| 3.2 | Update `render_tool_display(...)` in the same file so incoming `success` and `pending` results render `status + summary` together when a summary exists, while preserving the current error-path behavior. | 2 | yes | Incoming `Read` and `Bash` success lines render with useful slots instead of bare `successful`. |
| 3.3 | Remove malformed-asset badge emission from `claudine/lib/src/stream/badges.rs` without touching the underlying diagnostics counter. | 2 | yes | Final summaries no longer include `Config - Skipped ... malformed OpenCode assets`, but diagnostics still record the count. |
| 3.4 | Update sink, badge, and wrapper integration tests to reflect the new human-visible contract. | 3.1, 3.2, 3.3 | no | Targeted tests assert step markers are absent, tool summaries are present, and malformed assets are reported exactly once. |

Validation checkpoint:

- Run `cargo test -p claudine-cli live_semantic_sink -- --nocapture`
- Run `cargo test -p claudine badges -- --nocapture`
- Run `cargo test -p claudine-cli wrap_commands -- --nocapture`
- Exit when stderr replay coverage shows no step-marker lines, no duplicate malformed-asset trailer, and useful slots on incoming successful tool results.

## Phase 4 - Acceptance Validation and Documentation

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 4.1 | Replay the OpenCode acceptance fixture or equivalent local session and compare the rendered stderr against the spec's target characteristics: no step markers, no double blank lines, useful `Read` / `Bash` slots, no malformed-asset trailer badge. | 3 | no | A captured stderr transcript matches the expected shape from `spec.md`. |
| 4.2 | Update `claudine/docs/topics/non-interactive-sessions.md` to document the new incoming result rendering contract where status and summary can co-exist. | 4.1 | yes | Docs no longer describe the old "status wins over summary" rule. |
| 4.3 | Run the final focused suite for both crates and record any residual risk if broader provider regression coverage is intentionally deferred. | 4.1, 4.2 | no | Final validation output is clean, or remaining risk is explicitly documented with scope and reason. |

Validation checkpoint:

- Run `cargo test -p claudine semantic_fidelity -- --nocapture`
- Run `cargo test -p claudine tool_display -- --nocapture`
- Run `cargo test -p claudine badges -- --nocapture`
- Run `cargo test -p claudine-cli live_semantic_sink -- --nocapture`
- Run `cargo test -p claudine-cli wrap_commands -- --nocapture`
- Exit when the acceptance transcript satisfies all four user-visible requirements from `spec.md`.

## Parallel Work Notes

- Phase 1 steps `1.1` and `1.2` can be split across two workers if desired.
- In Phase 3, `3.1`, `3.2`, and `3.3` touch different concerns and can proceed in parallel once Phase 2 lands; `3.4` waits for all three.
- Documentation in `4.2` can start as soon as the renderer contract stabilizes, but it should not merge until `4.1` confirms the accepted behavior.

## Acceptance Gate

The plan is complete only when all of the following are directly observable:

1. No rendered `step_start` or `step_finish` lines remain in OpenCode stderr output.
2. There are no runs of two blank lines in the Tool Use & Events section for the acceptance replay.
3. Successful incoming `Read` and `Bash` results render with meaningful summaries derived from the original tool input when available.
4. Malformed OpenCode assets are surfaced once per warning line and never repeated as a trailer badge.
