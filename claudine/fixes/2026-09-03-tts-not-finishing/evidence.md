# Phase 7 verification evidence

Verification was performed on Ken's macOS host on 2026-09-04. The host was
identified as macOS 27.0. Windows and Linux results below came from the
repository's standing native build hosts.

## Playback measurements

The motivating artifact is the exact 24 kHz, mono, 16-bit PCM Kokoro WAV with
an expected duration of 5.970 seconds.

| State | Route | Observed result | Evidence |
| --- | --- | --- | --- |
| Before D3 | explicit mpv | `Truncated { missing: ~1.21s }`; 4.76 seconds elapsed versus 5.970 seconds expected | Baseline measured on this host on 2026-09-03 and recorded in `spec.md`; the report verdict follows the Phase 2 threshold. |
| After D1 | automatic native | `Native`, expected 5.970 seconds, `Complete` | Phase 2's checked real-audio checkpoint. |
| After D3 | explicit mpv | `Host(Mpv)`, expected 5.970 seconds, `Complete` | Phase 2's checked real-audio checkpoint after conditional mono upmix. |

The Phase 7 rerun could not reproduce the successful real routes because the
host audio services were unavailable. Native device open returned
`audio device did not respond within 5s` and took the test's documented skip;
explicit mpv did not return before nextest's 30-second real-test timeout. These
are environment exceptions, not report assertion failures. The dedicated
`real_explicit_mpv_mono_reports_complete` test now preserves the exact artifact
and asserts the route, expected duration, and verdict independently of other
installed players.

## Requirement-to-test mapping

| Changed behavior | Public observable result | Tests |
| --- | --- | --- |
| Native-first routing and report contract | Route, expected/elapsed duration, verdict, fallback, effective-speed boundary, and exactly one truncation warning | `playback_phase2::{original_24khz_mono_5970ms_input_is_probed, metadata_probe_preserves_multichannel_and_rejects_unknown_sources, report_schema_round_trips_and_defaults_additive_fields, verdict_uses_exact_threshold_and_effective_speed}`, report module warning tests, and `real_playback_reports` |
| Conditional mpv mono handling | Both public command paths add stereo output only for positively probed mono; unknown, stereo, and multichannel inputs remain unchanged | `build_command_mpv_upmixes_only_probed_mono`, `build_player_args_mpv_upmixes_only_probed_mono`, and `real_explicit_mpv_mono_reports_complete` |
| Durable, private, ordered detached playback | Durable sequence, one-at-a-time dispatch, empty-handoff ownership, requester-exit survival, timeout/crash quarantine, executable replacement failure, path round trips, and redacted journal | Playa detached module tests, `detached_phase3`, and `playa-cli::detached_cli` |
| Detached TTS cache hit/miss | Cache miss reserves a durable ordered slot, provider selection survives detachment, cached jobs publish ready, and dry-run has no side effects | `biscuit-speaks::detached_phase4` and `biscuit-speaks-cli::detached_background` |
| Claudine lifecycle/hook ordering | `say_first`/effect order, immediate durable publication, requester-exit survival, and dry-run absence of spool/cache/process state | lifecycle `audio_emission`, dispatch runner tests, `tts_phase5_contract`, and `claudine-cli::detached_audio` |
| Shipped protocol/configuration compatibility | All shipped v1 artifacts deserialize, v2 is rejected, defaults are accepted, and persisted jobs survive repeated write/read/read | `tts_phase1_contract`, `shipped_v1_protocol_corpus_deserializes_and_v2_remains_unsupported`, `shipped_ready_artifact_runs_through_normal_stale_policy`, and `persisted_job_survives_repeated_write_read_round_trip` |

Phase 7 changed no production behavior. It added one dedicated real-mpv
acceptance test and made the existing foreground-provider-selection regression
self-contained on Windows by placing the current test executable on `PATH` as
`kokoro-tts`. The latter is the original provider-selection input and still
asserts the downstream ready job and provider state.

## Deterministic gates

- Root `just ci-local --lint-only`: 20/20 gates passed across 19 affected
  packages in 5m21s.
- Playa: `just lint`, `just check`, and `just doctest` passed; `just test`
  passed 160/160 with 8 tier-gated skips.
- biscuit-speaks: `just lint`, `just check`, and `just doctest` passed. The
  initial unmodified-host `just test` passed 471 tests but timed out in seven
  macOS `say` voice-service cases. A device-free `say` fixture rerun passed
  478/478 with 19 tier-gated skips.
- Claudine: `just lint`, `just check`, and `just doctest` passed; `just test`
  passed 6,726/6,726 with 11 tier-gated skips.
- Stress: 20 repetitions of six focused Playa library scenarios passed
  120/120; 20 repetitions of both CLI multiprocess/requester-exit scenarios
  passed 40/40 under nextest concurrency.

The biscuit-speaks doctest gate passed 10 tests and retained 33 pre-existing
provider-heavy ignored examples. Playa passed one doctest. Claudine passed 20
library doctests (7 ignored), 3 compile-fail cases, and 2 contract doctests.

## Cross-platform gates

- Native Windows: Playa 84/84, biscuit-speaks 370/370, and Claudine
  4,033/4,033 passed. `claudine/just check-windows` also passed. It emitted one
  unrelated pre-existing unused-import warning in
  `compose_caller_file_provenance.rs`.
- Linux: Playa passed 83/83. biscuit-speaks compilation reached its ALSA/native
  dependencies, but its standing-clone target contained unwritable shared
  artifacts; a fresh-target retry was then blocked during SSH banner exchange.
  Neither attempt produced a Rust diagnostic.
- Manifest inspection confirms every crate enabling native playback declares
  Ubuntu's `libasound2-dev`; Playa additionally declares `libpulse-dev`.
- The root cross-check script requires Bash 4 associative arrays, so it was run
  with `/opt/homebrew/bin/bash` instead of macOS Bash 3.2.

## Real and manual gates

- Playa `just test-real`: the native route took its explicit unavailable-device
  skip; the aggregate installed-host case and dedicated mpv case timed out
  because host playback did not return.
- biscuit-speaks `just test-real`: the default provider did not finish within
  30 seconds. Manual Kokoro helpers likewise remained in Python/ONNX synthesis
  and were terminated after their isolated evidence was captured.
- Claudine `just test-real`: four unrelated live Codex-provider contract tests
  failed with provider errors; the OpenCode case passed or skipped. No failure
  reached the audio assertions.
- `just _speak "the fix is in"` returned successfully immediately.
- Two branch `so-you-say --provider kokoro --background` calls returned
  successfully in under one millisecond and durably reserved sequences 1 and
  2. The configured `human_in_the_loop` invocation returned successfully and
  published ready effect jobs at sequences 3 and 4. `playa spool` showed all
  four in strict order after the requesters exited.
- A normal `claudine compose` invocation using a stub OpenCode provider and a
  prompt with `start.say: "phase seven lifecycle speech"` returned successfully
  in 3.61 seconds and left sequence 1 durably reserved after provider exit.

Full-sentence audibility and completed manual spool outcomes could not be
observed while CoreAudio/Kokoro were unavailable. The deterministic worker
tests cover completion, ordering, requester exit, and failure advancement; the
manual run confirms prompt return and durable publication on the real host.
