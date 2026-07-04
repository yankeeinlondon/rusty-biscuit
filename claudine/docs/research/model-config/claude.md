---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://json.schemastore.org/claude-code-settings.json

model_config_paths:
  - scope: user
    path: ~/.claude/settings.json
    format: json
    notes: 'Global user settings. Top-level `model` key pins the default model; `env` block persists model-related environment variables. Observed on this host with `"model": "claude-fable-5[1m]"`.'
  - scope: repo
    path: .claude/settings.json
    format: json
    notes: 'Project-shared settings committed to source control. Lower precedence than local settings.'
  - scope: repo
    path: .claude/settings.local.json
    format: json
    notes: 'Personal project-only overrides. Gitignored by default when created by Claude Code.'
  - scope: env
    path: managed-settings.json / MDM plist / Windows registry / server-managed
    format: json
    notes: 'Organization-managed policy that cannot be overridden by user, project, or local settings. Delivery paths vary by OS and admin mechanism.'

api_standards:
  - standard: anthropic_compatible
    base_url_site: ANTHROPIC_BASE_URL
    auth_site: ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, or apiKeyHelper
    adapter:
    notes: 'Primary path for user-added arbitrary models. Claude Code emits the Anthropic Messages API to this base URL. Cloud-provider deployments (Bedrock, Vertex/Agent Platform, Foundry, Claude Platform on AWS) use their own base-url variables and CLAUDE_CODE_USE_* toggles but are intended for Claude-family models, not generic user models.'

metadata_overrides:
  - model_id
  - display_name
  - name
  - description
  - supported_capabilities

merge_semantics: merge

local_runners:
  - runner: ollama
    integration: first_class
    standard: anthropic_compatible
    example: 'ollama launch claude  # or manually: ANTHROPIC_BASE_URL=http://localhost:11434 ANTHROPIC_AUTH_TOKEN=ollama claude --model qwen3:1.7b'
    notes: 'Ollama serves an Anthropic-compatible `/v1/messages` endpoint and ships a native `ollama launch claude` hook. Size/quantization tags such as `:1.7b` are part of the model identifier.'
  - runner: omlx
    integration: first_class
    standard: anthropic_compatible
    example: 'omlx launch claude  # or manually: ANTHROPIC_BASE_URL=http://localhost:8000 ANTHROPIC_AUTH_TOKEN=<key> claude --model Qwen3.6-35B-A3B-oQ6'
    notes: 'oMLX serves an Anthropic-compatible `/v1/messages` endpoint and ships a native `omlx launch claude` hook. Default port is 8000.'
  - runner: lmstudio
    integration: base_url_override
    standard: anthropic_compatible
    example: 'ANTHROPIC_BASE_URL=http://localhost:1234 ANTHROPIC_AUTH_TOKEN=lmstudio claude --model openai/gpt-oss-20b'
    notes: 'LM Studio serves an Anthropic-compatible `/v1/messages` endpoint. Set auth token to `lmstudio` when authentication is disabled, or to the configured API token when required.'
  - runner: llamacpp
    integration: base_url_override
    standard: anthropic_compatible
    example: 'ANTHROPIC_BASE_URL=http://localhost:8080 ANTHROPIC_AUTH_TOKEN=<LLAMA_API_KEY-if-set> claude --model gemma-3-1b-it.Q4_K_M.gguf'
    notes: 'llama-server serves an Anthropic-compatible `/v1/messages` endpoint (added in build b7187). The model ID is the `--alias` value or the GGUF filename.'
  - runner: vllm
    integration: base_url_override
    standard: anthropic_compatible
    example: 'ANTHROPIC_BASE_URL=http://localhost:8000 ANTHROPIC_AUTH_TOKEN=EMPTY claude --model Qwen/Qwen2.5-1.5B-Instruct'
    notes: 'vLLM serves an Anthropic-compatible `/v1/messages` endpoint (v0.11.1+). Each vLLM process hosts one model; use `--served-model-name` aliases if needed.'
  - runner: other
    integration: base_url_override
    standard: anthropic_compatible
    example: 'ANTHROPIC_BASE_URL=http://localhost:<port> ANTHROPIC_AUTH_TOKEN=<token> claude --model <runner-model-id>'
    notes: 'Any local runner that exposes an Anthropic Messages API-compatible `/v1/messages` endpoint can be pointed at directly. Runners that speak only the OpenAI Chat Completions API require a translating gateway.'

cloud_bridge:
  supported: true
  mechanism: ANTHROPIC_BASE_URL pointing at an Anthropic-Messages-compatible gateway or proxy
  example: |
    export ANTHROPIC_BASE_URL="https://openai-gateway.example.com/anthropic"
    export ANTHROPIC_AUTH_TOKEN="gateway-token"
    claude --model "gpt-4.1"

default_model_site: 'Top-level `model` key in ~/.claude/settings.json (or project/local/managed settings.json); session-scope override via ANTHROPIC_MODEL env var or --model flag; precedence is --model > ANTHROPIC_MODEL > settings.json `model`.'

env_vars:
  - name: ANTHROPIC_BASE_URL
    effect: Redirects the Anthropic Messages API endpoint to a gateway, proxy, or local runner. Changes where requests go, not which model answers.
  - name: ANTHROPIC_MODEL
    effect: Sets the active model for the launched session. Lower precedence than --model, higher than the settings.json `model` key.
  - name: ANTHROPIC_API_KEY
    effect: API key sent as `x-api-key`. Overrides a saved claude.ai login when set.
  - name: ANTHROPIC_AUTH_TOKEN
    effect: Bearer token sent in the `Authorization` header. Preferred for gateway and local-runner authentication.
  - name: ANTHROPIC_DEFAULT_FABLE_MODEL
    effect: Pins what the `fable` alias resolves to and identifies Fable 5 for automatic fallback on third-party providers.
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL
    effect: Pins what the `opus` alias and opusplan plan-phase resolve to.
  - name: ANTHROPIC_DEFAULT_SONNET_MODEL
    effect: Pins what the `sonnet` alias and opusplan execution-phase resolve to.
  - name: ANTHROPIC_DEFAULT_HAIKU_MODEL
    effect: Pins what the `haiku` alias resolves to and the background/small-fast model.
  - name: ANTHROPIC_DEFAULT_FABLE_MODEL_NAME
    effect: Display name for the pinned `fable` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL_NAME
    effect: Display name for the pinned `opus` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_SONNET_MODEL_NAME
    effect: Display name for the pinned `sonnet` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME
    effect: Display name for the pinned `haiku` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_FABLE_MODEL_DESCRIPTION
    effect: Display description for the pinned `fable` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL_DESCRIPTION
    effect: Display description for the pinned `opus` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_SONNET_MODEL_DESCRIPTION
    effect: Display description for the pinned `sonnet` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_HAIKU_MODEL_DESCRIPTION
    effect: Display description for the pinned `haiku` alias in the model picker.
  - name: ANTHROPIC_DEFAULT_FABLE_MODEL_SUPPORTED_CAPABILITIES
    effect: JSON capability flags for the pinned `fable` alias.
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL_SUPPORTED_CAPABILITIES
    effect: JSON capability flags for the pinned `opus` alias.
  - name: ANTHROPIC_DEFAULT_SONNET_MODEL_SUPPORTED_CAPABILITIES
    effect: JSON capability flags for the pinned `sonnet` alias.
  - name: ANTHROPIC_DEFAULT_HAIKU_MODEL_SUPPORTED_CAPABILITIES
    effect: JSON capability flags for the pinned `haiku` alias.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION
    effect: Adds one custom model ID to the /model picker without replacing built-in aliases.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_NAME
    effect: Display name for the custom picker entry.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION
    effect: Display description for the custom picker entry.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_SUPPORTED_CAPABILITIES
    effect: Declares capability hints for the custom picker entry as a JSON object.
  - name: CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY
    effect: When 1, queries the gateway GET /v1/models endpoint at startup and populates the /model picker with discovered Claude/Anthropic-prefixed IDs.
  - name: CLAUDE_CODE_SUBAGENT_MODEL
    effect: Overrides the model used for all subagents and agent teams. Set to `inherit` to use normal model resolution instead.
  - name: CLAUDE_CODE_USE_BEDROCK
    effect: Switches wire format to Bedrock InvokeModel; aliases resolve to Bedrock inference-profile ARNs.
  - name: CLAUDE_CODE_USE_VERTEX
    effect: Switches wire format to Agent Platform rawPredict; aliases resolve to Vertex version names.
  - name: CLAUDE_CODE_USE_FOUNDRY
    effect: Routes through Microsoft Foundry in Anthropic Messages format; aliases resolve to Foundry deployment names.
  - name: ANTHROPIC_BEDROCK_BASE_URL
    effect: Override the Bedrock endpoint URL.
  - name: ANTHROPIC_VERTEX_BASE_URL
    effect: Override the Vertex AI endpoint URL.
  - name: ANTHROPIC_FOUNDRY_BASE_URL
    effect: Override the Microsoft Foundry endpoint URL.
  - name: ANTHROPIC_AWS_BASE_URL
    effect: Override the Claude Platform on AWS endpoint URL.

changes:
  - 'Local runners (Ollama, oMLX, LM Studio, llama.cpp, vLLM) are now supported paths via ANTHROPIC_BASE_URL because they expose Anthropic-compatible /v1/messages endpoints; the previous revision incorrectly classified them as unsupported/gateway-required.'
  - 'Ollama and oMLX ship first-class `launch claude` integration hooks in addition to the base-URL override path.'
  - 'Claude Code settings precedence clarified as managed > CLI arguments > local > project > user.'
  - 'Added CLAUDE_CODE_SUBAGENT_MODEL and ANTHROPIC_DEFAULT_*_MODEL_{NAME,DESCRIPTION,SUPPORTED_CAPABILITIES} variables, which were not present in the prior revision.'
  - 'Confirmed that gateway model discovery only adds IDs prefixed with `claude` or `anthropic`, and that discovered explicit IDs fold into built-in alias rows when they resolve to the same model.'

requires_claudine_update: true
reason: 'Claudine should treat Ollama and oMLX as first-class Claude Code integrations (detecting `ollama launch claude` / `omlx launch claude` hooks) and LM Studio, llama.cpp, and vLLM as valid base-URL-override targets for Claude Code via their Anthropic-compatible endpoints, rather than requiring a translating gateway.'
---

# Claude Code User-Side Model Configuration

## Introduction to Claude Code Model Configuration

Claude Code stores model configuration in JSON `settings.json` files layered by scope. The precedence order, from highest to lowest, is:

1. Managed settings (server-managed, MDM plist, Windows registry, `managed-settings.json`)
2. CLI arguments (`--model`, `--fallback-model`, etc.)
3. Local settings (`.claude/settings.local.json`)
4. Project settings (`.claude/settings.json`)
5. User settings (`~/.claude/settings.json`)

| Scope | Path | Format | Who it affects |
| :---- | :--- | :----- | :------------- |
| User | `~/.claude/settings.json` | JSON | You, across all projects |
| Project | `.claude/settings.json` | JSON | Everyone in the repository |
| Local | `.claude/settings.local.json` | JSON | You, in this repository only |
| Managed | server-managed, MDM plist, Windows registry, or `managed-settings.json` | JSON | Organization members |

The actual host `~/.claude/settings.json` contains a top-level `"model"` key, for example `"model": "claude-fable-5[1m]"`. This is the persistent user default. CLI flags plus environment variables apply session-scoped overrides on top of the settings files.

A formal machine-readable schema exists for `settings.json`: the `https://json.schemastore.org/claude-code-settings.json` JSON Schema, referenced by the official documentation as the schema for Claude Code settings. Anthropic notes that schemastore updates may lag the latest CLI release, so a validation warning on a newly documented field does not necessarily mean the configuration is invalid.

## Adding Cloud Models

Claude Code is a first-party Anthropic client out of the box and does not speak the OpenAI Chat Completions API. The supported way to use a cloud model that is not in the built-in catalog is to route Claude Code through an **Anthropic-Messages-compatible gateway or proxy**. The gateway is responsible for translating between the Anthropic Messages API and the upstream provider's native format.

### Concrete example

```bash
# Point Claude Code at your gateway
export ANTHROPIC_BASE_URL="https://gateway.example.com/anthropic"
export ANTHROPIC_AUTH_TOKEN="gateway-api-key"

# Use any model string the gateway accepts
claude --model "my-gateway/claude-opus-4-8"
```

Equivalent persistent configuration in `~/.claude/settings.json`:

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://gateway.example.com/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "gateway-api-key"
  },
  "model": "my-gateway/claude-opus-4-8"
}
```

### What each piece means

| Setting | Effect |
| :------ | :----- |
| `ANTHROPIC_BASE_URL` | Redirects every Anthropic Messages API request to the gateway. It changes *where* requests go, not *which* model answers. |
| `ANTHROPIC_AUTH_TOKEN` | The credential sent to the gateway as `Authorization: Bearer <token>`. Use this for bearer-token gateways and for local runners that accept any token. |
| `ANTHROPIC_API_KEY` | The credential sent to the gateway as `x-api-key`. Use this when the gateway expects an API-key header. |
| `--model` / `ANTHROPIC_MODEL` / `model` | The model ID passed to the gateway. Can be any string the gateway understands. |

### Adapter mechanism

There is **no adapter mechanism** like an npm package key or provider plug-in. Claude Code expects the gateway or local runner to expose the Anthropic Messages API. The gateway is responsible for translating to the upstream provider's native format.

### Per-model metadata

Users cannot declare rich per-model metadata such as cost, context-window size, modalities, or reasoning support directly in `settings.json`. The metadata surface is minimal and exposed through environment variables:

| Metadata | Where it lives | Notes |
| :------- | :------------- | :---- |
| `model_id` | `ANTHROPIC_CUSTOM_MODEL_OPTION` or the gateway `id` field | Required identifier. |
| `display_name` | Gateway `/v1/models` response `display_name` | Used for discovered entries. |
| `name` | `ANTHROPIC_CUSTOM_MODEL_OPTION_NAME` or `ANTHROPIC_DEFAULT_*_MODEL_NAME` | Display name for the single custom picker entry or a pinned alias. |
| `description` | `ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION` or `ANTHROPIC_DEFAULT_*_MODEL_DESCRIPTION` | Display description for the entry or alias. |
| `supported_capabilities` | `ANTHROPIC_CUSTOM_MODEL_OPTION_SUPPORTED_CAPABILITIES` or `ANTHROPIC_DEFAULT_*_MODEL_SUPPORTED_CAPABILITIES` | Capability hints as a JSON object. |

### Interaction with the built-in catalog

User-added models **merge** with the built-in catalog rather than replacing it:

- `ANTHROPIC_CUSTOM_MODEL_OPTION` adds a single entry at the bottom of the `/model` picker.
- Gateway discovery (`CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY=1`) adds discovered `claude`/`anthropic`-prefixed IDs alongside built-ins, skipping exact duplicates and folding entries that resolve to the same model as a built-in alias.
- `ANTHROPIC_DEFAULT_*_MODEL` variables shadow the built-in alias targets (`fable`, `opus`, `sonnet`, `haiku`) but do not remove the aliases from the catalog.
- `modelOverrides` in `settings.json` rewrites Anthropic IDs to provider-specific strings for the picker/wire, but again does not replace the catalog.

Because the built-in catalog self-updates with new Claude Code releases, a manual gateway-routed entry should be removed once the CLI natively lists that model. Claude Code does not automate this cleanup; users must delete the custom env var or settings entry themselves.

### Cross-cloud bridging

Claude Code can be routed at a different cloud vendor's API via `ANTHROPIC_BASE_URL` pointed at a gateway that translates the Anthropic Messages API to the target provider's format. The same mechanism works for local runners that speak the Anthropic Messages API. For provider-specific Claude deployments (Bedrock, Vertex AI/Agent Platform, Foundry, Claude Platform on AWS), use the dedicated variables described in the environment overrides section.

```bash
# Route Claude Code through a gateway that forwards to a non-Anthropic cloud API
export ANTHROPIC_BASE_URL="https://openai-gateway.example.com/anthropic"
export ANTHROPIC_AUTH_TOKEN="gateway-token"
claude --model "gpt-4.1"
```

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Claude Code "knowing about" a runner. Any runner that exposes an Anthropic-Messages-compatible endpoint can be used by pointing `ANTHROPIC_BASE_URL` at it. Most popular local runners now do this.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | First-class | `ollama launch claude` configures Claude Code automatically; manual setup uses `ANTHROPIC_BASE_URL=http://localhost:11434`. |
| oMLX | First-class | `omlx launch claude` configures Claude Code automatically; manual setup uses `ANTHROPIC_BASE_URL=http://localhost:8000`. |
| LM Studio | Base-URL override | Start the LM Studio server, then set `ANTHROPIC_BASE_URL=http://localhost:1234`. |
| llama.cpp | Base-URL override | Start `llama-server`, then set `ANTHROPIC_BASE_URL=http://localhost:8080`. |
| vLLM | Base-URL override | Start `vllm serve`, then set `ANTHROPIC_BASE_URL=http://localhost:8000`. |
| Other | Base-URL override or unsupported | Works if the runner speaks the Anthropic Messages API; otherwise requires a translating gateway. |

### Practical example for Ollama

```bash
# First-class hook
ollama launch claude

# Manual equivalent
export ANTHROPIC_BASE_URL="http://localhost:11434"
export ANTHROPIC_AUTH_TOKEN="ollama"
claude --model "qwen3:1.7b"
```

The model ID string is whatever the runner accepts; size and quantization tags such as `:1.7b` are part of the runner's model namespace.

### Practical example for LM Studio

```bash
export ANTHROPIC_BASE_URL="http://localhost:1234"
export ANTHROPIC_AUTH_TOKEN="lmstudio"
claude --model "openai/gpt-oss-20b"
```

If LM Studio has "Require Authentication" enabled, set `ANTHROPIC_AUTH_TOKEN` to the configured API token instead of `lmstudio`.

## Environment Overrides

Environment variables take precedence over the corresponding `settings.json` field when both exist. Variables can be set either in the shell before launching `claude` or under the `env` key in a `settings.json` file. Settings-file `env` values persist across launches and are scoped by the file's location (user, project, local, or managed).

The variables that redirect model endpoints or selection are:

| Variable | Effect | Precedence |
| :------- | :----- | :--------- |
| `ANTHROPIC_BASE_URL` | Redirect API endpoint to a gateway or local runner. | Overrides the default Anthropic API endpoint. |
| `ANTHROPIC_MODEL` | Set the session model. | `--model` > `ANTHROPIC_MODEL` > `model` setting. |
| `ANTHROPIC_DEFAULT_FABLE_MODEL` | Pin the `fable` alias target. | Overrides built-in alias resolution. |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | Pin the `opus` alias target. | Overrides built-in alias resolution. |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | Pin the `sonnet` alias target. | Overrides built-in alias resolution. |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | Pin the `haiku` alias target. | Overrides built-in alias resolution. |
| `ANTHROPIC_CUSTOM_MODEL_OPTION` | Add one custom model to the picker. | Merges with built-in catalog. |
| `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY` | Populate picker from gateway `/v1/models`. | Merges discovered entries with built-in catalog. |
| `CLAUDE_CODE_USE_BEDROCK` | Use Bedrock InvokeModel format. | Switches provider backend. |
| `CLAUDE_CODE_USE_VERTEX` | Use Agent Platform rawPredict format. | Switches provider backend. |
| `CLAUDE_CODE_USE_FOUNDRY` | Use Microsoft Foundry backend. | Switches provider backend. |
| `CLAUDE_CODE_SUBAGENT_MODEL` | Override the model used for subagents and agent teams. | Overrides subagent frontmatter and per-invocation model parameter. |

## Changelog

- **2026-07-02** — Reclassified local runners. Ollama and oMLX now have first-class `launch claude` hooks; LM Studio, llama.cpp, and vLLM work via `ANTHROPIC_BASE_URL` base-URL override on their Anthropic-compatible endpoints. The previous revision incorrectly treated all local runners as unsupported/gateway-required.
- **2026-07-02** — Added `CLAUDE_CODE_SUBAGENT_MODEL` and the `ANTHROPIC_DEFAULT_*_MODEL_{NAME,DESCRIPTION,SUPPORTED_CAPABILITIES}` variable families.
- **2026-07-02** — Clarified settings precedence as managed > CLI arguments > local > project > user.
- **2026-07-02** — Updated gateway model discovery behavior: only `claude`/`anthropic`-prefixed IDs are added, and explicit IDs that resolve to the same model as a built-in alias are folded into the alias row.

## Sources

- [Claude Code — Model configuration](https://code.claude.com/docs/en/model-config)
- [Claude Code — Settings](https://code.claude.com/docs/en/settings)
- [Claude Code — Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code — LLM gateway](https://code.claude.com/docs/en/llm-gateway)
- [Claude Code — Connect to an LLM gateway](https://code.claude.com/docs/en/llm-gateway-connect)
- [Claude Code — Gateway protocol reference](https://code.claude.com/docs/en/llm-gateway-protocol)
- [Claude Code — CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code settings JSON Schema](https://json.schemastore.org/claude-code-settings.json)
- [Ollama Anthropic compatibility](https://docs.ollama.com/api/anthropic-compatibility)
- [oMLX GitHub repository](https://github.com/jundot/omlx)
- [LM Studio Anthropic-compatible endpoints](https://lmstudio.ai/docs/developer/anthropic-compat)
- [llama.cpp server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [vLLM online serving reference](https://docs.vllm.ai/en/latest/online_serving/)
