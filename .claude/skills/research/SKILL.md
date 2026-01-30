---
name: research
description: Expert knowledge for the dockhand research package - an AI-powered library research tool that uses a two-phase LLM pipeline to generate comprehensive documentation, skills, and deep dives for software libraries. Use when working in research/, running research commands, or building AI research automation.
---

# Research Package

The `research` package provides AI-powered automated research for software libraries using a two-phase LLM pipeline. It generates structured documentation suitable for both humans and Claude Code skills.

## Package Structure

```
research/
├── cli/          # Binary: `research` command
└── lib/          # Core research library
```

## Quick Reference

### Commands

| Command | Purpose |
|---------|---------|
| `research library <TOPIC>` | Research a software library |
| `research list [FILTERS]` | List all research topics |
| `research link [FILTERS]` | Create symlinks to skill directories |
| `research show <TOPIC>` | Open deep dive document |
| `research pull <TOPIC>` | Copy skill to current repository |
| `research api <NAME>` | Research a public API (placeholder) |

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `RESEARCH_DIR` | Base output directory | No (default: `$HOME`) |
| `OPENAI_API_KEY` | GPT-5.2 for synthesis | Yes |
| `GEMINI_API_KEY` | Gemini Flash for research | Yes |
| `ZAI_API_KEY` | GLM-4.7 for overview | No (fallback to Gemini) |
| `BRAVE_API_KEY` | Web search tool | No |
| `BRAVE_PLAN` | Brave plan tier: `free`, `base`, `pro` | No (default: `free`) |

### Output Location

```
${RESEARCH_DIR:-$HOME}/.research/library/<topic>/
├── metadata.json           # Research metadata (v1 schema)
├── overview.md             # Phase 1: Library analysis
├── similar_libraries.md    # Phase 1: Alternatives
├── integration_partners.md # Phase 1: Ecosystem
├── use_cases.md            # Phase 1: Patterns
├── changelog.md            # Phase 1: Version history
├── question_N.md           # Phase 1: Custom questions
├── deep_dive.md            # Phase 2: Dense reference
├── brief.md                # Phase 2b: Quick summary
└── skill/
    └── SKILL.md            # Phase 2: Claude Code skill
```

## Topics

### Architecture & Pipeline

- [Pipeline Architecture](./pipeline.md) - Two-phase research pipeline, model selection, parallel execution
- [Metadata System](./metadata.md) - Schema versions, topic types, inventory system

### CLI Usage

- [CLI Commands](./cli-commands.md) - Detailed command reference with examples
- [Incremental Research](./incremental.md) - DRY approach, overlap detection, re-synthesis

### Development

- [Testing](./testing.md) - Test patterns, mocking, environment isolation
- [Extending](./extending.md) - Adding new research types, prompts, providers

## Common Operations

### Basic Library Research

```bash
# Research a Rust crate
research library clap

# With additional questions
research library tokio "How does the runtime scheduler work?" "What are the channel types?"

# Custom output filenames
research library serde "json -> How does JSON support work?"

# Force full re-research
research library clap --force

# Regenerate skill only
research library clap --skill
```

### Managing Research

```bash
# List all research topics
research list

# Filter by type
research list -t library

# Filter by pattern
research list "serde*" "tokio"

# Verbose with issues
research list --verbose

# JSON output
research list --json

# Migrate v0 metadata
research list --migrate
```

### Pulling Skills to Repos

```bash
# Pull skill to .claude/skills/<topic>/
research pull clap

# Also copy underlying research docs
research pull clap --local
```

## Two-Phase Pipeline Summary

**Phase 1** (parallel): 5 standard prompts + custom questions
- Uses Gemini Flash (fast) for most tasks
- Uses GLM-4.7 for overview, GPT-5.2 for changelog
- All run concurrently via `tokio::join!`

**Phase 2** (after Phase 1): Synthesis
- **2a** (parallel): `skill/SKILL.md` + `deep_dive.md` via GPT-5.2
- **2b** (sequential): `brief.md` via Gemini Flash

See [Pipeline Architecture](./pipeline.md) for detailed model selection rationale.

## Development Commands

```bash
# Build
cargo build -p research-cli
just -f research/justfile build

# Test
cargo test -p research-lib
just -f research/justfile test

# Install
just -f research/justfile install

# Run in development
just -f research/justfile cli library clap
```

## Key Dependencies

- **rig-core**: LLM agent framework
- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **pulldown-cmark**: Markdown normalization
- **biscuit-speaks**: TTS completion announcements
