---
$schema: ./_schema.yaml
schema_revision: 2
provider: claude
created: 2026-09-29
last_updated: 2026-09-29
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
versions_examined:
- 2.1.284
evidence:
- claim: claude --version prints 2.1.284 (Claude Code); claude --help lists a launch flag --effort <level> described as 'Effort level for the current session (low, medium, high, xhigh, max)'; claude agents --help lists --effort <level> described as 'Default effort level for sessions dispatched from agent view'.
  id: local-help-2-1-284
  limitations: The top-level help string omits ultracode, which the flag also accepts, and omits that the env var and /effort additionally accept auto; help text is not the authority on accepted values.
  location: claude --version, claude --help, and claude agents --help on this host (binary at /Users/ken/.local/bin/claude)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'Documents --effort: options low, medium, high, xhigh, max, or ultracode; available levels depend on the model; ultracode requests xhigh effort with ultracode on (v2.1.203+); the flag overrides the modelSettings and effortLevel settings for the session and does not persist. Also documents claude agents accepting --effort to set defaults for dispatched background sessions.'
  id: docs-cli-reference
  limitations: Prose documentation; the per-model level lists live on the model-config page and were not re-verified here beyond the models the local tests exercised.
  location: https://code.claude.com/docs/en/cli-reference
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'Effort levels control adaptive reasoning. The model table gives Fable 5.1, Fable 5, Opus 5.5, Sonnet 5.5, Opus 5, Sonnet 5, Opus 4.8, and Opus 4.7 all five levels (low, medium, high, xhigh, max) and Opus 4.6 and Sonnet 4.6 only low, medium, high, max; models not listed do not support effort. A level the model does not support falls back to the highest supported level at or below the one set. The default is high on every effort model except Opus 5.5 and Sonnet 5.5 (medium) and Opus 4.7 (xhigh), plus an organization can set a default effort for its organization default model. Resolution order: explicit choice (CLAUDE_CODE_EFFORT_LEVEL, --effort, /effort) over settings (saved per-model level or effortLevel) over the model default. As of v2.1.284 ultracode is a Claude Code setting rather than a model effort level: /effort ultracode and the ultracode setting leave the effort level unchanged, only --effort ultracode and the Agent SDK effortLevel value turn it on and set xhigh, the persisted effortLevel key and CLAUDE_CODE_EFFORT_LEVEL do not accept it, and it is unavailable when workflows are off or the model lacks xhigh (then --effort ultracode starts with ultracode off at the highest level allowed, up to xhigh). Enter in the /effort slider or /model picker saves the level as the per-model default (modelSettings); s applies it to the current session only (v2.1.257+); a -p /effort is always session-only. max is session-scoped unless set via the environment variable. /effort works mid-turn, /effort auto clears the saved level, the /model picker has an effort slider, Remote Control devices offer a session-only effort control (v2.1.234+), skill and subagent effort frontmatter overrides the session level but not the environment variable and is still capped, the session header shows the level next to the model name and the footer at startup and on change, ultrathink is an in-context keyword that leaves the sent effort level unchanged, CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING reverts Opus 4.6/Sonnet 4.6 to the fixed budget, and the ANTHROPIC_DEFAULT_*_MODEL_SUPPORTED_CAPABILITIES variables declare effort, xhigh_effort, and max_effort per pinned model.'
  id: docs-model-config-effort
  limitations: Documents intended behavior; the level actually used per request was verified against the session transcript separately.
  location: https://code.claude.com/docs/en/model-config#adjust-effort-level
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'Organizations can cap effort per model: per-role limits on Claude Enterprise plans and the maxEffortLevel managed setting on any plan or provider, with the lower cap applying. Levels above the cap are not offered in the /effort picker, and naming a higher level with --effort or /effort runs at the cap; interactive sessions and plain-text --print runs print a warning naming the requested and applied levels, while json or stream-json output and background agents clamp silently. Requires v2.1.195+.'
  id: docs-model-config-org-limits
  limitations: No organization cap exists on the account used for local testing, so the clamp was not observed directly.
  location: https://code.claude.com/docs/en/model-config#organization-effort-limits
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: The effortLevel settings key accepts only low, medium, high, or xhigh (not max, not ultracode), defaults to unset, and --effort takes precedence over it for one session while CLAUDE_CODE_EFFORT_LEVEL takes precedence over both. Within the same settings file a model's saved level (modelSettings) beats this key. In the user settings file the key is ignored by Opus 5.5 and models released after it; in project, local, and managed settings and via --settings it applies to every model. /effort writes modelSettings since v2.1.251 (before that it wrote this key).
  id: docs-settings-effortlevel
  limitations: Prose documentation; the cross-file resolution rules are stated on the modelSettings entry. The --settings behavior was verified locally.
  location: https://code.claude.com/docs/en/settings-reference#effortlevel
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: modelSettings maps a model's canonical name (such as claude-opus-5-5, matched against the model's alias, dated, [1m], and recognized provider-specific IDs) to an effortLevel (low, medium, high, or xhigh) and optionally a maxEffortLevel cap (v2.1.267+). A model's entry beats a top-level effortLevel in the same file; across files each model is resolved separately by settings precedence, so a managed effortLevel outranks a level saved in user settings. /effort auto clears the saved level for the active model.
  id: docs-settings-modelsettings
  limitations: Prose documentation; not every cross-file combination was exercised locally.
  location: https://code.claude.com/docs/en/settings-reference#modelsettings
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'maxEffortLevel (any settings scope, v2.1.267+) caps the effort level: any higher level from /effort, the /model picker, --effort, CLAUDE_CODE_EFFORT_LEVEL, skill or subagent effort frontmatter, or the model''s own default runs at the cap instead. Claude Code applies the cap itself before each request, so it holds on every provider; when several scopes set a cap the lowest applies, a max value sets no cap, a per-model maxEffortLevel in modelSettings replaces the source''s cap for that model (with max exempting it), and an organization effort limit combines by taking the lower cap.'
  id: docs-settings-maxeffortlevel
  limitations: Whether an edit to the key reaches a session that has already started is not stated (see gaps).
  location: https://code.claude.com/docs/en/settings-reference#maxeffortlevel
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'The ultracode boolean setting starts sessions with ultracode on without changing the effort level (before v2.1.284 it forced xhigh, and a cap below xhigh kept ultracode off); Claude Code reads the key but never writes it; /effort ultracode and /effort ultracode off toggle it for one session; --effort ultracode turns it on for one session at xhigh effort (v2.1.203+); an effort cap lowers the level without turning ultracode off; an Agent SDK apply_flag_settings control request accepts both the key and effortLevel: ultracode, the latter turning it on and setting xhigh.'
  id: docs-settings-ultracode
  limitations: Ultracode workflow behavior itself is out of scope for this topic.
  location: https://code.claude.com/docs/en/settings-reference#ultracode
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'alwaysThinkingEnabled: false turns extended thinking off for every session (no effect on Opus 5.5, Sonnet 5.5, or Fable models; on third-party providers the thinking parameter is omitted instead); MAX_THINKING_TOKENS takes precedence over it for one session (0 off, a positive value on even when the key is false). With thinking turned off on the Anthropic API, Claude Code sends effort high instead of a higher level to models it knows reject that combination, such as Opus 5. showThinkingSummaries: true only affects the interactive Ctrl+O expansion of thinking summaries; when unset or false the Anthropic API redacts thinking blocks, and third-party providers do not redact.'
  id: docs-settings-thinking
  limitations: The session toggle Option+T/Alt+T and the redaction behavior were observed through documentation plus local stream tests, not an interactive session.
  location: 'https://code.claude.com/docs/en/settings-reference#alwaysthinkingenabled (and #showthinkingsummaries)'
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'CLAUDE_CODE_EFFORT_LEVEL accepts low, medium, high, xhigh, max, or auto (the model default) and takes precedence over --effort, /effort, and the modelSettings and effortLevel settings, with a maxEffortLevel cap still applying. MAX_THINKING_TOKENS is the fixed thinking budget (0 disables thinking on the Anthropic API except on Opus 5.5, Sonnet 5.5, and the Fable; nonzero values are ignored on adaptive-reasoning models; on third-party providers 0 omits the thinking parameter). CLAUDE_CODE_DISABLE_THINKING=1 omits the thinking parameter entirely as a gateway-compatibility option and behaves like MAX_THINKING_TOKENS=0 on third-party providers. CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1 reverts Opus 4.6 and Sonnet 4.6 to that fixed budget. CLAUDE_CODE_ALWAYS_ENABLE_EFFORT=1 sends the effort parameter even for unrecognized model IDs, excluding models that reject it at the API (Claude 3 models, Sonnet 4.0 and 4.5, Opus 4.0 and 4.1, Haiku 4.5). CLAUDE_EFFORT is an output variable: set automatically in Bash tool subprocesses and hook commands to the effort level in effect when the subprocess starts (low, medium, high, xhigh, or max), matching the effort.level field passed to hooks, and only set when the current model supports effort.'
  id: docs-env-vars
  limitations: The env-var-over-flag claim was also verified locally; the adaptive-thinking fallback and the hooks effort.level field were not exercised directly.
  location: https://code.claude.com/docs/en/env-vars
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: Settings precedence is managed settings over command line (--settings) over project local over shared project over user; --settings JSON merges by the same rules, taking a key it sets over the same key in local, project, or user settings. Environment variables are not a level in this stack; their interaction with keys is decided per pair, and the page names CLAUDE_CODE_EFFORT_LEVEL as overriding --effort and /effort. The page names --effort as the per-session flag for effortLevel and modelSettings, and lists effortLevel and modelSettings among keys read once at session start, directing mid-session changes to /effort.
  id: docs-settings-precedence
  limitations: Generic precedence documentation; effort-specific ordering comes from the effort sources themselves.
  location: https://code.claude.com/docs/en/settings
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: claude agents accepts --effort to set the effort default for every session dispatched from agent view, taking the same values as the top-level --effort flag including ultracode; the active defaults appear in the footer. The effort a background session runs at persists when the supervisor stops and restarts its process, and a mid-session /effort change is kept; a session that took its effort from settings re-reads them on each process start.
  id: docs-agent-view
  limitations: Agent view requires an interactive terminal, so the flag was not exercised in a disposable non-interactive run.
  location: https://code.claude.com/docs/en/agent-view#dispatch-defaults
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'With thinking turned off, an effort level above high is rejected by the model: "API Error: Effort ''<level>'' isn''t available with thinking turned off on this model · run /effort high to continue, or turn thinking back on (unset MAX_THINKING_TOKENS=0)", with the hint varying by session (a non-interactive session reads ''use --effort high (or the effortLevel setting)''). Since v2.1.251 Claude Code sends effort high instead to models it knows reject the combination (such as Opus 5), so the error reaches the user only from a model Claude Code does not know rejects it.'
  id: docs-errors-effort-thinking
  limitations: Not reproduced locally; the tested model (Sonnet 5.5) cannot have thinking turned off.
  location: https://code.claude.com/docs/en/errors#effort-isnt-available-with-thinking-turned-off
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'Each assistant record in the session transcript carries an effort field with the level actually used for that request plus a perTurnEffort field holding the same level: --effort low recorded effort low and perTurnEffort low on claude-sonnet-5-5.'
  id: test-effort-transcript
  limitations: Run on one account whose default model is sonnet (resolving to claude-sonnet-5-5) with subscription auth on the Anthropic API; other models were not exercised.
  location: disposable claude -p --effort runs in /tmp/claude-effort-r2; transcripts under ~/.claude/projects/-private-tmp-claude-effort-r2/<session-id>.jsonl
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'With no effort control set, the transcript records effort medium for claude-sonnet-5-5, matching the documented model default. The host''s ~/.claude/settings.json carries effortLevel: high, and the run still used medium, confirming locally that a user-file effortLevel does not apply to Sonnet 5.5.'
  id: test-effort-default
  limitations: Confirms the Sonnet 5.5 default only; the Opus 5.5 (medium) and Opus 4.7 (xhigh) defaults rest on documentation.
  location: 'claude -p --max-turns 1 ''Reply with exactly: ok'' in /tmp/claude-effort-r2, transcript inspected afterwards'
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'With both the environment variable (high) and the launch flag (low) set, the transcript records effort high: the environment variable wins over the flag.'
  id: test-effort-env-precedence
  limitations: Tests env var versus flag only; the env-var-over-/effort and env-var-over-settings claims rest on documentation.
  location: 'CLAUDE_CODE_EFFORT_LEVEL=high claude -p --effort low --max-turns 1 ''Reply with exactly: ok'' in /tmp/claude-effort-r2'
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: '--effort ultracode is accepted on 2.1.284: exit 0, no warning on stderr, and the transcript records effort xhigh.'
  id: test-effort-ultracode
  limitations: A trivial prompt, so ultracode's workflow orchestration itself did not run.
  location: 'claude -p --effort ultracode --max-turns 1 ''Reply with exactly: ok'' in /tmp/claude-effort-r2'
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'An unknown --effort value does not stop or fail the run: exit code 0, the reply arrives, and stderr carries "Warning: Unknown --effort value ''bogus'' — ignoring it and using the default effort. Valid values: low, medium, high, xhigh, max."; the transcript shows the default effort (medium) in force. --effort auto is rejected the same way, although CLAUDE_CODE_EFFORT_LEVEL and /effort accept auto.'
  id: test-effort-invalid
  limitations: Tests the flag's unknown-token path; the model-unsupported and cap-clamp fallbacks rest on documentation.
  location: claude -p --effort bogus and claude -p --effort auto runs in /tmp/claude-effort-r2 (stderr captured)
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: '/effort works as a session command in a -p run with no model request: it prints ''Set effort level to high (this session only): Comprehensive implementation with extensive testing and documentation'' and exits 0. A bad argument produces the option list: ''Invalid argument: bogus. Valid options are: low, medium, high, xhigh, max, auto, ultracode [on|off]''.'
  id: test-effort-command
  limitations: The session-only (not saved) effect in -p mode rests on documentation and the command's own output.
  location: claude -p '/effort high' and claude -p '/effort bogus' runs in /tmp/claude-effort-r2
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'A --settings override carrying effortLevel low applies to claude-sonnet-5-5: the transcript records effort low, unlike the same key in the user settings file, which Sonnet 5.5 ignores.'
  id: test-effort-settings-override
  limitations: Tests --settings only; the project, local, and managed scopes of the same rule rest on documentation.
  location: 'claude -p --settings ''{"effortLevel": "low"}'' --max-turns 1 ''Reply with exactly: ok'' in /tmp/claude-effort-r2, transcript inspected afterwards'
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'The stream-json output nowhere states the effort level: the init system message carries only per_turn_effort_active: true (plus the command name in the slash-command list), the assistant messages carry model and content but no effort, and the result message (its full key set inspected) has no effort field. The transcript of the same run records effort max.'
  id: test-stream-json-no-effort
  limitations: Checked with --verbose and without --include-partial-messages; the partial-message delta stream was not inspected.
  location: claude -p --effort max --output-format stream-json --verbose runs in /tmp/claude-effort-r2 (stream captured)
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: In stream-json the assistant message contains a thinking content block whose thinking text is empty (with a 1244-character signature), the answer arrives in a second assistant message, and the result message reports the thinking token count at usage.output_tokens_details.thinking_tokens (112 for the test prompt). The reasoning text itself does not reach a non-interactive caller on the Anthropic API.
  id: test-thinking-redacted
  limitations: Subscription auth on the Anthropic API only; third-party providers and API-key auth were not tested (see gaps).
  location: claude -p --effort max with a reasoning-inducing prompt in /tmp/claude-effort-r2
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
- claim: 'The Bash tool subprocess inside the xhigh run saw CLAUDE_EFFORT=xhigh (the tool result of printenv CLAUDE_EFFORT was the bare word xhigh): Claude Code exports the in-effect effort level to tool subprocesses, giving a second channel besides the transcript to read the level a session actually used.'
  id: test-claude-effort-output-var
  limitations: One run at one level; the matching effort.level hook field was not exercised.
  location: 'claude -p --effort xhigh --allowedTools ''Bash(printenv *)'' with the prompt ''Run this exact command with the Bash tool: printenv CLAUDE_EFFORT'' in /tmp/claude-effort-r2'
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.284
support: some_models
levels:
- evidence_ids:
  - local-help-2-1-284
  - docs-model-config-effort
  - docs-settings-effortlevel
  meaning: 'Least reasoning: quick exchanges where you review each result, such as brainstorming, a first sketch, or a small change like a rename; fastest and cheapest.'
  native: low
  normalized: low
- evidence_ids:
  - docs-model-config-effort
  - docs-settings-effortlevel
  - test-effort-default
  meaning: The default on Opus 5.5 and Sonnet 5.5, where it fits day-to-day engineering with a clear scope; on other models it reduces token usage for cost-sensitive work that can trade off some intelligence.
  native: medium
  normalized: medium
- evidence_ids:
  - docs-model-config-effort
  - docs-settings-effortlevel
  - test-effort-command
  meaning: Balances token usage and intelligence; work where verification matters or edge cases are likely, such as fixing a bug in an existing codebase; the default on every effort model except Opus 5.5, Sonnet 5.5, and Opus 4.7.
  native: high
  normalized: high
- evidence_ids:
  - docs-model-config-effort
  - docs-settings-effortlevel
  - test-effort-transcript
  meaning: Deeper reasoning at higher token spend; the default on Opus 4.7; not accepted by Opus 4.6 or Sonnet 4.6.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - docs-model-config-effort
  - test-stream-json-no-effort
  meaning: The deepest reasoning level, for hard problems you want Claude to work through without you, such as finding security vulnerabilities; may show diminishing returns and is prone to overthinking. Applied to the current session only unless set through CLAUDE_CODE_EFFORT_LEVEL.
  native: max
  normalized: maximum
- evidence_ids:
  - docs-cli-reference
  - docs-model-config-effort
  - docs-settings-ultracode
  - test-effort-ultracode
  meaning: 'Not a model effort level but a compound agent mode: turns dynamic-workflow orchestration on. Since v2.1.284 it runs at whichever effort level the session uses, and only the --effort flag and the Agent SDK effortLevel value turn it on together with xhigh; the persisted effortLevel key and CLAUDE_CODE_EFFORT_LEVEL do not accept it, and it is unavailable when workflows are off or the model lacks xhigh.'
  native: ultracode
  normalized: outside_scale
default_level:
  decided_by: model
  evidence_ids:
  - docs-model-config-effort
  - test-effort-default
  native: high
controls:
- arguments:
  - --effort
  - <level>
  changes_running_session: no
  evidence_ids:
  - local-help-2-1-284
  - docs-cli-reference
  - test-effort-transcript
  - test-effort-ultracode
  - test-effort-invalid
  id: effort-launch-flag
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-env-vars
  - test-effort-env-precedence
  id: effort-env-var
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: CLAUDE_CODE_EFFORT_LEVEL
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-model-config-effort
  - test-effort-command
  id: effort-session-command
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: /effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-model-config-effort
  id: model-picker-effort-slider
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /model
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings-effortlevel
  - docs-settings-precedence
  id: effort-level-setting
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: effortLevel
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings-modelsettings
  - docs-settings-precedence
  id: model-effort-settings
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelSettings
  value: level_token
- arguments:
  - --settings
  - '{"effortLevel": "<level>"}'
  changes_running_session: no
  evidence_ids:
  - docs-settings-precedence
  - docs-settings-effortlevel
  - test-effort-settings-override
  id: settings-override-flag
  kind: config_override_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --settings
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-model-config-effort
  id: skill-subagent-effort-frontmatter
  kind: config_file_key
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-model-config-effort
  - docs-settings-ultracode
  id: sdk-effort-field
  kind: request_field
  lasts: unknown
  launch_modes:
  - non_interactive
  name: effortLevel
  value: level_token
- arguments:
  - agents
  - --effort
  - <level>
  changes_running_session: no
  evidence_ids:
  - docs-cli-reference
  - docs-agent-view
  - local-help-2-1-284
  id: agent-view-effort-default
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  name: --effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings-ultracode
  id: ultracode-setting
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: ultracode
  value: on_or_off
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-settings-maxeffortlevel
  - docs-model-config-org-limits
  id: max-effort-cap
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: maxEffortLevel
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-env-vars
  - docs-settings-thinking
  - docs-errors-effort-thinking
  id: max-thinking-tokens
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: MAX_THINKING_TOKENS
  value: token_budget
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings-thinking
  - docs-errors-effort-thinking
  id: always-thinking-enabled
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: alwaysThinkingEnabled
  value: on_or_off
precedence:
- max-effort-cap
- effort-env-var
- skill-subagent-effort-frontmatter
- effort-launch-flag
- effort-session-command
- model-picker-effort-slider
- settings-override-flag
- model-effort-settings
- effort-level-setting
models:
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - docs-model-config-effort
  model: claude-opus-4-6
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - docs-model-config-effort
  model: claude-sonnet-4-6
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - docs-model-config-effort
  model: claude-opus-5-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - docs-model-config-effort
  - test-effort-default
  model: claude-sonnet-5-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: xhigh
  evidence_ids:
  - docs-model-config-effort
  model: claude-opus-4-7
- accepts: []
  evidence_ids:
  - docs-env-vars
  model: claude-haiku-4-5
invalid_level:
  behavior: uses_default
  evidence_ids:
  - test-effort-invalid
  message: 'Warning: Unknown --effort value ''bogus'' — ignoring it and using the default effort. Valid values: low, medium, high, xhigh, max.'
  warns: yes
reporting:
  evidence_ids:
  - test-effort-transcript
  - test-effort-default
  - test-claude-effort-output-var
  - docs-model-config-effort
  - docs-env-vars
  field: /effort
  locator: ~/.claude/projects/<munged-cwd>/<session-id>.jsonl
  notes: 'The effort field sits on each assistant record in the transcript, one per model request, holding the level actually used; each record also carries a perTurnEffort field holding the same level. Interactively the level also shows in the session header next to the model name (for example ''with low effort'') and in the footer at startup and on change. The -p result message and stream-json output state no level (the init message carries only per_turn_effort_active), so a wrapper resolves the transcript from the result message''s session_id; transcripts are suppressed under --no-session-persistence or CLAUDE_CODE_SKIP_PROMPT_HISTORY. Two more channels carry the level: Bash tool subprocesses and hook commands receive it as the CLAUDE_EFFORT environment variable (verified xhigh in a subprocess), and hook payloads carry the same value as effort.level.'
  source: session_record
reasoning_output:
  evidence_ids:
  - test-thinking-redacted
  - docs-settings-thinking
  reaches_caller: hidden
gaps:
- area: reasoning_output
  detail: Whether the reasoning text reaches a non-interactive caller unredacted on third-party providers (Amazon Bedrock, Google Cloud's Agent Platform, Microsoft Foundry) or with API-key auth; the docs say third-party providers do not redact, but every local observation used the Anthropic API with subscription auth, where the thinking block arrives empty.
  next_check: Run claude -p --output-format stream-json with a thinking-inducing prompt against a third-party provider or an ANTHROPIC_API_KEY login and inspect the thinking block text.
- area: controls
  detail: Whether editing maxEffortLevel in a settings file takes effect in a session that has already started; the cap is enforced before each request and settings files hot-reload, but whether this key is re-read like permissions or once at session start like effortLevel and modelSettings is not documented.
  entry: max-effort-cap
  next_check: Edit maxEffortLevel in a settings file during an open session and read the effort field of the next assistant record in the transcript.
- area: controls
  detail: How long an effort level set through the Agent SDK apply_flag_settings effortLevel field stays in force — whether it survives session resume the way a persisted settings key does, or behaves like a per-session flag.
  entry: sdk-effort-field
  next_check: Send an effortLevel control request in an Agent SDK session, resume the session, and read the effort field of the next assistant record in the transcript.
- area: reporting
  detail: The meaning of the undocumented per_turn_effort_active field in the stream-json init message (observed true on claude-sonnet-5-5); it is not the effort level, and no level appears anywhere in the stream-json or -p result output.
  next_check: Consult the Agent SDK type definitions or a --debug run's internals for the field's producer to learn what it gates.
changes:
- 'Re-verified every local finding at the same version (2.1.284) with fresh disposable runs in a clean directory: --effort low and max recorded in the transcript, --effort ultracode accepted and recorded as xhigh, the medium default on claude-sonnet-5-5 (with the host''s user-file effortLevel: high still ignored), CLAUDE_CODE_EFFORT_LEVEL winning over --effort, the unknown-token warning with the default still in force (including the --effort auto rejection), the /effort command output and its option list, no effort level anywhere in stream-json, and the redacted empty thinking block with the thinking token count on the result message.'
- 'New findings: each assistant transcript record now also carries a perTurnEffort field holding the same level as effort; a --settings override carrying effortLevel was verified locally to apply to claude-sonnet-5-5 (transcript effort low), confirming the documented difference from the user-file key, which Sonnet 5.5 ignores; CLAUDE_CODE_DISABLE_THINKING is now documented as a gateway-compatibility variable that omits the thinking parameter; the CLAUDE_EFFORT subprocess export was re-verified at xhigh from the tool result itself.'
- 'Ultracode reclassified: v2.1.284 decoupled it from xhigh (the setting and /effort ultracode now leave the effort level unchanged; only --effort ultracode and the SDK effortLevel value turn it on with xhigh), and the provider''s own documentation now calls it a Claude Code setting rather than a model effort level, so under contract revision 2 it is recorded with normalized outside_scale instead of very_high.'
- 'Migrated to contract revision 2: gaps now name their area (and entry where one control is concerned), and environment-variable controls carry empty arguments per the revised arguments rule.'
requires_claudine_update: true
reason: 'The generated provider metadata is wrong on every axis: claudine/docs/providers/facts/claude.yaml feeds claudine/lib/src/provider/claude/data.rs a reasoning entry of NamedLevels { flag: thinking_effort, levels: [low, medium, high] }, but Claude Code 2.1.284 has no thinking_effort flag — the launch flag is --effort, the levels are low/medium/high/xhigh/max (plus the compound ultracode mode, outside the scale), CLAUDE_CODE_EFFORT_LEVEL outranks the flag, and the persisted keys are effortLevel and modelSettings. The facts file''s own header says the reasoning field graduates to its research topic when that topic lands (delete-on-graduate), so graduating this research requires deleting the facts key and regenerating. Claudine also needs the confirmation reader (the effort and perTurnEffort fields on assistant records in the transcript, with CLAUDE_EFFORT and the hook effort.level field as secondary channels) and a neutral-scale mapping that accounts for the model-dependent default, the outside-scale ultracode mode, and the silent clamps (model-unsupported levels, maxEffortLevel and organization caps) if it offers one effort setting across providers.'
contract_checked: 2026-09-29
---

# Reasoning Level Support in Claude Code

Claude Code exposes reasoning depth as **effort levels** — named levels that control
*adaptive reasoning*, where the model decides per step whether and how much to think.
Higher effort buys deeper reasoning at higher token spend; lower effort answers sooner
and cheaper. On current models effort is the primary reasoning control: thinking is
always on and its depth follows the level.

## Levels

The scale, weakest to strongest, plus one mode that is not on the scale:

| Level | Normalized | What it does |
|-------|------------|--------------|
| `low` | low | Least reasoning; quick exchanges you review yourself (brainstorming, a first sketch, a rename) |
| `medium` | medium | Default on Opus 5.5 and Sonnet 5.5; on other models, a cost-saving step down |
| `high` | high | The balance point and the default on most models; verification-heavy work |
| `xhigh` | very_high | Deeper reasoning at higher spend; default on Opus 4.7 |
| `max` | maximum | The deepest level, for hard unsupervised problems; session-scoped unless set via the environment variable |
| `ultracode` | outside_scale | Not a level: a Claude Code setting that turns dynamic-workflow orchestration on |

Three caveats worth knowing before touching the scale:

- **The scale is calibrated per model.** The same level name does not represent the
  same underlying value across models — Opus 5.5 at `medium` matches or exceeds
  Opus 5 at `high` in Anthropic's testing.
- **`ultracode` is a mode, not a point on the scale.** As of v2.1.284 it runs at
  whichever effort level the session uses: `/effort ultracode` and the `ultracode`
  setting leave the effort level unchanged, and only `--effort ultracode` and the
  Agent SDK `effortLevel` value turn it on *together with* `xhigh`. The persisted
  `effortLevel` key and `CLAUDE_CODE_EFFORT_LEVEL` do not accept it, and it is
  unavailable when workflows are off or the model lacks `xhigh`. A generic effort
  setting should not map to it.
- **`auto` is not a level** but a reset value: `CLAUDE_CODE_EFFORT_LEVEL=auto` and
  `/effort auto` mean "use the model default" (and the latter clears your saved
  level). `--effort auto` is *rejected* — the flag accepts only the five levels
  plus `ultracode`.

Observed discrepancy: local `claude --help` and the unknown-value warning list only
`low, medium, high, xhigh, max`, omitting `ultracode` (accepted since v2.1.203) —
trust the docs and the running binary over the help string.

## Choosing a Level

Every control, with one working example each. Interactive picks confirmed with
`Enter` save the level as your per-model default (`modelSettings`); `s` in the
`/effort` slider or `/model` picker applies it to the current session only
(v2.1.257+).

**Launch flag** — `--effort <level>`, one session, never persisted:

```bash
claude --effort high
claude -p --effort xhigh "summarize this failing test"
```

**Environment variable** — `CLAUDE_CODE_EFFORT_LEVEL`, read at startup, beats the
flag, the session command, and the settings keys; accepts `auto`:

```bash
CLAUDE_CODE_EFFORT_LEVEL=high claude
```

**Session command** — `/effort <level>` sets it directly, `/effort` opens the
slider, `/effort auto` clears your saved level, `/effort ultracode [off]` toggles
ultracode. It works while Claude is mid-turn (applies to the next request, after
a cache warning when one shows) and inside `-p` runs, where it is session-only:

```text
/effort high        → "Set effort level to high (this session only): Comprehensive
                       implementation with extensive testing and documentation"
```

**Model picker slider** — in `/model`, the left/right arrows adjust the effort
slider for the selected model.

**Settings keys** — `effortLevel` (top-level default; `low`/`medium`/`high`/`xhigh`
only — no `max`, no `ultracode`) and `modelSettings` (the per-model saved level;
what `/effort` writes since v2.1.251):

```json
{
  "effortLevel": "high",
  "modelSettings": { "claude-opus-5-5": { "effortLevel": "high" } }
}
```

A user-file `effortLevel` is ignored by Opus 5.5 and later models — confirmed
locally in both directions: this host's user settings carry `effortLevel: high`,
yet a run with no other control still used Sonnet 5.5's own `medium` default;
passing `--settings '{"effortLevel": "low"}'` on the same host did apply
(`low` recorded). In project, local, and managed settings (and via `--settings`)
the key applies to every model. Both keys are read once at session start; use
`/effort` to change effort mid-session.

**Config override flag** — `--settings` with inline JSON, ranked between managed
settings and every file below them:

```bash
claude --settings '{"effortLevel": "low"}'
```

**Skill and subagent frontmatter** — an `effort:` key in a skill's or subagent's
Markdown frontmatter overrides the session level while that skill or subagent
runs, but not the environment variable, and an effort cap still limits it.

**SDK request field** — the Agent SDK accepts `effortLevel` (including
`ultracode`) in an `applyFlagSettings()` control request, which can also change
it mid-session.

**Agent-view dispatch default** — `claude agents --effort <level>` sets the
effort default for every background session dispatched from agent view, taking
the same values as the top-level flag including `ultracode`:

```bash
claude agents --model opus --effort high
```

A background session's effort persists across supervisor restarts, and a
mid-session `/effort` change is kept; a session that took its effort from
settings re-reads them on each process start.

**Cap** — `maxEffortLevel` in any settings scope (managed settings to enforce
organization-wide) caps the level: anything higher — from any source, including
the model's own default — runs at the cap. The lowest cap across scopes wins
(with a per-model `max` in `modelSettings` exempting that model from that
source's cap), and Claude Enterprise organizations can additionally cap per
role, the lower of the two applying.

Two adjacent controls gate *thinking* rather than the level: `MAX_THINKING_TOKENS`
(the fixed thinking budget, `0` to disable thinking; ignored on adaptive-reasoning
models) and `alwaysThinkingEnabled: false` (thinking off). Neither has any effect
on Opus 5.5, Sonnet 5.5, or the Fable models, which cannot have thinking turned
off. With thinking off, Claude Code itself sends `high` instead of a higher level
on models it knows reject the combination, such as Opus 5. A third, newly
documented variable — `CLAUDE_CODE_DISABLE_THINKING=1` — omits the `thinking`
parameter from requests entirely as a gateway-compatibility option.

Also observed, with no control kind of their own: a Remote Control session
can be steered from the effort control on a connected phone or browser
(session-only, v2.1.234+); and the `ultrathink` keyword, typed anywhere in a
prompt, asks for deeper reasoning on that single turn *without* changing the
effort level sent to the API — the phrases "think", "think hard", and "think
more" are not recognized at all.

One control shape this provider does not have: a suffix on the model name. The
only model-name suffix is `[1m]` (a 1M-token context window variant), and it
selects context, not a reasoning level — checked against the model-configuration
page's alias and suffix tables.

### Precedence

Strongest to weakest, with the cap applied after resolution:

1. `maxEffortLevel` cap (and an organization effort limit) — clamps whatever wins below
2. `CLAUDE_CODE_EFFORT_LEVEL`
3. Skill/subagent `effort` frontmatter, while it runs
4. `--effort` at launch, or `/effort` / the `/model` slider in the session —
   whichever acted last
5. The `--settings` JSON, then the saved `modelSettings` level, then `effortLevel`,
   with settings files layered managed > `--settings` > local > project > user
   and a model's `modelSettings` entry beating a top-level `effortLevel` in the
   same file
6. The model's default

```mermaid
flowchart TD
    START[Request about to send] --> R1
    R1{"CLAUDE_CODE_EFFORT_LEVEL set?<br/>(auto means the model default)"} -- yes --> CAP{"Above a maxEffortLevel cap or<br/>organization effort limit?"}
    R1 -- no --> R2{"Skill or subagent with<br/>effort frontmatter running?"}
    R2 -- yes --> CAP
    R2 -- no --> R3{"--effort at launch, or /effort or<br/>the /model slider used this session?"}
    R3 -- yes --> CAP
    R3 -- no --> R4{"A saved level for the model<br/>(modelSettings) or effortLevel key?"}
    R4 -- yes --> CAP
    R4 -- no --> DEF["Model default: high; medium on<br/>Opus 5.5 and Sonnet 5.5; xhigh on Opus 4.7"]
    DEF --> CAP
    CAP -- yes --> CLAMP["Run at the cap"]
    CAP -- no --> OK{"Model accepts this level?"}
    CLAMP --> OK
    OK -- no --> NEAR["Fall back to the highest supported<br/>level at or below it"]
    OK -- yes --> SEND[Send the request]
    NEAR --> SEND
```

## Models

Only the models below differ from the full five-level list; **models not listed
have no adjustable reasoning** (Haiku 4.5, Claude 3 models, Sonnet 4.0/4.5, and
Opus 4.0/4.1 reject the effort parameter at the API outright):

| Model | Levels | Default |
|-------|--------|---------|
| `claude-fable-5-1`, `claude-fable-5`, `claude-opus-5`, `claude-sonnet-5`, `claude-opus-4-8` | all five | `high` |
| `claude-opus-5-5`, `claude-sonnet-5-5` | all five | `medium` |
| `claude-opus-4-7` | all five | `xhigh` |
| `claude-opus-4-6`, `claude-sonnet-4-6` | `low`, `medium`, `high`, `max` — no `xhigh` | `high` |

The default therefore depends on the model, not the account. One account-level
exception: an organization can set a default effort for its organization default
model, and that level is the default when you run that model. When a model does
not support the level you set, Claude Code silently falls back to the highest
supported level at or below it (`xhigh` runs as `high` on Opus 4.6); above an
effort cap it runs at the cap, with a warning naming both levels in interactive
sessions and plain-text `--print` runs but silently under `json`/`stream-json`
output.

For gateway or custom model IDs that Claude Code cannot recognize (Bedrock ARNs,
deployment names), capability detection fails and effort appears unavailable;
`CLAUDE_CODE_ALWAYS_ENABLE_EFFORT=1` sends the effort parameter anyway, and the
`ANTHROPIC_DEFAULT_*_MODEL_SUPPORTED_CAPABILITIES` variables declare specific
capabilities (`effort`, `xhigh_effort`, `max_effort`, `thinking`, ...) per pinned
model.

## Confirming the Level

There is **no effort field in the `-p` result message or anywhere in `stream-json`
output** — the init message carries only an undocumented `per_turn_effort_active`
flag, and the level a run used is not on the wire. Three places state it:

- **The session transcript** (authoritative): every assistant record in
  `~/.claude/projects/<munged-cwd>/<session-id>.jsonl` carries an `effort` field
  with the level actually used for that request, plus a `perTurnEffort` field
  holding the same level. Verified by setting `low`, `max`, and `ultracode`
  (recorded as `xhigh`) explicitly and reading each level back, and by observing
  `medium` with no control set on Sonnet 5.5.
- **The interactive session header**: the level shows next to the model name (for
  example "with low effort"), and the footer shows it at startup and on change.
- **Subprocess and hook channels**: Bash tool subprocesses and hook commands
  receive the in-effect level as the `CLAUDE_EFFORT` environment variable
  (verified: the `printenv CLAUDE_EFFORT` tool result inside an `xhigh` run was
  the bare word `xhigh`), and hook payloads carry the same value as
  `effort.level`. Both are set only when the model supports effort.

For a wrapper, the practical confirmation is: resolve the transcript path for the
session ID (the `-p` result message carries `session_id`), then read `/effort` off
the assistant records. Note that transcripts are suppressed under
`--no-session-persistence` or `CLAUDE_CODE_SKIP_PROMPT_HISTORY`.

## Reasoning Output

A non-interactive caller does **not** receive the reasoning text. On the Anthropic
API the assistant message contains a `thinking` content block whose text arrives
redacted — an empty string plus a signature (1244 characters in the observed
test) — and only the token count is reported on the result message
(`usage.output_tokens_details.thinking_tokens`, 112 for the observed run). The
transcript stores the same empty block, and `showThinkingSummaries: true` changes
nothing in the stream; it only affects the interactive `Ctrl+O` expansion.
Documentation notes that third-party providers do not redact, which was not
observable from this host (see gaps).

## Sources

- Local inspection, Claude Code 2.1.284 on macOS: `claude --version`, `claude --help`, `claude agents --help`, and disposable `claude -p` runs (valid, invalid, `auto`, and `ultracode` levels; `/effort`; env-var precedence; `--settings` override; stream-json; transcripts; the `CLAUDE_EFFORT` subprocess export)
- [CLI reference](https://code.claude.com/docs/en/cli-reference) — the `--effort` flag
- [Model configuration: adjust effort level](https://code.claude.com/docs/en/model-config#adjust-effort-level) — levels, model table, defaults, resolution order, `/effort`, frontmatter, `ultrathink`, gateway capability variables
- [Model configuration: organization effort limits](https://code.claude.com/docs/en/model-config#organization-effort-limits)
- [Agent view: dispatch defaults](https://code.claude.com/docs/en/agent-view#dispatch-defaults) — `claude agents --effort`, effort persistence across restarts
- [Settings reference](https://code.claude.com/docs/en/settings-reference) — `effortLevel`, `modelSettings`, `maxEffortLevel`, `ultracode`, `alwaysThinkingEnabled`, `showThinkingSummaries`
- [Settings files and precedence](https://code.claude.com/docs/en/settings)
- [Environment variables](https://code.claude.com/docs/en/env-vars) — `CLAUDE_CODE_EFFORT_LEVEL`, `MAX_THINKING_TOKENS`, `CLAUDE_CODE_DISABLE_THINKING`, `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING`, `CLAUDE_CODE_ALWAYS_ENABLE_EFFORT`, `CLAUDE_EFFORT`
- [Error reference: effort with thinking turned off](https://code.claude.com/docs/en/errors#effort-isnt-available-with-thinking-turned-off)

## Changelog

- **2026-09-29 (revision 2)** — re-verified every local finding at 2.1.284 with
  fresh disposable runs in a clean directory, and migrated the document to
  contract revision 2 (`area`/`entry` gaps, empty `arguments` for
  environment-variable controls). New findings: the `perTurnEffort` transcript
  field; local verification that a `--settings` `effortLevel` applies to Sonnet
  5.5 while the user-file key does not; `CLAUDE_CODE_DISABLE_THINKING`.
  `ultracode` reclassified from `very_high` to `outside_scale` following the
  v2.1.284 semantics change (the setting and `/effort ultracode` now leave the
  effort level unchanged) and the provider's own "setting rather than a level"
  wording.
- **2026-09-29 (re-verification)** — every finding re-established at 2.1.284
  with fresh disposable runs. Added: local confirmation that a user-file
  `effortLevel` is ignored on Sonnet 5.5 (the host settings carry `high`, the
  default run used `medium`); the `CLAUDE_EFFORT` subprocess export and the
  matching `effort.level` hook field as confirmation channels; the thinking
  token count on the result message; and the `claude agents --effort`
  dispatch-default control. Fixed the contract shape of `invalid_level`
  (`uses_default` with `warns: yes`) and reduced `reporting.locator` to the
  transcript path pattern.
- **2026-09-29 (first version)** — researched against Claude Code 2.1.284 on
  macOS with disposable non-interactive runs. Established the level list including
  the compound `ultracode` value, the full control surface, the model-dependent
  default (`high`, with `medium` on Opus 5.5/Sonnet 5.5 and `xhigh` on Opus 4.7),
  the precedence order (environment variable over launch flag, cap applied last),
  the unknown-token fallback (warning on stderr, default effort in force), the
  transcript `effort` field as the confirmation source, and the redacted thinking
  block in non-interactive output.