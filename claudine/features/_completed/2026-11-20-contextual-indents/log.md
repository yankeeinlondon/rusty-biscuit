---
implementation_1: "2026-11-20T22:51:51-08:00"
---

# Contextual Indents Review Log

## Implementation of Review Findings #1

> **started at:** 2026-11-20T22:51:51-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/coding/rusty-biscuit/claudine/features/2026-11-20-contextual-indents/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false` at start
- no local OS testing rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS proof is left to CI

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | No-op Markdown saves rewrite authored text and resolve indentation | fixed | every required child fixed or clean; see children |
| R1-F1.1 | Markdown frontmatter editor via `md set --save`, equal quoted scalar → original bytes | fixed | `frontmatter_path_edit::setting_a_quoted_scalar_to_its_own_value_keeps_the_source_and_resolves_nothing` (exact bytes, 0 fallback calls); CLI `get_set_rm::unchanged_saves::setting_a_quoted_scalar_to_its_own_value_leaves_the_file_unwritten` (bytes + mtime unchanged); pass |
| R1-F1.2 | Same editor, equal whole mapping (block, quoted, comment) → original bytes | fixed | `frontmatter_path_edit::setting_a_block_mapping_to_its_own_value_keeps_its_quotes_and_inner_comment`; CLI `unchanged_saves::setting_a_block_mapping_or_sequence_to_its_own_value_keeps_its_comments`; pass |
| R1-F1.3 | Same editor, equal whole sequence (block, quoted, comment) → original bytes | fixed | `frontmatter_path_edit::setting_a_block_sequence_to_its_own_value_keeps_its_quoted_item_and_comment`; same CLI test; pass |
| R1-F1.4 | CLI destination lookup before the editor on equal value → no indentation lookup | fixed | `md set`/`md rm` pass a lazy `|| destination_indentation(..)`; the library asks it only after the no-op check (counting-closure tests show 0 calls); unchanged `--save` skips the write and reports `saved: false` (`unchanged_saves::an_unchanged_save_reports_that_nothing_was_saved`) |
| R1-F1.5 | `EffectEngine::set_frontmatter` equal title → original bytes, no lookup | fixed | verb returns before `save` (sole `destination_indentation` caller in `verbs.rs`); `markdown_save_indentation::setting_a_property_to_the_value_it_holds_leaves_the_file_unwritten` (exact bytes + mtime, two-space `.editorconfig` repo); pass |
| R1-F1.6 | `EffectEngine::merge_frontmatter` existing title → original bytes, no lookup | fixed | `markdown_save_indentation::merging_values_that_already_hold_leaves_the_file_unwritten`; pass |
| R1-F1.7 | `EffectEngine::delete_frontmatter` absent property → original bytes, no lookup | fixed | `markdown_save_indentation::deleting_an_absent_property_leaves_the_file_unwritten`; pass |
| R1-F1.8 | Plain-YAML editor equal scalar → original bytes (control) | clean | `yaml_edit.rs` guard `if expected == original { return Ok(source.to_string()) }`; `yaml_edit::indentation_of_new_values` pass |
| R1-F1.9 | `ConfigTransaction` incl. Claudine, equal second save → unchanged, no resolver call (control) | clean | `config_write_indentation::the_preference_is_resolved_once_across_batches_and_never_when_nothing_renders` (0 resolver calls, `written: false`); claudine `config_indentation::create_edit_and_an_unchanged_save_round_trip`; pass |
| R1-F1.10 | Hash save, unchanged baseline → no fallback call, no write (control) | clean | `stamp_baseline_with_indentation` calls `fallback()` only inside `if decision.new_stored.is_some()`; `hash.rs` writes only `if let Some(written) = stamp.text`; `baseline::tests::unchanged_content_needs_no_write` |
| R1-F1.11 | Claudine compatibility writer, no changes → not called (control) | clean | `classify.rs` `if changed { write_markdown_document(..) }`; new `linking::compatibility::tests::a_skill_needing_no_upgrade_is_left_unwritten` (exact bytes with quoted, commented `name`); pass |
| R1-F1.12 | Removal CLI; increment/decrement, append/prepend → not a no-op path (control) | clean | `md rm` requires ≥1 prop and errors on a missing key; `bump` always writes n±1 or errors; `array_mutate` always grows or errors; `rm` now resolves lazily too |
| R1-F1.13 | Sibling: DMLS frontmatter providers/code actions → no save path | clean | `dmls/src` calls none of the editor/save APIs; code actions emit LSP insertions of missing keys only |
| R1-F1.14 | Sibling: `claudine/gen` agent-errors check, Schematic OpenAPI export → whole-document regeneration | clean | regenerate from a value with no authored text to preserve; output is deterministic; the no-op rule is under “Edits to existing text” |
| R1-F2   | In-memory document editing ignores explicit indentation | fixed | source-less branch of `serialized_into_source` now uses `as_string_with_indentation(indentation)`; all children verified |
| R1-F2.1 | Source-less branch of `serialized_into_source`, two-space argument → two spaces | fixed | `frontmatter_path_edit::a_document_built_in_memory_is_serialized_with_the_callers_indentation` expects `meta:\n  child:\n    leaf: 1` with one fallback call; pass; fails with the fix reverted (four spaces) |
| R1-F2.2 | Source-present serialization branch → caller preference (control) | clean | new `control_flow_style_frontmatter_falls_back_to_the_callers_indentation` (flow `meta: {child: 0}` → whole-frontmatter fallback, two spaces); pass |
| R1-F2.3 | Source-preserving node edit branch → caller preference (control) | clean | new `control_an_empty_mapping_replaced_in_place_uses_the_callers_indentation` (`meta: {}` via `Layout`, two spaces); pass |
| R1-F2.4 | `Markdown::as_string_with_indentation` → caller preference (control) | clean | `output/string.rs` forwards the argument to `to_yaml_string`; `markdown_save_indentation::a_not_yet_existing_destination_resolves_its_worktrees_preference` (two spaces); pass |
| R1-F2.5 | Default wrappers → four spaces (control) | clean | unsuffixed wrappers delegate with `Indentation::default`; `a_document_built_in_memory_is_serialized`, `output_with_no_destination_nests_four_spaces` (four spaces); pass |
| R1-F2.6 | Sibling sweep: every other function taking an `Indentation`/fallback (Darkmatter hash save, YAML editor, `ConfigTransaction` stage, biscuit-file writer, Schematic export, Claudine compatibility writer) → no dropped argument | clean | code read: none ignores a supplied value; `stamp_baseline_with_indentation`'s plain `apply_hash_save_text` branch is the no-write path; Schematic JSON ignores YAML indentation by design |
| R1-F3   | Four schematic L1 tests retain failing package timings | fixed | all four slow tests repaired; Schematic L1 package run 3192 passed, none ≥ 5 s |
| R1-F3.1 | `generated_docs.rs` generated documentation test → ≤ 5 s alone and in package | fixed | was 5.137 s alone; now 4 cases, package run max 2.391 s (`::openai`), alone 2.28 s |
| R1-F3.2 | `artifact_drift.rs` generated Rust artifact test → ≤ 5 s alone and in package | fixed | was 5.215 s alone; now 4 cases, package run max 2.644 s (`::openai`), alone 2.47 s |
| R1-F3.3 | `fresh_output_compile.rs` generated REST source test → ≤ 5 s alone and in package | fixed | was 5.436 s alone; now 4 cases, package run max 2.554 s (`::openai`), alone 2.43 s |
| R1-F3.4 | `definition_bundles.rs` definition-bundle test → ≤ 5 s alone and in package | fixed | was 4.867 s alone (slow in package); now 3 cases, package run max 2.803 s (`::openai`), alone 2.60 s |
| R1-F3.5 | Feature tests rerun → none at 5 s (control) | clean | full L1 runs: Darkmatter 17301, Claudine 12665, Schematic 3192 — none at or above 5.0 s |
| R1-F3.6 | Coverage guard for the split cases → every REST module in exactly one bounded group | fixed | new `common::generation_groups::every_rest_module_is_in_exactly_one_bounded_generation_group` (0.919 s in package); pass |
| G-1 | Gate discovery: Claudine `dispatch_inventory::dispatch_inventory_matches_committed_file` → committed inventory matches source | fixed | the new R1-F1.11 test added one test-only `Provider::Claude` reference (`reference` class) and earlier edits shifted lines; regenerated with `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`; full Claudine run green |

### Work Log

- starting the work on 'R1-F1 no-op Markdown saves' at 22:52:45
        - defect class: a save path renders and resolves a layout for edits whose resulting values equal the values already stored, instead of returning the source bytes untouched
        - Markdown editor (`edit_frontmatter_paths_with_indentation`) now returns the source bytes when the source reloads to the expected values, before any `Layout` is built
        - decision: the fallback indentation is now a lazy `impl Fn() -> Indentation` (cached per call) on both `edit_frontmatter_paths_with_indentation` and `edited_document_text_with_indentation`, matching `stamp_baseline_with_indentation`; alternative (a third lazy variant) rejected because both functions are new on this branch
        - decision: `md set --save` with an unchanged value skips the write and reports `saved: false` (matches `md hash --save`); documented in `darkmatter/docs/cli/set.md`
        - lifecycle `set_frontmatter`/`merge_frontmatter`/`delete_frontmatter` return before `save` when no value changes; consequence: with auto-rehash, a no-op no longer bumps `last_updated` (documented in `darkmatter/docs/topics/side-effects.md`)
        - docs: `darkmatter/docs/cli/set.md`, `darkmatter/docs/topics/frontmatter.md`, `darkmatter/docs/topics/side-effects.md`, `.claude/skills/darkmatter/frontmatter.md`
        - subagent focused runs: darkmatter 55/55, 253/253, 34/34, 15/15; claudine 42/42; no SLOW; darkmatter clippy clean, `check-zed` fails for missing `wasm32-wasip2` (environment)
- work completed for 'R1-F1 no-op Markdown saves' at 23:06:08
- starting the work on 'R1-F2 in-memory document indentation' at 23:06:08
        - defect class: a serialization branch drops the caller's explicit formatting argument and uses the default layout
        - fixed the source-less branch; doc comment on `serialized_into_source` updated to name `as_string_with_indentation`
        - focused runs: `just test frontmatter_path_edit` 58/58, `just test markdown_save_indentation` 9/9, `just test path_edit` 58/58; no SLOW; clippy clean
        - note: the author committed the working tree concurrently (`291beca21`…`68881e682`); those commits carry R1-F1 and the new R1-F2 tests, while the R1-F2 fix in `darkmatter/lib/src/markdown/hash/path_edit.rs` is still uncommitted; this cycle makes no commits
- work completed for 'R1-F2 in-memory document indentation' at 23:11:39
- starting the work on 'R1-F3 slow Schematic L1 tests' at 23:11:39
        - defect class: an L1 test repeats expensive whole-catalog generation or setup inside one test body, so it exceeds the 5 s budget
        - before (alone, quiet host): docs 5.137 s, Rust drift 5.215 s, REST runtime 5.436 s, bundles 4.867 s
        - production redundancy removed (output byte-identical; drift cases match committed `schema/src`):
                - `DefinitionContext` caches flattened schemas, invalidated on register/compose (`schematic/define/src/registration.rs`)
                - flattening indexes names and skips a nested copy identical to one already agreed, never a `"$ref": "#"` copy (`schematic/define/src/components.rs`); resolving all APIs 1.66 s → 0.86 s
                - large modules are assembled and parsed once (`ModulePieces`, `schematic/gen/src/output/`); 3.27 s → 2.22 s
                - OpenAPI/Postman writers check exportability without a full warning export unless an error exists (`schematic/gen/src/validation.rs`)
                - Postman example validators cached by schema text (`schematic/gen/src/export/examples.rs`)
        - each old test is now a module of bounded group cases (`openai`, `anthropic_to_gitlab`, `huggingface_to_unfolded_circle`, plus crate-root/shared cases); guard `common::generation_groups::every_rest_module_is_in_exactly_one_bounded_generation_group` (every REST module in exactly one group, ≤ 300 endpoints per group)
        - decision: crate-wide sanity totals (`fences > 500`, `checked > 30`) became per-group thresholds; the "exactly one client name" assertion is checked as once in the owning group and zero elsewhere
        - after (alone): slowest new case 2.67 s; module run 34/34, no SLOW; `schematic-define`/`-definitions`/`-gen --lib` 1692 passed; schematic `just lint` clean
        - new test `schematic/define/tests/composition.rs::components_follow_registrations_and_compositions_after_a_read` covers cache invalidation
- work completed for 'R1-F3 slow Schematic L1 tests' at 23:47:36 (package-run measurement follows in the gate run)

### Phase Gates

- starting gate runs at 23:47:36; load: 1.40, 2.21, 1.88
- Schematic `just test`: L1 3192 passed, 23 skipped, `🐢 L1 run recorded: 3192 test(s), none at or above 5.0s` (load ≈ 6 on 32 cores); split cases max 2.803 s in package
        - first attempt: the `test-fresh-output` tier (`slow_` Cargo-build tier, outside L1) failed offline for a missing `target-tuple` crate, as the review recorded
        - fetched it with `cargo fetch` on a throwaway `/tmp` crate pinning `trybuild = "=1.0.122"` (a `cargo fetch` against `schematic/schema/Cargo.toml` re-resolved that committed lock, so the lock was restored with `git checkout` and left unchanged)
        - rerun: `just test-fresh-output` 17/17 passed (the one `slow_` test ran 76.5 s, expected for its Cargo-build tier); `just test-compile-fail` 1/1 passed
- Schematic `just lint`: pass
- Darkmatter `just test`: 17301 passed, 10 skipped, none at or above 5.0 s
- Darkmatter `just lint`: pass, including `check-zed` after `rustup target add wasm32-wasip2` (the missing target the review recorded)
- Claudine `just test`: first run failed only `dispatch_inventory::dispatch_inventory_matches_committed_file` (ledger G-1); after regeneration 12665 passed, 15 skipped, none at or above 5.0 s
- Claudine `just lint`: pass
- `just check-tier-coverage` for claudine, darkmatter, schematic: 0 stranded each

### Completion Report

- **outcome: complete**; elapsed 1 h 21 min (22:51:51 → 00:13 local)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed: Markdown frontmatter editor, `md set`/`md rm` lookup, and lifecycle `set_frontmatter`/`merge_frontmatter`/`delete_frontmatter` leave unchanged documents byte-identical and resolve no indentation; controls R1-F1.8–R1-F1.14 clean
        - R1-F2 fixed: the in-memory (source-less) branch now honors the caller's indentation; controls R1-F2.2–R1-F2.6 clean
        - R1-F3 fixed: the four Schematic tests are split into bounded group cases with a coverage guard, and five redundant production steps were removed; every case is ≤ 2.81 s in the package run
- additional gate defect G-1 (dispatch inventory drift) fixed
- open finding/child IDs: none
- no performance measurement was deferred
- changed files this cycle (uncommitted unless the author's concurrent commits picked them up; this cycle made no commits)
        - R1-F1: `darkmatter/lib/src/markdown/hash/path_edit.rs`, `darkmatter/cli/src/commands/frontmatter.rs`, `darkmatter/lib/src/effects/verbs.rs`, `darkmatter/lib/tests/l1/frontmatter_path_edit.rs`, `darkmatter/lib/tests/l1/markdown_save_indentation.rs`, `darkmatter/cli/tests/l1/get_set_rm.rs`, `claudine/lib/src/linking/compatibility/tests.rs`, `darkmatter/docs/cli/set.md`, `darkmatter/docs/topics/frontmatter.md`, `darkmatter/docs/topics/side-effects.md`, `.claude/skills/darkmatter/frontmatter.md` (committed by the author in `291beca21`…`3bd5b7a39`)
        - R1-F2: `darkmatter/lib/src/markdown/hash/path_edit.rs` (uncommitted fix), `darkmatter/lib/tests/l1/frontmatter_path_edit.rs` (tests already in `291beca21`)
        - R1-F3: `schematic/define/src/{components.rs,registration.rs}`, `schematic/define/tests/composition.rs`, `schematic/gen/src/output/{mod.rs,assemble/api.rs,assemble/mod.rs}`, `schematic/gen/src/{validation.rs,export/openapi.rs,export/postman/mod.rs,export/examples.rs}`, `schematic/gen/tests/common/{mod.rs,generation_groups.rs}`, `schematic/gen/tests/l1/{generated_docs.rs,artifact_drift.rs,fresh_output_compile.rs,definition_bundles.rs}`, `.claude/skills/schematic-define/schematic/gen.md`
        - G-1: `claudine/docs/providers/dispatch-inventory.json`
- verification tied to findings
        - R1-F1/R1-F2: Darkmatter `just test` 17301/17301, `just lint` pass; Claudine `just test` 12665/12665, `just lint` pass
        - R1-F3: Schematic `just test` (L1 3192/3192, none ≥ 5 s; fresh-output 17/17; compile-fail 1/1), `just lint` pass
        - tiers: `just check-tier-coverage` claudine/darkmatter/schematic, 0 stranded
- environment notes: no local OS rigs are configured; set `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` to SSH hosts to enable cross-OS runs; macOS/Windows/WSL2 proof is left to CI
