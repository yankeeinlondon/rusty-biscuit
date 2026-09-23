---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 5
package: darkmatter-cli
host: macOS aarch64-apple-darwin
---

# Guard after-scans — `darkmatter-cli`

The two path-sensitive guards that scan `darkmatter/cli/tests/`, compared by
path with `../baseline/guard-scans.md` (acceptance 7). Each before path was
mapped through `../darkmatter-cli-migration.json`; no helper directory moved.
Produced the Phase 1 way: a temporary probe test appended to each guard's file,
calling the guard's own population functions (`collect_rust_files`,
`governed_files` with `governs_spawn` / `governs_isolation`, and
`test_toolkit::archive_guard::{collect_rust_files, is_eligible}`). Each probe
ran once, and each file was restored and verified clean.

| Guard | Before | After | Extra after mapping | Missing after mapping | Eligibility changed |
|---|---:|---:|---|---|---|
| `spawn_site_guard.rs` walked | 58 | 61 | `l1/main.rs`, `level2/main.rs`, `l1/test_layout.rs` | none | — |
| spawn gate, governed | 42 | 44 | `l1/main.rs`, `l1/test_layout.rs` | none | — |
| isolation gate, governed | 40 | 40 | none | none | — |
| archive-path guard, `darkmatter/cli/tests/**` | 58 | 61 | the same three as "walked" | none | none (all `true`) |

The guard's census agrees with the probe: `governed_files` 44 (spawn) and 40
(isolation), with 0 raw spawns and the 2 allow-listed `.current_dir` sites in
`l1/md_process_fixture.rs`.

**One path repair was needed to keep the intended coverage.** The guard excluded
Level 2 files by their `level2_` file-name prefix. The move put them under
`level2/`, and `level2_harness_integrity.rs` became `level2/harness_integrity.rs`
(the R2 neutral alias), so its name lost the prefix. Without a repair, that file
and `level2/main.rs` would have joined the spawn population, which the baseline
does not have. `excluded` now also treats a first path segment equal to a tier
prefix without its `_` (`level2/`) as excluded. With the repair disabled,
`tier_naming_decides_what_the_guard_governs` and
`the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site` fail on
`level2/harness_integrity.rs`. With it, both pass.

The archive-path guard's summary line reads `mode=full-tree files-checked=3491
ineligible=0 violations=5`: Phase 4's 3,488 plus these three files. The 5
violations are the baseline's 5 allow-listed live sites.

## `spawn_site_guard.rs` walked (61, relative to `darkmatter/cli/tests/`)

```text
common/fixture.rs
common/level2.rs
common/mod.rs
common/protected_env.rs
common/source_scan.rs
l1/clean.rs
l1/clean_frontmatter.rs
l1/clean_json.rs
l1/clean_schema.rs
l1/code_block.rs
l1/compose_array_rendering.rs
l1/compose_base_schema.rs
l1/compose_basic.rs
l1/compose_interpolation.rs
l1/compose_layout.rs
l1/compose_page_blocks.rs
l1/compose_perf.rs
l1/compose_refs_and_missing.rs
l1/compose_remote_caching.rs
l1/compose_schema.rs
l1/compose_schema_file_rewrite.rs
l1/compose_shell.rs
l1/compose_state_set.rs
l1/compose_terminal_detection.rs
l1/compose_transclusion.rs
l1/delta.rs
l1/get_set_rm.rs
l1/graph.rs
l1/hash.rs
l1/hash_directory.rs
l1/hash_kind_save_diff.rs
l1/help.rs
l1/layout_alignment.rs
l1/layout_fill.rs
l1/layout_flags.rs
l1/layout_style_frontmatter.rs
l1/main.rs
l1/md_process_fixture.rs
l1/render_basic.rs
l1/rm.rs
l1/schema_about.rs
l1/schema_detect.rs
l1/schema_triggers.rs
l1/schema_validate.rs
l1/schema_validate_baseline.rs
l1/spawn_site_guard.rs
l1/test_layout.rs
l1/toc.rs
l1/validate_refs.rs
level2/harness_integrity.rs
level2/level2_code_block_styling.rs
level2/level2_disclosure_blocks.rs
level2/level2_errors.rs
level2/level2_frontmatter_images.rs
level2/level2_frontmatter_tables.rs
level2/level2_horizontal_rules.rs
level2/level2_layout_dimensions.rs
level2/level2_ordered_lists.rs
level2/level2_schema_about.rs
level2/level2_schema_validate.rs
level2/main.rs
```

## Spawn gate, governed (44)

```text
l1/clean.rs
l1/clean_frontmatter.rs
l1/clean_json.rs
l1/clean_schema.rs
l1/code_block.rs
l1/compose_array_rendering.rs
l1/compose_base_schema.rs
l1/compose_basic.rs
l1/compose_interpolation.rs
l1/compose_layout.rs
l1/compose_page_blocks.rs
l1/compose_perf.rs
l1/compose_refs_and_missing.rs
l1/compose_remote_caching.rs
l1/compose_schema.rs
l1/compose_schema_file_rewrite.rs
l1/compose_shell.rs
l1/compose_state_set.rs
l1/compose_terminal_detection.rs
l1/compose_transclusion.rs
l1/delta.rs
l1/get_set_rm.rs
l1/graph.rs
l1/hash.rs
l1/hash_directory.rs
l1/hash_kind_save_diff.rs
l1/help.rs
l1/layout_alignment.rs
l1/layout_fill.rs
l1/layout_flags.rs
l1/layout_style_frontmatter.rs
l1/main.rs
l1/md_process_fixture.rs
l1/render_basic.rs
l1/rm.rs
l1/schema_about.rs
l1/schema_detect.rs
l1/schema_triggers.rs
l1/schema_validate.rs
l1/schema_validate_baseline.rs
l1/spawn_site_guard.rs
l1/test_layout.rs
l1/toc.rs
l1/validate_refs.rs
```

## Isolation gate, governed (40)

```text
l1/clean.rs
l1/clean_frontmatter.rs
l1/clean_json.rs
l1/clean_schema.rs
l1/code_block.rs
l1/compose_array_rendering.rs
l1/compose_base_schema.rs
l1/compose_basic.rs
l1/compose_interpolation.rs
l1/compose_layout.rs
l1/compose_page_blocks.rs
l1/compose_perf.rs
l1/compose_refs_and_missing.rs
l1/compose_remote_caching.rs
l1/compose_schema.rs
l1/compose_schema_file_rewrite.rs
l1/compose_shell.rs
l1/compose_state_set.rs
l1/compose_terminal_detection.rs
l1/compose_transclusion.rs
l1/delta.rs
l1/get_set_rm.rs
l1/graph.rs
l1/hash.rs
l1/hash_directory.rs
l1/hash_kind_save_diff.rs
l1/help.rs
l1/layout_fill.rs
l1/layout_flags.rs
l1/layout_style_frontmatter.rs
l1/md_process_fixture.rs
l1/render_basic.rs
l1/rm.rs
l1/schema_about.rs
l1/schema_detect.rs
l1/schema_triggers.rs
l1/schema_validate.rs
l1/schema_validate_baseline.rs
l1/toc.rs
l1/validate_refs.rs
```

## Archive-path guard population (repository-relative; `path<TAB>eligible`)

```text
darkmatter/cli/tests/common/fixture.rs	true
darkmatter/cli/tests/common/level2.rs	true
darkmatter/cli/tests/common/mod.rs	true
darkmatter/cli/tests/common/protected_env.rs	true
darkmatter/cli/tests/common/source_scan.rs	true
darkmatter/cli/tests/l1/clean.rs	true
darkmatter/cli/tests/l1/clean_frontmatter.rs	true
darkmatter/cli/tests/l1/clean_json.rs	true
darkmatter/cli/tests/l1/clean_schema.rs	true
darkmatter/cli/tests/l1/code_block.rs	true
darkmatter/cli/tests/l1/compose_array_rendering.rs	true
darkmatter/cli/tests/l1/compose_base_schema.rs	true
darkmatter/cli/tests/l1/compose_basic.rs	true
darkmatter/cli/tests/l1/compose_interpolation.rs	true
darkmatter/cli/tests/l1/compose_layout.rs	true
darkmatter/cli/tests/l1/compose_page_blocks.rs	true
darkmatter/cli/tests/l1/compose_perf.rs	true
darkmatter/cli/tests/l1/compose_refs_and_missing.rs	true
darkmatter/cli/tests/l1/compose_remote_caching.rs	true
darkmatter/cli/tests/l1/compose_schema.rs	true
darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs	true
darkmatter/cli/tests/l1/compose_shell.rs	true
darkmatter/cli/tests/l1/compose_state_set.rs	true
darkmatter/cli/tests/l1/compose_terminal_detection.rs	true
darkmatter/cli/tests/l1/compose_transclusion.rs	true
darkmatter/cli/tests/l1/delta.rs	true
darkmatter/cli/tests/l1/get_set_rm.rs	true
darkmatter/cli/tests/l1/graph.rs	true
darkmatter/cli/tests/l1/hash.rs	true
darkmatter/cli/tests/l1/hash_directory.rs	true
darkmatter/cli/tests/l1/hash_kind_save_diff.rs	true
darkmatter/cli/tests/l1/help.rs	true
darkmatter/cli/tests/l1/layout_alignment.rs	true
darkmatter/cli/tests/l1/layout_fill.rs	true
darkmatter/cli/tests/l1/layout_flags.rs	true
darkmatter/cli/tests/l1/layout_style_frontmatter.rs	true
darkmatter/cli/tests/l1/main.rs	true
darkmatter/cli/tests/l1/md_process_fixture.rs	true
darkmatter/cli/tests/l1/render_basic.rs	true
darkmatter/cli/tests/l1/rm.rs	true
darkmatter/cli/tests/l1/schema_about.rs	true
darkmatter/cli/tests/l1/schema_detect.rs	true
darkmatter/cli/tests/l1/schema_triggers.rs	true
darkmatter/cli/tests/l1/schema_validate.rs	true
darkmatter/cli/tests/l1/schema_validate_baseline.rs	true
darkmatter/cli/tests/l1/spawn_site_guard.rs	true
darkmatter/cli/tests/l1/test_layout.rs	true
darkmatter/cli/tests/l1/toc.rs	true
darkmatter/cli/tests/l1/validate_refs.rs	true
darkmatter/cli/tests/level2/harness_integrity.rs	true
darkmatter/cli/tests/level2/level2_code_block_styling.rs	true
darkmatter/cli/tests/level2/level2_disclosure_blocks.rs	true
darkmatter/cli/tests/level2/level2_errors.rs	true
darkmatter/cli/tests/level2/level2_frontmatter_images.rs	true
darkmatter/cli/tests/level2/level2_frontmatter_tables.rs	true
darkmatter/cli/tests/level2/level2_horizontal_rules.rs	true
darkmatter/cli/tests/level2/level2_layout_dimensions.rs	true
darkmatter/cli/tests/level2/level2_ordered_lists.rs	true
darkmatter/cli/tests/level2/level2_schema_about.rs	true
darkmatter/cli/tests/level2/level2_schema_validate.rs	true
darkmatter/cli/tests/level2/main.rs	true
```
