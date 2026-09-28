---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-25-lifecycle-message-exit-race/spec.md`
fix: 2026-09-25-lifecycle-message-exit-race/review-3.md
spec: 2026-09-25-lifecycle-message-exit-race/spec.md
previous: 2026-09-25-lifecycle-message-exit-race/review-2.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T00:45:40-07:00
implemented: true
next: 2026-09-25-lifecycle-message-exit-race/review-4.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-25-lifecycle-message-exit-race/implementation-log.md
ready: false
human_review: true
has_blocked_findings: true
blocked: false
recurrence: true
findings:
    - "[medium] The test-seam guard accepts production-enabled feature predicates"
    - "[high] Ctrl+C during the drain still lacks operating-system keyboard verification"
human_review_items:
    - |-
        A second Ctrl+C during Claudine's message-delivery wait must force an immediate exit. The current tests prove this with process signals and commands sent to a terminal, but none sends an operating-system keyboard event to that terminal. The fix specification also forbids a test from focusing a terminal window; on macOS, an operating-system key event sent to an unfocused Kitty window did not reach it. Please choose one resolution:

        - APPROVED: Permit a focused terminal inside an isolated test desktop that never takes focus from the host user's desktop. Add keyboard-injection tests for the four interactive commands during the delivery wait.
        - Accept terminal-encoded Ctrl+C as sufficient evidence for this specific wait. Reclassify the Kitty remote-key tests as Level 2 and document that the operating-system keyboard layer remains untested here.

        The existing process-level and terminal-level tests remain useful under either choice.
---

# Review 3: Lifecycle Message Exit Race

## Verdict

**Not ready for production.** The previous review's unblocked notification finding is fixed: the stall seam is absent from a default build, and the end-to-end timeout test still passes. The keyboard question remains blocked on the human choice above. I also found a gap in the new test-seam guard: it can approve a gate that includes a normal production build.

The change does not add or modify a file-format or configuration reader, so the input robustness matrix does not apply.

## Prior review disposition

| Review 2 finding | Result in this iteration |
| --- | --- |
| Notification test stall is enabled in production builds | Fixed. The notification seam and its branch in the `claudine` library are both gated on `test-fixtures`; the `claudine-cli` feature enables that library feature for fixture builds. The stalled-notification process test passed with the feature enabled. The new guard has a separate flaw below. |
| Ctrl+C during the drain lacks operating-system keyboard verification | Still blocked. No human choice is recorded, and the implementation log expressly deferred it. The four Kitty tests still use `kitty @ send-key` and are still labeled Level 3. |

Review 2's blocked finding was not unblocked before this implementation.

## Unblocked Findings

### [medium] The test-seam guard accepts production-enabled feature predicates

**Defect class:** A source guard mistakes the presence of a test-feature word inside a conditional compilation expression for proof that every enabled branch is test-only.

The `claudine-cli` [test-seam guard](../../cli/tests/l1/test_seam_gate_guard.rs) splits a `#[cfg(...)]` expression into words and accepts it whenever it contains `feature` or `test` and does not contain `not`. Thus `#[cfg(any(feature = "test-fixtures", unix))]` passes, although the enclosed seam is compiled into every normal Unix build. It also accepts an unrelated production feature. This guard was added to keep the notification fix from regressing, so its false acceptance weakens that protection. The three current seam sites are safely gated; this finding concerns the guard that is meant to preserve them.

| Seam site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Library desktop notification | Current `#[cfg(feature = "test-fixtures")]` | Guard passes; seam is absent by default; clean | Pass |
| Library desktop notification | Temporarily change the constant and send-branch gates to `#[cfg(any(feature = "test-fixtures", unix))]` | Guard still passes on macOS, although normal Unix builds would now honor the stall variable | Reject the production-enabled gates |
| CLI diagnostic snapshot | Current `#[cfg(feature = "test-fixtures")]` | Guard passes; clean | Pass |
| CLI teardown hold | Current `#[cfg(all(unix, feature = "terminal-tests"))]` | Guard passes; clean for the declared test feature | Pass |

I reproduced the failing row by changing both real notification gates, running the declared Level 1 guard test, and restoring the original source. The test passed with the production-enabled gates. Evaluate the conditional expression rather than searching its tokens: every satisfiable branch must require an explicitly approved test-only feature or `test`. Include cases for `all`, `any`, nested expressions, and an unrelated feature in the guard's own tests.

## Blocked Findings

### [high] Ctrl+C during the drain still lacks operating-system keyboard verification

**Defect class:** A keyboard-dependent shutdown promise is checked through direct process or terminal control, while the operating-system keyboard input layer remains untested.

The [shared drain fixture](../../cli/tests/common/drain_interrupt.rs) covers every interactive command, and the terminal tests exercise a real pane. The [Kitty tests](../../cli/tests/level3/level3_drain_ctrl_c.rs) call `kitty @ send-key ctrl+c`, which does not create an operating-system keyboard event. Naming these tests `level3_` and selecting them with `test-l3` does not change their verification level. Review 2 established the focus conflict; the implementation log confirms that no resolution was chosen.

| Command site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` | Two Ctrl+C inputs while a webhook reply is withheld | Level 1 signal, Level 2 tmux command, and Kitty remote key; no OS keypress | OS keyboard proof, or an explicit decision to accept and correctly label terminal-level evidence |
| `inline-compose` | Same pending delivery and retained compose interrupt guard | Same coverage and gap | Same |
| `sequence` | Same pending delivery with shutdown-installed interrupt handler | Same coverage and gap | Same |
| Provider wrapper | Same pending hook message with shutdown-installed interrupt handler | Same coverage and gap | Same |
| `handle` | Noninteractive hook subprocess with a pending message | Deadline and process-exit tests; clean, because no terminal keypress is promised | No keyboard test |

The tmux, Kitty, and Windows console tests share the withheld-webhook scenario, so this is one gap across four interactive sites. The present tests still provide useful process and terminal verification. Resolving this finding requires the human choice in the frontmatter because the specification forbids focusing a window, while the attempted macOS operating-system event did not reach an unfocused window.

## Recurrence

The blocked finding is the same defect class as Review 1's **“Ctrl+C during the delivery drain lacks real-keyboard verification”** and Review 2's **“Ctrl+C during the drain still lacks OS-keyboard verification.”** The earlier fix needed to sweep `compose`, `inline-compose`, `sequence`, and the provider wrapper with an operating-system keypress during the delivery wait, or obtain a decision changing that evidence requirement. It swept all four command paths with terminal remote input but did neither of those remaining steps. The table above includes all four and the clean noninteractive `handle` path.

## Verification performed

- The stalled desktop-notification Level 1 process test passed with `test-fixtures` enabled in 10.347 seconds.
- The test-seam guard passed on the unmodified source. It also passed after the temporary, production-enabled `any(test-fixtures, unix)` mutation; the source was restored immediately afterward.
- `just check-tier-coverage claudine` reported zero stranded tests. The tmux and Kitty files are declared by their respective consolidated test targets; tier placement does not prove OS keyboard input.
- Source review covered the previous findings, the three current test seams, the new guard, the drain tracker and shutdown path, and the interactive command fixture. No terminal or browser window was focused.
