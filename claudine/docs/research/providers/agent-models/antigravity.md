---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
has_official_schema: informal
schema_url: https://codelabs.developers.google.com/antigravity-cli-hands-on
default_models:
  - id: gemini-3.5-flash
    is_default: true
    notes: "Default value documented by the bundled Antigravity CLI guide for settings.json; exact reasoning-tier resolution is unknown."
  - id: Gemini 3.5 Flash (Medium)
  - id: Gemini 3.5 Flash (High)
  - id: Gemini 3.5 Flash (Low)
  - id: Gemini 3.1 Pro (Low)
  - id: Gemini 3.1 Pro (High)
    notes: "Persisted locally in /Users/ken/.gemini/antigravity-cli/settings.json and propagated to the backend as label=\"Gemini 3.1 Pro (High)\"."
  - id: Claude Sonnet 4.6 (Thinking)
  - id: Claude Opus 4.6 (Thinking)
  - id: GPT-OSS 120B (Medium)
model_selection:
  - method: cli_flag
    site: --model
    example: agy --model "Gemini 3.5 Flash (Low)"
    notes: "Launch-time, session-only selection documented by the Google codelab and present in agy --help. No short alias was shown by help."
  - method: config_file
    site: model
    example: '"model": "Gemini 3.1 Pro (High)"'
    notes: "Key in ~/.gemini/antigravity-cli/settings.json. Local logs show the persisted value propagated to the backend when the launch flag model is empty."
  - method: interactive_command
    site: /model
    example: /model
    notes: "Runtime command for changing or inspecting the active model in the session."
  - method: wire_envelope
    site: unknown
    example: unknown
    notes: "Logs show a backend selected-model override is propagated as label=\"Gemini 3.1 Pro (High)\", but the request field name is not observable from local logs."
precedence: "runtime_interactive_command > cli_flag > config_file; no model env var found. Evidence is mixed documented/observed: --model is documented as session-only, logs show config is used when launch model is empty, and /model changes the active runtime model."
dynamic_listing:
  available: true
  list_program: agy
  list_args:
    - models
  method: "Authenticated CLI subcommand"
  example: agy models
changes: []
requires_claudine_update: true
reason: "Antigravity has a stock model catalog and dynamic listing surface not currently represented in Claudine's generated provider/model ground-truth data."
---

# Antigravity CLI Model Surface

## Models Available

A stock Antigravity CLI install exposes its model list dynamically after authentication. On this host, `agy models` is present but returned `Error: Please sign in to view available models. Launch the CLI without arguments to sign in.` The local install therefore confirms the listing mechanism, not a live authenticated catalog snapshot.

The Google codelab documents the following sample `agy models` output and says those names can be passed to `--model`. These strings are treated as exact selectable model labels for the stock CLI:

| Exact selectable string | Alias or internal ID | Context window | Notes |
| --- | --- | --- | --- |
| `Gemini 3.5 Flash (Medium)` | unknown | unknown | Listed by the codelab sample output. |
| `Gemini 3.5 Flash (High)` | unknown | unknown | Listed by the codelab sample output and shown in the codelab `settings.json` example. |
| `Gemini 3.5 Flash (Low)` | unknown | unknown | Listed by the codelab sample output and used in the codelab `--model` example. |
| `Gemini 3.1 Pro (Low)` | unknown | unknown | Listed by the codelab sample output. |
| `Gemini 3.1 Pro (High)` | `MODEL_PLACEHOLDER_M16` in local IDE state | unknown | Persisted locally in `/Users/ken/.gemini/antigravity-cli/settings.json`; local CLI logs propagate `label="Gemini 3.1 Pro (High)"` to the backend. |
| `Claude Sonnet 4.6 (Thinking)` | unknown | unknown | Listed by the codelab sample output. |
| `Claude Opus 4.6 (Thinking)` | unknown | unknown | Listed by the codelab sample output. |
| `GPT-OSS 120B (Medium)` | `MODEL_OPENAI_GPT_OSS_120B_MEDIUM` appears in the local binary | unknown | Listed by the codelab sample output. |

The bundled Antigravity guide installed at `/Users/ken/.gemini/antigravity/builtin/skills/antigravity_guide/references/cli.md` documents the `settings.json` `model` key default as `gemini-3.5-flash`. That appears to be a family/default alias rather than one of the display labels in the codelab sample. The exact resolution of `gemini-3.5-flash` to Low/Medium/High is unknown and may vary by account type or backend policy.

Local state under `/Users/ken/.antigravity` contained IDE/extensions data but no model catalog or selection cache. The active user CLI state was under `/Users/ken/.gemini/antigravity-cli/`; it contained a persisted model but no readable `availableModels` cache.

## Model Selection

| Mechanism | Site | Example | Scope and persistence | Evidence |
| --- | --- | --- | --- | --- |
| CLI flag | `--model` | `agy --model "Gemini 3.5 Flash (Low)"` | Session-only launch override. The codelab explicitly describes it as using a specific model "during its session only." | `agy --help`; Google codelab. |
| Config file key | `model` | `"model": "Gemini 3.1 Pro (High)"` in `~/.gemini/antigravity-cli/settings.json` | Persisted default. Direct edits require restart according to the codelab; `/settings` and `/config` are the supported TUI editors. | Google codelab, bundled Antigravity CLI guide, and local `/Users/ken/.gemini/antigravity-cli/settings.json`. |
| Interactive command | `/model` | `/model` | Runtime active-model switch or inspection command. Persistence behavior is unknown; local history records `/model`, but the unauthenticated session could not complete a switch. | Bundled Antigravity CLI guide; codelab says to check the selected model with `/model`. |
| Wire/backend field | unknown | unknown | The local logs show a backend override is propagated as `label="Gemini 3.1 Pro (High)"` after `v1internal:fetchAvailableModels`, but the request envelope field name is not observable from logs. | Local `/Users/ken/.gemini/antigravity-cli/log/*.log`. |

No model-selection environment variable was found in `agy --help`, the bundled guide, local JSON settings, or targeted binary-string searches. The binary contains unrelated environment variables such as `AGY_CLI_CMD_OUTPUT_PERCENTAGE`, `AGY_CLI_HIDE_ACCOUNT_INFO`, `AGY_BUSINESS_PAYGO_TIER`, and sidecar variables, but no documented model override variable.

Highest-wins precedence:

1. Runtime interactive command (`/model`) for the currently active session.
2. Launch flag (`--model`) for the session being started.
3. Persisted config file key (`model`) in `~/.gemini/antigravity-cli/settings.json`.

Evidence: the codelab documents `--model` as a session-only launch override; the local logs show print mode started with `model=""` and then propagated the persisted config label to the backend; the bundled guide documents `/model` as changing the active model during the session. The exact persistence of `/model` is unknown.

On session resume, model behavior is unknown. `agy --help` exposes `--continue` and `--conversation`, but the local unauthenticated install could not be used to observe whether a resumed conversation keeps its previous model or re-applies current config.

## Configuration Schema

No formal machine-readable model-configuration schema was found.

Antigravity publishes an informal documented shape for `~/.gemini/antigravity-cli/settings.json` in the Google codelab, including a sample with `"model": "Gemini 3.5 Flash (High)"`. The bundled Antigravity guide installed with the product also documents `settings.json` keys and describes `model` as a string with default `gemini-3.5-flash`.

Classification: `informal`.

## Dynamic Listing

Programmatic listing is available through an authenticated subcommand:

```bash
agy models
```

Unauthenticated local output:

```text
Error: Please sign in to view available models. Launch the CLI without arguments to sign in.
```

Documented sample output shape:

```text
Gemini 3.5 Flash (Medium)
Gemini 3.5 Flash (High)
Gemini 3.5 Flash (Low)
Gemini 3.1 Pro (Low)
Gemini 3.1 Pro (High)
Claude Sonnet 4.6 (Thinking)
Claude Opus 4.6 (Thinking)
GPT-OSS 120B (Medium)
```

Negative and supporting probes:

- `agy --help` lists `models` as `List available models`.
- `agy models --help` shows only `-h` and `--help`; no JSON or format flag was found.
- Local CLI logs show the backend fetch endpoint as `https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels`, but no public REST contract or response schema was found.
- No readable model catalog cache file was found under `/Users/ken/.antigravity`, `/Users/ken/.gemini/antigravity-cli`, or the Claudine shadow Antigravity home. Logs refer to `Cache(availableModels)`, but the cache content was not present as a simple JSON file.

## Extending the Model Set

- `~/.gemini/antigravity-cli/settings.json` key `gcp`: GCP project/location configuration channel for cloud-backed Antigravity access; model-config owns the setup details.
- `~/.gemini/config/mcp_config.json`: MCP server registration channel; this extends tools, not the stock model catalog.
- Plugins under `~/.gemini/config/` or managed by `agy plugin`: customization channel for skills/agents/sidecars; not an out-of-box model catalog mechanism.

No first-class local-runner or bespoke model registration key was found in the out-of-box CLI documentation checked for this topic.

## Sources

- [Antigravity CLI codelab](https://codelabs.developers.google.com/antigravity-cli-hands-on)
- [Antigravity CLI GitHub repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI README](https://raw.githubusercontent.com/google-antigravity/antigravity-cli/main/README.md)
- [Antigravity CLI changelog](https://raw.githubusercontent.com/google-antigravity/antigravity-cli/main/CHANGELOG.md)
- [Antigravity models documentation](https://antigravity.google/docs/models)
- [Antigravity CLI features documentation](https://antigravity.google/docs/cli-features)
