# Dockhand

> A monorepo for AI-powered research and automation tools

## Architecture

This monorepo is organized into **areas**, each containing related modules:

### Research Area (`/research`)

The Research area provides automated research capabilities for software libraries using a two-phase LLM pipeline.

- **Research Library** (`/research/lib`) - Core library providing AI integration via the `rig` crate
- **Research CLI** (`/research/cli`) - Command-line interface exposing research capabilities
  - Binary name: `research`
  - Usage: `research library <topic> [additional questions...]`

For detailed documentation, see [`/research/README.md`](./research/README.md).


### SoYouSay Area (`/so-you-say`)

A CLI which leverages the shared library's TTS functionality to provide a cross-platform way to announce progress leveraging the underlying host's resources.

- **Binary name:** `speak`
- **Usage:** Convert text to speech using system TTS

**Examples:**

```bash
# Speak text from command-line arguments
speak Hello world

# Speak text from stdin (pipe support)
echo "Hello world" | speak

# Speak text from a file
cat announcement.txt | speak

# Multi-word with punctuation
speak "Good morning, team!"
```

**Installation:**

```bash
# Install globally
cargo install --path so-you-say

# Binary will be available at ~/.cargo/bin/speak
speak "Installation successful"
```

### Shared Library (`/shared`)

Common utilities shared across multiple areas of the monorepo.

### TUI Area (`/tui`)

A `ratatui`-based TUI application for interactive chat. (Future development)

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RESEARCH_DIR` | Base directory for research output | `$HOME` |
| `OPENAI_API_KEY` | OpenAI API key | (required) |
| `GEMINI_API_KEY` | Google Gemini API key | (required) |
| `ZAI_API_KEY` | ZAI API key | (required) |

### Research Output Location

Research is stored at: `${RESEARCH_DIR:-$HOME}/.research/library/<pkg>/`

## Usage

### Building

```bash
# Build all areas
just build

# Build specific area
just -f research/justfile build
```

### Testing

```bash
# Test all areas
just test

# Test specific area
just -f research/justfile test
```

### Installing

```bash
# Install binaries from all areas
just install
```

### Research CLI

```bash
# Research a library
research library clap

# Research with additional questions
research library clap "How does it compare to structopt?" "What are the derive macros?"

# Run in development mode
just -f research/justfile cli library clap
```

## Library Research Pipeline

The research system uses a two-phase LLM pipeline:

```
Phase 1: Underlying Research (parallel)
├── overview.md          [GLM-4-7]      - Library features and API
├── similar_libraries.md [Gemini Flash] - Alternatives and comparisons
├── integration_partners.md [Gemini Flash] - Ecosystem libraries
├── use_cases.md         [Gemini Flash] - Patterns and examples
├── changelog.md         [GPT-5.2]      - Version history
└── question_N.md        [Gemini Flash] - Additional prompts

Phase 2: Synthesis (parallel)
├── skill/SKILL.md       [GPT-5.2]      - Claude Code skill
└── deep_dive.md         [GPT-5.2]      - Comprehensive reference
```

### Incremental Research (DRY)

The system detects existing research via `metadata.json` and:

- Runs overlap detection on new prompts
- Interactive selection for conflicts
- Re-synthesizes Phase 2 with expanded corpus

### Metadata

Each research output includes `metadata.json` tracking:

- Library info (package manager, language, URL)
- Additional research prompts
- Creation and update timestamps
