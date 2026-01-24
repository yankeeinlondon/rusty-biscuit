# Rust TTS Crates

Top Rust crates for Text-to-Speech, categorized by use case.

## High-Level Cross-Platform

### tts - The Standard Choice

Unified API across multiple operating systems. Best for "write-once, run-anywhere" solutions.

```rust
use tts::Tts;

fn main() -> Result<(), tts::Error> {
    let mut tts = Tts::default()?;

    // List available voices
    for voice in tts.voices()? {
        println!("{}: {}", voice.name(), voice.language());
    }

    // Speak with default voice
    tts.speak("Hello, world!", false)?;

    // Adjust rate and volume
    tts.set_rate(tts.normal_rate())?;
    tts.set_volume(tts.max_volume())?;

    Ok(())
}
```

| Platform | Backend |
|----------|---------|
| Windows | SAPI5 |
| macOS | AVSpeechSynthesizer |
| Linux | Speech Dispatcher |
| iOS/Android | Native APIs |
| WebAssembly | Web Speech API |

**When to use**: Cross-platform apps, accessibility tools, games needing system voices

### natural-tts - Simple Alternative

Newer, simpler API. Less mature but easier for smaller projects.

```rust
use natural_tts::{NaturalTts, Backend};

fn main() -> Result<(), natural_tts::Error> {
    let tts = NaturalTts::new(Backend::default())?;
    tts.say("Hello world")?;
    Ok(())
}
```

## Cloud-Based Clients

### whispr - OpenAI Audio API

Clean async API for OpenAI's TTS and STT services.

```rust
use whispr::{Client, Voice, TtsModel, AudioFormat};

#[tokio::main]
async fn main() -> Result<(), whispr::Error> {
    let client = Client::from_env()?; // Uses OPENAI_API_KEY

    // Basic TTS
    let audio = client
        .speech()
        .text("Hello, world!")
        .voice(Voice::Nova)
        .generate()
        .await?;

    std::fs::write("hello.mp3", &audio)?;

    // Advanced with instructions
    let audio = client
        .speech()
        .text("Welcome to the app!")
        .model(TtsModel::Gpt4oMiniTts)
        .voice(Voice::Alloy)
        .format(AudioFormat::Mp3)
        .speed(1.2)
        .instructions("Speak enthusiastically")
        .generate()
        .await?;

    Ok(())
}
```

**Feature Flags**:
- `realtime` - WebSocket real-time audio streaming
- `stream` - Streaming TTS generation

**Streaming Example**:
```rust
use whispr::Client;
use futures::StreamExt;

async fn stream_tts() -> Result<(), whispr::Error> {
    let client = Client::from_env()?;

    let mut stream = client
        .speech()
        .text("This is streamed...")
        .generate_stream()
        .await?;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        // Play or process chunk
    }
    Ok(())
}
```

### google-cloud-texttospeech-v1

Official Google Cloud TTS client.

```rust
use google_cloud_texttospeech_v1::*;

async fn synthesize() -> Result<(), Box<dyn std::error::Error>> {
    let client = TextToSpeechClient::new().await?;

    let request = SynthesizeSpeechRequest {
        input: Some(SynthesisInput {
            input_source: Some(synthesis_input::InputSource::Text(
                "Hello from Google Cloud".into()
            )),
        }),
        voice: Some(VoiceSelectionParams {
            language_code: "en-US".into(),
            name: "en-US-Wavenet-D".into(),
            ..Default::default()
        }),
        audio_config: Some(AudioConfig {
            audio_encoding: AudioEncoding::Mp3 as i32,
            ..Default::default()
        }),
    };

    let response = client.synthesize_speech(request).await?;
    std::fs::write("output.mp3", response.audio_content)?;
    Ok(())
}
```

## Platform-Specific

### sapi-lite - Windows SAPI

Lightweight wrapper for Windows Speech API.

```rust
use sapi_lite::{tts, initialize_com};

fn main() -> Result<(), sapi_lite::Error> {
    initialize_com()?;

    // List voices
    for voice in tts::voices()? {
        println!("{}", voice.name()?);
    }

    // Speak to default audio device
    tts::speak("Hello from SAPI", None)?;

    // Output to file
    let audio = tts::synthesize("Save this", None)?;
    std::fs::write("output.wav", audio)?;

    Ok(())
}
```

**When to use**: Windows-only apps needing tight OS integration

### msedge_tts - Microsoft Edge Neural Voices

Access Microsoft's high-quality neural voices without Azure account.

```rust
use msedge_tts::{tts::client::connect, voice::get_voices_list};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // List available voices
    let voices = get_voices_list().await?;
    for voice in &voices {
        println!("{}: {}", voice.short_name, voice.locale);
    }

    // Synthesize speech
    let mut tts = connect().await?;
    let audio = tts
        .synthesize("Hello from Edge TTS", "en-US-AriaNeural")
        .await?;

    std::fs::write("output.mp3", &audio.audio_bytes)?;
    Ok(())
}
```

**Pros**: Many high-quality neural voices, no API key, free
**Cons**: Requires internet, not officially documented API

## Local Neural TTS

### kokoroxide - Kokoro-82M

High-performance local neural TTS using ONNX Runtime.

```rust
use kokoroxide::{KokoroTTS, load_voice_style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = KokoroTTS::new(
        "kokoro.onnx",
        "tokenizer.json"
    )?;

    let voice = load_voice_style("voice.bin")?;

    // Generate speech
    let audio = tts.speak("Hello world", &voice)?;
    audio.save_to_wav("output.wav")?;

    // With custom speed
    let audio = tts.generate_speech("Faster!", &voice, 1.5)?;

    // From phonemes (precise control)
    let audio = tts.generate_speech_from_phonemes(
        "həˈloʊ wɜːld",
        &voice,
        1.0
    )?;

    Ok(())
}
```

**Requirements**:
- ONNX Runtime
- espeak-ng (for text-to-phoneme)
- Model files (~80MB)

### piper-tts-rust - Piper TTS

Rust wrapper for Piper, fast local neural TTS.

```rust
// Note: Piper models must be downloaded separately
// from https://huggingface.co/rhasspy/piper-voices

use piper_tts_rs::PiperTts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = PiperTts::new("en_US-lessac-medium.onnx")?;
    let audio = tts.synthesize("Hello from Piper")?;
    // audio is raw PCM samples
    Ok(())
}
```

## Decision Guide

```
Cross-platform with system voices?
    -> tts crate

Cloud API with best quality?
    -> whispr (OpenAI) or google-cloud-texttospeech

Free neural voices, online OK?
    -> msedge_tts

Offline neural TTS?
    -> kokoroxide (Kokoro) or piper-tts-rust

Windows only, deep integration?
    -> sapi-lite
```

## Crate Comparison

| Crate | Type | Offline | Quality | Setup |
|-------|------|---------|---------|-------|
| `tts` | System | Yes | Medium | Easy |
| `whispr` | Cloud (OpenAI) | No | High | Easy |
| `msedge_tts` | Cloud (Free) | No | High | Easy |
| `kokoroxide` | Local Neural | Yes | High | Medium |
| `piper-tts-rust` | Local Neural | Yes | High | Medium |
| `sapi-lite` | Windows | Yes | Medium | Easy |
| `google-cloud-*` | Cloud | No | High | Medium |
