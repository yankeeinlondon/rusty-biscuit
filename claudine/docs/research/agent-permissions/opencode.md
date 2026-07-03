---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: auto
    style: switch
    description: "Start the session in auto-approve mode. Any permission request that is not explicitly denied is approved automatically instead of prompting."
    example: "opencode run --auto \"refactor the auth module\""
    example_description: "Runs a headless prompt where all non-denied permission requests are approved without interactive approval."
  - param: agent
    style: switch
    description: "Select the active agent for the session. Each agent can define its own permission profile, so this flag determines which permission set is evaluated for tool calls."
    example: "opencode --agent plan"
    example_description: "Starts an interactive session with the plan agent, which defaults bash and edit to ask."
  - param: permissions
    style: switch
    description: "For opencode agent create only. Comma-separated list of permissions to allow when scaffolding a new agent. Anything omitted is denied in the generated agent. Aliased as --tools."
    example: "opencode agent create --permissions read,grep --mode subagent"
    example_description: "Creates a new read-only subagent that is allowed only read and grep."
  - param: pure
    style: switch
    description: "Run without external plugins. This removes plugin-provided tools, hooks, and customizations from the session, reducing the accessible tool surface."
    example: "opencode --pure"
    example_description: "Starts a session with only built-in tools and configuration, ignoring external plugins."

env_vars:
  - name: OPENCODE_PERMISSION
    effect: "Provides an inline JSON permissions configuration that is merged into the effective config for the session."
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Provides inline JSON config content that can include a permission object and overrides most config file values."
  - name: OPENCODE_CONFIG
    effect: "Points to a custom config file path; that file may define permissions and is loaded between global and project config."
  - name: OPENCODE_CONFIG_DIR
    effect: "Points to a custom config directory that is searched for agents, commands, modes, and plugins like the standard .opencode directory."
  - name: OPENCODE_ENABLE_EXA
    effect: "Enables the websearch tool when set to any truthy value."
  - name: OPENCODE_EXPERIMENTAL_LSP_TOOL
    effect: "Enables the experimental lsp tool when set to true."
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: "Disables default plugins, reducing the tool surface available to the session."
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: "Disables reading from .claude directories, including prompts and skills."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: "Disables reading ~/.claude/CLAUDE.md."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: "Disables loading .claude/skills."

config_files:
  - os: all
    user: ~/.config/opencode/opencode.json
    repo: opencode.json
    notes: "Both JSON and JSONC are supported. TUI-specific settings live in tui.json alongside the config file."
  - os: macos
    user: /Library/Application Support/opencode/opencode.json
    repo: ""
    notes: "File-based managed config. Requires admin/root access to write."
  - os: linux
    user: /etc/opencode/opencode.json
    repo: ""
    notes: "File-based managed config. Requires admin/root access to write."
  - os: windows
    user: "%ProgramData%\\opencode\\opencode.json"
    repo: ""
    notes: "File-based managed config. Requires admin access to write."

precedence:
  - source: cli
    scope: [approval_mode]
    merge_strategy: none
    notes: "CLI flags such as --auto are temporary session overrides."
  - source: env
    scope: [permissions, config_content, config_path, config_dir]
    merge_strategy: none
    notes: "OPENCODE_PERMISSION, OPENCODE_CONFIG_CONTENT, OPENCODE_CONFIG, and OPENCODE_CONFIG_DIR apply for the session. They override file-based config except where managed settings take precedence."
  - source: managed_preferences
    scope: [all_config]
    merge_strategy: none
    notes: "On macOS, .mobileconfig deployed via MDM under ai.opencode.managed is the highest-priority config source and cannot be overridden by users."
  - source: managed_config_files
    scope: [all_config]
    merge_strategy: none
    notes: "opencode.json under /Library/Application Support/opencode/, /etc/opencode/, or %ProgramData%\\opencode\\ overrides all lower config sources."
  - source: inline_config
    scope: [config]
    merge_strategy: deep
    notes: "OPENCODE_CONFIG_CONTENT is loaded after .opencode directories and before managed config files."
  - source: project_config
    scope: [config]
    merge_strategy: deep
    notes: "opencode.json in the project root. Project configs override global and remote defaults; later conflicting keys win."
  - source: custom_config_path
    scope: [config]
    merge_strategy: deep
    notes: "OPENCODE_CONFIG file is loaded between global and project config."
  - source: dot_opencode_directories
    scope: [agents, commands, plugins, modes, tools, skills, themes, permissions]
    merge_strategy: deep
    notes: ".opencode directories are loaded after project config. Agent-specific permission objects override global permission objects."
  - source: global_user_config
    scope: [config]
    merge_strategy: deep
    notes: "~/.config/opencode/opencode.json overrides remote organizational defaults."
  - source: remote_well_known_config
    scope: [config]
    merge_strategy: deep
    notes: ".well-known/opencode endpoint provides organizational defaults and is loaded first."

default_posture: "With no configuration, OpenCode uses permissive defaults: most built-in tools are allowed automatically, while doom_loop and external_directory ask for approval. The read tool is allowed by default, but .env files are denied."

cli_zero_permissions:
  supported: false
  invocation: "OPENCODE_PERMISSION='{\"*\":\"deny\"}' opencode run \"...\""
  mechanism: "OpenCode has no dedicated CLI flag for a no-permissions baseline. The closest session-scoped option is the OPENCODE_PERMISSION environment variable, which sets a deny-all rule."
  limitations: "There is no native --permission or --no-tools runtime flag. --pure removes external plugins but leaves built-in tools. --agent can select a locked-down agent, but the agent must already be configured. Additional permissions must be added via OPENCODE_PERMISSION or pre-configured agents."

agent_permissions:
  allowed: true
  fm_properties:
    - permission
    - tools

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--auto flag (or setting permission to allow/all in config). In the TUI, auto-approve permissions can also be toggled from the command palette."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "OpenCode permissions are tool-centric (read, edit, bash, webfetch, skill, question, doom_loop, etc.) and support wildcard/last-match-wins patterns, while PolicyEngine's canonical model is organized around filesystem, command, network, MCP, agent, and runtime axes."
    - "The default permissive posture (most tools allow by default) is the inverse of PolicyEngine's typical ask/deny defaults, requiring explicit modeling."
    - "external_directory is a path-scoped permission key rather than a true tool, and PolicyEngine would need to represent it as a workspace/external path rule with matching semantics."
    - "doom_loop is a runtime recovery guard, not a standard tool or resource permission."
    - "Agent-specific permissions and task subagent permissions are supported by OpenCode but require PolicyEngine to scope rules by agent name."
    - "MCP tools are addressed by server-prefixed wildcard names (e.g., mymcp_*); PolicyEngine's MCP axis may not support arbitrary tool-name wildcard rules."
    - "OpenCode has no OS-enforced sandbox to model; its security boundary is the static policy engine."
    - "There is no CLI flag for a zero-permission baseline, so PolicyEngine mutations would need to emit env var or config-file changes."

permission_entities:
  - entity: tool
    native_names: [permission, tools]
    notes: "Built-in tools such as bash, read, edit, glob, grep, list, lsp, skill, todowrite, webfetch, websearch, and question are gated by permission keys."
  - entity: tool_group
    native_names: [edit]
    notes: "The edit permission covers edit, write, and apply_patch as a group."
  - entity: command
    native_names: [bash]
    notes: "bash permission rules match parsed command strings with glob semantics."
  - entity: path
    native_names: [read, edit, glob, grep, list, external_directory]
    notes: "read, edit, glob, grep, list, and external_directory rules can match file paths, glob patterns, regexes, or external directory prefixes."
  - entity: workspace
    native_names: [external_directory]
    notes: "external_directory gates access to paths outside the project working directory."
  - entity: mcp_server
    native_names: [mcp]
    notes: "MCP servers are configured under the mcp object and can be enabled or disabled."
  - entity: mcp_tool
    native_names: ["<server>_*", "<server>_<tool>"]
    notes: "MCP tools are registered with the server name as a prefix and can be targeted by permission wildcards."
  - entity: agent
    native_names: [agent, default_agent]
    notes: "Agents can define their own permission objects that override global permissions."
  - entity: subagent
    native_names: [task]
    notes: "The task permission controls which subagents can be spawned."
  - entity: mode
    native_names: ["--auto", auto-approve]
    notes: "--auto changes the session baseline so non-denied requests are approved automatically."
  - entity: approval_category
    native_names: [allow, ask, deny]
    notes: "The three decision values for permission rules."
  - entity: hook
    native_names: []
    notes: "OpenCode supports custom tools and plugins but does not have a documented PreToolUse hook for permission decisions."
  - entity: extension
    native_names: [plugin, --pure]
    notes: "Plugins can add tools and hooks; --pure disables external plugins for the session."
  - entity: slash_command
    native_names: []
    notes: "Slash commands such as /init and /undo are not separately permission-gated."

approval_modes:
  - name: default
    effect: "Most built-in tools are allowed automatically; doom_loop and external_directory ask for approval."
    interactive: true
    non_interactive: true
    aliases: [default]
  - name: auto-approve
    effect: "Non-denied permission requests are approved automatically. Explicit deny rules are still enforced."
    interactive: true
    non_interactive: true
    aliases: ["--auto", auto, "Enable auto-approve permissions"]
  - name: plan-agent
    effect: "The built-in plan agent sets edit and bash to ask by default, preventing file modifications."
    interactive: true
    non_interactive: false
    aliases: [plan, "--agent plan"]

rule_model:
  decisions: [allow, ask, deny]
  syntax: "permission_key -> action string, or permission_key -> {pattern: action}. Custom/MCP tools can be targeted by wildcard keys."
  precedence: "Rules are evaluated in order across merged rulesets; the last matching rule wins. Deny rules do not have special precedence over allow rules except by order."
  merge_semantics: "Config files merge with later sources overriding earlier sources for conflicting keys. Permission rulesets are concatenated in precedence order, so later sources' rules can override earlier sources. Agent-specific permission objects override global permission objects for that agent."
  matcher_semantics: "* matches zero or more characters; ? matches exactly one character; all other characters match literally. ~ and $HOME at the start of a pattern expand to the user's home directory."
  default_decision: "Most permissions default to allow. doom_loop and external_directory default to ask. read is allow but .env files are denied."

tool_visibility:
  supported: true
  mechanisms:
    - "The legacy tools object can disable individual tools or MCP server tool patterns by setting them to false."
    - "A permission deny rule with pattern * for a tool removes it from the model's context."
    - "--pure disables external plugins, removing plugin-provided tools."
    - "Agent-specific permission objects can restrict the tool surface for that agent."
  notes: "Tool visibility and approval policy are both expressed through the same permission object. A denied tool is hidden from the model."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: ""
  network_control: ""
  notes: "OpenCode does not provide an OS-enforced sandbox. Bash commands run in the user's shell environment with the user's privileges."

trust_and_admin:
  folder_trust: "OpenCode does not document a folder/project trust gate that disables project config, memory, or extensions."
  managed_policy: "Managed settings can be delivered via file-based opencode.json in system directories (/Library/Application Support/opencode/, /etc/opencode/, or %ProgramData%\\opencode\\) or via macOS MDM .mobileconfig under the ai.opencode.managed domain. These occupy the highest precedence tier and cannot be overridden by user or project configuration."
  safe_mode: "OpenCode does not have a dedicated safe-mode flag. --pure disables external plugins but keeps built-in tools and permissions."
  notes: "Enterprise deployments can use central config and SSO integration to restrict providers and configuration."

mcp_permissions:
  supported: true
  server_filters:
    - "MCP servers can be enabled or disabled with enabled: true/false."
    - "The tools object (deprecated) can disable an entire MCP server or pattern."
    - "Permission rules can target MCP tools by server-prefixed wildcard names."
  tool_filters:
    - "Permission rules such as mymcp_*: deny or mymcp_write_file: ask apply to MCP tools."
    - "Agent-specific permission objects can further restrict MCP tools for that agent."
  trust_model: "OAuth tokens for remote MCP servers are stored per user in ~/.local/share/opencode/mcp-auth.json. Project-scoped MCP servers load if already configured; no interactive trust dialog is documented."
  notes: "MCP tools run outside any OS sandbox. stdio MCP servers are local subprocesses; remote MCP servers make network requests from outside OpenCode's process."

headless_behavior: "In non-interactive opencode run mode, interactive permission prompts cannot be shown. Any tool call that would ask for approval is effectively blocked unless --auto is used or the permission is pre-approved. Use OPENCODE_PERMISSION or an allow-all/deny-all config to avoid hangs."

approval_persistence: "Approvals granted with always persist only for the rest of the current OpenCode session. They are not saved across sessions or projects."

protected_paths:
  - "*.env"
  - "*.env.*"

security_posture: "OpenCode's permission system is a client-side static policy engine with advisory prompts, not an OS-enforced sandbox. Rules are evaluated by the OpenCode process; a bypass in the client or a model-level jailbreak could circumvent them. There is no separate sandbox layer for Bash subprocesses."

changes:
  - "Updated config precedence to match current docs: managed config files and macOS MDM preferences are the highest tier; OPENCODE_CONFIG_CONTENT sits between .opencode directories and managed config."
  - "Documented the deprecated status of the tools object and its merge into permission."
  - "Added list as a granular permission key in addition to read, edit, glob, grep, bash, task, external_directory, lsp, and skill."
  - "Clarified that OpenCode has no OS-enforced sandbox; security posture is static policy plus advisory prompts."
  - "Corrected rule precedence: last matching rule wins across merged rulesets, not deny-wins."
  - "Documented subagent permission derivation: parent session deny and external_directory rules are inherited, plus default denials for task and todowrite unless the subagent permits them."
  - "Updated YOLO/auto-approve coverage: only --auto and config allow values; no root/sudo restrictions documented."
  - "Added --pure as a session-scoped way to reduce tool surface by disabling external plugins."
  - "Recorded that OpenCode has no native CLI flag for a zero-permission baseline; OPENCODE_PERMISSION env var is the session-scoped alternative."
  - "Expanded MCP permission coverage with server enablement, tool-level wildcard rules, and OAuth credential storage."

requires_claudine_update: true
reason: "OpenCode's tool-based permission grammar (wildcard patterns, last-rule-wins evaluation, external_directory, doom_loop, agent/task permissions, permissive defaults, and lack of OS sandbox) does not map cleanly to PolicyEngine's canonical axes. Supporting OpenCode permissions accurately in Claudine will require backend work in the PolicyEngine OpenCode backend and mutation planning for opencode.json permission objects."
---

# OpenCode CLI Permissions

## Introduction to OpenCode CLI Permissions

OpenCode controls tool access with a single `permission` configuration object. Each permission key maps to one or more tools and resolves to one of three actions:

- `"allow"` — run without approval
- `"ask"` — prompt the user for approval
- `"deny"` — block the action

Permissions can be configured through JSON config files, inline environment variables, Markdown agent frontmatter, and a small set of CLI flags. Unlike some other agents, OpenCode defaults to a permissive posture: most tools are allowed unless a rule says otherwise.

### Configuration files

The `permission` key lives in `opencode.json` (or `opencode.jsonc`). It can be a single action string that applies to all tools, or an object that maps tool names to action strings or granular pattern objects. See [Configuring the Default](#configuring-the-default) for file locations and examples.

### Environment variables

The main environment variables that influence permissions are:

| Variable | Effect |
| :----- | :----- |
| `OPENCODE_PERMISSION` | Inline JSON permissions config merged into the effective config. |
| `OPENCODE_CONFIG_CONTENT` | Inline JSON config content; can include a full `permission` object. |
| `OPENCODE_CONFIG` | Path to a custom config file that may contain a `permission` object. |
| `OPENCODE_CONFIG_DIR` | Path to a custom config directory searched for agents, commands, modes, and plugins. |
| `OPENCODE_ENABLE_EXA` | Enables the `websearch` tool when set to a truthy value. |
| `OPENCODE_EXPERIMENTAL_LSP_TOOL` | Enables the experimental `lsp` tool. |
| `OPENCODE_DISABLE_DEFAULT_PLUGINS` | Disables default plugins, reducing the tool surface. |
| `OPENCODE_DISABLE_CLAUDE_CODE` | Disables reading `.claude` directories. |
| `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT` | Disables reading `~/.claude/CLAUDE.md`. |
| `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` | Disables loading `.claude/skills`. |

### CLI parameters

Only a few CLI switches directly affect permissions or the tool surface:

| Flag | What it does |
| :----- | :----- |
| `--auto` | Enable auto-approve mode for the session. Non-denied requests are approved automatically. |
| `--agent <name>` | Use the named agent, whose `permission` profile (if any) is applied. |
| `--permissions <list>` | Only for `opencode agent create`. Lists permissions to allow in the generated agent. Aliased as `--tools`. |
| `--pure` | Run without external plugins, removing plugin-provided tools and hooks. |

### Precedence

Effective permissions are built from multiple layers. Config files merge together, with later sources overriding earlier sources for conflicting keys. Within that merged config, permission rules are evaluated in order and the **last matching rule wins**.

Config-source ordering (later wins):

1. Remote config from `.well-known/opencode`
2. Global user config `~/.config/opencode/opencode.json`
3. Custom config path from `OPENCODE_CONFIG`
4. Project `opencode.json`
5. `.opencode` directories (agents, commands, plugins, modes, tools, skills, themes)
6. Inline config from `OPENCODE_CONFIG_CONTENT`
7. Managed config files in system directories
8. macOS managed preferences via MDM `.mobileconfig`

Session-scoped overrides sit above the config stack:

- Environment variables such as `OPENCODE_PERMISSION` and `OPENCODE_CONFIG_CONTENT` apply for the session.
- CLI flags such as `--auto` apply for the session.
- Managed settings (file-based and MDM) occupy the highest precedence tier and cannot be overridden by users.

Within any config file, an agent-specific `permission` object overrides the global `permission` object for that agent.

### Permission policy vs tool visibility

OpenCode does not have a separate visibility layer independent of approval policy. Both concerns are expressed through the same `permission` object (and the deprecated `tools` object):

- A tool with `"deny"` and pattern `"*"` is removed from the model's context entirely.
- A tool with `"allow"` is visible and runs without prompting.
- A tool with `"ask"` is visible but prompts before each call.
- `--pure` removes plugin-provided tools without changing permission rules.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, OpenCode starts from permissive defaults:

- Most tools are `"allow"`.
- `doom_loop` and `external_directory` are `"ask"`.
- `read` is `"allow"`, but `.env` files are denied by default:

```json
{
  "permission": {
    "read": {
      "*": "allow",
      "*.env": "deny",
      "*.env.*": "deny",
      "*.env.example": "allow"
    }
  }
}
```

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for workspace paths; Deny for `.env` files.
- `can_write(path)` → Allow for workspace paths.
- `can_execute(command)` → Allow for bash commands.
- `can_access_domain(domain)` → Allow for webfetch/websearch.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Allow.
- `can_spawn_subagent(agent)` → Allow.
- `can_loop_recovery()` → Ask (doom_loop).
- `can_access_external_directory(path)` → Ask.

This use case is not ergonomic in PolicyEngine without adjustments. PolicyEngine's canonical axes (filesystem, command, network, MCP, agent, runtime) do not line up one-to-one with OpenCode's tool keys, and the permissive default is the opposite of PolicyEngine's usual ask/deny baseline. No changes are required to describe the broad idea, but full coverage of the default posture would need new mappings for `doom_loop`, `external_directory`, and the `.env` deny rule.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, set a global deny rule and then add specific allow or ask rules.

```json
{
  "$schema": "https://opencode.ai/config.json",
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

In an interactive session, `ask` causes OpenCode to prompt. In a non-interactive run, `ask` is effectively deny because there is no user to approve, so you should pre-declare `allow` rules for any tool the headless session needs.

Because OpenCode does not have a dedicated runtime `--permission` flag, you usually whitelist through config or environment:

```bash
# Headless run with a locked-down allowlist via env
OPENCODE_PERMISSION='{"*":"deny","read":"allow","grep":"allow","bash":{"git status *":"allow"}}' \
  opencode run "summarize the auth module"

# Use the built-in plan agent to default bash/edit to ask
opencode --agent plan

# Create and use a read-only subagent
opencode agent create --permissions read,grep --mode subagent --description "read-only explorer"
opencode --agent read-only-explorer
```

PolicyEngine can express this use case as `SetApprovalMode` to a deny-by-default posture plus explicit `GrantRead`, `AllowCommand`, and similar rules. It is not fully ergonomic because OpenCode's tool-key wildcard patterns and last-match-wins ordering do not map directly to PolicyEngine's rule model. Without changes, PolicyEngine could describe the intent but not the exact pattern-matching behavior or agent-scoped deny defaults.

### YOLO

OpenCode's YOLO mode is called **auto-approve**. A session can be put into this mode by:

- Starting with `--auto`, for example `opencode --auto` or `opencode run --auto "..."`.
- Setting `permission: "allow"` or `permission: { "*": "allow" }` in config.
- Using an agent whose permissions are all `allow`.
- Toggling **Enable auto-approve permissions** from the TUI command palette.

Availability:

- **Interactive sessions**: yes, via `--auto` or the TUI toggle.
- **Non-interactive sessions**: yes, via `opencode run --auto`.

When in auto-approve mode:

- **Allowed**: any tool call that is not explicitly denied is approved automatically, including bash, edit/write, webfetch, websearch, MCP tools, and subagent spawns.
- **Still enforced**: explicit `"deny"` rules in config are still enforced; if a tool is denied it will not run.
- **Not allowed**: auto-approve cannot override managed/MDM config that denies an action.

### Root User

The public OpenCode documentation does not describe any special permission behavior when the CLI is started as root or under `sudo`. Unlike Claude Code, there is no documented restriction that disables auto-approve/YOLO mode for root sessions. Therefore, YOLO mode remains available to root sessions unless an administrator blocks it through managed config.

### Configuring the Default

Default permissions are configured through JSON config files at two main scopes:

- **User scope**: `~/.config/opencode/opencode.json` (also supported as `.jsonc`). Applies across all projects.
- **Repo scope**: `opencode.json` in the project root. Applies to everyone working in the repository and can be checked into version control.

Agent-specific defaults can also be defined in Markdown files under `~/.config/opencode/agents/` or `.opencode/agents/`.

Examples that illustrate the grammar:

```json
// ~/.config/opencode/opencode.json — user-wide defaults
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "*": "ask",
    "bash": {
      "*": "ask",
      "git *": "allow",
      "npm *": "allow"
    },
    "read": "allow"
  }
}
```

```json
// opencode.json — repo-shared defaults
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "edit": "ask",
    "bash": {
      "*": "ask",
      "npm test": "allow"
    },
    "external_directory": {
      "~/shared/**": "allow"
    }
  }
}
```

```json
// Agent config in opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "agent": {
    "review": {
      "mode": "subagent",
      "description": "Read-only code reviewer",
      "permission": {
        "edit": "deny",
        "write": "deny",
        "bash": "deny"
      }
    }
  }
}
```

```markdown
<!-- ~/.config/opencode/agents/review.md -->
---
description: Code review without edits
mode: subagent
permission:
  edit: deny
  bash: ask
  webfetch: deny
---
Only analyze code and suggest changes.
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes or CLI flags.

**Example 1: user allows, repo denies.**

User `~/.config/opencode/opencode.json`:

```json
{
  "permission": {
    "bash": {
      "rm *": "allow"
    }
  }
}
```

Repo `opencode.json`:

```json
{
  "permission": {
    "bash": {
      "rm *": "deny"
    }
  }
}
```

Result: `rm` is blocked in the repository because the later project config overrides the earlier global config.

**Example 2: repo default ask, CLI auto-approve override.**

Repo `opencode.json`:

```json
{
  "permission": {
    "edit": "ask",
    "bash": "ask"
  }
}
```

CLI:

```bash
opencode run --auto "apply the suggested refactor"
```

Result: the non-interactive run auto-approves non-denied edit and bash requests for this session.

**Example 3: global whitelist plus project additions.**

Global config:

```json
{
  "permission": {
    "*": "deny",
    "read": "allow",
    "grep": "allow"
  }
}
```

Repo `opencode.json`:

```json
{
  "permission": {
    "bash": {
      "npm test": "allow"
    }
  }
}
```

Result: in this repo, read, grep, and `npm test` are allowed; everything else is denied.

## Tools and Permissions

OpenCode provides the following built-in tools. Each tool is gated by a permission key. Some keys cover multiple tools.

| Tool | Permission key | Permission required by default |
| :----- | :----- | :----- |
| `bash` | `bash` | Allow |
| `edit` | `edit` | Allow |
| `write` | `edit` (covers all file modifications) | Allow |
| `apply_patch` | `edit` (covers all file modifications) | Allow |
| `read` | `read` | Allow, except `.env` files are denied |
| `grep` | `grep` | Allow |
| `glob` | `glob` | Allow |
| `list` | `list` | Allow |
| `lsp` | `lsp` | Allow (requires experimental flag) |
| `skill` | `skill` | Allow |
| `todowrite` / `todoread` | `todowrite` | Allow |
| `webfetch` | `webfetch` | Allow |
| `websearch` | `websearch` | Allow (requires OpenCode provider or `OPENCODE_ENABLE_EXA`) |
| `question` | `question` | Allow |
| `task` (subagent spawn) | `task` | Allow |

Permission rules match the tool input. For example, `bash` rules match parsed command strings, `read`/`edit` rules match file paths, `glob` rules match glob patterns, `grep` rules match regex patterns, and `webfetch` rules match URLs. Wildcards follow simple glob semantics: `*` matches zero or more characters, `?` matches exactly one character, and all other characters match literally. `~` and `$HOME` at the start of a pattern expand to the user's home directory.

Rules are evaluated in order across merged rulesets and the **last matching rule wins**, so a common pattern is to place `"*": "ask"` first and more specific allow/deny rules after it.

### Native permission entities

OpenCode's permission system is tool-centric. The native entities it can target are:

- **Tools** — each built-in tool has a permission key.
- **Tool groups** — `edit` covers `edit`, `write`, and `apply_patch`.
- **Commands** — `bash` permission rules match parsed command strings.
- **Paths** — `read`, `edit`, `glob`, `grep`, `list`, and `external_directory` match file paths or patterns.
- **Workspace/external directories** — `external_directory` gates paths outside the working directory.
- **MCP servers** — enabled/disabled via the `mcp` config object.
- **MCP tools** — targeted by server-prefixed wildcard names such as `mymcp_*` or `mymcp_search`.
- **Agents/subagents** — agents define their own `permission` object; `task` controls subagent spawning.
- **Mode** — `--auto` toggles auto-approve for the session.
- **Approval category** — `allow`, `ask`, `deny`.
- **Extensions/plugins** — `--pure` removes plugin-provided tools for the session.

### Rule grammar

Permission rules follow this grammar:

```json
{
  "permission": {
    "<tool-key>": "<action>",
    "<tool-key>": {
      "<pattern>": "<action>",
      "<pattern>": "<action>"
    }
  }
}
```

- `<action>` is one of `allow`, `ask`, `deny`.
- `<tool-key>` can be any built-in key, custom tool name, or MCP tool wildcard.
- `<pattern>` uses `*` (zero or more), `?` (one), and literal matching. `~` and `$HOME` expand to the home directory.

Conflict resolution is last-match-wins across the merged ruleset. A broad deny rule placed after a narrow allow rule will win, and vice versa. There is no special deny-beats-allow semantics.

### Approval modes

OpenCode does not use named coarse permission modes like Claude Code. Instead it has:

- **Default** — permissive defaults as described above.
- **Auto-approve** (`--auto`) — non-denied requests are approved automatically.
- **Plan agent** (`--agent plan`) — a built-in primary agent that defaults `edit` and `bash` to `ask`.

There is no `dontAsk`, `bypassPermissions`, or classifier-based `auto` mode.

### Persistence

When OpenCode prompts for approval, the UI offers three outcomes:

- `once` — approve just this request.
- `always` — approve future requests matching the suggested patterns for the rest of the current OpenCode session.
- `reject` — deny the request.

`always` approvals are session-only and are lost when OpenCode exits.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

OpenCode does **not** provide an OS-enforced sandbox. Bash commands run in the user's shell environment with the user's privileges. There is no Seatbelt, bubblewrap, seccomp, or similar isolation layer documented or implemented.

Because there is no sandbox, the permission system is the primary security control. A model-level bypass or prompt injection that convinces OpenCode to call a permitted tool will execute with the user's privileges.

### Trust and administrative controls

**Folder/project trust**: OpenCode does not document a folder trust gate that disables project config, memory, extensions, or MCP servers.

**Managed/admin policy**: managed settings can be delivered in two ways:

- **File-based**: drop an `opencode.json` or `opencode.jsonc` in `/Library/Application Support/opencode/` on macOS, `/etc/opencode/` on Linux, or `%ProgramData%\opencode\` on Windows. These directories require admin access to write.
- **macOS MDM**: deploy a `.mobileconfig` with PayloadType `ai.opencode.managed`. OpenCode reads `/Library/Managed Preferences/<user>/ai.opencode.managed.plist` and `/Library/Managed Preferences/ai.opencode.managed.plist`.

Managed settings occupy the highest precedence tier and cannot be overridden by user, project, or local config, nor by most environment variables or CLI flags. The `permission` object in a managed config is enforced like any other managed key.

**Safe/minimal mode**: OpenCode does not have a dedicated safe-mode flag. `--pure` disables external plugins for the session but keeps built-in tools and permissions.

### Protected paths

The only provider-reserved path protection documented is the default `.env` deny rule:

- `*.env` — denied
- `*.env.*` — denied
- `*.env.example` — explicitly allowed

There is no extensive list of protected dotfiles or provider config paths like some other agents maintain.

### Security posture

OpenCode's permission system is a **client-side static policy engine with advisory prompts**. It is not an OS-enforced sandbox. Managed settings provide administrative policy, but they are still enforced by the client. Effective security requires combining strict permission rules, careful agent configuration, and managed policy where available.

## MCP and Permissions

MCP servers add external tools that appear alongside built-in tools. Once a server is configured under the `mcp` object, its tools are registered with the server name as a prefix (for example, a server named `mymcp` exposes tools like `mymcp_search`).

Permissions interact with MCP tools in three ways:

1. **Server enablement**: A server can be enabled or disabled with `enabled: true`/`false`. A disabled server is not available.
2. **Tool-level rules**: The global `permission` object can target MCP tools by name or wildcard:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "mymcp_*": "ask",
    "mymcp_write_file": "deny",
    "mymcp_read_file": "allow"
  }
}
```

3. **Legacy `tools` object**: The deprecated `tools` object can also disable an entire MCP server or pattern:

```json
{
  "tools": {
    "mymcp_*": false
  }
}
```

To make MCP safer:

- Deny all MCP tools by default and allow only specific servers or operations.
- Disable high-risk servers globally and enable them only for specific agents.
- Use `ask` for write/delete operations while keeping read operations allowed.
- Keep the MCP server list short to reduce context size and attack surface.
- Use the experimental `policies` feature to deny untrusted LLM providers, since MCP servers may forward requests through configured providers.

MCP tools run **outside** any OpenCode sandbox. Remote MCP servers make network requests from outside the OpenCode process, and stdio MCP servers run as local subprocesses with the user's environment.

## Non-Interactive Behavior

In non-interactive `opencode run` mode, interactive permission prompts cannot be shown. Any tool call that would `ask` for approval is effectively blocked because there is no user to approve it. To avoid hangs:

- Pass `--auto` to approve non-denied requests automatically.
- Set `OPENCODE_PERMISSION` with explicit `allow` rules for every tool the headless session needs.
- Pre-configure a locked-down agent and select it with `--agent`.

`ask` rules do not automatically become `allow` in headless mode; they block the call.

## Sources

- [OpenCode docs - Permissions](https://opencode.ai/docs/permissions)
- [OpenCode docs - Config](https://opencode.ai/docs/config)
- [OpenCode docs - Tools](https://opencode.ai/docs/tools)
- [OpenCode docs - Agents](https://opencode.ai/docs/agents)
- [OpenCode docs - MCP servers](https://opencode.ai/docs/mcp-servers)
- [OpenCode docs - Policies](https://opencode.ai/docs/policies)
- [OpenCode docs - CLI](https://opencode.ai/docs/cli)
- [OpenCode config schema](https://opencode.ai/config.json)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)

## Changelog

- 2026-07-02: Refreshed research against current OpenCode documentation (v1.17.13) and config schema. Updated precedence to include managed config files and macOS MDM preferences. Documented the deprecated `tools` object, `list` permission key, subagent permission derivation, MCP server/tool controls, and the lack of OS-enforced sandbox. Expanded frontmatter to the full schema contract and flagged Claudine updates as required.
