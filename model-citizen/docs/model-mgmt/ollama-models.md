---
prompt: |-
    Do a deep dive on Ollama's metadata and configurability for models.

    - where does Ollama store it's information about default metadata to use with each model?
    - can you get this information from the API?
    - how do you list the models Ollama has access to via the API?
    - how do you list the models Ollama has access to via the file system?
    - how do you run a model in Ollama with custom temperatures?
    - does Ollama support using GBNF? If so how is this done?

    All code examples should be done in Rust.
---
This deep dive covers Ollama's metadata architecture, API capabilities, and configuration.

### 1. Default Metadata Storage

Ollama stores model information in a specific directory structure that separates **manifests** (metadata) from **blobs** (model weights and configuration layers).

* **Location:**
* **Linux:** `/usr/share/ollama/.ollama/models` (if installed via systemd) or `~/.ollama/models`
* **macOS:** `~/.ollama/models`
* **Windows:** `C:\Users\<username>\.ollama\models`


* **Internal Structure:**
* **`manifests/`**: Contains JSON files that act as the entry point. They define the model's layers (config, weights, license, etc.).
* *Path format:* `manifests/registry.ollama.ai/library/<model_name>/<tag>`


* **`blobs/`**: A content-addressable storage (CAS) folder where files are named by their SHA256 hash (e.g., `sha256-e411...`).
* These blobs contain the actual GGUF weights, the `Modelfile` text, and the JSON configuration.





---

### 2. Getting Metadata via API

Yes, you can retrieve the full metadata (including the `Modelfile`, template, and parameter defaults) using the `/api/show` endpoint.

**Rust Example:**

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    // Request metadata for a specific model (e.g., "llama3")
    let res = client.post("http://localhost:11434/api/show")
        .json(&json!({
            "model": "llama3"
        }))
        .send()
        .await?;

    if res.status().is_success() {
        let text = res.text().await?;
        println!("Model Metadata:\n{}", text);
    } else {
        eprintln!("Failed to get metadata: {:?}", res.status());
    }

    Ok(())
}

```

---

### 3. Listing Models via API

The `/api/tags` endpoint returns the list of models currently available to the Ollama server.

**Rust Example:**

```rust
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, Debug)]
struct ModelInfo {
    name: String,
    size: u64,
    // other fields like modified_at, digest, etc.
}

#[derive(Deserialize, Debug)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let res = client.get("http://localhost:11434/api/tags")
        .send()
        .await?;

    let tags: TagsResponse = res.json().await?;

    println!("Available Models via API:");
    for model in tags.models {
        println!("- {} ({} bytes)", model.name, model.size);
    }

    Ok(())
}

```

---

### 4. Listing Models via File System

To list models manually, you must traverse the directory structure of the manifests. This is useful if the API is down or you are inspecting a dormant volume.

**Rust Example:**

```rust
use std::fs;
use std::path::Path;
use dirs::home_dir;

fn main() {
    // Determine default path based on OS (assuming macOS/Linux user structure here)
    let home = home_dir().expect("Could not find home directory");
    let base_path = home.join(".ollama/models/manifests/registry.ollama.ai/library");

    if !base_path.exists() {
        println!("Ollama models directory not found at {:?}", base_path);
        return;
    }

    println!("Models found in file system:");
    visit_dirs(&base_path, &base_path);
}

fn visit_dirs(dir: &Path, base: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, base);
                } else {
                    // Start of the file path relative to the library root
                    if let Ok(stripped) = path.strip_prefix(base) {
                        // typically: <model_name>/<tag>
                        if let Some(model_str) = stripped.to_str() {
                            // Convert path separators to standard format "model:tag"
                            // Windows uses \, so we normalize
                            let model_tag = model_str.replace(std::path::MAIN_SEPARATOR, ":");
                            println!("- {}", model_tag);
                        }
                    }
                }
            }
        }
    }
}

```

---

### 5. Running a Model with Custom Temperatures

Ollama allows you to override parameters defined in the `Modelfile` by passing an `options` object in the API request.

**Rust Example:**

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let payload = json!({
        "model": "llama3",
        "prompt": "Why is the sky blue?",
        "stream": false,
        "options": {
            "temperature": 0.8, // High creativity
            "top_k": 50,
            "top_p": 0.95,
            "seed": 42
        }
    });

    let res = client.post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?;

    let response_text = res.text().await?;
    println!("Response with custom temperature:\n{}", response_text);

    Ok(())
}

```

---

### 6. Does Ollama Support GBNF?

**Directly? No. Indirectly? Yes.**

Ollama does **not** currently expose a raw `grammar` parameter (for passing a raw GBNF string) in its public API, unlike `llama.cpp`.

However, it uses GBNF internally to power its **Structured Outputs** feature. You can pass a **JSON Schema** to the `format` parameter, and Ollama will convert that schema into a GBNF grammar under the hood to constrain the output.

If you strictly need raw GBNF (e.g., to force a specific chess notation or a non-JSON format), you cannot currently do this via the standard Ollama API. You would need to use `llama.cpp` directly or wait for Ollama to expose the `grammar` parameter.

**Rust Example (Structured Output / JSON Schema):**
This achieves the same goal as GBNF for JSON structures.

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    // Define a schema for a specific character profile
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "class": { "type": "string", "enum": ["Warrior", "Mage", "Rogue"] },
            "level": { "type": "integer" }
        },
        "required": ["name", "class", "level"]
    });

    let payload = json!({
        "model": "llama3",
        "prompt": "Generate a random RPG character profile.",
        "stream": false,
        "format": schema // Ollama converts this to GBNF internally
    });

    let res = client.post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?;

    println!("Structured Output:\n{}", res.text().await?);

    Ok(())
}

```

## Reading Blobs

This "Deep Dive" focuses on extracting the raw configuration directly from the disk. This allows you to inspect static model properties (like quantization levels, architecture families, and architecture tags) without relying on the Ollama API.

### The Ollama Storage Architecture

Ollama uses a content-addressable storage system similar to Docker.

1. **Manifest:** A JSON file that lists the "ingredients" of the model.
2. **Blobs:** The actual files (weights, config, license). They are renamed to their SHA256 hash.

The **Config Blob** is a specific JSON file referenced by the manifest that contains the model's architectural metadata.

### Rust Implementation: Reading the Config Blob

This Rust program takes a model name (e.g., `llama3`), finds its manifest, extracts the **Config Blob** hash, and reads that JSON file from the blobs directory.

**Dependencies (`Cargo.toml`):**

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dirs = "5.0"
sha2 = "0.10"
hex = "0.4"

```

**`main.rs`:**

```rust
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use dirs::home_dir;
use std::env;

#[derive(Deserialize, Debug)]
struct Manifest {
    config: ConfigRef,
    layers: Vec<LayerRef>,
}

#[derive(Deserialize, Debug)]
struct ConfigRef {
    digest: String,
    #[serde(rename = "mediaType")]
    media_type: String,
}

#[derive(Deserialize, Debug)]
struct LayerRef {
    digest: String,
    #[serde(rename = "mediaType")]
    media_type: String,
    #[serde(default)]
    size: u64,
}

fn main() {
    // 1. Get model name from args or default to "llama3"
    let args: Vec<String> = env::args().collect();
    let model_name = if args.len() > 1 { &args[1] } else { "llama3" };

    // Handle tags (default to "latest" if not provided)
    let (name, tag) = if model_name.contains(':') {
        let parts: Vec<&str> = model_name.split(':').collect();
        (parts[0], parts[1])
    } else {
        (model_name, "latest")
    };

    println!("🔍 Inspecting model: {}:{}", name, tag);

    // 2. Locate the Manifest File
    // Standard path: ~/.ollama/models/manifests/registry.ollama.ai/library/<name>/<tag>
    let home = home_dir().expect("Could not find home directory");
    let manifest_path = home
        .join(".ollama/models/manifests/registry.ollama.ai/library")
        .join(name)
        .join(tag);

    if !manifest_path.exists() {
        eprintln!("❌ Manifest not found at: {:?}", manifest_path);
        return;
    }

    // 3. Parse the Manifest
    let manifest_content = fs::read_to_string(&manifest_path).expect("Failed to read manifest");
    let manifest: Manifest = serde_json::from_str(&manifest_content).expect("Failed to parse manifest JSON");

    println!("✅ Manifest parsed.");
    println!("   Config Digest: {}", manifest.config.digest);
    println!("   Layer Count:   {}", manifest.layers.len());

    // 4. Locate and Read the Config Blob
    // The digest looks like "sha256:1234..." -> we need to convert to path "sha256-1234..."
    let config_hash = manifest.config.digest.replace(":", "-");
    let blob_path = home
        .join(".ollama/models/blobs")
        .join(&config_hash);

    if !blob_path.exists() {
        eprintln!("❌ Config blob not found at: {:?}", blob_path);
        return;
    }

    // 5. Output the Config JSON
    let config_content = fs::read_to_string(&blob_path).expect("Failed to read config blob");

    // Pretty print the JSON
    let parsed_config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    println!("\n📜 **Model Configuration (from Disk):**");
    println!("{}", serde_json::to_string_pretty(&parsed_config).unwrap());

    // Optional: List other layers (Weights, System Prompts, etc.)
    println!("\n📦 **Model Layers:**");
    for (i, layer) in manifest.layers.iter().enumerate() {
        println!("   Layer {}: {} (Size: {} bytes)", i + 1, layer.media_type, layer.size);
        if layer.media_type.contains("params") {
            println!("      -> This layer likely contains custom parameter overrides.");
        }
    }
}

```

### What you will see in the Output

When you run this, you will see the raw metadata Ollama uses to categorize the model.

* **`model_type`**: e.g., "8B" or "70B".
* **`file_type`**: The specific quantization format, e.g., `Q4_K_M`.
* **`model_families`**: e.g., `["llama"]` or `["bert"]`.
* **`architecture`**: usually `amd64` (referring to the system arch required to run the runner, not the model weights themselves).

### Critical Distinction

This **Config Blob** contains the *Ollama registry metadata*.
If you are looking for the **Internal GGUF Metadata** (like exact context window size `llama.context_length` or tensor count), that information is embedded inside the binary header of the large **Weight Blob** (the layer with media type `application/vnd.ollama.image.model`). Reading that requires a GGUF binary parser, which is more complex than reading the JSON config blob.

## Example: Parsing modelfile

This script inspects the distinct **layers** of a model (stored in the `manifest`) and reads the text-based blobs to reconstruct the "Modelfile" components (like the Prompt Template, System Message, and Parameters).

### Understanding the "Modelfile" on Disk

Ollama does **not** store the raw `Modelfile` as a single file on disk (like a Dockerfile). Instead, it parses your Modelfile at creation time and splits it into distinct **layers**:

* `application/vnd.ollama.image.template`: Stores the template string.
* `application/vnd.ollama.image.params`: Stores parameters (stop tokens, temperature) as JSON.
* `application/vnd.ollama.image.license`: Stores the license text.
* `application/vnd.ollama.image.system`: Stores the system message (if defined).

### Rust Implementation

This code finds the manifest for a model (e.g., `llama3`), iterates through its layers, identifies the text-based configuration layers, and reads them from the `blobs/` directory.

**`main.rs`**

```rust
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use dirs::home_dir;
use std::env;

#[derive(Deserialize, Debug)]
struct Manifest {
    layers: Vec<Layer>,
}

#[derive(Deserialize, Debug)]
struct Layer {
    digest: String,
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
}

// Structure for the "params" layer which is stored as JSON
#[derive(Deserialize, Serialize, Debug)]
struct ModelParams {
    #[serde(default)]
    stop: Vec<String>,
    #[serde(default)]
    temperature: f64,
    #[serde(default)]
    top_p: f64,
}

fn main() {
    // 1. Setup paths
    let args: Vec<String> = env::args().collect();
    let model_name = if args.len() > 1 { &args[1] } else { "llama3" };

    let (name, tag) = if model_name.contains(':') {
        let parts: Vec<&str> = model_name.split(':').collect();
        (parts[0], parts[1])
    } else {
        (model_name, "latest")
    };

    let home = home_dir().expect("No home directory");
    let manifest_path = home
        .join(".ollama/models/manifests/registry.ollama.ai/library")
        .join(name)
        .join(tag);

    if !manifest_path.exists() {
        eprintln!("❌ Manifest not found: {:?}", manifest_path);
        return;
    }

    // 2. Parse Manifest
    let content = fs::read_to_string(&manifest_path).unwrap();
    let manifest: Manifest = serde_json::from_str(&content).unwrap();

    println!("🔍 Reconstructing Modelfile for '{}' from layers...\n", model_name);

    // 3. Iterate Layers and Read Blobs
    for layer in manifest.layers {
        let blob_hash = layer.digest.replace(":", "-");
        let blob_path = home.join(".ollama/models/blobs").join(&blob_hash);

        if !blob_path.exists() { continue; }

        match layer.media_type.as_str() {
            // TEMPLATE LAYER
            "application/vnd.ollama.image.template" => {
                let content = fs::read_to_string(blob_path).unwrap_or_default();
                println!("--- [TEMPLATE] ---");
                println!("{}", content);
                println!("--------------------\n");
            },

            // SYSTEM PROMPT LAYER
            "application/vnd.ollama.image.system" => {
                let content = fs::read_to_string(blob_path).unwrap_or_default();
                println!("--- [SYSTEM] ---");
                println!("{}", content);
                println!("----------------\n");
            },

            // PARAMETERS LAYER (Stored as JSON)
            "application/vnd.ollama.image.params" => {
                let content = fs::read_to_string(blob_path).unwrap_or_default();
                println!("--- [PARAMETER OVERRIDES] ---");
                // Try to parse prettily, otherwise print raw
                if let Ok(params) = serde_json::from_str::<serde_json::Value>(&content) {
                    println!("{}", serde_json::to_string_pretty(&params).unwrap());
                } else {
                    println!("{}", content);
                }
                println!("-----------------------------\n");
            },

            // LICENSE LAYER
            "application/vnd.ollama.image.license" => {
               println!("found License layer ({} bytes) - skipping output...", layer.size);
            },

            // WEIGHTS LAYER
            "application/vnd.ollama.image.model" => {
                println!("found GGUF Weights layer ({} bytes)", layer.size);
            },

            _ => println!("found Other layer: {}", layer.media_type),
        }
    }
}

```

### 3. How to use this

1. **Run the tool:** `cargo run llama3`
2. **Observe:** You will see the raw Golang template string used for the prompt and any `stop` parameters the model creator set.

### 4. Why this matters?

If you are debugging why a model refuses to stop generating (run-on sentences) or has a weird personality, checking the `application/vnd.ollama.image.params` layer directly on disk is the source of truth. It confirms if the stop tokens were actually registered in the binary blob, independent of what the API might claim.
