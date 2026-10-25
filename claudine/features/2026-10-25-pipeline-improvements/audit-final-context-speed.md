---
kind: implementation-log
---

# Final Context and Reference Test Speed Audit

The full Claudine L1 run exposed **20 test identities over five seconds** in
this slice. Nineteen are additional distinct original identities;
`embedded_sequence_types_exports_resolve_without_disk_schemas` had already
been repaired during the per-spec audits and regressed under broader parallel
load, so it must not be counted twice in the aggregate. The log is
`/tmp/pipeline-final-claudine-l1.log`.

| Original identity, relative to its test module | Before (seconds) | After (seconds) |
| --- | ---: | ---: |
| `invocation_context::tests::current_branch_observes_a_branch_switched_after_launch` | 5.044 | 0.450 |
| `invocation_context::document_tests::a_preparation_without_a_snapshot_reads_its_resolution_context_map` | 5.101 | 0.681 |
| `invocation_context::document_tests::every_consumer_of_the_launch_document_reads_its_captured_map` | 5.216 | 0.706 |
| `invocation_context::document_tests::a_second_document_reads_its_own_map_and_the_first_keeps_its_own` | 5.247 | 0.760 |
| `invocation_context::document_tests::re_preparing_a_document_captures_no_new_environment` | 5.247 | 0.737 |
| `claudine_schemas_embedded::embedded_sequence_group_export_resolves_without_disk_schemas` | 5.375 | 2.180 |
| `claudine_schemas_embedded::embedded_sequence_step_export_resolves_without_disk_schemas` | 5.435 | 2.100 |
| `claudine_schemas_embedded::embedded_sequence_task_export_resolves_without_disk_schemas` | 5.431 | 2.057 |
| `claudine_schemas_embedded::embedded_sequence_types_exports_resolve_without_disk_schemas` | 5.509 | 2.055 |
| `compose_prompt_tiers::nested_magic_reference_in_another_repository_resolves_from_the_launch_tree` | 6.282 | 0.468 |
| `compose_prompt_tiers::local_root_file_wins_over_user_prompt_tier_when_the_local_tree_is_home` | 7.144 | 0.415 |
| `ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents` | 8.157 | 2.474 |
| `ctx_per_run::a_proxy_target_observes_its_sources_staging_regardless_of_first_mention` | 6.395 | 1.890 |
| `document_environment_routes::re_entering_a_document_reads_its_map_without_capturing_again` | 6.111 | 2.132 |
| `document_environment_routes::a_step_runs_approved_text_only_while_its_map_leaves_it_unchanged` | 8.384 | 1.046 |
| `research_sequence_artifacts::shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli` | 6.548 | 4.751 |
| `sequence_magic_reference::sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored` | 5.034 | 1.264 |
| `sequence_overlay_pty::pty_sequence_authored_row_projection_satisfies_required_property` | 6.442 | 2.593 |
| `sequence_schema::sequence_shell_expanded_value_violating_schema_aborts_step` | 5.741 | 1.760 |
| `completion::schema_completion::tests::shipped_container_schema_resolves_for_authored_parameter_completion` | 6.062 | 0.636 |

## Changes and coverage preservation

The four library environment-preparation cases explicitly load the real
shipped inline metadata schema from a fixture-owned user schema root. Their
documents contain no lifecycle controls; expanding unrelated lifecycle types
does not contribute to their environment-ownership assertions. Every
expression, transclusion, approved and executed shell, schema-root, model,
snapshot, and capture-count assertion remains. Each test keeps its
`include_str!` literal at the test site. Lifecycle-bearing CLI cases retain
the full schemas.

The branch-refresh case retains its real initial commit and real
`git checkout`. Fixture commits explicitly disable signing so host signing
configuration cannot add work or block setup. An attempted unborn-branch
fixture failed the real observation contract (`branch` was null), so the
committed fixture was restored before final verification.

Each embedded sequence-export test now resolves only the export it judges.
Previously all four exports were expanded independently in every test even
though only one export's positive/negative values were checked. The existing
roster guard still requires one scheduled test for each export. Positive and
negative validation, embedded origin, absence of disk dependencies, and
absence of extracted schema files remain checked for all four exports.

The two prompt-tier matrices, loop-source positions, proxy first-mention
states, approval environment states, and magic/source-relative sequence
references now schedule their cases separately. All original cases and
assertions remain, with six roster guards preventing
unscheduled cases. Reference-only fixtures also explicitly select the real
shipped metadata schema at the fixture user schema root; source and launch
trees, competing paths, and actual CLI reference resolution remain real.

The authored-row PTY and post-shell enum-validation cases also use the shipped
metadata override for the unrelated top-level lifecycle trigger. Their full
sequence trigger, authored schemas, real collection/expansion behavior, and
provider-launch assertions remain. The container-completion test retains all
three actual shipped schema files, its completion assertion, and the same
structural acceptance/refusal controls. It uses a fixture-owned home and the
effective schema's already-compiled validator for those raw structural
controls instead of asking the diagnostic/coercion walkers to visit them.

The research-fleet write/read/reread chain is deliberately unchanged: its
causal persistence proof cannot be separated into independent scheduling cases
without losing information. It passed in **4.751 seconds** in the final
combined run after the surrounding contention was reduced; it is not counted
as a modified slow test.

The combined loop/retry environment case uses the two loop passes necessary
to distinguish first entry from loop reentry, retaining a retry within that
loop. It now requires three starts and two successful passes. The previous
`starts >= 2` assertion could be satisfied by the retry alone and did not
prove that the loop advanced. Captured maps, file-reference results,
finalization, and the single new-document capture remain asserted.

## Count changes

- **20** over-budget identities identified; **19** modified and one research
  chain retained unchanged and now measured below the budget. One modified identity
  was already repaired earlier, so this is **18 additional distinct original
  slow identities modified**, plus that repeated repair.
- **20** gross and net added test identities; **zero** removed identities.
- The 20 additions comprise **14 scheduling identities** and **six roster
  guard identities**. They do not represent 20 new functional scenarios.
- **One** existing test strengthened to independently prove both loop and
  retry transitions.
- **Zero** production behavior changes, timeout increases, skips, retries,
  runner-configuration changes, or tier changes.

## Verification

The coordinator's final combined parallel L1 run passed **1,269/1,269 tests**
with no test over five seconds. Log: `/tmp/pipeline-final-speed-verified.log`;
Nextest run ID `67830caf-931f-47fc-80d7-cc8c45165925`. The canonical root
`just test` invocation selected `claudine`, `claudine-cli`, `claudine-gen`,
`darkmatter-cli`, `dmls`, and `messenger` with the coordinator's scoped filter.
Compilation took 9.37 seconds and test execution 81.071 seconds; compile time
is separate from test duration.

The **582 selected identities** in this slice all passed. Module-level counts
and maxima include the retained regression siblings, not just the original
slow identities in the before/after table:

| Module | Tests passed | Maximum (seconds) |
| --- | ---: | ---: |
| `invocation_context` | 53 | 0.760 |
| `claudine_schemas_embedded` | 409 | 3.542 |
| `compose_prompt_tiers` | 27 | 1.888 |
| `ctx_launch_anchor` | 12 | 2.511 |
| `ctx_per_run` | 30 | 2.407 |
| `document_environment_routes` | 17 | 2.296 |
| `research_sequence_artifacts` | 6 | 4.751 |
| `sequence_magic_reference` | 6 | 2.045 |
| `sequence_overlay_pty` | 7 | 3.328 |
| `sequence_schema` | 14 | 2.399 |
| Shipped container-schema completion control | 1 | 0.636 |

The initial combined attempt caught the invalid unborn-branch simplification;
its fail-fast outcome is preserved in `/tmp/pipeline-final-speed-combined.log`.
The final run uses the restored committed fixture. No production assertion
was relaxed to accept the null branch.

`git diff --check` passes. This agent ran no builds or tests concurrently
with the coordinator's full or combined gates. This report records local
macOS evidence; no additional cross-OS run is claimed.
