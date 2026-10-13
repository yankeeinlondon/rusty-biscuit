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
message_to_agent: |-
    Phase 3 is in; read the "## Phase 3" log section, especially "Departures" and "Regressions found and fixed".

    - All five file producers now have `prepare_resolved`, and `Speak::prepare_provider` calls each of them. The retained `ResolvedSynthesis` variants map to these entry points for Phase 4:
        - `Say{voice, rate_wpm}`: `SayProvider::synthesize(text, &Resolution { voice: Some(voice), rate_wpm: Some(rate) })`. It returns `Synthesized::Cached` and re-synthesizes into the cache if the file vanished.
        - `ElevenLabs{voice, model, speed}`: `generate_to_cache(text, &voice, &model, speed)`.
        - Play both through their `playback_config` (speed `Normal`).
    - ElevenLabs no longer applies speed twice: `speak`, `speak_with_result`, and both detached paths use `playback_config`. Phase 4's per-provider speed test should still cover it.
    - The Say system rate is unresolved by design (the author's rate question went unanswered; ruling 8 fallback). Normal-speed Say never prepares, and Phase 4 must not "fix" that. The system voice does resolve, through `osascript`.
    - Say `speak_with_result` now speaks with the resolved voice (a gender gives Alex/Samantha). It keeps the old best-listed-voice choice only when nothing resolves.
    - ElevenLabs records: `cached_path` (no network) backs `cached_detached_job`, and `select_and_record` runs in every synthesis path. In-crate tests need `with_transport(...).with_account_context(Some(..))` plus the `RecordFixture` pattern.
    - Any test fixture that isolates the audio cache must set TMPDIR, TMP, and TEMP. On Windows, `temp_dir()` ignores TMPDIR, and that leaked into the build host in this phase.
    - Fake `say` scripts must write bytes to `-o`: Say now rejects empty output.
    - Run gates with absolute paths; a cross-check can abort if Claudine's tests are writing temp files at the same moment.
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
