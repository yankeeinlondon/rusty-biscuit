---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-26T21:39:16+01:00
spec: 2026-08-12-ctx-launch-anchor/spec.md
log: claudine/fixes/2026-08-12-ctx-launch-anchor/log.md
implemented: true
implemented_by: codex/default
description: "A **fix** review of `2026-08-12-ctx-launch-anchor/spec.md`"
fix: 2026-08-12-ctx-launch-anchor/review-3.md
previous: 2026-08-12-ctx-launch-anchor/review-2.md
---

# Review 3: Ctx Launch Anchor

## Verdict

The fix is **not ready for production**. Review 2 added counted deltas, canonical
retry/resume dispatch, and a complete WSL validation run. The new evidence still
does not meet AC5 for retry/resume, loop, or concurrent sequence epochs, and the
required Linux, native-Windows, and hosted-CI gates remain open. The public
file-reference topic also now contradicts the resolution order protected by
AC10.

## Findings

### 1. High: retry and resume epochs bypass shell preflight

D4 says every direct, proxy-target, retry, and resume epoch reuses its one
snapshot for narrow/full shell preflight, body, frontmatter, schema, loop, and
lifecycle consumers (`spec.md:254-273`). AC5 correspondingly requires the
per-epoch proof to include preflight (`spec.md:357-365`). The harness path only
runs `preflight_proxy_target` when `bootstrap_pending()` identifies a pending
proxy; retry and resume return before that call
(`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:1209-1241`).

The new retry/resume regression normalizes that omission. It invokes
`materialize_harness_prompt` directly, manually invokes the lifecycle observer,
and expects only `body`, `effective-frontmatter`, and `lifecycle`; `preflight`
is absent from both fresh epochs
(`loop_control/tests/retry_resume.rs:55-80`, `106-131`). Thus the test can pass
while a changed retry/resume document's shell-bearing bytes never traverse a
fresh preflight with that epoch's context.

**Required change:** route retry/resume coverage through the production attempt
preparation path and run the fresh document's preflight with the new epoch
snapshot. Preserve the existing frozen approval policy if approvals must not
reopen, but do not skip the context-bearing preflight audit. Use a fixture with
a shell-bearing `ctx.*` expression and require `preflight` in each exact epoch
map.

Verification level present: Level 1 materialization and transition-helper
coverage. Required level: Level 1 through the real retry/resume preparation
seam, including shell preflight.

### 2. High: the exact-epoch proof is not attributable under concurrency and manufactures loop observations

`InvocationWorkSnapshot::document_epoch_since` subtracts invocation-global
counters and consumer maps; neither snapshot carries an epoch or task identity
(`claudine/lib/src/invocation_context.rs:111-149`). That is exact only while no
other preparation overlaps the bracket. Sequence groups do overlap work:
`run_parallel` launches task executions on scoped worker threads
(`claudine/lib/src/composition/sequence/task/group.rs:204-305`), and each task
captures from the shared invocation
(`claudine/cli/src/commands/wrap/sequence/task_run.rs:202-220`). A task's
before/after delta can therefore contain sibling constructions and consumers.
The new sequence assertion covers one serial `compose_step` only
(`sequence/jit/tests.rs:14-79`), so it cannot prove AC5's per-task epoch
contract on the parallel route.

The loop proof is also self-fulfilling: its test code directly records
`Preflight`, `LoopCondition`, and `Lifecycle` before asserting those names
(`claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs:27-33`,
`45-49`). Those calls do not show that the production consumer seams observed a
populated context. The retry/resume test has the same problem for lifecycle by
calling `observe_reentry_lifecycle_context` itself rather than reaching it
through `start_lifecycle_phase`.

**Required change:** make observations intrinsically epoch-attributable (for
example, with an epoch token/guard or epoch-local recorder) so overlapping task
preparations cannot contaminate one another. Exercise an actually parallel
sequence group and the production loop/re-entry seams. Remove test-authored
consumer records; a regression must fail when production observation wiring is
removed.

Verification level present: Level 1 exact deltas for serial paths plus
test-authored loop/re-entry observations. Required level: Level 1 through the
real consumer seams, including deliberately overlapping parallel task epochs.

### 3. High: AC13/AC14 still lacks Linux, native-Windows, and hosted-CI evidence

The implementation log now records complete current-worktree L1/L2/lint runs
on macOS and WSL, including the affected Darkmatter gates
(`log.md:121-152`). That materially improves Review 2. It also records that
`build-linux` became unreachable before reconstruction/testing, native Windows
had no volume satisfying the repository's 50 GiB preflight, and hosted CI could
not consume the uncommitted snapshot (`log.md:153-172`). No Linux or native
Windows L1/L2/lint verdict, and no hosted-CI verdict, is claimed.

AC14 explicitly requires the complete affected L1 and L2 suites to be green on
macOS, `build-linux`, `build-win`, and `build-win-native` before hosted CI starts
(`spec.md:397-401`). Source portability and WSL execution cannot establish
native-Windows prefix, separator, command-resolution, or filesystem behavior.

**Required change:** run the exact implementation through the remaining
canonical Linux and native-Windows L1/L2/lint gates, rerun L1 after any L2 fix,
and then run the hosted workflow. Keep L2 non-focus-taking and record exact
results. Remove the residual `/tmp/rb-ctx-anchor-a3Nzbs` staging directory after
the Linux host becomes reachable, as already noted in the log.

Verification level present: macOS and WSL Level 1/Level 2/lint. Required level:
the native multi-OS and hosted-CI matrix mandated by AC13/AC14.

### 4. Medium: the file-reference topic reverses AC10's implicit resolution order

The current public topic says an implicit `path/to/file.md` resolves from the
current working directory first and the repository root second
(`claudine/docs/topics/file-referencing.md:89-113`). It later defines the
composition CWD as the composed file's directory
(`file-referencing.md:160-166`). Together those statements describe
source-first, repository-second resolution.

The implemented and specified contract is repository-first, then
source-relative for implicit document references, while explicit references
pin to the source directory (`spec.md:384-387`;
`claudine/lib/src/composition/sequence/source.rs:29-31`). Existing conflicting
fixtures lock that behavior in. The runtime is correct, but the new
authoritative documentation tells prompt authors to expect the losing file
when both candidates exist.

**Required change:** rewrite the implicit-path and CWD sections to distinguish
the caller's launch directory, the document source directory, and the process
CWD, and state the implemented repository-first/source-relative ordering
unambiguously. Keep `$schema`'s document-relative rule separate.

Verification level present: Level 1 conflicting-candidate runtime fixtures.
Required level: Level 1 is sufficient for behavior; the documentation must be
made consistent with those fixtures and AC10.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 direct/loop launch-area and lifecycle parity | Level 1 real CLI | Appropriate level. |
| AC2 opposing-area body, frontmatter, preflight, and lifecycle | Level 1 real CLI with conflicting launch/source areas | Appropriate level. |
| AC3 external-source and outside-repository matrix | Level 1 real CLI on macOS and WSL | Appropriate semantic level; remaining native portability is tracked under AC13/AC14. |
| AC4 real CLI capture owner | Level 1 binary execution | Appropriate level. |
| AC5 exact per-epoch snapshot reuse | Level 1 serial deltas and helper-level re-entry/loop tests | **Gap:** retry/resume omit preflight, some consumers are recorded by test code, and invocation-global deltas cannot identify overlapping epochs. |
| AC6 target identity across direct/proxy/re-entry/loop/sequence | Level 2 direct/proxy plus Level 1 route tests | Appropriate semantic level; retry/resume epoch completeness remains an AC5 gap. |
| AC7 sequence graph/task/source parity | Level 1 graph, task, and serial epoch tests | Functional semantics are covered at the right level; exact parallel task attribution is an AC5 gap. |
| AC8 proxy/re-entry and loop parity | Level 2 direct/proxy plus Level 1 re-entry and loop tests | **Gap:** retry/resume shell preflight is skipped and the loop consumer proof is test-authored. |
| AC9 system-prompt and overlay relocation | Level 1 canonical-owner relocation tests | Appropriate level. |
| AC10 source-owned file and schema resolution | Level 1 conflicting launch/source fixtures | Runtime level is appropriate; public documentation contradicts the verified contract. |
| AC11 no extra discovery | Level 1 invocation work counters | Partial: retained evidence is counted, but global deltas are not safely attributable to concurrent epochs. |
| AC12 capture-owner guard | Level 1 production-source inventory with seeded violation | Appropriate level. |
| AC13 cross-platform paths | macOS and WSL Level 1 | **Gap:** no current implementation verdict on Linux or native Windows. |
| AC14 complete validation matrix | macOS and WSL Level 1/Level 2/lint plus affected Darkmatter gates | **Gap:** Linux, native Windows, and hosted CI remain unverified. |

Level 1 is the correct behavioral tier for context values, target identity,
ownership, and file resolution because terminal rendering and terminal input
encoding do not affect those semantics. AC14 independently requires Level 2
execution across the named environments. No Level 3 coverage is applicable:
the fix has no keyboard, paste, IME, mouse, or terminal-input encoder behavior.

## Review 2 Closure Status

- Finding 1, per-document epoch accounting: partially closed. Count-preserving
  deltas and serial route assertions were added, and retry/resume dispatch is no
  longer selected by directly mutating `state.entry`. Findings 1 and 2 above
  show that the required production-seam and concurrent attribution proof is
  still incomplete.
- Finding 2, cross-platform validation: partially closed. The current macOS and
  WSL matrices are green; Linux, native Windows, and hosted CI remain open as
  Finding 3 above.

## Verification Run

- Focused launch-anchor accounting and CLI regressions passed: 12 tests run,
  12 passed.
- `cd claudine && just test` passed all five package groups on the macOS review
  host.
- `cd claudine && just lint` passed, including 18 error guards, the lifecycle
  documentation guard, and Clippy for all five package groups.
- The implementation log records canonical macOS L2 success (230 CLI and three
  generator tests), a complete WSL L1/L2/lint matrix, and the complete affected
  macOS/WSL Darkmatter gates. This review did not rerun those remote/L2 gates.
- Linux, native Windows, and hosted CI have no successful current-implementation
  result.

## Production Readiness

Not production ready. Close the retry/resume preflight defect, make the AC5
proof production-seam and concurrency-safe, correct the public file-resolution
contract, and complete the remaining AC13/AC14 execution matrix before the next
readiness review.
