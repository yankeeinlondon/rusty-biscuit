---
created: 2026-09-10
status: in_progress
spec: ./spec.md
---

# Audio Test Findings and Verification

## Source of the reported announcement

Source tracing identified two L1 tests in
`claudine/cli/tests/compose_caller_file_provenance.rs`:

- `shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target`
- `shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan`

Both copy the literal `prompts/_implement/implement-suggestions.md`, invoke
composition through `run_compose`, and use a successful fake Goose provider.
The shipped target's success notification says “the review findings in ...
were implemented successfully” and plays `bong`. The fixture did not suppress
audio publication. This establishes a test path to the exact reported template;
the investigation intentionally did not replay it audibly to establish which
past test invocation the user heard.

This also explains why the fake agent did not prevent real audio: provider
execution and lifecycle TTS are separate paths. Tests pinning Kokoro or eSpeak
elsewhere exercise specific TTS backends and need not match the user's normal
Claudine voice. The user's personal TTS configuration was not inspected.

## Inventory

Paths are relative to the repository root. L1 includes Rust integration binaries
without a real-resource or terminal-tier prefix.

| Test path | Boundary and finding | Required containment |
|-----------|----------------------|----------------------|
| `claudine/cli/tests/compose_caller_file_provenance.rs` | L1, real shipped success/failure lifecycle notifications after fake provider execution | Child-local audio dry-run and private spool; assert no spool publication while retaining literal templates |
| `claudine/cli/tests/detached_audio.rs` | L1, real doorbell file job, default volume, private spool and held worker lock | Zero-volume job plus cleanup under worker ownership |
| `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs` | L1, eSpeak presence stub and real effect payloads; worker lock blocks scheduling | Explicit test speech, retained emitter coverage, cleanup before releasing lock |
| `claudine/lib/src/dispatch/runner/tests.rs` | L1, eSpeak stub plus effect at 0.5; invalid-spool warning cases | Zero effect volume, explicit test speech, locked cleanup; retain failure-before-playback cases |
| `biscuit-speaks/cli/tests/detached_background.rs` | L1, private fake-only PATH, fake Kokoro/mpv, preparation and playback release markers | Preserve stubs and ordered completion; bound cleanup during success and unwinding |
| `biscuit-speaks/lib/tests/detached_phase4.rs` | L1, provider argument/cache-job construction and locked cached publication | Preserve nonzero serialization checks where non-executing; verify zero-volume handoff and updated provider contracts |
| `playa/lib/src/detached/tests.rs` | L1, injected scheduler execution and preparing-head ordering | Explicit test speech; preserve injected execution boundary |
| `biscuit-speaks/lib/tests/real_detached_phase4.rs` | Real-resource Kokoro/native completion, previously unmuted completion-style speech | Explicit zero volume and test message; retain route and completion checks |
| `biscuit-speaks/lib/src/providers/host/*` real tests | Real synthesis/playback in inline provider tests | Audit every real playback call, use zero volume and explicit test messages |
| `playa/lib/tests/real_playback_reports.rs` | Real native and host playback of zero-filled PCM WAV | Retain intrinsically silent fixtures and completion/timing assertions |

## Volume contract findings

`Speak::with_volume`, `TtsConfig::with_volume`, and `VolumeLevel` already expose
first-class normalized volume. The file bridge transfers it into Playa options.
Claudine enables native Playa playback directly; biscuit-speaks' `playa` feature
enables it transitively, including in the shipped CLI and canonical L1 recipe.

The initial provider gaps were macOS `say` ignoring volume and eSpeak omitting
amplitude arguments. SAPI already builds its native volume argument. Playa's
automatic player selection already filters for volume capability, but explicit
host-player APIs previously ignored requested volume for incapable players.

Native output applies gain before submission. Feature enablement does not prove
the native device opens successfully, so fallback and explicit-host boundaries
also require verification. No host-wide volume change is part of this work.

## Validation record

Implementation and validation are in progress. Results below will distinguish
controlled execution, real backend completion, skips, and provisioning failures.

The session declares `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL`. Initial SSH
checks reached native Windows and WSL2. Local discovery through `sniff` found
macOS `say`, eSpeak NG, Kokoro, Echogarden, gTTS, mpv, ffplay, mpg123, and afplay.
These are availability observations, not playback evidence.

| Environment / command | Result |
|-----------------------|--------|
| macOS, `playa/`: `just test` | 188 passed; 8 tier-filtered |
| macOS, `playa/`: `just lint` | Passed for library and CLI |
| macOS, `playa/`: `just test-real --success-output immediate` | Four real library tests passed; no resource skips; CLI defines no real tests |
| macOS, `biscuit-speaks/`: final `just test -j 2` | 487 passed; 19 filtered/ignored; includes all seven background fixture cases |
| macOS, `biscuit-speaks/`: final `just lint` | Passed for library and CLI |
| macOS, biscuit-speaks no-feature controlled tests | Three passed, including eSpeak amplitude and `say` → `afplay -v` zero/intermediate volume |
| macOS, biscuit-speaks focused feature-enabled library tests | 16 passed after the cache-rate correction |
| macOS, `just _test_real biscuit-speaks --features playa --success-output immediate` | Two passed with no resource skips: muted `say` (3.367 s) and Kokoro/af_heart (5.389 s), both asserting native `Complete` |
| macOS, `claudine/`: `just test --no-fail-fast` | 6,861 passed; 11 filtered; includes both exact-template provenance cases and all new audio-spool tests |
| macOS, `claudine/`: `just lint` | Passed |
| macOS, headless tmux L2, filter `shipped_implement` | All three selected Claudine CLI tests passed; no focus changes. The area recipe then failed backend proof for `claudine-gen`, where the filter selected zero tests; this is not counted as a passing aggregate recipe |
| macOS, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux BISCUIT_L2_THREADS=2 just _test_l2 claudine-cli --features terminal-tests shipped_implement` | Three passed; required tmux backend proof passed (`run=3 skip=0 panic=0`) |
| macOS, `biscuit-speaks/`: `just test -j 2` after review-1 finding 3 | 491 passed; 19 skipped. Up from 487 by the four `test_support` switch-parsing tests; the skipped count is unchanged because the six former `#[ignore]` tests became tier-filtered `real_` tests |
| macOS, `biscuit-speaks/`: `just lint` after review-1 finding 3 | Passed for library and CLI |
| macOS, `cargo nextest list -p biscuit-speaks --features playa -E "$(just _tier_filter real)"` | 8 real-tier tests selected, up from 2. Selection only — nothing was executed |
| Native Windows, `cargo check -p biscuit-speaks --features playa --all-targets --target x86_64-pc-windows-gnu` | Passed. Compile evidence only; not behavioral evidence |
| Linux, `ssh build-linux` reachability probe | Timed out at 30 s, as in the earlier attempts below; no Linux compile or test evidence for finding 3 |
| Native Windows, `just cross-check playa --os windows --features native-playback,async` | Compilation stopped on OS error 112 (disk full), before changed-code validation |
| WSL2 | Not launched after native Windows exhausted the shared physical build drive; no WSL2 behavioral or archive evidence obtained |
| Linux, initial Playa cross-check | Failed before compilation: global Git config `/home/build/.config/git/config` was inaccessible (`Host is down`) |

The Playa real run reported native `Complete` for 5.97-second PCM (elapsed
7.037 seconds) and mpv `Complete` for the same duration (elapsed 8.308 seconds).
The all-installed-player and zero-volume stereo cases also passed. Audio data
was zero PCM throughout; listening was not used as the proof of silence.

The Windows build drive had about 400 KiB free. Its shared test target measured
88.73 GiB. Dry-run sweeps with the standard 80 GB and reduced 40 GB caps offered
14.23 GiB and 32.91 GiB respectively, still below the 50 GiB build headroom.
Deletion of the shared target was requested separately; no existing cache was
deleted without approval.

Tooling observations: cross-check initially selected macOS Bash 3 and failed
on associative arrays; invoking the recipe with Homebrew Bash first on PATH
resolved that startup failure. GitNexus refresh completed in 356 seconds. Its
parser omitted an unrelated `schematic/definitions/src/openai/mod.rs` after a
timeout; graph impacts are lower bounds, supplemented by source tracing.

An isolated Linux probe with `GIT_CONFIG_GLOBAL=/dev/null` succeeded. The first
retry then hit the same unavailable home configuration mount for Git ignore and
attributes and nextest configuration. A second temporary cross-check script
isolates `XDG_CONFIG_HOME` and Git configuration before shell startup, preserves
the canonical checkout/patch/owner-lock safeguards, and selects `just _test`
with its storage preflight. The repository script and host configuration are
unchanged. SSH staging/session startup also stalled before compilation, so
these attempts do not establish Linux compatibility.

Independent review caught a Say cache identity bug before completion: the old
multiplier-based key conflated system-default rate with an explicit 175 WPM,
and rounded multipliers could represent different effective WPM. The corrected
key uses resolved integer WPM and does not reuse unknown system defaults.
Controlled regression tests distinguish these cases and confirm equivalent
effective rates reuse the same cached audio.

The final background fixture tests verify exact zero through the public library
enqueuer, preparation record, ready file job, and delegated player's captured
arguments. They also force cooperative cleanup to time out, then verify scoped
termination of fixture-owned executables and removal of runnable queue records.
Ownership uses canonical executable paths plus PID/start-time revalidation; no
name-wide process termination is used.

These tests exposed another isolation requirement: a requested `Samantha` voice
was upgraded to `Samantha (Enhanced)` by the real voice capability cache. The
fixture now supplies its own `BISCUIT_SPEAKS_CACHE` and fake voice inventory.
Production voice matching remains unchanged.

Final GitNexus `detect_changes` reported 110 affected symbols across 34 tracked
files with LOW aggregate risk and no reported affected processes. Dynamic
execution and test-only paths mean this is a lower-bound graph result, not proof
of no impact. Source review and the behavioral checks above supplement it.
`git diff --check` passed. No commit or `cargo fmt` was performed.

The final Linux attempt was stopped after bounded SSH staging/session delays.
No compilation or test result was produced. Cleanup confirmed no matching remote
validation process and no cross-check lock remained, removed only this run's
script/patch and empty temporary XDG directory, and preserved the standing
checkout and host configuration. The local launcher and transport were reaped.

Implementation is complete locally; cross-platform validation remains open.
The Windows shared-target cleanup approval is still pending, and Linux needs
working host configuration/session access before the remaining checks can run.

## Real backends exercised, and what is still owed

Review-1 finding 3 moved the EchoGarden and gTTS provider checks out of
`#[ignore]` and into the `real_` tier, where a canonical recipe can reach them.
The tier now selects eight biscuit-speaks tests instead of two:

| Test | Backend | Status |
|------|---------|--------|
| `providers::host::say::tests::real_say_provider_speaks_muted` | macOS `say` + native Playa | Exercised earlier in this fix; muted, native `Complete` |
| `real_detached_phase4::real_kokoro_provider_reports_muted_native_complete` | Kokoro `af_heart` + native Playa | Exercised earlier in this fix; muted, native `Complete` |
| `providers::host::echogarden::tests::real_echogarden_speaks_muted` | `echogarden` (default Kokoro engine) | **Not executed** — deferred |
| `providers::host::echogarden::tests::real_echogarden_speaks_muted_with_requested_voice` | `echogarden` Kokoro `Heart` | **Not executed** — deferred |
| `providers::host::echogarden::tests::real_echogarden_lists_installed_voices` | `echogarden` voice enumeration | **Not executed** — deferred |
| `providers::host::gtts::tests::real_gtts_speaks_muted` | `gtts-cli` + Google TTS | **Not executed** — deferred |
| `providers::host::gtts::tests::real_gtts_lists_installed_voices` | `gtts-cli` voice enumeration | **Not executed** — deferred |
| `providers::host::gtts::tests::real_gtts_reports_reachable_network` | Google TTS endpoint | **Not executed** — deferred |

The six new rows are recorded as deferred, not as passes. `just test-real` was
deliberately not run for EchoGarden or gTTS on this host: it performs real
synthesis and playback, and the specification forbids using listening as the
proof of silence, so a local run here would produce no evidence this fix can
rely on. Their execution belongs on a provisioned host or a CI leg, which can
set `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts` (see the biscuit-speaks
README, "Silent audio tests") so an absent backend fails the tier instead of
skipping it. Selection was proved by `cargo nextest list`, which builds the test
binaries without running any of them.

## Post-completion correction (2026-09-10, evening)

The user still heard `small-group-cheer` after this fix closed. The inventory
above missed `claudine/cli/tests/shipped_prompt_contract.rs`
(`feature_review_cli_preserves_numeric_iteration_and_dependent_paths`): it copies
the shipped corpus, its `codex` stub writes `ready: true` into the review, and
the shipped `success` stack then publishes `effect: small-group-cheer` to the
host's default spool. The host journal recorded a 10,597 ms file job (the
clip's exact length) at 20:28 local.

Correction: `CliProcessFixture::command()` now sets child-local
`PLAYA_DRY_RUN=1` and `PLAYA_SPOOL_DIR=<workspace>/audio-spool` for every L1
spawn; the contract test asserts that spool is never created, and
`cli_process_fixture.rs` proves both keys reach the child. `detached_audio.rs`
keeps its per-key opt-out. Verified silently by re-running the full
`claudine-cli` suite with an ambient `PLAYA_SPOOL_DIR` whose `worker.lock` was
held by a `perl flock` process: with the default, nothing was published; with
the default removed, the contract test left one pending job from the debug
`claudine` binary.
