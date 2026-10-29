---
kind: implementation-log
---

# Final full-area timing corrections: DMLS and CLI matrices

The coordinator's full-area L1 runs found the following over-budget test
identities after the serial spec audits. They are additional observations,
not reruns counted as newly added behavioral coverage.

## Changes and accounting

| Module | Net new identities | New roster guards | Extra scheduling identities |
| --- | ---: | ---: | ---: |
| DMLS `call_check_agreement` | 4 | 1 | 3 |
| DMLS `expression_scopes` | 4 | 1 | 3 |
| CLI `compose_initialize_staged_boot` | 7 | 1 | 6 |
| CLI `compose_schema_cli` | 4 | 1 | 3 |
| CLI `entry_point_parity` | 60 | 1 | 59 |
| CLI `guarded_shell_values` | 6 | 1 | 5 |
| CLI `launch_trigger_schemas` | 5 | 1 | 4 |
| CLI `lifecycle_prep` | 2 | 1 | 1 |
| CLI `shell_missing_values` | 4 | 1 | 3 |
| **Total** | **96** | **9** | **87** |

No original test identity was removed: each original name remains on its
first scheduled group or case. One existing candidate-roster guard was
strengthened. These changes add no production behavior and remove no input,
entry point, assertion, or guarded execution mode.

DMLS now schedules four six-document call-agreement groups and four seven-row
expression-branch groups. The latter retain ternary, conjunction, and
frontmatter hovers for every original single-guard row. Guards pin contiguous,
complete coverage and group-size bounds.

Early catches schedule all seven trigger kinds independently, retaining both
approval modes and every no-side-effect/event-count assertion. Sidecar
advisories schedule all four command/silence combinations independently; inline
persistence is checked even in the silent case.

The entry-point matrix schedules each of its 53 ordinary rows and all 18
supplied-value glob candidates independently. The nested four-thread shard
fanout was removed, leaving parallelism to Nextest. The existing candidate
guard now also checks the actual generated test roster. A new ordinary-row
guard verifies exact row coverage and that every owned entry point has rows.
Only these data-only file-reference fixtures explicitly opt into the shipped
inline metadata trigger through the shared fixture helper: their authored
file, glob, and completion schemas remain unchanged, and no lifecycle or
shipped-trigger validation test uses that opt-in.

The remaining CLI splits retain the selected/unselected guard cases, all four
write-invalidation shapes, positive/negative launch verdicts, target break-code
isolation, and all four missing-shell surfaces. Shipped embedded schemas remain
in use for lifecycle and launch validation. The indivisible inline-preparation
chain is retained unchanged for a fair combined rerun: it must still verify
helper completion, mutation isolation, file persistence, both prompts, and
caller resumption in the same invocation.

## Original full-run observations

| Original identity | Before seconds | After: maximum retained-case seconds |
| --- | ---: | ---: |
| `dmls::l1 call_check_agreement::composition_and_the_editor_reject_the_same_documents` | 5.088 | 0.981 |
| `dmls::l1 expression_scopes::expression_branch_rows_agree_with_the_library` | 5.984 | 0.911 |
| `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell` | 9.477 | 2.376 |
| `claudine-cli::l1 compose_schema_cli::compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it` | 7.080 | 1.348 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference_shard_1` | 6.283 | 0.598 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference_shard_3` | 6.277 | 0.626 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference` | 6.431 | 0.627 |
| `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference_shard_4` | 6.157 | 0.607 |
| `claudine-cli::l1 entry_point_parity::supplied_match_validation_row_0_candidates_part_0` | 5.790 | 0.572 |
| `claudine-cli::l1 entry_point_parity::supplied_match_validation_row_0_candidates_part_1` | 5.793 | 0.586 |
| `claudine-cli::l1 entry_point_parity::supplied_match_validation_row_0_candidates_part_2` | 5.463 | 0.645 |
| `claudine-cli::l1 entry_point_parity::supplied_match_validation_row_1_candidates_part_0` | 5.557 | 0.862 |
| `claudine-cli::l1 entry_point_parity::supplied_match_validation_row_1_candidates_part_1` | 5.258 | 0.829 |
| `claudine-cli::l1 guarded_shell_values::a_lifecycle_guard_proves_its_commit_message_present` | 5.328 | 1.820 |
| `claudine-cli::l1 guarded_shell_values::a_prompt_documents_guarded_command_settles_under_the_sequence_approval` | 6.321 | 2.005 |
| `claudine-cli::l1 guarded_shell_values::a_write_through_an_alias_an_ancestor_or_an_inner_branch_ends_the_guards_proof` | 5.536 | 1.106 |
| `claudine-cli::l1 launch_trigger_schemas::a_sequence_document_is_judged_by_the_embedded_sequence_trigger_before_any_step` | 5.246 | 1.876 |
| `claudine-cli::l1 launch_trigger_schemas::a_templated_control_is_judged_after_composition` | 5.500 | 1.634 |
| `claudine-cli::l1 launch_trigger_schemas::compose_reports_an_omitted_authored_property_and_still_applies_the_payload` | 5.442 | 1.557 |
| `claudine-cli::l1 launch_trigger_schemas::inline_completion_rejects_an_invalid_control_the_agent_wrote` | 5.076 | 1.861 |
| `claudine-cli::l1 lifecycle_prep::prep_dispatches_an_inline_prompt_before_resuming_its_caller` | 5.269 | 2.769 |
| `claudine-cli::l1 lifecycle_prep::a_targets_break_stays_in_the_target_and_its_code_decides_the_call` | 6.931 | 2.238 |
| `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface` | 5.003 | 0.945 |

There are **23 observed slow identities** in this assigned scope. Twenty-two
are directly changed by the splits/setup above; the inline-preparation chain
is intentionally kept intact and now measures 2.769 seconds after shared
contention was removed. That
chain was itself new coverage added during serial step 15, so it must not be
misreported as an additional original pre-audit test. Slow-test totals should
be deduplicated by exact identity against the serial reports.

## Verification

Edits and `git diff --check` completed while the coordinator's full L1 gate
was still running. No child build, test, lint, or formatting command overlapped
that gate. The coordinator owns the combined parallel rerun and supplies its
results below; the pre-change full-run timings are not post-change proof. The combined
post-change verification below is that proof.


## Combined verification result

The coordinator's final canonical, parallel L1 invocation passed **1,269 of
1,269 tests**, with **zero tests over five seconds** and an overall maximum of
**4.751 seconds**. Log: `/tmp/pipeline-final-speed-verified.log`. These are
1269 combined observations, not 1269 newly added tests.
The exact combined filter, package/feature recipe, and complete evidence paths
are recorded in [the coordinator's verification report](verification.md).

The exact owned package/module slices comprise **292 passing tests**:

The coordinator ran a wider combined scope. To replay only the owned slices
from the repository root, use the following scoped canonical command; it was
not run again separately after the successful combined observation:

```sh
BISCUIT_TEST_FILTER='(package(dmls) & (test(call_check_agreement::) | test(expression_scopes::))) | (package(claudine-cli) & (test(compose_initialize_staged_boot::) | test(compose_schema_cli::) | test(entry_point_parity::) | test(guarded_shell_values::) | test(launch_trigger_schemas::) | test(lifecycle_prep::) | test(shell_missing_values::)))' just test dmls claudine-cli
```

| Package/module | Passing identities | Maximum seconds |
| --- | ---: | ---: |
| DMLS `call_check_agreement` | 6 | 0.981 |
| DMLS `expression_scopes` | 15 | 0.911 |
| Claudine CLI `compose_initialize_staged_boot` | 21 | 2.376 |
| Claudine CLI `compose_schema_cli` | 78 | 2.315 |
| Claudine CLI `entry_point_parity` | 75 | 1.850 |
| Claudine CLI `guarded_shell_values` | 19 | 2.508 |
| Claudine CLI `launch_trigger_schemas` | 57 | 2.460 |
| Claudine CLI `lifecycle_prep` | 15 | 2.795 |
| Claudine CLI `shell_missing_values` | 6 | 1.810 |

The after column compares each original matrix with the maximum of the
scheduled cases that preserve its original footprint, rather than with only
the first surviving test name. For the former six parity shards and
three-part candidate sweeps, this follows their original modulo assignment.
The inline-preparation chain stayed unchanged and passed at **2.769 seconds**;
it is a load-sensitive observation resolved by removing contention, not a
modified slow test or additional original pre-audit identity.

The first combined attempt compiled but stopped on an unrelated branch-fixture
failure, which its owner repaired. Five doc comments moved by these splits
were changed to ordinary contract comments to avoid unattached-doc warnings;
no lint allowances were added. The successful combined rerun supersedes that
interrupted attempt. `git diff --check` passes. The coordinator owns final
lint; this child ran no overlapping build/test/lint and no formatting command.
No production code, CI configuration, skip, timeout, or retry policy changed.
