# vLLM

Open-source high-throughput serving engine (PagedAttention). Single-model server exposing
OpenAI-compatible (`/v1`), Anthropic-compatible (`/v1/messages`), pooling, and speech-to-text
endpoints. FastAPI/OpenAPI. Primarily Linux. **Binds `0.0.0.0` by default.**

## Platforms & binaries

| OS | Support | Binary | Alt binaries | Install |
| --- | --- | --- | --- | --- |
| Linux | native | `vllm` | `python`, `uv` | `pip install vllm`, `uv pip install vllm`, Docker `vllm/vllm-openai` |
| macOS | separate_project | `vllm` | `vllm-metal` | Experimental Apple Silicon CPU support is build-from-source only (no wheels); GPU is the separate vLLM-Metal project (MLX; mlx-community models) |
| Windows | wsl | `vllm` | — | WSL2 with Linux install steps (no native Windows) |

## API endpoints

Default listen: `0.0.0.0:8000`. Auth optional via `--api-key` / `VLLM_API_KEY`; when set, **every**
endpoint (including `/health` and `/version`) requires `Authorization: Bearer <key>`.

| Purpose | Method | Path | auth_gated | Notes |
| --- | --- | --- | --- | --- |
| version / identity | GET | `/version` | when auth on | `{"version":"..."}`. Strong identity marker. |
| health | GET | `/health` | when auth on | Empty HTTP 200 body when healthy; 503 when the engine is dead. |
| model list | GET | `/v1/models` | when auth on | Single model (or `--served-model-name` aliases). |
| metrics | GET | `/metrics` | when auth on | Prometheus (GPU, scheduler, request). |
| tokenizer info | GET | `/tokenizer_info` | when auth on | Gated by `--enable-tokenizer-info-endpoint`. |
| LoRA load / unload | POST | `/v1/load_lora_adapter`, `/v1/unload_lora_adapter` | when auth on | Gated by `--enable-lora` + `VLLM_ALLOW_RUNTIME_LORA_UPDATING=True`. Local dev only. |
| OpenAI chat | POST | `/v1/chat/completions` | opt | Base URL `http://localhost:8000/v1`. Also `/v1/embeddings`, `/v1/audio/*`, `/rerank`. |
| Anthropic messages | POST | `/v1/messages`, `/v1/messages/count_tokens` | opt | Base URL `http://localhost:8000`. `/v1/messages` since v0.11.1; `count_tokens` since v0.17.0 (PR #35588). |
| docs | GET | `/docs` | when auth on | Swagger UI; needs internet unless `--enable-offline-docs`. |

## Config & env vars

Config mechanism: **mixed** (CLI flags primary; subset persistable in YAML via `--config serve_args.yaml`).

| Flag / var | Effect |
| --- | --- |
| `--host` / `--port` | HTTP bind host / port (default host binds all interfaces). |
| `--api-key` / `VLLM_API_KEY` | Required API key(s). |
| `--model` / `--served-model-name` | Model to serve / API-facing name(s). |
| `--enable-auto-tool-choice` + `--tool-call-parser` | Enable tool calling (off by default). |
| `VLLM_HOST_IP` | Internal distributed IP — **NOT** the API bind address. |
| `VLLM_PORT` | Internal distributed port — **NOT** the API server port (common trap). |
| `HF_HOME` / `VLLM_USE_MODELSCOPE` | HuggingFace cache dir / use ModelScope instead. |
| `VLLM_CACHE_ROOT` / `VLLM_CONFIG_ROOT` | Runtime cache (`~/.cache/vllm`) / config root (`~/.config/vllm`). |

## Model store paths

`~/.cache/huggingface/hub` (relocate via `HF_HOME`/`HF_HUB_CACHE`); `~/.cache/vllm` runtime cache;
`--download-dir` per-invocation override.

## Model acquisition & ID grammar

HuggingFace model IDs, local paths, or ModelScope IDs (`VLLM_USE_MODELSCOPE=true`). Formats:
Safetensors, PyTorch bin, GGUF. API-facing name is `--model` unless `--served-model-name` overrides.
Examples: `Qwen/Qwen2.5-1.5B-Instruct`, `meta-llama/Llama-3.1-8B-Instruct`, `/path/to/local/model`.

- HuggingFace: `vllm serve Qwen/Qwen2.5-1.5B-Instruct`
- Manual: `vllm serve /path/to/local/model --served-model-name my-model`

## Traps

- `VLLM_PORT` and `VLLM_HOST_IP` configure internal distributed communication, **not** the API server.
  Use `--host` and `--port`.
- The default `--host` binds **all interfaces** (`0.0.0.0`), unlike runners defaulting to `127.0.0.1`.
- One vLLM process serves **one model**; run multiple processes on different ports for multi-model.
- Models without a chat template in `tokenizer_config.json` require `--chat-template` or chat fails.
- `--api-key` (or `VLLM_API_KEY`) gates **all** endpoints, including `/health` and `/version`.
- Tool calling needs explicit `--enable-auto-tool-choice`, `--tool-call-parser`, and often
  `--chat-template`; it does not work out of the box.
- Offline `/docs` requires `--enable-offline-docs`; otherwise Swagger tries to fetch CDN assets.

## Integration hooks

None. vLLM has no runner-native agentic-CLI launch command; start `vllm serve` and point the agent at it.

## OpenCode provider block

```json
{
  "provider": {
    "vllm": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "vLLM (local)",
      "options": {
        "baseURL": "http://localhost:8000/v1",
        "apiKey": "EMPTY"
      },
      "models": {
        "Qwen/Qwen2.5-1.5B-Instruct": { "name": "Qwen2.5 1.5B Instruct (local vLLM)" }
      }
    }
  }
}
```

Claude Code: `ANTHROPIC_BASE_URL=http://localhost:8000 ANTHROPIC_AUTH_TOKEN=EMPTY claude --model Qwen/Qwen2.5-1.5B-Instruct`

## Source

`claudine/docs/research/local_runners/vllm.md`
