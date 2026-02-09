# Scanner Details

## Ollama Scanner (`scanner/ollama.rs`)

### Discovery

1. Scans filesystem manifests at platform-specific paths:
   - **macOS**: `~/.ollama/models/manifests/`
   - **Linux**: `/usr/share/ollama/.ollama/models` (system) or `~/.ollama/models`
   - **Windows**: `%LOCALAPPDATA%\Ollama\models`
2. Parses manifest JSON (`registry.ollama.ai/library/<model>/<tag>`) for model name and size
3. Calls `/api/tags` for basic enrichment (quantization, modified_at)
4. Falls back to API-only mode if no filesystem models found

### Lazy Enrichment (`enrich()`)

Called only during `model info`, not listing. `POST /api/show` returns:
- `parameters` string (newline-separated key-value pairs, parsed by `parse_parameters()`)
- `families`, `parent_model`, `license`, `chat_template`
- Architecture-prefixed metadata keys (context_length, embedding_length, head_count, layer_count)
- HuggingFace repo from `model_info` JSON
- Inference defaults: temperature, top_k, top_p, repeat_penalty, stop tokens
- `capabilities` array (vision, tools, etc.)

### API Types

Uses `schematic-definitions` Ollama types:
- `ListModelsResponse` / `ModelResponse` for `/api/tags`
- `ShowModelResponse` for `/api/show`

---

## LM Studio Scanner (`scanner/lmstudio.rs`)

### Discovery

1. Detects models directory:
   - From `settings.json` `downloadsFolder` override
   - **macOS default**: `~/Library/Application Support/LM Studio/models`
   - **Linux default**: `~/.cache/lm-studio/models`
   - **Windows default**: `%APPDATA%\LM Studio\models`
2. Recursively scans for:
   - **GGUF files** (`.gguf` extension): Parsed with `gguf.rs` utilities
   - **MLX directories**: Must contain both `config.json` AND `.safetensors` file(s)

### MLX Model Detection

- **Size**: Sum of all `.safetensors` files in directory
- **Quantization**: From `config.json` keys `quantization.bits` or `quantization_config.bits`
- **Architecture**: From `config.json` `model_type` (fallback: name-based detection)
- **Context/Embedding**: Standard HuggingFace config keys:
  - `max_position_embeddings` / `max_seq_len` / `max_sequence_length` -> context_length
  - `hidden_size` -> embedding_length
  - `num_attention_heads` -> head_count
  - `num_hidden_layers` -> layer_count

### HuggingFace Repo Extraction

- From GGUF headers: `general.source.huggingface.repository` or `general.source.url`
- From directory structure: `models_dir/publisher/repo-name` -> `publisher/repo-name`

### API Enrichment

Single call to `/v1/models`. Matches by ID containment in model name/path. Populates: parameters, capabilities (vision, function_calling), publisher.

---

## Llama.cpp Scanner (`scanner/llamacpp.rs`)

### Discovery

- Scans user-configured directories only (no fixed path)
- Configured via `LLAMA_CPP_MODELS` env var (comma-separated) or config file `llamacpp_models_dirs`
- Recursively walks directories for `.gguf` files
- Parses each file with `gguf.rs`: quantization from filename/header, architecture from name, rich metadata from GGUF headers

### Key Differences

- No API integration (Llama.cpp server has no model inventory endpoint)
- `enrich()` is a no-op (all data comes from filesystem scan)
- `is_available()` returns `true` if any configured directories exist

---

## Registry Deduplication

`ModelRegistry` uses `HashSet<String>` of normalized file paths. First scanner to discover a file path wins. Handles symlinks via the sharing module.

## Adding a New Scanner

1. Create `lib/src/scanner/<name>.rs`
2. Implement `ModelScanner` trait:
   ```rust
   pub struct MyScanner { config: Config }

   #[async_trait]
   impl ModelScanner for MyScanner {
       fn name(&self) -> &'static str { "my-runner" }
       async fn is_available(&self) -> bool { /* check API/filesystem */ }
       async fn scan(&self) -> Result<Vec<UnifiedModel>> { /* discover models */ }
       async fn enrich(&self, model: &mut UnifiedModel) -> Result<()> { /* optional */ Ok(()) }
   }
   ```
3. Add `pub mod <name>;` in `scanner/mod.rs`
4. Register scanner in CLI command handlers (see `commands/list.rs` for the pattern of building a registry)
5. Add `ModelSource` variant in `model.rs` if needed
6. Add runner documentation in `docs/apps/<name>.md`
