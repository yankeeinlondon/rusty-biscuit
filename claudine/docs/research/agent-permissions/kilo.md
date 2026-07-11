---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: auto
    style: switch
    description: "For `kilo run`; auto-replies `once` to permission prompts for the root session and tracked Task child sessions when the request is not already denied by policy."
    example: 'kilo run --auto "run tests and fix failures"'
    example_description: "Runs a non-interactive session that approves ordinary permission prompts for the main run and its tracked subagents."
  - param: dangerously-skip-permissions
    style: switch
    description: "For `kilo run`; auto-replies `once` to permission prompts that are not explicitly denied. In current source it also replies to tracked Task child-session prompts."
    example: 'kilo run --dangerously-skip-permissions "update dependencies"'
    example_description: "Runs a headless session with prompts auto-approved instead of rejected."
  - param: agent
    style: switch
    description: "Selects the primary agent for the session. Agents have their own `permission` rules, so this changes the effective policy."
    example: 'kilo --agent plan'
    example_description: "Starts the TUI with the built-in plan agent, which denies normal edit operations."
  - param: pure
    style: switch
    description: "Runs without external plugins. This reduces plugin-provided hooks and custom tools but does not disable built-in tools or MCP configured in ordinary config."
    example: 'kilo --pure'
    example_description: "Starts an interactive session without loading external plugins."
  - param: interactive
    style: switch
    description: "For `kilo run`; enables direct interactive split-footer mode. Interactive sessions can display permission prompts instead of auto-rejecting them."
    example: 'kilo run --interactive "inspect this repo"'
    example_description: "Runs through the interactive prompt UI so permission requests can be answered by a user."
  - param: format
    style: switch
    description: "For `kilo run`; accepts `default` or `json`. It changes output shape only; permission prompts in non-interactive mode are still handled by run-loop auto-approve or auto-reject logic."
    example: 'kilo run --format json "summarize changes"'
    example_description: "Streams raw JSON events while keeping the same permission behavior as non-interactive run mode."
  - param: dir
    style: switch
    description: "For `kilo run`; selects the working directory, which changes the worktree-relative paths used by file permissions and the sandbox writable project root."
    example: 'kilo run --dir ./packages/api "fix lint"'
    example_description: "Runs in a narrower directory context."
  - param: permissions
    style: switch
    description: "For `kilo agent create`; comma-separated permission names to allow in the generated agent. Unlisted known permissions are written as deny rules. Alias: `--tools`."
    example: 'kilo agent create --path .kilo --description "read-only reviewer" --mode subagent --permissions read,grep,glob'
    example_description: "Creates a generated subagent that denies all standard permissions except read, grep, and glob."
  - param: tools
    style: switch
    description: "Alias for `kilo agent create --permissions`; accepts the same comma-separated permission allowlist."
    example: 'kilo agent create --path .kilo --description "docs editor" --mode primary --tools read,edit'
    example_description: "Creates an agent with only read and edit enabled among the standard agent-create permissions."

env_vars:
  - name: KILO_PERMISSION
    effect: "Runtime JSON overlay merged into the effective `permission` object near the end of config loading."
    effect_category: policy_overlay
  - name: KILO_CONFIG_CONTENT
    effect: "Inline JSON/JSONC config content; can include permissions, agents, MCP, plugins, and sandbox settings."
    effect_category: config_injection
  - name: KILO_CONFIG
    effect: "Path to an extra config file loaded after global config and before project/config-directory sources."
    effect_category: config_path_override
  - name: KILO_CONFIG_DIR
    effect: "Adds or replaces an extra config directory in the load chain; it may contain `kilo.jsonc`, agents, commands, plugins, and tools."
    effect_category: config_path_override
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: "Disables project root config files and project `.kilo`/`.kilocode` config directories for this process."
    effect_category: config_source_toggle
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: "Disables Kilo default plugins, reducing plugin-provided behavior."
    effect_category: customization_lockdown
  - name: KILO_PURE
    effect: "Environment equivalent used by runtime flag handling for pure mode, disabling external plugins."
    effect_category: customization_lockdown
  - name: KILO_ENABLE_QUESTION_TOOL
    effect: "Enables the question tool for clients that would not normally expose it."
    effect_category: tool_surface
  - name: KILO_EXPERIMENTAL_LSP_TOOL
    effect: "Enables the LSP tool, adding another permission-targetable tool."
    effect_category: tool_surface
  - name: KILO_EXPERIMENTAL_SCOUT
    effect: "Enables scout/repository tools such as repo clone and repo overview in current source."
    effect_category: tool_surface
  - name: KILO_BWRAP_PATH
    effect: "Overrides the Linux bubblewrap executable used by the sandbox backend."
    effect_category: sandbox_control

config_files:
  - os: macos
    user: ".config/kilo/kilo.jsonc; .kilo/kilo.jsonc; .kilocode/kilo.jsonc"
    repo: "kilo.jsonc; kilo.json; .kilo/kilo.jsonc; .kilo/kilo.json; .kilocode/kilo.jsonc; .kilocode/kilo.json"
    notes: "Current CLI uses xdg-basedir with app name `kilo`, so on this host the global config root is `~/.config/kilo`, not `~/Library/Application Support/kilo`. The CLI also reads home `.kilo` and `.kilocode` config directories."
  - os: linux
    user: ".config/kilo/kilo.jsonc; .kilo/kilo.jsonc; .kilocode/kilo.jsonc"
    repo: "kilo.jsonc; kilo.json; .kilo/kilo.jsonc; .kilo/kilo.json; .kilocode/kilo.jsonc; .kilocode/kilo.json"
    notes: "XDG config roots can move when `XDG_CONFIG_HOME` is set. Legacy OpenCode names `opencode.jsonc` and `opencode.json` are also loaded."
  - os: windows
    user: "AppData\\Roaming\\kilo\\kilo.jsonc; .kilo\\kilo.jsonc; .kilocode\\kilo.jsonc"
    repo: "kilo.jsonc; kilo.json; .kilo\\kilo.jsonc; .kilo\\kilo.json; .kilocode\\kilo.jsonc; .kilocode\\kilo.json"
    notes: "Uses the xdg-basedir Windows config root plus home `.kilo` and `.kilocode`; matching is case-insensitive on Windows."
  - os: macos
    user: "/Library/Application Support/kilo/kilo.jsonc; /Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    repo: ""
    notes: "Managed config files and macOS MDM preferences are read after normal config and override it."
  - os: linux
    user: "/etc/kilo/kilo.jsonc"
    repo: ""
    notes: "Managed config files are read after normal config and override it."
  - os: windows
    user: "%ProgramData%\\kilo\\kilo.jsonc"
    repo: ""
    notes: "Managed config files are read after normal config and override it."

precedence:
  - source: cli
    scope: [approval_mode, agents, tool_visibility]
    merge_strategy: none
    notes: "`kilo run --auto`, `--dangerously-skip-permissions`, and `--agent` are session controls; `--pure` affects plugin loading."
  - source: runtime_api
    scope: [approval_mode, rules]
    merge_strategy: none
    notes: "The local permission API can enable allow-everything globally or for one session and can persist selected always-rules."
  - source: managed_preferences
    scope: [general_config, rules, mcp, sandbox, tool_visibility]
    merge_strategy: deep
    notes: "macOS managed preferences are loaded last and act as admin-controlled overrides."
  - source: managed_config
    scope: [general_config, rules, mcp, sandbox, tool_visibility]
    merge_strategy: deep
    notes: "System config under `/Library/Application Support/kilo`, `/etc/kilo`, or `%ProgramData%\\kilo` is loaded after ordinary user/project config."
  - source: cloud_org_config
    scope: [general_config, provider_model, rules, mcp, tool_visibility]
    merge_strategy: deep
    notes: "Active Kilo Cloud organization config is loaded after env content and before managed local config."
  - source: env
    scope: [rules, config_loading, extensions]
    merge_strategy: deep
    notes: "`KILO_PERMISSION` is applied near the end as a permission overlay; `KILO_CONFIG_CONTENT` and `KILO_CONFIG` participate in the ordinary deep-merge chain."
  - source: config_directories
    scope: [agents, slash_commands, extensions, customization_resources, rules, mcp]
    merge_strategy: deep
    notes: "Config directories load from global, primary-worktree fallback, project `.kilocode`/`.kilo`, home `.kilocode`/`.kilo`, and `KILO_CONFIG_DIR`; later merges override or append depending on field."
  - source: repo_config
    scope: [general_config, rules, agents, mcp, sandbox]
    merge_strategy: deep
    notes: "Project `kilo.jsonc`/`kilo.json` and legacy `opencode` files load after `KILO_CONFIG` and before config directories unless project config is disabled."
  - source: custom_config_path
    scope: [general_config, rules, agents, mcp]
    merge_strategy: deep
    notes: "`KILO_CONFIG` loads after global config and before project config."
  - source: user_config
    scope: [general_config, rules, agents, mcp]
    merge_strategy: deep
    notes: "Global files load in order: `config.json`, `kilo.json`, `kilo.jsonc`, `opencode.json`, `opencode.jsonc`."
  - source: remote_well_known_config
    scope: [general_config, rules, provider_model]
    merge_strategy: deep
    notes: "Remote `.well-known/opencode` config is loaded before local global config."

default_posture: "Kilo's raw permission evaluator defaults unmatched checks to `ask`, but the built-in default primary agent supplies a permissive ruleset: most tools are allowed, `.env` and `.env.*` reads ask, `.env.example` is allowed, `external_directory` asks, and `doom_loop` asks. Several auxiliary tools such as question, plan transitions, repo_clone, and repo_overview are denied unless enabled by an agent or experimental flag."

cli_zero_permissions:
  supported: false
  invocation: 'KILO_PERMISSION=''{"*":"deny"}'' KILO_DISABLE_PROJECT_CONFIG=1 kilo run "..."'
  mechanism: "Kilo has no CLI-only no-tools or deny-all switch. The closest session-scoped wrapper posture uses environment overlays to deny all permissions and disable project config."
  limitations: "This is not CLI-only, and additional permissions cannot be added back with Kilo CLI flags in the same invocation. Claudine would need to set `KILO_PERMISSION`/`KILO_CONFIG_CONTENT` or launch a preconfigured locked-down agent."

agent_permissions:
  allowed: true
  fm_properties:
    - permission
    - tools

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "Interactive TUI/VS Code can toggle runtime allow-everything through the permission API; non-interactive `kilo run` exposes `--auto` and `--dangerously-skip-permissions`; config can set `permission: {\"*\":\"allow\"}`."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Kilo's native rule entity is an ordered `(permission, pattern, action)` ruleset with last-match-wins wildcard semantics; PolicyEngine does not yet model provider-specific ordered wildcard rules directly."
    - "Tool visibility and approval are coupled: deny-all for a tool can remove it from the model tool surface."
    - "Kilo's built-in agents inject permissive defaults even though raw unmatched rules ask."
    - "Sensitive `.env` read hardening cannot be overridden by broad allow or allow-everything rules."
    - "Protected config paths force ask and disable always approvals."
    - "Session-scoped and persisted always approvals are provider runtime state, not just static config."
    - "Kilo sandbox state has per-session, per-directory, and config-default layers that PolicyEngine does not model."
    - "MCP and custom plugin tools are permissioned by generated tool names rather than stable provider-neutral entities."

permission_entities:
  - entity: tool
    native_names: [bash, read, edit, write, apply_patch, glob, grep, list, task, todowrite, todoread, webfetch, websearch, lsp, skill, question, suggest, agent_manager, interactive_terminal, repo_clone, repo_overview, notebook_read, notebook_edit, notebook_execute]
    notes: "Built-in and Kilo-specific tools are evaluated through permission names; write and apply_patch are grouped under `edit`."
  - entity: tool_group
    native_names: [edit, tools]
    notes: "`edit` covers edit/write/apply_patch; legacy `tools` booleans are converted to permission allow/deny rules."
  - entity: command
    native_names: [bash]
    notes: "Bash rules match parsed command strings; multi-command shell inputs may generate multiple checks."
  - entity: path
    native_names: [read, edit, glob, grep, list, external_directory, .kilocodeignore]
    notes: "File permissions match worktree-relative paths; external directory checks use absolute paths; `.kilocodeignore` is migrated into deny rules."
  - entity: workspace
    native_names: [external_directory, dir, worktree]
    notes: "Access outside the worktree asks by default; `--dir` changes the runtime directory and sandbox project root."
  - entity: mcp_server
    native_names: [mcp, enabled]
    notes: "MCP servers can be disabled with `enabled: false`; local and remote server configs have different runtime boundaries."
  - entity: mcp_tool
    native_names: ["{server}_{tool}", "{server}_*"]
    notes: "MCP tools are exposed with sanitized server-prefixed names and use the ordinary permission ruleset."
  - entity: mcp_resource
    native_names: ["{server}:{resource}"]
    notes: "MCP resources are listed/read through the MCP service; no separate documented permission key was found."
  - entity: agent
    native_names: [agent, mode, default_agent]
    notes: "Agents carry `permission` rules and may be selected by `--agent` or configured as built-ins/custom agents."
  - entity: subagent
    native_names: [task, subagent_type]
    notes: "`permission.task` controls which subagents can be launched; Kilo also hard-denies nested task/question/interactive_terminal in child sessions."
  - entity: mode
    native_names: [build, plan, general, explore, scout, code, ask, orchestrator]
    notes: "Built-in agent modes supply different permission defaults; plan denies normal edits."
  - entity: approval_category
    native_names: [allow, ask, deny, once, always, reject]
    notes: "Static rules use allow/ask/deny; prompts can be answered once, always, or reject."
  - entity: sandbox
    native_names: [experimental.sandbox, experimental.sandbox_restrict_network, kilocode.sandbox, /sandbox]
    notes: "Sandboxing is separate from approval policy and can be toggled per session."
  - entity: hook
    native_names: [plugin, "tool.execute.before", "tool.execute.after", "permission.ask"]
    notes: "Plugins can add hooks and tools; no PreToolUse policy hook equivalent was found for replacing permission evaluation."
  - entity: extension
    native_names: [plugin, pure]
    notes: "External plugins can add tools and hooks; `--pure` suppresses external plugins."
  - entity: slash_command
    native_names: [/sandbox, /init, /profile, command]
    notes: "Slash commands can trigger provider behavior; `/sandbox` toggles the sandbox, while custom workflows live under config directories."

approval_modes:
  - name: default
    effect: "Built-in agent rules allow most tools while prompting for external-directory access, `.env` reads, and doom-loop continuation."
    interactive: true
    non_interactive: true
    aliases: [build, code, default]
  - name: ask-by-default
    effect: "The raw permission evaluator returns ask when no rule matches; this appears when an agent lacks Kilo's built-in permissive fallback."
    interactive: true
    non_interactive: true
    aliases: [unmatched-rule-default]
  - name: auto
    effect: "Non-interactive run auto-approves permission prompts for the root and tracked Task child sessions unless denied by policy."
    interactive: false
    non_interactive: true
    aliases: ["--auto"]
  - name: dangerously-skip-permissions
    effect: "Non-interactive run auto-approves permission prompts that are not explicitly denied."
    interactive: false
    non_interactive: true
    aliases: ["--dangerously-skip-permissions"]
  - name: allow-everything
    effect: "Runtime API or config adds `{permission:'*', pattern:'*', action:'allow'}`; explicit denies and hard protections still apply."
    interactive: true
    non_interactive: true
    aliases: ["permission.allow_everything", "permission: {\"*\":\"allow\"}", "shield toggle"]
  - name: plan
    effect: "Built-in primary agent that denies ordinary edit permission while allowing plan-specific transitions and plan-file writes."
    interactive: true
    non_interactive: true
    aliases: ["--agent plan", plan]

rule_model:
  decisions: [allow, ask, deny]
  syntax: "Config accepts `permission: \"allow\"|\"ask\"|\"deny\"|null` as shorthand for `*`, or `permission: { key: { pattern: action } }`; agent Markdown uses the same shape in YAML frontmatter. Runtime rules normalize to ordered `{permission, pattern, action}` records."
  precedence: "Within the effective ruleset, the last matching rule wins. Kilo's resolver adds hard behavior: base deny wins over saved/session approvals, saved deny can still deny, base ask is not bypassed unless a saved allow is at least as broad, `.env` broad allows are hardened to ask, and protected config edits force ask."
  merge_semantics: "Config sources deep-merge. Permission objects preserve author key order during parsing; legacy `tools` booleans are converted into permission allow/deny and merged under `permission`. Agent and session rules are merged by concatenating ordered rulesets."
  matcher_semantics: "`*` becomes `.*`, `?` becomes `.`, regex metacharacters are escaped, backslashes normalize to `/`, Unix matching is case-sensitive, Windows matching is case-insensitive, and a trailing ` *` also matches the bare command. `~` and `$HOME` prefixes are expanded at config load."
  default_decision: "Raw unmatched permission checks ask. The default built-in agent supplies explicit allow/ask/deny rules that make most common tools allow by default."

tool_visibility:
  supported: true
  mechanisms:
    - "Tools whose permission is denied by a broad `*` pattern are omitted from the model tool list."
    - "The legacy `tools` config object maps booleans to permission allow/deny."
    - "`--pure`, `KILO_DISABLE_DEFAULT_PLUGINS`, and plugin config affect custom tool availability."
    - "Experimental flags/config can expose extra tools such as LSP and scout/repository tools."
  notes: "Kilo does not expose a clean CLI `--tools` allowlist for runtime sessions; visibility and approval share the permission ruleset."

sandbox:
  supported: true
  modes: [disabled, enabled-network-deny, enabled-network-allow]
  backends: ["macOS sandbox-exec/Seatbelt", "Linux bubblewrap/bwrap", "Windows unsupported"]
  filesystem_control: "When enabled, shell commands and file-write tools are confined to the project/worktree plus Kilo data/cache/config/state/tmp/bin/log/repos. `.git`, sandbox policy store, and sandbox preference store are denied for writes. File reads are not confined."
  network_control: "When `experimental.sandbox_restrict_network` is true, outbound network is denied for model-originated shell commands, first-party HTTP tools, custom tools, and remote MCP delegated authority. Provider/model traffic, local MCP servers, and plugin hooks are not covered."
  notes: "Sandbox is off by default and experimental. Per-session toggles and per-directory preferences seed future sessions. If the backend is unavailable, confinement stays off or the support reason is reported."

trust_and_admin:
  folder_trust: "No separate folder-trust prompt was found. Project config and project `.kilo`/`.kilocode` directories load by default unless `KILO_DISABLE_PROJECT_CONFIG` is set."
  managed_policy: "Managed local config files and macOS MDM preferences are loaded after user, project, env-content, and cloud organization config. Active Kilo Cloud organization config is also a managed layer before local managed files."
  safe_mode: "There is no dedicated safe-mode flag. `--pure`, `KILO_PURE`, `KILO_DISABLE_PROJECT_CONFIG`, and strict deny rules are the practical safe-start controls."
  notes: "Protected config paths are permission-gated even in broad allow modes; this is a client-side guard, not a project-trust framework."

mcp_permissions:
  supported: true
  server_filters:
    - "`mcp.<name>.enabled: false` disables a server."
    - "`--pure` can suppress plugin-provided MCP-adjacent behavior but not ordinary configured MCP."
  tool_filters:
    - "Each MCP tool gets a sanitized permission key such as `github_create_pull_request`."
    - "Rules can use wildcard groups such as `github_*`."
    - "Subagents inherit parent denials for MCP-prefixed tool names."
  trust_model: "No folder trust gate was found for MCP config. Remote MCP OAuth stores per-user auth; remote MCP tools are marked as delegated network authority for sandbox network checks."
  notes: "Local MCP servers and plugin hooks are outside the sandbox network restriction; remote MCP calls are checked by Kilo's in-process network assertion when sandbox network restriction is active. MCP response sanitization beyond normal truncation/attachment conversion was not found."

headless_behavior: "Plain non-interactive `kilo run` cannot prompt a human; permission prompts for the root session are auto-rejected unless `--auto` or `--dangerously-skip-permissions` is set. Headless child-session asks are denied instead of hanging, and tracked Task prompts are auto-approved in `--auto`."

approval_persistence: "Interactive `always` approvals and selected allow/deny always-rules can be persisted into global config through the permission API. Config-file edit requests disable always approval and are downgraded to once."

protected_paths:
  - "*.env"
  - "*.env.*"
  - ".env"
  - ".env.*"
  - "kilo.json"
  - "kilo.jsonc"
  - "opencode.json"
  - "opencode.jsonc"
  - "AGENTS.md"
  - "AGENT.md"
  - ".kilo/**"
  - ".kilocode/**"
  - ".git/**"
  - "kilo-sandbox-policy/**"
  - "kilo-sandbox-preference/**"

security_posture: "Kilo combines a client-side static permission engine, advisory/runtime approval prompts, persisted config rules, managed config layers, and an optional OS-enforced sandbox for writes and selected network surfaces. The sandbox is not a complete OS isolation boundary because reads, provider traffic, local MCP servers, and plugin hooks can remain outside it."

changes:
  - "Changed frontmatter agent/model to `codex`/`default` and refreshed `last_updated` to 2026-07-03."
  - "Corrected global config paths: current CLI uses xdg-basedir `~/.config/kilo` on this macOS host; the inspected local config only contained `$schema` and no permission rules."
  - "Corrected raw default decision to `ask` while documenting that built-in default agents add permissive allow rules."
  - "Corrected `.env` handling from deny to ask for broad read approvals; `.env.example` is explicitly allowed."
  - "Added protected config-file behavior: edits to Kilo config paths and AGENTS files force approval and disable always grants."
  - "Updated non-interactive behavior: plain `kilo run` auto-rejects root asks, denies headless child asks, and `--auto` tracks Task child sessions."
  - "Updated approval persistence: always approvals can write global config, but protected config edits are once-only."
  - "Updated sandbox details against current source and docs: sandbox is off by default, experimental, unavailable on Windows, and covers writes plus selected network surfaces rather than file reads."
  - "Expanded MCP coverage to include server `enabled`, sanitized server-prefixed tool names, remote MCP network checks, and local MCP sandbox bypass."
  - "Recorded that no true CLI-only zero-permissions/no-tools baseline exists."

requires_claudine_update: true
reason: "Kilo is not yet a compiled Claudine provider, and accurate PolicyEngine support needs Kilo/OpenCode-style ordered wildcard rules, agent-scoped permissions, runtime/persisted approvals, protected config-path semantics, tool visibility coupling, and sandbox metadata that the current provider-neutral model cannot fully express."
---

# Kilo Code Permissions and Security Controls

## Introduction to Kilo Code Permissions

Kilo Code uses an ordered permission ruleset. Native runtime rules have three fields: `permission`, `pattern`, and `action`. The action is one of `allow`, `ask`, or `deny`. User config can write the same idea compactly as either a scalar action or a pattern map:

```yaml
permission:
  read: allow
  edit:
    "*": deny
    "*.md": allow
  bash:
    "*": ask
    "git status *": allow
```

The public docs describe the same model: rules decide whether a tool call runs, prompts, or is blocked, and later matching rules win. Current source confirms the matcher is not a general regex exposed to users; Kilo escapes regex metacharacters, treats `*` and `?` as wildcards, normalizes backslashes to `/`, and is case-insensitive on Windows.

Configuration can define permissions in several places:

- Global config files in the xdg-basedir config root for app `kilo`, usually `~/.config/kilo/kilo.jsonc` on this host.
- Project root `kilo.jsonc` / `kilo.json`.
- Config directories such as `.kilo/`, `.kilocode/`, `~/.kilo/`, and `~/.kilocode/`.
- Agent Markdown frontmatter under `permission`.
- Runtime overlays from `KILO_PERMISSION` or `KILO_CONFIG_CONTENT`.
- Managed config files and macOS MDM preferences.

The local inspection found `/Users/ken/.claudine/.config/kilo/kilo.jsonc` with only:

```json
{
  "$schema": "https://app.kilo.ai/config.json"
}
```

No local permission rules were present to inspect. The installed `kilo` and `kilocode` commands both reported version `7.3.45`.

Environment variables that matter most are `KILO_PERMISSION`, `KILO_CONFIG_CONTENT`, `KILO_CONFIG`, `KILO_CONFIG_DIR`, `KILO_DISABLE_PROJECT_CONFIG`, `KILO_PURE`, and plugin/tool experimental flags. `KILO_PERMISSION` is the important one for a wrapper: it is JSON merged into the effective `permission` object near the end of config loading.

CLI permission controls are limited. `kilo run --auto` and `kilo run --dangerously-skip-permissions` are non-interactive auto-approval modes. `--agent` selects an agent and therefore its permission profile. `--pure` reduces external plugin/tool/hook surface. `kilo agent create --permissions` or `--tools` writes a new agent with denied permissions for unselected standard keys; it does not alter a live session.

There is no independent runtime `--tools` allowlist. Tool visibility and approval are coupled through permissions: if the effective ruleset broadly denies a tool, Kilo can omit it from the model's tool surface. The legacy `tools` config object is converted into permission allow/deny rules.

## Permissions Use Cases

### Default

Two defaults must be distinguished:

- The raw evaluator defaults unmatched permission checks to `ask`.
- Kilo's built-in default primary agent adds an explicit permissive ruleset, so most common tools are allowed out of the box.

The built-in default posture allows most tools, asks for `external_directory`, asks for `doom_loop`, and hardens broad `.env` read approvals back to `ask`. `.env.example` is allowed. Some auxiliary tools are denied by default unless an agent or flag enables them, including question, plan transitions, repo clone, and repo overview.

A PolicyEngine representation would need rules like:

- Read workspace paths: allow, except `.env` / `.env.*`: ask.
- Write workspace paths: allow, except provider config paths: ask and once-only.
- Execute bash: allow, with sandbox metadata if sandbox is enabled.
- External directory: ask.
- Doom-loop continuation: ask.
- Subagent task: allow only if the chosen agent policy allows the subagent pattern.

PolicyEngine can describe the broad intent, but it is not ergonomic or complete. The missing pieces are ordered last-match-wins wildcard rules, Kilo's built-in agent default layer, hard `.env` read behavior, config-path protection, session/global always approvals, and sandbox state.

### Whitelisting

Kilo supports whitelisting through config or env, not through a dedicated CLI flag:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "*": "deny",
    "read": "allow",
    "grep": "allow",
    "glob": "allow",
    "bash": {
      "*": "ask",
      "git status *": "allow",
      "git log *": "allow"
    },
    "edit": "ask"
  }
}
```

In interactive sessions, `ask` produces a prompt. In plain non-interactive `kilo run`, root-session prompts are auto-rejected unless `--auto` or `--dangerously-skip-permissions` is set; child-session prompts are denied instead of hanging.

The best locked-down wrapper invocation is not CLI-only:

```bash
KILO_DISABLE_PROJECT_CONFIG=1 \
KILO_PERMISSION='{"*":"deny","read":"allow","grep":"allow","glob":"allow"}' \
  kilo run "summarize the repository"
```

Concrete examples:

```bash
# Deny all, then allow read-only search.
KILO_PERMISSION='{"*":"deny","read":"allow","grep":"allow","glob":"allow"}' \
  kilo run "find auth entry points"

# Deny all, ask for bash, allow two safe git command patterns.
KILO_PERMISSION='{"*":"deny","bash":{"*":"ask","git status *":"allow","git diff *":"allow"}}' \
  kilo run --interactive "inspect git state"

# Select a preconfigured locked-down agent.
kilo run --agent plan "make a plan without editing files"
```

For Claudine, `KILO_PERMISSION` plus `KILO_DISABLE_PROJECT_CONFIG=1` is the practical session-scoped posture. It is not pure CLI, and adding permissions back in the same run must be done by building the JSON overlay before launch. PolicyEngine needs a Kilo backend that emits provider-native ordered rules.

### YOLO

Kilo's YOLO modes are:

- `kilo run --auto`
- `kilo run --dangerously-skip-permissions`
- Runtime allow-everything through the TUI/VS Code permission API
- Static config such as `permission: {"*": "allow"}`
- An agent whose rules allow everything

YOLO is available in interactive sessions through the runtime toggle and config. It is available in non-interactive sessions through `--auto` and `--dangerously-skip-permissions`.

YOLO does not bypass everything. Explicit deny rules still deny. Broad allow does not bypass sensitive `.env` read hardening. Provider config-file edits still force ask and disable always approval. Managed config loaded after normal config can constrain the result. The sandbox can still deny writes or selected network operations when enabled.

### Root User

No root/sudo-specific permission gate was found in docs or source. Kilo does not appear to disable `--auto`, `--dangerously-skip-permissions`, or allow-everything for root sessions. Running as root therefore increases the blast radius of allowed shell/file operations unless policy and sandboxing are configured carefully.

### Configuring the Default

User-scope defaults usually live in:

- macOS on this host: `~/.config/kilo/kilo.jsonc`
- Linux: `~/.config/kilo/kilo.jsonc`
- Windows: `%APPDATA%\kilo\kilo.jsonc`

Kilo also reads home `.kilo` / `.kilocode` config directories and legacy `opencode` file names. Repo-scope defaults can live in `kilo.jsonc`, `kilo.json`, `.kilo/kilo.jsonc`, or `.kilocode/kilo.jsonc`.

Examples:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "*": "ask",
    "read": "allow",
    "grep": "allow",
    "glob": "allow",
    "bash": {
      "*": "ask",
      "git status *": "allow",
      "git diff *": "allow"
    }
  }
}
```

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "agent": {
    "docs-writer": {
      "description": "Writes documentation only",
      "mode": "primary",
      "permission": {
        "*": "deny",
        "read": "allow",
        "edit": {
          "*": "deny",
          "*.md": "allow"
        }
      }
    }
  }
}
```

```markdown
---
description: Read-only security audit
mode: subagent
permission:
  "*": deny
  read: allow
  grep: allow
  glob: allow
  bash:
    "*": deny
    "git log *": allow
---

Review code and report findings without editing files.
```

### Extending the Base

A user can set broad personal defaults and let a repo narrow them:

```json
{
  "permission": {
    "*": "allow",
    "bash": "ask"
  }
}
```

```json
{
  "permission": {
    "edit": {
      "*": "deny",
      "docs/**": "allow"
    },
    "bash": {
      "*": "deny",
      "npm test *": "allow"
    }
  }
}
```

CLI and env can narrow further for one wrapper run:

```bash
KILO_DISABLE_PROJECT_CONFIG=1 KILO_PERMISSION='{"*":"deny","read":"allow"}' \
  kilo run "read package metadata only"
```

Managed config and macOS managed preferences load late and should be treated as administrative overrides.

## Tools and Permissions

Kilo's built-in/default tool surface includes:

- Shell/command: `bash`
- File/read/search: `read`, `glob`, `grep`, `list`
- File/write: `edit`, `write`, `apply_patch`
- Planning/todos: `todowrite`, plan enter/exit tools
- Delegation: `task`
- Web: `webfetch`, `websearch`
- Skills and context: `skill`, LSP when enabled
- Kilo-specific tools: `question`, `suggest`, `agent_manager`, `interactive_terminal`, `repo_clone`, `repo_overview`, notebook tools, semantic/code search when enabled
- MCP tools: sanitized names such as `{server}_{tool}`
- Custom plugin tools: exported from configured plugin/tool files

Permission checks map tool calls to permission names. `write` and `apply_patch` use `edit`. `task` checks the requested subagent name. MCP checks use the generated tool key. Bash checks parse shell commands and may also trigger `external_directory` if a command touches paths outside the worktree.

Rule decisions are `allow`, `ask`, and `deny`; prompt replies are `once`, `always`, and `reject`. Static rule conflicts are last-match-wins, but runtime saved approvals cannot override base denials, and broad saved allows cannot bypass base asks unless the saved pattern is at least as broad.

Approvals can persist. Source shows `always` and explicit always-rule saves update global config, while the UI text says the approval lasts until Kilo is restarted. The implementation is more durable than that text for ordinary rules. Protected config-path requests are the exception: they force `ask`, hide/disable always, and downgrade always replies to once.

## Sandboxing, Trust, and Administrative Controls

Kilo's sandbox is separate from approval mode. It is configured with:

```json
{
  "experimental": {
    "sandbox": true,
    "sandbox_restrict_network": true
  }
}
```

It is off by default. When enabled, it uses macOS `sandbox-exec`/Seatbelt or Linux bubblewrap. Windows has no backend. The sandbox restricts shell commands and file-write tools to the project/worktree plus Kilo state roots. It denies writes to `.git`, the sandbox policy store, and sandbox preference store. It does not confine file reads.

Network restriction is on by default when sandboxing is enabled. It blocks model-originated shell network, first-party HTTP tools, custom tools, and remote MCP delegated authority. It does not block provider/model traffic, local MCP servers, or plugin hooks. `allowedHosts` and proxy modes exist in the lower-level profile type but current source fails closed for non-empty `allowedHosts` and proxy.

No folder-trust prompt was found. Project config loads by default unless `KILO_DISABLE_PROJECT_CONFIG=1` is set. Managed config files and macOS MDM preferences load late and override lower layers. Kilo also has Kilo Cloud organization config for managed org settings.

Protected paths and files include sensitive `.env` reads, provider config files, AGENTS files, `.kilo` / `.kilocode` config paths, `.git` writes under sandbox, and sandbox policy/preference stores.

The honest security posture is mixed: static client-side policy plus UX prompts, with optional OS-enforced sandboxing for writes and selected network operations. It is not a full containment sandbox.

## MCP and Permissions

MCP tools use the same permission system as built-in tools. Each tool is named with a sanitized server prefix, for example `github_create_pull_request`, so rules can target a single tool or a group:

```json
{
  "mcp": {
    "github": {
      "type": "remote",
      "url": "https://example.com/mcp",
      "enabled": true
    }
  },
  "permission": {
    "github_*": "ask",
    "github_create_pull_request": "deny"
  }
}
```

MCP can be made safer by disabling unused servers, denying broad server patterns by default, and allowing only selected tools. Subagents inherit parent denials for MCP-prefixed tool names, so Plan Mode and locked-down orchestrators can constrain child sessions.

Server-level filtering is `enabled: false`. Tool-level filtering is ordinary permission rules. Remote MCP OAuth is stored per user. MCP resources are exposed through the MCP service, but no separate resource-level permission key was found. Local MCP servers and plugin hooks are outside sandbox network restriction; remote MCP tools are marked as delegated authority and are checked by Kilo's sandbox network assertion when restriction is active.

## Non-Interactive Behavior

Plain `kilo run` subscribes to permission events and auto-rejects root-session prompts. Child-session asks from a headless root are denied to avoid hanging. `--auto` auto-approves the root and tracked Task child sessions. `--dangerously-skip-permissions` auto-approves non-denied prompts. `--format json` changes event output but not this permission behavior.

Kilo has a programmatic approval API: list pending permissions, reply once/always/reject, save always allow/deny rules, and enable allow-everything. That makes it possible for wrappers to supervise prompts when attached to a server, but the CLI has no native no-tools/no-permissions flag.

## Changelog

- **2026-07-03** — Refreshed against current upstream `main` commit `419ff008ef180dd7076f679a89442883ba8f8d86`, installed CLI `7.3.45`, current Kilo docs, and local config inspection. Corrected defaults, `.env` behavior, global config path, non-interactive behavior, approval persistence, config-path protection, sandbox scope, MCP handling, and frontmatter metadata.
- **2026-07-02** — Legacy research recorded an OpenCode-like permissions model, Kilo-specific permission keys, sandbox support, managed config layers, and CLI auto-approval flags. This update verifies and revises those claims instead of treating them as current truth.

## Sources

- [Kilo Code Agent Permissions](https://kilo.ai/docs/customize/agent-permissions)
- [Kilo Code Auto-Approving Actions](https://kilo.ai/docs/getting-started/settings/auto-approving-actions)
- [Kilo Code Sandboxing](https://kilo.ai/docs/getting-started/settings/sandboxing)
- [Kilo Code CLI Command Reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code Custom Subagents](https://kilo.ai/docs/customize/custom-subagents)
- [Kilo Code .kilocodeignore](https://kilo.ai/docs/customize/context/kilocodeignore)
- [Kilo Code repository](https://github.com/Kilo-Org/kilocode)
- [Source: `packages/core/src/permission.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/permission.ts)
- [Source: `packages/core/src/plugin/agent.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/plugin/agent.ts)
- [Source: `packages/opencode/src/permission/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/permission/index.ts)
- [Source: `packages/opencode/src/config/config.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/config.ts)
- [Source: `packages/opencode/src/config/permission.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/permission.ts)
- [Source: `packages/opencode/src/config/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/paths.ts)
- [Source: `packages/opencode/src/config/managed.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/managed.ts)
- [Source: `packages/opencode/src/cli/cmd/run.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/run.ts)
- [Source: `packages/opencode/src/kilocode/permission/config-paths.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/permission/config-paths.ts)
- [Source: `packages/opencode/src/kilocode/sandbox/policy.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/sandbox/policy.ts)
- [Source: `packages/kilo-sandbox/src/backend.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-sandbox/src/backend.ts)
- [Source: `packages/opencode/src/mcp/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/index.ts)
