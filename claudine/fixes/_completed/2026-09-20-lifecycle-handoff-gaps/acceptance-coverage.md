---
created: 2026-09-28
---

# Acceptance Coverage: Lifecycle Handoff Gaps

This table maps every bullet of the spec's "Verification" and "Additional acceptance coverage" sections, and every row of each per-finding table in `review-1.md`, to the tests that prove it. It was audited on 2026-09-28 by reading each test body, not its name. Tests marked **(new)** were added during that audit because the requirement had no test, or had only a neighboring one.

Every test listed is compiled by a declared target:

- `claudine-cli` L1 files are `mod` entries in `cli/tests/l1/main.rs`, and the L2 file is a `mod` entry in `cli/tests/level2/main.rs`, which needs `terminal-tests`, one of the package's CI features.
- `claudine` library tests are `#[cfg(test)]` unit modules or `tests/l1/main.rs` modules.
- Darkmatter and DMLS tests are modules of their `[[test]] l1` targets or `#[cfg(test)]` units.
- `biscuit-terminal` tests are `#[cfg(test)]` units.

No L1 test has a `level2_`, `level3_`, `browser_`, `real_`, or `slow_` name segment. No L1 test needs a feature outside its package's `[package.metadata.ci.tests] features`. Repository files are read through `include_str!` or a literal joined onto `manifest_dir!()` in the same expression. The platform gates are:

- `lifecycle_downgrade_outcome::inline_success_error_fails_with_one_diagnostic` is `cfg(unix)`. The compose rows cover the same `drive_terminal_recovery` path on Windows.
- `level2_dry_run_metadata_capture` is `cfg(unix)`, the recorded Windows L2 policy gap.
- Stub providers have `cfg(unix)`/`cfg(windows)` twins.

Abbreviations for the test files:

| Key | File |
| --- | --- |
| CPR | `claudine/cli/tests/l1/ctx_per_run.rs` |
| IC | `claudine/lib/src/invocation_context/tests.rs` |
| DME | `darkmatter/lib/tests/l1/request_context_epoch.rs` |
| DMG / DMC | Darkmatter units in `compose/context/capture/groups.rs` / `compose/context/current.rs` |
| HO | `claudine/cli/tests/l1/handoff_owners.rs` |
| CO | `claudine/lib/src/composition/coordinator/tests.rs` |
| UH | `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/unowned_handoff.rs` |
| PEP | `claudine/cli/tests/l1/preflight_execution_parity.rs` |
| PCS | `darkmatter/lib/tests/l1/preflight_child_state_parity.rs` |
| DG | `claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs` |
| LGA | `claudine/cli/tests/l1/loop_gate_ambient.rs` |
| PCT / GA | `claudine/lib/src/composition/looping/engine/tests/{post_checked_timing,gate_ambient}.rs` |
| SLE | `claudine/lib/tests/l1/single_loop_engine.rs` |
| SSV | `claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs` |
| LSV | `claudine/cli/tests/l1/lifecycle_set_shell_values.rs` |
| SRV | `darkmatter/lib/tests/l1/shell_result_values.rs` |
| DMLS | `darkmatter/dmls/tests/l1/shell_suffixes.rs` |
| TSF | `darkmatter/lib/tests/l1/transcluded_shell_failure.rs` |
| ATR | `claudine/cli/tests/l1/authored_text_rendering.rs` |
| DMLO | `darkmatter/cli/tests/l1/compose_literal_only.rs` |
| L2D | `claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs` |
| BTP | `biscuit-terminal/lib/src/components/prose/{markdown.rs,mod.rs}` units |
| PEG / DPEG | `claudine/cli/tests/l1/prose_escape_guard.rs` / `darkmatter/cli/tests/l1/prose_escape_guard.rs` |
| RPT | `claudine/lib/src/harness/report.rs` units |
| PGD | `claudine/cli/tests/l1/prompt_guide_defects.rs` |
| PRR | `claudine/cli/tests/l1/pr_flow_rehearsal.rs` |
| SPC | `claudine/cli/tests/l1/shipped_prompt_contract.rs` |

## Context evidence (R1, R5)

| Requirement | Test(s) |
| --- | --- |
| Review: a direct run keeps one stable observation | CPR `a_run_keeps_its_own_observation_after_it_stages`; IC `repeated_requests_in_one_run_observe_volatile_evidence_once` |
| Review / R1: proxy target, the source mentioned the property → `1` | CPR `a_proxy_target_observes_its_sources_staging_regardless_of_first_mention` (mentioned row) |
| Review / R1: proxy target, the source did not mention it → `1`; first-mention independence | same test (unmentioned row); IC `first_mention_does_not_decide_a_later_runs_value` |
| R1: stable `ctx.cwd`/`ctx.repo_root` identical across every hop | CPR proxy and branch rows; sequence, loop, retry, resume, serial, and parallel rows through `staged_with_stable_identity` **(new)** |
| Review / R1: later sequence step | CPR `a_later_sequence_step_observes_an_earlier_steps_staging` |
| Review / R1: later loop iteration | CPR `a_later_loop_iteration_observes_the_previous_iterations_staging` |
| Review / R1: retry, resume | CPR `a_retry_attempt_observes_the_previous_attempts_staging`, `a_resumed_attempt_observes_the_previous_attempts_staging` |
| Review: serial group task; acceptance: serial tasks observe earlier mutations | CPR `a_serial_group_member_observes_an_earlier_members_staging` |
| Review / acceptance: parallel siblings share the initial observation; a re-entering sibling sees its siblings' changes | CPR `parallel_siblings_share_one_capture_until_one_reenters`; IC `epochs_joining_one_run_share_its_observation` |
| Acceptance: parallel siblings keep their own provider identity | IC `siblings_sharing_one_run_keep_their_own_target_identity` **(new)**. Library level only: a group task cannot name its own provider or model (a sequence resolves them once per step, by design), so no process-level fixture can give two siblings different identities. |
| Acceptance / review: branch change | CPR `a_proxy_target_observes_a_branch_its_source_created`; IC `a_branch_change_between_runs_is_observed_by_the_later_run` |
| Review: working-tree change | CPR `a_proxy_target_observes_a_working_tree_edit_its_source_made` **(new)** |
| R1: one prepared-context construction per run; no repeated identity, topology, or host discovery; volatile work once per group per run | IC `each_run_constructs_one_context_and_rediscovers_no_stable_evidence` **(new)**, `each_run_observes_volatile_evidence_once_and_keeps_stable_evidence` |
| Acceptance: volatile observations counted apart from stable discovery | IC `repeated_requests_in_one_run_observe_volatile_evidence_once`, `runs_that_name_no_git_fact_observe_no_git_state`; CPR `a_fenced_example_in_a_partial_observes_no_git_state` |
| Sequence static pre-flight is one discovery run (found by the performance spike) | CPR `sequence_preflight_observes_git_state_once_for_every_prompt_document` **(new)** |
| Review: root-only `ctx` reference | CPR `a_run_keeps_its_own_observation_after_it_stages` |
| Review / R1: a transcluded partial shares its parent's observation | CPR `a_partial_shares_its_parents_observation` (now also asserts one `file_changes` observation for the run) **(strengthened)**; DME `siblings_and_nested_children_read_one_capture_of_a_child_introduced_group` |
| R5: the F5 table; Claudine output equals Darkmatter's, including two levels deep and an interpolated `::file` | CPR `a_partial_only_property_renders_as_darkmatter_renders_it` (direct, two levels, interpolated; F5 row 2 and a local-overlay row **(new)**) |
| Review: nested or interpolated `::file` discovery executes no side effects | CPR `discovering_an_include_executes_none_of_its_commands` **(new)** |
| Acceptance / review: body-only property captured before `initialize` | CPR `a_body_property_is_captured_before_initialize` |
| Acceptance / review: the include exception, captured after `initialize` | CPR `an_include_first_naming_a_group_captures_it_after_initialize` |
| Review: an include does not refresh a group the root already captured | CPR `an_include_reads_the_group_its_root_captured_before_initialize` **(new)**; IC `each_run_observes_volatile_evidence_once_and_keeps_stable_evidence` |
| Root names the property only in lifecycle frontmatter | CPR `a_lifecycle_only_property_is_captured_before_initialize` **(new)** |
| R1: `current` refreshes once per event | CPR `current_is_observed_once_per_event`; DMC `a_memoized_authority_agrees_across_scopes_until_re_memoized` |
| Acceptance, transclusion: local overlays | CPR `a_partial_only_property_renders_as_darkmatter_renders_it` (local overlay row) **(new)**; PCS "transclusion-local set" rows |
| Acceptance, transclusion: conditional includes | DME `a_group_named_only_by_an_unreachable_child_is_never_captured`; PCS conditional rows |
| Acceptance, transclusion: nested includes | DME `siblings_and_nested_children_…`; CPR two-level row |
| Acceptance, transclusion: cycles | CPR `a_transclusion_cycle_during_ctx_collection_is_the_cycle_error` **(new)** |
| Acceptance, transclusion: fenced examples | CPR `a_fenced_example_in_a_partial_observes_no_git_state`; DMG `document_body_code_blocks_demand_no_context`, `interpolated_code_blocks_demand_their_context` |
| Acceptance, transclusion: literal spans | DMG `literal_masks_ctx_key`, `ctx_key_outside_literal_still_triggers_group`; DME `a_graph_naming_no_discovery_backed_group_captures_none` |
| Acceptance, transclusion: requirements used only in included lifecycle frontmatter | CPR `an_included_files_lifecycle_is_scanned_but_not_run` **(new)**: the include's lifecycle never runs, its `ctx` group is still captured once, and parsing succeeds |

## Handoffs (R2, R8)

| Requirement | Test(s) |
| --- | --- |
| Review: loop source `initialize` proxy | earlier loop-adoption test (clean control in the review) |
| Review / R2: loop source `success`, `failure`, `finalize` proxy | HO `loop_success_proxy_hands_off_after_one_iteration`, `loop_failure_proxy_hands_off_after_one_iteration`, `loop_finalize_proxy_hands_off_without_rerunning_finalize`; also `loop_terminal_proxy_does_not_fire_the_abandoned_iterations_gate`, `loop_start_proxy_…`, `loop_proxy_after_a_retry_…` |
| R2: the chain has exactly two entries | CO `one_accepted_handoff_adds_exactly_one_entry`; HO loop rows (no false cycle) |
| Review / R8: sequence task `initialize`, `start`, terminal proxy | HO `sequence_task_{initialize,start,success,finalize,failure}_proxy_runs_target_within_the_step`, `sequence_task_proxy_after_a_retry_…` |
| Review: direct provider wrapper keeps refusing | UH `an_unowned_handoff_is_refused_with_a_typed_diagnostic` and siblings; L2 `level2_lifecycle_wrapper_passthrough_raises_no_proxy_handoff` |
| Review / acceptance: refused resolution leaves the chain unchanged | CO `a_refused_resolution_leaves_the_ledger_untouched`; HO `a_refused_resolution_leaves_no_entry_for_the_next_request` |
| Review: refused overlay evaluation | HO `a_refused_overlay_evaluation_leaves_no_entry_for_the_next_request` |
| Review / acceptance: a refused cycle | CO `a_refused_cycle_leaves_the_ledger_untouched_and_a_legitimate_hop_follows`; HO `a_refused_request_leaves_no_entry_for_the_next_one_to_collide_with`, `a_cycle_within_one_tasks_chain_is_refused` |
| Review: the hop limit | CO `a_refused_hop_limit_leaves_the_ledger_untouched`; HO `a_chain_past_the_hop_limit_is_refused_without_recording_the_refused_target` **(new)** |
| Review / acceptance: an adopted target that fails remains recorded | HO `compose_an_adopted_target_that_fails_remains_recorded`, `sequence_task_an_adopted_target_that_fails_remains_recorded` **(new)**; CO `an_adopted_target_that_later_fails_stays_recorded` **(strengthened)** |
| Review / acceptance: setup, teardown, and output publication once | HO `sequence_task_setup_teardown_and_output_run_once_across_a_handoff`, `…_across_a_terminal_handoff` **(new)**, `sequence_body_step_proxy_publishes_the_targets_output_once` |
| Acceptance: failure under `fail_fast: false` (teardown once, nothing published) | HO `sequence_task_failing_target_continues_under_fail_fast_false`, `…_halts_under_fail_fast_true` |
| R8: two-step sequence, target ran, step succeeded once, step 2 ran | HO `task_stacks_row` rows |

## Preflight parity (R3)

| Requirement | Test(s) |
| --- | --- |
| Review / R3: the five F3 rows (inline + overlay; partial + caller value; no interpolation; partial + overlay; direct + caller value), approval set = executed bytes | PEP `every_launched_command_is_approved_with_its_executed_bytes`; PCS `child_commands_are_approved_with_the_bytes_they_execute` |
| Review: nested or conditional partial, local `set.*`, untaken branch | PEP rows of the same table, `an_untaken_branch_is_discovered_without_executing`; PCS rows |
| Loop iteration re-audit | PEP `a_loop_iteration_audits_the_bytes_its_own_state_produces` |
| Review: `NotPreApproved` is not absorbed by `when_error` | PCS `a_when_error_block_does_not_absorb_a_pre_approval_violation` **(new)**; units `a_pre_approval_violation_in_a_child_is_fatal`, `a_pre_approval_violation_is_restored_not_absorbed` |
| Review: nor by lifecycle `no_error` | PEP `a_lifecycle_no_error_shell_does_not_absorb_an_unapproved_command` **(new)**, `an_unknowable_command_fails_preparation_despite_when_error_and_no_error` |

## Downgrade outcome (R4)

| Requirement | Test(s) |
| --- | --- |
| Review: `start` and `finalize` errors are preserved | DG `start_error_fails_with_one_diagnostic`, `finalize_error_fails_with_one_diagnostic` |
| Review / R4: `success` error, first attempt; the L1 row beside `level2_lifecycle_success_stack_error_downgrades_keeps_success_comm` | DG `success_error_on_first_attempt_fails_with_one_diagnostic` (exit 1, one `Error:` line, success `info` kept, event order `[success, failure, finalize]`) |
| Review / R4: after retry or resume, unrecovered and recovered | DG `success_error_after_exhausted_{retry,resume}_…`, `success_error_recovered_by_{retry,resume,proxy}_…` |
| Review / R4: sequence step under `fail_fast` true and false | DG `sequence_step_downgrade_halts_under_fail_fast_true`, `…_continues_and_fails_under_fail_fast_false` (one step-failure block, no extra `Error:` line **(strengthened)**, wrap-independent counting **(fixed for Linux)**) |
| Review / R4: loop iteration | DG `loop_iteration_downgrade_halts_under_fail_fast_true` **(strengthened)**, `loop_iteration_downgrade_reaches_gate_under_fail_fast_false` |
| Review: one diagnostic for terminal text and machine output | DG `success_error_reaches_err_msg_as_the_rendered_error` **(new)**. `-o json` only forwards the provider's own output flag, so the machine projection is the `err.*` diagnostic. |
| Inline compose | DG `inline_success_error_fails_with_one_diagnostic` (`cfg(unix)`) |

## Loop engine and gate (R7, R11)

| Requirement | Test(s) |
| --- | --- |
| Review / R7: the F6 table against `execute_loop_with_lifecycle`, count and body values | PCT `post_checked_condition_counts_iterations_and_body_state` |
| Review: prechecked engine removed; its three test modules ported | `iteration_actions`, `rate_limits`, and `seed_state` call `execute_loop_with_lifecycle` |
| R7: a workspace search finds no other `execute_loop`; `claudine::composition` exports one entry point | SLE `no_second_loop_engine_in_lib_or_cli_sources`, `composition_exports_exactly_one_loop_entry_point` **(new)** |
| Review / R11: the gate's `info`/`warn`/`message`/stack read the just-finished iteration's `_loop_*` | GA `loop_gate_concerns_read_just_finished_iteration_ambient_on_every_pass` |
| R11: the F8 document renders the iteration number on every pass | LGA `loop_gate_info_renders_loop_count_on_every_pass` |
| Review: the `lifecycle.md` example runs as written | LGA `lifecycle_reference_loop_example_runs_as_written` (`include_str!`) |

## Shell results (R9)

| Requirement | Test(s) |
| --- | --- |
| Review / R9: `::ok`, `::exit-code`, `::result` × exit 0, non-zero, timeout, allowed timeout | SRV `each_suffix_reads_each_outcome_as_a_typed_value` |
| Review / R9: `&&`/`||` chains, fallback, ternary to command, chain, or literal | SRV `a_result_suffix_applies_to_every_shape` |
| R9: two result suffixes fail to parse | SRV `two_result_suffixes_fail_to_parse_naming_both`; LSV `a_duplicate_result_suffix_fails_the_composition_naming_both` **(new)** |
| Review / R9: `::timeout` and `::no-cache` in both orders; `$(cmd)` + `$(cmd)::result` run once; concurrent reuse | SRV `a_result_suffix_combines_with_timeout_and_no_cache_in_either_order`, `one_command_runs_once_for_every_reader_in_a_document`; unit `concurrent_identical_requests_share_one_execution` |
| Review: top-level value judged by `$schema` after expansion; acceptance: post-expansion type errors | SRV `schema_validates_the_expanded_value` |
| Review: suffix grammar matrix through the public result | SRV `frontmatter_shell_value_reader_robustness_matrix`; SSV `set_shell_value_reader_robustness_matrix`; LSV `an_unknown_set_suffix_fails_the_composition_and_lists_the_valid_ones` **(new)** |
| DMLS completion, hover, diagnostic | DMLS `suffix_completion_hover_and_diagnostic_cover_all_five` |
| R9: a `start` `set` gates its next item; approval under the bare text; `initialize` still rejected; typed `::result` reaches a later `when:` | SSV `a_start_set_runs_its_command_and_gates_the_next_item`, `preflight_approves_the_bare_command_under_its_property`, `initialize_refuses_a_shell_assignment_in_any_item`; LSV `a_later_when_reads_the_typed_result_of_a_start_set`, `pre_flight_approves_the_bare_command_before_anything_runs` |
| Acceptance: failed mappings leave all destinations unchanged | SSV `a_failed_mapping_writes_no_destination` |
| Acceptance: false guards execute no assignment | SSV `a_false_guard_and_an_unresolved_value_run_nothing` |
| Acceptance: dry runs execute no assignment | LSV `a_dry_run_runs_no_lifecycle_assignment_command` with control `a_real_run_runs_the_lifecycle_assignment_command_once` **(new)**, `a_dry_run_executes_no_lifecycle_assignment` |
| Acceptance: successive events recapture a changed result | SSV `a_later_event_recaptures_a_changed_result` **(new)**, `each_executed_assignment_has_a_fresh_result_cache` |
| Acceptance: missing executable, denial, blacklist | SSV `missing_and_interrupted_commands_fail_the_set`, `approval_applies_shell_policy_to_a_set_command`; SRV `resolved_values::an_unapproved_command_is_refused` |
| Acceptance: cancellation | SSV `an_interruption_while_a_set_command_runs_fails_the_set` **(new)**, `missing_and_interrupted_commands_fail_the_set`; SRV `a_signal_is_never_a_value` |
| Acceptance: timeout policy | SSV `a_timed_out_command_fails_the_set`; SRV timeout rows |
| Acceptance: approved bytes unchanged after an intervening `set` | SSV `approved_bytes_do_not_follow_a_later_runtime_write`; SRV `command_bytes_are_fixed_when_resolved` |

## Rendering (R12, R13)

| Requirement | Test(s) |
| --- | --- |
| Review / R12: `md compose` literal-only, direct and included | DMLO `literal_only_file_converts_when_composed_directly`, `literal_only_file_converts_when_transcluded` |
| Review / R12: `claudine compose` literal-only, direct and included | ATR `literal_only_prompt_converts_when_composed_directly`, `literal_only_partial_converts_when_transcluded` |
| Review: real span beside a literal | ATR and DMLO `literal_beside_a_real_span_converts` |
| R12: the guide's artificial span is gone | PGD `the_guide_needs_no_artificial_span_for_its_literals_to_convert` **(new)** |
| Review / R13: the header description is exact after stripping styling | ATR `header_description_renders_exactly_as_authored`; BTP pre-processor and rendered-text cases |
| Acceptance: narrow widths, color disabled, not exact ANSI | ATR `header_description_keeps_every_character_at_a_narrow_width` (40 columns); every ATR row runs with `NO_COLOR=1` and strips styling |
| Other header rows (document label, model) | ATR `header_document_label_and_model_render_exactly_as_authored` **(new)** |
| Run header file name and source-file status line | ATR `run_header_and_source_status_keep_an_underscored_file_name` **(new)**; RPT `escaped_text_renders_exactly_as_written`, `a_linked_file_name_keeps_its_underscores` **(new)** |
| Review: the header's glyphs and styling in a real terminal, no focus change | L2D `level2_dry_run_description_renders_as_authored_in_tmux` |
| Review / R13: a diagnostic quotes `_loop_count` exactly | ATR `diagnostic_quotes_an_underscored_root_exactly` |
| Review: every author-text escaper is swept | PEG and DPEG source guards **(new)**. The guard found two escapers the earlier sweep missed, `harness/report.rs::prose_escape` and the `output/mod.rs` run header, and both were fixed. |

## Guide and PR flow (R10, R6)

| Requirement | Test(s) |
| --- | --- |
| Review / acceptance: resolved by directory identity; relocation | PGD `control_renders_every_warning_and_no_notice`, `a_relocated_spec_is_still_found`; SPC `shipped_prompts_name_specs_by_directory_identity` |
| Review / acceptance: completed status hides the section | PGD `a_completed_spec_hides_the_section` |
| Review / acceptance: missing or ambiguous lookup is reported | PGD `a_missing_or_ambiguous_spec_is_reported` |
| R10: `fixed: []` then one id listed; only that warning disappears; temporary copies | PGD `each_listed_id_hides_exactly_its_own_warning`, `every_id_listed_renders_no_warning` (frozen fixture `tests/fixtures/prompt_guide/spec.md`) |
| Review: F6 and D2 warnings exist | PGD `control_renders_every_warning_and_no_notice` |
| R10 robustness matrix | PGD `every_malformed_field_renders_a_notice_and_every_warning` plus the lookup rows |
| Review: stale staged-list text gone from `dirty.md`/`fix.md` handoffs | PRR `dirty_tree_with_commit_commits_then_pushes`, `blocked_push_with_fix_fixes_and_commits_without_asking` (captured prompts) **(strengthened)** |
| Review: `push.md` reads refreshed `ctx` | PRR `dirty_tree_with_commit_commits_then_pushes` (clean-tree text) **(strengthened)**, `a_path_the_commit_stage_leaves_dirty_is_listed_in_the_push_prompt` **(new)** |
| Review: `push.md` uses the shared `_facts.md` partial | PRR `a_fact_that_cannot_be_gathered_stops_the_push_stage` |
| Review: duplicate `warn` gone from `fix.md` | PRR `a_fix_that_never_verifies_fails_after_its_budget` **(strengthened)** |
| R6: clean tree; dirty `commit`, `abort`, no answer without a TTY; base branch; blocked push under `ask`, `fix`, `stop`; later-verifying fix; never-verifying fix | PRR `clean_tree_pushes_and_opens_the_pull_request`, `dirty_tree_with_commit_…`, `dirty_tree_with_abort_…`, `dirty_tree_without_an_answer_or_a_terminal_names_the_question`, `starting_on_the_base_branch_…`, `blocked_push_with_{ask,fix,stop}_…`, `a_fix_that_verifies_on_a_later_attempt_is_committed`, `a_fix_that_never_verifies_fails_after_its_budget` |
| R6: `shipped_prompt_contract` stays green | SPC (whole module), including `shipped_prompt_stacks_do_not_fall_through_into_an_unconditional_error` |

## Known Limits

- The interactive triage prompt reaches the stub in argv, which the `.cmd` wrapper does not forward. The triage route is asserted through the trail, but its prompt text is not captured.
- A Claudine interrupt alone cannot stop a running `set` command. In production the signal reaches the whole process group. SSV's cancellation row stands in for the signal with a stop file.
- The L2 header capture runs at 80 columns only. The narrow-width requirement is proven at L1.
