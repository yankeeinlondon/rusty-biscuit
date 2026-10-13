---
blast_radius:
  - biscuit-speaks/lib/src/audio_cache.rs
  - biscuit-speaks/lib/src/cache.rs
  - biscuit-speaks/lib/src/playback.rs
  - biscuit-speaks/lib/src/playa_bridge.rs
  - biscuit-speaks/lib/src/types.rs
  - biscuit-speaks/lib/src/providers/host/kokoro.rs
  - biscuit-speaks/lib/src/providers/host/gtts.rs
  - biscuit-speaks/lib/src/providers/host/echogarden.rs
  - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
  - biscuit-speaks/cli/src/main.rs
---

# TTS Caching

The biscuit-speaks library uses two independent caching systems, both implemented at the library level. The CLI (`so-you-say`) is a consumer only -- it does not implement caching logic.

## Audio File Cache

The audio file cache avoids redundant TTS generation by storing rendered audio in the system temp directory. When the same text is spoken with the same provider, voice, format, and (where applicable) speed, the cached file is returned immediately instead of re-rendering.

### Cache Key

A `CacheKey` (`audio_cache.rs`) is built from:

| Component | Description |
|-----------|-------------|
| Provider  | Name string (e.g., `"kokoro"`, `"gtts"`, `"elevenlabs"`) |
| Voice ID  | Provider-specific identifier |
| Text      | The text to be spoken |
| Format    | Audio file extension (`"wav"`, `"mp3"`) |
| Speed     | Optional -- only for providers that bake speed into audio |

The key is hashed with xxHash (via `biscuit-hash`) to produce a 16-character hex string. The final cache path is:

```
{temp_dir}/biscuit-speaks-{hash}.{format}
```

For example: `/tmp/biscuit-speaks-a1b2c3d4e5f6g7h8.wav`

### Speed Inclusion Rules

Whether speed is included in the cache key depends on how the provider handles speed:

| Provider    | Speed in Key | Reason |
|-------------|--------------|--------|
| Kokoro      | No           | Playa adjusts playback speed at play time |
| gTTS        | No           | Playa adjusts playback speed at play time |
| EchoGarden  | Yes          | Speed baked into audio via `--speed` flag |
| ElevenLabs  | Yes          | Speed baked into audio via API parameter |
| Say         | Yes          | Speed baked into PCM WAV by `say -r` |

Providers where playa handles speed can share a single cached file across different speed settings. Providers that bake speed into the generated audio need separate cache entries per speed value.

### Provider Integration

Kokoro, gTTS, EchoGarden, and ElevenLabs implement a `generate_to_cache` method following the same pattern:

1. Build a `CacheKey` from provider-specific inputs
2. Call `cache_exists()` -- if `true`, return `(cache_path, true)` immediately
3. Generate the audio into a staging file beside `cache_path`, then rename it into place (or use `write_atomic()` for byte data)
4. Return `(cache_path, false)`

A cache entry therefore only ever appears complete. If synthesis fails, or the
caller drops the future mid-synthesis, the tool's child process is killed and
the staging file (named `.biscuit-speaks-staging-*`) is deleted:

```text
kokoro-tts input.txt /tmp/.biscuit-speaks-staging-a1B2c3.wav --voice af_heart
  ok   -> rename to /tmp/biscuit-speaks-<hash>.wav
  fail -> delete staging file, no cache entry
```

The return tuple is `(PathBuf, bool)` where the bool indicates a cache hit. This flows into `SpeakResult.cache_hit` which the CLI displays as `Cache: hit` or `Cache: miss`.

Providers that use audio caching:

- **Kokoro** -- generates WAV via `kokoro-tts`; keyed by the voice, which is the requested voice or the best built-in voice for the language and gender
- **gTTS** -- generates MP3 via `gtts-cli --output`; keyed by the language code
- **EchoGarden** -- generates WAV via `echogarden speak`; keyed by engine, voice, and speed. The voice is the requested one, the Kokoro engine's English default for the gender (`Heart` or `Michael`), or the best match from `echogarden list-voices <engine>`. When none of these yields a voice, echogarden picks its own default, which can change outside this library, so that audio is synthesized into a temporary file and never cached
- **ElevenLabs** -- fetches MP3 bytes from API, uses `write_atomic()` to persist; keyed by voice, model, and speed. Without a requested voice, the voice is selected from the account's voice list, and each selection from a successful listing is recorded locally (beside the voice-capability cache file) for 24 hours so a detached request can find the file without listing voices again
- **Say** -- generates PCM WAV via `say -o`; keyed by the voice and the exact integer words per minute, in foreground and detached playback alike. The voice is the requested one, `Alex`/`Samantha` for a gender, or the system voice read through AppKit's `NSSpeechSynthesizer`; the rate comes from an explicit speed (175 WPM times the multiplier). The system's own rate is not read yet, so normal-speed Say speech is synthesized into a temporary file and never cached. Volume is applied at playback

Audio for a voice or rate the library cannot pin down (an unresolved system
default) is never read from or published to the cache, since those defaults can
change independently of the request. Foreground playback deletes that temporary
file afterwards; a detached job keeps it, because Playa deletes only files it
owns. Say never reads legacy `system-default` cache entries.

Providers that do **not** use audio caching:

- **eSpeak** -- speaks directly through the audio system, no intermediate file
- **SAPI** (Windows) -- speaks directly through the audio system, no intermediate file

### Atomic Writes

The `write_atomic()` function in `audio_cache.rs` prevents partial writes from corrupting cache files:

1. Write data to a uniquely-named temp file in the same directory (using PID + timestamp)
2. `sync_all()` to flush to disk
3. `rename()` to the final path (atomic on POSIX filesystems)

ElevenLabs uses this for API response bytes. The CLI-based providers have their tool write a staging file beside the entry and rename it into place, as described above.

### Cache Lifetime

Files live in the OS temp directory and are cleaned up by the operating system on its own schedule. There is no explicit TTL, eviction policy, or manual cleanup. The cache is opportunistic -- it helps when repeated expressions happen before the OS clears temp files.

## Provider Capability Cache

A separate cache at `~/.biscuit-speaks-cache.json` stores discovered TTS provider capabilities (installed voices, available voices). This avoids expensive re-enumeration of the host system on every TTS operation.

### Location

`BISCUIT_SPEAKS_CACHE` names the cache file directly and takes priority over the home-directory default; an empty value is treated as unset. Its parent directory must already exist.

Because environment is inherited by child processes, setting it also confines
the detached preparation helper used by `--background` cache misses.

`cache_file_path()` reads the variable; `resolve_cache_path()` applies the precedence rule and is what the tests exercise, so the rule can be asserted without mutating process-global environment state.

### Structure

The cache file is a JSON envelope with:

- `schema_version` -- integer for forward compatibility (currently `1`). A version mismatch causes a cache miss, forcing re-enumeration.
- `capabilities` -- a `HostTtsCapabilities` struct containing a list of `HostTtsCapability` entries, each with a provider, its installed voices, and its available (not-yet-installed) voices.
- `last_updated` -- Unix timestamp of the last write.

### Operations

| Function | Purpose |
|----------|---------|
| `read_from_cache()` | Load capabilities; returns error on missing file, parse failure, or schema mismatch |
| `update_provider_in_cache()` | Upsert a single provider's voice data |
| `bust_host_capability_cache()` | Delete the cache file entirely |
| `populate_cache_for_all_providers()` | Re-enumerate all providers and rebuild the cache |

All writes use the temp file + rename pattern (via `tempfile::NamedTempFile::persist()`).

### CLI Interaction

The CLI exposes `--refresh-cache` which:

1. Calls `bust_host_capability_cache()` to delete the file
2. Calls `populate_cache_for_all_providers()` to rebuild from scratch
3. Continues with normal operation (does not exit early)

This is the only cache the CLI interacts with directly. The audio file cache is fully transparent to the CLI.

## Playback Flow

After caching, the audio file is played through `playback.rs`, which bridges to
Playa's native-first builder pipeline:

1. Provider returns `(PathBuf, cache_hit)` from `generate_to_cache()`
2. `play_audio_file()` converts biscuit-speaks types to playa types via `playa_bridge.rs`
3. `Playa::play_async_with_report()` performs playback and returns route/timing evidence

The playa bridge maps volume (`VolumeLevel` to `f32`) and speed (`SpeedLevel` to `f32`) for providers where playa controls playback speed. Say, EchoGarden, and ElevenLabs already contain the requested tempo, so they play at normal speed and the speed is applied once.

For detached cache misses, the requested sequence is reserved before synthesis.
A private helper creates or reuses the cache file, atomically publishes the
ready job into the same slot, or records failure. The preparation deadline is
ten minutes; later audio cannot overtake the reserved sequence while it is
pending. Preparation records may contain speech text and are never projected
into Playa's journal or `playa spool` output.
