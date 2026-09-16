---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-08-12T04:45:05+01:00
spec: 2026-08-12-optional-params/spec.md
implemented: false
description: A **fix** review of `2026-08-12-optional-params/spec.md`
fix: 2026-08-12-optional-params/review-1.md
---

# Review 1: Optional Params

## Verdict

The fix is **ready for production**. I found no functional, correctness,
performance, ergonomics, or test-rigor defects that should block release.

The implementation uses the intended schema-validation seam and grows the
known-root set only by absent, optional, no-default top-level properties from
the document's own inline or referenced SimplifiedSchema. It preserves caller
values, idempotence, validation-only passivity, and strict failure for genuinely
undeclared roots. Baseline, trigger, raw JSON Schema, root-union, nested,
required, and defaulted properties remain outside materialization.

## Findings

None.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---:|---|
| Unset `commit_message` on the real shipped `implement-plan.md` passes lifecycle-shell preflight, reaches the provider, and selects the AI-message branch | Level 2: `level2_shipped_implement_plan_unset_commit_message_reaches_provider_and_auto_branch`; Level 1: `shipped_implement_plan_prepares_with_unset_optional_commit_message` | Sufficient. The L2 test copies the shipped artifact byte-for-byte, runs the normal `claudine compose` path in tmux, observes provider execution, and records the selected lifecycle command. |
| A supplied `commit_message` remains exact and selects the explicit-message branch | Level 2: `level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch`; Level 1: `shipped_implement_plan_preserves_supplied_commit_message_in_preflight_command` | Sufficient. The command double observes one exact `chore: x` argument and the opposite branch does not execute. |
| Undeclared shell/interpolation and `when:` roots still fail closed | Level 1: `materialized_optional_parameter_is_known_to_strict_subtree_compose`, `when_unknown_root_typo_fails_closed`, and the existing lifecycle undefined-variable suites | Sufficient. This is expression/state behavior; no real-terminal encoder or rendering dependency exists. |
| Inline and whole-file referenced optional properties materialize as ordered null bindings; a second schema pass is idempotent | Level 1: `document_optional_bindings_materialize_before_coercion_and_are_idempotent`, `referenced_document_optional_binding_materializes`, and `compose_frontmatter_materializes_optional_parameters_as_ordered_nulls` | Sufficient. |
| Existing values win, including explicit null and falsey scalar/container values | Level 1: `present_optional_values_are_preserved`, `compose_frontmatter_caller_value_wins_over_optional_materialization`, and `optional_null_and_supplied_empty_string_remain_distinct_in_body_expressions` | Sufficient. The schema-stage helper uses presence-only `entry(...).or_insert(...)` semantics after overrides merge, so value origin and truthiness cannot alter preservation. |
| Baseline, trigger, raw JSON Schema, root-union, nested, required, and defaulted declarations do not materialize | Level 1: `non_document_owned_and_nested_properties_do_not_materialize`, `defaulted_document_property_does_not_materialize`, `raw_json_schema_property_does_not_materialize`, `trigger_payload_property_does_not_materialize`, `normal_darkmatter_baseline_does_not_flood_frontmatter`, and `document_schema_missing_required_fails` | Sufficient. Each source/shape boundary is exercised at the owning schema stage. |
| Validation-only APIs remain passive and retain missing/explicit-null acceptance | Level 1 plus type-level enforcement: nullable SimplifiedSchema conversion tests cover missing and explicit null; `DarkmatterSchemas::validate` accepts `&Markdown`, so it cannot mutate the document | Sufficient. Composition-only mutation is confined to `schema_validation.rs`. |
| Materialized nulls serialize and re-compose stably | Level 1: `compose_frontmatter_materialized_null_is_stable_after_write_and_recompose` | Sufficient. The test performs the required write/read/write-equivalent round trip and compares both parsed frontmatter and emitted bytes. |
| First-pass frontmatter interpolation remains before materialization, while later body/subtree consumers see the binding | Level 1: `first_frontmatter_interpolation_precedes_optional_materialization`, `materialized_optional_parameter_is_known_to_strict_subtree_compose`, and `optional_null_and_supplied_empty_string_remain_distinct_in_body_expressions` | Sufficient. |
| Shipped schemas remain passively parseable and a shipped prompt uses the ordinary invocation path | Level 1: `shipped_prompts_have_parseable_schemas_and_expressions`; Level 2: both shipped `implement-plan.md` regressions | Sufficient. Passive corpus validation performs no lifecycle effects; the L2 rows provide the effectful normal-path counterpart. |

## Implementation Assessment

- `materialize_optional_document_bindings` is small and runs exactly between
  effective-schema resolution and coercion/validation, matching the specified
  ownership boundary.
- Eligibility is derived from the resolved SimplifiedSchema AST and
  `SchemaOriginKind`, rather than nullable JSON Schema shape heuristics.
- `PropertyDef::is_required` and `has_default` correctly hoist property-level
  semantics across union arms and reuse the same value/array constraint view as
  JSON Schema lowering.
- The insertion loop is linear in the number of declared top-level properties,
  preserves schema declaration order, and performs no unnecessary work when the
  document has no eligible SimplifiedSchema.
- No Level 3 verification is applicable: this fix has no keyboard, mouse,
  paste, IME, or terminal input-encoder behavior.

## Validation Performed

- `darkmatter/just test`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `darkmatter/just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `claudine/just test`: passed for all five package-area members. The two new
  shipped-prompt Level 1 regressions passed.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux claudine/just test-l2`: passed; 230/230
  `claudine-cli` L2 tests and 3/3 `claudine-gen` L2 tests passed. Backend proof
  recorded 199 tmux executions for `claudine-cli` and 2 for `claudine-gen`; both
  optional-parameter shipped-prompt rows passed.
- `claudine/just lint`: passed for all package-area members and its diagnostic
  architecture guards.

The macOS linker emitted its compact-unwind-size warning while building the
large Claudine test binary. It did not fail a gate and is unrelated to this
fix.
