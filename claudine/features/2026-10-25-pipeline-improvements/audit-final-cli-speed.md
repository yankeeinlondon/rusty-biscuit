---
kind: implementation-log
---

# Final gate: CLI and generator test speed

The full Claudine L1 gate found 15 additional over-budget identities in this
slice. The changes retain their real CLI, persistence, provider-stub, and
watchdog boundaries; no production behavior, tier, or runner configuration
changed. The separate `audit-final-clean-speed.md` covers one additional
Darkmatter identity, making **16 original slow identities** for this agent's
final-gate work.

## Original identities and changes

| Module / original identity | Original duration | Change |
| --- | ---: | --- |
| `agent_text_is_data::inline_agent_added_frontmatter_survives_a_second_run` | 5.165 s | Explicit metadata-only schema fixture; retain both real inline runs and exact readback. |
| `agent_text_is_data::inline_agent_values_are_stored_as_data_and_read_back_exactly` | 5.974 s | Same fixture opt-in; retain literal-token bytes, authored-template evaluation, completion schema, stamped hash, and second-run idempotence. |
| `agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text` | 8.347 s | Split four malformed input classes and the repaired control into independently scheduled tests; guard the structural-opener roster. |
| `authored_text_rendering::header_document_label_and_model_render_exactly_as_authored` | 5.112 s | Split the two-width/two-label Cartesian matrix; guard every cell exactly once. |
| `inline_completion_lifecycle::a_conditional_required_property_never_blocks_an_inline_launch` | 6.366 s | Metadata-only fixture opt-in; retain authored required-expression schema, real agent call count, surviving artifact, and supplied-value control. |
| `inline_completion_lifecycle::a_proxy_with_overlay_satisfies_the_targets_completion_schema` | 6.744 s | Schedule the standalone negative control separately, sharing the exact target text; retain full lifecycle schemas for the actual proxy. |
| `inline_completion_lifecycle::sequence_task_with_satisfies_the_referenced_documents_completion_schema` | 7.290 s | Schedule the standalone negative control separately, sharing exact member text; retain full sequence/lifecycle schemas for the actual sequence. |
| `provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface` | 8.269 s | Split four launch commands; each retains dry-run, debug/notice, `AGENT_PARAMS`, and exact child argv assertions. Guard launch surfaces. |
| `provider_tail_launch::other_failures_are_not_attributed_to_the_tail` | 6.987 s | Split all six failure classes, preserving timeout/interrupt semantics and assertions. Guard shared roster and uniqueness. |
| `provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known` | 9.137 s | Split both switch states across four commands; retain notice wording, excluded claims, exit status, and exact forwarded argv. Guard all eight cells. |
| `contamination_probes::an_uncontaminated_run_matches_the_expectations_every_probe_asserts` | 6.010 s | Schedule its three independent baseline observations separately. The default command fixture remains unchanged for every contamination probe. |
| `generate_ux::roster_alias_rename_is_regenerated_until_check_converges` | 6.215 s | Keep real `generate --yes`, then check the changed name-table artifact with the same public checker instead of another full ten-provider CLI pass. |
| `generate_ux::steering_activation_policy_grant_is_written_and_checks_clean` | 5.924 s | Build expected steering bytes once, prove drift, run real generation, then compare the written bytes exactly. Avoid rebuilding identical expected bytes after writing. |
| `generate_ux::yes_writes_all_drift_and_check_reconverges` | 5.849 s | Keep real `generate --yes`; perform subsequent checks through the public functions the CLI reports. All ten providers and every aggregate artifact, plus roster validation, remain checked. CLI clean-report/exit wiring has its existing separate test. |
| `wrap_watchdog_startup_stall::watchdog_startup_bytes_move_the_deadline` | 5.335 s | Retain the actual two-second silence budget and real provider stderr. Chatter lasts 2.8 seconds on Unix or three seconds on Windows; a new completion marker proves the child survived the entire interval. |

## Fixture scope and coverage

`CliProcessFixture::seed_inline_metadata_schema(schema)` is an explicit opt-in
that writes the supplied shipped inline schema into the fixture's private
`home/schemas/claudine.yaml`. The real shipped `inline-compose.yaml` supplies
the same prompt, hash, and date types, while avoiding expansion of unrelated
recursive lifecycle types for data-only tests. Each call site carries its own
`include_str!` reference so CI can retain its test-input dependency. The
fixture's default behavior is unchanged. Lifecycle, proxy, and sequence tests
retain the complete shipped schemas. Other agents may opt their own data-only
fixtures into this helper and count their changes separately.

The watchdog test keeps its configured deadline, watchdog cadence, escalation
grace, actual subprocess, output stream, and exit-reason assertions. It now
also requires the provider's final marker, so an unrelated early exit cannot
pass merely by omitting a timeout message. The Windows script uses its
existing console-safe `ping` delay; the OS skill was read before that change.

## Counts

For this CLI/generator report: **15 original slow identities modified**,
**36 gross new identities**, **five old identities replaced**, **net +31**.
The net increase comprises **five new coverage guards** and **26 extra
scheduling identities**. One existing watchdog test is strengthened. The
generator and metadata-fixture optimizations add no test identities.

Including the separate clean-parity change: **16 original slow identities**,
**51 gross new identities**, **six replaced**, **net +45**, comprising **six
new coverage guards** and **39 extra scheduling identities**.

## Verification

The coordinator owns the combined parallel L1 run, preventing concurrent
builds or tests from competing with these timing measurements. Final results
and per-module maximum durations are pending that run. This agent ran no
build, test, or lint during the coordinator's full-area gate or while the
other final-speed edits were pending. `git diff --check` passed.

Original timing evidence: `/tmp/pipeline-final-claudine-l1.log`.
