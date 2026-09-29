---
$schema: ./_schema.yaml
schema_revision: 2
provider: goose
created: 2026-09-29
last_updated: 2026-09-29
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
versions_examined:
- 1.52.0
evidence:
- claim: v1.52.0 (released 2026-09-23) is the latest goose release at research time, and main is still at commit b92a80da, the commit this research examined; the version whose documentation, source, and behavior this document describes
  id: goose-releases
  limitations: release notes were scanned for reasoning-related changes but not exhaustively reviewed; goose is not installed on the research host so the version was not confirmed locally
  location: https://github.com/aaif-goose/goose/releases
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the Muse Spark section documents the global GOOSE_THINKING_EFFORT environment variable accepting off, low, medium, high, or max, a goose configure prompt offering Off/Low/Medium/High/Max, and that Max is sent as Meta's xhigh while Off is clamped to low because Muse Spark cannot disable reasoning
  id: docs-effort-variable
  limitations: the page does not state precedence against other controls or the behavior of GOOSE_THINKING_EFFORT on non-Muse models
  location: https://goose-docs.ai/docs/getting-started/providers#meta-muse-spark-reasoning-effort
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: Gemini 3 thinking levels are Low (default, lighter reasoning) and High (deeper reasoning), offered as a Desktop dropdown and a goose configure prompt, with documented priority request_params.thinking_level, then GEMINI3_THINKING_LEVEL, then the default low
  id: docs-gemini3-levels
  limitations: the page documents only low and high; it does not mention the medium level the source maps for non-pro Gemini 3 models, nor that GEMINI3_THINKING_LEVEL is now a legacy key folded into the unified effort
  location: https://goose-docs.ai/docs/getting-started/providers#gemini-3-thinking-levels
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the environment-variables guide documents CLAUDE_THINKING_TYPE (adaptive, enabled, disabled) for the Anthropic and Databricks providers with a stated default of adaptive for Claude 4.6+ models and disabled otherwise, and states GOOSE_CLI_SHOW_THINKING=1 is additionally required to see Claude thinking in the CLI; it also lists GEMINI3_THINKING_LEVEL (low, high) and the rule that environment variables take precedence over config file settings
  id: docs-claude-thinking
  limitations: the stated Claude 4.6 default conflicts with the source, which sends an explicit thinking disable for adaptive-mode Claude models when no effort is chosen; documentation is unversioned and may lag the release
  location: https://goose-docs.ai/docs/guides/environment-variables#claude-thinking-configuration
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: goose captures reasoning from DeepSeek-R1 and Kimi (reasoning_content field), Gemini (thinking blocks), and Claude (thinking blocks); the CLI hides it by default and shows it only when GOOSE_CLI_SHOW_THINKING is set, and then only when stdout is a terminal
  id: docs-viewing-reasoning
  limitations: the page does not describe the json and stream-json output formats, which deliver reasoning without a terminal
  location: https://goose-docs.ai/docs/getting-started/providers#viewing-model-reasoning
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: OPENROUTER_PARAMETERS in config.yaml (YAML object or JSON string) adds fields such as reasoning.effort to every OpenRouter chat completion request, while goose ignores reserved request fields it manages (model, messages, stream, stream_options)
  id: docs-openrouter-parameters
  limitations: the page does not state how a passthrough reasoning.effort interacts with goose's unified thinking effort
  location: https://goose-docs.ai/docs/getting-started/providers#openrouter-advanced-parameters
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: goose run and goose session accept --provider and --model overrides, the /model session command shows or switches the model or provider mid-session, --output-format accepts text, json, or stream-json, and the documented flag and slash-command sets contain no effort or thinking option
  id: docs-cli-commands
  limitations: the page does not document the effort suffix goose accepts on model names
  location: https://goose-docs.ai/docs/guides/goose-cli-commands
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: config.yaml lives at ~/.config/goose/config.yaml (macOS/Linux) or %APPDATA%\Block\goose\config\config.yaml (Windows), environment variables take precedence over config file settings, and goose info -v prints all active configuration values; GOOSE_THINKING_EFFORT is absent from the settings table
  id: docs-config-files
  limitations: the settings table not listing GOOSE_THINKING_EFFORT is a documentation gap, not evidence the key is unread; the source reads it from the same file
  location: https://goose-docs.ai/docs/guides/config-files
  method: official_docs
  observed_on: 2026-09-29
  version: 1.52.0
- claim: ThinkingEffort is the enum Off, Low, Medium, High, Max; FromStr accepts off, disabled, and none as Off, low as Low, medium and med as Medium, high as High, max and xhigh as Max (case-insensitive), rejecting anything else, and Display emits off, low, medium, high, max
  id: src-thinking-enum
  limitations: pinned to main commit b92a80da (2026-09-29), the direct successor of release 1.52.0; defines the type only, not per-model acceptance
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/thinking.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: a trailing -none, -low, -medium, -high, or -xhigh on an OpenAI Responses model (o-series, gpt-5, gpt-6) or an xAI reasoning model (grok-4.5, grok-4.3, grok-3-mini) is stripped from the model name and stored as request_params.thinking_effort (none to off, xhigh to max), but only when no explicit thinking_effort param already exists; the suffix is not stripped for other models, so claude-sonnet-4-high keeps its full name
  id: src-model-suffix
  limitations: does not show where GOOSE_THINKING_EFFORT is read or how the param reaches the wire
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/model.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: get_goose_thinking_effort reads the GOOSE_THINKING_EFFORT param with the environment variable checked before the config.yaml key, silently discards values that do not parse as ThinkingEffort, and then falls back to the legacy keys CLAUDE_THINKING_TYPE (adaptive or enabled to high, disabled to off), CLAUDE_THINKING_ENABLED (bool to high or off), and GEMINI3_THINKING_LEVEL (low or high), in that order
  id: src-config-effort-key
  limitations: covers only the config layer, not the ACP or provider paths
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/config/base.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: at session start the model config receives with_default_thinking_effort(get_goose_thinking_effort()), so the configured effort applies only when the model name carried no effort suffix and no explicit thinking_effort request param exists; a persisted raw value that does not parse (such as a harness default) is still an explicit pick and is never overwritten
  id: src-model-config-default
  limitations: does not describe what each wire format does when the effort remains unset
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/model_config.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: goose configure shows a Select thinking effort prompt with the items off (Off - No extended thinking), low (Low - Better latency, lighter reasoning), medium (Medium - Moderate thinking), high (High - Deep reasoning), and max (Max - No constraints on thinking depth), initial value off, whenever the chosen model is a reasoning model, and persists the choice with set_goose_thinking_effort, which writes GOOSE_THINKING_EFFORT to config.yaml
  id: src-configure-prompt
  limitations: the prompt appears only when model metadata marks the model as reasoning; the choice is a default for future sessions, not a live change
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/commands/configure.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the /model session command rebuilds the model config from the new model name through model_config_from_user_config, so an effort suffix in the name changes the session's effort, and the handler reports Session model switched from '<old>' to '<new>' for provider '<provider>'; switching is refused for ACP providers and providers that manage their own conversation context, and /model with no argument prints only the model and provider
  id: src-model-command
  limitations: the success message prints model names, which have the suffix already stripped, so it does not state the effort
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/mod.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the ACP method session/set_thinking_effort, served by goose serve and goose acp, forwards the raw effort id to Agent::update_thinking_effort for a running session
  id: src-acp-set-thinking-effort
  limitations: the method is an ACP protocol request, not a CLI flag; no local ACP client was available to exercise it
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/acp/server.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: 'update_thinking_effort either hands the raw value to a provider that manages its own effort (persisting it verbatim) or, on the legacy path, parses it as ThinkingEffort and rejects an unparseable value with the ProviderError::InvalidValue message Invalid thinking effort: <value>'
  id: src-agent-update-effort
  limitations: the provider-managed path accepts harness vocabularies that are not ThinkingEffort members; which providers implement it was not enumerated
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/agents/agent.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the ACP and Desktop effort menu offers only off for non-reasoning models, low, high, and max for GLM-5.3 and Kimi K3 reasoning models, and off, low, medium, high, max otherwise, with the current value defaulting to max for GLM-5.3 and Kimi K3 when nothing is configured and off otherwise
  id: src-effort-menus
  limitations: menu values, not wire parameters; a menu value can be advertised for a model whose serving format sends no effort field
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/acp/response_builder.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: for Anthropic-format providers an effort of low, medium, high, or max on an adaptive-mode Claude model sends thinking type adaptive plus output_config.effort; on other Claude reasoning models it sends thinking type enabled with budget_tokens 4000, 10000, 16000, or 32000; effort off or unset sends thinking type disabled; always-on adaptive models cannot disable and default to effort high; an explicit budget_tokens request param (minimum 1024) takes precedence over the effort-derived budget
  id: src-anthropic-format
  limitations: whether Anthropic accepts output_config.effort max on the wire was not verified; the explicit-disable-when-unset behavior contradicts the environment-variables guide's stated default
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/anthropic.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the canonical model registry marks Claude Opus 4.6, 4.7, 4.8, Opus 5, Sonnet 4.6, and Sonnet 5 as thinking_mode adaptive, and Claude Fable 5, Fable 5.1, and Opus 5.5 as thinking_mode always_on_adaptive, all with reasoning true
  id: src-canonical-thinking-modes
  limitations: the registry drives classification only; it does not itself define the wire mapping or the per-model effort lists of non-Claude providers
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/canonical/data/canonical_models.json
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: OpenAI Responses and xAI chat models receive reasoning_effort picked from a per-model supported list — gpt-5 and gpt-6 pro models only high; gpt-6 Sol and Luna none, low, medium, high, xhigh, max; gpt-6-astra low, medium, high, xhigh, max; gpt-5.4, gpt-5.5, and gpt-5.6 none, low, medium, high, xhigh; other gpt-5 and o-series low, medium, high; grok-4.5 low, medium, high; grok-4.3 none, low, medium, high; grok-3-mini low, high — with goose's effort clamped to the first preferred level the model supports, and models outside these families receiving no reasoning_effort at all
  id: src-openai-format
  limitations: the supported lists are goose's own compatibility table, not a live query of the provider
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/openai.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: Gemini 3 models map effort to generationConfig.thinkingConfig.thinkingLevel — low for low, low for medium on gemini-3-pro and medium on other Gemini 3 models, high for high and max — with includeThoughts true, while effort off sends no thinkingConfig so the API default applies; Gemini 3.5 and 3.6 map off to thinkingLevel minimal; Gemini 2.5 models use a thinking_budget request param with a default of 8192 and gemini-2.5-flash maps off to budget 0
  id: src-google-format
  limitations: the Gemini API's own default thinking level (documented as low) was taken from goose's docs, not verified against Google
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/google.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the Meta provider maps thinking effort onto Muse Spark's reasoning_effort as low for off and low, medium for medium, high for high, and xhigh for max, omitting the field entirely when no effort is chosen
  id: src-meta-mapping
  limitations: covers the openai-engine Meta provider; the subscription muse_code provider follows the Anthropic format's adaptive mapping instead
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-providers/src/openai.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the codex provider (ChatGPT Codex subscription, models gpt-5.2-codex, gpt-5.2, gpt-5.1-codex-max, gpt-5.1-codex-mini) passes -c model_reasoning_effort=<value> to the codex CLI with none, low, medium, high, or xhigh, defaults to high when no effort is set, and reads the legacy CODEX_REASONING_EFFORT config key when the unified effort is absent
  id: src-codex-provider
  limitations: applies only to the codex CLI provider; acceptance of each level by each codex model is the codex CLI's concern, not verified here
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/providers/codex.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: glm-5.3 and glm-5.3-flash are declared reasoning models on both the zai provider (Anthropic-compatible endpoint, where effort maps through the Anthropic format) and the zai_coding_plan provider (OpenAI-compatible chat endpoint)
  id: src-glm-definitions
  limitations: the OpenAI chat-completions format sends no effort field for GLM models, so the effort's wire effect on zai_coding_plan is unverified
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-providers/src/declarative/definitions/zai_coding_plan.json
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: 'CLI text mode renders model thinking under a dimmed Thinking: header only when the GOOSE_CLI_SHOW_THINKING param is truthy AND stdout is a terminal, so piped or captured text output contains no reasoning'
  id: src-cli-thinking-display
  limitations: covers the text renderer only
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/output.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: with --output-format stream-json every assistant message, including its Thinking and RedactedThinking content blocks, is emitted as a type:message NDJSON event on stdout, and with --output-format json the final payload's messages array (user_visible_messages, which keep Thinking blocks) is printed in full
  id: src-json-output
  limitations: the stream events carry no effort field; the effort lives on the session's model config, not on messages
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/mod.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: goose session export --session-id <id> --format json serializes the whole session record, including model_config with its request_params.thinking_effort, so the effort goose applied to a session can be read back after the run; the record lives in sessions.db under the goose data dir
  id: src-session-record
  limitations: the stored value is what goose applied to the model config, not a provider-side confirmation of the level actually used
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/commands/session.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: the CLI installs a request logger that writes every provider request and response body to llm_request.<N>.jsonl files under the goose state dir's logs directory, keeping the 10 most recent
  id: src-request-log
  limitations: the log holds the wire request, which shows the translated parameter (reasoning_effort, thinking, thinkingConfig) rather than the goose level token
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/providers/utils.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
- claim: at launch the model name is resolved from --model first, then a resumed session's saved config, then recipe settings, then the GOOSE_MODEL environment variable, then config, and the effort-suffix normalization runs on the resolved name
  id: src-model-resolution
  limitations: does not itself address effort precedence beyond carrying the model name
  location: https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/builder.rs
  method: source_code
  observed_on: 2026-09-29
  version: 1.52.0
support: some_models
levels:
- evidence_ids:
  - src-thinking-enum
  - src-anthropic-format
  - src-openai-format
  - src-meta-mapping
  - docs-effort-variable
  meaning: No extended thinking. The thinking_effort tokens disabled and none parse to this level. On the wire it is an explicit thinking disable for adaptive-mode Claude models, an omitted thinking block for other Claude reasoning models, and reasoning_effort none for OpenAI models that accept none; models that always reason (Muse Spark, always-on Claude) clamp it to their lightest or default level instead.
  native: off
  normalized: off
- evidence_ids:
  - src-thinking-enum
  - src-anthropic-format
  - src-openai-format
  - src-google-format
  - docs-effort-variable
  meaning: Better latency, lighter reasoning (goose configure wording). Maps to Anthropic budget_tokens 4000, OpenAI reasoning_effort low, Gemini 3 thinkingLevel low, and Muse Spark reasoning_effort low.
  native: low
  normalized: low
- evidence_ids:
  - src-thinking-enum
  - src-anthropic-format
  - src-openai-format
  - src-google-format
  - src-effort-menus
  meaning: Moderate thinking (goose configure wording). Maps to Anthropic budget_tokens 10000, OpenAI reasoning_effort medium, and Gemini 3 thinkingLevel medium (low on gemini-3-pro). Not offered for GLM-5.3, Kimi K3, grok-3-mini, or grok-4.5, where it clamps to high or low.
  native: medium
  normalized: medium
- evidence_ids:
  - src-thinking-enum
  - src-anthropic-format
  - src-openai-format
  - src-google-format
  - src-codex-provider
  - docs-effort-variable
  meaning: Deep reasoning (goose configure wording). Maps to Anthropic budget_tokens 16000 (also the effort sent for always-on adaptive Claude models when nothing is chosen), OpenAI reasoning_effort high, Gemini 3 thinkingLevel high, and Muse Spark reasoning_effort high. The codex CLI provider defaults to this level.
  native: high
  normalized: high
- evidence_ids:
  - src-thinking-enum
  - src-anthropic-format
  - src-openai-format
  - src-meta-mapping
  - docs-effort-variable
  meaning: No constraints on thinking depth (goose configure wording). The alias xhigh parses to this level and is its model-name suffix form. Maps to Anthropic budget_tokens 32000, OpenAI reasoning_effort xhigh or max depending on the model, and Muse Spark reasoning_effort xhigh.
  native: max
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - src-model-config-default
  - src-anthropic-format
  - src-google-format
  - src-codex-provider
  - src-effort-menus
  - docs-gemini3-levels
controls:
- arguments:
  - --model
  - <model>-<level>
  changes_running_session: yes
  evidence_ids:
  - src-model-suffix
  - src-model-resolution
  - src-model-command
  id: model-suffix
  kind: model_suffix
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: -high
  value: level_token
- arguments:
  - --model
  - <model>-<level>
  changes_running_session: no
  evidence_ids:
  - docs-cli-commands
  - src-model-resolution
  - src-model-suffix
  id: model-launch-flag
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --model
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-effort-variable
  - src-config-effort-key
  - src-model-config-default
  id: goose-thinking-effort-env
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: GOOSE_THINKING_EFFORT
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - src-config-effort-key
  - src-configure-prompt
  - docs-config-files
  id: goose-thinking-effort-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: GOOSE_THINKING_EFFORT
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-gemini3-levels
  - src-config-effort-key
  id: gemini3-thinking-level
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: GEMINI3_THINKING_LEVEL
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-claude-thinking
  - src-config-effort-key
  id: claude-thinking-type
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: CLAUDE_THINKING_TYPE
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - src-config-effort-key
  id: claude-thinking-enabled
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: CLAUDE_THINKING_ENABLED
  value: on_or_off
- arguments: []
  changes_running_session: no
  evidence_ids:
  - src-codex-provider
  id: codex-reasoning-effort
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: CODEX_REASONING_EFFORT
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-cli-commands
  - src-model-command
  id: model-command
  kind: session_command
  lasts: one_session
  launch_modes:
  - interactive
  name: /model
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - src-acp-set-thinking-effort
  - src-agent-update-effort
  id: acp-set-thinking-effort
  kind: request_field
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: session/set_thinking_effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - src-model-suffix
  - src-agent-update-effort
  id: request-params-thinking-effort
  kind: request_field
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: request_params.thinking_effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - src-anthropic-format
  id: anthropic-budget-tokens
  kind: request_field
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: budget_tokens
  value: token_budget
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - src-google-format
  id: gemini-thinking-budget
  kind: request_field
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinking_budget
  value: token_budget
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-openrouter-parameters
  id: openrouter-parameters
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: OPENROUTER_PARAMETERS
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-viewing-reasoning
  - src-cli-thinking-display
  id: goose-cli-show-thinking
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  name: GOOSE_CLI_SHOW_THINKING
  value: on_or_off
precedence:
- acp-set-thinking-effort
- request-params-thinking-effort
- model-suffix
- goose-thinking-effort-env
- goose-thinking-effort-config
- claude-thinking-type
- claude-thinking-enabled
- gemini3-thinking-level
- codex-reasoning-effort
- openrouter-parameters
models:
- accepts:
  - low
  - high
  - max
  default: max
  evidence_ids:
  - src-effort-menus
  - src-glm-definitions
  model: glm-5.3
- accepts:
  - low
  - high
  - max
  default: max
  evidence_ids:
  - src-effort-menus
  - src-glm-definitions
  model: glm-5.3-flash
- accepts:
  - low
  - high
  - max
  default: max
  evidence_ids:
  - src-effort-menus
  model: kimi-k3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - src-openai-format
  model: o3-mini
- accepts:
  - off
  - low
  - medium
  - high
  evidence_ids:
  - src-openai-format
  model: gpt-5.4
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - src-openai-format
  model: gpt-6-astra
- accepts:
  - high
  evidence_ids:
  - src-openai-format
  model: gpt-*-pro
- accepts:
  - low
  - high
  evidence_ids:
  - src-openai-format
  model: grok-3-mini
- accepts:
  - off
  - low
  - medium
  - high
  evidence_ids:
  - src-openai-format
  model: grok-4.3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - src-openai-format
  model: grok-4.5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - src-meta-mapping
  - docs-effort-variable
  model: muse-spark-1.3
- accepts:
  - low
  - high
  evidence_ids:
  - src-google-format
  - docs-gemini3-levels
  model: gemini-3-pro
- accepts:
  - off
  - low
  - medium
  - high
  - max
  default: off
  evidence_ids:
  - src-anthropic-format
  - src-canonical-thinking-modes
  model: claude-sonnet-4-6
- accepts:
  - low
  - medium
  - high
  - max
  default: high
  evidence_ids:
  - src-anthropic-format
  - src-canonical-thinking-modes
  model: claude-fable-5
- accepts:
  - off
  - low
  - medium
  - high
  - max
  default: high
  evidence_ids:
  - src-codex-provider
  model: gpt-5.2-codex
invalid_level:
  behavior: uses_nearest_level
  evidence_ids:
  - src-openai-format
  - src-meta-mapping
  - src-google-format
  - src-effort-menus
  - src-config-effort-key
  - src-agent-update-effort
  message: 'Invalid thinking effort: <value>'
  warns: no
reporting:
  evidence_ids:
  - src-session-record
  - src-request-log
  - docs-config-files
  field: /model_config/request_params/thinking_effort
  locator: sessions.db
  notes: Read the record back with goose session export --session-id <id> --format json; the field is absent when no effort was chosen, in which case the format-specific default applied. The value is what goose put on the model config, not a provider echo. The translated wire parameter for each request (reasoning_effort, thinking, thinkingConfig) is in the rotated request logs at llm_request.<N>.jsonl under the goose state dir's logs directory, of which the 10 most recent are kept. goose info -v prints the config-file GOOSE_THINKING_EFFORT, but not a value supplied only through the environment.
  source: session_record
reasoning_output:
  control_id: goose-cli-show-thinking
  evidence_ids:
  - src-json-output
  - src-cli-thinking-display
  - docs-viewing-reasoning
  reaches_caller: full_text
gaps:
- area: other
  detail: goose is not installed on the research host, so no local runtime verification was possible; goose --version, help output, a disposable non-interactive session, and the request logs were all examined only through documentation and the source at commit b92a80da.
  next_check: Install goose on a disposable host (curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash), confirm the version, and re-run the observed checks; a GOOSE_THINKING_EFFORT run, a suffixed --model run, and the resulting llm_request log entries.
- area: models
  detail: Whether the effort chosen for glm-5.3 over the zai_coding_plan provider (OpenAI-compatible chat endpoint) reaches the API; the chat-completions format sends no effort field for GLM models, while goose's Desktop menu advertises low, high, and max for them.
  entry: glm-5.3
  next_check: Run goose with GOOSE_PROVIDER=zai_coding_plan, GOOSE_MODEL=glm-5.3, and GOOSE_THINKING_EFFORT=high, then inspect the newest llm_request.<N>.jsonl for a reasoning or thinking field in the request body.
- area: models
  detail: Whether Anthropic accepts output_config.effort max, which goose sends for ThinkingEffort::Max on adaptive-mode Claude models.
  entry: claude-sonnet-4-6
  next_check: Send one request with GOOSE_THINKING_EFFORT=max on claude-sonnet-4-6 and observe whether the API accepts or rejects the effort value.
- area: controls
  detail: The interaction between OPENROUTER_PARAMETERS reasoning.effort and goose's unified thinking effort on OpenRouter models is undocumented and was not verified; both can address reasoning on the same request.
  entry: openrouter-parameters
  next_check: Configure the openrouter provider with both OPENROUTER_PARAMETERS reasoning.effort and GOOSE_THINKING_EFFORT set to different values, run one prompt, and read which value appears in the request log.
- area: controls
  detail: The environment-variables guide states CLAUDE_THINKING_TYPE defaults to adaptive for Claude 4.6+ models, but the source sends an explicit thinking disable for those models when no effort is chosen; the runtime behavior was not observed directly.
  entry: claude-thinking-type
  next_check: Run goose with an Anthropic provider and claude-sonnet-4-6 with no effort set, and read the thinking field in the request log to confirm the disable is sent.
changes:
- Migrated to contract revision 2; gaps now record area and entry instead of the revision-1 subject key.
- Re-verified every finding for this refresh; v1.52.0 is still the latest release and main is still at commit b92a80da, so no behavioral drift was found in goose itself.
- Added canonical-registry evidence (src-canonical-thinking-modes) backing the Claude adaptive and always-on model rows directly.
- 'invalid_level now records the exact ACP refusal text for unparseable efforts (Invalid thinking effort: <value>).'
- Broadened the gpt-5-pro models entry to the gpt-*-pro pattern, which is what the source matches.
requires_claudine_update: true
reason: Claudine's goose wrapper cannot set a reasoning effort today, and this research shows a single reliable lever; the GOOSE_THINKING_EFFORT environment variable (tokens off, low, medium, high, max). Every model accepts it without error; on reasoning models goose translates it to the provider's parameter and clamps it to the nearest level the model supports, while on non-reasoning models it has no effect beyond off. Mapping Claudine's neutral effort scale onto that variable, with the model-name suffix (none/low/medium/high/xhigh, OpenAI Responses and xAI models only) as an alternative, is the change needed to offer one effort setting on goose.
contract_checked: 2026-09-29
---

# Reasoning Level on Goose CLI

Goose is an open-source agent (CLI, desktop app, and ACP server) that fronts
more than 40 LLM providers. It normalizes "how hard should the model think"
into one internal `ThinkingEffort` value that every control feeds, and then
translates that value into whatever each provider's wire format expects:
OpenAI `reasoning_effort`, Anthropic `thinking`/`output_config.effort`,
Gemini `thinkingLevel`, or a token budget. This document is about the CLI
(v1.52.0 era, examined 2026-09-29); goose was not installed on the research
host, so every finding rests on the official documentation and the source at
commit `b92a80da` (main, the direct successor of the v1.52.0 tag).

## Levels

Goose's own scale has five levels, `off`, `low`, `medium`, `high`, `max`,
defined by the `ThinkingEffort` enum. The parser is case-insensitive and also
accepts aliases: `disabled` and `none` for `off`, `med` for `medium`, and
`xhigh` for `max`. The model-name suffix form uses `xhigh` (not `max`).

| Token | What goose says it does | What goes over the wire |
| --- | --- | --- |
| `off` | No extended thinking | explicit `thinking: disabled` (adaptive Claude), no thinking block (other Claude), `reasoning_effort: none` (OpenAI models that accept it); always-reasoning models clamp it |
| `low` | Better latency, lighter reasoning | budget 4000 / `low` / Gemini `low` |
| `medium` | Moderate thinking | budget 10000 / `medium` / Gemini `medium` (`low` on gemini-3-pro) |
| `high` | Deep reasoning | budget 16000 / `high` / Gemini `high`; the default for the codex CLI provider and for always-on adaptive Claude models |
| `max` | No constraints on thinking depth | budget 32000 / `xhigh` or `max` (model-dependent) |

There is **no single provider-wide default**. When nothing is chosen, goose
sends no effort parameter and each format applies its own behavior, so the
effective default is decided by the model:

- Claude adaptive models (opus-4-6, opus-4-7, opus-4-8, opus-5, sonnet-4-6,
  sonnet-5): thinking is explicitly **disabled** (see the drift note below).
- Claude always-on adaptive models (fable-5, fable-5.1, opus-5-5): adaptive
  thinking at effort **high**; it cannot be turned off.
- Pre-4.6 Claude reasoning models: thinking **disabled**.
- Gemini 3: no `thinkingLevel` sent; Google's default **low** applies.
- Gemini 2.5: a `thinking_budget` of **8192** tokens.
- OpenAI Responses models: no `reasoning_effort` sent; OpenAI's own default.
- The codex CLI provider: **high**.
- GLM-5.3 and Kimi K3: goose's own menu reports **max** as the current value.
- Muse Spark: no `reasoning_effort` sent; Meta's default.

Only reasoning models get a choice at all: goose's menus offer exactly `off`
for non-reasoning models (gpt-4o, llama, …), which is why `support` is
`some_models`.

**Drift note.** The environment-variables guide says `CLAUDE_THINKING_TYPE`
"defaults to `adaptive` for Claude 4.6+ models". The source at `b92a80da`
disagrees: with no effort chosen, `thinking_type_for_provider` returns
`Disabled` for adaptive-mode Claude models and `apply_thinking_config` sends
`thinking: {type: "disabled"}` (these models require an explicit disable).
Per this repository's rules the code is treated as correct and the doc claim
as drift; a runtime confirmation is recorded in `gaps`.

## Choosing a Level

There is no dedicated `--effort` flag and no `/effort` slash command. Every
control either feeds the one `ThinkingEffort` value or bypasses it for a raw
parameter.

**Model-name suffix** — append `-none`, `-low`, `-medium`, `-high`, or
`-xhigh` to a model name. Goose strips the suffix and records the effort.
This works wherever a model name is given: the `--model` flag, the
`GOOSE_MODEL` variable, the config file, recipe and subagent
`settings.goose_model`, and the `/model` session command. It only takes effect
for **OpenAI Responses models (o-series, gpt-5, gpt-6) and xAI reasoning
models (grok-4.5, grok-4.3, grok-3-mini)** — for anything else the "suffix"
is left in the name and the provider will reject the model.

```sh
goose run --provider openai --model o3-mini-high -t "explain this diff"
```

**`GOOSE_THINKING_EFFORT` environment variable** — the one lever that works
for every model. An unparseable value is silently discarded, not rejected.

```sh
GOOSE_THINKING_EFFORT=high goose run -t "explain this diff"
```

**`GOOSE_THINKING_EFFORT` config key** — same key in
`~/.config/goose/config.yaml` (macOS/Linux;
`%APPDATA%\Block\goose\config\config.yaml` on Windows). `goose configure`
writes it: choosing a reasoning model triggers a "Select thinking effort"
prompt (`off`/`low`/`medium`/`high`/`max`, initially `off`). The key is read
from the file even though the settings table does not list it.

```yaml
GOOSE_THINKING_EFFORT: high
```

**`/model` session command** — switch model (or provider) inside an
interactive session; a suffix in the new name sets the effort. The
confirmation message prints model names only, and the suffix is already
stripped, so it never states the effort. Switching is refused for ACP
providers and providers that manage their own conversation context.

```text
/model o3-mini-high
```

**ACP `session/set_thinking_effort`** — a request method served by
`goose serve` and `goose acp` (this is how goose Desktop changes effort on a
running session). The value is the raw effort id; on the legacy path an
unparseable value fails the request with `Invalid thinking effort: <value>`.

**`request_params.thinking_effort`** — the model-config request field all of
the above funnel into. It can also be set directly in a custom provider's
model definitions (`~/.config/goose/custom_providers/*.json`) or through the
ACP provider-update request. An explicit value present before construction is
the one thing the model-name suffix will not overwrite — even a value that
does not parse as a `ThinkingEffort` (a harness `default`, say) is kept as an
explicit pick.

**Token budgets** — two request params bypass the named levels entirely:
`budget_tokens` (Anthropic-format providers; while it is set with no named
effort, it decides the budget directly; minimum 1024, clamped to leave room
for at least 1024 answer tokens) and `thinking_budget` (Gemini 2.5; default
8192, `0` disables on gemini-2.5-flash, a negative value is replaced by the
default with a warning).

**Legacy keys** — still honored when `GOOSE_THINKING_EFFORT` is unset or
unparseable, in this order: `CLAUDE_THINKING_TYPE` (`adaptive`/`enabled` →
high, `disabled` → off), `CLAUDE_THINKING_ENABLED` (bool), and
`GEMINI3_THINKING_LEVEL` (`low`/`high`). The codex CLI provider additionally
reads `CODEX_REASONING_EFFORT` (`none`/`low`/`medium`/`high`/`xhigh`) when no
unified effort exists at all. Note that the legacy fallbacks fold into the
one unified effort — `GEMINI3_THINKING_LEVEL` set in the environment affects
every provider, not only Gemini.

**`OPENROUTER_PARAMETERS`** — a config key (YAML object or JSON string) whose
contents are merged into every OpenRouter request, so
`reasoning: {effort: high}` reaches the API without touching
`ThinkingEffort`. How it interacts with the unified effort is unverified
(see `gaps`).

### Precedence

From strongest to weakest, with the evidence for each step:

1. `session/set_thinking_effort` — writes the value onto a running session's
   model config (`src-acp-set-thinking-effort`, `src-agent-update-effort`).
2. An explicit `request_params.thinking_effort` — the suffix normalization
   only fills the param when it is absent (`src-model-suffix`).
3. Model-name suffix — applied at construction before any default
   (`src-model-suffix`); a unit test pins `o3-mini-none` to `off` even with
   `GOOSE_THINKING_EFFORT=high` in the environment.
4. `GOOSE_THINKING_EFFORT` environment variable — `get_param` checks the
   environment before the config file (`src-config-effort-key`); the docs
   state the same order (`docs-config-files`).
5. `GOOSE_THINKING_EFFORT` config key.
6. `CLAUDE_THINKING_TYPE`, then `CLAUDE_THINKING_ENABLED`, then
   `GEMINI3_THINKING_LEVEL` — the legacy fallback chain inside
   `get_goose_thinking_effort` (`src-config-effort-key`).
7. `CODEX_REASONING_EFFORT` — read only when the unified effort is entirely
   absent (`src-codex-provider`).
8. `OPENROUTER_PARAMETERS` — a different mechanism (passthrough body fields)
   whose ordering against the above is unverified.

A resumed session reuses its persisted model config, so the session's stored
effort outlives changes to the environment.

### An invalid level

Two different behaviors, depending on where the bad value arrives:

- A level **token the model does not accept** is never an error. Goose clamps
  it to the nearest level the model supports (`medium` → `high` on grok-3-mini,
  `off` → `low` on Muse Spark, `max` → `xhigh` on gpt-5.4) and sends that
  instead, without warning. This is the behavior recorded in `invalid_level`.
- A value that **does not parse at all** is silently discarded on the
  launch/config path (`get_goose_thinking_effort` drops it and falls through
  to the legacy chain), so the run proceeds with whatever the next control in
  the chain supplies. The one refusal is the ACP method:
  `session/set_thinking_effort` with an unparseable value fails the request
  with `Invalid thinking effort: <value>`.

## Models

Which levels a model accepts is goose's compatibility table plus the
provider's own support; goose never errors on a level a model lacks — it
clamps to the nearest supported one (see above). Levels below are goose
tokens; the wire value may differ (`max` often travels as `xhigh`).

| Model(s) | Accepts | Notes |
| --- | --- | --- |
| most reasoning models | off, low, medium, high, max | full list |
| glm-5.3, glm-5.3-flash | low, high, max | off → low, medium → high; menu default max |
| kimi-k3 (and `moonshot/kimi-k3`) | low, high, max | same clamping; menu default max |
| gpt-5.4 / 5.5 / 5.6 | off, low, medium, high | max clamps to xhigh |
| gpt-6 (Sol, Luna) | off, low, medium, high, max | full list on the wire |
| gpt-6-astra | low, medium, high, max | off clamps to low |
| gpt-5-pro, gpt-6-pro (`gpt-*-pro`) | high | every effort maps to high |
| o-series and other gpt-5 (o3-mini …) | low, medium, high | off → low, max → high |
| grok-4.3 | off, low, medium, high | max → high |
| grok-4.5 | low, medium, high | off → low, max → high |
| grok-3-mini | low, high | off → low, medium/max → high |
| muse-spark-1.1 … 1.3 | low, medium, high, max | max → xhigh, off → low; always reasons |
| gemini-3-pro | low, high | off → API default low; medium → low |
| other gemini-3 | low, medium, high | off → API default low |
| gemini-2.5 | budget-based | `thinking_budget` request param, default 8192 |
| claude-sonnet-4-6, opus-4-6/4-7/4-8, opus-5, sonnet-5 | off, low, medium, high, max | effort → adaptive `output_config.effort`; unset → disabled |
| claude-fable-5, fable-5.1, opus-5-5 | low, medium, high, max | cannot disable; unset → high |
| claude-sonnet-4-5 and earlier | off, low, medium, high, max | effort → `budget_tokens` 4000/10000/16000/32000; unset → disabled |
| gpt-5.2-codex and other codex CLI models | off, low, medium, high, max | sent as `-c model_reasoning_effort=none|low|medium|high|xhigh`; default high |
| non-reasoning models (gpt-4o, llama, …) | off | the menu offers nothing else |

The Claude rows are backed by the canonical model registry
(`canonical_models.json`), which marks Opus 4.6/4.7/4.8/5 and Sonnet 4.6/5
`adaptive` and Fable 5/5.1 and Opus 5.5 `always_on_adaptive`.

One caveat from the source: for GLM-5.3 served through `zai_coding_plan`
(the OpenAI-compatible coding endpoint), the chat-completions format maps no
effort field at all, even though goose's menu advertises low/high/max for the
model — through the Anthropic-compatible `zai` provider the effort does map.
This is recorded in `gaps`.

## Confirming the Level

No provider ever reports back which level it used, so confirmation is
goose-side:

1. **Session record** — the effort goose applied is persisted on the
   session's model config:

   ```sh
   goose session export --session-id 20260929_1 --format json
   # → .model_config.request_params.thinking_effort == "high"
   ```

   An absent field means nothing was chosen and the format default applied.

2. **Request logs** — every provider request body is written to
   `llm_request.<N>.jsonl` under the goose state dir's `logs` directory
   (`~/.local/share/goose/logs/` on Linux,
   `~/Library/Application Support/Block/goose/logs/` on macOS; the 10 most
   recent are kept). The translated parameter (`reasoning_effort`,
   `thinking`, `thinkingConfig`) is visible in the request, which is the
   closest available proof of what the run used.

3. **`goose info -v`** — prints every config-file value, including
   `GOOSE_THINKING_EFFORT` when it was set through `goose configure` or the
   config file. It does not show values supplied only through the
   environment, and it shows configuration, not what a past run used.

4. **In-session** — the `/model` command with no argument prints the current
   model (suffix stripped); it does not print the effort.

## Reasoning Output

Whether a non-interactive caller receives the model's reasoning depends on
the output format:

- **`--output-format stream-json`** — every assistant message is emitted as a
  `{"type":"message",...}` NDJSON event on stdout, and the message's content
  includes full `Thinking` (and `RedactedThinking`) blocks. No switch needed.
- **`--output-format json`** — the final payload's `messages` array keeps
  `Thinking` blocks as well.
- **plain text (the default)** — reasoning is hidden. `GOOSE_CLI_SHOW_THINKING=1`
  shows it under a dimmed "Thinking:" header, but only when stdout is a
  terminal, so a piped or captured `goose run` receives nothing regardless of
  the variable.

Goose captures reasoning for DeepSeek-R1 and Kimi (`reasoning_content`),
Gemini (thinking blocks), and Claude (thinking blocks). A wrapper that wants
the reasoning text should use `stream-json` or `json`, not the text path.

## Sources

- goose releases (v1.52.0 is latest at research time, released 2026-09-23):
  <https://github.com/aaif-goose/goose/releases>
- Providers guide (effort variable, Muse Spark, Gemini 3 levels, OpenRouter
  parameters, viewing reasoning):
  <https://goose-docs.ai/docs/getting-started/providers>
- Environment variables guide:
  <https://goose-docs.ai/docs/guides/environment-variables>
- CLI commands guide: <https://goose-docs.ai/docs/guides/goose-cli-commands>
- Configuration files guide: <https://goose-docs.ai/docs/guides/config-files>
- Source, commit-pinned to `b92a80daf4a77d7e854709965bdfdc489c0472d2`
  (main, 2026-09-29):
  - [crates/goose-provider-types/src/thinking.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/thinking.rs)
  - [crates/goose-provider-types/src/model.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/model.rs)
  - [crates/goose-provider-types/src/formats/anthropic.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/anthropic.rs)
  - [crates/goose-provider-types/src/formats/openai.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/openai.rs)
  - [crates/goose-provider-types/src/formats/google.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/formats/google.rs)
  - [crates/goose-provider-types/src/canonical/data/canonical_models.json](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-provider-types/src/canonical/data/canonical_models.json)
  - [crates/goose/src/config/base.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/config/base.rs)
  - [crates/goose/src/model_config.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/model_config.rs)
  - [crates/goose/src/agents/agent.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/agents/agent.rs)
  - [crates/goose/src/acp/server.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/acp/server.rs)
  - [crates/goose/src/acp/response_builder.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/acp/response_builder.rs)
  - [crates/goose/src/providers/codex.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/providers/codex.rs)
  - [crates/goose/src/providers/utils.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose/src/providers/utils.rs)
  - [crates/goose-providers/src/openai.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-providers/src/openai.rs)
  - [crates/goose-providers/src/declarative/definitions/zai_coding_plan.json](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-providers/src/declarative/definitions/zai_coding_plan.json)
  - [crates/goose-cli/src/commands/configure.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/commands/configure.rs)
  - [crates/goose-cli/src/session/mod.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/mod.rs)
  - [crates/goose-cli/src/session/builder.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/builder.rs)
  - [crates/goose-cli/src/session/output.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/session/output.rs)
  - [crates/goose-cli/src/commands/session.rs](https://github.com/aaif-goose/goose/blob/b92a80daf4a77d7e854709965bdfdc489c0472d2/crates/goose-cli/src/commands/session.rs)

Note: goose's documentation moved. The URLs under `block.github.io/goose/`
redirect to `goose-docs.ai`, and the repository moved from `block/goose` to
`aaif-goose/goose` under the Agentic AI Foundation.

## Changelog

- 2026-09-29 (refresh) — re-verified every finding; v1.52.0 is still the
  latest release and main is still at `b92a80da`, so no goose-side drift was
  found. Migrated the document to contract revision 2 (gaps now record
  `area` and `entry`). Added canonical-registry evidence for the Claude
  adaptive and always-on model rows, recorded the exact ACP refusal message
  for unparseable efforts, and broadened the pro-model entry to `gpt-*-pro`.
- 2026-09-29 (initial) — established the five-level `ThinkingEffort`
  scale (`off`/`low`/`medium`/`high`/`max` with aliases), the
  `GOOSE_THINKING_EFFORT` variable and config key, the model-name effort
  suffix (OpenAI Responses and xAI models only), the `/model` session command,
  the ACP `session/set_thinking_effort` method, legacy `CLAUDE_THINKING_TYPE`,
  `CLAUDE_THINKING_ENABLED`, `GEMINI3_THINKING_LEVEL`, and
  `CODEX_REASONING_EFFORT` keys, per-model level sets and clamping, the
  session-record and request-log confirmation surfaces, and the
  `stream-json`/`json` reasoning delivery. Found the docs/source drift on the
  Claude 4.6 default. Goose was not installed locally, so nothing was verified
  at runtime (see `gaps`).