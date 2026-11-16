---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T05:05:40-08:00
spec: 2026-11-14-concise-message-clamp/spec.md
implemented: false
description: "A **fix** review of `2026-11-14-concise-message-clamp/spec.md`"
fix: 2026-11-14-concise-message-clamp/review-1.md
---

# Review 1: Concise Message Clamp

The fix is production ready. No findings or unresolved author decisions were identified. This is the first implementation review, so there are no previous findings to reconcile.

## Findings

None.

## Implementation decisions (informational)

Two parallel readers inspected the implementation log and the specification/plan. The following decisions and omitted work do not need a human ruling:

- **Remove duplicate message storage and helpers.** The implementer removed `DiagnosticSnapshot.full_message`, `LifecycleErrorInfo.full_msg`, `actionable_msg()`, `concise_and_full`, and `single_line_message`, which had appeared after the plan was written. I recommend keeping this choice: the spec rejects a second full-message store, and the normal message now retains the entire explanation. Inspection of the previous definition confirms `full_message` had `#[serde(skip)]`; its removal does not remove a serialized field or require a snapshot version change. The in-process API changes have corresponding caller updates.
- **Move mechanical Claudine edits into Phase 1.** Removing the duplicate APIs required caller and fixture updates immediately. Keeping the tree coherent is appropriate; phase placement does not change the delivered contract.
- **Pull failing cap assertions and comments into Phase 2.** Updating tests when the behavior changed, rather than waiting for Phase 3, preserves their purpose. The later phase completed the remaining coverage.
- **Check the mutation root's final folder component instead of its entire rendered path.** This still detects the old loss of the message ending without depending on Windows path spelling. Keep this inexpensive, portable assertion.
- **Place the caller-skill explanation after the catalog-command sentence.** This preserves the surrounding paragraph's flow. Ordinary interpolation braces inside the fenced YAML example survive publication; inspection of the installed caller skill confirms the example retains `{{ err.code }}`.
- **Keep historical limit references and unrelated previews.** Fixture descriptions about exceeding the old limit and historical timeline entries do not promise a current cap. The OpenCode bridge's inline preview and shell-test panic preview are outside the selected error-message contract and may remain bounded.
- **Run Phase 3 serially.** This is an internal execution choice with no effect on scope or behavior.
- **Leave substantive Claudine work until Phase 2 after Phase 1's mechanical updates.** This was a temporary phase handoff, not deferred deliverable work; later records and the current source show it completed.
- **Use recording emitters and isolated command fixtures instead of live speech, notifications, or real-terminal tests.** The spec expressly requires no live-resource verification for this change. The message contract is observable before a destination renders or speaks it, so Level 1 is appropriate.
- **Leave Messenger unchanged and avoid a new provider matrix.** The spec preserves existing adapter policies and requires Messenger checks only if adapter code changes. No adapter change is needed here.
- **Leave cross-OS execution evidence to CI after the Windows attempt was blocked by storage preflight.** The log records no successful Windows run and does not present one as passing. The plain-name filter workaround for the runner's filter-quoting problem was appropriate. Neither runner repair nor storage-guard override is needed to deliver this fix.
- **Leave unrelated edits and Windows-only unused-import warnings outside this implementation.** They do not demonstrate a wrong message result or missing requirement of this fix. No required feature is recorded as deferred.

The first, second, and fourth decisions above were explicitly flagged in `decisions_for_review`. I recommend retaining all three; none warrants interrupting the author. No verified false behavior statement in the implementation record required correction.

## Contract and verification

The review followed the selected message through Darkmatter's shared hygiene functions and lifecycle constructors, Claudine's typed/code/cause snapshot constructors and restored diagnostics, provider-failure selection, task-ledger composition, both changed OpenCode error producers, and the terminal/data/lifecycle consumers affected by removal of duplicate storage.

| Requirement | Evidence and appropriate level |
| --- | --- |
| One hygiene implementation, no length budget | Level 1: envelope tests compare 239/240/241-character inputs and a long Unicode value; cover ANSI, OSC hyperlinks, LF/CRLF, tabs, stray controls, Unicode separators, empty input, preserved interior spaces, idempotence, and complete normalized suffixes. The implementation uses the existing terminal escape stripper and linear segment folding. |
| Complete snapshots, one-level causes, and `err.msg` | Level 1: `every_projection_route_keeps_the_complete_normalized_message` checks exact text through typed selection, cause projection, code/label constructors, facet-less actions, catalog misses, and JSON round trips. Reviewed constructors use the shared hygiene or mirror the already projected snapshot. |
| Provider precedence and retry suffixes | Level 1: the runtime tests retain explicit multiline endings, normalize escape-only errors before fallback, preserve long guard/stderr explanations, and exercise precedence and attempt suffixes. Stderr retains its existing last-useful-line selection. |
| Complete displaced failure and task list | Level 1: ledger tests retain long prior failures and every task in order. The spawned Claudine replay test checks the lifecycle message, serialized task facts, and terminal diagnostic. Classification and exit-code behavior remain separately represented. |
| OpenCode producer caps removed | Level 1: long multibyte headline and non-JSON error-tag tests preserve the complete value. Existing stack/cause boundary selection remains unchanged, as required. The JSON summarizer retains its selected provider-message extraction. |
| Normal command paths | Level 1, actual isolated CLI processes: Darkmatter tests retain a long terminal diagnostic and finalize message; its JSON test asserts the exact folded message, one envelope, and empty stderr. Claudine tests observe a long initialize error in finalize and all incomplete tasks in failure output and machine records. |
| Delivery isolation and full speech text | Level 1: recording-emitter tests verify a long speech action arrives whole; interpolated `err.msg` reaches speech and a subsequent message unchanged, with the original error equal to its pre-delivery copy. Existing destination fitting and failure handling are unchanged. |
| Current guidance and caller publication | Source/document audit found no remaining shared error caps or current 240-character promises in the affected contracts. The published caller skill states the complete-message contract and retains the short, deliberately authored announcement example. |

No requirement needs terminal-emulator rendering or keyboard encoding evidence: the spec changes message retention and explicitly excludes live delivery tests. The input robustness matrix does not apply. No configuration loader or field validator changed; the OpenCode edits remove output caps while retaining existing parsing and selection.

## Review runs

Focused runs used the package-area `just test` recipes, which execute nextest:

- Darkmatter `just test message`: 43 passed; none at or above five seconds.
- Claudine `just test message`: 217 passed; none at or above five seconds.
- Darkmatter `just test lifecycle::envelope:: a_long_composed_error a_lifecycle_run_writes_only`: 9 passed; longest test 0.387 seconds.
- Claudine `just test harness::runtime::tests:: stream::task_ledger::tests:: error_headline non_json_error_tag a_long_err_msg executor_publishes_a_long an_initialize_error the_headline_and_the_machine_record`: 60 passed; longest test 0.481 seconds.
- `just check-tier-coverage darkmatter claudine`: passed, no stranded tests.
- `git diff --check`: passed.

The filter runs overlap; these counts are invocations, not a distinct-test total. A combined expression-filter invocation failed in the recipe's shell forwarding before tests ran; ordinary positional name filters successfully selected the intended tests instead.

The implementation log records full affected-area L1 runs and lint checks for the completed phases, with no tests at or above five seconds. This review reran the focused behavior checks rather than repeating those full suites. Both CLI suites declare their L1 targets explicitly and include the changed modules in their target's `main.rs`; the observed nextest runs confirm the new regressions are compiled and selected.

Cross-OS evidence remains CI's responsibility and does not block this review. No code changes or commits were made during review.
