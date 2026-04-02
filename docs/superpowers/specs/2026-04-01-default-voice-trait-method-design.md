# Default Voice Trait Method

Add a `default_voice` method to `TtsVoiceInventory` so callers can discover the default male, female, or overall voice for any provider without speaking or manually filtering voice lists.

## Trait Change

Add `default_voice` as a required async method on `TtsVoiceInventory`:

```rust
pub trait TtsVoiceInventory: Send + Sync {
    fn list_voices(&self)
        -> impl Future<Output = Result<Vec<Voice>, TtsError>> + Send;

    /// Return the provider's default voice for the given gender.
    ///
    /// When `gender` is `Gender::Any`, returns the provider's overall
    /// best default voice regardless of gender.
    /// Always returns a concrete `Voice` -- never `None`.
    fn default_voice(
        &self,
        gender: Gender,
    ) -> impl Future<Output = Result<Voice, TtsError>> + Send;
}
```

Key decisions:

- Lives on `TtsVoiceInventory` (not `TtsExecutor` or a new trait)
- Async, matching `list_voices()`
- Returns `Result<Voice, TtsError>` -- always a concrete voice, not `Option`
- Required method -- every provider implements it directly

## Provider Implementations

### Hardcoded Providers

These providers know their defaults statically. Implementation is a simple match on `Gender`.

**Kokoro**

| Gender | Voice | Quality |
|--------|-------|---------|
| Male | `am_adam` | Excellent |
| Female | `af_heart` | Excellent |
| Any | `af_heart` | Excellent |

**eSpeak**

| Gender | Voice | Quality |
|--------|-------|---------|
| Male | `en+m3` | Low |
| Female | `en+f3` | Low |
| Any | `en+f3` | Low |

**Echogarden**

| Gender | Voice | Quality |
|--------|-------|---------|
| Male | Michael | Good |
| Female | Heart | Good |
| Any | Heart | Good |

**gTTS**

gTTS voices are language-based with no gender distinction. Returns the same default for all gender values:

| Gender | Voice | Quality |
|--------|-------|---------|
| Male | English (`en`) | Good |
| Female | English (`en`) | Good |
| Any | English (`en`) | Good |

### Dynamic Providers

These providers query available voices at runtime and select the best match.

**Say (macOS)**

1. Call `list_voices()`
2. Filter to English-language voices matching the requested gender
3. Sort by quality descending, then name ascending (alphabetical tiebreak)
4. Return the first result
5. If no voices match the gender filter, fall back to the highest-quality English voice regardless of gender

This ensures the default is always the highest-quality installed voice rather than a hardcoded name that may not be present.

**SAPI (Windows)**

Same strategy as Say:

1. Call `list_voices()`
2. Filter by gender
3. Sort by quality descending, then name ascending
4. Return the first result
5. If no voices match the gender filter, fall back to the highest-quality voice regardless of gender

Quality tiers: OneCore/Neural (Excellent) > Desktop (Good) > other (Moderate).

**ElevenLabs**

| Gender | Strategy |
|--------|----------|
| Any | Hardcoded: Rachel (voice ID `21m00Tcm4TlvDq8ikWAM`), Excellent quality |
| Male | Query API voice list, filter by "male" label, return first match; fall back to Rachel if none found |
| Female | Query API voice list, filter by "female" label, return first match; fall back to Rachel if none found |

## Error Handling

No new `TtsError` variants. Providers use existing variants:

- Hardcoded providers cannot fail -- always return `Ok(...)`
- Dynamic providers propagate errors from `list_voices()` (`VoiceEnumerationFailed`) or API calls (`ProviderFailed`)

## Re-exports

No changes needed. `TtsVoiceInventory` and `Gender` are already re-exported from the crate root.

## Testing

**Hardcoded providers** (Kokoro, eSpeak, Echogarden, gTTS): Unit tests calling `default_voice` for each `Gender` variant, asserting expected `Voice` fields (name, gender, quality).

**Dynamic providers** (Say, SAPI): Unit tests with mocked/stubbed voice lists verifying the filter-and-sort logic. Platform-gated integration tests (`#[cfg(target_os = "macos")]` for Say, `#[cfg(target_os = "windows")]` for SAPI) calling `default_voice` against real system voices.

**ElevenLabs**: Unit test for `Gender::Any` hardcoded path. Integration test (`#[ignore]`) for API-backed gender paths.

All tests follow existing patterns in the codebase -- no new test infrastructure.
