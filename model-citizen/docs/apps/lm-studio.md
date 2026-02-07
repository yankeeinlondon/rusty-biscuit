---
prompt: >-
    - Do a deep dive on the LM Studio application. List out it's feature set, describe it's architecture, describe the licensing it is offered under.
    - Then describe what ENV variables the program uses for API Keys, for Model discovery, or for anything else.
    - Describe the file locations which it expects models to be in and how any metadata for using the model is stored.
    - Describe the API surface of LM Studio
        - Does it support the OpenAI standard API?
        - How does it handle authentication?
        - Does the API allow switching the model being used/served?
    - Does LM Studio handle processing multiple prompts at the same time?
    - What kind of models does it support? Is it just GGUF?
    - If you're trying to detect whether LM Studio is running (or is installed) on the local host, how is that best done?
---

### 1. Overview: Feature Set and Architecture

**LM Studio** is a desktop application designed to run Large Language Models (LLMs) locally on consumer hardware. It provides a user-friendly graphical interface (GUI) for discovering, downloading, and running models, while also exposing a local server for API integration.

#### **Feature Set**

* **Model Hub Integration:** Built-in search and download functionality directly from Hugging Face, allowing users to browse and filter models by name, task, or popularity. See [Download an LLM](https://lmstudio.ai/docs/app/basics/download-model).
* **Local Inference:** Full offline capabilities once models are downloaded. No data is sent to the cloud.
* **Hardware Acceleration:** Support for GPU acceleration (via Vulkan, Metal, and CUDA) and CPU inference through the `llama.cpp` runtime for GGUF models. On Apple Silicon, LM Studio also supports the MLX runtime.
* **Chat Interface:** A full-featured GUI chat client with support for system prompts, temperature adjustment, top-p sampling, and context length management.
* **Local Server Mode:** A built-in server that exposes both LM Studio's native REST API and OpenAI/Anthropic-compatible endpoints so other apps can connect to local models. See [REST API](https://lmstudio.ai/docs/developer/rest) and [OpenAI compatibility](https://lmstudio.ai/docs/developer/openai-compat).
* **Dual Mode:** Users can run models in a "Chat" tab for conversation or a "Developer" tab to test prompts and API behavior.
* **MCP Client:** Install and use MCP servers directly inside LM Studio. See [Use MCP Servers](https://lmstudio.ai/docs/app/mcp).
* **Cross-Platform:** Available on Windows, macOS, and Linux (Apple Silicon and Intel).

#### **Architecture**
LM Studio provides a desktop UI backed by local inference runtimes and a local API server.

* **Inference Runtimes:** LM Studio uses `llama.cpp` to run GGUF models on macOS/Windows/Linux, and supports MLX models on Apple Silicon.
* **Server Architecture:** The local server exposes HTTP endpoints for inference and model management (native REST plus OpenAI/Anthropic-compatible endpoints).

#### **Licensing**

* **Application License:** LM Studio is **proprietary software**. It is free to download and use, but the source code is not open source.
* **Model Licenses:** The application itself does not dictate the license of the models you run. Users must adhere to the specific licenses of the models downloaded (e.g., Llama 2/3 are community licenses, Mistral is Apache 2.0, etc.).
* **Dependencies:** It bundles open-source libraries such as `llama.cpp` (MIT license). Each dependency retains its own license.

---

### 2. Environment Variables

Unlike backend-heavy frameworks (e.g., typical Dockerized AI stacks), LM Studio is a desktop-first application. Consequently, **it does not rely heavily on environment variables for configuration**. Most settings are managed via the GUI and stored in configuration files.

There are no documented, required environment variables for LM Studio itself. Authentication is configured in the app UI and clients pass an API token in the `Authorization` header when auth is enabled. See [Server Settings](https://lmstudio.ai/docs/developer/core/server/settings) and [Authentication](https://lmstudio.ai/docs/developer/core/authentication).

**Summary:** There are **no standard API key ENV variables** required for LM Studio to function. If you want to use a shell variable for convenience, it is common to set something like `LM_API_TOKEN` and pass it as `Authorization: Bearer $LM_API_TOKEN` in your client.

---

### 3. File Locations and Metadata

LM Studio uses a specific directory structure to store models, configuration, and application data.

#### **Model Locations**
LM Studio expects models under `~/.lmstudio/models` using a `publisher/model/model-file` layout. See [Import Models](https://lmstudio.ai/docs/app/advanced/import-model).

* **Note:** On Windows, `~` resolves to your user profile directory (e.g., `C:\Users\<Username>`).

**Structure:**
The files are organized hierarchically, typically mirroring the Hugging Face repository structure:
`~/.lmstudio/models/<Repository_Organization>/<Model_Name>/<Model_File>`
*Example (GGUF):* `~/.lmstudio/models/lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF/Meta-Llama-3-8B-Instruct-Q4_K_M.gguf`

#### **Metadata and Configuration**

* **Application Settings:** Stored in the `.lmstudio` application data directory (exact filenames may vary by version).
* **Model Metadata:**
    * LM Studio preserves Hugging Face-style directory structure under `~/.lmstudio/models` and reads model metadata from the model files themselves.
    * GGUF models include metadata such as tokenizer and prompt template.
    * The LM Studio model catalog uses `model.yaml` to describe models and variants across formats (GGUF, MLX, etc), and to store defaults and metadata. See [model.yaml](https://lmstudio.ai/docs/app/modelyaml).

---

### 4. API Surface

LM Studio is widely used because its Local Server feature provides a drop-in replacement for the OpenAI API.

#### **Does it support the OpenAI standard API?**
**Yes.** LM Studio implements OpenAI-compatible endpoints and works with standard OpenAI client libraries when you set the `base_url`.

* **Base URL:** `http://localhost:1234/v1` (default). See [OpenAI compatibility](https://lmstudio.ai/docs/developer/openai-compat).
* **Supported OpenAI-Compatible Endpoints:**
    * `GET /v1/models`
    * `POST /v1/responses`
    * `POST /v1/chat/completions`
    * `POST /v1/completions` (Legacy)
    * `POST /v1/embeddings`
* **Native REST API (v1):**
    * **Base URL:** `http://localhost:1234/api/v1`
    * `POST /api/v1/chat`
    * `GET /api/v1/models`
    * `POST /api/v1/models/load`
    * `POST /api/v1/models/unload`
    * `POST /api/v1/models/download`
    * `GET /api/v1/models/download/status`
    * See [REST API quickstart](https://lmstudio.ai/docs/developer/rest/quickstart).
* **Anthropic-Compatible Endpoint:**
    * `POST /v1/messages`
    * See [Anthropic compatibility](https://lmstudio.ai/docs/developer/anthropic-compat).

#### **How does it handle authentication?**
By default, **authentication is disabled**.

* When auth is disabled, requests proceed without validation.
* LM Studio 0.4.0+ supports **API Tokens** that you can enable in Server Settings. When enabled, all REST/OpenAI/Anthropic-compatible requests must include a valid `Authorization: Bearer <token>` header. See [Authentication](https://lmstudio.ai/docs/developer/core/authentication).

#### **Does the API allow switching the model being used/served?**
**Yes.**
You can switch models in two ways:

1. **Request Level:** In the API payload, you specify the `model` field.

    ```json
    {
      "model": "lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF",
      "messages": [...]
    }
    ```

    When the server receives a request for a model that is not loaded, it can load that model from disk (unloading the previous one if needed) and then process the prompt.
2. **Explicit REST Control:** The native REST API lets you load/unload models via `/api/v1/models/load` and `/api/v1/models/unload`.

---

### 5. Concurrency: Processing Multiple Prompts

**Yes, LM Studio can handle multiple prompts concurrently.**

LM Studio supports **parallel requests via continuous batching** when using the `llama.cpp` runtime (requires an updated GGUF runtime, e.g., llama.cpp v2.0.0). You can set **Max Concurrent Predictions** per model load (default is 4). With this enabled, multiple requests are processed in parallel instead of strictly queued. See [Parallel Requests](https://lmstudio.ai/docs/app/advanced/parallel-requests).

**Summary:** Concurrency is supported via continuous batching for the `llama.cpp` runtime, and you can control the degree of parallelism per model. MLX parallel requests are not yet documented as supported.

## Model Types

**The Short Answer:**

LM Studio primarily supports **GGUF** models via `llama.cpp`, and on Apple Silicon it also supports **MLX** models (which are typically distributed as `.safetensors`). It does **not** natively support PyTorch checkpoints meant for Hugging Face Transformers, nor formats like GPTQ, AWQ, or EXL2.

Here is the detailed breakdown of what it supports and why.

### 1. The Primary Format: GGUF
**GGUF** is the current standard file format for LM Studio. It is a binary file format that stores the model weights and metadata (tokenizer, prompt template, architecture parameters) in a single file.

* **Why GGUF?**
    * **Quantization:** GGUF allows models to be compressed (quantized) to lower precisions (e.g., 4-bit, 5-bit, 8-bit). A model that requires 16GB of VRAM in FP16 (raw format) might only require 4-5GB in a Q4_K_M GGUF format.
    * **CPU Offloading:** The format is structured specifically to allow partial CPU offloading (loading some layers on GPU, some on system RAM), which is crucial for running large models on consumer hardware.
* **Single File:** Unlike PyTorch models which often come in shards (multiple files like `model-00001-of-00003.safetensors`), a GGUF model is typically a single file (e.g., `llama-3-8b-instruct-q4_k_m.gguf`), making it easy to manage.

### 1b. MLX on Apple Silicon
On Apple Silicon, LM Studio supports the MLX runtime. MLX models are commonly distributed as `.safetensors` weights and are described by `model.yaml` in the LM Studio model catalog.

### 2. Supported Architectures (via GGUF or MLX)
LM Studio supports a wide range of model **architectures**, provided they are available in GGUF (for `llama.cpp`) or MLX (for Apple Silicon).

It supports virtually any model that `llama.cpp` supports, including:

* **Llama (1, 2, 3, 3.1)**
* **Mistral / Mixtral**
* **Gemma (Google)**
* **Qwen (Alibaba)**
* **Phi (Microsoft)**
* **StarCoder / SantaCoder**
* **Falcon**
* **StableLM / Zephyr**
* **Command R**

### 3. What it does NOT support
This is the most common point of confusion for users coming from other tools like Oobabooga (Text Generation WebUI).

* **No Native PyTorch / Hugging Face Transformers:** You cannot download a raw Transformers checkpoint and load it directly into LM Studio unless it is in a supported runtime format (GGUF for `llama.cpp`, or MLX for Apple Silicon).
* **No GPTQ / AWQ / EXL2:** These are alternative quantization formats popular in the Python/AutoGPTQ community. LM Studio uses the `llama.cpp` backend, which does not understand these formats. You would need to convert these models to GGUF first (though performance is usually better if you just find the native GGUF release).

### 4. Legacy Format: GGML

* **Status:** Deprecated.
* **Details:** Before GGUF, there was GGML. LM Studio historically supported GGML files (`.ggml`), but support is fading. The application and the underlying `llama.cpp` library have moved almost exclusively to GGUF. You should avoid GGML files now as they lack modern features like Flash Attention and are slower.

### Summary Workflow
If you want to use a model in LM Studio, use a GGUF build for `llama.cpp` or an MLX build on Apple Silicon.

**Incorrect for LM Studio (as-is):**

* `meta-llama/Meta-Llama-3-8B` (Contains raw `safetensors`)

**Correct for LM Studio (GGUF):**

* `lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF` (Contains `.gguf` files)
* `TheBloke/Mistral-7B-Instruct-v0.2-GGUF` (Contains `.gguf` files)

**Correct for LM Studio (MLX on Apple Silicon):**

* A model repository that provides MLX weights (typically `.safetensors`) intended for MLX runtimes.

## Detection

Detecting whether LM Studio is installed or running depends on whether you need to interact with it (API) or simply manage the application software (OS level).

Here is the breakdown of the best methods, ranked from most reliable to least reliable.

---

### 1. Detecting if Running (Best for Integration)

If you are building a script or another application that needs to use LM Studio, the most reliable method is **checking the Local Server API**. This confirms that the application is not just open, but that the server is actively listening for connections.

#### **Method A: The HTTP Health Check (Recommended)**
Since LM Studio exposes an OpenAI-compatible API, the standard way to "ping" it is to query the models endpoint.

* **Target URL:** `http://localhost:1234/v1/models` (OpenAI-compatible) or `http://localhost:1234/api/v1/models` (native REST)
* **Logic:** Attempt a connection. If you receive a JSON response listing models, LM Studio is running. If you get a "Connection Refused" error, it is not.

**Example using `curl` (Bash/Terminal):**

```bash
# Try to fetch the model list
curl -s http://localhost:1234/v1/models

# Check exit code (0 = success/running, non-zero = not running)
if [ $? -eq 0 ]; then
    echo "LM Studio is running."
else
    echo "LM Studio is not running."
fi
```

**Example using Python:**

```python
import requests

try:
    response = requests.get("http://localhost:1234/v1/models", timeout=2)
    if response.status_code == 200:
        print("LM Studio is running and active.")
except requests.exceptions.ConnectionError:
    print("LM Studio is not running.")
```

*Note: This assumes the user has not changed the default port from 1234 in their settings. If authentication is enabled, include a valid `Authorization: Bearer <token>` header. See [REST API quickstart](https://lmstudio.ai/docs/developer/rest/quickstart) and [Authentication](https://lmstudio.ai/docs/developer/core/authentication).*

---

### 2. Detecting if Running (OS Level)

If you cannot use the HTTP method (perhaps the network stack is down or you don't care about the server status), you can check for the running process.

#### **Windows**
The executable name is typically `LM Studio.exe`. You can use PowerShell or Tasklist.

**PowerShell:**

```powershell
Get-Process | Where-Object { $_.ProcessName -like "*LM Studio*" }
```

#### **macOS / Linux**
The process name is usually `LM Studio`.

**Bash:**

```bash
pgrep -f "LM Studio"
# Returns Process ID (PID) if running, nothing if not.
```

---

### 3. Detecting if Installed (File System Check)

If you need to verify if the user has the software on their machine (regardless of whether it is open), check the default installation directories.

#### **Windows**
LM Studio installs per-user by default, and the exact path can vary by installer version or user choice. If you need a file check, look for `LM Studio.exe` under `%LOCALAPPDATA%` or use the Windows Apps list as the source of truth.

**PowerShell Check:**

```powershell
if (Get-ChildItem -Path "$env:LOCALAPPDATA" -Filter "LM Studio.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1) {
    Write-Host "LM Studio is installed."
}
```

#### **macOS**
It is installed as a standard `.app` bundle.

* **Path:** `/Applications/LM Studio.app`

**Bash Check:**

```bash
if [ -d "/Applications/LM Studio.app" ]; then
    echo "LM Studio is installed."
fi
```

#### **Linux**
Since LM Studio is often distributed as an AppImage on Linux, the location varies. The most consistent indicator of an "installed" state (i.e., used at least once) is the creation of its config directory.

* **Config/Data Path:** `~/.lmstudio`

**Bash Check:**

```bash
if [ -d "$HOME/.lmstudio" ]; then
    echo "LM Studio data/config exists."
fi
```

---

### Summary: Which should you use?

| Scenario | Best Method |
| :--- | :--- |
| **I want to send a prompt to the model** | **HTTP Health Check** (`curl localhost:1234/v1/models`). This guarantees the server is ready. |
| **I want to start the app if it's closed** | **Process Check** (`pgrep` / `Get-Process`). This tells you if the UI is open. |
| **I am writing an installer** | **File Path Check** (Verify `AppData` or `/Applications` exists). |
