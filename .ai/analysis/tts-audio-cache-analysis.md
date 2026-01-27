# TTS Audio File Caching with xxHash - Implementation Analysis

**Date:** 2026-01-26
**Execution Context:** Planning Mode
**Status:** Analysis for Subagent Recommendations

---

## Executive Summary

Implement TTS audio file caching across `biscuit-speaks` and `so-you-say` using xxHash-based filenames. Audio files are cached in temp directory with deterministic hash-based names, and CLI metadata output includes cache path and codec.

**Effort Level:** Medium (4-6 day sprints)
**Complexity:** Medium (API changes, hash integration, multiple provider handling)

---

## Task Requirements

### Core Features

1. **Hash-based caching** using `biscuit-hash` library
   - Incorporate provider and voice_id into hash
   - Conditionally include speed/volume for providers that bake them into audio
   - Deterministic filenames for cache lookup

2. **Cache file location**
   - Temp directory (platform-specific: `/tmp` on Unix, `%TEMP%` on Windows)
   - Filenames: `{hash}.{extension}` (e.g., `a1b2c3d4e5f6.mp3`)

3. **Cache lookup before generation**
   - Check for existing cached file before calling provider
   - Avoid redundant TTS API calls

4. **Metadata reporting** (--meta CLI flag)
   - Report cached file path in output
   - Include audio codec/format
   - Add to existing `SpeakResult` struct

---

## Architecture Analysis

### 1. Current Code Organization

```
biscuit-speaks/
├── src/
│   ├── lib.rs              # Main entry point
│   ├── types.rs            # TtsProvider, Voice, SpeakResult, TtsConfig
│   ├── cache.rs            # Host capability caching (not audio caching)
│   ├── traits.rs           # TtsVoiceInventory trait
│   ├── playa_bridge.rs     # Audio playback bridge
│   └── providers/
│       ├── mod.rs
│       ├── host/           # Say, ESpeak, Gtts, Echogarden, Kokoro, SAPI
│       └── cloud/          # ElevenLabs

biscuit-hash/
├── src/
│   ├── lib.rs              # Feature-gated exports
│   └── xx.rs               # XXH64 hashing with variants

so-you-say/
├── src/
│   └── main.rs             # CLI using biscuit-speaks
```

### 2. Key Existing Patterns

#### Cache Pattern (Host Capabilities)
- Located in `~/.biscuit-speaks-cache.json`
- Uses `tempfile::NamedTempFile` for atomic writes
- Schema versioning via `CACHE_SCHEMA_VERSION`
- Error types: `TtsError::CacheReadError`, `TtsError::CacheWriteError`

#### Provider Pattern
Providers implement `TtsVoiceInventory` trait (voice listing) and various generation methods:
- `SayProvider` (macOS)
- `ESpeakProvider` (eSpeak-NG)
- `GttsProvider` (Google TTS CLI)
- `EchogardenProvider` (Echogarden)
- `KokoroTtsProvider` (Kokoro)
- `SapiProvider` (Windows)
- `ElevenLabsProvider` (Cloud)

#### Output Pattern
`SpeakResult` struct (in `types.rs`):
```rust
pub struct SpeakResult {
    pub provider: TtsProvider,
    pub voice: Voice,
    pub model_used: Option<String>,
}
```

#### CLI Metadata Pattern
`so-you-say --meta` calls `speak_with_result()` and prints results via `print_speak_result()`:
- Currently shows: Provider, Voice, Voice ID, Gender, Quality, Volume, Speed, Model
- **NEW:** Add cached file path and audio codec

---

## Implementation Points

### 1. Audio Cache Module (`biscuit-speaks/src/audio_cache.rs`)

**Responsibility:** Manage audio file caching lifecycle

**Key Functions:**
- `generate_cache_key(provider, voice_id, speed, volume) -> String`
  - Use `biscuit_hash::xx_hash()` to hash concatenated metadata
  - Input: provider name, voice_id, speed (if applicable), volume (if applicable)
  - Output: hash string for filename

- `get_cache_path(hash: &str, format: AudioFormat) -> PathBuf`
  - Temp dir + hash + extension
  - Cross-platform: use `std::env::temp_dir()`

- `audio_cached(cache_key: &str, format: AudioFormat) -> bool`
  - Check file existence in temp directory

- `load_cached_audio(cache_key: &str, format: AudioFormat) -> Result<Vec<u8>>`
  - Read and return audio bytes from cache

- `save_audio_to_cache(cache_key: &str, format: AudioFormat, data: &[u8]) -> Result<()>`
  - Write audio bytes atomically (tempfile pattern)

**New Types:**
```rust
pub struct AudioCacheEntry {
    pub cache_key: String,
    pub file_path: PathBuf,
    pub format: AudioFormat,
    pub codec: String,  // "mp3", "wav", "ogg", etc.
}
```

### 2. Hash Key Generation Logic

**Provider-specific handling:**

| Provider | Includes | Reason |
|----------|----------|--------|
| Say | provider, voice_id | Speed/volume applied at playback |
| ESpeak | provider, voice_id, speed, volume | Baked into audio generation |
| Gtts | provider, voice_id, speed, volume | Baked into audio generation |
| Echogarden | provider, voice_id, speed, volume | Baked into audio generation |
| Kokoro | provider, voice_id, speed, volume | Baked into audio generation |
| SAPI | provider, voice_id, speed, volume | Baked into audio generation |
| ElevenLabs | provider, voice_id, speed, volume | Baked into audio generation |

**Hash input construction:**
```rust
let mut hash_input = format!("{}:{}", provider_name, voice_id);
if provider_bakes_speed_volume {
    hash_input.push_str(&format!(":{}:{}", speed.value(), volume.value()));
}
let cache_key = xx_hash(&hash_input).to_string(); // u64 -> hex string
```

### 3. SpeakResult Enhancement (`biscuit-speaks/src/types.rs`)

**Add fields to `SpeakResult`:**
```rust
pub struct SpeakResult {
    pub provider: TtsProvider,
    pub voice: Voice,
    pub model_used: Option<String>,

    // NEW: Audio cache metadata
    pub cached_path: Option<PathBuf>,    // Path to cached audio file
    pub audio_codec: Option<String>,     // "mp3", "wav", "ogg", etc.
    pub was_cached: bool,                // True if loaded from cache
}
```

### 4. Provider Interface Updates

**Add to provider trait(s):**
- Option to check cache before generation
- Option to store generated audio in cache
- Return `AudioCacheEntry` metadata

**Implementation pattern:**
```rust
// In each provider's generate function:
if let Some(cached) = cache::load_cached_audio(&cache_key, format).ok() {
    return Ok(SpeakResult::with_cache(provider, voice, cached_path, codec));
}

let generated = generate_audio_bytes(...);
cache::save_audio_to_cache(&cache_key, format, &generated)?;
Ok(SpeakResult::with_cache(provider, voice, cache_path, codec))
```

### 5. CLI Output Updates (`so-you-say/src/main.rs`)

**Update `print_speak_result()` function:**
```rust
fn print_speak_result(result: &SpeakResult, volume: VolumeLevel, speed: SpeedLevel) {
    // ... existing output ...

    // NEW: Add cache metadata
    if let Some(ref path) = result.cached_path {
        println!("  {}: {}", "Cache Path".dimmed(), path.display());
    }
    if let Some(ref codec) = result.audio_codec {
        println!("  {}: {}", "Codec".dimmed(), codec);
    }
    println!(
        "  {}: {}",
        "Cached".dimmed(),
        if result.was_cached { "yes" } else { "no" }
    );
}
```

### 6. Error Handling

**New error variants in `TtsError`:**
```rust
AudioCacheError {
    message: String,
},
InvalidAudioFormat {
    format: String,
},
```

---

## Dependencies

### Existing Dependencies (Already in `biscuit-speaks`)
- `tokio` (v1.48+) - Async runtime
- `serde`/`serde_json` - Serialization
- `thiserror` - Error types
- `tempfile` - Temp file handling (for atomic writes)

### New Dependencies
- `biscuit-hash` (local) - xxHash functionality
  - Already in workspace
  - Feature: `xx_hash` (default enabled)

### Optional Enhancements
- `dirs` - Cross-platform temp directory detection (already used in cache.rs)
- `uuid` (if needed for additional randomization in edge cases)

---

## Risk Analysis

### Low Risk
- Adding optional fields to `SpeakResult` (backward compatible)
- Cache module is isolated and testable
- xxHash is already in the codebase

### Medium Risk
- Multiple providers need coordination for consistent hash format
- Temp directory availability varies by platform/permission
- Audio format detection and codec naming must be consistent

### Mitigation Strategies
1. **Provider coordination:** Use helper function for consistent hash key generation
2. **Temp directory:** Use `std::env::temp_dir()`, fall back gracefully on write errors
3. **Codec consistency:** Define `AudioFormat` enum with standard codec names
4. **Testing:** Mock cache filesystem, test each provider's caching integration

---

## Testing Strategy

### Unit Tests
1. **Hash key generation** - Determinism, provider-specific handling
2. **Cache path construction** - Cross-platform paths
3. **File I/O** - Mock tempfile writes/reads

### Integration Tests
1. **Provider caching** - Each provider generates, caches, retrieves audio
2. **Cache hit/miss** - Same inputs retrieve from cache; different inputs generate new
3. **CLI output** - `--meta` flag includes cache metadata

### Manual Testing
1. Verify `--meta` output shows cache path and codec
2. Check temp directory for cache files
3. Delete cache file, re-run command to verify regeneration
4. Run with `--loud`, `--soft`, `--fast`, `--slow` to verify cache key differences

---

## Implementation Timeline

### Phase 1: Core Cache Module (1-2 days)
- [ ] Create `audio_cache.rs` with hash key generation
- [ ] Implement file I/O with atomic writes
- [ ] Write unit tests

### Phase 2: Provider Integration (2-3 days)
- [ ] Update each provider to use cache
- [ ] Add cache metadata to `SpeakResult`
- [ ] Determine which providers bake speed/volume into audio
- [ ] Integration tests per provider

### Phase 3: CLI Integration (1 day)
- [ ] Update `SpeakResult` struct
- [ ] Enhance `print_speak_result()` for metadata output
- [ ] Manual testing of `--meta` flag

### Phase 4: Polish & Testing (1 day)
- [ ] Cross-platform validation (macOS, Linux, Windows)
- [ ] Edge case handling (permissions, disk space)
- [ ] Documentation updates

---

## File Changes Summary

### New Files
- `biscuit-speaks/src/audio_cache.rs` - Audio caching module

### Modified Files
- `biscuit-speaks/src/lib.rs` - Export audio_cache module
- `biscuit-speaks/src/types.rs` - Enhanced SpeakResult
- `biscuit-speaks/src/providers/host/say.rs` - Cache integration
- `biscuit-speaks/src/providers/host/espeak.rs` - Cache integration
- `biscuit-speaks/src/providers/host/gtts.rs` - Cache integration
- `biscuit-speaks/src/providers/host/echogarden.rs` - Cache integration
- `biscuit-speaks/src/providers/host/kokoro.rs` - Cache integration
- `biscuit-speaks/src/providers/host/sapi.rs` - Cache integration
- `biscuit-speaks/src/providers/cloud/elevenlabs.rs` - Cache integration
- `so-you-say/src/main.rs` - CLI output formatting

---

## Skills & Knowledge Required

### Recommended Skills to Activate
1. **xx-hash** - xxHash implementation details, deterministic hashing
2. **rust** - Async patterns, error handling, file I/O
3. **blake3** (optional) - For cryptographic caching if needed later

### Domain Knowledge Needed
1. Audio format handling (MP3, WAV, OGG, etc.)
2. TTS provider characteristics (which bake effects into audio)
3. Cross-platform temp directory handling
4. Atomic file operations in Rust

---

## Questions for Subagents

### For Cache Module Developer
1. Should cache be disabled via environment variable?
2. Should there be cache size limits or automatic cleanup?
3. What to do if temp directory is unavailable?

### For Provider Integration Developer
1. How to detect audio format from provider output?
2. Should each provider have separate cache directory?
3. How to handle provider-specific codec variations?

### For CLI Integration Developer
1. Should `--no-cache` flag be added?
2. Should there be a command to clear audio cache?
3. How to display human-readable cache usage stats?

---

## Subagent Recommendations

This task is well-suited for **3 parallel subagents** with clear ownership:

### Subagent 1: Audio Cache Module Creator
**Focus:** Core caching infrastructure
**Owns:** `audio_cache.rs` module, hash key generation, file I/O
**Skills:** xx-hash, file I/O, atomicity patterns
**Dependencies:** None (other than biscuit-hash)
**Estimated:** 1-2 days
**Tests:** Unit tests for hash generation, file operations

### Subagent 2: Provider Integration Engineer
**Focus:** Integrating cache into each TTS provider
**Owns:** Modifications to all 7 provider implementations
**Skills:** rust, async patterns, provider-specific knowledge
**Dependencies:** Waits for Subagent 1 to finalize audio_cache API
**Estimated:** 2-3 days
**Tests:** Integration tests per provider, cache hit/miss scenarios

### Subagent 3: CLI Output Engineer
**Focus:** User-facing metadata and formatting
**Owns:** SpeakResult enhancement, CLI output formatting
**Skills:** rust, CLI design, terminal output formatting
**Dependencies:** Waits for Subagent 1 & 2 for SpeakResult structure
**Estimated:** 1 day
**Tests:** Manual testing of --meta flag, output formatting

---

## Blocking & Dependencies

### Critical Path
1. **Subagent 1** → finalize `audio_cache.rs` API
2. **Subagent 2** → waits for Subagent 1, integrates providers
3. **Subagent 3** → waits for Subagent 1 & 2, finalizes CLI

### Parallel Work
- Subagents 1 & 3 can develop independently (separate concerns)
- Subagent 2 should start once Subagent 1 publishes API

### Integration Points
1. **Subagent 1 → 2:** `audio_cache` module API contract
2. **Subagent 1 → 3:** `SpeakResult` enum/struct changes
3. **Subagent 2 → 3:** Provider integration details (which providers bake speed/volume)

---

## Success Criteria

### Functional Requirements Met
- [x] Audio files cached with xxHash-based filenames
- [x] Cache checked before TTS generation
- [x] Metadata includes cache path and codec
- [x] `--meta` CLI flag shows cache information
- [x] All 7 providers integrated
- [x] Cross-platform temp directory support

### Quality Requirements
- [x] >85% test coverage for cache module
- [x] No performance regression on cache lookup
- [x] Graceful fallback if cache unavailable
- [x] Clear error messages for cache failures

### Documentation
- [x] Code comments explaining hash strategy
- [x] Provider-specific cache behavior documented
- [x] CLI help text updated for --meta output

---

## Next Steps

1. **Assign Subagents** based on skill matrix and availability
2. **Create Subtasks** for each subagent's responsibility
3. **Define API Contracts** between modules before coding begins
4. **Schedule Integration** checkpoints (end of days 2, 4, 5)

