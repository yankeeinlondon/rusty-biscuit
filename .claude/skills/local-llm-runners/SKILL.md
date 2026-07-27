---
name: local-runners
description: Detecting, configuring, and wiring local model runners into agentic CLIs. Use when working with Ollama, LM Studio, oMLX, llama.cpp (llama-server), or vLLM; probing which local runners are installed or running; hitting OpenAI-compatible local endpoints (/v1) or Anthropic-compatible /v1/messages; serving local models; or connecting a local model to OpenCode or Claude Code via base URL, ANTHROPIC_BASE_URL/ANTHROPIC_AUTH_TOKEN, or runner-native launch hooks.
last_updated: 2026-07-02
hash: 36cdc2cd15df026f-d34ddf8923ebe8ed
---

# Local Model Runners

Five local LLM runners covered here — **Ollama, LM Studio, oMLX, llama.cpp (`llama-server`),
and vLLM** — each run as an HTTP server. All five now expose **both** an OpenAI-compatible
API (paths under `/v1`, so the base URL *includes* `/v1`) **and** an Anthropic Messages API
(`POST /v1/messages`, so the base URL *excludes* `/v1` because Anthropic SDKs append it). This
dual surface lets OpenCode point at the OpenAI base URL and Claude Code point at the Anthropic
base URL against the same running server.

## Cross-runner comparison

| Runner | Binary | Port | Bind | OpenAI base URL | Anthropic base URL | Identity probe | Auth | Config |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Ollama | `ollama` | 11434 | 127.0.0.1 | `http://localhost:11434/v1` | `http://localhost:11434` | `GET /` → `Ollama is running` | none | env vars |
| LM Studio | `lms` | 1234 | 127.0.0.1 | `http://localhost:1234/v1` | `http://localhost:1234` | `GET /v1/models` → `owned_by: organization_owner` | optional key | JSON files + CLI + GUI |
| oMLX | `omlx` | 8000 | 127.0.0.1 | `http://localhost:8000/v1` | `http://localhost:8000` | `GET /health` → `status: healthy` | optional key | JSON files + CLI + env |
| llama.cpp | `llama-server` | 8080 | 127.0.0.1 | `http://localhost:8080/v1` | `http://localhost:8080` | `GET /health` → `status: ok`; `GET /v1/models` → `owned_by: llamacpp` (single-model mode) | optional key | CLI flags + env + router presets |
| vLLM | `vllm` | 8000 | **0.0.0.0** | `http://localhost:8000/v1` | `http://localhost:8000` | `GET /version` → `{version}` | optional key | CLI flags + YAML |

Watch: **vLLM binds `0.0.0.0`** (all interfaces), unlike the others. **vLLM and oMLX both default
to port 8000** — disambiguate by response marker (`GET /version` string for vLLM vs `GET /health`
`status: healthy` / `GET /openapi.json` `title: oMLX API` for oMLX).

## Detection quick reference

Install-check (fast, offline) then running-check (HTTP probe, ungated endpoints only):

| Runner | Install check | Running check (ungated) |
| --- | --- | --- |
| Ollama | `ollama` binary; macOS `/Applications/Ollama.app` | `GET /` → `Ollama is running` (also `GET /api/version`, `GET /api/tags`) |
| LM Studio | `lms` binary; macOS `/Applications/LM Studio.app`; `~/.lmstudio-home-pointer` | `GET /v1/models` → `owned_by: organization_owner` (no ungated positive health marker; `/` and `/health` return an error body) |
| oMLX | `omlx` binary; macOS `/Applications/oMLX.app` (bundle id `app.omlx`) | `GET /health` → `status: healthy` (ungated even with auth) |
| llama.cpp | `llama-server` binary/process | `GET /health` → `status: ok`; `GET /v1/models` → `owned_by: llamacpp` in single-model mode (both ungated; router-mode `/models` returns status/path metadata) |
| vLLM | `vllm` binary/entry-point | `GET /version` (ungated only when auth off; `--api-key` gates every endpoint) |

A port-only probe never identifies a runner — always confirm the response marker.

## Model ID grammar

| Runner | Grammar | Example |
| --- | --- | --- |
| Ollama | `name[:tag]`, `namespace/model`, `hf.co/{user}/{repo}[:quant]` | `llama3.2:70b`, `hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q8_0` |
| LM Studio | `publisher/model`, `publisher/repo/file.gguf`, `id@quant` | `openai/gpt-oss-20b`, `llama-3.1-8b@q4_k_m` |
| oMLX | model directory name, `{owner}/{model}`, `<model>:<profile>` | `Qwen3.6-35B-A3B-oQ6`, `mlx-community/MiniCPM-V-4.6-bf16` |
| llama.cpp | `--alias`, else GGUF filename, else HF repo `[:quant]` (no registry namespace) | `gemma-3-1b-it.Q4_K_M.gguf`, `ggml-org/gemma-3-1b-it-GGUF` |
| vLLM | HuggingFace model ID, local path, ModelScope ID, `--served-model-name` | `Qwen/Qwen2.5-1.5B-Instruct`, `/path/to/local/model` |

## Wiring into agentic CLIs

**OpenCode** — add a provider block using the AI SDK OpenAI-compatible adapter, pointing
`options.baseURL` at the runner's OpenAI base URL (the one that *includes* `/v1`), and map model IDs:

```json
{
  "provider": {
    "<runner>": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "<Runner> (local)",
      "options": { "baseURL": "http://localhost:<port>/v1" },
      "models": { "<model-id>": { "name": "<display name>" } }
    }
  }
}
```

See per-runner docs for concrete blocks (vLLM adds `"apiKey": "EMPTY"`).

**Claude Code** — point directly at the Anthropic base URL (the one that *excludes* `/v1`):

```bash
export ANTHROPIC_BASE_URL=http://localhost:<port>
export ANTHROPIC_AUTH_TOKEN=<key-or-placeholder>
claude --model <model-id>
```

**Runner-native launch hooks** — only Ollama and oMLX ship these:

- `ollama launch <claude|opencode|codex|codex-app>` (v0.15+)
- `omlx launch <claude|codex|opencode|openclaw|pi|hermes|copilot|codex_app>`

LM Studio, llama.cpp, and vLLM have no launch command — start the server and point the agent CLI at it.

## Per-runner references

- [ollama.md](./ollama.md) — GGUF, native `/api/*`, no auth, `hf.co/...` model refs
- [lmstudio.md](./lmstudio.md) — GGUF + MLX, `lms` CLI, relocatable home pointer
- [omlx.md](./omlx.md) — macOS-only MLX server, admin API, SSD KV cache
- [llamacpp.md](./llamacpp.md) — `llama-server`, no config file, public `/health` + `/models`
- [vllm.md](./vllm.md) — GPU throughput engine, single model, `0.0.0.0` bind, `VLLM_PORT` trap

Authoritative research (cite for depth): `claudine/docs/research/local_runners/<runner>.md`.