# CLI Reference

## Synopsis

```
so-you-say [OPTIONS] [TEXT]...
speak [OPTIONS] [TEXT]...
```

## Arguments

### `[TEXT]...`

Text to speak. Multiple arguments are joined with spaces.

If no text is provided, reads from stdin (up to 10,000 characters).

## Options

### Provider Selection

#### `--list-providers`

List all available TTS providers on the system and exit.

```bash
speak --list-providers
# Output:
# Available TTS providers:
#
#   - say (macOS)
#   - kokoro (Kokoro TTS)
#   - echogarden
#   - elevenlabs (ElevenLabs API) [cloud]
```

Cloud providers are tagged with `[cloud]`.

---

#### `--list-voices`

List available voices for a provider and exit.

```bash
# Interactive provider selection
speak --list-voices

# Specific provider
speak --list-voices --provider say

# Filter by language
speak --list-voices --provider gtts --lang fr
```

Output is rendered as a markdown table with columns: Voice, Description, Language, Quality, Gender.

---

#### `--provider <name>`

Use a specific TTS provider.

**Valid provider names**:
- `say` - macOS built-in
- `espeak` - eSpeak-NG
- `sapi` - Windows Speech API
- `kokoro` - Kokoro TTS
- `echogarden` - Multi-engine (Kokoro/VITS)
- `gtts` - Google TTS CLI
- `elevenlabs` - ElevenLabs cloud API

```bash
speak --provider say "Hello from macOS"
speak --provider elevenlabs "Premium cloud voice"
```

**Error on unknown provider**:
```
Error: Unknown provider 'foo'
Use --list-providers to see available providers
```

---

### Voice Selection

#### `--voice <name>`

Select a specific voice by name. Case-insensitive.

```bash
# By display name
speak --voice Samantha "Hello"

# By technical name (VITS-style)
speak --voice de_DE-thorsten-high "Hallo"

# Case-insensitive
speak --voice HEART "Hello"
```

**Voice Resolution**:
1. Matches against display name (extracted from technical names)
2. Matches against original voice name
3. Deduplicates quality variants (keeps highest quality)
4. Returns error if ambiguous (use `--lang` to disambiguate)

---

#### `--gender <male|female>`, `-g <male|female>`

Prefer a voice of the specified gender.

```bash
speak --gender male "Hello"
speak -g female "Hello"
```

**Note**: Ignored if `--voice` is also specified.

---

#### `--lang <code>`, `-l <code>`

Language code for voice selection.

```bash
# Set language preference
speak --lang fr "Bonjour le monde"

# Filter voice listing
speak --list-voices --provider say --lang en

# Disambiguate same-name voices
speak --voice Alex --lang es "Hola"
```

**Language matching**:
- `en` matches `Language::English` and `en-*` variants
- `fr` matches `fr`, `fr-CA`, `fr-FR`, etc.

---

### Audio Controls

#### `--loud`

Increase volume to maximum level. Conflicts with `--soft`.

```bash
speak --loud "Attention please!"
```

---

#### `--soft`

Decrease volume to softer level. Conflicts with `--loud`.

```bash
speak --soft "Quiet message"
```

---

#### `--fast`

Increase speech rate. Conflicts with `--slow`.

```bash
speak --fast "Quick update"
```

---

#### `--slow`

Decrease speech rate. Conflicts with `--fast`.

```bash
speak --slow "Take your time"
```

---

### Metadata and Debugging

#### `--background`

Durably hand speech to Playa's private per-user queue and return immediately.
Cache hits publish ready audio; cache misses reserve their sequence before a
detached preparation helper synthesizes. Jobs remain globally ordered with
other Playa, biscuit-speaks, and Claudine audio and survive requester exit.

Preparation failure or the ten-minute deadline marks the job failed and allows
the queue to continue. `PLAYA_DRY_RUN=1` returns successfully without touching
the capability cache, audio cache, spool, journal, or subprocesses.

```bash
so-you-say --background "Build complete"
playa spool
```

#### `--meta`

Display metadata about the voice and provider used after speaking.

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

Additional fields for specific providers:
- **Voice ID**: For ElevenLabs voices
- **Model**: For model-based providers
- **Audio File**: Path to cached/generated audio
- **Codec**: Audio format used

---

#### `--refresh-cache`

Clear the TTS provider cache and repopulate with fresh data.

```bash
speak --refresh-cache
# Output:
# Clearing TTS provider cache...
# Repopulating cache from all available providers...
# Cache refreshed successfully.
```

Use after installing new voices or when voice listings appear stale.

**Cache location**: `~/.biscuit-speaks-cache.json`

---

### Standard Options

#### `--help`, `-h`

Show help message and exit.

---

#### `--version`, `-V`

Show version information and exit.

---

## Input Handling

### Command-Line Arguments

```bash
# Single argument
speak "Hello world"

# Multiple arguments (joined with spaces)
speak Hello world from tests
# Speaks: "Hello world from tests"

# Unicode and special characters
speak "Hello 世界 🚀"
speak "Hello, world! How's it going?"
```

### Stdin

When no text arguments are provided, reads from stdin.

```bash
# Pipe text
echo "Hello world" | speak

# Here-doc
speak <<EOF
This is a multi-line
message to speak.
EOF

# From file
cat message.txt | speak
```

**Limits**:
- Maximum 10,000 characters from stdin
- Empty stdin results in error (exit code 1)

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (no input, unknown provider, voice not found, TTS failure) |

---

## Environment Variables

These are read by the underlying `biscuit-speaks` library:

| Variable | Purpose |
|----------|---------|
| `ELEVENLABS_API_KEY` | ElevenLabs API authentication |
| `ELEVEN_LABS_API_KEY` | Alternative ElevenLabs key |
| `KOKORO_MODEL` | Path to Kokoro ONNX model |
| `KOKORO_VOICES` | Path to Kokoro voice embeddings |
| `TTS_PROVIDER` | Override default provider selection |
| `PREFER_LANGUAGE` | Default language preference |
| `PREFER_GENDER` | Default gender preference |
| `PREFER_VOICE` | Default voice preference |
| `PREFER_SPEED` | Default speed (fast/slow) |
| `DEBUG` | Enable debug output for voice resolution |
