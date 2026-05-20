---
phases: 5
created: 2026-05-16
start_phase: 1
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
    - Retain `status_code`, `error_name`, `reset_at`, `provider_id`, `model_id`, and `provider_error`.
- [ ] Update all usage sites of `LogClassification::RateLimit` across the codebase to use `ProviderLimit`.

## Phase 2 — Classification Logic Enhancement

Implement the four-kind resolution logic in the LLM failure classifier.

- [ ] Update `claudine/lib/src/stream/logs/opencode/errors.rs`:
    - Implement `is_overload` detection (case-insensitive "overload" in message or `engine_overloaded_error` type).
    - Implement `has_cap` detection (`1308` code, `exceeded_current_quota_error`, or "Usage limit reached" phrase).
    - Update `classify_llm_failure` to return `ProviderLimit` with the correct `kind`:
        1. `status_code == 429` AND (`AI_RetryError` OR `maxRetriesExceeded`) -> `RetriesExhausted`.
        2. `has_cap` AND error context present -> `UsageCap`.
        3. `has_cap` WITHOUT error context -> `ApiFailure` (advisory path).
        4. `status_code == 429` AND `is_overload` -> `Overloaded`.
        5. `status_code == 429` -> `RateLimited`.
- [ ] Implement the advisory path: if a cap phrase is found without 429/error context, emit a non-fatal `ApiFailure` carrying the extracted provider message.

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

- [ ] Update `.claude/skills/claudine/opencode-event-sources.md`:
    - Add a new "Failure Classifications" table or expand the existing one to include `ProviderLimit`.
    - Describe the four `ProviderLimitKind` variants and the two-axis model (provider capacity vs. account consumption).
- [ ] Update any other relevant docs (e.g., `claudine/docs/topics/errors.md` if it exists).

## Phase 5 — Verification and Regression Testing

Validate the new behavior with fixtures and updated unit tests.

- [ ] Create `claudine/lib/tests/fixtures/logs/opencode-429-overload.txt`:
    - Content: A real JSONL line from a Kimi overload (HTTP 429, `rate_limit_error`, message "The engine is currently overloaded...").
- [ ] Update `claudine/lib/tests/semantic_fidelity.rs` (and other test files):
    - Add test case: 429 Overload produces non-terminal Warning.
    - Add test case: 429 Throttled (no overload text) produces non-terminal Warning.
    - Add test case: 429 + `maxRetriesExceeded` produces terminal Error.
    - Update `1308` (ZAI) tests: verify it produces `UsageCap` and terminates even after stdout.
    - Update test for "Usage limit reached" without 429: verify it produces non-fatal `ApiFailure`.
- [ ] Run `cargo test -p claudine` and `cargo clippy -p claudine` to ensure everything is correct and idiomatic.
