---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-10-audio-tests-should-be-silent/spec.md`
fix: 2026-09-10-audio-tests-should-be-silent/review-1.md
spec: 2026-09-10-audio-tests-should-be-silent/spec.md
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-10T14:33:10-07:00
implemented: false
ready: false
human_review: false
findings:
    - "High — Publication cleanup bypasses queue ownership and can suppress failure"
    - "High — Native mute is not verified at the output gain boundary"
    - "High — Muted EchoGarden and gTTS playback tests are unreachable from the real-resource tier"
---

# Review 1: Audio Tests Should Be Silent

## Verdict

The fix is **not ready for production**. The implementation correctly repairs
the main production paths: eSpeak now receives explicit amplitude, macOS Say
synthesizes to a file before volume-controlled playback, Say cache identity
separates unresolved defaults from resolved voice/rate settings, Playa rejects
explicit volume on incapable players, native-to-host fallback retains mute,
and the reported Claudine template path now runs with child-only dry-run and
proves that it publishes no spool.

Three safety-verification gaps remain. Two test cleanup guards delete durable
records outside the queue's mutation lock, the real native tests do not prove
that zero reaches the native player's gain control, and the changed
EchoGarden/gTTS playback tests are still ignored tests that no canonical recipe
can execute. These gaps matter because a green test can otherwise leave work
that becomes audible or can pass while the native/provider boundary ignores
mute.

No Level 2 or Level 3 terminal test is required. The specification has no
terminal-rendering or OS-keyboard behavior; its extra boundary is the
repository's `real_` resource tier, which is separate from the terminal
L1/L2/L3 hierarchy.

## Verification performed

| Command | Result | What it establishes |
| --- | --- | --- |
| `cd playa && just test` | 188 passed; 8 tier-filtered | L1 player selection, typed unsupported-volume errors, fallback argument propagation, and detached queue behavior |
| `cd biscuit-speaks && just test -j 2` | 487 passed; 19 tier-filtered | L1 provider arguments, Say file bridge/cache identity, detached handoff, forced fixture cleanup, and dry-run containment |
| `cd claudine && just test --no-fail-fast` | 6,861 passed; 11 tier-filtered | L1 shipped-template dry-run/no-publication behavior and Claudine locked-spool publication tests |
| `cd playa && just test-real --success-output immediate` | 4 passed; no resource skips | Real native and installed host-player completion using zero-filled PCM; explicit zero-volume host completion |
| `just _test_real biscuit-speaks --features playa --success-output immediate` | 2 passed; no resource skips | Real Kokoro and Say synthesis followed by muted native completion |

GitNexus was current at `d76e90e81`. Upstream analysis of
`build_player_command` reported CRITICAL blast radius: 27 direct dependents, 37
total affected symbols, and eight affected modules. The broad surface is
consistent with the public and builder-path coverage run above; it also makes
the boundary-specific gap in Finding 2 important.

## Requirement verification levels

“Real” below means an opt-in/external-resource test in addition to the prompt's
terminal-focused L1/L2/L3 classification.

| Required behavior | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Real playback is effectively zero; non-audio tests dry-run and publish nothing | L1 fake-provider/player arguments; real Kokoro/Say and zero-PCM completion | **Gap:** native completion does not observe the gain applied at the native sink (Finding 2). The Claudine provenance path is adequately verified at L1. |
| 2. Argument/routing/serialization/failure tests cannot reach real output; environment changes are guarded | L1 serialized `EnvGuard` tests and fixture-only executables | Adequate. Intentional nonzero values stay behind invalid audio and a fixture-only player path. |
| 3. Silence survives detached preparation/delegation/fallback; incapable players are excluded or fail typed | L1 multi-process detached test, player-command tests, and native-failure fallback tests | Correct level and broadly covered. Cleanup ownership remains incomplete under failure (Finding 1). |
| 4. Work-status speech fixtures become recognizable test messages | L1 fixture assertions plus source inventory | Adequate for executable paths inspected in scope. Production lifecycle templates remain unchanged. |
| 5. Real synthesis/playback, completion, routing, ordering, and timing remain meaningful | L1 controlled process tests plus real Kokoro/Say/Playa completion | Adequate for Kokoro, Say, and Playa. EchoGarden/gTTS are not reachable through `test-real` (Finding 3). |
| 6. No executable jobs/workers escape completion, failure, panic, or cleanup | L1 normal, unwind, and forced-timeout cleanup tests | **Gap:** two publication guards do not take `queue.lock`, and one discards cleanup errors (Finding 1). |
| 7. Production defaults and announcements remain unchanged | Source diff plus L1 shipped-template tests | Adequate. The implementation changes explicit volume handling without changing `VolumeLevel::Normal` or template text. |
| 8. Test-specific provider/voice selection is documented | Static inventory/docs plus L1 pinned fixture assertions | Adequate. Kokoro/`af_heart`, Say/Samantha, and eSpeak fixture selection are explicit. |
| 9. Affected consumers enable native Playa; volume is verified through native and fallback output | L1 feature assertion and fallback command capture; real native completion | **Gap at native gain application** (Finding 2). Host fallback proof is adequate. |
| 10. First-class TTS volume is effective in foreground and detached provider paths | L1 eSpeak, Say, SAPI/job, cached-file, and delegated-player assertions; real Kokoro/Say | Appropriate levels for covered providers. The unreachable real provider tests are Finding 3. |
| 11. Say synthesis cache includes resolved synthesis settings, distinguishes defaults, and excludes playback-only volume | L1 cache-path and synthesized-WPM regressions | Adequate. Unknown default rate does not reuse explicit 175 WPM; equivalent resolved rates reuse identity; playback volume is absent from the key. |

## Findings

### 1. High — Publication cleanup bypasses queue ownership and can suppress failure

The detached protocol serializes every publication/replacement under
`queue.lock`, but two new cleanup guards scan and delete `*.pending.json`
without taking that lock:

- `playa/cli/tests/detached_cli.rs:61-82` (`PublicationGuard`)
- `biscuit-speaks/lib/tests/detached_phase4.rs:321-332`
  (`PendingJobsGuard`)

Both hold `worker.lock`, which prevents scheduler execution, but it does not
prevent a publisher or preparation helper from committing/replacing a record.
A cleanup scan can therefore miss a record published just after its directory
iteration, or race a preparation replacement. Once the guard releases
`worker.lock`, that surviving record is executable. `PublicationGuard::drop`
also discards `clear_pending()` errors at line 81, so the test can unwind while
silently releasing worker ownership over runnable work.

The regression at `playa/cli/tests/detached_cli.rs:185-197` writes an inert file
directly and has no concurrent queue mutator, so it cannot distinguish this
broken cleanup from protocol-correct cleanup. The manual success-path deletion
in `biscuit-speaks/lib/tests/detached_phase4.rs:395-403` has the same missing
lock.

Required change: use one shared publication fixture that holds worker
exclusion, acquires `queue.lock` before scanning/removing pending records, and
reports cleanup failure without double-panicking. Add an unwind regression
with a publisher/preparation mutation synchronized at the queue boundary, then
prove no record remains before worker ownership can be reacquired. The existing
Claudine `LockedAudioSpool::clear_pending` already demonstrates the required
two-lock shape.

Verification level present: L1 deterministic cleanup without concurrent queue
mutation. Required level: L1 synchronized protocol-boundary regression. L2/L3
is not applicable.

### 2. High — Native mute is not verified at the output gain boundary

The native implementation applies volume through `Player::set_volume` in both
the cached-default and one-shot channel paths
(`playa/lib/src/native_player.rs:407-452`). No test observes either call. The
real Kokoro regression asserts only `Native` route and `Complete` verdict
(`biscuit-speaks/lib/tests/real_detached_phase4.rs:22-46`). It would still pass
if volume were dropped between `TtsConfig` and `Player::set_volume`, producing
audible synthesized speech. Playa's real zero-volume control uses zero-filled
PCM (`playa/lib/tests/real_playback_reports.rs:116-133`), so it is silent even
if gain control is ignored.

Host-player fallback is better verified: L1 tests capture the exact zero
argument after an injected native failure. That does not establish the native
branch required by specification rows 1, 3, and 9.

Required change: introduce a narrow test seam around native player creation or
gain application and assert `0.0` is applied before append/submission for both
the cached-default and channel-specific paths. Retain the real Kokoro/Say
completion checks; the L1 seam proves effective parameters while the real tier
proves synthesis, routing, and drain-to-completion.

Verification level present: L1 source/config propagation plus real completion.
Required level: L1 at the native output boundary plus the existing real-resource
completion. No terminal L2/L3 test is applicable.

### 3. High — Muted EchoGarden and gTTS playback tests are unreachable from the real-resource tier

The implementation correctly changes the EchoGarden and gTTS playback fixtures
to explicit zero volume and recognizable test messages, but the tests remain
`#[ignore]` and retain ordinary `test_*` names:

- `biscuit-speaks/lib/src/providers/host/echogarden.rs:1291-1323`
- `biscuit-speaks/lib/src/providers/host/gtts.rs:785-800`

The canonical `test` recipe skips ignored tests, and `test-real` selects
`real_` names but does not opt into ignored tests. Consequently these changed
tests ran in neither the 487-test L1 gate nor the two-test biscuit-speaks real
gate. Their comments also still say “Produces audio,” which is stale after the
mute change and conflicts with the purpose of this fix.

Required change: convert these to canonical `real_` resource tests, remove
`#[ignore]`, retain explicit availability/network skips, and provide a
provider-specific required-resource switch so CI or a provisioned host can
turn a skip into failure. Assert a playback report/route where the provider API
supports it rather than only `Result::is_ok()`. Correct the stale comments to
state that synthesis/playback is real but explicitly muted.

Verification level present: none in canonical recipes; source inspection only.
Required level: real-resource execution on hosts where each dependency is
available, with L1 controlled argument/route coverage where deterministic.
L2/L3 is not applicable.

## Production readiness

Not production ready. Close the queue-lock cleanup race, add native gain-boundary
proof, and make the affected EchoGarden/gTTS checks reachable through the
canonical real-resource tier before the next review. Cross-OS evidence remains
a CI concern and does not affect this verdict. Human review is not required for
these mechanical corrections.
