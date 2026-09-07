---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-04T03:43:55+01:00
spec: 2026-09-03-tts-not-finishing/spec.md
implemented: false
description: "A **fix** review of `2026-09-03-tts-not-finishing/spec.md`"
fix: 2026-09-03-tts-not-finishing/review-4.md
previous: 2026-09-03-tts-not-finishing/review-3.md
---

# Review 4: TTS Not Finishing

## Verdict

The fix is **not ready for production**. Review 3's missing public-path
truncation regression is now implemented correctly for both synchronous and
asynchronous playback. No new code defect was found in that change. The two
remaining blockers are acceptance evidence the implementation still has not
produced: successful completion through a real audio sink on the current
implementation, and execution of the cross-platform requester-exit paths on
Windows.

## Findings

### 1. High: the repaired implementation still has no successful current real-audio acceptance

The reported user-facing defect is premature termination of spoken audio. The
spec consequently requires real-resource acceptance proving `Native` /
`Complete` for the 24 kHz mono fixture, `Complete` for explicit mpv and every
installed host player, and an end-to-end biscuit-speaks `Complete` result. It
also requires completed outcomes for back-to-back background speech and a
lifecycle `say` invocation (`spec.md:706-745`).

The current evidence still cannot demonstrate those outcomes. CoreAudio and
the USB DAC were unresponsive; `afplay`, mpv, `say`, and Kokoro did not finish,
and several audio clients remained blocked even after `SIGKILL`. The manual
checks prove prompt return, durable publication, and queue order, but explicitly
do not prove full-sentence audibility or completed spool outcomes
(`evidence.md:79-128`). Earlier successful native/mpv measurements predate the
latest native-fallback changes, so they are useful historical evidence but not
acceptance of the current implementation.

**Required change:** reset the audio sink, then run Playa and biscuit-speaks
`just test-real` with `PLAYA_REAL_AUDIO_REQUIRED=1`. Record current
`Native` / `Complete`, explicit-mpv `Complete`, every-installed-player
`Complete`, and biscuit-speaks end-to-end `Complete` results. Then verify two
back-to-back background jobs and one lifecycle `say` reach completed journal
outcomes in durable order. A timeout or skip on the designated audio host is
not acceptance evidence.

Verification present: Level 1 fake-backend/process coverage, historical
real-audio measurements, and unsuccessful current `real_` attempts. Required
verification: a successful current `real_` run on a functioning sink. Level 2
and Level 3 are not applicable.

### 2. High: the requester-exit and lifecycle handoff paths still have no Windows execution evidence

The implementation now makes the relevant fixtures platform-neutral, and the
evidence reports broad Windows suites from an earlier phase. However, the
specific repaired `biscuit-speaks-cli::detached_background` requester-exit
tests and Claudine default-emitter `audio_emission` tests have not executed on
Windows in their current form. The latest cycle could only retain an isolated
Windows-target type-check because the Windows runner was unavailable without
an SSH-backed cross-check or a commit/push (`log.md`, “Implementation of Review
Findings #3”).

These tests cover the behavior that motivated the detached worker: the caller
must exit promptly while the worker continues, and ordered audio must later
complete. The standard-library constructs compiling for a Windows target does
not exercise executable discovery, process creation flags, environment
handoff, spool locking, or the custom `harness = false` test protocol on
Windows.

**Required change:** run the actual biscuit-speaks detached-background tests
and Claudine audio-emission tests on Windows through the canonical Level 1 CI
leg or the repository's native Windows cross-check. Record that the requester
returns while playback is blocked and that the reserved work subsequently
completes in order.

Verification present: Level 1 execution on macOS, a Windows-target type-check,
and older broad Windows results that predate the repaired fixtures. Required
verification: Level 1 execution of the current tests on Windows. Level 2 and
Level 3 are not applicable, but the platform coverage is incomplete for a
cross-platform contract.

## Closed Since Review 3

Review 3 finding 2 is closed. `playa/lib/tests/truncated_verdict.rs` drives a
two-second probed mono WAV through the public report-returning explicit-player
APIs using a portable fake mpv that exits successfully after about 100 ms. The
synchronous and asynchronous cases each assert `Ok`, `Host(Mpv)`,
`Truncated`, and exactly one warning. The test does not require an audio device
and is correctly classified as Level 1.

The review reran the focused canonical test selection: 2 tests passed and 189
were skipped by the filter. The implementation log also records red-first
checks against both timing and warning propagation, which demonstrates that
the new assertions are not vacuous.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 native-first free functions and fallback boundary | Level 1 public sync/async pipeline tests | Appropriate and covered. |
| R2 metadata probing | Level 1 fixture and corpus tests | Appropriate and covered. |
| R3 public reports and versioned serialization | Level 1 API, Serde, and malformed-record tests | Appropriate and covered. |
| R4 truncation warning without error | Level 1 public sync/async process-boundary tests | Appropriate and now covered. |
| R5 mono-only mpv upmix | Level 1 argument tests; historical real-player evidence | Correct L1 coverage; current real acceptance remains open under Finding 1. |
| R6 private, durable, serialized spool | Level 1 unit and multiprocess tests | Appropriate; GitNexus identifies this as the highest-blast-radius seam. |
| R7 spool display and background CLI | Level 1 CLI/process tests | Appropriate; the spec does not require exact terminal styling or geometry. |
| R8/R10/R14 dependency and native CI metadata | Static manifest checks and platform builds | Appropriate. |
| R9/R11 detached biscuit-speaks routing | Level 1 provider, job, failover, and DTO tests | Appropriate. |
| R12 background requester handoff | Level 1 on macOS; target-only check for the repaired Windows fixture | **Gap:** current Windows execution is missing (Finding 2). |
| R13/R16 documentation and skills | Static contract checks | Appropriate. |
| R15 Claudine handoff and phase ordering | Level 1 on macOS; target-only check for the repaired Windows fixture | **Gap:** current Windows execution is missing (Finding 2). |
| R17 unchanged notification recipes | Static contract test and manual publication evidence | Appropriate. |
| Audible sentence/player completion | Historical success; current required-sink runs time out or skip | **Gap:** current successful real-resource acceptance is missing (Finding 1). |

No requirement concerns keyboard, mouse, paste, IME, hotkeys, or terminal input
encoding, so Level 3 is not applicable. No requirement specifies exact terminal
glyphs, widths, SGR styling, or scrolling, so Level 2 is not required. The
relevant user-observable boundary is actual audio completion, which belongs in
the `real_` tier.

## Verification Run

- `git diff --check`: passed before the review document edits.
- `cd playa && just test -- truncated_verdict`: 2 passed, 189 skipped.
- GitNexus `detect_changes`: 48 changed symbols in 15 files, one affected
  execution flow, aggregate risk medium.
- GitNexus upstream impact: `enqueue_state_at` is high risk with three direct
  callers and the cross-package `Speak::play_detached` flow; the inspected
  report/delegate paths were low risk.
- Existing implementation evidence reports green full Level 1, lint, check,
  and doctest gates for Playa and Claudine, with biscuit-speaks' unrelated host
  `say` cases requiring a device-free fixture because this host's audio service
  is wedged.

## Production Readiness

Not production ready. The deterministic implementation and regression coverage
are strong, and the public-path truncation gap is closed, but production
readiness cannot be claimed until the current code completes the motivating
audio through a functioning real sink and the repaired requester-exit paths
execute successfully on Windows.
