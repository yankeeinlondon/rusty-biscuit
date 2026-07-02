# LM Studio

Closed-source desktop app + headless daemon (`llmster`) serving GGUF (llama.cpp) and MLX models.
Native `/api/v1` REST API plus OpenAI-compatible (`/v1`) and Anthropic-compatible (`/v1/messages`).
Ships the `lms` CLI. Auth optional (disabled by default).

## Platforms & binaries

| OS | Support | Binary | Alt binaries | Service |
| --- | --- | --- | --- | --- |
| macOS | native (Apple Silicon, macOS 14+) | `lms` | `LM Studio.app/Contents/MacOS/LM Studio`, `llmster` | login item / tray; `lms daemon up` |
| Linux | native (Ubuntu 20.04+) | `lms` | `llmster` | systemd (headless docs); `lms daemon up` |
| Windows | native | `lms.exe` | `LM Studio.exe`, `llmster.exe` | tray app / login item; `lms daemon up` |

## API endpoints

Default listen: `127.0.0.1:1234`. Auth optional — enable "Require Authentication" in Server Settings;
then send `Authorization: Bearer <token>` or `x-api-key: <token>`.

| Purpose | Method | Path | auth_gated | Notes |
| --- | --- | --- | --- | --- |
| model list (OpenAI) | GET | `/v1/models` | yes | `owned_by: organization_owner` is a strong identity marker. |
| model list (native) | GET | `/api/v1/models` | yes | Root key `models`; richer metadata + `loaded_instances`. |
| model list (legacy) | GET | `/api/v0/models` | yes | Backward-compat v0 API. |
| load / unload | POST | `/api/v1/models/load`, `/api/v1/models/unload` | yes | Requires LM Studio 0.4.0+. |
| OpenAI chat | POST | `/v1/chat/completions` | yes* | Base URL `http://localhost:1234/v1`. |
| Anthropic messages | POST | `/v1/messages` | yes* | Base URL `http://localhost:1234`. Since v0.4.1. |
| (no health) | GET | `/`, `/health` | no | Return 200 with an `Unexpected endpoint or method` error body — **not** positive markers. |

\* auth_gated only when "Require Authentication" is on (off by default).

## Config files & env vars

Config mechanism: **mixed** (JSON files under the resolved home dir + CLI flags + GUI toggles).

| File / var | Role |
| --- | --- |
| `~/.lmstudio-home-pointer` | Points to the active home dir. Fresh installs use `~/.lmstudio` (Windows `%USERPROFILE%\.lmstudio`); `~/.cache/lm-studio` occurs only as a legacy migration (this host). |
| `{home}/.internal/http-server-config.json` | Port, bind (`networkInterface`), CORS, JIT loading. |
| `{home}/settings.json` | App preferences; `downloadsFolder` overrides model dir. |
| `LMS_SERVER_HOST` | Default bind for `lms server start` (overridden by `--bind`). |
| `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` | Point Claude Code at LM Studio. |

## Model store paths

Canonical `~/.lmstudio/models` (Windows `%USERPROFILE%\.lmstudio\models`); redirectable via the
home-pointer file. Structure: `publisher/model/model-file.gguf`.

## Model acquisition & ID grammar

`publisher/model` (e.g. `openai/gpt-oss-20b`, `mlx-community/qwen2.5-7b-instruct-4bit`). `lms get`
accepts a quantization suffix `id@quant` (e.g. `llama-3.1-8b@q4_k_m`). File imports use
`publisher/repo/filename.gguf`. `lms load <id> --identifier=<alias>` assigns an arbitrary alias.

- Registry: `lms get openai/gpt-oss-20b`
- HuggingFace: `lms get https://huggingface.co/mlx-community/Qwen2.5-7B-Instruct-4bit`
- Manual: `lms import /path/to/model.gguf`

## Traps

- The home directory is relocatable via `~/.lmstudio-home-pointer`; do not hard-code `~/.lmstudio`.
- `GET /` and `GET /health` return HTTP 200 with an `Unexpected endpoint or method` error body;
  they are **not** positive health checks. Use `/v1/models` or `/api/v1/models`.
- `lms server start` without `--port` reuses the last-used port (from http-server-config.json), not
  necessarily 1234.
- `justInTimeModelLoading` changes whether `/v1/models` lists all downloaded models or only loaded ones.
- `/openapi.json` returns HTTP 200 with an `{"error":"Unexpected endpoint or method..."}` body —
  there is no OpenAPI document; not a schema source.
- The `lms` CLI ships with the app but only works after LM Studio has run at least once, then
  `~/.lmstudio/bin/lms bootstrap` (Windows: `cmd /c %USERPROFILE%/.lmstudio/bin/lms.exe bootstrap`).

## Integration hooks

None. LM Studio has no `lms launch <agent>` command; start the server and point the agent CLI at it
(e.g. Codex: `lms server start --port 1234` then `codex --oss -m openai/gpt-oss-20b`).

## OpenCode provider block

```json
{
  "provider": {
    "lmstudio": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LM Studio",
      "options": { "baseURL": "http://localhost:1234/v1" },
      "models": {
        "openai/gpt-oss-20b": { "name": "GPT-OSS 20B" },
        "qwen3-coder-next": { "name": "Qwen3 Coder Next" }
      }
    }
  }
}
```

Claude Code: `ANTHROPIC_BASE_URL=http://localhost:1234 ANTHROPIC_AUTH_TOKEN=lmstudio claude --model openai/gpt-oss-20b`

## Source

`claudine/docs/research/local_runners/lmstudio.md`
