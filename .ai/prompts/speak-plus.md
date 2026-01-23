# Biscuit Speaks upgrade

Currently the `biscuit-speaks` package exports two utility functions:

- `speak_when_able`
- `speak`

Both have identical signatures but handle error conditions differently.

This functionality is wrapped around the `tts` crate and the original idea was to have a zero dependency way of leveraging what TTS capability the host had on it's system. However, the `tts` crate DOES require dependencies on Linux and possibly Windows too so we're going to move away from the `tts` package.

> The source file `biscuit-speaks/src/old.rs` contains many of the code used in the original/current implementation with ties to the `tts` crate which we're eliminating.

## Refactoring `biscuit-speaks`

We will refactor with the same goal we had originally in mind: leverage a host's TTS capabilities to provide TTS functionality. However, this time we will add in one cloud provider to allow some additional flexibility.

- We will use the `InstalledTtsClients` struct from the **Sniff** library (in this monorepo)
    - This will immediately give us a set of tools which the host has installed
- We will combine that with a type safe API client for [ElevenLabs](https://elevenlabs.com) TTS capabilities which is provided by the `schematic/schema` package (in this monorepo)

### Provider Type Separation

TTS providers are separated into two enums:

1. **`HostTtsProvider`** - Local CLI-based TTS tools:
   - `Say` (macOS), `EchoGarden`, `KokoroTts`, `Sherpa`, `ESpeak`, `SAPI` (Windows), `Festival`, `Pico2Wave`, `Mimic3`, `Gtts`, `SpdSay`
   - See `biscuit-speaks/src/types.rs` for full documentation on each provider

2. **`CloudTtsProvider`** - Cloud-based TTS APIs:
   - `ElevenLabs` - Requires API key via `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`
   - Future: Additional cloud providers can be added here

### Environment Variables

The following environment variables will play a role in which TTS solution and configuration we use on the host:

- `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`
    - when provided we know -- or at least strongly suspect -- that the **ElevenLabs** cloud TTS is an option
- `PREFER_TTS` -- if set to a valid TTS provider this is seen as an explicit desire to use this TTS provider. They will be placed at the top of the TTS stack (but we'll still fall back if for some reason their choice is not available)
- `PREFER_TTS_GENDER` will determine the preferred gender (male/female) of the voice used if the call didn't explicitly state this
- `PREFER_TTS_VOICE_MALE`, `PREFER_TTS_VOICE_FEMALE`
- `PREFER_TTS_LANG` or `PREFER_TTS_LANGUAGE`


### TTS Prioritization

The process we'll use to select the TTS provider is:

- create a force-ranked stack of desired options
    - these rankings are static by OS (defined in `biscuit-speaks/src/types.rs`):
        - **Linux**: EchoGarden → KokoroTts → Sherpa → ESpeak → SpdSay
        - **macOS**: EchoGarden → KokoroTts → Sherpa → Say → ESpeak
        - **Windows**: EchoGarden → KokoroTts → Sherpa → SAPI → ESpeak
    - and generally equate to which TTS is better than another on that OS
    - exception:
        - the cloud option with ElevenLabs is always ranked last if there is an API Key (and not listed at all if there is no API Key)
        - we do this because use of the API _can_ cost money (although there is a generous free tier) and we want the user to explicitly opt-in to using the API for this reason
        - a user opts-in by setting the `PREFER_TTS` to `elevenlabs` (capitalization insensitive)
- then filter out those options which are _not available_ (because no local program or lack of API Key)
- We will then select the top ranked TTS provider which matches the desired language choice (which is English if not set otherwise)
- **Language fallback**: If the selected provider doesn't support the requested language, fall back to the next provider in the stack that does

### Voice, Gender, and Volume

The _gender_ of the speaker, the _volume_ at which the speaker speaks, and even the _specific voice_ used are things which are desirable to configure in a TTS:

- **Volume** is abstracted via the `VolumeLevel` enum:
    - `Soft`, `Normal`, `Loud`, and `Explicit(f32)` variants
    - If a TTS provider doesn't support volume control, the setting is ignored
    - Where supported, volume settings are mapped to the provider's native API

- **Gender** is abstracted via the `Gender` enum:
    - `Male`, `Female`, `Any` (default) variants
    - Marked `#[non_exhaustive]` for future extension
    - When a provider doesn't support gender selection, falls back to any available voice

- **Language** is abstracted via the `Language` enum:
    - `English` (default) and `Custom(String)` for BCP-47 codes (e.g., "fr-FR", "es-MX")
    - Marked `#[non_exhaustive]` for future extension
    - If the selected TTS provider doesn't support the requested language, fall back to the next provider in the stack

## TTS Orchestration via the `Speak` struct

The main way that people will interact with the `biscuit-speaks` library is via the `Speak` struct.

### Speak Struct Fields

```rust
pub struct Speak {
    /// The text to be spoken
    pub text: String,
    /// Cached audio bytes (populated after prepare() or speak())
    audio: Option<Vec<u8>>,
    /// Volume level for playback
    pub volume: VolumeLevel,
    /// Explicit voice name requested by user (provider-specific)
    requested_voice: Option<String>,
    /// Preferred gender for voice selection
    pub gender: Gender,
    /// Preferred language for voice selection
    pub language: Language,
}
```

### Builder Pattern API

The `Speak` struct uses a fluent builder pattern:

```rust
// Basic usage
Speak::new("Hello, world!").speak().await?;

// With configuration
Speak::new("Bonjour!")
    .volume(VolumeLevel::Loud)
    .gender(Gender::Female)
    .language(Language::Custom("fr-FR".into()))
    .speak()
    .await?;

// Pre-generate audio for later playback
let prepared = Speak::new("Long text...")
    .prepare()
    .await?;
// ... later ...
prepared.play().await?;
```

### Async-Only API

All TTS operations are **async only**. Callers must be in an async context:

- `async fn speak(&self) -> Result<(), SpeakError>` - Generate and play audio
- `async fn prepare(&mut self) -> Result<&mut Self, SpeakError>` - Pre-generate audio without playing
- `async fn play(&self) -> Result<(), SpeakError>` - Play previously prepared audio

## Client's of this Library

There are no external client's of this library but there are several packages within this monorepo which depend on this library. Because this is a major breaking change these clients will all have to be adopted to the new API surface being provided.

- `so-you-say` is a simple CLI package which provides TTS services using **biscuit-speaks** as the underlying library
- `research/lib` is the shared library support the research package; it too interacts with this library
- `research/cli` apparently also interacts with this library

