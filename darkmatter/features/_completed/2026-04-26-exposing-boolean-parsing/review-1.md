---
ready: false
agent: ${env.AGENT}
---

# Review: Exposing Boolean Parsing Feature

## Summary

This feature successfully extracts the boolean conditional logic from `conditions.rs` and the interpolation evaluator into a unified `compose::expression` module, exposing a clean shortcut API (`evaluate_condition_against`) for external callers. All 2,128 lib tests and 147 doctests pass. The implementation correctly handles lazy context loading and short-circuit evaluation.

However, **cleanup tasks identified in the design phase remain incomplete**, preventing production readiness.

## Verification Results

### Test Results
- ✅ `cargo test -p darkmatter conditions` - 71 passed
- ✅ `cargo test -p darkmatter interpolation` - 207 passed  
- ✅ `cargo test -p darkmatter expression` - 178 passed
- ✅ `cargo test -p darkmatter --lib` - 2,128 passed
- ✅ `cargo test -p darkmatter --doc` - 147 passed
- ✅ `cargo clippy -p darkmatter --all-targets -- -D warnings` - clean

### Architecture Implementation

| Design Requirement | Status | Location |
|-------------------|--------|----------|
| Extract core expression module | ✅ Done | `compose/expression/{ast,lexer,parser,mod}.rs` |
| Rename `InterpolationLookup` to `EvaluationLookup` | ✅ Done | `expression/mod.rs:82` with backward-compatible alias in `interpolation/mod.rs:74` |
| Refactor conditions.rs to use core evaluator | ✅ Done | `conditions.rs:8,124` |
| Refactor interpolation evaluator | ✅ Done | `interpolation/evaluator.rs:70,240,264` |
| Implement shortcut API | ✅ Done | `conditions.rs:207-232` |
| Lazy context loading | ✅ Done | `conditions.rs:237-246,278-291` |
| Documentation updates | ⚠️ Partial | `docs/topics/boolean-conditional-logic.md` has quality issues |
| Remove duplicated tests | ❌ Not done | `transclusion/conditions.rs:45-174` still contains 13 duplicate tests |

## Issues Found

### 2. Documentation Quality Issue

**Severity:** Low (cosmetic but noticeable)

`darkmatter/docs/topics/boolean-conditional-logic.md` lines 1-13 contain incomplete/awkward phrasing:

```markdown
Darkmatter relies on boolean logic expressions in various places in Darkmatter:

- the `when="..."` clause of ...
- beyond directives with a `when` parameter, we can also use 

uses a shared condition evaluator for `when="..."` expressions. That evaluator powers:
```

The text appears to be partially edited with sentence fragments and abrupt transitions.

**Recommendation:** Rewrite the introduction for clarity and flow.

### 3. Performance: Unnecessary HashSet Allocation

**Severity:** Low (minor optimization)

`ShortcutLookup::capture_group()` allocates a new `HashSet` for every single group capture:

```rust
fn capture_group(&self, group: super::context::capture::ContextGroup) {
    let mut groups = HashSet::new();  // Allocates every time
    groups.insert(group);
    let (values, _diagnostics, _timings) =
        super::context::capture::capture_runtime_context_for_groups(self.work_dir, &groups);
    // ...
}
```

**Recommendation:** Use `std::iter::once(group).collect()` to avoid the allocation, or refactor `capture_runtime_context_for_groups` to accept a single group or slice.

### 4. Missing Test: Ternary Short-Circuit Lazy Loading

**Severity:** Low (test coverage gap)

While short-circuit lazy loading is tested for `&&` and `||` operators, there's no test verifying that ternary expressions only capture context for the evaluated branch:

```rust
// Should NOT capture ctx.repo because condition is false
let result = evaluate_condition_against(
    "false ? ctx.repo : 'default'",
    &data,
    Path::new("."),
);
```

**Recommendation:** Add a test to ensure ternary expressions don't eagerly capture both branches' context groups.

## Positive Findings

### 1. Core Architecture Well-Implemented

The expression module extraction is clean and follows Rust best practices:
- Proper module hierarchy with `pub mod expression` in `compose/mod.rs:53`
- Core types exported at appropriate visibility levels
- Backward-compatible re-exports in `interpolation/mod.rs:68-76`

### 2. Lazy Context Loading Correctly Implemented

`ShortcutLookup` correctly implements lazy loading:
- Only captures `ctx.*` groups when referenced
- Respects short-circuit evaluation (verified by tests `shortcut_and_short_circuits_prevents_ctx_capture` and `shortcut_or_short_circuits_prevents_ctx_capture`)
- Unknown context keys don't trigger capture (test: `shortcut_unknown_ctx_key_does_not_capture`)
- Same group captured only once per evaluation (test: `shortcut_same_group_captured_only_once`)

### 3. Error Handling Consistent

`ConditionError` properly implements `biscuit_terminal::errors::BlockError` for rich terminal rendering, as specified in the design:

```rust
impl biscuit_terminal::errors::BlockError for ConditionError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock { ... }
}
```

### 4. Shortcut API Ergonomic

The `evaluate_condition_against` function signature is clean and easy to use:

```rust
pub fn evaluate_condition_against(
    expr: &str,
    data: &Value,
    work_dir: &Path,
) -> Result<bool, ConditionError>
```

The doctest example compiles and runs correctly, demonstrating the intended use case.

### 5. Variable Resolution Order Correct

The shortcut lookup implements the correct resolution order:
1. Top-level and nested paths from provided `data`
2. `env.*` from system environment
3. `ctx.*` via lazy runtime context capture
4. Unprefixed missing keys fall back to `ctx.*`

This matches the behavior of `EffectiveState` as required.

## Recommendations

### Before Production

1. **Remove duplicated tests** from `translocation/conditions.rs` (lines 45-174)
2. **Fix documentation** introduction in `docs/topics/boolean-conditional-logic.md`

### Nice-to-Have

3. Optimize `capture_group` to avoid HashSet allocation
4. Add ternary short-circuit lazy loading test

## Conclusion

The feature is **functionally complete and all tests pass**, but cleanup tasks explicitly identified in the design phase remain undone. The duplicated tests represent maintenance debt and the documentation quality issue is noticeable. Once these two items are addressed, the feature will be production-ready.

**Verdict:** `ready: false` - Minor cleanup required before production deployment.
