---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
has_official_schema: informal
model_config_paths:
  - scope: user
    path: ~/.gemini/antigravity-cli/settings.json
    format: json
    notes: >-
      Persistent CLI settings. The observed host file contains `model:
      "Gemini 3.1 Pro (High)"`, `enableTelemetry`, and `trustedWorkspaces`.
      Official docs describe this as the plain-JSON settings file and the CLI
      changelog says `--model` and the `models` subcommand were added in 1.0.5.
      No observed or documented key adds custom providers, base URLs, API keys,
      or per-model metadata.
api_standards: []
metadata_overrides: []
merge_semantics: unknown
local_runners:
  - runner: ollama
    integration: unsupported
    notes: >-
      Ollama serves OpenAI- and Anthropic-compatible endpoints, but Antigravity
      CLI 1.1.0 exposes no user base-URL override or custom-provider adapter for
      either standard.
  - runner: omlx
    integration: unsupported
    notes: >-
      oMLX serves OpenAI- and Anthropic-compatible endpoints, but Antigravity
      CLI 1.1.0 exposes no user base-URL override or custom-provider adapter for
      either standard.
  - runner: lmstudio
    integration: unsupported
    notes: >-
      LM Studio serves OpenAI- and Anthropic-compatible endpoints, but
      Antigravity CLI 1.1.0 exposes no user base-URL override or custom-provider
      adapter for either standard.
  - runner: llamacpp
    integration: unsupported
    notes: >-
      llama.cpp's `llama-server` serves OpenAI- and Anthropic-compatible
      endpoints, but Antigravity CLI 1.1.0 exposes no user base-URL override or
      custom-provider adapter for either standard.
  - runner: vllm
    integration: unsupported
    notes: >-
      vLLM serves OpenAI- and Anthropic-compatible endpoints, but Antigravity CLI
      1.1.0 exposes no user base-URL override or custom-provider adapter for
      either standard.
cloud_bridge:
  supported: false
  mechanism: No documented or observed custom-provider, base-URL, or adapter mechanism in Antigravity CLI 1.1.0.
  example: >-
    Not available. Use `agy --model "<catalog model>"` or set
    `{"model":"<catalog model>"}` in `~/.gemini/antigravity-cli/settings.json`;
    there is no supported block for routing to another cloud vendor's native or
    OpenAI-compatible API.
default_model_site: '`model` key in ~/.gemini/antigravity-cli/settings.json; session override via `agy --model "<catalog model>"`; interactive selection via `/model`; list catalog-visible models with `agy models` after sign-in.'
env_vars:
  - name: AGY_CLI_DISABLE_LATEX
    effect: Disables LaTeX rendering; not a model endpoint or selection override.
  - name: AGY_CLI_HIDE_ACCOUNT_INFO
    effect: Hides account/plan details in the header; not a model endpoint or selection override.
  - name: AGY_CLI_CMD_OUTPUT_PERCENTAGE
    effect: Sets command output height in the TUI; not a model endpoint or selection override.
changes: []
requires_claudine_update: true
reason: >-
  Claudine should model Antigravity CLI as a provider whose current user-side
  model configuration is catalog selection only: `settings.json` `model`,
  `--model`, `/model`, and `agy models`, with no supported custom cloud/local
  model extension path.
---

# Antigravity CLI User-Side Model Configuration

## Introduction to Antigravity Model Configuration

Antigravity CLI stores persistent CLI preferences in
`~/.gemini/antigravity-cli/settings.json`. The file is JSON. On this host, the
file exists and contains:

```json
{
  "enableTelemetry": false,
  "model": "Gemini 3.1 Pro (High)",
  "trustedWorkspaces": [
    "/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff",
    "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine"
  ]
}
```

The `model` key is the only observed model-related setting. The `agy --help`
output for version 1.1.0 exposes `--model` as "Model for the current CLI
session" and a `models` subcommand that lists available models after sign-in.
The changelog says `--model` and `agy models` were added in 1.0.5.

Antigravity also has shared state under `~/.gemini/config/`, including
`config.json` and `projects/*.json`, but the observed files do not contain model
provider configuration. The host also has `/Users/ken/.antigravity/argv.json`,
which is VS Code-style application argv configuration and does not configure
models.

No formal machine-readable JSON Schema for
`~/.gemini/antigravity-cli/settings.json` was found. Official documentation
describes the settings file and individual keys in prose, so the schema status
for this topic is `informal`.

## Adding Cloud Models

Antigravity CLI 1.1.0 does not expose a supported user-side mechanism for adding
cloud models outside its own catalog. There is no observed or documented
`providers`, `modelConfigs`, `openai-compatible`, `anthropic-compatible`,
`baseURL`, `apiKey`, `npm`, or adapter block for agent model routing.

The supported cloud-model operation is catalog selection:

```bash
agy models
agy --model "Gemini 3.1 Pro (High)"
```

Or persist the selected catalog model:

```json
{
  "model": "Gemini 3.1 Pro (High)"
}
```

That block belongs in `~/.gemini/antigravity-cli/settings.json`. The value is a
catalog display/model name, not a custom model definition.

The following is a common-looking shape, but it is not a supported Antigravity
CLI model configuration:

```json
{
  "providers": {
    "deepseek": {
      "baseURL": "https://api.deepseek.com/v1",
      "apiKey": "${DEEPSEEK_API_KEY}",
      "models": {
        "deepseek-chat": {
          "name": "DeepSeek Chat"
        }
      }
    }
  },
  "model": "deepseek/deepseek-chat"
}
```

Do not document that as working. Forum users have reported trying
OpenAI-compatible settings under `.antigravity/settings.json` and failing to
route Antigravity's agent to the custom endpoint. Perplexity's Antigravity
integration docs explicitly state that Antigravity does not currently expose a
custom-provider or BYOK setting to replace the built-in agent model.

Because user-added models are not supported, there is no confirmed metadata
surface for display name, context limit, output limit, price, modalities,
reasoning support, or per-model capability declarations. Merge behavior with the
built-in catalog is therefore unknown: users can select models from the live
catalog, but cannot merge or shadow catalog records with local definitions.

Cross-cloud bridging is not supported. There is no `OPENAI_BASE_URL`,
`ANTHROPIC_BASE_URL`, `GOOGLE_GEMINI_BASE_URL`, settings-file base URL, or
adapter key that redirects Antigravity CLI's own agent model client to another
cloud vendor. If a future Antigravity release adds such a mechanism, examples
must match the actual API standard the client speaks; a direct base URL to a
vendor's native API is not valid unless that API implements the expected
standard.

## Adding Local Models

Local-runner support is a property of API-standard bridging. The local-runner
ground-truth frontmatter says Ollama, oMLX, LM Studio, llama.cpp, and vLLM all
serve OpenAI-compatible endpoints, and all five also expose an
Anthropic-compatible `/v1/messages` path.

Antigravity CLI 1.1.0 does not expose a base-URL override for either standard.
That makes the current classification genuinely unsupported, not merely
"not first-class":

| Runner | Runner API standards | Antigravity path | Classification |
| :--- | :--- | :--- | :--- |
| Ollama | OpenAI-compatible at `http://localhost:11434/v1`; Anthropic-compatible at `http://localhost:11434` | No Antigravity base-URL override or adapter | Unsupported |
| oMLX | OpenAI-compatible at `http://localhost:8000/v1`; Anthropic-compatible at `http://localhost:8000` | No Antigravity base-URL override or adapter | Unsupported |
| LM Studio | OpenAI-compatible at `http://localhost:1234/v1`; Anthropic-compatible at `http://localhost:1234` | No Antigravity base-URL override or adapter | Unsupported |
| llama.cpp | OpenAI-compatible at `http://localhost:8080/v1`; Anthropic-compatible at `http://localhost:8080` | No Antigravity base-URL override or adapter | Unsupported |
| vLLM | OpenAI-compatible at `http://localhost:8000/v1`; Anthropic-compatible at `http://localhost:8000` | No Antigravity base-URL override or adapter | Unsupported |

If Antigravity later adds OpenAI-compatible configuration, local model IDs would
normally be the IDs returned by the runner's model-list endpoint, for example
`qwen3:1.7b` on Ollama, `openai/gpt-oss-20b` in LM Studio, a llama.cpp served
alias or GGUF filename, or a vLLM `--served-model-name`. As of 1.1.0 there is no
place to put those IDs in Antigravity as custom local model definitions.

## Environment Overrides

No environment variable was found that redirects Antigravity CLI's agent model
endpoint or adds a model provider. The observed and documented environment
variables are UI/runtime controls:

| Variable | Effect |
| :--- | :--- |
| `AGY_CLI_DISABLE_LATEX` | Disables LaTeX rendering. |
| `AGY_CLI_HIDE_ACCOUNT_INFO` | Hides email and plan tier in the header. |
| `AGY_CLI_CMD_OUTPUT_PERCENTAGE` | Adjusts maximum command-output height in the TUI. |

Model selection is controlled by `--model`, `/model`, and the persisted
`model` key in `~/.gemini/antigravity-cli/settings.json`. `agy models` requires
sign-in on this host; without sign-in it returns "Please sign in to view
available models."

## Sources

- [Antigravity CLI README](https://github.com/google-antigravity/antigravity-cli/blob/main/README.md)
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [Antigravity CLI settings documentation](https://antigravity.google/docs/cli-settings)
- [Antigravity CLI reference documentation](https://antigravity.google/docs/cli-reference)
- [Antigravity models documentation](https://antigravity.google/docs/models)
- [Perplexity with Google Antigravity](https://docs.perplexity.ai/docs/getting-started/integrations/antigravity)
- [Google AI Developers Forum: custom OpenAI-compatible models in Antigravity IDE](https://discuss.ai.google.dev/t/how-to-properly-configure-custom-openai-compatible-models-in-antigravity-ide/168654)
- Local Antigravity CLI 1.1.0 inspection: `agy --help`, `agy models`, `/Users/ken/.gemini/antigravity-cli/settings.json`, `/Users/ken/.gemini/config/config.json`, and `/Users/ken/.antigravity/argv.json`.
- Local runner ground truth: `claudine/docs/research/local_runners/ollama.md`, `omlx.md`, `lmstudio.md`, `llamacpp.md`, and `vllm.md`.
