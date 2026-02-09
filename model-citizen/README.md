# Model Citizen

> A library and CLI for managing local AI models across multiple runners

Model Citizen provides:

- A unified inventory of models downloaded for **Ollama**, **LM Studio**, and **Llama.cpp**
- Symlink-based model sharing between runners (avoids duplicate storage)
- Metadata inspection (quantization, architecture, context length, inference defaults)
- HuggingFace search and GGUF model downloads with progress tracking
- Support for both **GGUF** and **MLX/Safetensors** model formats

## CLI

> `model <subcommand> [options]`

| Command | Description |
|---------|-------------|
| `list` | List all models across all runners |
| `info <model>` | Show detailed metadata about a model |
| `search <query>` | Search for GGUF models on HuggingFace |
| `download <repo> [variant]` | Download model variants from HuggingFace |
| `remove <model>` | Remove a model (with confirmation) |
| `completions` | Shell completion setup instructions |

**Global options:** `--format table|json`

### Examples

```bash
model list                          # List all models (table)
model list --runner ollama          # Filter by runner
model list --size                   # Sort by size (largest first)
model list --format json            # JSON output
model info llama3                   # Detailed model info
model search "qwen2 7b"            # Search HuggingFace
model search "phi" --sort likes     # Sort by likes
model download bartowski/Qwen2.5-7B-Instruct-GGUF
model remove mistral --force        # Skip confirmation
```

## Configuration

Config file: `~/.config/model-citizen/config.toml`

```toml
shared_models_dir = "/path/to/shared/models"
llamacpp_models_dirs = ["/path/to/llama/models"]
enable_sharing = true

[scanners.ollama]
enabled = true
timeout_secs = 5

[scanners.lmstudio]
enabled = true
timeout_secs = 5

[scanners.llamacpp]
enabled = true
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `MODEL_CITIZEN_SHARED_DIR` | Shared models directory |
| `LLAMA_CPP_MODELS` | Comma-separated Llama.cpp model directories |
| `OLLAMA_HOST` | Ollama API host (default: `http://localhost:11434`) |
| `LM_STUDIO_HOST` | LM Studio API host (default: `http://localhost:1234`) |
| `HF_TOKEN` | HuggingFace API token (for authenticated downloads) |

## Sub-packages

- **[lib/](lib/)** - Core library: scanners, GGUF parsing, HuggingFace client, sharing
- **[cli/](cli/)** - Binary `model`: interactive CLI with table/JSON output
