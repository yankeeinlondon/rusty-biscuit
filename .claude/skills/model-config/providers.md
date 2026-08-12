# Per-Provider Model Configuration Reference

Distilled from the schema-validated frontmatter of
`claudine/docs/research/model-config/<provider>.md` (2026-07-02). Ports and endpoint
shapes follow the local-runners ground truth — see the **local-llm-runners** skill.

## Claude Code

- **Config:** `~/.claude/settings.json` (user) · `.claude/settings.json` (repo) ·
  `.claude/settings.local.json` (personal repo) · managed policy (MDM/registry). JSON;
  formal schema at `https://json.schemastore.org/claude-code-settings.json`.
- **Client speaks:** Anthropic Messages only. Base URL: `ANTHROPIC_BASE_URL` (no `/v1`
  suffix — the SDK appends `/v1/messages`). Auth: `ANTHROPIC_AUTH_TOKEN` (Bearer) or
  `ANTHROPIC_API_KEY` (`x-api-key`).
- **Local models** (all runners serve Anthropic Messages — no gateway needed):

  ```bash
  ollama launch claude   # first-class hook; or manually:
  ANTHROPIC_BASE_URL=http://localhost:11434 ANTHROPIC_AUTH_TOKEN=ollama \
      claude --model qwen3:1.7b
  ```

  oMLX has the same first-class hook (`omlx launch claude`, port 8000); LM Studio
  (1234), llama.cpp (8080, build b7187+), and vLLM (8000, v0.11.1+) are base-URL
  overrides.
- **Cloud bridge:** point `ANTHROPIC_BASE_URL` at an Anthropic-compatible gateway;
  the model string is whatever the gateway accepts.
- **Default model:** `model` key in settings.json; precedence `--model` >
  `ANTHROPIC_MODEL` > settings. Alias pins via `ANTHROPIC_DEFAULT_{FABLE,OPUS,SONNET,HAIKU}_MODEL`;
  one-off picker entry via `ANTHROPIC_CUSTOM_MODEL_OPTION`. Merge semantics: **merge**.

## Codex CLI

- **Config:** `~/.codex/config.toml` (user) · `.codex/config.toml` (repo, trusted
  projects; cannot override provider/auth keys) · profiles · `/etc/codex/config.toml`.
  Formal schema in the codex repo (`config.schema.json`).
- **Client speaks:** OpenAI **Responses API only** (`wire_api = "responses"`; `chat`
  removed). Base URL: `[model_providers.<id>].base_url` or `openai_base_url`.
- **Local models:** Ollama and LM Studio are built-in OSS providers
  (`codex --oss [-m id]`, `--local-provider lmstudio`, `model_provider = "ollama"`);
  `ollama launch codex` also exists. oMLX / llama.cpp / vLLM are custom providers:

  ```toml
  model = "Qwen/Qwen2.5-1.5B-Instruct"
  model_provider = "local_vllm"

  [model_providers.local_vllm]
  name = "vLLM"
  base_url = "http://localhost:8000/v1"
  ```

- **Cloud bridge:** the target must serve `/v1/responses`. Chat-Completions-only
  vendors (e.g. Mistral) need a Responses-translating proxy (e.g. LiteLLM) —
  `base_url` points at the proxy, never at the vendor directly.
- **Default model:** top-level `model` key; `--model`/`-m` or `/model` per session.
  Merge semantics: **replace** (`model_catalog_json` replaces the catalog; reserved
  provider ids `openai`/`ollama`/`lmstudio` cannot be overridden).
  `CODEX_OSS_BASE_URL`/`CODEX_OSS_PORT` redirect the built-in OSS providers.

## Gemini CLI

- **Config:** `~/.gemini/settings.json` (user) · `.gemini/settings.json` (repo) ·
  `.env` files. Model at `model.name`.
- **Client speaks:** bespoke Gemini API only — no OpenAI or Anthropic client ships.
  Base URL: `GOOGLE_GEMINI_BASE_URL` (gateway auth mode, v0.46.0+) or
  `GOOGLE_VERTEX_BASE_URL`.
- **Local models:** every runner requires a **Gemini-compatible translating proxy**
  (no runner serves the Gemini API): set `GOOGLE_GEMINI_BASE_URL=http://localhost:<proxy-port>`
  and let the proxy translate to the runner's OpenAI endpoint. Run the proxy on a port
  that doesn't collide with the runner's own (llama.cpp also defaults to 8080).
- **Cloud bridge:** same mechanism — the gateway must speak the Gemini API and
  translate to the target vendor. A direct base URL at a non-Gemini API will not work.
- **Default model:** `model.name` in settings.json; `GEMINI_MODEL` env; `--model`.
  Merge semantics: **merge**.

## Goose

- **Config:** `~/.config/goose/config.yaml` + `custom_providers/*.json` +
  `secrets.yaml`. Provider/model via `GOOSE_PROVIDER` / `GOOSE_MODEL`.
- **Client speaks:** OpenAI-compatible and Anthropic-compatible (custom provider JSON
  `engine: openai|anthropic` + `base_url`), plus a native Ollama provider
  (`OLLAMA_HOST`).
- **Local models:** Ollama first-class (built-in provider), LM Studio first-class
  (built-in provider on its OpenAI endpoint); oMLX / llama.cpp / vLLM as custom
  OpenAI-compatible providers:

  ```yaml
  GOOSE_PROVIDER: ollama
  GOOSE_MODEL: qwen2.5
  OLLAMA_HOST: http://localhost:11434
  ```

- **Cloud bridge:** custom provider JSON with `engine: openai` (or `anthropic`) and
  `base_url` at the vendor/gateway; `requires_auth` + `api_key_env` for credentials.
- **Default model:** `GOOSE_MODEL` in config.yaml or env. Merge semantics: **merge**.

## Kimi Code CLI

- **Config:** `~/.kimi/config.toml` (user; `$KIMI_SHARE_DIR` and `--config-file`
  overrides). Models are declared in `[models.<key>]` tables referencing
  `[providers.<name>]` blocks.
- **Client speaks:** `openai_legacy`, `openai_responses`, `anthropic`, and the bespoke
  `kimi` type. Base URL: `providers.<name>.base_url`; `OPENAI_BASE_URL` env works for
  OpenAI types only (no `ANTHROPIC_*` env override — Anthropic types are config-only).
- **Local models:** all five runners are base-URL overrides on OpenAI endpoints:

  ```toml
  [providers.ollama-local]
  type = "openai_legacy"
  base_url = "http://localhost:11434/v1"
  api_key = "ollama"

  [models."ollama/qwen3:1.7b"]
  provider = "ollama-local"
  model = "qwen3:1.7b"
  ```

- **Cloud bridge:** a provider block whose `base_url` targets a gateway speaking a
  supported standard (LiteLLM for anything else).
- **Default model:** top-level `default_model` (must name a declared `[models]` key).
  Merge semantics: **shadow** — a manual entry with a managed key silently wins.

## OpenCode

- **Config:** `~/.config/opencode/opencode.json(c)` (user) · `opencode.json(c)` (repo)
  · `OPENCODE_CONFIG` / `OPENCODE_CONFIG_CONTENT` (env) · managed locations. Formal
  schema `https://opencode.ai/config.json`.
- **Client speaks:** OpenAI-compatible via ai-sdk adapters — `provider.<id>.npm`
  (`@ai-sdk/openai-compatible` for `/v1/chat/completions`, `@ai-sdk/openai` for
  `/v1/responses`) + `options.baseURL`.
- **Local models:** Ollama and oMLX first-class (`ollama launch opencode`,
  `omlx launch opencode`); the rest base-URL overrides:

  ```jsonc
  { "provider": { "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": { "gemma3:27b": { "name": "Gemma 3 27B" } } } },
    "model": "ollama/gemma3:27b" }
  ```

- **Cloud bridge:** direct for OpenAI-compatible vendors/aggregators (OpenRouter et
  al.); LiteLLM proxy for vendors that speak only another standard.
- **Default model:** top-level `model` (`<provider>/<id>`). Merge semantics: **merge**
  (models.dev catalog + user blocks; same-id user block augments/overrides fields).

## Qwen Code

- **Config:** `~/.qwen/settings.json` (user) · `.qwen/settings.json` (repo) · `.env`
  files · system defaults/overrides per OS.
- **Client speaks:** OpenAI, Anthropic, Gemini, and Vertex protocols via
  `modelProviders.<type>` with per-model `baseUrl`; `OPENAI_BASE_URL` /
  `ANTHROPIC_BASE_URL` env overrides exist for those types.
- **Local models:** all five runners are base-URL overrides on OpenAI endpoints —
  add under `modelProviders.openai.models`:

  ```json
  { "id": "gemma3:27b", "name": "Gemma 3 27B (Ollama)",
    "envKey": "OLLAMA_API_KEY", "baseUrl": "http://localhost:11434/v1" }
  ```

- **Cloud bridge:** per-model `baseUrl` under the matching protocol type at the
  vendor/gateway endpoint.
- **Default model:** `model.name` (+ disambiguating `model.baseUrl`) in settings.
  Merge semantics: **merge**.

## Pi

- **Config:** `~/.pi/agent/models.json` (providers/models) ·
  `~/.pi/agent/settings.json` (defaults) · `.pi/settings.json` (repo).
- **Client speaks:** `api:` per provider — `openai-completions`, `openai-responses`,
  `anthropic-messages`, and bespoke variants. Base URL: `baseUrl` at provider or model
  level; `compat` flags tune protocol quirks.
- **Local models:** oMLX first-class (`omlx launch pi`); the rest base-URL overrides:

  ```json
  { "providers": { "ollama": {
      "baseUrl": "http://localhost:11434/v1", "api": "openai-completions",
      "apiKey": "ollama", "models": [{ "id": "llama3.1:8b" }] } } }
  ```

  (Ollama ignores the key but Pi requires one — use a dummy.)
- **Cloud bridge:** provider block with `baseUrl` at the vendor/aggregator (OpenRouter
  example in the research doc), `api` set to the standard it serves.
- **Default model:** `defaultProvider` + `defaultModel` in settings.json. Merge
  semantics: **shadow**.

## Kilo Code

- **Config:** `~/.config/kilo/kilo.jsonc` (user) · `kilo.jsonc` / `.kilo/kilo.jsonc`
  (repo) · `KILO_CONFIG` / `KILO_CONFIG_CONTENT` (env) · managed locations.
- **Client speaks:** OpenAI- and Anthropic-compatible via AI SDK adapters
  (`provider.<id>.npm` or `api` field) + `options.baseURL` — the OpenCode-derived
  shape.
- **Local models:** all five runners are base-URL overrides on OpenAI endpoints
  (same block shape as OpenCode; no `kilo` launch hooks exist in any runner):

  ```jsonc
  { "provider": { "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": { "gemma3:27b": {} } } },
    "model": "ollama/gemma3:27b" }
  ```

- **Cloud bridge:** `provider.<id>.options.baseURL` + matching adapter at the
  vendor/gateway (LiteLLM for non-matching standards).
- **Default model:** top-level `model` key; `KILO_PROVIDER` / `KILO_<FIELD>` env
  overrides. Merge semantics: **merge**.
