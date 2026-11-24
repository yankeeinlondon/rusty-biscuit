---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Group-framing tests fail under a supported non-UTF-8 locale"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-21T08:19:16+00:00"
spec: 2026-11-20-loop-reread-context-drift/spec.md
implemented: true
implemented_by: claude/opus
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/fixes/2026-11-20-loop-reread-context-drift/log.md
description: "A **fix** review of `2026-11-20-loop-reread-context-drift/spec.md`"
fix: 2026-11-20-loop-reread-context-drift/review-1.md
next: 2026-11-20-loop-reread-context-drift/review-2.md
---

# Review 1: loop reread context drift

The loop fix implements the specified read boundaries and context handoff.
Readiness is blocked by one existing test defect exposed in the implementation
record and reproduced during review: four sequence-framing tests fail when
the effective locale selects ASCII output. This needs a small fixture repair,
not a change to production rendering. No author decision is required.

Reviewed the changes in commits `1f0ac4363` and `82af77a43`, the specification,
plan, implementation log, affected callers, current documentation, and test
registration. This is the first review; there are no previous findings.

## Findings

### R1-F1 (high): Group-framing tests fail under a supported non-UTF-8 locale

**Authority: defect.** In the Claudine library's
[group-framing tests](../../lib/src/composition/sequence/task/tests.rs#L5250),
four assertions require `▶` while their terminal fixture allows the host
locale to select `>`. A developer or unattended build using the standard `C`
locale therefore gets failed tests despite correct production output. The
review contract explicitly includes defects noticed in unchanged code; being
pre-existing does not exempt this one.

**Class:** terminal-output tests depend on ambient glyph capabilities while
asserting a fixed glyph.

Reproduction, from `claudine/`, without changing any fixture:

```sh
LANG=C LC_ALL=C LC_CTYPE=C just test-library group_framing --max-fail=10
```

Result: 12 selected, 8 passed, 4 failed. For example, the task-stream test
observes `│ > alpha` and counts zero headers because it searches for
`▶ alpha`. The geometry test cannot find its arrow; the channel test
misclassifies the ASCII header as leaked payload; the body-attribution test
cannot find the matching header. All failures are below 0.2 seconds.

The control `LANG=C just test-library group_framing` passed all 12 in this
session because inherited `LC_ALL=C.UTF-8` takes precedence. The complete
UTF-8 run also exercises these tests successfully. Explicitly setting every
locale variable distinguishes the failing input from that control.

The entire 12-test module was run under both configurations. Its sibling
sites and results are:

| Child ID | Test in `composition::sequence::task::tests::group_framing` | Shape tested | Observed in C locale | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | `every_member_task_opens_and_closes_exactly_one_stream` | ASCII headers | fails: zero headers | one header and footer per task |
| R1-F1.2 | `a_cap_of_one_preserves_the_group_bar_at_the_same_left_edge` | ASCII headers, serial and parallel | fails: no arrow found | equal content columns |
| R1-F1.3 | `a_members_body_output_lands_on_the_data_channel_not_the_status_one` | ASCII status headers | fails: header classified as payload | headers on status, body on data |
| R1-F1.4 | `every_body_line_carries_its_own_tasks_bar` | forced color, ASCII header | fails: matching header absent | body and header attribution match |
| R1-F1.5 | `a_parallel_group_gives_each_task_its_own_palette_entry` | forced color, ASCII header | passes | three distinct task prefixes |
| R1-F1.6 | `a_no_color_terminal_still_attributes_every_task_by_name` | no color, ASCII header | passes | task names preserved |
| R1-F1.7 | `a_failed_task_reports_its_own_outcome_in_its_footer` | failed task | passes | each footer reports its task's outcome |
| R1-F1.8 | `concurrent_siblings_never_split_one_frame_group` | concurrent writes | passes | frame groups remain intact |
| R1-F1.9 | `framing_never_reaches_the_captured_payload` | framed rendering and captured output | passes | captured payload excludes framing |
| R1-F1.10 | `a_capped_members_body_keeps_its_group_bar_geometry` | serial group body | passes | attribution gutter preserved |
| R1-F1.11 | `partial_output_from_a_failing_task_is_still_framed` | failing task with partial output | passes | partial body retains framing |
| R1-F1.12 | `a_silent_run_renders_no_frames_at_all` | silent output | passes | no frames |

**Recommended repair:** explicitly select Unicode support on terminals used
by assertions that require Unicode headers. The biscuit-terminal library's
`Terminal::builder().supports_unicode(true)` provides this without mutating
the process environment (see [biscuit-terminal's builder](../../../biscuit-terminal/lib/src/terminal.rs#L912)). Preserve the forced color capabilities needed by
the body-prefix test; `Terminal::new_forced()` alone does not force Unicode.
Alternatively, assert using the selected header glyph. Keep production's
ASCII fallback. Rerun this module under explicit C and UTF-8 locales, then
the area's normal L1 gate. No new rendering abstraction or large test matrix
is needed.

## Spec coverage and verification level

The changed behavior concerns preparation, filesystem reads, context reuse,
and provider attempt routing. Level 1 process and in-process tests are the
appropriate evidence: this fix introduces no keyboard, terminal-encoder,
scrolling, or terminal-style requirement needing Level 2 or Level 3.

| User-facing requirement | Strongest relevant evidence | Assessment |
| --- | --- | --- |
| Keep the startup root through later passes, including post-initialize edits, caller inputs, and proxy inputs | L1 CLI: `loop_startup_source::staged_proxy_loop_retains_its_first_validated_root` | verifies two launches, one initialization, stable body/actions, fresh working tree, stabilized condition values, and new-run visibility |
| Retain exactly the read the harness judged | L1 CLI: `delegated_proxy_loop_retains_the_harness_read` | distinguishes original, stabilized, and provider-rewritten roots; editor runs once |
| No later startup reread or launch verdict left pending | L1 public preparation API: `loop_iteration_skips_the_launch_verdict_without_deferring_it`; delegated CLI case; root-deletion CLI case | validating control refuses missing required input; skipped verdict applies defaults and retains schema without deferral |
| Lifecycle actions and context agree after successful rereads | L1 CLI staged-single and staged-loop cases; actual-guard harness tests on stabilize-only and adopted routes | checks added context group, retained earlier observation, and one observation each for file changes and Git through work counters |
| Failure events use the selected lifecycle/context pair | L1 CLI denied-audit case; actual-guard shell-audit and plan-failure cases on both harness routes | checks previous pair before installation, replacement pair afterward, no provider launch, and locked early-handler shell |
| Post-pass condition uses that pass's preparation | L1 staged proxy loop with `required_passes` supplied only by stabilized source | ends at two launches instead of the cap |
| Retry/resume read fresh without replacing retained source | L1 new retry CLI case; existing resume context/lifecycle cases; shared recovery installation and source-publication code | retry returns to original source on next ordinary pass; resume uses the same fresh-read path and cannot publish retention after attempt one |
| Sequence task entry remains a fresh boundary | L1 `sequence_task_reopens_root_between_invocations` and existing sequence suites | separate task invocations see root edits; each task's loop keeps its own source |
| Inline change detection and rollback use the live pass baseline | L1 public preparation API and CLI unchanged-second-pass regression, with existing two-edit positive case | prior deliverable is preserved; earlier edit does not satisfy later pass |
| Current documentation explains root edits, runtime values, and explicit reads | L1 executable looping-guide examples, plus source/doc comparison | examples demonstrate `set`, explicit file reads, retained root, and next-run visibility |

New tests are registered in the consolidated L1 target, their fixture feature
is enabled by the canonical recipe, and their names do not route them into an
unselected tier. Repository documentation reads in the example tests use
`include_str!` inside the test. The input robustness matrix does not apply:
the change selects preparation sources and carries results; it does not add
or change a configuration loader, field validator, or file-format grammar.
The defaulted empty approval set is fail-closed and is not a parser silently
discarding invalid input.

Review runs:

- `LANG=C.UTF-8 just test-cli loop_startup_source overlay_layering loop_cli ctx_per_run lifecycle_traces loop_retry_passes sequence_referenced_state sequence_frozen_graph inline_prompt_shell_preflight ctx_launch_anchor`: 297 passed, no test at or above five seconds.
- `LANG=C.UTF-8 just test-library --features work-counters composition::prepare::service`: 24 passed, none at or above five seconds.
- Locale controls and failing reproduction: 12/12 passed with inherited UTF-8 override; 8/12 passed with every locale variable set to C, as detailed in R1-F1.
- `LANG=C.UTF-8 LC_ALL=C.UTF-8 LC_CTYPE=C.UTF-8 just test`: 12,676 passed, 15 skipped, none at or above five seconds; nextest finished in 132.295 seconds, recipe exit 0.
- `git diff --check`: clean.

The implementer's recorded final UTF-8 run selected 12,676 tests and reported
none at or above five seconds. Its lint and tier-coverage checks also passed.
Cross-OS execution remains CI evidence and is not a readiness finding.

## Implementation decisions (informational)

Two agents independently read the implementation log and spec in parallel
before the implementation comparison. Their combined inventory is below.
None needs an author ruling or a later human-review interruption.

- **Failing Phase 1 checkpoint:** keep the nine honest regressions visible
  before repairing them in Phase 2. This follows the explicit baseline
  checkpoint and adds no final behavior or release decision.
- **UTF-8 verification and deferred locale repair:** using UTF-8 proves the
  loop change but does not resolve the four known test failures. R1-F1 gives
  the small recommended repair; no choice needs to be reserved for the author.
- **Retry versus resume coverage:** keep the focused new retry process test
  and existing resume tests. Both use shared fresh-read and installation code;
  retention publication is first-attempt-only. Separate new tests for both
  verbs were not a spec requirement.
- **Harness context installation:** keep the prepared context rather than
  mechanically duplicating the command pipeline's context-extension and
  environment-overlay steps. Canonical preparation already captures authored
  requirements and applies the identity selector's environment. The tested
  guard and body agree. No realistic failing input was established for the
  omitted extra extension, so it is not a speculative repair requirement.
- **Retained-source publication:** allow successful first-attempt adopted
  boot as well as stabilize-only boot to publish the source. The owning loop
  consumes it only for delegated startup; retry/resume never replace it.
  Restricting unused publication further is a cheap internal choice.
- **Pass-preparation publication timing:** publish after installation and
  before plan parsing. A plan error exits the attempt before the loop gate
  consumes the value, preserving current behavior.
- **Inline sentinel example:** keep the explicit live-file read rather than
  changing the gate to live deliverable frontmatter. This follows the spec's
  retained-source rule and its recommendation to use explicit reads. Positive
  and negative tests verify the corrected example.
- **Expected hostname:** compare the event's nonempty hostname with the
  composed prompt's hostname rather than independently observing the host.
  This directly tests shared preparation; counters verify observation reuse.
- **Early-failure fixture:** use the timeout-plan error to reach the early
  catch, placing its marker before the forbidden shell. The fixture proves
  failure routing and shell refusal without a real provider or editor.
- **Reused provider identity:** use the existing target-launch fixture rather
  than adding provider-dispatch sites solely for a test.
- **Unforced target-rebuild failure:** no inexpensive reproduction was found.
  Successful adopted rebuild and later plan failure are exercised; source
  inspection confirms installation occurs before rebuild failure routing.
  A synthetic rebuild failure hook would add more machinery than this review
  needs.
- **Documentation timing and caller publication:** completing skill updates
  in Phase 3 and publishing to a review directory under `target/` are internal
  workflow choices. The record says publication succeeded and inspected
  examples retained their literal expressions.
- **Cross-OS execution:** leave environment-specific evidence to CI as the
  review contract permits; this is not deferred implementation scope.

## Implementation-record correction

Corrected only the implementation log's claim that `LANG=C` alone always
reproduces the locale failures. A dated 2026-11-21 note records both direct
runs and explains `LC_ALL`/`LC_CTYPE` precedence. Historical run results were
preserved. This factual correction is separate from the test defect in R1-F1
and does not request human review.
