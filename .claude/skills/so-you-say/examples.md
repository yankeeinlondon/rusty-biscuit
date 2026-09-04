# Usage Examples

## Basic Text-to-Speech

### Ordered background speech

```bash
so-you-say --background "First message"
so-you-say --background "Second message"
playa spool
```

Both invocations return after durable handoff. The private Playa scheduler
plays them in publication order without overlap, even after either requesting
shell exits. A cache miss reserves its position before synthesis, so a later
cache hit cannot overtake it. Delivery is best-effort and at-most-once after
playback starts.

### Simple Usage

```bash
# Speak text directly
speak "Hello, world!"

# Multiple words as arguments
speak Hello world from the command line

# With unicode
speak "Bonjour 世界 🎉"
```

### From Stdin

```bash
# Pipe from echo
echo "This is piped text" | speak

# From a file
cat announcement.txt | speak

# From command output
git log --oneline -1 | speak

# Multi-line heredoc
speak <<EOF
Welcome to the system.
Please stand by for further instructions.
EOF
```

---

## Voice Selection

### By Voice Name

```bash
# macOS voices
speak --voice Samantha "Hello"
speak --voice "Ava (Premium)" "Premium quality"

# Kokoro voices
speak --voice Heart "Neural voice"
speak --voice Michael "Male neural voice"

# VITS voices (via echogarden)
speak --voice Thorsten "Hallo"  # Matches de_DE-thorsten-high
```

### By Gender

```bash
# Prefer female voice
speak --gender female "Hello"
speak -g female "Short form"

# Prefer male voice
speak --gender male "Hello"
speak -g male "Short form"
```

### By Language

```bash
# French
speak --lang fr "Bonjour le monde"

# German
speak --lang de "Guten Tag"

# Spanish
speak --lang es "Hola mundo"

# With voice disambiguation
speak --voice Alex --lang es "Spanish Alex voice"
speak --voice Alex --lang pt "Portuguese Alex voice"
```

### Combined Selection

```bash
# French female voice
speak --lang fr --gender female "Bonjour"

# English male, slower speed
speak --lang en --gender male --slow "Take your time"
```

---

## Provider Control

### Listing Providers

```bash
# See what's available
speak --list-providers

# Example output:
# Available TTS providers:
#
#   - say (macOS)
#   - kokoro (Kokoro TTS)
#   - echogarden
#   - espeak (eSpeak-NG)
#   - elevenlabs (ElevenLabs API) [cloud]
```

### Listing Voices

```bash
# Interactive selection
speak --list-voices

# For specific provider
speak --list-voices --provider say
speak --list-voices --provider kokoro
speak --list-voices --provider echogarden

# Filtered by language
speak --list-voices --provider gtts --lang fr
speak --list-voices --provider say --lang en
```

### Using Specific Provider

```bash
# macOS native
speak --provider say "Using macOS Say"

# High-quality neural (local)
speak --provider kokoro "Using Kokoro TTS"

# Cloud provider (requires API key)
speak --provider elevenlabs "Using ElevenLabs"

# Fallback to formant synthesis
speak --provider espeak "Using eSpeak"
```

---

## Audio Controls

### Volume

```bash
# Maximum volume
speak --loud "ATTENTION PLEASE!"

# Soft/quiet
speak --soft "Whispered message"
```

### Speed

```bash
# Fast delivery
speak --fast "This is urgent news"

# Slow and clear
speak --slow "Please listen carefully"
```

### Combined

```bash
# Loud and fast (announcement)
speak --loud --fast "Breaking news!"

# Soft and slow (relaxation)
speak --soft --slow "Relax and breathe"
```

---

## Metadata and Debugging

### Show What Was Used

```bash
speak --meta "Hello"
# Output:
#
#   Provider: say (macOS)
#   Voice: Samantha
#   Gender: Female
#   Quality: Good
#   Volume: normal
#   Speed: normal
#   Cache: miss
```

### With ElevenLabs

```bash
speak --provider elevenlabs --meta "Premium voice"
# Output:
#
#   Provider: elevenlabs (ElevenLabs API)
#   Voice: Rachel
#   Voice ID: 21m00Tcm4TlvDq8ikWAM
#   Gender: Female
#   Quality: Excellent
#   Volume: normal
#   Speed: normal
#   Model: eleven_multilingual_v2
#   Audio File: /tmp/tts-cache/...
#   Codec: mp3
#   Cache: hit
```

### Cache Management

```bash
# Refresh after installing new voices
speak --refresh-cache

# Then check updated voice list
speak --list-voices --provider say
```

---

## Scripting Patterns

### Notification System

```bash
#!/bin/bash
# notify.sh - Speak a notification

MESSAGE="${1:-Task complete}"
speak --soft "$MESSAGE"
```

### Language Detection

```bash
#!/bin/bash
# speak-detected.sh - Detect language and speak

TEXT="$1"
LANG=$(echo "$TEXT" | head -c 100 | langdetect 2>/dev/null || echo "en")
speak --lang "$LANG" "$TEXT"
```

### Build Notification

```bash
#!/bin/bash
# build-notify.sh - Announce build status

if cargo build --release 2>&1; then
    speak "Build successful"
else
    speak --loud "Build failed!"
    exit 1
fi
```

### Reading Clipboard

```bash
# macOS
pbpaste | speak

# Linux (xclip)
xclip -selection clipboard -o | speak

# Linux (xsel)
xsel --clipboard | speak
```

### Timer Announcement

```bash
#!/bin/bash
# timer.sh - Countdown timer with voice

MINUTES=${1:-5}
sleep "${MINUTES}m"
speak --loud "Timer complete! $MINUTES minutes have passed."
```

---

## Error Handling

### Unknown Provider

```bash
speak --provider invalid "Hello"
# Error: Unknown provider 'invalid'
# Use --list-providers to see available providers
```

### Voice Not Found

```bash
speak --provider say --voice NonexistentVoice "Hello"
# Error: Voice 'NonexistentVoice' not found for say (macOS)
# Use --list-voices --provider say to see available voices
```

### No Input

```bash
speak
# (with closed stdin)
# Error: No input provided
# Usage: so-you-say <text> or echo "text" | so-you-say
```

### Conflicting Options

```bash
speak --loud --soft "Hello"
# error: the argument '--loud' cannot be used with '--soft'

speak --fast --slow "Hello"
# error: the argument '--fast' cannot be used with '--slow'
```

---

## Development Workflows

### Testing Voice Changes

```bash
# Quick iteration on voice selection
for voice in Samantha Alex Ava; do
    echo "Testing: $voice"
    speak --voice "$voice" --meta "This is $voice"
done
```

### Provider Comparison

```bash
# Compare same text across providers
TEXT="The quick brown fox jumps over the lazy dog."

for provider in say kokoro echogarden; do
    echo "=== $provider ==="
    speak --provider "$provider" --meta "$TEXT"
done
```

### Voice Quality Audit

```bash
# List all voices sorted by quality
speak --list-voices --provider say | head -20
```
