---
phases: 5
created: 2026-05-16
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - .claude/skills/claudine/opencode-event-sources.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/opencode-event-sources.md
source_files_during_phase_5:
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/tests/fixtures/logs/opencode-429-overload.txt
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - claudine
---

# Plan — OpenCode 429 Classification Refinement

This plan implements a refined classification model for OpenCode HTTP 429 (Too Many Requests) errors. It distinguishes between transient provider overloads (which should continue) and terminal usage caps (which should stop), resolving an issue where "server overloaded" messages from Kimi were misclassified as fatal usage caps.

## Phase 1 — Domain Model and Types

In this phase, we introduce the `ProviderLimitKind` enum and update the `LogClassification` model to support multi-kind classification.

- [ ] Add `ProviderLimitKind` enum to `claudine/lib/src/stream/logs/opencode/events.rs`.
    - Variants: `Overloaded`, `RateLimited`, `UsageCap`, `RetriesExhausted`.
- [ ] Rename `LogClassification::RateLimit` to `LogClassification::ProviderLimit` in `claudine/lib/src/stream/logs/opencode/events.rs`.
- [ ] Update `LogClassification::ProviderLimit` fields:
    - Replace `is_fatal: bool` with `kind: ProviderLimitKind`.
    - **Drop `error_name`** — it was a proxy for the fatal/non-fatal distinction that `kind` now encodes directly, so retaining it would duplicate information.
    - Retain `status_code`, `reset_at`, `provider_id`, `model_id`, and `provider_error`.
- [ ] Update all usage sites of `LogClassification::RateLimit` across the codebase to use `ProviderLimit` (including `errors.rs` tests, `reasoning.rs` match arms, and any external consumers of the `error_name` field — these must be reworked to read `kind` instead).

## Phase 2 — Classification Logic Enhancement

Implement the four-kind resolution logic in the LLM failure classifier.

- [ ] Update `claudine/lib/src/stream/logs/opencode/errors.rs`:
    - Implement `is_overload` detection using the existing **`contains_any_ci(haystack, &["overload", "engine_overloaded_error"])`** helper. Do **not** use `str::contains` — it is case-sensitive in Rust and would miss `"Overloaded"` / `"OVERLOAD"`.
    - Implement `has_cap` detection (`"\"code\":\"1308\""`, `exceeded_current_quota_error`, or `"Usage limit reached"` phrase).
    - Implement the **error-context gate** strictly as `record.tags.get("error").is_some()`. Status code or known error names are **not** sufficient on their own — only the presence of an `error` tag proves the line came from an OpenCode error envelope rather than echoed/quoted text. This gate is the primary defense against false-positive termination.
    - Update `classify_llm_failure` to return `ProviderLimit` with the correct `kind`. **Resolution order is critical — cap-with-context wins over retries-exhausted on purpose** (a 429 that exhausts retries while carrying a 1308 / `exceeded_current_quota_error` signal is fundamentally a cap; demoting it to the vague `RetriesExhausted` message would undo the distinction this feature exists to create):
        1. `has_cap` AND error context present → `UsageCap`.
        2. `status_code == 429` AND (`AI_RetryError` OR `maxRetriesExceeded`) → `RetriesExhausted`.
        3. `has_cap` WITHOUT error context → `ApiFailure` (advisory path).
        4. `status_code == 429` AND `is_overload` → `Overloaded`.
        5. `status_code == 429` → `RateLimited`.
- [ ] Implement the advisory path: if a cap phrase is found without an `error` tag, emit a non-fatal `ApiFailure` carrying the extracted provider message (via `extract_provider_message` at `errors.rs:186`, falling back to `record.message` if extraction yields nothing).

## Phase 3 — Event Handling and Reasoning

Update the reasoning bridge to handle the new classification kinds and adjust termination behavior.

- [ ] Rename `on_rate_limit` to `on_provider_limit` in `claudine/lib/src/stream/logs/opencode/reasoning.rs`.
- [ ] Implement branching logic in `on_provider_limit`:
    - **`Overloaded`**: Emit non-terminal `Warning { "server overloaded; will retry" }`. Do not set `state.rate_limit`.
    - **`RateLimited`**: Emit non-terminal `Warning { "request throttled; will retry" }`. Do not set `state.rate_limit`.
    - **`UsageCap`**: Emit terminal `Error` with rendered message. Set `state.rate_limit`. Trigger early termination.
    - **`RetriesExhausted`**: Emit terminal `Error` ("provider 429s did not clear after retries"). Set `state.rate_limit`. Trigger early termination.
- [ ] Remove the `stdout_seen` guard for terminal kinds (`UsageCap`, `RetriesExhausted`). Terminal limits now always kill the child process regardless of whether output was already observed.
- [ ] Update `EarlyTermination` enum if necessary, or ensure `EarlyTermination::RateLimit` is only fired for `UsageCap`/`RetriesExhausted`.

## Phase 4 — Documentation and Skill Update

Document the new classification model and the distinction between capacity and consumption limits.

- [ ] Update **`<repo-root>/.claude/skills/claudine/opencode-event-sources.md`** (the file lives at the repo root, **not** under `claudine/.claude/`; verify with `find . -name opencode-event-sources.md` before editing):
    - Add a new "Failure Classifications" table or expand the existing one to include `ProviderLimit`.
    - Describe the four `ProviderLimitKind` variants and the two-axis model (provider capacity vs. account consumption).
    - Call out the `kimi-for-coding` gap: the coding endpoint has no confirmed cap type, so a real allowance exhaustion surfaces as `RetriesExhausted`, not `UsageCap`.
- [ ] After editing the skill, regenerate its `hash:` frontmatter with `md hash <file>` (Darkmatter hasher for Markdown).
- [ ] Update any other relevant docs (e.g., `claudine/docs/topics/errors.md` if it exists).

## Phase 5 — Verification and Regression Testing

Validate the new behavior with fixtures and updated unit tests.

- [ ] Create `claudine/lib/tests/fixtures/logs/opencode-429-overload.txt`:
    - Content: A real JSONL line from a Kimi overload (HTTP 429, `rate_limit_error`, message "The engine is currently overloaded...").
- [ ] Update `claudine/lib/tests/semantic_fidelity.rs` and the inline tests in `errors.rs` / `reasoning.rs`:
    - Add test case: 429 Overload produces non-terminal `Warning { "server overloaded; will retry" }`; `state.rate_limit` stays `None`; no early termination.
    - Add test case: 429 Throttled (no overload text) produces non-terminal `Warning { "request throttled; will retry" }`; no early termination.
    - Add test case: 429 + `maxRetriesExceeded` produces `RetriesExhausted` → terminal `Error` + early termination.
    - Add test case: `exceeded_current_quota_error` (standard Kimi API) **with** `error` tag → `UsageCap` → terminal `Error` + early termination.
    - Add test case: cap phrase **without** `error` tag → non-fatal `ApiFailure` carrying the provider message; no termination.
    - Add test case for the resolution-order rule: 429 + `AI_RetryError` + `"\"code\":\"1308\""` + `error` tag → `UsageCap` (NOT `RetriesExhausted`). This guards the cap-wins-over-exhausted ordering against regression.
    - Update `1308` (ZAI) tests: verify they produce `UsageCap` and terminate regardless of stdout activity.
- [ ] **Rename the following tests in lockstep with the assertion updates** (the old names assert the opposite of the new behavior and must not survive):
    - `classifies_rate_limit_with_reset_time` → `classifies_usage_cap_with_reset_time`
    - `fixture_rate_limit_classifies` → `fixture_usage_cap_classifies`
    - `rate_limit_after_stdout_emits_warning_no_early_terminate` → `usage_cap_after_stdout_emits_terminal_error_and_early_terminate`
    - `rate_limit_without_retry_error_is_warning_even_before_stdout` → `usage_cap_without_retry_error_still_terminates`
    - `rate_limit_before_stdout_emits_terminal_error_and_early_terminate` → `usage_cap_before_stdout_emits_terminal_error_and_early_terminate`
    - `rate_limit_fires_early_termination_only_once` → `provider_limit_fires_early_termination_only_once`
- [ ] Run `cargo test -p claudine` and `cargo clippy -p claudine` to ensure everything is correct and idiomatic.
