# Python TTS Libraries

Python offers the richest ecosystem of TTS libraries, from simple offline tools to advanced AI-powered systems.

## Quick Comparison

| Library | Type | Offline | Voice Cloning | Best For |
|---------|------|---------|---------------|----------|
| pyttsx3 | System | Yes | No | Simple offline projects |
| gTTS | Cloud | No | No | Quick multilingual |
| Coqui TTS | Open-source | Yes | Yes | Customization, research |
| Piper | On-device | Yes | No | Fast local neural |
| Smallest.ai | Cloud | No | Yes | Ultra-low latency |

## Offline & Cross-Platform

### pyttsx3 - The Reliable Workhorse

Lightweight, offline TTS using system voices.

```python
import pyttsx3

engine = pyttsx3.init()

# List available voices
for voice in engine.getProperty('voices'):
    print(f"{voice.id}: {voice.name}")

# Configure
engine.setProperty('rate', 150)     # Words per minute
engine.setProperty('volume', 0.9)   # 0.0 to 1.0

# Select voice
voices = engine.getProperty('voices')
engine.setProperty('voice', voices[0].id)

# Speak
engine.say("Hello, world!")
engine.runAndWait()

# Save to file
engine.save_to_file("Save this", "output.mp3")
engine.runAndWait()
```

**Platforms**: Windows (SAPI5), macOS (NSSpeechSynthesizer), Linux (eSpeak)

**When to use**: Privacy-sensitive apps, accessibility tools, embedded systems

## Cloud-Based

### gTTS - Google Translate API

Simple interface to Google's TTS. Requires internet.

```python
from gtts import gTTS
import os

# Basic usage
tts = gTTS(text="Hello world", lang='en')
tts.save("hello.mp3")

# With language/accent options
tts = gTTS(
    text="Hello world",
    lang='en',
    tld='co.uk',  # British accent
    slow=False
)
tts.save("british.mp3")

# Multiple languages
languages = ['en', 'es', 'fr', 'de']
for lang in languages:
    tts = gTTS(f"Hello in {lang}", lang=lang)
    tts.save(f"hello_{lang}.mp3")
```

**Pros**: 30+ languages, simple API
**Cons**: Requires internet, potential rate limits, no fine control

### Smallest.ai Waves - Ultra-Low Latency

Premium API for real-time applications.

```python
from smallest import Waves

client = Waves(api_key="your_api_key")

# Generate speech
audio = client.speak(
    text="Welcome to the future!",
    voice_name="your_voice",
    output_format="mp3"
)

with open("output.mp3", "wb") as f:
    f.write(audio)
```

**Key Features**:
- <100ms latency
- Voice cloning from 5 seconds of audio
- 30+ languages

## Open-Source & Customizable

### Coqui TTS (XTTS-v2) - The Powerhouse

Deep learning toolkit with voice cloning in 17 languages.

```python
from TTS.api import TTS

# Load model (downloads on first use)
tts = TTS("tts_models/multilingual/multi-dataset/xtts_v2", gpu=True)

# Basic synthesis
tts.tts_to_file(
    text="Hello from Coqui TTS!",
    file_path="output.wav",
    language="en"
)

# Voice cloning (3-6 seconds of reference audio)
tts.tts_to_file(
    text="I can clone any voice!",
    file_path="cloned.wav",
    speaker_wav="reference_audio.wav",
    language="en"
)

# Cross-language cloning
# Clone English voice, synthesize in Spanish
tts.tts_to_file(
    text="Hola, puedo hablar en espanol con tu voz!",
    file_path="spanish.wav",
    speaker_wav="english_reference.wav",
    language="es"
)

# List available models
print(TTS().list_models())
```

**Features**:
- 17 language voice cloning
- <200ms streaming latency
- Fine-tuning on custom datasets

**Note**: Coqui AI shut down in 2024, but the project is community-maintained.

### Piper TTS - Fast Local Neural

Optimized for CPU, great for Raspberry Pi and edge devices.

```python
from pypipertts import PiperTTS

# Download model from https://huggingface.co/rhasspy/piper-voices
tts = PiperTTS(model_path="en_US-lessac-medium.onnx")

# Generate speech
audio = tts.synthesize("Hello from Piper!")

# Save to file
tts.synthesize_to_file(
    "Hello from Piper!",
    "output.wav"
)
```

**When to use**: Home automation, privacy-focused apps, embedded devices

## Specialized Models

### Orpheus - Real-Time Empathetic Speech

Llama-based speech LLM with streaming support.

```python
from orpheus import OrpheusModel

model = OrpheusModel()
audio = model.generate(
    "I understand how you feel.",
    emotion="empathetic"
)
```

### Kokoro - Efficient High-Quality

Lightweight 82M parameter model with Apache license.

```python
from kokoro import generate

audio = generate(
    "Hello world",
    voice="af_heart"
)

import soundfile as sf
sf.write("output.wav", audio, 24000)
```

### VibeVoice - Long-Form Multi-Speaker

For podcasts and conversations with up to 4 speakers.

```python
from vibevoice import generate_dialogue

script = [
    {"speaker": "A", "text": "Welcome to the show!"},
    {"speaker": "B", "text": "Thanks for having me!"},
]

audio = generate_dialogue(script, duration_minutes=90)
```

## Decision Flowchart

```
Need offline?
├── Yes -> Basic quality OK?
│   ├── Yes -> pyttsx3
│   └── No -> Coqui TTS or Piper
└── No -> Need voice cloning?
    ├── Yes -> Budget OK?
    │   ├── Yes -> Smallest.ai
    │   └── No -> Coqui TTS
    └── No -> gTTS (simple) or Google Cloud (production)
```

## Installation

```bash
# Create virtual environment
python -m venv tts_env
source tts_env/bin/activate

# Basic libraries
pip install pyttsx3 gTTS

# Advanced (Coqui TTS)
pip install TTS

# Piper
pip install PyPiperTTS
# Download models from HuggingFace

# Smallest.ai
pip install smallestai
```

## Best Practices

1. **Text Preprocessing**: Clean text before TTS (handle abbreviations, numbers, symbols)

2. **GPU for Advanced Models**: Coqui TTS benefits significantly from CUDA

3. **License Review**:
   - pyttsx3, Piper, Kokoro: Commercial-friendly
   - Coqui TTS: Check Coqui Public Model License
   - gTTS: Check Google's terms

4. **Error Handling**:
```python
try:
    tts.synthesize(text)
except NetworkError:
    # Fallback to offline
    pyttsx3_engine.say(text)
except RateLimitError:
    # Implement backoff
    time.sleep(60)
```

## Comparison Table

| Feature | pyttsx3 | gTTS | Coqui | Piper | Smallest.ai |
|---------|---------|------|-------|-------|-------------|
| Offline | Yes | No | Yes | Yes | No |
| Quality | Medium | Medium | High | High | High |
| Latency | Low | High | Medium | Low | Ultra-Low |
| Voice Cloning | No | No | Yes | No | Yes |
| Languages | Limited | 30+ | 17 | 30+ | 30+ |
| Commercial | Yes | Check | Check | Yes | Yes |
