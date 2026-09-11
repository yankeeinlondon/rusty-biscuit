---
created: 2026-09-10
status: in_progress
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-10
implemented: false
review_iterations: 3
area: claudine
areas:
    - biscuit-speaks
    - playa
    - claudine
---

# Make Audio Tests Inaudible and Clearly Identify Test Speech

## Problem

Recent test runs have produced audible TTS messages and sound effects on the
host. Speech resembling “the review findings ... were implemented successfully”
can be mistaken for a real update about ongoing work.

The user also reports that the audible test voice, and apparently its underlying
TTS engine, differs from the voice/engine used during normal Claudine operation.
That difference initially made the source confusing. Provider-specific coverage
may legitimately use a different engine; matching the user's configured voice
is not a requirement of this fix.

Tests may exercise actual synthesis and playback, but their output must be
inaudible. Speech fixtures must identify themselves as tests, for example,
“This is a test message.” This applies to foreground playback, detached workers,
and fallback providers, including work that continues after the test parent exits.

The initial inventory below came from source and Git history inspection,
without replaying potentially audible tests. Subsequent source tracing
identified the exact test path for the reported announcement. Implementation
findings and validation are tracked in [evidence.md](./evidence.md).

> **Reader's note (inline review, 2026-09-10):** Silence is enforced at three
> different boundaries because no single switch preserves every test contract.
> Tests unrelated to audio use child-local dry-run and assert that no job was
> published; publication tests use a private, locked spool whose pending work is
> removed before worker ownership is released; real-resource tests reach the
> backend with explicit zero volume. Treating these as interchangeable would
> either discard meaningful coverage or leave detached work able to escape.

## Findings

Paths in this section are relative to the repository root.

### Confirmed real playback without a mute setting

`biscuit-speaks/lib/tests/real_detached_phase4.rs` contains
`real_default_provider_reports_native_complete`. It selects Kokoro and calls
`Speak::new("Phase four playback is complete.").play_with_result()` without a
volume override. It can synthesize and play on a configured host. A missing
provider or `PLAYA_DRY_RUN` can skip the test, but neither guarantees silence
when the real-resource test actually executes.

This file was introduced by `e372ba4e1` on 2026-09-04. The area's `just test-real`
enables the `playa` feature. This is a confirmed audible-capable path, not proof
that it caused the particular announcement reported by the user.

### Misleading speech fixtures

The following files contain the fixture “Phase 1 of the plan in the claudine
package area, was implemented successfully”:

| File | Relevant coverage |
|------|-------------------|
| `biscuit-speaks/lib/tests/detached_phase4.rs` | eSpeak detached argument preservation |
| `biscuit-speaks/cli/tests/detached_background.rs` | Background cache miss and reservation completion |
| `playa/lib/src/detached/tests.rs` | Preparing-head queue ordering |
| `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs` | Lifecycle publication order and failed speech handoff |
| `claudine/lib/src/dispatch/runner/tests.rs` | Hook publication order and failed speech handoff |

Several of these tests only construct jobs, use stub programs, or hold a worker
lock; the wording alone does not establish that they emit sound. Replace it
while preserving punctuation, Unicode, argument-boundary, or ordering coverage
where those properties are part of the test contract.

The exact review-findings announcement also exists as a legitimate lifecycle
`say` template in `prompts/_implement/implement-suggestions.md`. Source tracing
established that two tests in
`claudine/cli/tests/compose_caller_file_provenance.rs` copy and execute that
literal template with a fake Goose provider. The fake provider isolates agent
execution, but it does not suppress the template's independent lifecycle audio.
Those provenance tests must retain the shipped template, set child-only
`PLAYA_DRY_RUN=1`, use a private spool, and assert that the spool is never
created. The production announcement is legitimate and must not be changed.

### Test provider selection differs from normal Claudine usage

The real playback test explicitly sets `TTS_PROVIDER=kokoro`. The Claudine
lifecycle and dispatch publication tests explicitly configure eSpeak, with
fixture executables intended to prevent real synthesis. Other detached tests
inspect provider-specific voice arguments such as `en+f3` or `Samantha`.
These choices can explain a difference from the user's normal voice/engine,
but the engine responsible for the reported audible announcement has not been
identified conclusively. The user's actual Claudine TTS configuration was not
inspected during this investigation.

Keep explicit provider selection where it serves the test contract. Test names,
fixture comments, or test documentation should make that selection clear rather
than implying that a pinned engine represents the user's configured default.

### Detached sound-effect containment needs verification

`claudine/cli/tests/detached_audio.rs` queues `doorbell-2` without an explicit
volume, removes `PLAYA_DRY_RUN`, and holds `worker.lock` while inspecting the
durable job. The local worker handle is dropped before the enclosing fixture.
Confirm that a detached scheduler cannot consume the job during teardown.

The Claudine lifecycle publication test similarly queues effects in a private
spool while holding the worker lock. The dispatch publication test explicitly
uses effect volume `0.5`. Both rely on containment rather than muted payloads.
These are audit targets, not reproduced teardown races. Failure-handoff tests
use an invalid spool path and should fail before any playback begins.

`biscuit-speaks/cli/tests/detached_background.rs` supplies fake Kokoro and mpv
executables. Preserve that isolation and verify fallback cannot reach a real
player. A private spool by itself does not mute playback.

### Existing silent playback coverage

`playa/lib/tests/real_playback_reports.rs` uses zero-filled PCM WAV fixtures for
real native and host playback. Its stereo control also passes volume `0.0`.
These tests illustrate how to retain completion and timing coverage without
audible content; they are not identified as sources of the reported sound.

## Required Behavior

1. Every automated test that reaches a real speech or audio playback boundary
   must apply effective zero volume before playback starts. Low or “soft” volume
   is insufficient. Tests that do not cover audio behavior must instead use a
   child-scoped dry-run boundary and prove that no durable job was published.
   Never change the host's global volume or mute setting.
2. Keep tests of argument construction, routing, serialization, and failures
   isolated from real output. Tests that intentionally inspect nonzero volume
   values may retain them only when their execution is conclusively stubbed or
   cannot reach playback. Environment-based dry-run setup must use the
   repository's guarded, serialized environment-test conventions; do not mutate
   process-global environment concurrently.
3. Carry silence through detached publication, preparation, delegation, and
   provider/player fallback. Verify direct streaming speech separately from
   file playback; a file-player volume option does not establish that a speech
   command is muted. Automatic player selection must exclude players that cannot
   honor a requested volume, while an explicitly selected incapable player must
   fail with a typed unsupported-volume error before spawning. A test may use a
   silent fixture or controlled backend instead when volume behavior is not its
   subject.
4. Replace work-status speech fixtures with explicit test messages. Include a
   recognizable “test message” phrase even in fixtures that retain extra text
   for escaping, Unicode, or argument-preservation assertions.
5. Keep real-resource tests meaningful: successful synthesis, actual playback
   completion, route selection, queue ordering, and timing assertions must
   remain exercised where required. Blanket dry-run or skipping is not a
   substitute for muted real playback coverage.
6. Tests must not leave executable audio jobs or workers that can become audible
   after completion, failure, panic, or fixture cleanup. Publication fixtures
   must remove runnable records while still holding both the worker-exclusion
   and queue-mutation locks. Tests that launch fixture-owned helpers must observe
   normal completion or terminate only those helpers after validating process
   identity; name-wide process termination is prohibited.
7. Preserve normal application volume defaults and legitimate lifecycle
   announcements. Do not introduce behavior that mutes ordinary user sessions.
8. Document intentional test-specific provider/voice selection. Do not change
   tests to inherit personal Claudine configuration merely to make their voices
   match; keep provider coverage reproducible and silent.
9. Use Playa with its native playback feature for the affected consumers.
   Verify volume at the native output boundary and through any fallback path;
   an enabled feature does not guarantee an available device or successful
   native playback.
10. TTS volume must be a first-class, effective library control. The existing
    `Speak::with_volume`, `TtsConfig::with_volume`, and `VolumeLevel` API already
    provides the public setting; fix provider paths that ignore it rather than
    adding a duplicate API or relying solely on test stubs. Zero must mean mute.
    Preserve configured provider/voice selection when adding volume support.
11. Any synthesis-to-file route introduced to make volume effective must
    preserve synthesis cache correctness. Cache identity must include resolved
    synthesis-affecting settings such as voice and rate, must not conflate an
    unknown system default with an explicit rate, and must not include volume
    when volume is applied only during playback.

## Scope and Implementation Constraints

Audit automated speech/effect call sites and shared fixtures across
`biscuit-speaks`, `playa`, and `claudine`, starting with the findings above and
the September 4 detached-audio changes. Expand to other packages only where a
caller or shared test helper exposes the same defect. Record which paths were
confirmed, already isolated, or corrected.

Prefer existing `VolumeLevel::Explicit(0.0)` and playback volume configuration
when their propagation is verified. Do not assume all providers honor those
settings. Introduce a shared testing mechanism only if existing controls cannot
cover the affected paths cleanly. The user's planning clarification explicitly
includes repairing ineffective TTS volume support, including production provider
paths where necessary. Preserve default configuration values while making an
explicit volume request effective. Run GitNexus impact analysis before editing
symbols and inspect comments for drift alongside each behavior change.

The solution must work on macOS, Linux, native Windows, and WSL2. Load the `os`
skill before platform-specific implementation or test planning. Update relevant
testing documentation and skills if a new shared test convention is introduced.

## Design Decisions and Test Boundaries

The audit is bounded to executable automated-test paths in `biscuit-speaks`,
`playa`, and `claudine`, plus shared helpers and directly affected callers found
during tracing. Text matches alone are inventory leads: a speech-like string in
a serialization-only or fully stubbed test does not require real playback.
Conversely, a private spool alone is not containment because another scheduler
can consume a runnable record after the lock is released.

Use the narrowest boundary that retains the behavior under test:

| Test purpose | Required boundary | Required proof |
|--------------|-------------------|----------------|
| Composition, provenance, or another non-audio behavior that executes lifecycle actions | Child-only `PLAYA_DRY_RUN=1` and a private spool | The command completes and the spool is never created |
| Durable publication, ordering, or handoff | Private spool held by the shared locked-spool fixture | Payload assertions pass and pending records are removed under lock, including while unwinding |
| Provider/player argument construction | Fake executable or injected backend with no path to host playback | Captured arguments preserve the expected zero or intentional nonzero value |
| Real synthesis/playback completion | Pinned provider and voice with explicit zero volume | Completion plus provider/player-boundary evidence that zero was honored |

Real-resource availability differs by operating system and host. Compatibility
requires compilation and deterministic tests on macOS, Linux, native Windows,
and WSL2. Real-backend checks run where the named dependency and audio backend
are available; an explicit, recorded skip is acceptable elsewhere, but a skip
must not be reported as playback evidence. Validation records must distinguish
passes, controlled-backend checks, explicit skips, and infrastructure failures.

No open design question remains for this fix. A future request to make all test
processes globally dry-run by default would change the real-resource and durable
publication contracts and therefore requires a separate specification.

## Acceptance Criteria and Validation

- [x] Record the affected tests and trace any test execution of real lifecycle
  templates, distinguishing the user's reported symptom from confirmed causes.
- [x] Real TTS and non-silent sound-effect tests request effective zero volume;
  automated speech fixtures clearly identify themselves as test messages.
- [x] Intentional differences between test providers/voices and normal Claudine
  configuration are documented, including the real test's pinned Kokoro engine.
- [x] Verify mute propagation at the actual provider/player boundary, including
  detached children and supported fallback routes. Test an unsupported-volume
  route without allowing it to emit sound.
- [x] Verify automatic selection rejects volume-incapable players and explicit
  selection returns the typed unsupported-volume failure before spawning.
- [x] Affected consumers enable native Playa playback, and first-class TTS
  volume is honored in foreground and detached provider paths. Exercise zero
  and representative nonzero levels without changing host-wide audio settings.
- [x] Preserve relevant synthesis, completion-report, durable-publication,
  ordering, and failure assertions; assertions against nonzero configuration
  remain safely isolated.
- [x] Exercise teardown and failure paths to establish that no pending job can
  escape containment after its parent test ends.
- [x] Preserve cache identity across any new synthesis-to-file path, including
  distinct unknown-default and explicit-rate cases and reuse for equivalent
  resolved rates.
- [x] Run the affected areas' canonical `just test` and applicable `just test-l2`
  recipes, plus `just test-real` for the affected real-resource coverage after
  muting is verified. Use nextest through repository recipes, not `cargo test`.
- [ ] Record compilation and deterministic-test evidence for macOS, Linux,
  native Windows, and WSL2. Record which real backends were exercised and any
  explicit skips without treating skips or infrastructure failures as passes.
  L2/L3 checks must not give terminal or browser windows focus.
- [x] No application volume defaults, production announcement templates, or
  host-wide audio settings change as a side effect of this fix.

Do not use listening alone as proof of silence: muted speakers or unavailable
hardware can hide a regression. Combine assertions about effective playback
parameters or silent samples with completion evidence on available real backends.
