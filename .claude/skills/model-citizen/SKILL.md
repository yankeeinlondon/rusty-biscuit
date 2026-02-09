---
name: model-citizen
description: Expert knowledge for the model-citizen Rust library and CLI for managing local LLM models across Ollama, LM Studio, and Llama.cpp. Use when working in the model-citizen/ directory, adding scanner support, modifying GGUF parsing, HuggingFace integration, model sharing, or the `model` CLI.
---

# Model Citizen

Local LLM model management library and CLI (`model` binary). Provides unified model discovery across Ollama, LM Studio, and Llama.cpp with GGUF/MLX format support.

## Package Structure

```
model-citizen/
├── lib/          # Core library (model-citizen crate)
├── cli/          # Binary: `model` (model-citizen-cli crate)
├── docs/         # Runner documentation and model management guides
└── justfile      # Build/test/lint/install recipes
```

## Scanner Architecture

The `ModelScanner` trait defines runner-specific model discovery:

```rust
#[async_trait]
pub trait ModelScanner: Send + Sync {
    fn name(&self) -> &'static str;
    async fn is_available(&self) -> bool;
    async fn scan(&self) -> Result<Vec<UnifiedModel>>;
    async fn enrich(&self, model: &mut UnifiedModel) -> Result<()> { Ok(()) }
}
```

| Scanner | Discovery | Enrichment | Formats |
|---------|-----------|------------|---------|
| Ollama | Filesystem manifests + `/api/tags` | Lazy `/api/show` | GGUF |
| LM Studio | Filesystem + `/v1/models` | During scan | GGUF, MLX |
| Llama.cpp | Configured directories | GGUF headers only | GGUF |

`ModelRegistry` runs scanners concurrently (5s timeout), deduplicates by path, reports per-scanner status via `ScanResult`.

## Key Types

| Type | Purpose |
|------|---------|
| `UnifiedModel` | Core model: id, name, size, quantization, architecture, source, format, path, metadata |
| `ModelMetadata` | Optional rich data: context_length, parameters, capabilities, inference defaults, HF repo |
| `QuantizationType` | 25+ variants: GGUF quants (Q4_K_M etc), float (F16/F32), IQ series, MLX (Bf16/Bit4/Bit8) |
| `ModelArchitecture` | 9 families: Llama, Mistral, Qwen, Phi, Gemma, Command, Yi, DeepSeek, StarCoder |
| `ModelSource` | Ollama, LmStudio, LlamaCpp |
| `Config` | Merged config: env vars > TOML file > defaults |

## CLI Commands

Binary: `model` with global `--format table|json`

| Command | Description |
|---------|-------------|
| `list` | All models. Flags: `--runner`, `--verbose`, `--app`, `--size` |
| `info <model>` | Detailed metadata with lazy enrichment. Interactive selection on ambiguous match |
| `search <query>` | HuggingFace GGUF search. `--limit`, `--sort` (downloads/likes/trending/created/modified) |
| `download <repo> [variant]` | Stream download with progress. Interactive multi-select if no variant |
| `remove <model>` | Delete with confirmation. `--runner`, `--force`. Warns about dependent symlinks |
| `completions` | Shell completion setup (Bash/Zsh/Fish) |

## Common Tasks

### Adding a new scanner

1. Create `lib/src/scanner/<runner>.rs` implementing `ModelScanner`
2. Add module in `lib/src/scanner/mod.rs`
3. Register in CLI startup (see `commands/list.rs` for pattern)
4. Add runner docs in `docs/apps/`

### Adding a new quantization type

1. Add variant to `QuantizationType` enum in `lib/src/model.rs`
2. Update `from_str_loose()` with pattern matching
3. Update filename detection in `lib/src/gguf.rs` (`quantization_from_filename`)

### Modifying GGUF parsing

Key functions in `lib/src/gguf.rs`:
- `quantization_from_filename()` - 30+ case-insensitive patterns
- `extract_metadata()` - Architecture-prefixed key extraction via `gguf-rs-lib`
- `detect_quantization()` - Filename detection + header fallback

## Detailed Topics

- [Scanners](./scanners.md) - Per-scanner discovery details, platform paths, API integration
- [Types & Config](./types.md) - Full type details, configuration, environment variables

## Build & Test

```bash
just -f model-citizen/justfile build
just -f model-citizen/justfile test
just -f model-citizen/justfile lint
just -f model-citizen/justfile install
cargo test -p model-citizen -p model-citizen-cli
```

## Key Dependencies

**Library**: `thiserror`, `tokio`, `async-trait`, `reqwest` (json/stream), `serde`/`serde_json`/`toml`, `gguf-rs-lib`, `schematic-definitions`, `schematic-schema`, `sniff`, `dirs`

**CLI**: `clap` (derive) + `clap_complete` (dynamic), `biscuit-terminal`, `tabled`, `inquire`, `indicatif`, `color-eyre`

## Resources

- [Package README](../../../model-citizen/README.md)
- [Library README](../../../model-citizen/lib/README.md)
- [CLI README](../../../model-citizen/cli/README.md)
