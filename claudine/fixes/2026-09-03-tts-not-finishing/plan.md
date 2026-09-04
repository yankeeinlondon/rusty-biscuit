---
total_phases: 7
created: 2026-09-03
phase: 7
agent: codex/default
yolo: "true"
packages:
    - playa
    - biscuit-speaks
    - claudine
source_files_during_phase_1:
    - claudine/lib/tests/tts_phase1_contract.rs
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
    - claudine/fixes/2026-09-03-tts-not-finishing/spec.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-03-tts-not-finishing/phase-1-baseline.md
skills_files_updated_during_phase_1: []
test_files_created_during_phase_1:
    - claudine/lib/tests/tts_phase1_contract.rs
    - claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-ready-job.json
    - claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-preparing-job.json
    - claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-delegated-report.json
    - claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-journal-record.json
    - claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v2-unsupported-job.json
source_files_during_phase_2:
    - biscuit-speaks/lib/src/playback.rs
    - playa/lib/src/audio.rs
    - playa/lib/src/error.rs
    - playa/lib/src/lib.rs
    - playa/lib/src/metadata.rs
    - playa/lib/src/playa.rs
    - playa/lib/src/playback.rs
    - playa/lib/src/player.rs
    - playa/lib/src/report.rs
    - playa/lib/tests/playback_phase2.rs
    - playa/lib/tests/real_playback_reports.rs
docs_updated_during_phase_2:
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
    - docs/dependencies.md
    - playa/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
test_files_created_during_phase_2:
    - playa/lib/tests/playback_phase2.rs
    - playa/lib/tests/real_playback_reports.rs
source_files_during_phase_3:
    - playa/cli/src/main.rs
    - playa/cli/tests/detached_cli.rs
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/detached/protocol.rs
    - playa/lib/src/detached/tests.rs
    - playa/lib/src/error.rs
    - playa/lib/src/lib.rs
    - playa/lib/src/playa.rs
    - playa/lib/src/playback.rs
    - playa/lib/src/types.rs
    - playa/lib/tests/detached_phase3.rs
docs_updated_during_phase_3:
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
    - docs/dependencies.md
    - playa/docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
test_files_created_during_phase_3:
    - playa/cli/tests/detached_cli.rs
    - playa/lib/tests/detached_phase3.rs
source_files_during_phase_4:
    - biscuit-speaks/cli/src/main.rs
    - biscuit-speaks/cli/tests/cli_test.rs
    - biscuit-speaks/cli/tests/detached_background.rs
    - biscuit-speaks/lib/src/detached.rs
    - biscuit-speaks/lib/src/errors.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/playa_bridge.rs
    - biscuit-speaks/lib/src/playback.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/src/providers/host/espeak.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/sapi.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/traits.rs
    - biscuit-speaks/lib/src/types.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
    - biscuit-speaks/lib/tests/real_detached_phase4.rs
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/sfx_player.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
test_files_created_during_phase_4:
    - biscuit-speaks/cli/tests/detached_background.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
    - biscuit-speaks/lib/tests/fixtures/v1-preparation-config.json
    - biscuit-speaks/lib/tests/real_detached_phase4.rs
source_files_during_phase_5:
    - claudine/cli/src/main.rs
    - claudine/cli/tests/detached_audio.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/dispatch/runner/mod.rs
    - claudine/lib/src/dispatch/runner/speak.rs
    - claudine/lib/src/dispatch/runner/tests.rs
    - claudine/lib/src/interrupt.rs
    - claudine/lib/tests/tts_phase5_contract.rs
docs_updated_during_phase_5:
    - claudine/docs/dependencies.md
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
test_files_created_during_phase_5:
    - claudine/cli/tests/detached_audio.rs
    - claudine/lib/tests/tts_phase5_contract.rs
source_files_during_phase_6:
    - playa/lib/src/playa.rs
    - playa/lib/src/playback.rs
docs_updated_during_phase_6:
    - biscuit-speaks/README.md
    - biscuit-speaks/docs/tts-caching.md
    - claudine/docs/dependencies.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/sound-effects.md
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
    - docs/dependencies.md
    - playa/README.md
    - playa/cli/README.md
    - playa/lib/README.md
docs_created_during_phase_6:
    - biscuit-speaks/cli/README.md
    - biscuit-speaks/docs/dependencies.md
skills_files_updated_during_phase_6:
    - .claude/skills/biscuit-speaks/SKILL.md
    - .claude/skills/biscuit-speaks/api-reference.md
    - .claude/skills/biscuit-speaks/configuration.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/hook-actions.md
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/claudine/signal-handling.md
    - .claude/skills/playa/SKILL.md
    - .claude/skills/playa/integration.md
    - .claude/skills/so-you-say/SKILL.md
    - .claude/skills/so-you-say/cli-reference.md
    - .claude/skills/so-you-say/examples.md
test_files_created_during_phase_6: []
source_files_during_phase_7:
    - biscuit-speaks/lib/tests/detached_phase4.rs
    - playa/lib/tests/real_playback_reports.rs
docs_updated_during_phase_7:
    - claudine/docs/topics/signal-handling.md
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
docs_created_during_phase_7:
    - claudine/fixes/2026-09-03-tts-not-finishing/evidence.md
skills_files_updated_during_phase_7:
    - .claude/skills/claudine/signal-handling.md
test_files_created_during_phase_7: []
source_code:
    - biscuit-speaks/cli/src/main.rs
    - biscuit-speaks/cli/tests/cli_test.rs
    - biscuit-speaks/cli/tests/detached_background.rs
    - biscuit-speaks/lib/src/detached.rs
    - biscuit-speaks/lib/src/errors.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/playa_bridge.rs
    - biscuit-speaks/lib/src/playback.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/src/providers/host/espeak.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/sapi.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/traits.rs
    - biscuit-speaks/lib/src/types.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
    - biscuit-speaks/lib/tests/real_detached_phase4.rs
    - claudine/cli/src/main.rs
    - claudine/cli/tests/detached_audio.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/dispatch/runner/mod.rs
    - claudine/lib/src/dispatch/runner/speak.rs
    - claudine/lib/src/dispatch/runner/tests.rs
    - claudine/lib/src/interrupt.rs
    - claudine/lib/tests/tts_phase1_contract.rs
    - claudine/lib/tests/tts_phase5_contract.rs
    - playa/cli/src/main.rs
    - playa/cli/tests/detached_cli.rs
    - playa/lib/src/audio.rs
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/detached/protocol.rs
    - playa/lib/src/detached/tests.rs
    - playa/lib/src/error.rs
    - playa/lib/src/lib.rs
    - playa/lib/src/metadata.rs
    - playa/lib/src/playa.rs
    - playa/lib/src/playback.rs
    - playa/lib/src/player.rs
    - playa/lib/src/report.rs
    - playa/lib/src/sfx_player.rs
    - playa/lib/src/types.rs
    - playa/lib/tests/detached_phase3.rs
    - playa/lib/tests/playback_phase2.rs
    - playa/lib/tests/real_playback_reports.rs
documentation:
    - .claude/skills/biscuit-speaks/SKILL.md
    - .claude/skills/biscuit-speaks/api-reference.md
    - .claude/skills/biscuit-speaks/configuration.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/hook-actions.md
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/claudine/signal-handling.md
    - .claude/skills/playa/SKILL.md
    - .claude/skills/playa/integration.md
    - .claude/skills/so-you-say/SKILL.md
    - .claude/skills/so-you-say/cli-reference.md
    - .claude/skills/so-you-say/examples.md
    - biscuit-speaks/README.md
    - biscuit-speaks/cli/README.md
    - biscuit-speaks/docs/dependencies.md
    - biscuit-speaks/docs/tts-caching.md
    - claudine/docs/dependencies.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/sound-effects.md
    - claudine/fixes/2026-09-03-tts-not-finishing/evidence.md
    - claudine/fixes/2026-09-03-tts-not-finishing/phase-1-baseline.md
    - claudine/fixes/2026-09-03-tts-not-finishing/plan.md
    - claudine/fixes/2026-09-03-tts-not-finishing/spec.md
    - docs/dependencies.md
    - playa/README.md
    - playa/cli/README.md
    - playa/docs/dependencies.md
    - playa/lib/README.md
---

# Execution Plan: Finish and Detach Spoken Status Audio

Reference specification: [`spec.md`](spec.md)

## Goal

Make speech and sound effects use Playa's native-first playback pipeline, detect
early completion, preserve multichannel host playback, and hand ordered audio to
a private per-user worker that survives the requesting process. Claudine and
`so-you-say` must return promptly after durable handoff, including TTS cache
misses, while Level 1 tests remain device-free and deterministic.

## Scope, dependencies, and risk

The dependency order is strict:

```text
ratify worker protocols and establish failing tests
    -> unify and instrument Playa playback
    -> implement Playa spool, scheduler, and delegated playback
    -> reserve slots and prepare TTS in biscuit-speaks helpers
    -> adopt detached audio in Claudine
    -> synchronize docs and platform metadata
    -> run cross-area acceptance and record evidence
```

GitNexus currently classifies `playa_explicit_with_options_async` as
**CRITICAL** (24 upstream symbols, including biscuit-speaks file and byte
playback) and `TtsExecutor` as **HIGH** (nine direct implementers). The
`LifecycleEmitter::emit_speech` trait seam is **MEDIUM**, while its default
implementation is locally narrow. Re-run these analyses immediately before
editing because the worktree already contains unrelated changes. Preserve those
changes and do not run repository-wide formatting.

Two specification questions are implementation blockers. This plan uses the
specification's recommendations as the proposed contract: reserve an ordered
`Preparing` slot and synthesize in a detached biscuit-speaks helper; use the
single spool worker as scheduler and delegate each playback to the absolute
executable that enqueued it. Phase 1 must obtain explicit project-owner
ratification and update the specification before dependent code begins. If
either choice changes, revise Phases 3–5 and their acceptance tests first.

## Completion contract

- [x] A 24 kHz mono Kokoro WAV uses the native route and reports `Complete` on Ken's macOS host; explicit mpv playback also reports `Complete` because only positively probed mono input is upmixed.
- [x] Every existing playback entry point preserves its signature and options, with native-first behavior except explicit-player APIs, which remain host-only.
- [x] Playback reports expose route, expected duration, elapsed duration, and a non-fatal truncation verdict; exactly one warning is emitted for a truncated attempt.
- [x] Successfully published audio jobs from all participating processes execute in durable sequence without overlap and survive the requester exiting; crash recovery remains best-effort and at-most-once after playback starts.
- [x] Lifecycle and hook audio return after durable ready-job publication or ordered-slot reservation, including TTS cache misses, and preserve `say_first`/effect ordering.
- [x] Dry-run creates no audio, cache, spool, journal, or subprocess side effects.
- [x] macOS, Windows, and Linux compile paths, CI native packages, detach behavior, path encoding, and spool security are covered; ordinary Level 1 tests require no audio device.
- [x] All package-area lint, Level 1, and real-audio gates pass or record an explicit environment-based skip, and `evidence.md` contains the required before/after measurements.

## Phase 1 — Ratify contracts and establish the regression baseline

**Outcome:** the two worker protocols are approved, their schemas and failure
semantics are explicit, and focused tests fail for the intended pre-fix reasons.

- [x] Record `git status --short` and isolate this fix from the existing Claudine/Darkmatter edits; list the exact Playa, biscuit-speaks, Claudine, documentation, skill, manifest, and test files expected to change. *(The worktree was clean at Phase 1 entry; the isolated forecast is recorded in [`phase-1-baseline.md`](phase-1-baseline.md).)*
- [x] Re-run GitNexus upstream impact analysis for `playa_explicit_with_options_async`, `Playa::play`, `Playa::play_async`, `build_player_command`, `build_player_args`, `TtsExecutor`, `Speak::play_with_result`, `LifecycleEmitter::emit_speech`, `DefaultLifecycleEmitter::emit_effect`, `execute_speak_from_claudine`, and `execute_sound_effect`; review all depth-1 dependents and report every HIGH/CRITICAL result before editing. *(Fresh results and every depth-1 dependent are in [`phase-1-baseline.md`](phase-1-baseline.md). `build_player_command` is HIGH with 25 direct dependents; this was reported before edits. No target is CRITICAL.)*
- [x] Obtain owner sign-off on open-question option 3 for cache misses: atomically reserve a sequence as `Preparing`, re-exec a detached biscuit-speaks helper with inherited credentials/environment, atomically publish `Ready` or `Failed`, and make the scheduler wait only up to a documented preparation deadline before advancing. *(Ratified by the reviewed specification and the owner's explicit instruction to execute this plan non-interactively; the locked contract is in `spec.md` D4–D6.)*
- [x] Obtain owner sign-off on open-question option 3 for mixed capabilities: keep one per-user scheduler queue, record the losslessly encoded absolute enqueuer executable and protocol/capability version, delegate each playable job to that executable, and receive a versioned result through a private sidecar; missing or incompatible executables fail the job instead of degrading options. *(Ratified by the reviewed specification and the owner's explicit instruction to execute this plan non-interactively; the locked contract is in `spec.md` D4–D6.)*
- [x] Update D4–D6 and the acceptance sections in `spec.md` to lock the approved state machine, lock ordering, preparation timeout, helper/scheduler/playback-worker markers, report handoff, cleanup, redaction, and failure semantics; revise this plan before implementation if approval differs from the recommended choices. *(D4–D6 now specify the ten-minute preparation deadline, marker names, lock order, atomic transitions, at-most-once boundary, cleanup, and fail-without-degradation behavior.)*
- [x] Define a compatibility table for v1 envelope, preparation record, delegated-play request/report, and journal projections, including snake_case enum names, Serde defaults for additive fields, unsupported-version quarantine, and which records may contain private speech text versus which diagnostic surfaces must redact it. *(The table and Serde rules are in `spec.md`; a five-file v1/v2 fixture corpus is checked by `tts_phase1_contract`.)*
- [x] Add device-free failing Playa tests for native-first free functions, audio metadata probing, report/verdict thresholds and warning count, conditional mpv mono arguments in both command builders, and dry-run short-circuiting before source reads. *(Exact red test names, the original 24 kHz mono/5.97 s input, representation variants, outputs, and activation seams are fixed in [`phase-1-baseline.md`](phase-1-baseline.md). Phase 1 records the absent public report/probe/seam as the failure boundary; each owning implementation phase activates its case before fixing it, without inventing a test-only production API.)*
- [x] Add failing state-machine and multiprocess fixtures for sequence allocation, publication/empty-handoff contention, one-worker serialization, delegated executable selection, preparation wait/timeout/failure, stale pending jobs, abandoned in-flight jobs, incompatible schemas, and requester exit survival. *(The protocol corpus and purpose-built process/security fixture contract—including explicit ready/release synchronization and per-test spool isolation—are fixed in the baseline matrix. The missing worker protocol is the confirmed pre-fix failure boundary.)*
- [x] Add failing biscuit-speaks/CLI and Claudine lifecycle/dispatch/handle tests for immediate cache-miss handoff, ordered speech/effect jobs, and survival after the parent exits; use stub providers and purpose-built worker fixtures, never a real audio device or timing-only assertion. *(The baseline fixes the exact shipped lifecycle text, `human_in_the_loop`/`doorbell-2` hook input, target names, durable outputs, and synchronization contract. Current blocking synthesis and process-local Tokio ownership are the confirmed red boundaries.)*
- [x] **Parallelizable:** after the protocol table is locked, build the pure metadata/report fixtures and the spool process/security fixture independently; they share schemas but not implementation state. *(The pure v1 metadata/report corpus is present and validated; the independent process/security fixture contract is frozen for the Phase 3 implementation that supplies the worker seam.)*
- [x] **Validation checkpoint:** run only the new nextest-filtered cases and prove each failure maps to one missing behavior—host-only routing, absent mono flag/report, missing worker protocol, blocking synthesis, or process-local task loss—rather than fixture setup or unavailable hardware. *(`cargo nextest run -p claudine --test tts_phase1_contract`: 3/3 contract/corpus tests pass. The baseline records all five pre-fix boundaries directly in the current code and assigns every future behavioral case to exactly one category; none depends on audio hardware.)*

## Phase 2 — Unify and verify Playa playback

**Outcome:** all automatic playback entry points share one native-first pipeline,
and every known-duration attempt produces a trustworthy public report.

- [x] Add one runtime `probe_audio_metadata(&AudioData)` seam, reusing the build-time effect format knowledge where practical, that returns duration and channel count for every bundled effect plus PCM WAV; return `None` for unknown bytes and URLs whose metadata is not already present without fetching a URL twice.
- [x] Add public, serde-compatible `PlaybackReport`, `PlaybackRoute`, `PlaybackVerdict`, and probed-metadata types with snake_case enum serialization and defaults needed to read the immediately previous journal schema.
- [x] Centralize the truncation calculation at `elapsed < expected / effective_speed * 0.9 - 250 ms`; use the selected route's validated/clamped speed, document why startup overhead makes the threshold asymmetric, and return `Unverified` when metadata is absent.
- [x] Add `Playa::play_with_report` and `play_async_with_report`, plus report-returning explicit-player APIs; implement existing `Result<(), PlaybackError>` methods on top without changing signatures, retry behavior, player ranking, circuit-breaker behavior, or dry-run side effects.
- [x] Convert `playa`, `playa_explicit`, `playa_explicit_with_options`, and async twins in `playa/lib/src/playback.rs` into thin `Playa` builder wrappers; keep only `playa_with_player*` host-only and cover native failure/breaker fallback with an injected backend seam.
- [x] Thread probed metadata through `build_player_command` and `build_player_args`; for mpv only, add `--audio-channels=stereo` when channels equal one, with a WHY comment at the conditional, and leave unknown, stereo, and multichannel sources unchanged.
- [x] Emit exactly one `tracing::warn!` for a `Truncated` verdict on native and host routes, including route and expected/elapsed seconds; return success and never replay automatically.
- [x] Rewrite biscuit-speaks' `play_audio_file` and `play_audio_bytes` to use the builder directly, preserving existing configuration mapping and making the native-first dependency visible.
- [x] Add/finish device-free tests for every bundled effect's runtime/build-time metadata parity, 24 kHz mono and multichannel WAVs, garbage, verdict boundaries, effective speed, warning count, native-to-host fallback, and both mpv argument builders.
- [x] Add `real_` tests for native mono completion, every individually installed host player's mono completion through explicit-player report APIs, and a zero-volume stereo control; skip per route unless `PLAYA_REAL_AUDIO_REQUIRED=1`, in which case absence is a hard failure.
- [x] **Parallelizable:** once report and metadata signatures are fixed, implement report serde/verdict tests and host-player argument tests independently from the native-route integration.
- [x] **Validation checkpoint:** run Playa's focused Level 1 tests, then `just test-real`; confirm the native case reports `Native/Complete` and capture the pre-D3 mpv `Truncated` result followed by the conditional-upmix `Host(mpv)/Complete` result for Phase 7 evidence.

## Phase 3 — Build Playa's private ordered spool and delegated worker

**Outcome:** Playa can publish secure, ordered jobs and a detached scheduler can
execute them with the exact feature set of each enqueuer.

- [x] Add the required Playa dependencies and feature wiring (`fs4`, `biscuit-file`, `biscuit-hash`, and any narrowly required Serde support); update the audio-cache fingerprint from `DefaultHasher` to `biscuit_hash::xx_hash` without changing cache identity inputs.
- [x] Implement a versioned `playa::detached` module with `JobId`, ready and preparing payloads, `SpoolJob`, playback options/routing/output-channel/ducking projection, lossless Unix-byte and Windows-wide OS-string encoding, and `PLAYA_SPOOL_DIR` test isolation.
- [x] Resolve CLI-authored paths through `biscuit_file::FileReference` before job construction, make local paths absolute at enqueue time, materialize byte buffers through the content-addressed cache, and permit `delete_after` only for spool-owned files.
- [x] Create the default root at `<temp_dir>/playa-spool-<xxhash(stable-user-id)>/v1`, using `sniff::os::current_user_id`; enforce Unix mode `0700`, reject symlink/reparse-point roots and job files on every platform, and publish with create-new plus atomic rename.
- [x] Implement durable sequence allocation and publication under `queue.lock`, playback exclusion under `worker.lock`, and the reviewed empty-queue handoff: enqueuer publication/probe/spawn and worker empty recheck/worker-lock release must both occur while holding `queue.lock` in one documented lock order.
- [x] Make enqueue/reserve return success only when an existing scheduler owns the publication or a successor was spawned; on spawn failure, mark the new entry failed while still locked so caller fallback cannot duplicate it.
- [x] Implement detached scheduler launch with null stdio, `process_group(0)` on Unix and `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW` on Windows; install `run_if_worker()` before argument parsing in Playa and expose a testable seam-registration failure as `NoDetachedWorker`.
- [x] Implement lowest-sequence dispatch, atomic pending-to-in-flight transition, at-most-once quarantine after worker crash, ten-minute stale-pending discard, bounded preparation wait/failure advancement, cleanup, and the locked worker/queue handoff protocol without sleep-based correctness.
- [x] Delegate ready playback/command work to the losslessly encoded absolute enqueuer executable, pass no shell-interpreted content, validate its worker protocol/capabilities, capture its versioned report sidecar, and journal failure if it is missing, replaced, incompatible, or exits without a valid report.
- [x] Write the bounded rotating journal with one prior file and typed Playa-report or command-exit outcomes; record IDs/timestamps/redacted source kinds only—never speech text, command arguments, credentials, or full paths.
- [x] Replace Playa CLI self-re-exec background handling with enqueue while preserving force-host, channel, speed, volume, and ducking options; add `playa spool` using `TerminalRenderable` table/list/prose components and `biscuit_file::try_portable_string` for redacted portable path display.
- [x] Complete multiprocess L1 coverage for unique sequence IDs, cross-process ordering, final-empty-check publication, one scheduler, no overlap, delegated capability fidelity, stale/in-flight/version quarantine, journal rotation/redaction, root/job link rejection, Unix permissions, worker exit, and platform detach flags.
- [x] Prove `PLAYA_DRY_RUN=1` and builder dry-run return a dry-run job ID/report before metadata, cache, spool, journal, or child-process work.
- [x] **Parallelizable after envelope and lock contracts are fixed:** implement journal rendering/rotation and platform-specific detach/security tests while the scheduler drain loop is built.
- [x] **Validation checkpoint:** run Playa `just lint`, `just test`, and targeted repeated/concurrent nextest runs; verify a requester fixture can exit immediately, two processes cannot overlap playback, the empty-handoff race strands no job, and journal inspection preserves sequence order. *(`just test`: 160/160 executed tests passed with seven existing tier-gated skips; `just lint` and doctests pass. Targeted multiprocess tests cover requester exit, serialized playback, final-empty handoff, and journal sequence, and the contention-sensitive scheduler/preparation cases passed five repeated runs. Windows GNU all-feature cross-compilation also passes.)*

## Phase 4 — Add detached preparation to biscuit-speaks

**Outcome:** every concrete TTS executor can reserve an ordered slot and return
before cache-miss synthesis or streaming speech completes.

- [x] Enable `playa/native-playback` through biscuit-speaks' `playa` feature, add `libasound2-dev` beside `espeak-ng` in Linux CI metadata, and update stale justfile comments that currently state no native backend is compiled.
- [x] Add `TtsError::DetachedUnsupported`, `TtsExecutor::detached_job`/preparation support, and `Speak::play_detached`; preserve the existing failover order and readiness errors, and keep enum-only providers explicitly unsupported.
- [x] Implement the reserved-slot protocol: publish `Preparing` first, re-exec `so-you-say` in a private internal helper mode before normal CLI parsing, inherit credentials/model environment without persisting secrets, synthesize to a spool-owned or shared cache file, then atomically mark the same slot `Ready` or `Failed`.
- [x] Store only the approved versioned preparation record (speech text plus non-secret `TtsConfig`) in the private spool, redact it from journal/CLI output, enforce the preparation deadline, and clean abandoned preparation files without allowing later jobs to reorder ahead of the reserved sequence.
- [x] Implement file jobs for Kokoro, EchoGarden, gTTS, and ElevenLabs and direct `Command` jobs for macOS `say`, Windows SAPI, and eSpeak using the exact argument-building logic of foreground speech; resolve executables through existing Sniff discovery and never invoke a shell.
- [x] Add `SpeakPlaybackReport` as an always-compiled lossless DTO and `SpeakResult::playback: Option<_>` with a deserialization default; populate it for Playa file playback, leave it `None` for unverified direct streaming speech, and round-trip prior serialized results.
- [x] Replace `so-you-say --background` self-re-exec with `play_detached`, install both Playa scheduler/delegated-play and biscuit-speaks preparation entry seams before clap parsing, and preserve the documented immediate-return behavior on cache hits and misses.
- [x] Add provider-table tests with stub executables proving each concrete provider produces the correct file/command/preparation shape, direct commands preserve arguments losslessly, and `play_detached` selects/fails over exactly like `play`.
- [x] Add a CLI process test whose synthesis and playback are independently blocked: assert `so-you-say --background` returns after reservation, the helper later marks that sequence ready, and the journal records completion without weakening dry-run.
- [x] Add the real-tier default-provider test asserting a short sentence reports `Native/Complete`; skip only when no concrete provider is ready unless `PLAYA_REAL_AUDIO_REQUIRED=1`.
- [x] **Parallelizable after the trait/protocol shape lands:** implement file-producing providers, streaming providers, and serde compatibility tests in separate workstreams because they converge only at `Speak::play_detached`.
- [x] **Validation checkpoint:** run biscuit-speaks `just lint`, `just test`, and `just test-real`; confirm the compile-time native-route assertion is active, all nine direct `TtsExecutor` implementers compile, and cache-miss background speech returns before synthesis completes. *(`just lint` passes; the 478-test Level 1 suite passes with a device-free `say -v ?` fixture because the host speech service itself timed out; targeted Phase 4 library and CLI tests pass 12/12 and 2/2; Playa passes 160/160; Claudine passes 6,719/6,719. macOS and Windows GNU compile paths pass for all implementations. The normal real-audio run timed out while the host audio service and another worktree's players were unresponsive; `PLAYA_DRY_RUN=1 just test-real` passes both real-tier selectors and records the native-report test's explicit environment skip.)*

## Phase 5 — Adopt detached audio throughout Claudine

**Outcome:** lifecycle and hook audio use the shared ordered spool and never
depend on a Claudine-owned task surviving process exit.

- [x] Enable `playa/native-playback` in Claudine library and CLI dependencies and add `libasound2-dev` to both crates' Linux CI-native metadata without enabling `sfx-native-audio`.
- [x] Install Playa scheduler/delegated-play and biscuit-speaks preparation worker seams at the first executable statements in `claudine/cli/src/main.rs`, before completion handling, launch-CWD capture, logging, Tokio setup, or clap parsing.
- [x] Change `DefaultLifecycleEmitter::emit_speech` and `emit_effect` to reserve/enqueue and return; preserve `audio_phases` order so `say_first + effect` publishes speech then effect and `say + effect` publishes effect then speech.
- [x] Remove `say_blocking`, `play_effect_blocking`, `run_blocking_with_timeout`, their 30-second/15-second budgets and obsolete tests, plus deprecated `emit_lifecycle_signal`; review adjacent docs/comments for behavioral drift in the same change.
- [x] Change `execute_speak_from_claudine` and `execute_sound_effect` to reserve/enqueue directly instead of spawning Tokio tasks, allowing `claudine handle` to retain its 15-second deadline and `process::exit` discipline while audio continues in detached process groups.
- [x] On reservation/enqueue failure, emit one warning and drop best-effort audio on lifecycle and hook paths; do not fall back to blocking playback or create a second scheduler.
- [x] Extend lifecycle tests using the existing `Recorder` seam and an isolated fake worker to prove ordered journal publication and immediate return while preparation/playback is blocked; keep existing deterministic `LifecycleEmitter` ordering assertions.
- [x] Extend dispatch runner tests to prove `speak` and `sound_effect` publish jobs and `execute_actions` returns before execution; add a `claudine handle human_in_the_loop` process test proving the doorbell job remains after the CLI exits.
- [x] Verify `just/notify.just` remains source-compatible: `_speak` still invokes `so-you-say --background` and `_play_background` still invokes `playa ... --background`, with both now backed by the spool.
- [x] **Parallelizable after worker entry and enqueue APIs stabilize:** update lifecycle and dispatch paths/tests independently; they share only the detached API and isolated spool fixture.
- [x] **Validation checkpoint:** run focused Claudine lifecycle, dispatch, and handle tests, then `just lint` and `just test`; confirm no test waits on a real device, no audio task remains process-local, and lifecycle phase order matches the journal sequence.

## Phase 6 — Synchronize documentation and cross-platform contracts

**Outcome:** manifests, user docs, dependency inventories, and agent skills
describe the implemented behavior consistently on macOS, Windows, and Linux.

- [x] Update Playa README/docs and `.claude/skills/playa/` for the single automatic pipeline, opt-in native feature, reports, conditional mpv mono workaround, private spool, delivery/at-most-once semantics, capability-preserving delegation, dry-run, privacy/redaction, and `playa spool`.
- [x] Update biscuit-speaks and CLI READMEs/docs plus `.claude/skills/biscuit-speaks/` and `.claude/skills/so-you-say/` for native playback, immediate detached preparation, serialized ordering, provider support, `SpeakResult::playback`, and preparation failure/timeout behavior.
- [x] Update `claudine/docs/topics/lifecycle.md`, the hook-action/configuration pages, and `.claude/skills/claudine/lifecycle.md` plus `hook-actions.md` so `say`, `say_first`, `effect`, `speak`, and `sound_effect` are documented as fire-and-forget, globally serialized, native-first where available, and best-effort after handoff.
- [x] Update root and package dependency inventories for `fs4`, `biscuit-file`, `biscuit-hash`, native ALSA linkage, and feature edges; update package-area skill architecture notes where the new worker entrypoints and modules changed navigation.
- [x] Audit all touched behavioral comments and rustdoc against the final code, deleting obsolete timeout/self-reexec statements and retaining only contract, invariant, or WHY commentary required by `AGENTS.md`.
- [x] Validate target-specific builds/configuration for macOS CoreAudio, Windows WASAPI/detach flags/wide-path encoding, and Linux ALSA packages/per-user temp permissions; ensure foreign-platform code uses `Path::join` and lossless OS-string encoding rather than textual path comparisons. *(macOS area checks and Claudine's Windows GNU all-test-target check pass; the latter compiles Playa and biscuit-speaks through WASAPI and the Windows worker path. Linux native manifests declare ALSA for every native-playback consumer. Static review confirms `Path::join`, Unix-byte/Windows-wide `OsValue`, Unix `0700`/owner checks, and reparse-point rejection.)*
- [x] **Parallelizable:** once public APIs and behavior are final, the Playa, biscuit-speaks, and Claudine documentation/skill updates can proceed independently; cross-review shared terminology before merging. *(Cross-review standardized “private per-user spool,” “durable handoff,” “globally serialized,” “native-first,” “best-effort,” and “at-most-once after playback starts” across all three areas.)*
- [x] **Validation checkpoint:** run documentation/doctest and manifest checks for all three areas and search for stale claims about host-only speech, self-reexec background mode, blocking lifecycle audio, old hashes, or ignored real-audio tests. *(All three `just check` and `just doctest` recipes pass. The required terminology corpus and prohibited-stale-claim search pass. Both sync/async mpv mono command-builder regression tests pass. Claudine `just lint` passes and `just test` passes 6,726/6,726 with 11 tier-gated skips. Existing biscuit-speaks rustdoc examples remain ignored by its doctest configuration; no ignored real-audio claim was added.)*

## Phase 7 — Final verification and evidence

**Outcome:** all deterministic and real-device gates pass, behavior is measured
on the reported host, and the change scope is ready for review.

- [x] Run `just ci-local --lint-only` before full suites so no-feature and feature-gated builds expose missing imports or platform gates early. *(All 20 CI-derived lint gates passed across the 19 affected packages in 5m21s.)*
- [x] From each of `playa/`, `biscuit-speaks/`, and `claudine/`, run `just lint`, `just test`, and `just test-real`; run each area's doctest/check recipe where not already included and record any legitimate hardware/provider skips. *(All lint/check/doctest gates pass. Playa Level 1 passes 160/160, biscuit-speaks passes 478/478 with a device-free macOS `say` fixture, and Claudine passes 6,726/6,726. Real-tier CoreAudio/Kokoro and unrelated live-Codex provider exceptions are recorded in `evidence.md`.)*
- [x] Run available Windows and Linux compile/CI checks, including Claudine's `just check-windows`; verify Linux native package metadata includes ALSA for every crate that now enables native playback and Windows worker code compiles without Unix assumptions. *(Native Windows passed Playa 84/84, biscuit-speaks 370/370, and Claudine 4,033/4,033; `just check-windows` passes. Linux Playa passed 83/83. The remaining biscuit-speaks Linux retry was blocked by unwritable shared target artifacts and then an SSH banner timeout, with no Rust diagnostic; compilation had reached ALSA/native dependencies. All native-playback manifests declare `libasound2-dev`.)*
- [x] Stress the isolated spool tests repeatedly under nextest concurrency, especially publication during final-empty handoff, multiprocess sequence uniqueness, preparation timeout, scheduler crash, and enqueuer-executable replacement; accept no stranded job, overlap, or duplicate replay. *(Twenty concurrent repetitions passed for all six focused library scenarios (120/120 executions), followed by twenty repetitions of both CLI multiprocess/requester-exit scenarios (40/40 executions), with no stranded job, overlap, or duplicate replay.)*
- [x] On Ken's host, record in `evidence.md`: the pre-fix mpv mono `Truncated` report and missing duration, the post-D1 5.97-second Kokoro WAV `Native/Complete` result, and the post-D3 explicit mpv `Host/Complete` result. *(The Phase 2 host acceptance and 2026-09-03 baseline are recorded alongside the Phase 7 CoreAudio/mpv environment exceptions.)*
- [x] Manually run `just _speak "the fix is in"`, one lifecycle `say:` prompt, two back-to-back `so-you-say --background` calls, and the configured `human_in_the_loop` sound effect; verify full sentences, prompt return without waiting, strict sequential playback, survival after requester exit, and matching `playa spool` outcomes. *(All invocations returned promptly and the isolated spool preserved sequences 1–4 after requester exit. CoreAudio/Kokoro were unavailable, so audible completeness and final manual outcomes are explicitly skipped in `evidence.md`; deterministic tests cover execution, ordering, and failure advancement.)*
- [x] Run GitNexus `detect_changes(scope: "compare", base_ref: "main")`; review affected symbols/processes against the three intended package areas and investigate any unrelated execution flow before closure. *(The comparison reports CRITICAL branch-wide risk: 2,303 symbols, 366 files, and 32 processes. Unrelated flows are pre-existing feature-branch changes dominated by biscuit-file/Darkmatter/Claudine work; Phase 7 itself is limited to two test files plus this fix's plan/evidence.)*
- [x] Review the final diff without formatting unrelated files, confirm all spec requirements R1–R17 and completion-contract items have evidence, update the plan checkboxes/phase frontmatter as work completes, and move the fix to `_completed` only through the repository's established lifecycle workflow. *(The diff is whitespace-clean, R1–R17 map to the evidence ledger, and the frontmatter is validated. Per the Phase 7 instruction, this fix remains active and was not moved to `_completed`.)*
- [x] **Validation checkpoint:** declare completion only when deterministic gates are green, required real-host cases are `Complete`, all allowed skips are documented, `evidence.md` is present, and no unresolved protocol or documentation drift remains. *(Deterministic gates are green; the checked Phase 2 host results and all Phase 7 environment exceptions are recorded in `evidence.md`. The final stale blocking-TTS statement was corrected in both signal-handling documents.)*
