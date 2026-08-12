---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: "--auto"
    style: switch
    description: "Run with auto-approval for permission requests that would otherwise prompt; explicit deny rules are still enforced."
    example: "opencode run --auto \"Refactor this module\""
    example_description: "Runs a headless prompt and replies once to each non-denied permission request."
  - param: "--dangerously-skip-permissions"
    style: switch
    description: "Hidden alias for the same auto-approval path as --auto; accepted by opencode run and treated as dangerous YOLO behavior."
    example: "opencode run --dangerously-skip-permissions \"Apply the requested changes\""
    example_description: "Auto-approves promptable permissions for one run without changing config."
  - param: "--yolo"
    style: switch
    description: "Hidden compatibility alias for auto-approval in opencode run."
    example: "opencode run --yolo \"Update dependencies\""
    example_description: "Uses the same auto-approval mechanism as --auto."
  - param: "--agent"
    style: switch
    description: "Select the active agent; agent permission rules are merged after global rules and can narrow or widen the effective policy."
    example: "opencode run --agent plan \"Review the design\""
    example_description: "Runs with the plan agent, whose built-in rules deny broad edit access."
  - param: "--dir"
    style: switch
    description: "Set the directory for the run. This determines the project/worktree boundary used by external_directory checks for local runs."
    example: "opencode run --dir packages/api \"Inspect this package\""
    example_description: "Runs from packages/api, affecting relative paths and external-directory evaluation."
  - param: "--attach"
    style: switch
    description: "Attach opencode run to a running OpenCode server. Permission prompts and saved approvals are handled by that server/session."
    example: "opencode run --attach http://127.0.0.1:4096 --password \"$OPENCODE_SERVER_PASSWORD\" \"Continue\""
    example_description: "Uses an existing server and supplies Basic Auth credentials."
  - param: "--password"
    style: switch
    description: "Basic Auth password for --attach; defaults to OPENCODE_SERVER_PASSWORD."
    example: "opencode run --attach http://127.0.0.1:4096 --password secret \"Summarize\""
    example_description: "Authenticates to a password-protected OpenCode server."
  - param: "--username"
    style: switch
    description: "Basic Auth username for --attach; defaults to OPENCODE_SERVER_USERNAME or opencode."
    example: "opencode run --attach http://127.0.0.1:4096 --username opencode --password secret \"Summarize\""
    example_description: "Overrides the Basic Auth username for an attached server."
  - param: "--pure"
    style: switch
    description: "Global flag that sets OPENCODE_PURE=1 and runs without external plugins, reducing plugin-provided tools and hooks."
    example: "opencode --pure run \"Inspect this repo\""
    example_description: "Starts a run with external plugins disabled for the process."
  - param: "agent create --permissions"
    style: switch
    description: "For opencode agent create only. Takes a comma-separated list of permission keys to allow in the generated agent; omitted permissions are denied. Alias: --tools."
    example: "opencode agent create --path .opencode --description \"Read-only reviewer\" --mode subagent --permissions read,grep,glob"
    example_description: "Creates a subagent whose frontmatter denies every listed agent-create permission not named in the allow list."
  - param: "agent create --tools"
    style: switch
    description: "Alias for agent create --permissions."
    example: "opencode agent create --description \"No shell reviewer\" --mode subagent --tools read,grep,glob"
    example_description: "Uses the legacy tools alias to generate an agent with selected permissions."
  - param: "serve --hostname"
    style: switch
    description: "Bind address for the headless API server; affects exposure of the programmatic permission API."
    example: "OPENCODE_SERVER_PASSWORD=secret opencode serve --hostname 127.0.0.1"
    example_description: "Starts a local-only server with Basic Auth enabled."
  - param: "serve --port"
    style: switch
    description: "Port for the headless API server; affects where programmatic permission replies can be sent."
    example: "OPENCODE_SERVER_PASSWORD=secret opencode serve --port 4096"
    example_description: "Starts the API server on a predictable port."

env_vars:
  - name: OPENCODE_PERMISSION
    effect: "Inline JSON object merged into the effective legacy permission config after all file, directory, inline, and managed config sources; useful for session-scoped policy overlays."
    effect_category: policy_overlay
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Inline JSON/JSONC config loaded near the end of config precedence; can define permission, agent, mcp, plugin, and other security-control settings for one process."
    effect_category: config_injection
  - name: OPENCODE_CONFIG
    effect: "Path to a custom config file loaded after global config and before project config."
    effect_category: config_path_override
  - name: OPENCODE_CONFIG_DIR
    effect: "Custom config directory used like the standard config directory for opencode.json/opencode.jsonc, agents, commands, plugins, and related resources."
    effect_category: config_path_override
  - name: OPENCODE_PURE
    effect: "Truthy value disables external plugins; the CLI --pure flag sets this for the process."
    effect_category: customization_lockdown
  - name: OPENCODE_DISABLE_PROJECT_CONFIG
    effect: "Truthy value disables project config discovery, including project opencode.json/opencode.jsonc and project .opencode directories."
    effect_category: config_source_toggle
  - name: OPENCODE_ENABLE_EXA
    effect: "Truthily enables the websearch tool in current public docs when not using the OpenCode provider."
    effect_category: tool_surface
  - name: OPENCODE_WEBSEARCH_PROVIDER
    effect: "Selects the websearch backend in current source, which affects whether the websearch tool can execute."
    effect_category: tool_surface
  - name: EXA_API_KEY
    effect: "Credential used by the Exa websearch backend; presence affects websearch availability."
    effect_category: credential
  - name: PARALLEL_API_KEY
    effect: "Credential used by the Parallel websearch backend; presence affects websearch availability."
    effect_category: credential
  - name: OPENCODE_SERVER_PASSWORD
    effect: "Enables HTTP Basic Auth for opencode serve/web and is also the default password used by opencode run --attach."
    effect_category: credential
  - name: OPENCODE_SERVER_USERNAME
    effect: "Overrides the Basic Auth username for opencode serve/web and --attach; defaults to opencode."
    effect_category: credential

config_files:
  - os: macos
    user: ".config/opencode/opencode.json or .config/opencode/opencode.jsonc"
    repo: "opencode.json or opencode.jsonc; .opencode/opencode.json or .opencode/opencode.jsonc; .opencode/agents/*.md"
    notes: "OpenCode uses XDG config paths on macOS by default. Managed config can also live in /Library/Application Support/opencode/opencode.json or opencode.jsonc and macOS MDM managed preferences under ai.opencode.managed."
  - os: linux
    user: ".config/opencode/opencode.json or .config/opencode/opencode.jsonc"
    repo: "opencode.json or opencode.jsonc; .opencode/opencode.json or .opencode/opencode.jsonc; .opencode/agents/*.md"
    notes: "OpenCode uses XDG config paths. Managed config can also live in /etc/opencode/opencode.json or opencode.jsonc."
  - os: windows
    user: ".config\\opencode\\opencode.json or .config\\opencode\\opencode.jsonc"
    repo: "opencode.json or opencode.jsonc; .opencode\\opencode.json or .opencode\\opencode.jsonc; .opencode\\agents\\*.md"
    notes: "The source uses xdg-basedir for user config and %ProgramData%\\opencode for managed config. Windows shell/path behavior can affect external_directory matching."

precedence:
  - source: cli
    scope: [approval_mode, agents, security_controls, extensions, workspace]
    merge_strategy: none
    notes: "--auto/--yolo/--dangerously-skip-permissions are session flags. --pure sets OPENCODE_PURE. --agent selects the agent whose permissions are evaluated."
  - source: env_OPENCODE_PERMISSION
    scope: [rules]
    merge_strategy: deep
    notes: "Merged into result.permission after managed config and before legacy tools migration; later rule order can override earlier rules."
  - source: managed_preferences_macos
    scope: [general_config]
    merge_strategy: deep
    notes: "macOS MDM managed preferences under ai.opencode.managed override file, directory, inline, remote, and account config before OPENCODE_PERMISSION is applied."
  - source: managed_config_files
    scope: [general_config]
    merge_strategy: deep
    notes: "System managed opencode.json/opencode.jsonc is loaded from /Library/Application Support/opencode, /etc/opencode, or %ProgramData%\\opencode."
  - source: console_account_config
    scope: [general_config, provider_model]
    merge_strategy: deep
    notes: "Active OpenCode Console organization config is loaded late and can manage providers."
  - source: env_OPENCODE_CONFIG_CONTENT
    scope: [general_config, rules, agents, mcp, extensions]
    merge_strategy: deep
    notes: "Inline config is local-scoped and loaded after file and directory config."
  - source: config_directories
    scope: [rules, agents, slash_commands, extensions, mcp]
    merge_strategy: deep
    notes: "Global, discovered .opencode directories, and OPENCODE_CONFIG_DIR are loaded with nearer project directories applied later."
  - source: repo_config
    scope: [general_config, rules, agents, mcp, extensions]
    merge_strategy: nearest
    notes: "Project opencode.json/opencode.jsonc files are discovered upward from the run directory to the worktree and can be disabled with OPENCODE_DISABLE_PROJECT_CONFIG."
  - source: env_OPENCODE_CONFIG
    scope: [general_config]
    merge_strategy: deep
    notes: "Custom config file is loaded after global config and before project config."
  - source: user_config
    scope: [general_config]
    merge_strategy: deep
    notes: "User config includes config.json, opencode.json, and opencode.jsonc in the OpenCode config directory."
  - source: remote_well_known_config
    scope: [general_config]
    merge_strategy: deep
    notes: "Remote .well-known/opencode organizational config loads before user config."

default_posture: "The default build agent is permissive: most actions allow, external_directory and doom_loop ask, and question/plan control permissions are adjusted for the active agent. Current source asks before reading .env and .env.* files by default while allowing .env.example; older docs and prior research that said deny are no longer current."

cli_zero_permissions:
  supported: true
  invocation: "OPENCODE_PERMISSION='{\"*\":\"deny\"}' opencode --pure run \"...\""
  mechanism: "Session-scoped deny-all rule injected through OPENCODE_PERMISSION, optionally combined with --pure to remove external plugin tools."
  limitations: "There is no first-class --no-tools or --permission flag for run. Additional permissions cannot be added back with CLI flags in the same command except by encoding the entire rule overlay in OPENCODE_PERMISSION or OPENCODE_CONFIG_CONTENT. Built-in non-tool behavior, startup config loading, and server behavior still exist."

agent_permissions:
  allowed: true
  fm_properties:
    - permission
    - permissions
    - agent.<name>.permission
    - agents.<name>.permissions
    - tools

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--auto is documented for interactive and run modes; opencode run also accepts hidden --yolo and --dangerously-skip-permissions aliases. Non-interactive run auto-replies once to promptable permissions; explicit deny still blocks."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Claudine's current OpenCode backend must understand current source behavior: --auto, hidden YOLO aliases, OPENCODE_PERMISSION, OPENCODE_CONFIG_CONTENT, JSONC config files, managed config, .opencode directories, and saved always approvals."
    - "OpenCode permissions are flat ordered action/resource/effect rules internally but public docs still expose legacy permission object grammar; PolicyEngine needs exact migration/order semantics."
    - "Tool visibility is derived from whole-tool deny rules, not a separate allowlist; ask rules keep tools visible."
    - "MCP tools use server-prefixed tool names, while MCP resource tools use read permission with mcp:server:resource patterns."
    - "The default .env posture changed from deny to ask in current source, requiring provider metadata and tests to avoid stale assumptions."
    - "No OS sandbox exists, so PolicyEngine can model approvals but cannot honestly represent isolation."
    - "Agent/subagent permission inheritance includes parent session deny and external_directory rules plus subagent defaults; this is more nuanced than a simple per-agent override."

permission_entities:
  - entity: tool
    native_names: [bash, edit, write, apply_patch, read, grep, glob, list, lsp, skill, todowrite, webfetch, websearch, question]
    notes: "Tools are gated by permission action names. write and apply_patch map to edit; MCP resource list/read tools map to read."
  - entity: tool_group
    native_names: [edit, read]
    notes: "edit covers edit/write/apply_patch. read covers ordinary reads and MCP resource list/read helper tools for visibility decisions."
  - entity: command
    native_names: [bash]
    notes: "Shell permission resources are parsed command strings; patterns are wildcard matched."
  - entity: path
    native_names: [read, edit, glob, grep, external_directory]
    notes: "Path and pattern resources are wildcard matched. Leading ~ and $HOME expand in legacy permission config."
  - entity: workspace
    native_names: [external_directory, references]
    notes: "external_directory asks when a tool touches paths outside the project working directory; configured references are automatically allowed through this boundary."
  - entity: mcp_server
    native_names: [mcp, mcp.servers, enabled, disabled]
    notes: "Legacy docs/config use enabled false; current v2 schema uses disabled true. Server config determines whether MCP tools/resources exist."
  - entity: mcp_tool
    native_names: ["<server>_<tool>", "mcp_*"]
    notes: "MCP tools are registered under server-prefixed names and are permissioned by those names."
  - entity: mcp_resource
    native_names: [list_mcp_resources, list_mcp_resource_templates, read_mcp_resource, "mcp:<server>:<uri>"]
    notes: "MCP resource helpers ask for read permission with mcp:server:* or mcp:server:uri resource patterns."
  - entity: agent
    native_names: [agent, agents, default_agent, "--agent"]
    notes: "Agents can define permission rules and are selectable at runtime."
  - entity: subagent
    native_names: [task, general, explore]
    notes: "The task permission gates subagent launch by subagent type. Subagent sessions derive additional rules from parent denies and external_directory rules."
  - entity: mode
    native_names: [build, plan, "--auto", "--dangerously-skip-permissions", "--yolo"]
    notes: "Build and plan are built-in primary agents with different permissions. Auto/YOLO flags change approval response behavior."
  - entity: approval_category
    native_names: [allow, ask, deny, once, always, reject]
    notes: "allow/ask/deny are rule effects; once/always/reject are runtime replies to an ask."
  - entity: hook
    native_names: [permission.ask, tool.execute.before, tool.execute.after, shell.env]
    notes: "Plugins can observe permission asks and wrap tool execution; hooks are not an OS security boundary."
  - entity: extension
    native_names: [plugin, plugins, "--pure", OPENCODE_PURE]
    notes: "Plugins can add tools and hooks. --pure/OPENCODE_PURE removes external plugins for the process."
  - entity: slash_command
    native_names: [command, commands, ".opencode/commands"]
    notes: "Custom slash commands can execute shell interpolation during prompt construction and are loaded from user/project config surfaces."
  - entity: sandbox
    native_names: []
    notes: "OpenCode explicitly does not provide an OS sandbox."

approval_modes:
  - name: default
    effect: "Use configured allow/ask/deny rules. For the default build agent, most tools allow, external_directory and doom_loop ask, and .env reads ask."
    interactive: true
    non_interactive: true
    aliases: [build, default]
  - name: auto
    effect: "Automatically replies once to permission requests that reach ask; explicit deny remains denied."
    interactive: true
    non_interactive: true
    aliases: ["--auto", "--dangerously-skip-permissions", "--yolo"]
  - name: plan
    effect: "Built-in primary agent that denies broad edit access while allowing plan-file writes and plan exit."
    interactive: true
    non_interactive: true
    aliases: [plan, "--agent plan"]

rule_model:
  decisions: [allow, ask, deny]
  syntax: "Public legacy grammar: permission: { action: \"allow|ask|deny\" } or permission: { action: { resource_pattern: \"allow|ask|deny\" } }. Current internal grammar: ordered rules { action, resource, effect }. A top-level wildcard action \"*\" can provide a fallback or override depending on order."
  precedence: "Last matching rule wins among flattened rules. There is no hard deny-wins rule at pure evaluation time, but the v2 runtime checks configured denies before applying saved always approvals."
  merge_semantics: "Config sources deep-merge; legacy permission objects are converted to ordered rules preserving top-level key order and nested pattern order. Rule sets are concatenated, so later rules can override earlier ones."
  matcher_semantics: "Wildcard matching supports * and ? style matching through OpenCode's Wildcard matcher. Leading ~/ and $HOME in legacy permission pattern keys expand to the user's home directory; tildes in the middle of a path do not expand."
  default_decision: "Pure evaluation returns ask if no rule matches, but built-in agents seed defaults first. The build agent seeds * allow, doom_loop ask, external_directory ask with allowlisted internal/reference paths, question allow, plan_enter allow, plan_exit deny, and read rules that ask for .env and .env.*."

tool_visibility:
  supported: true
  mechanisms:
    - "Whole-tool deny rules with pattern/resource * hide tools from the model."
    - "The legacy tools object is migrated to permission deny/allow rules and remains supported."
    - "--pure / OPENCODE_PURE removes external plugin-provided tools and hooks."
    - "Agent permissions can hide task subagents, skills, and tools for a selected agent."
  notes: "Visibility is not the same as approval: ask keeps the tool visible and prompts at runtime, while deny with a full wildcard can remove the tool from the tool registry or skill list."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: "No OS-enforced filesystem sandbox. Permissions check tool calls in-process; bash runs with the user's host privileges."
  network_control: "No OS-enforced network sandbox. Network tools, remote MCP servers, and shell commands can use the host network if allowed or not otherwise blocked."
  notes: "OpenCode's SECURITY.md tells users to use Docker or a VM for true isolation."

trust_and_admin:
  folder_trust: "No folder trust prompt was found. Project config and .opencode directories are loaded by default; OPENCODE_DISABLE_PROJECT_CONFIG can disable project config discovery for a process."
  managed_policy: "Managed config files and macOS MDM preferences are supported and loaded at high precedence. Active Console organization config can also manage provider config."
  safe_mode: "There is no dedicated safe mode. --pure/OPENCODE_PURE disables external plugins but does not disable built-in tools or project config."
  notes: "Custom commands, agents, plugins, and MCP config are loaded from user and project config surfaces unless project config is disabled or plugins are suppressed."

mcp_permissions:
  supported: true
  server_filters:
    - "Configure only approved MCP servers under mcp/mcp.servers."
    - "Disable a server with enabled: false in legacy config or disabled: true in current schema."
    - "Use --pure to remove plugin-added MCP/tool surfaces, but configured MCP servers are still config-controlled."
  tool_filters:
    - "Set permission entries such as myserver_*: ask or myserver_write_file: deny for MCP tools."
    - "Set read rules for mcp:server:* and mcp:server:uri patterns to control MCP resources."
    - "Agent permission overrides can narrow MCP tool/resource access per agent."
  trust_model: "MCP servers are trusted configured tools. Local servers are subprocesses; remote servers make network calls. OAuth-capable remote servers store tokens per user, and MCP behavior is outside OpenCode's security boundary."
  notes: "MCP tools and resources do not run inside an OpenCode OS sandbox. Resource output is formatted/truncated, unsupported or oversized binary resources are omitted, and plugin tool hooks can observe executions."

headless_behavior: "In opencode run, permission requests are not presented interactively. Without --auto/--yolo/--dangerously-skip-permissions, run prints a permission requested message, auto-rejects the request, and continues; with auto mode it replies once. The server API exposes programmatic permission list/reply endpoints for clients that attach to or drive a running server."

approval_persistence: "Legacy interactive allow always persists only in the current process/session's approved list. Current v2 PermissionSaved stores always approvals in the database by projectID as allow rules, so wrappers must distinguish session-only legacy behavior from project-scoped saved approvals in the v2 server path."

protected_paths:
  - "*.env"
  - "*.env.*"
  - ".env.example"
  - "~/.config/opencode/opencode.json"
  - "~/.config/opencode/opencode.jsonc"
  - ".opencode/opencode.json"
  - ".opencode/opencode.jsonc"
  - ".opencode/agents/*.md"
  - ".opencode/plugins/*"

security_posture: "OpenCode permissions are an in-process static policy plus advisory/user-approval workflow, with plugin hooks and managed config layers. They are not an OS-enforced sandbox; root or ordinary user processes run with the privileges of the invoking user."

changes:
  - "Updated metadata to agent=codex/model=default and last_updated=2026-07-03."
  - "Replaced stale os: all config metadata with separate macOS, Linux, and Windows records."
  - "Verified installed OpenCode 1.17.13 and current source/docs instead of relying on prior research."
  - "Documented current --auto support plus hidden --yolo and --dangerously-skip-permissions aliases for opencode run."
  - "Changed default .env posture from deny to ask based on current source defaults."
  - "Added current config precedence details, including OPENCODE_CONFIG_CONTENT, OPENCODE_PERMISSION, OPENCODE_DISABLE_PROJECT_CONFIG, managed config files, macOS MDM preferences, and Console account config."
  - "Documented current internal permission rules as flat ordered action/resource/effect rules while keeping the public legacy permission grammar examples."
  - "Added session-scoped CLI zero-permission posture using OPENCODE_PERMISSION deny-all plus --pure."
  - "Added MCP resource permission behavior using read permission patterns such as mcp:server:* and mcp:server:uri."
  - "Added approval persistence distinction between legacy session always approvals and v2 project-scoped saved approvals."
  - "Added explicit security posture from OpenCode SECURITY.md: no OS sandbox."

requires_claudine_update: true
reason: "Claudine's OpenCode PolicyEngine/provider metadata should be updated for current OpenCode behavior: --auto aliases, JSONC/user/repo/.opencode config discovery, OPENCODE_CONFIG_CONTENT and OPENCODE_PERMISSION, managed config layers, v2 saved approvals, MCP resource rules, --pure, and the current default .env ask posture."
---

# OpenCode CLI Permissions and Security Controls

## Introduction to OpenCode CLI Permissions

OpenCode controls tool execution with permission rules. The public docs present the legacy user-facing config key as `permission`, while current source migrates that object into an ordered internal ruleset shaped like `{ action, resource, effect }`. The three native effects are `allow`, `ask`, and `deny`.

The public config grammar is still the safest grammar for Claudine to emit for users:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "*": "ask",
    "bash": {
      "*": "ask",
      "git status*": "allow",
      "git push*": "deny"
    },
    "edit": "deny",
    "external_directory": {
      "$HOME/projects/shared/*": "allow",
      "*": "ask"
    }
  }
}
```

Configuration can live in user config, repo config, `.opencode` directories, inline environment config, custom config paths, managed config, and agent Markdown frontmatter. OpenCode supports JSON and JSONC, and current source also seeds a user `opencode.json` with a schema pointer if no routed config is present.

Permission and tool visibility are related but distinct. `ask` leaves a tool visible to the model and prompts at runtime. A full wildcard `deny` for a tool can hide that tool from the model. The legacy `tools` boolean object is deprecated but still supported; it is migrated into permission allow/deny rules.

Important environment variables:

- `OPENCODE_PERMISSION` overlays a JSON permission object for the process.
- `OPENCODE_CONFIG_CONTENT` provides inline config content for the process.
- `OPENCODE_CONFIG` points at a custom config file.
- `OPENCODE_CONFIG_DIR` points at a custom config directory.
- `OPENCODE_PURE` disables external plugins; `--pure` sets it.
- `OPENCODE_DISABLE_PROJECT_CONFIG` disables project config discovery.

Relevant CLI switches:

- `--auto` auto-approves permission prompts that are not explicitly denied.
- Hidden `--dangerously-skip-permissions` and `--yolo` are accepted by `opencode run` and map to the same auto path.
- `--agent` selects the agent whose rules are evaluated.
- `--dir` changes the run directory and therefore the workspace boundary used by external-directory checks.
- `--pure` disables external plugins.
- `opencode agent create --permissions` or `--tools` generates an agent file where unlisted permissions are denied.

CLI/runtime switches have session/process scope and do not mutate config. `OPENCODE_PERMISSION` is applied after file and managed config, making it the best current wrapper mechanism for one-shot policy overlays.

## Permissions Use Cases

### Default

With no user, repo, env, or CLI policy override, OpenCode starts with the built-in `build` agent. Current source seeds:

- `*`: `allow`
- `doom_loop`: `ask`
- `external_directory`: `ask`, with internal temporary/skill/reference paths allowlisted
- `read`: `allow`, but `.env` and `.env.*` are `ask`, and `.env.example` is `allow`
- `question`: `allow` for the build agent
- `plan_enter`: `allow` and `plan_exit`: `deny` for the build agent

The current PolicyEngine can approximate filesystem, command, and subagent permissions, but it is not ergonomic for OpenCode because OpenCode's model is ordered tool/action/resource rules with tool visibility side effects. Claudine also needs updates for the current `.env` default, JSONC paths, env overlays, managed config, saved approvals, and MCP resource rules.

### Whitelisting

OpenCode does not expose a dedicated `--no-tools` or `--permissions` runtime flag for `opencode run`. The best CLI-only, session-scoped locked-down launch is:

```sh
OPENCODE_PERMISSION='{"*":"deny"}' opencode --pure run "..."
```

This starts from a deny-all permission overlay and disables external plugins. To add back permissions in the same run, encode the complete rule set in `OPENCODE_PERMISSION`:

```sh
OPENCODE_PERMISSION='{"*":"deny","read":"allow","grep":"allow","glob":"allow"}' opencode --pure run "Audit the repo"
```

```sh
OPENCODE_PERMISSION='{"*":"deny","read":"allow","bash":{"*":"ask","git status*":"allow"},"edit":"deny"}' opencode --pure run "Inspect status"
```

```sh
OPENCODE_PERMISSION='{"*":"deny","mymcp_*":"ask","read":{"mcp:docs:*":"ask"}}' opencode --pure run "Use docs only if needed"
```

PolicyEngine can describe this if it can emit an OpenCode env overlay rather than only persistent config edits. Current Claudine support is incomplete because the OpenCode backend does not fully model `OPENCODE_PERMISSION`, `--auto`, JSONC, managed config, and MCP resources.

### YOLO

YOLO-style operation is available as documented `--auto` and as hidden `opencode run` aliases `--dangerously-skip-permissions` and `--yolo`. In non-interactive `opencode run`, these flags automatically reply `once` to permission prompts. Explicit `deny` rules still block the action.

Interactive availability is documented for `opencode --auto`. The source inspected here specifically shows the hidden aliases on `opencode run`; the docs do not advertise the hidden aliases.

YOLO does not create a sandbox exception because there is no sandbox. It only changes `ask` to auto-approved; `deny` still wins.

### Root User

I found no OpenCode behavior that changes permission policy when run as root or administrator. Since OpenCode does not sandbox the agent, root execution is more dangerous: allowed shell/file operations run with root privileges. YOLO/auto mode is still a CLI/session behavior and is not documented or implemented as disabled for root.

### Configuring the Default

User-scope config:

- macOS/Linux: `~/.config/opencode/opencode.json` or `~/.config/opencode/opencode.jsonc`
- Windows: xdg-basedir-backed `.config\opencode\opencode.json` or `.config\opencode\opencode.jsonc` under the user's home-like config location in current source

Repo-scope config:

- `opencode.json` or `opencode.jsonc`
- `.opencode/opencode.json` or `.opencode/opencode.jsonc`
- `.opencode/agents/*.md`
- `.opencode/commands/*.md`
- `.opencode/plugins/*`

Agent Markdown frontmatter example:

```markdown
---
description: Read-only reviewer
mode: subagent
permission:
  "*": deny
  read: allow
  grep: allow
  glob: allow
---

Review code without modifying files.
```

### Extending the Base

User config can set a broad baseline:

```json
{
  "permission": {
    "*": "ask",
    "read": "allow",
    "edit": "deny"
  }
}
```

A repo can then allow a narrow write path by placing a later rule in project config:

```json
{
  "permission": {
    "edit": {
      "*": "deny",
      "docs/generated/*": "allow"
    }
  }
}
```

A wrapper can override both for one process:

```sh
OPENCODE_PERMISSION='{"edit":"deny","bash":{"*":"ask","just test*":"allow"}}' opencode run "Run focused checks"
```

Because last matching rule wins after merge/flattening, order matters. A later wildcard can override an earlier specific rule.

## Tools and Permissions

OpenCode's default built-in tool surface includes:

- `bash`
- `edit`
- `write`
- `read`
- `grep`
- `glob`
- `lsp`
- `apply_patch`
- `skill`
- `todowrite`
- `webfetch`
- `websearch`
- `question`
- `task` for subagents
- plan-control tools in current source when plan mode is enabled
- MCP tools and MCP resource helper tools when MCP servers expose them

Permission mapping:

| Tool or feature | Permission action |
| --- | --- |
| `bash` / shell | `bash` |
| `edit`, `write`, `apply_patch` | `edit` |
| `read` | `read` |
| `grep` | `grep` |
| `glob` | `glob` |
| `lsp` | `lsp` |
| `skill` | `skill` |
| `todowrite` | `todowrite` |
| `webfetch` | `webfetch` |
| `websearch` | `websearch` |
| `question` | `question` |
| subagent launch | `task` |
| outside workspace | `external_directory` |
| repeated identical tool call | `doom_loop` |
| MCP tool | generated name such as `server_tool` |
| MCP resources | `read` with `mcp:server:*` or `mcp:server:uri` resource |

Native permission entities are tool/action, resource pattern, command string, filesystem path, workspace boundary, MCP server/tool/resource, agent/subagent, plugin hook, slash command, and approval reply. There is no separate OS sandbox entity.

Rule decisions are `allow`, `ask`, and `deny`. Runtime prompt replies are `once`, `always`, and `reject`. `always` can add remembered allow rules; see persistence notes.

## Sandboxing, Trust, and Administrative Controls

OpenCode's own security policy says it does not sandbox the agent. Permissions are a UX feature and not security isolation. Shell commands run in the user's environment. File operations run with the user's filesystem privileges. Network-capable tools and MCP servers use the host network.

There is no documented folder trust prompt. Project config loads by default, including project `.opencode` directories. Use `OPENCODE_DISABLE_PROJECT_CONFIG=1` to disable project config discovery for a process.

Administrative controls:

- macOS/Linux/Windows managed config files in system directories.
- macOS MDM managed preferences under `ai.opencode.managed`.
- OpenCode Console account/org config for managed providers.
- Experimental provider policies under `experimental.policies`, currently for `provider.use`.

Protected or guarded paths are mostly modeled as permission defaults, not immutable provider-reserved paths. Current source asks for `.env` and `.env.*` reads by default and allows `.env.example`.

Security posture: advisory/static policy plus prompts and managed config. Use a container or VM for real isolation.

## MCP and Permissions

MCP servers are configured under `mcp` in config. Legacy config uses per-server `enabled`; current v2 schema uses `disabled`. Remote MCP can use OAuth or headers. Local MCP servers run as local subprocesses.

MCP tool permissions use the generated tool name. For example:

```json
{
  "permission": {
    "github_*": "ask",
    "github_delete_issue": "deny"
  }
}
```

MCP resources are different. If an MCP server exposes resources, OpenCode adds resource helper tools. These ask for `read` with resource patterns:

```json
{
  "permission": {
    "read": {
      "mcp:docs:*": "ask",
      "mcp:prod-secrets:*": "deny"
    }
  }
}
```

To make MCP safer:

- Disable unneeded servers in config.
- Use `--pure` to suppress plugin-added tool surfaces.
- Put MCP tools behind `ask` by server wildcard.
- Deny known destructive MCP tools explicitly.
- Gate MCP resources with `read` rules.
- Use agent-specific permissions for subagents that should not see MCP tools.

MCP tools and servers are not sandboxed by OpenCode. Local MCP servers can run subprocess logic, and remote MCP servers are outside OpenCode's trust boundary.

## Non-Interactive Behavior

`opencode run` cannot show an interactive prompt. When a permission request is emitted and auto mode is not enabled, it prints a warning, replies `reject`, and continues. With `--auto`, `--yolo`, or `--dangerously-skip-permissions`, it replies `once`.

The server API exposes pending permission listing and permission reply endpoints, so a client can programmatically approve or reject requests when driving a running server. That is different from plain one-shot `opencode run`, which auto-rejects unless auto mode is active.

## Changelog

- 2026-07-03: Refreshed against OpenCode docs, installed CLI 1.17.13, current source, and observed local config. Added current config precedence, CLI/env metadata, no-sandbox posture, MCP resource rules, and v2 approval persistence notes. Marked Claudine update required.
- 2026-07-02: Prior research documented the merged legacy `tools`/`permission` model and managed config at a high level.

## Sources

- [OpenCode Permissions](https://opencode.ai/docs/permissions/)
- [OpenCode Config](https://opencode.ai/docs/config/)
- [OpenCode CLI](https://opencode.ai/docs/cli/)
- [OpenCode Tools](https://opencode.ai/docs/tools/)
- [OpenCode Agents](https://opencode.ai/docs/agents/)
- [OpenCode Agent Skills](https://opencode.ai/docs/skills/)
- [OpenCode MCP servers](https://opencode.ai/docs/mcp-servers/)
- [OpenCode Policies](https://opencode.ai/docs/policies/)
- [OpenCode References](https://opencode.ai/docs/references/)
- [OpenCode Plugins](https://opencode.ai/docs/plugins/)
- [OpenCode SECURITY.md](https://github.com/anomalyco/opencode/blob/dev/SECURITY.md)
- [OpenCode source: permission/index.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/permission/index.ts)
- [OpenCode source: core permission.ts](https://github.com/anomalyco/opencode/blob/dev/packages/core/src/permission.ts)
- [OpenCode source: config/config.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/config.ts)
- [OpenCode source: config/managed.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/managed.ts)
- [OpenCode source: cli/cmd/run.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts)
- [OpenCode source: agent/agent.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/agent/agent.ts)
- [OpenCode source: session/tools.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/tools.ts)
- [Local observed config: /Users/ken/.claudine/.config/opencode/opencode.jsonc](/Users/ken/.claudine/.config/opencode/opencode.jsonc)
