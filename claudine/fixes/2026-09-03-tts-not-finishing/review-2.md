---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-04T01:54:49+01:00
spec: 2026-09-03-tts-not-finishing/spec.md
implemented: false
description: "A **fix** review of `2026-09-03-tts-not-finishing/spec.md`"
fix: 2026-09-03-tts-not-finishing/review-2.md
previous: 2026-09-03-tts-not-finishing/review-1.md
---

# Review 2: TTS Not Finishing

## Verdict

The fix is **not ready for production**. The implementation now has the broad
native/reporting, durable spool, detached TTS, and Claudine handoff surfaces
described by the specification, and the current Playa and Claudine Level 1
suites are green. Two protocol defects remain: a tripped native-audio circuit
breaker never reaches the promised host fallback, and version-1 spool records
accept missing executable identity even though that identity is required for
safe execution. The prescribed production-seam fallback regression was also
replaced by a reducer-only test, which is why the first defect is currently
green.

## Findings

### 1. High: native device failures cannot reach host fallback

The established contract says device-open timeouts and stream failures trip a
process-local breaker and subsequent playback routes to a host player
(`spec.md:190-198`). `play_native` does trip that breaker, but
`NativePlaybackError::should_fallback_to_host` permits fallback only for
unsupported format, URL, and decode errors
(`playa/lib/src/native_player.rs:242-250`). Consequently the initial
`DeviceOpenTimeout` is returned as `PlaybackError::AudioSubsystem`, and the
next call's `NativePlaybackDisabled` is also returned as a fatal error rather
than consulting a host player (`playa/lib/src/playa.rs:220-243`, `313-338`).
The sync and async public pipelines therefore contradict their own docs and
the spec's headless-host behavior.

Detached playback makes the defect deterministic across jobs. The scheduler
launches the recorded enqueuer executable separately for every ready job and
waits for its report (`playa/lib/src/detached/mod.rs:820-875`). Each delegate
starts with a fresh process-local breaker, so a host whose native device open
times out pays that timeout and fails every job; it never reaches a later call
in the same process that could use a host player. This affects the exact
Claudine and biscuit-speaks paths introduced by the fix.

The required Level 1 test would have exposed this. The spec calls for
`free_functions_take_native_route_when_available` to inject at the actual
playback boundary and prove both the native and post-breaker host routes
(`spec.md:642-653`). The implemented
`injected_backend_seam_covers_native_fallback_and_fatal_paths` instead passes
already-classified `NativeAttempt` values directly to the private
`resolve_native_attempt` reducer (`playa/lib/src/playa.rs:566-595`). It never
calls `play_native`, never classifies a concrete native error, and never calls
a free playback function. The real native test also treats every error or
non-native route as a clean skip unless `PLAYA_REAL_AUDIO_REQUIRED=1`
(`playa/lib/tests/real_playback_reports.rs:41-60`); the Phase 7 evidence records
this exact timeout as a skip.

**Required change:** define the retry boundary explicitly. Failures known to
occur before audio is submitted (including a disabled breaker and device-open
failure) can safely fall back in the same call. A post-submit stall must not be
silently retried if that could play audible content twice. Make the
process-local breaker actually route subsequent foreground calls to host, and
account for the one-job-per-delegate lifecycle so detached playback can use a
safe host fallback without relying on process-local state surviving re-exec.
Add the specified Level 1 injection seam through the public/free production
pipeline and assert concrete `DeviceOpenTimeout`, stream/open failure, and
`NativePlaybackDisabled` behavior for both sync and async paths.

Verification level present: Level 1 reducer tests plus a skippable real-device
test. Required level: Level 1 through the production playback boundary, plus a
required-sink real-audio run for the acceptance route. This is a coverage gap,
not evidence that fallback works.

### 2. High: v1 accepts a job without the required executable identity

The ratified v1 protocol says executable identity is required and must never
receive a Serde default; a missing required field must quarantine the record
(`spec.md:432-464`). `JobEnvelope` nevertheless models
`enqueuer_fingerprint` as a defaulted `Option<u64>`
(`playa/lib/src/detached/protocol.rs:252-265`). Deserialization therefore
accepts a structurally incomplete v1 record, only for `delegate_job` to reject
it later as a runtime protocol error (`playa/lib/src/detached/mod.rs:825-834`).
`DelegatedReport` repeats the same issue for `delegate_fingerprint`
(`playa/lib/src/detached/protocol.rs:414-423`) even though the table requires
delegate identity.

The checked-in `fixtures/v1-ready-job.json` omits the fingerprint. Its
execution-path regression does not reveal the incompatibility because the
fixture timestamp is stale: the scheduler discards it before invoking the
delegate, and the test explicitly asserts zero delegate calls
(`playa/lib/src/detached/tests.rs:430-453`). Thus the evidence claim that the
shipped ready artifact runs through the normal path establishes stale cleanup,
not that a shipped v1 ready record remains executable.

**Required change:** make all ratified v1 executable-identity fields required
at deserialization, or explicitly revise/version the wire contract and supply
a migration policy. Update the shipped corpus to contain a complete v1
identity projection, add a missing-identity quarantine test, and add a
non-stale fixture that reaches delegation with a valid executable/fingerprint
pair rather than terminating at stale-record handling.

Verification level present: Level 1 deserialization and stale-policy tests.
Required level: Level 1 malformed-record quarantine and non-stale delegation
through the production scheduler seam. Level 1 is the correct tier, but the
current assertions do not exercise the required behavior.

### 3. Medium: the background TTS process regression is Unix-only

The required `so-you-say --background` regression is a process-level test that
proves the requester returns before blocked synthesis/playback and that the
same reserved slot completes. The implementation covers that behavior well on
Unix, but the entire test target is disabled with `#![cfg(unix)]`
(`biscuit-speaks/cli/tests/detached_background.rs:1-2`). Windows has unit and
cross-build coverage for SAPI/job construction and Playa detach flags, but no
equivalent CLI-process regression for the user-facing background path. The
same limitation applies to Claudine's strongest default-emitter ordering and
non-blocking test, which is guarded by `#[cfg(unix)]`
(`claudine/lib/src/composition/lifecycle/tests/audio_emission.rs:66-69`).

**Required change:** add Windows-capable fixture executables or a
platform-neutral test-worker mode so the `so-you-say` requester-exit path and
Claudine default-emitter handoff/order run as Level 1 process tests on Windows.
Do not weaken the behavior to a job-shape assertion; the important contract is
that the real caller exits while downstream work remains blocked.

Verification level present: Level 1 process execution on Unix and Level 1
unit/cross-build checks on Windows. Required level: Level 1 process execution
on Windows. No Level 2 or Level 3 terminal test is applicable.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 free functions use native-first routing | Level 1 reducer and real-audio test | **Gap:** no production-boundary injection; concrete breaker errors do not fall back. |
| R2 metadata probing | Level 1 unit/integration fixtures | Appropriate level and covered. |
| R3 public reports and stable journal projection | Level 1 API/Serde tests | Appropriate level and covered. |
| R4 one truncation warning, still `Ok` | Level 1 direct report-warning tests and real host playback | **Gap:** no specified short-success fake host process through a public playback API. |
| R5 conditional mpv mono upmix | Level 1 argument tests and real mpv test | Appropriate levels; the latest real run timed out rather than producing a current completion verdict. |
| R6 durable private serialized spool | Level 1 unit, multiprocess, and CLI tests | Correct tier, but executable-identity v1 coverage is invalid as described in Finding 2. |
| R7 spool display and background CLI | Level 1 CLI capture/process tests | Appropriate. No exact terminal-emulator rendering or input behavior is specified, so Level 2/3 is not required. |
| R8 Playa docs/dependency records | Static/source contract checks | Appropriate. |
| R9 biscuit-speaks native builder route | Level 1 compile/API tests and real provider playback | Appropriate tier; real audio was unavailable in the latest evidence run. |
| R10 Linux native dependency metadata | Manifest checks and cross-platform build evidence | Appropriate. |
| R11 detached APIs for concrete providers | Level 1 provider/job/failover tests | Appropriate for projection; Windows requester behavior remains part of Finding 3. |
| R12 `so-you-say --background` handoff | Level 1 Unix CLI process test | **Gap on Windows:** only lower-fidelity unit/cross-build evidence is present. |
| R13 biscuit-speaks docs/skills | Static/source contract checks | Appropriate. |
| R14 Claudine native feature/CI metadata | Feature/manifest checks and cross-platform builds | Appropriate. |
| R15 lifecycle and dispatch handoff/order | Level 1 unit/process tests, strongest default-emitter case Unix-only | **Gap on Windows:** no equivalent production-emitter process proof. |
| R16 Claudine docs/skills | Static/source contract checks | Appropriate. |
| R17 unchanged `just` notification recipes | Static contract checks plus manual command evidence | Appropriate. |

The Level 1/2/3 terminal hierarchy is not the real-device hierarchy. This fix
has no keyboard, paste, IME, mouse, hotkey, or terminal-input encoding
requirement, and it specifies no exact terminal glyph/width/style acceptance
that needs a real emulator. Accordingly, no Level 2 or Level 3 test is
required. Audible completion and real player/device routing instead require
the separate `real_` tier defined by the spec; those acceptance runs must not
go green through a skip on a host designated to provide an audio sink.

## Previous Review Status

The referenced previous review could not be inspected or updated because
`prompts/_reviews/claudine/fixes/2026-09-03-tts-not-finishing/review-1.md` does
not exist in this worktree, any other registered worktree, or repository
history. This review therefore independently assessed the complete spec and
implementation rather than claiming closure of review-1 findings. Its
frontmatter retains the requested `previous` relationship so the missing
artifact can be restored without changing this review's identity.

## Verification Run

- `cd playa && just test`: 160 passed; 8 tier-gated skips.
- `cd biscuit-speaks && just test`: 471 passed; 7 macOS host `say` discovery
  or cache tests timed out; 19 tier-gated skips. The implementation evidence
  records a device-free fixture rerun of 478/478.
- `cd claudine && just test`: 6,726 passed; 11 tier-gated skips.
- The implementation evidence records green lint/check/doctest gates and
  native Windows suites. Its Linux retry did not reach a Rust verdict, and
  its latest real-audio runs could not establish completed native/mpv or
  end-to-end TTS playback because the host audio services were unavailable.

## Production Readiness

Not production ready. Restore safe native-to-host behavior for foreground and
per-job delegate processes, enforce the ratified v1 executable identity at the
wire boundary, replace the false-green seams with production-path tests, and
add the missing Windows process regressions before the next review.
