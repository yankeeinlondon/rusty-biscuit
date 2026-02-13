---
name: so-you-say
description: CLI for text-to-speech using system TTS providers. Use when working on the `speak` binary, adding CLI features, debugging TTS from command line, or testing voice/provider configurations. For library-level TTS work, use the biscuit-speaks skill instead.
---

## Purpose

`so-you-say` is a CLI wrapper around `biscuit-speaks` that provides:
- Text-to-speech from command line or stdin
- Voice/provider discovery and listing
- Voice selection with intelligent resolution
- Volume and speed controls
- Provider-specific configuration
- Cache management

**Binary Name**: `so-you-say` (package: `biscuit-speaks-cli`, located at `biscuit-speaks/cli`)

## Quick Reference

```bash
# Basic usage
speak "Hello, world!"
echo "Hello" | speak

# Voice selection
speak --voice Samantha "Hello"
speak --gender female "Hello"
speak --lang fr "Bonjour"

# Provider control
speak --list-providers
speak --list-voices --provider say
speak --provider elevenlabs "Premium voice"

# Audio controls
speak --loud "Announcement!"
speak --soft "Whisper"
speak --fast "Quick message"
speak --slow "Deliberate speech"

# Metadata and cache
speak --meta "Show what was used"
speak --refresh-cache
```

## CLI Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--list-providers` | | List available TTS providers |
| `--list-voices` | | List voices for a provider (prompts if no `--provider`) |
| `--provider <name>` | | Use specific provider (say, espeak, elevenlabs, etc.) |
| `--voice <name>` | | Select voice by name (case-insensitive) |
| `--gender <m\|f>` | `-g` | Prefer male or female voice |
| `--lang <code>` | `-l` | Language code (en, fr, de, etc.) |
| `--loud` | | Maximum volume |
| `--soft` | | Reduced volume |
| `--fast` | | Faster speech rate |
| `--slow` | | Slower speech rate |
| `--meta` | | Display voice/provider metadata after speaking |
| `--refresh-cache` | | Clear and repopulate voice cache |

## Voice Resolution

The CLI performs intelligent voice name resolution:

1. **Display name matching**: `--voice Thorsten` matches `de_DE-thorsten-high`
2. **Case-insensitive**: `--voice heart` matches `Heart`
3. **Quality deduplication**: Multiple quality variants (Premium/Enhanced) resolved to highest
4. **Language disambiguation**: Use `--lang` when same name exists in multiple languages

## Key Implementation Details

- **Input handling**: Joins CLI args with spaces; reads stdin if no args (10KB limit)
- **Exit codes**: 0 on success, 1 on error (no input, unknown provider, etc.)
- **Markdown output**: Voice lists rendered as markdown tables via `darkmatter-lib`
- **Cache location**: `~/.biscuit-speaks-cache.json`

## Related Skills

- **[biscuit-speaks](../biscuit-speaks/SKILL.md)**: Underlying TTS library (providers, traits, caching)

## Detailed Documentation

- [CLI Reference](cli-reference.md) - Complete argument documentation with examples
- [Examples](examples.md) - Common usage patterns and workflows
