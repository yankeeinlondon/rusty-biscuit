# Phase 1 baseline: detached spoken-status audio

Captured 2026-09-03 before implementation changes.

## Entry state and isolation

`git status --short` produced no output. The worktree was clean, so there were
no pre-existing Claudine or Darkmatter edits to preserve or disentangle. This
fix remains limited to the `playa`, `biscuit-speaks`, and `claudine` package
areas plus this fix's specification artifacts and the corresponding agent
skills.

The expected implementation file set is deliberately explicit. A later phase
may remove an item after proving it unnecessary, but adding an unlisted area
requires revisiting impact and scope first.

| Area | Expected files |
|---|---|
| Playa library | `playa/lib/Cargo.toml`; `playa/lib/src/{lib,playa,playback,player,audio_cache}.rs`; new `playa/lib/src/detached.rs`; focused unit/integration tests under `playa/lib/src/` and `playa/lib/tests/` |
| Playa CLI | `playa/cli/Cargo.toml`; `playa/cli/src/main.rs`; CLI/process fixtures under `playa/cli/tests/` |
| biscuit-speaks library | `biscuit-speaks/lib/Cargo.toml`; `biscuit-speaks/lib/src/{error,lib,playback,speak,traits}.rs`; concrete executors under `biscuit-speaks/lib/src/providers/{cloud,host}/`; focused tests beside those modules |
| so-you-say CLI | `biscuit-speaks/cli/Cargo.toml`; `biscuit-speaks/cli/src/main.rs`; `biscuit-speaks/cli/tests/cli_test.rs` and purpose-built worker fixtures |
| Claudine library | `claudine/lib/Cargo.toml`; `claudine/lib/src/composition/lifecycle/{mod,audio}.rs`; `claudine/lib/src/dispatch/runner/{mod,speak}.rs`; focused lifecycle/dispatch tests |
| Claudine CLI | `claudine/cli/Cargo.toml`; `claudine/cli/src/main.rs`; a focused `claudine/cli/tests/handle_detached_audio.rs` process test |
| Manifests and automation | the six crate manifests above and only narrowly required package-area `justfile`/CI metadata; `just/notify.just` is compatibility-checked and is not expected to change |
| Product documentation | `playa/README.md`, `playa/docs/`, `biscuit-speaks/{README.md,docs/}`, `claudine/docs/{lifecycle,hook-actions}.md`, and dependency docs in an affected package area when its dependency set changes |
| Skills | `.claude/skills/playa/SKILL.md`, `.claude/skills/biscuit-speaks/SKILL.md`, `.claude/skills/so-you-say/SKILL.md`, and `.claude/skills/claudine/{SKILL,lifecycle,hook-actions}.md` only when the implemented public workflow changes |
| Fix artifacts | `claudine/fixes/2026-09-03-tts-not-finishing/{plan,spec,phase-1-baseline,evidence}.md` and protocol/test fixtures owned by this fix |

## Fresh impact analysis

GitNexus was run against the index for this exact worktree and commit
`9009e17d90675fe4d32170b3b3af04b89b2a1c71`. Qualified method names were
resolved to their indexed UIDs where needed. The index reported no affected
execution process for these symbols.

| Target | Risk | Depth-1 dependents |
|---|---:|---|
| `playa_explicit_with_options_async` | LOW | `play_audio_bytes`, `play_audio_file`, `playa_explicit_async` |
| `Playa::play` | LOW | none indexed |
| `Playa::play_async` | LOW | none indexed |
| `build_player_command` | **HIGH** | production: `playa_with_player_and_options`; tests: `build_command_mpv_basic`, `build_command_mpv_with_volume_and_speed`, `build_command_ffplay_basic`, `build_command_ffplay_with_volume_and_speed`, `build_command_sox_basic`, `build_command_sox_with_volume_and_speed`, `build_command_vlc_basic`, `build_command_vlc_with_volume`, `build_command_mplayer_basic`, `build_command_mplayer_with_volume`, `build_command_gstreamer_basic`, `build_command_gstreamer_with_volume`, `build_command_paplay_basic`, `build_command_paplay_with_volume`, `build_command_pipewire_basic`, `build_command_pipewire_with_volume`, `build_command_mpg123_basic`, `build_command_ogg123_basic`, `build_command_aplay_basic`, `build_command_afplay_basic`, `build_command_afplay_with_volume_and_speed`, `build_command_afplay_clamps_speed`, `build_command_pacat_basic`, `assert_source_is_single_argument` |
| `build_player_args` | MEDIUM | production: `playa_with_player_and_options_async`; tests: `build_player_args_mpv_basic`, `build_player_args_mpv_with_volume_and_speed`, `build_player_args_ffplay_with_options`, `build_player_args_sox_speed_after_source` |
| `TtsExecutor` | MEDIUM | `ElevenLabsProvider`, `EchogardenProvider`, `ESpeakProvider`, `GttsProvider`, `KokoroTtsProvider`, `SapiProvider`, `SayProvider`, test `MockExecutor`, test `MinimalExecutor` |
| `Speak::play_with_result` | LOW | none indexed |
| `LifecycleEmitter::emit_speech` | MEDIUM | `DefaultLifecycleEmitter` plus six test recorders in lifecycle, looping, sequence, and CLI wrap tests |
| `DefaultLifecycleEmitter::emit_effect` | LOW lower bound | none indexed directly; dynamic dispatch through the seven `LifecycleEmitter` implementations is an explicit analysis boundary |
| `execute_speak_from_claudine` | LOW | `DispatchConfig::execute_speak` |
| `execute_sound_effect` | LOW | `execute_actions`, `play_default_sound_for_event` |

The HIGH result for `build_player_command` was reported before any edit. Phase
2 must rerun its impact analysis immediately before changing it and must retain
the non-mpv argument matrix unchanged.

## Requirement-to-test map

The exact motivating input is a 24 kHz, mono, 16-bit PCM WAV containing 5.97
seconds of Kokoro-style speech. Device-free cases use a generated silent WAV
with the identical container properties; the real tier retains the measured
Kokoro sample and the same mpv invocation from the diagnosis.

For the lifecycle regression, the exact shipped template is
`Phase {{phase}} of the plan in the {{area}} package area, was implemented
successfully`; the v1 preparation fixture contains its original rendered input
for this report: `Phase 1 of the plan in the claudine package area, was
implemented successfully`. The hook regression retains the original
`human_in_the_loop` event and `doorbell-2` effect.

| Changed public behavior | Targeted tests | Level and observable assertion |
|---|---|---|
| Automatic entry points are native-first; explicit-player entry points remain host-only | `free_functions_take_native_route_when_available`; `explicit_player_stays_host_only` | Playa L1 with injected backends; report route and host invocation count |
| Metadata covers shipped effects and PCM WAV variants | `probe_audio_metadata_shipped_effect_corpus`; `probe_audio_metadata_pcm_variants`; `probe_audio_metadata_rejects_garbage_and_does_not_refetch_url` | Playa L1 passive corpus plus public probe result; mono/stereo/6-channel, empty/truncated/garbage, local bytes/path/URL |
| Reports and verdict threshold are stable | `playback_report_serde_compatibility`; `verdict_boundaries`; `truncated_verdict_warns_once_and_returns_ok` | Playa L1; route, expected, elapsed, missing duration, downstream success, exactly one warning, old/new JSON forms |
| mpv only upmixes positively probed mono | sync and async `mpv_args_upmix_only_mono` | Playa L1; captured child argv for unknown/mono/stereo/6-channel and the exact original mpv flags |
| Dry-run reads or creates nothing | `dry_run_short_circuits_before_source_reads`; `dry_run_detached_has_no_side_effects` | Playa L1; missing source still succeeds and no cache/spool/journal/process marker appears |
| Queue allocation/publication is globally ordered and race-free | `spool_allocates_unique_sequence_across_processes`; `spool_publication_empty_handoff_cannot_strand`; `spool_serializes_one_worker_without_overlap` | Playa L1 multiprocess fixtures; journal order, unique IDs, ownership markers, maximum concurrency one |
| Scheduler delegates to the exact enqueuer capability | `spool_selects_lossless_enqueuer_executable`; `spool_rejects_missing_replaced_or_incompatible_delegate`; `delegated_report_round_trip` | Playa L1 process fixtures; executable identity/capability version, private report sidecar, failed job without fallback |
| Preparing slots preserve order and advance after bounded failure | `preparing_waits_at_head_then_publishes_ready`; `preparing_timeout_and_failed_advance`; `stale_pending_and_abandoned_in_flight_are_not_replayed` | Playa/biscuit-speaks L1; state/journal transitions and later-job ordering, with explicit synchronization rather than elapsed-time-only assertions |
| Spool data is private, versioned, compatible, and redacted | `protocol_corpus_is_compatible`; `unsupported_versions_are_quarantined`; `spool_rejects_links_and_enforces_private_permissions`; `journal_rotation_and_redaction` | Playa L1 passive fixture corpus plus filesystem/process tests; v1 and additive fields, unsupported versions, Unix 0700, symlink/reparse rejection, no text/args/credentials/full paths |
| Background TTS returns after reservation even on a cache miss | provider-table `detached_job_*`; `play_detached_walks_failover_like_play`; CLI `background_cache_miss_returns_after_reservation` | biscuit-speaks L1/library plus CLI process test; exact provider output shape/argv, selected provider, reservation marker before a blocked synthesis completes |
| Persisted speak results remain compatible | `speak_result_playback_serde` | biscuit-speaks L1; missing versus present `playback`, snake_case native/host/dry-run variants, write/read/write/read equality |
| Lifecycle ordering and dispatch survive requester exit | `default_emitter_preserves_say_effect_order_while_blocked`; `dispatch_audio_returns_after_publication`; `handle_audio_survives_parent_exit` | Claudine L1 library plus CLI process test; effect/speech journal order, action completion state, job/report after parent exit |
| Shipped configuration/document behavior remains valid | existing shipped prompt corpus plus `shipped_audio_actions_use_detached_contract` | Claudine L1 passive corpus and one real shipped prompt through normal lifecycle invocation; missing/present `say`, `say_first`, and `effect` combinations |
| Native and each installed host route finish the original mono shape | `real_native_plays_mono_wav_to_the_end`; `real_every_installed_host_player_plays_mono_wav_to_the_end`; biscuit-speaks `real_default_provider_reports_complete` | real tier; `Native/Complete`, per-player `Host/Complete`, and `SpeakResult.playback`; route-specific skip unless required |

Malformed or invalid variants are covered by garbage/truncated headers,
unsupported schema versions, incompatible delegate capabilities, failed and
timed-out preparation, missing executables, unsafe links, and invalid/missing
persisted fields. Persistence coverage includes repeated read/write/read for
both the spool envelope/journal projection and `SpeakResult`.

## Red-baseline activation

Tests whose public APIs or injection seams do not exist yet are kept as
named, ignored red cases in their owning phase so ordinary L1 remains green;
they are activated only after the minimal seam needed to compile the public
observation. Each owning phase must first run its focused case against the
pre-fix route and capture the single expected failure category listed below:

| Failure category | Cases |
|---|---|
| host-only routing | `free_functions_take_native_route_when_available` |
| absent mono flag/report | metadata, report/verdict/warning, and both mpv builder groups |
| missing worker protocol | spool, schema, security, delegation, journal, and dry-run-detached groups |
| blocking synthesis | biscuit-speaks cache-miss CLI and Claudine default-emitter cases |
| process-local task loss | Claudine dispatch and `handle` parent-exit cases |

This is intentionally not a timing-only test design. Blocking fixtures expose
explicit ready/release files or pipes; process survival is proven by a durable
journal/report written after the requester PID exits.

## Confirmed pre-fix failure boundaries

The Phase 1 contract corpus is green, while the behavioral cases remain red at
the following product boundaries. These observations were made from the exact
pre-fix worktree before any implementation edit:

- automatic `playa_explicit_with_options_async` selects a host player directly;
  it cannot report `Native`;
- neither mpv command builder emits `--audio-channels=stereo`, and no public
  metadata/report type exists;
- no `playa::detached` module, queue, worker entry seam, preparation state, or
  delegated report handoff exists;
- `DefaultLifecycleEmitter` calls `say_blocking`/`play_effect_blocking`, so a
  cache miss and playback still block the lifecycle path;
- hook speech uses `tokio::spawn` and hook effects use
  `tokio::task::spawn_blocking`, so neither task is owned after the requester
  exits.

The future behavioral tests are intentionally not compiled into Phase 1 with
invented test-only production APIs. Their exact names, inputs, observable
outputs, fixture synchronization, and activation phase are fixed above. This
keeps ordinary L1 green while ensuring each implementation phase begins by
adding only its minimal injection/API seam, activating its named red case, and
showing the expected single failure before the fix.

## Phase 1 verification

- Focused: `cargo nextest run -p claudine --test tts_phase1_contract` — 3
  passed, 0 skipped.
- Claudine L1: `just test` — 6,719 passed, 11 skipped, 0 failed. The skips are
  the existing four `completion_perf` cases, one `compose_ttff_perf` case,
  four `system_prompt_perf_bench` cases, `slow_compose_sigint_during_prep_exits_130_with_notice`,
  and `real_corpus_builds_deterministically`.
- Claudine lint: `just lint` — diagnostic guard 18 passed, 0 skipped; Clippy
  passed for `claudine-catalog-types`, `claudine`, `claudine-contract`,
  `claudine-cli`, and `claudine-gen`.
- `git diff --check` and JSON parsing of every protocol fixture passed.
- GitNexus `detect_changes(scope: "unstaged")` reported LOW risk and no
  affected execution processes for indexed changes. New untracked corpus/test
  files were reviewed separately because GitNexus does not include them.
