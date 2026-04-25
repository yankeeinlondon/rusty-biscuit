---
phases: 5
created: 2026-04-24
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/interpolation/lexer.rs
  - darkmatter/lib/src/markdown/compose/interpolation/parser.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/ast.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/interpolation/parser.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/interpolation/ast.rs
  - darkmatter/lib/src/markdown/compose/interpolation/parser.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - darkmatter
---

# Execution Plan: Consistent Use of Logic Operators

This plan implements the removal of bare `|` as a valid operator across all darkmatter expression modes, making `||` the canonical spelling for fallback (interpolation) and logical OR (conditions).

**Source Documents:**
- Functional Spec: `spec.md`
- Technical Design: `tech-design.md`

---

## Phase 1: Lexer Foundation

**Goal:** Update tokenization so bare `|` produces an actionable parse error in both interpolation and condition modes, while `||` continues to work.

### Step 1.1: Update Lexer Token Contract
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- **Action:** Modify the `|` branch in `Lexer::next_token()`:
  - If `||` is encountered, emit the mode-appropriate token (`Token::Pipe` for interpolation, `Token::OrOr` for condition)
  - If bare `|` is encountered, emit a `LexerError` with the mode-appropriate hint message
- **Observable:** Single `|` in any expression mode should trigger a lexer error with the message "Unexpected '|'. Use '||' for fallback." (interpolation) or "Unexpected '|'. Use '||' for logical OR." (condition)

### Step 1.2: Update ParseMode Documentation
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- **Action:** Refresh rustdoc and inline comments on `ParseMode` to state:
  - Interpolation mode: `||` maps to `Token::Pipe`; bare `|` and `&&` are invalid
  - Condition mode: `||` maps to `Token::OrOr`; `&&` maps to `Token::AndAnd`; bare `|` is invalid
- **Observable:** `ParseMode` docs no longer reference bare `|` as a valid operator

### Step 1.3: Update Token::Pipe Documentation
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- **Action:** Update rustdoc on `Token::Pipe` from "The pipe operator `|`" to "The `||` fallback operator in interpolation mode"
- **Observable:** `Token::Pipe` docs accurately describe the new token contract

#### Checkpoint 1-A
Run lexer-focused tests. All existing tests using `||` should pass. No tests using bare `|` should pass yet (they will be updated in Phase 4).

```sh
cargo test -p darkmatter interpolation::lexer
```

---

## Phase 2: Parser Changes

**Goal:** Remove condition-mode fallback support and update grammar comments. Depends on Phase 1.

### Step 2.1: Update Interpolation Parser Grammar Comments
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
- **Action:** Update `parse_fallback()` grammar comments to read `fallback = comparison ("||" comparison)*`
- **Observable:** Parser comments no longer mention bare `|` for fallback

### Step 2.2: Remove Condition-Mode Fallback
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
- **Action:** In condition mode, update `parse_logical_and()` to call `parse_comparison()` directly instead of `parse_fallback()`
- **Observable:** `when="a | b"` now fails at the lexer (from Phase 1) and never reaches the fallback parser path

### Step 2.3: Update Condition Error Hints
- **File:** `darkmatter/lib/src/markdown/compose/conditions.rs`
- **Action:** Update `ConditionError` operator hint to remove any reference to bare fallback and list: `Operators: &&  ||  !  ==  !=  >  >=  <`
- **Observable:** Condition parse error messages no longer suggest `|` as a valid operator

#### Checkpoint 2-A
Run parser-focused tests. Interpolation `||` tests should pass. Condition `||` and `&&` tests should pass.

```sh
cargo test -p darkmatter interpolation::parser
cargo test -p darkmatter markdown::compose::conditions
```

---

## Phase 3: Evaluator and Error Path Verification

**Goal:** Ensure evaluator semantics remain correct and error messages flow properly. Depends on Phase 2.

### Step 3.1: Verify Interpolation Evaluator
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`
- **Action:** Confirm `Expr::Fallback` evaluation logic is unchanged (evaluate primary, return if truthy, otherwise evaluate and return fallback)
- **Observable:** `{{ missing || "default" }}` renders as "default"; `{{ primary || "default" }}` renders as "primary"

### Step 3.2: Verify Condition Evaluator
- **File:** `darkmatter/lib/src/markdown/compose/conditions.rs`
- **Action:** Confirm `||` evaluates as boolean OR with short-circuiting and `&&` evaluates as boolean AND with short-circuiting
- **Observable:** `when="a || b"` evaluates as boolean OR; `when="a && b"` evaluates as boolean AND

### Step 3.3: Verify Error Reporting Flow
- **Files:** `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`, `darkmatter/lib/src/markdown/compose/conditions.rs`
- **Action:** Confirm bare-pipe lexer errors flow through the correct error paths:
  - Interpolation errors for `{{ ... }}`
  - `ConditionError::Parse` for `when="..."` (rendered through `BlockError`)
- **Observable:** Bare `|` in both expression contexts produces a clear, actionable error message with position information

#### Checkpoint 3-A
Run evaluator and integration tests.

```sh
cargo test -p darkmatter interpolation::evaluator
cargo test -p darkmatter markdown::compose::tests::infix_logic_conditions
```

---

## Phase 4: Test Migration and New Rejection Tests

**Goal:** Update all existing tests to use `||` and add tests that verify bare `|` is rejected. Can be parallelized with Phase 5. Depends on Phases 1-3.

### Step 4.1: Migrate Lexer Tests
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- **Action:**
  - Update all existing tests using `|` for fallback to use `||`
  - Add rejection test: interpolation mode rejects `a | b`
  - Add rejection test: condition mode rejects `a | b`
  - Verify: string literals containing pipes still tokenize correctly (`"a | b"`, `"a || b"`)
- **Observable:** All lexer tests pass, including new rejection tests

### Step 4.2: Migrate Parser Tests
- **File:** `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
- **Action:**
  - Update all existing parser tests using `|` to use `||`
  - Add test: `parse(r#"foo || "default""#)` returns `Expr::Fallback`
  - Add test: `parse(r#"foo | "default""#)` returns `ParseError`
  - Add test: chained interpolation fallback uses `||`
  - Add test: `parse_condition("a || b")` returns lowered `Or(...)`
  - Add test: `parse_condition("a && b")` returns lowered `And(...)`
  - Add test: `parse_condition("a | b")` returns `ParseError`
  - Add test: `parse_condition("(a || b) && c")` preserves precedence
  - Add test: `parse_condition("a || b && c")` parses as `Or(a, And(b, c))`
- **Observable:** All parser tests pass, including new rejection and precedence tests

### Step 4.3: Migrate Evaluator and Compose Tests
- **Files:** `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`, `darkmatter/lib/src/markdown/compose/mod.rs`
- **Action:**
  - Update all existing tests using `|` for fallback to use `||`
  - Add test: `{{ missing || "default" }}` renders "default"
  - Add test: `{{ primary || "default" }}` renders "primary"
  - Add test: `{{ missing || backup || "default" }}` uses first truthy value
  - Add test: `{{ missing | "default" }}` produces the new parse error
  - Add test: `::block when="a || b"` keeps logical OR behavior
  - Add test: `::block when="a && b"` keeps logical AND behavior
  - Add test: `::block when="a | b"` fails with `ConditionError::Parse`
- **Observable:** All evaluator and compose integration tests pass

### Step 4.4: Add Shell Regression Tests
- **File:** `darkmatter/lib/src/markdown/compose/mod.rs` (or appropriate test file)
- **Action:**
  - Add/verify test: `key: "$(echo a | cat)"` remains rejected by shell tokenizer
  - Add/verify test: `key: "$(false || echo fallback)"` remains rejected by shell tokenizer
- **Observable:** Shell tokenizer behavior is unchanged; pipe characters in `$(...)` are still rejected

#### Checkpoint 4-A
Run full test suite for darkmatter compose and interpolation.

```sh
cargo test -p darkmatter interpolation::
cargo test -p darkmatter markdown::compose::conditions
cargo test -p darkmatter markdown::compose::tests::infix_logic_conditions
```

---

## Phase 5: Documentation and Skill Updates

**Goal:** Update all active documentation, rustdoc, and skill content. Can be partially parallelized with Phase 4.

### Step 5.1: Audit Expression Usage
- **Action:** Run grep to find all instances of bare `|` in expressions across docs and skills

```sh
rg -n '\{\{[^}\n]*\|[^|}]' darkmatter .claude/skills/darkmatter
rg -n 'when="[^"]*\|[^|"]*"' darkmatter .claude/skills/darkmatter
rg -n 'fallback.*\\\||\|.*fallback' darkmatter/docs .claude/skills/darkmatter
```

- **Observable:** A complete list of files and line numbers requiring updates

### Step 5.2: Update Skill Documentation
- **Files:**
  - `.claude/skills/darkmatter/SKILL.md`
  - `.claude/skills/darkmatter/compose.md`
- **Action:**
  - Replace all bare `|` fallback examples with `||`
  - Update operator tables to remove bare `|` row
  - Update text explaining `||` semantics per mode
- **Observable:** Skill docs no longer reference bare `|` as a valid operator

### Step 5.3: Update Topic Documentation
- **Files:**
  - `darkmatter/docs/inline/interpolation.md`
  - `darkmatter/docs/inline/fm-interpolation.md`
  - `darkmatter/docs/topics/boolean-conditional-logic.md`
- **Action:**
  - Replace all bare `|` fallback examples with `||`
  - Rewrite the mode-comparison table at lines 91-94 in `boolean-conditional-logic.md`
  - Add explicit table showing `||` has mode-specific semantics:
    | Surface | `||` meaning |
    | --- | --- |
    | `{{ ... }}` | fallback, first truthy value wins |
    | `when="..."` | logical OR, returns a boolean |
- **Observable:** All docs use `||` for fallback and correctly explain mode-specific semantics

### Step 5.4: Update Rustdoc Examples
- **Files:**
  - `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
  - `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
  - `darkmatter/lib/src/markdown/compose/interpolation/mod.rs`
  - `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`
  - `darkmatter/lib/src/markdown/compose/interpolation/ast.rs`
- **Action:** Replace all bare `|` examples in rustdoc comments with `||`
- **Observable:** All rustdoc examples compile and demonstrate correct syntax

### Step 5.5: Update Compose Tests and Examples
- **File:** `darkmatter/lib/src/markdown/compose/mod.rs`
- **Action:** Update any test comments or example strings that use bare `|` for fallback
- **Observable:** All compose module tests and examples use `||`

#### Checkpoint 5-A
Verify documentation consistency.

```sh
rg -n '\{\{[^}\n]*\|[^|}]' darkmatter .claude/skills/darkmatter
rg -n 'when="[^"]*\|[^|"]*"' darkmatter .claude/skills/darkmatter
```

Both commands should return zero results in active documentation (excluding `_completed/` and historical docs).

---

## Phase 6: Final Validation and Integration

**Goal:** Run full test suite and verify no regressions. Depends on completion of all prior phases.

### Step 6.1: Run Full Darkmatter Test Suite
```sh
cargo test -p darkmatter
```
- **Observable:** All tests pass

### Step 6.2: Verify No Bare Pipe in Active Code
```sh
rg -n '\{\{[^}\n]*\|[^|}]' darkmatter/lib/src
rg -n 'when="[^"]*\|[^|"]*"' darkmatter/lib/src
```
- **Observable:** No bare `|` usage remains in active source code or tests

### Step 6.3: Verify Shell Tokenizer Unchanged
```sh
cargo test -p darkmatter shell
```
- **Observable:** Shell tokenizer tests pass; no regression in frontmatter shell expression handling

### Step 6.4: Verify Lint and Typecheck
If available in the darkmatter package area:
```sh
just lint   # or cargo clippy -p darkmatter
just test   # or cargo test -p darkmatter
```
- **Observable:** No clippy warnings or test failures introduced

#### Checkpoint 6-A (Final)
All tests pass, documentation is consistent, and the operator surface is unified.

---

## Parallelization Opportunities

| Phase | Parallelizable Work |
|-------|---------------------|
| Phase 1 | Steps 1.1, 1.2, 1.3 can be done in any order within the same file |
| Phase 4 | Steps 4.1-4.4 can be worked on in parallel by different contributors |
| Phase 5 | Steps 5.2-5.5 can be worked on in parallel with Phase 4 (after Step 5.1 audit) |

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Condition fallback semantics confusion | Confirm Option A (logical OR only) before implementing Phase 2. The spec says `||` should always mean logical OR; condition-mode `||` remains boolean OR |
| Accidental edits to unrelated pipe syntax | Use targeted regex in audit (Step 5.1); manually review results; do not edit `::toc-linking`, Markdown tables, or shell commands |
| `Token::Pipe` naming inconsistency | Acceptable for minimal diff; ensure docs/comments are updated so future work does not reintroduce bare-pipe assumptions |
| Breaking change for document authors | Provide clear error messages (Phase 1); update all in-tree docs/examples (Phase 5); call out in CHANGELOG |

## Completion Criteria

- [ ] Bare `|` produces a parse error in interpolation expressions with the message "Unexpected '|'. Use '||' for fallback."
- [ ] Bare `|` produces a parse error in condition expressions with the message "Unexpected '|'. Use '||' for logical OR."
- [ ] `||` continues to work as fallback in interpolation mode
- [ ] `||` continues to work as logical OR in condition mode
- [ ] `&&` continues to work as logical AND in condition mode
- [ ] All active documentation, rustdoc, and skill examples use `||` for fallback
- [ ] All tests pass (including new rejection tests)
- [ ] Shell tokenizer behavior is unchanged
- [ ] No bare `|` usage remains in active source code, tests, or documentation
