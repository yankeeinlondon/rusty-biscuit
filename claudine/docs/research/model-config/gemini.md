---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json
config_files:
  - scope: user
    path: ~/.gemini/settings.json
    format: json
    notes: >-
      Global user settings. `model.name` pins the default model and
      `modelConfigs` declares custom aliases, overrides, and model definitions.
      Observed on this host with `security.auth.selectedType: gemini-api-key`
      and `general.previewFeatures: true`.
  - scope: repo
    path: .gemini/settings.json
    format: json
    notes: Project-specific settings that override the user settings file.
  - scope: env
    path: .gemini/.env
    format: other
    notes: >-
      Auto-loaded env file (project first, then ~/.gemini/.env). Used for
      `GEMINI_API_KEY`, `GOOGLE_CLOUD_PROJECT`, `GEMINI_MODEL`, etc. The first
      file found wins; values are not merged.
  - scope: env
    path: Shell environment variables
    format: other
    notes: Session-scoped overrides such as `GEMINI_MODEL` and API credentials.
api_standards:
  - standard: bespoke
    base_url_site: Hard-coded Gemini API / Vertex endpoint (not user-configurable)
    auth_site: GEMINI_API_KEY, GOOGLE_API_KEY, GOOGLE_APPLICATION_CREDENTIALS, or OAuth
    notes: >-
      The CLI uses the Google GenAI SDK and speaks the Gemini API (or Vertex AI).
      There is no OpenAI-compatible or Anthropic-compatible adapter in the
      shipping CLI; community PRs for OpenAI-compatible providers were closed
      without merge.
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
  - generateContentConfig.temperature
  - generateContentConfig.topP
  - generateContentConfig.maxOutputTokens
  - generateContentConfig.thinkingConfig
merge_semantics: merge
local_runners:
  - runner: ollama
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim in the shipping CLI.
  - runner: omlx
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim in the shipping CLI.
  - runner: lmstudio
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim in the shipping CLI.
  - runner: llamacpp
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim in the shipping CLI.
  - runner: vllm
    supported: unsupported
    notes: No native integration and no OpenAI-compatible shim in the shipping CLI.
  - runner: other
    supported: native
    example: >-
      { "experimental": { "gemmaModelRouter": { "enabled": true,
      "classifier": { "host": "http://localhost:9379", "model":
      "gemma3-1b-gpu-custom" } } } }
    notes: >-
      Local Gemma via LiteRT-LM is supported only as an experimental routing
      classifier, not as a general chat model.
default_model_site: model.name key in ~/.gemini/settings.json (or .gemini/settings.json); session override via GEMINI_MODEL env var or --model flag
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
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system defaults settings file.
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system override settings file.
changes: []
requires_claudine_update: false
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

Gemini CLI is a first-party Google model client. The only cloud models it can
use are models served by the Gemini API or Vertex AI. There is no generic
"add a provider" plug-in, no `baseUrl` setting, and no OpenAI-compatible
adapter in the released CLI.

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

### What each piece means

| Key | Effect |
| :-- | :----- |
| `model.name` / `--model` / `GEMINI_MODEL` | The string used as the requested model. If it matches a `customAliases` key, that alias is resolved. |
| `customAliases.<name>.extends` | Inherits from another alias (built-in or custom). |
| `customAliases.<name>.modelConfig.model` | The concrete model ID passed to the Google GenAI SDK. |
| `modelDefinitions.<id>` | Registry entry that tells the CLI how to display the model and what capabilities it has. |

### Supported API standard

Gemini CLI speaks the **Google GenAI / Gemini API** (or Vertex AI). It does not
support OpenAI-compatible, Anthropic-compatible, or other bespoke provider APIs.

| Aspect | How it is specified |
| :----- | :------------------ |
| Base URL | Hard-coded by the SDK based on auth mode (Gemini API or Vertex). Not user-configurable. |
| Auth | `GEMINI_API_KEY` (AI Studio), `GOOGLE_API_KEY` (Vertex/Cloud), `GOOGLE_APPLICATION_CREDENTIALS` (ADC service account), or OAuth sign-in. |
| Adapter | None. |

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

Generation parameters (`temperature`, `topP`, `maxOutputTokens`,
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

## Adding Local Models

Gemini CLI has **no native support** for using Ollama, oMLX, LM Studio,
llama.cpp, or vLLM as the chat model, and no OpenAI-compatible shim is present
in the released CLI. The only local-model integration is the experimental
**Gemma model router**, which uses a local Gemma model only to classify and
route requests to cloud Gemini models.

| Runner | Supported | Notes |
| :----- | :-------- | :---- |
| Ollama | Unsupported | No integration; community PRs were closed without merge. |
| oMLX | Unsupported | No integration. |
| LM Studio | Unsupported | No integration. |
| llama.cpp | Unsupported | No integration. |
| vLLM | Unsupported | No integration. |
| Gemma via LiteRT-LM | Native (experimental) | Supported only as a routing classifier, not as a general chat model. |

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
| `GEMINI_API_KEY` | AI Studio API key for the Gemini API. |
| `GOOGLE_API_KEY` | Google Cloud/Vertex AI API key. |
| `GOOGLE_CLOUD_PROJECT` | Google Cloud project for Vertex/OAuth. |
| `GOOGLE_CLOUD_PROJECT_ID` | Fallback project ID. |
| `GOOGLE_CLOUD_LOCATION` | Vertex AI region/location. |
| `GOOGLE_APPLICATION_CREDENTIALS` | Service account JSON for ADC. |
| `GOOGLE_GENAI_USE_VERTEXAI` | Switches the SDK to Vertex AI. |
| `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` | Override system defaults file path. |
| `GEMINI_CLI_SYSTEM_SETTINGS_PATH` | Override system override file path. |

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
