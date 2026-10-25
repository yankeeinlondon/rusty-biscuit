---
kind: implementation-log
---

# Final Context and Reference Test Speed Audit

The full Claudine L1 run exposed **15 additional distinct original test
identities over five seconds** in this slice. These are additional to the
slow identities repaired during the per-spec audits. Its log is
`/tmp/pipeline-final-claudine-l1.log`.

| Original identity, relative to its test module | Before (seconds) |
| --- | ---: |
| `invocation_context::tests::current_branch_observes_a_branch_switched_after_launch` | 5.044 |
| `invocation_context::document_tests::a_preparation_without_a_snapshot_reads_its_resolution_context_map` | 5.101 |
| `invocation_context::document_tests::every_consumer_of_the_launch_document_reads_its_captured_map` | 5.216 |
| `invocation_context::document_tests::a_second_document_reads_its_own_map_and_the_first_keeps_its_own` | 5.247 |
| `invocation_context::document_tests::re_preparing_a_document_captures_no_new_environment` | 5.247 |
| `claudine_schemas_embedded::embedded_sequence_group_export_resolves_without_disk_schemas` | 5.375 |
| `claudine_schemas_embedded::embedded_sequence_step_export_resolves_without_disk_schemas` | 5.435 |
| `claudine_schemas_embedded::embedded_sequence_task_export_resolves_without_disk_schemas` | 5.431 |
| `claudine_schemas_embedded::embedded_sequence_types_exports_resolve_without_disk_schemas` | 5.509 |
| `compose_prompt_tiers::nested_magic_reference_in_another_repository_resolves_from_the_launch_tree` | 6.282 |
| `compose_prompt_tiers::local_root_file_wins_over_user_prompt_tier_when_the_local_tree_is_home` | 7.144 |
| `ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents` | 8.157 |
| `ctx_per_run::a_proxy_target_observes_its_sources_staging_regardless_of_first_mention` | 6.395 |
| `document_environment_routes::re_entering_a_document_reads_its_map_without_capturing_again` | 6.111 |
| `document_environment_routes::a_step_runs_approved_text_only_while_its_map_leaves_it_unchanged` | 8.384 |

## Changes and coverage preservation

The four library environment-preparation cases explicitly load the real
shipped inline metadata schema from a fixture-owned user schema root. Their
documents contain no lifecycle controls; expanding unrelated lifecycle types
does not contribute to their environment-ownership assertions. Every
expression, transclusion, approved and executed shell, schema-root, model,
snapshot, and capture-count assertion remains. Each test keeps its
`include_str!` literal at the test site. Lifecycle-bearing CLI cases retain
the full schemas.

The branch-refresh case now switches between unborn branches instead of
creating an unrelated initial commit. It still observes the real Git branch
before and after a real `git checkout`. Fixture commits used elsewhere in
the module explicitly disable signing so host signing configuration cannot
add work or block setup.

Each embedded sequence-export test now resolves only the export it judges.
Previously all four exports were expanded independently in every test even
though only one export's positive/negative values were checked. The existing
roster guard still requires one scheduled test for each export. Positive and
negative validation, embedded origin, absence of disk dependencies, and
absence of extracted schema files remain checked for all four exports.

The two prompt-tier matrices, loop-source positions, proxy first-mention
states, and approval environment states now schedule their cases separately.
All original cases and assertions remain, with five roster guards preventing
unscheduled cases. Reference-only fixtures also explicitly select the real
shipped metadata schema at the fixture user schema root; source and launch
trees, competing paths, and actual CLI reference resolution remain real.

The combined loop/retry environment case uses the two loop passes necessary
to distinguish first entry from loop reentry, retaining a retry within that
loop. It now requires three starts and two successful passes. The previous
`starts >= 2` assertion could be satisfied by the retry alone and did not
prove that the loop advanced. Captured maps, file-reference results,
finalization, and the single new-document capture remain asserted.

## Count changes

- **15** original slow identities addressed in this slice.
- **18** gross and net added test identities; **zero** removed identities.
- The 18 additions comprise **13 scheduling identities** and **five roster
  guard identities**. They do not represent 18 new functional scenarios.
- **One** existing test strengthened to independently prove both loop and
  retry transitions.
- **Zero** production behavior changes, timeout increases, skips, retries,
  runner-configuration changes, or tier changes.

## Verification

`git diff --check` passes. This agent did not run builds or tests while the
coordinator's full gates were running. The coordinator owns the combined
parallel L1 verification of the accumulated final speed edits; results will
be added here when available.
