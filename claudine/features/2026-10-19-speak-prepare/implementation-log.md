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
packages:
    - biscuit-speaks
human_review: false
message_to_agent: |-
    Phase 1 left scaffolding for you; read the "## Phase 1" log section first.

    - Test harness: `biscuit-speaks/lib/tests/common/mod.rs` (`TtsFixture`, `FakeTool`, `Behavior`), used by `lib/tests/prepare.rs`. Put Phase 2 prepare/cache tests in `prepare.rs` (Unix-only; `#[serial_test::serial]` on every fixture test). `FakeTool::Mpv` is the play/play_with_result boundary: Playa fails to decode the fake bytes and falls back to the fake `mpv`. `assert_spool_absent()` covers both spool and journal. `lock_spool()` is for detached publication. Note that Kokoro/gTTS/Echogarden today write straight to the cache path, so `FailAfterPartialWrite` currently leaves a partial cache entry; Phase 2 must fix that.
    - ElevenLabs: `ElevenLabsProvider::with_transport(Arc<dyn ElevenLabsTransport>, Arc<dyn Clock>)` is `#[cfg(test)]`, and the fakes are in `elevenlabs::fake`. `Speak` and `detached.rs` still construct `ElevenLabsProvider::new()` directly, so fresh-`Speak` ElevenLabs tests in Phase 3/4 need a route to inject the fake there. The `clock` field and `Clock` trait carry `#[allow(dead_code, reason = "scaffolding…")]`; remove it when the snapshot reads the clock.
    - Say: `providers/host/say/system_defaults.rs` has `SystemSpeechDefaults::lookup()` (async, returns voice + rate together; this departs from the plan's two sync methods, see the log), `OsSpeechDefaults` (placeholder, always unresolved), and `FakeSpeechDefaults`. Remove its module-level `#![allow(dead_code…)]` once it is wired. Spike A found the lookup (JXA `NSSpeechSynthesizer.defaultVoice` + `rate`, byte-verified against plain `say`), but rate tracking after a System Settings change is unverified. Do not implement `OsSpeechDefaults` for the rate until that is checked.
    - Rulings 1-11 stand. Rulings 5 and 8 have open author questions that matter for Phase 3, not Phase 2.
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
