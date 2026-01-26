# Biscuit Speaks TTS Library

<table>
<tr>
<td><img src="../assets/biscuit-speaks-512.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>biscuit-speaks</h2>
<p>This library provides TTS functionality to Rust programs:</p>

<ul>
    <li>leverages the host's existing capabilities for small yet cross-platform TTS functionality</li>
    <li>includes <i>cloud</i> support for <a href="https://elevenlabs.io/docs/overview/intro">ElevenLabs</a> TTS</li>
    <li>automatic failover between providers</li>
</ul>

<p>
    This library is the TTS functionality behind the <code>so-you-say</code> CLI.
</p>
</td>
</tr>
</table>

## Providers

This library can use any of the following TTS providers found on the host (or with the appropriate API Key for cloud providers):

### Local Providers

| Provider   | OS             | Vol | Speed | Notes                                                                      |
|------------|----------------|:---:|:-----:| ----------------------------------------------------------------------------|
| Say        | macOS          | ❌  | ✅    | Built-in macOS TTS with decent quality                                     |
| eSpeak     | Cross-platform | ✅  | ✅    | Massive language library but voices a bit robotic                          |
| SAPI       | Windows        | ✅  | ✅    | Windows Speech API with system voices                                      |
| echogarden | Cross-platform | 🔺  | 🔺    | Uses very high quality **kokoro** voices or decent quality **vits** voices |
| kokoro-cli | Cross-platform | 🔺  | 🔺    | Provides a good set of very high quality **kokoro** voices                 |

### Hybrid Providers

| Provider | OS             | Vol | Speed | Notes                                                                    |
|----------|----------------|:---:|:-----:|--------------------------------------------------------------------------|
| gTTS     | Cross-platform | 🔺  | ❌    | uses a locally installed client to interact with Google TTS in the cloud |

### Cloud Providers

| Provider   | ENV                                             | Vol | Speed | Notes                           |
|------------|------------------------------------------------|:---:|:-----:|---------------------------------|
| ElevenLabs | `ELEVEN_LABS_API_KEY` _or_ `ELEVENLABS_API_KEY` | 🔺  | ✅    | Excellent TTS done in the cloud |

<br/>

🔺 - certain TTS providers only produce an _audio file_ and then rely on other software on the host to play the audio; we use the `Playa` library to detect and use the best headless audio player on the host.

## Useful Defaults

There are times where we will want to have fine grained control over the TTS's voice, the volume of the voice, the provider we want to use, etc. but on the other side of the spectrum we often just want to "say something" and not concern ourselves with details.

The **biscuit-speaks** library caters to both ends of this spectrum as well as all points in-between by providing _useful defaults_ which can progressively be overridden when more specificity/control is desired. The way _defaults_ are arrived at is:

- **Capabilities**: _establish a cache file of providers and voices [`~/.biscuit-speaks-cache.json`]_:
    - the host system is evaluated for it's installed TTS programs as well as the voices available
    - we also check ENV variables to see if an [ElevenLabs](https://elevenlabs.io) API Key is detected and if it is then it is added to the list of providers
- **Language**:
    - when no specific language is formally specified we _default_ to English (or whatever `PREFER_LANGUAGE` is set to)
- **Provider**:
    - each operating system (Windows, Linux, macOS) have a statically defined "stack" of TTS providers which in general try to order the TTS providers from best-to-worst (in terms of voice quality)
    - If a user specified the language they wish to use then we will use the first provider in the stack
    - we iterate through this stack until we find one which the host system has _installed_ and which provides the at least one voice for the selected language we're targeting
- **Gender**:
    - if no gender is specified then we will _prefer_ the highest quality voice available but if there are both male and female variants at that same quality level we will prefer a female voice by default (or whatever the `PREFER_GENDER` environment variable states).
    - **Note:** "gender" is always ignored when a specific "voice" is chosen
- **Voice**:
    - each provider has a set of voices which the host has available to them
    - you can use the `with_voice(String)` builder function to specify a voice in code
    - if no voice is specified then we'll see if `PREFER_VOICE` is available to the host as a valid voice and use it if it is
    - if the voice is not passed in via code or suggested via an ENV variable then the highest quality voice matching the language/gender constraints is selected
- **Volume**:
    - the _volume_ that the spoken voice is spoken at defaults to a "normal" level but you can specify a volume level with the `with_volume(VolumeLevel)` builder function
- **Speed**:
    - the _speed_ at which the text is spoken can be modified from its default speed by using the `with_speed(SpeedLevel)` builder function
    - if not set programmatically the speed will also be influenced by the `PREFER_SPEED` environment variable set to either `fast` or `slow` (capitalization doesn't matter).


One important point to note, when you don't specify the provider, the general rule of thumb is use the highest quality provider which is available. However, there is one exception ... we will not use a cloud based API (for now that just means ElevenLabs) unless explicitly asked to. That's not because it isn't high quality (it is) but because it could cost money (even though the free tier is generous). For this reason we felt it would be better to require a caller to explicitly use a cloud based provider.

## Usage

### The Basics

```rust
use biscuit_speaks::Speak;

// lazily specify the text you will want to speak at
// some point in the future
let hello = Speak::new("Hello World");

// ... then later speak the phrase with the voice, TTS provider,
// and language all set to the defaults
hello.play().await?;
```


### Being More Explicit

In the first usage example we simply accepted the defaults, and in many cases that will be sufficient. However, when you need more control:

```rust
use biscuit_speaks::{Speak, TtsConfig, Gender, VolumeLevel};

// Simple usage with defaults (async)
Speak::new("Hello, world!").play().await?;

// With custom voice selection
Speak::new("Custom voice")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_volume(VolumeLevel::Soft)
    .play()
    .await?;

// Fire-and-forget (ignores errors)
biscuit_speaks::speak_when_able("Task complete!", &TtsConfig::default()).await;
```


## Deferred Providers

The following providers are defined in the type system but not yet implemented:

| Provider    | Platform       | Requirements                                |
|------------ |----------------|---------------------------------------------|
| Festival    | Linux          | `festival` binary installed                 |
| Pico2Wave   | Linux          | `pico2wave` binary (SVOX Pico)              |
| Mimic3      | Cross-platform | Mycroft neural TTS, SSML support            |
| Sherpa-ONNX | Cross-platform | high quality multi-modal solution based on the ONNX C library |
| SpdSay      | Linux          | Speech Dispatcher (`spd-say`)               |
| Piper       | Cross-platform | Fast local neural TTS with ONNX             |

These providers are often found on hosts and may be added to this library in the future.

## Environment Variables

| Variable              | Description                         |
|-----------------------|-------------------------------------|
| `ELEVENLABS_API_KEY`  | ElevenLabs API key (preferred)      |
| `ELEVEN_LABS_API_KEY` | ElevenLabs API key (alternative)    |
| `TTS_PROVIDER`        | Override default provider selection |

## API Overview

### `Speak` Builder

The main API for TTS operations:

```rust
use biscuit_speaks::{Speak, TtsFailoverStrategy, TtsProvider, CloudTtsProvider};

// Use ElevenLabs specifically
Speak::new("High quality voice")
    .with_failover(TtsFailoverStrategy::SpecificProvider(
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
    ))
    .play()
    .await?;

// Prefer cloud providers
Speak::new("Cloud first")
    .with_failover(TtsFailoverStrategy::PreferCloud)
    .play()
    .await?;
```

### Direct Provider Access

```rust
use biscuit_speaks::{ElevenLabsProvider, TtsExecutor, TtsVoiceInventory, TtsConfig};

// Direct ElevenLabs usage
let provider = ElevenLabsProvider::new()?;
provider.speak("Direct API call", &TtsConfig::default()).await?;

// Generate audio bytes without playing
let audio_bytes = provider.generate_audio("Get audio", &TtsConfig::default()).await?;

// List available voices (raw ElevenLabs API response)
let response = provider.list_voices_raw().await?;
for voice in response.voices {
    println!("{}: {}", voice.voice_id, voice.name);
}

// Or use the TtsVoiceInventory trait for normalized Voice structs
let voices = provider.list_voices().await?;
for voice in voices {
    println!("{}: {}", voice.identifier.as_deref().unwrap_or("?"), voice.name);
}

// List available models
let models = provider.list_models().await?;
for model in models {
    println!("{}: {}", model.model_id, model.name);
}

// Create sound effects
let sfx = provider.create_sound_effect("dog barking loudly", Some(3.0)).await?;
```

### Available Providers

```rust
use biscuit_speaks::get_available_providers;

// List detected providers
for provider in get_available_providers() {
    println!("Available: {:?}", provider);
}
```

## Provider Specific Notes

### ElevenLabs

- **Voice Selection**: Specifying a voice requires a "voice ID" (e.g., `21m00Tcm4TlvDq8ikWAM`), which has no semantic meaning. The library can list available voices via `list_voices()` to discover human-readable names and their corresponding IDs.
- **Model Selection**: ElevenLabs offers different models with varying capabilities:
    - Default: `eleven_multilingual_v2` - best for multilingual support
    - The quality and character of a voice depends on which model you use
    - Voices are optimized for specific models. The API returns `high_quality_base_model_ids` for each voice indicating which models produce best results
    - Using a mismatched voice/model combination may result in lower quality audio, unnatural speech, or the voice not sounding like itself
- **Speed Control**: ElevenLabs supports speed adjustment between 0.7x and 1.2x (our speed values are clamped to this range)
- **Sound Effects**: The provider also supports generating AI sound effects from text prompts via `create_sound_effect()`

### Say (macOS)

- **Platform**: Built-in on all macOS systems, uses the `say` command
- **Voice Quality Tiers**: Voice quality varies significantly based on installed voices:
    - "Enhanced" and "Premium" voices provide good quality neural TTS
    - Standard voices provide moderate quality
    - "Eloquence" voices (robotic, low quality) are automatically filtered out
- **Available Voices**: Depends on user's Siri/dictation settings and downloaded voice packs. Run `say -v '?'` to see what's installed
- **Speed Control**: Supports rate adjustment via the `-r` flag (words per minute)
- **No Volume Control**: The macOS `say` command does NOT support a volume flag

### eSpeak

- **Platform**: Cross-platform, uses `espeak-ng` (preferred) or `espeak` binary
- **Voice Quality**: Uses formant synthesis which produces robotic but reliable output. All voices are marked as `Low` quality
- **Language Support**: Supports 100+ languages with compact voice data files
- **Voice Selection**: Uses language codes with optional gender suffixes:
    - `en` - English, any gender
    - `en+f3` - English, female variant 3
    - `en+m3` - English, male variant 3
- **Speed Control**: Supports rate adjustment via the `-s` flag (words per minute)

### SAPI (Windows)

- **Platform**: Windows only, uses PowerShell to access Windows Speech API
- **Voice Types**: Supports both SAPI5 and OneCore voices
    - OneCore/Neural voices are excellent quality
    - Desktop voices are good quality
    - Legacy SAPI5 voices are moderate quality
- **Status**: Currently a stub implementation - `speak()` is not yet functional

### Echogarden

- **Platform**: Cross-platform, requires `echogarden` npm package installed globally
- **Engine Options**: Supports multiple TTS backends:
    - **Kokoro**: High-quality neural TTS (default) - Excellent quality
    - **VITS**: Neural TTS with broad language support - Good quality
- **Voice Selection**: Default voices by gender:
    - Female: "Heart" (Kokoro)
    - Male: "Michael" (Kokoro)
- **VITS Quality Filtering**: VITS voices come in quality tiers (`high`, `medium`, `low`, `x_low`). The library automatically filters out `low` and `x_low` variants, keeping only the best quality version of each voice
- **Output Quirk**: Echogarden writes status output to stderr, not stdout

### Kokoro-TTS

- **Platform**: Cross-platform, requires `kokoro-tts` CLI and model files
- **Model Requirements**: Requires two model files to be present:
    - `kokoro-v1.0.onnx` - The ONNX model
    - `voices-v1.0.bin` - Voice embeddings
    - Set `KOKORO_MODEL` and `KOKORO_VOICES` environment variables to specify custom paths
- **Voice Naming Convention**: Voices use a 2-character prefix:
    - First char: language (a=American, b=British, j=Japanese, z=Mandarin, e=Spanish, f=French, h=Hindi, i=Italian, p=Portuguese)
    - Second char: gender (f=Female, m=Male)
    - Example: `af_heart` = American Female voice named "heart"
- **Voice Count**: 54 voices across 9 languages
- **Quality**: All voices are neural TTS with excellent quality

### gTTS

- **Platform**: Cross-platform, requires `gtts-cli` Python package (`pip install gTTS`)
- **Network Dependency**: Requires internet connectivity - uses Google's TTS API
- **Connectivity Caching**: After a network failure, the provider caches the failure state to provide fast-fail behavior on subsequent calls
- **Voice Selection**: Uses language codes (e.g., `en`, `fr`, `de`, `en-au`)
- **Gender**: gTTS does not distinguish between male and female voices
- **Quality**: Good quality (Google's neural TTS)
- **Supported Languages**: 70+ languages and regional variants
