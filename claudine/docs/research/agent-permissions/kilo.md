---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: auto
    style: switch
    description: Start a non-interactive `kilo run` session in auto-approve mode. Permission requests for the main session and tracked Task child sessions are approved automatically unless explicitly denied.
    example: kilo run --auto "refactor the auth module"
    example_description: Runs a headless task where all non-denied permission requests are approved, including permissions requested by spawned subagent tasks.
  - param: dangerously-skip-permissions
    style: switch
    description: Start a non-interactive `kilo run` session that auto-approves any permission request that is not explicitly denied. Unlike --auto, this flag does not track or auto-approve Task child-session permissions.
    example: kilo run --dangerously-skip-permissions "deploy to staging"
    example_description: Runs a headless deployment prompt with all non-denied permission requests approved for the main session only.
  - param: agent
    style: switch
    description: Select the active agent for the session. Each agent can define its own permission profile, so this flag determines which permission set is evaluated for tool calls.
    example: kilo --agent plan
    example_description: Starts an interactive session with the plan agent, which defaults write and bash permissions to ask.
  - param: permissions
    style: switch
    description: For `kilo agent create` only. Comma-separated list of permissions to allow when scaffolding a new agent. Any permission not listed is denied in the generated agent.
    example: kilo agent create --permissions read,grep --mode subagent
    example_description: Creates a new read-only subagent that is allowed only read and grep.

env_vars:
  - name: KILO_PERMISSION
    effect: Provides an inline JSON permissions configuration that is merged into the effective config for the session, overriding config-file permissions.
  - name: KILO_CONFIG_CONTENT
    effect: Provides inline JSON config content that can include a permission object and overrides most config file values.
  - name: KILO_CONFIG
    effect: Points to a custom config file path; that file may define permissions and is loaded between global and project config.
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: When set, skips loading project-level config files (kilo.json, opencode.json, .kilo/, .kilocode/, .opencode/), so only global, env, and CLI sources shape permissions.
  - name: KILO_SANDBOX
    effect: Controls sandbox behavior for Bash subprocesses. Values include `allow` (network allowed), `deny` (network denied), and `proxy` (experimental; not fully supported as of current source).
  - name: KILO_SANDBOX_ALLOWED_HOSTS
    effect: When set, restricts sandboxed network access to the listed hosts. Support status is experimental as of current source.

config_files:
  - os: macos
    user: ~/Library/Application Support/kilo/kilo.jsonc
    repo: kilo.json
    notes: "User-scope config on macOS. JSONC is supported. Legacy paths opencode.json/opencode.jsonc and ~/.opencode/ are also read for backward compatibility."
  - os: linux
    user: ~/.config/kilo/kilo.jsonc
    repo: kilo.json
    notes: "User-scope config on Linux. JSONC is supported. Legacy paths opencode.json/opencode.jsonc and ~/.opencode/ are also read for backward compatibility."
  - os: windows
    user: "%APPDATA%\\kilo\\kilo.jsonc"
    repo: kilo.json
    notes: "User-scope config on Windows. JSONC is supported. Legacy paths opencode.json/opencode.jsonc and ~/.opencode/ are also read for backward compatibility."
  - os: macos
    user: /Library/Application Support/kilo/kilo.jsonc
    repo: ""
    notes: "File-based managed config. Requires admin/root access to write."
  - os: linux
    user: /etc/kilo/kilo.jsonc
    repo: ""
    notes: "File-based managed config. Requires admin/root access to write."
  - os: windows
    user: "%ProgramData%\\kilo\\kilo.jsonc"
    repo: ""
    notes: "File-based managed config. Requires admin access to write."

precedence:
  - source: cli
    scope: [approval_mode]
    merge_strategy: none
    notes: "CLI flags such as --auto and --dangerously-skip-permissions are temporary session overrides."
  - source: env
    scope: [permissions, config_content, config_path, sandbox]
    merge_strategy: none
    notes: "KILO_PERMISSION, KILO_CONFIG_CONTENT, KILO_CONFIG, and KILO_SANDBOX apply for the session. They override file-based config except where managed settings take precedence."
  - source: managed_preferences
    scope: [all_config]
    merge_strategy: none
    notes: "On macOS, .mobileconfig deployed via MDM under the managed domain is the highest-priority config source and cannot be overridden by users."
  - source: managed_config_files
    scope: [all_config]
    merge_strategy: none
    notes: "kilo.jsonc under /Library/Application Support/kilo/, /etc/kilo/, or %ProgramData%\\kilo\\ overrides all lower config sources."
  - source: organization_config
    scope: [permissions, providers, rules]
    merge_strategy: none
    notes: "Active organization config from Kilo Gateway occupies the highest precedence tier for enterprise-managed rules and cannot be overridden by user or project config."
  - source: inline_config
    scope: [config]
    merge_strategy: deep
    notes: "KILO_CONFIG_CONTENT is loaded after .kilo/.kilocode directories and before managed config files."
  - source: project_config
    scope: [config]
    merge_strategy: deep
    notes: "kilo.jsonc in the project root. Project configs override global and remote defaults; later conflicting keys win."
  - source: dot_kilo_directories
    scope: [agents, commands, plugins, modes, tools, skills, themes, permissions]
    merge_strategy: deep
    notes: ".kilo and .kilocode directories (legacy .opencode) are loaded after project config. Agent-specific permission objects override global permission objects."
  - source: custom_config_path
    scope: [config]
    merge_strategy: deep
    notes: "KILO_CONFIG file is loaded between global and project config."
  - source: global_user_config
    scope: [config]
    merge_strategy: deep
    notes: "~/.config/kilo/kilo.jsonc (or XDG/macOS/Windows equivalents) overrides remote organizational defaults."
  - source: remote_well_known_config
    scope: [config]
    merge_strategy: deep
    notes: ".well-known/opencode endpoint provides organizational defaults and is loaded first."

default_posture: "With no configuration, Kilo Code uses permissive defaults: most built-in tools are allowed automatically, while doom_loop and external_directory ask for approval. The read tool is allowed by default, but .env files are denied."

cli_zero_permissions:
  supported: false
  invocation: 'KILO_PERMISSION=''{"*":"deny"}'' kilo run "..."'
  mechanism: "Kilo has no dedicated CLI flag for a no-permissions baseline. The closest session-scoped option is the KILO_PERMISSION environment variable, which sets a deny-all rule."
  limitations: "There is no native --permission or --no-tools runtime flag. --agent can select a locked-down agent, but the agent must already be configured. Additional permissions must be added via KILO_PERMISSION or pre-configured agents."

agent_permissions:
  allowed: true
  fm_properties:
    - permission
    - tools

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--auto or --dangerously-skip-permissions on `kilo run`; interactive TUI sessions can also toggle auto-approve permissions from the command palette."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Kilo Code permissions are tool-centric (read, edit, bash, webfetch, skill, question, doom_loop, external_directory, etc.) and support wildcard/last-match-wins patterns, while PolicyEngine's canonical model is organized around filesystem, command, network, MCP, agent, and runtime axes."
    - "The default permissive posture (most tools allow by default) is the inverse of PolicyEngine's typical ask/deny defaults, requiring explicit modeling."
    - "external_directory is a path-scoped permission key rather than a true tool, and PolicyEngine would need to represent it as a workspace/external path rule with matching semantics."
    - "doom_loop is a runtime recovery guard, not a standard tool or resource permission."
    - "Agent-specific permissions and task subagent permissions are supported by Kilo but require PolicyEngine to scope rules by agent name."
    - "MCP tools are addressed by server-prefixed wildcard names (e.g., mymcp_*); PolicyEngine's MCP axis may not support arbitrary tool-name wildcard rules."
    - "Kilo-specific permission keys (agent_manager, notebook_read, notebook_edit, notebook_execute, repo_clone, repo_overview) extend the OpenCode grammar and have no direct PolicyEngine mapping."
    - "Kilo adds an OS-enforced sandbox (Seatbelt on macOS, bubblewrap on Linux) for Bash subprocesses with filesystem and network controls; PolicyEngine does not currently model sandbox boundaries."

permission_entities:
  - entity: tool
    native_names: [permission, tools]
    notes: "Built-in tools such as bash, read, edit, glob, grep, list, lsp, skill, todowrite, webfetch, websearch, question, repo_clone, repo_overview, agent_manager, and notebook_read/notebook_edit/notebook_execute are gated by permission keys."
  - entity: tool_group
    native_names: [edit]
    notes: "The edit permission covers edit, write, and apply_patch as a group."
  - entity: command
    native_names: [bash]
    notes: "bash permission rules match parsed command strings with glob semantics."
  - entity: path
    native_names: [read, edit, glob, grep, list, external_directory]
    notes: "read, edit, glob, grep, list, and external_directory rules can match file paths, glob patterns, or external directory prefixes."
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
    native_names: ["--auto", "--dangerously-skip-permissions", auto-approve]
    notes: "--auto and --dangerously-skip-permissions change the session baseline so non-denied requests are approved automatically."
  - entity: approval_category
    native_names: [allow, ask, deny]
    notes: "The three decision values for permission rules."
  - entity: sandbox
    native_names: [sandbox]
    notes: "Kilo supports an OS-enforced sandbox for Bash subprocesses with filesystem and network controls, separate from the permission rule engine."
  - entity: hook
    native_names: []
    notes: "Kilo supports custom tools and plugins but does not have a documented PreToolUse hook for permission decisions."
  - entity: extension
    native_names: [plugin]
    notes: "Plugins can add tools and hooks; external plugins can be disabled by not loading them."
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
  - name: dangerously-skip-permissions
    effect: "Non-denied permission requests are approved automatically for the main session only; Task child-session permissions are not tracked or auto-approved."
    interactive: false
    non_interactive: true
    aliases: ["--dangerously-skip-permissions"]
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
    - "A permission deny rule with pattern * for a tool removes it from the model's context entirely."
    - "Agent-specific permission objects can restrict the tool surface for that agent."
  notes: "Tool visibility and approval policy are both expressed through the same permission object. A denied tool is hidden from the model."

sandbox:
  supported: true
  modes: [auto-allow, regular-permissions]
  backends: ["macOS Seatbelt", "Linux/WSL2 bubblewrap"]
  filesystem_control: "Default write scope is the working directory plus a session temp directory. allowWrite, denyWrite, denyRead, and allowRead arrays configure boundaries."
  network_control: "Network modes include allow, deny, and proxy. allowedHosts can restrict outbound hosts. Proxy and allowedHosts support is experimental as of current source."
  notes: "Sandboxing applies only to Bash subprocesses. Native Windows is not supported (use WSL2). The sandbox is separate from the static permission engine; a tool may be permitted while the sandbox blocks its filesystem/network access."

trust_and_admin:
  folder_trust: "Kilo Code does not document a folder/project trust gate that disables project config, memory, or extensions."
  managed_policy: "Managed settings can be delivered via file-based kilo.jsonc in system directories (/Library/Application Support/kilo/, /etc/kilo/, or %ProgramData%\\kilo\\), via macOS MDM .mobileconfig under the managed domain, or through Kilo Gateway organization config. These occupy the highest precedence tier and cannot be overridden by user, project, or local config, nor by most environment variables or CLI flags."
  safe_mode: "Kilo does not have a dedicated safe-mode flag. Plugins can be avoided by not loading them, but built-in tools and permissions remain active."
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
  trust_model: "OAuth tokens for remote MCP servers are stored per user. Project-scoped MCP servers load if already configured; no interactive trust dialog is documented."
  notes: "MCP tools run outside any Kilo sandbox. stdio MCP servers are local subprocesses; remote MCP servers make network requests from outside Kilo's process."

headless_behavior: "In non-interactive `kilo run` mode, interactive permission prompts cannot be shown. Any tool call that would ask for approval is effectively blocked unless --auto or --dangerously-skip-permissions is used, or the permission is pre-approved. Use KILO_PERMISSION or an allow-all/deny-all config to avoid hangs."

approval_persistence: "Approvals granted with 'always' persist only for the rest of the current Kilo session. They are not saved across sessions or projects."

protected_paths:
  - "*.env"
  - "*.env.*"

security_posture: "Kilo Code's permission system is a client-side static policy engine with advisory prompts, layered with an optional OS-enforced sandbox (Seatbelt on macOS, bubblewrap on Linux/WSL2) for Bash subprocesses. Managed settings provide administrative policy, but they are still enforced by the client. Effective security requires combining strict permission rules, sandbox boundaries, and managed policy where available."

changes:
  - "Refreshed config paths against current source: Kilo uses XDG-basedir with app name 'kilo', so user config lives at ~/Library/Application Support/kilo/kilo.jsonc on macOS, ~/.config/kilo/kilo.jsonc on Linux, and %APPDATA%\\kilo\\kilo.jsonc on Windows."
  - "Documented Kilo-specific permission keys: agent_manager, repo_clone, repo_overview, notebook_read, notebook_edit, notebook_execute, plus list and skill."
  - "Updated precedence to include managed config files, macOS MDM preferences, and Kilo Gateway organization config."
  - "Added sandbox coverage: Seatbelt on macOS, bubblewrap on Linux/WSL2, filesystem/network controls, auto-allow vs regular-permissions modes, and experimental proxy/allowedHosts status."
  - "Documented --dangerously-skip-permissions as a separate non-interactive YOLO mode that does not auto-approve Task child sessions."
  - "Expanded frontmatter to the full schema contract including permission_entities, approval_modes, rule_model, tool_visibility, sandbox, trust_and_admin, mcp_permissions, headless_behavior, approval_persistence, protected_paths, and security_posture."
  - "Corrected rule precedence: last matching rule wins across merged rulesets, not deny-wins."
  - "Recorded that Kilo has no documented root/sudo restriction on YOLO mode."

requires_claudine_update: true
reason: "Kilo Code's permission grammar is inherited from OpenCode (tool-centric wildcard patterns, last-rule-wins evaluation, external_directory, doom_loop, agent/task permissions, permissive defaults, and Kilo-specific keys) and does not map cleanly to PolicyEngine's canonical axes. In addition, Kilo's OS-enforced sandbox for Bash subprocesses and its experimental network controls are not modeled by PolicyEngine. Supporting Kilo permissions accurately will require backend work in the PolicyEngine OpenCode/Kilo backend and mutation planning for kilo.jsonc permission objects."
---

# Kilo Code Permissions

## Introduction to Kilo Code Permissions

Kilo Code controls tool access with a single `permission` configuration object. Each permission key maps to one or more tools and resolves to one of three actions:

- `"allow"` — run without approval
- `"ask"` — prompt the user for approval
- `"deny"` — block the action

Kilo CLI is a fork of [OpenCode](https://opencode.ai), and the permission system is largely the same. Permissions can be configured through JSON/JSONC config files, inline environment variables, Markdown agent frontmatter, and a small set of CLI flags. Unlike some other agents, Kilo defaults to a permissive posture: most tools are allowed unless a rule says otherwise.

Kilo adds one significant layer that OpenCode does not provide: an OS-enforced sandbox for Bash subprocesses using Seatbelt on macOS and bubblewrap on Linux/WSL2. The sandbox is separate from the permission rule engine and can block filesystem or network access even when a tool call has been approved.

### Configuration files

The `permission` key lives in `kilo.json` or `kilo.jsonc` (legacy `opencode.json` / `opencode.jsonc` and `.opencode/` directories are still read). It can be a single action string that applies to all tools, or an object that maps tool names to action strings or granular pattern objects. See [Configuring the Default](#configuring-the-default) for file locations and examples.

### Environment variables

The main environment variables that influence permissions and sandbox behavior are:

| Variable | Effect |
| :----- | :----- |
| `KILO_PERMISSION` | Inline JSON permissions config merged into the effective config. |
| `KILO_CONFIG_CONTENT` | Inline JSON config content; can include a full `permission` object. |
| `KILO_CONFIG` | Path to a custom config file that may contain a `permission` object. |
| `KILO_DISABLE_PROJECT_CONFIG` | Skip loading project-level config files, so only global/env/CLI sources apply. |
| `KILO_SANDBOX` | Control sandbox network mode: `allow`, `deny`, or `proxy` (experimental). |
| `KILO_SANDBOX_ALLOWED_HOSTS` | Restrict sandboxed network access to listed hosts (experimental). |

### CLI parameters

Only a few CLI switches directly affect permissions:

| Flag | What it does |
| :----- | :----- |
| `--auto` | On `kilo run`, enable auto-approve mode for the main session and tracked Task child sessions. |
| `--dangerously-skip-permissions` | On `kilo run`, auto-approve non-denied permission requests for the main session only. |
| `--agent <name>` | Use the named agent, whose `permission` profile (if any) is applied. |
| `--permissions <list>` | Only for `kilo agent create`. Lists permissions to allow in the generated agent. |

### Precedence

Effective permissions are built from multiple layers. Highest-wins ordering is:

1. CLI flags such as `--auto` and `--dangerously-skip-permissions`
2. `KILO_PERMISSION`
3. macOS managed preferences / MDM
4. Managed config files in system directories
5. Active organization config from Kilo Gateway
6. `KILO_CONFIG_CONTENT`
7. Project `kilo.json` / `kilo.jsonc` and `.kilo/` / `.kilocode/` config directories (legacy `.opencode/` also read)
8. Custom config path from `KILO_CONFIG`
9. Global user config (`~/.config/kilo/kilo.jsonc` or OS equivalent)
10. Remote `.well-known/opencode` organizational defaults

Within any config object, rules are evaluated in order and the **last matching rule wins**.

### Permission policy vs tool visibility

Kilo does not have a separate visibility layer independent of approval policy. Both concerns are expressed through the same `permission` object (and the deprecated `tools` object):

- A tool with `"deny"` and pattern `"*"` is removed from the model's context entirely.
- A tool with `"allow"` is visible and runs without prompting.
- A tool with `"ask"` is visible but prompts before each call.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Kilo Code starts from permissive defaults:

- Most tools are `"allow"`.
- `doom_loop` and `external_directory` are `"ask"`.
- `read` is `"allow"`, but `.env` files are denied by default.
- Several auxiliary tools default to `"deny"` unless enabled: `suggest`, `question` (interactive), `interactive_terminal`, `plan_enter`, `plan_exit`, `repo_clone`, `repo_overview`.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for workspace paths; Deny for `.env` files.
- `can_write(path)` → Allow for workspace paths.
- `can_execute(command)` → Allow for bash commands (subject to sandbox constraints).
- `can_access_domain(domain)` → Allow for webfetch/websearch.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Allow.
- `can_spawn_subagent(agent)` → Allow.
- `can_loop_recovery()` → Ask (doom_loop).
- `can_access_external_directory(path)` → Ask.

This use case is not ergonomic in PolicyEngine without adjustments. PolicyEngine's canonical axes (filesystem, command, network, MCP, agent, runtime) do not line up one-to-one with Kilo's tool keys, and the permissive default is the opposite of PolicyEngine's usual ask/deny baseline. No changes are required to describe the broad idea, but full coverage of the default posture would need new mappings for `doom_loop`, `external_directory`, and the `.env` deny rule.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, set a global deny rule and then add specific allow or ask rules.

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

In an interactive session, `ask` causes Kilo to prompt. In a non-interactive `kilo run`, `ask` is effectively deny because there is no user to approve, so you should pre-declare `allow` rules for any tool the headless session needs.

Because Kilo does not have a dedicated `--permission` runtime flag, you usually whitelist through config or environment:

```bash
# Headless run with a locked-down allowlist via env
KILO_PERMISSION='{"*":"deny","read":"allow","grep":"allow","bash":{"git status *":"allow"}}' \
  kilo run "summarize the auth module"

# Use the built-in plan agent to default bash/edit to ask
kilo --agent plan

# Create and use a read-only subagent
kilo agent create --permissions read,grep --mode subagent --description "read-only explorer"
kilo --agent read-only-explorer
```

PolicyEngine can express this use case as `SetApprovalMode` to a deny-by-default posture plus explicit `GrantRead`, `AllowCommand`, and similar rules. It is not fully ergonomic because Kilo's tool-key wildcard patterns and last-match-wins ordering do not map directly to PolicyEngine's rule model. Without changes, PolicyEngine could describe the intent but not the exact pattern-matching behavior or agent-scoped deny defaults.

### YOLO

Kilo Code's YOLO mode is called **auto-approve**. A session can be put into this mode by:

- Starting `kilo run` with `--auto`, for example `kilo run --auto "..."`.
- Starting `kilo run` with `--dangerously-skip-permissions`.
- Setting `permission: "allow"` or `permission: { "*": "allow" }` in config.
- Using an agent whose permissions are all `allow`.
- Toggling **Enable auto-approve permissions** from the interactive TUI command palette (inherited from OpenCode).

Availability:

- **Interactive sessions**: yes, via the TUI command palette auto-approve toggle or by using an agent configured with all-allow permissions.
- **Non-interactive sessions**: yes, via `kilo run --auto` or `kilo run --dangerously-skip-permissions`.

When in auto-approve mode:

- **Allowed**: any tool call that is not explicitly denied is approved automatically, including bash, edit/write, webfetch, websearch, MCP tools, and subagent spawns.
- **Still enforced**: explicit `"deny"` rules in config are still enforced; if a tool is denied it will not run.
- **Not allowed**: auto-approve cannot override managed/MDM config that denies an action.

`--auto` and `--dangerously-skip-permissions` differ in one important way: `--auto` tracks and auto-approves permissions requested by spawned Task child sessions, while `--dangerously-skip-permissions` only auto-approves the main session.

### Root User

The public Kilo Code documentation does not describe any special permission behavior when the CLI is started as root or under `sudo`. The source code does not contain a root/sudo gate for auto-approve mode. Unlike Claude Code, there is no documented restriction that disables YOLO mode for root sessions. Therefore, YOLO mode remains available to root sessions unless an administrator blocks it through managed config.

### Configuring the Default

Default permissions are configured through JSON/JSONC config files at two main scopes:

- **User scope**: OS-specific XDG path (`~/Library/Application Support/kilo/kilo.jsonc` on macOS, `~/.config/kilo/kilo.jsonc` on Linux, `%APPDATA%\kilo\kilo.jsonc` on Windows). Legacy `opencode.json` / `opencode.jsonc` are also read. Applies across all projects.
- **Repo scope**: `kilo.json` or `kilo.jsonc` in the project root, or inside `.kilo/` / `.kilocode/` (legacy `.opencode/` is also read). Applies to everyone working in the repository and can be checked into version control.

Agent-specific defaults can also be defined in Markdown files under `~/.config/kilo/agents/` or `.kilo/agents/`.

Examples that illustrate the grammar:

```json
// ~/Library/Application Support/kilo/kilo.jsonc — user-wide defaults (macOS)
{
  "$schema": "https://app.kilo.ai/config.json",
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
// kilo.json — repo-shared defaults
{
  "$schema": "https://app.kilo.ai/config.json",
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
// Agent config in kilo.json
{
  "$schema": "https://app.kilo.ai/config.json",
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
<!-- ~/.config/kilo/agents/review.md -->
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

User `~/.config/kilo/kilo.jsonc`:

```json
{
  "permission": {
    "bash": {
      "rm *": "allow"
    }
  }
}
```

Repo `kilo.json`:

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

Repo `kilo.json`:

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
kilo run --auto "apply the suggested refactor"
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

Repo `kilo.json`:

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

Kilo Code provides the following built-in tools. Each tool is gated by a permission key. Some keys cover multiple tools.

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
| `websearch` | `websearch` | Allow |
| `question` | `question` | Allow |
| `task` (subagent spawn) | `task` | Allow |
| `repo_clone` | `repo_clone` | Deny |
| `repo_overview` | `repo_overview` | Deny |
| `agent_manager` | `agent_manager` | Deny |
| `notebook_read` | `notebook_read` | Allow |
| `notebook_edit` | `notebook_edit` | Allow |
| `notebook_execute` | `notebook_execute` | Allow |

Additional tools such as `suggest`, `interactive_terminal`, `plan_enter`, and `plan_exit` are gated and default to `deny` because they are interactive or UI-oriented.

Permission rules match the tool input. For example, `bash` rules match parsed command strings, `read`/`edit` rules match file paths, `glob` rules match glob patterns, `grep` rules match regex patterns, and `webfetch` rules match URLs. Wildcards follow simple glob semantics: `*` matches zero or more characters, `?` matches exactly one character, and all other characters match literally. `~` and `$HOME` at the start of a pattern expand to the user's home directory.

Rules are evaluated in order across merged rulesets and the **last matching rule wins**, so a common pattern is to place `"*": "ask"` first and more specific allow/deny rules after it.

### Native permission entities

Kilo's permission system is tool-centric. The native entities it can target are:

- **Tools** — each built-in tool has a permission key.
- **Tool groups** — `edit` covers `edit`, `write`, and `apply_patch`.
- **Commands** — `bash` permission rules match parsed command strings.
- **Paths** — `read`, `edit`, `glob`, `grep`, `list`, and `external_directory` match file paths or patterns.
- **Workspace/external directories** — `external_directory` gates paths outside the working directory.
- **MCP servers** — enabled/disabled via the `mcp` config object.
- **MCP tools** — targeted by server-prefixed wildcard names such as `mymcp_*` or `mymcp_search`.
- **Agents/subagents** — agents define their own `permission` object; `task` controls subagent spawning.
- **Mode** — `--auto` and `--dangerously-skip-permissions` toggle auto-approve for the session.
- **Approval category** — `allow`, `ask`, `deny`.
- **Sandbox** — separate filesystem/network isolation for Bash subprocesses.
- **Extensions/plugins** — plugin-provided tools can be avoided by not loading the plugin.

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

Kilo does not use named coarse permission modes like Claude Code. Instead it has:

- **Default** — permissive defaults as described above.
- **Auto-approve** (`--auto`) — non-denied requests are approved automatically, including Task child sessions.
- **Dangerously-skip-permissions** (`--dangerously-skip-permissions`) — non-denied requests are approved automatically for the main session only.
- **Plan agent** (`--agent plan`) — a built-in primary agent that defaults `edit` and `bash` to `ask`.

There is no `dontAsk`, `bypassPermissions`, or classifier-based `auto` mode.

### Persistence

When Kilo prompts for approval, the UI offers three outcomes:

- `once` — approve just this request.
- `always` — approve future requests matching the suggested patterns for the rest of the current Kilo session.
- `reject` — deny the request.

`always` approvals are session-only and are lost when Kilo exits.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Kilo Code provides an OS-enforced sandbox for Bash subprocesses that is separate from the permission rule engine.

- **Backends**: macOS uses Seatbelt; Linux and WSL2 use bubblewrap.
- **Modes**:
  - **Auto-allow**: sandboxed Bash commands run without prompting; the sandbox boundary substitutes for the bare `bash` ask rule.
  - **Regular permissions**: sandboxed commands still go through the regular permission flow.
- **Filesystem**: by default, sandboxed commands can write only to the working directory and a session temp directory. Use `sandbox.filesystem.allowWrite`, `denyWrite`, `denyRead`, and `allowRead` to customize paths.
- **Network**: modes include `allow`, `deny`, and `proxy`. Use `allowedHosts` to restrict outbound hosts. As of current source, `proxy` and `allowedHosts` support is experimental.
- **Scope**: sandboxing applies only to Bash subprocesses. Built-in file tools, MCP tools, and other operations run outside this boundary.
- **Platform differences**: Native Windows is not supported; use WSL2. If sandbox dependencies are missing, Kilo warns and may fall back to unsandboxed execution depending on configuration.

Permissions and sandboxing are complementary:

- Permission rules block Kilo from attempting restricted actions.
- Sandbox restrictions prevent Bash commands from reaching resources outside defined boundaries, even if a prompt injection bypasses Kilo's decision-making.

### Trust and administrative controls

**Folder/project trust**: Kilo Code does not document a folder trust gate that disables project config, memory, extensions, or MCP servers.

**Managed/admin policy**: managed settings can be delivered in several ways:

- **File-based**: drop a `kilo.jsonc` in `/Library/Application Support/kilo/` on macOS, `/etc/kilo/` on Linux, or `%ProgramData%\kilo\` on Windows. These directories require admin access to write.
- **macOS MDM**: deploy a `.mobileconfig` with the managed domain.
- **Kilo Gateway**: active organization config from Kilo Gateway can enforce permissions, allowed providers, and other rules.

Managed settings occupy the highest precedence tier and cannot be overridden by user, project, or local config, nor by most environment variables or CLI flags. The `permission` object in a managed config is enforced like any other managed key.

**Safe/minimal mode**: Kilo does not have a dedicated safe-mode flag. Plugins can be avoided by not loading them, but built-in tools and permissions remain active.

### Protected paths

The only provider-reserved path protection documented is the default `.env` deny rule:

- `*.env` — denied
- `*.env.*` — denied
- `*.env.example` — explicitly allowed

There is no extensive list of protected dotfiles or provider config paths like Claude Code maintains.

### Security posture

Kilo Code's permission system is a **client-side static policy engine with advisory prompts**, layered with an **optional OS-enforced sandbox** for Bash subprocesses. Managed settings provide administrative policy, but they are still enforced by the client. Effective security requires combining strict permission rules, sandbox boundaries, and managed policy where available.

## MCP and Permissions

MCP servers add external tools that appear alongside built-in tools. Once a server is configured under the `mcp` object, its tools are registered with the server name as a prefix (for example, a server named `mymcp` exposes tools like `mymcp_search`).

Permissions interact with MCP tools in three ways:

1. **Server enablement**: A server can be enabled or disabled with `enabled: true`/`false`. A disabled server is not available.
2. **Tool-level rules**: The global `permission` object can target MCP tools by name or wildcard:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
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

MCP tools run **outside** the Kilo sandbox. Remote MCP servers make network requests from outside the Kilo process, and stdio MCP servers run as local subprocesses with the user's environment.

## Non-Interactive Behavior

In non-interactive `kilo run` mode, interactive permission prompts cannot be shown. Any tool call that would `ask` for approval is effectively blocked because there is no user to approve it. To avoid hangs:

- Pass `--auto` to approve non-denied requests automatically (main session and tracked Task child sessions).
- Pass `--dangerously-skip-permissions` to approve non-denied requests automatically for the main session only.
- Set `KILO_PERMISSION` with explicit `allow` rules for every tool the headless session needs.
- Pre-configure a locked-down agent and select it with `--agent`.

`ask` rules do not automatically become `allow` in headless mode; they block the call.

## Sources

- [Kilo Code docs - Permissions](https://docs.kilo.ai/permissions)
- [Kilo Code docs - Config](https://docs.kilo.ai/config)
- [Kilo Code docs - Tools](https://docs.kilo.ai/tools)
- [Kilo Code docs - Agents](https://docs.kilo.ai/agents)
- [Kilo Code docs - MCP servers](https://docs.kilo.ai/mcp-servers)
- [Kilo Code docs - CLI](https://docs.kilo.ai/cli)
- [Kilo Code config schema](https://app.kilo.ai/config.json)
- [Kilo Code GitHub repository](https://github.com/Kilo-Org/kilocode)

## Changelog

- 2026-07-02: Refreshed research against Kilo Code v7.3.45, current documentation, and source on GitHub. Updated config paths to XDG-basedir equivalents for macOS/Linux/Windows. Documented Kilo-specific permission keys (agent_manager, repo_clone, repo_overview, notebook_*) and the additional list/skill keys. Added coverage of the OS-enforced sandbox (Seatbelt/bubblewrap), network modes, and experimental proxy/allowedHosts status. Distinguished `--auto` from `--dangerously-skip-permissions` (Task child-session tracking). Expanded frontmatter to the full schema contract and flagged Claudine updates as required.
