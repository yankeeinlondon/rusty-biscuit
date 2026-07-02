# Ollama

Open-source GGUF model runner. Serves a native `/api/*` REST API plus OpenAI-compatible
(`/v1`) and Anthropic-compatible (`/v1/messages`) endpoints. No auth for local inference.

## Platforms & binaries

| OS | Support | Binary | Alt binaries | Service |
| --- | --- | --- | --- | --- |
| macOS | native | `ollama` | `Ollama.app/Contents/MacOS/Ollama`, `Ollama.app/Contents/Resources/ollama` | app menubar/login item; `ollama serve` |
| Linux | native | `ollama` | `ollama` | systemd (official installer); `ollama serve` |
| Windows | native | `ollama.exe` | `Ollama.exe` | tray app / login item; `ollama serve` |

Actual inference runs in per-model `llama-server` child processes.

## API endpoints

Default listen: `127.0.0.1:11434`. Auth: none (keys accepted but ignored).

| Purpose | Method | Path | auth_gated | Notes |
| --- | --- | --- | --- | --- |
| health / identity | GET | `/` | no | Returns `Ollama is running`. No `/health` route. |
| version | GET | `/api/version` | no | `{"version":"..."}`. |
| model list | GET | `/api/tags` | no | Ollama-specific; entries carry `details.format: gguf`. |
| loaded models | GET | `/api/ps` | no | Empty `models` array when idle; entries appear while a model is loaded. |
| model info | POST | `/api/show` | no | Body `{"model":"..."}`. |
| load / unload | POST | `/api/generate` or `/api/chat` | no | Empty prompt loads; `keep_alive: 0` unloads. |
| OpenAI chat | POST | `/v1/chat/completions` | no | Base URL `http://localhost:11434/v1`. Since v0.1.24. |
| Anthropic messages | POST | `/v1/messages` | no | Base URL `http://localhost:11434`. Since v0.14.0. |

## Config & env vars

Config mechanism: **env vars** (no primary config file; optional `~/.ollama/server.json` cloud toggle).

| Variable | Effect |
| --- | --- |
| `OLLAMA_HOST` | Bind address **and** port (`127.0.0.1:11434`); must include both. |
| `OLLAMA_MODELS` | Model store path. |
| `OLLAMA_CONTEXT_LENGTH` | Default context window (VRAM-dependent default; often too small). |
| `OLLAMA_KEEP_ALIVE` | Time a model stays loaded (`5m`; `-1` = forever, `0` = unload now). |
| `OLLAMA_NUM_PARALLEL` | Parallel requests per model (scales context allocation). |
| `OLLAMA_MAX_LOADED_MODELS` | Concurrent loaded models. |

## Model store paths

| OS | Path |
| --- | --- |
| macOS | `~/.ollama/models` |
| Linux | `/usr/share/ollama/.ollama/models` |
| Windows | `C:\Users\%username%\.ollama\models` |

## Model acquisition & ID grammar

`name[:tag]` where `name` may include a `namespace/model`; `tag` defaults to `latest`. HuggingFace
GGUF repos: `hf.co/{user}/{repo}[:quant]` or `huggingface.co/{user}/{repo}[:quant]`.

- Registry: `ollama pull llama3.2`
- HuggingFace: `ollama run hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q8_0`
- Manual: `Modelfile` with `FROM ./model.gguf` (or Safetensors), then `ollama create`.

## Traps

- `OLLAMA_HOST` sets **both** host and port, not just the host.
- Default context length is VRAM-dependent and may be much smaller than the model's maximum;
  agentic coding tools should set `OLLAMA_CONTEXT_LENGTH=64000` or higher.
- `keep_alive: 0` unloads a model immediately; `keep_alive: -1` keeps it loaded indefinitely.
- `OLLAMA_NUM_PARALLEL` scales context allocation, so memory grows with context × parallelism.
- A port-only probe on 11434 is not enough to identify Ollama; verify the `GET /` identity marker.
- No built-in web UI; references to "Ollama WebUI" are third-party projects.

## Integration hooks

Since v0.15+, one-command setup: `ollama launch claude`, `ollama launch opencode`,
`ollama launch codex`, `ollama launch codex-app`. Each prompts for a model and configures the agent.

## OpenCode provider block

```json
{
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Ollama (local)",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": {
        "qwen3:1.7b": { "name": "Qwen3 1.7B (local)" }
      }
    }
  }
}
```

Claude Code: `ANTHROPIC_BASE_URL=http://localhost:11434 ANTHROPIC_AUTH_TOKEN=ollama claude --model qwen3-coder`

## Source

`claudine/docs/research/local_runners/ollama.md`
