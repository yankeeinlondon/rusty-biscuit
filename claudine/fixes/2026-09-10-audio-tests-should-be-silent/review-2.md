---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-10T15:54:24-07:00
spec: 2026-09-10-audio-tests-should-be-silent/spec.md
implemented: false
description: A **fix** review of `2026-09-10-audio-tests-should-be-silent/spec.md`
fix: 2026-09-10-audio-tests-should-be-silent/review-2.md
previous: 2026-09-10-audio-tests-should-be-silent/review-1.md
findings:
    - "High — Cleanup releases queue ownership before worker ownership"
---

# Review 2: Audio Tests Should Be Silent

## Verdict

The fix is **not ready for production**. Review 1's native-gain and
real-resource reachability findings are closed: the native player is observed
after gain is applied and before submission on both output paths, and the
canonical real tier now reaches and successfully executes the muted
EchoGarden/gTTS cases with provider requirements enforced.

The publication-cleanup finding is only partially closed. The shared fixture
now removes pending records while holding `queue.lock`, but its destructor
releases that lock before releasing `worker.lock`. A publisher can commit in
that interval, observe the fixture as the apparent active worker, and leave a
runnable record behind when the fixture subsequently releases worker
ownership. This is the exact lock-handoff condition the production scheduler
avoids.

No Level 2 or Level 3 terminal test is required. The specification introduces
no terminal-rendering or OS-keyboard behavior. Its additional boundary is the
real-resource tier for synthesis and playback.

## Finding

### High — Cleanup releases queue ownership before worker ownership

`LockedAudioSpool::clear_pending` unlocks `queue.lock` at
`tools/test-toolkit/src/spool.rs:149`, then `Drop::drop` returns and only
afterward allows the `_worker` field to release `worker.lock`
(`tools/test-toolkit/src/spool.rs:154-162`). That creates a publication window:

1. cleanup scans and removes pending records;
2. cleanup releases `queue.lock` while the fixture still owns `worker.lock`;
3. a publisher acquires `queue.lock`, commits a new pending record, and probes
   `worker.lock`;
4. the probe sees the fixture's lock, so the publisher correctly assumes an
   active worker owns the record and does not spawn another scheduler;
5. the fixture releases `worker.lock` without another scan.

The record is then executable by a later scheduler. The production scheduler's
final-empty handoff deliberately does the opposite: it holds `queue.lock`,
releases `worker.lock`, and only then releases `queue.lock`
(`playa/lib/src/detached/mod.rs:746-758`).

The new synchronized regressions do not cover this window. Their publisher
already owns `queue.lock` before fixture cleanup starts
(`tools/test-toolkit/tests/audio_spool.rs:82-100` and
`playa/cli/tests/detached_cli.rs:177-195`). They prove cleanup waits for a
publication already in progress, but not that no publication can commit between
the final scan and worker release.

The same destructor also still suppresses `clear_pending` errors while a test
is unwinding (`tools/test-toolkit/src/spool.rs:156-161`). Its documentation says
the error is discarded, while the implementation log claims it is reported.
If cleanup fails, the guard can therefore silently release worker ownership
over a runnable record, contrary to Required Behavior 6 and Review 1's explicit
requirement to report cleanup failure without double-panicking.

Required change: make destruction perform one atomic handoff matching
`run_scheduler_with`: acquire `queue.lock`, remove pending records, release
`worker.lock` while `queue.lock` remains held, then release `queue.lock`.
Represent worker ownership so it can be explicitly released in that order.
Report cleanup errors during unwind without initiating a second panic, and do
not silently make uncleared work executable. Add a deterministic regression in
the CI-reachable Playa test target where the publisher is blocked acquiring
`queue.lock` until after the cleanup scan; prove it either becomes owned by a
new scheduler or cannot commit until worker ownership has been released safely.

Verification level present: Level 1 covers a publisher that held `queue.lock`
before cleanup. Required level: Level 1 synchronized coverage of the
cleanup-to-worker-release handoff. L2/L3 is not applicable.

## Review 1 Closure

### Native gain boundary

Closed. `submit_to_mixer` is shared by the cached-default and channel-specific
native paths. Six Level 1 tests use a device-free rodio mixer and read back the
real player's volume, speed, and queued-source count. Both mute cases establish
`volume == 0.0` while `queued == 0`, proving gain precedes submission rather
than merely echoing `PlaybackOptions`.

### EchoGarden and gTTS real-tier reachability

Closed. The six formerly ignored tests now have canonical `real_` names and no
`#[ignore]`. `just test-real` selects them with the `playa` feature, and
`BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts` turns resource absence into
a hard failure. This review executed that required-provider tier: all eight
library real tests passed, including the three EchoGarden and three gTTS tests;
the Biscuit Speaks CLI real test also passed.

## Requirement Verification Levels

"Real" below is the external-resource tier in addition to the prompt's
terminal-focused Level 1/2/3 classification.

| Required behavior | Strongest verification | Assessment |
| --- | --- | --- |
| 1. Real playback is effectively muted; non-audio tests publish nothing | L1 provider/player and dry-run assertions; Real EchoGarden, gTTS, Kokoro, and Say completion | Appropriate, subject to the cleanup handoff defect. |
| 2. Argument, routing, serialization, and failure tests cannot reach host output | L1 fake executables, guarded environment tests, and typed player errors | Appropriate. |
| 3. Silence survives detached preparation, delegation, and fallback | L1 multi-process handoff and fallback capture; Real provider routes | **Gap:** the final detached cleanup handoff can strand runnable work. |
| 4. Speech fixtures identify themselves as tests | L1 captured text plus source inventory | Appropriate; production announcements remain unchanged. |
| 5. Real synthesis/playback and completion remain meaningful | Real tier, 8 library tests and 1 CLI test executed | Appropriate. |
| 6. No executable job escapes completion, failure, panic, or cleanup | L1 normal, unwind, timeout, and concurrent-publication tests | **Gap:** no test covers publication after the cleanup scan but before worker release; unwind cleanup errors are silent. |
| 7. Application defaults and legitimate announcements remain unchanged | L1 defaults/template tests plus source diff | Appropriate. |
| 8. Test-specific providers and voices are documented | Documentation plus L1 pinned-provider assertions | Appropriate. |
| 9. Native and fallback output receive mute | L1 real-player gain observation and host-command capture; Real completion | Appropriate. |
| 10. First-class TTS volume works in foreground and detached paths | L1 provider/delegation assertions; Real provider completion | Appropriate. |
| 11. Synthesis cache identity preserves voice/rate semantics and excludes playback-only volume | L1 cache-identity regressions | Appropriate. |

## Verification Performed

| Command | Result | Evidence |
| --- | --- | --- |
| `cd playa && just test` | 194 passed; 8 tier-filtered | Level 1 native gain, fallback, queue, and Playa CLI behavior |
| `cd biscuit-speaks && just test -j 2` | 491 passed; 19 tier-filtered | Level 1 provider, detached, cache, and CLI behavior |
| `cd claudine && just test audio` | 22 passed; 6,849 name-filtered | Level 1 lifecycle/dispatch audio publication consumers |
| `cargo nextest run -p test-toolkit -E 'binary(audio_spool)'` | 2 passed | Existing shared-fixture unwind and pre-held-queue regressions |
| `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts just test-real --success-output immediate` | 8 library tests and 1 CLI test passed; required providers did not skip | Real EchoGarden, gTTS, Kokoro, Say, and CLI resource execution |

The green tests do not invalidate the finding: none schedules a publisher in
the lock-release interval described above. Cross-OS result collection remains
CI/CD work and does not affect this readiness verdict.

## Production Readiness

Not production ready. Preserve `queue.lock` through the release of
`worker.lock`, report cleanup failure during unwind, and add the missing
synchronized Level 1 regression. Human review is not required for this
mechanical lock-lifetime correction.
