The document darkmatter/docs/topics/boolean-conditional-logic.md describes 


⏺ Yes — the condition evaluator is fully exposed as public API

  The module darkmatter/lib/src/markdown/compose/conditions.rs is declared pub mod conditions; in compose/mod.rs:44, so consumers
  can import the evaluator directly.

  Two access tiers

  1. High-level evaluator (recommended for most consumers)

  use darkmatter::markdown::compose::conditions::{evaluate_condition, ConditionError};
  use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder, ComposeContext};

  let state = EffectiveStateBuilder::new()
      .with_frontmatter(my_map)
      .with_context(ctx)
      .build()?;

  let result: bool = evaluate_condition("env.AGENT == 'claude' && !draft", &state, 1)?;

  - evaluate_condition(expr, state, line) -> Result<bool, ConditionError> — conditions.rs:74
  - ConditionError carries Parse { expr, line, message, span } and Eval { expr, line, message } and implements both
  std::error::Error (via thiserror) and biscuit_terminal::errors::BlockError for styled rendering.
  - The same function is also re-exported at compose::transclusion::evaluate_condition (transclusion/mod.rs:16) — two import paths
  for the identical function.

  2. Low-level parser (for custom evaluation against a non-frontmatter runtime model)

  use darkmatter::markdown::compose::interpolation::{parse_condition, Expr, ComparisonOp};
  let ast: Expr = parse_condition("priority == 'high' && (env.PROD || env.FORCE)")?;
  // walk `ast` yourself

  interpolation/mod.rs re-exports Expr, ComparisonOp, Parser, ParseError, parse, parse_condition, plus Evaluator, EvalValue,
  EvalResult, and InterpolationLookup. So a consumer can drop the parser into a different evaluation strategy entirely.

  Required dependencies for the high-level path

  EffectiveState and EffectiveStateBuilder are re-exported from compose/mod.rs:66. The builder accepts:
  - with_frontmatter(HashMap<String, Value>)
  - with_external_state(Value) — inherited state
  - with_context(ComposeContext) — provides ctx.* and env.*
  - with_merge_strategy(...), with_allow_ctx_override(...)

  One ergonomic gap

  There is no shortcut that evaluates a when-expression against a plain serde_json::Value or HashMap without first constructing a
  ComposeContext + EffectiveState. A consumer who only wants the boolean DSL — with no frontmatter pipeline involvement — still
  needs to build a minimal ComposeContext (e.g. ComposeContext::fixed_for_testing() is used in the unit tests at conditions.rs:330,
  but that's a test helper, not a public production constructor). If reuse outside the compose pipeline becomes a real use-case, a
  thin façade like evaluate_condition_against(expr, &serde_json::Value) would be a worthwhile addition.
