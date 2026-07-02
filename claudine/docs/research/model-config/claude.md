---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://json.schemastore.org/claude-code-settings.json

config_files:
  - scope: user
    path: ~/.claude/settings.json
    format: json
    notes: 'Global user settings. Top-level `model` key pins the default model; `env` block persists model-related environment variables. Observed on this host with `"model": "claude-fable-5[1m]"`.'
  - scope: repo
    path: .claude/settings.json
    format: json
    notes: 'Project-shared settings committed to source control. Overrides user settings.'
  - scope: repo
    path: .claude/settings.local.json
    format: json
    notes: 'Personal project-only overrides. Gitignored by default when created by Claude Code.'
  - scope: env
    path: managed-settings.json / MDM plist / Windows registry / server-managed
    format: json
    notes: 'Organization-managed policy that cannot be overridden by user/project/local settings. Delivery paths vary by OS and admin mechanism.'

api_standards:
  - standard: anthropic_compatible
    base_url_site: ANTHROPIC_BASE_URL
    auth_site: ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, or gateway credential variables
    notes: The only supported path for user-added arbitrary models. Claude Code emits the Anthropic Messages API, so a gateway/proxy must translate. Provider-specific deployments (Bedrock, Vertex/Agent Platform, Foundry, Claude Platform on AWS) use their own base-url variables and CLAUDE_CODE_USE_* toggles but are intended for Claude-family models, not generic user models.

metadata_overrides:
  - model_id
  - display_name
  - name
  - description
  - supported_capabilities

merge_semantics: merge

local_runners:
  - runner: ollama
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim. A runner must be fronted by an Anthropic-Messages-compatible gateway.
  - runner: omlx
    supported: unsupported
    notes: No native integration. Requires a translating gateway.
  - runner: lmstudio
    supported: unsupported
    notes: No native integration. Requires a translating gateway.
  - runner: llamacpp
    supported: unsupported
    notes: No native integration. Requires a translating gateway.
  - runner: vllm
    supported: unsupported
    notes: No native integration. Requires a translating gateway.
  - runner: other
    supported: unsupported
    notes: Any local runner must be exposed through a gateway that implements the Anthropic Messages API.

default_model_site: model key in ~/.claude/settings.json (or project/local/managed settings.json); session-scope override via ANTHROPIC_MODEL env var or --model flag

env_vars:
  - name: ANTHROPIC_BASE_URL
    effect: Redirects the Anthropic Messages API endpoint to a gateway or proxy. Changes where requests go, not which model answers.
  - name: ANTHROPIC_MODEL
    effect: Sets the active model for the launched session. Lower precedence than --model, higher than the settings.json `model` key.
  - name: ANTHROPIC_DEFAULT_FABLE_MODEL
    effect: Pins what the `fable` alias resolves to and identifies Fable 5 for automatic fallback on third-party providers.
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL
    effect: Pins what the `opus` alias and opusplan plan-phase resolve to.
  - name: ANTHROPIC_DEFAULT_SONNET_MODEL
    effect: Pins what the `sonnet` alias and opusplan execution-phase resolve to.
  - name: ANTHROPIC_DEFAULT_HAIKU_MODEL
    effect: Pins what the `haiku` alias resolves to and the background/small-fast model.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION
    effect: Adds one custom model ID to the /model picker without replacing built-in aliases.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_NAME
    effect: Display name for the custom picker entry.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION
    effect: Display description for the custom picker entry.
  - name: ANTHROPIC_CUSTOM_MODEL_OPTION_SUPPORTED_CAPABILITIES
    effect: Declares capabilities for the custom picker entry.
  - name: CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY
    effect: When 1, queries the gateway GET /v1/models endpoint at startup and populates the /model picker with discovered Claude/Anthropic-prefixed IDs.
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

changes: []

requires_claudine_update: false
---

# Claude Code User-Side Model Configuration

## Introduction to Claude Code Model Configuration

Claude Code stores model configuration in JSON `settings.json` files layered by scope:

| Scope | Path | Format | Who it affects |
| :---- | :--- | :----- | :------------- |
| User | `~/.claude/settings.json` | JSON | You, across all projects |
| Project | `.claude/settings.json` | JSON | Everyone in the repository |
| Local | `.claude/settings.local.json` | JSON | You, in this repository only |
| Managed | server-managed, MDM plist, Windows registry, or `managed-settings.json` | JSON | Organization members |

The actual host `~/.claude/settings.json` contains a top-level `"model"` key, for example `"model": "claude-fable-5[1m]"`. This is the persistent user default. Project and managed settings override user settings, and CLI flags plus environment variables apply session-scoped overrides on top.

A formal machine-readable schema exists for `settings.json`: the `https://json.schemastore.org/claude-code-settings.json` JSON Schema, referenced by the official documentation as the schema for Claude Code settings. Anthropic notes that schemastore updates may lag the latest CLI release, so a validation warning on a newly documented field does not necessarily mean the configuration is invalid.

## Adding Cloud Models

Claude Code is a first-party Anthropic-only client out of the box. It does not speak the OpenAI Chat Completions API and has no generic "add a provider" plug-in. The only supported way to use a cloud model that is not in the built-in catalog is to route Claude Code through an **Anthropic-Messages-compatible gateway or proxy**.

### Concrete example

```bash
# Point Claude Code at your gateway
export ANTHROPIC_BASE_URL="https://gateway.example.com/anthropic"
export ANTHROPIC_API_KEY="gateway-api-key"

# Use any model string the gateway accepts
claude --model "my-gateway/claude-opus-4-8"
```

Equivalent persistent configuration in `~/.claude/settings.json`:

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://gateway.example.com/anthropic",
    "ANTHROPIC_API_KEY": "gateway-api-key"
  },
  "model": "my-gateway/claude-opus-4-8"
}
```

### What each piece means

| Setting | Effect |
| :------ | :----- |
| `ANTHROPIC_BASE_URL` | Redirects every Anthropic Messages API request to the gateway. It changes *where* requests go, not *which* model answers. |
| `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` | The credential sent to the gateway. `ANTHROPIC_AUTH_TOKEN` becomes an `Authorization: Bearer` header; `ANTHROPIC_API_KEY` becomes `X-Api-Key`. |
| `--model` / `ANTHROPIC_MODEL` / `model` | The model ID passed to the gateway. Can be any string the gateway understands. |

### Adapter mechanism

There is **no adapter mechanism** like an npm package key or provider plug-in. Claude Code expects the gateway to expose the Anthropic Messages API. The gateway is responsible for translating to the upstream provider's native format.

### Per-model metadata

Users cannot declare rich per-model metadata such as cost, context-window size, modalities, or reasoning support. The metadata surface is minimal:

| Metadata | Where it lives | Notes |
| :------- | :------------- | :---- |
| `model_id` | `ANTHROPIC_CUSTOM_MODEL_OPTION` or the gateway `id` field | Required identifier. |
| `display_name` | Gateway `/v1/models` response `display_name` | Used for discovered entries. |
| `name` | `ANTHROPIC_CUSTOM_MODEL_OPTION_NAME` | Display name for the single custom picker entry. |
| `description` | `ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION` | Display description for the single custom picker entry. |
| `supported_capabilities` | `ANTHROPIC_CUSTOM_MODEL_OPTION_SUPPORTED_CAPABILITIES` | Capability hints for the custom picker entry. |

### Interaction with the built-in catalog

User-added models **merge** with the built-in catalog rather than replacing it:

- `ANTHROPIC_CUSTOM_MODEL_OPTION` adds a single entry at the bottom of the `/model` picker.
- Gateway discovery (`CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY=1`) adds discovered `claude`/`anthropic`-prefixed IDs alongside built-ins, skipping exact duplicates and folding entries that resolve to the same model as a built-in alias.
- `ANTHROPIC_DEFAULT_*_MODEL` variables shadow the built-in alias targets (`fable`, `opus`, `sonnet`, `haiku`) but do not remove the aliases from the catalog.
- `modelOverrides` in `settings.json` rewrites Anthropic IDs to provider-specific strings for the picker/wire, but again does not replace the catalog.

Because the built-in catalog self-updates with new Claude Code releases, a manual gateway-routed entry should be removed once the CLI natively lists that model. Claude Code does not automate this cleanup; users must delete the custom env var or settings entry themselves.

## Adding Local Models

Claude Code has **no native support** for local model runners and no OpenAI-compatible shim. A raw Ollama, LM Studio, llama.cpp, or vLLM endpoint cannot be used directly. The only path is to put an Anthropic-Messages-compatible gateway in front of the runner.

| Runner | Supported | Notes |
| :----- | :-------- | :---- |
| Ollama | Unsupported | Use only through a translating gateway. |
| oMLX | Unsupported | Use only through a translating gateway. |
| LM Studio | Unsupported | Use only through a translating gateway. |
| llama.cpp | Unsupported | Use only through a translating gateway. |
| vLLM | Unsupported | Use only through a translating gateway. |

### Practical example for a gateway-fronted Ollama runner

Suppose you run a local gateway that translates Anthropic Messages API requests to Ollama's API. Configuration is identical to a cloud gateway:

```bash
export ANTHROPIC_BASE_URL="http://localhost:8080/anthropic"
export ANTHROPIC_MODEL="ollama/gemma3:27b"
claude
```

The model ID string is whatever the gateway expects; it is not validated by Claude Code. Size and quantization tags such as `:27b` are part of the gateway's model namespace, not Claude Code's.

### Practical example for LM Studio

```bash
export ANTHROPIC_BASE_URL="http://localhost:1234/anthropic"
export ANTHROPIC_MODEL="lm-studio/qwen2.5-coder-32b-instruct"
claude
```

In both cases the runner itself must be hidden behind a gateway because Claude Code will send Anthropic Messages API requests to `ANTHROPIC_BASE_URL`.

## Environment Overrides

Environment variables take precedence over the corresponding `settings.json` field when both exist. The variables that redirect model endpoints or selection are:

| Variable | Effect | Precedence |
| :------- | :----- | :--------- |
| `ANTHROPIC_BASE_URL` | Redirect API endpoint to a gateway. | Overrides the default Anthropic API endpoint. |
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

Variables can be set either in the shell before launching `claude` or under the `env` key in a `settings.json` file. Settings-file `env` values persist across launches and are scoped by the file's location (user, project, local, or managed).

## Sources

- [Claude Code — Model configuration](https://code.claude.com/docs/en/model-config)
- [Claude Code — Settings](https://code.claude.com/docs/en/settings)
- [Claude Code — Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code — LLM gateway](https://code.claude.com/docs/en/llm-gateway)
- [Claude Code — Gateway protocol reference](https://code.claude.com/docs/en/llm-gateway-protocol)
- [Claude Code — CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code settings JSON Schema](https://json.schemastore.org/claude-code-settings.json)
