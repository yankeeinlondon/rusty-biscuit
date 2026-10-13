---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-19-speak-prepare/spec.md"
plan: "claudine/features/2026-10-19-speak-prepare/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/say/system_defaults.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-speaks/SKILL.md
source_files_during_phase_2:
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/traits.rs
    - biscuit-speaks/lib/src/test_support.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
docs_updated_during_phase_2:
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-speaks/api-reference.md
packages:
    - biscuit-speaks
human_review: false
source_files_during_phase_3:
    - biscuit-speaks/lib/src/cache.rs
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/say/system_defaults.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
    - biscuit-speaks/lib/tests/volume_control.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
docs_updated_during_phase_3:
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-speaks/SKILL.md
    - .claude/skills/biscuit-speaks/configuration.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - biscuit-speaks/lib/src/detached.rs
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/mod.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/tests/prepare.rs
docs_updated_during_phase_4:
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-speaks/SKILL.md
source_files_during_phase_5:
    - biscuit-speaks/lib/src/audio_cache.rs
    - biscuit-speaks/lib/src/cache.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
docs_updated_during_phase_5:
    - biscuit-speaks/README.md
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/biscuit-speaks/SKILL.md
    - .claude/skills/biscuit-speaks/api-reference.md
    - .claude/skills/biscuit-speaks/providers.md
    - .claude/skills/biscuit-speaks/configuration.md
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - biscuit-speaks/lib/src/audio_cache.rs
    - biscuit-speaks/lib/src/cache.rs
    - biscuit-speaks/lib/src/detached.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/mod.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/say/system_defaults.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/test_support.rs
    - biscuit-speaks/lib/src/traits.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
    - biscuit-speaks/lib/tests/prepare.rs
    - biscuit-speaks/lib/tests/volume_control.rs
documentation:
    - biscuit-speaks/README.md
    - biscuit-speaks/docs/tts-caching.md
completed_phase: 6
implemented: true
---
# Implementation Log for 2026-10-19-speak-prepare (6 phases)

## Phase 1

### Rulings

Rulings 1–11 in the plan were adopted as written. Nothing in this phase
implemented or contradicted them. Rulings 5 (ElevenLabs account context) and 8
(Say rate semantics) still wait on the author's decision; see "Open items".

### Spike A — the voice and rate `say` uses with no `-v`/`-r` (macOS 27.2, build 26B5091g)

**Lookup:** AppKit's documented `NSSpeechSynthesizer` API, through JXA:

```sh
osascript -l JavaScript -e 'ObjC.import("AppKit");
  var v = $.NSSpeechSynthesizer.defaultVoice;
  var s = $.NSSpeechSynthesizer.alloc.initWithVoice(v);
  JSON.stringify({voice: ObjC.unwrap($.NSSpeechSynthesizer.attributesForVoice(v).objectForKey("VoiceName")), rate: s.rate})'
```

Output grammar: one line of JSON, `{"voice":"<name accepted by say -v>","rate":<number, words per minute>}`.
On this host it printed `{"voice":"Ava (Premium)","rate":175}` and took about 0.6 s.

**Rejected candidates:**

- `defaults read com.apple.speech.voice.prefs`: on this host it has no
  `SelectedVoiceName`/`SelectedVoiceID`/`VoiceRateDataArray` keys, only usage
  statistics. It is an undocumented preference, which the spec forbids treating
  as authoritative.
- `com.apple.speech.synthesis.general.prefs`: it holds Talking Alerts and time
  announcement settings, not `say`'s voice.
- `say -v '?'`: it marks no default voice.

**Verification (byte comparison, run on this host):**

- `say -o none.wav --file-format=WAVE --data-format=LEI16 "<text>"`, with no
  flags, produced the same SHA-1 as `-v "Ava (Premium)" -r 175` and as
  `-v "Ava (Premium)"`: `a5f50e74…`.
- `-v "Ava (Premium)" -r 200` and `-v Samantha -r 175` each produced different
  bytes. So both voice and rate change the output, and the lookup's pair
  reproduces the default output exactly.
- `say -v '?'` lists `Ava (Premium)`, so the reported name round-trips through
  `-v`.

**Trusted when:** `osascript` exits 0, prints exactly one JSON object with a
non-empty string `voice` and a positive integral `rate`, and the voice name
appears in `say -v '?'`. Anything else (absent, malformed, a non-integral or
zero rate, an unknown voice) is `None` ⇒ unresolved ⇒ no-op, as the spec
requires.

**Not verified (deliberately):** I did not change the system's Spoken Content
voice or rate on the author's machine in a headless run. So it is still
unproven that a fresh `NSSpeechSynthesizer` follows a changed speaking-rate
slider. The observed 175 equals the library's 175-WPM base, so this host
cannot tell "reads the user's rate" from "hard-coded default". Phase 3 must
not implement the production resolver until that is checked: change the voice
and rate in System Settings → Accessibility → Spoken Content, re-run the
lookup and the byte comparison, then restore. If the rate does not follow,
resolve the voice from this lookup and report the rate as unresolved (ruling
8), which keeps system-default Say uncached.

### Seams and scaffolding

- **ElevenLabs transport and clock** (`providers/cloud/elevenlabs.rs`):
    - `ElevenLabsTransport` is crate-private and object-safe (boxed `Send`
      futures). It carries `list_voices` (all pages) and `create_speech`.
      `HttpTransport` holds the old paging code and the schematic
      `create_speech` call unchanged.
    - The provider now holds `client: Arc<ElevenLabs>`, shared with the
      transport for `list_models` and `create_sound_effect`, plus
      `transport: Arc<dyn ElevenLabsTransport>` and `clock: Arc<dyn Clock>`.
    - `Clock::now_unix_seconds` comes from `SystemClock`.
    - `#[cfg(test)] fake` has `FakeTransport` (scripted or failing voice list,
      list/speech request counts, recorded speech calls), `voice(id, gender)`,
      and `FakeClock`. The crate-private `with_transport(transport, clock)` is
      `#[cfg(test)]` and needs no API key.
    - The clock field and the `Clock` trait carry
      `#[allow(dead_code, reason = "scaffolding…")]` until the snapshot reads
      them.
- **Say system defaults** (`providers/host/say/system_defaults.rs`, a new
  submodule of `say.rs`):
    - `SystemSpeechDefaults::lookup() -> impl Future<Output = SystemSpeech> + Send`,
      where `SystemSpeech { voice: Option<String>, rate_wpm: Option<u32> }`.
    - `OsSpeechDefaults` is the production placeholder; it reports both as
      unresolved. `FakeSpeechDefaults` (`#[cfg(test)]`) pins values and counts
      lookups.
    - Module-level `#![allow(dead_code, reason = "scaffolding…")]` until
      Phase 3 wires it.
    - **Departure from the plan's signature.** The plan sketched separate sync
      `voice()`/`rate_wpm()` methods. The verified lookup is one subprocess
      that returns both values, so one async call keeps the pair consistent
      and avoids blocking the executor for about 0.6 s.
- **Recording executable harness** (`lib/tests/common/mod.rs`, used by the new
  `lib/tests/prepare.rs` binary):
    - `TtsFixture` points `PATH` at fixture `bin/` only, sets
      `TMPDIR`/`TMP`/`TEMP` to fixture `tmp/` (the shared audio cache), sets
      `BISCUIT_SPEAKS_CACHE` to a fixture file, sets `PLAYA_SPOOL_DIR` to an
      uncreated fixture `spool/`, and removes `PLAYA_DRY_RUN`, all through
      `test_toolkit::EnvGuard`.
    - `install(FakeTool, Behavior)` writes a `/bin/sh` fake for `say`,
      `kokoro-tts`, `gtts-cli`, `echogarden`, or `mpv`. Behaviors:
      `Synthesize`, `FailAfterPartialWrite`, `HangAfterPartialWrite` (`exec
      sleep`, so the recorded PID is the live child).
    - Each fake records its argv, its stdin (`say` only), and the output path
      its tool's argument grammar names. `invocations`/`syntheses` read those
      records back.
    - Other helpers: `temp_entries`, `audio_cache_entries`,
      `assert_spool_absent` (the journal lives in the spool), `lock_spool`
      (`LockedAudioSpool` plus `files/` and `requests/`), and `process_alive`.
    - The fake `mpv` is the play/play_with_result boundary: Playa's native
      decoder rejects the fake bytes before opening a device and falls back to
      the host player.
    - The harness is Unix-only: on Windows, `Command::new("kokoro-tts")`
      resolves only `.exe`. A stub-`.exe` route would need CI delivery of
      extra binaries (as messenger does), which the plan's "no new CI
      cell/gate" rules out. Windows coverage stays with portable unit tests,
      as the spec allows.
- **Locked spool wiring:** confirmed end to end by
  `locked_spool_receives_a_ready_job_for_a_cached_file`. A fake synthesis
  fills the fixture cache, `Speak::play_detached` publishes a `ready` file
  job, `clear_pending` removes it, and nothing synthesizes again.

### Requirement-to-test map (Phase 1)

| Requirement | Test |
| --- | --- |
| Transport seam preserves gender-match → first-voice → configured-fallback selection (characterization for Phase 3) | `elevenlabs::tests::selection_prefers_the_first_voice_matching_the_requested_gender`, `selection_without_a_gender_match_uses_the_first_listed_voice`, `selection_falls_back_to_the_configured_voice_when_listing_fails_or_is_empty` |
| Explicit voice/model skip the voice list; model reaches the request | `a_requested_voice_and_model_skip_the_voice_list` |
| Fake counts requests; a changed account voice list changes selection | `a_changed_account_voice_list_changes_the_next_selection` |
| Clock seam | `fake_clock_reads_the_time_it_was_set_to`, `system_clock_reads_the_current_unix_time` |
| Say resolver seam: placeholder is unresolved; fake pins and updates values | `say::system_defaults::tests::production_lookup_leaves_both_settings_unresolved`, `fake_lookup_reports_the_latest_pinned_settings` |
| Fixture owns PATH, temp/audio cache, capability cache, spool; restores env | `prepare::fixture_owns_every_location_synthesis_can_touch` |
| Each fake parses its real tool's argument grammar; listings are not syntheses | `prepare::fakes_record_each_tool_argument_shape` |
| Failure/hang behaviors leave a partial file; hung child is observable and killable | `prepare::failing_and_hanging_fakes_leave_partial_output_behind` |
| Fixture drives a real provider: one synthesis for two calls, cache under fixture tmp, mpv boundary (`playa`) / `NoAudioPlayer` (no `playa`), spool absent | `prepare::kokoro_synthesizes_once_into_the_fixture_cache` |
| Locked spool receives a ready job without playback | `prepare::locked_spool_receives_a_ready_job_for_a_cached_file` |

The Input Robustness Matrix does not apply yet: this phase adds no parser or
configuration reader. The Spike A output grammar above is the input for the
Phase 3 matrix.

### Gates run

- `just lint` in `biscuit-speaks/`: pass.
- `just test` in `biscuit-speaks/` (with `playa`): 506 passed, 19 skipped.
- `cargo nextest run -p biscuit-speaks` (without `playa`): 366 passed, 10
  skipped. The one slow test is the existing `real_say_provider_speaks_muted`
  (about 6 s), which `just test` excludes.
- `cargo clippy -p biscuit-speaks --all-targets`, with and without `playa`:
  no warnings.
- `just test` in `claudine/`: 8738 passed, 9 skipped. `just lint` in
  `claudine/`: pass. Its only warning is the pre-existing macOS linker
  `__eh_frame` note.
- `just cross-check biscuit-speaks --os linux`: pass, including all `prepare`
  tests.
- Windows and WSL were not cross-checked. The new test binary is
  `#![cfg(unix)]`, and the library changes (an `Arc`-held trait object, an
  async trait with `Send` futures) have no platform-specific code.

### Open items for the author (not blocking Phase 2)

1. Ruling 5 (ElevenLabs account context strength) still needs a decision
   before Phase 3, Wave 6.
2. Spike A's rate-tracking check needs a manual System Settings change on a
   Mac before Phase 3, Wave 5 implements `OsSpeechDefaults`.

## Phase 2

### What landed

- **`lib/src/prepared.rs`** (new, crate-private):
    - `ResolvedSynthesis`: one variant per file-producing provider (ruling 1), externally tagged JSON (`{"kokoro":{"voice":"af_heart"}}`), `deny_unknown_fields`. The `Say` and `ElevenLabs` variants carry `#[allow(dead_code)]` until Phase 3 constructs them.
    - `dry_run_enabled()` is the only `PLAYA_DRY_RUN` reader. It is not gated on `playa`. `Speak::play_detached` now calls it, and the test-only duplicate in `test_support.rs` is gone.
    - `staging_file(cache_path)` and `publish(staged, cache_path)`: synthesis writes `.biscuit-speaks-staging-*` beside the entry, then `TempPath::persist` renames it into place. Dropping the staging file deletes it.
- **`TtsExecutor::prepare`** has a `Send` no-op default and is not gated on `playa`.
- **Kokoro, gTTS, Echogarden**:
    - Each has one resolver, a `cache_key` builder, and a `generate_to_cache`/`synthesize` that consults the cache, stages the output, and publishes it. `speak`, `speak_with_result`, `detached_job`, `cached_detached_job`, and `prepare` all go through them.
    - Children run with `kill_on_drop(true)`, so a cancelled future kills the tool.
    - The crate-private `prepare_resolved` returns `Option<ResolvedSynthesis>`, which `Speak::prepare` retains. The trait `prepare` discards it.
- **`Speak`**:
    - `audio`/`audio_format` are replaced by `prepared: Option<(TtsProvider, ResolvedSynthesis)>`.
    - `with_voice/gender/language/failover/config` clear it. `with_volume` and `Clone` keep it.
    - `prepare` order: dry run (return `self` unchanged) → strategy → no candidates ⇒ `NoProvidersAvailable` → first candidate only. The provider's error propagates. eSpeak, SAPI, Say, and ElevenLabs currently go through the trait default (no-op). Enum-only providers return `ProviderFailed{"Provider not yet implemented"}` (ruling 10).
    - Removing the in-memory audio also removed `play_with_result`'s "not supported for prepared audio" error path (ruling 3).

### Departures from the plan (spec unchanged)

1. **No `Resolution` enum, no `cache_path_for(&ResolvedSynthesis)`.** `prepare_resolved` returns `Option<ResolvedSynthesis>`, where `None` means nothing was prepared (dry run, or an unresolved default). Each provider's `cache_key` takes the resolved values directly, so Phase 4 builds the key from the retained variant's fields. One fewer type, same contract.
2. **Kokoro resolution is unified on the language-aware path.** Before, `speak`/`detached_job` used a gender-only default, while `speak_with_result` selected from the built-in list by language and gender. Every entry point now uses the list selection, which needs no subprocess. English results are identical (`af_heart`/`am_adam`). Non-English languages now get a matching voice in `speak` too (for example `ja` ⇒ `jf_alpha`).
3. **Echogarden resolution** (replaces the `voice.unwrap_or("default")` key):
    - The order is: requested voice, then the Kokoro engine's English gender default (`Heart`/`Michael`), then the best match from `echogarden list-voices <engine>`. If none of these yields a voice, the result is **unresolved**.
    - Unresolved audio is synthesized into `.biscuit-speaks-uncached-*.wav` with no `--voice`, never read from or published to the cache, and deleted after foreground playback. Playa may delete only spool-owned files, so the detached path keeps that file, as Say's unresolved detached audio already does.
    - Behavior change: `speak` for VITS or non-English now passes the selected `--voice`, as `speak_with_result` already did. `cached_detached_job` can spawn `echogarden list-voices` in the requesting process for those cases. The English Kokoro default stays subprocess-free.
    - `speak_with_result` reports the voice name `default` for unresolved audio.
4. **Echogarden no longer re-applies speed at playback.** This was pulled forward from Phase 4's volume/speed task because the Wave 3 Echogarden item required it. Foreground and detached playback use `SpeedLevel::Normal`. **ElevenLabs still passes speed through to playback (applied twice); Phase 4 must fix it.**
5. Playback does **not** yet try the retained settings first (Phase 4). Reuse between the returned and fresh objects works today because both resolve the same shared key.

### Requirement-to-test map (Phase 2)

| Requirement | Test(s) |
| --- | --- |
| Trait `prepare` is a `Send` no-op default; both feature configs compile | `traits::tests::default_prepare_is_a_send_no_op`; clippy `-D warnings` with and without `playa` |
| `ResolvedSynthesis` round-trips; malformed records rejected (missing, null, wrong type, unknown field, unknown variant, trailing content) | `prepared::tests::every_variant_survives_a_json_round_trip`, `the_wire_shape_names_the_provider_and_its_settings`, `missing_null_mistyped_and_unknown_fields_are_rejected` |
| Dry-run spellings are exactly `1`/`true`/`TRUE`/`yes` | `prepared::tests::dry_run_accepts_exactly_playas_spellings` |
| Staging is deleted on drop; publish replaces atomically | `prepared::tests::a_dropped_staging_file_leaves_nothing_and_a_published_one_replaces_the_entry` |
| Acceptance 2 + 3 (Kokoro/gTTS/Echogarden): two direct prepares plus `Speak::prepare` ⇒ one synthesis, one complete file, no staging leftovers, nothing played, spool/journal absent; returned and fresh objects reuse the file through `play`, `play_with_result` (provider, voice, `cache_hit`, path), and `play_detached` (ready job) | `prepare::kokoro_prepared_file_is_reused`, `gtts_prepared_file_is_reused`, `echogarden_prepared_file_is_reused` |
| Acceptance 5 (halves): volume not in the key, voice is; prepared file plays at playback volume | `prepare::synthesis_settings_change_the_key_and_volume_does_not` |
| Acceptance 5: baked speed applied once (Echogarden `--speed=1.25` at synthesis, `--speed=1` at playback); Kokoro speed at playback only | `prepare::baked_speed_is_applied_once_and_playback_speed_is_applied_at_playback` |
| Golden key spellings kept | `kokoro::tests::the_cache_key_keeps_its_existing_spelling_and_ignores_speed_and_volume`, `gtts::tests::the_cache_key_keeps_its_existing_spelling`, `echogarden::tests::the_cache_key_keeps_its_existing_spelling_and_bakes_in_speed` |
| Resolution rules | `kokoro::tests::resolve_voice_honors_the_language_preference` (+ existing gender tests), `echogarden::tests::a_non_english_kokoro_request_is_left_to_the_voice_list`, `playback_does_not_apply_baked_speed_again` |
| Unresolved default ⇒ no-op prepare, `is_prepared` false, only a listing ran, playback synthesizes uncached every time, no cache entry or leftover file | `prepare::an_unresolvable_echogarden_default_is_a_no_op_and_plays_uncached` |
| Acceptance 6 (half): the first candidate's error propagates, no other provider runs, partial output not published | `prepare::the_first_candidates_error_propagates_without_trying_another` |
| Acceptance 7: clone/volume keep, other inputs clear; no-op/dry run/unresolved stay false | `speak::tests::clone_and_volume_keep_preparation`, `synthesis_inputs_clear_preparation`, plus the dry-run/no-op/unresolved tests |
| Acceptance 8: dry run before any discovery, even for an unavailable provider; builder and direct calls; no subprocess, cache, capability cache, spool; existing preparation kept | `prepare::dry_run_prepares_nothing_before_any_discovery`, `dry_run_keeps_an_existing_preparation` (both binaries run with and without `playa`) |
| Acceptance 9 (halves): cancellation kills the child (state read with `ps`, so a zombie is not mistaken for a live process) and leaves no entry; concurrent prepares leave one complete file, duplicates allowed | `prepare::cancelled_preparation_kills_the_child_and_publishes_nothing`, `concurrent_preparation_publishes_a_complete_file` |
| eSpeak/SAPI keep the default: no speech subprocess, no file; `Speak::prepare` succeeds without preparing | `prepare::direct_speakers_prepare_as_a_no_op` |
| Ruling 10: enum-only provider errors | `prepare::an_enum_only_provider_reports_that_it_is_not_implemented` |
| No candidate ⇒ `NoProvidersAvailable` | `speak::tests::prepare_with_no_candidate_reports_no_providers` |

Regression evidence: before this phase the providers wrote straight to the
cache path, so the cancellation and failure tests would find a partial
`biscuit-speaks-*` entry. Echogarden playback passed `--speed=1.25` to the
player. The Phase 1 test `kokoro_synthesizes_once_into_the_fixture_cache` asserted the
old direct write; it now asserts the staging sibling is renamed away.

The harness (`tests/common/mod.rs`) gained `FakeTool::ESpeakNg`,
`FakeTool::Piper`, and `process_running(pid)`.

The Input Robustness Matrix does not apply yet. `ResolvedSynthesis` gains a
file reader only when Phase 4 puts it in `PreparationRecord`, and its decoding
already rejects each malformed shape above. The ElevenLabs snapshot and the Say
lookup matrices belong to Phase 3. Smell grep on the new paths: the only
`unwrap_or_default` is the staging-file suffix (not a parse), and there is no
`#[serde(default)]`.

### Gates run

- `just test` in `biscuit-speaks/` (with `playa`): 535 passed, 19 skipped.
- `cargo nextest run -p biscuit-speaks -E 'not test(/real_/)'` (without `playa`): 390 passed, 14 skipped.
- `just lint` in `biscuit-speaks/`, plus `cargo clippy -p biscuit-speaks --all-targets -D warnings` with and without `playa`: clean.
- `just test` in `claudine/`: 8738 passed, 9 skipped. `just lint` in `claudine/`: clean apart from the pre-existing macOS `__eh_frame` linker note.
- `just cross-check biscuit-speaks --os linux`: pass (417). `--os windows`: pass (398), covering the portable `prepared.rs` tests on native Windows.
- WSL was not cross-checked: no WSL-specific code changed.
- No skipped or failing tests other than the existing `real_*`/ignored ones.
- Local-run note: `cd biscuit-speaks && …` from a shell already in `biscuit-speaks/` silently ran `just lint` against the main checkout (`/Volumes/coding/personal/rusty-biscuit`). Every gate above was re-run with absolute paths and shows worktree paths.

### Docs touched

- `biscuit-speaks/docs/tts-caching.md`: corrected the statements this phase made false. Providers no longer "write directly to cache path"; publication is staged and atomic. Per-provider identity is documented, including Echogarden's unresolved default. Baked speed is applied once for Say and Echogarden, and ElevenLabs still passes speed through. The full preparation write-up is Phase 5.
- `.claude/skills/biscuit-speaks/api-reference.md`: `prepare` returned a nonexistent `PreparedSpeech`. It now returns `Self`, and `is_prepared` is added. The pre-existing drift is resolved in favor of the code.
- No change to the `claudine` skill was needed; Claudine's code is untouched.

## Phase 3

### Author questions: no answer arrived, so the plan's fallbacks apply

- **Say rate (ruling 8):** Option B. The production lookup resolves the system
  *voice* and reports the system *rate* as unresolved. Explicit-speed Say
  requests now prepare and cache. Normal-speed Say requests stay uncached and
  behave as before. The lookup still prints the rate, so enabling it later is a
  one-line change in `OsSpeechDefaults::rate_wpm` plus a test, once someone
  confirms the rate follows the Spoken Content slider.
- **ElevenLabs account context (ruling 5):** implemented as written. The
  context is the API base URL, plus the name of the environment variable the
  key comes from (in the client's precedence order), plus the optional
  `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` label.

### What landed

- **Say** (`providers/host/say.rs`, `say/system_defaults.rs`):
    - `SayProvider::resolve(config, defaults) -> Resolution { voice, rate_wpm }`
      is the one resolution path. Voice order: requested, then gender
      (`Alex`/`Samantha`), then the system voice. Rate order: an explicit speed
      (175 × multiplier), then the system rate. The rate is resolved first, and
      the system voice is looked up only when the rate is known, so a
      normal-speed request runs no lookup at all.
    - `cache_key(text, voice, rate)` keeps the existing spelling
      `"{voice};rate={wpm}"`. `synthesize` consults the cache, stages the
      output, and publishes it with `prepared::staging_file`/`publish`.
      Unresolved requests go to `.biscuit-speaks-uncached-*.wav` with only the
      known flags. Foreground playback deletes that file afterwards; a
      detached job keeps it (`TempPath::keep`, as Echogarden does).
    - `speak`, `speak_with_result`, `detached_job`, `cached_detached_job`, and
      `prepare_resolved` all use these. Ordinary playback (with Playa, or with
      `afplay` without `playa`) now reads and publishes the cache. The legacy
      `system-default` key and the path-based unique key in `detached_job` are
      gone.
    - `OsSpeechDefaults::voice` runs the Spike A JXA lookup through
      `osascript`. It parses one JSON object strictly, then requires the voice
      in `say -v '?'`.
    - `run_say` rejects empty output, so an empty file can never be published
      as speech.
- **ElevenLabs** (`providers/cloud/elevenlabs.rs`, new
  `elevenlabs/snapshot.rs`):
    - `select(config) -> Selection { voice, model, source }` replaces
      `resolve_voice_id`. Behavior is unchanged: a requested voice wins, then a
      gender match, then the first listed voice, then the configured default
      for an empty list or a failed listing. `source` is
      `Requested`/`Account`/`Fallback`.
    - `select_and_record` writes a record for every `Account` selection made in
      `speak`, `speak_with_result`, `detached_job` (the helper), and `prepare`.
    - Records are `.biscuit-speaks-elevenlabs-voice-{xx_hash}.json` beside the
      `BISCUIT_SPEAKS_CACHE` file, published with `write_atomic`. They hold
      `voice, model, inputs{gender, language, model, fallback}, account_context,
      written_at` and have no credential field. They are trusted when
      `written_at <= now < written_at + 24h` and every field matches.
    - `cached_path`, which `cached_detached_job` uses, needs no API request:
      it takes the requested voice, or the voice from a trusted record. Any
      other case is a miss and goes to today's helper path.
    - `prepare_resolved` always selects through the API (refreshing the
      record). A failed voice listing is unresolved and makes `prepare` a
      no-op.
    - Speed is no longer applied twice. `speak`, `speak_with_result`, and the
      detached jobs play at `SpeedLevel::Normal` (`playback_config`).
- `Speak::prepare_provider` now calls `prepare_resolved` for Say and
  ElevenLabs. The `dead_code` allowances on `ResolvedSynthesis::Say` and
  `ElevenLabs` are removed.
- `cache::cache_file_path` is now `pub(crate)` so records can share its
  directory.

### Departures from the plan (spec unchanged)

1. **`SystemSpeechDefaults` has separate `voice()`/`rate_wpm()` methods**
   instead of Phase 1's single `lookup()`. With the rate unresolved, a single
   lookup would have run the roughly 0.6 s `osascript` call on every
   normal-speed `speak` only to discard the result. The separate methods match
   the plan's original sketch.
2. **Say `speak_with_result` now speaks with the resolved voice.** Before, it
   always picked the best listed voice, even for a gender, so it could differ
   from `speak`, which used `Alex`/`Samantha`. Now a gender selects
   `Alex`/`Samantha` in both, which preserves the spec's "Say's gender
   choices". When no voice resolves (no gender, and the system voice is
   unknown), it keeps its old behavior of naming and speaking the best listed
   voice, so metadata still names the actual voice.
3. **Record names hash `(inputs, account_context)` as JSON**, the identity
   ruling 6 describes. The fields are also checked after reading, so a
   mismatched or tampered record is a miss even when its name matches.
4. **No snapshot from `generate_audio`.** That public method returns bytes
   and never touches the audio cache, so it does not record a selection.
5. **ElevenLabs speed fix pulled forward from Phase 4.** The Phase 2 handoff
   asked for it "when you touch its playback". The per-provider speed table
   test in Phase 4 still applies.

### Verification of the real macOS lookup

- Phase 1 established that the JXA lookup reproduces `say`'s no-flag output
  byte for byte.
- This phase verified the exact case production now uses. On macOS 27.2,
  `say -r 219` with no voice and `say -v "Ava (Premium)" -r 219` produced the
  same SHA-1 (`2d6137ba…`), and `-v Samantha -r 219` differed.
- `real_system_voice_lookup_names_a_listed_voice` runs the real lookup and
  checks `say` lists the voice. It is excluded from `just test` (`real_`), and
  it passed on this host.

### Requirement-to-test map (Phase 3)

| Requirement | Test(s) |
| --- | --- |
| Acceptance 3 (Say): fresh object reuses the prepared file through `speak` (Playa → `mpv`, no `playa` → `afplay -v`), `speak_with_result` (provider, voice, `cache_hit`, path), and the detached lookup; preparation plays nothing; spool absent | `prepare::say_prepares_the_system_voice_and_reuses_it_everywhere` (both feature configs) |
| Acceptance 2 (Say): two prepares synthesize once, one complete file, no staging leftovers | same test; `say::tests::resolved_system_defaults_prepare_once_and_a_changed_default_misses` |
| Acceptance 4 (Say): no requested voice + normal speed returns a ready lookup when both defaults resolve; gender choices preserved; unresolved voice or rate gives no file and no cache lookup | `say::tests::resolution_keeps_requested_and_gender_voices_and_reads_only_missing_defaults`, `an_unresolved_system_setting_leaves_the_request_unresolved`, `resolved_system_defaults_prepare_once_and_a_changed_default_misses`, `unresolved_system_defaults_prepare_nothing`; `prepare::unresolved_say_defaults_prepare_nothing_and_never_use_the_cache` (also proves a normal-speed request runs no `osascript`, and that a legacy `system-default` entry is never played) |
| Acceptance 4 (ElevenLabs): default-voice prepare, then a fresh detached lookup returns a ready job with zero API requests | `elevenlabs::tests::a_prepared_default_voice_is_found_by_a_fresh_detached_lookup_without_any_request` |
| Acceptance 4 (ElevenLabs characterization, written in Phase 1, still green after the refactor) | `selection_prefers_the_first_voice_matching_the_requested_gender`, `selection_without_a_gender_match_uses_the_first_listed_voice`, `selection_falls_back_to_the_configured_voice_when_listing_fails_or_is_empty`, `a_requested_voice_and_model_skip_the_voice_list`, `a_changed_account_voice_list_changes_the_next_selection` |
| Acceptance 5: changed default makes a fresh object miss while the retained pair still names the file; rate and voice are in the key, volume is not | `say::tests::resolved_system_defaults_prepare_once_and_a_changed_default_misses`, `prepare::a_changed_system_voice_makes_a_fresh_say_object_miss`, `say::tests::the_cache_key_keeps_its_existing_spelling_and_ignores_volume`, `volume_control::say_foreground_preserves_voice_and_applies_output_volume` (a volume change reuses the entry) |
| Acceptance 5: baked speed applied once (ElevenLabs sends 1.2 to the API, the key differs, the detached job plays at 1.0; Say plays at `--speed=1`) | `elevenlabs::tests::baked_speed_reaches_the_api_and_the_key_but_not_playback`, the `detached_hit` helper's speed assertion, `prepare::say_prepares_the_system_voice_and_reuses_it_everywhere` |
| Acceptance 7: Speak retains Say only when both settings resolve | `prepare::speak_retains_say_preparation_only_when_both_settings_resolve` (macOS, because Speak selects Say only there) |
| Acceptance 8: dry run before any lookup (Say direct and builder, ElevenLabs direct) | `prepare::dry_run_prepares_nothing_before_any_discovery` (now includes Say and `osascript`), `say::tests::dry_run_prepares_nothing_before_any_lookup`, `elevenlabs::tests::dry_run_prepares_nothing_and_makes_no_request` |
| Acceptance 9 (Say): failure publishes nothing; empty output is an error | `prepare::a_failed_say_synthesis_publishes_nothing`, `say::tests::empty_output_is_an_error_and_is_never_published` |
| Acceptance 11: a second prepare re-checks through the API even with a fresh record; a changed voice list updates the record | `elevenlabs::tests::every_prepare_rechecks_the_selection_and_a_changed_voice_list_updates_the_record` |
| Acceptance 11: expired (exactly 24 h), future, other account, no account, other inputs (language, fallback), corrupt, missing each miss with zero requests; the helper then resolves through the API and the record is trusted again | `elevenlabs::tests::untrusted_records_miss_until_the_helper_resolves_through_the_api` |
| Acceptance 11: the record holds no credential (sentinel key and its hash absent); the context names the variable, honors the label, follows the client's precedence, and is `None` without a key | `elevenlabs::tests::the_account_context_and_record_hold_no_credential` |
| Ruling 4: a failed listing prepares and records nothing; a requested voice prepares without listing or recording | `a_failed_voice_listing_prepares_and_records_nothing`, `a_requested_voice_prepares_without_listing_or_recording` |
| Normal synthesis records the selection (helper path) | `elevenlabs::tests::helper_synthesis_records_the_default_voice` |
| Input Robustness Matrix: ElevenLabs record | `elevenlabs::tests::every_malformed_record_shape_is_a_miss` |
| Input Robustness Matrix: Say lookup output | `say::system_defaults::tests::only_the_exact_lookup_shape_resolves_a_voice` |

#### Input Robustness Matrix results

- **ElevenLabs record.**
    - The test walks a record the provider really wrote. The control row is a
      hit. Restoring the original file is a hit again, which proves each
      edit was load-bearing.
    - For each of `voice`, `model`, `inputs`, `account_context`, and
      `written_at`, the edits absent, `null`, wrong type, empty (`""` or
      `{}`), and duplicate key (same value) are all misses.
    - Each of the four `inputs` fields gets absent, `null`, wrong type, empty,
      and duplicate key. The `inputs` object as a whole also gets an unknown
      field and an array in declaration order.
    - Whole-file and timing rows: an unknown top-level field, trailing
      garbage, a second document, the fields as an array in declaration
      order, an empty file, stale, future, a negative time, a mismatched
      account, mismatched inputs, and `model` differing from `inputs.model`.
- **Mutation checks (temporary edits, all reverted).**
    - Removing the freshness check fails the expiry rows.
    - Removing the object-only deserializer at either level fails the matching
      "array in declaration order" row. serde's derived structs accept a JSON
      array, so a plain `#[derive(Deserialize)]` would have taken it. This is
      why `snapshot::object_only` exists.
- **Say lookup output.**
    - The control row prints the real host's output, with and without a
      trailing newline.
    - For `voice` and for `rate`, the rows absent, `null`, wrong type, and
      duplicate are each unresolved.
    - `voice` also gets empty and blank. `rate` also gets zero, negative, and
      fractional.
    - Whole-output rows: an unknown field, `{}`, an array, trailing content,
      two objects, empty output, and osascript error text.
    - The array row failed before the leading-brace check was added.
- **Smell grep** (`#[serde(default)]`, `unwrap_or_default`, `.ok()`,
  `filter_map`) on `snapshot.rs` and `system_defaults.rs`. Each hit is
  intentional:
    - `unwrap_or_default` on the optional account label: an absent label means
      no label. This is a setting, not a record field.
    - `.ok()?` on reading the record: missing or unreadable is a miss, as the
      spec requires.
    - `.ok()?` on serializing our own types: it cannot fail.
    - `.ok()?` on Say's voice listing: a failure leaves the voice unresolved.
    - There is no `#[serde(default)]`.

### Regressions found and fixed during this phase

- `volume_control::say_foreground_preserves_voice_and_applies_output_volume`
  had no `TMPDIR` isolation, and its fake `say` wrote nothing.
    - Once Say cached concrete settings, the `playa` run published an empty
      cache entry into the host's real temp directory. The next run hit it,
      so `say` never ran and the test failed.
    - Fixes: Say rejects empty output; the test owns its `TMPDIR` and asserts
      the second volume reuses the entry; the fake writes bytes. I deleted the
      one stray 0-byte `biscuit-speaks-efc13211163837c6.wav` my run had
      created in this Mac's temp directory.
    - `detached_phase4::say_detached_job_preserves_lossless_arguments`'s fake
      also writes bytes now.
- The ElevenLabs test fixture first set only `TMPDIR`. On native Windows,
  `temp_dir()` reads `TMP`/`TEMP`, so the first `cross-check` wrote fake mp3
  entries into `build-win-native`'s real temp directory, and later runs hit
  them.
    - The fixture now sets all three, and the trap is recorded in the `os`
      skill.
    - A few small `biscuit-speaks-*.mp3` files from that run remain in the
      Windows host's user temp directory. They are harmless; I did not log in
      to remove them.

### Gates run

- `just test` in `biscuit-speaks/` (with `playa`): 558 passed, 20 skipped.
- `cargo nextest run -p biscuit-speaks -E 'not test(/real_/)'` (without
  `playa`): 412 passed, 15 skipped.
- `just lint` in `biscuit-speaks/`, and `cargo clippy -p biscuit-speaks
  --all-targets -D warnings` with and without `playa`: clean.
- `just test` in `claudine/`: 8738 passed, 9 skipped. `just lint` in
  `claudine/`: clean apart from the existing macOS `__eh_frame` linker note.
- `just cross-check biscuit-speaks --os linux`: pass (439). `--os windows`:
  pass (412) after the `TMP`/`TEMP` fix. One cross-check attempt aborted
  because it snapshotted the tree while Claudine's tests were writing temp
  files under `claudine/gen/`; the retry passed.
- WSL was not cross-checked: this phase has no WSL-specific code.
- No skipped or failing tests other than the existing `real_*` and ignored
  ones. The slowest L1 tests are the existing real-`say` listing tests (about
  4 s under load).

### Docs and skills touched

- `biscuit-speaks/docs/tts-caching.md`: corrected the statements this phase
  made false. Say now caches in foreground playback, not only detached, and
  says when. ElevenLabs no longer applies speed twice. Say no longer uses
  `write_atomic`. The ElevenLabs default-voice record and its 24-hour window
  get one sentence. The full preparation write-up is Phase 5.
- `.claude/skills/biscuit-speaks/SKILL.md`: Say caching rules, the
  empty-output rule for fakes, the new harness helpers, `with_account_context`,
  and the Windows `TMP`/`TEMP` trap. `configuration.md`: Say added to the speed
  tables.
- `.claude/skills/os/windows.md`: `temp_dir()` ignores `TMPDIR` on Windows.
- No change to the `claudine` skill was needed; Claudine's code is untouched.

### Open items for the author (not blocking Phase 4)

1. Say's system rate stays unresolved until someone confirms on a Mac that a
   changed Spoken Content rate changes what the JXA lookup reports. The steps
   are in the Phase 1 section. Until then, `prepare` does nothing for
   normal-speed Say speech, which is the most common Say configuration for
   Claudine's pre-warm.
2. Ruling 5's known limit stands: two API keys under the same variable name
   share records unless `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` tells them apart.

## Phase 4

### What landed

- **Resolved entry points per provider.** Kokoro, gTTS, Echogarden, Say,
  and ElevenLabs each gained crate-private `speak_resolved`,
  `speak_resolved_with_result`, `resolved_job`, and `cached_resolved_job`.
  They take settings that are already resolved. Each trait method
  (`speak`, `speak_with_result`, `detached_job`, `cached_detached_job`) now
  resolves once and delegates to them, so the public paths keep their
  behavior and share one body with retained playback. Echogarden's and Say's
  entry points still accept an unresolved request, which plays uncached as
  before. Say's `Resolution` became `pub(crate)`, with a
  `Resolution::retained(voice, rate)` constructor.
- **`ResolvedSynthesis` dispatch** (`prepared.rs`): `provider()`, `speak`,
  `speak_with_result`, `job` (helper only), and `cached_job` (no subprocess
  or request). Each goes straight to the provider that resolved the
  settings: an `EchogardenProvider` with the retained engine, or
  `ElevenLabsProvider::new()?` with no voice-list request.
- **`Speak::play` / `play_with_result`** try the retained settings first.
  On failure they run the strategy's *other* providers with the original
  config (`fallback_providers` removes the prepared provider). An empty
  strategy with no preparation is still `NoProvidersAvailable`.
  `SpecificProvider` therefore ends with one error and never switches.
- **`Speak::play_detached`**: a retained cache hit enqueues a ready job.
  On a miss, it reserves a slot whose record carries the retained settings.
  Synthesis never runs in the requesting process.
- **`PreparationRecord.prepared: Option<ResolvedSynthesis>`**
  (`detached.rs`):
    - The record uses `deny_unknown_fields`, `skip_serializing_if = none`,
      and a `present` deserializer, so an absent key decodes as `None` (an
      older or unprepared record) and an explicit `null` is an error.
    - The helper (`prepare_job`) tries the retained settings first, then the
      strategy's other providers with the record's config.
- **`run_preparation` fails an undecodable record's slot immediately**
  (`invalid_preparation_record`). Before, it returned without publishing,
  which left the slot `preparing` until the 10-minute deadline held up the
  queue.

### Departures from the plan (spec unchanged)

1. **`Speak` stores `Option<ResolvedSynthesis>`, not ruling 1's
   `Option<(TtsProvider, ResolvedSynthesis)>`.** Each variant already names
   its provider (`ResolvedSynthesis::provider()`), so the pair was redundant.
   Carrying the pair in the helper record would have added a "provider and
   settings disagree" shape to reject. The retained state and its
   clear/keep rules are unchanged.
2. **Test-only spawn capture.** `spawn_preparation` records the record path
   instead of spawning while a `#[cfg(test)]` static is armed. In a test,
   `current_exe()` is the libtest binary, which would rerun its whole suite
   as the "helper". The seam is compiled out of every non-test build.
3. **Pre-existing flaky test fixed.** `prepare::fakes_record_each_tool_argument_shape`
   (Phase 1) wrote the text to every fake's stdin, but only `say` reads it.
   On Linux a fake could exit first, and the write failed with `EPIPE`. This
   happened on 2 of 6 cross-check runs; the error was captured from one. It
   now pipes stdin only to `say`.

### Requirement-to-test map (Phase 4)

| Requirement | Test(s) |
| --- | --- |
| Acceptance 3: returned **and** fresh objects reuse the file through `play`, `play_with_result` (provider, voice, `cache_hit`, path), and `play_detached` (ready job), with no further synthesis | `prepare::{kokoro,gtts,echogarden}_prepared_file_is_reused` (helper extended to the returned object's `play_with_result` and `play_detached`) |
| Acceptance 5: the returned object keeps its settings after the default changes (no lookup, cache hit, ready detached job); a fresh object follows the new default and misses | `prepare::a_prepared_say_object_keeps_its_settings_after_the_system_voice_changes` (macOS, since `Speak` selects Say only there); `elevenlabs::tests::retained_settings_skip_the_voice_list_and_apply_speed_once` (changed account voice list: retained `f1` still used, zero list requests) |
| Acceptance 5: speed applied once, one row per provider (Kokoro and gTTS at the player; Echogarden baked into synthesis, player at 1) | `prepare::retained_playback_applies_speed_exactly_once_per_provider`; ElevenLabs row: `retained_settings_skip_the_voice_list_and_apply_speed_once` (API gets 1.2, job plays at 1.0); Say row: `a_prepared_say_object_keeps_its_settings…` (`-r 219`, player `--speed=1`) |
| Acceptance 5: volume stays out of the key, and a prepared file plays at playback volume | existing `prepare::synthesis_settings_change_the_key_and_volume_does_not` (now through retained playback) |
| Acceptance 6: fallback uses the original config, never the retained voice, and does not retry the prepared provider; metadata names the fallback | `prepare::a_failed_prepared_provider_falls_back_with_the_original_configuration` (Echogarden gets `--voice=Michael`, never `am_adam`; both feature configs) |
| Acceptance 6: `SpecificProvider` never switches | `prepare::a_specific_provider_does_not_fall_back_after_a_failed_retained_attempt`; helper side in `detached::tests::handoff::the_helper_prefers_retained_settings_and_falls_back_with_the_original_configuration` |
| Acceptance 6: a missing file is synthesized again through the correct path | `prepare::a_missing_prepared_file_is_synthesized_again_with_the_retained_settings`; `elevenlabs::tests::retained_settings_skip_the_voice_list…` |
| Acceptance 6: retained settings survive the helper handoff; the requester does not synthesize | `detached::tests::handoff::a_prepared_object_whose_file_vanished_hands_its_settings_to_the_helper` (record holds `{"kokoro":{"voice":"am_adam"}}`, slot `preparing` → `ready`); `…the_helper_prefers_retained_settings…` (helper synthesizes the retained `bf_emma`, not the configured `af_heart`) |
| Acceptance 9: cancellation during retained resynthesis kills the child and publishes nothing; concurrent playback leaves one complete file, duplicates allowed | `prepare::retained_resynthesis_is_cancellable_and_concurrent_publication_is_complete` |
| Acceptance 9: a failed retained or fallback attempt leaves no partial entry | the fallback test (one complete Echogarden entry) and the specific-provider test (temp directory empty) |
| Record compatibility and robustness | `detached::tests::prepared_settings_round_trip_and_an_old_record_decodes_as_unprepared`, `every_malformed_prepared_shape_is_rejected`, `handoff::an_undecodable_record_fails_its_slot` |
| `provider()` mapping | `prepared::tests::each_variant_names_the_provider_that_resolved_it` |

Detached tests own a `test_toolkit::LockedAudioSpool`, so nothing plays,
and every pending job is cleared. No test opens a terminal or browser
window.

**Mutation check.** I disabled the three retained-first branches in
`Speak::play`/`play_with_result`/`play_detached` and the helper (files
restored afterward). Ten of the new or extended tests failed, including the
Say retention, handoff, helper-preference, specific-provider, and speed-row
tests. The fallback test also passes under the old behavior, because Kokoro's
resolution is deterministic; it is a contract test, not a discriminator.

#### Input Robustness Matrix: `PreparationRecord.prepared`

The control row (`{"kokoro":{"voice":"am_adam"}}`) decodes. Every other row
must be rejected, and a rejected record fails its slot.

| Shape | Outcome |
| --- | --- |
| absent | decodes as `None`; the format defines this as an unprepared or older record |
| explicit `null` | rejected |
| wrong type, whole field (`"kokoro"`, `7`, the pair as an array) | rejected |
| wrong type, one element (`voice: 7`, `voice: null`) | rejected |
| wrong type, every element (`say` with `voice: 1`, `rate_wpm: "fast"`) | rejected |
| missing field (`say` with no `rate_wpm`) | rejected |
| empty (`{}`, `[]`) | rejected |
| unknown field, unknown provider, two providers | rejected |
| misspelled sibling key (`retained`) | rejected (`deny_unknown_fields`); not read as absent |
| duplicate key / trailing content | **not reachable at this layer.** Playa's `read_preparation_record` parses the private spool file and passes back a `serde_json::Map`, which keeps only the last of a duplicate key. The file is written by the same binary into a per-user private spool. Rejecting duplicates would be a Playa reader change and is out of this phase's scope. |

Smell grep on the new paths: one `#[serde(default)]` on `prepared`, which is
intentional and paired with the `present` deserializer, so only an absent key
defaults. No `unwrap_or_default`, `.ok()`, or `filter_map` was added to a
decode path.

### Gates run

- `just test` in `biscuit-speaks/` (with `playa`): 571 passed, 20 skipped.
- `cargo nextest run -p biscuit-speaks -E 'not test(/real_/)'` (without
  `playa`): 418 passed, 15 skipped.
- `just lint` in `biscuit-speaks/`, and `cargo clippy -p biscuit-speaks
  --all-targets -D warnings` with and without `playa`: clean.
- `just test` in `claudine/`: 8738 passed, 9 skipped.
- `just cross-check biscuit-speaks --os linux`:
    - Before the stdin fix, it failed twice in six runs on the pre-existing
      broken-pipe race above.
    - After the fix, 11 test runs passed (451 tests each).
    - Two more attempts never ran tests: `build-linux` timed out reaching
      github.com over SSH.
- `--os windows`: pass (416).
- WSL was not cross-checked; nothing in this phase is WSL-specific.
- The new tests each run in 0.25 s or less. The slow L1 tests (over 5 s
  under full-suite load) are the existing real-`say` listing tests and
  `cache::tests::test_populate_cache_for_all_providers_runs_without_panic`;
  none is new.

### Docs and skills touched

- `biscuit-speaks/docs/tts-caching.md`: retained-first playback, the helper
  handoff of retained settings, the immediate failure of an undecodable
  record, and a returned-vs-fresh example. The example uses an explicit speed,
  because normal-speed Say never prepares. The full preparation write-up
  remains Phase 5.
- `.claude/skills/biscuit-speaks/SKILL.md`: the same facts, compactly, plus
  how to test the helper (the `#[cfg(test)]` capture and in-process
  `run_preparation`).
- Source docs: `Speak::play`, `play_with_result`, `play_detached`, the
  `prepared` field, `PreparationRecord`, and the new entry points.
- No change to the `claudine` skill was needed; Claudine's code is untouched.

### Open items for the author (not blocking Phase 5)

- The two Phase 3 items still stand: Say's system rate is unresolved, and
  ruling 5's account-context limit applies.
- Duplicate keys in a spool record are last-wins in Playa's reader (see the
  matrix). This is pre-existing Playa behavior, recorded here so a reviewer
  can decide whether Playa should reject them.

## Phase 5

### What landed

- **`biscuit-speaks/README.md`**: a new "Preparing speech ahead of time"
  section. It has a compact returned-vs-fresh example (explicit speed, with the
  caveat that normal-speed Say never prepares) and covers first-provider-only
  errors, no-op cases and `is_prepared`, which setters keep or clear
  preparation, fallback with the original config, dry run, and the ElevenLabs
  24-hour default-voice record. The environment table gains
  `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` and the full `PLAYA_DRY_RUN` value list.
- **`biscuit-speaks/docs/tts-caching.md`**: a new "Preparing Speech" section
  replaces the short Phase 4 passage under "Playback Flow". It covers:
    - a Mermaid flowchart of prepare → resolve → publish/reuse → retain, and a
      fresh object's hit or miss;
    - the returned object versus a fresh object, `is_prepared`, and the
      setter rules;
    - a table of every no-op case;
    - dry run, including direct `TtsExecutor::prepare`, with and without
      `playa`;
    - concurrency and billing;
    - playback after preparation (retained first, fallback with the original
      config, helper handoff, speed applied once);
    - Say's system-default lookup and the unread rate;
    - the ElevenLabs default-voice record: location, contents, account
      context, 24-hour trust window, refresh on every `prepare`, and no
      credential.

  `blast_radius` now lists the files that carry this behavior. Neither doc
  names or links the feature directory.
- **Skills**: the four files below hold the same facts, kept compact.
    - `SKILL.md`: a Preparation block under Caching, `prepare` in the trait
      sketch, and a `prepare` line in Quick Start.
    - `api-reference.md`: the private preparation state, setter rules, and the
      `TtsExecutor::prepare` default.
    - `providers.md`: caching and `prepare` for Say, and the ElevenLabs
      default-voice record.
    - `configuration.md`: volume is never in a key, and unresolved defaults are
      never cached.

### Drift found and fixed (code wins)

1. **`audio_cache.rs`** module table had no Say row. Added; it notes that
   Say's WPM rate is carried in the voice ID rather than the speed field.
2. **`Speak::prepare` doc example** asserted `prepared.is_prepared()`. That is
   false after a no-op (eSpeak, an unresolved default), so the example now
   says so instead of asserting.
3. **Say and Echogarden module docs** said nothing about caching. Each gets
   one paragraph on its key, and on the unresolved default that is never
   cached and makes `prepare` a no-op.
4. **README**: "Temporary synthesis files are removed before handoff" was
   false for Say's unresolved detached audio, which keeps its uncached
   temporary file for Playa. Reworded.
5. **Skills documented `Speak::with_speed`, which does not exist.** Speed is
   set on `TtsConfig`. This drift predates the feature. Fixed in `SKILL.md`
   Quick Start, `api-reference.md`, and `configuration.md`'s speed examples.
6. **Rustdoc warnings** (`RUSTDOCFLAGS="-D warnings" cargo doc`):
    - `cache.rs` linked to `cache_file_path`, which Phase 3 made
      `pub(crate)`. That is a public-to-private link, now plain code.
    - `lib.rs` had an ambiguous `[speak]` link (both a module and a function)
      that predates the feature. It is now `mod@speak`.

   No gate runs rustdoc with `-D warnings`. Both were one-token fixes.

The stub TODO and the stale `is_prepared` doc were already gone; earlier phases
removed them.

### Comment-quality check

- I scanned every comment line the feature added under `biscuit-speaks/lib/src`
  (diff against `2a1e0270f`) for HOW-narration and quoted format strings.
- No format-string or glyph narration was found.
- The new `speak_resolved*` bodies carried narration comments that moved with
  the code: "Generate to cache (or get cached file)", "Play the audio file",
  "Find the voice with this ID", "Ensure the voice has its identifier set",
  and "Return the result with model info…". I deleted 10 such lines in
  `gtts.rs`, `kokoro.rs`, `echogarden.rs`, and `elevenlabs.rs`.
- I left untouched the same style of comment in `is_ready` bodies the feature
  did not change (Rule 3).

These are comment-only edits; no rendering, constant, or logic line changed.

### Requirement-to-test map (Phase 5)

Phase 5 is acceptance criterion 10 (docs). It changes no behavior, so it adds
no tests. Verification is by review, plus these checks:

| Requirement | Evidence |
| --- | --- |
| README, caching topic page, and skills describe preparation, retained settings, no-op behavior, cache reuse, the ElevenLabs record and its 24-hour window | The sections listed above; facts checked against `speak.rs`, `prepared.rs`, `snapshot.rs`, `say.rs`, and `say/system_defaults.rs` |
| Compact returned-vs-fresh example | README "Preparing speech ahead of time"; tts-caching "Preparing Speech" |
| One Mermaid diagram of the prepare → retain → fresh-hit flow | tts-caching "The returned object and a fresh object" |
| Docs never name the feature directory | `grep -n "2026-10-19\|speak-prepare" biscuit-speaks/README.md biscuit-speaks/docs/tts-caching.md .claude/skills/biscuit-speaks/*.md` finds nothing |
| Source docs agree with code | Drift list above; rustdoc `-D warnings` clean with default features and with `--no-default-features` |

### Gates run

- `just lint` in `biscuit-speaks/`: clean.
- `cargo clippy -p biscuit-speaks --all-targets --no-default-features -D warnings`:
  clean.
- `just test` in `biscuit-speaks/`: 571 passed, 20 skipped.
- `just lint` in `claudine/`: clean. The only warning is the existing macOS
  linker note about `__eh_frame` size.
- `just test` in `claudine/`: 8738 passed, 9 skipped. One test is slow,
  `error_guards::production_sources_pass_every_scan_backed_guard` (more than
  5 s). It predates this phase and is not a `biscuit-speaks` test.
- No cross-OS run: Phase 5 changes only comments and Markdown, so nothing
  OS-specific changed.

### Notes

- `SKILL.md` is 217 lines. It was already 201 before this phase, over the
  200-line guideline for skill entry points. The new Preparation block is one
  paragraph; moving more content into `api-reference.md` would be a separate
  cleanup.
- No change to the `claudine` skill was needed: Claudine's code and behavior
  are untouched.

## Phase 6

Final validation. No source, docs, or skill file changed in this phase; every
gate below ran against the tree Phases 1–5 left.

### Gates run

| Gate | Result |
| --- | --- |
| `just test` in `biscuit-speaks/` (L1, `--features playa` + CLI) | 571 passed, 20 skipped |
| `just test-l2` in `biscuit-speaks/` | "not applicable" stub; the area has no L2 tier |
| `just lint` in `biscuit-speaks/` | clean |
| `cargo clippy -p biscuit-speaks --all-targets --no-default-features -- -D warnings` (no `playa`; `default = []`) | clean |
| `cargo nextest run -p biscuit-speaks --no-default-features` (no tier filter, so `real_` Say tests ran too) | 423 passed, 10 skipped |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p biscuit-speaks --no-deps` | clean |
| `just doctest` in `biscuit-speaks/` (covers the Phase 5 `Speak::prepare` doc example edit) | all pass |
| `just test` in `claudine/` | 8738 passed, 9 skipped |
| `just lint` in `claudine/` | clean apart from the existing macOS linker note about `__eh_frame` size |
| `just cross-check biscuit-speaks --os linux` | pass, 451 tests |
| `just cross-check biscuit-speaks --os windows` | pass, 416 tests |
| `just check-tier-coverage biscuit-speaks` | 0 stranded, 0 stubbed tiers with tests |

WSL was not cross-checked: no code in this feature is WSL-specific, and the
Linux run covers the same `cfg(unix)` paths.

### `just ci-local --plan`

- The plan is computed for the whole branch against `origin/main`. It includes
  601 changed paths, most of them from other work on this branch
  (darkmatter, claudine, CI scripts).
- The rows for `biscuit-speaks` are its standard set: `lint` and `check` on
  ubuntu-latest, and `L1` on all four environments. The `check` row exists
  because of the area's existing example target, and it also compiles
  `biscuit-speaks-cli`, `research`, and `research-cli`.
- The feature adds no new cell, tier, gate, or target:
    - `git diff origin/main..HEAD -- biscuit-speaks/lib/Cargo.toml biscuit-speaks/cli/Cargo.toml`
      is empty.
    - The new `lib/tests/prepare.rs` is auto-discovered (the lib does not set
      `autotests = false`).
    - `cargo nextest list` shows all 29 `prepare` tests selected by L1.
- One row comes from this feature's files without being part of the feature:
  `test-toolkit/ubuntu-latest/lint` runs the archive-path guard because
  `biscuit-speaks/lib/src/audio_cache.rs` changed. That file's only change was
  a Phase 5 doc row. The guard runs alone and is cheap.
- The `biscuit-speaks/justfile` diff against `origin/main` (a `slow-tests`
  recipe) was not made on this branch; `main` added it after the branch point.
- Platform gates: Say's tests are `cfg(target_os = "macos")` and ran locally.
  The Linux and Windows cross-checks compiled every target with those gates
  off. The feature adds no SAPI code.

### Smell grep (snapshot and Say-lookup parse paths)

I grepped `snapshot.rs`, `say/system_defaults.rs`, `prepared.rs`, and
`detached.rs` for `#[serde(default)]`, `unwrap_or_default`, `.ok()`, and
`filter_map`. There are no `#[serde(default)]` or `filter_map` hits. Every
other hit is outside the record and lookup parsing:

| Site | Why it is acceptable |
| --- | --- |
| `snapshot.rs` `account_context`: `env::var(ACCOUNT_LABEL_ENV).unwrap_or_default()` | An unset label and an empty label give the same context. This is an environment variable, not a field of the record. |
| `snapshot.rs` `snapshot_path`: `serde_json::to_string(..).ok()?` | Serializing two plain structs cannot fail, and `None` only means "no record location". |
| `snapshot.rs` `load_trusted`: `fs::read(..).ok()?` | A record that is missing or unreadable is a cache miss. The bytes themselves go through `parse`, which is strict: `deny_unknown_fields`, object-only, `end()`, and no defaults. |
| `system_defaults.rs`: `list_voices().await.ok()?` | A failed voice listing leaves the default voice unresolved, so preparation does nothing. That is the outcome the spec requires. |
| `prepared.rs` `staging_file`: `extension().unwrap_or_default()` | Builds a temp-file suffix and has nothing to do with parsing. |

Phase 3 recorded the Input Robustness Matrix for both parse paths
(`elevenlabs::tests::every_malformed_record_shape_is_a_miss`,
`say::system_defaults::tests::only_the_exact_lookup_shape_resolves_a_voice`).
Phase 4 recorded it for `PreparationRecord.prepared`. Nothing has changed
since then, so the matrix stands.

### Closing summary for review

**Spike A result.** AppKit's `NSSpeechSynthesizer`, called through JXA,
returns the voice that `say` uses with no `-v`. The proof is a byte-identical
comparison on macOS 27.2. It is the only lookup trusted, and only when its
output has exactly the expected shape and the voice is in `say -v '?'`. The
lookup also reports a rate, but that rate is not trusted: on the test host it
equaled the library's built-in 175 WPM, so it could not be shown to follow
the user's setting.

**Ruling outcomes.**

- Rulings 1–11 were adopted.
- Ruling 8 (Say rate): the plan's fallback, Option B, applies. The voice is
  resolved and the system rate is left unresolved, so normal-speed Say is
  never prepared.
- Ruling 5 (ElevenLabs account context): implemented as written. It is the
  API base URL, plus the name of the key's environment variable, plus the
  optional `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` label.

**Departures from the plan.** The spec was left unchanged; the full entries
are in the earlier phases.

- Phase 1: a single async system-defaults lookup. Phase 3 changed it back to
  separate `voice()` and `rate_wpm()` methods.
- Phase 2:
    - no `Resolution` enum;
    - Kokoro and Echogarden voice resolution is unified across entry points;
    - Echogarden's unresolved audio is uncached;
    - Echogarden no longer applies speed a second time.
- Phase 3:
    - Say's `speak_with_result` speaks with the resolved voice;
    - record file names hash the selection inputs and the account context;
    - `generate_audio` writes no record;
    - the ElevenLabs speed fix was pulled forward from Phase 4.
- Phase 4:
    - `Speak` stores `Option<ResolvedSynthesis>` rather than a pair with the
      provider;
    - a test-only seam captures the helper spawn instead of running it;
    - a pre-existing broken-pipe flake in the Phase 1 harness was fixed.

**Open items for the reviewer.** None of these blocks review.

1. Say's system rate stays unresolved until someone checks, on a Mac, that
   changing the Spoken Content rate changes what the lookup reports. The
   steps are in the Phase 1 section. Enabling it then is a one-line change in
   `OsSpeechDefaults::rate_wpm`, plus a test.
2. Ruling 5's known limit: two ElevenLabs keys read from the same variable
   name share default-voice records unless the account label tells them
   apart. The 24-hour trust window and the refresh on every `prepare` limit
   the effect.
3. Playa's spool-record reader keeps the last value of a duplicate key. This
   predates the feature; whether Playa should reject duplicates is a separate
   decision.

### Requirement-to-test map (Phase 6)

Phase 6 covers acceptance criterion 12 (gates) and the "compile both
configurations" half of criterion 1. It changes no behavior, so it adds no
tests. The evidence is the gate table above. Criteria 1–11 map to the tests
listed in the Phase 2–4 requirement tables.

### Skills

No skill changes. Claudine's code is untouched, so the `claudine` skill does
not need an update. Phase 5 already brought the `biscuit-speaks` skill in line
with the code.

**Terminal state: implementation complete, ready for review.** The feature
directory stays where it is; moving it to `_completed` is the author's step.

### Definition of Done

I checked off the plan's four Definition-of-Done boxes. One caveat on the
first: criteria 10 (docs) and 12 (gates) have no L1 test. Criterion 10 is
shown by the Phase 5 review and the clean rustdoc run, and criterion 12 by the
gate table above. Criteria 1–9 and 11 have passing L1 tests, mapped in the
Phase 2–4 tables. The stub TODO and the stale `is_prepared` doc are gone; a
grep of `speak.rs` and `prepared.rs` for TODO/FIXME finds nothing.

## Review cycle 1 departure from the spec

The spec requires the ElevenLabs snapshot to carry "the account context needed
to tell callers apart". The non-secret context from the original
implementation (API address, key variable name, optional label) could not tell
two keys stored under the same variable apart. A record could not be keyed on
the key itself either, because the spec forbids persisting or hashing
credentials. The context is now `None` unless
`BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` holds a non-blank label. `None` counts as
uncertainty under the spec's snapshot rules: no record is written, and a fresh
detached default-voice lookup misses, so the helper resolves the voice through
the API. The audio cache still avoids a second synthesis once the voice is
resolved. Network-free default-voice reuse (criteria 4 and 11) therefore needs
that label. The docs and the `biscuit-speaks` skill state this. The spec is
left as decided.
