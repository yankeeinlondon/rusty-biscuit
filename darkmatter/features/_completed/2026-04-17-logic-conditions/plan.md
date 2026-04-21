---
phases: 4
created: 2026-04-17
start_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/lexer.rs
    - darkmatter/lib/src/markdown/compose/interpolation/parser.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/conditions.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/interpolation/lexer.rs
    - darkmatter/lib/src/markdown/compose/interpolation/parser.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
    - darkmatter/docs/topics/boolean-conditional-logic.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
    - darkmatter
---

# Execution Plan: Infix Logic Conditions

**Source documents**

- [spec.md](./spec.md)
- [tech-design.md](./tech-design.md)

## Objective

Add `&&` and `||` support to all `when=` condition surfaces while preserving existing `And(...)` / `Or(...)` support and avoiding regressions in general interpolation, where `||` must continue to behave as fallback sugar for `|`.

## Confidence

High. The design keeps the change localized to the existing interpolation lexer/parser, condition evaluation entrypoint, evaluator short-circuit behavior, and documentation. No cache, CLI, or public AST expansion is required.

## Delivery Constraints

- Keep regular interpolation parsing unchanged.
- Restrict infix boolean operators to condition parsing only.
- Lower infix operators into existing `And(...)` / `Or(...)` function-call AST nodes.
- Ensure `And(...)` and `Or(...)` short-circuit the same way as infix forms.

## Phase 1: Establish condition-specific parsing

**Goal:** Create a separate parse mode and condition entrypoint without changing existing interpolation behavior.

**Depends on:** none

**Parallelizable work:** Steps 1.2 and 1.3 can proceed in parallel once step 1.1 lands.

### Step 1.1: Add parse-mode plumbing

**Files**

- `darkmatter/lib/src/markdown/compose/interpolation/mod.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`

**Work**

- Introduce a parse-mode concept that distinguishes interpolation parsing from condition parsing.
- Add a new `parse_condition(input: &str) -> Result<Expr, ParseError>` entrypoint.
- Keep the existing `parse()` entrypoint as the interpolation-mode default.

**Observable outcome**

- The codebase has two explicit parse paths: one for interpolation, one for conditions.

### Step 1.2: Add condition-aware lexer tokens

**Files**

- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`

**Work**

- Add `Token::AndAnd` and `Token::OrOr`.
- Make lexer behavior mode-specific:
    - interpolation mode: `|` and `||` both map to fallback; `&&` stays invalid
    - condition mode: `|` stays fallback, `||` becomes logical OR, `&&` becomes logical AND
- Keep single `&` invalid in all modes.

**Observable outcome**

- Token streams differ by mode exactly where the design requires, with no interpolation regression.

### Step 1.3: Add condition precedence ladder

**Files**

- `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`

**Work**

- Add condition-mode parsing functions for:
    - logical OR
    - logical AND
    - fallback
    - comparison
    - unary
    - primary
- Implement precedence so fallback binds tighter than `&&`, and `&&` binds tighter than `||`.
- Lower `a && b` to `And(a, b)` and `a || b` to `Or(a, b)` using the existing function-call AST representation.

**Observable outcome**

- Condition parse trees can represent grouped and mixed boolean expressions without adding new public `Expr` variants.

### Validation checkpoint

- `cargo test -p darkmatter parser -- --nocapture`
- If parser-specific test filters are not practical, run `cargo test -p darkmatter interpolation`
- Confirm `{{ plan || "plan.md" }}` still parses and `a && b` still fails in interpolation mode.

## Phase 2: Switch condition evaluation to the new parse path

**Goal:** Route all `when=` evaluation through `parse_condition()` and align evaluator semantics.

**Depends on:** Phase 1

**Parallelizable work:** Steps 2.1 and 2.2 are sequential. Step 2.3 can begin once step 2.1 is complete.

### Step 2.1: Update the condition entrypoint

**Files**

- `darkmatter/lib/src/markdown/compose/conditions.rs`

**Work**

- Change `evaluate_condition()` to call `parse_condition()` instead of the default interpolation parser.
- Keep parse failures mapped through the existing `ConditionError::Parse` path.

**Observable outcome**

- Every `when=` condition now uses condition-mode parsing.

### Step 2.2: Add short-circuit evaluation for boolean functions

**Files**

- `darkmatter/lib/src/markdown/compose/conditions.rs`

**Work**

- Update `eval_function()` so `And(...)` and `Or(...)` evaluate operands incrementally.
- Stop evaluation on the first falsy operand for `And(...)`.
- Stop evaluation on the first truthy operand for `Or(...)`.

**Observable outcome**

- Legacy function-call boolean logic and new infix boolean logic share the same runtime behavior.

### Step 2.3: Confirm consumer coverage

**Files to inspect and adjust only if needed**

- `darkmatter/lib/src/markdown/compose/page_blocks/engine.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`
- `darkmatter/lib/src/markdown/reference/graph.rs`

**Work**

- Verify each `when=` surface still routes through `evaluate_condition()`.
- Only patch call sites if any consumer bypasses the shared condition evaluator.

**Observable outcome**

- Page blocks, transclusion, and reference-graph conditional logic all inherit the new behavior from one shared evaluation path.

### Validation checkpoint

- Run targeted condition tests once added.
- Manually validate these behaviors in tests or REPL-style helpers:
    - `false && UnknownFn(x)` returns `false`
    - `true || UnknownFn(x)` returns `true`
    - `And(false, UnknownFn(x))` returns `false`
    - `Or(true, UnknownFn(x))` returns `true`

## Phase 3: Add regression coverage across parser, evaluator, and consumers

**Goal:** Lock in grammar, compatibility, and integration behavior.

**Depends on:** Phase 2

**Parallelizable work:** Steps 3.1, 3.2, and 3.3 can be split across contributors after the parser and evaluator behavior is stable. Step 3.4 should follow the others.

### Step 3.1: Add lexer and parser unit coverage

**Files**

- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`

**Work**

- Add coverage for:
    - `a && b`
    - `a || b`
    - `a && b || c`
    - `a || b && c`
    - `(a || b) && c`
    - `a || (b | c)`
    - interpolation `plan || "plan.md"` still treated as fallback
    - interpolation `a && b` still rejected

**Observable outcome**

- Parser precedence and mode split are enforced by tests.

### Step 3.2: Add condition evaluator unit coverage

**Files**

- `darkmatter/lib/src/markdown/compose/conditions.rs`

**Work**

- Add tests for infix `&&` and `||`.
- Add tests for mixed precedence and explicit grouping.
- Preserve coverage for `And(...)` and `Or(...)`.
- Add regression tests for short-circuit behavior in both infix and function-call forms.

**Observable outcome**

- The evaluator contract is explicit, including error-avoidance via short-circuiting.

### Step 3.3: Add compose-level integration coverage

**Files**

- `darkmatter/lib/src/markdown/compose/mod.rs`
- Existing compose integration test modules under `darkmatter/lib/src/markdown/compose/`

**Work**

- Add or extend compose tests covering:
    - page blocks using `&&`
    - page blocks using `||`
    - transclusion directives using mixed infix logic
    - fallback and infix operators in one condition

**Observable outcome**

- End-to-end compose behavior matches the condition parser and evaluator semantics.

### Step 3.4: Add reference-graph coverage

**Files**

- `darkmatter/lib/src/markdown/reference/graph.rs`
- Existing reference-graph tests

**Work**

- Add at least one test proving conditional extraction respects infix boolean logic the same way compose does.

**Observable outcome**

- All known `when=` consumers are covered by tests, not just compose rendering.

### Validation checkpoint

- `cargo test -p darkmatter logic_conditions`
- If no dedicated filter exists, run `cargo test -p darkmatter compose`
- Run full crate validation before leaving the phase:
    - `cargo test -p darkmatter`

## Phase 4: Document behavior and complete release validation

**Goal:** Update user-facing docs and prove the feature is safe to merge.

**Depends on:** Phase 3

**Parallelizable work:** Documentation updates can happen in parallel with the final full-test run once implementation is stable.

### Step 4.1: Update boolean condition documentation

**Files**

- `darkmatter/docs/topics/boolean-conditional-logic.md`

**Work**

- Document `&&` and `||` as supported condition syntax.
- Retain `And(...)` and `Or(...)` as valid alternatives.
- Document precedence and grouping, including fallback `|`.
- Call out the mode split explicitly:
    - `when="a || b"` means logical OR
    - `{{ a || "default" }}` remains fallback sugar
- Add examples that exercise:
    - boolean infix conditions
    - grouped expressions
    - fallback mixed with boolean logic

**Observable outcome**

- The docs explain the exact compatibility story and operator precedence users need to avoid ambiguity.

### Step 4.2: Run merge-level validation

**Work**

- Run formatter if required by the crate workflow.
- Run final build and test validation for the affected crate.
- Confirm no doc examples contradict runtime behavior.

**Observable outcome**

- The branch is in a releasable state with passing tests and synchronized documentation.

### Validation checkpoint

- `cargo fmt --package darkmatter`
- `cargo test -p darkmatter`
- `cargo build -p darkmatter`

## Dependency Summary

1. Phase 1 must land before any condition consumer can safely switch behavior.
2. Phase 2 must land before integration tests can assert new runtime semantics.
3. Phase 3 should complete before docs are finalized, so examples reflect actual behavior.
4. Phase 4 closes the loop with documentation and full validation.

## Parallel Work Map

- After Step 1.1:
    - lexer token work
    - parser precedence work
- After Step 2.1:
    - evaluator short-circuit work
    - consumer-audit work
- After Phase 2:
    - parser tests
    - evaluator tests
    - compose integration tests
    - reference-graph test
- During Phase 4:
    - docs update
    - final validation run

## Done Criteria

- `when=` conditions accept `&&` and `||` everywhere they previously accepted `And(...)` and `Or(...)`.
- Regular interpolation still treats `||` as fallback and still rejects `&&`.
- `And(...)` and `Or(...)` short-circuit consistently with infix logic.
- Parser, evaluator, compose, and reference-graph tests cover the new behavior.
- Boolean-condition documentation is updated and matches the implementation.
