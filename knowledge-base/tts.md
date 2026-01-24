---
name: tts
description: Comprehensive guide to Text-to-Speech technologies, models, libraries, standards, and implementation strategies across Rust, Python, and TypeScript ecosystems
created: 2026-01-24
last_updated: 2026-01-24T12:00:00Z
hash: f40bfc691995629d
tags:
  - tts
  - text-to-speech
  - speech-synthesis
  - audio
  - voice
  - ssml
  - rust
  - python
  - typescript
---

# Text-to-Speech (TTS): A Comprehensive Guide

Text-to-Speech (TTS) technology has evolved from robotic, concatenative synthesis to sophisticated neural models capable of human-like emotional expression, real-time streaming, and voice cloning. In 2026, the industry has moved far beyond "sounding human" to focus on emotional nuance, ultra-low latency, and commercial compliance. This guide covers the complete TTS landscape: from core concepts and evaluation criteria to language-specific libraries, control standards, and deployment strategies.

## Table of Contents

- [Foundation: Understanding TTS](#foundation-understanding-tts)
- [Key Features to Evaluate](#key-features-to-evaluate)
- [Top TTS Models in 2026](#top-tts-models-in-2026)
- [Open-Source TTS Landscape](#open-source-tts-landscape)
- [Voice Control Standards](#voice-control-standards)
- [Local CLI Tools](#local-cli-tools)
- [Cloud Provider CLI Tools](#cloud-provider-cli-tools)
- [Rust TTS Libraries](#rust-tts-libraries)
- [Python TTS Libraries](#python-tts-libraries)
- [TypeScript/JavaScript TTS Libraries](#typescriptjavascript-tts-libraries)
- [Implementation Patterns](#implementation-patterns)
- [Quick Reference Tables](#quick-reference-tables)
- [Resources](#resources)

## Foundation: Understanding TTS

Modern TTS systems operate on several architectural paradigms:

**Concatenative Synthesis** (Legacy): Joins pre-recorded speech units. Fast but robotic-sounding.

**Parametric Synthesis** (e.g., eSpeak): Generates speech from acoustic parameters. Lightweight but lacks naturalness.

**Neural TTS** (Current Standard): Uses deep learning models to generate natural-sounding speech. Includes architectures like:
- **Tacotron/FastSpeech**: Encoder-decoder models generating mel-spectrograms
- **VITS/Flow Matching**: End-to-end models with variational inference
- **LLM-based TTS**: Large language models generating audio tokens directly

**Key Terminology**:
- **Mel-spectrogram**: Visual representation of audio frequencies over time
- **Vocoder**: Converts spectrograms to audio waveforms (e.g., HiFi-GAN, WaveRNN)
- **Phoneme**: Smallest unit of speech sound
- **Prosody**: Rhythm, stress, and intonation patterns
- **RTF (Real-Time Factor)**: Ratio of generation time to audio duration (RTF < 1.0 = faster than real-time)

## Key Features to Evaluate

When selecting a TTS solution, evaluate these critical dimensions:

### Audio Quality and Realism

- **Prosody and Intonation**: Does it naturally raise pitch for questions and pause at commas?
- **Emotional Range**: Can it express "whispering," "excited," or "empathetic" tones?
- **Long-Form Stamina**: Does quality degrade over extended audio (listening fatigue)?

### Technical Performance

| Metric | Target | Notes |
|--------|--------|-------|
| **Latency (TTFB)** | < 500ms | Time to First Byte for conversational AI |
| **Real-Time Factor** | < 0.3 | Generate audio 3x faster than playback |
| **Streaming Support** | WebSocket/WebRTC | Audio plays while text processes |

### Linguistic Capabilities

- **Multilingual Consistency**: Does the same voice maintain identity across languages?
- **Phonetic Accuracy**: Handles homographs ("live" concert vs. "I live") and technical jargon
- **SSML Compatibility**: Supports manual pauses, speed adjustments, pronunciation fixes

### Compliance and Customization

- **Voice Cloning**: Requires explicit consent mechanisms for deepfake prevention
- **Commercial Rights**: Verify license covers paid advertisements and commercial use
- **Data Privacy**: SOC 2 compliance, zero-retention policies for enterprise

## Top TTS Models in 2026

The 2026 landscape is defined by two shifts: **extreme low latency** for conversational AI and **open-source parity** with commercial offerings.

### Commercial APIs

| Model | Best For | Latency | Cost | Emotional Range |
|-------|----------|---------|------|-----------------|
| **ElevenLabs** | High-budget acting, audiobooks | Medium | $$$ | Highest |
| **OpenAI TTS** | Reliability, simple assistants | Low | $ | Medium |
| **Cartesia Sonic** | Real-time agents (< 100ms) | Ultra-Low | $$ | Medium-High |

**ElevenLabs**: Industry benchmark for emotional realism. Flash v2.5 model bridges quality and latency. Best for long-form content where emotional nuance is critical.

**OpenAI TTS**: Extremely cheap (~$15/1M characters), easiest API integration. GPT Realtime allows speech-to-speech with natural turn-taking.

**Cartesia**: Captures the real-time agent market with sub-100ms latency. Ideal for customer support bots requiring "barge-in" capability.

### Open-Source Models

| Model | Best For | Hardware Req | Latency |
|-------|----------|--------------|---------|
| **Kokoro-82M** | Local/Edge/Mobile | Tiny (CPU/Phone) | Low |
| **Qwen3-TTS** | Premium self-hosted agent | High (H100/4090) | Ultra-Low |
| **Gemma-TTS** | Native multi-speaker | Low (NPU) | Low |
| **Fish Speech 1.5** | Multilingual, cloning | High (GPU) | High |
| **CosyVoice 2/3** | Self-hosted streaming | High (GPU) | Low |
| **F5-TTS** | Training/fine-tuning | Medium | Medium |

**Kokoro-82M**: The efficiency miracle. Achieves near-ElevenLabs quality with only 82M parameters. Runs on Raspberry Pis, phones, and browsers via WebGPU. Apache 2.0 licensed.

**Qwen3-TTS**: The all-rounder disruptor. Combines emotional control of prompt-based models with ultra-low latency (~97ms first packet). Apache 2.0 licensed.

**Gemma-TTS**: Native multimodal model where the LLM generates audio tokens directly. Excels at multi-speaker dialogue ("podcast-style") generation.

### Selection Guide

1. **Indie/Startup App Developer**: Use **Kokoro-82M** - free, runs on cheap servers or user devices
2. **Enterprise Voice Bot**: Use **Cartesia** (buying) or **Qwen3-TTS** (building) for sub-100ms latency
3. **Content Creator**: Use **ElevenLabs** for superior "acting" capability that keeps retention high

## Open-Source TTS Landscape

Open-source TTS has reached parity with commercial offerings in 2026, with key trends:

### Core Technological Trends

**Emotional and Context-Aware Synthesis**: Models now support fine-grained emotional control through:
- Emotion embeddings (Sambert-Hifigan)
- Exaggeration parameters (Chatterbox: 0-1 scale)
- Semantic understanding from LLMs (Llasa, Qwen3-TTS)

**Efficiency and Real-Time Processing**:
- Kyutai's Pocket TTS: 100M parameters matches 10x larger models
- CosyVoice2: ~150ms streaming latency

**Multilingual Support**:
- Toucan TTS: Up to 7000 languages
- Chatterbox: 23 languages with zero-shot cloning

### Key Open-Source Models

| Model | Developer | Languages | License | Best For |
|-------|-----------|-----------|---------|----------|
| **Sambert-Hifigan** | ModelScope | Chinese | Apache 2.0 | Chinese SMEs, education |
| **Chatterbox** | Resemble AI | 23 | MIT | Multilingual content |
| **CosyVoice2-0.5B** | FunAudioLLM | Multi-dialect | Apache 2.0 | Voice assistants |
| **Fish Speech 1.5** | Fish Audio | Multiple | CC-BY-NC-SA | Audiobooks (non-commercial) |
| **GPT-SoVITS** | RVC-Boss | Dialects | MIT | Fast voice cloning |

### Frameworks and Tools

**Coqui TTS**: Mature framework with 1100+ pre-trained models. Supports Tacotron2, FastSpeech2, VITS architectures.

**ESPnet**: Powerful academic toolkit for end-to-end speech processing.

**PaddleSpeech**: Baidu's offering optimized for Chinese and real-time synthesis.

## Voice Control Standards

### SSML: The Foundation Standard

Speech Synthesis Markup Language (SSML) is an XML-based W3C standard for controlling TTS output. SSML 1.1 (September 2010) provides:

**Core Elements**:
- `<prosody>`: Control pitch, rate, volume
- `<voice>`: Select voice, language, style
- `<break>`: Insert pauses (time or strength-based)
- `<phoneme>`: Precise phonetic specification
- `<say-as>`: Context-dependent interpretation (date, currency, telephone)

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <voice name="en-US-JennyNeural">
    <prosody rate="90%" pitch="+10%">
      Welcome to our service.
    </prosody>
    <break time="500ms"/>
    <say-as interpret-as="date" format="mdy">01/25/2026</say-as>
  </voice>
</speak>
```

**Vendor Extensions**: Microsoft Azure adds `<mstts:dialog>` for multi-turn conversations, `style` attributes for emotion.

### EmotionML: Emotion Annotation Standard

W3C EmotionML 1.0 (May 2014) provides vocabulary for emotional representation:

- **Category-based**: Discrete labels (happiness, sadness, anger)
- **Dimensional**: Continuous values (valence, arousal, dominance)
- **Appraisal-based**: Cognitive evaluations triggering emotions

```xml
<emotion xmlns="http://www.w3.org/2009/10/emotionml">
  <category set="everyday-emotions" name="happiness"/>
  <dimension set="pad-dimensions" name="pleasure" value="0.8"/>
  <dimension set="pad-dimensions" name="arousal" value="0.6"/>
</emotion>
```

### LLM-TTS: Natural Language Instructions

LLM-TTS systems accept free-form text prompts instead of markup:

**Key Innovations**:
- Natural language instructions: "say this in a cheerful tone"
- Few-shot voice cloning from seconds of audio
- Contextual understanding for appropriate prosody

**Implementations**:
- **InstructTTS**: Expressive TTS via natural language prompts
- **CosyVoice3**: 3-second voice cloning, cross-lingual synthesis
- **Spark-TTS**: BiCodec tokenization for efficient LLM generation

### SABLE/JSML: Historical Standards

JSML (Java Speech Markup Language) and SABLE preceded SSML. While superseded, they established foundational concepts still influencing TTS control.

## Local CLI Tools

### Comprehensive Comparison

| Provider | Quality | Hardware | SSML | Cloning | Network | API Key |
|----------|---------|----------|------|---------|---------|---------|
| **[say](https://ss64.com/mac/say.html)** (macOS) | 3/5 | Low | No | No | No | No |
| **[sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/)** | 5/5 | High | Planned | No | No | No |
| **[espeak-ng](https://github.com/espeak-ng/espeak-ng)** | 2/5 | Low | Yes | No | No | No |
| **[Piper](https://github.com/OHF-Voice/piper1-gpl)** | 4/5 | Medium | No | No | No | No |
| **[KokoroTts](https://github.com/nazdridoy/kokoro-tts)** | 4/5 | Medium | No | No | No | No |
| **[Mimic3](https://github.com/MycroftAI/mimic3)** | 4/5 | Medium | Yes | No | No | No |
| **[gTTS](https://github.com/pndurette/gTTS)** | 3/5 | Low | Yes | No | Yes | No |
| **[ElevenLabs](https://elevenlabs.io/)** | 5/5 | Low | Yes | Yes | Yes | Yes |

### Recommended by Use Case

**Best for Quality (Offline)**: sherpa-onnx - High-quality neural TTS, cross-platform

**Best for Simplicity**:
- macOS: `say` - Built-in, no installation
- Windows: SAPI - Native integration
- Linux: SpdSay - Desktop integration

**Best for Low-End Hardware**: espeak-ng - Runs on anything

**Best for Privacy-Sensitive**: Piper (GPL-3.0) - Fast local neural TTS, 30+ languages

### Piper Note

The original `rhasspy/piper` was archived October 2025. Development continues at `OHF-Voice/piper1-gpl` under **GPL-3.0** license (changed from MIT). Voices hosted on Hugging Face.

## Cloud Provider CLI Tools

| Provider | CLI Tool | License | Free Tier |
|----------|----------|---------|-----------|
| **AWS Polly** | [aws cli](https://aws.amazon.com/cli/) | Apache-2.0 | 5M chars/month (12 months) |
| **Azure Speech** | [az cli](https://learn.microsoft.com/en-us/cli/azure/) | MIT | 5M chars/month (F0 tier) |
| **Google Cloud TTS** | [gcloud cli](https://cloud.google.com/sdk/docs/install) | Apache-2.0 | 4M chars/month (ongoing) |

### AWS Polly Example

```bash
aws polly synthesize-speech \
  --output-format mp3 \
  --voice-id Joanna \
  --text "Hello, this is AWS Polly." \
  speech.mp3
```

### Azure TTS Example

```bash
az cognitiveservices speech synthesize \
  --text "Hello from Azure." \
  --voice "en-US-JennyNeural" \
  --output speech.wav
```

### Google Cloud TTS Example

```bash
curl -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  --data '{"input":{"text":"Hello from Google."},"voice":{"languageCode":"en-US"},"audioConfig":{"audioEncoding":"MP3"}}' \
  "https://texttospeech.googleapis.com/v1/text:synthesize" \
  | jq -r '.audioContent' | base64 -d > speech.mp3
```

## Rust TTS Libraries

### High-Level Cross-Platform

| Crate | Description | Platforms | License |
|-------|-------------|-----------|---------|
| **[tts](https://docs.rs/tts)** | Multi-backend abstraction | Windows, Linux, macOS, iOS, Android, WASM | MIT |
| **[natural-tts](https://github.com/CodersCreative/natural-tts)** | Simple API, Piper support planned | Cross-platform | MIT |

**When to use `tts`**: Most mature option for write-once, run-anywhere. Leverages system voices.

### Cloud-Based Clients

| Crate | Provider | License |
|-------|----------|---------|
| **[google-cloud-texttospeech-v1](https://docs.rs/google-cloud-texttospeech-v1)** | Google Cloud | Apache-2.0 |
| **[whispr](https://docs.rs/whispr)** | OpenAI | MIT |

**whispr** (v0.2.0): General-purpose audio AI library for TTS, STT, and audio-to-audio transformation.

```rust
use whispr::{Client, Voice, TtsModel};

#[tokio::main]
async fn main() -> Result<(), whispr::Error> {
    let client = Client::from_env()?;

    let audio = client
        .speech()
        .text("Hello, world!")
        .voice(Voice::Nova)
        .model(TtsModel::Gpt4oMiniTts)
        .generate()
        .await?;

    std::fs::write("output.mp3", &audio)?;
    Ok(())
}
```

**Feature Flags**: `realtime` (WebSocket), `stream` (progressive generation), `multipart` (efficient uploads)

### Platform-Specific

| Crate | Platform | License |
|-------|----------|---------|
| **[sapi-lite](https://docs.rs/sapi-lite)** | Windows SAPI | MIT |
| **[msedge_tts](https://docs.rs/msedge-tts)** | Microsoft Edge (online) | MIT |

### Local Neural Engines

| Crate | Model | License |
|-------|-------|---------|
| **[kokoroxide](https://lib.rs/crates/kokoroxide)** | Kokoro (82M params) | MIT/Apache-2.0 |
| **[piper-tts-rust](https://lib.rs/crates/piper-tts-rs)** | Piper ONNX | Other |

**kokoroxide**: High-performance Kokoro implementation via ONNX Runtime.

```rust
use kokoroxide::{KokoroTTS, load_voice_style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = KokoroTTS::new("kokoro.onnx", "tokenizer.json")?;
    let voice = load_voice_style("voice.bin")?;

    let audio = tts.speak("Hello, world!", &voice)?;
    audio.save_to_wav("output.wav")?;
    Ok(())
}
```

**Requirements**: ONNX Runtime, espeak-ng for phoneme conversion

### Decision Guide

1. **Cross-Platform Desktop/Server**: Start with `tts`
2. **Highest Voice Quality (with budget)**: Use `whispr` with OpenAI
3. **Offline/Privacy-Sensitive**: Use `kokoroxide`
4. **Windows-Only**: Use `sapi-lite`

## Python TTS Libraries

### Quick Comparison

| Library | Type | Offline | Cloning | Best For |
|---------|------|---------|---------|----------|
| **pyttsx3** | Offline | Yes | No | Basic projects, privacy |
| **gTTS** | Cloud | No | No | Quick multilingual scripts |
| **Smallest.ai Waves** | Cloud | No | Yes | Real-time (< 100ms) |
| **Coqui TTS** | Open-source | Yes | Yes | Customization, research |
| **Piper TTS** | On-device | Yes | No | Embedded, privacy |
| **VibeVoice** | Open-source | Yes | Yes | Long-form, podcasts |
| **Orpheus** | Open-source | Yes | No | Real-time, empathetic |

### pyttsx3: Offline Workhorse

```python
import pyttsx3

engine = pyttsx3.init()
engine.say("Hello! This is pyttsx3 speaking offline.")
engine.runAndWait()
```

Cross-platform (SAPI5, NSSpeechSynthesizer, eSpeak). No internet required.

### gTTS: Simple Multilingual

```python
from gtts import gTTS
import os

tts = gTTS(text="Hello from Google.", lang='en')
tts.save("output.mp3")
os.system("output.mp3")
```

Requires internet. 30+ languages supported.

### Coqui TTS: Deep Learning Toolkit

```python
from TTS.api import TTS

tts = TTS("tts_models/multilingual/multi-dataset/xtts_v2", gpu=True)

# Voice cloning with 3-6 seconds of reference audio
tts.tts_to_file(
    text="Coqui TTS can clone voices!",
    file_path="cloned.wav",
    speaker_wav="reference.wav",
    language="en"
)
```

1100+ pre-trained models. Voice cloning in 17 languages. Streaming inference < 200ms.

> **Note**: Coqui AI (the company) shut down in 2024, but the open-source project remains community-maintained.

### Piper TTS

```python
from pypipertts import PiperTTS

tts = PiperTTS(model_path="en-us-lessac-medium.onnx")
tts.synthesize_to_file("Hello from Piper!", "output.wav")
```

Fast, local neural TTS. Privacy-focused.

## TypeScript/JavaScript TTS Libraries

### Cloud-Based

| Library | Provider | Best For |
|---------|----------|----------|
| **[@google-cloud/text-to-speech](https://www.npmjs.com/package/@google-cloud/text-to-speech)** | Google Cloud | Enterprise, multilingual |
| **[@mastra/voice-azure](https://www.npmjs.com/package/@mastra/voice-azure)** | Azure | Real-time streaming |
| **[@elizaos/plugin-tts](https://www.npmjs.com/package/@elizaos/plugin-tts)** | FAL.ai | AI-powered apps |

### Server-Side & Cross-Platform

| Library | Description | Best For |
|---------|-------------|----------|
| **[@lobehub/tts](https://www.npmjs.com/package/@lobehub/tts)** | Multi-provider (Edge, Microsoft, OpenAI) | Full-stack, React |
| **[tiktok-tts](https://www.npmjs.com/package/tiktok-tts)** | TikTok API wrapper | Hobby projects |

### Browser-Native

| Library | Description | Best For |
|---------|-------------|----------|
| **[jsvoice](https://www.npmjs.com/package/jsvoice)** | Web Speech API + wake words | Voice assistants |
| **[text-to-speech-js](https://www.npmjs.com/package/text-to-speech-js)** | Simple Web Speech wrapper | Basic web TTS |
| **[@capacitor-community/text-to-speech](https://www.npmjs.com/package/@capacitor-community/text-to-speech)** | Native mobile | Capacitor apps |

### @lobehub/tts Example

```javascript
import { EdgeSpeechTTS } from '@lobehub/tts';
import WebSocket from 'ws';
import fs from 'fs';

global.WebSocket = WebSocket; // Polyfill for Node.js

const tts = new EdgeSpeechTTS({ locale: 'en-US' });

const response = await tts.create({
  input: 'Hello from LobeHub TTS!',
  options: { voice: 'en-US-GuyNeural' }
});

const buffer = Buffer.from(await response.arrayBuffer());
fs.writeFileSync('output.mp3', buffer);
```

## Implementation Patterns

### Cross-Platform Strategy

1. **Check for system TTS first** (macOS `say`, Windows SAPI, Linux speech-dispatcher)
2. **Fall back to bundled engine** (Piper, Kokoro for offline)
3. **Use cloud API for premium features** (cloning, emotions)

### Streaming Pattern

For real-time applications, process audio in chunks:

```rust
// Rust with whispr
let mut stream = client.speech()
    .text("Long text for streaming...")
    .generate_stream()
    .await?;

while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    // Process/play immediately
}
```

### Error Handling Pattern

```rust
match result {
    Ok(audio) => { /* success */ }
    Err(Error::ApiError { status, message }) => {
        eprintln!("API error ({}): {}", status, message);
    }
    Err(Error::NetworkError(e)) => {
        // Fall back to offline TTS
    }
    Err(e) => eprintln!("Unexpected: {}", e),
}
```

### Voice Selection Fallback

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis">
  <voice name="en-US-AvaNeural">Preferred neural voice</voice>
  <voice name="en-US-Female">Fallback standard voice</voice>
  Default voice if others unavailable
</speak>
```

## Quick Reference Tables

### TTS Libraries by Language

| Language | Offline High-Quality | Cloud API | Simple/Quick |
|----------|---------------------|-----------|--------------|
| **Rust** | kokoroxide, piper-tts-rust | whispr, google-cloud-tts | tts, sapi-lite |
| **Python** | Coqui TTS, Piper | Smallest.ai Waves | pyttsx3, gTTS |
| **TypeScript** | - | @google-cloud/text-to-speech | @lobehub/tts, text-to-speech-js |

### Voice Control Standards

| Standard | Type | Use Case | Adoption |
|----------|------|----------|----------|
| **SSML** | XML markup | Precise phonetic/prosody control | Universal |
| **EmotionML** | XML markup | Emotion annotation | Limited |
| **LLM-TTS** | Natural language | Intuitive control | Emerging |

### Model Comparison

| Model | Parameters | Quality | Latency | License | Cost |
|-------|------------|---------|---------|---------|------|
| ElevenLabs | N/A | Excellent | Medium | Commercial | $$$ |
| Kokoro-82M | 82M | Very Good | Low | Apache 2.0 | Free |
| Qwen3-TTS | 1.7B | Excellent | Ultra-Low | Apache 2.0 | GPU |
| Piper | Varies | Good | Low | GPL-3.0 | Free |

## Resources

### Official Documentation

- [SSML 1.1 W3C Recommendation](https://www.w3.org/TR/speech-synthesis11/)
- [EmotionML 1.0 W3C Recommendation](https://www.w3.org/TR/emotionml/)
- [Web Speech API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Speech_API)

### Model Repositories

- [Hugging Face Speech Models](https://huggingface.co/models?pipeline_tag=text-to-speech)
- [Piper Voices](https://huggingface.co/rhasspy/piper-voices)
- [Coqui TTS Models](https://github.com/coqui-ai/TTS)

### Crates and Packages

- [Rust TTS Crates](https://crates.io/keywords/text-to-speech)
- [Python TTS on PyPI](https://pypi.org/search/?q=text-to-speech)
- [npm TTS Packages](https://www.npmjs.com/search?q=text-to-speech)

### Community Resources

- [TTS Arena Leaderboard](https://huggingface.co/spaces/Pendrokar/TTS-Leaderboard)
- [Piper Voice Samples](https://rhasspy.github.io/piper-samples/)
- [espeak-ng SSML Guide](https://github.com/espeak-ng/espeak-ng/blob/master/docs/markup.md)
