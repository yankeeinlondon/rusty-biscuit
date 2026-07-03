---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: yolo
    style: switch
    description: Auto-approve all tool calls for the session. Aliases are --yes, -y, and --auto-approve. AskUserQuestion can still reach the user unless AFK mode is also active.
    example: kimi --yolo -p "refactor the auth module"
    example_description: Runs a headless prompt with all approval prompts auto-approved.
  - param: afk
    style: switch
    description: Away-from-keyboard mode. Auto-approves all tool calls and auto-dismisses AskUserQuestion so the agent can run unattended.
    example: kimi --afk -p "run the full test suite"
    example_description: Runs non-interactively without stopping for approvals or clarifying questions.
  - param: plan
    style: switch
    description: Start the session in plan mode. The agent may only use read-only tools and must submit a written plan for approval before executing it.
    example: kimi --plan
    example_description: Starts an interactive planning session where file edits and commands are blocked until the plan is approved.
  - param: print
    style: switch
    description: Run in non-interactive print mode. Implicitly enables AFK mode, so all tool calls are auto-approved and AskUserQuestion is auto-dismissed.
    example: kimi --print -p "summarize README.md"
    example_description: Produces text output without prompting; all actions are auto-approved.
  - param: quiet
    style: switch
    description: Shortcut for --print --output-format text --final-message-only. Implicitly enables auto-approval because print mode implies AFK.
    example: kimi --quiet -p "generate a commit message"
    example_description: Returns only the final answer with all tool calls auto-approved.
  - param: add-dir
    style: switch
    description: Add an additional directory to the workspace scope. File tools can read and write in added directories subject to the same approval rules as the working directory.
    example: kimi --add-dir ../shared --add-dir ../docs
    example_description: Expands the accessible filesystem scope for the session.
  - param: work-dir
    style: switch
    description: Set the working directory. Determines the default filesystem scope for file tools; relative paths resolve against it.
    example: kimi --work-dir /path/to/project
    example_description: Changes the root directory whose files are accessible without absolute paths.
  - param: agent
    style: switch
    description: Select a built-in agent (default or okabe). Changes the tool set available to the model.
    example: kimi --agent okabe
    example_description: Uses the okabe agent, which adds SendDMail to the default tool set.
  - param: agent-file
    style: switch
    description: Load a custom agent YAML file that defines the available tools and subagents for the session.
    example: kimi --agent-file ./agents/readonly.yaml
    example_description: Restricts the session to the tools listed in the custom agent file.
  - param: mcp-config-file
    style: switch
    description: Load additional MCP server definitions from a JSON file. Adds MCP tools to the session under the same approval rules as built-in tools.
    example: kimi --mcp-config-file ./mcp.json
    example_description: Makes MCP tools from the file available for the session.
  - param: mcp-config
    style: switch
    description: Pass MCP server definitions as an inline JSON string.
    example: kimi --mcp-config '<inline-json-with-mcpServers>'
    example_description: Adds MCP tools for this session only.
  - param: config-file
    style: switch
    description: Load a full configuration file (TOML or JSON). Can set default_yolo and default_plan_mode among other values.
    example: kimi --config-file ./team-config.toml
    example_description: Uses an alternate config that may enable or disable auto-approve by default.
  - param: config
    style: switch
    description: Provide configuration content inline. Overrides the default config file for this run.
    example: kimi --config 'default_yolo = true' -p "deploy"
    example_description: Enables YOLO mode via inline TOML config without editing a file.

env_vars: []

config_files:
  - os: all
    user: ~/.kimi/config.toml
    repo: ""

precedence:
  - source: CLI flags > configuration file
    scope: [permissions]
    merge_strategy: none
    notes: "Previous prose summary: CLI flags > configuration file (~/.kimi/config.toml). No environment variables that influence Kimi Code CLI permission settings were identified."

default_posture: "With no configuration, Kimi Code CLI starts in interactive default mode: read-only tools (Glob, Grep, ReadFile, ReadMediaFile, TaskList, TaskOutput, SearchWeb, FetchURL, etc.) run without approval, while Shell, WriteFile, StrReplaceFile, TaskStop, ExitPlanMode, and MCP tool calls prompt for confirmation on each use."

agent_permissions:
  allowed: false

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--yolo (and aliases --yes/-y/--auto-approve), the /yolo slash command, default_yolo in ~/.kimi/config.toml, and --afk/--print for invocation-only auto-approve"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Kimi Code CLI has no allow/ask/deny rule syntax; PolicyEngine's canonical rule model has no native rules to map to."
    - "There is no per-tool, per-path, or per-domain allow/deny configuration, only session-wide YOLO/AFK/Plan modes."
    - "Network access (SearchWeb, FetchURL) is gated by service configuration rather than permission rules."
    - "MCP server or tool allow/deny lists do not exist; all MCP tools share the same approval path."
    - "Subagent policy is expressed through tool lists in agent YAML, not through approval modes scoped to a subagent."
    - "PreToolUse hooks are external shell commands and therefore runtime policy outside PolicyEngine's static model."
    - "Only one user-scoped config file exists; there is no repo-scoped or local config precedence to model."
    - "Approve for this session is a runtime state mutation, not a persisted policy rule."

changes: []

requires_claudine_update: true
reason: "Kimi Code CLI's permission surface is a coarse session-wide approval model (YOLO/AFK/Plan) plus optional PreToolUse hooks, rather than the allow/ask/deny rule model PolicyEngine assumes. Claudine would need a Kimi-specific PolicyEngine backend that maps the binary approval modes, plan mode, and hook-based blocks to canonical queries, plus mutation planning that targets default_yolo/default_plan_mode and hook definitions in ~/.kimi/config.toml."
---

# Kimi Code CLI Permissions

## Introduction to Kimi Code CLI Permissions

Kimi Code CLI uses a coarse-grained, session-wide permission model. Instead of per-tool, per-path, or per-domain rules, the CLI chooses between interactive approval and auto-approve modes. Every tool call that can change state either prompts the user once, runs without prompting because the session is in YOLO/AFK mode, or is blocked by a `PreToolUse` hook.

Permissions can be configured through:

1. **Configuration file** — `~/.kimi/config.toml` sets `default_yolo` and `default_plan_mode`.
2. **CLI flags** — `--yolo`, `--afk`, `--plan`, `--print`, `--agent-file`, `--mcp-config-file`, etc.
3. **In-session controls** — `/yolo`, `/afk`, and `/plan` toggle modes at runtime.
4. **Hooks** — `PreToolUse` hooks can block or allow individual tool calls based on external shell commands.

### Permission modes

| Mode | Behavior | Best for |
| :--- | :--- | :--- |
| `default` | Read-only tools run freely; Shell, WriteFile, StrReplaceFile, TaskStop, ExitPlanMode, and MCP tool calls prompt. | Everyday interactive work. |
| `plan` | Only read-only tools are allowed. The agent writes a plan and submits it for approval before execution. | Designing changes before implementing them. |
| `yolo` | All tool calls are auto-approved. AskUserQuestion still reaches the user. | Trusted, isolated environments. |
| `afk` | All tool calls are auto-approved and AskUserQuestion is auto-dismissed. | Unattended or CI-style runs. |

### Configuration file fields

The only permission-related fields in `~/.kimi/config.toml` are:

| Field | Type | Default | Effect |
| :--- | :--- | :--- | :--- |
| `default_yolo` | boolean | `false` | Start every new session in YOLO mode. |
| `default_plan_mode` | boolean | `false` | Start every new session in plan mode. |
| `hooks` | array | `[]` | Define `PreToolUse` hooks that can block tool calls. |

### CLI parameters and precedence

The CLI parameters that influence permissions are listed in the frontmatter. The effective precedence for permission-related settings is:

1. **CLI flags** (e.g., `--yolo`, `--plan`) override config-file defaults.
2. **Configuration file** (`~/.kimi/config.toml`) supplies the defaults when no CLI flag is present.

No environment variables that directly change Kimi Code CLI permissions were identified in the documentation or source code.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch changes permissions, Kimi Code CLI starts in `default` mode. Read-only tools run without approval, and state-changing tools prompt on each use.

A PolicyEngine description of this posture would be:

- `can_read(path)` → Allow for paths inside the working directory and added directories; sensitive files (`.env`, SSH keys, cloud credentials) are still rejected by the tool layer.
- `can_write(path)` → Ask for paths inside the working directory; absolute paths are required outside it.
- `can_execute(command)` → Ask for every `Shell` call.
- `can_access_domain(domain)` → Allow for `SearchWeb` and `FetchURL` (network access is not approval-gated, only service-config-gated).
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask for every MCP tool call.
- `can_spawn_subagent(agent)` → Allow to spawn, but the subagent's own state-changing tool calls are checked independently.

This use case is only partially ergonomic in PolicyEngine. The engine can model the high-level read/write/execute/MCP axes, but Kimi has no static allow/ask/deny rules, so PolicyEngine would have to invent canonical rules that do not exist in the native config. The actual ask/deny behavior is also influenced by hardcoded tool-layer filters and runtime hooks, which PolicyEngine cannot capture.

### Whitelisting

Kimi Code CLI does **not** support a true whitelisting model. There is no way to set the default posture to "no permissions" and then require every needed permission to be asked for or explicitly declared. Read-only tools are always available in `default` mode, and there is no per-tool allow/deny syntax.

The closest approximations are:

- **`default_plan_mode = true`** — restricts the agent to read-only tools at startup.
- **Custom agent file with a minimal `tools` list** — prevents the model from using removed tools, but the remaining tools still follow their normal approval rules.
- **`PreToolUse` hooks** — can block specific tool names or arguments, but hooks fail-open (a crashed hook allows the action).

Because Kimi lacks allow/deny rules, PolicyEngine cannot describe a whitelist for it without extending the engine. A `SetApprovalMode(dontAsk)` plus `allow` rules would not map to any native Kimi config, and there is no `deny` mechanism to fall back to.

### YOLO

A Kimi Code CLI session can be put into YOLO mode in several ways:

- Start with `--yolo`, `--yes`, `-y`, or `--auto-approve`.
- Start with `--afk` (AFK implies auto-approve).
- Start with `--print` or `--quiet` (print mode implies AFK).
- Toggle `/yolo` during an interactive session.
- Set `default_yolo = true` in `~/.kimi/config.toml`.

Availability:

- **Interactive sessions**: yes, via `--yolo` at startup or `/yolo` at runtime.
- **Non-interactive sessions**: yes, via `--yolo`, `--afk`, or `--print`/`--quiet` combined with `-p`.

When in YOLO mode (or any auto-approve mode):

- **Allowed**: Shell commands, file writes/edits, MCP tool calls, task stops, and plan-mode transitions are auto-approved.
- **Still gated**: ReadFile and Grep still reject sensitive files such as `.env`, SSH private keys, and cloud credentials. `PreToolUse` hooks can still block actions.
- **Not allowed**: YOLO cannot bypass missing API keys, quota limits, invalid configuration, or hook blocks that return exit code `2`.

### Root User

Kimi Code CLI does **not** appear to treat root users differently from regular users. There are no documented root/sudo blocks for YOLO or AFK mode, and no source-code checks that disable auto-approve based on the effective UID. YOLO remains allowed when running as root.

### Configuring the Default

Default permissions are configured in a single user-scoped file:

- **User scope**: `~/.kimi/config.toml`
- **Repo scope**: none. Kimi Code CLI does not load a project-scoped configuration file by default.

Examples that illustrate the available grammar:

```toml
# ~/.kimi/config.toml — enable YOLO by default
default_yolo = true
```

```toml
# ~/.kimi/config.toml — start new sessions in plan mode by default
default_plan_mode = true
```

```toml
# ~/.kimi/config.toml — block edits to .env files via a PreToolUse hook
[[hooks]]
event = "PreToolUse"
matcher = "WriteFile|StrReplaceFile"
command = ".kimi/hooks/protect-env.sh"
timeout = 10
```

```toml
# ~/.kimi/config.toml — block dangerous shell commands via a PreToolUse hook
[[hooks]]
event = "PreToolUse"
matcher = "Shell"
command = ".kimi/hooks/safety-check.sh"
timeout = 10
```

### Extending the Base

Because Kimi Code CLI has only one config-file scope, "narrower scope" overrides come from CLI flags, agent files, and hooks rather than from repo config files.

**Example 1: config enables YOLO, CLI forces plan mode**

User `~/.kimi/config.toml`:

```toml
default_yolo = true
```

CLI:

```bash
kimi --plan
```

Result: the session starts in plan mode. The CLI flag overrides the config default, so the agent is restricted to read-only tools despite `default_yolo`.

**Example 2: config enables plan mode, CLI enables YOLO**

User `~/.kimi/config.toml`:

```toml
default_plan_mode = true
```

CLI:

```bash
kimi --yolo -p "apply the refactor"
```

Result: the session runs in YOLO mode. The CLI flag overrides the config default, so all tool calls are auto-approved.

**Example 3: user config enables YOLO, custom agent file narrows the tool set**

User `~/.kimi/config.toml`:

```toml
default_yolo = true
```

`readonly.yaml`:

```yaml
version: 1
agent:
  name: readonly
  extend: default
  exclude_tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.web:SearchWeb"
    - "kimi_cli.tools.web:FetchURL"
```

CLI:

```bash
kimi --agent-file ./readonly.yaml
```

Result: YOLO mode is still active, but the agent cannot call Shell, SearchWeb, or FetchURL because they are removed from its tool set.

**Example 4: repo-specific PreToolUse hook adds an extra guard**

User `~/.kimi/config.toml`:

```toml
default_yolo = true
```

Repo `.kimi/hooks/protect-main.sh`:

```bash
#!/bin/bash
read JSON
echo "$JSON" | jq -r '.tool_input.path // .tool_input.file_path // ""' | grep -qE '(/|^)main\.rs$'
if [ $? -eq 0 ]; then
    echo "Error: direct edits to main.rs are not allowed." >&2
    exit 2
fi
exit 0
```

User `~/.kimi/config.toml` hook entry:

```toml
[[hooks]]
event = "PreToolUse"
matcher = "WriteFile|StrReplaceFile"
command = "./.kimi/hooks/protect-main.sh"
timeout = 10
```

Result: even in YOLO mode, the hook blocks writes to `main.rs` for that repository.

## Tools and Permissions

The default agent enables the following built-in tools. The "Approval Required" column reflects the behavior in `default` mode.

| Tool | Approval Required | Notes |
| :--- | :--- | :--- |
| `Agent` | No | Spawns subagents; subagent tool calls are checked independently. Not available to subagents themselves. |
| `AskUserQuestion` | No | Presents questions to the user; not an approval prompt. |
| `SetTodoList` | No | Manages session todo list. |
| `Shell` | Yes | Each command prompts for confirmation in default mode. |
| `ReadFile` | No | Rejects sensitive files such as `.env`, SSH keys, and cloud credentials. |
| `ReadMediaFile` | No | Reads images/videos; model must support the capability. |
| `Glob` | No | File discovery. |
| `Grep` | No | Sensitive files are filtered out even when matched. |
| `WriteFile` | Yes | Creates or overwrites files. |
| `StrReplaceFile` | Yes | Edits files via string replacement. |
| `SearchWeb` | No | Requires search service configuration. |
| `FetchURL` | No | Uses fetch service if configured, otherwise local HTTP. |
| `Think` | No | Records reasoning content. |
| `SendDMail` | Yes | Experimental delayed-message tool (only in `okabe` agent). |
| `EnterPlanMode` | Yes* | Prompts unless the session is in YOLO or AFK mode. |
| `ExitPlanMode` | Yes | Submits the plan for approval. |
| `TaskList` | No | Lists background tasks. |
| `TaskOutput` | No | Queries background task status/output. |
| `TaskStop` | Yes | Stops a running background task. |

Permissions map to tool calls at the tool-name level. There is no finer-grained rule syntax such as `Shell(rm *)` or `WriteFile(/secrets/**)`. The only ways to influence whether a tool runs are:

- Toggle YOLO/AFK to skip all approval prompts.
- Remove the tool from the agent's tool list via `--agent` or `--agent-file`.
- Add a `PreToolUse` hook that inspects the tool name and input and exits with code `2` to block it.

When a user chooses **Allow for this session** in the approval panel, the action name is added to the session's `auto_approve_actions` set and persisted with the session, so resumed sessions keep that auto-approval.

## MCP and Permissions

MCP servers extend Kimi Code CLI with external tools. Servers are configured in `~/.kimi/mcp.json` or loaded ad hoc via `--mcp-config-file` and `--mcp-config`.

Permission behavior for MCP:

- **Approval**: every MCP tool call follows the same approval mechanism as built-in tools. In `default` mode, each MCP tool call prompts for confirmation.
- **YOLO/AFK**: in YOLO, AFK, or print mode, MCP tool calls are auto-approved along with built-in tools.
- **No allow/deny lists**: there is no native mechanism to allow or deny specific MCP servers or tools beyond loading or not loading the server.
- **Subagent tool lists**: removing MCP tools from a custom agent file is not documented as a supported way to restrict MCP access; MCP tools are loaded globally for the session.

To make MCP usage safer:

- Load only MCP servers from trusted sources.
- Avoid YOLO/AFK/print mode when using untrusted MCP servers.
- Use `PreToolUse` hooks to inspect the `tool_name` and block calls to high-risk MCP tools (for example, tools that write to external systems).
- Keep `~/.kimi/mcp.json` minimal and use `--mcp-config-file` to scope servers to specific sessions.
- Store MCP OAuth tokens securely; Kimi Code CLI keeps them in `~/.kimi/mcp-oauth/`.
