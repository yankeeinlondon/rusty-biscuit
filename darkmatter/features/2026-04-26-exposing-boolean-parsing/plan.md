---
phases: 7
created: 2026-04-27
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - darkmatter/features/2026-04-26-exposing-boolean-parsing/plan.md
docs_created_during_phase_1:
  - darkmatter/features/2026-04-26-exposing-boolean-parsing/phase1-notes.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-04-26-exposing-boolean-parsing/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/darkmatter/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-04-26-exposing-boolean-parsing/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/context/capture.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-04-26-exposing-boolean-parsing/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .opencode/skill/darkmatter/SKILL.md
packages:
  - darkmatter
---

# Execution Plan: Exposing Boolean Parsing

## Phase 1: Baseline and Compatibility Map

Goal: establish the exact current behavior that must survive the refactor.

1. Inventory public and internal entry points:
   - `darkmatter/lib/src/markdown/compose/conditions.rs`
   - `darkmatter/lib/src/markdown/compose/interpolation/{ast,lexer,parser,evaluator,rewrite,mod}.rs`
   - `darkmatter/lib/src/markdown/compose/state.rs`
   - `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
   - call sites from page blocks, transclusion, shell expansion discovery, and compose orchestration.
2. Record the existing exports from `compose/mod.rs` and `compose/interpolation/mod.rs`, especially `conditions`, `evaluate_condition`, `parse_condition`, `Expr`, `ComparisonOp`, `Evaluator`, and `InterpolationLookup`.
3. Capture current behavior from tests for:
   - truthiness of null, booleans, numbers, strings, arrays, and objects
   - missing-value equality and inequality
   - numeric coercion
   - interpolation fallback semantics
   - condition `&&` / `||` short-circuit semantics
   - helper functions: `And`, `Or`, `HasKey`, `Contains`, `Length`, `number`, `round`
   - `env.*`, `ctx.*`, and unprefixed fallback-to-`ctx.*` lookup behavior.
4. Identify any duplicated tests that should be moved from condition/interpolation modules into the new expression module after extraction.

Validation checkpoint:

- Run `cargo test -p darkmatter conditions interpolation` or the closest package-specific filtered test command available.
- Save the failing/passing baseline in implementation notes before changing behavior.

Parallelizable work:

- Steps 1-3 can be split across separate readers because they only inspect files and tests.

## Phase 2: Extract the Core Expression Module

Goal: create `compose::expression` as the shared parser/evaluator foundation without changing behavior.

1. Add `darkmatter/lib/src/markdown/compose/expression/`.
2. Move or re-home the shared pieces from `interpolation` into `expression`:
   - `ast.rs`
   - `lexer.rs`
   - `parser.rs`
   - shared value conversion and truthiness helpers
   - shared function implementations where behavior is identical.
3. Rename `InterpolationLookup` to `EvaluationLookup` in the new module.
4. Keep the lookup contract path-based:
   - `get(&self, path: &str) -> Option<Value>`
   - string coercion can remain as a default/helper method if interpolation still needs it.
5. Preserve parser modes:
   - interpolation mode keeps `||` as fallback.
   - condition mode keeps `||` as logical OR and `&&` as logical AND.
6. Export the new module from `compose/mod.rs` with `pub mod expression;`.
7. Add compatibility re-exports from `compose/interpolation/mod.rs` for any existing public names that should not break immediately, including `Expr`, `ComparisonOp`, parser functions, and the old trait alias if practical.

Validation checkpoint:

- `cargo test -p darkmatter markdown::compose::interpolation`
- `cargo test -p darkmatter markdown::compose::conditions`
- Confirm no public doc examples fail because paths moved.

Parallelizable work:

- Moving AST/lexer/parser and moving evaluator helpers are separable if only one person owns final module exports.

## Phase 3: Unify Condition and Interpolation Evaluation

Goal: remove duplicated evaluation logic while preserving the two public semantics.

1. Implement a core expression evaluator in `compose::expression` that evaluates `Expr` to `serde_json::Value`.
2. Ensure core evaluation owns the shared behavior:
   - literals
   - variables through `EvaluationLookup`
   - unary not
   - ternary
   - comparisons
   - helper functions
   - short-circuit evaluation for `And` and `Or`
   - logical condition-mode operators lowered by the parser.
3. Refactor `conditions.rs` so `evaluate_condition(expr, state, line)`:
   - parses with condition mode from `expression`
   - evaluates with the core evaluator
   - converts the final value through shared truthiness
   - keeps `ConditionError` and its `BlockError` implementation unchanged.
4. Refactor `interpolation/evaluator.rs` so interpolation:
   - uses the core evaluator for expression evaluation
   - preserves `EvalResult`, `EvalValue`, and string output behavior for existing callers
   - preserves unresolved variables as empty strings in interpolation output.
5. Update `EffectiveState` and frontmatter seed state implementations from `InterpolationLookup` to `EvaluationLookup`.
6. Remove obsolete duplicated helper implementations from `conditions.rs` and `interpolation/evaluator.rs` after tests prove parity.

Validation checkpoint:

- Existing condition and interpolation tests must pass unchanged, except import paths if unavoidable.
- Add targeted parity tests proving condition and interpolation use the same comparison/function behavior where their semantics overlap.
- Add regression tests for `{{ a || "default" }}` versus `when="a || b"` so the mode split stays explicit.

Parallelizable work:

- Condition refactor and interpolation refactor can proceed in parallel after the core evaluator API is stable.

## Phase 4: Implement the Shortcut API

Goal: expose boolean evaluation against plain data without requiring callers to construct `EffectiveState`.

1. Add the public function in `compose::conditions`:

   ```rust
   pub fn evaluate_condition_against(
       expr: &str,
       data: &serde_json::Value,
       work_dir: &std::path::Path,
   ) -> Result<bool, ConditionError>
   ```

2. Implement a shortcut lookup type that implements `EvaluationLookup` and resolves:
   - unprefixed top-level and nested paths from `data`
   - `env.*` from `std::env`
   - `ctx.*` through lazy runtime context capture based on the referenced context group
   - unprefixed missing keys through the same fallback-to-`ctx.*` behavior as `EffectiveState`.
3. Keep `data` immutable and borrowed; do not clone the full payload unless a returned `Value` requires cloning.
4. Preserve the current `ConditionError` shape:
   - parse errors use the input expression and line `1`
   - eval errors use the input expression and line `1`.
5. Re-export the shortcut from the same public surface as `evaluate_condition`.

Validation checkpoint:

- Unit tests call the new shortcut with `serde_json::json!` payloads and no `ComposeContext`.
- Tests cover top-level lookup, nested lookup, env lookup, ctx lookup, missing values, comparisons, helpers, and short-circuit behavior.
- A compile check confirms external callers can import `darkmatter::markdown::compose::conditions::evaluate_condition_against`.

Parallelizable work:

- Shortcut API wiring and tests for plain-data behavior can be done in parallel with lazy context lookup tests once the lookup type exists.

## Phase 5: Prove Lazy Context Resolution

Goal: guarantee the shortcut does not perform expensive runtime capture unless the expression requires it.

1. Prefer reusing existing context group logic from `compose::context::capture` instead of inventing a separate mapping.
2. If necessary, expose a crate-internal function that maps a single `ctx` key to its `ContextGroup`, keeping `ContextGroup` itself private unless a cleaner API already exists.
3. Add a cache inside the shortcut lookup:
   - no context capture for expressions without `ctx.*` or missing unprefixed context fallbacks
   - one capture per needed group per evaluation
   - no repeated capture for multiple fields in the same group.
4. Ensure `ctx.today`, `ctx.year`, and date/time keys remain cheap and do not trigger git, docs, OS, or hardware probes.
5. Add instrumentation-friendly tests using a test-only lookup/capture hook or counters if direct subprocess avoidance is hard to assert.
6. Add short-circuit tests:
   - `false_flag && ctx.repo == "x"` must not capture repo context.
   - `true_flag || ctx.gpu` must not capture hardware/GPU context.
   - `draft == true` must not capture any runtime context.
   - `missing_repo_name` may fall back to `ctx.missing_repo_name` but must not capture unknown runtime groups.

Validation checkpoint:

- Tests must fail if eager `ComposeContext::capture_for_dir` is used by the shortcut.
- Run a focused test with tracing or counters showing zero expensive context groups for plain-data expressions.

Parallelizable work:

- Context-group API adjustment and shortcut lazy-cache implementation can proceed in parallel if the interface is agreed first.

## Phase 6: Documentation and Public API Polish

Goal: make the new API discoverable and keep public docs aligned with behavior.

1. Update Rustdoc for:
   - `compose::expression`
   - `EvaluationLookup`
   - `conditions::evaluate_condition`
   - `conditions::evaluate_condition_against`.
2. Update `darkmatter/docs/topics/boolean-conditional-logic.md` with:
   - the new shortcut API
   - a plain `serde_json::Value` example
   - lazy `ctx.*` behavior
   - the fact that `ConditionError` still implements `biscuit_terminal::errors::BlockError`.
3. Update `darkmatter/README.md` only if it already advertises condition/interpolation APIs.
4. Update `.claude/skills/darkmatter/SKILL.md` if the architecture or preferred usage changes for future agents.
5. Confirm no dependency documentation needs changes because the feature keeps the existing `biscuit-terminal` dependency.

Validation checkpoint:

- `cargo test -p darkmatter --doc`
- `rg "InterpolationLookup|EvaluationLookup|evaluate_condition_against|boolean" darkmatter/docs darkmatter/lib/src/markdown/compose .claude/skills/darkmatter`
- Verify rustdoc section headings follow the repo convention: summary, `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, `Notes`.

Parallelizable work:

- Rustdoc updates and topic documentation can be drafted in parallel after the public API signature is final.

## Phase 7: Full Verification and Cleanup

Goal: finish with a clean, observable implementation that is safe to merge.

1. Run formatting:
   - `cargo fmt --package darkmatter` if supported
   - otherwise `cargo fmt`.
2. Run focused tests:
   - expression parser/evaluator tests
   - condition tests
   - interpolation tests
   - page block conditional tests
   - transclusion conditional tests.
3. Run package-level validation:
   - `cargo test -p darkmatter`
   - `cargo clippy -p darkmatter --all-targets -- -D warnings` if this is the package lint convention.
4. Run doctests:
   - `cargo test -p darkmatter --doc`.
5. Inspect public API fallout:
   - search for stale module paths and trait names
   - verify compatibility re-exports are intentional
   - ensure no duplicate evaluator logic remains except thin adapters.
6. Review performance-sensitive code:
   - no unconditional `ComposeContext::capture()` or `capture_for_dir()` in the shortcut path
   - no context capture before parser/evaluator short-circuit decisions require a lookup
   - no subprocess-heavy groups loaded for unrelated `ctx.*` fields.
7. Final review for drift maintenance:
   - docs updated where behavior changed
   - skill updated if architecture changed
   - no dependency docs touched unless dependencies changed.

Validation checkpoint:

- All targeted and package-level checks pass.
- A final manual example compiles:

  ```rust
  use darkmatter::markdown::compose::conditions::evaluate_condition_against;
  use serde_json::json;
  use std::path::Path;

  let data = json!({ "draft": true, "audience": "internal" });
  let result = evaluate_condition_against(
      "draft && audience == 'internal'",
      &data,
      Path::new("."),
  )?;
  assert!(result);
  # Ok::<(), darkmatter::markdown::compose::conditions::ConditionError>(())
  ```

Parallelizable work:

- Focused test runs and docs grep checks can be run concurrently once code is formatted.
