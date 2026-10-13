---
name: biscuit-speaks
description: Cross-platform text-to-speech library with multi-provider support and automatic failover. Use when implementing TTS features, working with voice synthesis, integrating speech providers (ElevenLabs, macOS Say, eSpeak, Kokoro, Echogarden, gTTS, SAPI), or building the so-you-say CLI.
---

## Purpose

`biscuit-speaks` provides unified TTS functionality across multiple providers with:
- Automatic provider detection and failover
- OS-aware default provider stacks
- Voice capability caching
- Audio file caching (content-addressed via xxHash)
- Native-first Playa playback for file-producing providers when `playa` is enabled
- Durable, globally serialized detached speech via `Speak::play_detached`
- Builder pattern APIs for ergonomic configuration

## Quick Start

```rust
use biscuit_speaks::{Speak, TtsConfig, Gender, VolumeLevel, SpeedLevel};

// Simple usage - speaks with best available provider
Speak::new("Hello, world!").play().await?;

// With configuration (speed is set on TtsConfig; Speak has no with_speed)
let config = TtsConfig::new()
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_volume(VolumeLevel::Soft)
    .with_speed(SpeedLevel::Fast);
Speak::new("Custom voice").with_config(config).play().await?;

// Synthesize into the audio cache now, play later
let prepared = Speak::new("Done").prepare().await?;
prepared.play().await?;

// Get metadata about what was used
let result = Speak::new("Hello").play_with_result().await?;
println!("Used: {} via {:?}", result.voice.name, result.provider);
```

## Key Types

| Type | Purpose |
|------|---------|
| `Speak` | Builder for TTS operations |
| `TtsConfig` | Configuration container (voice, gender, language, volume, speed, failover) |
| `Voice` | Voice metadata (name, gender, quality, languages, identifier) |
| `TtsProvider` | Enum: `Host(HostTtsProvider)` or `Cloud(CloudTtsProvider)` |
| `TtsFailoverStrategy` | `FirstAvailable`, `PreferHost`, `PreferCloud`, `SpecificProvider(TtsProvider)` |
| `SpeakResult` | Metadata returned after speaking, including optional `SpeakPlaybackReport` |
| `SpeakPlaybackReport` | Always-compiled route, duration, and completion projection for file playback |

## Traits

```rust
// Required for all providers
pub trait TtsExecutor: Send + Sync {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError>;
    async fn speak_with_result(&self, text: &str, config: &TtsConfig) -> Result<SpeakResult, TtsError>;
    async fn is_ready(&self) -> bool;
    fn info(&self) -> &str;
    // Default: Ok(()) without work (eSpeak, SAPI)
    async fn prepare(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError>;
}

// Optional for voice enumeration
pub trait TtsVoiceInventory: Send + Sync {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError>;
}
```

## Provider Selection

Cloud providers (ElevenLabs) are NOT used automatically - must be explicitly requested via `TtsFailoverStrategy::SpecificProvider` or `PreferCloud`.

OS-specific default stacks (highest quality first):
- **macOS**: Say -> Kokoro -> EchoGarden -> Piper -> Sherpa -> ESpeak -> Gtts
- **Linux**: Kokoro -> EchoGarden -> Sherpa -> Piper -> Mimic3 -> ESpeak -> Festival -> SpdSay -> Gtts
- **Windows**: Sapi -> Piper -> EchoGarden -> Kokoro -> Sherpa -> ESpeak -> Gtts

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ELEVENLABS_API_KEY` / `ELEVEN_LABS_API_KEY` | ElevenLabs API authentication |
| `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` | Non-secret per-account label; required for network-free detached reuse of a default ElevenLabs voice |
| `KOKORO_MODEL` | Path to `kokoro-v1.0.onnx` model file |
| `KOKORO_VOICES` | Path to `voices-v1.0.bin` voice embeddings |
| `TTS_PROVIDER` | Override default provider selection |
| `PREFER_LANGUAGE` | Default language preference |
| `PREFER_GENDER` | Default gender preference |
| `PREFER_VOICE` | Default voice preference |
| `PREFER_SPEED` | Default speed (`fast` or `slow`) |

## Caching

**Voice Capability Cache** (`~/.biscuit-speaks-cache.json`):
- Persists detected providers and available voices
- Prevents expensive re-enumeration

**Audio File Cache**:
- Content-addressed using xxHash
- Cache key: provider + voice_id + text + format + speed (conditional)
- Atomic writes to prevent corruption

**Preparation** (`Speak::prepare`; rules in `biscuit-speaks/docs/tts-caching.md`):
first available provider only, its error returned, nothing played or spooled.
No-op success (`is_prepared()` false) for eSpeak/SAPI, dry run (checked before
discovery), and unresolved defaults (normal-speed Say, failed Say voice lookup,
Echogarden engine default, failed ElevenLabs listing). A *fresh* `Speak` with
the same resolved settings hits the same file (Claudine's pre-warm relies on
it); `with_volume` keeps preparation, other `with_*` setters clear it.
ElevenLabs default-voice records sit beside the capability cache file, trusted
24 h, keyed by selection inputs + non-secret account context, and exist only
while `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` is a non-blank per-account label
(unset/blank: a fresh detached call misses and the helper re-lists voices);
`prepare` always re-lists voices.

## Playback and detached preparation

The `playa` feature also enables `playa/native-playback`. File-producing
providers (Say, Kokoro, EchoGarden, gTTS, ElevenLabs) use the native route first and
fall back to ranked host players. Their `play_with_result` response includes a
`SpeakPlaybackReport`; direct streaming providers (SAPI, eSpeak) leave
the field absent because their completion duration is unverified.

`Speak::play_detached` returns after publishing a ready job or reserving an
ordered preparation slot in Playa's private per-user spool. Cache misses are
synthesized by an internal `so-you-say` helper and atomically transition the
same slot to ready or failed. The ten-minute preparation deadline prevents one
stuck synthesis from blocking the queue forever, while reservation prevents
later audio from overtaking it. Playback is serialized across all participating
processes and survives requester exit. Delivery is best-effort after handoff.

A `Speak` returned by `prepare` plays through its retained provider and
settings first (`play`, `play_with_result`, `play_detached`), skipping default
lookups; on failure, the strategy's *other* providers run with the original
config (`SpecificProvider` never switches). A vanished prepared file is
resynthesized with the retained settings, in the helper for `play_detached`:
the preparation record's optional `prepared` key carries them (absent = an
unprepared or older record; `null` or any malformed value fails the slot at
once). Tests of the helper use `detached.rs`'s `#[cfg(test)]` capture of the
spawned record and call `run_preparation` in-process: spawning the test binary
would rerun its whole suite.

Detached support is concrete for Kokoro, EchoGarden, gTTS, ElevenLabs, macOS
`say`, Windows SAPI, and eSpeak. Deferred enum-only providers return
`TtsError::DetachedUnsupported`. Preparation records may contain speech text;
status and journal projections redact it. `PLAYA_DRY_RUN=1` creates no cache,
spool, journal, or subprocess side effects.

## Detailed Documentation

- [Providers Reference](providers.md) - All supported TTS providers with configuration details
- [API Reference](api-reference.md) - Complete type definitions and usage patterns
- [Configuration Guide](configuration.md) - Builder patterns, failover strategies, caching

## Related Packages

- **biscuit-speaks-cli**: CLI that uses biscuit-speaks (`so-you-say` binary, located at `biscuit-speaks/cli`)
- **playa**: Audio playback library (optional feature for `biscuit-speaks`)
- **sniff-lib**: System detection for available TTS providers

## Volume and silent tests

`VolumeLevel::Explicit(0.0)` is mute; all concrete providers honor the normalized
output gain. eSpeak uses `-a` scaled to 0–100 (100 is unity, not its amplified
200 maximum), and SAPI uses native percentage volume. Say synthesizes PCM WAV
with the same selected voice/rate, then uses native Playa at the requested gain;
without `playa`, it uses macOS `afplay -v`. Say caches only when both the voice
and the integer WPM rate are resolved (requested, gender, explicit speed, or the
system voice via `osascript`/`NSSpeechSynthesizer`); the system *rate* is
deliberately unresolved until the author confirms it tracks Spoken Content, so
normal-speed Say speech is never cached. Unresolved audio goes to a
`.biscuit-speaks-uncached-*` temp file. A fake `say` must write bytes to its
`-o` path: Say refuses empty output rather than publish it to the cache.
Never apply playback speed a second time to Say's, Echogarden's, or
ElevenLabs' already rate-adjusted output.

Real speech tests must use zero volume and explicit test-message wording. They
are named `real_*` — including the ones inside `#[cfg(test)] mod tests` in
`lib/src/providers/host/`, which the `test(/(^|::)real_/)` filterset selects
through their module path — and must never be `#[ignore]`d, because no
canonical recipe runs ignored tests. `just test-real` enables `playa`, which is
what makes a `SpeakPlaybackReport` (route plus completion verdict) available to
assert instead of `Result::is_ok()`. A missing backend skips through
`test_support::skip_or_require`; `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts`
names the providers whose absence must instead fail, and composes with Playa's
all-or-nothing `PLAYA_REAL_AUDIO_REQUIRED=1`.

Kokoro completion coverage pins `af_heart`, deliberately independent of Claudine's
normal provider/voice. `lib/tests/volume_control.rs` uses recording executables
and invalid audio to verify zero and intermediate gains without audible output.
New synthesis and cache tests use the shared `lib/tests/common` `TtsFixture`
instead: it owns `PATH`, the temp dir, `BISCUIT_SPEAKS_CACHE`, and an
uncreated `PLAYA_SPOOL_DIR`, installs recording `say`/`kokoro-tts`/`gtts-cli`/
`echogarden` fakes that synthesize, fail, or hang after a partial write, and an
`mpv` fake that is the playback boundary, because Playa falls back to it when
native decoding of the fake bytes fails (`afplay` is the boundary for Say
without `playa`). `set_say_voices` scripts the `say -v '?'` listing and
`set_system_voice_lookup` scripts the fake `osascript` behind Say's system-voice
lookup. The fakes are `/bin/sh` scripts, so
these tests are Unix-only: Windows resolves a bare program name only to an
`.exe`. ElevenLabs tests inject `fake::FakeTransport`/`FakeClock` through the
crate-private `ElevenLabsProvider::with_transport` instead of making network
calls; `with_account_context` turns on its default-voice records, which live
beside the `BISCUIT_SPEAKS_CACHE` file. Tests that need the production context
calculation build through `new`/`with_base_url` and swap transport and clock
(`production_provider`); one that sets and drops key or label variables first
takes `EnvGuard::remove_safe` guards for them (`without_api_keys`), because
dropping a `set_safe` guard restores the developer's own value, not absence. An in-crate fixture that isolates the
audio cache must set `TMP` and `TEMP` as well as `TMPDIR`: Windows'
`temp_dir()` ignores `TMPDIR`, and a `TMPDIR`-only fixture wrote into, and then
hit, the Windows host's real temp directory.

CLI fixtures must also set `BISCUIT_SPEAKS_CACHE` to a private voice-capability
cache and supply a controlled provider inventory. `--voice Samantha` alone did
not pin the Say fixture: the user's cached inventory resolved it to
`Samantha (Enhanced)`. The fixture now answers `say -v '?'` with only Samantha
and keeps that inventory cache private. Preserve production voice matching;
isolate test inventories so personal voice installations cannot change what a
test selects. This voice-capability cache is separate from the synthesized-audio
cache controlled by the fixture's `TMPDIR`/`TMP`/`TEMP` settings.
