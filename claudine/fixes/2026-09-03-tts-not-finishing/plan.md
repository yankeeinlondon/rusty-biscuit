---
total_phases: 7
created: 2026-09-03
phase: 1
agent: codex/default
yolo: "true"
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

- [ ] A 24 kHz mono Kokoro WAV uses the native route and reports `Complete` on Ken's macOS host; explicit mpv playback also reports `Complete` because only positively probed mono input is upmixed.
- [ ] Every existing playback entry point preserves its signature and options, with native-first behavior except explicit-player APIs, which remain host-only.
- [ ] Playback reports expose route, expected duration, elapsed duration, and a non-fatal truncation verdict; exactly one warning is emitted for a truncated attempt.
- [ ] Successfully published audio jobs from all participating processes execute in durable sequence without overlap and survive the requester exiting; crash recovery remains best-effort and at-most-once after playback starts.
- [ ] Lifecycle and hook audio return after durable ready-job publication or ordered-slot reservation, including TTS cache misses, and preserve `say_first`/effect ordering.
- [ ] Dry-run creates no audio, cache, spool, journal, or subprocess side effects.
- [ ] macOS, Windows, and Linux compile paths, CI native packages, detach behavior, path encoding, and spool security are covered; ordinary Level 1 tests require no audio device.
- [ ] All package-area lint, Level 1, and real-audio gates pass or record an explicit environment-based skip, and `evidence.md` contains the required before/after measurements.

## Phase 1 — Ratify contracts and establish the regression baseline

**Outcome:** the two worker protocols are approved, their schemas and failure
semantics are explicit, and focused tests fail for the intended pre-fix reasons.

- [ ] Record `git status --short` and isolate this fix from the existing Claudine/Darkmatter edits; list the exact Playa, biscuit-speaks, Claudine, documentation, skill, manifest, and test files expected to change.
- [ ] Re-run GitNexus upstream impact analysis for `playa_explicit_with_options_async`, `Playa::play`, `Playa::play_async`, `build_player_command`, `build_player_args`, `TtsExecutor`, `Speak::play_with_result`, `LifecycleEmitter::emit_speech`, `DefaultLifecycleEmitter::emit_effect`, `execute_speak_from_claudine`, and `execute_sound_effect`; review all depth-1 dependents and report every HIGH/CRITICAL result before editing.
- [ ] Obtain owner sign-off on open-question option 3 for cache misses: atomically reserve a sequence as `Preparing`, re-exec a detached biscuit-speaks helper with inherited credentials/environment, atomically publish `Ready` or `Failed`, and make the scheduler wait only up to a documented preparation deadline before advancing.
- [ ] Obtain owner sign-off on open-question option 3 for mixed capabilities: keep one per-user scheduler queue, record the losslessly encoded absolute enqueuer executable and protocol/capability version, delegate each playable job to that executable, and receive a versioned result through a private sidecar; missing or incompatible executables fail the job instead of degrading options.
- [ ] Update D4–D6 and the acceptance sections in `spec.md` to lock the approved state machine, lock ordering, preparation timeout, helper/scheduler/playback-worker markers, report handoff, cleanup, redaction, and failure semantics; revise this plan before implementation if approval differs from the recommended choices.
- [ ] Define a compatibility table for v1 envelope, preparation record, delegated-play request/report, and journal projections, including snake_case enum names, Serde defaults for additive fields, unsupported-version quarantine, and which records may contain private speech text versus which diagnostic surfaces must redact it.
- [ ] Add device-free failing Playa tests for native-first free functions, audio metadata probing, report/verdict thresholds and warning count, conditional mpv mono arguments in both command builders, and dry-run short-circuiting before source reads.
- [ ] Add failing state-machine and multiprocess fixtures for sequence allocation, publication/empty-handoff contention, one-worker serialization, delegated executable selection, preparation wait/timeout/failure, stale pending jobs, abandoned in-flight jobs, incompatible schemas, and requester exit survival.
- [ ] Add failing biscuit-speaks/CLI and Claudine lifecycle/dispatch/handle tests for immediate cache-miss handoff, ordered speech/effect jobs, and survival after the parent exits; use stub providers and purpose-built worker fixtures, never a real audio device or timing-only assertion.
- [ ] **Parallelizable:** after the protocol table is locked, build the pure metadata/report fixtures and the spool process/security fixture independently; they share schemas but not implementation state.
- [ ] **Validation checkpoint:** run only the new nextest-filtered cases and prove each failure maps to one missing behavior—host-only routing, absent mono flag/report, missing worker protocol, blocking synthesis, or process-local task loss—rather than fixture setup or unavailable hardware.

## Phase 2 — Unify and verify Playa playback

**Outcome:** all automatic playback entry points share one native-first pipeline,
and every known-duration attempt produces a trustworthy public report.

- [ ] Add one runtime `probe_audio_metadata(&AudioData)` seam, reusing the build-time effect format knowledge where practical, that returns duration and channel count for every bundled effect plus PCM WAV; return `None` for unknown bytes and URLs whose metadata is not already present without fetching a URL twice.
- [ ] Add public, serde-compatible `PlaybackReport`, `PlaybackRoute`, `PlaybackVerdict`, and probed-metadata types with snake_case enum serialization and defaults needed to read the immediately previous journal schema.
- [ ] Centralize the truncation calculation at `elapsed < expected / effective_speed * 0.9 - 250 ms`; use the selected route's validated/clamped speed, document why startup overhead makes the threshold asymmetric, and return `Unverified` when metadata is absent.
- [ ] Add `Playa::play_with_report` and `play_async_with_report`, plus report-returning explicit-player APIs; implement existing `Result<(), PlaybackError>` methods on top without changing signatures, retry behavior, player ranking, circuit-breaker behavior, or dry-run side effects.
- [ ] Convert `playa`, `playa_explicit`, `playa_explicit_with_options`, and async twins in `playa/lib/src/playback.rs` into thin `Playa` builder wrappers; keep only `playa_with_player*` host-only and cover native failure/breaker fallback with an injected backend seam.
- [ ] Thread probed metadata through `build_player_command` and `build_player_args`; for mpv only, add `--audio-channels=stereo` when channels equal one, with a WHY comment at the conditional, and leave unknown, stereo, and multichannel sources unchanged.
- [ ] Emit exactly one `tracing::warn!` for a `Truncated` verdict on native and host routes, including route and expected/elapsed seconds; return success and never replay automatically.
- [ ] Rewrite biscuit-speaks' `play_audio_file` and `play_audio_bytes` to use the builder directly, preserving existing configuration mapping and making the native-first dependency visible.
- [ ] Add/finish device-free tests for every bundled effect's runtime/build-time metadata parity, 24 kHz mono and multichannel WAVs, garbage, verdict boundaries, effective speed, warning count, native-to-host fallback, and both mpv argument builders.
- [ ] Add `real_` tests for native mono completion, every individually installed host player's mono completion through explicit-player report APIs, and a zero-volume stereo control; skip per route unless `PLAYA_REAL_AUDIO_REQUIRED=1`, in which case absence is a hard failure.
- [ ] **Parallelizable:** once report and metadata signatures are fixed, implement report serde/verdict tests and host-player argument tests independently from the native-route integration.
- [ ] **Validation checkpoint:** run Playa's focused Level 1 tests, then `just test-real`; confirm the native case reports `Native/Complete` and capture the pre-D3 mpv `Truncated` result followed by the conditional-upmix `Host(mpv)/Complete` result for Phase 7 evidence.

## Phase 3 — Build Playa's private ordered spool and delegated worker

**Outcome:** Playa can publish secure, ordered jobs and a detached scheduler can
execute them with the exact feature set of each enqueuer.

- [ ] Add the required Playa dependencies and feature wiring (`fs4`, `biscuit-file`, `biscuit-hash`, and any narrowly required Serde support); update the audio-cache fingerprint from `DefaultHasher` to `biscuit_hash::xx_hash` without changing cache identity inputs.
- [ ] Implement a versioned `playa::detached` module with `JobId`, ready and preparing payloads, `SpoolJob`, playback options/routing/output-channel/ducking projection, lossless Unix-byte and Windows-wide OS-string encoding, and `PLAYA_SPOOL_DIR` test isolation.
- [ ] Resolve CLI-authored paths through `biscuit_file::FileReference` before job construction, make local paths absolute at enqueue time, materialize byte buffers through the content-addressed cache, and permit `delete_after` only for spool-owned files.
- [ ] Create the default root at `<temp_dir>/playa-spool-<xxhash(stable-user-id)>/v1`, using `sniff::os::current_user_id`; enforce Unix mode `0700`, reject symlink/reparse-point roots and job files on every platform, and publish with create-new plus atomic rename.
- [ ] Implement durable sequence allocation and publication under `queue.lock`, playback exclusion under `worker.lock`, and the reviewed empty-queue handoff: enqueuer publication/probe/spawn and worker empty recheck/worker-lock release must both occur while holding `queue.lock` in one documented lock order.
- [ ] Make enqueue/reserve return success only when an existing scheduler owns the publication or a successor was spawned; on spawn failure, mark the new entry failed while still locked so caller fallback cannot duplicate it.
- [ ] Implement detached scheduler launch with null stdio, `process_group(0)` on Unix and `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW` on Windows; install `run_if_worker()` before argument parsing in Playa and expose a testable seam-registration failure as `NoDetachedWorker`.
- [ ] Implement lowest-sequence dispatch, atomic pending-to-in-flight transition, at-most-once quarantine after worker crash, ten-minute stale-pending discard, bounded preparation wait/failure advancement, cleanup, and the locked worker/queue handoff protocol without sleep-based correctness.
- [ ] Delegate ready playback/command work to the losslessly encoded absolute enqueuer executable, pass no shell-interpreted content, validate its worker protocol/capabilities, capture its versioned report sidecar, and journal failure if it is missing, replaced, incompatible, or exits without a valid report.
- [ ] Write the bounded rotating journal with one prior file and typed Playa-report or command-exit outcomes; record IDs/timestamps/redacted source kinds only—never speech text, command arguments, credentials, or full paths.
- [ ] Replace Playa CLI self-re-exec background handling with enqueue while preserving force-host, channel, speed, volume, and ducking options; add `playa spool` using `TerminalRenderable` table/list/prose components and `biscuit_file::try_portable_string` for redacted portable path display.
- [ ] Complete multiprocess L1 coverage for unique sequence IDs, cross-process ordering, final-empty-check publication, one scheduler, no overlap, delegated capability fidelity, stale/in-flight/version quarantine, journal rotation/redaction, root/job link rejection, Unix permissions, worker exit, and platform detach flags.
- [ ] Prove `PLAYA_DRY_RUN=1` and builder dry-run return a dry-run job ID/report before metadata, cache, spool, journal, or child-process work.
- [ ] **Parallelizable after envelope and lock contracts are fixed:** implement journal rendering/rotation and platform-specific detach/security tests while the scheduler drain loop is built.
- [ ] **Validation checkpoint:** run Playa `just lint`, `just test`, and targeted repeated/concurrent nextest runs; verify a requester fixture can exit immediately, two processes cannot overlap playback, the empty-handoff race strands no job, and journal inspection preserves sequence order.

## Phase 4 — Add detached preparation to biscuit-speaks

**Outcome:** every concrete TTS executor can reserve an ordered slot and return
before cache-miss synthesis or streaming speech completes.

- [ ] Enable `playa/native-playback` through biscuit-speaks' `playa` feature, add `libasound2-dev` beside `espeak-ng` in Linux CI metadata, and update stale justfile comments that currently state no native backend is compiled.
- [ ] Add `TtsError::DetachedUnsupported`, `TtsExecutor::detached_job`/preparation support, and `Speak::play_detached`; preserve the existing failover order and readiness errors, and keep enum-only providers explicitly unsupported.
- [ ] Implement the reserved-slot protocol: publish `Preparing` first, re-exec `so-you-say` in a private internal helper mode before normal CLI parsing, inherit credentials/model environment without persisting secrets, synthesize to a spool-owned or shared cache file, then atomically mark the same slot `Ready` or `Failed`.
- [ ] Store only the approved versioned preparation record (speech text plus non-secret `TtsConfig`) in the private spool, redact it from journal/CLI output, enforce the preparation deadline, and clean abandoned preparation files without allowing later jobs to reorder ahead of the reserved sequence.
- [ ] Implement file jobs for Kokoro, EchoGarden, gTTS, and ElevenLabs and direct `Command` jobs for macOS `say`, Windows SAPI, and eSpeak using the exact argument-building logic of foreground speech; resolve executables through existing Sniff discovery and never invoke a shell.
- [ ] Add `SpeakPlaybackReport` as an always-compiled lossless DTO and `SpeakResult::playback: Option<_>` with a deserialization default; populate it for Playa file playback, leave it `None` for unverified direct streaming speech, and round-trip prior serialized results.
- [ ] Replace `so-you-say --background` self-re-exec with `play_detached`, install both Playa scheduler/delegated-play and biscuit-speaks preparation entry seams before clap parsing, and preserve the documented immediate-return behavior on cache hits and misses.
- [ ] Add provider-table tests with stub executables proving each concrete provider produces the correct file/command/preparation shape, direct commands preserve arguments losslessly, and `play_detached` selects/fails over exactly like `play`.
- [ ] Add a CLI process test whose synthesis and playback are independently blocked: assert `so-you-say --background` returns after reservation, the helper later marks that sequence ready, and the journal records completion without weakening dry-run.
- [ ] Add the real-tier default-provider test asserting a short sentence reports `Native/Complete`; skip only when no concrete provider is ready unless `PLAYA_REAL_AUDIO_REQUIRED=1`.
- [ ] **Parallelizable after the trait/protocol shape lands:** implement file-producing providers, streaming providers, and serde compatibility tests in separate workstreams because they converge only at `Speak::play_detached`.
- [ ] **Validation checkpoint:** run biscuit-speaks `just lint`, `just test`, and `just test-real`; confirm the compile-time native-route assertion is active, all nine direct `TtsExecutor` implementers compile, and cache-miss background speech returns before synthesis completes.

## Phase 5 — Adopt detached audio throughout Claudine

**Outcome:** lifecycle and hook audio use the shared ordered spool and never
depend on a Claudine-owned task surviving process exit.

- [ ] Enable `playa/native-playback` in Claudine library and CLI dependencies and add `libasound2-dev` to both crates' Linux CI-native metadata without enabling `sfx-native-audio`.
- [ ] Install Playa scheduler/delegated-play and biscuit-speaks preparation worker seams at the first executable statements in `claudine/cli/src/main.rs`, before completion handling, launch-CWD capture, logging, Tokio setup, or clap parsing.
- [ ] Change `DefaultLifecycleEmitter::emit_speech` and `emit_effect` to reserve/enqueue and return; preserve `audio_phases` order so `say_first + effect` publishes speech then effect and `say + effect` publishes effect then speech.
- [ ] Remove `say_blocking`, `play_effect_blocking`, `run_blocking_with_timeout`, their 30-second/15-second budgets and obsolete tests, plus deprecated `emit_lifecycle_signal`; review adjacent docs/comments for behavioral drift in the same change.
- [ ] Change `execute_speak_from_claudine` and `execute_sound_effect` to reserve/enqueue directly instead of spawning Tokio tasks, allowing `claudine handle` to retain its 15-second deadline and `process::exit` discipline while audio continues in detached process groups.
- [ ] On reservation/enqueue failure, emit one warning and drop best-effort audio on lifecycle and hook paths; do not fall back to blocking playback or create a second scheduler.
- [ ] Extend lifecycle tests using the existing `Recorder` seam and an isolated fake worker to prove ordered journal publication and immediate return while preparation/playback is blocked; keep existing deterministic `LifecycleEmitter` ordering assertions.
- [ ] Extend dispatch runner tests to prove `speak` and `sound_effect` publish jobs and `execute_actions` returns before execution; add a `claudine handle human_in_the_loop` process test proving the doorbell job remains after the CLI exits.
- [ ] Verify `just/notify.just` remains source-compatible: `_speak` still invokes `so-you-say --background` and `_play_background` still invokes `playa ... --background`, with both now backed by the spool.
- [ ] **Parallelizable after worker entry and enqueue APIs stabilize:** update lifecycle and dispatch paths/tests independently; they share only the detached API and isolated spool fixture.
- [ ] **Validation checkpoint:** run focused Claudine lifecycle, dispatch, and handle tests, then `just lint` and `just test`; confirm no test waits on a real device, no audio task remains process-local, and lifecycle phase order matches the journal sequence.

## Phase 6 — Synchronize documentation and cross-platform contracts

**Outcome:** manifests, user docs, dependency inventories, and agent skills
describe the implemented behavior consistently on macOS, Windows, and Linux.

- [ ] Update Playa README/docs and `.claude/skills/playa/` for the single automatic pipeline, opt-in native feature, reports, conditional mpv mono workaround, private spool, delivery/at-most-once semantics, capability-preserving delegation, dry-run, privacy/redaction, and `playa spool`.
- [ ] Update biscuit-speaks and CLI READMEs/docs plus `.claude/skills/biscuit-speaks/` and `.claude/skills/so-you-say/` for native playback, immediate detached preparation, serialized ordering, provider support, `SpeakResult::playback`, and preparation failure/timeout behavior.
- [ ] Update `claudine/docs/topics/lifecycle.md`, the hook-action/configuration pages, and `.claude/skills/claudine/lifecycle.md` plus `hook-actions.md` so `say`, `say_first`, `effect`, `speak`, and `sound_effect` are documented as fire-and-forget, globally serialized, native-first where available, and best-effort after handoff.
- [ ] Update root and package dependency inventories for `fs4`, `biscuit-file`, `biscuit-hash`, native ALSA linkage, and feature edges; update package-area skill architecture notes where the new worker entrypoints and modules changed navigation.
- [ ] Audit all touched behavioral comments and rustdoc against the final code, deleting obsolete timeout/self-reexec statements and retaining only contract, invariant, or WHY commentary required by `AGENTS.md`.
- [ ] Validate target-specific builds/configuration for macOS CoreAudio, Windows WASAPI/detach flags/wide-path encoding, and Linux ALSA packages/per-user temp permissions; ensure foreign-platform code uses `Path::join` and lossless OS-string encoding rather than textual path comparisons.
- [ ] **Parallelizable:** once public APIs and behavior are final, the Playa, biscuit-speaks, and Claudine documentation/skill updates can proceed independently; cross-review shared terminology before merging.
- [ ] **Validation checkpoint:** run documentation/doctest and manifest checks for all three areas and search for stale claims about host-only speech, self-reexec background mode, blocking lifecycle audio, old hashes, or ignored real-audio tests.

## Phase 7 — Final verification and evidence

**Outcome:** all deterministic and real-device gates pass, behavior is measured
on the reported host, and the change scope is ready for review.

- [ ] Run `just ci-local --lint-only` before full suites so no-feature and feature-gated builds expose missing imports or platform gates early.
- [ ] From each of `playa/`, `biscuit-speaks/`, and `claudine/`, run `just lint`, `just test`, and `just test-real`; run each area's doctest/check recipe where not already included and record any legitimate hardware/provider skips.
- [ ] Run available Windows and Linux compile/CI checks, including Claudine's `just check-windows`; verify Linux native package metadata includes ALSA for every crate that now enables native playback and Windows worker code compiles without Unix assumptions.
- [ ] Stress the isolated spool tests repeatedly under nextest concurrency, especially publication during final-empty handoff, multiprocess sequence uniqueness, preparation timeout, scheduler crash, and enqueuer-executable replacement; accept no stranded job, overlap, or duplicate replay.
- [ ] On Ken's host, record in `evidence.md`: the pre-fix mpv mono `Truncated` report and missing duration, the post-D1 5.97-second Kokoro WAV `Native/Complete` result, and the post-D3 explicit mpv `Host/Complete` result.
- [ ] Manually run `just _speak "the fix is in"`, one lifecycle `say:` prompt, two back-to-back `so-you-say --background` calls, and the configured `human_in_the_loop` sound effect; verify full sentences, prompt return without waiting, strict sequential playback, survival after requester exit, and matching `playa spool` outcomes.
- [ ] Run GitNexus `detect_changes(scope: "compare", base_ref: "main")`; review affected symbols/processes against the three intended package areas and investigate any unrelated execution flow before closure.
- [ ] Review the final diff without formatting unrelated files, confirm all spec requirements R1–R17 and completion-contract items have evidence, update the plan checkboxes/phase frontmatter as work completes, and move the fix to `_completed` only through the repository's established lifecycle workflow.
- [ ] **Validation checkpoint:** declare completion only when deterministic gates are green, required real-host cases are `Complete`, all allowed skips are documented, `evidence.md` is present, and no unresolved protocol or documentation drift remains.
