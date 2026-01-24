---
name: tts
description: Expert knowledge for Text-to-Speech synthesis covering neural TTS models (Kokoro, Piper, ElevenLabs, OpenAI), Rust/Python/TypeScript libraries, SSML/LLM-TTS voice control standards, local and cloud CLI tools, voice cloning, and emotional speech synthesis. Use when building speech synthesis features, selecting TTS providers, implementing voice control, or integrating TTS into applications.
last_updated: 2026-01-24T12:00:00Z
hash: 36299e0582a4522f
---

# Text-to-Speech (TTS)

Expert guidance for integrating speech synthesis into applications across Rust, Python, and TypeScript ecosystems.

## Core Principles

- **Match latency to use case**: Real-time agents need <100ms TTFB (Cartesia, Qwen3-TTS); content creation tolerates higher latency for quality (ElevenLabs)
- **Offline vs Cloud tradeoff**: Local models (Kokoro-82M, Piper) ensure privacy and zero cost; cloud APIs (ElevenLabs, OpenAI) provide best quality with usage fees
- **Neural beats concatenative**: Modern neural TTS (VITS, Flow Matching) far exceeds older formant/concatenative synthesis in naturalness
- **SSML for precision control**: Use SSML tags for pronunciation, pauses, emphasis, and prosody when automatic inference falls short
- **Voice cloning requires consent**: Always obtain explicit permission before cloning voices; many providers require verbal consent recordings
- **Model size vs quality**: Kokoro-82M achieves near-commercial quality at 1/10th typical model size; consider efficiency for edge deployment
- **Emotional control is emerging**: LLM-TTS enables natural language style prompts ("speak excitedly"); traditional SSML offers more predictable control
- **Sample rate matters**: 24kHz is standard for neural TTS; 16kHz for telephony; 48kHz for highest fidelity

## Quick Reference

### Choosing a TTS Solution

| Need | Best Choice |
|------|-------------|
| Highest quality, budget available | ElevenLabs, OpenAI TTS |
| Real-time voice agent (<100ms) | Cartesia Sonic, Qwen3-TTS |
| Offline/privacy-sensitive | Kokoro-82M, Piper |
| Cross-platform Rust library | `tts` crate (system voices) |
| Cloud TTS in Rust | `whispr` (OpenAI), `msedge_tts` |
| Python quick start | `pyttsx3` (offline), `gTTS` (cloud) |
| TypeScript/Node.js | `@lobehub/tts`, `@google-cloud/text-to-speech` |
| CLI tool | `say` (macOS), `espeak-ng` (cross-platform), Piper |

### Key Libraries by Language

**Rust**:
- [`tts`](https://docs.rs/tts) - Cross-platform, system voice abstraction
- [`whispr`](https://docs.rs/whispr) - OpenAI Audio API (TTS/STT)
- [`kokoroxide`](https://lib.rs/crates/kokoroxide) - Kokoro-82M local neural TTS
- [`msedge_tts`](https://docs.rs/msedge-tts) - Microsoft Edge neural voices (free, online)

**Python**:
- `pyttsx3` - Offline, cross-platform system voices
- `gTTS` - Google Translate TTS API
- `TTS` (Coqui) - Deep learning toolkit with voice cloning
- `piper-tts` - Fast local neural TTS

**TypeScript**:
- `@lobehub/tts` - Multi-provider (Edge, OpenAI, Azure)
- `@google-cloud/text-to-speech` - Google Cloud WaveNet/Neural2
- `text-to-speech-js` - Browser Web Speech API wrapper

## Topics

### TTS Models & Providers

- [Top TTS Models 2026](./models.md) - Commercial and open-source model comparison
- [Open Source Landscape](./open-source.md) - Emerging OSS models and trends

### Language-Specific Implementation

- [Rust TTS Crates](./rust.md) - tts, whispr, kokoroxide, msedge_tts, sapi-lite
- [Python TTS Libraries](./python.md) - pyttsx3, gTTS, Coqui TTS, Piper, Orpheus
- [TypeScript/npm Libraries](./ts.md) - LobeHub, Google Cloud, Azure, browser APIs

### CLI & Local Tools

- [Local CLI Tools](./local-cli.md) - say, espeak-ng, Piper, sherpa-onnx, Festival
- [Cloud Provider CLIs](./cloud-cli.md) - AWS Polly, Azure Speech, Google Cloud TTS

### Voice Control Standards

- [SSML Deep Dive](./ssml.md) - Prosody, phonemes, breaks, voice selection
- [LLM-TTS Standard](./llm-tts.md) - Natural language voice control, instruction-based synthesis

## Common Patterns

### Basic TTS in Rust (Cross-Platform)

```rust
use tts::Tts;

fn speak(text: &str) -> Result<(), tts::Error> {
    let mut tts = Tts::default()?;
    tts.speak(text, false)?; // false = don't interrupt
    Ok(())
}
```

### Streaming TTS with OpenAI (Rust)

```rust
use whispr::{Client, Voice};
use futures::StreamExt;

async fn stream_speech(text: &str) -> Result<(), whispr::Error> {
    let client = Client::from_env()?;
    let mut stream = client
        .speech()
        .text(text)
        .voice(Voice::Nova)
        .generate_stream()
        .await?;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        // Play or save audio chunk
    }
    Ok(())
}
```

### Python Quick Start

```python
# Offline (system voices)
import pyttsx3
engine = pyttsx3.init()
engine.say("Hello world")
engine.runAndWait()

# Cloud (Google Translate)
from gtts import gTTS
tts = gTTS("Hello world", lang="en")
tts.save("hello.mp3")
```

## Resources

- [ElevenLabs Docs](https://elevenlabs.io/docs)
- [OpenAI TTS API](https://platform.openai.com/docs/guides/text-to-speech)
- [W3C SSML 1.1](https://www.w3.org/TR/speech-synthesis11/)
- [Piper Voices](https://rhasspy.github.io/piper-samples/)
- [Coqui TTS Models](https://github.com/coqui-ai/TTS)
