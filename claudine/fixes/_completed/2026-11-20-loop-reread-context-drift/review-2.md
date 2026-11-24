---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-21T08:31:32+00:00"
spec: 2026-11-20-loop-reread-context-drift/spec.md
implemented: false
description: "A **fix** review of `2026-11-20-loop-reread-context-drift/spec.md`"
fix: 2026-11-20-loop-reread-context-drift/review-2.md
previous: 2026-11-20-loop-reread-context-drift/review-1.md
---

# Review 2: loop reread context drift

The fix is production ready. The previous finding is verified fixed across
all required instances. No new findings or decisions requiring a human were
identified.

Read the specification, previous review, repair's Finding Ledger, and original
implementation record before checking the code. Reviewed the original changes
in `1f0ac4363` and `82af77a43`, the current preparation and execution paths,
regression assertions, documentation, and the uncommitted repair in the
Claudine library's sequence task tests. The separate biscuit-terminal research
document edit is outside this review.

## Previous Finding Verification

The previous review has one finding and no human rulings. Its twelve child
instances and the repair ledger's additional class-sweep row are checked
below. “C and UTF-8” means separate runs with `LANG`, `LC_ALL`, and `LC_CTYPE`
all explicitly set to the indicated locale, rather than relying on inheritance.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Group-framing tests fail under a supported non-UTF-8 locale | R1-F1.1–R1-F1.12 and repair sweep R1-F1.S | verified fixed | Fixture changes inspected; every child passes under C and UTF-8. Production rendering is unchanged. | Nothing |
| R1-F1.1: One stream per member | `every_member_task_opens_and_closes_exactly_one_stream` | verified fixed | `run` selects Unicode explicitly; header/footer counts pass in C and UTF-8. | Nothing |
| R1-F1.2: Group bar geometry | `a_cap_of_one_preserves_the_group_bar_at_the_same_left_edge` | verified fixed | Both serial and parallel fixtures use the pinned terminal; column comparison passes in both locales. | Nothing |
| R1-F1.3: Output channels | `a_members_body_output_lands_on_the_data_channel_not_the_status_one` | verified fixed | Header fixture pins Unicode; status/data assertions pass in both locales. | Nothing |
| R1-F1.4: Body attribution | `every_body_line_carries_its_own_tasks_bar` | verified fixed | Forced-color terminal also pins Unicode; body/header prefix comparison passes in both locales. | Nothing |
| R1-F1.5: Palette attribution | `a_parallel_group_gives_each_task_its_own_palette_entry` | verified fixed | Forced color is preserved; three distinct prefixes pass in both locales. | Nothing |
| R1-F1.6: Colorless attribution | `a_no_color_terminal_still_attributes_every_task_by_name` | verified fixed | Explicit no-color and Unicode capabilities; names and absence of escapes pass in both locales. | Nothing |
| R1-F1.7: Failure footer | `a_failed_task_reports_its_own_outcome_in_its_footer` | verified fixed | Name/outcome assertions pass in both locales; no fixed header glyph is required. | Nothing |
| R1-F1.8: Concurrent frame integrity | `concurrent_siblings_never_split_one_frame_group` | verified fixed | Per-write ownership assertion passes in both locales; independent of header glyph. | Nothing |
| R1-F1.9: Captured payload | `framing_never_reaches_the_captured_payload` | verified fixed | Unframed payload assertion passes in both locales. | Nothing |
| R1-F1.10: Capped body geometry | `a_capped_members_body_keeps_its_group_bar_geometry` | verified fixed | Gutter assertions pass in both locales using the pinned shared fixture. | Nothing |
| R1-F1.11: Partial failure output | `partial_output_from_a_failing_task_is_still_framed` | verified fixed | Partial body assertion passes in both locales; independent of header glyph. | Nothing |
| R1-F1.12: Silent run | `a_silent_run_renders_no_frames_at_all` | verified fixed | Captured output and absence of framing pass in both locales. | Nothing |
| R1-F1.S: Sibling capability dependencies | Other Claudine assertions relying on Unicode headers | verified fixed | Full area L1 run under explicit C locale; inspected renderer fixtures, which already pin Unicode, and `a_limited_glyph_terminal_uses_the_ascii_header_marker`, which explicitly disables it. | Nothing |

The repair removes the ambient capability dependency without weakening the
four failing assertions. The two color-sensitive sites retain
`Terminal::new_forced()` capabilities while setting `supports_unicode` to
true. The colorless case still explicitly selects `ColorDepth::None`.
The existing renderer test continues to require the ASCII header when Unicode
support is false. No production output or public API changed in this repair.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification level

These requirements concern preparation state, filesystem boundaries, and
provider routing. Level 1 in-process and hermetic CLI tests exercise the
required behavior. The fix adds no keyboard, emulator rendering, or scrolling
requirement needing Level 2 or Level 3 evidence. The locale repair checks
fixture capability selection and emitted frames; it introduces no new
terminal-display contract.

| Requirement | Verification and code checked | Assessment |
| --- | --- | --- |
| Retain the first validated source, including edits from initialization and caller/proxy inputs | `loop_startup_source::staged_proxy_loop_retains_its_first_validated_root`; command loop retains the stabilized source and approvals | L1: two launches, one initialization, retained instructions/actions, fresh second-pass working tree, and new-run visibility |
| Keep the exact source the harness validated | `delegated_proxy_loop_retains_the_harness_read`; harness returns `RetainedLoopSource` through the attempt result | L1: distinguishes pre-read, validated, and provider-rewritten roots; no post-launch reload |
| Later ordinary passes owe no launch verdict | `loop_iteration_skips_the_launch_verdict_without_deferring_it`; `SchemaStage::SkipLaunchVerdict`; deleted-root CLI regression | L1: validating control fails, skipped preparation applies defaults and retains completion schema without a pending verdict |
| Install matching lifecycle actions, context, and document-run accounting | Staged-single CLI test and both `stabilized_harness_events_use_the_active_epoch` routes; shared installation helper | L1: actual guard output matches the prompt; work counters require one observation each for Git and file changes |
| Failure events use the pair selected by their failure boundary | Denied-audit CLI case; harness shell-audit and plan-failure cases on direct/adopted routes | L1: prior pair before installation, replacement pair after installation, and early-handler shell refusal |
| Post-pass evaluation uses the pass's preparation | Stabilized proxy test's `required_passes`; harness publishes `LoopPassPreparation`, command returns it to the loop engine | L1: stabilized field ends the loop after two passes rather than at the cap |
| Retry/resume read fresh without replacing the loop's retained source | New retry CLI test; existing retry/resume context and lifecycle tests; shared recovery installation and first-attempt-only retention publication | L1: recovery instructions/actions are fresh; next ordinary pass returns to startup instructions |
| Sequence task entry stays a fresh boundary | `sequence_task_reopens_root_between_invocations`; task runner enters shared command preparation | L1: four prompts show each task retains its own source and the next task sees the edit |
| Inline reconciliation uses the current pass's live baseline | `a_later_inline_pass_baselines_on_the_live_deliverable`; unchanged-second-pass CLI regression; existing two-edit positive case | L1: live bytes/hash and unreadable-file error checked; earlier deliverable survives a later unchanged-body verdict |
| Documentation describes the implemented boundaries | Looping, composition, pipeline, lifecycle, README, and caller-skill changes compared with execution paths; executable guide examples | L1 examples demonstrate retained root, carried `set`, explicit live read, and fresh-run visibility |

The new CLI test module is declared by `cli/tests/l1/main.rs` under
`test-fixtures`, which the area's canonical test recipe enables. Library
preparation and harness guard tests are declared unit modules; the canonical
area run enables work counters. No changed test has a tier prefix that
excludes it from L1. Guide reads use `include_str!` inside individual tests.

The input robustness matrix does not apply: this work changes which prepared
document is used and how its context is carried, without adding or changing
a configuration loader, field validator, or file-format grammar.

## Verification

Commands ran from `claudine/` unless stated otherwise:

- `LANG=C LC_ALL=C LC_CTYPE=C just test-library group_framing`: 12 passed; none at or above five seconds.
- `LANG=C.UTF-8 LC_ALL=C.UTF-8 LC_CTYPE=C.UTF-8 just test-library group_framing`: 12 passed; none at or above five seconds.
- `LANG=C LC_ALL=C LC_CTYPE=C just test`: 12,676 passed, 15 skipped; nextest finished in 132.156 seconds; none at or above five seconds; recipe exit 0. This run includes the preparation, actual-guard, loop, lifecycle, retry, resume, proxy, sequence, and inline reconciliation cases above. Output: `/tmp/loop-reread-review-2-l1.log`.
- `git diff --check` from the repository root: clean.

The repair record also reports full C and UTF-8 area runs with 12,676 passes,
15 skips, and no slow tests, plus successful lint and tier checks. Those
recorded results were assessed separately from the direct runs above.
Cross-OS evidence remains with CI, as permitted by this review's scope.

## Recurrence

None. R1-F1 is fully repaired; no unchanged instance, repair regression, or
missed sibling remains.

## Implementation record

No newly verified false statement requires an in-place correction. The
original implementation's decision paragraphs describe its historical state;
the repair log documents the subsequent locale fix. The previous review's
dated correction about locale-variable precedence remains accurate.
No accepted decision was reopened and no human review interruption is needed.
