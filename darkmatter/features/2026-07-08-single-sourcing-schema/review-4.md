---
ready: true
agent: codex/default
created: "2026-07-09T14:06:34"
---

# Review 4 - Single-Sourcing Schema

## Verdict

Ready for production.

The implementation now satisfies the specification's core contract: `darkmatter/docs/schemas/darkmatter.yaml` is the single authored source for `ctx.*` name/type/description/flags, the Rust catalog is projected from it, list-valued context variables are first-class arrays, the six list-formatting functions exist with typed signatures and verified examples, and `md schema about --verbose` exposes the derived context catalog plus expression-function signatures.

## Findings

No blocking findings.

## Verification Matrix

| Requirement | Strongest verification observed | Assessment |
|---|---:|---|
| YAML is the single source for `ctx.*` name/type/description/generated/required/default/order | Level 1: `projected_descriptors_match_base_schema`; `project_one_carries_default_constraint` | Covered at the right level. This is in-process data projection, not terminal behavior. |
| Grouping map is total and does not contain stale keys | Level 1: `grouping_map_is_total` | Covered at the right level. |
| Runtime capture keys match descriptors and capture values match projected shapes | Level 1: `every_descriptor_has_a_captured_runtime_key`; `capture_shape_matches_projected_type` | Covered at the right level. |
| `ctx.now` / `ctx.now_utc` are `datetime`; `today` family remains `date` | Level 1: `temporal_types_are_correct`; schema-about CLI assertion includes `ctx.now` + `datetime` | Covered at the right level. |
| Retired `_list` variables are absent from catalog/YAML and migrated docs | Level 1: `removed_list_twins_are_absent`; `context_variables_doc_matches_generated_catalog`; CLI assertion checks representative retired names are absent | Covered at the right level. |
| Bare array interpolation renders line-separated only at the interpolation output boundary | Level 1: `bare_array_renders_line_separated`; `scalar_string_keeps_json_array_form`; `equality_comparison_unaffected_by_line_separated_output` | Covered at the right level. No real-terminal behavior is involved. |
| Six D4 list formatters exist and render flat/nested/object-array lists | Level 1: function unit tests in `fn_list_formatting`, evaluator tests, and example-file execution | Covered at the right level. |
| Expression-function catalog carries typed signatures including `any[] -> string \| error` for list formatters | Level 1: `list_formatting_functions_are_typed`; `expression_function_signatures_render_typed_list_formatters` | Covered at the right level. |
| `md schema about --verbose` exposes context variables and typed expression functions | Level 1 binary test: `schema_about_verbose_prints_context_and_expression_sections` | Covered at the right level for content routing. Existing Level 2 schema-about tests cover real-terminal table/code rendering generally, but the new verbose content is plain rendered text and does not need new Level 2 coverage unless exact styling/glyph behavior becomes part of the requirement. |
| `context-variables.md` stays in sync with the generated catalog | Level 1: `context_variables_doc_matches_generated_catalog` | Covered at the right level. |

## Evidence

Targeted tests run:

```text
cargo nextest run --color=never -p darkmatter projected_descriptors_match_base_schema grouping_map_is_total removed_list_twins_are_absent temporal_types_are_correct collapsed_list_variables_are_arrays capture_shape_matches_projected_type bare_array_renders_line_separated as_csv_reproduces_comma_output as_unordered_list_renders_object_array_nested equality_comparison_unaffected_by_line_separated_output list_formatting_functions_are_typed example_files_evaluate_to_their_declared_returns
```

Result: 12 passed.

```text
cargo nextest run --color=never -p darkmatter-cli schema_about_verbose_prints_context_and_expression_sections context_variables_doc_matches_generated_catalog context_catalog_renders_migrated_variables_with_types expression_function_signatures_render_typed_list_formatters
```

Result: 4 passed.

I also attempted `just test` in the `darkmatter` package area, but stopped it after it exceeded the non-interactive 60-second threshold without producing output. The targeted nextest runs above cover the feature-specific acceptance criteria reviewed here.

## Notes

- The review-3 gaps are addressed: the generated context-variable docs now have parity coverage, and the binary-level schema-about test now asserts the new verbose sections are actually routed.
- The remaining `ContextValueType` name is no longer the old presentation enum; it is a small wrapper over `SimplifiedType` plus array/integer shape. That is compatible with the spec's intent to remove presentation-only types.
