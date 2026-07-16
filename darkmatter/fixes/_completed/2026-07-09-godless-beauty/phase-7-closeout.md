# Phase 7 closeout

## Test inventory

The final Darkmatter inventory contains 5,378 tests, ten more than the Phase 1 baseline of 5,368.
The additions are the three shared-reference parser regressions, five context-capture ownership and
GPU/demand regressions, and two expression-registration invariants. The Level-2 render-tree target
still contains exactly 20 tests, matching the Phase 1 count; its module-qualified names changed
only because Phase 3 moved the tests into responsibility-specific modules.

## Scope and portability review

The complete implementation diff adds no generic pass trait, plugin framework, or production
platform-command discovery. Production behavior changes remain limited to UTF-8-safe reference
title parsing, GPU-only context population, and the documented expression-catalog API migration.
The extracted code uses `Path`/`PathBuf`, platform-neutral module paths and fixtures, guarded
environment mutation, and `sniff` for repository and host discovery. No new Unix-only production
command, separator, or filesystem assumption was found.

Active comments, rustdoc, topic documentation, and the Darkmatter skill consistently identify the
compose pipeline, domain-owned expression registrations, and the public
`expression_function_descriptors()` accessor. Removed transform APIs and expression constants now
occur only in historical records, baselines, or explicit migration inventories.

## Validation

- `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine -p claudine-cli`: passed.
- Darkmatter `just test`: passed.
- Darkmatter `just test-l2`: passed (19 Darkmatter and 69 Darkmatter CLI tests; DMLS had no matching
  Level-2 tests).
- Darkmatter `just lint`: passed.
- Focused context-capture nextest run: 19 passed.
- Claudine library nextest run: 3,146 passed; Claudine contract: 47 passed.
- Claudine `just lint`: passed for the library, contract, and CLI.

The broad Claudine CLI test target was stopped after it exceeded the non-interactive session's
per-command time limit while continuing to pass. Phase 6 had already recorded its canonical
package checks; Phase 7 changed only whitespace in Darkmatter source and plan documentation.

## Post-review addendum — Phase 5 capture split (Review 3)

Review 3 found that the capture split was fully implemented in source but the plan checkboxes and
this closeout still described it as deferred. The records have since been corrected. The actual
post-split source layout is:

- `capture/mod.rs` — 113-line crate-private facade for group selection and population sequencing.
- `capture/groups.rs` — `ContextGroup`, `all`, demand scanning, and `for_key` delegation with
  per-domain `KEYS` ownership.
- `capture/snapshot.rs` — `ContextCapture::new` and concurrent sniff probe orchestration.
- `capture/{datetime,repo,changes,languages,docs,host,agent}.rs` — population code and owned keys
  per domain, with `sniff` retained as the discovery authority.

### Capture test relocation inventory (15 pre-move → 19 post-move)

The 15 pre-move tests all lived under `markdown::compose::context::capture::tests`. After the split
they were distributed to their owning domain modules without name or assertion changes, and four
intentional regression/invariant tests were added:

| Post-move module | Test name | Origin |
|---|---|---|
| `agent::tests` | `capture_runtime_context_includes_agent_group` | pre-move |
| `agent::tests` | `populate_agent_defaults_when_missing_or_empty` | pre-move |
| `agent::tests` | `populate_agent_uses_env_values_with_trim` | pre-move |
| `datetime::tests` | `day_of_month_suffixed_formats_correctly` | pre-move |
| `datetime::tests` | `populate_datetime_includes_utc_time_variants` | pre-move |
| `datetime::tests` | `populate_datetime_populates_documented_aliases` | pre-move |
| `datetime::tests` | `populate_datetime_produces_all_expected_keys` | pre-move |
| `datetime::tests` | `season_determination` | pre-move |
| `repo::tests` | `area_vars_empty_when_not_monorepo` | pre-move |
| `repo::tests` | `current_packages_captured_as_string_array` | pre-move |
| `repo::tests` | `depends_on_captured_as_object_array_scoped_to_area` | pre-move |
| `repo::tests` | `repo_root_has_no_trailing_slash` | pre-move |
| `repo::tests` | `strip_trailing_sep_removes_single_separator` | pre-move |
| `repo::tests` | `used_by_captured_as_object_array_with_empty_users` | pre-move |
| `capture::tests` | `content_without_runtime_context_only_populates_datetime` | pre-move |
| `host::tests` | `gpu_only_population_does_not_require_hardware_capture` | **added** — GPU-only regression |
| `groups::tests` | `aliases_are_allowlisted_and_unknown_keys_have_no_group` | **added** — group-ownership invariant |
| `groups::tests` | `every_generated_descriptor_maps_to_one_group_or_explicit_alias` | **added** — group-ownership invariant |
| `groups::tests` | `every_owned_key_has_exactly_one_group` | **added** — group-ownership invariant |

No pre-move test name was dropped or renamed; `#[serial]` groups on the agent tests were preserved.
The four additions are the GPU-only capture regression (verifying `ctx.gpu` population without the
hardware probe) and three group-ownership invariants (unique group per descriptor, allowlisted
aliases, and unknown-key rejection).
