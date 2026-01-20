# GGUF Models

Complete guide to working with GGUF format models on Hugging Face, including Rust implementations.

## Understanding GGUF

GGUF (GPT-Generated Unified Format) is a single-file format for storing quantized models:

- **Single-file distribution** - Model weights + metadata in one file
- **Multiple quantization types** - Trade size for quality
- **Efficient inference** - Optimized for CPU/edge devices
- **Wide compatibility** - Works with llama.cpp, LM Studio, Ollama, etc.

## Finding GGUF Models

### Python Search

```python
from huggingface_hub import HfApi, ModelFilter

api = HfApi()

# Search for GGUF models
gguf_filter = ModelFilter(
    tags="gguf",
    task="text-generation"
)

models = api.list_models(
    filter=gguf_filter,
    sort="downloads",
    direction=-1,
    limit=10
)

print("Top GGUF models:")
for model in models:
    print(f"- {model.modelId}: {model.downloads:,} downloads")
```

### Rust Implementation

```rust
// Cargo.toml dependencies
// [dependencies]
// reqwest = { version = "0.12", features = ["json"] }
// tokio = { version = "1", features = ["full"] }
// serde = { version = "1.0", features = ["derive"] }

use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ModelSummary {
    #[serde(rename = "modelId")]
    model_id: String,
    downloads: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Search for GGUF models
    let models: Vec<ModelSummary> = client
        .get("https://huggingface.co/api/models")
        .query(&[
            ("filter", "gguf"),
            ("sort", "downloads"),
            ("direction", "-1"),
            ("limit", "10"),
        ])
        .send()
        .await?
        .json()
        .await?;

    println!("Top GGUF models:");
    for model in models {
        println!("- {}: {} downloads", model.model_id, model.downloads);
    }

    Ok(())
}
```

## Popular GGUF Repositories

| Repository | Description | Quantizations |
|------------|-------------|---------------|
| `TheBloke/Mistral-7B-v0.1-GGUF` | Mistral 7B quantized | Q2_K through Q8_0 |
| `TheBloke/Llama-2-7B-Chat-GGUF` | Llama 2 chat model | Multiple variants |
| `QuantFactory/Meta-Llama-3-8B-GGUF` | Llama 3 8B | Q3_K_M, Q4_K_M, Q5_K_M, etc |

## Listing GGUF Variants

### Python: List All Variants

```python
def list_gguf_files(repo_id):
    """List all GGUF files in a repository"""
    api = HfApi()

    # Get repo files
    files = api.list_repo_files(repo_id)

    # Filter GGUF files
    gguf_files = [f for f in files if f.endswith('.gguf')]

    # Get file info
    repo_info = api.repo_info(repo_id)
    file_info = {}

    for sibling in repo_info.siblings:
        if sibling.rfilename in gguf_files:
            file_info[sibling.rfilename] = {
                'size': sibling.size,
                'size_mb': sibling.size / (1024 * 1024) if sibling.size else 0
            }

    return file_info

# Usage
files = list_gguf_files("TheBloke/Llama-2-7B-Chat-GGUF")
for filename, info in files.items():
    print(f"{filename}: {info['size_mb']:.1f} MB")
```

### Rust: List Variants

```rust
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct RepoFile {
    rfilename: String,
    size: Option<u64>,
}

#[derive(Deserialize)]
struct ModelInfo {
    siblings: Vec<RepoFile>,
}

async fn list_gguf_variants(repo_id: &str) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let model_info: ModelInfo = client.get(&url).send().await?.json().await?;

    let gguf_files: Vec<(String, u64)> = model_info
        .siblings
        .into_iter()
        .filter(|f| f.rfilename.ends_with(".gguf"))
        .filter_map(|f| f.size.map(|s| (f.rfilename, s)))
        .collect();

    Ok(gguf_files)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = list_gguf_variants("TheBloke/Llama-2-7B-Chat-GGUF").await?;

    println!("GGUF variants:");
    for (name, size) in files {
        let size_gb = size as f64 / (1024.0 * 1024.0 * 1024.0);
        println!("{}: {:.2} GB", name, size_gb);
    }

    Ok(())
}
```

## Downloading GGUF Files

### Python Download

```python
from huggingface_hub import hf_hub_download

def download_gguf(repo_id, filename, local_dir="."):
    """Download specific GGUF file"""
    path = hf_hub_download(
        repo_id=repo_id,
        filename=filename,
        local_dir=local_dir,
        local_dir_use_symlinks=False  # Get actual file
    )
    print(f"Downloaded to: {path}")
    return path

# Download Q4_K_M variant (recommended balance)
model_path = download_gguf(
    "TheBloke/Llama-2-7B-Chat-GGUF",
    "llama-2-7b-chat.Q4_K_M.gguf"
)
```

### Rust Download with Progress

```rust
use std::fs::File;
use std::io::Write;
use reqwest::Client;

async fn download_gguf_with_progress(
    repo_id: &str,
    filename: &str,
    local_path: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename
    );

    let client = Client::new();
    let mut response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()).into());
    }

    // Get content length
    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse::<u64>().ok())
        .unwrap_or(0);

    let mut file = File::create(local_path)?;
    let mut downloaded = 0u64;

    // Download in chunks
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        // Progress
        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            print!("\rProgress: {:.1}%", progress);
            std::io::stdout().flush()?;
        }
    }

    println!("\nDownload complete!");
    Ok(downloaded)
}
```

## Quantization Guide

### Quantization Types

| Type | Bits | Quality | Size Reduction | Use Case |
|------|------|---------|----------------|----------|
| **Q2_K** | ~2-3 | Low | 85-90% | Testing only |
| **Q3_K_M** | ~3 | Fair | 70-75% | Memory constrained |
| **Q4_K_M** | ~4 | Good | 60-65% | **Recommended** |
| **Q5_K_M** | ~5 | Very Good | 45-50% | Quality priority |
| **Q6_K** | ~6 | Excellent | 30-35% | Near-original |
| **Q8_0** | 8 | Best | 10-15% | Maximum quality |

### Choosing Quantization

```python
def recommend_quantization(ram_gb, quality_priority=0.5):
    """Recommend quantization based on available RAM"""
    # quality_priority: 0 = size, 1 = quality

    if ram_gb < 4:
        return "Q2_K"  # Emergency only
    elif ram_gb < 8:
        return "Q3_K_M" if quality_priority < 0.3 else "Q4_K_M"
    elif ram_gb < 16:
        return "Q4_K_M" if quality_priority < 0.7 else "Q5_K_M"
    else:
        return "Q5_K_M" if quality_priority < 0.8 else "Q6_K"

# Example usage
print(f"8GB RAM: {recommend_quantization(8)}")
print(f"16GB RAM, quality focus: {recommend_quantization(16, 0.8)}")
```

## Complete Workflow Examples

### Python: Find and Download Best Model

```python
async def find_and_download_best_gguf(search_term, quantization="Q4_K_M"):
    """Find popular GGUF model and download specified quantization"""
    api = HfApi()

    # Search for models
    models = api.list_models(
        search=search_term,
        filter=ModelFilter(tags="gguf"),
        sort="downloads",
        direction=-1,
        limit=5
    )

    if not models:
        print("No models found")
        return None

    # Use most popular
    best_model = models[0]
    print(f"Selected: {best_model.modelId}")

    # Find specific quantization
    files = api.list_repo_files(best_model.modelId)
    target_file = None

    for file in files:
        if quantization in file and file.endswith(".gguf"):
            target_file = file
            break

    if target_file:
        print(f"Downloading: {target_file}")
        return hf_hub_download(
            best_model.modelId,
            target_file,
            local_dir="./models"
        )
    else:
        print(f"Quantization {quantization} not found")
        return None

# Usage
model_path = await find_and_download_best_gguf("llama", "Q4_K_M")
```

### Rust: Automated Download Script

```rust
use std::path::Path;

async fn download_if_missing(
    repo_id: &str,
    quantization: &str,
    models_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create models directory
    std::fs::create_dir_all(models_dir)?;

    // Find matching file
    let variants = list_gguf_variants(repo_id).await?;
    let target = variants
        .iter()
        .find(|(name, _)| name.contains(quantization))
        .ok_or("Quantization not found")?;

    let local_path = Path::new(models_dir).join(&target.0);

    // Check if already exists
    if local_path.exists() {
        println!("Model already downloaded: {}", local_path.display());
        return Ok(local_path.to_string_lossy().to_string());
    }

    // Download
    println!("Downloading {} ({:.2} GB)...", target.0, target.1 as f64 / 1e9);
    download_gguf_with_progress(repo_id, &target.0, local_path.to_str().unwrap()).await?;

    Ok(local_path.to_string_lossy().to_string())
}

// Usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = download_if_missing(
        "TheBloke/Llama-2-7B-Chat-GGUF",
        "Q4_K_M",
        "./models",
    ).await?;

    println!("Ready to use: {}", model_path);
    Ok(())
}
```

## Integration with Inference

### Using with llama.cpp (via subprocess)

```python
import subprocess

def run_inference_gguf(model_path, prompt, max_tokens=100):
    """Run inference using llama.cpp"""
    cmd = [
        "./llama.cpp/main",  # Path to llama.cpp binary
        "-m", model_path,
        "-p", prompt,
        "-n", str(max_tokens),
        "--temp", "0.7",
        "-t", "4"  # Threads
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout

# Example
output = run_inference_gguf(
    "./models/llama-2-7b-chat.Q4_K_M.gguf",
    "Tell me a joke about programming"
)
print(output)
```

### Rust with Candle

```rust
// Using candle for GGUF inference (simplified example)
use candle::{Device, Tensor};
use candle_transformers::models::llama::Llama;

async fn load_and_run_gguf(model_path: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    // This is a conceptual example - actual implementation
    // depends on your specific candle setup

    let device = Device::Cpu;

    // Load GGUF model
    // let model = load_gguf_model(model_path, &device)?;

    // Tokenize and generate
    // let tokens = tokenizer.encode(prompt)?;
    // let output = model.generate(&tokens, 100)?;

    Ok("Generated text here".to_string())
}
```

## Best Practices

1. **Start with Q4_K_M** - Best balance of size and quality
2. **Check file sizes** - Some Q8 models can be 10+ GB
3. **Verify downloads** - Use checksums when available
4. **Cache models** - Reuse downloads across projects
5. **Test different quantizations** - Quality needs vary by use case
6. **Monitor RAM usage** - GGUF models still need memory for inference