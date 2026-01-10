# Rusty Biscuit
<img src="./assets/biscuit-and-crab.png" style="position: fixed; max-width: 30%; height: 150px; right: 0; top: 0; opacity: 0.75"></img>

> A monorepo for AI-powered research and automation tools

- All libraries, CLI's, and TUI's are written in Rust.
- Many CLI/TUI's are published to `npm` as well as `cargo`

## Packages

This monorepo hosts the following packages:

### Shared Libraries

1. **biscuit** [`./shared`]

    Provides utility functions and a highly capable markdown pipelining engine.

2. **schematic** [`./api`]

    Builds type-strong API's to be consumed by other libraries/apps.

3. **ai-pipeline** [`./ai-pipeline`]

    Provides a set of AI pipeline primitives for Agent composition while re-exporting some `rig` primitives to allow lower level interaction as well.


### Applications

1. **researcher** [`./research`]
2. **md** CLI [`./md`]
3. **observer** TUI [`./observer`]
4. **notable** [`./notable`]
5. **so-you-say** CLI [`so-you-say`]

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

**Examples:**

```bash
# Speak text from command-line arguments
so-you-say Hello world

# Speak text from stdin (pipe support)
echo "Hello world" | so-you-say

# Speak text from a file
cat announcement.txt | so-you-say

# Multi-word with punctuation
so-you-say "Good morning, team!"
```

**Installation:**

```bash
# Install globally
cargo install --path so-you-say

# Binary will be available at ~/.cargo/bin/speak
so-you-say "Installation successful"
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
