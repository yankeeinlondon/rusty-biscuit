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

Providers where playa handles speed can share a single cached file across different speed settings. Providers that bake speed into the generated audio need separate cache entries per speed value.

### Provider Integration

Each provider that generates audio files implements a `generate_to_cache` method following the same pattern:

1. Build a `CacheKey` from provider-specific inputs
2. Call `cache_exists()` -- if `true`, return `(cache_path, true)` immediately
3. Generate the audio, writing to `cache_path` (or using `write_atomic()` for byte data)
4. Return `(cache_path, false)`

The return tuple is `(PathBuf, bool)` where the bool indicates a cache hit. This flows into `SpeakResult.cache_hit` which the CLI displays as `Cache: hit` or `Cache: miss`.

Providers that use audio caching:

- **Kokoro** -- generates WAV via `kokoro-cli`, writes directly to cache path
- **gTTS** -- generates MP3 via `gtts-cli --output`, writes directly to cache path
- **EchoGarden** -- generates WAV via `echogarden speak-to-file`, writes directly to cache path
- **ElevenLabs** -- fetches MP3 bytes from API, uses `write_atomic()` to persist

Providers that do **not** use audio caching:

- **Say** (macOS) -- speaks directly through the audio system, no intermediate file
- **eSpeak** -- speaks directly through the audio system, no intermediate file
- **SAPI** (Windows) -- speaks directly through the audio system, no intermediate file

### Atomic Writes

The `write_atomic()` function in `audio_cache.rs` prevents partial writes from corrupting cache files:

1. Write data to a uniquely-named temp file in the same directory (using PID + timestamp)
2. `sync_all()` to flush to disk
3. `rename()` to the final path (atomic on POSIX filesystems)

ElevenLabs uses this for API response bytes. Other providers write directly to the cache path via their CLI tools' `--output` flags.

### Cache Lifetime

Files live in the OS temp directory and are cleaned up by the operating system on its own schedule. There is no explicit TTL, eviction policy, or manual cleanup. The cache is opportunistic -- it helps when repeated expressions happen before the OS clears temp files.

## Provider Capability Cache

A separate cache at `~/.biscuit-speaks-cache.json` stores discovered TTS provider capabilities (installed voices, available voices). This avoids expensive re-enumeration of the host system on every TTS operation.

### Location

`BISCUIT_SPEAKS_CACHE` names the cache file directly and takes priority over the home-directory default; an empty value is treated as unset. Its parent directory must already exist.

Because environment is inherited by child processes, setting it also confines a re-spawned `so-you-say` -- which is how `--background --refresh-cache`, whose rewrite happens in a detached grandchild, is kept out of the caller's real cache during tests.

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

After caching, the audio file is played through `playback.rs` which bridges to the playa library:

1. Provider returns `(PathBuf, cache_hit)` from `generate_to_cache()`
2. `play_audio_file()` converts biscuit-speaks types to playa types via `playa_bridge.rs`
3. `playa::playa_explicit_with_options_async()` handles actual audio playback

The playa bridge maps volume (`VolumeLevel` to `f32`) and speed (`SpeedLevel` to `f32`) for providers where playa controls playback speed. For providers that bake speed into audio, the speed option still passes through but the audio already contains the adjusted tempo.
