---
$schema: ./_schema.yaml
created: '2026-07-02'
last_updated: '2026-07-03'
agent: open_code
model: minimax/MiniMax-M3
docs: https://developers.openai.com/codex/cli
system_prompt_docs: https://developers.openai.com/codex/guides/agents-md
append_support: config
replace_support: config
cli_params:
  - flag: "-c instructions=\"<text>\""
    mode: replace
    value_shape: 'TOML key=value'
    description: 'Override Codex''s built-in base instructions (the system prompt) with inline text via the universal config-override flag. Resolution chain — runtime base_instructions, then model_instructions_file, then this instructions key, then the bundled default.'
    example: codex exec "Summarize" -c instructions="You are a TypeScript expert. Reply tersely."
    notes: 'Top-level TOML key instructions: string in config-schema.json is documented as "System instructions." Setting it replaces Codex''s bundled default; the schema does not advertise the key as append-friendly. Inline text on the shell needs TOML quoting/escaping (the documented shell-safe form is -c with single-quoted TOML).'
  - flag: "-c instructions_file=<path>"
    mode: replace
    value_shape: 'TOML key=absolute file path'
    description: 'Set the base instructions from a file. The docstring for model_instructions_file warns it "will override the built-in instructions for the selected model. Users are STRONGLY DISCOURAGED from using this field, as deviating from the instructions sanctioned by Codex will likely degrade model performance." Validated by reading the file at startup.'
    example: codex exec -c model_instructions_file=/tmp/claudine-instructions.md "Refactor auth"
    notes: 'This is the documented file-based replacement knob. model_instructions_file wins over the inline instructions key. Resolution order in codex-rs/core/src/config/mod.rs is runtime base_instructions, then file contents, then cfg.instructions. Use a shadow path under a temp file rather than mutating user config.toml.'
  - flag: "-c developer_instructions=\"<text>\""
    mode: append
    value_shape: 'TOML key=value'
    description: 'Insert a separate developer-role message alongside the base instructions. The schema description says "Developer instructions inserted as a developer role message." This is NOT the system prompt; it is an extra developer-role turn message Codex sends to the model on top of the base layer.'
    example: codex exec -c developer_instructions="Always use TypeScript" "Summarize"
    notes: 'Functionally an add-on message, not a system-prompt append. Multi-line strings need TOML triple-quote literals such as developer_instructions followed by three double quotes, then content, then three double quotes (TOML-formatted shell-safe form), or careful shell escaping. Used by Codex''s own custom-agent role files, each role TOML MUST define developer_instructions.'
  - flag: --enable <FEATURE>
    mode: modify
    value_shape: feature name
    description: Force-enable a feature flag; translates to `-c features.<name>=true`. Indirectly shapes the prompt by enabling surfaces such as `memories`, `multi_agent`, `personality`, `goals`, `plugins`, `hooks`, `collaboration_modes`, `tui_app_server`, and so on.
    example: codex --enable multi_agent
    notes: Feature enablement can add or change prompt layers. `features.personality=true` exposes the `personality` setting; `features.memories=true` exposes the memories layer; `features.multi_agent=true` enables subagent dispatch and the `spawn_agent` tool.
  - flag: --disable <FEATURE>
    mode: modify
    value_shape: feature name
    description: Force-disable a feature flag; translates to `-c features.<name>=false`. Indirectly shapes the prompt by removing optional layers.
    example: codex --disable memories
    notes: Combine with --enable to flip flags for a single invocation without mutating config.toml. `--disable memories` mutes the memory injection surface entirely.
  - flag: --strict-config
    mode: other
    value_shape: boolean
    description: Errors out when `config.toml` contains fields that the running Codex version does not recognize. Useful in CI for catching drift between an inherited config and the live binary schema.
    example: codex --strict-config "Summarize"
    notes: Affects only validation, not the system prompt surface. Pairs with the `#:schema https://developers.openai.com/codex/config-schema.json` directive in `config.toml` for editor-time checking.
  - flag: -p / --profile <CONFIG_PROFILE_V2>
    mode: other
    value_shape: profile name
    description: Layers `$CODEX_HOME/<name>.config.toml` on top of the base user config. Profiles can override any top-level key including `instructions`, `model_instructions_file`, `developer_instructions`, and `personality`.
    example: codex -p fast -c instructions="Be brief" "Summarize"
    notes: Profiles are a config layer, not a flag-direct prompt override; combine with -c to layer an inline override on top of a profile preset instructions.
  - flag: -m / --model <MODEL>
    mode: other
    value_shape: model name
    description: Override the configured model. Same effective knob as setting `model` in config.toml. Does not directly change the system prompt text but selects which bundled `base_instructions` default applies.
    example: codex -m gpt-5
    notes: Codex reads the model-specific default base instructions from `codex-rs/protocol/src/prompts/base_instructions/<model>.md`; Claude Code uses a single global preset, but Codex routes by model.
config_sources:
  - os: macos
    scope: user
    path: ~/.codex/config.toml
    mode: modify
    format: toml
    notes: User-level durable config. Can set `instructions`, `model_instructions_file`, `developer_instructions`, `model_reasoning_effort`, `personality`, `model`, `memories`, `compact_prompt`, `include_environment_context`, `include_permissions_instructions`, `include_apps_instructions`, `include_collaboration_mode_instructions`, `include_skill_instructions`, `project_doc_max_bytes`, `project_doc_fallback_filenames`, `[agents.<name>]`, `[features]`, `[plugins.<id>]`, `[projects.<path>].trust_level`, etc.
  - os: linux
    scope: user
    path: ~/.codex/config.toml
    mode: modify
    format: toml
    notes: Linux path equivalent; the binary resolves the same `~/.codex` root.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.codex\config.toml'
    mode: modify
    format: toml
    notes: Windows path equivalent; `codex.exe` honours the same `CODEX_HOME` semantics.
  - os: macos
    scope: user
    path: ~/.codex/AGENTS.md
    mode: append
    format: markdown
    notes: Global project-instructions file. Loaded only if `~/.codex/AGENTS.override.md` does not exist.
  - os: linux
    scope: user
    path: ~/.codex/AGENTS.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.codex\AGENTS.md'
    mode: append
    format: markdown
    notes: Windows path equivalent.
  - os: macos
    scope: user
    path: ~/.codex/AGENTS.override.md
    mode: append
    format: markdown
    notes: Temporary global override. Wins over `~/.codex/AGENTS.md` when present.
  - os: linux
    scope: user
    path: ~/.codex/AGENTS.override.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.codex\AGENTS.override.md'
    mode: append
    format: markdown
    notes: Windows path equivalent.
  - os: macos
    scope: repo
    path: AGENTS.md
    mode: append
    format: markdown
    notes: Project-level instructions. Codex walks from the project root (default markers `[".git"]`, override via `project_root_markers`) down to the current working directory and concatenates files. Per-directory `AGENTS.override.md` wins over `AGENTS.md`.
  - os: linux
    scope: repo
    path: AGENTS.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: repo
    path: AGENTS.md
    mode: append
    format: markdown
    notes: Windows path equivalent.
  - os: macos
    scope: repo
    path: AGENTS.override.md
    mode: append
    format: markdown
    notes: Per-directory override; takes precedence over `AGENTS.md` in the same directory.
  - os: linux
    scope: repo
    path: AGENTS.override.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: repo
    path: AGENTS.override.md
    mode: append
    format: markdown
    notes: Windows path equivalent.
  - os: macos
    scope: repo
    path: .codex/config.toml
    mode: modify
    format: toml
    notes: Project-scoped config loaded only when the project is trusted. Cannot override provider, auth, notify, profile, or telemetry keys (`openai_base_url`, `chatgpt_base_url`, `apps_mcp_product_sku`, `model_provider`, `model_providers`, `notify`, `profile`, `profiles`, `experimental_realtime_ws_base_url`, `otel`).
  - os: linux
    scope: repo
    path: .codex/config.toml
    mode: modify
    format: toml
    notes: Linux path equivalent.
  - os: windows
    scope: repo
    path: .codex/config.toml
    mode: modify
    format: toml
    notes: Windows path equivalent.
  - os: macos
    scope: agent
    path: ~/.codex/agents/*.toml
    mode: replace
    format: toml
    notes: Personal custom agent / role definitions. Each TOML must define `name`, `description`, and either a top-level `developer_instructions` (required) or point at a role-config TOML. Subagent picks this up when `spawn_agent(..., agent_type=<name>)` is called.
  - os: linux
    scope: agent
    path: ~/.codex/agents/*.toml
    mode: replace
    format: toml
    notes: Linux path equivalent.
  - os: windows
    scope: agent
    path: '%USERPROFILE%\.codex\agents\*.toml'
    mode: replace
    format: toml
    notes: Windows path equivalent.
  - os: macos
    scope: agent
    path: .codex/agents/*.toml
    mode: replace
    format: toml
    notes: Project-scoped custom agent definitions. Loaded only when the project is trusted.
  - os: linux
    scope: agent
    path: .codex/agents/*.toml
    mode: replace
    format: toml
    notes: Linux path equivalent.
  - os: windows
    scope: agent
    path: .codex/agents/*.toml
    mode: replace
    format: toml
    notes: Windows path equivalent.
  - os: macos
    scope: repo
    path: .codex/skills/*/SKILL.md
    mode: append
    format: markdown
    notes: Project-level skills. Closest to the working directory wins on name collisions. The schema declares skills under `[skills]` plus a `bundled_skills` table; the actual `SKILL.md` body is loaded only when the agent selects the skill.
  - os: linux
    scope: repo
    path: .codex/skills/*/SKILL.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: repo
    path: .codex/skills/*/SKILL.md
    mode: append
    format: markdown
    notes: Windows path equivalent.
  - os: macos
    scope: user
    path: ~/.codex/prompts/*.md
    mode: append
    format: markdown
    notes: Legacy custom slash-prompt files. The custom-prompt surface was largely superseded by `~/.codex/skills/` and per-session AGENTS.md files; treat as deprecated for prompt-system purposes.
  - os: linux
    scope: user
    path: ~/.codex/prompts/*.md
    mode: append
    format: markdown
    notes: Linux path equivalent.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.codex\prompts\*.md'
    mode: append
    format: markdown
    notes: Windows path equivalent.
env_vars:
  - name: CODEX_HOME
    effect: Sets the Codex state root (default `~/.codex`). Config files, AGENTS.md discovery, agents/, skills/, prompts/, sessions/, logs, and SQLite state all relocate when this is set; the directory must already exist.
    mode: other
  - name: CODEX_SQLITE_HOME
    effect: Overrides SQLite-backed state location. `sqlite_home` config option takes precedence; relative paths resolve from the current working directory.
    mode: other
  - name: CODEX_API_KEY
    effect: Provides an OpenAI API key for a single non-interactive `codex exec` run. Only honoured by `codex exec` (not the interactive TUI).
    mode: other
  - name: CODEX_ACCESS_TOKEN
    effect: Provides a ChatGPT or Codex access token for trusted automation. Pipe to `codex login --with-access-token` to persist.
    mode: other
  - name: CODEX_NON_INTERACTIVE
    effect: Set on the shell that runs `chatgpt.com/codex/install.sh` (or the PowerShell sibling) to skip installer prompts. Set to `1`, `true`, or `yes`. Documented as installer-only.
    mode: other
  - name: CODEX_INSTALL_DIR
    effect: Overrides where the standalone installer places the `codex` symlink. The package cache still lives under `$CODEX_HOME/packages/standalone`.
    mode: other
  - name: CODEX_CA_CERTIFICATE
    effect: PEM CA bundle for environments with corporate TLS interception or private root CAs. Takes precedence over `SSL_CERT_FILE`.
    mode: other
  - name: SSL_CERT_FILE
    effect: Fallback PEM CA bundle path when `CODEX_CA_CERTIFICATE` is unset.
    mode: other
  - name: RUST_LOG
    effect: Controls Rust log filtering and verbosity. Targets useful for verifying which AGENTS.md files load or which instructions layer resolved include `codex_core=debug`, `codex_core::agents_md=trace`, `codex_core::config=trace`. Setting `log_dir` explicitly enables a plaintext `codex-tui.log`.
    mode: inspect
prompt_layers:
  - source: Built-in base instructions (per model)
    mode: replace
    scope:
      - builtin
    order_notes: Lowest layer; replaced by `instructions` (config), `model_instructions_file`, or runtime `base_instructions`.
    notes: Bundled default lives at `codex-rs/protocol/src/prompts/base_instructions/<model>.md` (for example `default.md`). Selects automatically per `model` selection. Set `instructions` or `model_instructions_file` to override; the override is verbatim text (no template substitution).
  - source: "model_instructions_file"
    mode: replace
    scope:
      - session
    order_notes: Beats the inline `instructions` key and the built-in default. Read at startup; failure to read is propagated as an error.
    notes: Top-level TOML key, type `AbsolutePathBuf`. Docstring warns "Users are STRONGLY DISCOURAGED from using this field, as deviating from the instructions sanctioned by Codex will likely degrade model performance." This is the file-backed replacement surface Codex prefers to call out.
  - source: instructions
    mode: replace
    scope:
      - session
    order_notes: Beats the built-in default; loses to `model_instructions_file` (resolution chain in `codex-rs/core/src/config/mod.rs` is `runtime base_instructions` > file contents > `cfg.instructions`).
    notes: Top-level TOML key, type string. Schema description is the bare phrase "System instructions." Inline -c delivery is the documented wrapper path for replace.
  - source: "AGENTS.md hierarchy"
    mode: append
    scope:
      - user
      - repo
    order_notes: >-
      User-scope `~/.codex/AGENTS.override.md` wins over `~/.codex/AGENTS.md`,
      then repo-root-down walks (per directory, AGENTS.override.md, then AGENTS.md,
      then `project_doc_fallback_filenames`). Joined with a blank-line separator;
      the user-to-project transition inserts a literal `--- project-doc ---`
      divider (see `codex-rs/core/src/agents_md.rs` constant `AGENTS_MD_SEPARATOR`).
    notes: Concatenated with blank lines elsewhere; truncated to `project_doc_max_bytes` (default 32 KiB). Closest-to-CWD files appear later and override earlier guidance. `project_root_markers` defaults to `[".git"]` (configurable). Discovered per environment snapshot.
  - source: developer_instructions
    mode: append
    scope:
      - session
    order_notes: 'Inserted as a developer-role message alongside the base layer, in addition to any role file developer_instructions.'
    notes: 'Top-level TOML key. The schema description quotes "Developer instructions inserted as a developer role message." This is the surface custom-agent TOML files use; not the system prompt itself, but the next-busiest addition to the prompt.'
  - source: Personality
    mode: modify
    scope:
      - session
    order_notes: Applied through the `personality` setting (`none`, `friendly`, `pragmatic`); gated on `features.personality=true`.
    notes: Shapes communication style; does not replace tool guidance. The `pragmatic` value is the documented default for Opus-class models on GPT-5.x. The user's current `config.toml` carries `personality = "pragmatic"`.
  - source: Model reasoning effort
    mode: modify
    scope:
      - session
    order_notes: Applied via `model_reasoning_effort` (`minimal` | `low` | `medium` | `high` | `xhigh`).
    notes: Changes the reasoning-depth portion of the prompt. Coexists with a separate `plan_mode_reasoning_effort` override that only applies under the TUI Plan preset.
  - source: Memories
    mode: append
    scope:
      - session
    order_notes: Injected when `features.memories=true` and `memories.use_memories=true`.
    notes: Configurable rate-limit gating (`memories.min_rate_limit_remaining_percent`) and runtime knobs (`memories.max_rollouts_per_startup`, `memories.max_unused_days`, etc.). Schema at `[memories]` in config-schema.json.
  - source: Skill metadata
    mode: append
    scope:
      - session
    order_notes: Skill `name` and `description` injected at session start (under `include_skill_instructions=true`); full `SKILL.md` body loaded only when the agent selects the skill.
    notes: Discovery paths include `~/.codex/skills/`, `.codex/skills/`, and `.agents/skills/`. Closest-to-CWD wins on name collisions.
  - source: Apps / collaboration / permissions / environment context blocks
    mode: modify
    scope:
      - session
    order_notes: 'Each gate is a top-level boolean. Defaults match `codex-rs/protocol/src/context/*` and the `ModelsResponse` defaults.'
    notes: '`include_apps_instructions`, `include_collaboration_mode_instructions`, `include_permissions_instructions`, `include_environment_context`, and `include_skill_instructions` each gate a separate developer- or user-role block.'
  - source: Custom agent developer_instructions
    mode: replace
    scope:
      - subagent
    order_notes: Each spawned subagent receives its own developer-role message built from its role file's `developer_instructions` (and any overrides on `spawn_agent`).
    notes: Subagent prompts are isolated through their own config layer; the parent's `developer_instructions` is not automatically inherited unless the role file inherits the parent layer. `agents.max_depth` defaults to 1; `agents.max_threads` defaults to 6 (`Some(6)`); `agents.multi_agent_v2.default_wait_timeout_ms` is 30s.
agent_prompting:
  supported: true
  definition_surface: "TOML files in $CODEX_HOME/agents/, .codex/agents/, or the managed .codex/agents/ layer. Each file declares `name`, `description`, and a full TOML table; must define `developer_instructions` (or point at a config file via `config_file`). Inline `agents.<role>` declarations in `config.toml` follow the same shape."
  inheritance: "Subagents inherit the parent session's `model`, `model_provider`, `service_tier` (unless the role layer overrides one of those), and the parent's resolved config layer stack. The role layer is inserted at session-flag precedence so role settings can override persisted config. Each subagent runs its own session and developer-role message; the parent's `developer_instructions` is NOT carried over unless the role file references it explicitly. Built-in agents (see below) currently ship `explorer` (effectively empty `explorer.toml` inherits the default reasoning effort profile) and `awaiter` (a fully-defined role). `default` is the role-name used when no `agent_type` is supplied."
  isolation: "Each subagent runs in its own session/thread; only the final assistant reply returns to the parent. `agents.max_depth` defaults to 1 (so root sessions reach one layer of nesting); `agents.max_threads` defaults to 6 concurrency slots; `agents.multi_agent_v2` namespace owns the v2 `spawn_agent`/`send_message`/`followup_task`/`wait_agent`/`interrupt_agent`/`list_agents` surface and gives every v2 session one concurrency slot out of 4 by default."
  limitations: "Naming collisions across config layers follow normal layer precedence (managed > user > project; project subdirectory closest to CWD wins on nested `.codex/`). `agents.multi_agent_v2.hide_spawn_agent_metadata=true` is the default, so callers only see a canonical task name; the parent invocation cannot inject its own system prompt into a child unless that prompt lives in the child's role file. `Agent(agent_type)` deny/allowlist is enforced at spawn time by the v2 tool handler; rejection surfaces to the model as a function-call error, not as a session failure."
claudine_delivery:
  append_strategy: config_key_inline
  replace_strategy: config_key_file
  temp_file_required: true
  argv_limit: No published argv limit for the universal `-c` flag; prefer writing large prompts to a temporary file and passing `-c model_instructions_file=<tmp>` (replace) or `-c developer_instructions='"""..."""'` via a TOML triple-quote literal (append). Avoid `-c instructions="…"` for inline text past a few hundred chars because shell quoting/escaping becomes a footgun.
  notes: "Replace path: discover `system-prompt.md` from the launch-CWD hierarchy, compose with Darkmatter, write to a temp Markdown file, invoke `codex … -c model_instructions_file=<tmp>` so Codex reads the resolved content at session start. Append path: for true prompt augmentation (not just an extra developer-role message) the closest documented knob is `-c developer_instructions='…'`; pair it with an empty `model_instructions_file` only if you intend to replace the base. Avoid mutating user `~/.codex/config.toml`, `~/.codex/AGENTS.md`, or `~/.codex/agents/*.toml`; use `-c` overrides and temporary files so the wrapper is stateless."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Codex resolves `instructions` from Markdown-shaped strings (and `model_instructions_file` from a Markdown file). The built-in base instructions live at `codex-rs/protocol/src/prompts/base_instructions/<model>.md` — pure Markdown with `#` headings and bullet lists. Markdown blends cleanly with AGENTS.md concatenation and with the `developer_instructions` injection surface, and does not require XML wrapper tags. Replace-mode callers can keep XML wrapping (`<rules>`, `<constraints>`, `<context>`, `<examples>`) if their downstream consumer prefers sections, but Codex does not parse the resulting block - it is verbatim text. Keep multi-line `developer_instructions` strings as TOML triple-quote literals to avoid shell-escape mishaps."
recent_changes:
  - date: '2026-07-01'
    version: '0.142.5'
    change: '`experimental_instructions_file` was renamed to `model_instructions_file`; Codex deprecates the old key. The config reference page is annotated "Rename experimental_instructions_file to model_instructions_file. Codex deprecates the old key; update existing configs to the new name."'
    impact: Wrapper code referencing the old name silently falls through to "no file configured" (deprecated key only); update to `model_instructions_file` and document the rename in claudine metadata so wrappers using the old name flag a configuration drift instead of producing a no-op.
  - date: '2026-07-01'
    version: '0.142.5'
    change: Built-in subagents were pruned - `default`, `worker`, and the older roster are gone; only the `explorer` (empty config; inherits defaults) and `awaiter` roles ship. `DEFAULT_ROLE_NAME = "default"` resolves to the parent session's config layer when `agent_type` is omitted on `spawn_agent`.
    impact: "Prior research claimed built-ins `default`, `worker`, `explorer`. Today only `explorer` and `awaiter` ship; the others were removed. Wrapper docs/claudine metadata must drop the old list."
  - date: '2026-07-01'
    version: '0.142.5'
    change: Top-level config keys `include_apps_instructions`, `include_collaboration_mode_instructions`, `include_environment_context`, `include_permissions_instructions`, and `include_skill_instructions` were added (or stabilised) to gate individual developer-/user-role message blocks at session start.
    impact: A wrapper can now opt out of `<apps_instructions>`, `<environment_context>`, etc. per invocation without rewriting the base layer. Document each gate so wrappers know which block is being suppressed.
  - date: '2026-07-01'
    version: '0.142.5'
    change: Personality enum stabilised as `none | friendly | pragmatic`. `pragmatic` is the documented default for GPT-5-class models.
    impact: Old docs/wrapper config referencing `professional`, `concise`, or other non-canonical names must be rewritten; values outside the enum validate against `--strict-config`.
  - date: '2026-07-02'
    version: '0.142.5'
    change: Multi-agent v2 became the default path. The v2 `spawn_agent` tool namespace defaults to `collaboration`; `multi_agent_v2.default_wait_timeout_ms = 30_000`, `min_wait_timeout_ms = 10_000`, `max_wait_timeout_ms = 3_600_000`. `agents.max_depth` still defaults to 1; the old v1 `spawn_agent` / `send_input` / `resume_agent` / `wait_agent` / `close_agent` tool names were superseded.
    impact: Wrapper metadata that referenced the v1 names needs a refresh. The `multi_agent_v2` tool namespace lives outside `functions.exec` (intentional - documented in `codex-rs/core/src/agent/builtins`).
  - date: '2026-07-03'
    version: '0.143.0-alpha.35'
    change: Pre-release line is now `0.143.0-alpha.35` (published 2026-07-03T02:33:31Z); the previous stable `0.142.5` (2026-07-01T01:15:44Z) is the latest semver-strict release.
    impact: Wrapper release metadata should track both `latest` and `latest_pre` if it surfaces version info.
quirks:
  - "There is NO dedicated `--system-prompt` or `--append-system-prompt` flag. Codex routes prompt control through the universal `-c <key=value>` config override, where `instructions` and `model_instructions_file` provide replacement surfaces and `developer_instructions` provides an appended developer-role message."
  - "`developer_instructions` is NOT a system-prompt append. The schema description is precise: `Developer instructions inserted as a developer role message.` It is emitted on the developer channel, not into the base instructions themselves."
  - "`model_instructions_file` overrides the built-in instructions for the selected model. Docstring warns: `Users are STRONGLY DISCOURAGED from using this field, as deviating from the instructions sanctioned by Codex will likely degrade model performance.` Treat this as `STRONGLY DISCOURAGED` in wrapper docs."
  - "AGENTS.md discovery uses the same filename pattern as well-formed Claude-Code-style docs but the resolution order is hard-coded: `AGENTS.override.md` first, then `AGENTS.md`, then `project_doc_fallback_filenames`. A directory can contribute at most one file."
  - "The AGENTS.md chain is silently truncated to `project_doc_max_bytes` (default 32 KiB). Long guidance must be split across nested directories or the limit raised in `[project_doc_max_bytes]`."
  - "AGENTS.override.md in the same directory shadows AGENTS.md without warning; new users routinely leave an override file behind and forget why their base file stopped loading."
  - "`config.toml` rejects unknown fields under `--strict-config`. Wrappers that pass `-c foo.bar=baz` against an out-of-date schema fail closed in strict mode and pass through silently otherwise."
  - "Project-scoped `.codex/config.toml` cannot override provider, auth, notify, profile, or telemetry keys (`openai_base_url`, `chatgpt_base_url`, `apps_mcp_product_sku`, `model_provider`, `model_providers`, `notify`, `profile`, `profiles`, `experimental_realtime_ws_base_url`, `otel`). Putting these in `.codex/config.toml` is a silent ignore on untrusted projects and a hard error once the project is trusted."
  - "Built-in subagents currently ship as `explorer` (empty `explorer.toml` → defaults) and `awaiter` (full TOML with custom `developer_instructions`). Prior research listed `default`, `worker`, `explorer`; that roster is no longer accurate."
  - "`agents.max_depth` defaults to 1; recursive delegation requires explicitly raising it. `agents.max_threads` defaults to 6. `multi_agent_v2` defaults give every session one concurrency slot out of 4."
  - "Multi-agent v2 collaboration tools (`spawn_agent`, `send_message`, `followup_task`, `wait_agent`, `interrupt_agent`, `list_agents`) intentionally live OUTSIDE `functions.exec` and must be called as direct model tools (`to=functions.collaboration.spawn_agent`)."
  - "Custom-agent role files MUST define `developer_instructions`. A role file with only `config_file` (no inline `developer_instructions`) is rejected at parse time."
  - "`include_environment_context=true` injects an `<environment_context>` user-role message; disable via `-c include_environment_context=false` if you want a tighter prompt for automation."
  - "`model_instructions_file` resolution: `runtime override → file contents → cfg.instructions → bundled default`. Setting `cfg.instructions` does NOT beat `model_instructions_file`."
  - "AGENTS.md `--strict-config` does NOT apply to AGENTS.md files themselves; only the TOML config is strict-mode-checked."
  - "`~/.codex/instructions.md` is created as an empty file by the installer but is not a Codex-discovered file (Codex only reads `AGENTS.md` and `AGENTS.override.md`). Treat its presence as vestigial; wrappers should ignore it for discovery purposes."
  - "The legacy `~/.codex/prompts/*.md` custom-slash-prompt surface is deprecated in favour of skills (`~/.codex/skills/`) and per-repo AGENTS.md guidance."
  - "Codex does NOT expose a `/context`-style summary or a `codex prompt dump` command. Indirect verification requires `RUST_LOG=codex_core=trace codex -c log_dir=/tmp/... exec ...` and grepping the JSONL logs for AGENTS.md path entries, or asking the model to summarise loaded instructions."
gaps:
  - "OpenAI does not publish the full default Codex base instructions (per-model) outside the bundled source tree. Wrappers can read `codex-rs/protocol/src/prompts/base_instructions/<model>.md` directly or extract it from `~/.codex/sessions/<thread>.jsonl` `session_meta.payload.base_instructions`, but no `codex --print-instructions` command exists."
  - "The exact ordering between `developer_instructions` and built-in developer-role blocks (`<permissions_instructions>`, `<apps_instructions>`, `<collaboration_mode>`, `<skills_instructions>`) inside the final prompt is not documented; only the boolean gates are."
  - "Whether `project_root_markers` accepts `.git` only or any value at runtime is documented as yes, but the explicit list of supported markers (`.git`, `.codex`, `.jj`?) is not enumerated publicly."
  - "Whether `model_instructions_file` accepts shell commands, templates, or path-only values is documented as path-only; no template substitution is mentioned."
  - "Multi-environment AGENTS.md discovery (`for <env_id> with root <cwd>` labels in `LoadedAgentsMd::environment_labeled_text`) is recent and not yet surfaced in the public docs; behaviour for a single-environment Codex run keeps the legacy plain-AGENTS.md text."
  - "The exact merge precedence of `developer_instructions` set via `-c` versus `developer_instructions` declared inside a `spawn_agent(agent_type=...)` role file is implicit via config-layer ordering and not described in the public docs."
  - "The on-disk schema lacks documentation for individual `-c` keys. Wrapper code that wants to verify a config override is valid must consult the schema JSON (`developers.openai.com/codex/config-schema.json`) or the Rust source rather than a help-page listing."
changes:
  - "2026-07-03 refresh: replaced the prior `developer_instructions` (append) / `model_instructions_file` (replace) pair with a sharper classification (developer_role_message append + file_replace). Pulled the exact schema descriptions from `developers.openai.com/codex/config-schema.json` (`instructions` is `string` \"System instructions\", `developer_instructions` is `string` \"Developer instructions inserted as a `developer` role message\", `model_instructions_file` is `AbsolutePathBuf` with the strong-discouragement docstring). Replaced `os: all` config_sources records with macos/linux/windows triples. Verified the resolution order (`runtime > model_instructions_file > cfg.instructions > bundled default`) against `codex-rs/core/src/config/mod.rs`. Confirmed the built-in agent roster via `codex-rs/core/src/agent/role.rs` (only `explorer` and `awaiter`; `default` is the implicit role name, not a built-in TOML). Captured the multi-agent v2 namespace, `multi_agent_v2.default_wait_timeout_ms = 30_000`, and the v2 tool names. Recorded the `experimental_instructions_file` → `model_instructions_file` rename. Verified local Codex binary 0.142.5 and `~/.codex/config.toml` content (model gpt-5.5, personality pragmatic, multi_agent true). Verified `~/.codex/instructions.md` is empty and NOT a Codex-discovered surface."
requires_claudine_update: true
reason: "Provider metadata for Codex must (a) drop the deprecated `experimental_instructions_file` key from claudine's replace path in favor of `model_instructions_file`, (b) reclassify the append surface as a developer-role message rather than a system-prompt append so wrapper docs and the SystemPromptSpec gate the right behaviour, (c) trim the built-in subagent roster to `explorer` + `awaiter` + the implicit `default` role name, and (d) pick up the multi-agent v2 tool namespace and the new `include_*_instructions` boolean gates in claudine's provider-metadata facts."
---

# Codex CLI System-Prompt Surface

## Overview

Codex CLI builds the effective prompt for every session by stacking an order of layers: the **base instructions** (model-specific defaults shipped at `codex-rs/protocol/src/prompts/base_instructions/<model>.md`) at the bottom, optional `AGENTS.md` discovery chained on top, then a developer-role message built from `developer_instructions` (set in `config.toml`, passed inline via `-c`, or declared inside a custom-agent role file), then several opt-in developer/user blocks gated by `include_*_instructions` booleans. The base-instructions replacement surface lives behind the top-level `instructions` key and the documented `model_instructions_file` key. There is no dedicated `--system-prompt` flag; prompt control flows through the universal `-c <key=value>` config-override mechanism.

Claudine already implements this delivery correctly: `model_instructions_file` (replace, file-backed) and `developer_instructions` (append, inline) via `-c`. The schema-driven facts in this document tighten the classification, remove the deprecated `experimental_instructions_file` reference, and update the built-in subagent roster.

## CLI Parameters

Codex exposes one general-purpose flag that covers prompt overrides plus feature toggles that indirectly shape the prompt.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `-c instructions="..."` | Replace | Override the bundled base instructions with inline text. |
| `-c model_instructions_file=<path>` | Replace | Override the base instructions with the contents of a Markdown file (top-level `AbsolutePathBuf`). |
| `-c developer_instructions="..."` | Append | Insert an extra developer-role message alongside the base instructions (NOT a system-prompt append). |
| `-c developer_instructions="""..."""` | Append | Multi-line TOML triple-quote form of the same key; preferred for shell delivery. |
| `--enable <feature>` / `--disable <feature>` | Modify | Force-enable or force-disable a feature flag via `-c features.<name>`. |
| `--strict-config` | Other | Error on unknown config keys. Pairs with the `#:schema` directive in `config.toml`. |
| `-p / --profile <name>` | Other | Layer `$CODEX_HOME/<name>.config.toml` on top of the base user config; combine with `-c` for per-invocation overrides. |
| `-m / --model <MODEL>` | Other | Override the configured model; selects which bundled `base_instructions` default applies. |

`-c` values are parsed as TOML when possible; otherwise the literal string is used. Multi-line `instructions` or `developer_instructions` strings must be encoded as TOML triple-quote literals or as escape-quoted shell strings. The wrapper-facing `codex --help` does not list prompt-control flags explicitly; the surface is documented in `developers.openai.com/codex/cli/reference` and the configuration schema at `developers.openai.com/codex/config-schema.json`.

## Configuration and Discovery

### `config.toml` layers

User-level settings live in `~/.codex/config.toml` (or the directory named by `CODEX_HOME`). Project-scoped overrides live in `.codex/config.toml`, but Codex only loads the project layer when the project is trusted, and it ignores provider, auth, notify, profile, and telemetry keys from that layer.

The prompt-affecting keys (verified against `developers.openai.com/codex/config-schema.json`):

| Key | Effect |
| :--- | :--- |
| `instructions` | Inline system instructions (replace). Type: string. Description: "System instructions." |
| `model_instructions_file` | Path to a file whose contents override the bundled base instructions. Type: absolute path. |
| `developer_instructions` | Extra developer-role message appended to the prompt. Type: string. |
| `model_reasoning_effort` | Adjusts reasoning depth (`minimal` to `xhigh`). |
| `plan_mode_reasoning_effort` | Plan-mode-specific reasoning override. |
| `personality` | Personality layer. Enum: `none`, `friendly`, `pragmatic`. Gated on `features.personality = true`. |
| `project_doc_max_bytes` | Caps the combined AGENTS.md chain (default 32 KiB). |
| `project_doc_fallback_filenames` | Additional filenames treated as instruction files. |
| `project_root_markers` | Markers that delimit the project root; defaults to `[".git"]`. |
| `include_apps_instructions` | Inject the `<apps_instructions>` developer block. |
| `include_collaboration_mode_instructions` | Inject the `<collaboration_mode>` developer block. |
| `include_permissions_instructions` | Inject the `<permissions instructions>` developer block. |
| `include_environment_context` | Inject the `<environment_context>` user block. |
| `include_skill_instructions` | Inject the `<skills_instructions>` developer block. |
| `agents.max_depth` | Maximum subagent nesting depth (default 1). |
| `agents.max_threads` | Maximum concurrent subagent threads (default 6). |
| `agents.interrupt_message` | Whether to record a model-visible interrupt message (default true). |
| `agents.job_max_runtime_seconds` | Default max runtime in seconds for agent job workers. |
| `compact_prompt` | Compact prompt used for history compaction. |
| `memories.use_memories` | Skip injecting memory usage instructions into developer prompts. |

### AGENTS.md hierarchy

Codex reads `AGENTS.md` files before doing any work. Discovery follows this order (`codex-rs/core/src/agents_md.rs`):

1. **Global scope**: `$CODEX_HOME/AGENTS.override.md` if present, otherwise `$CODEX_HOME/AGENTS.md`.
2. **Project scope**: starting at the project root (closest ancestor whose directory contains one of `project_root_markers`, default `[".git"]`), walk down to the current working directory. In each directory, check `AGENTS.override.md` first, then `AGENTS.md`, then entries in `project_doc_fallback_filenames`. Include at most one file per directory.
3. **Merge order**: files are concatenated from root down to CWD. Files closer to the working directory appear later and therefore override earlier guidance. User-to-project transition is marked with the separator `\n\n--- project-doc ---\n\n`; otherwise blank lines.

Codex stops once the combined size reaches `project_doc_max_bytes` (default 32 KiB). Multi-environment snapshots (one agent running across multiple roots) relabel the body as `for <env_id> with root <cwd>` before each environment's section.

### Custom agents (subagents)

Custom agents are TOML files under `$CODEX_HOME/agents/` or `.codex/agents/`. Each file must define:

- `name` (string, required)
- `description` (string, required)
- a full `[…]` block with the same shape as `[profile]` plus the top-level `ConfigToml` keys

The minimal role file looks like:

```toml
# ~/.codex/agents/role-name.toml
name = "role-name"
description = "Human-facing summary for spawn_agent guidance"

[profile]
model = "gpt-5"
developer_instructions = """
You are …

Rules:
- …
"""
```

A role file may also point at a sibling config file via `config_file = "<path>.toml"`. Parsing requires the file to contain a valid TOML table; missing fields surface as parse warnings, and a blank `developer_instructions` field is rejected.

### Built-in subagents

`codex-rs/core/src/agent/role.rs` registers only two built-in roles today:

| Role | Defined in | Notes |
| :--- | :--- | :--- |
| `explorer` | `codex-rs/core/src/agent/builtins/explorer.toml` | Empty TOML; inherits the default profile. |
| `awaiter` | `codex-rs/core/src/agent/builtins/awaiter.toml` | Carries its own `developer_instructions` for awaiting long-running tasks. |

`default` is the role-name used when `spawn_agent` is called without `agent_type`; it is the parent's effective config, not a separate TOML.

### Skills and deprecated custom prompts

Skills are discovered under `$CODEX_HOME/skills/`, `.codex/skills/`, and `.agents/skills/`. The skill `name` and `description` are injected at session start; the full `SKILL.md` body is loaded when the agent selects the skill. Closest-to-CWD wins on name collisions. The legacy `$CODEX_HOME/prompts/*.md` custom-slash-prompt surface is deprecated in favour of skills and AGENTS.md.

## Prompt Layers and Precedence

Resolution of the base instructions is hard-coded in `codex-rs/core/src/config/mod.rs`:

```text
base_instructions =
    runtime_override
    .or(file_base_instructions)         # from model_instructions_file
    .or(cfg.instructions);               # from the `instructions` top-level key
```

Beyond the base instructions, the effective prompt is built from:

```mermaid
graph TD
    A[Bundled default base_instructions] --> B{instructions or model_instructions_file set?}
    B -- yes --> C[Override base instructions]
    B -- no --> A
    C --> D[AGENTS.md hierarchy]
    A --> D
    D --> E[developer_instructions]
    E --> F[Personality layer]
    F --> G[Model reasoning effort]
    G --> H[Memories]
    H --> I[Skill metadata]
    I --> J[Developer/user blocks]
    J --> K[User prompt]
```

Notes on precedence:

- `model_instructions_file` beats `instructions` beats the bundled default for the base instructions.
- `developer_instructions` is a separate developer-role message; it is layered alongside the base instructions, not appended into them.
- AGENTS.md files are concatenated and capped at `project_doc_max_bytes`.
- Personality, reasoning effort, memories, and skill metadata modify or append additional structure on top of the base chain.

## Agents and Subagents

Codex supports multi-agent workflows through built-in roles (`explorer`, `awaiter`) and user-defined role TOML files. Subagents run in isolated sessions and only their final assistant reply returns to the parent.

Key behaviours observed in `codex-rs/core/src/agent/role.rs` and `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`:

- `spawn_agent` accepts `message`, `task_name`, `agent_type`, `model`, `reasoning_effort`, `service_tier`, `fork_turns`. The v2 namespace is `collaboration`; v1 names (`spawn_agent` / `send_input` / `resume_agent` / `wait_agent` / `close_agent`) are superseded.
- The parent resolves `agent_type` to a role name, reads the role's TOML, applies it as a fresh config layer at session-flag precedence, and spawns a new thread.
- Subagents inherit the parent's `model_provider` and `service_tier` unless the role layer sets `model_provider` or `service_tier` explicitly. The parent's `developer_instructions` is NOT inherited automatically; roles must define their own.
- `agents.max_depth` defaults to 1 (root sessions reach one nesting layer). `agents.max_threads` defaults to 6 concurrent threads. The v2 subagent runtime gives every session one slot out of `multi_agent_v2.max_concurrent_threads_per_session` (default 4) with `default_wait_timeout_ms = 30_000`, `min_wait_timeout_ms = 10_000`, `max_wait_timeout_ms = 3_600_000`.
- `Agent(agent_type)` deny/allowlist rules enforced by the v2 tool handler; rejections surface to the parent as a function-call error.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Replace (file flag) | Pure Markdown | The bundled base instructions live in `*.md` files (`codex-rs/protocol/src/prompts/base_instructions/<model>.md`); `model_instructions_file` reads the file verbatim. |
| Replace (inline `-c instructions`) | Plain text or TOML-quoted | `instructions` is a TOML string; multi-line content needs TOML triple-quote literals (`instructions=""" … """`). |
| Append (developer_instructions) | Pure Markdown | `developer_instructions` injects text verbatim into a developer-role message. XML wrapper tags are optional because Codex does not parse the block. |
| Custom-agent role TOML | TOML with `name`, `description`, `developer_instructions` | The TOML parser rejects blank `developer_instructions` and surfaces missing fields as startup warnings. |

The file-backed replace path is preferred for any prompt larger than a few hundred characters to avoid shell-quoting footguns. The `-c developer_instructions='"""…"""'` form is preferred over inline shell escaping for multi-line append-style commands.

## Recent Changes

- **Renamed `experimental_instructions_file` to `model_instructions_file` (0.142.5, 2026-07-01)**: the config-reference page annotates "Rename `experimental_instructions_file` to `model_instructions_file`. Codex deprecates the old key; update existing configs to the new name." Wrappers using the old name silently get the bundled default; claudine metadata should migrate to the new key.
- **Built-in subagent roster trimmed (0.142.5)**: only `explorer` (empty config) and `awaiter` (full `developer_instructions`) ship now. The earlier roster of `default`, `worker`, `explorer` is gone. `DEFAULT_ROLE_NAME = "default"` continues to resolve to the parent session's effective config when `agent_type` is omitted.
- **`include_*_instructions` boolean gates added (0.142.5)**: `include_apps_instructions`, `include_collaboration_mode_instructions`, `include_environment_context`, `include_permissions_instructions`, and `include_skill_instructions` let wrappers opt into or out of individual developer/user blocks per invocation.
- **Personality enum stabilised (0.142.5)**: `none | friendly | pragmatic`. Strict-config validation rejects the older values (`concise`, `professional`, …) that circulated in earlier docs.
- **Multi-agent v2 becomes the default (0.142.5)**: tool names are `spawn_agent` / `send_message` / `followup_task` / `wait_agent` / `interrupt_agent` / `list_agents` under the `collaboration` namespace. v1 names (`send_input`, `resume_agent`, `close_agent`) are superseded.
- **Pre-release line at `0.143.0-alpha.35` (2026-07-03)**: latest stable is still `0.142.5` (2026-07-01T01:15:44Z). Wrapper release metadata should track both `latest` and `latest_pre` if it surfaces version info.

## Quirks and Workarounds

- Codex has NO dedicated `--system-prompt` / `--append-system-prompt` flag. Prompt control flows through the universal `-c <key=value>` config override, where `instructions` and `model_instructions_file` provide replacement and `developer_instructions` provides an appended developer-role message.
- Because `-c` values parse as TOML, multi-line `developer_instructions` strings need TOML triple-quote literals (`developer_instructions="""…"""`).
- AGENTS.md discovery prefers `AGENTS.override.md` first; a directory-level override silently shadows `AGENTS.md` in the same directory.
- The AGENTS.md chain is capped at 32 KiB by default. Split guidance across nested directories or raise `project_doc_max_bytes`.
- An `AGENTS.override.md` higher in the directory tree or under `$CODEX_HOME` shadows the regular `AGENTS.md`. Renaming/removing the override is the only way to fall back.
- `model_instructions_file` overrides the bundled base instructions but is documented as "STRONGLY DISCOURAGED" because deviating from Codex-sanctioned instructions is expected to degrade model performance. Wrappers should keep the override narrow or fall back to `developer_instructions` when safety against regression matters more than full control.
- Codex does not publish the full effective built-in prompt at runtime; the bundled defaults live in `codex-rs/protocol/src/prompts/base_instructions/<model>.md` in the source tree, and per-session snapshots live in `~/.codex/sessions/<thread>.jsonl` (`session_meta.payload.base_instructions`). Indirect verification runs `RUST_LOG=codex_core=trace codex -c log_dir=… exec …` and reads the JSONL log.
- Project-level `.codex/config.toml` cannot override provider, auth, profile, notify, or telemetry keys; placing those there is a silent ignore on untrusted projects and a hard error once the project is trusted.
- Custom-agent role files MUST define a non-blank `developer_instructions`; the parser rejects blanks and surfaces missing fields as startup warnings.
- Subagent recursion defaults to depth 1; nested delegation requires `agents.max_depth > 1`.
- Multi-agent v2 collaboration tools are intentionally outside `functions.exec`. Callers must invoke them as direct model tools (`to=functions.collaboration.spawn_agent`).
- `~/.codex/instructions.md` is created empty by the installer but is NOT a Codex-discovered file. Treat it as vestigial; do not add prompt content there and expect it to load.
- The legacy `~/.codex/prompts/*.md` slash-prompt surface is deprecated in favour of skills and AGENTS.md.

## Claudine Delivery Notes

Claudine should continue using the config-override delivery path:

- Discover a `system-prompt.md` file from the launch working-directory hierarchy.
- For **replace** mode, prepare the content with Darkmatter and pass the path via `-c model_instructions_file=<tmp>`. This avoids shell quoting/escaping inside the wrapper and keeps the call stateless.
- For **append** mode, prefer inline `-c developer_instructions="""…"""` with a TOML triple-quote literal. Use a temporary helper file only when the prompt is large enough that shelling out 32 KiB through TOML literal escapes becomes a footgun; in that case write the string to a file and pass `-c developer_instructions="$(cat /tmp/claudine-instructions.txt)"` (still parsed as a TOML literal string).
- Both modes are temporary per-invocation changes, so no user `config.toml`, `AGENTS.md`, role file, or skill is permanently mutated.
- Avoid placing prompt content at `~/.codex/instructions.md`; Codex does not read it. Use `~/.codex/AGENTS.md` or `~/.codex/AGENTS.override.md` only when an opt-in persistent layer is genuinely wanted.
- Project trust gates apply to project-scoped config; setting `projects.<path>.trust_level = "trusted"` in user config is the only way to grant a wrapper permission to load `.codex/config.toml` on an untrusted project.

## Changelog

- **2026-07-03 — refresh**: replaced the prior `developer_instructions` (append) / `model_instructions_file` (replace) pair with a sharper classification (developer-role message append + file-backed replace). Pulled exact schema descriptions from `developers.openai.com/codex/config-schema.json` (`instructions` → "System instructions", `developer_instructions` → "Developer instructions inserted as a `developer` role message", `model_instructions_file` → "Optional path to a file containing model instructions that will override the built-in instructions for the selected model. Users are STRONGLY DISCOURAGED …"). Replaced every `os: all` config_source record with the corresponding macOS/Linux/Windows triple. Verified the resolution order (`runtime > model_instructions_file > cfg.instructions > bundled default`) against `codex-rs/core/src/config/mod.rs`. Confirmed the built-in agent roster via `codex-rs/core/src/agent/role.rs` (only `explorer` and `awaiter`; the prior roster of `default`, `worker`, `explorer` is retired; `default` is the implicit role name). Captured multi-agent v2 namespace, `multi_agent_v2.default_wait_timeout_ms = 30_000`, and v2 tool names (`spawn_agent` / `send_message` / `followup_task` / `wait_agent` / `interrupt_agent` / `list_agents`). Recorded the `experimental_instructions_file` → `model_instructions_file` rename, the new `include_*_instructions` boolean gates, and the `personality` enum stabilisation. Verified the local Codex binary (0.142.5) and `~/.codex/config.toml` (model gpt-5.5, personality pragmatic, multi_agent true). Marked `~/.codex/instructions.md` as a non-Codex surface. Marked `requires_claudine_update: true` because provider metadata for Codex should drop the deprecated key, tighten the append classification, and pick up the new boolean gates.
- **2026-07-02 — initial research (prior refresh, carried in)**: introduced `developer_instructions` (inline) for append and `model_instructions_file` (file) for replace, recorded custom-agent `~/.codex/agents/*.toml`, AGENTS.md discovery walk, `project_doc_max_bytes`, `project_doc_fallback_filenames`, and the `multi_agent`, `personality`, `memories` feature flags.

## Sources

- [Codex CLI overview](https://developers.openai.com/codex/cli)
- [Command line options](https://developers.openai.com/codex/cli/reference)
- [Configuration Reference](https://developers.openai.com/codex/config-reference)
- [Configuration Reference JSON Schema](https://developers.openai.com/codex/config-schema.json)
- [Environment variables](https://developers.openai.com/codex/environment-variables)
- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Prompting Codex](https://developers.openai.com/codex/prompting)
- [Subagents (multi-agent v2)](https://developers.openai.com/codex/subagents)
- [Codex GitHub repository](https://github.com/openai/codex)
- [Codex releases](https://github.com/openai/codex/releases)
- [AGENTS.md discovery source (`codex-rs/core/src/agents_md.rs`)](https://github.com/openai/codex/blob/main/codex-rs/core/src/agents_md.rs)
- [Agent role resolution (`codex-rs/core/src/agent/role.rs`)](https://github.com/openai/codex/blob/main/codex-rs/core/src/agent/role.rs)
- [Custom agent config loader (`codex-rs/core/src/config/agent_roles.rs`)](https://github.com/openai/codex/blob/main/codex-rs/core/src/config/agent_roles.rs)
- [Multi-agent v2 spawn tool (`codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`)](https://github.com/openai/codex/blob/main/codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs)
- [Base-instructions defaults (`codex-rs/protocol/src/prompts/base_instructions/default.md`)](https://github.com/openai/codex/blob/main/codex-rs/protocol/src/prompts/base_instructions/default.md)
- [Built-in awaiter role (`codex-rs/core/src/agent/builtins/awaiter.toml`)](https://github.com/openai/codex/blob/main/codex-rs/core/src/agent/builtins/awaiter.toml)
