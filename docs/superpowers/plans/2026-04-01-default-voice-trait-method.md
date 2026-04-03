# Default Voice Trait Method Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `default_voice(gender)` to `TtsVoiceInventory` so callers can discover the default voice for any provider without speaking.

**Architecture:** Add a required async method to the existing `TtsVoiceInventory` trait. Hardcoded providers (Kokoro, eSpeak, Echogarden, gTTS) return static `Voice` values. Dynamic providers (Say, SAPI) call `list_voices()` and filter/sort. ElevenLabs uses a hybrid approach.

**Tech Stack:** Rust, tokio (async), existing biscuit-speaks types

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `biscuit-speaks/lib/src/traits.rs` | Modify | Add `default_voice` to `TtsVoiceInventory` trait + mock impls in tests |
| `biscuit-speaks/lib/src/providers/host/kokoro.rs` | Modify | Implement `default_voice` (hardcoded) + tests |
| `biscuit-speaks/lib/src/providers/host/espeak.rs` | Modify | Implement `default_voice` (hardcoded) + tests |
| `biscuit-speaks/lib/src/providers/host/echogarden.rs` | Modify | Implement `default_voice` (hardcoded) + tests |
| `biscuit-speaks/lib/src/providers/host/gtts.rs` | Modify | Implement `default_voice` (hardcoded) + tests |
| `biscuit-speaks/lib/src/providers/host/say.rs` | Modify | Implement `default_voice` (dynamic via `list_voices`) + tests |
| `biscuit-speaks/lib/src/providers/host/sapi.rs` | Modify | Implement `default_voice` (dynamic via `list_voices`) + tests |
| `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs` | Modify | Implement `default_voice` (hybrid) + tests |

---

### Task 1: Add `default_voice` to `TtsVoiceInventory` trait and update mock impls

**Files:**
- Modify: `biscuit-speaks/lib/src/traits.rs:142-154` (trait definition)
- Modify: `biscuit-speaks/lib/src/traits.rs:204-222` (MockExecutor impl)

- [ ] **Step 1: Add `default_voice` to the trait definition**

In `biscuit-speaks/lib/src/traits.rs`, add the new method to `TtsVoiceInventory` after `list_voices`:

```rust
pub trait TtsVoiceInventory: Send + Sync {
    /// List all available voices for this provider.
    ///
    /// Returns a list of voices that can be used with this provider.
    /// For host providers, this returns installed voices.
    /// For cloud providers, this may make an API call to fetch available voices.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::VoiceEnumerationFailed` if voice listing fails.
    fn list_voices(&self)
    -> impl std::future::Future<Output = Result<Vec<Voice>, TtsError>> + Send;

    /// Return the provider's default voice for the given gender.
    ///
    /// When `gender` is `Gender::Any`, returns the provider's overall
    /// best default voice regardless of gender.
    /// Always returns a concrete `Voice`.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if voice resolution fails (e.g., API call failure
    /// for cloud providers, or no voices available for dynamic providers).
    fn default_voice(
        &self,
        gender: Gender,
    ) -> impl std::future::Future<Output = Result<Voice, TtsError>> + Send;
}
```

Note: this requires adding `Gender` to the imports at the top of the file. Change line 9 from:

```rust
use crate::types::{SpeakResult, TtsConfig, Voice};
```

to:

```rust
use crate::types::{Gender, SpeakResult, TtsConfig, Voice};
```

- [ ] **Step 2: Update `MockExecutor` to implement `default_voice`**

In the test module's `MockExecutor` impl block for `TtsVoiceInventory` (around line 204), add:

```rust
impl TtsVoiceInventory for MockExecutor {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        if self.should_fail {
            Err(TtsError::VoiceEnumerationFailed {
                provider: "mock".into(),
                message: "intentional failure".into(),
            })
        } else {
            Ok(vec![
                Voice::new("MockVoice1")
                    .with_gender(Gender::Female)
                    .with_quality(VoiceQuality::Good),
                Voice::new("MockVoice2")
                    .with_gender(Gender::Male)
                    .with_quality(VoiceQuality::Moderate),
            ])
        }
    }

    async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
        if self.should_fail {
            Err(TtsError::VoiceEnumerationFailed {
                provider: "mock".into(),
                message: "intentional failure".into(),
            })
        } else {
            match gender {
                Gender::Male => Ok(Voice::new("MockVoice2")
                    .with_gender(Gender::Male)
                    .with_quality(VoiceQuality::Moderate)),
                Gender::Female | Gender::Any => Ok(Voice::new("MockVoice1")
                    .with_gender(Gender::Female)
                    .with_quality(VoiceQuality::Good)),
            }
        }
    }
}
```

- [ ] **Step 3: Add tests for MockExecutor `default_voice`**

Add these tests in the `mod tests` block of `traits.rs`:

```rust
#[tokio::test]
async fn test_mock_executor_default_voice_male() {
    let executor = MockExecutor {
        should_fail: false,
        is_ready: true,
    };
    let voice = executor.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.name, "MockVoice2");
    assert_eq!(voice.gender, Gender::Male);
}

#[tokio::test]
async fn test_mock_executor_default_voice_female() {
    let executor = MockExecutor {
        should_fail: false,
        is_ready: true,
    };
    let voice = executor.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.name, "MockVoice1");
    assert_eq!(voice.gender, Gender::Female);
}

#[tokio::test]
async fn test_mock_executor_default_voice_any() {
    let executor = MockExecutor {
        should_fail: false,
        is_ready: true,
    };
    let voice = executor.default_voice(Gender::Any).await.unwrap();
    assert_eq!(voice.name, "MockVoice1");
    assert_eq!(voice.gender, Gender::Female);
}

#[tokio::test]
async fn test_mock_executor_default_voice_failure() {
    let executor = MockExecutor {
        should_fail: true,
        is_ready: true,
    };
    let result = executor.default_voice(Gender::Any).await;
    assert!(result.is_err());
}
```

- [ ] **Step 4: Run tests to verify everything compiles and passes**

Run: `cargo test -p biscuit-speaks --lib traits`
Expected: All existing tests still pass, new tests pass. Compilation will fail for all other providers since they don't implement `default_voice` yet — that's expected.

- [ ] **Step 5: Commit**

```bash
git add biscuit-speaks/lib/src/traits.rs
git commit -m "feat(biscuit-speaks): add default_voice method to TtsVoiceInventory trait"
```

---

### Task 2: Implement `default_voice` for KokoroTtsProvider (hardcoded)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/kokoro.rs:388` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `kokoro.rs`:

```rust
#[tokio::test]
async fn test_default_voice_male() {
    let provider = KokoroTtsProvider::new();
    let voice = provider.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.name, "am_adam");
    assert_eq!(voice.gender, Gender::Male);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}

#[tokio::test]
async fn test_default_voice_female() {
    let provider = KokoroTtsProvider::new();
    let voice = provider.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.name, "af_heart");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}

#[tokio::test]
async fn test_default_voice_any() {
    let provider = KokoroTtsProvider::new();
    let voice = provider.default_voice(Gender::Any).await.unwrap();
    assert_eq!(voice.name, "af_heart");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}
```

- [ ] **Step 2: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for KokoroTtsProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    let (name, voice_gender) = match gender {
        Gender::Male => ("am_adam", Gender::Male),
        Gender::Female | Gender::Any => (Self::DEFAULT_VOICE, Gender::Female),
    };
    Ok(Voice::new(name)
        .with_gender(voice_gender)
        .with_quality(VoiceQuality::Excellent)
        .with_language(Language::English))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::kokoro`
Expected: All tests pass including the 3 new ones.

- [ ] **Step 4: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/kokoro.rs
git commit -m "feat(biscuit-speaks): implement default_voice for KokoroTtsProvider"
```

---

### Task 3: Implement `default_voice` for ESpeakProvider (hardcoded)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/espeak.rs:200` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `espeak.rs`:

```rust
#[tokio::test]
async fn test_default_voice_male() {
    let provider = ESpeakProvider::new();
    let voice = provider.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.name, "en+m3");
    assert_eq!(voice.gender, Gender::Male);
    assert_eq!(voice.quality, VoiceQuality::Low);
}

#[tokio::test]
async fn test_default_voice_female() {
    let provider = ESpeakProvider::new();
    let voice = provider.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.name, "en+f3");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Low);
}

#[tokio::test]
async fn test_default_voice_any() {
    let provider = ESpeakProvider::new();
    let voice = provider.default_voice(Gender::Any).await.unwrap();
    assert_eq!(voice.name, "en+f3");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Low);
}
```

- [ ] **Step 2: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for ESpeakProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    let (name, voice_gender) = match gender {
        Gender::Male => ("en+m3", Gender::Male),
        Gender::Female | Gender::Any => ("en+f3", Gender::Female),
    };
    Ok(Voice::new(name)
        .with_gender(voice_gender)
        .with_quality(VoiceQuality::Low)
        .with_language(Language::English))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::espeak`
Expected: All tests pass including the 3 new ones.

- [ ] **Step 4: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/espeak.rs
git commit -m "feat(biscuit-speaks): implement default_voice for ESpeakProvider"
```

---

### Task 4: Implement `default_voice` for EchogardenProvider (hardcoded)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/echogarden.rs:422` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `echogarden.rs`:

```rust
#[tokio::test]
async fn test_default_voice_male() {
    let provider = EchogardenProvider::new();
    let voice = provider.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.name, "Michael");
    assert_eq!(voice.gender, Gender::Male);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}

#[tokio::test]
async fn test_default_voice_female() {
    let provider = EchogardenProvider::new();
    let voice = provider.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.name, "Heart");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}

#[tokio::test]
async fn test_default_voice_any() {
    let provider = EchogardenProvider::new();
    let voice = provider.default_voice(Gender::Any).await.unwrap();
    assert_eq!(voice.name, "Heart");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
}
```

Note: quality is `Excellent` because `EchogardenProvider::new()` defaults to `EchogardenEngine::Kokoro`, and `EchogardenEngine::Kokoro.quality()` returns `VoiceQuality::Excellent`.

- [ ] **Step 2: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for EchogardenProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    let (name, voice_gender) = match gender {
        Gender::Male => ("Michael", Gender::Male),
        Gender::Female | Gender::Any => ("Heart", Gender::Female),
    };
    Ok(Voice::new(name)
        .with_gender(voice_gender)
        .with_quality(self.engine.quality())
        .with_language(Language::English))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::echogarden`
Expected: All tests pass including the 3 new ones.

- [ ] **Step 4: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/echogarden.rs
git commit -m "feat(biscuit-speaks): implement default_voice for EchogardenProvider"
```

---

### Task 5: Implement `default_voice` for GttsProvider (hardcoded)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/gtts.rs:332` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `gtts.rs`:

```rust
#[tokio::test]
async fn test_default_voice_male() {
    let provider = GttsProvider::new();
    let voice = provider.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.name, "English");
    assert_eq!(voice.gender, Gender::Any);
    assert_eq!(voice.quality, VoiceQuality::Good);
    assert_eq!(voice.identifier, Some("en".into()));
}

#[tokio::test]
async fn test_default_voice_female() {
    let provider = GttsProvider::new();
    let voice = provider.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.name, "English");
    assert_eq!(voice.gender, Gender::Any);
    assert_eq!(voice.quality, VoiceQuality::Good);
}

#[tokio::test]
async fn test_default_voice_any() {
    let provider = GttsProvider::new();
    let voice = provider.default_voice(Gender::Any).await.unwrap();
    assert_eq!(voice.name, "English");
    assert_eq!(voice.gender, Gender::Any);
    assert_eq!(voice.quality, VoiceQuality::Good);
}
```

- [ ] **Step 2: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for GttsProvider` block:

```rust
async fn default_voice(&self, _gender: Gender) -> Result<Voice, TtsError> {
    Ok(Voice::new("English")
        .with_gender(Gender::Any)
        .with_quality(VoiceQuality::Good)
        .with_language(Language::English)
        .with_identifier("en"))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::gtts`
Expected: All tests pass including the 3 new ones.

- [ ] **Step 4: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/gtts.rs
git commit -m "feat(biscuit-speaks): implement default_voice for GttsProvider"
```

---

### Task 6: Implement `default_voice` for SayProvider (dynamic)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/say.rs:362` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `say.rs`. First, a unit test using `select_best_voice` to verify the filter/sort logic (this doesn't require macOS), then platform-gated integration tests:

```rust
#[test]
fn test_default_voice_selection_prefers_highest_quality() {
    let voices = vec![
        Voice::new("Albert")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Moderate)
            .with_language(Language::English),
        Voice::new("Alex (Premium)")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
        Voice::new("Daniel")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Moderate)
            .with_language(Language::English),
    ];

    let config = TtsConfig::new().with_gender(Gender::Male);
    let best = SayProvider::select_best_voice(&voices, &config).unwrap();
    assert_eq!(best.name, "Alex (Premium)");
}

#[test]
fn test_default_voice_selection_alphabetical_tiebreak() {
    let voices = vec![
        Voice::new("Zara")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
        Voice::new("Amy")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
        Voice::new("Kate")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
    ];

    let config = TtsConfig::new().with_gender(Gender::Female);
    let best = SayProvider::select_best_voice(&voices, &config).unwrap();
    // Alphabetically first among same-quality voices
    assert_eq!(best.name, "Amy");
}

#[test]
fn test_default_voice_selection_gender_fallback() {
    // Only female voices available, but male requested
    let voices = vec![
        Voice::new("Samantha")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
    ];

    let config = TtsConfig::new().with_gender(Gender::Male);
    let best = SayProvider::select_best_voice(&voices, &config).unwrap();
    // Falls back to best available regardless of gender
    assert_eq!(best.name, "Samantha");
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_default_voice_integration_male() {
    let provider = SayProvider;
    let voice = provider.default_voice(Gender::Male).await.unwrap();
    assert_eq!(voice.gender, Gender::Male);
    assert!(!voice.name.is_empty());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_default_voice_integration_female() {
    let provider = SayProvider;
    let voice = provider.default_voice(Gender::Female).await.unwrap();
    assert_eq!(voice.gender, Gender::Female);
    assert!(!voice.name.is_empty());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_default_voice_integration_any() {
    let provider = SayProvider;
    let voice = provider.default_voice(Gender::Any).await.unwrap();
    assert!(!voice.name.is_empty());
}
```

- [ ] **Step 2: Update `select_best_voice` to sort alphabetically as tiebreak**

The existing `select_best_voice` in `say.rs` sorts by quality only. Add alphabetical tiebreak. Replace the sort closure:

```rust
// Sort by quality (highest first), then name alphabetically
candidates.sort_by(|a, b| {
    let quality_rank = |q: VoiceQuality| match q {
        VoiceQuality::Excellent => 0,
        VoiceQuality::Good => 1,
        VoiceQuality::Moderate => 2,
        VoiceQuality::Low => 3,
        VoiceQuality::Unknown => 4,
    };
    quality_rank(a.quality)
        .cmp(&quality_rank(b.quality))
        .then_with(|| a.name.cmp(&b.name))
});
```

- [ ] **Step 3: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for SayProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    let voices = self.list_voices().await?;

    let config = TtsConfig::new()
        .with_gender(gender)
        .with_language(Language::English);

    Self::select_best_voice(&voices, &config).ok_or_else(|| {
        TtsError::VoiceEnumerationFailed {
            provider: Self::PROVIDER_NAME.into(),
            message: "No voices available".into(),
        }
    })
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::say`
Expected: All tests pass including the new ones.

- [ ] **Step 5: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/say.rs
git commit -m "feat(biscuit-speaks): implement default_voice for SayProvider"
```

---

### Task 7: Implement `default_voice` for SapiProvider (dynamic)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/host/sapi.rs:146` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `sapi.rs`:

```rust
#[test]
fn test_select_best_default_voice_by_quality() {
    let voices = vec![
        Voice::new("Microsoft David Desktop")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
        Voice::new("Microsoft Mark OneCore")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Excellent)
            .with_language(Language::English),
    ];

    let best = SapiProvider::select_best_default_voice(&voices, Gender::Male).unwrap();
    assert_eq!(best.name, "Microsoft Mark OneCore");
}

#[test]
fn test_select_best_default_voice_gender_fallback() {
    let voices = vec![
        Voice::new("Microsoft Zira Desktop")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
    ];

    // No male voices, falls back to best available
    let best = SapiProvider::select_best_default_voice(&voices, Gender::Male).unwrap();
    assert_eq!(best.name, "Microsoft Zira Desktop");
}

#[test]
fn test_select_best_default_voice_alphabetical_tiebreak() {
    let voices = vec![
        Voice::new("Microsoft Zira OneCore")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Excellent)
            .with_language(Language::English),
        Voice::new("Microsoft Catherine OneCore")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Excellent)
            .with_language(Language::English),
    ];

    let best = SapiProvider::select_best_default_voice(&voices, Gender::Female).unwrap();
    assert_eq!(best.name, "Microsoft Catherine OneCore");
}

#[test]
fn test_select_best_default_voice_any_gender() {
    let voices = vec![
        Voice::new("Microsoft Zira Desktop")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
        Voice::new("Microsoft David Desktop")
            .with_gender(Gender::Male)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English),
    ];

    let best = SapiProvider::select_best_default_voice(&voices, Gender::Any).unwrap();
    // Any gender: alphabetically first among top quality
    assert_eq!(best.name, "Microsoft David Desktop");
}

#[test]
fn test_select_best_default_voice_empty_list() {
    let voices: Vec<Voice> = vec![];
    let result = SapiProvider::select_best_default_voice(&voices, Gender::Any);
    assert!(result.is_none());
}
```

- [ ] **Step 2: Add `select_best_default_voice` helper method**

Add this to the `impl SapiProvider` block:

```rust
/// Select the best default voice from a list, filtering by gender with fallback.
///
/// Sort by quality descending, then name ascending. If no voices match the
/// requested gender, falls back to the best voice regardless of gender.
fn select_best_default_voice(voices: &[Voice], gender: Gender) -> Option<Voice> {
    let quality_rank = |q: VoiceQuality| match q {
        VoiceQuality::Excellent => 0,
        VoiceQuality::Good => 1,
        VoiceQuality::Moderate => 2,
        VoiceQuality::Low => 3,
        VoiceQuality::Unknown => 4,
    };

    let mut candidates: Vec<&Voice> = voices.iter().collect();

    // Filter by gender if specified (not Any)
    if gender != Gender::Any {
        let gender_matches: Vec<&Voice> = candidates
            .iter()
            .filter(|v| v.gender == gender)
            .copied()
            .collect();

        if !gender_matches.is_empty() {
            candidates = gender_matches;
        }
    }

    // Sort by quality descending, then name ascending
    candidates.sort_by(|a, b| {
        quality_rank(a.quality)
            .cmp(&quality_rank(b.quality))
            .then_with(|| a.name.cmp(&b.name))
    });

    candidates.first().cloned().cloned()
}
```

- [ ] **Step 3: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for SapiProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    let voices = self.list_voices().await?;

    Self::select_best_default_voice(&voices, gender).ok_or_else(|| {
        TtsError::VoiceEnumerationFailed {
            provider: "SAPI".into(),
            message: "No voices available".into(),
        }
    })
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::host::sapi`
Expected: All tests pass including the new ones.

- [ ] **Step 5: Commit**

```bash
git add biscuit-speaks/lib/src/providers/host/sapi.rs
git commit -m "feat(biscuit-speaks): implement default_voice for SapiProvider"
```

---

### Task 8: Implement `default_voice` for ElevenLabsProvider (hybrid)

**Files:**
- Modify: `biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:811` (TtsVoiceInventory impl)

- [ ] **Step 1: Write the tests**

Add these tests to the `mod tests` block of `elevenlabs.rs`:

```rust
#[test]
fn test_rachel_default_voice() {
    let voice = ElevenLabsProvider::rachel_default_voice();
    assert_eq!(voice.name, "Rachel");
    assert_eq!(voice.gender, Gender::Female);
    assert_eq!(voice.quality, VoiceQuality::Excellent);
    assert_eq!(voice.identifier, Some(DEFAULT_VOICE_ID.to_string()));
}
```

- [ ] **Step 2: Add `rachel_default_voice` helper method**

Add this to the `impl ElevenLabsProvider` block:

```rust
/// Build the default Rachel voice as a `Voice` struct.
fn rachel_default_voice() -> Voice {
    Voice::new("Rachel")
        .with_gender(Gender::Female)
        .with_quality(VoiceQuality::Excellent)
        .with_language(Language::English)
        .with_identifier(DEFAULT_VOICE_ID)
}
```

- [ ] **Step 3: Implement `default_voice`**

Add the method to the existing `impl TtsVoiceInventory for ElevenLabsProvider` block:

```rust
async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
    if gender == Gender::Any {
        return Ok(Self::rachel_default_voice());
    }

    // Query API for voices matching the requested gender.
    // list_voices() already converts API responses to Voice structs
    // with gender populated via voice_response_to_voice.
    match self.list_voices().await {
        Ok(voices) => {
            voices
                .into_iter()
                .find(|v| v.gender == gender)
                .ok_or(())
                .or_else(|()| Ok(Self::rachel_default_voice()))
        }
        Err(_) => Ok(Self::rachel_default_voice()),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p biscuit-speaks --lib providers::cloud::elevenlabs`
Expected: All tests pass including the new one.

- [ ] **Step 5: Commit**

```bash
git add biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
git commit -m "feat(biscuit-speaks): implement default_voice for ElevenLabsProvider"
```

---

### Task 9: Full test suite and final verification

**Files:**
- All modified files from Tasks 1-8

- [ ] **Step 1: Run the full biscuit-speaks test suite**

Run: `just test` from the `biscuit-speaks` directory, or:
`cargo test -p biscuit-speaks`
Expected: All tests pass, no compilation errors.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p biscuit-speaks -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Commit any fixups**

If clippy or tests required changes, commit them:

```bash
git add -u biscuit-speaks/
git commit -m "fix(biscuit-speaks): address clippy warnings for default_voice"
```
