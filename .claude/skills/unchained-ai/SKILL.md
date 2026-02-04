---
name: unchained-ai
description: Expert knowledge for the unchained-ai LLM pipeline library including pipeline primitives, provider registry, model catalogs, rig-core integration, and code generation. Use when working in unchained-ai/, building LLM pipelines, adding providers/models, implementing pipeline steps, or running the model generator.
user-invocable: false
---

## Purpose

Unchained AI is the monorepo's LLM pipeline framework. It provides:

- **Pipeline primitives**: typed state, composable steps (`Runnable`), serial/parallel execution
- **Provider registry**: 13 providers with unified configuration (auth, endpoints, env vars)
- **Model catalogs**: auto-generated provider-specific enums with metadata from Parsera
- **Rig-core tools**: `BraveSearchTool` and `ScreenScrapeTool` implementations
- **Agent delegation**: `OpenCodeDelegation` for external agentic CLI integration
- **Abstract model selection**: `ModelCapability` enum for capability-based model choice

## Package Layout

```
unchained-ai/
├── lib/          # Core library (unchained-ai crate)
│   └── src/
│       ├── primitives/   # Pipeline state, Runnable trait, grouping
│       ├── rigging/      # Provider registry, model enums, rig tools
│       ├── models/       # ModelCapability, ModelMetadata
│       ├── api/          # OpenAI-compatible model discovery
│       └── utils/        # Epoch datetime helper
├── gen/          # Binary: gen-models (provider enum generator)
└── cli/          # Binary: unchained (FUTURE - not implemented)
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
| `Provider` | `rigging::providers::provider` | Provider enum with config |
| `OpenCodeDelegation` | `primitives::atomic` | Agent CLI delegation |

## Dependencies

- **rig-core** v0.29.0 (features: rayon, image, derive, audio, rmcp, pdf)
- **model_id** (workspace crate at `../../model_id`) - derive macro for model enums
- **tokio** v1.49, **reqwest** v0.12, **scraper** v0.25, **serde**/**serde_json** v1.0

## Common Commands

```bash
# Build
cargo build -p unchained-ai
cargo build -p unchained-ai-gen

# Test
cargo test -p unchained-ai

# Generate model enums from live APIs
cargo run -p unchained-ai-gen
cargo run -p unchained-ai-gen -- --providers openai,anthropic
cargo run -p unchained-ai-gen -- --dry-run
```

## Implementation Status

**Implemented**: Pipeline state/execution, Prompt building (multi-modal), OpenCode delegation, provider registry (13 providers), model enums (auto-generated), model metadata (Parsera), rig tools (BraveSearch, ScreenScrape), client adaptors (Z.ai, ZenMux), ModelCapability serialization

**Not implemented**: `Prompt::execute()` (returns fatal error - LLM execution not wired), CLI binary, `UserContent`/`Transcribe` (placeholder structs), `ForeignAgent` trait (incomplete skeleton), `SmartConcat`/`splinter` (scaffold), HuggingFace API module (empty)

## Detailed References

- [Pipeline Primitives](./pipeline-primitives.md) - State, Runnable trait, grouping, atomic steps
- [Providers and Models](./providers-and-models.md) - Provider registry, model enums, metadata, client adaptors
- [Model Generator](./model-generator.md) - gen-models CLI, Parsera integration, enum generation
