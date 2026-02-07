---
prompt: >-
    - Do a deep dive on the Ollama application. List out it's feature set, describe it's architecture, describe the licensing it is offered under.
    - Then describe what ENV variables the program uses for API Keys, for Model discovery, or for anything else.
    - Describe the file locations which it expects models to be in and how any metadata for using the model is stored.
    - Describe the API surface of Ollama
        - Does it support the OpenAI standard API?
        - How does it handle authentication?
        - Does the API allow switching the model being used/served?
    - Does Ollama handle processing multiple prompts at the same time?
    - What kind of models does it support? Is it just GGUF?
    - If you're trying to detect whether Ollama is running (or is installed) on the local host, how is that best done?
---

Here is a deep dive into the Ollama application, covering its architecture, feature set, configuration, file structure, API, and operational details.

---

### 1. Overview, Features, Architecture, and Licensing

#### Feature Set
Ollama is a tool designed to run Large Language Models (LLMs) locally. It abstracts away the complexity of setting up hardware acceleration, model quantization, and inference engines.

* **Local Inference:** Runs entirely on the user's machine, ensuring data privacy and no subscription fees.
* **Large Model Library:** Supports a vast ecosystem of models including Llama 3, Mistral, Gemma, Phi 3, and CodeLlama.
* **Simple CLI & REST API:** Offers a straightforward command-line interface for operations like `pull`, `run`, and `create`, as well as a robust HTTP API for integration into applications.
* **Model Customization (Modelfiles):** Allows users to create custom models using "Modelfiles" (Dockerfile-like syntax) to base models, set parameters (temperature, top_p), and include system prompts.
* **Hardware Acceleration:** Automatically utilizes GPU acceleration (Apple Silicon, NVIDIA CUDA, AMD ROCm) when available, falling back to CPU if necessary.
* **Cross-Platform:** Available on macOS, Linux, and Windows.
* **Session Context (CLI):** The CLI keeps conversation context only for the current session; persistent chat history is not stored by Ollama itself.

#### Architecture
Ollama’s architecture is designed as a **Client-Server** model.

1. **The Ollama Binary:** When you install Ollama, you are installing a single binary that acts as both the **Client** (CLI) and the **Server** (Daemon).
2. **The Server (`ollama serve`):** When the application starts, it launches a background HTTP server (usually listening on port 11434). This server handles the inference logic.
3. **Inference Engine:** Under the hood, Ollama primarily uses **llama.cpp**-based runners for GGUF models. It stores model data as content-addressed layers (blobs) and can import GGUF directly or convert safetensors into layers during model creation.
4. **Workflow:**
    * The user runs `ollama run llama3`.
    * The CLI client sends an HTTP request to the local Ollama server.
    * The server loads the model into VRAM/RAM (if not already loaded) and streams the response token-by-token back to the client.

#### Licensing

* **Ollama Application:** The core Ollama software is licensed under the **MIT License**. This is a permissive license that allows for commercial use, modification, distribution, and private use.
* **Models:** The models you run via Ollama have their own distinct licenses (e.g., Llama 3 is licensed under the Llama Community License, Gemma is open-source). Ollama itself does not change the license of the models it serves.

Sources: [Ollama docs](https://docs.ollama.com/index.md), [Ollama README](https://github.com/ollama/ollama), [Ollama License](https://github.com/ollama/ollama/blob/main/LICENSE)

---

### 2. Environment Variables

Ollama relies heavily on environment variables for configuration, particularly for server binding and hardware settings.

**Authentication:**

* **None by default:** Ollama does not have a built-in API key environment variable. For OpenAI-compatible SDKs, an `api_key` value is required by the client but ignored by Ollama.
* **Remote access:** If you expose Ollama beyond localhost, secure it with a reverse proxy (TLS + auth).

**Model Discovery & Network:**

* **`OLLAMA_HOST`:** Defines the IP address and port the Ollama server listens on. Default is `127.0.0.1:11434` (localhost). To expose it to the network, you might set `OLLAMA_HOST=0.0.0.0:11434`.
* **`OLLAMA_ORIGINS`:** A comma-separated list of allowed origins for CORS. By default, Ollama allows requests from `127.0.0.1` and `0.0.0.0`.
* **`HTTPS_PROXY`:** Routes model downloads through a proxy server when required by your network.

**Models & Storage:**

* **`OLLAMA_MODELS`:** Sets the directory where models are stored.

**Concurrency & Lifecycle:**

* **`OLLAMA_MAX_QUEUE`:** Maximum number of requests that can be queued when busy.
* **`OLLAMA_MAX_LOADED_MODELS`:** Maximum number of models that can be loaded concurrently (if memory allows).
* **`OLLAMA_NUM_PARALLEL`:** Maximum number of parallel requests per model (default is `1`).
* **`OLLAMA_KEEP_ALIVE`:** Controls how long a model stays loaded in memory after a request (e.g., `24h` or `-1` to keep it loaded indefinitely).
* **`OLLAMA_CONTEXT_LENGTH`:** Sets the default context window size for the server.

**Hardware:**

* **`OLLAMA_FLASH_ATTENTION`:** Set to `1` to enable flash attention (if supported by the hardware/model) for faster inference.
* **`OLLAMA_KV_CACHE_TYPE`:** Sets the K/V cache quantization type (e.g., `f16`, `q8_0`, `q4_0`).
* **`OLLAMA_VULKAN`:** Set to `1` to enable experimental Vulkan support.
* **`CUDA_VISIBLE_DEVICES` / `ROCR_VISIBLE_DEVICES` / `GGML_VK_VISIBLE_DEVICES`:** Limit which GPUs are used for CUDA, ROCm, or Vulkan.
* **`HSA_OVERRIDE_GFX_VERSION`:** AMD ROCm override for unsupported GPUs on Linux.

Sources: [Ollama FAQ](https://docs.ollama.com/faq.md), [Hardware support](https://docs.ollama.com/gpu.md)

---

### 3. File Locations and Metadata

Ollama stores data in specific locations depending on the operating system.

**File Locations:**

* **macOS:** `~/.ollama/models`
* **Linux:** `/usr/share/ollama/.ollama/models`
* **Windows:** `C:\Users\%username%\.ollama\models`

**Model Storage Structure:**
Inside the `models` directory, Ollama does not store files as simple loose `.gguf` files alone. It organizes them using a Content-Addressable Storage (CAS) system (blobs).

* **`manifests/`:** This directory contains the metadata files.
* **`blobs/`:** This directory contains the actual binary data (the model weights and configuration). The files are named based on their SHA-256 hash.

**Metadata:**
When you "pull" a model (e.g., `llama3`), Ollama creates a file in the `manifests/registry.ollama.ai/` directory (path structure mimics the remote registry). This file is a **JSON manifest**.

The manifest contains:

1. **Architecture:** Which model architecture it is (e.g., `llama`).
2. **Layers:** A list of "blobs" (the hash IDs pointing to files in the `blobs/` directory) that make up the model.
3. **Modelfile Digest:** The hash of the Modelfile used to create the model.
4. **Parameters:** Template details, system prompts, and license information embedded in the model configuration.

This approach ensures that if multiple models share the same underlying layers (e.g., a base model and a fine-tuned version of it), the binary data is stored only once on disk.

Sources: [Ollama FAQ](https://docs.ollama.com/faq.md)

---

### 4. API Surface

Ollama provides a RESTful API over HTTP.

**Does it support the OpenAI standard API?**
**Yes, partially.** Ollama provides OpenAI-compatible endpoints under `/v1`.

* Supported endpoints include `/v1/chat/completions`, `/v1/completions`, `/v1/embeddings`, `/v1/models`, and `/v1/responses` (plus `/v1/images/generations` experimentally).
* Many OpenAI client libraries can point to `http://localhost:11434/v1` instead of `https://api.openai.com/v1`.

Sources: [API Introduction](https://docs.ollama.com/api/introduction.md), [OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility.md)

**How does it handle authentication?**

* **Default:** None. By default, Ollama listens on `127.0.0.1` and trusts any request coming from the local machine.
* **OpenAI SDKs:** Clients require an `api_key` value, but Ollama ignores it.
* **Remote Access:** If you expose Ollama via `OLLAMA_HOST=0.0.0.0`, it does **not** automatically implement API Key authentication or TLS/SSL.
* **Security:** To secure Ollama on a network, run it behind a reverse proxy (Nginx, Caddy, Traefik) that handles authentication and HTTPS.

**Does the API allow switching the model being used/served?**
**Yes.**

* The API is stateless regarding the active model. Every API request (POST to `/api/generate` or `/api/chat`) includes a JSON body specifying the `"model": "llama3"` (or whatever model name).
* Ollama will load that model into memory if it isn't already loaded, unload a previous model if memory is tight, and process the request.
* The `/api/tags` endpoint lists all available models currently installed.

Sources: [Generate API](https://docs.ollama.com/api/generate.md), [Chat API](https://docs.ollama.com/api/chat.md), [List models](https://docs.ollama.com/api/tags.md)

---

### 5. Processing Multiple Prompts (Concurrency)

**Does Ollama handle processing multiple prompts at the same time?**

Yes. It can process requests concurrently, with behavior depending on available memory and server settings.

1. **Multiple models:** If there is sufficient RAM/VRAM, Ollama can load multiple models at once (`OLLAMA_MAX_LOADED_MODELS`).
2. **Parallel requests per model:** A single model can handle multiple parallel requests (`OLLAMA_NUM_PARALLEL`), which increases memory usage because context is multiplied by the number of parallel requests.
3. **Queueing:** When resources are exhausted, new requests are queued up to `OLLAMA_MAX_QUEUE` and processed in order.

Sources: [Ollama FAQ](https://docs.ollama.com/faq.md)

---

### 6. Model Support

**What kind of models does it support? Is it just GGUF?**

* **Format:** Ollama runs GGUF-based models and can import GGUF directly. It also supports importing **safetensors** via `ollama create`, converting them into its internal layer format.
* **Why GGUF?** GGUF is designed for `llama.cpp` and enables fast loading and memory mapping, which is crucial for large local models.
* **Supported Architectures:** While the *format* is GGUF, the *architectures* supported are vast because `llama.cpp` supports many. This includes:
    * Llama 2 / Llama 3
    * Mistral / Mixtral
    * Gemma
    * Qwen
    * Phi
    * BERT / GPT-2 (for embeddings/older models)
* **Conversion:** Ollama can import safetensors directly during `ollama create`; GGUF conversion via `llama.cpp` remains an option if you want a manual pipeline.

Sources: [Importing a model](https://docs.ollama.com/import.md), [Ollama README](https://github.com/ollama/ollama)

---

### 7. Detecting if Ollama is Running or Installed

If you are writing a script or application and need to check if Ollama is present and active:

**1. Check if it is Running (Server is up):**
The standard way is to perform a simple HTTP GET request to the Ollama host.

* **Endpoint:** `http://127.0.0.1:11434/api/tags` (or `/`)
* **Logic:**
    * If you receive an HTTP 200 OK response, Ollama is running.
    * If you get a "Connection Refused" error, the server is not running.
* **cURL example:**

    ```bash
    curl http://127.0.0.1:11434/api/tags
    ```

Sources: [List models](https://docs.ollama.com/api/tags.md)

**2. Check if it is Installed (Binary exists):**

* **macOS/Linux:** Check if the `ollama` binary exists in the user path or the default application location.

    ```bash
    command -v ollama
    # or
    ollama --version
    ```

* **Windows:** Check if the executable exists in `C:\Users\<User>\AppData\Local\Programs\Ollama\` or check the registry.
* **Programmatic Check:**
    * **Node.js/Python:** Try to spawn the `ollama --version` child process. If it exits with code 0 and outputs a version string, it is installed.

**Best Practice:** The most reliable "health check" is hitting the `/api/tags` endpoint. It confirms not only that the binary exists but that the background service is ready to accept inference requests.
