---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Changed review-screen behavior lacks terminal and keyboard verification"
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-09-29T13:22:22-07:00
spec: 2026-09-27-sequence-review-screen/spec.md
implemented: true
next: 2026-09-27-sequence-review-screen/review-2.md
description: A **fix** review of `2026-09-27-sequence-review-screen/spec.md`
fix: 2026-09-27-sequence-review-screen/review-1.md
---

# Sequence review screen: review 1

**Not production ready under this review's test requirements.** The implementation matches the layout and filtering contract in the cases inspected and tested. One verification class remains incomplete: the changed screen is tested through buffers and callbacks, without exercising its rendered behavior in a terminal or its key promises through OS keyboard events. No additional runtime defect was established.

No human design decision is needed. The missing coverage can be implemented as automated tests; this review did not run tests that take desktop focus.

## Finding: high — Changed review-screen behavior lacks terminal and keyboard verification

**Defect class:** Tests stop below the boundary needed to establish the changed user-visible terminal behavior, so passing results cannot establish that the shipped screen renders correctly and applies or cancels choices when driven by a user's keyboard.

In **claudine-cli**, [review_sequence](../../cli/src/commands/wrap/selection_ui.rs:73) constructs the table, runs the terminal event loop, and decodes submitted rows. Its new filtering and merging tests instead call [live_targets](../../cli/src/commands/wrap/sequence/review.rs:129) with callbacks that return already-created targets or already-created cancellation errors. These prove the mapping and error handling, but they do not traverse the input reader, table edits, submission, decoding, and execution boundary together. A cancellation test would still pass if the actual review screen stopped producing the cancellation error.

In **biscuit-tui**, [InputTable](../../../biscuit-tui/lib/src/components/input_table/table.rs:384) renders the changed layout into a headless buffer. That establishes the allocation algorithm and buffer contents, but does not prove terminal-emulator glyph widths, clipping, resize redraws, or scrolling. The spec explicitly accepts buffer-style evidence for the underline diagnosis; that exception is satisfied and is not a separate missing-test finding.

### Instance sweep

I searched the production `InputTable` consumers and both areas' declared terminal and keyboard test suites. The two shipped consumers are Claudine's sequence review and `question input-table`. I checked the requirements and sibling paths below against the same test inventory, including clean cases. “Missing” means missing verification, not an observed failure of the running UI.

| Site / requirement | Shape checked | Observed result / strongest relevant evidence | Expected verification |
| --- | --- | --- | --- |
| biscuit-tui static labels | `12 review-5`, off-screen longer row, narrow and restored widths | Clean L1 buffer assertions; no real-terminal table capture | L2 capture of normal and narrow layouts, scrolling, and resize restoration |
| biscuit-tui Unicode clipping | Wide Japanese characters, combining accent, joined emoji, allocations of zero and one | Clean L1 buffer assertions; no real-terminal table capture | L2 capture proving the displayed label and following column remain within bounds |
| Claudine review submission | Interleaved prompt, shell, side-effect, task, group, and body steps; repeated names; hidden first/last steps; distinct provider/model choices | Clean L1 callback merge tests and separate row-construction/decoder tests; no terminal submission through this filtered screen | L2 end-to-end submission and observed choices at fake execution endpoints; keyboard evidence appropriate to the Ctrl+S promise |
| Claudine review cancellation | Esc and Ctrl+C before sequence work starts | L1 callbacks manufacture `ABORTED_KIND` / `CANCELLED_KIND`; no review-screen key event or process-level no-work assertion | Exercise the actual screen, assert exit 130 and no provider, hidden shell, or side-effect work; L3 evidence for the literal OS-keyboard promises |
| `question input-table` submission, sibling consumer | Text input plus switch; manufactured Ctrl+S bytes | Existing `level2_input_table_submits_json_output_via_pty` uses an expect-driven pseudo-terminal, so it is L1 under the review rubric despite its name; it also has no static column | Real-terminal coverage of the changed static layout; this existing test does not supply it |
| Default focused choice and text cells | Blank-cell modifiers, visible focus, active-option styling, focus moved in the same buffer | Clean L1 style assertions explain and remove the blanket underline | Satisfied under the spec's explicit headless evidence allowance |
| Empty eligible set and deterministic bypass | All-hidden steps, explicit provider, auto-selected provider, headless prompting state, dry-run | Clean L1 callback/gate tests and shipped-CLI headless/dry-run tests | L1 is appropriate for these decisions; no separate finding |
| Existing sequence interrupt coverage | Ctrl+C while parallel tasks are already running | L3 tests exist, but launch with an explicit provider and bypass review | Does not cover cancellation before review submission |

The existing `question` L2 terminal tests exercise choice widgets, not `InputTable`. Claudine's L2 sequence capture exercises task output, not the provider/model review screen. Neither closes these gaps. These are all instances of one verification-boundary class, not separate findings per label or key.

### Required change

Add a declared, selected terminal test using the repository's harness and a hermetic sequence fixture with interleaved eligible and hidden steps. Capture original numbering and full labels, resize to force clipping, scroll, and restore the width. Edit providers/models, submit through the actual event loop, and assert the choices reaching fake execution endpoints while hidden steps retain their baseline targets. Include the shared model editor's catalog-choice and free-text forms.

Exercise Esc and Ctrl+C on that screen and assert exit 130 with no execution markers. Add OS-keyboard coverage for the key promises required by this review's rubric; manufactured key events and callback errors are not that evidence. Keep real-terminal tests from taking desktop focus, and keep keyboard tests in their dedicated gated tier. Declare any new files in the consolidated test binaries and run the relevant tier recipes. One fixture can cover multiple requirements without duplicating expensive launches.

## Contract and implementation assessment

The implementation sizes static columns from schema text and all current rows, measures display cells, saturates long widths, protects editable budgets, and falls back to an even allocation when necessary. Clipping retains whole graphemes and does not mutate submitted values. Multiple static columns, all-static tables, and the `u16` boundary have passing L1 tests.

Eligibility uses only the normalized outer executable. Prompt, task, group, and body steps remain eligible; shell and side-effect steps retain baseline entries. Choices merge by original index, not name or filtered position. Short and long submitted results are rejected before merging. The empty-table exception preserves the earlier provider gate. The documented cancellation change to exit 130 is recorded in the implementation log and current guides.

The sequence guide, execution-flow guide, InputTable guide, and touched symbol comments describe the implemented behavior. No additional abstraction or performance change is warranted by the inspected code. The input robustness matrix does not apply: this fix changes presentation and mapping of already-normalized steps, rather than a file-format or configuration reader. The existing row decoder was not changed.

## Validation

| Check run during review | Result |
| --- | --- |
| biscuit-tui `NEXTEST_TEST_THREADS=6 just test` | 1,012 passed; 7 skipped |
| biscuit-tui `just lint` | Passed |
| claudine `just lint` | Passed |
| `just check-tier-coverage biscuit-tui` and `just check-tier-coverage claudine` | Passed; no stranded tests |
| claudine `NEXTEST_TEST_THREADS=6 just test` | Stopped after 5,304 passed and one failed; 2,746 tests were not run; 9 skipped |
| claudine `just test-cli sequence::review::tests::` | 17 passed |
| claudine `just test-cli selection_ui::tests::` | 16 passed |
| claudine `just test-cli wrap_sequence_composition::` | 17 passed |

The full Claudine run failed [a_stalled_terminal_notify_is_reported_as_unknown_after_the_drain_budget](../../cli/tests/l1/lifecycle_message_drain.rs:622), an unchanged test of desktop-notification shutdown outside this fix. It passed in a separate diagnostic invocation in 10.9 seconds. That does not erase the failed full run or establish its cause; it is recorded as an unrelated validation limitation, not a blocking finding against the review-screen implementation.

The new unit modules are compiled through their parent modules, and the new CLI test is declared in the L1 binary. No L2/L3 test was added by this fix. No cross-OS evidence is required for this readiness decision. No source changes, formatting, commits, or lifecycle-directory moves were made during review.
