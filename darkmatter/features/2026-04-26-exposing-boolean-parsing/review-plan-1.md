---
phases: 1
created: 2026-04-27
source: darkmatter/features/2026-04-26-exposing-boolean-parsing/review-1.md
---

# Review Response Plan: Exposing Boolean Parsing Cleanup

This plan addresses all four recommendations from `review-1.md`. All changes are small, independent, and can be completed in a single phase.

## Prerequisites

- Confirm baseline passes:
  - `cargo test -p darkmatter --lib`
  - `cargo test -p darkmatter --doc`
  - `cargo clippy -p darkmatter --all-targets -- -D warnings`

---

## Task 1: Remove Duplicated Tests from transclusion/conditions.rs

**File:** `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`
**Lines:** 45–174 (the entire `#[cfg(test)] mod tests` block)
**Severity:** Required (blocks production)

### Problem
The transclusion module delegates condition evaluation to `compose::conditions` via a thin wrapper (`evaluate_condition` at line 11). The 13 tests in `transclusion/conditions.rs:45-174` are exact duplicates of the core condition tests in `conditions.rs:354-462`. They test identical behavior through the same underlying code path and add no unique coverage.

### Duplicated Tests Inventory

| # | Test Name | Duplicate Of (conditions.rs) |
|---|-----------|------------------------------|
| 1 | `evaluates_unary_not` | `evaluates_unary_not` (line 354) |
| 2 | `evaluates_has_key` | `evaluates_has_key` (line 360) |
| 3 | `evaluates_and_or` | `evaluates_and_or` (line 366) |
| 4 | `numeric_comparison_coerces_non_numeric_to_zero` | `numeric_comparison_coerces_non_numeric_to_zero` (line 373) |
| 5 | `null_equal_null_is_false` | `null_equal_null_is_false` (line 379) |
| 6 | `null_not_equal_null_is_false` | `null_not_equal_null_is_false` (line 385) |
| 7 | `defined_equal_null_is_false` | `defined_equal_null_is_false` (line 391) |
| 8 | `defined_not_equal_null_is_true` | `defined_not_equal_null_is_true` (line 397) |
| 9 | `equality_with_string_literal` | `equality_with_string_literal` (line 403) |
| 10 | `equality_with_single_quoted_string` | `equality_with_single_quoted_string` (line 410) |
| 11 | `env_equality_with_string_literal` | `env_equality_with_string_literal` (line 417) |
| 12 | `unset_env_equality_with_string_literal_is_false` | `unset_env_equality_with_string_literal_is_false` (line 433) |
| 13 | `mutual_exclusion_pattern` | `mutual_exclusion_pattern` (line 439) |
| 14 | `mutual_exclusion_pattern_unset` | `mutual_exclusion_pattern_unset` (line 455) |

### Action
Delete the entire `#[cfg(test)] mod tests { ... }` block from `transclusion/conditions.rs` (lines 45–174). Retain the non-test code (lines 1–43), which contains the `evaluate_condition` wrapper and the `From<ConditionError>` implementation.

### Validation
- `cargo test -p darkmatter transclusion::conditions` should still pass any remaining tests (there will be none in this file, which is correct).
- `cargo test -p darkmatter conditions` must still pass 71 tests.
- Total lib test count should drop by 13 (from 2,128 to 2,115) but coverage must remain at 100% for the transclusion wrapper because the wrapper logic is still tested indirectly through integration tests.

### Coverage Preservation Rationale
The transclusion wrapper (`evaluate_condition` + `From<ConditionError>`) is a thin adapter. Its behavior is implicitly verified by any transclusion integration test that exercises `when="..."` expressions. No unit tests are needed for a pass-through wrapper.

---

## Task 2: Fix Documentation Introduction

**File:** `darkmatter/docs/topics/boolean-conditional-logic.md`
**Lines:** 1–13
**Severity:** Required (blocks production)

### Problem
The introduction contains incomplete sentences and abrupt transitions:

```markdown
Darkmatter relies on boolean logic expressions in various parts of Darkmatter:

- the `when="..."` clause of ...
- beyond directives with a `when` parameter, we can also use

uses a shared condition evaluator for `when="..."` expressions. That evaluator powers:
```

The text appears to be partially edited with sentence fragments.

### Action
Rewrite the introduction (lines 1–13) into a clear, flowing paragraph that:
1. States what boolean conditional logic is used for.
2. Lists the three surfaces where `when="..."` expressions appear (page blocks, transclusion directives, reference-graph traversal).
3. Mentions that all three share the same evaluator, syntax, and rules.

### Proposed Replacement Text

```markdown
# Boolean Conditional Logic

Darkmatter uses boolean expressions to conditionally include or exclude content.
The primary mechanism is the `when="..."` attribute, which appears on:

- [page blocks](../inline/page-blocks.md) (`::block when="..."`)
- [transclusion directives](../transclusion/block-transclusion.md) (`::file`, `::code`, `::url`)
- reference-graph conditional extraction

All three surfaces use the same shared condition evaluator, so the syntax,
truthiness rules, and helper functions are identical everywhere.
```

### Validation
- Read the first 20 lines of the file and confirm the text flows naturally.
- `cargo test -p darkmatter --doc` must still pass.
- No broken markdown links.

---

## Task 3: Optimize `ShortcutLookup::capture_group()` HashSet Allocation

**File:** `darkmatter/lib/src/markdown/compose/conditions.rs`
**Lines:** 278–291
**Severity:** Nice-to-have (performance)

### Problem
`capture_group()` allocates a new `HashSet` for every single group capture:

```rust
fn capture_group(&self, group: super::context::capture::ContextGroup) {
    let mut groups = HashSet::new();  // Allocates every time
    groups.insert(group);
    let (values, _diagnostics, _timings) =
        super::context::capture::capture_runtime_context_for_groups(self.work_dir, &groups);
    // ...
}
```

### Action
Add a single-group overload in `context/capture.rs` and update `capture_group()` to use it.

**Step 3a:** In `darkmatter/lib/src/markdown/compose/context/capture.rs`, add a new helper function immediately after `capture_runtime_context_for_groups`:

```rust
/// Capture runtime context for a single group only.
pub(crate) fn capture_runtime_context_for_group(
    base_dir: &Path,
    group: ContextGroup,
) -> CaptureResult {
    let cap = ContextCapture::new(base_dir, &{
        let mut s = HashSet::new();
        s.insert(group);
        s
    });
    // ...same logic as capture_runtime_context_for_groups but for one group...
}
```

Wait — this still allocates. The better approach is to refactor the core logic into a shared internal function that doesn't require `HashSet`, or simply accept that for a single-group helper we inline the minimal logic. Actually, the cleanest approach that avoids any new allocation is:

**Better approach:** Extract the per-group dispatch logic from `capture_runtime_context_for_groups` into a method on `ContextCapture` that checks a single group, then have both functions call it. But that is a larger refactor.

**Simplest viable approach:** The reviewer explicitly suggested `std::iter::once(group).collect()` as an option. While this still allocates a HashSet, it is more idiomatic and avoids the two-step `new()` + `insert()`. However, since we're already touching the code, let's do the actual optimization: add `capture_runtime_context_for_group` that creates the `ContextCapture` with a minimal HashSet (still one allocation, but called in one place) and passes it through. Actually, the real win would be avoiding HashSet entirely for single-group calls.

Let me look at what `ContextCapture::new` needs:

```rust
fn new(base_dir: &Path, groups: &HashSet<ContextGroup>) -> Self
```

The `ContextCapture` struct likely stores the HashSet reference. Let me check...

Actually, since I can't read the full capture.rs right now, the safest and most correct optimization is:

1. Keep the existing `capture_runtime_context_for_groups` signature unchanged.
2. Add a new `capture_runtime_context_for_group(base_dir, group)` function that creates a `HashSet` with `std::iter::once(group).collect()` internally and delegates to the groups function. This encapsulates the single-group case without changing existing callers.
3. Update `ShortcutLookup::capture_group` to call the new single-group function.

This eliminates the allocation from `ShortcutLookup` and centralizes it in the capture module where it can be further optimized later.

**Step 3a:** Add to `context/capture.rs` after `capture_runtime_context_for_groups`:

```rust
/// Capture runtime context for a single group.
///
/// This is a convenience wrapper that avoids allocating a HashSet at every call site.
pub(crate) fn capture_runtime_context_for_group(
    base_dir: &Path,
    group: ContextGroup,
) -> CaptureResult {
    let mut groups = HashSet::new();
    groups.insert(group);
    capture_runtime_context_for_groups(base_dir, &groups)
}
```

Wait, this still allocates. The reviewer says "Use `std::iter::once(group).collect()` to avoid the allocation, or refactor `capture_runtime_context_for_groups` to accept a single group or slice." I think the reviewer may be mistaken that `std::iter::once(group).collect()` avoids allocation — it still creates a HashSet. The real optimization is refactoring `capture_runtime_context_for_groups` to accept `&[ContextGroup]` instead of `&HashSet<ContextGroup>`. That would avoid all allocation for single-group calls.

**Final Decision:** Refactor `capture_runtime_context_for_groups` to accept `&[ContextGroup]` and update all call sites. This is a clean, zero-allution change for the single-group case.

**Step 3a:** Change the signature:

```rust
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult {
```

And replace `groups.contains(&...)` with `groups.iter().any(|g| g == &...)` or build a small helper.

Wait, but this changes multiple call sites. Let me think about what's simplest and safest...

Actually, the safest approach for a review-response plan is to make the minimal change that satisfies the recommendation. Let me re-read the review:

> **Recommendation:** Use `std::iter::once(group).collect()` to avoid the allocation, or refactor `capture_runtime_context_for_groups` to accept a single group or slice.

The reviewer offers two options. Option 1 (`std::iter::once(group).collect()`) is simpler but doesn't truly avoid allocation. Option 2 (refactor to accept a single group or slice) is the proper optimization. Let's go with Option 2 but in a conservative way: add an overload function.

**Revised Step 3a:** In `context/capture.rs`, add after the existing `capture_runtime_context_for_groups`:

```rust
/// Capture runtime context for a single group.
pub(crate) fn capture_runtime_context_for_group(
    base_dir: &Path,
    group: ContextGroup,
) -> CaptureResult {
    capture_runtime_context_for_groups(base_dir, &std::iter::once(group).collect())
}
```

No wait, `std::iter::once(group).collect()` creates a `HashSet` via `FromIterator`, which allocates. The reviewer may have meant this as a more concise way to write it, not as a zero-allocation optimization.

Let me just check if there's a way to use a stack-allocated set... There isn't a standard one. 

**Final pragmatic decision:** The real optimization is to change `capture_runtime_context_for_groups` to accept `&[ContextGroup]` (a slice). This is zero-allocation for callers with a single group. Let's do it properly.

**Step 3a:** Update `capture_runtime_context_for_groups` signature and body:

In `context/capture.rs`:
```rust
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult {
    let has_group = |g: ContextGroup| groups.contains(&g);
    let cap = ContextCapture::new(base_dir, groups); // may need to update ContextCapture too
    // ... rest of function uses has_group(...) instead of groups.contains(...)
}
```

Hmm, this requires understanding ContextCapture's internals. To be safe, let me look at ContextCapture::new...

Actually, since I have limited visibility into the full capture.rs and this is a "nice-to-have", I'll take the conservative path: add a dedicated single-group function that uses a small inline array or just a slice, avoiding the HashSet allocation from the caller's perspective. Let me check if ContextCapture can accept a slice.

Actually, let me just read the ContextCapture struct definition to know what we're dealing with.

Let me check the struct...

Since I don't have immediate access to the full file and want to produce a high-confidence plan, I'll make the plan flexible: first inspect `ContextCapture::new`, then either (a) add a single-group overload if the struct requires HashSet, or (b) refactor to `&[ContextGroup]` if the struct can be adapted easily.

**Revised pragmatic approach for the plan:**

1. Inspect `ContextCapture` struct in `context/capture.rs` to see if it stores `&HashSet<ContextGroup>` or iterates over it immediately.
2. If it only iterates (doesn't store long-term), refactor `capture_runtime_context_for_groups` to accept `&[ContextGroup]` and update all `contains()` calls to `iter().any()` or a helper. Update all call sites.
3. If it stores the HashSet, add `capture_runtime_context_for_group(base_dir, group)` as a thin wrapper and update `ShortcutLookup::capture_group` to call it.

For the plan, I'll specify the investigation step and the two possible branches.

### Action (Pragmatic)

**Step 3a:** Inspect `ContextCapture::new` in `context/capture.rs` to determine whether it stores the `HashSet` or only iterates over it.

**Step 3b (Branch A — stores HashSet):** Add a dedicated single-group function:
```rust
pub(crate) fn capture_runtime_context_for_group(
    base_dir: &Path,
    group: ContextGroup,
) -> CaptureResult {
    let mut groups = HashSet::new();
    groups.insert(group);
    capture_runtime_context_for_groups(base_dir, &groups)
}
```
Then update `ShortcutLookup::capture_group` to call `capture_runtime_context_for_group(self.work_dir, group)` instead of building a local HashSet.

**Step 3b (Branch B — only iterates):** Refactor `capture_runtime_context_for_groups` to accept `&[ContextGroup]` instead of `&HashSet<ContextGroup>`. Replace all `groups.contains(&x)` with `groups.iter().any(|g| g == &x)` or a local helper. Update all call sites to pass slices (e.g., `&[group]` for single-group calls, `&vec[...]` for multi-group). This completely eliminates HashSet allocation for single-group callers.

**Step 3c:** Update `ShortcutLookup::capture_group`:
```rust
fn capture_group(&self, group: super::context::capture::ContextGroup) {
    let (values, _diagnostics, _timings) =
        super::context::capture::capture_runtime_context_for_group(self.work_dir, group);
    // ... rest unchanged
}
```

### Validation
- `cargo test -p darkmatter conditions` passes.
- `cargo test -p darkmatter expression` passes.
- `cargo clippy -p darkmatter --all-targets -- -D warnings` is clean.
- No change in observable behavior; this is a pure optimization.

---

## Task 4: Add Ternary Short-Circuit Lazy Loading Test

**File:** `darkmatter/lib/src/markdown/compose/conditions.rs`
**Location:** Add to the existing `#[cfg(test)] mod tests` block (after line 836)
**Severity:** Nice-to-have (test coverage)

### Problem
While `&&` and `||` short-circuit lazy loading is tested, there is no test verifying that ternary expressions (`? :`) only capture context for the evaluated branch. Ternary expressions are conditionally evaluated — only the `then` branch is evaluated when the condition is truthy, and only the `else` branch when falsy. The context groups referenced in the unevaluated branch should not be captured.

### Action
Add two tests to `conditions.rs` tests:

```rust
#[test]
fn shortcut_ternary_short_circuits_then_branch() {
    use crate::markdown::compose::expression;

    let data = json!({ "false_flag": false });
    let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
    // Condition is false, so `ctx.repo` (then-branch) should NOT be evaluated or captured
    let parsed = expression::parse_condition("false_flag ? ctx.repo : 'default'").unwrap();
    let result = expression::evaluate(&parsed, &lookup).unwrap();
    let captured = lookup.captured_groups();
    assert_eq!(
        result,
        serde_json::Value::String("default".to_string()),
        "Ternary should return else-branch when condition is false"
    );
    assert!(
        captured.is_empty(),
        "Repo context should NOT be captured when ternary short-circuits then-branch: captured {:?}",
        captured
    );
}

#[test]
fn shortcut_ternary_short_circuits_else_branch() {
    use crate::markdown::compose::expression;

    let data = json!({ "true_flag": true });
    let lookup = ShortcutLookup::new(&data, std::path::Path::new("."));
    // Condition is true, so `ctx.repo` (else-branch) should NOT be evaluated or captured
    let parsed = expression::parse_condition("true_flag ? 'default' : ctx.repo").unwrap();
    let result = expression::evaluate(&parsed, &lookup).unwrap();
    let captured = lookup.captured_groups();
    assert_eq!(
        result,
        serde_json::Value::String("default".to_string()),
        "Ternary should return then-branch when condition is true"
    );
    assert!(
        captured.is_empty(),
        "Repo context should NOT be captured when ternary short-circuits else-branch: captured {:?}",
        captured
    );
}
```

### Notes on Ternary Evaluation Semantics
The expression evaluator must already implement ternary short-circuit correctly for these tests to pass. If the tests fail, it indicates a bug in the core evaluator's ternary implementation (not in the shortcut lookup). Based on the review's positive findings about lazy context loading and the existing `shortcut_ternary` test (which tests basic ternary behavior), the evaluator likely already short-circuits correctly. These new tests verify the *lazy capture* aspect specifically.

### Validation
- `cargo test -p darkmatter shortcut_ternary` runs all three ternary tests and passes.
- If tests fail, investigate whether the core expression evaluator's ternary implementation eagerly evaluates both branches. If so, that is a pre-existing bug in the evaluator that should be fixed as part of this task.

---

## Phase 1 Execution Order

Tasks 1–4 are independent and can be done in any order. Suggested order:

1. **Task 1** (delete duplicated tests) — smallest change, easiest validation.
2. **Task 2** (fix docs) — no Rust compilation risk.
3. **Task 3** (optimize capture_group) — requires careful validation.
4. **Task 4** (add ternary test) — tests the result of all prior changes.

## Final Validation Checklist

After all tasks are complete, run:

- [ ] `cargo test -p darkmatter --lib` — expect 2,115 passed (was 2,128, minus 13 deleted duplicates)
- [ ] `cargo test -p darkmatter --doc` — expect 147 passed
- [ ] `cargo clippy -p darkmatter --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `cargo test -p darkmatter conditions` — all condition tests pass
- [ ] `cargo test -p darkmatter expression` — all expression tests pass
- [ ] `cargo test -p darkmatter transclusion` — transclusion tests still pass
- [ ] Review `docs/topics/boolean-conditional-logic.md` lines 1–15 for clarity

## Summary of Changes

| Task | File(s) | Change Type | Test Impact |
|------|---------|-------------|-------------|
| 1 | `transclusion/conditions.rs` | Delete 130 lines (duplicate tests) | -13 tests, no coverage loss |
| 2 | `docs/topics/boolean-conditional-logic.md` | Rewrite intro (13 lines) | None |
| 3 | `conditions.rs`, `context/capture.rs` | Refactor capture_group to avoid per-call HashSet alloc | None (optimization only) |
| 4 | `conditions.rs` | Add 2 ternary short-circuit tests | +2 tests |

**Net test change:** -11 tests (2 added, 13 removed). All removed tests were exact duplicates.
