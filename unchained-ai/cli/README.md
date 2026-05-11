# Unchained CLI

The `unchained` CLI provides tools for monitoring and managing AI agentic platforms.

## Installation

```bash
cargo install --path .
# or via justfile:
just -f unchained-ai/justfile install
```

## Commands

### `unchained limits`

Show usage limits and cap status for detected agentic platforms (Claude Code, Codex).

```bash
# Show limits for all detected platforms
unchained limits

# Filter to a specific platform
unchained limits --platform claude
unchained limits --platform codex

# Output as JSON
unchained limits --json
```

Output includes progress bars showing short-term and long-term cap usage for each platform.

### `unchained models`

List all known provider models with optional metadata display.

```bash
# List all models across all providers
unchained models

# Filter to a specific provider
unchained models --provider openai
unchained models --provider anthropic

# Show verbose metadata (context window, modalities, pricing, etc.)
unchained models --provider openai --verbose

# Output as JSON
unchained models --json
```

## Shell Completions

```bash
# Generate bash completions
unchained --completions bash > /usr/local/share/bash-completion/completions/unchained

# Generate zsh completions
unchained --completions zsh > ~/.zfunc/_unchained
```

## Architecture

- Uses `sniff` library to detect installed agentic platforms
- Uses `portable_pty` to spawn status commands in a pseudo-terminal
- Parses output with ANSI stripping for reliable text extraction
- Renders progress bars using `biscuit-terminal` Progress component
