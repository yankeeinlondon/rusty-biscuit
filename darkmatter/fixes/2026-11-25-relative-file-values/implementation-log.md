---
spec: /home/build/wt/rusty-biscuit/feat-fix-sequence/darkmatter/fixes/2026-11-25-relative-file-values/spec.md
plan: darkmatter/fixes/2026-11-25-relative-file-values/plan.md
implemented-by: codex/default
started-phase: "1"
source_files_during_phase_1:
  - biscuit-file/cli/src/main.rs
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/justfile
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/portable/diagnostics.rs
  - biscuit-file/lib/src/file_reference/portable/evaluate.rs
  - biscuit-file/lib/src/file_reference/portable/mod.rs
  - biscuit-file/lib/src/file_reference/portable/path_identity.rs
  - biscuit-file/lib/src/file_reference/portable/shadow.rs
  - biscuit-file/lib/src/file_reference/portable/strategy.rs
  - biscuit-file/lib/src/file_reference/portable/text/tests.rs
  - biscuit-file/lib/src/file_reference/portable/text.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/src/path_text.rs
  - biscuit-file/lib/tests/l1/file_tree.rs
  - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
  - biscuit-file/lib/tests/l1/glob_reference/directory_case.rs
  - biscuit-file/lib/tests/l1/glob_reference/grammar.rs
  - biscuit-file/lib/tests/l1/glob_reference/literal.rs
  - biscuit-file/lib/tests/l1/glob_reference/unreadable.rs
  - biscuit-file/lib/tests/l1/launch_lookup.rs
  - biscuit-file/lib/tests/l1/magic_local_roots.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/tests/l1/portable_path/mod.rs
  - biscuit-file/lib/tests/l1/portable_path/originating_context.rs
  - biscuit-file/lib/tests/l1/portable_path/platform.rs
  - biscuit-file/lib/tests/l1/value_spelling.rs
  - claudine/cli/src/cli_utils.rs
  - claudine/cli/src/commands/agents.rs
  - claudine/cli/src/commands/compose/prep/tests.rs
  - claudine/cli/src/commands/config_tui/mod.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init/prompts.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/cli/src/commands/link_display.rs
  - claudine/cli/src/commands/mcp/init.rs
  - claudine/cli/src/commands/prep_target.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/setup.rs
  - claudine/cli/src/commands/signals.rs
  - claudine/cli/src/commands/skills.rs
  - claudine/cli/src/commands/slash_commands.rs
  - claudine/cli/src/commands/uninstall.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/target.rs
  - claudine/cli/src/commands/wrap/composition/timeouts.rs
  - claudine/cli/src/commands/wrap/env/package_context.rs
  - claudine/cli/src/commands/wrap/exec/watchdog/spawn.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/provider_extension_and_opencode.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
  - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
  - claudine/cli/src/commands/wrap/session_report.rs
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/completion/autocomplete_ui.rs
  - claudine/cli/src/completion/composition/compose.rs
  - claudine/cli/src/completion/composition/setter_value.rs
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/schema_completion/candidates.rs
  - claudine/cli/src/completion/schema_completion/parity_tests.rs
  - claudine/cli/src/completion/schema_completion/tests.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/completion/setter_value.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/src/steering/owner.rs
  - claudine/cli/src/steering/tests.rs
  - claudine/cli/tests/l1/compose_caller_file_provenance.rs
  - claudine/cli/tests/l1/compose_schema_cli.rs
  - claudine/cli/tests/l1/contamination_probes.rs
  - claudine/cli/tests/l1/ctx_launch_anchor.rs
  - claudine/cli/tests/l1/entry_point_parity.rs
  - claudine/cli/tests/l1/home_lookup_round_trip.rs
  - claudine/cli/tests/l1/inline_compose_cli.rs
  - claudine/cli/tests/l1/invalid_trigger_warnings.rs
  - claudine/cli/tests/l1/level1_provider_overlay_home.rs
  - claudine/cli/tests/l1/override_boundary_guard.rs
  - claudine/cli/tests/l1/setup_command.rs
  - claudine/cli/tests/l1/switch_catalog_guard.rs
  - claudine/cli/tests/level2/level2_file_resolution_capture.rs
  - claudine/cli/tests/prompts/pr_flow_rehearsal.rs
  - claudine/cli/tests/prompts/review_fix_rehearsal.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/main.rs
  - claudine/gen/src/report.rs
  - claudine/lib/src/composition/coordinator/commit.rs
  - claudine/lib/src/composition/coordinator/invocation.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/lifecycle.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/provider.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/file_detail.rs
  - claudine/lib/src/composition/guardrails.rs
  - claudine/lib/src/composition/launch_workspace.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
  - claudine/lib/src/composition/lifecycle/prep.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/resolve/tests.rs
  - claudine/lib/src/composition/resolve.rs
  - claudine/lib/src/composition/runtime_state.rs
  - claudine/lib/src/composition/schema/status_render.rs
  - claudine/lib/src/composition/schema/tests.rs
  - claudine/lib/src/composition/sequence/data.rs
  - claudine/lib/src/composition/sequence/declarations.rs
  - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
  - claudine/lib/src/composition/sequence/preflight/mod.rs
  - claudine/lib/src/composition/sequence/task/tests.rs
  - claudine/lib/src/error.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/resolve/tests.rs
  - claudine/lib/src/invocation_context/document_tests.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/linking/compatibility/classify.rs
  - claudine/lib/src/linking/mod.rs
  - claudine/lib/src/linking/symlink.rs
  - claudine/lib/src/mcp/export.rs
  - claudine/lib/src/mcp/import/tests.rs
  - claudine/lib/src/mcp/import.rs
  - claudine/lib/src/mcp/validation.rs
  - claudine/lib/src/permissions/query.rs
  - claudine/lib/src/render/prompt/system/tests.rs
  - claudine/lib/src/render/prompt/system.rs
  - claudine/lib/src/stream/path_link.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
  - claudine/lib/tests/l1/installed_registration.rs
  - claudine/lib/tests/l1/invalid_trigger_findings.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/output/mod.rs
  - darkmatter/cli/src/render_style.rs
  - darkmatter/cli/tests/l1/compose_setters.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/cli/tests/l1/dynamic_schema_slots.rs
  - darkmatter/cli/tests/l1/entry_point_parity.rs
  - darkmatter/cli/tests/l1/load_data.rs
  - darkmatter/cli/tests/l1/output_envelope.rs
  - darkmatter/cli/tests/l1/render_style_files.rs
  - darkmatter/cli/tests/l1/render_style_wiring.rs
  - darkmatter/dmls/src/workspace/discover.rs
  - darkmatter/dmls/tests/l1/schema_slots_editor.rs
  - darkmatter/dmls/tests/l1/value_hovers.rs
  - darkmatter/dmls/tests/l1/workspace_file_list.rs
  - darkmatter/lib/src/config/migration_receipt.rs
  - darkmatter/lib/src/config/write.rs
  - darkmatter/lib/src/lifecycle/error.rs
  - darkmatter/lib/src/lifecycle/executor.rs
  - darkmatter/lib/src/lifecycle/preflight.rs
  - darkmatter/lib/src/markdown/compose/context/capture/invocation.rs
  - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture/repo.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/context/request.rs
  - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs_tests.rs
  - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
  - darkmatter/lib/src/markdown/compose/expression/read_gate.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
  - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
  - darkmatter/lib/src/markdown/compose/preflight/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/reference/report.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/rewrite.rs
  - darkmatter/lib/src/markdown/schemas/schema_files.rs
  - darkmatter/lib/src/markdown/schemas/slots/approval_tests.rs
  - darkmatter/lib/src/markdown/schemas/slots/cache.rs
  - darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs
  - darkmatter/lib/src/markdown/schemas/slots/session.rs
  - darkmatter/lib/src/markdown/schemas/slots/session_tests.rs
  - darkmatter/lib/src/markdown/schemas/slots/tests.rs
  - darkmatter/lib/tests/common/entry_point_parity/mod.rs
  - darkmatter/lib/tests/l1/compose_read_set.rs
  - darkmatter/lib/tests/l1/compose_setters.rs
  - darkmatter/lib/tests/l1/config_init.rs
  - darkmatter/lib/tests/l1/config_migration_receipt.rs
  - darkmatter/lib/tests/l1/context_functions.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_precedence.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_read_order.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_template_triggers.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs
  - darkmatter/lib/tests/l1/dynamic_schema_slots.rs
  - darkmatter/lib/tests/l1/editor_observation.rs
  - darkmatter/lib/tests/l1/entry_point_parity.rs
  - darkmatter/lib/tests/l1/expression_recovery_data.rs
  - darkmatter/lib/tests/l1/expression_recovery_slots.rs
  - darkmatter/lib/tests/l1/file_predicates.rs
  - darkmatter/lib/tests/l1/file_tree_roots.rs
  - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
  - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
  - darkmatter/lib/tests/l1/glob_consumers.rs
  - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
  - darkmatter/lib/tests/l1/lifecycle_edit_action.rs
  - darkmatter/lib/tests/l1/link_interpolation_integration.rs
  - darkmatter/lib/tests/l1/load_data_destination.rs
  - darkmatter/lib/tests/l1/load_data_diagnostics.rs
  - darkmatter/lib/tests/l1/load_data_execution.rs
  - darkmatter/lib/tests/l1/load_data_options.rs
  - darkmatter/lib/tests/l1/load_data_options_slots.rs
  - darkmatter/lib/tests/l1/load_data_runtime.rs
  - darkmatter/lib/tests/l1/load_data_uncontracted.rs
  - darkmatter/lib/tests/l1/match_keyword_input_matrix.rs
  - darkmatter/lib/tests/l1/nested_composition.rs
  - darkmatter/lib/tests/l1/path_function_contracts.rs
  - darkmatter/lib/tests/l1/recursive_schema_rewrite.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - darkmatter/lib/tests/l1/request_context_builder.rs
  - darkmatter/lib/tests/l1/required_context.rs
  - darkmatter/lib/tests/l1/schema_roots.rs
  - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
  - darkmatter/lib/tests/l1/slot_permissions.rs
  - playa/cli/src/main.rs
  - simplified-schema/lib/src/file_match.rs
  - simplified-schema/lib/src/format.rs
  - simplified-schema/lib/src/resolve/bundle.rs
  - simplified-schema/lib/tests/l1/bundle_resolution.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/tests/l1/cli.rs
  - sniff/cli/tests/l1/snapshots.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/level2_god_files.rs
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/lib/src/file/naming.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
docs_updated_during_phase_1:
  - biscuit-file/README.md
  - biscuit-file/cli/README.md
  - biscuit-file/docs/dependencies.md
  - biscuit-file/docs/topics/file-references.md
  - darkmatter/docs/topics/magic-paths.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
  - .claude/skills/biscuit-file/references/cli.md
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows-paths.md
packages:
  - biscuit-file
  - biscuit-file-cli
  - claudine
  - claudine-cli
  - claudine-gen
  - darkmatter
  - darkmatter-cli
  - darkmatter-lsp
  - playa-cli
  - simplified-schema
  - sniff
  - sniff-detection-cli
  - tree-hugger
  - tree-hugger-cli
  - worktree
decisions-for-review:
  - |-
    For a missing one-component path, the nearer base itself is the parent.
    I check that parent even when the first component is absent. The plan's
    absent-component shortcut would otherwise print new.md for a file meant
    for the repository root even though planning would write it in tools/.
    Restoring the shortcut would require changing output planning or accepting
    that those two operations name different files.
  - |-
    I added an optional admission hook to ShadowProbe alongside its required
    exists method. The hook can authorize a candidate before a containment
    check reads metadata. Using exists alone would either read metadata before
    authorization or perform containment after probing the candidate. Removing
    the hook would require another way to authorize reads before both checks.
message-to-agent: |-
  Phase 2 can opt into ImplicitBases::Launch and use the frozen launch
  context through derivations. Select or plan local paths before calling
  PortablePath::from_path with VALUE_STRATEGY. Reuse a request-scoped
  ShadowCache and supply known foreign checkout roots. Put the read policy
  and read-set accounting in ShadowProbe::admit so it runs before shadow
  containment and existence checks. ShadowCache::probe_count counts
  cached first-component observations, and full entry or parent checks
  also go through the probe. Existing file admission and target access
  checks remain the responsibility of the selecting caller.
  Derive from the launch context when admitting an external source. The
  with_launch_scope builder copies the anchors used by @, while derivation
  also retains the frozen launch tree used to check implicit paths. Include
  the implicit lookup mode when keying a cache of resolution outcomes.
---

# Implementation Log for 2026-11-25-relative-file-values (6 phases)

## Phase 1

The public API and temporary filesystem form the F2/L1 boundary for the new behavior. Existing resolver and portability suites cover authoring and default strategy compatibility. The targeted regressions cover launch order and derived contexts, completion and glob results, containment and I/O, directory selection and missing outputs, and portable spellings with shadow and fallback cases. Phase 1 does not change a configuration loader or shipped schema.

### Requirements and tests

| Phase 1 contract | Public test evidence |
| --- | --- |
| R1–R5, ordered bases and isolation | `launch_lookup::bases_deduplicate_in_launch_order`, `outside_repository_uses_only_launch_directory`, `launch_package_area_and_repository_shadows_are_tried_in_order`, and the existing `implicit_relative` suite for authoring defaults |
| R4, lookup through each consumer | `launch_lookup::completion_glob_and_recursive_search_share_launch_roots` |
| R5, boundaries and failure semantics | `launch_lookup::launch_and_source_keep_their_own_tree_boundaries`, `a_launch_symlink_escape_stops_before_the_repository_match`, `launch_lookup_preserves_host_miss_and_io_semantics`. Win32 reports the file-in-parent-path fixture as a miss. Unix reports an I/O failure. |
| R6–R7, entry type, provenance, and output planning | `launch_lookup::directory_probe_skips_a_nearer_file_and_reports_the_matching_base` and `plan_uses_the_first_existing_parent_or_the_launch_directory`. Inputs include the plan's two review/output values, an explicit relative path, environment expansion, and recursive refusal. |
| R8 and R10, spelling and eligibility | `value_spelling::spec_directory_and_index_paths_are_short_and_resolve_back`, `repository_root_and_foreign_checkout_do_not_get_implicit_spellings`, `shadow_and_outside_launch_use_environment_home_and_absolute_in_order`, and `a_missing_target_requires_the_same_planned_parent` |
| R9, probe count and refusal | `value_spelling::shared_cache_probes_one_first_component_per_nearer_base`, `a_denied_shadow_check_records_the_probe_and_uses_the_fallback`, `deep_shadow_check_can_be_refused_before_probing_the_candidate`, and `a_symlinked_launch_base_outside_the_tree_cannot_prove_a_short_value` |
| R11, native spelling and unrepresentable names | `portable_path::platform::a_unix_backslash_in_a_name_is_never_split` and `a_non_unicode_target_is_unrenderable_even_with_absolute_path`, `portable::text::tests::a_unix_backslash_name_uses_native_text_to_preserve_components`, and the Windows-only `value_spelling::absolute_value_names_the_native_windows_target` |

The focused command was `cargo nextest run -p biscuit-file --features fetch,delimited --lib --test l1 -E 'test(launch_lookup::) | test(value_spelling::) | test(backslash) | test(non_unicode_target)'`.
The full Biscuit File suite passed 1,344 default-feature tests and six syntax-only tests. Its syntax-only invocation filtered out 126 other tests. Biscuit File lint passed in all four recipe configurations.

The scope command was `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`. It returned `change_class: package` and a nonempty package list covering the packages in this log's frontmatter plus `test-toolkit` through its archive-path guard. Full Biscuit File and Darkmatter gates apply.

The Rust renames are workspace-wide, and references to the renamed APIs were updated in live documentation and skills. Earlier specs and plans retain their original names. There is no configuration migration or new persistence path in this phase.

Darkmatter's initial L1 run executed 18,147 of 18,314 selected tests and
recorded no slow tests. The existing DMLS shipped-review hover test failed
because its temporary directory copied `feature-review.yaml` without the
sibling import `review-fix-cycle.yaml`. The isolated rerun reproduced the
same missing-import error in 56 ms. The fixture now copies both shipped
schemas. The hover assertion and shipped schemas retain their original text.

The first native Windows suite passed 1,264 of 1,265 tests and identified
a Unix-only assumption in the new blocked-parent fixture. The fixture now
checks the same miss and error semantics the resolver already used on each
host. The corrected Windows case passed alone in 20 ms.

### Verification gates

- Biscuit File `just test` and `just lint` passed on Linux. The default
  suite ran 1,344 tests and the syntax-only smoke suite ran six.
- Darkmatter `just test` passed after the fixture repair. The isolated hover
  test took 88 ms. The full rerun passed 18,314 tests, with ten excluded by the
  normal L1 selection. No executed L1 test reached five seconds. Darkmatter
  `just lint` passed, including the Zed WASM check.
- `just check-tier-coverage biscuit-file darkmatter` found no stranded tests.
  The new filesystem tests run in L1. Phase 1 introduced no terminal or browser
  behavior that requires L2, L3, browser, or live-service tests.
- `just cross-check biscuit-file --os windows` passed all 1,265 tests after
  the isolated rerun. The Windows absolute-path round trip passed, with no
  L1 test at or above five seconds. This was a working-tree validation run.
- `cargo check --workspace --all-targets` passed after the rename and again
  on the completed Phase 1 source. A source scan found no retired API names
  outside historical specs and plans.

### Dependent-package verification

The targeted checks ran serially. The final Biscuit File focused rerun passed
all 30 Linux tests, including the renamed host-semantics regression.

| Package | Selected tests | Passed | Filtered |
| --- | --- | ---: | ---: |
| simplified-schema | Schema files, reference error provenance, and eager file validation of an external document | 23 | 2,570 |
| claudine | Composition resolution and captured document contexts | 43 | 7,249 |
| claudine-cli | Completion round trips and setter completion | 20 | 3,174 |
| claudine-gen | Report rendering | 10 | 97 |
| sniff | Git worktree library | 19 | 2,021 |
| sniff-detection-cli | L1 worktree cases | 27 | 456 |
| tree-hugger | File naming and path projections | 7 | 258 |
| tree-hugger-cli | Grouping reports | 5 | 2,328 |
| worktree | Removal lifecycle | 59 | 811 |
| biscuit-terminal | Existing repository-relative image resolution | 1 | 2,132 |
| content-policy | File adapter with its feature enabled | 13 | 0 |
| messenger | `research_validation` with `--features research` | 31 | 0 |
| messenger-cli | Prepared run checks resolve under another root | 1 | 12 |

The initial Messenger library command selected zero tests because its research
target is feature-gated. The corrected command ran all 31 tests above.
