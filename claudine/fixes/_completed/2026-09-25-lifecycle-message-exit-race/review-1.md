---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-25-lifecycle-message-exit-race/spec.md`
fix: 2026-09-25-lifecycle-message-exit-race/review-1.md
spec: 2026-09-25-lifecycle-message-exit-race/spec.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T21:39:00-07:00
implemented: true
next: 2026-09-25-lifecycle-message-exit-race/review-2.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-25-lifecycle-message-exit-race/implementation-log.md
ready: false
human_review: false
recurrence: false
findings:
    - "[high] A stalled desktop notification has no CLI-level exit test"
    - "[high] Ctrl+C during the delivery drain lacks real-keyboard verification"
    - "[medium] The direct-exit guard allows calls to move within an allowlisted file"
---

# Review 1: Lifecycle Message Exit Race

## Verdict

**Not ready for production.** The main message path works: a terminal-event message reaches a withheld webhook before `compose` exits, a hook message is drained, failures are reported, and the exit code is preserved. Two user-visible promises lack the required tests, and the direct-exit guard does not enforce exact sites. These findings need no design decision or human review.

The change does not add or modify a file-format or configuration reader, so the input robustness matrix does not apply.

## Findings

### [high] A stalled desktop notification has no CLI-level exit test

**Defect class:** A user-visible delivery-timeout promise is asserted only on an injected internal task, without exercising the notification helper and the CLI shutdown path together.

The `claudine` library's [execute_notification](../../lib/src/messaging/send.rs) starts the task, and the `claudine-cli` binary's [finish](../../cli/src/shutdown.rs) must wait, print the safe `desktop notification` label when time expires, and retain the command's exit code. The current CLI test makes the desktop backend fail immediately, so it cannot detect a missing registration, a premature exit, or a warning that omits the label when a real notification stalls. The library's paused-time test registers an arbitrary future directly with the tracker; it does not call `execute_notification` or run the CLI. This is a Level 1 gap for the spec's terminal `notify` requirement.

| Delivery site | Shape checked | Observed verification | Expected verification |
| --- | --- | --- | --- |
| `compose` terminal `message` | Webhook never replies | Level 1 CLI test waits about 10 seconds and checks the warning, redaction, and exit code; clean | Same |
| `handle` hook `message` | Webhook never replies | Level 1 CLI test checks the remaining handler deadline and exit code; clean | Same |
| `compose` terminal `notify` | Backend fails immediately | Level 1 CLI test checks the existing failure warning; clean for failure, silent about timeout | Keep this test |
| `compose` terminal `notify` | Backend remains pending | Only a library test of a synthetic tracked future checks the label; no CLI test | Level 1 CLI test with a silent, stalled backend checks the 10-second bound, `desktop notification` warning, and unchanged exit code |

The reproduction is the existing `a_terminal_notify_is_drained_and_its_failure_reported_without_host_ui` CLI case with the backend changed from immediate failure to a stalled test backend. With the present fixture it exits in about 0.3 seconds and prints only the failure warning, so the timeout branch is never reached. Add a test-owned backend seam that never opens a notification window, then assert the public process result. On Windows the current missing application ID causes an immediate failure, so that host needs a test seam capable of holding the send open without showing a toast.

### [high] Ctrl+C during the delivery drain lacks real-keyboard verification

**Defect class:** A keypress-dependent shutdown promise is tested by sending control events directly to Claudine, without proving that a real terminal encodes the user's Ctrl+C press and reaches the same path.

The new [drain interrupt tests](../../cli/tests/l1/lifecycle_message_drain_interrupt.rs) send `SIGINT` on Unix or a console control event on Windows. Those are useful Level 1 tests of Claudine's handler, but they bypass the terminal's keyboard encoder. The Level 2 and Level 3 Ctrl+C tests elsewhere in the package exercise ordinary lifecycle interruption, not a press while a delivery is draining. The spec expressly promises that a second Ctrl+C during the drain forces exit immediately. Under the review's test rigor rule, a keypress promise needs Level 3 verification.

| Command path | Shape checked | Strongest drain-specific verification | Expected verification |
| --- | --- | --- | --- |
| `compose` | Two presses while a webhook reply is withheld | Level 1 direct control events pass; clean for handler logic | Level 3 OS keyboard presses in a real terminal, including immediate forced exit |
| `inline-compose` | Same retained compose guard | No drain-specific child test; shares the implementation above | Include in the Level 3 drain check or prove the shared entry path in a focused lower-level test |
| `sequence` | Two presses while a webhook reply is withheld | Level 1 direct control events pass; clean for handler logic | Level 3 OS keyboard presses in a real terminal |
| Provider wrapper | A terminal message is still pending at exit | No drain-specific press test | Level 3 keyboard check for the shutdown-installed interrupt ladder |
| `handle` | A hook message is still pending at exit | Deadline tests cover the wait; this is a noninteractive hook subprocess | No keyboard test applies to this path |

The existing Level 1 reproduction creates a local webhook listener that never replies, waits for the command's final output, then sends two control events. It passes, but the terminal never receives a keyboard event. Extend that same fixture through the repository's Level 3 terminal harness and inject actual OS keypresses into the spawned terminal without bringing a window to the foreground. Keep the Level 1 cases because they isolate the handler logic. The Level 3 test should be environment gated and selected by the live `test-l3` recipe.

### [medium] The direct-exit guard allows calls to move within an allowlisted file

**Defect class:** A source guard that promises an exact set of escape sites verifies file-level counts instead of the position and purpose of each exit call.

The `claudine-cli` [exit-site guard](../../cli/tests/l1/exit_site_guard.rs) records each direct exit's line and form, but `violations` compares only each file's call count. A future call added to an allowed file can replace an existing call and pass the guard even if it exits before `finish` drains deliveries. This misses the spec's exact-allowlist requirement. The planted-site tests check an extra exit and an unlisted file, but never a replacement within an allowed file.

| Site class | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| `shutdown.rs` ordinary exits | Same number of calls, one moved before the drain | Guard accepts the file because its count is still two | Guard rejects any ordinary exit before the drain |
| `main.rs` audio-worker exit | Same number of calls, one moved into ordinary dispatch | Guard accepts the file because its count is still one | Guard accepts only the audio-worker branch |
| `compose/interrupt.rs` forced exits | Same number of calls, one moved outside a forced-interrupt branch | Guard accepts the file because its count is still five | Guard accepts only forced-interrupt branches |
| Any unlisted CLI source file | Add one direct exit | Existing Level 1 detector test rejects it; clean | Keep rejecting it |

Use the guard's existing synthetic `census` fixture to replace an allowed site's line or enclosing function while keeping its count. It currently returns no violations; the expected result is a violation. Pin the approved sites by a stable structural marker, such as enclosing function plus call form, rather than by line number, which changes when unrelated lines are inserted.

## Verification performed

- `just check-tier-coverage claudine`: passed; no stranded tests.
- Four focused `claudine-cli` Level 1 tests passed: terminal message delivery, terminal notification failure, hook message delivery, and the direct-exit guard.
- Source review covered the three send helpers, the delivery tracker, ordinary shutdown, `handle` deadlines, exit-site guard, and the registered Level 1, Level 2, and Level 3 test targets.
- The implementation log records passing full local tests and cross-OS runs; this review does not treat those results as a production-readiness requirement.
- I found no additional implementation change justified by ergonomics or performance in this bounded shutdown path.
