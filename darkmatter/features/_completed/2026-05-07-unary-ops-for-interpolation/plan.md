---
phases: 5
created: 2026-05-07
start_phase: 1
---

# Execution Plan: Unary Conditionals (Nested Ternaries) for Interpolation

## Overview

Enable nested/recursive ternary expressions in Darkmatter interpolation so that any branch of a `{cond} ? {when_truthy} : {when_falsy}` expression may itself contain another ternary. Parentheses for visual grouping are already supported by the existing primary-expression parser and require no code changes.

## Phase 1: Analysis & Design

- [ ] Read and confirm understanding of `parse_ternary` / `parse_ternary_branch` in `darkmatter/lib/src/markdown/compose/expression/parser.rs`
- [ ] Confirm the exact grammar change: `ternary = fallback ("?" ternary ":" ternary)?`
- [ ] Verify evaluator already recursively evaluates `Expr::Ternary` (no evaluator changes needed)
- [ ] List all test cases from the functional spec (valid nested patterns, invalid parenthesis groupings)
- [ ] Identify documentation files to update

## Phase 2: Parser Implementation

- [ ] Modify `parse_ternary` in `parser.rs` so `then_branch` and `else_branch` are parsed via `parse_ternary()` instead of `parse_ternary_branch()`
- [ ] Update the grammar comment block at the top of `parser.rs` to reflect recursive ternary rule
- [ ] Run `cargo check -p darkmatter` to verify compilation

## Phase 3: Unit & Regression Testing

- [ ] Add parser unit tests for:
  - `a ? b ? c : d : e` (nested in true branch)
  - `a ? b : c ? d : e` (nested in false branch)
  - `a ? (b ? c : d) : e` (parenthesized nested ternary)
  - Deep nesting: `a ? b ? c ? d : e : f : g`
- [ ] Add parser error-case tests for:
  - `a ? (b : c)` — unclosed / invalid grouping (expect error)
  - `a ? b) : c` — unmatched paren (expect error)
  - `a ? (b ? c : d` — unbalanced paren (expect error)
- [ ] Add evaluator unit tests for:
  - Nested truthy/falsy resolution
  - Mixed variables and string literals in nested branches
- [ ] Run `just test darkmatter` to confirm all existing tests still pass
- [ ] Run `just lint darkmatter` to confirm no new warnings

## Phase 4: Integration & End-to-End Validation

- [ ] Add an integration test in `darkmatter/lib/tests/` (or appropriate location) that runs `interpolate_text` with a nested ternary against `EffectiveState`
- [ ] Validate that `{{ctx.current_package}} ? 'in a package directory: {{ctx.current_package}}' : 'not in a package directory'` produces correct output
- [ ] Confirm frontmatter interpolation (`fm-interpolation`) also supports nested ternaries (it reuses the same parser)
- [ ] Run full test suite: `just test`

## Phase 5: Documentation Updates

- [ ] Update `darkmatter/docs/inline/interpolation.md` to document nested ternary syntax and provide examples
- [ ] Update `darkmatter/docs/topics/boolean-conditional-logic.md` to mention that interpolation ternaries now support nesting
- [ ] Update module-level doc comments in `parser.rs` and `interpolation/mod.rs` with nested-ternary examples
- [ ] Run `cargo doc -p darkmatter --no-deps` to verify doc generation succeeds

## Parallelizable Work

- Phase 2 and Phase 3 test authoring can be done in parallel by different contributors (parser tests vs evaluator tests).
- Phase 5 documentation can begin as soon as Phase 2 is complete (no need to wait for Phase 4).

## Validation Checkpoints

1. **After Phase 2**: `cargo check -p darkmatter` compiles without errors.
2. **After Phase 3**: `just test darkmatter` passes with 100% success rate.
3. **After Phase 4**: `just test` passes for the entire workspace.
4. **After Phase 5**: `cargo doc -p darkmatter --no-deps` succeeds and produced docs include the new examples.

## Files to Modify

| File | Purpose |
|------|---------|
| `darkmatter/lib/src/markdown/compose/expression/parser.rs` | Grammar change + new tests |
| `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs` | New nested-ternary eval tests |
| `darkmatter/lib/src/markdown/compose/expression/mod.rs` | Update module docs if needed |
| `darkmatter/lib/src/markdown/compose/interpolation/mod.rs` | Update module docs with nested examples |
| `darkmatter/docs/inline/interpolation.md` | User-facing docs |
| `darkmatter/docs/topics/boolean-conditional-logic.md` | Cross-reference update |
