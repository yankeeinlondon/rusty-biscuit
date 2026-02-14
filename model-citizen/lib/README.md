# model-citizen (library)

Core library for local LLM model discovery, metadata extraction, and management.

## Architecture

### Scanner System

The `ModelScanner` trait defines the interface for runner-specific model discovery:

```rust
#[async_trait]
pub trait ModelScanner: Send + Sync {
    fn name(&self) -> &'static str;
    async fn is_available(&self) -> bool;
    async fn scan(&self) -> Result<Vec<UnifiedModel>>;
    async fn enrich(&self, model: &mut UnifiedModel) -> Result<()> { Ok(()) }
}
```

Three scanners are provided:

| Scanner | Discovery | Enrichment | Formats |
|---------|-----------|------------|---------|
| **Ollama** | Filesystem manifests + API `/api/tags` | Lazy via `/api/show` | GGUF |
| **LM Studio** | Filesystem scan + API `/v1/models` | During scan | GGUF, MLX/Safetensors |
| **Llama.cpp** | Configured directories | None (GGUF headers) | GGUF |

### Registry

`ModelRegistry` aggregates scanners and runs them concurrently with timeout (default: 5s). Features:
- Graceful degradation on scanner failures
- Deduplication by file path
- `ScanResult` enum for per-scanner diagnostics (Success/Error/Timeout/Unavailable)

### Model Types

- `UnifiedModel` - Core model representation with id, name, size, quantization, architecture, source, format, path, metadata
- `ModelMetadata` - Rich optional metadata (context length, parameters, capabilities, inference defaults, HuggingFace repo)
- `QuantizationType` - 27 variants (GGUF quants, float types, IQ series, MLX types)
- `ModelArchitecture` - 10 variants: 9 families (Llama, Mistral, Qwen, Phi, Gemma, Command, Yi, DeepSeek, StarCoder) + Unknown
- `ModelFormat` - File format: Gguf, Safetensors, Unknown
- `ModelSource` - Runner origin: Ollama, LmStudio, LlamaCpp

### GGUF Parsing (`gguf` module)

- Filename-based quantization detection (61 patterns covering case and separator variants)
- Header validation (magic bytes, version)
- Full metadata extraction via `gguf-rs-lib` (architecture, context length, embedding dimensions)

### HuggingFace Client (`huggingface` module)

- Search models with sort options (downloads, likes, trending, created, modified)
- List GGUF variants for a repository
- Stream downloads with progress callbacks and `.tmp` file safety
- Optional auth via `HF_TOKEN` / `HUGGING_FACE_API_KEY` / `HF_API_KEY`

### Sharing (`sharing` module)

- Cross-platform symlink creation (Unix symlinks, Windows fallback chain: symlink -> hard link -> copy)
- `ShareRegistry` tracks symlink relationships in caller-specified JSON file (default via `default_registry_path()`: `<data_local_dir>/model-citizen/shares.json`)
- Symlink validation and recursive resolution (max depth: 20)

## Key Dependencies

- `thiserror` - Error types
- `tokio`, `async-trait`, `futures` - Async runtime
- `reqwest` (json, stream) - HTTP client
- `serde`, `serde_json`, `toml` - Serialization
- `gguf-rs-lib` - GGUF file parsing
- `schematic-definitions` - Ollama/LM Studio API types
- `schematic-schema` - HuggingFace API client
- `sniff` - OS detection for platform-specific paths
- `dirs` - Platform directory resolution

## Lessons Learned

- Ollama enrichment (`/api/show`) is expensive per-model; use lazy enrichment only for `info` command
- LM Studio MLX detection requires checking for both `config.json` AND `.safetensors` files
- GGUF filename patterns vary wildly; case-insensitive matching with multiple separators is essential
- Platform-specific model paths differ significantly; use `sniff` for OS detection
