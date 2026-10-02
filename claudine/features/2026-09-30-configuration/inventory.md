---
prompt: |-
    
---

Claudine reads two files, both parsed as JSON5 (`biscuit_file::Json5`) and then deserialized with `deny_unknown_fields`:

- **user**: `~/.claudine/config.json`, deserialized as `ClaudineConfig` (`lib/src/config/claudine_config.rs:186`)
- **repo**: `{repo-root}/.claudine/config.json`, deserialized as `RepoOverrideConfig` (`lib/src/config/claudine_config.rs:343`). It is merged into the user config by `merge_repo_override` (`lib/src/config/merge.rs:21`), and it is skipped when it resolves to the same file as the user config

Loading, merging, and validation happen in `load_claudine_config` (`lib/src/dispatch/loader.rs:387`). A file in the pre-2026 per-provider format (`version` + `providers`, or top-level provider keys) is renamed to `.bak` and ignored (`lib/src/config/migration.rs:23`). Types below use [`SimplifiedSchema`](../../../darkmatter/docs/topics/schemas/definition.md) notation; `A | B` marks an untagged union.

The `provider` enum used in several places is the serde form of `Provider` (`lib/src/provider_id.rs:26`): `enum(claude, codex, gemini, goose, kimi_code, open_code, qwen_code, kilo, pi, antigravity)`.

The `event` enum is the serde form of `AgenticEvent` (`lib/src/events/agentic_event.rs:14`): `enum(session_start, session_end, before_prompt, before_tool, after_tool, tool_error, permission_request, turn_complete, turn_error, subagent_start, subagent_stop, before_model, after_model, before_compact, notification, human_in_the_loop)`.

## Configuration items

- **`tts`**: enables text-to-speech for `speak` hook actions and lifecycle `say` effects, and optionally pins the TTS provider and voice. `true` auto-detects the provider and voice. `false` turns off all `speak` actions.
    - **Type:** `boolean | { provider: string(required), voice: string | { male: string, female: string }, gender: enum(male, female) }`
    - **Default:** `false` · **Scope:** user only
    - **Used in:**
        - `lib/src/config/tts.rs:1` (`TtsValue`, `TtsConfigSettings`, `VoiceSelection`, `Gender`)
        - `lib/src/dispatch/runner/speak.rs:30`: a `false` value skips the `speak` action
        - `lib/src/dispatch/runner/speak.rs:52`: `tts_config_from_claudine` resolves the provider, a single or gendered voice, and the gender
        - `lib/src/dispatch/loader.rs:216`: `bridge_tts_settings` converts this value to `events::TtsSettings` for composition lifecycle audio
        - `cli/src/commands/wrap/composition/staged_boot.rs:165`, `cli/src/commands/wrap/composition/pipeline.rs:1324`: callers of the bridge
        - `lib/src/composition/lifecycle/audio.rs:137`, `lib/src/composition/lifecycle/executor.rs:1108`, `lib/src/composition/lifecycle/executor.rs:1421`, `lib/src/composition/lifecycle/mod.rs:898`: consumers of the bridged settings
        - `cli/src/commands/config_tui/tabs/tts.rs:12`: config TUI editor
    - `tts.provider`: the TTS engine name, parsed by `biscuit_speaks::parse_provider_name` (`biscuit-speaks/lib/src/detection.rs:136`). An unrecognized name is silently ignored rather than rejected
        - **Type:** `string`, in practice `enum(say, espeak, piper, echogarden, sherpa, mimic3, festival, gtts, sapi, kokoro, pico, spd, elevenlabs)`; each engine also accepts aliases
    - `tts.voice`: either one voice name, or a `{ male, female }` pair that `tts.gender` chooses between. A `voice` on an individual `speak` action takes precedence
        - **Type:** `string | { male: string(required), female: string(required) }`
    - `tts.gender`: selects the voice from a gendered pair. A `gender` on an individual `speak` action takes precedence
        - **Type:** `enum(male, female)` · **Default:** `female`

- **`messenger`**: named outbound messaging routes (Discord, Slack, Signal, WhatsApp, and Discord/Slack webhooks) and the currently active route. `message` hook actions send to the active route.
    - **Type:** `{ active_config: string, configurations: object }`
    - **Default:** absent (no messaging) · **Scope:** user (the repo can only override `active_config`; see `active_messenger`)
    - **Used in:**
        - `lib/src/config/messaging_block.rs:1` (`ClaudineMessengerConfig`, `MessengerProviderConfig`, `validate`)
        - `lib/src/dispatch/loader.rs:166`, `lib/src/dispatch/loader.rs:257`, `lib/src/dispatch/loader.rs:286`: converts the block to `RuntimeMessagingSettings` / `MessagingRouteConfig`
        - `lib/src/messaging/resolve.rs:64`: `resolve_effective_route`, where the repo scope takes precedence over the user scope
        - `lib/src/messaging/resolve.rs:114`: `resolve_secret` reads an inline secret, otherwise the named environment variable
        - `lib/src/messaging/send.rs:131`: `execute_message`
        - `lib/src/dispatch/mod.rs:338`: passes the settings to the action runner
        - `cli/src/commands/config_tui/tabs/messenger/`: config TUI route manager
    - `messenger.active_config`: the key of the active route in `configurations`. Validation fails when it is blank or names a key that does not exist
        - **Type:** `string`
    - `messenger.configurations`: map from route name to route definition. The `provider` field selects the route kind and determines its remaining fields. The `*_env` fields name environment variables that hold the secret or connection value
        - **Type:** `object` of `{ provider: enum(discord, slack, signal, whatsapp, discord_webhook, slack_webhook), ... }` (`discord-webhook` and `slack-webhook` are accepted as aliases)
        - `discord`: `{ channel_id: string(required), bot_token_env: string(default(DISCORD_BOT_TOKEN)) }`
        - `slack`: `{ channel_id: string(required), bot_token_env: string(default(SLACK_BOT_TOKEN)) }`
        - `signal`: `{ recipient: string(required), rpc_url_env: string(default(SIGNAL_RPC_URL)), account_env: string(default(SIGNAL_ACCOUNT)) }`
        - `whatsapp`: `{ recipient: string(required), access_token_env: string(default(WHATSAPP_ACCESS_TOKEN)), phone_number_id_env: string(default(WHATSAPP_PHONE_NUMBER_ID)) }`
        - `discord_webhook`: `{ webhook_url: url, webhook_url_env: string(default(DISCORD_WEBHOOK_URL)) }`. `webhook_url` must be a valid Discord webhook URL
        - `slack_webhook`: `{ webhook_url: url, webhook_url_env: string(default(SLACK_WEBHOOK_URL)) }`. `webhook_url` must be a valid Slack webhook URL

- **`active_messenger`** *(repo only)*: overrides `messenger.active_config` for this repository. `null` disables messaging in the repository. An absent key inherits the user setting.
    - **Type:** `string` (nullable; an explicit `null` is different from an absent key)
    - **Default:** absent (inherit) · **Scope:** repo only
    - **Used in:**
        - `lib/src/config/claudine_config.rs:370` (custom deserializer at `:406`)
        - `lib/src/config/merge.rs:38`: overwrites `messenger.active_config`, but only when a user `messenger` block exists
        - `cli/src/commands/config_tui/tabs/messenger/modal.rs:51`, `cli/src/commands/config_tui/tabs/messenger/input.rs:32`, `cli/src/commands/config_tui/tabs/messenger/render.rs:71`: config TUI

- **`logging`**: turns on the JSONL event log. When it is on, every dispatched hook event is appended to `~/.claudine/logs/YYYY-MM-DD.jsonl`, whether or not any actions are bound to that event.
    - **Type:** `boolean`
    - **Default:** `true` · **Scope:** user only
    - **Used in:**
        - `lib/src/dispatch/mod.rs:350`: gates `log_dispatch_event` (`lib/src/dispatch/logging.rs:34`)
        - `cli/src/commands/config_tui/tabs/services.rs:108`, `cli/src/commands/config_tui/tabs/services.rs:356`: config TUI toggle

- **`protect`**: the Protect service, a regex deny catalog checked against Bash commands, write/edit paths, and MCP responses during hook dispatch. `true` or `false` is shorthand for turning the whole service on or off. The object form allows per-group control and custom patterns.
    - **Type:** `boolean | { enabled: boolean, rules: object, custom_patterns: object[] }`
    - **Default:** `true` (all groups on) · **Scope:** user only
    - **Used in:**
        - `lib/src/protect/config.rs:13` (`ProtectConfig`, custom deserializer, `validate`)
        - `lib/src/dispatch/loader.rs:153`: constructs `ProtectService` only when `enabled` is true
        - `lib/src/dispatch/mod.rs:267`, `lib/src/dispatch/mod.rs:358`: protect pre- and post-evaluation
        - `lib/src/protect/matcher.rs:141`: `is_group_enabled` filters the rule groups
        - `lib/src/protect/service.rs:88`, `lib/src/protect/service.rs:158`: `get_allow_paths`
        - `cli/src/commands/hooks/mod.rs:86`: `claudine hooks` shows whether Protect is on
        - `cli/src/commands/init/mod.rs:277`: init wizard
        - `cli/src/commands/config_tui/tabs/services.rs:111`, `cli/src/commands/config_tui/tabs/services.rs:360`: config TUI
    - `protect.enabled`: master switch
        - **Type:** `boolean` · **Default:** `true`
    - `protect.rules.<group>`: turns on or off one rule group from the built-in catalog. The object form also takes `allow_paths`, which is accepted only for `filesystem_destruction` and `sensitive_paths`; validation rejects it for any other group
        - **Type:** `boolean | { enabled: boolean(required), allow_paths: string[] }`
        - **Groups:** `enum(filesystem_destruction, disk_manipulation, remote_execution, git_destructive, system_sabotage, network_sabotage, container_cloud, database_nukes, obfuscated_execution, prompt_injection, credential_exfiltration, sensitive_paths)` (`lib/src/protect/config.rs:155`)
        - **Default:** each group is on unless it is listed with `false`
    - `protect.custom_patterns`: user-authored deny regexes. Each pattern must compile, and `write_path` is rejected as a surface
        - **Type:** `{ name: string, pattern: string, surface: enum(bash_command, mcp_response) }[]`
        - **Default:** `[]`. `surface` defaults to `bash_command`
        - **Used in:** `lib/src/protect/matcher.rs:186`, `lib/src/protect/config.rs:138` (validation)

- **`actions`**: cross-provider hook bindings, as a map from event to an ordered list of actions. The set of event keys also decides which native hooks `claudine sync` registers with each provider. In the repo file, an event entry replaces the user's entry for that same event.
    - **Type:** `object` keyed by `event`, of `action[]` (see the action variants below)
    - **Default:** `{}` · **Scope:** user and repo (repo replaces per event)
    - **Used in:**
        - `lib/src/dispatch/loader.rs:113`: `compile_canonical_runtime` builds a `RuntimeEventBinding` for each event
        - `lib/src/dispatch/mod.rs:303`: dispatch runs the actions of the bound event
        - `lib/src/dispatch/runner/mod.rs:152`: evaluates each action's `when` condition, then runs the action
        - `lib/src/config/merge.rs:28`: repo replaces the user entry per event
        - `cli/src/commands/sync.rs:183`, `cli/src/commands/sync.rs:193`: the expected native hook registrations come from the event keys
        - `cli/src/commands/hooks/list.rs:35`, `cli/src/commands/hooks/list.rs:112`, `cli/src/commands/hooks/list.rs:398`: `claudine hooks`
        - `cli/src/commands/actions.rs:63`, `cli/src/commands/actions.rs:174`: `claudine actions`
        - `cli/src/commands/init/mod.rs:141`: init wizard seeds recommended actions
        - `cli/src/commands/config_tui/tabs/actions/`: config TUI
    - **Action variants** (`HookAction`, `lib/src/actions/hook_action.rs:116`). The `type` field selects the variant, and every variant accepts an optional `when: expression`, a Darkmatter condition (`lib/src/dispatch/runner/mod.rs:95`):
        - `sound_effect`: `{ effect: string(required), volume: number(default(1.0)), speed: number(default(1.0)) }`. Plays a Playa sound effect (`lib/src/dispatch/runner/mod.rs:340`, `:430`)
        - `speak`: `{ message: string(required), voice: string, gender: enum(male, female) }`. Speaks a message template through TTS (`lib/src/dispatch/runner/mod.rs:172`, `lib/src/dispatch/runner/speak.rs:18`)
        - `bash`: `{ command: string(required), params: string(default("")) }`. Runs fire-and-forget with a fixed 3 s timeout (`lib/src/dispatch/runner/mod.rs:205`, `lib/src/dispatch/runner/bash.rs:30`)
        - `call`: `{ command: string(required), args: string[], timeout_ms: number(integer; default(60000)), mapper: { type: enum(json_field, json_object, exit_code, regex), field: string, pattern: string } }`. A blocking call that can return a hook decision (`lib/src/dispatch/runner/mod.rs:222`). The mapper regex is compiled at load time (`lib/src/dispatch/loader.rs:202`)
        - `report`: `{ handler: { format: enum(text, json, compact)(required), template: string, include_metadata: boolean(default(false)) } }`. Writes a report (`lib/src/dispatch/runner/mod.rs:192`, `lib/src/dispatch/runner/report.rs:8`)
        - `message`: `{ message: string(required), image: string }`. Sends to the active messenger route (`lib/src/dispatch/runner/mod.rs:358`, `lib/src/messaging/send.rs:131`)

- **`matchers`**: an optional filter for each event, stored as a string. The string is first parsed as a Darkmatter condition (`tool_name == 'Bash'`). If that fails, it is compiled as a regex (`Bash|Edit`). If both fail, a warning is logged and the binding fires unconditionally. A matcher can exist without any actions.
    - **Type:** `object` keyed by `event`, of `string`
    - **Default:** `{}` · **Scope:** user and repo (repo replaces per event)
    - **Used in:**
        - `lib/src/dispatch/loader.rs:102`, `lib/src/dispatch/loader.rs:136`: compiled with `compile_many` (`lib/src/dispatch/matcher.rs:89`)
        - `lib/src/dispatch/mod.rs:299`: `matcher::matches` gates the binding
        - `lib/src/config/merge.rs:33`: repo replaces the user entry per event

- **`preferred_agent`**: the favorite provider, used by `compose`, `inline-compose`, and `sequence` when the document does not choose one. Also accepted as `favorite_agent` when read.
    - **Type:** `provider`
    - **Default:** absent · **Scope:** user only
    - **Used in:**
        - `cli/src/commands/wrap/composition/selection.rs:46`: loaded into `SelectionConfig.favorite`
        - `cli/src/commands/wrap/composition/target.rs:111`, `cli/src/commands/wrap/composition/target.rs:226`: provider resolution
        - `cli/src/commands/wrap/sequence/mod.rs:357`: sequence provider resolution
        - `cli/src/commands/init/mod.rs:512`, `cli/src/commands/init_wizard.rs:254`: init wizard
        - `cli/src/commands/config_tui/tabs/preferences.rs:250`, `cli/src/commands/config_tui/tabs/preferences.rs:344`: config TUI

- **`canonical_provider`**: the canonical provider for shared-resource linking (skills, commands, agents) in this scope.
    - **Type:** `provider`
    - **Default:** absent · **Scope:** user and repo (repo value wins)
    - **Used in:**
        - `lib/src/config/merge.rs:23`: the repo value overrides the user value
        - `cli/src/commands/link_display.rs:33`: `repo_canonical_needs_init` makes `skills --fix`, `commands --fix`, and `agents --fix` stop when the repo has no value (`cli/src/commands/skills.rs:47`, `cli/src/commands/slash_commands.rs:44`, `cli/src/commands/agents.rs:44`)
        - `cli/src/commands/link_display.rs:47`: `render_canonical_providers` shows the user and repo values
        - `cli/src/commands/init/mod.rs:160`, `cli/src/commands/init/mod.rs:481`: init sets it to the top-ranked installed provider
        - `cli/src/commands/config_tui/tabs/preferences.rs:261`, `cli/src/commands/config_tui/tabs/preferences.rs:377`, `cli/src/commands/config_tui/tabs/preferences.rs:415`: config TUI (user and repo)

- **`models`**: per-provider overrides for the model catalog used to validate the frontmatter `model` hint during composition. A bare list adds to the fetched catalog. The object form can instead replace it.
    - **Type:** `object` keyed by `provider`, of `string[] | { mode: enum(add, replace), values: (string | { id: string(required), catalog_id: string })[] }`
    - **Default:** `{}`. `mode` defaults to `add` · **Scope:** user only
    - **Used in:**
        - `lib/src/config/claudine_config.rs:63` (`ProviderModelOverride`, `DetailedModelOverride`, `ModelOverrideValue`, `ModelOverrideMode`)
        - `lib/src/model_catalog/config.rs:15`: `merge_overrides` applies add or replace
        - `lib/src/model_catalog/service.rs:129`, `lib/src/model_catalog/service.rs:165`: `with_overrides`, `from_config`
        - `cli/src/commands/wrap/composition/selection.rs:47`: `SelectionConfig.model_overrides`
        - `cli/src/commands/wrap/composition/target.rs:107`, `cli/src/commands/wrap/composition/target.rs:218`, `cli/src/commands/wrap/sequence/mod.rs:346`, `cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:630`: model validation during launch
        - `cli/src/commands/wrap/catalog_drift.rs:19`: listing-drift check

- **`default_sounds`**: sound effects played after dispatch for three outcome categories. `success` plays on `session_end` and `turn_complete`, `attention` plays on `human_in_the_loop`, and `error` plays when Protect blocked the event.
    - **Type:** `{ success: string, attention: string, error: string }`. Each value must be a `playa::SoundEffect` name, one of the 88 embedded effects, checked at load time
    - **Default:** all absent · **Scope:** user only
    - **Used in:**
        - `lib/src/config/claudine_config.rs:459`: validated by `validate_sound_name`
        - `lib/src/dispatch/runner/mod.rs:402`: `default_sound_for_event`
        - `lib/src/dispatch/mod.rs:389`: `play_default_sound_for_event`
        - `cli/src/commands/config_tui/tabs/preferences.rs:119`, `cli/src/commands/config_tui/tabs/preferences.rs:469`: config TUI

- **`prompt_for_missing`**: when a composition document is missing required `$schema` properties, prompts for them interactively (this needs stdin and stderr on a TTY and no `--silent`). When `false`, the run fails with `MissingProperties` instead.
    - **Type:** `boolean`
    - **Default:** `true` · **Scope:** user only (read with `load_claudine_config(None, None)`, so the repo is never consulted)
    - **Used in:**
        - `cli/src/commands/schema_interactive/mod.rs:57`: `resolve_interactive_options`
        - `lib/src/composition/schema/mod.rs:84`, `lib/src/composition/schema/mod.rs:118`: `InteractiveSchemaOptions` gate
        - `lib/src/composition/error/render/schema.rs:227`: wording of the error hint
        - `cli/src/commands/config_tui/tabs/preferences.rs:151`, `cli/src/commands/config_tui/tabs/preferences.rs:318`, `cli/src/commands/config_tui/mod.rs:140`: config TUI

- **`harvest_unmatched`**: opt-in. Wrapped runs append scrubbed error- or warning-class signal payloads that matched no detection record to `~/.claudine/harvest/<provider>/<date>.jsonl`.
    - **Type:** `boolean`
    - **Default:** `false` · **Scope:** user only
    - **Env override:** `CLAUDINE_HARVEST` (`1`/`true`/`0`/`false`) wins over the file (`lib/src/signals/harvest.rs:158`)
    - **Used in:**
        - `cli/src/commands/wrap/policy.rs:106`: `harvest_enabled`
        - `lib/src/signals/harvest.rs:1`: harvest implementation
        - Not shown in the config TUI

- **`exit_expressions`**: runaway-output guard rules. A literal or regex that appears in the agent's streamed output aborts the run. Rules come from three layers (user, repo, and document frontmatter). The bare-array form uses the layer's default combine mode; the object form sets the mode explicitly.
    - **Type:** `entry[] | { mode: enum(merge, override), rules: entry[] }`, where `entry` is `{ pattern: string, patterns: string[], kind: enum(literal, regex), ignore_case: boolean, scope: string }`. An entry must set exactly one of `pattern` or `patterns`
    - **Default:** absent (no rules). The repo layer's `mode` defaults to `override`, and the frontmatter layer's to `merge` · **Scope:** user, repo, and frontmatter
    - `kind` defaults to `literal`. `ignore_case` defaults to `false` and applies to literals only. `scope` is `{agent}` or `{agent}/{model}` and is split on the first `/`; when it is absent, the entry applies globally. An unknown agent fails validation
    - **Used in:**
        - `lib/src/runaway/config.rs:57` (`ExitExpressionEntry`), `lib/src/runaway/config.rs:164` (`ExitExpressionsLayer`), `lib/src/runaway/config.rs:182` (`ExitExpressionsValue`)
        - `lib/src/runaway/config.rs:324`: `resolve_exit_expressions` merges the three layers
        - `lib/src/runaway/config.rs:476`: `validate_exit_expressions`
        - `lib/src/runaway/config.rs:364`: `extract_frontmatter_exit_expressions`
        - `cli/src/commands/wrap/runaway_guard.rs:259`, `cli/src/commands/wrap/runaway_guard.rs:325`: `resolve_guard_inputs` reads the user and repo files directly
        - `cli/src/commands/wrap/runaway_guard.rs:134`: `compile_for_model`
        - `lib/src/config/merge.rs:49`: the generic merge replaces the user value wholesale, which is a different rule from the mode-aware resolver above

- **`guard_settings`**: thresholds and on/off switches for the two runaway volume guards, repetition (group cycles) and volume (lines/bytes). Precedence is last-writer-wins: frontmatter, then repo, then user, then the built-in values. A repo value replaces the whole user object; sub-fields are not merged.
    - **Type:** `{ repetition: { enabled: boolean, max_repeats: number(integer; min(0)), max_cycle_length: number(integer; min(0)) }, volume: { enabled: boolean, max_lines: number(integer; min(0)), max_bytes: number(integer; min(0)) } }`
    - **Default:** `repetition = { enabled: true, max_repeats: 30, max_cycle_length: 16 }` and `volume = { enabled: true, max_lines: 50000, max_bytes: 33554432 }` (`lib/src/runaway/mod.rs:74`) · **Scope:** user, repo, and frontmatter
    - **Used in:**
        - `lib/src/runaway/config.rs:394` (`GuardSettings`, `RepetitionGuardSettings`, `VolumeGuardSettings`)
        - `lib/src/runaway/config.rs:454`: `resolve_guard_settings`
        - `cli/src/commands/wrap/runaway_guard.rs:150`: builds the detector configuration
        - `cli/src/commands/wrap/runaway_guard.rs:378`: frontmatter layer
        - `lib/src/runaway/detector.rs:207`, `lib/src/runaway/detector.rs:302`, `lib/src/runaway/detector.rs:336`, `lib/src/runaway/detector.rs:357`, `lib/src/runaway/detector.rs:501`: thresholds applied
        - `lib/src/composition/sequence/task/shell.rs:154`: volume cap on sequence shell tasks
        - `lib/src/config/merge.rs:54`: repo replaces the user value

- **`steering.automatic.enabled`**: automatic repetition help. On an early repetition warning, Claudine sends the agent a message telling it that it may be looping, at most 3 times per execution. Turning this off does not change repetition detection or manual `claudine steer`.
    - **Type:** `{ automatic: { enabled: boolean } }`. An explicit `null` is an error, not a way to write "absent"
    - **Default:** `true` · **Scope:** user and repo (a repo value wins; an absent key inherits)
    - **Env override:** `CLAUDINE_AUTO_STEER` (`true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`) wins over both files (`lib/src/steering/automatic.rs:25`)
    - **Used in:**
        - `lib/src/steering/automatic.rs:42` (`SteeringConfig`, `AutomaticSteeringConfig`)
        - `lib/src/steering/automatic.rs:110`: `resolve_enabled`
        - `cli/src/commands/wrap/runaway_guard.rs:337`: resolved with the other guard inputs
        - `cli/src/commands/wrap/wrapper_exec.rs:78`: gates automatic help
        - `lib/src/config/merge.rs:60`: repo overrides user

## Observations for the redesign

- **Scope asymmetry.** The repo file accepts only `canonical_provider`, `actions`, `matchers`, `active_messenger`, `exit_expressions`, `guard_settings`, and `steering`. Any other key fails `deny_unknown_fields`, so a repo cannot set `tts`, `logging`, `protect`, `models`, and so on.
- **Inconsistent readers.** `prompt_for_missing`, `harvest_unmatched`, and `claudine sync` load the user file only (`load_claudine_config(None, None)`). Hook dispatch and composition selection load user plus repo. The runaway guard reads both files separately so that it can do its own layered resolution.
- **`canonical_provider` is display and gate only.** Init computes a full `CanonicalProviderSettings` (a canonical provider per scope and per resource, `lib/src/events/config.rs:68`) and then discards it, persisting only the top-ranked provider (`cli/src/commands/init/mod.rs:189`). The linking code's per-slot `canonical_provider()` reader (`lib/src/linking/canonical.rs:117`) is never fed from the config file.
- **Orphaned types.** `LogTarget` (`lib/src/actions/hook_action.rs:12`) is built by the init wizard's logging prompt (`cli/src/commands/init/prompts.rs:213`), but no `HookAction` variant carries it and `logging` is a bare boolean. The `default_log_target`, `linking`, `protect`, and `messaging` fields of `events::GlobalSettings` (`lib/src/events/config.rs:11`) are never populated from a file; only `tts` is filled, through `bridge_tts_settings`.
- **`.json5` mismatch.** Shell completion treats `~/.claudine/config.json5` as a user config (`cli/src/completion/engine/mod.rs:552`), but the loader only reads `config.json` (`lib/src/dispatch/loader.rs:11`).
- **Two merge rules for `exit_expressions`.** `merge_repo_override` replaces the whole value, while `resolve_exit_expressions` honors `mode`.
- **Hard-coded values that could be configuration:** the `bash` action timeout (3 s), the `call` default timeout (60 s), the automatic-steering budget (3 per execution), and the JSONL log path.
