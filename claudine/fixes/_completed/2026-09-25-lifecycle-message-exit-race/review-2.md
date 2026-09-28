---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-25-lifecycle-message-exit-race/spec.md`
fix: 2026-09-25-lifecycle-message-exit-race/review-2.md
spec: 2026-09-25-lifecycle-message-exit-race/spec.md
previous: 2026-09-25-lifecycle-message-exit-race/review-1.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T23:58:23-07:00
implemented: true
next: 2026-09-25-lifecycle-message-exit-race/review-3.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-25-lifecycle-message-exit-race/implementation-log.md
ready: false
human_review: true
has_blocked_findings: true
blocked: false
recurrence: true
findings:
    - "[high] Ctrl+C during the drain still lacks OS-keyboard verification"
    - "[medium] The notification test stall is enabled in production builds"
human_review_items:
    - |-
        The review requires a real operating-system Ctrl+C keypress during the message drain, but these tests must not bring a terminal window into focus. The new Kitty tests send a remote command to the terminal; they do not inject an operating-system keypress. Please choose how to resolve this conflict:

        - Permit a focused terminal in an isolated test desktop, while ensuring the test never takes focus from the host user's desktop. Then add a real keyboard-injection test for the drain.
        - Accept terminal-encoded Ctrl+C as the required evidence for this specific drain behavior. Then classify the Kitty test as Level 2 and update the review's verification requirement accordingly.

        The existing command-level and terminal-level tests should remain in either case.
---

# Review 2: Lifecycle Message Exit Race

## Verdict

**Not ready for production.** The first review's desktop-notification timeout gap and direct-exit guard gap are addressed. The interrupt tests now cover every interactive command through a terminal, but their claimed Level 3 tests use Kitty remote control rather than operating-system keyboard events. A new test-only notification stall also changes the behavior of ordinary production builds when its environment variable is set.

This change does not add or change a file-format or configuration reader, so the input robustness matrix does not apply.

## Prior review disposition

| Review 1 finding | Result in this iteration |
| --- | --- |
| Stalled desktop notification has no CLI-level exit test | Addressed: `a_stalled_terminal_notify_is_reported_as_unknown_after_the_drain_budget` runs a real CLI child, waits for the 10-second cap, checks the safe label and unchanged exit code. It passed locally. The seam used by that test has a separate production-build problem below. |
| Ctrl+C during the delivery drain lacks real-keyboard verification | Still open: the new Kitty command exercises Kitty's key encoder, but no operating-system keyboard event reaches the terminal. See the blocked finding and recurrence section. |
| Direct-exit guard allows calls to move within an allowlisted file | Addressed: the guard now matches file, function, branch position, and call form. Its replacement-site tests and a source mutation described in the implementation log show that moving the ordinary exit before the drain fails. The focused guard tests passed locally. |

Review 1 had no blocked findings to reassess.

## Unblocked Findings

### [medium] The notification test stall is enabled in production builds

**Defect class:** A test-only environment override is compiled into the shipped notification path and can suppress a real notification.

The `claudine` library's [send_desktop_notification](../../lib/src/messaging/send.rs) checks `CLAUDINE_TEST_DESKTOP_NOTIFICATION=stall` before it calls the desktop backend, then waits forever. The `claudine-cli` shutdown cap turns that into a 10-second delay and an uncertain-delivery warning. The branch has no test-only feature or build guard. This conflicts with the spec's instruction not to add a production override solely to make the timeout test run. It also means a caller that forwards this environment variable can silently lose every desktop notification.

| Delivery site or control | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Terminal `notify` through `execute_notification` | Test CLI child sets `CLAUDINE_TEST_DESKTOP_NOTIFICATION=stall` | Notification never reaches the backend; CLI waits about 10 seconds, then reports unknown delivery | Keep this process-level timeout test, with its stall mechanism absent from production builds |
| Terminal `notify` through `execute_notification` | No stall variable; backend unavailable | Existing CLI test reports the backend failure and exits `0`; clean | Keep the normal behavior and test |
| Embedding application calling the public `execute_notification` helper | Same stall variable | The shared `send_desktop_notification` branch suppresses this notification too | Production library notification must not honor a test-only stall |
| Outbound `message` helpers | Same variable | No stall branch; clean | Keep message sends unaffected |

The first row is reproduced by the new CLI test, which passed in 10.359 seconds. The source has no `#[cfg(test)]` or feature gate around the branch, and `claudine` has no feature that restricts it to test builds. Put the stall behind a test-only feature enabled only by the relevant test target, or inject a silent backend in a test-only build. Preserve the end-to-end CLI assertion.

## Blocked Findings

### [high] Ctrl+C during the drain still lacks OS-keyboard verification

**Defect class:** A keyboard-dependent exit promise is verified with programmatic terminal input, while the required operating-system keyboard layer remains untested.

The new [Kitty drain tests](../../cli/tests/level3/level3_drain_ctrl_c.rs) call `kitty @ send-key ctrl+c`. Kitty encodes the input and the pane receives it, which is useful real-terminal evidence. It does not inject an operating-system key event. The test file itself documents that distinction. The [tmux drain tests](../../cli/tests/level2/level2_drain_ctrl_c_tmux.rs) also inject through a terminal command, and the Windows ConPTY test writes the Ctrl+C byte into a console. Under the review's stated levels, these verify at most Level 2 input behavior, whatever tier selects their files. The four Kitty tests are named `level3_` and selected by `test-l3`, so naming and gating currently overstate the evidence.

The common [drain interrupt fixture](../../cli/tests/common/drain_interrupt.rs) runs the same withheld-webhook scenario for all four interactive commands. Its command-by-command sweep is:

| Command site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` | Two Ctrl+C presses while its exit drain waits | Level 1 process signal, Level 2 tmux, and Kitty remote-key test; no OS key event | An OS keyboard event reaches the terminal during the drain and the second press exits `130` promptly |
| `inline-compose` | Same withheld reply and retained compose interrupt guard | Same three inputs; no OS key event | Same OS keyboard proof |
| `sequence` | Same withheld reply and shutdown-installed interrupt guard | Same three inputs; no OS key event | Same OS keyboard proof |
| Provider wrapper | Same withheld reply from a hook message and shutdown-installed interrupt guard | Same three inputs; no OS key event | Same OS keyboard proof |
| `handle` | Noninteractive hook subprocess with a pending message | Deadline and process-exit tests; clean because there is no terminal keypress contract | No keyboard test needed |

The four command cases appear in the Unix Kitty and tmux files and in the Windows console file. All use the common withheld-response fixture, so the missing layer is the same in every row. Existing OS-keyboard tests elsewhere in Claudine exercise interruption during a running command, not during this exit drain.

The implementation log reports that macOS did not deliver a key event posted to an unfocused Kitty window, while the review also forbids bringing a terminal window into focus. This makes the specified Level 3 proof blocked by conflicting test constraints. The human review item above asks for a choice that resolves the conflict. Until then, treat the Kitty coverage as terminal-level evidence and do not claim the drain has Level 3 verification.

## Recurrence

This is the same defect class as Review 1's **“Ctrl+C during the delivery drain lacks real-keyboard verification.”** That fix needed to sweep `compose`, `inline-compose`, `sequence`, and the provider wrapper using a real operating-system keypress during their drains. It swept all four commands with the shared fixture, but substituted terminal remote commands for the required input level. The table above includes every sibling site and the clean noninteractive `handle` path.

## Verification performed

- `cargo nextest run -p claudine-cli --features test-fixtures --test l1` with a focused filter ran the stalled-notification CLI test and all 10 exit-site guard tests: **11 passed**.
- `cargo nextest run -p claudine-cli --features terminal-tests --test level2` with a focused filter ran the four tmux drain cases: **4 passed** without focusing a window.
- `just check-tier-coverage claudine`: **0 stranded tests**. The new Level 2 and Level 3 files are declared by their consolidated targets and selected by live recipes; their tier placement does not establish OS-keyboard coverage.
- Source review covered the shared drain fixture, all new interrupt input paths, the notification helper and stall seam, the exit-site guard, and the test target declarations. No terminal or browser window was focused during this review.
