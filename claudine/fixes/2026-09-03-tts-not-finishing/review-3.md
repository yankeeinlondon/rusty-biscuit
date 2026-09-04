---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-04T03:14:54+01:00
spec: 2026-09-03-tts-not-finishing/spec.md
implemented: true
implemented_by: claude/default
log: claudine/fixes/2026-09-03-tts-not-finishing/log.md
description: "A **fix** review of `2026-09-03-tts-not-finishing/spec.md`"
fix: 2026-09-03-tts-not-finishing/review-3.md
previous: 2026-09-03-tts-not-finishing/review-2.md
---

# Review 3: TTS Not Finishing

## Verdict

The fix is **not ready for production**. The implementation closes the three
code defects reported by review 2: native failures now cross the public
sync/async routing boundary to a safe host fallback, v1 executable identity is
required and malformed records are quarantined, and the requester-exit tests
are no longer Unix-only. The central audible-completion acceptance has not,
however, passed on the current implementation, and one explicitly required
Level 1 regression is still absent.

## Findings

### 1. High: the current implementation has no successful real-audio completion acceptance

The user-facing defect is that spoken sentences stop before their end. The
spec therefore makes completed real-device and real-player runs the acceptance
tests: native mono playback must report `Native` / `Complete`, every installed
host player must report `Complete`, and biscuit-speaks must report a completed
native route (`spec.md:715-745`). These are not terminal Level 2 tests; they
belong to the separate `real_` resource tier because an actual audio sink and
player are the behavior under test.

The latest evidence does not satisfy that acceptance. All four Playa `real_`
tests timed out, biscuit-speaks' real test timed out, and the manual checks
could establish only prompt return and durable queue publication—not full
sentence audibility or a completed spool outcome (`log.md:38-44`,
`evidence.md:79-94`). Earlier evidence records successful native and mpv
measurements before the review-2 fallback changes, but no required-sink run
passed on the current implementation. In particular, the fallback change now
makes a native-open timeout proceed to the already wedged host player, so this
branch state has materially different failure behavior from the previously
measured state.

**Required change:** restore a working sink, then run Playa and
biscuit-speaks `just test-real` with `PLAYA_REAL_AUDIO_REQUIRED=1`. Record a
current `Native` / `Complete` result for the 24 kHz mono fixture, `Complete` for
explicit mpv and every installed host player, and an end-to-end
biscuit-speaks `Complete` result. Also record completed outcomes for two
back-to-back background jobs and one lifecycle `say` invocation. A skip or
timeout is not acceptance evidence on the designated audio host.

Verification present: Level 1 fake-backend/process coverage plus unsuccessful
current `real_` runs. Required verification: successful `real_` execution on a
host with a required audio sink. Level 2 and Level 3 are not applicable.

### 2. High: the required public-path truncation regression is still missing

The spec requires `truncated_verdict_warns_once_and_returns_ok` to run a short
successful fake host player through a public playback API, then assert a
`Truncated` report, an `Ok` return, and exactly one warning
(`spec.md:660-664`). The implementation instead constructs already-truncated
`PlaybackReport` values and calls the private `warn_if_truncated` method
directly (`playa/lib/src/report.rs:168-223`). Those tests prove the logging
helper, but they do not prove that elapsed time from a successful host process
is converted to `Truncated`, propagated as success, and warned exactly once by
the production wrapper. The implementation log explicitly confirms that the
specified fake-host regression was never implemented (`log.md:26`).

This is the regression that makes a status-zero early player exit visible—the
same failure mode as the original mpv clipping. A future disconnect between
the public playback pipeline and `warn_if_truncated` would leave the existing
tests green.

**Required change:** add the specified Level 1 process regression through a
report-returning public explicit-player entry point. Use a two-second probed
WAV and a fixture executable that exits successfully after roughly 100 ms;
assert `Ok`, `Host(..)`, `Truncated`, and one warning. Cover both sync and async
public entry points, and make the fixture executable portable so the test runs
on macOS, Linux, and Windows.

Verification present: Level 1 unit tests of a preconstructed report. Required
verification: Level 1 through the public playback/process boundary. The tier
is correct, but the current seam is too low to verify the requirement.

### 3. Medium: the repaired Windows process coverage has not run on Windows

Review 2 required the real `so-you-say --background` requester-exit path and
Claudine's default-emitter handoff/order test to execute as Level 1 process
tests on Windows. The Unix gates are now removed and the test executable is
copied under provider/player names with `EXE_SUFFIX`, which is a sound portable
design. The implementation nevertheless could not compile or execute the
workspace tests for Windows; it compiled a reduced probe with two workspace
imports replaced (`log.md:97-105`). That proves the OS-sensitive standard
library constructs type-check, but it does not prove the actual binaries,
provider discovery, worker spawning, or custom `harness = false` test protocol
on Windows.

**Required change:** run the actual `biscuit-speaks-cli::detached_background`
tests and Claudine `audio_emission` tests on a Windows host through the
canonical Level 1 recipe or CI. Record that the requester exits while work is
blocked and that the reserved job subsequently completes. No Level 2 or Level
3 test is needed.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 native-first free functions and breaker fallback | Level 1 public sync/async pipeline tests | Appropriate and now covered. |
| R2 metadata probing | Level 1 fixture tests | Appropriate and covered. |
| R3 reports and versioned serialization | Level 1 API/Serde and malformed-record tests | Appropriate and covered. |
| R4 truncation warning without error | Level 1 direct helper tests | **Gap:** no public playback/process-boundary regression (Finding 2). |
| R5 mono-only mpv upmix | Level 1 argument tests; earlier real-player evidence | Correct L1 tier; current real acceptance remains open under Finding 1. |
| R6 durable private serialized spool | Level 1 unit and multiprocess tests | Appropriate; required v1 identity and quarantine paths are now covered. |
| R7 spool display/background CLI | Level 1 CLI/process tests | Appropriate; no exact terminal-rendering contract requires Level 2. |
| R8/R10/R14 dependency and CI metadata | Static manifest checks and platform builds | Appropriate. |
| R9/R11 detached biscuit-speaks routing | Level 1 provider/job/failover tests | Appropriate. |
| R12 background requester handoff | Level 1 process test on macOS/Linux-capable source | Correct tier; actual Windows execution is still unverified (Finding 3). |
| R13/R16 documentation and skills | Static contract checks | Appropriate. |
| R15 Claudine handoff and phase ordering | Level 1 process tests on macOS/Linux-capable source | Correct tier; actual Windows execution is still unverified (Finding 3). |
| R17 unchanged notification recipes | Static checks and manual publication evidence | Appropriate. |
| Audible sentence/player completion | Earlier real-audio success; current `real_` runs timed out | **Gap:** current required-sink acceptance is missing (Finding 1). |

There are no keyboard, mouse, paste, IME, hotkey, or terminal-encoder
requirements in this fix, so Level 3 is not applicable. There is also no exact
terminal glyph, width, styling, or scrolling acceptance, so Level 2 is not
applicable. The production-blocking user-observable behavior is audio
completion, which requires the spec's `real_` tier.

## Verification Run

- `git diff --check`: passed.
- `cd playa && just test -- <routing/protocol filters>`: 29 passed, 160
  filtered out.
- `cd biscuit-speaks && just test -- <detached-background filters>`: 2 passed,
  495 filtered out.
- `cd claudine && just test -- <audio-emission/protocol filters>`: 3 passed,
  6,734 filtered out. The build emitted one macOS linker compact-unwind warning;
  no Rust test failed.
- Implementation evidence reports green full Playa and Claudine Level 1
  suites. The full biscuit-speaks run had seven host `say` timeouts, and all
  current real-audio acceptance runs remained unsuccessful.

## Production Readiness

Not production ready. The review-2 implementation is substantially improved
and its three reported code defects are closed, but the exact regression that
detects status-zero truncation is still missing and the current branch has not
demonstrated the core audible-completion behavior on a functioning sink.
