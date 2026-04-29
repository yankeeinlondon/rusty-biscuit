The document darkmatter/docs/topics/boolean-conditional-logic.md describes our current parsing logic for boolean operations using in various places in Darkmatter. In this feature we will make this functionality available to external library callers.


The module darkmatter/lib/src/markdown/compose/conditions.rs is declared pub mod conditions; in compose/mod.rs:44, so consumers can import the evaluator directly.

## Architectural Unification

Currently, there is significant code duplication between the boolean condition evaluator (`conditions.rs`) and the string interpolation evaluator (`interpolation/evaluator.rs`). Both implement identical logic for operators (`==`, `>`, `&&`) and helper functions (`length`, `contains`, `haskey`). Furthermore, the `interpolation` module has organically grown into a general-purpose expression engine, making it awkward to use for non-interpolated boolean logic.

To resolve this technical debt and improve maintainability, we will extract a core expression engine and unify the evaluation pathways:
1. **Extract Core Engine:** We will create a new `darkmatter/lib/src/markdown/compose/expression/` module to house the shared Lexer, Parser, AST, and Evaluator.
2. **Rename the Lookup Trait:** The existing `InterpolationLookup` trait will be moved to the new `expression` module and renamed to `EvaluationLookup`.
3. **Refactor Consumers:** The standalone evaluator in `conditions.rs` and the interpolation logic in `interpolation/` will be refactored to act as specialized consumers of the new core `expression` engine.

This architectural cleanup ensures that boolean parsing relies on a generically named `EvaluationLookup` trait rather than one tied to string interpolation. This trait-based approach perfectly complements the new shortcut API, as it allows us to inject custom, lazily-evaluated lookup implementations for the `env.*` and `ctx.*` namespaces without relying on the heavy `EffectiveState` object.


## Required dependencies for the high-level path:

EffectiveState and EffectiveStateBuilder are re-exported from compose/mod.rs:66. The builder accepts:

- with_frontmatter(HashMap<String, Value>)
- with_external_state(Value) — inherited state
- with_context(ComposeContext) — provides ctx.* and env.*
- with_merge_strategy(...), with_allow_ctx_override(...)

### Error Handling Dependencies

The `ConditionError` returned by the evaluators currently implements `biscuit_terminal::errors::BlockError` for rich terminal rendering. When exposing the parsing logic to external library consumers, we will **keep this implementation as-is**. Library consumers will inherit the `biscuit-terminal` dependency. This maintains consistency across the monorepo and avoids introducing complex feature flags or wrapper types for error handling.

## Ergonomics and the Shortcut API

Currently, there is no shortcut that evaluates a when-expression against a plain `serde_json::Value` or `HashMap` without first constructing a `ComposeContext` + `EffectiveState`. A consumer who only wants the boolean DSL — with no frontmatter pipeline involvement — still needs to build a minimal `ComposeContext`.

To resolve this, we will introduce a new shortcut function:
`evaluate_condition_against(expr: &str, data: &serde_json::Value, work_dir: &Path) -> Result<bool, ConditionError>`

### Context Resolution and Lazy Loading

When using this shortcut:
1. **Top-level properties** are resolved against the provided `data`.
2. **`env.*` properties** are automatically resolved against the system environment.
3. **`ctx.*` properties** (Repo, Hardware, OS, FileChanges, etc.) are automatically resolved using the provided `work_dir` as the base path.

**Critical Requirement: Lazy Loading.** The `ctx.*` namespaces carry significant I/O and subprocess overhead (e.g., executing `git status`, probing GPU hardware). It is absolutely essential that this data is **lazily loaded**. The shortcut API must only incur the performance penalty for the specific context groups that are actually referenced in the `expr`. If a user evaluates `draft == true`, no disk I/O or system profiling should occur.

