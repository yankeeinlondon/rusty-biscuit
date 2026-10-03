---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-25-lifecycle-message-exit-race/spec.md`
fix: 2026-09-25-lifecycle-message-exit-race/review-4.md
spec: 2026-09-25-lifecycle-message-exit-race/spec.md
previous: 2026-09-25-lifecycle-message-exit-race/review-3.md
reviewed_by: codex/gpt-6-sol
created: 2026-09-28T01:27:30-07:00
implemented: false
ready: true
human_review: true
has_blocked_findings: true
blocked: true
recurrence: true
findings:
    - "[high] OS keyboard coverage during the delivery drain remains Linux-only"
human_review_items:
    - |-
        A second Ctrl+C must stop Claudine promptly while it waits for an outbound message at exit. The approved isolated-desktop approach now proves this with real keyboard events on Linux for compose, inline-compose, sequence, and a provider wrapper. The same drain behavior on macOS and Windows has no operating-system keyboard test. Existing tests send process signals, console bytes, or terminal remote commands, which do not exercise those systems' keyboard input paths.

        Please choose how to resolve the remaining test gap:

        - Provide an isolated macOS login session or virtual machine and an isolated Windows session or virtual machine where these keyboard tests can run without taking focus from the host user's desktop. The implementation can then add the four drain cases on each system.
        - APPROVED: Accept Linux operating-system keyboard coverage plus the existing macOS and Windows process and terminal tests as sufficient for this specific drain promise. Record that narrower verification requirement in the fix's current documentation.

        The first option continues the isolated-desktop approach already approved in Review 3. The second changes the required verification level on macOS and Windows; it does not change Claudine's promised behavior.
---

# Review 4: Lifecycle Message Exit Race

## Verdict

**Not ready for production.** The test-seam guard now evaluates conditional compilation correctly. The approved isolated-desktop approach also provides a genuine operating-system Ctrl+C test on Linux. The same keyboard-dependent exit promise remains unverified at that level on macOS and Windows. This is a test-coverage gap, not a request for cross-OS run results: those two test implementations do not exist.

This fix does not add or change a file-format or configuration reader, so the input robustness matrix does not apply. I found no additional functionality, ergonomics, or performance defect in the delivery tracker and ordinary shutdown path.

## Prior review disposition

| Review 3 finding | Result in this iteration |
| --- | --- |
| Test-seam guard accepts production-enabled feature predicates | Fixed. The shared conditional-compilation evaluator rejects `any(feature = "test-fixtures", unix)`, unrelated features, negation, and malformed predicates. The seam guard and error-source guard use it. Eleven focused Level 1 tests passed, including the source scan. |
| Ctrl+C during the drain lacks operating-system keyboard verification | Partly fixed after the author approved an isolated test desktop. Four Linux Level 3 cases now inject XTEST key events into Kitty on a private Xvfb display. The Kitty remote-command cases were correctly moved to Level 2. macOS and Windows still lack OS keyboard cases during the drain, as the implementation log expressly defers. |

Review 3's blocked keyboard finding was unblocked before the last implementation by the author's approval of an isolated test desktop. That approval was implemented on Linux; the remaining platforms need the decision in the frontmatter.

## Blocked Findings

### [high] OS keyboard coverage during the delivery drain remains Linux-only

**Defect class:** A keyboard-dependent shutdown promise is verified through the operating-system input path on one supported platform but only through lower-level control inputs on the other supported desktop platforms.

The `claudine-cli` [Linux Level 3 test](../../cli/tests/level3/level3_drain_ctrl_c.rs) calls the shared [drain scenario](../../cli/tests/common/drain_interrupt.rs) with XTEST Ctrl+C presses. The scenario waits for a withheld webhook response, checks that the first press leaves the drain running, and requires the second to exit with code `130` promptly. The [private-display harness](../../../biscuit-test-harness/src/xvfb.rs) focuses Kitty only inside its own Xvfb display. The implementation log records four passing cases in a Linux container, including a mutation in which replacing the second Ctrl+C with Ctrl+X made the test fail after the drain budget.

The test target declares and selects these Linux cases, and the four Kitty remote-key cases are now correctly declared and selected as Level 2. On macOS and Windows, the existing tests do not send an OS keyboard event to a terminal while this drain is active. This matters because the terminal's input encoder and the platform's focus and console behavior sit between a physical keypress and Claudine's interrupt handler. The specification requires the second-press check on macOS, Linux, and Windows, and the review's test-rigor rule requires Level 3 for a keypress promise.

| Command site | Shape tested on each platform | Observed result | Expected result |
| --- | --- | --- | --- |
| `compose` | Two Ctrl+C presses while the webhook reply is withheld | Linux: Level 3 XTEST through Kitty, clean. macOS: Level 2 Kitty remote key and Level 1 process signal. Windows: Level 1 console byte/control event. | OS keyboard event reaches the terminal during the drain on each supported desktop platform, or an explicit decision narrows the verification requirement. |
| `inline-compose` | Same drain and retained compose interrupt guard | Same Linux coverage and macOS/Windows gap. | Same. |
| `sequence` | Same drain and shutdown-installed interrupt guard | Same Linux coverage and macOS/Windows gap. | Same. |
| Provider wrapper | Same drain after a hook message | Same Linux coverage and macOS/Windows gap. | Same. |
| `handle` | Noninteractive hook subprocess with a pending message | Deadline and process-exit tests; clean. There is no terminal keypress contract for this command. | No OS keyboard test. |

I checked the declared test targets, live tier recipes, four command-specific tests at each available level, and the shared fixture. `just check-tier-coverage claudine` reports zero stranded tests. No macOS or Windows Level 3 drain case exists to run, so a local reproduction cannot turn this absence into a passing public result. The human review item offers the infrastructure path approved in Review 3 or an explicit narrower evidence requirement.

## Recurrence

This is the same defect class as Review 1's **“Ctrl+C during the delivery drain lacks real-keyboard verification,”** Review 2's **“Ctrl+C during the drain still lacks OS-keyboard verification,”** and Review 3's **“Ctrl+C during the drain still lacks operating-system keyboard verification.”** Those fixes needed to sweep `compose`, `inline-compose`, `sequence`, and the provider wrapper on every platform for which the specification promises keyboard behavior. The last fix completed that sweep on Linux, but stopped at process and terminal input on macOS and Windows. The table above includes all four sibling command sites and the clean noninteractive `handle` path.

## Verification performed

- Eleven focused Level 1 conditional-compilation and error-source guard tests passed.
- `cargo nextest list` found all four renamed Kitty cases in the declared Level 2 target.
- `just check-tier-coverage claudine` reported zero stranded tests.
- Source review covered the new Xvfb harness, Linux Level 3 cases, renamed Kitty Level 2 cases, shared drain fixture, delivery tracker, ordinary shutdown, and the prior review findings. The Linux Level 3 execution and mutation results above come from the implementation log; this review did not rerun those Linux-only tests on the macOS host.
