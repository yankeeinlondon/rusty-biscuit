---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-10T16:44:42-07:00
spec: 2026-09-10-audio-tests-should-be-silent/spec.md
implemented: false
description: A **fix** review of `2026-09-10-audio-tests-should-be-silent/spec.md`
fix: 2026-09-10-audio-tests-should-be-silent/review-3.md
previous: 2026-09-10-audio-tests-should-be-silent/review-2.md
---

# Review 3: Audio Tests Should Be Silent

## Verdict

The fix is **ready for production**. Review 2's remaining lock-handoff defect is
closed: `LockedAudioSpool` now removes runnable records and releases
`worker.lock` within one `queue.lock` critical section, matching Playa's
production scheduler handoff. If cleanup fails, the fixture reports the error
and retains worker ownership rather than exposing a record it could not remove.

No Level 2 or Level 3 terminal test is required. The specification introduces
no terminal-rendering or OS-keyboard behavior. Its additional boundary is the
real-resource tier for synthesis and playback, whose muted provider coverage
was executed in Review 2.

## Findings

None.

## Review 2 Closure

### Cleanup handoff ordering

Closed. Destruction acquires `queue.lock`, scans and removes runnable records,
then explicitly releases `worker.lock` before releasing `queue.lock`. A
publisher can therefore either commit before the scan while the fixture owns
the worker, or observe worker ownership as free only after the cleaned queue is
available; it cannot commit behind the final scan and defer its record to a
worker that is about to disappear.

The new Level 1 regression installs an observer after the final scan and has a
publisher probe `queue.lock` while the fixture still owns both locks. The probe
must fail. The test then verifies that the spool is empty and both locks are
available after destruction. This regression is present in the shared toolkit
and in Playa's CI-reachable CLI test target.

### Cleanup failure during unwind

Closed. A cleanup error is written to stderr, the original panic remains the
propagated panic, and the worker handle is deliberately retained so surviving
work cannot become runnable. The focused test run observed the diagnostic and
confirmed that no second panic replaced the original failure.

## Requirement Verification Levels

“Real” below is the external-resource tier in addition to the prompt's
terminal-focused Level 1/2/3 classification.

| Required behavior | Strongest verification | Assessment |
| --- | --- | --- |
| 1. Real playback is effectively muted; non-audio tests publish nothing | L1 provider/player and dry-run assertions; Real EchoGarden, gTTS, Kokoro, and Say completion | Appropriate. |
| 2. Argument, routing, serialization, and failure tests cannot reach host output | L1 fake executables, guarded environment tests, and typed player errors | Appropriate. |
| 3. Silence survives detached preparation, delegation, and fallback | L1 multi-process handoff, fallback capture, and synchronized final-handoff regression; Real provider routes | Appropriate. |
| 4. Speech fixtures identify themselves as tests | L1 captured text plus source inventory | Appropriate. |
| 5. Real synthesis/playback and completion remain meaningful | Real tier with required-provider execution recorded in Review 2 | Appropriate. |
| 6. No executable job escapes completion, failure, panic, or cleanup | L1 normal, unwind, timeout, concurrent-publication, final-handoff, and cleanup-failure tests | Appropriate. |
| 7. Application defaults and legitimate announcements remain unchanged | L1 defaults/template tests plus source diff | Appropriate. |
| 8. Test-specific providers and voices are documented | Documentation plus L1 pinned-provider assertions | Appropriate. |
| 9. Native and fallback output receive mute | L1 native-player gain observation and host-command capture; Real completion | Appropriate. |
| 10. First-class TTS volume works in foreground and detached paths | L1 provider/delegation assertions; Real provider completion | Appropriate. |
| 11. Synthesis cache identity preserves voice/rate semantics and excludes playback-only volume | L1 cache-identity regressions | Appropriate. |

## Verification Performed

| Command | Result | Evidence |
| --- | --- | --- |
| `cd tools && just test` | 129 passed; 2 skipped | Level 1 shared-fixture handoff, unwind, and failed-cleanup behavior |
| `cd playa && just test` | 195 passed; 8 tier-filtered | Level 1 native gain, fallback, queue, and CI-reachable handoff behavior |
| `cd tools && just lint` | Passed | Shared fixture and regression lint gate |
| `cd playa && just lint` | Passed | Playa library and CLI lint gate |
| Focused Nextest run with successful output enabled | 1 passed; cleanup diagnostic present on stderr and original panic preserved | Unwind error reporting at the test-process boundary |

Review 2 already executed the required-provider real tier: eight Biscuit
Speaks library tests and one CLI test passed with EchoGarden and gTTS required,
alongside Kokoro and Say. Replaying synthesized speech was unnecessary for this
lock-order-only iteration. Cross-OS result collection remains CI/CD work and
does not affect readiness under the review contract.

## Production Readiness

Production ready. All prior findings are closed, each user-observable
requirement has the appropriate verification boundary, and no human-only design
or validation decision remains.
