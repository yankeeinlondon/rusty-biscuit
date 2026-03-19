# Rusty Biscuit Monorepo


## Monorepo Structure

The repository is organized into the following package areas and packages:

```txt
• biscuit-file
    • biscuit-file-cli v0.1.0 (biscuit-file/cli) [Rust]
    • biscuit-file v0.1.0 (biscuit-file/lib) [Rust]
• biscuit-hash
    • biscuit-hash-cli v0.1.0 (biscuit-hash/cli) [Rust]
    • biscuit-hash v0.1.0 (biscuit-hash/lib) [Rust]
• biscuit-speaks
    • biscuit-speaks-cli v0.1.0 (biscuit-speaks/cli) [Rust]
    • biscuit-speaks v0.1.0 (biscuit-speaks/lib) [Rust]
• biscuit-terminal
    • biscuit-terminal-cli v0.1.0 (biscuit-terminal/cli) [Rust]
    • biscuit-terminal v0.1.0 (biscuit-terminal/lib) [Rust]
• biscuit-visualized
    • biscuit-visualized v0.1.0 (biscuit-visualized) [Rust]
• claudine
    • claudine-cli v0.1.0 (claudine/cli) [Rust]
    • claudine v0.1.0 (claudine/lib) [Rust]
• darkmatter
    • darkmatter-cli v0.1.0 (darkmatter/cli) [Rust]
    • darkmatter v0.1.0 (darkmatter/lib) [Rust]
• homelab
    • homelab-cli v0.1.0 (homelab/cli) [Rust]
    • homelab v0.1.0 (homelab/lib) [Rust]
    • homelab-server v0.1.0 (homelab/server) [Rust]
• model-citizen
    • model-citizen-cli v0.1.0 (model-citizen/cli) [Rust]
    • model-citizen v0.1.0 (model-citizen/lib) [Rust]
• unchained-ai
    • model_id v0.1.0 (unchained-ai/model_id) [Rust]
    • unchained-ai-cli v0.1.0 (unchained-ai/cli) [Rust]
    • unchained-ai-gen v0.1.0 (unchained-ai/gen) [Rust]
    • unchained-ai v0.1.0 (unchained-ai/lib) [Rust]
• playa
    • playa-cli v0.1.0 (playa/cli) [Rust]
    • playa v0.1.0 (playa/lib) [Rust]
• queue
    • queue-cli v0.1.0 (queue/cli) [Rust]
    • queue v0.1.0 (queue/lib) [Rust]
• research
    • research-cli v0.1.0 (research/cli) [Rust]
    • research v0.1.0 (research/lib) [Rust]
• schematic
    • schematic-define v0.1.0 (schematic/define) [Rust]
    • schematic-definitions v0.1.0 (schematic/definitions) [Rust]
    • schematic-gen v0.1.0 (schematic/gen) [Rust]
    • schematic-schema v0.1.0 (schematic/schema) [Rust]
• sniff
    • sniff-cli v0.1.0 (sniff/cli) [Rust]
    • sniff v0.1.0 (sniff/lib) [Rust]
• root
    • tabby v0.1.0 (tabby) [Rust]
    • tui v0.1.0 (tui) [Rust]
• tabby
    • ui v0.1.0 (tabby/ui) [Rust]
• tree-hugger
    • tree-hugger-cli v0.1.0 (tree-hugger/cli) [Rust]
    • tree-hugger v0.1.0 (tree-hugger/lib) [Rust]
```

## Common Commands

This monorepo uses the `just` runner and the root directory as well as every "package area" has a `justfile` so you can target either ALL packages (from root) or any package area by moving into that directory. Commands found everywhere are:

- `just test` - for running unit and integration tests
- `just lint` - for running all lint tests
- `just build` - build
- `just install` - build for release and install all binary packages

You should also use `just` and these **just** commands whenever you need to run tests, lints, builds (unless there is an explicit reason to revert to just the underlying commands which the runner uses).

## Local Skills

This repository has the following local Agent skills defined `.claude/skills/`:

- `biscuit-hash` - Hashing trifecta: xxHash (fast), BLAKE3 (crypto), Argon2id (passwords)
- `biscuit-speaks` - Cross-platform TTS library and CLI (`so-you-say`) with multi-provider support (ElevenLabs, Say, eSpeak, Kokoro, etc.)
- `biscuit-terminal` - **Terminal authority**: detection, image rendering (Kitty/iTerm2 protocols), mermaid diagrams
- `claudine` - Universal hook/event handler for agentic CLIs (Claude, Codex, Gemini, Goose, Kimi, OpenCode, Qwen)
- `darkmatter` - Markdown parsing/rendering (delegates terminal rendering to biscuit-terminal)
- `homelab` - Home automation AV control (Sony ES receivers, Arcam amplifiers) via CLI and REST API
- `model-citizen` - Local LLM model management across Ollama, LM Studio, and Llama.cpp
- `playa` - Audio playback via host players, format detection, 88 embedded sound effects
- `research` - AI-powered library research with two-phase LLM pipeline
- `schematic` - REST API client code generation, OpenAPI import/export, Headers builder
- `sniff` - System detection (OS, hardware, network, filesystem, programs, services)
- `so-you-say` - TTS CLI (`speak` binary) wrapping biscuit-speaks (located at `biscuit-speaks/cli`)
- `clap` - Command-line argument parsing
- `color-eyre` - Error reporting
- `ratatui` - Terminal UI framework
- `resvg` - SVG rendering
- `rig` - LLM agent framework
- `syntect` - Syntax highlighting
- `thiserror` - Error derive macros
- `unchained-ai` - LLM pipeline primitives, provider registry, model catalogs, rig-core integration, and agent status monitoring

**IMPORTANT:** All local skills are defined locally because they have strong relevance to some areas

## Rust Documentation Best Practices

- Avoid explicit `# Heading` (H1) inside a `///` docblock unless intentionally titling the item
    - Rustdoc already supplies the item name as a top-level title.
    - Adding an H1 duplicates visual hierarchy and is usually redundant.
- Use `## Heading` (H2) for primary sections
    - Example Sections:
        - `## Returns`
        - `## Errors`
        - `## Panics`
        - `## Safety`
        - `## Examples`
        - `## Notes`
- This aligns with:
    - Rust Standard Library documentation
    - rustc and clippy codebases
    - IDE hover and symbol views
- Use ### Heading (H3) only for subsections
    - Example:
        - `## Environment Variables`
        - `### Priority Order`
        - `### Fallback Behavior`
- Recommended section order
  1. Brief summary paragraph (no heading)
  2. `## Examples`
  3. `## Returns` (functions)
  4. `## Errors` (if applicable)
  5. `## Panics` (if applicable)
  6. `## Safety` (for unsafe APIs)
  7. `## Notes` or `## Implementation Notes`

## Testing

- **wiremock** (v0.6): HTTP mocking for provider API tests
- **tempfile** (v3.15): Temporary directories for research output tests
- **serial_test**: Test isolation for environment variable manipulation

For complete dependency information, see `docs/dependencies.md`.

### Error Handling

- Library code uses `thiserror` for error types
- No `unwrap()` or `expect()` in production code paths (only in tests)
- All public functions return `Result` types

### Documentation Conventions

**Package READMEs** follow a layered structure (see `docs/package-structure.md` for full details):

- **Base README** (package area root): Functional goals, links to sub-module READMEs
- **Sub-module READMEs** (lib/, cli/, etc.): Technical approach, key crates, lessons learned section
- **`docs/` folder**: `dependencies.md` plus research/design documents

**Avoiding Drift**: When modifying code, update relevant documentation in the same change:

- READMEs when changing public APIs or behavior
- `docs/dependencies.md` when adding or removing crates
- Skill files (`.claude/skills/`) when changing patterns or architecture
- This `CLAUDE.md` when changing workflows or project conventions

## Additional Documentation

For deeper architectural details, see:

- **`docs/package-structure.md`**: README conventions, docs folder patterns, drift prevention
- **`docs/dependencies.md`**: Complete dependency list with descriptions and links
- **`docs/tracing.md`**: Comprehensive tracing architecture (665 lines) - libraries emit/apps configure, PromptHook implementation, OpenTelemetry integration
- **`research/docs/architecture.md`**: Research pipeline internals - prompt templates, metadata schema, package manager detection, LLM provider rationale
- **Code review from 2025-12-30**: `.ai/code-reviews/20251230.provider-base-implementation.md` - identifies code duplication issues in provider module
