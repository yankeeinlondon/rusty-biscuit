---
ready: false
agent: codex/default
created: 2026-06-20T11:51:37
implemented: true
---

# Review 3

Not ready for production. The review-2 functional gap appears fixed: `parse_shell_value` now trims before parsing, padded valid `$()` values are executed, padded malformed/no-command shell shapes fail, and the Claudine CLI regression for the original malformed `spec_path` passes. The remaining blocker is verification: the required Darkmatter Level 1 package run is not green.

## Findings

### High: required Darkmatter Level 1 tests still fail

Acceptance criterion 7 requires `just test` to pass in the touched `darkmatter` and `claudine` package areas. I ran:

```text
just test darkmatter
just test claudine-cli
```

`just test claudine-cli` passed: 1668 tests run, 1668 passed, 72 skipped. The Claudine regression `compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking` at `claudine/cli/tests/compose_cli.rs:107` passed.

`just test darkmatter` failed in the `darkmatter` crate before the area completed successfully. `darkmatter-cli` and `dmls` passed afterward, but the workspace summary still failed because `darkmatter` failed.

Observed `darkmatter` failures:

- 13 `darkmatter::schemas_convert_snapshots` tests failed with `.snap.new` output. The diffs show optional schema conversion now emitting `anyOf` wrappers with `{"type":"null"}` for atoms that the checked-in snapshots still expect as direct typed schemas, for example `snapshot_atom_number` at `darkmatter/lib/tests/schemas_convert_snapshots.rs:75`, `snapshot_atom_boolean` at `darkmatter/lib/tests/schemas_convert_snapshots.rs:90`, `snapshot_atom_boolish` at `darkmatter/lib/tests/schemas_convert_snapshots.rs:95`, `snapshot_atom_file_match` at `darkmatter/lib/tests/schemas_convert_snapshots.rs:110`, and `snapshot_atom_enum_members` at `darkmatter/lib/tests/schemas_convert_snapshots.rs:120`.
- `darkmatter markdown::compose::preflight::acceptance_tests::execution_subset_of_approval_across_randomized_conditions` timed out after 30 seconds.

The schema snapshot failures are especially relevant because this implementation also changed validation problem handling around nullable `anyOf` wrappers in `darkmatter/lib/src/markdown/schemas/validate.rs:188` and `darkmatter/lib/src/markdown/schemas/validate.rs:236`. If the nullable schema conversion output is intentional, the snapshots need to be reviewed and updated as part of this change. If it is not intentional, the conversion behavior needs to be corrected. Either way, the requested package-level verification is currently failing, so this feature cannot be marked production-ready.

Verification level: Level 1 is the correct tier for these requirements because the feature is compose/parser/schema behavior, not real-terminal rendering or keyboard encoder behavior. The strongest required Level 1 package verification is red.

## Requirement Coverage

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Whole-value `{{ ... }}` parse/evaluation failures are fatal even with `fail_fast = false` | Level 1 Darkmatter unit coverage plus Level 1 Claudine CLI dry-run regression | OK |
| Typed whole-value `{{ ... }}` results are preserved | Level 1 Darkmatter unit tests | OK |
| Mixed malformed interpolation remains lenient | Level 1 Darkmatter unit test | OK |
| Whole-value `$()` failures are fatal when shell expansion is enabled | Level 1 Darkmatter tests for padded malformed/no-command cases | OK |
| Padded valid whole-value `$()` values expand when shell expansion is enabled | Level 1 Darkmatter execution tests at `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:2964` and `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:2986` | OK |
| Raw expansion syntax does not appear in successful effective frontmatter | Level 1 leak-guard tests and Level 1 Claudine dry-run regression | OK |
| Required package tests pass | `just test claudine-cli` passed; `just test darkmatter` failed | Gap |

## Notes

No Level 2 or Level 3 testing is required for this spec. The user-observable behavior is compose preparation, diagnostics, schema validation, and effective-frontmatter content, all of which are appropriately verified at Level 1 once the Darkmatter package run is green.
