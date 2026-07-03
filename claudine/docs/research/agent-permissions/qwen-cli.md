---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: approval-mode
    style: switch
    description: Set the approval mode for the session. Values are plan (read-only analysis), default (ask before edits/shell), auto-edit (auto-approve file edits), auto (classifier-evaluated), and yolo (auto-approve all tool calls).
    example: qwen -p "refactor auth" --approval-mode auto-edit
    example_description: Starts a headless session that auto-approves file edits but still prompts for shell commands.
  - param: yolo
    style: switch
    description: Enable YOLO mode for the session, auto-approving all tool calls. Equivalent to --approval-mode yolo.
    example: qwen -p "run tests and commit" --yolo
    example_description: Runs a headless prompt with every tool call approved automatically.
  - param: allowed-tools
    style: switch
    description: Comma-separated list of tool rules that bypass the confirmation dialog for this session. Accepts tool names and Tool(specifier) patterns.
    example: qwen -p "..." --allowed-tools "Shell(npm test),Read"
    example_description: Auto-approves npm test shell commands and all read operations for this session.
  - param: exclude-tools
    style: switch
    description: Comma-separated list of tool names to remove from the session's available tool surface.
    example: qwen -p "..." --exclude-tools "Shell,Write,Edit"
    example_description: Prevents the model from using shell, write, and edit tools in this session.
  - param: include-tools
    style: switch
    description: Comma-separated allowlist of tool names for the session.
    example: qwen -p "..." --include-tools "Read,Grep,Glob"
    example_description: Restricts the session to read-only search tools.
  - param: disabled-slash-commands
    style: switch
    description: Comma-separated or repeated list of slash command names to disable for the session. Unioned with the slashCommands.disabled setting.
    example: qwen --disabled-slash-commands "auth,mcp,extensions"
    example_description: Disables the /auth, /mcp, and /extensions slash commands for this session.
  - param: safe-mode
    style: switch
    description: Disable customizations including context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, settings-sourced approval mode overrides, memory, and sandbox settings. --yolo and --approval-mode still take effect.
    example: qwen -p "debug" --safe-mode
    example_description: Starts an isolated session without project customizations while still respecting explicit permission flags.
  - param: sandbox
    style: switch
    description: Enable sandbox mode for the session, running shell and file-modifying tools inside a sandbox (sandbox-exec on macOS or Docker/Podman).
    example: qwen -s -p "analyze the code"
    example_description: Runs the session with filesystem/process isolation enabled.
  - param: sandbox-image
    style: switch
    description: Set the Docker/Podman image used when sandboxing.
    example: qwen -s --sandbox-image ghcr.io/qwenlm/qwen-code:0.19.4 -p "..."
    example_description: Uses a specific sandbox image for the session.
  - param: include-directories
    style: switch
    description: Add additional directories to the workspace scope for the session.
    example: qwen -p "..." --include-directories ../shared,../docs
    example_description: Expands the working directory set for this session.

env_vars:
  - name: QWEN_SANDBOX
    effect: Enable or configure sandbox mode (true/false/docker/podman/sandbox-exec). Takes precedence over the --sandbox flag and tools.sandbox setting.
  - name: QWEN_SANDBOX_IMAGE
    effect: Override the sandbox Docker/Podman image. Takes precedence over --sandbox-image and tools.sandboxImage.
  - name: SEATBELT_PROFILE
    effect: macOS-only. Select the sandbox-exec profile (permissive-open, permissive-closed, restrictive-open, etc.).
  - name: QWEN_CODE_SAFE_MODE
    effect: Equivalent to --safe-mode. Disables customizations including permission rules and settings-sourced approval mode overrides; explicit --yolo/--approval-mode still apply.
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: Comma-separated list of slash commands to disable. Unioned with slashCommands.disabled and --disabled-slash-commands.
  - name: QWEN_HOME
    effect: Changes the global configuration directory (default ~/.qwen), affecting where user-scoped settings, skills, agents, and memory are loaded.

config_files:
  - os: all
    user: ~/.qwen/settings.json
    repo: .qwen/settings.json

precedence:
  - source: CLI flags > environment variables > system settings file > project settings > user settings > system defaults file > hardcoded defaults
    scope: [permissions]
    merge_strategy: none
    notes: "Previous prose summary: CLI flags > environment variables > system settings file (/etc/qwen-code/settings.json) > project settings (.qwen/settings.json) > user settings (~/.qwen/settings.json) > system defaults file > hardcoded defaults. Within permission rules, deny > ask > allow, and a deny rule from any scope overrides allow rules from any scope."

default_posture: "With no configuration, Qwen Code starts in default approval mode (Ask Permissions): read-only built-in tools run without confirmation, while file edits, shell commands, web fetches, MCP tool calls, and other state-changing actions prompt for approval."

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
  ergonomic: true
  provides_coverage: true
  gaps:
    - Auto mode relies on an LLM classifier with natural-language hints; the classifier decision is not a deterministic static rule.
    - Meta-category rules such as Read cover read_file, grep_search, glob, and list_directory, requiring backend-specific expansion.
    - Path-pattern prefixes (//, ~/, /, ./) and shell-command word-boundary matching differ from generic glob semantics.
    - Protected self-modification and persistence paths are hard-coded exceptions in auto mode and cannot be expressed as static rules.
    - Subagent permission inheritance and parent-mode override (e.g., a yolo parent forces yolo on subagents) are runtime behaviors outside static policy.
    - MCP server-level allow/deny (mcp.allowed/mcp.excluded), per-server includeTools/excludeTools, and the trust flag are additional policy surfaces.
    - Folder trust and safe mode are trust/scope gates rather than permission rules.

changes: []

requires_claudine_update: true
reason: "Qwen Code's permission model uses approval modes, allow/ask/deny rules with meta-categories and path-pattern prefixes, subagent-scoped overrides, MCP include/exclude/trust, and an LLM-based auto-mode classifier. Fully representing these in Claudine's PolicyEngine will require backend updates to the Qwen backend and mutation planning for settings.json permission objects."
---

# Qwen CLI Permissions

## Introduction to Qwen CLI Permissions

Qwen CLI controls tool access through a combination of **approval modes**, **permission rules**, **sandboxing**, and **tool allowlists/blocklists**. The goal is to let the agent act autonomously when the risk is acceptable while keeping destructive or sensitive operations under user control.

Permissions can be defined in three ways:

1. **Configuration files** — `settings.json` at user, project, system-defaults, and system-override scopes.
2. **CLI flags** — `--approval-mode`, `--yolo`, `--allowed-tools`, `--exclude-tools`, `--safe-mode`, `--sandbox`, etc.
3. **In-session controls** — `/approval-mode`, `/permissions`, `/plan`, and the `Shift+Tab` (or `Tab` on Windows) mode switcher.

### Approval modes

Qwen Code supports five approval modes. The mode acts as the baseline; `permissions.allow`, `permissions.ask`, and `permissions.deny` rules refine it.

| Mode | File edits | Shell commands | Best for |
| :----- | :----- | :----- | :----- |
| `plan` | Not executed | Not executed | Safe exploration and planning |
| `default` | Ask | Ask | Daily interactive work |
| `auto-edit` | Auto-approve | Ask | Refactoring and code changes |
| `auto` | Classifier-evaluated | Classifier-evaluated | Long autonomous sessions with a safety net |
| `yolo` | Auto-approve | Auto-approve | Trusted automation and CI/CD |

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
- `--include-tools <tools>` restricts the session to an allowlist of tools.
- `--disabled-slash-commands <commands>` disables slash commands.
- `--safe-mode` strips project customizations, including permission rules, while leaving explicit flags in effect.
- `--sandbox` / `--sandbox-image` enable filesystem/process isolation.
- `--include-directories` expands the workspace scope.

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
qwen -p "explain the auth module" --include-tools "Read,Grep,Glob"

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

- **Interactive sessions**: yes, via `/approval-mode yolo` or `Shift+Tab` (or `Tab` on Windows) cycling.
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
- **Safe mode**: `--safe-mode` disables custom MCP servers and only loads built-in subagents.

When an MCP server is blocked by policy, it is not available to the model. In `default` mode, MCP tool calls prompt for approval unless covered by an allow rule or the server is trusted.
