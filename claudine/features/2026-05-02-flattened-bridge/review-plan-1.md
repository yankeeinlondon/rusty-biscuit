# Implementation Plan: Review-1 Fixes for `2026-05-02-flattened-bridge`

**Goal:** Resolve all findings from `review-1.md` so the feature can be marked production-ready.

---

## Phase 1 — Documentation: Add `when` to User-Facing Topic Guide

**File:** `claudine/docs/topics/configuring-actions.md`

**Task:** Insert a new "Conditional Execution (`when`)" section after the existing "Action Types" section and before "Template Variables".

**Required content:**
- State that every action supports an optional `when` field.
- List the full path surface: `env.*`, `extra.*`, `tool_input.*`, `tool_response.*`, `os.*`, `hardware.*`, `git.*`, `project.*`, top-level event fields, and `ctx.*`.
- Explain that falsy/invalid conditions skip the action non-fatally (the rest of the binding continues).
- Provide two JSON examples:
  1. `git.branch == 'main'` on a `speak` action.
  2. `ctx.today != ''` on a `call` action.

**Verification:** Read the rendered doc to confirm the section flows naturally and cross-references `hook-actions.md` (skill doc) for deeper reference.

---

## Phase 2 — Test Hardening: Strengthen Weak `ctx` Regression Test

**File:** `claudine/lib/src/dispatch/runner/mod.rs`

**Task:** Rewrite `when_ctx_fields_do_not_require_precomputed_event_metadata` (line 909) so it asserts the action *ran*, not just that the runner didn't error.

**Approach:**
- Replace the `HookAction::Report` with `HookAction::Call`.
- Use a command that is guaranteed to fail (e.g., `__claudine_missing_when_ctx_weak__`), `can_block = true`, and assert that a `HookDecision::Deny` blocking response is produced.
- Keep the `when: Some("ctx.today != ''"` condition.

**Rationale:** A bug causing `ctx.*` to silently evaluate as falsy would previously pass because `result.is_ok()` is true even when the action is skipped. Asserting a blocking response proves the condition evaluated truthy.

**Verification:** `cargo test -p claudine dispatch::runner::tests::when_ctx_fields_do_not_require_precomputed_event_metadata`

---

## Phase 3 — Test Coverage: Add Missing Runner-Level `when` Tests

**File:** `claudine/lib/src/dispatch/runner/mod.rs`

### 3A. `when_tool_response_path_resolves`

- Populate `meta.tool_response` with `{"exit_code": 0}`.
- Action: `HookAction::Call` with `when: Some("tool_response.exit_code == 0"`.
- Assert the call runs and produces a blocking `Deny` response (via missing command).

### 3B. `when_env_fallback_syntax_works`

- Action: `HookAction::Call` with `when: Some("env.CLAUDINE_TEST_MISSING || \"default\" == \"default\""`.
- Assert the call runs and produces a blocking `Deny` response.
- No env var setup/cleanup needed because the variable is intentionally missing.

**Verification:** `cargo test -p claudine dispatch::runner::tests::when_tool_response` and `when_env_fallback`

---

## Phase 4 — Test Coverage: Add Additional `ctx.*` Field Test

### 4A. Expression-Level Test

**File:** `claudine/lib/src/dispatch/expression.rs`

Add `condition_lookup_ctx_year_resolves` test:
- Create `EventMetaConditionLookup` with `Path::new(".")`.
- Assert `lookup.get("ctx.year")` returns `Some(Value::String(_))` where the string is non-empty.
- This proves that *any* captured `ctx.*` key flows through, not just the hard-coded `today` in existing tests.

### 4B. Runner-Level Test (Optional but Recommended)

**File:** `claudine/lib/src/dispatch/runner/mod.rs`

Add `when_ctx_year_resolves`:
- Same pattern as `when_ctx_today_resolves` but with `when: "ctx.year != ''"`.
- Assert the call runs.

**Verification:** `cargo test -p claudine dispatch::expression::tests::condition_lookup_ctx_year` and `dispatch::runner::tests::when_ctx_year`

---

## Phase 5 — Code Quality: Add `Debug` to `EventMetaConditionLookup`

**File:** `claudine/lib/src/dispatch/expression.rs`

**Task:** Add `#[derive(Debug)]` to `EventMetaConditionLookup`.

**Prerequisite:** `CtxLookup` in Darkmatter must also derive `Debug`. 

**File:** `darkmatter/lib/src/markdown/compose/expression/ctx.rs`

**Task:** Add `#[derive(Debug)]` to `CtxLookup`. `ContextGroup` already derives `Debug` (confirmed in `darkmatter/lib/src/markdown/compose/context/capture.rs:32`), so this is a one-line change.

**Verification:** `cargo check -p claudine -p darkmatter`

---

## Phase 6 — Code Quality: Module-Level Doc Comment for `meta_json.rs`

**File:** `claudine/lib/src/dispatch/runner/meta_json.rs`

**Task:** Add the following line at the top of the file:

```rust
//! Utility module for JSON null stripping (retained after flattening removal).
```

**Verification:** `cargo doc -p claudine` builds without warnings for this module.

---

## Phase 7 — Verification: Confirm Darkmatter Changes Are Committed

**Task:** Run `git status --short darkmatter/` and confirm no modified/untracked files exist.

**Expected:** Clean working tree for `darkmatter/`.

**If uncommitted changes are found:** Commit them with a message referencing this feature.

---

## Phase 8 — Lint and Format

**Commands:**
```bash
cargo fmt --all
cargo clippy -p claudine -p darkmatter -- -D warnings
```

**Remediation:** Fix any new warnings introduced by the changes above.

---

## Phase 9 — Final Test Run

**Commands:**
```bash
cargo test -p claudine dispatch::runner::tests::when
cargo test -p claudine dispatch::expression
cargo test -p darkmatter markdown::compose::expression::ctx
cargo test -p claudine dispatch::template
cargo test -p claudine dispatch::matcher
cargo test -p claudine harness::validate::tests::render_template
```

**Success criteria:** All tests pass, clippy is clean.

---

## Summary of Changes by File

| File | Change |
|------|--------|
| `claudine/docs/topics/configuring-actions.md` | Add "Conditional Execution (`when`)" section |
| `claudine/lib/src/dispatch/runner/mod.rs` | Strengthen weak `ctx` test; add `tool_response` test; add fallback syntax test; add `ctx.year` test |
| `claudine/lib/src/dispatch/expression.rs` | Add `#[derive(Debug)]` to `EventMetaConditionLookup`; add `condition_lookup_ctx_year_resolves` test |
| `darkmatter/lib/src/markdown/compose/expression/ctx.rs` | Add `#[derive(Debug)]` to `CtxLookup` |
| `claudine/lib/src/dispatch/runner/meta_json.rs` | Add module-level `//!` doc comment |

---

## Risk Notes

- **Darkmatter `Debug` derive:** `CtxLookup` contains `RefCell`s; `#[derive(Debug)]` will show the inner map/set contents. This is purely additive and non-breaking.
- **Test command names:** Use `__claudine_missing_*` prefixes consistently so missing commands fail in a predictable way and produce `HookDecision::Deny`.
- **Documentation duplication:** The `configuring-actions.md` `when` section will overlap with `.claude/skills/claudine/hook-actions.md`. This is intentional: the topic guide is user-facing, the skill doc is agent-facing. Keep them in sync but do not rely on one referencing the other for critical user discovery.
