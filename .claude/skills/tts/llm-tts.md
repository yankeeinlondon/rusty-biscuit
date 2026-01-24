# LLM-TTS Voice Control

LLM-TTS represents a paradigm shift from formal markup (SSML) to natural language instructions for controlling speech synthesis.

## Core Concept

Instead of:
```xml
<prosody rate="fast" pitch="+20%">Exciting news!</prosody>
```

You write:
```
"Say this excitedly and quickly"
```

The LLM interprets instructions and adjusts synthesis parameters automatically.

## Architecture Patterns

### Two-Stage Architecture

Most common approach separating linguistic understanding from speech generation.

```
Text + Instructions → LLM → Semantic + Prosodic Annotations → TTS Model → Audio
```

**Examples**: Most current systems (CosyVoice, InstructTTS)

### End-to-End Integrated

LLM generates audio tokens directly alongside text tokens.

```
Text + Instructions → Multimodal LLM → Audio Tokens → Vocoder → Audio
```

**Examples**: Spark-TTS, GLM-TTS, Gemma-TTS

### Operationalist Framework (BatonVoice)

Explicit decoupling with LLM as "conductor" generating vocal feature plans.

```
Instructions → LLM (Conductor) → Feature Plan (pitch, energy, rate) → TTS (Orchestra) → Audio
```

## Key Capabilities

### Natural Language Instructions

```python
# Describe desired characteristics
generate_speech(
    text="Welcome to our store!",
    instructions="Speak warmly and enthusiastically, like greeting a friend"
)

# Adjust specific aspects
generate_speech(
    text="The meeting is canceled.",
    instructions="Deliver this news gently and apologetically"
)

# Character/persona
generate_speech(
    text="Your mission, should you choose to accept it...",
    instructions="Sound like a mysterious spy briefing agent"
)
```

### Discrete Speech Tokenization

Speech represented as discrete tokens for LLM processing.

**BiCodec (Spark-TTS)**:
- **Semantic tokens**: Linguistic content (~50 tokens/second)
- **Global tokens**: Speaker characteristics, style

**Benefits**:
- Efficient LLM-native processing
- Separable control of content vs style

### Reinforcement Learning Alignment

Systems like GLM-TTS use multi-reward RL (e.g., GRPO) to optimize:
- Pronunciation accuracy
- Emotional appropriateness
- Naturalness

## Implementation Examples

### InstructTTS-Style Control

```python
from instructts import InstructTTS

tts = InstructTTS()

# Natural language style prompt
audio = tts.generate(
    text="I can't believe we won!",
    instruction="Express extreme excitement and joy, speaking quickly with high energy"
)

# Voice persona description
audio = tts.generate(
    text="Please proceed to gate 14.",
    instruction="Professional female announcer, calm and clear, slight British accent"
)
```

### CosyVoice3 Instruction Control

```python
from cosyvoice import CosyVoice

model = CosyVoice()

# Fine-grained control
audio = model.synthesize(
    text="The results are in.",
    voice_prompt="A nervous teenager speaking quickly",
    pitch_adjustment="+10%",
    speaking_rate=1.2
)

# Cross-lingual with instructions
audio = model.synthesize(
    text="Bonjour, comment allez-vous?",
    reference_audio="english_speaker.wav",  # 3-second English sample
    instruction="Speak the French text with a slight American accent"
)
```

### Spark-TTS Zero-Shot Cloning

```python
from spark_tts import SparkTTS

tts = SparkTTS()

# Clone voice with natural language description
audio = tts.generate(
    text="Let me explain the situation.",
    reference_audio="speaker.wav",
    style_description="Calm and reassuring, speaking slowly"
)

# Adjust cloned voice characteristics
audio = tts.generate(
    text="HURRY UP!",
    reference_audio="speaker.wav",
    style_description="Same voice but now urgent and stressed"
)
```

## Comparison: SSML vs LLM-TTS

| Aspect | SSML | LLM-TTS |
|--------|------|---------|
| Control precision | Exact (Hz, ms, %) | Approximate (natural language) |
| Learning curve | Steep (XML syntax) | Low (plain English) |
| Reproducibility | Perfect | Variable |
| Expressiveness | Limited by tags | Unlimited descriptions |
| Cross-platform | Good (W3C standard) | Limited (emerging) |
| Real-time capability | Good | Improving |

### When to Use SSML

- Precise timing requirements (subtitles, sync)
- Specific pronunciation (proper nouns, technical terms)
- Reproducible results across runs
- Cross-platform compatibility needed

### When to Use LLM-TTS

- Emotional/expressive content
- Rapid prototyping
- Non-technical users
- Creative/narrative applications
- Character voice design

## Current Systems

| System | Approach | Latency | Cloning | Languages |
|--------|----------|---------|---------|-----------|
| InstructTTS | LLM + TTS | Medium | Limited | Research |
| CosyVoice3 | Instruction + Streaming | ~150ms | 3-second | Multi |
| Spark-TTS | BiCodec | Medium | Zero-shot | Multi |
| GLM-TTS | Two-stage + RL | Low | Planned | CN/EN |
| OpenAI TTS | Implicit | Low | No | Multi |
| ElevenLabs | AI analysis | Low | 1-minute | Multi |

## Technical Specifications

### Interface Protocols

**HTTP Streaming**:
```json
POST /v1/speech
{
  "text": "Hello world",
  "voice": "nova",
  "instructions": "Speak cheerfully",
  "stream": true
}
```

Response: Chunked audio stream

**WebSocket** (for real-time):
```json
{
  "type": "generate",
  "text": "Responding to your question...",
  "instructions": "Thoughtful and measured"
}
```

### Audio Specifications

| Format | Sample Rate | Use Case |
|--------|-------------|----------|
| PCM S16LE | 16kHz | Telephony |
| PCM S16LE | 24kHz | Standard neural TTS |
| MP3 | 48kHz | Highest quality |
| OPUS | Variable | Streaming/bandwidth |

### Latency Targets

- First audio frame: <500ms
- Complete sentence: <1s
- Real-time factor: <0.3 (3x faster than real-time)

## Challenges

### Technical

- **Latency**: Large LLMs slow for real-time
- **Compute**: Significant GPU requirements
- **Consistency**: Same instruction may yield different results

### Practical

- **Evaluation**: Subjective quality hard to measure
- **Cultural variation**: Emotion expression varies by culture
- **Bias**: Training data reflects societal biases

## Future Directions

### Enhanced Control

- Continuous emotion sliders alongside instructions
- Real-time adaptation based on listener feedback
- Multimodal input (facial expressions, gestures)

### Efficiency

- Model distillation for edge deployment
- Speculative decoding for lower latency
- 4-bit quantization without quality loss

### Personalization

- User preference learning
- Context-aware adaptation
- Federated learning for privacy

## Resources

- [InstructTTS Paper](https://arxiv.org/abs/2305.18703)
- [CosyVoice GitHub](https://github.com/FunAudioLLM/CosyVoice)
- [Spark-TTS](https://github.com/SparkAudio/Spark-TTS)
- [GLM-TTS](https://modelscope.cn/models/ZhipuAI/GLM-TTS)
- [VoxBox Dataset](https://github.com/SparkAudio/Spark-TTS) - 100K hours for controllable TTS
