# Unchained AI

`unchained-ai` is the Dockhand monorepo area for LLM workflow primitives, provider/model plumbing, and agent-facing CLI utilities.

## Package Surface

| Path | Crate / Binary | Purpose |
| --- | --- | --- |
| `unchained-ai/lib` | `unchained-ai` | Core library: pipeline primitives, provider registry, model catalogs, rig tools |
| `unchained-ai/cli` | `unchained` | CLI for agent platform usage/cap monitoring |
| `unchained-ai/gen` | `gen-models` | Generates provider model enums and metadata lookup tables |
| `unchained-ai/model_id` | `model_id` | Proc-macro derive used by generated provider model enums |

Note: the `unchained-ai` workspace includes `lib`, `cli`, and `gen`; `model_id` is a sibling crate consumed by `lib`.

## Architecture Summary

- `primitives`: typed pipeline state, `Runnable`, serial composition (`Pipeline`), and read-only grouped execution (`InParallel`).
- `primitives::atomic`: `Prompt` plus `OpenCodeDelegation`, with additional placeholders (`UserContent`, `Transcribe`).
- `primitives::services`: platform detection and cap parsing for Claude Code/Codex via PTY status checks.
- `rigging::providers`: provider registry/config plus generated model enums and aggregated `ProviderModel`.
- `rigging::tools`: rig `Tool` implementations (`BraveSearchTool`, `ScreenScrapeTool`).
- `models`: abstract model capability targeting (`ModelCapability`) and runtime metadata types (`ProviderModelMetadata`, `ModelPricing`, `ModelDefaultParameters`).
- `api`: OpenAI-compatible model discovery utilities.

## Current Status

- Implemented:
    - pipeline/state primitives and validation
    - provider registry and generated provider model enums
    - models.dev-backed metadata generation and runtime metadata lookup
    - Provider-native metadata merging (OpenRouter pricing, parameters, architecture)
    - OpenCode delegation primitive
    - agent cap monitoring for Claude Code and Codex
    - rig tools for web search and scraping
    - `Prompt::execute` via the `execution/` surface and capability-based model resolver
    - `UnchainedInferenceAdapter` in `unchained-ai-contract` for `biscuit-contract`
- Not fully implemented:
    - placeholder scaffolds (`UserContent`, `Transcribe`, `foreign_agent`, parts of functional grouping)

## Development

```bash
# Build lib + cli + generator
just -f unchained-ai/justfile build

# Test lib + cli + generator
just -f unchained-ai/justfile test

# Run CLI in development mode
just -f unchained-ai/justfile cli limits --json

# Regenerate provider model enums and metadata
just -f unchained-ai/justfile gen --dry-run
```

## Readme Map

- Library overview: `unchained-ai/lib/README.md`
- CLI usage: `unchained-ai/cli/README.md`
- Generator usage: `unchained-ai/gen/README.md`
- Model derive macro: `unchained-ai/model_id/README.md`
