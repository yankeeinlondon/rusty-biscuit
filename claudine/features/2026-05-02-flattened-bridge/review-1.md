---
agent: kimi_code
model: ""
ready: false
---

# Review: Unify the Hook `when` Lookup with `EventMetaExpressionLookup`

## Summary

The implementation successfully realizes the spec's target architecture:

- `CtxLookup` extracted from `ShortcutLookup` in Darkmatter and made public
- `EventMetaConditionLookup` composite introduced in Claudine
- `evaluate_when` rewired to use `parse_condition` + `evaluate` against the composite
- `flatten_event_meta_aliases` and `event_meta_to_json` deleted
- Skill docs updated to describe the composite lookup

All existing tests pass and clippy is clean. However, **the Darkmatter Phase 1 changes are uncommitted in the working tree**, the user-facing action-configuration doc omits the `when` field entirely, and a few test coverage gaps remain.

---

## Findings


### 🟡 High — Missing `when` Documentation in User-Facing Topic Guide

**Severity:** high  
**Location:** `claudine/docs/topics/configuring-actions.md`

The topic guide that describes how users configure actions does not mention the `when` conditional-execution field at all. This field is a first-class user-facing feature: every action variant supports it, it drives whether actions run, and it accepts a full Darkmatter condition expression. Users reading `configuring-actions.md` will not discover that conditional execution exists.

The spec's Phase 5 Step 1 anticipated "no changes needed" because the doc "currently describes user-facing surface only." The omission of `when` from that surface is a pre-existing gap, but it is made more acute by this feature because `when` is now a robust, fully integrated capability that users should be taught to use.

**Required:** Add a "Conditional Execution (`when`)" section to `configuring-actions.md` covering:
- That every action supports an optional `when` field
- The path surface available (`env.*`, `extra.*`, `tool_input.*`, `tool_response.*`, `os.*`, `hardware.*`, `git.*`, `project.*`, top-level event fields, `ctx.*`)
- That falsy/invalid conditions skip the action non-fatally
- One or two examples (e.g. `git.branch == 'main'`, `ctx.today != ''`)

---

### 🟡 High — Weak `ctx` Regression Test

**Severity:** high  
**Location:** `claudine/lib/src/dispatch/runner/mod.rs`, test `when_ctx_fields_do_not_require_precomputed_event_metadata`

This test only asserts `result.is_ok()`. It does **not** verify that the `ctx.*` expression actually evaluated to truthy and allowed the action to run. A bug that caused `ctx.*` to silently evaluate as falsy (or that changed `CtxLookup` to return `None` for all keys) would pass this test.

The companion test `when_ctx_today_resolves` (line 938) is strong—it asserts the action ran and produced a blocking response—but it does not exercise the same code path (the weak test uses `Report` with `can_block = false`; the strong test uses `Call` with `can_block = true`).

**Required:** Strengthen `when_ctx_fields_do_not_require_precomputed_event_metadata` to assert the action actually executed. For a `Report` action this can be done by capturing stdout or by switching the test to use a `Call` action and asserting `result.is_none()` when the condition is intentionally falsy, or by adding a second assertion inside the test that verifies the report was emitted.

---

### 🟠 Medium — Missing Runner-Level `when` Tests for `tool_response.*` and Fallback Syntax

**Severity:** medium  
**Location:** `claudine/lib/src/dispatch/runner/mod.rs` tests

The spec's Behavior Parity Invariants table (§E) lists `extra.<path>`, `tool_input.<path>`, `tool_response.<path>` as requirements. Runner tests cover `tool_input.command` and `extra.attempt`, but there is **no runner-level test** for:

- `tool_response.*` resolution in a `when` expression
- Fallback syntax (`env.MISSING || "default"`) in a `when` expression

These paths are covered at the `EventMetaExpressionLookup` unit-test level, but the runner is the only place where the full `parse_condition → evaluate → is_truthy → action execution` pipeline is exercised end-to-end. A regression in how `tool_response` is populated into `EventMeta` or how fallback operators interact with `Null` returns from the lookup would only be caught by runner tests.

**Recommended:** Add two tests:
1. `when_tool_response_path_resolves` — set `meta.tool_response` to a JSON object and assert a `when` like `tool_response.exit_code == 0` is truthy
2. `when_env_fallback_syntax_works` — assert `env.CLAUDINE_TEST_MISSING || "default" == "default"` evaluates truthy

---

### 🟠 Medium — No Test for Additional `ctx.*` Fields Beyond `today`

**Severity:** medium  
**Location:** `claudine/lib/src/dispatch/runner/mod.rs` and `claudine/lib/src/dispatch/expression.rs` tests

Only `ctx.today` is tested. Darkmatter's context capture surface includes other groups (e.g. `DateTime` provides `year`, `month`, `day`, `weekday`; `Git` provides `ctx.git_branch` in some configurations; etc.). The `CtxLookup` test in darkmatter only tests `ctx.today` as well.

While exhaustive context-group testing belongs in Darkmatter, Claudine's composite lookup should have at least one test proving that **any** captured `ctx.*` key flows through, not just the one special-cased in tests. This guards against a future refactor of `CtxLookup` that accidentally hard-codes `today` handling.

**Recommended:** Add a test in `expression.rs` or `runner/mod.rs` that references a second `ctx.*` key (e.g. `ctx.year`) and asserts it resolves to a non-empty string.

---

### 🟢 Low — `EventMetaConditionLookup` Lacks `Debug` Derive

**Severity:** low  
**Location:** `claudine/lib/src/dispatch/expression.rs`

`EventMetaExpressionLookup` derives `Debug, Clone, Copy`. `EventMetaConditionLookup` derives nothing. Because it contains a `CtxLookup` (which holds `RefCell`s), it cannot be `Copy` or `Clone`, but it **can** implement `Debug` manually or via a derived impl on the struct with a custom impl for the `ctx` field. This is a minor ergonomic papercut for anyone trying to log or inspect the lookup in tests or tracing.

**Recommended:** Add `#[derive(Debug)]` to `EventMetaConditionLookup` (this works because `CtxLookup` does not need to be `Debug` for the struct to derive it—wait, actually `#[derive(Debug)]` requires all fields to implement `Debug`, and `CtxLookup` doesn't derive `Debug`. So either add `Debug` to `CtxLookup` in Darkmatter, or implement `Debug` for `EventMetaConditionLookup` manually showing the meta fields.)

---

### 🟢 Low — `meta_json.rs` Module Could Use a Doc Comment

**Severity:** low  
**Location:** `claudine/lib/src/dispatch/runner/meta_json.rs`

The file now contains only `strip_nulls` and its helper. It has no module-level doc comment explaining why the module still exists after the flattening layer was removed. A one-line `//!` comment would help future readers understand this is a utility module retained for report null-stripping.

**Recommended:** Add `//! Utility module for JSON null stripping (retained after flattening removal).`

---

## Test Rigor Assessment

This feature is pure expression-evaluation plumbing. The spec correctly notes: "No Level 2 (terminal) or Level 3 (interactive) tests apply."

| Requirement | Strongest Test Level | Assessment |
|---|---|---|
| `tool_name == 'Bash'` resolves | Level 1 — `when_condition_true_executes_action_and_can_block` | ✅ Appropriate |
| `git.branch == 'main'` resolves | Level 1 — `when_git_branch_matches_main_resolves_truthy` | ✅ Appropriate |
| `hardware.cores > 8` numeric comparison | Level 1 — `when_hardware_cores_numeric_comparison` | ✅ Appropriate |
| `git.is_dirty` boolean usability | Level 1 — `when_git_is_dirty_resolves_as_boolean` | ✅ Appropriate |
| `env.*` fallback (`\|\|`) | Level 1 — `evaluator_handles_fallback_for_missing_env_var` (expression.rs only) | ⚠️ Runner-level test missing (see Findings) |
| `ctx.today` resolves | Level 1 — `when_ctx_today_resolves` | ✅ Appropriate |
| Falsy result skips action | Level 1 — `when_condition_false_skips_call_action_and_no_blocking_response` | ✅ Appropriate |
| Invalid expression warns + skips | Level 1 — `when_invalid_expression_skips_action_non_fatally` | ✅ Appropriate |
| Skipped `Call` cannot replace prior response | Level 1 — `when_skipped_call_does_not_replace_prior_selected_response` | ✅ Appropriate |
| `extra.*`, `tool_input.*`, `tool_response.*` nested JSON | Level 1 — expression.rs unit tests + runner tests for `tool_input` and `extra` | ⚠️ `tool_response` runner test missing (see Findings) |

No Level 2 or Level 3 requirements exist for this feature.

---

## Verification Checklist

- [x] `flatten_event_meta_aliases` and `event_meta_to_json` no longer exist in source
- [x] Hook `when`, templates, matchers, and harness validation derive non-`ctx` paths from `EventMetaExpressionLookup`
- [x] `ctx.*` resolves under hook `when` and remains unresolved elsewhere
- [x] All existing `dispatch::runner::tests::when*` pass
- [x] All `dispatch::template`, `dispatch::matcher`, `harness::validate::tests::render_template` tests pass
- [x] New `when_ctx_today_resolves` test passes
- [x] Skill docs (SKILL.md, architecture.md, hook-actions.md) describe composite adapter
- [x] `cargo clippy -p claudine -p darkmatter -- -D warnings` is clean
- [ ] Darkmatter `CtxLookup` changes are committed to version control
- [ ] `claudine/docs/topics/configuring-actions.md` documents the `when` field
- [ ] Runner tests cover `tool_response.*` and fallback syntax in `when`

---

## Recommendation

**Do not mark production-ready yet.**

The code is architecturally sound and all tests pass in the current working tree, but three issues must be resolved before this feature is "ready":

1. **Commit the Darkmatter changes.** The untracked/modified Darkmatter files are a hard blocker; without them the branch does not build from a clean state.
2. **Document `when` in `configuring-actions.md`.** Users cannot discover conditional execution from the current topic guide.
3. **Strengthen the weak `ctx` regression test** and add the missing `tool_response.*` / fallback runner tests so the full invariant surface is pinned end-to-end.

Once those items are addressed, this feature should be re-reviewed with a focus on the delta.
