# TTS Providers Reference

## Provider Overview

| Provider | Type | Platform | Quality | Vol | Speed | Notes |
|----------|------|----------|---------|:---:|:-----:|-------|
| Say | Host | macOS | Moderate-Good | - | Yes | Built-in macOS TTS |
| eSpeak | Host | Cross-platform | Low | Yes | Yes | Formant synthesis, 100+ languages |
| SAPI | Host | Windows | Moderate-Excellent | Yes | Yes | Windows Speech API |
| Echogarden | Host | Cross-platform | Good-Excellent | - | - | Multi-engine (Kokoro, VITS) |
| Kokoro TTS | Host | Cross-platform | Excellent | - | Yes | Neural TTS, 54 voices |
| gTTS | Hybrid | Cross-platform | Good | - | - | Google TTS via local CLI |
| ElevenLabs | Cloud | Cross-platform | Excellent | - | Yes | Premium cloud TTS |

## Host Providers

### Say (macOS)

**Binary**: `say`

**Implementation**: Direct subprocess with `-v` flag for voice selection.

```rust
use biscuit_speaks::SayProvider;

let provider = SayProvider;
let voices = provider.list_voices().await?;
provider.speak("Hello", &TtsConfig::default()).await?;
```

**Voice Quality Tiers**:
- Premium voices: Excellent quality neural TTS
- Enhanced voices: Good quality
- Standard voices: Moderate quality
- Eloquence voices: Filtered out (robotic)

**Voice Enumeration**: Parses `say -v '?'` output.

**Limitations**: No volume control (macOS `say` lacks volume flag).

---

### eSpeak / eSpeak-NG

**Binary**: `espeak-ng` (preferred) or `espeak`

**Implementation**: Subprocess-based with language codes and gender suffixes.

```rust
use biscuit_speaks::ESpeakProvider;

let provider = ESpeakProvider::new();
let voices = provider.list_voices().await?;
```

**Voice Selection Format**: `{language}+{gender}{variant}`
- `en` - English, any gender
- `en+f3` - English, female variant 3
- `en+m1` - English, male variant 1

**Quality**: All voices marked `VoiceQuality::Low` (formant synthesis).

**Features**:
- Speed control via `-s` flag (words per minute)
- Volume control via `-a` flag (amplitude)
- 100+ language support

---

### SAPI (Windows)

**Binary**: PowerShell (Windows only)

**Implementation**: Uses PowerShell to access Windows Speech API.

```rust
use biscuit_speaks::SapiProvider;

let provider = SapiProvider::new();
let voices = provider.list_voices().await?;
```

**Voice Types**:
- OneCore/Neural: Excellent quality
- Desktop: Good quality
- Legacy SAPI5: Moderate quality

**Gender Inference**: Pattern-based (contains "Zira", "David", etc.)

**Note**: Returns error on non-Windows platforms.

---

### Echogarden

**Binary**: `echogarden` (npm package)

**Implementation**: Subprocess with JSON I/O, multi-engine support.

```rust
use biscuit_speaks::{EchogardenProvider, EchogardenEngine};

let provider = EchogardenProvider::new();
// Or with specific engine:
let provider = EchogardenProvider::with_engine(EchogardenEngine::Vits);
```

**Engines**:
| Engine | Quality | Notes |
|--------|---------|-------|
| Kokoro | Excellent | High-quality neural TTS (default) |
| VITS | Good | Neural TTS with broad language support |

**VITS Quality Filtering**: Automatically filters out `low` and `x_low` variants.

**Default Voices**:
- Female: "Heart" (Kokoro)
- Male: "Michael" (Kokoro)

**Installation**: `npm install -g echogarden`

---

### Kokoro TTS

**Binary**: `kokoro-tts`

**Implementation**: Subprocess-based with model file requirements.

```rust
use biscuit_speaks::KokoroTtsProvider;

let provider = KokoroTtsProvider::new();
let voices = provider.list_voices().await?;
```

**Model Requirements**:
- `KOKORO_MODEL` env var -> path to `kokoro-v1.0.onnx`
- `KOKORO_VOICES` env var -> path to `voices-v1.0.bin`

**Voice Naming Convention** (2-char prefix):
| First Char | Language |
|------------|----------|
| `a` | American English |
| `b` | British English |
| `j` | Japanese |
| `z` | Mandarin Chinese |
| `e` | Spanish |
| `f` | French |
| `h` | Hindi |
| `i` | Italian |
| `p` | Portuguese |

| Second Char | Gender |
|-------------|--------|
| `f` | Female |
| `m` | Male |

**Example**: `af_heart` = American Female voice "Heart"

**Voice Count**: 54 voices across 9 languages.

**Quality**: All voices `VoiceQuality::Excellent`.

---

### gTTS (Google Text-to-Speech)

**Binary**: `gtts-cli` (Python package)

**Implementation**: Network-based via Google's TTS API.

```rust
use biscuit_speaks::GttsProvider;

let provider = GttsProvider::new();
let voices = provider.list_voices().await?;
```

**Installation**: `pip install gTTS`

**Features**:
- 70+ languages and regional variants
- Language codes (e.g., `en`, `fr`, `de`, `en-au`)
- Network connectivity required

**Connectivity Caching**: Uses `AtomicBool` to cache network failure state for fast-fail on subsequent calls.

**Limitations**:
- No voice selection within languages
- No gender distinction
- Requires internet connectivity

---

## Cloud Providers

### ElevenLabs

**API**: HTTP-based via schematic-generated client.

```rust
use biscuit_speaks::ElevenLabsProvider;

let provider = ElevenLabsProvider::new()?;
let voices = provider.list_voices().await?;
let models = provider.list_models().await?;

// Generate sound effects
let sfx = provider.create_sound_effect("dog barking", Some(3.0)).await?;
```

**Authentication**:
- `ELEVENLABS_API_KEY` (preferred)
- `ELEVEN_LABS_API_KEY` (alternative)

**Default Voice**: `21m00Tcm4TlvDq8ikWAM` (Rachel)

**Default Model**: `eleven_multilingual_v2`

**Voice Selection**: Requires voice ID (not human-readable name). Use `list_voices()` to discover IDs.

**Model Selection**:
- API returns `high_quality_base_model_ids` for each voice
- Using mismatched voice/model may degrade quality
- Voices are optimized for specific models

**Speed Control**: Supports 0.7x to 1.2x (values clamped to this range).

**Features**:
- Premium neural voices
- Multiple models
- Sound effect generation via `create_sound_effect()`

---

## Provider Detection

The library uses `sniff-lib` to detect available providers at runtime.

```rust
use biscuit_speaks::get_available_providers;

// Returns Vec<&'static TtsProvider> of detected providers
let providers = get_available_providers();
for provider in providers {
    println!("Available: {:?}", provider);
}
```

**Detection Methods**:
- Binary existence check (using `which`)
- Environment variable presence (for cloud providers)
- Runtime feature detection

---

## Deferred Providers

The following providers are defined in the type system but not yet fully implemented:

| Provider | Platform | Requirements |
|----------|----------|--------------|
| Festival | Linux | `festival` binary |
| Pico2Wave | Linux | `pico2wave` (SVOX Pico) |
| Mimic3 | Cross-platform | Mycroft neural TTS |
| Sherpa-ONNX | Cross-platform | ONNX C library |
| SpdSay | Linux | Speech Dispatcher |
| Piper | Cross-platform | Fast local neural TTS |

---

## Adding a New Provider

1. Create provider struct implementing `TtsExecutor` trait
2. Optionally implement `TtsVoiceInventory` for voice enumeration
3. Add to `HostTtsProvider` or `CloudTtsProvider` enum
4. Update detection logic in `detection.rs`
5. Add to OS-specific default stacks
