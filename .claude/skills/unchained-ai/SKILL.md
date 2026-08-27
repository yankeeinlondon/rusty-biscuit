---
name: unchained-ai
description: Expert knowledge for the unchained-ai LLM pipeline library including pipeline primitives, provider registry, model catalogs, rig-core integration, code generation, and agent status monitoring. Use when working in unchained-ai/, building LLM pipelines, adding providers/models, implementing pipeline steps, running the model generator, or querying agentic platform limits.
user-invocable: false
---

## Purpose

Unchained AI is the monorepo's LLM pipeline framework. It provides:

- **Pipeline primitives**: typed state, composable steps (`Runnable`), serial/parallel execution
- **Provider registry**: 13 providers with unified configuration (auth, endpoints, env vars)
- **Model catalogs**: auto-generated provider-specific enums with metadata from models.dev and provider-native sources
- **Rig-core tools**: `BraveSearchTool` and `ScreenScrapeTool` implementations
- **Agent delegation**: `OpenCodeDelegation` for external agentic CLI integration
- **Abstract model selection**: `ModelCapability` enum for capability-based model choice
- **Agent services**: platform detection, PTY-based status queries, cap limit monitoring

## Package Layout

```
unchained-ai/
├── lib/          # Core library (unchained-ai crate)
│   └── src/
│       ├── primitives/   # Pipeline state, Runnable trait, grouping
│       │   └── services/ # Agent status detection, PTY runner, parsers
│       ├── rigging/      # Provider registry, model enums, rig tools
│       ├── models/       # ModelCapability, ProviderModelMetadata, ModelPricing, ModelDefaultParameters
│       ├── execution/    # CompletionBackend seam + complete()/complete_blocking()
│       ├── api/          # OpenAI-compatible model discovery
│       └── utils/        # Epoch datetime helper
├── contract/     # unchained-ai-contract: InferenceAdapter over the execution surface
├── gen/          # Binary: gen-models (provider enum generator)
├── cli/          # Binary: unchained (agent status monitoring CLI)
│   └── src/
│       ├── main.rs          # CLI entry point (clap + clap_complete)
│       └── commands/
│           ├── limits.rs    # `limits` subcommand (progress bars, JSON)
│           └── models.rs    # `models` subcommand (model listing, verbose metadata)
```

## Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `PipelineState` | `primitives::state` | Heterogeneous typed state container |
| `Runnable` | `primitives::runnable` | Step execution trait (mutable + read-only) |
| `Pipeline` | `primitives::grouping` | Serial step composition |
| `InParallel<R>` | `primitives::grouping` | Parallel homogeneous execution |
| `Prompt<V>` | `primitives::atomic` | Multi-modal prompt builder |
| `ModelCapability` | `models::model_capability` | Abstract model selection |
| `ProviderModel` | `rigging::providers::models` | Concrete provider/model pair |
| `ProviderModelMetadata` | `models::model_metadata` | Rich metadata (pricing, params, modalities, etc.) |
| `ModelPricing` | `models::model_pricing` | Per-token/request pricing (USD) |
| `ModelDefaultParameters` | `models::model_default_parameters` | Provider-recommended generation defaults |
| `ModelIdentity` | `models::identity` | Wire-id identity grammar: family/version/variant decomposition, `latest_in_family` |
| `CompletionBackend` | `execution` | Injectable seam for text completions |
| `CompletionRequest` / `CompletionOutput` | `execution` | Single-turn execution surface |
| `ResolvedParameters` | `execution` | Metadata defaults + caller overrides |
| `EnvView` / `HashMapEnvView` / `StdEnvView` | `models::selection` | Environment view for resolver tests |
| `Provider` | `rigging::providers::provider` | Provider enum with config |
| `UnchainedInferenceAdapter` | `unchained-ai-contract` | `InferenceAdapter` implementation for `biscuit-contract` |
| `OpenCodeDelegation` | `primitives::atomic` | Agent CLI delegation |
| `AgentStatus` | `primitives::services::agent_status` | Platform detection and cap limit queries |
| `AgenticStatusPlatform` | `primitives::services::agent_status` | Platform enum (ClaudeCode, Codex) |
| `AgenticCapLimit` | `primitives::services::agent_status` | Short/long-term cap usage data |
| `AgentStatusError` | `primitives::services::error` | Error types for agent status operations |

## Dependencies

- **rig-core** v0.31.0 (features: rayon, image, derive, audio, rmcp, pdf)
- **model_id** (workspace crate at `../../model_id`) - derive macro for model enums
- **sniff** (workspace crate) - platform detection for `InstalledAiClients`
- **portable-pty** v0.9 - PTY spawning for agent status commands
- **strip-ansi-escapes** v0.2 - ANSI code stripping from PTY output
- **tokio** v1.49, **reqwest** v0.12, **scraper** v0.25, **serde**/**serde_json** v1.0

## Common Commands

```bash
# Build all
just -f unchained-ai/justfile build

# Test all
just -f unchained-ai/justfile test

# Run CLI in dev mode
just -f unchained-ai/justfile cli limits
just -f unchained-ai/justfile cli limits --platform claude --json

# Install CLI binary
just -f unchained-ai/justfile install

# Generate model enums from live APIs
just -f unchained-ai/justfile gen
cargo run -p unchained-ai-gen -- --providers openai,anthropic
cargo run -p unchained-ai-gen -- --dry-run
```

`unchained-ai-contract` keeps its installed-provider integration target behind
the internal `real-tests` feature. Ordinary L1 omits that target;
`just test-real` enables it together with the runtime environment gate.

## Implementation Status

**Implemented**: Pipeline state/execution, Prompt building (multi-modal), `Prompt::execute()` via `complete_blocking()` + capability resolver, execution surface (`CompletionBackend`, `complete()`, `complete_blocking()`), capability-based model resolver (`models::selection`), OpenCode delegation, provider registry (13 providers), model enums (auto-generated), model metadata (models.dev + provider-native merge), rich OpenRouter metadata (pricing, architecture, default parameters), rig tools (BraveSearch, ScreenScrape), client adaptors (Z.ai, ZenMux), ModelCapability serialization, `UnchainedInferenceAdapter` in `unchained-ai-contract`, agent status detection (ClaudeCode, Codex), PTY-based status command execution, cap limit parsing, CLI binary with `limits` subcommand (terminal + JSON output)

**Not implemented**: `UserContent`/`Transcribe` (placeholder structs), `ForeignAgent` trait (incomplete skeleton, not publicly exported), `SmartConcat` (scaffold), HuggingFace API module (empty)

## Detailed References

- [Pipeline Primitives](./pipeline-primitives.md) - State, Runnable trait, grouping, atomic steps
- [Providers and Models](./providers-and-models.md) - Provider registry, model enums, metadata, client adaptors
- [Model Generator](./model-generator.md) - gen-models CLI, models.dev integration, enum generation
- [Agent Services](./agent-services.md) - Platform detection, PTY runner, cap limit parsing, CLI usage
- [Rig](./rig/SKILL.md) - The `rig-core` framework that underpins the `rigging` module — type-safe agents, tool calling, agentic loops, embeddings, vector-store/RAG integration, and custom providers. Reach for this when working with the provider registry, rig tools, or building agent/RAG pipelines on top of rig.
