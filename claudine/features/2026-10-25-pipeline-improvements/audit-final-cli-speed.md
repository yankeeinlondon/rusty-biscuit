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

The coordinator's combined parallel L1 run passed **1,269 / 1,269 tests**,
with zero tests over five seconds. This agent's nine modules account for
**161 passing tests**, including the 15 Darkmatter clean-parity tests covered
by the separate report. The maximum in this slice is **3.608 seconds**.

| Module | Tests passed | Maximum duration |
| --- | ---: | ---: |
| `agent_text_is_data` | 33 | 2.963 s |
| `authored_text_rendering` | 13 | 0.917 s |
| `inline_completion_lifecycle` | 23 | 2.735 s |
| `provider_tail_launch` | 27 | 2.195 s |
| `provider_tail_notice` | 19 | 1.995 s |
| `contamination_probes` | 11 | 2.483 s |
| `generate_ux` | 15 | 1.438 s |
| `wrap_watchdog_startup_stall` | 5 | 3.608 s |
| `clean_parity` (separate report) | 15 | 0.969 s |

The original inline persistence identities now take **1.071 / 1.162 s**;
the malformed-input replacements take at most **0.555 s**; the header matrix
replacements at most **0.917 s**. The conditional required-property identity
takes **1.131 s**. The proxy positive/control pair takes **1.540 / 2.054 s**;
the sequence positive/control pair takes **1.944 / 2.055 s**. Credential
surface replacements take at most **2.195 s**, other-failure replacements
**1.854 s**, and unknown-switch replacements **1.248 s**. The three baseline
contamination observations take at most **1.313 s**. The original roster,
steering-grant, and all-drift generator identities take **0.948 / 1.250 /
1.438 s**, respectively. The watchdog identity takes **3.608 s**, including
its retained real two-second budget and completion-marker assertion.

These are host timings from the final scoped parallel invocation; the
original timings came from the full-area gate, which ran more tests. No
cross-OS timing result is claimed. This agent ran no independent build,
test, or lint while the coordinator owned the final gates.
`git diff --check` passed after the edits.

Original timing evidence: `/tmp/pipeline-final-claudine-l1.log`.
Final evidence: `/tmp/pipeline-final-speed-verified.log`, Nextest run
`67830caf-931f-47fc-80d7-cc8c45165925`. The combined execution took **81.071
seconds**, separate from compilation.

To reproduce this agent's slice from the repository root:

```sh
BISCUIT_TEST_FILTER='test(clean_parity::) | test(agent_text_is_data::) | test(authored_text_rendering::) | test(inline_completion_lifecycle::) | test(provider_tail_launch::) | test(provider_tail_notice::) | test(contamination_probes::) | test(generate_ux::) | test(wrap_watchdog_startup_stall::)' just test darkmatter-cli claudine-cli claudine-gen
```

This reproduction command was not an additional run; the passing evidence
comes from the coordinator's broader combined invocation.
