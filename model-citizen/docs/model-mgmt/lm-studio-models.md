---
prompt: |-
    Do a deep dive on LM Studio's metadata and configurability for models.

    - where does LM Studio store it's information about default metadata to use with each model?
    - can you get this information from the API?
    - how do you list the models LM Studio has access to via the API?
    - how do you list the models LM Studio has access to via the file system?
    - how do you run a model in LM Studio with custom temperatures?
    - does LM studio support using GBNF? If so how is this done?
---
LM Studio acts as a sophisticated wrapper around `llama.cpp`, adding a layer of configuration management ("Presets") on top of raw GGUF metadata.

### 1. Where does LM Studio store default metadata?

LM Studio separates the *model file* (the weights) from the *configuration* (how to run it).

* **Presets (User Config):** When you save a configuration (System Prompt, Temperature, Context Length), it is saved as a **Preset**.
* **Windows:** `%USERPROFILE%\.lmstudio\config-presets\`
* **Mac/Linux:** `~/.lmstudio/config-presets/`
* *Note:* These are JSON files. You can manually back them up or edit them.


* **Model Card Metadata:** When you download a model, LM Studio caches the "Model Card" (README.md info from Hugging Face) and basic metadata (quantization level, size).
* This is typically stored alongside the model files or in the internal application database (`indexedDB` within the Electron app data), which is less accessible to users than the Presets folder.



### 2. Can you get this information from the API?

**Partially.**

* **What you CAN get:** You can get the list of models and their IDs using the standard OpenAI-compatible endpoint.
* **What you CANNOT get:** You generally cannot retrieve the *full* internal metadata (like the specific GGUF layer count, exact quantization details, or the currently loaded System Prompt) via the standard `/v1/models` endpoint. That endpoint returns a sanitized list compliant with the OpenAI spec.

To get the list of models programmatically:

```bash
curl http://localhost:1234/v1/models

```

### 3. Listing models via the API

LM Studio exposes an OpenAI-compatible endpoint. You can use `curl` or any OpenAI SDK.

**Using curl:**

```bash
curl http://localhost:1234/v1/models

```

**Using Python (openai library):**

```python
from openai import OpenAI

client = OpenAI(base_url="http://localhost:1234/v1", api_key="lm-studio")
models = client.models.list()

for model in models:
    print(model.id)

```

### 4. Listing models via the File System

If you want to bypass the API and scan the files directly, you need to look in the default model directory. LM Studio enforces a specific folder hierarchy: `Publisher > Repository > ModelFile`.

**Default Paths:**

* **Windows:** `%USERPROFILE%\.lmstudio\models\`
* **macOS / Linux:** `~/.lmstudio/models/`

**Directory Structure:**
Inside that folder, models are organized by:
`.../models/{Publisher_Name}/{Repository_Name}/{file_name}.gguf`

*Example:*
`C:\Users\You\.lmstudio\models\TheBloke\Llama-2-7B-Chat-GGUF\llama-2-7b-chat.Q4_K_M.gguf`

### 5. Running a model with custom temperatures

You can control the temperature in two ways: via the GUI (Persistent) or the API (Per-Request).

**A. Via the API (Per-Request Override)**
When using the local server, the API request's `temperature` parameter overrides the default loaded settings for that specific message.

```bash
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
    "messages": [
      { "role": "system", "content": "You are a helpful assistant." },
      { "role": "user", "content": "Hello!" }
    ],
    "temperature": 0.8  <-- CUSTOM TEMPERATURE HERE
  }'

```

**B. Via the GUI (Persistent Preset)**

1. Open the **Chat** tab.
2. On the right sidebar, scroll down to **"Parameters"**.
3. Adjust the **Temperature** slider.
4. (Optional) Click the "Save Preset" icon at the top of the sidebar to save this as a permanent configuration (e.g., "Creative Writing - Temp 0.9").

### 6. Does LM Studio support GBNF?

**Yes, but it is abstracted as "Structured Output" (JSON Schemas).**

Under the hood, LM Studio uses `llama.cpp`, which uses GBNF (Grammar-Based Normalization Form) to constrain generation. However, LM Studio's API and GUI primarily expose this via **JSON Schemas**, which it internally converts to a grammar.

**How to use it (API):**
You use the `response_format` parameter, similar to OpenAI's structured output.

```python
from openai import OpenAI
import json

client = OpenAI(base_url="http://localhost:1234/v1", api_key="lm-studio")

# Define the schema
schema = {
  "type": "json_schema",
  "json_schema": {
    "name": "get_weather",
    "strict": "true",
    "schema": {
      "type": "object",
      "properties": {
        "location": {"type": "string"},
        "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
      },
      "required": ["location", "unit"],
      "additionalProperties": False
    }
  }
}

response = client.chat.completions.create(
  model="model-identifier",
  messages=[
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the weather in Tokyo?"}
  ],
  response_format=schema
)

print(response.choices[0].message.content)

```

*Note: This forces the model to output valid JSON matching that schema. LM Studio handles the GBNF conversion automatically.*

**Can you inject raw GBNF?**
Officially, the LM Studio GUI and standard API endpoints focus on JSON Schemas. If you need raw GBNF support (e.g., to force a specific non-JSON format like Chess notation), you would typically need to use the `llama.cpp` server directly or check if LM Studio has exposed a `grammar` parameter in their specific API fork (currently, they prioritize OpenAI compatibility which relies on `response_format`).

## Example: Scanning `.lmstudio` directory with Rust

Here is a robust Rust function to scan the local `.lmstudio` directory.

This solution avoids heavy external dependencies by using the standard library (`std`) where possible, but I recommend the `dirs` crate for reliable cross-platform home directory detection.

### The Rust Implementation

Add the `dirs` dependency to your `Cargo.toml` first:

```toml
[dependencies]
dirs = "5.0"
# Optional: 'walkdir' if you prefer it over manual recursion,
# but this script uses std::fs for zero-extra-dep scanning.

```

Here is the complete generic function and executable script:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::io;

#[derive(Debug)]
pub struct ModelInfo {
    pub publisher: String,
    pub repository: String,
    pub filename: String,
    pub size_bytes: u64,
    pub path: PathBuf,
}

/// Scans the default LM Studio model directory for GGUF files.
pub fn scan_lmstudio_models() -> Result<Vec<ModelInfo>, io::Error> {
    let mut models = Vec::new();

    // 1. Determine the root path based on OS
    let home_dir = dirs::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not determine home directory")
    })?;

    // Windows: %USERPROFILE%\.lmstudio\models
    // Mac/Linux: ~/.lmstudio/models
    let base_path = home_dir.join(".lmstudio").join("models");

    if !base_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("LM Studio model directory not found at {:?}", base_path)
        ));
    }

    // 2. Walk the directory structure: Publisher -> Repository -> File
    // LM Studio structure is strict: base_path/{publisher}/{repo}/{file.gguf}

    // Level 1: Publishers (e.g., "TheBloke")
    for publisher_entry in fs::read_dir(&base_path)? {
        let publisher_entry = publisher_entry?;
        let publisher_path = publisher_entry.path();

        if publisher_path.is_dir() {
            let publisher_name = publisher_entry.file_name().to_string_lossy().to_string();

            // Level 2: Repositories (e.g., "Mistral-7B-v0.1-GGUF")
            for repo_entry in fs::read_dir(&publisher_path)? {
                let repo_entry = repo_entry?;
                let repo_path = repo_entry.path();

                if repo_path.is_dir() {
                    let repo_name = repo_entry.file_name().to_string_lossy().to_string();

                    // Level 3: The Model Files (.gguf)
                    for file_entry in fs::read_dir(&repo_path)? {
                        let file_entry = file_entry?;
                        let file_path = file_entry.path();

                        // Check for .gguf extension
                        if file_path.is_file() {
                            if let Some(ext) = file_path.extension() {
                                if ext == "gguf" {
                                    let metadata = file_entry.metadata()?;

                                    models.push(ModelInfo {
                                        publisher: publisher_name.clone(),
                                        repository: repo_name.clone(),
                                        filename: file_entry.file_name().to_string_lossy().to_string(),
                                        size_bytes: metadata.len(),
                                        path: file_path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(models)
}

fn main() {
    match scan_lmstudio_models() {
        Ok(models) => {
            println!("Found {} models:", models.len());
            for model in models {
                // Convert bytes to GB for readability
                let size_gb = model.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                println!(
                    "- [{}] {} ({:.2} GB)\n  Path: {:?}",
                    model.publisher, model.filename, size_gb, model.path
                );
            }
        }
        Err(e) => eprintln!("Error scanning models: {}", e),
    }
}

```

### How it works

1. **Path Resolution:** It uses the `dirs` crate to reliably find `~` (on Linux/Mac) or `%USERPROFILE%` (on Windows).
2. **Structural Assumption:** It relies on the fixed hierarchy LM Studio uses:
`.../models/{Publisher}/{Repository}/{ModelFile.gguf}`
3. **Filtering:** It specifically looks for files ending in `.gguf` to avoid picking up `README.md` or `.json` metadata files that often sit alongside the weights.

### Important Edge Case

If you have **customized** your model download path in LM Studio settings (moving it away from the default `.lmstudio` folder), this script will fail to find them.

To fix that, you would need to parse `~/.cache/lm-studio/user-settings.json` (location varies by OS) to find the `modelDownloadsPath` key, but that file is often compressed or part of the internal Electron state, making the default path scan the safest programmatic method.


