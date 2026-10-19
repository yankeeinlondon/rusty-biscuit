---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Sequence preflight coverage passes with preflight preparation disabled"
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-14T09:54:23-07:00"
spec: "2026-10-22-tts-prewarm/spec.md"
implemented: true
implemented_by: claude/opus
log: claudine/features/2026-10-22-tts-prewarm/log.md
description: "A **feature** review of `2026-10-22-tts-prewarm/spec.md`"
feature: "2026-10-22-tts-prewarm/review-2.md"
previous: "2026-10-22-tts-prewarm/review-1.md"
next: "2026-10-22-tts-prewarm/review-3.md"
---

# Review of TTS Pre-warm

The feature is **not production ready** because one acceptance requirement lacks an effective command-level test: collecting every sequence prompt's early phrases before the first step starts. Both findings from the first review are repaired. The current production code submits during preflight; the remaining finding concerns a test that can no longer detect losing that behavior.

## Prior unblocked findings

The first review's two findings were both unblocked. Their repairs were checked first, including every affected task execution site.

Claudine's [task stack submission](../../lib/src/composition/lifecycle/prewarm/sequence.rs) now revisits setup and teardown with the task's captured context and the step's effective frontmatter. The CLI calls it when a top-level task, serial member, or parallel member establishes its context. The shared invocation handle retains de-duplication and exclusions for sequence-injected keys.

Claudine's [worker](../../lib/src/composition/lifecycle/prewarm/worker.rs) now carries each candidate's playback configuration in both the queue and its de-duplication key. Task preparation and playback share [task_playback_settings](../../lib/src/composition/sequence/task/mod.rs), which preserves automatic provider selection for task stacks. Document lifecycles retain the user's provider and voice. The user's pre-warm switch governs both paths independently of those playback settings.

The [CLI regression tests](../../cli/tests/l1/tts_prewarm.rs) exercise these instances with silent, never-firing phrases:

| Site | Deferred `env`, `ctx`, and interpolated frontmatter | Preparation configuration |
| --- | --- | --- |
| Top-level task setup | Resolved and prepared once | Matches task playback |
| Top-level task teardown | Resolved and prepared once | Matches task playback |
| Serial member setup | Resolved and prepared once | Matches task playback |
| Serial member teardown | Resolved and prepared once | Matches task playback |
| Parallel member setup | Resolved and prepared once | Matches task playback |
| Parallel member teardown | Resolved and prepared once | Matches task playback |
| Prompt document lifecycle | Captured `env` and `ctx` resolve; clean control | Uses the user's Kokoro provider and `af_heart` voice in the mixed sequence |
| Group container with task-level setup | Rejected before preparation | No speech site |

The library tests additionally verify that the same text under two playback configurations produces two preparations, repeated text under one configuration produces one, and default task playback cannot override `prewarm: false`. Both prior findings are addressed.

## Findings

### high: Sequence preflight coverage passes with preflight preparation disabled

**Authority: spec.** Acceptance criterion 4 requires: “A sequence of three prompts submits every eligible early phrase from all three during preflight, before step one launches.” The corresponding command-level test no longer distinguishes this behavior from collecting phrases only when each task or prompt runs. This leaves the required ordering untested at the boundary that connects preflight to execution.

**Defect class:** assertions made only after the entire command finishes allow a later preparation phase to stand in for the earlier phase the test claims to verify.

In `claudine-cli`, [a_three_prompt_sequence_submits_every_document_before_step_one](../../cli/tests/l1/tts_prewarm.rs#L220) relies on step one's `setting up` phrase as evidence that preflight submitted work. Its comment says “only preflight collects it.” The repair correctly added [task-context submission](../../cli/src/commands/wrap/sequence/task_run.rs#L101), which also collects that phrase before preparing the first prompt. Consequently, the blocking case still records `setting up` without preflight. In the recording case, ordinary preparation of the three prompts eventually produces every expected phrase. Neither assertion observes whether future steps were submitted before the first step launched.

Reproduction used the existing three-prompt fixture without changing its input or assertions. Temporarily make [submit_preflight_prewarm](../../cli/src/commands/wrap/sequence/mod.rs#L158) return immediately by changing its gate to `if true || !handle.is_enabled()`. Run:

```sh
PLAYA_DRY_RUN=1 just test-cli tts_prewarm::
```

**All 19 CLI tests pass**, including both modes of the test named above. The fixture overrides the parent dry-run value to enable its recording preparer. Restore the source afterward; the restored suite also passes all 19 tests. This removes all sequence preflight submission from the executed command, so the surviving test is a direct demonstration of the missing assertion, rather than a hypothetical input edge case.

The bounded sweep covered the preflight helper and every CLI pre-warm test that checks sequence preparation or launch progress:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Three-prompt CLI test, blocking mode | Existing fixture; preflight submission bypassed | Passes with one `setting up` record and three launches | The overall ordering test must reject the missing preflight submission; this case still proves launch does not wait on preparation |
| Three-prompt CLI test, recording mode | Same bypass; existing three prompt phrases | Passes after all three prompts run | Reject collection deferred until each prompt runs |
| Mixed-configuration sequence test | Same bypass; task stacks and prompt lifecycles use different configurations | Passes | Clean for its configuration contract; it supplies no before-first-step assertion |
| Six deferred task tests and prompt-context control | Same bypass; captured `env`, `ctx`, and derived frontmatter | Pass | Clean for reconsideration at task/document preparation; these tests intentionally exercise the later phase |
| Compose blocked-preparation test | Same bypass; blocking preparer and recording provider | Passes | Clean: independently verifies launch can finish while preparation is blocked |
| Library three-prompt preflight test | Direct public submission call with the parsed graph and a blocked preparer | Queues all six expected candidates and returns | Clean helper coverage; it cannot detect the CLI omitting the call |

The remaining CLI tests cover gates, proxy adoption, retry, initialization rereads, and invalid group stacks. They also pass under this bypass and make no claim about sequence preflight ordering. No other test in this feature establishes that ordering through the command.

**Recommended repair:** strengthen the existing sequence fixture so later steps cannot run until the test has observed their preparation records. For example, hold the first fake provider on a bounded handshake until the recorder contains the later prompts' phrases, then release it. Keep the blocked-preparer case as the separate proof that synthesis does not delay launch. Use never-firing speech and readiness synchronization, not a fixed sleep. Confirm that bypassing preflight submission makes the strengthened test fail. Update the stale “only preflight collects it” comment alongside the assertion. This needs a focused test change; no production preparation redesign is requested.

## Recurrence

No finding repeats an earlier finding's defect class. The first review identified missing task submissions and mismatched playback configurations. Both are fixed. The new finding is loss of a test's ability to distinguish preflight from the task submission added by that repair.

## Verification and requirement coverage

- `PLAYA_DRY_RUN=1 just test` in `claudine/`: **9,151 passed, 9 skipped**.
- `PLAYA_DRY_RUN=1 just lint` in `claudine/`: **passed**.
- With sequence preflight submission temporarily bypassed, `just test-cli tts_prewarm::`: **19 passed**, demonstrating the finding.
- After restoring the source byte for byte, the same CLI suite: **19 passed**.
- `git diff --check`: **passed** before writing this review.

An initial test run overlapped lint's test recipe, which builds a different CLI feature variant into the same output directory. Completion-fixture tests then failed. The full sequential rerun above passed; that interference is not attributed to this feature. All subsequent mutation checks ran without concurrent builds.

| Acceptance criterion | Verification level and evidence | Assessment |
| --- | --- | --- |
| 1: stacks, aliases, bundles, nesting; initialization excluded | L1 parsed collection tests and CLI records | Covered |
| 2: early captured values and frontmatter; late globals and sequence inputs excluded | L1 classification tables, task submission test, six deferred CLI cases | Covered; prior gap repaired |
| 3: no speculative shell/functions; returned strings remain data; empty speech skipped | L1 authored-expression and resolution tests | Covered |
| 4: all sequence prompts submitted during preflight; unresolved inputs reconsidered | L1 direct helper tests and CLI sequence test | Helper and reconsideration covered; command ordering gap reported above |
| 5: proxy target prepared with its overlay | L1 CLI proxy fixture | Covered |
| 6: setting and command dry-run gates | L1 worker and CLI gate tests | Covered |
| 7: configuration shapes, compatibility, rejection, editor preservation | L1 public-loader matrix, hook/lifecycle controls, editor state and save/reload tests | Covered |
| 8: best-effort errors, sanitized single warning, later candidates continue | L1 failing preparer and tracing assertions | Covered |
| 9: blocked preparation does not prevent provider launch | L1 blocked preparer and recording CLI provider | Covered |
| 10: audio dry run leaves isolated caches and spool untouched | L1 CLI fixture with private cache/temp locations and accepted-value tests | Covered |
| 11: lifecycle/configuration docs and skill | Direct review of current topic pages and skill | Updated for both repairs |
| 12: shared ownership, re-entry, configuration-aware de-duplication | L1 handle/key tests and initialization/retry/proxy/sequence CLI tests | Covered |
| 13: playback remains independent; no-op, failure, and cancellation behavior | L1 controllable preparer and recording publisher | Covered |
| 14: package tests and lint | Canonical recipes above | Passed |

L1 is appropriate for these configuration, background preparation, and publication contracts. The finding is about what an L1 test observes, not a need for a higher tier. This feature adds no terminal input encoding or visual rendering requirement. The existing detached-playback boundary remains unchanged, so the conditional L2 run is unnecessary.

The CLI tests are declared by the consolidated `l1` target and enabled by `test-fixtures` in the local recipe and CI metadata. Their Unix restriction comes from shell-script provider stubs; portable library tests cover the underlying behavior. No new test is stranded in an unselected tier. Cross-OS execution evidence remains CI's responsibility.

The configuration input matrix is specified in the plan and exercised through the public loader and settings bridge for `tts`, optional `provider`, and `prewarm`. It covers the positive control, absence, explicit null, wrong types, empty supported shapes, duplicate keys, trailing content, and unknown keys. JSON input uses the same JSON5 loader. Null is rejected for the newly optional fields; existing duplicate-key behavior remains last-wins. This repair adds no configuration field or reader.

No human-only verification or unresolved design decision blocks the focused test repair. No implementation changes were retained by this review.

## Observations

None.
