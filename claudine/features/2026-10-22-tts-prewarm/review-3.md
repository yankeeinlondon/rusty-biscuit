---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-14T10:08:49-07:00"
spec: "2026-10-22-tts-prewarm/spec.md"
implemented: false
description: "A **feature** review of `2026-10-22-tts-prewarm/spec.md`"
feature: "2026-10-22-tts-prewarm/review-3.md"
previous: "2026-10-22-tts-prewarm/review-2.md"
---

# Review of TTS Pre-warm

The feature is **production ready**. The previous review's unblocked finding is repaired, the first review's two repairs remain intact, and this review found no additional defects requiring a fix cycle.

## Prior unblocked findings

Checked the previous review's finding first: **Sequence preflight coverage passes with preflight preparation disabled**.

Claudine's [three-prompt CLI test](../../cli/tests/l1/tts_prewarm.rs#L237) now makes the first fake provider wait for preparation records from the second and third prompts, then records whether it observed them. Those later steps cannot execute until the first provider exits. This observes preparation during the command, so ordinary preparation of later steps cannot satisfy the assertion afterward. The separate blocked-preparation test proves that all three providers still launch when synthesis cannot finish. The stale comment claiming the setup phrase was collected only by preflight is removed.

Independently repeated the previous review's reproduction: temporarily bypass [Claudine's sequence preflight submission](../../cli/src/commands/wrap/sequence/mod.rs#L150) with `if true || !handle.is_enabled()`, without editing the fixture or its assertions. The strengthened test **fails**, reporting `timeout` instead of `prepared`. All three providers still launch and the later phrases are eventually collected; the new assertion detects precisely the ordering lost by the bypass. Restored the source byte for byte afterward. The restored full package suite passes, including this test in 1.7 seconds.

The sweep of the earlier finding's sibling tests is complete:

| Site | Check | Result |
| --- | --- | --- |
| Three-prompt recording test | Bypass preflight; observe later prompt records while the first provider runs | Fails as required; passes with preflight restored |
| Three-prompt blocked-preparation test | Hold the first preparation indefinitely | All three providers launch; no audio published |
| Mixed-configuration sequence test | Inspect preparation settings for task stacks and prompt lifecycles | Passes; claims configuration correctness, not preflight ordering |
| Six deferred task-stack tests | Top-level, serial-member, and parallel-member setup and teardown | Pass; explicitly test reconsideration when task inputs exist |
| Prompt-context control | Captured document `env` and `ctx` | Passes; explicitly tests document preparation |
| Compose blocked-preparation test | Launch with preparation blocked | Passes; no sequence preflight claim |
| Library three-prompt preflight test | Direct submission through the public helper before any step runs | Passes; verifies queued candidates independently of worker completion |
| Remaining CLI tests | Gates, proxy, retry, initialization reread, invalid group stacks | Pass; no claim that final recorder contents prove preflight ordering |

Also rechecked both findings from `review-1.md`. Claudine submits deferred task stacks at all three task context entry points in [task_run.rs](../../cli/src/commands/wrap/sequence/task_run.rs): top-level tasks, serial members, and parallel members. Setup and teardown share the [task-stack collector](../../lib/src/composition/lifecycle/prewarm/sequence.rs), preserving the task's captured context and exclusions for sequence inputs. Their six CLI cases verify resolved `env`, `ctx`, and interpolated frontmatter, with once-only literal controls.

The [Claudine pre-warm worker](../../lib/src/composition/lifecycle/prewarm/worker.rs) carries each candidate's playback configuration in its queue and de-duplication key. Task preparation and playback both use [task_playback_settings](../../lib/src/composition/sequence/task/mod.rs); document lifecycles use the bridged user settings. The user's pre-warm switch governs both independently. Library and CLI mixed-configuration tests cover top-level, serial-member, and parallel-member setup and teardown, plus prompt document controls. Both earlier repairs remain correct.

## Findings

None.

## Observations

None.

## Recurrence

No finding recurs. The two implementation defects in `review-1.md` and the test assertion defect in `review-2.md` are addressed across their reported sibling sites.

## Verification and requirement coverage

Executed from `claudine/`, with parent `PLAYA_DRY_RUN=1` to keep ordinary fixtures silent:

- `just test-cli tts_prewarm::`: **20 passed**.
- Mutation check of the repaired ordering test: **failed as expected** with preflight bypassed; source restored byte for byte.
- `just test`: **9,152 passed, 9 skipped**. The pre-warm CLI tests all completed within 5 seconds; an unrelated existing source-guard test was reported slow.
- `just lint`: **passed**.
- From the repository root, `just check-tier-coverage claudine`: **passed; zero stranded tests**.
- `git diff --check`: **passed**.

| Acceptance criterion | Strongest relevant verification | Assessment |
| --- | --- | --- |
| 1: all supported stacks and speech forms; initialization excluded | L1 parsed collection fixtures for aliases, bundles, named and positional arguments, both branches, depth-three nesting, and loose loop verbs; CLI recorder | Covered |
| 2: early frontmatter and captured values; late globals and sequence values excluded | L1 classification tables, scoped-handle tests, public task submission, six deferred CLI cases and document control | Covered |
| 3: no speculative shells or functions; returned strings stay data; empty results suppressed | L1 classification tests for untaken branches, zero-argument and shell-probing calls, returned syntax, resolution failure, and empty text | Covered |
| 4: every statically reached prompt submitted before step one; unresolved inputs reconsidered | L1 public graph submission and strengthened CLI observation during step one; bypass makes that test fail; deferred task/document tests | Covered; previous gap repaired |
| 5: proxy target uses its applied overlay | L1 CLI proxy fixture records the overlaid text and excludes the authored value | Covered |
| 6: configuration and command dry-run gates | L1 worker gates and CLI configuration/dry-run fixtures | Covered |
| 7: configuration shapes, compatibility, rejection, editor preservation | L1 public loader/bridge matrix, hook and lifecycle controls, editor state changes and save/reload | Covered |
| 8: best-effort failures, one sanitized warning, continuation, no retry | L1 failing preparer and captured tracing records, with CLI launch/outcome controls | Covered |
| 9: blocked synthesis never prevents provider launch | L1 blocked preparer with recording CLI providers for compose and all three sequence steps | Covered |
| 10: audio dry run leaves isolated caches and spool untouched | L1 CLI frozen-prompt fixture with private cache/temp directories; accepted dry-run spelling and discovery-gate tests | Covered |
| 11: lifecycle/configuration docs and skill reflect behavior and limits | Direct review of current topic pages and the claudine skill | Covered |
| 12: shared ownership and de-duplication across snapshots, re-entry and task scopes | L1 worker/key and scoped-handle tests; initialization, retry, proxy and sequence CLI records; shared preparation call sites inspected | Covered |
| 13: independent event-time publication, no-op success, cancellation without draining | L1 controllable preparer futures and recording emitter; completed work retained and queued work abandoned on drop | Covered |
| 14: package tests and lint; conditional detached-playback verification | Canonical package recipes above; detached-playback boundary unchanged | Passed |

L1 is appropriate for these configuration, background-work, and publication contracts. The editor changes preserve data and explain the existing toggle; they introduce no new keyboard interaction, terminal encoding, color, width, or scrolling contract. No requirement has only a lower verification level than it needs. L2 is not required by the spec's conditional rule because the detached-playback boundary was not changed.

Claudine's CLI tests are declared in the consolidated `l1` target, selected by the L1 recipe, and enabled by `test-fixtures` in both local and CI metadata. The CLI recorder tests use Unix shell stubs; portable library tests exercise the underlying collector, classifier, worker, configuration, and task submission. Cross-OS execution evidence remains CI's responsibility and does not block this assessment.

The configuration robustness matrix in the plan names outcomes for `tts`, optional `provider`, and `prewarm`. The public loader/bridge test covers a positive control, absence, explicit null, wrong types, supported empty values, duplicate keys, trailing content, and unknown keys. Null is rejected for the newly optional fields; scalar fields have no element cases. JSON and JSON5 use the same JSON5 loader, preserving its existing last-wins duplicate-key behavior. This repair adds no configuration field or reader.

No human-only check or unresolved design decision is needed. No production or test source changes were retained by this review.
