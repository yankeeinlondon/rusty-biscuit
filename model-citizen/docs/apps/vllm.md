---
prompt: >-
    - Do a deep dive on the vLLM application. List out it's feature set, describe it's architecture, describe the licensing it is offered under.
    - Then describe what ENV variables the program uses for API Keys, for Model discovery, or for anything else.
        - If the locations are driven more by configuration then ENV variables, where is the configuration file located and is a URL which describes it's format?
    - Describe the file locations which it expects models to be in and how any metadata for using the model is stored.
    - Describe the API surface of vLLM
        - Does it support the OpenAI standard API?
        - How does it handle authentication?
        - Does the API allow switching the model being used/served?
    - Does vLLM handle processing multiple prompts at the same time?
    - What kind of models does it support?
    - If you're trying to detect whether vLLM is running (or is installed) on the local host, how is that best done?

    Make sure all links use proper Markdown link formatting.
---



Based on my research, here is a comprehensive deep dive into the vLLM application:

## Overview and Feature Set

vLLM is a high-throughput, memory-efficient inference and serving engine for Large Language Models (LLMs). Originally developed at UC Berkeley's Sky Computing Lab, it has evolved into a community-driven project with contributions from both academia and industry.

### Key Features

**Performance Optimizations:**

- **PagedAttention**: Efficient management of attention key and value memory (vLLM's signature innovation)
- **Continuous Batching**: Dynamic batching of incoming requests to maximize GPU utilization
- **CUDA/HIP Graphs**: Fast model execution with optimized kernels
- **Speculative Decoding**: Accelerates generation by predicting future tokens
- **Chunked Prefill**: Splits prefill operations into chunks for better latency

**Quantization Support:**

- GPTQ, AWQ, AutoRound, INT4, INT8, and FP8 quantization formats

**Parallelism:**

- Tensor parallelism, pipeline parallelism, data parallelism, and expert parallelism for distributed inference

**Advanced Features:**

- **Prefix Caching**: Caches prompt prefixes for faster repeated queries
- **Multi-LoRA Support**: Allows runtime loading/unloading of LoRA adapters
- **Streaming Outputs**: Real-time token streaming
- **Hardware Support**: NVIDIA GPUs, AMD CPUs/GPUs, Intel CPUs/GPUs, PowerPC CPUs, Arm CPUs, TPUs, Intel Gaudi, IBM Spyre, and Huawei Ascend

**API Compatibility:**

- OpenAI-compatible API server
- Support for completions, chat completions, embeddings, audio transcription/translation, and custom endpoints

## Architecture

vLLM employs a sophisticated architecture centered around several key components:

1. **LLM Engine**: The core inference engine that manages model execution, scheduling, and memory management
2. **Scheduler**: Implements continuous batching using a dynamic, iteration-level scheduling approach
3. **Worker Processes**: Handle actual model execution, can be distributed across multiple GPUs/nodes
4. **API Server**: HTTP server providing OpenAI-compatible endpoints
5. **Memory Manager**: Implements PagedAttention for efficient KV cache management

The architecture supports both single-node and multi-node deployments, with various parallelism strategies for scaling.

## Licensing

vLLM is licensed under the **Apache License 2.0** . This is a permissive open-source license that allows for commercial use, modification, distribution, and private use. Note that while vLLM itself is Apache 2.0, individual models served through vLLM may have their own license restrictions.

## Environment Variables

vLLM uses an extensive set of environment variables, all prefixed with `VLLM_` (with some exceptions) :

### API Key and Authentication

- `VLLM_API_KEY`: Sets the API key for the vLLM API server. Can also be set via `--api-key` CLI flag

### Model Discovery and Loading

- `VLLM_USE_MODELSCOPE`: Set to `true` to load models from ModelScope instead of Hugging Face Hub
- `HF_HOME`: Controls where Hugging Face models are cached (standard Hugging Face variable)
- `VLLM_ALLOW_RUNTIME_LORA_UPDATING`: Set to `1` or `true` to allow loading/unloading LoRA adapters at runtime

### Distributed and Network Configuration

- `VLLM_HOST_IP`: IP address for vLLM internal communication (not the API server host)
- `VLLM_PORT`: Port for vLLM internal usage (not the API server port)
- `VLLM_RPC_BASE_PATH`: Path for IPC when frontend API server communicates with backend engine
- `VLLM_INSTANCE_ID`: Instance identifier for distributed environments
- `LOCAL_RANK`: Local rank in distributed settings for GPU device ID determination
- `CUDA_VISIBLE_DEVICES`: Controls GPU visibility

### Performance and Backend

- `VLLM_ATTENTION_BACKEND`: Choose attention implementation (`TORCH_SDPA`, `FLASH_ATTN`, `XFORMERS`, `ROCM_FLASH`, `FLASHINFER`, `FLASHMLA`)
- `VLLM_USE_TRITON_FLASH_ATTN`: Control Triton flash attention usage
- `VLLM_USE_V1`: Set to `1` to use the V1 code path (default as of recent versions)
- `VLLM_ENGINE_ITERATION_TIMEOUT_S`: Timeout for engine iterations (default: 60s)
- `VLLM_WORKER_MULTIPROC_METHOD`: Multiprocessing method for workers (`spawn` or `fork`)

### Storage and Cache

- `VLLM_CONFIG_ROOT`: Root directory for vLLM configuration files (defaults to `~/.config/vllm`)
- `VLLM_CACHE_ROOT`: Root directory for vLLM cache files (defaults to `~/.cache/vllm`)
- `VLLM_ASSETS_CACHE`: Cache for downloaded assets
- `VLLM_XLA_CACHE_PATH`: XLA persistent cache for TPUs

### Multimodal and Timeouts

- `VLLM_IMAGE_FETCH_TIMEOUT`: Timeout for fetching images (default: 5s)
- `VLLM_VIDEO_FETCH_TIMEOUT`: Timeout for fetching videos (default: 30s)
- `VLLM_AUDIO_FETCH_TIMEOUT`: Timeout for fetching audio (default: 10s)
- `VLLM_MM_INPUT_CACHE_GIB`: Cache size for multimodal inputs (default: 8GiB)

### Development and Debugging

- `VLLM_SERVER_DEV_MODE`: Set to `1` to enable development endpoints (dangerous for production)
- `VLLM_TRACE_FUNCTION`: Set to `1` to trace function calls
- `VLLM_TORCH_PROFILER_DIR`: Enable PyTorch profiler and set trace directory
- `VLLM_LOGGING_LEVEL`: Set logging level (default: `INFO`)
- `VLLM_LOGGING_CONFIG_PATH`: Path to custom logging configuration

### Configuration File Location

vLLM does not use a traditional configuration file by default. Instead, it relies on:

1. **Command-line arguments** passed to `vllm serve` or the Python API
2. **Environment variables** as documented above
3. **Model configuration** from Hugging Face `config.json` files

The configuration root directory is determined by `VLLM_CONFIG_ROOT` (defaulting to `~/.config/vllm` or `$XDG_CONFIG_HOME/vllm`), but this is primarily used for internal vLLM configuration rather than user-facing server configuration.

## Model Storage and Metadata

### Model Locations

vLLM expects models to be stored in the following locations:

1. **Hugging Face Cache**: By default, models are downloaded from Hugging Face Hub and cached according to the `HF_HOME` environment variable (typically `~/.cache/huggingface/hub`)

2. **Local Paths**: You can specify a local directory path directly:

   ```bash
   vllm serve /path/to/local/model
   ```

3. **Custom Cache**: Change download/cache location via:

   ```bash
   export HF_HOME=/custom/path
   ```

### Model Metadata Storage

Model metadata is stored in the standard Hugging Face format:

- **`config.json`**: Contains model architecture, hyperparameters, and configuration. The `"architectures"` field determines which vLLM model implementation is used
- **`tokenizer_config.json`**: Tokenizer settings, including chat templates
- **`generation_config.json`**: Default generation parameters (can be overridden with `--generation-config vllm`)
- **`tokenizer.json` / `tokenizer.model`**: Tokenizer vocabulary and merges

For multimodal models, additional processor configurations are stored in the model directory.

### ModelScope Alternative

To use ModelScope instead of Hugging Face:

```bash
export VLLM_USE_MODELSCOPE=True
```

## API Surface

### OpenAI Standard API Compatibility

**Yes, vLLM fully supports the OpenAI API standard** . It provides an HTTP server that implements:

- **Completions API** (`/v1/completions`)
- **Chat Completions API** (`/v1/chat/completions`)
- **Embeddings API** (`/v1/embeddings`)
- **Responses API** (`/v1/responses`)
- **Audio APIs** (`/v1/audio/transcriptions`, `/v1/audio/translations`)
- **Realtime API** (`/v1/realtime`) - WebSocket-based streaming

### Authentication

Authentication is handled via **Bearer token** in the `Authorization` header :

```bash
# Set API key via environment variable
export VLLM_API_KEY=token-abc123

# Or via CLI flag
vllm serve model --api-key token-abc123

# Client usage
curl -H "Authorization: Bearer token-abc123" http://localhost:8000/v1/chat/completions
```

**Important Security Limitations** :

- API key authentication **only applies to `/v1/*` endpoints**
- Many endpoints (`/invocations`, `/pooling`, `/classify`, `/score`, `/health`, etc.) remain **unprotected** even when API key is set
- Do not rely exclusively on `--api-key` for production security; use a reverse proxy for proper access control

### Model Switching

**No, vLLM does not support switching models via the API at runtime.** Each vLLM server instance serves **only one model at a time** . To serve multiple models, you must:

1. Start multiple vLLM instances on different ports
2. Use a load balancer or API gateway to route requests appropriately

However, vLLM does support:

- **Runtime LoRA adapter switching** (if `VLLM_ALLOW_RUNTIME_LORA_UPDATING=1` is set)
- **Multiple model versions** through the `/v1/models` endpoint (which lists the currently served model)

## Concurrent Request Handling

**Yes, vLLM handles multiple prompts simultaneously** through its **continuous batching** mechanism :

- **Dynamic Batching**: Requests are automatically grouped into batches constrained by GPU memory, `max_num_seqs`, and `max_num_batched_tokens`
- **Continuous Batching**: As soon as space is available (e.g., when a request finishes), new requests are immediately added to the next batch—this happens at each iteration rather than waiting for full batch completion
- **Scheduler Optimization**: The scheduler fills batches with pending requests, considering queue state, prompt lengths, and memory limits to maximize GPU utilization

This applies to both prefill and decode phases, allowing vLLM to achieve high throughput with low latency.

## Supported Models

vLLM supports hundreds of models across multiple categories :

### Text-Only Language Models

- **Llama family**: Llama 2, Llama 3, Llama 3.1, Llama 3.2, Llama 4
- **Mistral family**: Mistral 7B, Mixtral 8x7B/8x22B, Mistral Nemo
- **Qwen family**: Qwen2, Qwen2.5, Qwen3, Qwen-VL
- **DeepSeek**: DeepSeek-V2, DeepSeek-V3, DeepSeek-R1
- **Other architectures**: GPT-J, GPT-NeoX, Falcon, Gemma, Phi, Yi, InternLM, and many more

### Multimodal Models

- **Vision-Language**: LLaVA, CLIP, Qwen-VL, InternVL, MiniCPM-V, Pixtral, Mistral Small 3
- **Audio**: Whisper (transcription/translation)
- **Video**: Video-LLaMA, Qwen2-VL (video support)

### Pooling Models (Embeddings/Classification)

- **Embedding**: E5-Mistral, BGE models, GTE models, Jina embeddings
- **Rerankers/Cross-encoders**: BGE-reranker, mxbai-reranker
- **Classification**: Various sequence classification models
- **Reward Models**: For RLHF applications

### Transformers Backend
vLLM also supports any Transformers-compatible model via the "Transformers modeling backend," even if not natively implemented .

## Detecting vLLM Installation and Runtime

### Check if vLLM is Installed

```bash
# Check if vLLM is installed
python -c "import vllm; print(vllm.__version__)"

# Or via pip
pip show vllm

# Check for vLLM CLI
which vllm
vllm --version
```

### Check if vLLM Server is Running

**Health Check Endpoint** :

```bash
# Standard health check
curl http://localhost:8000/health

# Returns 200 OK if healthy
```

**Model List Endpoint**:

```bash
# List available models (requires API key if configured)
curl http://localhost:8000/v1/models

# Expected response includes model info:
# {
#   "object": "list",
#   "data": [
#     {
#       "id": "model-name",
#       "object": "model",
#       "owned_by": "vllm",
#       ...
#     }
#   ]
# }
```

**Version Endpoint**:

```bash
curl http://localhost:8000/version
```

**Process Detection**:

```bash
# Check for vLLM processes
ps aux | grep vllm

# Check for specific ports (default 8000)
netstat -tlnp | grep 8000
lsof -i :8000
```

**Kubernetes/Docker**:
In containerized environments, use the readiness/liveness probes:

- Readiness: `GET /health` on port 8000
- Liveness: `GET /health` on port 8000

The `/health` endpoint is the most reliable method for health checking, as it verifies both the HTTP server and the underlying engine are operational.
