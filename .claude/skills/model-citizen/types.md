# Types & Configuration

## Core Types

### UnifiedModel (`model.rs`)

```rust
pub struct UnifiedModel {
    pub id: String,              // Format: "source:name:quantization"
    pub name: String,            // Human-readable name
    pub size_bytes: u64,         // File size
    pub quantization: QuantizationType,
    pub architecture: ModelArchitecture,
    pub source: ModelSource,
    pub format: ModelFormat,     // Gguf | Safetensors | Unknown
    pub path: PathBuf,           // Filesystem location
    pub metadata: ModelMetadata, // Rich optional metadata
}
```

### ModelMetadata (`model.rs`)

All fields are `Option` or `Vec` (may be empty):

```rust
pub struct ModelMetadata {
    // Model info
    pub parameters: Option<String>,     // e.g., "8B"
    pub context_length: Option<u64>,
    pub embedding_length: Option<u64>,
    pub head_count: Option<u64>,
    pub layer_count: Option<u64>,

    // Capabilities
    pub vision: Option<bool>,
    pub function_calling: Option<bool>,
    pub capabilities: Vec<String>,      // e.g., ["vision", "tools"]

    // Lineage
    pub families: Vec<String>,
    pub parent_model: Option<String>,
    pub publisher: Option<String>,
    pub huggingface_repo: Option<String>,

    // Inference defaults
    pub temperature: Option<f64>,
    pub top_k: Option<u64>,
    pub top_p: Option<f64>,
    pub repeat_penalty: Option<f64>,
    pub stop: Vec<String>,

    // Other
    pub license: Option<String>,
    pub chat_template: Option<String>,
    pub modified_at: Option<String>,
}
```

### QuantizationType (`model.rs`)

25+ variants with flexible parsing:

| Category | Variants |
|----------|----------|
| GGUF standard | Q4_0, Q4_1, Q4Km, Q4Ks, Q5_0, Q5_1, Q5Km, Q5Ks, Q6K, Q8_0 |
| Float | F16, F32 |
| IQ series | Iq1S, Iq2Xxs, Iq2Xs, Iq2S, Iq3Xxs, Iq3Xs, Iq3S, Iq4Xs, Iq4Nl |
| MLX | Bf16, Bit4, Bit6, Bit8, Mxfp4 |

Key methods:
- `as_str()` - Display string
- `from_str_loose(s)` - Handles case variations, separators (-, _, .)
- `from_mlx_bits(bits)` - Convert MLX bit count to type

### ModelArchitecture (`model.rs`)

```rust
pub enum ModelArchitecture {
    Llama, Mistral, Qwen, Phi, Gemma,
    Command, Yi, DeepSeek, StarCoder, Unknown,
}
```

`from_name(name)` detects from model name using keyword matching.

### ModelSource (`model.rs`)

```rust
pub enum ModelSource { Ollama, LmStudio, LlamaCpp }
```

Methods: `as_str()`, `display_name()`

### ModelCitizenError (`error.rs`)

```rust
pub enum ModelCitizenError {
    IoError(#[from] std::io::Error),
    ConfigError(String),
    NetworkError(String),
    ParseError(String),
    NotFound { path: PathBuf },
}
```

Convenience constructors: `config()`, `network()`, `parse()`, `not_found()`
Conversions: `From<io::Error>`, `From<toml::de::Error>`, `From<reqwest::Error>`

---

## Configuration (`config.rs`)

### Config Priority

1. Environment variables (highest)
2. Config file `~/.config/model-citizen/config.toml`
3. Default values

### Config Struct

```rust
pub struct Config {
    pub shared_models_dir: Option<PathBuf>,
    pub llamacpp_models_dirs: Vec<PathBuf>,
    pub enable_sharing: bool,
    pub scanners: ScannersConfig,
}

pub struct ScannersConfig {
    pub ollama: OllamaConfig,     // enabled, api_host, timeout_secs
    pub lmstudio: LmStudioConfig, // enabled, api_host, timeout_secs
    pub llamacpp: LlamaCppConfig,  // enabled
}
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `MODEL_CITIZEN_SHARED_DIR` | Shared models directory | `~/.local/share/model-citizen/shared` |
| `LLAMA_CPP_MODELS` | Comma-separated Llama.cpp directories | (none) |
| `OLLAMA_HOST` | Ollama API host | `http://localhost:11434` |
| `LM_STUDIO_HOST` | LM Studio API host | `http://localhost:1234` |
| `HF_TOKEN` / `HUGGING_FACE_API_KEY` / `HF_API_KEY` | HuggingFace auth token | (none) |

### Config Methods

- `Config::load()` - Full merge: file + env
- `Config::from_env()` - Environment only
- `Config::from_file(path)` - TOML file only
- `Config::from_default_file()` - Default path if exists
- `ollama_host()` / `lmstudio_host()` - Effective API hosts

---

## HuggingFace Client (`huggingface.rs`)

### Types

```rust
pub struct HuggingFaceClient { /* reqwest::Client + optional token */ }

pub struct GgufVariant {
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: QuantizationType,
    pub download_url: String,
}

pub struct SearchResult {
    pub repo_id: String,
    pub author: String,
    pub downloads: u64,
    pub likes: u64,
    pub variant_count: usize,
}

pub enum SortOrder { Downloads, Likes, Trending, Created, Modified }
```

### Methods

- `search_models(query, limit, sort)` -> `Vec<SearchResult>` (auto-appends "gguf" to query)
- `list_variants(repo_id)` -> `Vec<GgufVariant>` (filters `.gguf` files from `/api/models/{repo}/tree/main`)
- `download(repo_id, filename, dest_dir, progress_callback)` -> `PathBuf` (streams with `.tmp` safety)
- `cleanup_temp_files(dir)` - Removes `.tmp` download artifacts

---

## Sharing (`sharing.rs`)

### ShareRegistry

JSON persistence at `~/.local/share/model-citizen/shares.json`:

```rust
pub struct ShareRegistry {
    pub shares: HashMap<String, Vec<String>>, // original_path -> [symlink_paths]
}
```

Methods: `load()`, `save()`, `add_share()`, `remove_share()`, `get_shares()`, `find_original()`

### Symlink Functions

- `create_symlink(src, dest)` - Unix: symlink, Windows: symlink -> hard link -> copy
- `validate_symlink(path)` - Checks symlink target exists
- `resolve_original(path)` - Follows chain (max depth: 20)
- `is_symlink(path)` - Simple check
- `default_shared_dir()` - Platform-specific shared dir
- `default_registry_path()` - Registry JSON path

---

## GGUF Utilities (`gguf.rs`)

### Functions

- `parse_header(path)` -> `GgufHeader` (reads first 1KB, validates magic `GGUF`)
- `is_gguf_file(path)` -> `bool` (magic byte check)
- `quantization_from_filename(path)` -> `Option<QuantizationType>` (30+ case-insensitive patterns)
- `detect_quantization(path)` -> `QuantizationType` (filename + header fallback)
- `model_name_from_filename(path)` -> `String` (strips quant suffix)
- `extract_metadata(path)` -> `ModelMetadata` (full `gguf-rs-lib` parse)

### Metadata Keys

Architecture-prefixed: `{arch}.context_length`, `{arch}.embedding_length`, `{arch}.attention.head_count`, `{arch}.block_count`

General: `general.architecture`, `general.license`, `general.author`, `general.size_label`, `general.source.huggingface.repository`, `general.source.url`
