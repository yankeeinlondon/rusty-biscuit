# Llama.cpp (llama-server)

Open-source C/C++ GGUF inference engine. The `llama-server` binary exposes OpenAI-compatible (`/v1`),
Anthropic-compatible (`/v1/messages`), and native REST endpoints. No daemon/service — run foreground,
under a process manager, or in Docker. Auth optional.

## Platforms & binaries

| OS | Support | Binary | Alt binaries | Install |
| --- | --- | --- | --- | --- |
| macOS | native | `llama-server` | `server` | `brew install llama.cpp`, GitHub tar.gz, source, conda-forge, MacPorts, Nix |
| Linux | native | `llama-server` | `server` | GitHub tar.gz, source, conda-forge, Nix, Docker `ghcr.io/ggml-org/llama.cpp:server` |
| Windows | native | `llama-server.exe` | `server.exe` | `winget install llama.cpp`, GitHub zip, source, conda-forge |

## API endpoints

Default listen: `127.0.0.1:8080`. Auth optional via `--api-key` / `--api-key-file` / `LLAMA_API_KEY`;
then send `Authorization: Bearer KEY` or `X-Api-Key: KEY`. Note: `/health` and `/models` stay public
even when auth is enabled.

| Purpose | Method | Path | auth_gated | Notes |
| --- | --- | --- | --- | --- |
| health / identity | GET | `/health` (alias `/v1/health`) | no | `{"status":"ok"}`. 503 while loading. |
| model list | GET | `/models` (alias `/v1/models`) | no | `owned_by: llamacpp` on `/v1/models` in single-model mode; router-mode `/models` returns per-model status/path metadata. No `/api/*` routes (Ollama-only; removed by PR #22165). |
| model info / build | GET | `/props` | yes | `build_info` (`b8168-...`); auth-gated when `--api-key` set. |
| slots | GET | `/slots` | yes | Per-slot processing state; `--no-slots` disables; requires key when `--api-key` set. |
| metrics | GET | `/metrics` | yes | Prometheus; gated by `--metrics` (501 otherwise). |
| OpenAI chat | POST | `/v1/chat/completions` | opt | Base URL `http://localhost:8080/v1`. Also `/v1/embeddings`, `/v1/rerank`. |
| Anthropic messages | POST | `/v1/messages`, `/v1/messages/count_tokens` | opt | Base URL `http://localhost:8080`. Since build `b7187`. |
| web UI | GET | `/` | no | Built-in SPA; `--no-webui` disables. |

## Config & env vars

Config mechanism: **mixed** — CLI flags are primary (each maps to a `LLAMA_ARG_*` env var); router
mode can additionally load model presets from an INI file via `--models-preset`.

| Variable | Effect |
| --- | --- |
| `LLAMA_ARG_HOST` | Bind address (default `127.0.0.1`; accepts `.sock` path). |
| `LLAMA_ARG_PORT` | Listen port (default `8080`). There is **no** `LLAMA_PORT`. |
| `LLAMA_API_KEY` | Optional API key(s), comma-separated; equals `--api-key`. |
| `LLAMA_ARG_MODEL` | GGUF path to load (`-m`). |
| `LLAMA_ARG_HF_REPO` / `_HF_FILE` | HuggingFace repo `<user>/<model>[:quant]` (`-hf`) / specific file. |
| `LLAMA_ARG_EMBEDDINGS` / `_RERANKING` | Enable embedding-only / reranking mode. |
| `LLAMA_ARG_N_PARALLEL` | Server slots / parallel requests (`-np`). |
| `LLAMA_OFFLINE` | Force cache, no network. |

## Model store paths

No dedicated store. `-m` loads a GGUF path directly; `-hf` downloads into the HuggingFace cache
(`~/.cache/huggingface/hub`; Windows `C:\Users\%username%\.cache\huggingface\hub`).

## Model acquisition & ID grammar

No registry namespace. Client-facing model ID is, in order: the `--alias` value; else the GGUF filename
(e.g. `model-Q4_K_M.gguf`); for `-hf` loads, the HF repo shorthand `<user>/<repo>[:quant]`.

- HuggingFace: `llama-server -hf ggml-org/gemma-3-1b-it-GGUF`
- Manual: `llama-server -m /path/to/model-Q4_K_M.gguf`

## Traps

- `--embedding` / `--embeddings` puts the server in embedding-only mode; chat/completion endpoints
  then fail (the model computes no logits).
- `--rerank` / `--reranking` forces embedding mode + rank pooling; it is not a chat server.
- `LLAMA_ARG_PORT` sets the API port. There is no `LLAMA_PORT`; vLLM's `VLLM_PORT` is unrelated.
- `--api-key` does **not** gate `/health`, `/models`, `/v1/models`, or `/` (the public allowlist),
  while `/props`, `/slots`, and `/metrics` require the key when auth is enabled.
- In single-model mode the request `model` field is **ignored** (any value accepted); it only
  routes requests in router mode.
- `-hf` downloads share the HuggingFace cache; there is no separate llama.cpp model store.
- The client-visible model ID is the `--alias` value or GGUF filename, not a registry name.

## Integration hooks

None. No built-in `launch` subcommand; start manually or via a wrapper and consume the HTTP API.
Tool calling requires `--jinja` (enabled by default); sending `tools` with `--no-jinja` errors.

## OpenCode provider block

```json
{
  "provider": {
    "llamacpp": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Llama.cpp (local)",
      "options": { "baseURL": "http://localhost:8080/v1" },
      "models": {
        "gemma-3-1b-it.Q4_K_M.gguf": { "name": "Gemma 3 1B Q4_K_M (local)" }
      }
    }
  }
}
```

Set the model key to the GGUF filename or the `--alias` value.
Claude Code: `ANTHROPIC_BASE_URL=http://localhost:8080 ANTHROPIC_AUTH_TOKEN=<LLAMA_API_KEY-if-set> claude`

## Source

`claudine/docs/research/local_runners/llamacpp.md`
