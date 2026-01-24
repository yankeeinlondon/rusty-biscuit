# Local TTS CLI Tools

Comparison of local TTS solutions that run without cloud APIs.

## Summary Table

| Provider | Quality | Hardware | Base Voices | SSML | Output to OS | Network |
|----------|---------|----------|-------------|------|--------------|---------|
| [say](https://ss64.com/mac/say.html) | *** | Low | Yes | No | Yes | No |
| [espeak-ng](https://github.com/espeak-ng/espeak-ng) | ** | Low | Yes | Yes | Yes | No |
| [Piper](https://github.com/OHF-Voice/piper1-gpl) | **** | Medium | Download | No | Yes | No |
| [sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/) | ***** | High | Download | Planned | Yes | No |
| [Festival](http://www.cstr.ed.ac.uk/projects/festival/) | ** | Low | Yes | Partial | Yes | No |
| [Mimic3](https://github.com/MycroftAI/mimic3) | **** | Medium | Download | Yes | Yes | No |
| [KokoroTts](https://github.com/nazdridoy/kokoro-tts) | **** | Medium | Download | No | Yes | No |

## macOS: say

Built-in, no installation required.

```bash
# List voices
say -v ?

# Speak with specific voice
say -v "Alex" "Hello world"

# Adjust rate (words per minute)
say -r 200 "This is faster"

# Save to file
say -o output.aiff "Save to file"

# Read from file
say -f document.txt
```

**Pros**: Zero setup, reliable
**Cons**: macOS only, limited voices, no SSML

## espeak-ng - Cross-Platform Formant

100+ languages, robotic but functional.

```bash
# Install
sudo apt-get install espeak-ng  # Linux
brew install espeak-ng           # macOS

# List voices
espeak-ng --voices

# Speak
espeak-ng "Hello world"

# Adjust speed and pitch
espeak-ng -s 150 -p 50 "Faster and higher"

# Save to WAV
espeak-ng -f input.txt -w output.wav

# SSML input
espeak-ng --markup '<speak>Hello <break time="500ms"/> world</speak>'
```

**Pros**: Tiny footprint, many languages, SSML support
**Cons**: Robotic quality, not suitable for production audio

## Piper - Fast Neural TTS

Modern neural quality with CPU efficiency. Runs on Raspberry Pi.

```bash
# Install
pip install piper-tts

# Download voice (from https://huggingface.co/rhasspy/piper-voices)
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json

# Synthesize
echo "Hello world" | piper --model en_US-lessac-medium.onnx --output_file output.wav

# Adjust speed (length_scale: <1.0 faster, >1.0 slower)
echo "Faster" | piper --model en_US-lessac-medium.onnx --length_scale 0.8 --output_file fast.wav

# Pipe to audio player
echo "Play immediately" | piper --model en_US-lessac-medium.onnx --output-raw | aplay -r 22050 -f S16_LE
```

**Note**: Project moved to [OHF-Voice/piper1-gpl](https://github.com/OHF-Voice/piper1-gpl) with GPL-3.0 license.

**Voice Sources**:
- Official: [rhasspy/piper-voices on HuggingFace](https://huggingface.co/rhasspy/piper-voices)
- Samples: [rhasspy.github.io/piper-samples](https://rhasspy.github.io/piper-samples/)
- Community: HAL 9000, GLaDOS, game characters on Home Assistant forums

**Pros**: Good quality, fast, runs on low-end hardware
**Cons**: No SSML, voice download required, GPL license

## sherpa-onnx - High-Quality Neural

Production-grade neural TTS with multiple model support.

```bash
# Build from source
git clone https://github.com/k2-fsa/sherpa-onnx.git
cd sherpa-onnx
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j4

# Download model (e.g., VITS)
./bin/sherpa-onnx-offline-tts --help

# Synthesize
./bin/sherpa-onnx-offline-tts \
  --vits-model=vits-piper-en_US-lessac-medium.onnx \
  --tokens=vits-piper-en_US-lessac-medium.tokens \
  --output-file=output.wav \
  "Hello world"
```

**Platforms**: Linux, macOS, Windows, Android, iOS, Raspberry Pi, WebAssembly

**Pros**: Highest quality offline, cross-platform including mobile
**Cons**: Complex setup, large models (hundreds of MB)

## Festival - Academic Classic

Mature system from Edinburgh University. Complex but extensible.

```bash
# Install
sudo apt-get install festival

# Interactive mode
festival
> (SayText "Hello world")
> (SayText "Save to file" nil "output.wav")

# Command line
echo "Hello world" | festival_client

# Server mode
festival --server &
echo "Hello" | festival_client
```

**Pros**: Extensible, academic support
**Cons**: Dated quality, complex configuration, slow development

## SpdSay - Linux Speech Dispatcher

Frontend for multiple TTS backends on Linux desktops.

```bash
# Install
sudo apt-get install speech-dispatcher

# List voices
spd-say -v ?

# Speak
spd-say "Hello world"

# Adjust volume
spd-say -i 50 "Lower volume"

# Use specific backend
spd-say -o espeak "Using espeak"
```

**Pros**: Integrates with Linux accessibility, multiple backends
**Cons**: Linux only, quality depends on backend

## Mimic3 - Mycroft Neural TTS

Neural TTS from Mycroft AI. Runs on Raspberry Pi 4.

```bash
# Install
sudo apt-get install mimic3-text-to-speech

# List voices
mimic3 --voices

# Synthesize
mimic3 --voice "en_US/ljspeech" "Hello world" > output.wav

# Adjust rate
mimic3 --voice "en_US/ljspeech" --rate 1.5 "Faster" > fast.wav

# SSML
mimic3 --voice "en_US/ljspeech" --ssml input.ssml > output.wav
```

**Warning**: Project is no longer maintained. Consider Piper instead.

## KokoroTts - Kokoro-82M CLI

High-quality 82M parameter model.

```bash
# Install
pip install kokoro-tts

# Python usage (no direct CLI)
python -c "
from kokoro import generate
import soundfile as sf
audio = generate('Hello world', voice='af_heart')
sf.write('output.wav', audio, 24000)
"
```

**Pros**: Near-commercial quality, tiny model
**Cons**: Limited CLI, American English primarily

## Recommendation by Use Case

```
Quick demo, macOS:
    -> say (built-in, zero setup)

Cross-platform, low resources:
    -> espeak-ng (robotic but universal)

Good quality, CPU-only:
    -> Piper (best quality/speed ratio)

Best quality, have GPU:
    -> sherpa-onnx with VITS models

Accessibility/screen reader:
    -> SpdSay (integrates with Linux a11y)

Home automation:
    -> Piper (Home Assistant integration)
```

## Quality vs Requirements Tradeoff

```
         Quality
            ^
            |     sherpa-onnx
            |       *
            |    Piper  KokoroTts
            |      *      *
            |   Mimic3
            |     *
            |
            |  say   Festival
            |   *      *
            |
            |  espeak-ng
            |     *
            +-----------------------> Hardware
              Low    Medium    High
```
