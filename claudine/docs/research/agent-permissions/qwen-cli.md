---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: approval-mode
    style: switch
    description: "Set the session approval mode. CLI values are plan (read-only analysis), default (ask before edits/shell), auto-edit (auto-approve file edits), and yolo (auto-approve all tool calls). The auto mode can be configured via settings.json or /approval-mode."
    example: "qwen -p \"refactor auth\" --approval-mode auto-edit"
    example_description: "Starts a headless session that auto-approves file edits but still prompts for shell commands."
  - param: yolo
    style: switch
    description: "Enable YOLO mode for the session, auto-approving all tool calls. Equivalent to --approval-mode yolo."
    example: "qwen -p \"run tests and commit\" --yolo"
    example_description: "Runs a headless prompt with every tool call approved automatically."
  - param: allowed-tools
    style: switch
    description: "Comma-separated or repeated list of tool rules that bypass the confirmation dialog for this session. Accepts tool names and Tool(specifier) patterns."
    example: "qwen -p \"...\" --allowed-tools \"Shell(npm test),Read\""
    example_description: "Auto-approves npm test shell commands and all read operations for this session."
  - param: exclude-tools
    style: switch
    description: "Comma-separated or repeated list of tool names to remove from the session's available tool surface."
    example: "qwen -p \"...\" --exclude-tools \"Shell,Write,Edit\""
    example_description: "Prevents the model from using shell, write, and edit tools in this session."
  - param: core-tools
    style: switch
    description: "Comma-separated or repeated list of core tool names/paths to include in the session."
    example: "qwen -p \"...\" --core-tools Read,Edit"
    example_description: "Restricts the session to the listed core tools."
  - param: disabled-slash-commands
    style: switch
    description: "Comma-separated or repeated list of slash command names to disable for the session. Unioned with slashCommands.disabled and QWEN_DISABLED_SLASH_COMMANDS."
    example: "qwen --disabled-slash-commands \"auth,mcp,extensions\""
    example_description: "Disables the /auth, /mcp, and /extensions slash commands for this session."
  - param: sandbox
    style: switch
    description: "Enable sandbox mode for the session, running shell and file-modifying tools inside a sandbox (sandbox-exec on macOS or Docker/Podman)."
    example: "qwen -s -p \"analyze the code\""
    example_description: "Runs the session with filesystem/process isolation enabled."
  - param: sandbox-image
    style: switch
    description: "Set the Docker/Podman image used when sandboxing."
    example: "qwen -s --sandbox-image ghcr.io/qwenlm/qwen-code:0.19.4 -p \"...\""
    example_description: "Uses a specific sandbox image for the session."
  - param: include-directories
    style: switch
    description: "Add additional directories to the workspace scope for the session."
    example: "qwen -p \"...\" --include-directories ../shared,../docs"
    example_description: "Expands the working directory set for this session."
  - param: allowed-mcp-server-names
    style: switch
    description: "Comma-separated or repeated list of MCP server names that are allowed to load for the session."
    example: "qwen --allowed-mcp-server-names \"puppeteer,github\" -p \"...\""
    example_description: "Only loads the named MCP servers."
  - param: mcp-config
    style: switch
    description: "Load MCP servers from a JSON file or inline JSON string for the session."
    example: "qwen --mcp-config ./mcp.json -p \"...\""
    example_description: "Loads MCP servers defined in a project-local configuration file."
  - param: bare
    style: switch
    description: "Minimal mode that skips implicit startup auto-discovery and only honors explicitly provided CLI inputs."
    example: "qwen --bare -p \"Summarize this file\" --allowed-tools Read"
    example_description: "Runs a headless summary task with no project configuration loaded."

env_vars:
  - name: QWEN_SANDBOX
    effect: "Enable or configure sandbox mode (true/false/docker/podman/sandbox-exec). Takes precedence over the --sandbox flag and tools.sandbox setting."
  - name: QWEN_SANDBOX_IMAGE
    effect: "Override the sandbox Docker/Podman image. Takes precedence over --sandbox-image and tools.sandboxImage."
  - name: SEATBELT_PROFILE
    effect: "macOS-only. Select the sandbox-exec profile (permissive-open, permissive-closed, restrictive-open, etc.)."
  - name: QWEN_CODE_SAFE_MODE
    effect: "Equivalent to safe mode. Disables customizations including context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, settings-sourced approval mode overrides, memory, and sandbox settings; explicit --yolo/--approval-mode still apply."
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: "Comma-separated list of slash commands to disable. Unioned with slashCommands.disabled and --disabled-slash-commands."
  - name: QWEN_HOME
    effect: "Changes the global configuration directory (default ~/.qwen), affecting where user-scoped settings, skills, agents, and memory are loaded."

config_files:
  - os: all
    user: ~/.qwen/settings.json
    repo: .qwen/settings.json

precedence:
  - source: "CLI flags > environment variables > system settings file > project settings > user settings > system defaults file > hardcoded defaults"
    scope: [permissions]
    merge_strategy: none
    notes: "CLI flags are temporary session overrides and win over environment variables and config files. Environment variables override all settings-file layers except CLI flags. System override settings (/etc/qwen-code/settings.json) win over project and user settings. Project settings override user settings. For permission rules specifically, deny > ask > allow, and a deny rule from any scope overrides allow rules from any scope."

default_posture: "With no configuration, Qwen Code starts in default approval mode (Ask Permissions): read-only built-in tools run without confirmation, while file edits, shell commands, web fetches, MCP tool calls, and other state-changing actions prompt for approval."

cli_zero_permissions:
  supported: false
  invocation: ""
  mechanism: "Qwen Code has no single CLI flag that denies all tool calls. The closest approximation is to combine --approval-mode plan with --exclude-tools for the tools to remove, or to use broad deny rules in settings.json."
  limitations: "There is no universal deny-all or empty-tool-list flag. --exclude-tools removes named tools but does not block remaining tools from prompting or being approved; --approval-mode plan stops edits/shell but still allows read-only tools."

agent_permissions:
  allowed: true
  fm_properties:
    - approvalMode
    - tools
    - disallowedTools
    - permissionMode

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--yolo or --approval-mode yolo CLI flags; /approval-mode yolo in an interactive session; or tools.approvalMode: yolo in settings.json"

policy_engine:
  ergonomic: false
  provides_coverage: true
  gaps:
    - "Auto mode relies on an LLM classifier with natural-language hints; the classifier decision is not a deterministic static rule."
    - "Meta-category rules such as Read cover read_file, grep_search, glob, and list_directory, requiring backend-specific expansion."
    - "Path-pattern prefixes (//, ~/, /, ./) and shell-command word-boundary matching differ from generic glob semantics."
    - "Protected self-modification and persistence paths are hard-coded exceptions in auto mode and cannot be expressed as static rules."
    - "Subagent permission inheritance and parent-mode override (e.g., a yolo parent forces yolo on subagents) are runtime behaviors outside static policy."
    - "MCP server-level allow/deny (mcp.allowed/mcp.excluded), per-server includeTools/excludeTools, and the trust flag are additional policy surfaces."
    - "Folder trust and safe mode are trust/scope gates rather than permission rules."

permission_entities:
  - entity: tool
    native_names: ["permissions.allow", "permissions.ask", "permissions.deny", "--allowed-tools", "--exclude-tools"]
    notes: "Built-in tools such as Bash/Shell, Read, Edit, Write, WebFetch, Agent, Skill, etc. Bare tool names match all uses; specifiers narrow matching. --exclude-tools hides tools from the model context."
  - entity: tool_group
    native_names: ["Read, Grep, Glob, ListFiles", "Edit, Write, NotebookEdit"]
    notes: "Read rules apply to read_file, grep_search, glob, and list_directory. Edit rules apply to edit, write_file, and notebook_edit."
  - entity: command
    native_names: ["Bash", "Shell"]
    notes: "Shell/Bash rules match command strings with glob semantics and word-boundary matching."
  - entity: path
    native_names: ["Read(...)", "Edit(...)", "Write(...)"]
    notes: "Path rules follow gitignore-style patterns with //, ~/, /, and relative anchors."
  - entity: workspace
    native_names: ["includeDirectories", "--include-directories", "--add-dir"]
    notes: "Additional directories extend where Qwen can read and edit, but do not load most .qwen configuration."
  - entity: mcp_server
    native_names: ["mcpServers", "--mcp-config", "--allowed-mcp-server-names", "mcp.allowed", "mcp.excluded"]
    notes: "MCP servers can be scoped user/project and filtered by name or trust."
  - entity: mcp_tool
    native_names: ["mcp__<server>", "mcp__<server>__<tool>", "includeTools", "excludeTools"]
    notes: "MCP tools are governed by the same permission rule syntax as built-in tools; per-server include/exclude lists further narrow the surface."
  - entity: agent
    native_names: ["Agent", "Agent(<name>)"]
    notes: "The Agent tool spawns subagents; rules can allow/deny specific agent types or the tool itself."
  - entity: subagent
    native_names: ["tools", "disallowedTools", "permissionMode"]
    notes: "Subagent frontmatter can restrict tools and set a permission mode."
  - entity: mode
    native_names: ["tools.approvalMode", "--approval-mode", "permissionMode"]
    notes: "Approval modes set the session baseline for approvals: plan, default, auto-edit, auto, yolo."
  - entity: approval_category
    native_names: ["permissions.allow", "permissions.ask", "permissions.deny"]
    notes: "Fine-grained rule decisions. Deny > ask > allow; deny from any scope beats allow from any scope."
  - entity: sandbox
    native_names: ["tools.sandbox", "tools.sandboxImage", "--sandbox", "--sandbox-image", "QWEN_SANDBOX"]
    notes: "OS-level isolation for shell/file-modifying tools; separate from the permission rule engine."
  - entity: hook
    native_names: ["hooks"]
    notes: "Hooks can extend or mediate behavior; safe mode disables them."
  - entity: extension
    native_names: ["extensions", "--extensions"]
    notes: "Extensions can add tools and commands; safe mode disables custom extensions."
  - entity: slash_command
    native_names: ["slashCommands.disabled", "--disabled-slash-commands", "QWEN_DISABLED_SLASH_COMMANDS"]
    notes: "Slash commands can be disabled globally or per session."

approval_modes:
  - name: plan
    effect: "Read-only exploration only; file edits and shell commands are not executed. Presents a plan for approval before exiting."
    interactive: true
    non_interactive: true
    aliases: ["plan", "Plan mode"]
  - name: default
    effect: "Read-only tools run without approval; state-changing tools prompt for approval."
    interactive: true
    non_interactive: true
    aliases: ["default", "Ask Permissions"]
  - name: auto-edit
    effect: "Auto-approves file edits; shell commands and other state-changing actions still prompt."
    interactive: true
    non_interactive: true
    aliases: ["auto-edit", "Edit automatically"]
  - name: auto
    effect: "Routes tool calls through an LLM classifier that auto-approves routine actions and blocks risky ones. Explicit ask rules still prompt; deny rules still block."
    interactive: true
    non_interactive: true
    aliases: ["auto", "Auto mode"]
  - name: yolo
    effect: "Skips permission prompts and safety checks so tool calls execute immediately. Explicit ask rules and deny rules still apply."
    interactive: true
    non_interactive: true
    aliases: ["yolo", "--yolo", "YOLO mode"]

rule_model:
  decisions: ["allow", "ask", "deny"]
  syntax: "Tool or Tool(specifier). Specifiers include Bash(pattern), Shell(pattern), Read/Edit/Write(path-pattern), WebFetch(domain:host), Agent(name), Skill(name), mcp__server__tool. Natural-language hints also drive the auto-mode classifier."
  precedence: "Deny rules are evaluated first, then ask rules, then allow rules. A matching deny rule always wins over a matching ask or allow rule, even if the allow rule is more specific. Deny rules from any settings scope override allow rules from any scope. The active permission mode applies after rules."
  merge_semantics: "Permission allow/ask/deny arrays merge across user, project, and system settings scopes. Other settings generally replace by precedence."
  matcher_semantics: "Bash/Shell rules use glob patterns with *; a space before * enforces a word boundary. Read/Edit/Write rules follow gitignore patterns with // (absolute), ~/ (home), / (project root), and relative anchors. WebFetch uses domain: prefixes with * wildcards. MCP rules use mcp__server__tool naming."
  default_decision: "In default mode, read-only tools are allowed and everything else asks. In plan mode, edits/shell are blocked. In yolo mode, the default is allow (subject to deny/ask rules)."

tool_visibility:
  supported: true
  mechanisms:
    - "--exclude-tools removes named tools from the session surface."
    - "--core-tools restricts the session to the listed core tools."
    - "Subagent frontmatter tools/disallowedTools restricts subagent tool surface."
    - "permissions.deny with a bare tool name removes that tool from the model's context."
    - "--allowed-mcp-server-names limits which configured MCP servers load."
  notes: "--exclude-tools and --core-tools affect the available tool surface; a denied tool is hidden from the model entirely, while an allowed tool may still prompt depending on the mode and rules."

sandbox:
  supported: true
  modes: ["regular-permissions"]
  backends: ["macOS Seatbelt (sandbox-exec)", "Linux/WSL Docker/Podman", "Windows Docker/Podman"]
  filesystem_control: "Sandboxed tools run with restricted filesystem access. The working directory and included directories are available; additional paths are denied unless the sandbox image/profile permits them."
  network_control: "Network access depends on the sandbox backend and image configuration. No domains are pre-allowed by default when using container sandboxing."
  notes: "Sandboxing applies to shell and file-modifying tools. Built-in read tools, MCP tools, and subagent tool calls run outside this boundary unless the provider also isolates them. If sandbox dependencies are missing, Qwen Code warns and may fall back to unsandboxed execution."

trust_and_admin:
  folder_trust: "First-time launches in a project directory prompt a workspace trust dialog. Untrusted folders ignore project .qwen settings, context files, hooks, extensions, skills, MCP servers, and custom subagents. Trust is saved per directory. Trust verification is skipped in non-interactive -p mode."
  managed_policy: "System-level settings files (/etc/qwen-code/settings.json and system-defaults.json) provide the managed/admin layer. They occupy a higher precedence tier than user/project config and cannot be overridden by lower scopes. Specific managed-only keys may lock approval modes or sandbox policy."
  safe_mode: "QWEN_CODE_SAFE_MODE=1 (or safe mode) disables context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, settings-sourced approval mode overrides, memory, and sandbox settings. Built-in tools and explicit CLI permission flags continue to work."
  notes: "Folder trust and safe mode are trust/scope gates rather than permission rules. --bare skips implicit startup auto-discovery and honors only explicit CLI inputs."

mcp_permissions:
  supported: true
  server_filters:
    - "--allowed-mcp-server-names restricts which configured servers load for the session."
    - "mcp.allowed / mcp.excluded in settings.json filter servers by name."
    - "Per-server trust: true bypasses confirmation prompts for that server's tools."
    - "--mcp-config loads a session-specific MCP server set."
    - "Safe mode disables custom MCP servers."
  tool_filters:
    - "Permission rules mcp__<server> and mcp__<server>__<tool> allow/ask/deny specific tools."
    - "Per-server includeTools/excludeTools restrict the tool surface."
    - "--allowed-tools accepts the same MCP patterns."
  trust_model: "MCP servers can be configured at user or project scope. A trusted server bypasses confirmation prompts for its tools. Untrusted project-scoped servers require user approval via a trust dialog; untrusted repos ignore project MCP approvals. In non-interactive mode, project MCP servers load only if already approved or allowed by user/managed settings."
  notes: "MCP tools run outside the Qwen sandbox. stdio servers are local subprocesses and can access the user's environment unless constrained by safe mode or environment scrubbing."

headless_behavior: "In non-interactive -p mode, interactive permission prompts cannot be shown. Use --allowed-tools, --approval-mode plan, or --approval-mode yolo to avoid hangs. Auto mode's classifier blocks risky actions; repeated blocks may abort the session. MCP servers requiring user approval are unavailable unless already trusted or allowed. Workspace trust dialogs are skipped in -p mode, so project MCP servers and project settings load only when already approved or allowed by user/managed settings."

approval_persistence: "Session-level approvals granted via --allowed-tools or YOLO mode do not persist beyond the session. Settings.json rules persist until the file is changed. 'Yes, don't ask again' style approvals, if offered, are typically scoped to the project directory and command pattern; specifics are provider-version dependent."

protected_paths:
  - ".qwen"
  - ".qwen/settings.json"
  - ".git"
  - ".gitconfig, .gitmodules"
  - ".bashrc, .bash_profile, .bash_login, .bash_aliases, .bash_logout, .zshrc, .zprofile, .zshenv, .zlogin, .zlogout, .profile, .envrc"
  - "shell and SSH configuration files"
  - "Qwen Code internal state directories"

security_posture: "Qwen Code's default security is a client-side static policy engine with advisory prompts, not an OS-enforced sandbox. An optional OS-enforced sandbox (Seatbelt on macOS, Docker/Podman containers on Linux/Windows) can restrict shell and file-modifying tool filesystem and network access. Auto mode adds a model-based classifier. System settings provide administrative policy but are still client-side controls. Defense-in-depth requires combining permission rules, sandboxing, and managed policy."

changes:
  - "Refreshed research against Qwen Code 0.15.6 and current documentation (2026-07-02)."
  - "Corrected CLI flags: removed --include-tools and --safe-mode; added --core-tools, --allowed-mcp-server-names, and --bare."
  - "Limited --approval-mode CLI choices to plan/default/auto-edit/yolo and noted auto mode is available via settings.json and /approval-mode."
  - "Added all schema-required frontmatter fields: cli_zero_permissions, permission_entities, approval_modes, rule_model, tool_visibility, sandbox, trust_and_admin, mcp_permissions, headless_behavior, approval_persistence, protected_paths, security_posture."
  - "Updated policy_engine assessment to ergonomic: false because of Qwen-specific meta-categories, classifier-based auto mode, and additional trust/MCP/sandbox surfaces."

requires_claudine_update: true
reason: "Qwen Code's permission model combines approval modes, allow/ask/deny rules with meta-categories and path-pattern prefixes, subagent-scoped overrides, MCP include/exclude/trust, sandbox gates, folder trust, safe mode, and an LLM-based auto-mode classifier. Fully representing these in Claudine's PolicyEngine will require backend updates to the Qwen backend and mutation planning for settings.json permission objects."
---

# Qwen CLI Permissions

## Introduction to Qwen CLI Permissions

Qwen CLI controls tool access through a combination of **approval modes**, **permission rules**, **sandboxing**, and **tool allowlists/blocklists**. The goal is to let the agent act autonomously when the risk is acceptable while keeping destructive or sensitive operations under user control.

Permissions can be defined in three ways:

1. **Configuration files** — `settings.json` at user, project, system-defaults, and system-override scopes.
2. **CLI flags** — `--approval-mode`, `--yolo`, `--allowed-tools`, `--exclude-tools`, `--sandbox`, etc.
3. **In-session controls** — `/approval-mode`, `/permissions`, `/plan`, and the mode switcher.

### Approval modes

Qwen Code supports five approval modes. The mode acts as the baseline; `permissions.allow`, `permissions.ask`, and `permissions.deny` rules refine it.

| Mode | File edits | Shell commands | Best for |
| :----- | :----- | :----- | :----- |
| `plan` | Not executed | Not executed | Safe exploration and planning |
| `default` | Ask | Ask | Daily interactive work |
| `auto-edit` | Auto-approve | Ask | Refactoring and code changes |
| `auto` | Classifier-evaluated | Classifier-evaluated | Long autonomous sessions with a safety net |
| `yolo` | Auto-approve | Auto-approve | Trusted automation and CI/CD |

The `--approval-mode` CLI flag accepts `plan`, `default`, `auto-edit`, and `yolo`. The `auto` mode is available via `settings.json` (`tools.approvalMode: auto`) and the `/approval-mode auto` slash command.

### Permission rule syntax

Permission rules live under the `permissions` object in `settings.json` as `allow`, `ask`, and `deny` arrays. Rules use the form `ToolName` or `ToolName(specifier)`. Decision priority is `deny > ask > allow > default`.

| Rule | Effect |
| :----- | :----- |
| `"Bash"` | Matches all shell commands |
| `"Bash(npm run *)"` | Matches commands starting with `npm run ` |
| `"Read(./secrets/**)"` | Matches reads under `./secrets/` (covers read_file, grep, glob, list) |
| `"ReadFile(./.env)"` | Matches only `read_file` of `./.env` |
| `"Edit(/src/**/*.ts)"` | Matches edits under project-root `/src/` (covers edit, write_file, notebook_edit) |
| `"WebFetch(api.example.com)"` | Matches fetches from `api.example.com` and subdomains |
| `"mcp__puppeteer"` | Matches every tool from the `puppeteer` MCP server |
| `"Agent"` | Matches subagent spawns |

Path-pattern prefixes:

| Prefix | Meaning | Example |
| :----- | :----- | :----- |
| `//` | Absolute from filesystem root | `//etc/passwd` |
| `~/` | Relative to home directory | `~/Documents/*.pdf` |
| `/` | Relative to project root | `/src/**/*.ts` |
| `./` | Relative to current working directory | `./secrets/**` |
| (none) | Same as `./` | `secrets/**` |

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--approval-mode <mode>` sets the session approval mode.
- `--yolo` is a shortcut for `--approval-mode yolo`.
- `--allowed-tools <rules>` adds temporary allow rules.
- `--exclude-tools <tools>` removes tools from the session surface.
- `--core-tools <tools>` restricts the session to the listed core tools.
- `--disabled-slash-commands <commands>` disables slash commands.
- `--sandbox` / `--sandbox-image` enable filesystem/process isolation.
- `--include-directories` expands the workspace scope.
- `--allowed-mcp-server-names` limits which MCP servers load.
- `--mcp-config` loads MCP servers from a file or JSON string.
- `--bare` skips implicit startup auto-discovery and honors only explicit CLI inputs.

Precedence is documented in the frontmatter. The key points are:

- CLI flags are temporary session overrides and win over environment variables and config files.
- Environment variables override all settings-file layers except CLI flags.
- System override settings (`/etc/qwen-code/settings.json`) win over project and user settings.
- Project settings override user settings.
- For permission rules specifically, `deny` rules from any scope override `allow` and `ask` rules.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Qwen Code starts in `default` mode (Ask Permissions): read-only tools such as `read_file`, `grep_search`, `glob`, and `list_directory` run without approval, while file edits, shell commands, web fetches, MCP tool calls, and other state-changing tools prompt for approval.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for workspace paths and additional included directories.
- `can_write(path)` → Ask for paths in the workspace; Deny for paths outside it until approved.
- `can_execute(command)` → Ask for shell commands.
- `can_access_domain(domain)` → Ask for web fetches.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask until approved or denied.
- `can_spawn_subagent(agent)` → Allow to spawn, but the subagent's own tool calls are checked independently.

This use case is ergonomic in PolicyEngine because the engine already models read, write, execute, network, MCP, and agent axes. The main limitation is that the interactive approval prompt itself is a runtime UI concern, not a static policy fact.

### Whitelisting

Qwen Code does not provide a single "deny everything" wildcard. To start with minimal permissions and require every needed action to be asked for or explicitly declared, set broad `deny` rules for the categories you want blocked, leave needed categories unset (so `default` mode asks), and add explicit `allow` rules for automation.

In `settings.json`:

```json
{
  "tools": {
    "approvalMode": "default"
  },
  "permissions": {
    "deny": ["Bash", "Edit", "Write", "NotebookEdit", "WebFetch", "Agent", "Skill"],
    "ask": ["Edit", "Bash"],
    "allow": ["Read", "Grep", "Glob"]
  }
}
```

With this configuration, read-only tools are allowed, edits and shell commands ask for approval, and the explicitly denied categories cannot be used even if the model requests them.

CLI examples:

```bash
# Headless run that can only read and search
qwen -p "explain the auth module" --exclude-tools "Shell,Write,Edit" --allowed-tools "Read,Grep,Glob"

# Allow only npm test and reads for a CI step
qwen -p "run the test suite" --allowed-tools "Shell(npm test),Read"

# Block risky tools for an exploration session
qwen -p "..." --exclude-tools "Shell,Write,Edit"
```

PolicyEngine can describe this use case by setting broad `Deny` rules on command, write, network, MCP, and agent axes, then adding explicit `GrantRead`, `AllowCommand`, and `Ask` rules. It is mostly ergonomic, but the lack of a universal deny-all wildcard means the engine must enumerate Qwen's meta-categories rather than using a single rule.

### YOLO

YOLO mode in Qwen Code is called `yolo` and is the `tools.approvalMode` value `yolo`. A session can be put into YOLO mode by:

- Starting with `--yolo`.
- Starting with `--approval-mode yolo`.
- Using `/approval-mode yolo` inside an interactive session.
- Setting `tools.approvalMode` to `yolo` in a settings file.

Availability:

- **Interactive sessions**: yes, via `/approval-mode yolo` or the mode switcher.
- **Non-interactive sessions**: yes, `qwen -p "..." --yolo` works.
- **Root/sudo**: the public documentation does not describe any root-specific restriction, so YOLO remains available to root sessions unless blocked by managed configuration.

When in YOLO mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits, shell commands, web fetches, MCP tool calls, and subagent spawns.
- **Still enforced**: explicit `permissions.deny` rules still block actions; explicit `permissions.ask` rules still force a prompt; managed settings that disable YOLO still prevent it.
- **Not allowed**: it cannot bypass folder-trust safe mode or managed system settings.

### Root User

The Qwen Code documentation does not describe any special permission behavior when the CLI is started as root or under `sudo`. Unlike Claude Code, there is no documented restriction that disables YOLO/bypass mode for root sessions. Therefore, all approval modes, including YOLO, remain available to root sessions unless an administrator blocks them through system-level configuration.

### Configuring the Default

Default permissions are configured through `settings.json` files at multiple scopes:

- **User scope**: `~/.qwen/settings.json` applies across all projects.
- **Repo/project scope**: `.qwen/settings.json` applies to everyone working in the repository.
- **System defaults**: `/etc/qwen-code/system-defaults.json` (Linux), `C:\ProgramData\qwen-code\system-defaults.json` (Windows), `/Library/Application Support/QwenCode/system-defaults.json` (macOS).
- **System override**: `/etc/qwen-code/settings.json`, `C:\ProgramData\qwen-code\settings.json`, or `/Library/Application Support/QwenCode/settings.json`.

For the schema's `config_files` field, user scope is `~/.qwen/settings.json` and repo scope is `.qwen/settings.json`.

Examples that illustrate the grammar:

```json
// ~/.qwen/settings.json — user-wide defaults
{
  "tools": {
    "approvalMode": "auto-edit"
  },
  "permissions": {
    "allow": ["Bash(npm run *)", "Bash(git status *)", "WebFetch(domain:docs.rs)"],
    "deny": ["Bash(curl *)", "Bash(wget *)", "Read(~/\.ssh/**)"]
  }
}
```

```json
// .qwen/settings.json — repo-shared defaults
{
  "tools": {
    "approvalMode": "default"
  },
  "permissions": {
    "allow": ["Bash(npm run lint)", "Bash(npm run test *)"],
    "deny": ["Read(./.env)", "Read(./secrets/**)"]
  }
}
```

```json
// .qwen/settings.json — auto mode with classifier hints
{
  "tools": {
    "approvalMode": "auto"
  },
  "permissions": {
    "autoMode": {
      "hints": {
        "allow": ["Running pytest, mypy, and ruff on this Python repo"],
        "softDeny": ["Editing Qwen Code settings unless explicitly requested"],
        "hardDeny": ["Sending secrets or .env contents to any network endpoint"]
      },
      "environment": ["Open-source monorepo; commits are signed"]
    }
  }
}
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes or CLI flags.

**Example 1: user allows, repo denies.**

User `~/.qwen/settings.json`:

```json
{
  "permissions": {
    "allow": ["Bash(curl *)"]
  }
}
```

Repo `.qwen/settings.json`:

```json
{
  "permissions": {
    "deny": ["Bash(curl *)"]
  }
}
```

Result: `curl` is blocked in the repository because deny rules from any scope override allow rules.

**Example 2: user default mode, CLI override.**

User `~/.qwen/settings.json`:

```json
{
  "tools": {
    "approvalMode": "auto-edit"
  }
}
```

CLI:

```bash
qwen -p "..." --approval-mode plan
```

Result: the session starts in `plan` mode; CLI flags override settings.

**Example 3: project whitelist, local addition.**

Repo `.qwen/settings.json`:

```json
{
  "tools": {
    "approvalMode": "default"
  },
  "permissions": {
    "allow": ["Read", "Grep", "Bash(npm test)"]
  }
}
```

If a user also has `~/.qwen/settings.json`:

```json
{
  "permissions": {
    "allow": ["Bash(npm run build)"]
  }
}
```

Result: in this repository, `npm test`, `npm run build`, Read, and Grep are all allowed because `allow` rules merge across scopes.

## Tools and Permissions

Qwen Code provides the following built-in tools and tool groups. The "Permission required" column indicates the effective behavior in `default` mode when no explicit rules match.

| Tool / group | Permission required | Notes |
| :----- | :----- | :----- |
| `read_file` (Read/ReadFile) | No | Reads file contents. The `Read` meta-category also covers grep, glob, and list. |
| `grep_search` (Grep) | No | Content search. |
| `glob` (Glob/FindFiles) | No | File pattern matching. |
| `list_directory` (ListFiles) | No | Directory listing. |
| `edit` (Edit/EditFile) | Yes | Targeted file edits. Covered by the `Edit` meta-category. |
| `write_file` (Write/WriteFile) | Yes | Creates or overwrites files. Covered by the `Edit` meta-category. |
| `notebook_edit` (NotebookEdit) | Yes | Jupyter notebook edits. Covered by the `Edit` meta-category. |
| `run_shell_command` (Bash/Shell) | Yes | Shell command execution. Read-only commands such as `ls`, `cat`, and `git status` are auto-approved. |
| `monitor` | Yes | Long-lived shell command background tasks. |
| `web_fetch` (WebFetch) | Yes | Fetches content from URLs. |
| `agent` / `task` (Agent) | Yes (spawn) | Spawns subagents; the subagent's own tool calls are checked independently. |
| `skill` (Skill) | Yes | Executes a skill. |
| `todo_write` | No | Session checklist management. |
| `exit_plan_mode` | Yes | Exits plan mode and presents a plan for approval. |
| `save_memory` | Yes | Persists durable memory. |
| `computer_use_*` | Yes | Native desktop automation tools. |
| MCP tools (`mcp__<server>__<tool>`) | Yes | External tools exposed through configured MCP servers. |

Permissions map to tool calls through the rule syntax described above. An `Edit(...)` allow rule also grants read access to the same path. Bash permission rules support glob patterns and recognize common read-only commands. MCP tools use the `mcp__<server>` or `mcp__<server>__<tool>` naming convention.

## MCP and Permissions

MCP servers extend Qwen Code with external tools. Once connected, their tools appear as `mcp__<server>__<tool>` and are governed by the same permission system as built-in tools.

Permission rules for MCP:

- `mcp__<server>` matches any tool from that server.
- `mcp__<server>__*` matches every tool from that server.
- `mcp__<server>__<tool>` matches a specific tool.
- `mcp__*` as a deny rule removes every MCP tool from the model's context.

MCP servers can be configured at user scope (`~/.qwen/settings.json`), project scope (`.qwen/settings.json`), or via `qwen mcp add`. Each server entry supports:

- `includeTools` / `excludeTools` — per-server tool allowlist/blocklist.
- `trust: true` — bypasses confirmation prompts for that server's tools.
- `mcp.allowed` / `mcp.excluded` — global server allowlist/denylist in `settings.json`.

Administrators can make MCP safer through several mechanisms:

- **Global allow/deny lists**: use `mcp.allowed` and `mcp.excluded` to control which servers load.
- **Per-server tool filtering**: use `includeTools`/`excludeTools` to expose only safe operations.
- **Trust control**: avoid setting `trust: true` unless the server is fully trusted.
- **Permission rules**: add `deny` rules such as `mcp__filesystem__write_file` or `mcp__github__create_issue` to block specific high-risk operations while keeping the server connected.
- **Safe mode**: `QWEN_CODE_SAFE_MODE=1` disables custom MCP servers and only loads built-in subagents.
- **Session server filtering**: use `--allowed-mcp-server-names` to load only named servers for one run.

When an MCP server is blocked by policy, it is not available to the model. In `default` mode, MCP tool calls prompt for approval unless covered by an allow rule or the server is trusted.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Qwen Code's sandbox is a separate layer from permission modes and rules. It provides OS-level filesystem and network isolation for shell and file-modifying tools.

- **Backends**: macOS uses Seatbelt (`sandbox-exec`); Linux/Windows use Docker/Podman containers.
- **Filesystem**: sandboxed tools can access the working directory and any `--include-directories`; other paths are denied unless the sandbox image/profile permits them.
- **Network**: container sandboxing has no pre-allowed domains; network access is controlled by the sandbox image configuration.
- **Scope**: sandboxing applies to shell and file-modifying tools. Built-in read tools, MCP tools, and subagent tool calls run outside this boundary.
- **Fallback**: if sandbox dependencies are missing, Qwen Code warns and may fall back to unsandboxed execution.

Permissions and sandboxing are complementary:

- Permission rules block Qwen from attempting restricted actions.
- Sandbox restrictions prevent shell commands from reaching resources outside defined boundaries, even if a prompt injection bypasses Qwen's decision-making.

### Trust and administrative controls

**Folder/project trust**: first-time launches in a project directory prompt a workspace trust dialog. Untrusted folders ignore project `.qwen/settings.json`, context files, hooks, extensions, skills, MCP servers, and custom subagents. Trust is saved per directory. Trust verification is skipped in non-interactive `-p` mode.

**Managed/admin policy**: system-level settings files (`/etc/qwen-code/settings.json` and system-defaults paths) provide the managed layer. They occupy a higher precedence tier than user/project config and cannot be overridden by lower scopes. Specific managed-only keys may lock approval modes or sandbox policy.

**Safe mode**: `QWEN_CODE_SAFE_MODE=1` disables context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, settings-sourced approval mode overrides, memory, and sandbox settings. Built-in tools and explicit CLI permission flags continue to work.

**Bare mode**: `--bare` skips implicit startup auto-discovery and honors only explicit CLI inputs. It is useful for reproducible CI runs.

### Protected paths

Auto mode and other approval logic protect a set of self-modification and persistence paths. Writes to these paths are never auto-approved except in YOLO mode, and may still prompt in other modes:

- Qwen Code internal state: `.qwen/`, `.qwen/settings.json`.
- Version control: `.git/`, `.gitconfig`, `.gitmodules`.
- Shell configuration: `.bashrc`, `.bash_profile`, `.bash_login`, `.bash_aliases`, `.bash_logout`, `.zshrc`, `.zprofile`, `.zshenv`, `.zlogin`, `.zlogout`, `.profile`, `.envrc`.
- SSH and other sensitive configuration files.

## Non-Interactive Behavior

In non-interactive `-p` mode, Qwen Code cannot show interactive permission prompts. Use one of these strategies:

- Pass `--allowed-tools` with the rules you want auto-approved.
- Start in `--approval-mode plan` for read-only exploration.
- Start in `--approval-mode yolo` for fully auto-approved runs.
- Use `--bare` to skip auto-discovery of project/user customizations and make runs reproducible.

Auto mode in `-p` works, but if the classifier blocks an action repeatedly the session may abort because there is no user to prompt. MCP servers requiring user approval are unavailable in `-p` unless already trusted or allowed by user/managed settings. Workspace trust dialogs are skipped in `-p` mode, so project-local MCP servers and project settings load only when already approved or allowed.

## Sources

- Qwen Code CLI help (`qwen --help --all`), version 0.15.6.
- Local installation at `/opt/homebrew/bin/qwen`.
- Qwen Code documentation: approval modes, permissions, sandboxing, settings, headless usage, sub-agents, MCP, trusted folders, hooks, and permission mediation.

## Changelog

- 2026-07-02: Refreshed research against Qwen Code 0.15.6 and current documentation. Corrected CLI flags (removed `--include-tools` and `--safe-mode`; added `--core-tools`, `--allowed-mcp-server-names`, and `--bare`). Limited `--approval-mode` CLI choices to `plan`/`default`/`auto-edit`/`yolo` and noted `auto` is available via settings.json and `/approval-mode`. Added schema-required frontmatter fields and updated the PolicyEngine assessment. Flagged Claudine backend/mutation updates as required.
