---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json
model_config_paths:
  - scope: user
    path: ~/.gemini/settings.json
    format: json
    notes: >-
      Global user settings. `model.name` pins the default model and
      `modelConfigs` declares custom aliases, overrides, and model definitions.
      Observed on this host with no `model.name` or `modelConfigs` block, so the
      CLI is using the default `auto` model selection.
  - scope: repo
    path: .gemini/settings.json
    format: json
    notes: Project-specific settings that override the user settings file.
  - scope: env
    path: .gemini/.env or ~/.gemini/.env
    format: other
    notes: >-
      Auto-loaded env file (project first, then ~/.gemini/.env). Used for
      `GEMINI_MODEL`, `GEMINI_API_KEY`, `GOOGLE_GEMINI_BASE_URL`,
      `GOOGLE_VERTEX_BASE_URL`, etc. The first file found wins; values are not
      merged.
  - scope: env
    path: Shell environment variables
    format: other
    notes: Session-scoped overrides such as `GEMINI_MODEL`, API credentials, and gateway base URLs.
api_standards:
  - standard: bespoke
    base_url_site: GOOGLE_GEMINI_BASE_URL / GOOGLE_VERTEX_BASE_URL env vars (no settings.json key)
    auth_site: GEMINI_API_KEY, GOOGLE_API_KEY, GOOGLE_APPLICATION_CREDENTIALS, or OAuth
    notes: >-
      The CLI uses the Google GenAI SDK and speaks the Gemini API (or Vertex AI).
      A "gateway" auth mode lets users redirect to a custom Gemini API-compatible
      base URL via `GOOGLE_GEMINI_BASE_URL`. There is no OpenAI-compatible or
      Anthropic-compatible adapter in the shipping CLI.
metadata_overrides:
  - displayName
  - tier
  - family
  - isPreview
  - isVisible
  - dialogDescription
  - features.thinking
  - features.multimodalToolUse
  - modelConfig.model
  - modelConfig.generateContentConfig.temperature
  - modelConfig.generateContentConfig.topP
  - modelConfig.generateContentConfig.topK
  - modelConfig.generateContentConfig.maxOutputTokens
  - modelConfig.generateContentConfig.thinkingConfig
merge_semantics: merge
local_runners:
  - runner: ollama
    integration: proxy_required
    standard: bespoke
    example: >-
      A Gemini API-compatible translating proxy must sit between Gemini CLI and
      Ollama. `export GOOGLE_GEMINI_BASE_URL=http://localhost:8080` and
      `export GEMINI_API_KEY=ollama`, then `gemini --model qwen3:1.7b`. The proxy
      translates Gemini API requests to Ollama's OpenAI-compatible
      `/v1/chat/completions` endpoint.
    notes: >-
      Ollama serves OpenAI- and Anthropic-compatible endpoints, but Gemini CLI
      does not speak those standards. A direct base URL to Ollama will not work.
  - runner: omlx
    integration: proxy_required
    standard: bespoke
    example: >-
      `export GOOGLE_GEMINI_BASE_URL=http://localhost:8080` and
      `export GEMINI_API_KEY=<omlx-key-or-ignored>`, then
      `gemini --model Qwen3.6-35B-A3B-oQ6`. A Gemini-to-OpenAI/Anthropic
      translating proxy is required.
    notes: >-
      oMLX serves OpenAI- and Anthropic-compatible endpoints, but Gemini CLI
      only emits the Gemini API. A direct base URL to oMLX will not work.
  - runner: lmstudio
    integration: proxy_required
    standard: bespoke
    example: >-
      `export GOOGLE_GEMINI_BASE_URL=http://localhost:8080` and
      `export GEMINI_API_KEY=lmstudio`, then
      `gemini --model openai/gpt-oss-20b`. A Gemini-to-OpenAI/Anthropic
      translating proxy is required.
    notes: >-
      LM Studio serves OpenAI- and Anthropic-compatible endpoints, but Gemini CLI
      does not speak those standards. A direct base URL to LM Studio will not work.
  - runner: llamacpp
    integration: proxy_required
    standard: bespoke
    example: >-
      `export GOOGLE_GEMINI_BASE_URL=http://localhost:8080` and
      `export GEMINI_API_KEY=<key-if-set>`, then
      `gemini --model gemma-3-1b-it.Q4_K_M.gguf`. A Gemini-to-OpenAI/Anthropic
      translating proxy is required.
    notes: >-
      llama-server serves OpenAI- and Anthropic-compatible endpoints, but Gemini
      CLI only emits the Gemini API. A direct base URL to llama-server will not
      work.
  - runner: vllm
    integration: proxy_required
    standard: bespoke
    example: >-
      `export GOOGLE_GEMINI_BASE_URL=http://localhost:8080` and
      `export GEMINI_API_KEY=EMPTY`, then
      `gemini --model Qwen/Qwen2.5-1.5B-Instruct`. A Gemini-to-OpenAI/Anthropic
      translating proxy is required.
    notes: >-
      vLLM serves OpenAI- and Anthropic-compatible endpoints, but Gemini CLI does
      not speak those standards. A direct base URL to vLLM will not work.
  - runner: other
    integration: first_class
    standard: bespoke
    example: >-
      { "experimental": { "gemmaModelRouter": { "enabled": true,
      "classifier": { "host": "http://localhost:9379", "model":
      "gemma3-1b-gpu-custom" } } } }
    notes: >-
      Local Gemma via LiteRT-LM is supported only as an experimental routing
      classifier, not as a general chat model. Setup is normally done with
      `gemini gemma setup`.
cloud_bridge:
  supported: true
  mechanism: GOOGLE_GEMINI_BASE_URL env var (or GOOGLE_VERTEX_BASE_URL for Vertex-compatible gateways); optionally set security.auth.selectedType to "gateway" for ACP/IDE mode
  example: |
    # Route Gemini CLI through a Gemini API-compatible gateway that proxies to a non-Google API
    export GOOGLE_GEMINI_BASE_URL="https://my-gateway.example.com/gemini"
    export GEMINI_API_KEY="gateway-key"
    gemini --model "openai/gpt-4.1"

    # If the target vendor's API is not Gemini-compatible (e.g. native OpenAI),
    # the gateway must translate the Gemini API to that vendor's protocol.
    # A direct base URL such as https://api.openai.com will not work.
default_model_site: 'model.name key in ~/.gemini/settings.json (or .gemini/settings.json); session override via GEMINI_MODEL env var or --model flag; precedence is --model > GEMINI_MODEL > model.name'
env_vars:
  - name: GEMINI_MODEL
    effect: Sets the active model for the launched session.
  - name: GEMINI_API_KEY
    effect: Authenticates to the Gemini API (Google AI Studio key).
  - name: GOOGLE_API_KEY
    effect: Authenticates to Google Cloud/Vertex AI requests.
  - name: GOOGLE_CLOUD_PROJECT
    effect: Specifies the Google Cloud project for Vertex AI or OAuth.
  - name: GOOGLE_CLOUD_PROJECT_ID
    effect: Fallback project ID when GOOGLE_CLOUD_PROJECT is not set.
  - name: GOOGLE_CLOUD_LOCATION
    effect: Specifies the Vertex AI region/location.
  - name: GOOGLE_APPLICATION_CREDENTIALS
    effect: Path to a service account JSON key for ADC-based Vertex auth.
  - name: GOOGLE_GENAI_USE_VERTEXAI
    effect: Switches the SDK to the Vertex AI endpoint.
  - name: GOOGLE_GEMINI_BASE_URL
    effect: Redirects the Gemini API endpoint to a custom gateway. Triggers the "gateway" auth mode.
  - name: GOOGLE_VERTEX_BASE_URL
    effect: Redirects the Vertex AI endpoint to a custom gateway.
  - name: GEMINI_API_KEY_AUTH_MECHANISM
    effect: 'Controls API-key header style: `x-goog-api-key` (default) or `bearer`.'
  - name: GOOGLE_GENAI_API_VERSION
    effect: Overrides the Google GenAI SDK API version string.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system defaults settings file.
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system override settings file.
changes:
  - 'Discovered a gateway auth mode in Gemini CLI 0.46.0 driven by GOOGLE_GEMINI_BASE_URL (and GOOGLE_VERTEX_BASE_URL). This enables custom Gemini API-compatible endpoints, but it is not an OpenAI/Anthropic base-URL override.'
  - 'Confirmed that the interactive CLI auth picker does not expose gateway; it is env/ACP-driven. Persistent selection can be recorded via security.auth.selectedType: "gateway" when an ACP/IDE client initializes the session.'
  - 'Reclassified local runners (Ollama, oMLX, LM Studio, llama.cpp, vLLM) as proxy_required via a Gemini API-compatible translating proxy, not unsupported. A direct base-URL override onto their OpenAI/Anthropic endpoints does not work because Gemini CLI has no such client.'
  - 'Added GEMINI_API_KEY_AUTH_MECHANISM and GOOGLE_GENAI_API_VERSION to the environment-override list; both affect how the SDK sends credentials and which API version is requested.'
  - 'Updated modelConfigs coverage: the schema now also documents aliases, customAliases, overrides, customOverrides, modelDefinitions, modelIdResolutions, classifierIdResolutions, and modelChains.'
requires_claudine_update: true
reason: >-
  Claudine should recognize GOOGLE_GEMINI_BASE_URL / GOOGLE_VERTEX_BASE_URL as
  Gemini CLI endpoint overrides, model the gateway auth type in provider
  metadata, and classify local runners as proxy_required through a
  Gemini-compatible translating proxy rather than unsupported.
---

# Gemini CLI User-Side Model Configuration

## Introduction to Gemini CLI Model Configuration

Gemini CLI stores persistent configuration in JSON `settings.json` files layered
by scope. The model-related parts are the top-level `model.name` key and the
`modelConfigs` object.

| Scope | Path | Format | Effect |
| :---- | :--- | :----- | :----- |
| System defaults | `/etc/gemini-cli/system-defaults.json` (Linux), `/Library/Application Support/GeminiCli/system-defaults.json` (macOS), `C:\ProgramData\gemini-cli\system-defaults.json` (Windows) | JSON | Lowest precedence base layer; path overridable via `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`. |
| User | `~/.gemini/settings.json` | JSON | Applies to all sessions for the current user. |
| Project | `.gemini/settings.json` | JSON | Applies only when running from that project directory. |
| System override | `/etc/gemini-cli/settings.json` (Linux), `/Library/Application Support/GeminiCli/settings.json` (macOS), `C:\ProgramData\gemini-cli\settings.json` (Windows) | JSON | Highest-precedence settings file; path overridable via `GEMINI_CLI_SYSTEM_SETTINGS_PATH`. |
| Env / dotenv | `.gemini/.env` or `~/.gemini/.env` | env | First file found wins; not merged. |

A formal JSON Schema exists for `settings.json`:
[settings.schema.json](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json).
The docs note that the schema may lag the latest release, so a validation
warning on a newly documented field does not always mean the config is invalid.

The actual host `~/.gemini/settings.json` on this machine contains the usual
top-level buckets (`general`, `security`, `tools`, `ui`) but no `model.name` or
`modelConfigs` block, so the CLI is using the default `auto` model selection.

## Adding Cloud Models

Gemini CLI is a first-party Google model client. Out of the box it talks to the
Gemini API or Vertex AI. The only way to point it at a different cloud model is
to use the **gateway** auth mode, which redirects the Google GenAI SDK to a
custom base URL that must speak the **Gemini API** protocol.

To use a Google model that is not in the built-in alias list, add a
`modelConfigs` block with a `customAliases` entry and, if necessary, a
`modelDefinitions` entry.

### Concrete example

```json
{
  "model": {
    "name": "my-gemini-3-pro"
  },
  "modelConfigs": {
    "customAliases": {
      "my-gemini-3-pro": {
        "extends": "chat-base-3",
        "modelConfig": {
          "model": "gemini-3-pro-preview"
        }
      }
    },
    "modelDefinitions": {
      "gemini-3-pro-preview": {
        "displayName": "Gemini 3 Pro Preview",
        "tier": "pro",
        "family": "gemini-3",
        "isPreview": true,
        "isVisible": true,
        "features": {
          "thinking": true,
          "multimodalToolUse": true
        }
      }
    }
  }
}
```

To redirect requests to a custom Gemini API-compatible gateway, use environment
variables (there is no `baseUrl` key in `settings.json`):

```bash
export GOOGLE_GEMINI_BASE_URL="https://my-gateway.example.com/gemini"
export GEMINI_API_KEY="gateway-key"
gemini --model my-gemini-3-pro
```

### What each piece means

| Key | Effect |
| :-- | :----- |
| `model.name` / `--model` / `GEMINI_MODEL` | The string used as the requested model. If it matches a `customAliases` key, that alias is resolved. |
| `customAliases.<name>.extends` | Inherits from another alias (built-in or custom). |
| `customAliases.<name>.modelConfig.model` | The concrete model ID passed to the Google GenAI SDK (and onward to the gateway). |
| `modelDefinitions.<id>` | Registry entry that tells the CLI how to display the model and what capabilities it has. |
| `GOOGLE_GEMINI_BASE_URL` | Env-only override that switches the SDK to gateway mode and points it at a custom Gemini API-compatible endpoint. |
| `GEMINI_API_KEY` | Credential sent to the gateway. The header style is controlled by `GEMINI_API_KEY_AUTH_MECHANISM` (`x-goog-api-key` or `bearer`). |

### Supported API standard

Gemini CLI speaks the **Google GenAI / Gemini API** (or Vertex AI). It does not
support OpenAI-compatible, Anthropic-compatible, or other bespoke provider APIs
natively.

| Aspect | How it is specified |
| :----- | :------------------ |
| Base URL | Hard-coded by the SDK based on auth mode, or overridden via `GOOGLE_GEMINI_BASE_URL` / `GOOGLE_VERTEX_BASE_URL`. Not configurable inside `settings.json`. |
| Auth | `GEMINI_API_KEY` (AI Studio / gateway), `GOOGLE_API_KEY` (Vertex/Cloud), `GOOGLE_APPLICATION_CREDENTIALS` (ADC service account), or OAuth sign-in. |
| Adapter | None. The gateway must implement the Gemini API protocol. |

### Per-model metadata

Users can declare the following metadata in `modelDefinitions`:

| Key | Meaning |
| :-- | :------ |
| `displayName` | Human-readable name in the UI. |
| `tier` | `pro`, `flash`, `flash-lite`, `custom`, or `auto`. |
| `family` | Model family string, e.g. `gemini-3`. |
| `isPreview` | Whether the model is a preview release. |
| `isVisible` | Whether the model appears in selection dialogs. |
| `dialogDescription` | Extra description shown in the model picker. |
| `features.thinking` | Whether the model supports thinking/reasoning output. |
| `features.multimodalToolUse` | Whether the model supports multimodal tool use. |

Generation parameters (`temperature`, `topP`, `topK`, `maxOutputTokens`,
`thinkingConfig`, etc.) go inside `modelConfig.generateContentConfig`, not in
`modelDefinitions`.

### Interaction with the built-in catalog

User-defined `customAliases`, `customOverrides`, and `modelDefinitions` are
**merged** with the built-in catalog. When the same key exists in both, the
user-defined entry wins (shadows the built-in).

Because the built-in catalog self-updates with new CLI releases, a manual
block for a model that later ships built-in becomes redundant. Gemini CLI does
not auto-remove these manual blocks; users must delete them from their
`settings.json`.

### Cross-cloud bridging

Gemini CLI can be routed at a different cloud vendor's API, but only by pointing
`GOOGLE_GEMINI_BASE_URL` at a **Gemini API-compatible gateway or proxy**. The
gateway is responsible for translating the Gemini API protocol to the upstream
vendor's native format.

```bash
# Route Gemini CLI through a gateway that forwards to a non-Google cloud API
export GOOGLE_GEMINI_BASE_URL="https://openai-gateway.example.com/gemini"
export GEMINI_API_KEY="gateway-token"
gemini --model "openai/gpt-4.1"
```

If the target vendor's native API is not Gemini-compatible (for example, native
OpenAI or Anthropic endpoints), a direct base URL will fail. You must run a
translating proxy that exposes the Gemini API to Gemini CLI and forwards to the
vendor's API on the back end.

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Gemini
CLI "knowing about" a runner. Gemini CLI's client only speaks the Gemini API, so
a runner that exposes OpenAI- or Anthropic-compatible endpoints cannot be used
directly. It can only be used through a translating proxy that exposes a
Gemini-compatible surface to Gemini CLI.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | Proxy required | Start Ollama, then run a Gemini-to-OpenAI/Anthropic proxy and point `GOOGLE_GEMINI_BASE_URL` at it. |
| oMLX | Proxy required | Start oMLX, then run a Gemini-to-OpenAI/Anthropic proxy and point `GOOGLE_GEMINI_BASE_URL` at it. |
| LM Studio | Proxy required | Start LM Studio server, then run a Gemini-to-OpenAI/Anthropic proxy and point `GOOGLE_GEMINI_BASE_URL` at it. |
| llama.cpp | Proxy required | Start `llama-server`, then run a Gemini-to-OpenAI/Anthropic proxy and point `GOOGLE_GEMINI_BASE_URL` at it. |
| vLLM | Proxy required | Start `vllm serve`, then run a Gemini-to-OpenAI/Anthropic proxy and point `GOOGLE_GEMINI_BASE_URL` at it. |
| Gemma via LiteRT-LM | First-class (experimental) | Supported only as a routing classifier via `experimental.gemmaModelRouter`, not as a general chat model. |

### Practical example for Ollama

```bash
# A Gemini API-compatible proxy is listening on :8080 and translating to
# Ollama's OpenAI-compatible /v1/chat/completions endpoint.
export GOOGLE_GEMINI_BASE_URL="http://localhost:8080"
export GEMINI_API_KEY="ollama"
gemini --model "qwen3:1.7b"
```

The model ID string is whatever the runner accepts; size and quantization tags
such as `:1.7b` are part of the runner's model namespace.

### Practical example for LM Studio

```bash
export GOOGLE_GEMINI_BASE_URL="http://localhost:8080"
export GEMINI_API_KEY="lmstudio"
gemini --model "openai/gpt-oss-20b"
```

Again, `http://localhost:8080` must be a Gemini API-compatible proxy that
translates to LM Studio's OpenAI- or Anthropic-compatible server endpoint. A
direct `GOOGLE_GEMINI_BASE_URL=http://localhost:1234` will not work.

### Local Gemma router example

This configures a local Gemma 3 1B model served by LiteRT-LM to make routing
decisions (simple requests → Flash, complex requests → Pro):

```json
{
  "experimental": {
    "gemmaModelRouter": {
      "enabled": true,
      "classifier": {
        "host": "http://localhost:9379",
        "model": "gemma3-1b-gpu-custom"
      }
    }
  }
}
```

Set up is normally performed with:

```bash
gemini gemma setup
```

The local server must expose the **Gemini API** (`/v1beta/models/...:generateContent`),
not the OpenAI Chat Completions API. If the local router is down, the CLI
silently falls back to the cloud classifier.

## Environment Overrides

Environment variables take precedence over `settings.json` values when the same
setting is expressed in both. Model selection follows this order:

1. `--model` command-line flag
2. `GEMINI_MODEL` environment variable
3. `model.name` in `settings.json`
4. Local Gemma router (if enabled)
5. Default `auto` model

| Variable | Effect |
| :------- | :----- |
| `GEMINI_MODEL` | Sets the active model for the session. |
| `GEMINI_API_KEY` | AI Studio / gateway API key for the Gemini API. |
| `GOOGLE_API_KEY` | Google Cloud/Vertex AI API key. |
| `GOOGLE_CLOUD_PROJECT` | Google Cloud project for Vertex/OAuth. |
| `GOOGLE_CLOUD_PROJECT_ID` | Fallback project ID. |
| `GOOGLE_CLOUD_LOCATION` | Vertex AI region/location. |
| `GOOGLE_APPLICATION_CREDENTIALS` | Service account JSON for ADC. |
| `GOOGLE_GENAI_USE_VERTEXAI` | Switches the SDK to Vertex AI. |
| `GOOGLE_GEMINI_BASE_URL` | Redirects the Gemini API endpoint to a gateway; triggers gateway auth mode. |
| `GOOGLE_VERTEX_BASE_URL` | Redirects the Vertex AI endpoint to a gateway. |
| `GEMINI_API_KEY_AUTH_MECHANISM` | API-key header style: `x-goog-api-key` (default) or `bearer`. |
| `GOOGLE_GENAI_API_VERSION` | Overrides the Google GenAI SDK API version. |
| `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` | Override system defaults file path. |
| `GEMINI_CLI_SYSTEM_SETTINGS_PATH` | Override system override file path. |

## Changelog

- **2026-07-02** — Documented the gateway auth mode driven by `GOOGLE_GEMINI_BASE_URL` / `GOOGLE_VERTEX_BASE_URL`. This enables custom Gemini API-compatible endpoints but is not an OpenAI/Anthropic base-URL override.
- **2026-07-02** — Reclassified local runners as `proxy_required` through a Gemini-compatible translating proxy; a direct base-URL override onto their OpenAI/Anthropic endpoints does not work.
- **2026-07-02** — Added `GEMINI_API_KEY_AUTH_MECHANISM` and `GOOGLE_GENAI_API_VERSION` to the environment-override list.
- **2026-07-02** — Expanded `modelConfigs` coverage to include `aliases`, `customAliases`, `overrides`, `customOverrides`, `modelDefinitions`, `modelIdResolutions`, `classifierIdResolutions`, and `modelChains`.

## Sources

- [Gemini CLI — Configuration reference](https://www.geminicli.com/docs/reference/configuration/)
- [Gemini CLI — Advanced Model Configuration](https://www.geminicli.com/docs/cli/generation-settings/)
- [Gemini CLI — Model routing](https://www.geminicli.com/docs/cli/model-routing/)
- [Gemini CLI — Model selection (`/model`)](https://www.geminicli.com/docs/cli/model/)
- [Gemini CLI — `gemini gemma` local routing setup](https://www.geminicli.com/docs/core/gemma-setup/)
- [Gemini CLI — Manual local model routing](https://www.geminicli.com/docs/core/local-model-routing/)
- [Gemini CLI — Authentication](https://www.geminicli.com/docs/get-started/authentication/)
- [Gemini CLI — CLI commands](https://www.geminicli.com/docs/reference/commands/)
- [Gemini CLI — FAQ](https://www.geminicli.com/docs/resources/faq/)
- [Gemini CLI settings JSON Schema](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
