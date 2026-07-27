# oMLX

Open-source, **macOS-only** (Apple Silicon, macOS 15+) MLX model server with paged SSD KV caching
and continuous batching. Serves LLM, VLM, embedding, reranker, audio, and OCR models. OpenAI-compatible
(`/v1`), Anthropic-compatible (`/v1/messages`), and a native admin API (`/admin/api`). FastAPI/OpenAPI.

## Platforms & binaries

| OS | Support | Binary | Alt binaries | Service |
| --- | --- | --- | --- | --- |
| macOS | native | `omlx` | `oMLX.app/Contents/MacOS/oMLX`, `.../omlx-cli`, `omlx-server` | brew services/launchd; menubar/login item; `omlx serve` |
| Linux | unsupported | — | — | — |
| Windows | unsupported | — | — | — |

## API endpoints

Default listen: `127.0.0.1:8000`. Auth optional — `omlx serve` defaults to no auth, but the macOS app
setup wizard and Homebrew flow write a required key. When enabled: OpenAI `Authorization: Bearer <key>`,
Anthropic `x-api-key <key>`.

| Purpose | Method | Path | auth_gated | Notes |
| --- | --- | --- | --- | --- |
| health / identity | GET | `/health` | no | `{"status":"healthy","engine_pool":{...}}`. Ungated even with auth — ideal probe. |
| version / schema | GET | `/openapi.json` | no | `{"info":{"title":"oMLX API"}}`. Confirms identity regardless of auth. |
| model list | GET | `/v1/models` | yes | `owned_by: omlx`. |
| loaded models | GET | `/v1/models/status` | yes | Per-model load state, memory estimates. |
| load / unload | POST | `/v1/models/{model_id}/load`, `/unload` | yes | Engine pool. |
| MCP server list | GET | `/v1/mcp/servers` | yes | MCP server list. |
| MCP tool list | GET | `/v1/mcp/tools` | yes | MCP tool list. |
| MCP tool execution | POST | `/v1/mcp/execute` | yes | MCP tool execution. |
| OpenAI chat | POST | `/v1/chat/completions` | opt | Base URL `http://localhost:8000/v1`. Also `/v1/rerank`, `/v1/audio/*`. |
| Anthropic messages | POST | `/v1/messages`, `/v1/messages/count_tokens` | opt | Base URL `http://localhost:8000`. |
| admin UI | GET | `/admin` | no | Web dashboard (offline-first, vendored assets). |
| admin API | * | `/admin/api/*` | admin session | Requires admin login — **not** the same Bearer token as `/v1/*`. |

## Config files & env vars

Config mechanism: **mixed** (JSON settings + CLI flags + env vars).

| File / var | Role |
| --- | --- |
| `~/.omlx/settings.json` | Primary: server host/port, auth, `model_dirs`, cache, integrations. |
| `~/.omlx/model_settings.json` | Per-model overrides (aliases, thinking, pinning, TTL). |
| `~/Library/Application Support/oMLX/base-path` | If present, CLI shim exports `OMLX_BASE_PATH`, relocating `~/.omlx`. |
| `OMLX_BASE_PATH` | Relocates the oMLX data/config home (default `~/.omlx`). |
| `OMLX_HOST` / `OMLX_PORT` | Override bind host / port. |
| `OMLX_API_KEY` / `OMLX_SECRET_KEY` | Client API key / admin session signing secret. |
| `OMLX_PAGED_SSD_CACHE_DIR` / `_MAX_SIZE` | Persistent SSD KV cache dir and cap. |

## Model store paths

`~/.omlx/models` (configurable via `settings.json` `model_dirs` array, or legacy `model_dir`).

## Model acquisition & ID grammar

Models addressed by the **directory name** under the model dir. Two-level `{owner}/{model}` supported
(since v0.3.9.dev2). A per-model alias can be set; `/v1/models` returns the alias and requests accept
both alias and directory name. Profiles exposed as `<model>:<profile>` or `<alias>:<profile>`.
Examples: `Qwen3.6-35B-A3B-oQ6`, `mlx-community/MiniCPM-V-4.6-bf16`.

- Manual: place an MLX model dir (`config.json` + `*.safetensors`) under `model_dirs`.
- HuggingFace: `POST /admin/api/hf/download` with `{"repo_id":"mlx-community/Llama-3.2-3B-Instruct-4bit"}`.

## Traps

- The CLI shim reads `~/Library/Application Support/oMLX/base-path` and exports `OMLX_BASE_PATH`, so
  `~/.omlx` is not a fixed location.
- `settings.json` has both `model_dirs` (active array) and legacy `model_dir`; edit `model_dirs`.
- Default port 8000 is shared with vLLM and others; `/health` or `/openapi.json` confirms identity.
- API-key enforcement depends on install path: `omlx serve` defaults to no auth, but the macOS
  app / Homebrew setup writes a required key.
- Admin endpoints under `/admin/api` require admin session auth, not the `/v1/*` Bearer token.
- The web UI is offline-first (vendored CDN assets) but still served from `/admin`.

## Integration hooks

`omlx launch <tool>` sets env vars and execs the agent: `claude` (supports `--opus/--sonnet/--haiku`),
`codex`, `opencode`, `openclaw` (`--tools-profile`), `pi`, `hermes`, `copilot`, `codex_app`.

## OpenCode provider block

```json
{
  "provider": {
    "omlx": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "oMLX (local)",
      "options": { "baseURL": "http://localhost:8000/v1" },
      "models": {
        "Qwen3.6-35B-A3B-oQ6": {
          "name": "Qwen3.6-35B-A3B-oQ6 (local)",
          "limit": { "context": 262144, "output": 98304 }
        }
      }
    }
  }
}
```

Claude Code: `ANTHROPIC_BASE_URL=http://localhost:8000 ANTHROPIC_AUTH_TOKEN=<omlx-api-key> claude --model Qwen3.6-35B-A3B-oQ6`

## Source

`claudine/docs/research/local_runners/omlx.md`
