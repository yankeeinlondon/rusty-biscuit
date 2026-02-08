---
prompt: |-
    Do a deep dive on the Llama.cpp application (both CLI and new GUI).

    - List out it's feature set, describe it's architecture, describe the licensing it is offered under.
    - Then describe what ENV variables the program uses for API Keys, for Model discovery, or for anything else.
    - Describe the file locations which it expects models to be in and how any metadata for using the model is stored.
    - What API does the server expose?
        - Does it expose a OpenAI compatible API?
        - How does it handle auth?
        - Can the API be used to change the model being served/used?
    - Provide all code examples in Rust
    - Provide configuration examples of:
        - How to run a model with various parameters changed (temp, context window, etc.)
        - What are the key parameters when starting a server?
---

This deep dive covers the **llama.cpp** project, focusing on its dual nature as both a raw command-line interface (CLI) tool and a robust HTTP server for local LLM inference.

---

### 1. Feature Set, Architecture, and Licensing

#### **Feature Set**

**llama.cpp** is widely considered the industry standard for local LLM inference due to its speed and ubiquity.

* **Apple Silicon First:** Originally optimized for Apple M-series chips (Metal), it now supports nearly every hardware backend (CUDA for NVIDIA, ROCm for AMD, Vulkan, and pure CPU).
* **GGUF Format:** Uses the GGUF (GPT-Generated Unified Format) standard, which stores model weights, tokenizer data, and metadata in a single file, allowing for memory mapping (loading models instantly without reading into RAM first).
* **Quantization:** Famous for its "k-quants" (e.g., Q4_K_M), reducing model size by 50-75% with negligible precision loss.
* **Server Mode:** Includes a built-in HTTP server (`llama-server`) with OpenAI-compatible endpoints, Anthropic-compatible messages, and a bundled Web UI. See [llama-server](https://github.com/ggml-org/llama.cpp/tree/master/tools/server).
* **Grammar Sampling:** Can force the model to output valid JSON or code based on a specific schema/grammar.

Sources: [llama.cpp README](https://github.com/ggml-org/llama.cpp), [llama-server docs](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)

#### **Architecture**

The architecture is designed for **low-latency, edge-compute inference**.

1. **Core (ggml/gguf):** A tensor library written in C (no C++ standard library dependencies in the core) that handles memory allocation and tensor operations. It uses a "compute graph" approach similar to TensorFlow but significantly more lightweight.
2. **Backends:** It dynamically offloads layers. For example, if you have 8GB VRAM and a 12GB model, it can offload 20 layers to the GPU and run the remaining 10 on the CPU system RAM (hybrid inference).
3. **Inference Loop:** It uses a key-value (KV) cache to store context tokens, preventing the model from re-computing the entire prompt for every new token generated.

#### **Licensing**

* **License:** **MIT License**.
* This makes it highly permissible for both open-source and proprietary commercial applications.

---

### 2. Environment Variables & Configuration

llama.cpp is primarily configured via command-line flags. The `llama-server` binary exposes environment variables that mirror many CLI flags (for example `LLAMA_ARG_HOST`), plus API key handling. See [llama-server docs](https://github.com/ggml-org/llama.cpp/tree/master/tools/server).

#### **Runtime Environment Variables**

| Variable | Purpose |
| --- | --- |
| `LLAMA_API_KEY` | Sets the API key required for client requests when running in server mode. |
| `LLAMA_ARG_HOST` | The IP address to bind the server to (default: 127.0.0.1). |
| `LLAMA_ARG_PORT` | The port to listen on (default: 8080). |
| `LLAMA_ARG_MODEL` | Path to the GGUF model file. |
| `LLAMA_ARG_CTX_SIZE` | Sets context window size (e.g., 4096). |
| `LLAMA_ARG_N_GPU_LAYERS` | Number of layers to offload to GPU. |
| `HF_TOKEN` | Hugging Face token used when downloading gated models via `-hf`. |

#### **Build-Time Environment Variables**

If you are compiling from source, these CMake flags determine hardware support:

* `-DGGML_CUDA=ON`: Enable NVIDIA GPU support.
* `-DGGML_METAL=ON`: Enable Apple Silicon support.
* `-DGGML_HIP=ON`: Enable AMD ROCm support.

See [Build docs](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md).

---

### 3. File Locations & Metadata

#### **Model Storage**

Unlike Ollama or HuggingFace, llama.cpp **does not enforce a central model registry**. It expects you to manage your own file system.

* **Expected Location:** You pass the direct file path to the binary.
* *Example:* `./llama-server -m /home/user/models/llama-3-8b-Q4_K_M.gguf`

* **Metadata:** All metadata (context length limit, chat template, architecture, tensor names) is baked directly into the `.gguf` binary header. You do not need separate `.json` config files.

#### **Discovery**

There is no built-in "discovery" mechanism scanning folders. You must point the software to the specific file or use `-hf` to download from Hugging Face on demand (and control sources with `MODEL_ENDPOINT`).

Sources: [Obtaining and quantizing models](https://github.com/ggml-org/llama.cpp#obtaining-and-quantizing-models)

---

### 4. Server API

The `llama-server` exposes a REST API.

#### **OpenAI Compatibility**

**Yes, partially.** It provides OpenAI-compatible endpoints for chat, responses, and embeddings. Feature parity varies by release.

* `POST /v1/chat/completions`: The standard chat endpoint.
* `POST /v1/responses`: Responses API support.
* `POST /v1/embeddings`: For vector embeddings.
* `GET /v1/models`: Lists the currently loaded model.
* `POST /v1/messages`: Anthropic-compatible Messages API.

#### **Native Endpoints (Non-OpenAI)**

It also exposes raw endpoints for finer control:

* `POST /completion`: Raw text completion (no chat formatting).
* `POST /tokenize`: Converts text to integer tokens.
* `GET /props`: Returns model properties (quantization type, context size). Requires `--props` to be enabled.
* `GET /health`: Returns server status (OK/Error). `/v1/health` is also supported.

#### **Authentication**

* **Flag:** Start the server with `--api-key "sk-..."` (or set `LLAMA_API_KEY`).
* **Header:** Clients must send `Authorization: Bearer <key>`.
* Multiple keys can be provided as a comma-separated list.
* If no key is set at startup, the server is open to the public network.

#### **Changing Models**

Standard `llama-server` is designed to load **one model at a time** into memory.

* **Can you change it via API?** Not in the single-model mode; you typically restart with a different `-m` or `-hf` argument.
* **Multi-model option:** The router mode (`--models-dir`, `--models-max`, `--models-autoload`) can serve multiple models and select by `model` in requests when configured.

Sources: [llama-server docs](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)

---

### 5. Rust Code Examples

These examples demonstrate how to interact with the **llama.cpp server** from Rust.

**Dependencies (`Cargo.toml`):**

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

```

#### **Rust Client: Chat Completion**

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let api_key = "secret-key"; // If configured on server

    // The llama.cpp server URL
    let url = "http://localhost:8080/v1/chat/completions";

    let payload = json!({
        "model": "gpt-3.5-turbo", // required by the OpenAI spec; use an alias if you set one on the server
        "messages": [
            {"role": "system", "content": "You are a helpful Rust assistant."},
            {"role": "user", "content": "Write a hello world function in Rust."}
        ],
        "temperature": 0.7,
        "max_tokens": 100,
        "stream": false
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No content found");
        println!("Response: {}", content);
    } else {
        println!("Error: {:?}", response.status());
    }

    Ok(())
}

```

#### **Rust Client: Changing Parameters (Raw Completion)**

Using the native endpoint allows for specific parameters like `dynatemp` or `grammar`.

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = "http://localhost:8080/completion";

    let payload = json!({
        "prompt": "The capital of France is",
        "n_predict": 50, // Limits tokens
        "temperature": 0.1, // Low temp for facts
        "stop": ["\n"], // Stop generating at new line
        "cache_prompt": true // Use KV Cache
    });

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    println!("Completion: {}", body["content"].as_str().unwrap());

    Ok(())
}

```

---

### 6. Configuration Examples

#### **Key Startup Parameters (Server)**

This is the command to start the server. You can run this in a terminal or wrap it in a systemd service/Dockerfile.

```bash
./llama-server \
  -m models/mistral-7b-instruct.Q4_K_M.gguf \
  --port 8080 \
  --host 0.0.0.0 \
  --ctx-size 4096 \
  --n-gpu-layers 99 \
  --api-key "my-secret-key"

```

* `--n-gpu-layers 99`: Offloads all layers to GPU (if VRAM permits).
* `--host 0.0.0.0`: Exposes server to the local network (not just localhost).

#### **Runtime Parameter Tuning**

When sending a request (via JSON payload), you can tune the model behavior per request:

**Scenario A: Coding Assistant (Precise)**

* **Temperature:** `0.2` (Low randomness)
* **Top_P:** `0.9` (Focus on probable tokens)
* **Repeat Penalty:** `1.1` (Prevent loops)

**Scenario B: Creative Writing (Diverse)**

* **Temperature:** `0.8` (Higher randomness)
* **Min_P:** `0.05` (Cut off very low probability tokens)
* **Mirostat:** `2` (Advanced entropy-based sampling, specific to llama.cpp)

**JSON Payload Example for Scenario B:**

```json
{
  "messages": [{"role": "user", "content": "Write a sci-fi story."}],
  "temperature": 0.8,
  "min_p": 0.05,
  "mirostat": 2,
  "mirostat_tau": 5.0,
  "mirostat_eta": 0.1
}

```

## Dockerfile for llama.cpp

I have prepared both the Docker containerization guide and the Model Conversion workflow below.

---

### Part 1: Containerizing Llama.cpp (Dockerfile)

While `llama.cpp` offers pre-built images, building your own ensures you target your specific hardware (especially for GPU support or CPU AVX optimizations).

#### **The Dockerfile**

This multi-stage Dockerfile builds the server from source and results in a lightweight runtime image.

```dockerfile
# --- Stage 1: Builder ---
FROM ubuntu:22.04 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    curl \
    python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Clone the repository (pinned to a specific tag for stability, or use master)
RUN git clone https://github.com/ggml-org/llama.cpp .

# Create build directory
WORKDIR /app/build

# Configure and Build
# NOTE: Add -DGGML_CUDA=ON if building for NVIDIA GPU (requires nvidia-container-toolkit on host)
RUN cmake .. -DGGML_CUDA=OFF -DCMAKE_BUILD_TYPE=Release \
    && cmake --build . --config Release --target llama-server -j$(nproc)

# --- Stage 2: Runtime ---
FROM ubuntu:22.04 as runtime

# Install runtime dependencies (OpenMP, etc)
RUN apt-get update && apt-get install -y \
    libgomp1 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/build/bin/llama-server /app/llama-server

# Create a directory for models to be mounted
RUN mkdir /models

# Expose the API port
EXPOSE 8080

# Environment variables with defaults
ENV LLAMA_ARG_HOST=0.0.0.0
ENV LLAMA_ARG_PORT=8080
ENV LLAMA_ARG_MODEL=/models/model.gguf

# Entrypoint runs the server using the env vars
CMD ["/bin/sh", "-c", "/app/llama-server -m $LLAMA_ARG_MODEL --host $LLAMA_ARG_HOST --port $LLAMA_ARG_PORT"]

```

#### **How to Run It**

1. **Build the image:**

```bash
docker build -t my-llama-server .

```


1. **Run the container:**
You must mount a volume containing your `.gguf` model.

```bash
docker run -d \
  -p 8080:8080 \
  -v /path/to/your/local/models:/models \
  -e LLAMA_ARG_MODEL=/models/mistral-7b.gguf \
  my-llama-server

```



---

### Part 2: Converting HuggingFace Models to GGUF

If you want to run a fresh model from HuggingFace (HF) that hasn't been quantized by the community yet, you need to convert it.

**Prerequisites:**

* Python 3.9+
* The `llama.cpp` source code cloned locally.

#### **Step 1: Setup Python Environment**

Navigate to the root of the `llama.cpp` repository and install the Python requirements.

```bash
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp
pip install -r requirements.txt

```

#### **Step 2: Download the Raw Model**

You need the original model (usually `.safetensors` format). You can use the `huggingface-cli` to download it efficiently.

```bash
pip install huggingface_hub
huggingface-cli download meta-llama/Llama-3.2-1B-Instruct --local-dir models/raw-model

```

#### **Step 3: Convert to GGUF (FP16)**

First, convert the raw weights to the GGUF format. This usually results in a 16-bit floating point file (high precision, large size).

```bash
# Usage: python convert_hf_to_gguf.py [dir-path-to-raw-model] --outfile [output-path]
python convert_hf_to_gguf.py models/raw-model \
  --outfile models/llama-3.2-1b-f16.gguf \
  --outtype f16

```

#### **Step 4: Quantize the Model (Optional but Recommended)**

The F16 model is large. To make it efficient (the "magic" of llama.cpp), you use the built-in `llama-quantize` binary tool to compress it to 4-bit, 5-bit, or 8-bit.

*Note: You must compile llama.cpp (run `make` in the root folder) to get this tool.*

```bash
# Usage: ./llama-quantize [source-gguf] [destination-gguf] [quantization-method]

# Example: Compressing to Q4_K_M (Balanced quality/size)
./llama-quantize models/llama-3.2-1b-f16.gguf models/llama-3.2-1b-Q4_K_M.gguf Q4_K_M

```

**Common Quantization Types:**

* **Q4_K_M:** Recommended. Good balance of perplexity (smarts) and size.
* **Q5_K_M:** Higher accuracy, slightly slower/larger.
* **Q8_0:** Almost indistinguishable from F16, but half the size.

## Using Grammar Sampling

This section covers **Grammar Sampling**, one of `llama.cpp`'s most powerful features. Unlike "prompt engineering" where you *ask* the model to output JSON and hope it complies, Grammar Sampling **mathematically constrains** the token selection process.

It effectively "turns off" any token that would violate your defined grammar. If you define a grammar for a JSON integer, the model physically cannot output a letter "A" because that token's probability is forced to zero.

### 1. The GBNF Format

`llama.cpp` uses **GBNF** (GGML Backus-Naur Form). It is a formal syntax for defining valid output structures.

**Key Concepts:**

* **`root`**: The starting point of your grammar.
* **Terminals**: Literal characters (e.g., `"{"`, `[0-9]`).
* **Non-terminals**: Reusable rules (e.g., `boolean`, `integer`).

**Example GBNF for a simple "Character" JSON:**

```gbnf
root   ::= "{" ws "\"name\":" ws string "," ws "\"age\":" ws integer "}"
string ::= "\"" ([^"]*) "\""
integer::= [0-9]+
ws     ::= [ \t\n]*

```

---

### 2. Rust Code: Enforcing JSON Output

This Rust example demonstrates how to send a raw GBNF grammar to the server. This guarantees the output is **100% valid JSON** matching your schema, even with smaller, dumber models.

**Dependencies (`Cargo.toml`):**

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1"

```

**The Code (`main.rs`):**

```rust
use reqwest::Client;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = "http://localhost:8080/completion"; // Use the native completion endpoint for raw grammars

    // 1. Define the GBNF Grammar
    // This strictly enforces: { "character": "String", "power_level": Int, "is_hero": Bool }
    let grammar = r#"
        root        ::= "{" space "\"character\":" space string "," space "\"power_level\":" space integer "," space "\"is_hero\":" space boolean "}"
        space       ::= [ \t\n]*
        boolean     ::= "true" | "false"
        integer     ::= [0-9]+
        string      ::= "\"" [^"]* "\""
    "#;

    // 2. Construct the Payload
    let payload = json!({
        "prompt": "Generate a profile for a sci-fi protagonist:",
        "n_predict": 128,
        "temperature": 0.5,
        "grammar": grammar // <--- The magic happens here
    });

    println!("Sending request with Grammar Constraints...");

    // 3. Send Request
    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?;

    // 4. Parse Result
    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        let text = body["content"].as_str().unwrap_or("");

        println!("\n--- Raw Model Output (Guaranteed JSON) ---");
        println!("{}", text);

        // Prove it parses as JSON
        let parsed: serde_json::Value = serde_json::from_str(text)?;
        println!("\n--- Parsed Rust Struct ---");
        println!("Name: {}", parsed["character"]);
        println!("Power: {}", parsed["power_level"]);
    } else {
        eprintln!("Error: {:?}", response.status());
    }

    Ok(())
}

```

### 3. How to Generate GBNF Automatically

Writing GBNF by hand is tedious for complex objects. `llama.cpp` provides a TypeScript script to convert existing TypeScript interfaces or JSON Schemas into GBNF.

If you have the repo cloned (as discussed in the previous step), you can use the pre-made Python converter:

```bash
# Assuming you are in the llama.cpp root
python3 examples/json_schema_to_grammar.py schema.json > my_grammar.gbnf

```

You can then read this file in Rust and pass it into the `grammar` field dynamically.


## Who uses GBNF?

Yes, but it is primarily found in the **llama.cpp ecosystem** and tools that wrap it.

While GBNF is not an industry-wide standard like JSON or Regex, its efficiency has led to adoption in several high-performance inference engines. Outside of llama.cpp, most other engines (like vLLM or HuggingFace TGI) historically used Python-based libraries (like `Outlines` or `Guidance`) to achieve the same result, but they are increasingly adding support for GBNF or compatible lower-level grammars.

### 1. Native GBNF Support

These tools explicitly use the `.gbnf` string format because they are either built on top of `llama.cpp` or have ported its grammar engine.

* **Fireworks AI:** A popular commercial inference provider that explicitly supports GBNF for its "Grammar Mode" to force structured output.
* **LM-Kit.NET:** A .NET inference engine that uses GBNF for its grammar sampling features.
* **Jan.ai / LM Studio:** Local GUI desktop applications that run `llama.cpp` under the hood expose GBNF features (often hidden in advanced settings).
* **Ollama:** Since Ollama wraps `llama.cpp`, it supports GBNF, though it often abstracts it away in favor of its own "Modelfile" format or generic JSON mode.

### 2. The Major Alternative: "Outlines" & "Guidance"

If you look at the Python-heavy side of the ecosystem (NVIDIA, HuggingFace), they typically use a library called **Outlines** instead of GBNF.

* **vLLM:** The massive industry-standard inference engine (used by many enterprise APIs). It historically used *Outlines* (regex/FSM based).
* *Update:* vLLM recently integrated **XGrammar**, which *does* support GBNF to speed up structured output, effectively bridging the gap.


* **HuggingFace TGI (Text Generation Inference):** Uses `outlines` to enforce JSON schemas. It does not natively parse GBNF strings but achieves the exact same mathematical constraint.

### 3. Comparison of Approaches

| Feature | **GBNF (llama.cpp)** | **Outlines / Guidance (Python Ecosystem)** |
| --- | --- | --- |
| **Where it runs** | **C++ Level** (Deep inside the inference loop) | **Python Level** (Often acts as a wrapper/logit processor) |
| **Performance** | Extremely fast, negligible overhead. | Can introduce latency (CPU overhead) on very complex schemas. |
| **Syntax** | Custom variant of Backus-Naur Form. | Standard Regex or Pydantic/JSON Schema definitions. |
| **Portability** | Harder to write manually, but highly portable binary strings. | Easier to write (Python code), but tied to Python runtime. |

### Summary

If you are building in **Rust**, **C++**, or **Go**, GBNF (via llama.cpp) is likely your best and most performant option. If you are building strictly in **Python** with heavy GPU clusters (vLLM), you will likely encounter *Outlines* or *XGrammar* instead, though the lines are blurring.

## Some Rust Examples of GBNF

Here are three practical use cases for **llama.cpp** with **Rust** and **GBNF**.

These examples assume you are running the `llama-server` on port 8080. We will use the `reqwest` crate (as established in the previous step) to send these requests. The "magic" is entirely in the `grammar` string passed in the payload.

### Use Case 1: The Strict Sentiment Analyzer (Enum Enforcement)

**The Problem:** You want an LLM to classify customer reviews.
**The Risk:** A normal LLM might reply with "I think this is positive" or "It's mostly good." You want *only* specific data labels.
**The Solution:** A grammar that forces the output to be exactly one of three words.

**The GBNF Grammar:**

```gbnf
root ::= "Positive" | "Negative" | "Neutral"

```

**Rust Implementation:**

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // We restrict the model so it physically CANNOT output anything else.
    let grammar = r#"root ::= "Positive" | "Negative" | "Neutral""#;

    let reviews = vec![
        "The product arrived broken and late.",
        "It does exactly what it says on the box.",
    ];

    for review in reviews {
        let payload = json!({
            "prompt": format!("Classify the sentiment of this review: '{}'. Sentiment:", review),
            "grammar": grammar,
            "n_predict": 10, // We only need 1 token
            "temperature": 0.0 // Deterministic
        });

        let res: serde_json::Value = client.post("http://localhost:8080/completion")
            .json(&payload).send().await?.json().await?;

        println!("Review: '{}' -> Verdict: {}", review, res["content"].as_str().unwrap());
    }

    Ok(())
}

```

---

### Use Case 2: The "Safe" SQL Generator

**The Problem:** You want to let users ask questions about your data ("Who are the active users?"), but you are terrified the LLM might hallucinate a `DROP TABLE` or `DELETE` command.
**The Solution:** A grammar that supports *only* `SELECT` statements on specific table names.

**The GBNF Grammar:**

```gbnf
root         ::= "SELECT " column_list " FROM " table_name constraint
column_list  ::= "*" | "id" | "name" | "email"
table_name   ::= "users" | "orders" | "products"
constraint   ::= "" | " WHERE " condition
condition    ::= "active = true" | "id > 100"

```

**Rust Implementation:**

```rust
// ... (imports and client setup same as above)

let grammar = r#"
    root         ::= "SELECT " column_list " FROM " table_name constraint
    column_list  ::= "*" | "id" | "name" | "email"
    table_name   ::= "users" | "orders" | "products"
    constraint   ::= "" | " WHERE " condition
    condition    ::= "active = true" | "id > 100"
"#;

let user_query = "Show me all the users who are active.";

let payload = json!({
    "prompt": format!("Translate to SQL: {}", user_query),
    "grammar": grammar,
    "n_predict": 64,
});

// The model effectively searches for the "path" through the grammar
// that best matches the semantic meaning of the prompt.
// It CANNOT generate "DROP TABLE users" because "DROP" is not in the grammar.

```

---

### Use Case 3: Structuring Unstructured Data (Resume Parsing)

**The Problem:** You have a blob of text (a resume or bio) and need to extract specific fields into a Rust struct.
**The Solution:** A grammar that defines the JSON structure for a "Person".

**The GBNF Grammar:**
This looks complex, but it essentially says: "Output a JSON object with keys `name` (string), `years_exp` (int), and `skills` (array of strings)."

```gbnf
root        ::= "{" ws "\"name\":" ws string "," ws "\"years_exp\":" ws integer "," ws "\"skills\":" ws string_list "}"
string_list ::= "[" ws (string ("," ws string)*)? "]"
string      ::= "\"" [^"]* "\""
integer     ::= [0-9]+
ws          ::= [ \t\n]*

```

**Rust Implementation:**

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Candidate {
    name: String,
    years_exp: u32,
    skills: Vec<String>,
}

// ... inside main ...

let resume_text = "John Doe is a senior engineer. He has been coding for 12 years, mostly in Rust, C++, and Python.";

let grammar = r#"
    root        ::= "{" ws "\"name\":" ws string "," ws "\"years_exp\":" ws integer "," ws "\"skills\":" ws string_list "}"
    string_list ::= "[" ws (string ("," ws string)*)? "]"
    string      ::= "\"" [^"]* "\""
    integer     ::= [0-9]+
    ws          ::= [ \t\n]*
"#;

let payload = json!({
    "prompt": format!("Extract candidate info from this text:\n{}\nJSON:", resume_text),
    "grammar": grammar,
    "n_predict": 128
});

let res: serde_json::Value = client.post("http://localhost:8080/completion")
    .json(&payload).send().await?.json().await?;

let json_str = res["content"].as_str().unwrap();

// Because the output is guaranteed to be valid JSON by the grammar,
// we can unwrap safely or handle errors minimally.
let candidate: Candidate = serde_json::from_str(json_str)?;

println!("Parsed Candidate: {:?}", candidate);
// Output: Candidate { name: "John Doe", years_exp: 12, skills: ["Rust", "C++", "Python"] }

```

### Why this matters for Rust developers

In Python, if an LLM returns bad JSON, you wrap it in a `try/except` block and retry. In Rust, we prefer **compile-time guarantees**.

GBNF acts like a **runtime compiler for the LLM**. It ensures that the "data types" returned by the probabilistic model match the strict types expected by your Rust `structs`.
