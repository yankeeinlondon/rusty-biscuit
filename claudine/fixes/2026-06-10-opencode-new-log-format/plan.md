---
phases: 3
created: 2026-06-10
start_phase: 1
agent: open_code/zai-coding-plan/glm-5.1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/stream/logs/opencode/events.rs
docs_updated_during_phase_1:
  - claudine/fixes/2026-06-10-opencode-new-log-format/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt
docs_updated_during_phase_2:
  - claudine/fixes/2026-06-10-opencode-new-log-format/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/cli/src/commands/wrap/env.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
docs_updated_during_phase_3:
  - claudine/fixes/2026-06-10-opencode-new-log-format/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/cli/src/commands/wrap/env.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
documentation:
  - claudine/fixes/2026-06-10-opencode-new-log-format/plan.md
packages:
  - claudine
---

# Execution Plan: OpenCode New Log Format Parser

## Scope

Production changes are limited to the OpenCode stderr log parser and classifier
keyword detection. The bridge tests consume a new fixture, but bridge production
behavior and profile config are not modified.

---

## Phase 1 — Parser Extension (`events.rs`)

Goal: `parse_line` recognises the new `timestamp=... level=...` envelope and
produces `ParsedOpenCodeStderrLine::Structured(...)` with correct field
extraction.

- [x] Add a second `LazyLock<Regex>` (`NEW_HEADER_RE`) matching lines that start with `timestamp=YYYY-MM-DDTHH:MM:SS(.sss)?Z level=(DEBUG|INFO|WARN|ERROR)` followed by an optional body
- [x] Extend `parse_timestamp` (or add a companion `parse_timestamp_with_millis`) to accept ISO 8601 strings with optional `.sssZ` suffix using `%.3fZ` alongside the existing `%Y-%m-%dT%H:%M:%S` format
- [x] Update `parse_line` to try `NEW_HEADER_RE` when `HEADER_RE` does not match; on success extract `timestamp`, `level`, set `delta_ms = 0`, and parse the remainder with the existing `parse_body`
- [x] Update the module-level doc comment to mention both supported formats

**Validation checkpoint** — all existing tests in `events.rs` continue to pass
(old-format support is unchanged):

- [x] `cargo test -p claudine --lib stream::logs::opencode::events` — green

---

## Phase 2 — Unit Tests

Goal: prove every parsing and classification dimension for the new format.
Most changes are tests in existing `#[cfg(test)]` modules. The classifier helper
also treats exact `message=` tag values as lifecycle keywords so new-format
`message=loop` / `message="exiting loop"` records classify correctly.

### 2A — Parser tests in `events.rs` (parallel with 2B)

These test `parse_line` in isolation against new-format samples.

- [x] `new_format_parses_info_level` — a simple new-format INFO line returns `Structured` with correct `level`, `timestamp`, `tags`, `delta_ms == 0`
- [x] `new_format_parses_all_levels` — one line each for `DEBUG`, `INFO`, `WARN`, `ERROR` parses with the correct `LogLevel`
- [x] `new_format_timestamp_includes_millis` — `2026-06-10T16:11:27.352Z` parses to a UTC `DateTime` with sub-millisecond accuracy (the `.352` part)
- [x] `new_format_preserves_raw_line` — `record.raw` equals the original input string
- [x] `new_format_extracts_tags` — `run=abc service=session id=ses_123 parentID=ses_parent title=My task` extracts all tag pairs correctly
- [x] `new_format_message_tag_captured` — `message=tracking hash=abc123` results in `tags["message"] == "tracking"`
- [x] `new_format_rejects_non_matching` — lines that do not start with `timestamp=YYYY-...` still fall through to `RawText` when they also don't match the old header
- [x] `new_format_without_message_tag` — a line with only structural tags parses correctly with no `message` tag
- [x] `new_format_without_millis` — a new-format line with `timestamp=2026-06-10T16:11:27Z` (no fractional seconds) still parses

### 2B — Classifier round-trip tests in `errors.rs` (parallel with 2A)

These feed new-format lines through `parse_line` → `classify` and assert
the expected `LogClassification` variant. They prove the parser output is
consumable by the existing classifiers without classifier changes.

- [x] `new_format_classifies_session_created` — `service=session ... created` → `SessionCreated`
- [x] `new_format_classifies_session_created_subagent` — with `parentID=` tag → `SessionCreated { parent_id: Some(...) }`
- [x] `new_format_classifies_llm_call` — `service=llm ... mode=primary` → `LlmCall`
- [x] `new_format_classifies_step_loop` — `service=session.prompt ... step=N ... message=loop` → `StepLoop`
- [x] `new_format_classifies_step_exit` — `service=session.prompt ... message="exiting loop"` → `StepExit`
- [x] `new_format_classifies_permission_evaluated` — `service=permission ... message=evaluated` → `PermissionEvaluated`
- [x] `new_format_classifies_tracking_as_unclassified` — `message=tracking` lines parse but classify as `Unclassified`

### 2C — Bridge integration test in `reasoning.rs` (depends on 2A + 2B)

- [x] Create fixture file `claudine/lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt` containing representative new-format lines covering every lifecycle classification
- [x] `new_format_bridge_consumes_lifecycle_lines` — replay the fixture through `OpenCodeLogBridge::ingest`; assert every line returns `Consumed`, the expected `SemanticEvent` sequence is emitted, and no raw text leaks through

**Validation checkpoint** — all new + existing tests pass:

- [x] `cargo test -p claudine --lib stream::logs::opencode` — green (covers `events`, `errors`, `reasoning`)

---

## Phase 3 — Regression Guard and Edge Cases

Goal: ensure mixed-format resilience and edge-case correctness.

- [x] `mixed_format_stream_parses_both` — a single test that feeds interleaved old-format and new-format lines through `parse_line` and asserts every line returns `Structured` with correct attributes
- [x] `new_format_quoted_message_value` — verify that `message="llm runtime selected"` is tolerated by classifiers (the value is extracted as-is including quotes; classifiers that check tag values should not break)
- [x] `new_format_error_with_inline_json` — a new-format `level=ERROR` line with `error={...json...}` parses and classifies correctly via the existing LLM-failure path

**Final validation checkpoint:**

- [x] `cargo test -p claudine` — full crate green; Phase 3 also normalized host-sensitive color/path assertions exposed by the full claudine area test run
- [x] Grep `reasoning.rs`, `errors.rs`, `profile/opencode.rs` — confirm zero production-line changes in files the spec marks as must-not-change

---

## File Impact Summary

| File | Action |
|---|---|
| `claudine/lib/src/stream/logs/opencode/events.rs` | Production change: new regex + `parse_line` update + new timestamp parser |
| `claudine/lib/src/stream/logs/opencode/events.rs` (tests) | New tests: 2A items |
| `claudine/lib/src/stream/logs/opencode/errors.rs` | Production change: exact `message=` lifecycle keyword matching + new tests: 2B items |
| `claudine/lib/src/stream/logs/opencode/reasoning.rs` (tests) | New tests: 2C item |
| `claudine/lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt` | New fixture file |

## Parallelism

- Phase 2A and 2B are independent and can be implemented concurrently.
- Phase 2C depends on 2A + 2B (needs working parser and classifiers).
- Phase 3 can overlap with 2C once the parser is stable.
