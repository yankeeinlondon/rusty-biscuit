---
ready: false
agent: codex/default
created: 2026-07-09T12:45:59
implemented: true
---

# Review 2: Single-Sourcing Schema

## Verdict

Not production ready.

The implementation covers the central library path well: `ctx.*` descriptors are projected from `darkmatter.yaml`, list-valued captures moved to arrays, the six list-formatting functions exist, and the scalar/interpolation boundary split is explicitly tested at Level 1. I did not find a Level 2 or Level 3 requirement in this feature; the user-observable behavior here is schema/catalog/compose/CLI text behavior, so Level 1 unit and CLI integration tests are the right baseline unless the review is about terminal styling.

## Findings

### High: `md schema about` does not expose the migrated catalog/function surface

- Location: `darkmatter/cli/src/commands/schema/about.rs:46`
- Requirement: spec acceptance criterion 11 says "`md schema about`, `context-variables.md`, `md schema validate`, and compose output remain correct for the migrated variables"; D7 also makes typed expression-function signatures catalog data that downstream consumers should see.
- Current behavior: `run_about` still renders only the generic SimplifiedSchema language reference: shapes, type keywords, constraints, and verbose inline-object/coercion/validation notes. It does not consume `context_variable_descriptors()` or `expression_function_descriptors()`, so users cannot verify from `md schema about` that `ctx.packages` is `string[]`, `ctx.now` is `datetime`, the removed `_list` twins are gone, or the six `as_*` functions exist with typed signatures.
- Test level: no new Level 1 CLI integration test asserts this CLI surface. Existing `schema_about` tests cover generic schema vocabulary and terminal component shape, not the migrated `ctx.*` or list-formatting function content. Level 1 is sufficient here; Level 2 is only needed for style/rendering fidelity.
- Impact: the implementation meets the library catalog path but leaves a named acceptance surface stale. A user relying on `md schema about` gets no evidence of the single-sourced ctx catalog or the new formatter functions.
- Suggested fix: extend `md schema about` or add the intended schema-about subview so it renders the derived ctx descriptors and expression function typed signatures, then add Level 1 CLI assertions for representative migrated entries: `ctx.now: datetime`, `ctx.packages: string[]`, absence of `packages_list`, and `as_csv(list: any[]) -> string | error`.

### Medium: the catalog drift guard omits the projected `default` field

- Location: `darkmatter/lib/src/markdown/compose/context/catalog.rs:418`
- Requirement: D1/D5 make YAML authoritative for `name`, `type`, `description`, and flags including `generated`, `required`, and `default`; acceptance criterion 2 requires a drift-guard test for the projected catalog and YAML.
- Current behavior: `project_one` projects `Constraint::Default` into `ContextVariableDescriptor::default`, but `projected_descriptors_match_base_schema` checks only name/order, base type, array, integer, description, `required`, and `generated`. It never asserts that descriptor defaults match schema defaults.
- Test level: this is a Level 1 drift-guard gap. No terminal tier is relevant.
- Impact: a future ctx default could be added to `darkmatter.yaml` and projected incorrectly without tripping the promised drift guard.
- Suggested fix: in `projected_descriptors_match_base_schema`, compute the expected `Constraint::Default` value and compare it to `d.default`.

## Coverage Notes

- Level 1 coverage present: descriptor/schema projection for name/type/description/generated/required, grouping totality, removed `_list` absence, corrected temporal types, capture shape, list-formatting functions, example files, bare array interpolation, and `scalar_string` preservation.
- Level 1 coverage missing: `md schema about` migrated ctx/function content, and descriptor default drift.
- Level 2 coverage: existing schema-about rendering tests are useful for terminal rendering, but this feature's new functional requirements do not require Level 2 unless the implementation adds styled tables/sections for the new content.
- Level 3 coverage: not applicable. The spec has no OS keyboard or input-encoder requirements.

## Verification

- `just test darkmatter` (Level 1) passed for all three package-area crates: `darkmatter`, `darkmatter-cli`, and `dmls`.
