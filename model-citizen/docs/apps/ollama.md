---
prompt:
model: glm-4.7
---

### 1. Overview
LM Studio is a desktop application that allows users to run Large Language Models (LLMs) locally on their computer. It is designed to make the complexities of downloading models, managing hardware acceleration (GPU/CPU), and running an inference server accessible to users without command-line expertise.

### 2. Feature Set

LM Studio offers a robust set of features catering to both casual users and developers:

* **Local Model Inference:** Runs models 100% offline on the user's hardware, ensuring privacy and data sovereignty.
* **Model Hub Integration:** A built-in search interface to browse and download models directly from Hugging Face. It filters for supported formats (like GGUF).
* **Hardware Acceleration:**
    * **Apple Silicon:** optimized for Metal (MPS) on M1/M2/M3 chips.
    * **NVIDIA:** Supports CUDA acceleration.
    * **AMD:** Supports ROCm acceleration on Linux/Windows.
    * **CPU Fallback:** Uses CPU inference if a GPU is unavailable.
* **Dual Interface:**
    * **Chat UI:** A user-friendly interface to chat with models, save conversation histories, and adjust system prompts.
    * **Developer Server:** A built-in local server that mimics the OpenAI API format (`/v1/chat/completions`, `/v1/models`). This allows users to swap the `base_url` in existing applications (like LangChain, LlamaIndex, or custom scripts) to point to `http://localhost:1234`.
* **GGUF Format Support:** Primarily utilizes the GGUF (GPT-Generated Unified Format) file format, which is the current standard for models running via `llama.cpp`.
* **Configuration Management:**
    * Adjustable context window sizes (context length).
    * Temperature, Top-P, and repetition penalty sliders.
    * GPU offloading controls (selecting how many layers to load onto VRAM vs. System RAM).
* **Prompt Templates:** Automatically handles prompt templating for different model families (e.g., Llama 3 uses `<|begin_of_text|>`, while Mistral uses `[INST]`), removing the need for manual formatting.

### 3. Architecture

LM Studio is essentially a graphical frontend wrapped around a powerful inference backend.

* **Core Technology (The Backend):**
    * The inference engine is built on top of **`llama.cpp`**. This is a C++ port of Meta's Llama model, optimized for inference on consumer-grade hardware.
    * LM Studio compiles `llama.cpp` bindings to communicate efficiently with the host OS and hardware drivers (Metal, CUDA, etc.).

* **User Interface (The Frontend):**
    * The desktop application is built using **Electron** (or similar web-view technologies).
    * The UI is likely built with **React** or a modern JavaScript framework.
    * The Frontend communicates with the backend via Inter-Process Communication (IPC) or local HTTP requests.

* **The "Local Server" Architecture:**
    * When the "Server Mode" is active, the application spawns a lightweight web server (likely Node.js-based or embedded in the backend) listening on a specific port (default 1234).
    * This server parses incoming JSON requests compatible with the OpenAI API schema, translates them into internal `llama.cpp` commands, executes the inference, and streams the response back via Server-Sent Events (SSE).

### 4. Licensing

The licensing of LM Studio is dual-layered:

1. **Application License (Proprietary):**
    * LM Studio itself is **proprietary software**. It is not open source. It is free to download and use for personal and commercial purposes (freeware), but the source code is not publicly available for modification or redistribution by the public.

2. **Model Licenses:**
    * The models you download via LM Studio retain their original licenses. LM Studio acts as a downloader and runner.
    * Common licenses include:
        * **Apache 2.0:** Permissive, allows commercial use (e.g., Llama 3).
        * **MIT:** Highly permissive.
        * **Llama Community License:** Specific restrictions on usage if user count exceeds a certain threshold (applicable to older Llama 2 versions).
        * **Gemma License:** Google's specific usage terms.

3. **Dependencies:**
    * Since it relies on `llama.cpp`, the backend components inherit the MIT license of that project, but the wrapping application remains proprietary.

### 5. Environment Variables

LM Studio operates primarily via its GUI, but it relies on standard system environment variables for connectivity and can be configured (specifically the server component) using variables.

#### A. Connectivity / Discovery
These are standard Node.js/Electron variables inherited by the application:

* `HTTP_PROXY` / `HTTPS_PROXY`: If set, LM Studio will use these proxies to download models from Hugging Face.
* `NO_PROXY`: Bypass proxy settings for specific hosts.

#### B. Server & API Configuration
When running the local server (useful for headless operations or scripting), the application looks for or respects the following configurations (often set in the settings UI, but applicable to the runtime environment):

* `LM_STUDIO_API_KEY`: While the local server defaults to open, you can enforce security.
* `PORT`: Defaults to `1234`. If this environment variable is set in the shell running the application (or conflicting with the config), it may alter the binding port.
* `HOST`: Defaults to `0.0.0.0` (accepting connections from anywhere) or `127.0.0.1` (local only).

#### C. Hardware / Backend

* `CUDA_VISIBLE_DEVICES`: Used on Linux/Windows with NVIDIA cards to restrict which GPU the application sees.
* `HIP_VISIBLE_DEVICES`: Used for AMD ROCm configurations.

### 6. File Locations and Metadata

LM Studio manages a specific directory structure to store models and the database of user interactions.

#### A. Base Directory (User Data)
The application stores all data (models, settings, chat history) in a single root folder depending on the OS:

* **Windows:** `C:\Users\[Username]\AppData\Roaming\LM-Studio`
* **macOS:** `~/Library/Application Support/LM Studio`
* **Linux:** `~/.config/LM-Studio` (Sometimes `~/.local/share/LM-Studio` depending on the specific build/flavor)

#### B. Model Storage Location
Inside the base directory, models are stored in:
`.../LM-Studio/models/`

**Structure:**
LM Studio organizes models by their Hugging Face repository ID to prevent naming conflicts.

* **Path Format:** `models/[AuthorName]/[ModelName]/[Filename].gguf`
* **Example:** `models/berkeley-nest/Starling-LM-7B-alpha/starling-lm-7b-alpha.Q5_K_M.gguf`

#### C. Metadata and Database
LM Studio does not rely solely on file names for metadata. It uses internal JSON files to track model parameters, chat history, and downloaded states.

* **`models.json` (or similar DB):** Located in the root config directory. This file maps the downloaded `.gguf` files to the metadata displayed in the UI (such as description, license type, and RAM requirements).
* **`library.db` / `chats.db`:** Recent versions of LM Studio utilize SQLite databases (`.db` files) to store chat history and "My Models" library states. This allows for faster searching and tagging of conversations compared to raw text files.
* **Settings:** `settings.json` usually contains your GPU offloading preferences, server port configuration, and theme settings.
