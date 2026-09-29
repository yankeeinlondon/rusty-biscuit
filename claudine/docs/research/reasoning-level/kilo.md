---
$schema: ./_schema.yaml
schema_revision: 1
provider: kilo
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 7.3.45
evidence:
- claim: Installed `kilo --version` reports 7.3.45; `kilo run --help` exposes `--variant` and `--thinking`.
  id: cli-help
  limitations: Help does not establish variant precedence or backend acceptance.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-run-help-7.3.45.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: The inspected ~/.kilo path was absent; ~/.config/kilo/kilo.jsonc contained only the schema URL and no reasoning setting.
  id: config-inspection
  limitations: This is only the inspected account on this host.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-config-inspection.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: '`kilo models --verbose` listed 606 model entries with named variants; the union of their variant keys is the token list and each model entry records its own accepted subset.'
  id: model-catalog
  limitations: Catalog presence is not a successful request test and can change as providers update.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-7.3.45-model-variants.json
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: Kilo documents `/variant`, Shift+Tab cycling, remembered per-agent choices, session override precedence, and invalid per-task selections failing rather than silently falling back.
  id: model-selection-docs
  limitations: The page does not state how `kilo run --variant` ranks against a remembered selection or define a global default reasoning level.
  location: https://kilo.ai/docs/code-with-ai/agents/model-selection
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Agent definitions accept `variant`; configuration merges built-in, global, project, .kilo/agent files, then KILO_CONFIG_CONTENT.
  id: custom-modes-docs
  limitations: Does not fully order the explicit CLI option against a persisted session choice.
  location: https://kilo.ai/docs/customize/custom-modes
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Variants are named provider-specific configurations merged into the request; users can define model variant names and options.
  id: custom-models-docs
  limitations: There is no provider-wide finite meaning for custom names.
  location: https://kilo.ai/docs/code-with-ai/agents/custom-models
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: The run command accepts `--variant` for provider-specific reasoning effort and `--thinking` to show thinking blocks.
  id: cli-reference
  limitations: Does not specify model-specific support or actual-use reporting.
  location: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '`reasoning_display` controls the presentation of reasoning blocks; it is not an effort setting.'
  id: settings-docs
  limitations: Display configuration does not prove reasoning content will be returned by every provider/model.
  location: https://kilo.ai/docs/getting-started/settings
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: A request with `--variant definitely-not-a-variant` reached an API auth failure (401) before variant validation.
  id: invalid-variant-attempt
  limitations: Authentication prevented observing invalid-variant handling.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-invalid-variant-session.txt
  method: disposable_test
  observed_on: 2026-09-29
  version: 7.3.45
- claim: The export stores the selected variant at `/info/variant` in the assistant message record, including on the failed attempt.
  id: session-export
  limitations: This is the selected value, not evidence that the provider actually used it.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-invalid-variant-session.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: 'The installed Kilo SDK exposes `variant?: string` on agent/model configuration types.'
  id: sdk-types
  limitations: This type inspection does not identify a stable public request field for choosing a run variant.
  location: ~/.config/kilo/node_modules/@kilocode/sdk/dist/v2/gen/types.gen.d.ts
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
support: some_models
levels:
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Fast, non-thinking variant on models that provide the instant/thinking pair; Kilo documents variant options as model/provider-specific.
  native: instant
  normalized: off
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: A no-reasoning variant where the model catalog provides this token.
  native: none
  normalized: off
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: The lowest named effort level in the model variants that expose minimal.
  native: minimal
  normalized: minimal
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Low effort, when listed for this model.
  native: low
  normalized: low
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Medium effort, when listed for this model.
  native: medium
  normalized: medium
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: High effort, when listed for this model.
  native: high
  normalized: high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Thinking-enabled variant on models that expose the instant/thinking pair; the model defines its exact behavior.
  native: thinking
  normalized: high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Extra-high effort, ordered above high where the model exposes it.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Maximum named effort, ordered above other numeric effort choices where exposed.
  native: max
  normalized: maximum
default_level:
  decided_by: unknown
  evidence_ids:
  - model-catalog
  - config-inspection
  - model-selection-docs
controls:
- arguments:
  - --variant
  - <level>
  changes_running_session: no
  evidence_ids:
  - cli-help
  - cli-reference
  id: run-variant
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: global-agent-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.code.variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: project-agent-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.code.variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: agent-file-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: config-content
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: KILO_CONFIG_CONTENT
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - model-selection-docs
  id: session-variant
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /variant
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - model-selection-docs
  id: shift-tab-variant
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: Shift-Tab
  value: level_token
- arguments:
  - --thinking
  changes_running_session: no
  evidence_ids:
  - cli-help
  - cli-reference
  id: show-thinking
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - non_interactive
  name: --thinking
  value: on_or_off
precedence:
- session-variant
- config-content
- agent-file-variant
- project-agent-variant
- global-agent-variant
models:
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/aion-labs/aion-2.0
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/aion-labs/aion-3.0
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/aion-labs/aion-3.0-mini
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/aion-labs/aion-3.5
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/aion-labs/aion-3.5-mini
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/amazon/nova-2-lite-v1
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-fable-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-fable-5.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-haiku-4.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.5
- accepts:
  - none
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.7
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.8
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-5.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-4
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-4.5
- accepts:
  - none
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-4.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-5.5
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/arcee-ai/trinity-large-thinking
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-1.6
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-1.6-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-2-1-turbo
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-2.0-code
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-2.0-lite
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/bytedance-seed/seed-2.0-mini
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/cohere/command-a-plus
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/cohere/north-mini-code:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-chat-v3.1
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-r1
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-r1-0528
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v3.1-terminus
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v3.2
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v3.2-exp
- accepts:
  - none
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-flash
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-flash-0731
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-flash-vision-exp
- accepts:
  - none
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-pro
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-pro-0813
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4.1-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/dots-studio/dots-3-note-preview:free
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/fireworks/ember-1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-2.5-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-2.5-flash-lite
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-2.5-pro
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-2.5-pro-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3-flash-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-flash-lite
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-flash-lite-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-pro-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-pro-preview-customtools
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.5-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.5-flash-lite
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.6-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.7-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.8-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemma-4-26b-a4b-it
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/google/gemma-4-31b-it
- accepts:
  - none
  - low
  - high
  evidence_ids:
  - model-catalog
  model: kilo/ibm-granite/granite-4.2-8b
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/inception/mercury-2
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/inception/mercury-2.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/inclusionai/ling-3.0-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/inclusionai/ling-3.0-flash-fin
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/inclusionai/ling-3.0-flash-sante:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/inclusionai/ling-3.0-flash-vl
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/liquid/lfm-2.5-2.6b:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/meituan/longcat-2.0
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-glimmer-30b
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-spark-1.1
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-spark-1.2
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-spark-1.2-contributor
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-spark-1.3
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/meta/muse-spark-1.3-contributor
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m1
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m2
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m2.1
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m2.5
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m2.7
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/minimax/minimax-m3
- accepts:
  - none
  - high
  evidence_ids:
  - model-catalog
  model: kilo/mistralai/mistral-medium-3-5
- accepts:
  - none
  - high
  evidence_ids:
  - model-catalog
  model: kilo/mistralai/mistral-small-2603
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/moonshotai/kimi-k2-thinking
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/moonshotai/kimi-k2.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/moonshotai/kimi-k2.6
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/moonshotai/kimi-k2.7-code
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/moonshotai/kimi-k3
- accepts:
  - none
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/nex-agi/nex-n2.5-pro
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-nano-30b-a3b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free
- accepts:
  - none
  - low
  - medium
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-super-120b-a12b
- accepts:
  - none
  - low
  - medium
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-super-120b-a12b:free
- accepts:
  - none
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-ultra-550b-a55b
- accepts:
  - none
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3-ultra-550b-a55b:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3.5-lightning
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/nvidia/nemotron-3.5-lightning:free
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5-mini
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5-nano
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5-pro
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.1-codex
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.1-codex-max
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.1-codex-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.2
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.2-codex
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.2-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.3-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.4
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.4-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.4-nano
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.4-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.5
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.5-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-luna-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-sol-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-terra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-5.6-terra-pro
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-astra
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-astra-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-luna-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-sol-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-oss-120b
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-oss-20b
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-oss-safeguard-20b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/o1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/o3
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/o3-mini
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/o3-mini-high
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/o3-pro
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/openai/o4-mini
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: kilo/openai/o4-mini-high
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/perceptron/perceptron-mk1.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/poolside/laguna-s-2.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/poolside/laguna-s-2.1:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/poolside/laguna-xs-2.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/poolside/laguna-xs-2.1:free
- accepts:
  - none
  - medium
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/prism-ml/ternary-bonsai-2-27b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen-plus-2025-07-28
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-14b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-235b-a22b
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-235b-a22b-thinking-2507
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-30b-a3b
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-30b-a3b-thinking-2507
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-32b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-8b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-max
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-max-thinking
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-next-80b-a3b-thinking
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-vl-235b-a22b-thinking
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-vl-30b-a3b-thinking
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3-vl-8b-thinking
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-122b-a10b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-27b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-35b-a3b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-397b-a17b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-9b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-flash-02-23
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-plus-02-15
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.5-plus-20260420
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.6-27b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.6-35b-a3b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.6-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.6-max-preview
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.6-plus
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.7-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.7-max
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.7-plus
- accepts:
  - low
  - medium
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-2.4t-a95b
- accepts:
  - none
  - low
  - medium
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-27b
- accepts:
  - none
  - low
  - medium
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-27b:free
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-max-0902
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-max-prime
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/qwen/qwen3.8-omni-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/rekaai/reka-edge
- accepts:
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/sakana/fugu-max
- accepts:
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/sakana/fugu-ultra
- accepts:
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/sakana/fugu-ultra-v2
- accepts:
  - none
  - high
  evidence_ids:
  - model-catalog
  model: kilo/sakana/sakana-namazu
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/stealth/claude-opus-4.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/stealth/claude-opus-4.7
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/stealth/claude-opus-4.8
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/stealth/claude-sonnet-4.6
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/stealth/qwen3.6-plus
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/stealth/space-bunny-alpha
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/stepfun/step-3.5-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/stepfun/step-3.7-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/stepfun/step-3.7-flash:free
- accepts:
  - none
  - low
  - high
  evidence_ids:
  - model-catalog
  model: kilo/tencent/hy3
- accepts:
  - none
  - low
  - high
  evidence_ids:
  - model-catalog
  model: kilo/tencent/hy3-preview
- accepts:
  - none
  - low
  - high
  evidence_ids:
  - model-catalog
  model: kilo/tencent/hy4-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/thinkingmachines/inkling
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/thinkingmachines/inkling-small
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/thinkingmachines/inkling-small:free
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/upstage/solar-mini4
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/upstage/solar-pro-3
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/upstage/solar-pro4
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-4.20
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-4.3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-4.5
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-4.6
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-4.7
- accepts:
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/x-ai/grok-build-0.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/xiaomi/mimo-v2.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/xiaomi/mimo-v2.5-pro
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/xiaomi/mimo-v2.6-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/xiaomi/mimo-v2.6-pro
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/xiaomi/mimo-v2.6-pro-ultraspeed
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.5-air
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.5v
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.6
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.6v
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.7
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-4.7-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5-turbo
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.1
- accepts:
  - none
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.2
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.3
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.3-flash
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.3-flashx
- accepts:
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5.3-prime
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: kilo/z-ai/glm-5v-turbo
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-fable-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-fable-5-1
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-haiku-4-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-4-5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-4-6
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-4-7
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-4-8
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-opus-5-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-sonnet-4
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-sonnet-4-5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-sonnet-4-6
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-sonnet-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/claude-sonnet-5-5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/deepseek-v4-flash
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/deepseek-v4-flash-vision-exp
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/deepseek-v4-pro
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: opencode/deepseek-v4.1-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.1-pro
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.5-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.5-flash-lite
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.6-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.7-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gemini-3.8-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/glm-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/glm-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/glm-5.2
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/glm-5.3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/glm-5.3-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5-codex
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5-nano
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.1-codex
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.1-codex-max
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.1-codex-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.2
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.2-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.3-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.3-codex-spark
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.4
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.4-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.4-nano
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.4-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.5
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.5-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-5.6-terra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-6-astra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/gpt-6-sol
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/kimi-k2.5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/kimi-k2.6
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/kimi-k2.7-code
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/kimi-k3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/ling-3.0-flash-fin-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/longcat-2.5-preview-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/mimo-v2.6-flash-free
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/muse-spark-1.2
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/muse-spark-1.3
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: opencode/muse-spark-1.3-contributor-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/nemotron-3-ultra-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/nemotron-3.5-lightning-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: opencode/space-bunny-free
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-fable-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-fable-5-1
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-haiku-4-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-haiku-4-5-20251001
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-5-20251101
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-6
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-7
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-8
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-4-8-fast
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-5-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-5-5-fast
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-opus-5-fast
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-sonnet-4-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-sonnet-4-5-20250929
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-sonnet-4-6
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-sonnet-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: anthropic/claude-sonnet-5-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: deepseek/deepseek-flash
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: deepseek/deepseek-v4-pro
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/deep-research-max-preview-04-2026
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/deep-research-preview-04-2026
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: google/gemini-2.5-computer-use-preview-10-2025
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: google/gemini-2.5-flash
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: google/gemini-2.5-flash-image
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: google/gemini-2.5-flash-lite
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: google/gemini-2.5-pro
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3-flash-preview
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3-pro-image
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3-pro-image-preview
- accepts:
  - minimal
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-image
- accepts:
  - minimal
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-image-preview
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-lite
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-lite-image
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-live-preview
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-flash-tts-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-pro-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.1-pro-preview-customtools
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.5-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.5-flash-lite
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.6-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.7-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-3.8-flash
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-flash-latest
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-flash-lite-latest
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/gemini-omni-flash-preview
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/gemma-4-26b-a4b-it
- accepts:
  - low
  - high
  evidence_ids:
  - model-catalog
  model: google/gemma-4-31b-it
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: groq/openai/gpt-oss-120b
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: groq/openai/gpt-oss-20b
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: groq/openai/gpt-oss-safeguard-20b
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: mistral/mistral-medium-2604
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: mistral/mistral-medium-latest
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: mistral/mistral-small-2603
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: mistral/mistral-small-latest
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai/kimi-k2.6
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai/kimi-k2.7-code
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai/kimi-k2.7-code-highspeed
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai/kimi-k3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai-cn/kimi-k2.6
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai-cn/kimi-k2.7-code
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai-cn/kimi-k2.7-code-highspeed
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: moonshotai-cn/kimi-k3
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/gpt-5
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/gpt-5-mini
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/gpt-5-nano
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: openai/gpt-5-pro
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.1
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.2
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.2-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.3-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.3-codex-spark
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4-mini-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4-nano
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.4-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.5
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.5-fast
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.5-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-luna-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-luna-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-sol-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-sol-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-terra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-terra-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-5.6-terra-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-astra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-astra-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-astra-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-luna-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-luna-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-sol-fast
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-6-sol-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-daybreak-blue-latest
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-daybreak-red-latest
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openai/gpt-realtime-2.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/o3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openai/o3-pro
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-fable-5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-fable-5.1
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-haiku-4.5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-4.1
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-4.5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-4.6
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-4.7
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-4.8
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-opus-5.5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-sonnet-4
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-sonnet-4.5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-sonnet-4.6
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-sonnet-5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/anthropic/claude-sonnet-5.5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3-flash-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3-pro-image
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3-pro-image-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-flash-image
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-flash-image-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-flash-lite
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-flash-lite-image
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-flash-lite-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-pro-preview
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.1-pro-preview-customtools
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.5-flash
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.5-flash-lite
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.6-flash
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.7-flash
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/google/gemini-3.8-flash
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/inception/mercury-2
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/inception/mercury-2.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/moonshotai/kimi-k2-thinking
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/moonshotai/kimi-k2.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/moonshotai/kimi-k2.6
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/moonshotai/kimi-k2.7-code
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/moonshotai/kimi-k3
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5-image
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5-image-mini
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5-mini
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5-nano
- accepts:
  - high
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5-pro
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.1-codex
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.1-codex-max
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.1-codex-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.2
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.2-codex
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.2-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.3-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.4
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.4-image-2
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.4-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.4-nano
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.4-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.5
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.5-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-luna-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-sol-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-terra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-5.6-terra-pro
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-astra
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-astra-pro
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-luna
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-luna-pro
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-sol
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-6-sol-pro
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-oss-120b
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-oss-20b
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: openrouter/openai/gpt-oss-safeguard-20b
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.5-air
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.5v
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.6
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.6v
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.7
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-4.7-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5-turbo
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.1
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.2
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.3
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.3-flash
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.3-flashx
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5.3-prime
- accepts:
  - instant
  - thinking
  evidence_ids:
  - model-catalog
  model: openrouter/z-ai/glm-5v-turbo
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-fable-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-fable-5.1
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4.5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4.6
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4.7
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-4.8
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-opus-5.5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-sonnet-4
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-sonnet-4.5
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-sonnet-4.6
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-sonnet-5
- accepts:
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/anthropic/claude-sonnet-5-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/baidu/ernie-5.0-thinking-preview
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/deepseek/deepseek-v4-flash
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/deepseek/deepseek-v4-pro
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: zenmux/deepseek/deepseek-v4.1-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-2.5-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-2.5-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-3-flash-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-3.1-flash-lite
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-3.1-pro-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/google/gemini-3.5-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/inclusionai/ring-2.6-1t
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k2.5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k2.6
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k2.7-code
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k2.7-code-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k2.7-code-highspeed
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/moonshotai/kimi-k3-free
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5-codex
- accepts:
  - none
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.1-codex
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.1-codex-mini
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.2
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.2-codex
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.2-pro
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.3-codex
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.4
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.4-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.5-fast
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.5-instant
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.5-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.6-luna
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.6-sol
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-5.6-terra
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-6-astra
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/openai/gpt-6-sol
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/sapiens-ai/agnes-1.5-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/stepfun/step-3.7-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/stepfun/step-3.7-flash-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/tencent/hy3-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/volcengine/doubao-seed-1.8
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/volcengine/doubao-seed-2.0-lite
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/volcengine/doubao-seed-2.0-mini
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/volcengine/doubao-seed-2.0-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2-omni
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2.5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2.5-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/xiaomi/mimo-v2.6-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.5-air
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.6
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.6v
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.6v-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.6v-flash-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.7
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.7-flash-free
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-4.7-flashx
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5-turbo
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5.2
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5.3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5.3-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5.3-flashx
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: zenmux/z-ai/glm-5v-turbo
invalid_level:
  behavior: unknown
  evidence_ids:
  - invalid-variant-attempt
  warns: unknown
reporting:
  evidence_ids:
  - session-export
  field: /info/variant
  locator: session-record
  notes: Run `kilo export <sessionID>` and inspect the assistant message record at /info/variant. This confirms Kilo recorded the selection; it does not confirm an upstream model honored it.
  source: session_record
reasoning_output:
  control_id: show-thinking
  evidence_ids:
  - cli-reference
  - settings-docs
  reaches_caller: unknown
gaps:
- detail: The default reasoning variant varies by model/provider or may not be set; the inspected local config is empty and the docs do not define a universal Kilo default.
  next_check: Run authenticated requests without --variant against representative models and inspect the exported session plus provider request/response.
  subject: default-level
- detail: The docs order picker/session selections and configuration layers, but do not establish where the launch --variant option wins against remembered session selections, so run-variant is not placed in the precedence list.
  next_check: With an authenticated disposable profile, set conflicting values in --variant, agent config, and a resumed session, then inspect the resolved request and export.
  subject: precedence
- detail: The only invalid-token attempt was stopped by a 401 authentication error before validation.
  next_check: Repeat the disposable run with an authenticated account and an invalid --variant on a model with catalogued variants.
  subject: invalid-level
- detail: The export confirms the selected variant but not the effort actually honored by the upstream model.
  next_check: Compare successful exported assistant message records with the provider request payload or a provider response that reports its resolved effort.
  subject: reporting
- detail: The --thinking flag enables reasoning-block display, but whether non-interactive output contains full text, a summary, or no content depends on provider/model response behavior.
  next_check: Run authenticated non-interactive JSON and default-format sessions on representative models with and without --thinking, then inspect emitted events.
  subject: reasoning-output
- detail: Thirty catalog entries use a ~ alias in the model identifier, which the contract model pattern rejects; those entries cannot be represented verbatim in models.
  next_check: Update the reasoning-level contract model identifier pattern to allow the exact catalog identifier grammar, then include these entries.
  subject: catalog-model-identifiers
- detail: The documented Shift+Tab shortcut selects variants, but the control name pattern disallows the exact plus sign, so the shortcut is described in the body and cannot be represented as an exact structured control.
  next_check: Extend the control name pattern to allow +, then add Shift+Tab as a session_command control.
  subject: keyboard-shortcut
- detail: No model-name suffix control was found in installed help or the inspected official selection/configuration docs.
  next_check: Inspect the 7.3.45 model-selection parser or test a documented model ID with a variant suffix.
  subject: model-suffix
changes:
- Replaced stale universal-looking level claims with the exact token union from the installed 7.3.45 model catalog.
- Expanded model coverage to every catalog entry whose accepted variants differ from the union (606 models).
- Added Shift+Tab, actual config locations, and clarified that --thinking controls display rather than effort.
- 'Corrected invalid-level and reporting claims: the captured attempt hit authentication failure, and the export stores a selected value rather than proof of upstream use.'
requires_claudine_update: true
reason: Claudine should expose Kilo variant selection through --variant or provider-specific config and must not assume one universal effort mapping or that the recorded selection proves the model honored it.
contract_checked: 2026-09-29
---
## Levels

Kilo calls reasoning choices model variants. The installed model catalog exposes these exact tokens: `instant`, `none`, `minimal`, `low`, `medium`, `high`, `thinking`, `xhigh`, and `max`. Variant support is per model, and the variant keys are provider-specific. The two-token `instant`/`thinking` pair is not an effort ladder; its meaning comes from that model. The normalized values below are approximate mappings to Claudine’s scale.

## Choosing a Level

For a non-interactive run, pass a level as the run option:

```sh
kilo run --model openai/gpt-6-luna --variant high --format json "Explain the change"
```

Other controls:

- `--variant <level>` sets the variant for a run. Use a token listed for the selected model.
- In an interactive session, type `/variant` to open the variant selector. `Shift+Tab` cycles available variants.
- Configure a default for an agent with `agent.code.variant` in global `~/.config/kilo/kilo.jsonc` or project `kilo.jsonc`. For an agent Markdown file, use YAML frontmatter `variant: high`.
- `KILO_CONFIG_CONTENT` supplies an environment config overlay and has highest precedence among documented config layers. Example: `KILO_CONFIG_CONTENT='{"agent":{"code":{"variant":"high"}}}' kilo run "Explain the change"`.
- `--thinking` displays thinking blocks; it does not choose a reasoning level.
- Kilo documents config merge order as built-in defaults, global config, project config, `.kilo/` configs/agent files, then `KILO_CONFIG_CONTENT`. Its model selection docs say session override, last picked per agent, per-agent config, then global config. They do not say how the explicit `--variant` flag ranks against a remembered choice.

No variant suffix on the model name, separate config-override flag, or reasoning request field was documented in the checked CLI help or official pages.

## Models

The model catalog has 606 entries. Every catalog entry differs from the union of all tokens, so `models` above records each one’s exact accepted variant list. Check the installed `kilo models --verbose` output for current support; the catalog changes over time.

## Confirming the Level

Run `kilo export <sessionID>` and inspect the assistant message’s `/info/variant`. The export records Kilo’s selected variant. It does not establish that the upstream provider accepted or used that effort; the captured invalid-level attempt also recorded its selection before returning an authentication error.

## Sources

- [Kilo Code model selection](https://kilo.ai/docs/code-with-ai/agents/model-selection)
- [Kilo Code custom modes and configuration precedence](https://kilo.ai/docs/customize/custom-modes)
- [Kilo Code custom models and variants](https://kilo.ai/docs/code-with-ai/agents/custom-models)
- [Kilo Code CLI reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code settings: reasoning blocks](https://kilo.ai/docs/getting-started/settings)
- Local CLI help, model catalog, configuration inspection, and sanitized session export listed in frontmatter evidence.

## Changelog

The previous version described a mostly universal ladder and treated catalog labels as if they established acceptance and behavior. This revision records model-specific variant support from the installed catalog, includes all differing models, separates the `--thinking` display toggle from effort selection, and corrects invalid-request and reporting claims to reflect the authentication-blocked attempt.