---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: approval-mode
    style: switch
    description: Set the session approval mode. Values are default, auto_edit, plan, and yolo. Overrides general.defaultApprovalMode for this session.
    example: gemini --approval-mode plan
    example_description: Starts a read-only planning session where write operations require explicit approval.
  - param: yolo
    style: switch
    description: Deprecated alias for --approval-mode=yolo. Auto-approves all tool actions for the session.
    example: gemini -y -p "deploy to staging"
    example_description: Runs a headless prompt with all permission prompts auto-approved.
  - param: sandbox
    style: switch
    description: Enable sandboxed execution. Accepts a boolean or a provider such as docker, podman, sandbox-exec, runsc, or lxc.
    example: gemini -s -p "run the test suite"
    example_description: Runs the session inside the default sandbox provider for the platform.
  - param: skip-trust
    style: switch
    description: Trust the current workspace for this session, bypassing the folder trust dialog.
    example: gemini --skip-trust -p "summarize"
    example_description: Runs non-interactively in a folder without prompting for trust.
  - param: include-directories
    style: switch
    description: Add additional directories to the workspace. Repeatable or comma-separated.
    example: gemini --include-directories ../lib,../docs
    example_description: Expands read/write scope to sibling directories for this session.
  - param: allowed-mcp-server-names
    style: switch
    description: Restrict which configured MCP servers are available. Repeatable or comma-separated.
    example: gemini --allowed-mcp-server-names github,slack
    example_description: Only allows tools from the listed MCP servers for this session.
  - param: allowed-tools
    style: switch
    description: Deprecated tools allowlist. Use the Policy Engine instead.
    example: gemini --allowed-tools read_file
    example_description: Legacy way to allow specific tools without confirmation.
  - param: admin-policy
    style: switch
    description: Load supplemental admin policy files or directories. Ignored if standard system policy directories already contain .toml files.
    example: gemini --admin-policy /etc/gemini-cli/policies
    example_description: Loads enterprise policy TOML files at Admin tier.

env_vars:
  - name: GEMINI_SANDBOX
    effect: Enables sandboxing and optionally selects the provider (true, docker, podman, sandbox-exec, runsc, or lxc).
  - name: GEMINI_SANDBOX_IMAGE
    effect: Specifies a custom container image for Docker or Podman sandboxing.
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: When set to true, trusts the current workspace for the session, equivalent to --skip-trust.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Overrides the default path for the trusted folders registry (~/.gemini/trustedFolders.json).
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system-wide settings override file.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system-wide defaults file.
  - name: GEMINI_CLI_HOME
    effect: Redirects all user-scoped config and state to a different directory, useful for isolation.
  - name: SEATBELT_PROFILE
    effect: Selects a macOS Seatbelt sandbox profile (permissive-open, permissive-proxied, restrictive-open, restrictive-proxied, strict-open, strict-proxied).
  - name: SANDBOX_MOUNTS
    effect: Comma-separated list of host:container:opts mounts to add to a container sandbox.
  - name: SANDBOX_FLAGS
    effect: Passes custom flags into the docker or podman sandbox command.
  - name: SANDBOX_SET_UID_GID
    effect: Enables or disables host UID/GID mapping for Linux sandboxes.

config_files:
  user: ~/.gemini/settings.json
  repo: .gemini/settings.json

precedence: "CLI arguments > environment variables > system settings file > project settings > user settings > system defaults file > built-in defaults. Policy Engine TOML rules use tier precedence: Admin > User > Default (workspace tier is currently disabled)."

default_posture: "When nothing is configured, Gemini CLI uses general.defaultApprovalMode='default': read-only tools run automatically, write and shell tools prompt for confirmation, folder trust is disabled, sandboxing is off, and MCP servers require per-server trust."

agent_permissions:
  allowed: true
  fm_properties:
    - tools
    - mcpServers
    - subagent

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--approval-mode=yolo (or deprecated --yolo/-y)"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - TOML policy tier math (Admin/User/Default priority bases) is not represented in PolicyEngine's flat rule model.
    - Regex-based argsPattern, commandRegex, and toolAnnotations matching are outside the canonical allow/ask/deny rule shape.
    - Workspace policy tier exists in Gemini CLI but is currently non-functional.
    - Sandboxing (GEMINI_SANDBOX, sandbox providers, seatbelt profiles) is an orthogonal execution layer not modeled by PolicyEngine.
    - Folder trust safe-mode overrides policy and disables project settings, .env, MCP servers, and auto-acceptance.
    - MCP server trust, includeTools/excludeTools, and mcp.allowed/mcp.excluded lists are server-level controls beyond tool-level rules.
    - tools.core allowlisting and deprecated tools.exclude blocklisting are separate from policy rules.
    - security.disableYoloMode is an administrative lockout not expressed as a policy rule.
    - ask_user decisions become deny in non-interactive mode, which is a runtime mapping rather than a static policy effect.

changes: []

requires_claudine_update: true
reason: "Gemini CLI's permission surface combines approval modes, TOML policy tiers, regex/argsPattern rules, toolAnnotations, sandbox configuration, folder trust, MCP server trust/include/exclude/allowed lists, coreTools allowlisting, and disableYoloMode. Claudine's PolicyEngine would need a Gemini-specific backend extension to accurately model these layers and mutate them via config/policy files."
---

# Gemini CLI Permissions

## Introduction to Gemini CLI Permissions

Gemini CLI uses a layered permission model. The highest-level knob is the **approval mode**, which selects a broad posture such as read-only or auto-approve. Under that, the **Policy Engine** evaluates TOML rules that decide whether an individual tool call is allowed, denied, or requires user confirmation. Finally, **sandboxing** and **folder trust** provide isolation and trust-gating that can override or restrict what the policy engine would otherwise permit.

Permissions can be defined through:

1. **Configuration files** — JSON `settings.json` at system, user, and project scopes, plus TOML policy files in `~/.gemini/policies/` (User tier) and system policy directories (Admin tier).
2. **Environment variables** — such as `GEMINI_SANDBOX`, `GEMINI_CLI_TRUST_WORKSPACE`, and paths that relocate config files.
3. **CLI flags** — such as `--approval-mode`, `--sandbox`, `--skip-trust`, and `--admin-policy`.

### Approval modes

| Mode | Behavior |
| :--- | :--- |
| `default` | Read-only tools run automatically; write and shell tools ask for confirmation. |
| `auto_edit` | Optimized for automated editing; certain write operations are auto-approved. |
| `plan` | Read-only mode for research and design; edits always require approval. |
| `yolo` | All tools are auto-approved. Can only be enabled via CLI (`--approval-mode=yolo` or deprecated `--yolo`). |

The active mode can be set per session with `--approval-mode` or configured as `general.defaultApprovalMode` in `settings.json`. The value `yolo` is not allowed in `general.defaultApprovalMode`; it must be requested explicitly.

### Policy Engine basics

Policy rules are written in TOML and define a decision (`allow`, `deny`, or `ask_user`) for matching tool calls. Rules can match by `toolName`, `commandPrefix`, `commandRegex`, `argsPattern`, `mcpName`, `subagent`, `toolAnnotations`, `modes`, and `interactive`. Higher-priority rules win. The tiers are:

| Tier | Base | Location |
| :--- | :--- | :--- |
| Default | 1 | Built-in policies shipped with Gemini CLI. |
| Extension | 2 | Policies defined in extensions. |
| Workspace | 3 | **Currently disabled.** `.gemini/policies/*.toml` has no effect. |
| User | 4 | `~/.gemini/policies/*.toml` |
| Admin | 5 | System directories or supplemental `--admin-policy` paths. |

Final priority is computed as `tier_base + (toml_priority / 1000)`, so Admin rules always beat User rules, which beat Default rules.

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--approval-mode <mode>` sets the session approval mode.
- `--yolo` (deprecated) is an alias for `--approval-mode=yolo`.
- `--sandbox` enables sandboxed execution.
- `--skip-trust` bypasses the folder trust check.
- `--include-directories` expands the workspace.
- `--allowed-mcp-server-names` restricts which MCP servers load.
- `--allowed-tools` is a deprecated tool allowlist.
- `--admin-policy` loads supplemental Admin-tier policy files.

Precedence is documented in the frontmatter. Key points:

- CLI flags are temporary session overrides and beat environment variables and file config.
- Environment variables beat all settings files.
- The system settings override file has the final say among settings files.
- Project settings override user settings, which override system defaults.
- For policy rules, Admin-tier rules beat User-tier rules, which beat Default-tier rules. Workspace-tier policies are currently disabled.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Gemini CLI starts in `default` approval mode. Read-only tools such as `read_file`, `glob`, and `google_web_search` run without prompting, while `write_file`, `replace`, and `run_shell_command` require confirmation. Folder trust is disabled by default, so project `settings.json` and `.env` files load normally. MCP servers are discovered but their tools still require confirmation unless the server is marked `trust: true`.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for files under the workspace and included directories.
- `can_write(path)` → Ask for paths in the workspace; behavior outside the workspace depends on sandbox and trust settings.
- `can_execute(command)` → Ask for `run_shell_command`.
- `can_access_domain(domain)` → Ask for `web_fetch`; `google_web_search` is allowed.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask unless the server is trusted.
- `can_spawn_subagent(agent)` → Ask for remote agents; local subagent tool calls are checked individually.

This use case is only partially ergonomic in PolicyEngine. The engine can model the read/write/execute/network/MCP/agent axes, but Gemini CLI's default posture is also shaped by the active approval mode, sandbox state, and folder trust, none of which collapse cleanly into static allow/ask/deny rules.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, use `plan` mode or a restrictive Policy Engine configuration. In `plan` mode, the CLI is read-only by default and write operations always prompt.

You can also deny all tools by default and then allow only specific ones. In `~/.gemini/policies/lockdown.toml`:

```toml
[[rule]]
toolName = "*"
decision = "deny"
priority = 100

[[rule]]
toolName = "read_file"
decision = "allow"
priority = 200

[[rule]]
toolName = "run_shell_command"
commandPrefix = "git"
decision = "ask_user"
priority = 200
```

CLI examples:

```bash
# Run a read-only exploration with no edits allowed
gemini --approval-mode plan "explain the auth module"

# Non-interactive read-only summary; ask_user becomes deny
gemini -p --approval-mode plan "summarize README.md"

# Allow only a specific MCP server for one session
gemini --allowed-mcp-server-names github "list my open PRs"
```

In interactive sessions, the `/permissions` command can change folder trust, but it cannot override a Policy Engine `deny` rule or an approval mode that forbids auto-approval.

PolicyEngine can describe this use case by setting `SetApprovalMode(plan)` and adding allow rules for the approved surface. It is not fully ergonomic because Gemini CLI's deny-by-default can only be expressed through Policy Engine rules if the engine supports wildcard deny rules, and because `ask_user` maps to `deny` in non-interactive mode.

### YOLO

In Gemini CLI, YOLO mode is the `yolo` approval mode. A session can be put into this mode by:

- Starting with `--approval-mode=yolo`.
- Starting with the deprecated `--yolo` or `-y` flag.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags.
- **Non-interactive sessions**: yes, `gemini -p --approval-mode=yolo` works.
- **Root/sudo on macOS and Linux**: no documented restriction. YOLO remains available to root unless an administrator disables it with `security.disableYoloMode`.

When in `yolo` mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits, shell commands, web fetch, MCP tool calls, and subagent spawns.
- **Still constrained**: sandbox boundaries, folder trust safe-mode, and `security.disableYoloMode` still apply. A `security.disableYoloMode: true` setting blocks YOLO entirely.
- **Not allowed**: it cannot override a disabled YOLO mode or Admin-tier deny rules that outrank the built-in YOLO allow rule.

### Root User

Gemini CLI does not document any special permission behavior when started as the root user. Unlike some other agentic CLIs, there is no published check that refuses YOLO mode or sandbox bypass based on UID. Root sessions can still use `--approval-mode=yolo` unless `security.disableYoloMode` is set, and sandboxing can still be enforced or disabled via flags and config.

### Configuring the Default

Default permissions are configured through JSON and TOML files at several scopes:

- **User scope**: `~/.gemini/settings.json` applies across all projects.
- **Repo/project scope**: `.gemini/settings.json` applies when running from that project directory.
- **Policy scope**: `~/.gemini/policies/*.toml` for User-tier rules; system policy directories for Admin-tier rules.
- **System scope**: `/etc/gemini-cli/settings.json` (Linux), `/Library/Application Support/GeminiCli/settings.json` (macOS), or `C:\ProgramData\gemini-cli\settings.json` (Windows) for machine-wide overrides.
- **System defaults scope**: the corresponding `system-defaults.json` paths provide machine-wide baselines.

For the schema's `config_files` field, user scope is `~/.gemini/settings.json` and repo scope is `.gemini/settings.json`.

Examples that illustrate the grammar:

```json
// ~/.gemini/settings.json — user-wide defaults
{
  "general": {
    "defaultApprovalMode": "plan"
  },
  "security": {
    "disableYoloMode": true
  }
}
```

```json
// .gemini/settings.json — repo-shared defaults
{
  "tools": {
    "sandbox": "docker"
  },
  "mcp": {
    "allowed": ["corp-tools"]
  }
}
```

```toml
# ~/.gemini/policies/user-defaults.toml
[[rule]]
toolName = "run_shell_command"
commandPrefix = "npm"
decision = "allow"
priority = 100
modes = ["default", "auto_edit"]

[[rule]]
toolName = "write_file"
argsPattern = '"file_path":".*\\.env"'
decision = "deny"
priority = 200
denyMessage = "Writing .env files is not allowed."
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows a shell command, repo denies it.**

User `~/.gemini/policies/user-defaults.toml`:

```toml
[[rule]]
toolName = "run_shell_command"
commandPrefix = "curl"
decision = "allow"
priority = 100
```

Repo `.gemini/settings.json`:

```json
{
  "tools": {
    "core": ["ReadFileTool", "GlobTool", "ShellTool(ls)"]
  }
}
```

Result: in the repository, only the allowlisted tools are available, so `curl` is effectively blocked because `run_shell_command` is no longer exposed to the model.

**Example 2: user default mode, CLI override.**

User `~/.gemini/settings.json`:

```json
{
  "general": {
    "defaultApprovalMode": "auto_edit"
  }
}
```

CLI:

```bash
gemini --approval-mode plan
```

Result: the session starts in `plan` mode. CLI flags override settings file values.

**Example 3: user denies an MCP server, project allows a specific tool.**

User `~/.gemini/policies/mcp.toml`:

```toml
[[rule]]
mcpName = "third-party-analyzer"
decision = "deny"
priority = 100
```

Repo `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "third-party-analyzer": {
      "command": "/usr/local/bin/start-analyzer.sh",
      "includeTools": ["code-search"]
    }
  }
}
```

Result: the server is configured, but the User-tier deny rule blocks all of its tools regardless of the project include list.

## Tools and Permissions

Gemini CLI provides the following built-in tools. The "Default Policy" column indicates the Policy Engine decision in `default` approval mode.

| Tool | Category | Default Policy | Notes |
| :--- | :--- | :--- | :--- |
| `run_shell_command` | Execution | `ask_user` | Requires confirmation by default. |
| `glob` | File System | `allow` | Read-only search. |
| `grep_search` | File System | `allow` | Read-only search. |
| `list_directory` | File System | `allow` | Read-only directory listing. |
| `read_file` | File System | `allow` | Reads text, images, audio, and PDF. |
| `read_many_files` | File System | `allow` | Triggered by `@` file references. |
| `replace` | File System | `ask_user` | File edits require confirmation. |
| `write_file` | File System | `ask_user` | File writes require confirmation. |
| `ask_user` | Interaction | `allow` | Prompts the user for clarification. |
| `write_todos` | Interaction | `allow` | Internal task tracking. |
| `list_mcp_resources` | MCP | `allow` | Discovers MCP resources. |
| `read_mcp_resource` | MCP | `allow` | Reads MCP resources. |
| `activate_skill` | Memory | `allow` | Loads an agent skill. |
| `get_internal_docs` | Memory | `allow` | Retrieves CLI documentation. |
| `enter_plan_mode` | Planning | `allow` | Switches to read-only plan mode. |
| `exit_plan_mode` | Planning | `ask_user` | Presents the plan for approval. |
| `complete_task` | System | `allow` | Subagent completion tool. |
| `tracker_create_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_update_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_get_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_list_tasks` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_add_dependency` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_visualize` | Task Tracking | `allow` | Experimental task tracker. |
| `update_topic` | Task Tracking | `allow` | Updates session topic/status. |
| `google_web_search` | Web | `allow` | Google Search is allowed by default. |
| `web_fetch` | Web | `ask_user` | Fetching arbitrary URLs requires confirmation. |

Permissions map to tool calls through the Policy Engine. Rules can target built-in tool names, MCP tool FQNs (`mcp_{serverName}_{toolName}`), or subagent names. In `plan` mode, write tools always ask; in `yolo` mode, a high-priority rule allows all tools.

## MCP and Permissions

MCP servers extend Gemini CLI with external tools. Their configuration lives in the `mcpServers` object of `settings.json` and is governed by several permission layers.

Permission controls for MCP:

- **Server allowlist**: the global `mcp.allowed` array restricts which configured servers connect. If it is set, servers not in the list are ignored.
- **Server blocklist**: the global `mcp.excluded` array disables specific servers.
- **Per-server trust**: `mcpServers.<name>.trust: true` bypasses confirmation for all tools from that server.
- **Tool filtering**: `includeTools` exposes only listed tools; `excludeTools` removes listed tools and takes precedence over `includeTools`.
- **Environment redaction**: sensitive host environment variables are automatically redacted from MCP server processes unless explicitly listed in the server's `env` block.
- **Policy Engine rules**: use `mcpName` to target a server, optionally combined with `toolName`, or `mcpName = "*"` for all servers.
- **Folder trust**: when a workspace is untrusted, MCP servers do not connect at all, regardless of policy.

MCP tools are registered with fully qualified names of the form `mcp_{serverName}_{toolName}`. Avoid underscores in server names because the policy parser splits the FQN on the first underscore after `mcp_`.

To make MCP safer:

- Define approved servers in system `settings.json` and list them in `mcp.allowed`.
- Use `includeTools` to expose only the tools a workflow needs.
- Set `trust: false` by default and rely on Policy Engine rules or per-session approval.
- Run in a sandbox so MCP server side effects are isolated from the host.
- In untrusted workspaces, folder trust automatically disables MCP connections.
