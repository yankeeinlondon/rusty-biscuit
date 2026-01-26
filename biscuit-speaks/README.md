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

The **biscuit-speaks** library caters to both ends of this spectrum as well as all points in-between by providing _useful defaults_ which can progressively be overridden when more specificity/control is desired. The way _defaults_ are arrives at is:

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
    - if the voice is not passed in via code or suggested via an ENV variable then
- **Volume**:
    - the _volume_ that the spoken voice is spoken at defaults to a "normal" level but you can specify a volume level with the `with_volume(VolumeLevel)` builder function.
    -
- **Speed**:
    - the _speed_ at which the text is spoken can be modified from it's default speed by using the `at_speed(Speed)`
    - if not set programmatically the speed will also be influenced by the `PREFER_SPEED` environment variable set to either `fast` or `slow` (capitalization doesn't matter).




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

In our first usage example we simply accepted the defaults and in many cases that will be

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

| Provider   | Platform       | Requirements                                |
|------------|----------------|---------------------------------------------|
| Festival   | Linux          | `festival` binary installed                 |
| Pico2Wave  | Linux          | `pico2wave` binary (SVOX Pico)              |
| Mimic3     | Cross-platform | Mycroft neural TTS, SSML support            |
| Sherpa-ONNX   | Cross-platform | high quality multi-modal solution based on the ONNX C library|
| SpdSay     | Linux          | Speech Dispatcher (`spd-say`)               |
| Piper      | Cross-platform | Fast local neural TTS with ONNX             |

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
use biscuit_speaks::{ElevenLabsProvider, TtsExecutor, TtsConfig};

// Direct ElevenLabs usage
let provider = ElevenLabsProvider::new()?;
provider.speak("Direct API call", &TtsConfig::default()).await?;

// Generate audio bytes without playing
let audio_bytes = provider.generate_audio("Get audio", &TtsConfig::default()).await?;

// List available voices
let voices = provider.list_voices().await?;
for voice in voices.voices {
    println!("{}: {}", voice.voice_id, voice.name);
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

- Specifying a voice in eleven labs requires a "voice ID" which has no semantic meaning
- Instead we need to
- ElevenLabs offers different models, currently the latest model is `v3`
    - the quality of the voice will differ based on the model used
    - Not all voices work equally well with all models. A voice is essentially "trained" or "optimized" for specific models. That's why the API returns high_quality_base_model_ids for each voice - it tells you which models produce the best results.
    - If you use a mismatched model, you might get:
        - Lower quality audio
        - The voice not sounding like itself
        - Artifacts or unnatural speech

### say (on macOS)

- there is a wide range of voice qualities available from the voices on say and depending on how the user has setup Siri, voice dictation, etc. will influence what voices are actually available
- the lowest quality voices are labelled as "eloquence" voices and are filtered out because of their low quality
- the highest quality voices are labelled as
