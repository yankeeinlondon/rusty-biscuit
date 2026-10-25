---
kind: implementation-log
---

# Test audit: 2026-10-19-speak-prepare

Added **three genuine coverage tests**, with three gross and net additional
test identities. No test matrices were split and no identities were removed.
Modified **one originally slow real-resource test**; zero L1 tests needed a
speed repair. No existing test was separately strengthened.

## Changes

The new cloud-provider tests exercise an explicit-voice synthesis API failure
and cancellation while its response is pending. They assert the precise error
or cancellation result, no published audio, and no voice-selection record.
Cancellation waits for the request to start and verifies that its future was
dropped; it does not wait out a production timeout. Both tests run with and
without Playa and need no credentials, network, model, or audio device.

The third regression supplies an existing nonempty directory as an atomic
publication destination. Before the fix, publication failed but left its
`.biscuit-speaks-tmp-*` sibling behind. The test failed with that leaked path
in its observed directory contents. The fix uses an owned temporary file in
the destination directory, retains `write_all` and `sync_all`, closes the file,
and publishes through `TempPath::persist`. Failed writing, synchronization,
or publication now drops and removes the staging file. The regression also
checks that the destination's existing sentinel is preserved. Successful
replacement and last-rename-wins semantics are unchanged.

An initial broad filter inadvertently included the `real_` Say tests. The
muted native speech case took **5.842 seconds**. Its purpose is real synthesis
and native-backend completion, not system-default discovery, which has its own
native lookup test. It now pins Samantha and an explicit fast rate, speaks the
recognizable phrase “This is a test,” and owns a private audio cache. All mute,
native-route, and complete-verdict assertions remain. The canonical real-test
rerun took **2.939 seconds** with cold private audio storage. No audio was
audible and no window was opened.

## Requirement coverage

| Requirement | Boundary and evidence |
| --- | --- |
| Send no-op trait default; direct-streaming and unsupported providers | Trait compile contract, direct speaker no-op test, enum-only provider error, and both feature configurations. |
| Each file-producing provider prepares one reusable complete file without playback | Recording-executable preparation tests cover Say, Kokoro, gTTS, and Echogarden; injected transport tests cover ElevenLabs. Assertions count synthesis and inspect cache bytes, playback calls, and isolated spool state. |
| Returned and fresh objects; ordinary, reported, and detached reuse | Existing provider matrices inspect synthesis counts, metadata, exact ready-job paths, and locked spool publication. Prepared settings/helper handoff is decoded and run through the real helper function. |
| Default resolution and changed defaults | Injected Say resolver tests, recorded executable lookup tests, ElevenLabs gender/first-voice/fallback tests, and changed-default cache-hit/miss controls. Normal-speed Say remains a documented unresolved-rate no-op. |
| Retained settings, volume, speed, failover, missing files, and builder invalidation | Exact argument and key checks, retained versus original-config fallback, specific-provider refusal to switch, vanished-file resynthesis, every builder setter, and speed-once tables. |
| Dry run before discovery and side effects | Builder and direct-provider tests assert no tool/API request, cache, spool, journal, or discovery output; retained preparation survives dry run. |
| Failure, cancellation, concurrent publication | Existing partial-write fake processes and cancellation cleanup; complete concurrent output assertions; new cloud request failure/cancellation and atomic-publication failure tests. |
| ElevenLabs snapshot trust and account isolation | Injected transport/clock tests cover exact 24-hour boundary, future times, changed inputs/account/fallback, corrupt/missing/malformed files, relisting on every prepare, changed voice lists, absent/blank labels, and absence of credentials. |
| Consumer compatibility | Claudine pre-warm and TTS contract slice, recorded below. |

Recording tools and injected API responses are **F1** for their foreign input
formats; their tests prove dispatch, caching, publication, and cancellation
logic. The source schema/generated client and existing provider integration
tests remain the basis for ElevenLabs wire behavior; this slice makes no live
paid API claim. Real filesystem operations witness atomic replacement/error
behavior. The muted Say test crosses the actual native synthesis/playback
boundary (**F3**); the initial native system-voice lookup also passed in
**4.490 seconds**. The original implementation log records the byte-equality
experiment establishing the system-voice lookup's relationship to `say`.

No requirement uses a terminal or browser. The area's `test-l2` recipe is a
not-applicable stub; no L2 test was invented or skipped to hide missing coverage.

## Verification

All runs use Nextest through the repository recipes. Compilation time is
excluded from the test durations. L1 runs use the runner's normal parallelism;
the real-resource run is separate.

From `biscuit-speaks/`:

```sh
BISCUIT_TEST_FILTER='(binary(prepare) | test(prepared::) | test(providers::cloud::elevenlabs::) | test(providers::host::say::) | test(speak::tests::) | test(detached::tests::) | test(audio_cache::) | test(test_populate_cache_for_all_providers_runs_without_panic)) & not test(/(^|::)real_/)' just test
just test-real real_say_provider_speaks_muted
```

The final Playa-enabled L1 run passed **207 tests**, maximum **2.452 seconds**.
The new tests took **0.027–0.043 seconds**. The real Say rerun passed its one
selected test in **2.939 seconds**; the CLI tail selected zero matching tests
and completed successfully.

From the repository root:

```sh
BISCUIT_TEST_FILTER='(binary(prepare) | test(prepared::) | test(providers::cloud::elevenlabs::) | test(providers::host::say::) | test(speak::tests::) | test(detached::tests::) | test(audio_cache::)) & not test(/(^|::)real_/)' just _test biscuit-speaks --no-default-features
```

The no-Playa configuration passed **196 selected L1 tests**, maximum **2.457
seconds**. These repeat feature-independent identities and are configuration
observations, not 196 additional tests.

Logs: `/tmp/pipeline-s09-final-l1.log`, `/tmp/pipeline-s09-no-playa.log`,
`/tmp/pipeline-s09-real.log`, and the red-before proof in
`/tmp/pipeline-s09-red.log`.

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(prewarm) | test(tts_phase1_contract) | test(tts_phase5_contract)' just test-library
```

All **61 consumer L1 tests passed**, maximum **0.139 seconds**. Together with
the Playa-enabled speech slice this is **268 distinct L1 identities**, plus
the 196 no-Playa configuration observations and one final real-resource test.
The consumer log is `/tmp/pipeline-s09-claudine.log`.

Exact-package lint:

```sh
cargo clippy -p biscuit-speaks --all-targets --features playa -- -D warnings
```

Clippy passed; log `/tmp/pipeline-s09-clippy.log`. The test dependency now
explicitly enables Tokio's `sync` feature for the cancellation notification,
rather than relying on another dependency enabling it. This adds no crate.
`just test-l2` confirmed the area's not-applicable stub after L1 finished.
`git diff --check` passed. No full-area or cross-OS test run is claimed by this
slice; the coordinator owns final broader verification.
