---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-09-29T14:16:08-07:00
spec: 2026-09-27-sequence-review-screen/spec.md
implemented: false
description: A **fix** review of `2026-09-27-sequence-review-screen/spec.md`
fix: 2026-09-27-sequence-review-screen/review-2.md
previous: 2026-09-27-sequence-review-screen/review-1.md
---

# Sequence review screen: review 2

**Production ready for the specified scope.** The first review's verification gap is addressed. The changed layout and row mapping have appropriate tests, and submitted model choices now survive launch preparation. No remaining blocking defect or additional human decision was established.

## Previous review findings

[Review 1](review-1.md) contains one unblocked finding, “high: Changed review-screen behavior lacks terminal and keyboard verification,” although it does not use an `Unblocked Findings` heading. It has no blocked findings, so none needed an intervening decision.

The implementation adds actual terminal capture for both shipped `InputTable` consumers, submission through the real review event loop, process-level cancellation assertions, and OS keyboard tests on a private Linux display. Importantly, the submission tests assert providers and models recorded by fake executables, rather than only the selections painted on screen. They also assert that hidden shell and side-effect steps still execute after submission and remain untouched after cancellation.

## Requirement and sibling-site sweep

The earlier finding's class was **verification below the boundary needed to establish user-visible terminal behavior**. I checked every sibling identified in that review, including the unaffected picker, against the new tests. The real-terminal cases use static fixture documents and fake providers; no live provider or lifecycle audio is needed.

| Site / requirement | Shape tested | Observed result | Required level and assessment |
| --- | --- | --- | --- |
| Claudine sequence labels and filtering | Interleaved prompt, shell, side-effect, task, group, and body steps; repeated names; hidden first and last steps | Real pane displays only original positions 2, 4, 5, and 6 with full labels | L2 present; passed during this review |
| Claudine clipping, scrolling, and resize | 120-column pane, 60-column short pane, last-row navigation, restored width | Ellipsis appears beside intact provider cells; scrolling preserves column widths; restoring width restores the full long label | L2 present; passed during this review |
| Claudine catalog-model submission | Distinct provider/model choices on eligible rows, followed by Ctrl+S | Fake providers receive the expected choices for prompt, task, group, and body; hidden steps execute in order | L2 through the real event loop present; passed during this review; L3 submission test also exists |
| Claudine free-text model submission | Empty catalog, typed model on one row, different provider on another | Typed model reaches the corresponding executable and other rows retain their choices | L2 present; passed during this review |
| Claudine cancellation | Esc and Ctrl+C on the open review screen | Both exit 130; no provider launch, hidden-step output, or audio publication | L2 present; both passed during this review; separate L3 tests cover the OS keys |
| `question input-table`, sibling consumer | Two static columns, off-screen longest label, Japanese text, combining accents, joined emoji; narrow and restored layouts | Tags remain intact beside clipped labels; scrolling preserves widths; submission returns all original labels | L2 present; passed during this review |
| Shared InputTable allocation | Preferred widths, protected editable budgets, multiple static columns, all-static table, emergency allocation, zero/one-cell widths, long strings | L1 assertions cover allocations and rendering bounds, including saturation and grapheme clipping | L1 is appropriate for exhaustive allocation cases; representative terminal behavior is covered at L2 |
| Shared focus painting | Focused choice and text cells, blank padding, focus moved in the same buffer | Default blank cells are not underlined; focus remains visible and previous styling clears | L1 satisfies the spec's explicit style-inspection allowance |
| Sequence mapping and opening gates | Repeated names, hidden endpoints, all-hidden steps, wrong submitted count, explicit provider, headless prompting, dry-run | Index-based merge retains hidden targets and reasons; empty table is bypassed; count errors and existing gates remain enforced | L1 present; focused review and decoder suites passed |
| Compose / inline-compose provider picker | Existing single-provider picker | Uses `ChooseOne`, rather than the changed table; existing terminal coverage remains applicable | Clean sibling; no new requirement from this fix |

The L3 tests in [level3_sequence_review_screen_keys.rs](../../cli/tests/level3/level3_sequence_review_screen_keys.rs) press Ctrl+S, Escape, and Ctrl+C through XTEST on an isolated Xvfb display. Kitty encodes those OS events, and the shipped CLI handles the resulting input. This is actual L3 coverage, unlike injected tmux bytes. I inspected the test and harness paths; [the implementation log](log.md) records three passing Linux-container tests. I did not independently rerun L3 during this review. The macOS focus-taking L3 recipe was not invoked. Cross-OS execution evidence is left to CI and is not a readiness condition here.

## Launch-model correction

The new terminal assertions exposed a model choice being dropped after row submission. In **claudine-cli**, [planned_fallback_model](../../cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs) carries the sequence's selected model into the per-attempt launch rebuild. The rebuild uses it when the launched document names no model and keeps the planned provider. An explicit CLI model and the launched document's own model retain their existing precedence; changing the provider discards the previous provider's planned model.

I checked the sibling routes: prompt and body steps, task and group launches, direct composition, inline composition, and provider passthrough. Sequence routes carry their planned target. Direct and inline composition do not retain a model originating in the launched document's frontmatter, allowing a refreshed document to remove it. Passthrough has no separate planned-model source. The two shipped-CLI model tests verify inherited versus document-authored models and explicit CLI precedence; the launch-rebuild suite checks provider changes and reuse of the recorded launch plan. The terminal fixture independently verifies reviewed models through prompt, task, group, and body execution.

The sequence guide and execution-flow/composition guides describe this fallback. The sizing guide and touched symbol comments match the implemented layout and eligibility contract. No additional abstraction or performance change is warranted for this fix.

## Findings

None blocking production readiness. The implementation log also records an existing scroll-offset behavior: after increasing pane height, rows above the retained scroll position can remain hidden. This is outside the changed width-allocation behavior, and navigating back still reveals those rows; the tests accurately assert width restoration rather than claiming that resizing resets the scroll position.

## Recurrence

No recurring finding. The only earlier review's verification class is now covered across its sibling sites, including both shipped table consumers and the keyboard submission/cancellation paths.

## Input robustness matrix

Not applicable. This fix consumes already-normalized sequence steps and typed table values. The follow-up model correction carries an already-resolved target into launch preparation. Neither change adds or modifies a file-format parser, configuration loader, or deserializer. The fixture's JSON/YAML readers are existing application paths, not new parsers introduced by this fix.

## Validation

| Check during this review | Result |
| --- | --- |
| Claudine `just test-l2 sequence_review_screen` | 4 passed |
| biscuit-tui `just test-l2 input_table_layout` | 1 passed |
| Claudine launch-rebuild L1 suite | 36 passed |
| Claudine `sequence_planned_model` shipped-CLI L1 suite | 2 passed |
| Claudine sequence-review L1 suite | 17 passed |
| Claudine selection-UI L1 suite | 16 passed |
| Claudine `NEXTEST_TEST_THREADS=6 just test` | 8,059 passed; 9 skipped |
| biscuit-tui `NEXTEST_TEST_THREADS=6 just test` | 1,012 passed; 7 skipped |
| Claudine and biscuit-tui `just lint` | Passed |
| Root `just check-tier-coverage claudine` and `just check-tier-coverage biscuit-tui` | Passed; zero stranded tests |
| `git diff --check` | Passed |

The new files are declared in the consolidated test binaries, and their features are enabled by the tier recipes and CI metadata. Linux-only keyboard tests are selected by the live L3 recipe; Unix terminal fixtures and shell stubs are guarded accordingly. Level-2 tests ran without taking desktop focus.

This review changes only the review documents and requested spec metadata. It makes no source edits, runs no formatter, creates no commits, and moves no lifecycle directory.
