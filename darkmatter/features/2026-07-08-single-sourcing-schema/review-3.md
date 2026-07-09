---
ready: false
agent: codex/default
created: 2026-07-09T13:17:18
implemented: true
---

# Review 3: Single-Sourcing Schema

## Verdict

Not production ready.

The Review 2 findings are materially addressed: the default drift guard now checks `default(...)`, and verbose `md schema about` now renders the derived `ctx.*` catalog and typed expression-function signatures. The remaining blocker is the other documented/public reference surface: `context-variables.md` is still hand-authored and does not render from the derived catalog required by the spec.

## Findings

### High: `context-variables.md` is still not generated from the derived catalog

- Location: `darkmatter/docs/topics/context-variables.md:65`, `darkmatter/docs/topics/context-variables.md:86`, `darkmatter/docs/topics/context-variables.md:125`
- Requirement: the spec says "`context-variables.md` and `md schema about` render from the same derived catalog" (`spec.md:194`) and acceptance criterion 11 requires `context-variables.md` to remain correct for migrated variables (`spec.md:421`).
- Current behavior: the document is still a separate hand-authored table. It labels schema-typed values with presentation/runtime labels such as `String` for `today`, `now`, and `now_utc`, and `Number` for `timestamp`/`timestamp_ms`, while the derived catalog now exposes `date`, `datetime`, and `number(integer)`. That means the docs do not share the same source as `md schema about`, and the exact drift this feature is meant to eliminate is still possible on a named public surface.
- Test level: missing Level 1 documentation/catalog parity coverage. No Level 2 or Level 3 is required because this is generated/static documentation content, not terminal rendering fidelity or keyboard/input behavior.
- Impact: users and downstream consumers reading `context-variables.md` can get a different type vocabulary than `md schema about` and DMLS hover/completion. Future YAML changes can update the derived catalog while leaving the topic page stale.
- Suggested fix: generate the variable reference section of `context-variables.md` from `context_variable_descriptors()` or move it behind the same renderer used by `md schema about`, then add a Level 1 parity test that representative rows match the derived catalog (`ctx.now` = `datetime`, `ctx.today` = `date`, `ctx.packages` = `string[]`, `ctx.timestamp` = `number(integer)`) and that removed `_list` variables appear only in migration prose, not catalog rows.

### Medium: verbose `md schema about` CLI coverage does not assert the newly added content

- Location: `darkmatter/cli/tests/schema_about.rs:252`; helper-only assertions at `darkmatter/cli/src/commands/schema/about.rs:693`
- Requirement: acceptance criterion 11 names `md schema about` as a user-facing surface for the migrated variables; D7 requires typed expression-function signatures to be catalog data consumers can see.
- Current behavior: the new helper tests assert `context_catalog_markdown()` contains `ctx.now`, `ctx.packages`, absence of `packages_list`, and `as_csv(list: any[]) -> string | error`. The CLI integration test for `--verbose` still checks only the older advanced sections (`Nested Objects`, coercion, validation notes). If future routing accidentally stops calling `report.context_variables()` or `report.expression_functions()`, the binary-level test suite would still pass.
- Test level: Level 1 CLI integration is the right level. Level 2 is not required unless the claim becomes terminal layout/style fidelity for the new sections.
- Impact: lower confidence in the exact public command path, though the current implementation is wired correctly by inspection.
- Suggested fix: extend `schema_about_verbose_prints_advanced_sections_as_readable_lists` or add a focused Level 1 CLI integration test asserting the stripped stdout contains `Context Variables`, `ctx.now`, `datetime`, `ctx.packages`, `string[]`, `Expression Functions`, and `as_csv(list: any[]) -> string | error`, and does not contain removed `_list` variables in catalog output.

## Coverage Notes

- Level 1 coverage present: schema projection for name/type/description/generated/required/default, grouping totality, removed `_list` catalog absence, corrected temporal types, capture shapes, list-formatting functions, verified formatter example files, interpolation output boundary for arrays, and `scalar_string` preservation.
- Level 1 coverage incomplete: `context-variables.md` parity with the derived catalog, and binary-level assertions for the new verbose `md schema about` sections.
- Level 2 coverage: existing schema-about Level 2 rendering tests are useful for terminal layout, but this feature has no new requirement that requires Level 2 unless the new sections make style/layout promises.
- Level 3 coverage: not applicable. The spec has no OS keyboard/input-encoder requirement.

## Verification

- `cargo nextest run --color=never -p darkmatter -p darkmatter-cli context_catalog_renders_migrated_variables_with_types expression_function_signatures_render_typed_list_formatters projected_descriptors_match_base_schema project_one_carries_default_constraint list_formatting_functions_are_typed example_files_evaluate_to_their_declared_returns schema_about_verbose_prints_advanced_sections_as_readable_lists` passed: 7 tests.
- `just test` was started for the darkmatter package area and stopped after 55.785s because it was still running beyond the review's focused need: 2,406/5,187 tests had run, all 2,406 passed, 111 were skipped, and the run was interrupted before completion.
