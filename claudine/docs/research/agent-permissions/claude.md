---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: permission-mode
    style: switch
    description: Begin the session in the specified permission mode. Values are default, acceptEdits, plan, auto, dontAsk, and bypassPermissions. Overrides the defaultMode setting for this session.
    example: claude --permission-mode plan
    example_description: Starts an interactive planning session where file edits are never auto-approved.
  - param: allowedTools
    style: switch
    description: Add allow rules for the session. Matching tool calls execute without prompting. Accepts the same Tool(specifier) syntax used in settings.json.
    example: claude --allowedTools "Bash(npm run *),Read,Edit"
    example_description: Auto-approves npm run commands and all Read/Edit calls for the session.
  - param: disallowedTools
    style: switch
    description: Add deny rules for the session. A bare tool name removes the tool from the model's context; a scoped rule blocks matching calls. Accepts Tool(specifier) syntax.
    example: claude --disallowedTools "Agent(Explore),Bash(rm *)"
    example_description: Disables the Explore subagent and blocks rm commands while leaving other Bash calls available.
  - param: dangerously-skip-permissions
    style: switch
    description: Equivalent to --permission-mode bypassPermissions. Skips permission prompts for the session. Refused when running as root on macOS/Linux outside a recognized sandbox.
    example: claude -p --dangerously-skip-permissions "deploy to staging"
    example_description: Runs a non-interactive deployment prompt with all permission prompts auto-approved.
  - param: allow-dangerously-skip-permissions
    style: switch
    description: Adds bypassPermissions to the interactive Shift+Tab mode cycle without starting in it. Useful when you want to begin in another mode and switch later.
    example: claude --permission-mode plan --allow-dangerously-skip-permissions
    example_description: Starts in plan mode but lets you cycle into bypassPermissions later via Shift+Tab.
  - param: add-dir
    style: switch
    description: Add additional working directories that Claude may read and edit. Files in these directories follow the same permission rules as the launch directory.
    example: claude --add-dir ../shared ../docs
    example_description: Grants access to sibling directories for this session only.
  - param: tools
    style: switch
    description: Restrict which built-in tools Claude can use. Pass a comma-separated list, default for the full set, or an empty string to disable all built-in tools. MCP tools are not constrained by this flag.
    example: claude --tools "Bash,Read,Edit"
    example_description: Limits the session to Bash, Read, and Edit tools.
  - param: mcp-config
    style: switch
    description: Load MCP servers from a JSON file or inline JSON string. Servers added this way are available for the session.
    example: claude --mcp-config ./mcp.json
    example_description: Loads MCP servers defined in a project-local configuration file.
  - param: strict-mcp-config
    style: switch
    description: Only use MCP servers provided via --mcp-config, ignoring user, project, plugin, and claude.ai connector servers.
    example: claude --strict-mcp-config --mcp-config ./ci-mcp.json
    example_description: Runs a locked-down session where only the explicitly supplied MCP servers load.
  - param: permission-prompt-tool
    style: switch
    description: In non-interactive mode, route permission prompts to the named MCP tool for programmatic approval.
    example: claude -p --permission-prompt-tool mcp_auth_tool "query"
    example_description: Delegates permission decisions to an MCP tool during a headless run.
  - param: safe-mode
    style: switch
    description: Disables customizations such as CLAUDE.md, skills, plugins, hooks, MCP servers, commands/agents, and output styles. Built-in tools and permissions continue to work normally.
    example: claude --safe-mode
    example_description: Starts a session free of project customizations while keeping the permission system intact.
  - param: bare
    style: switch
    description: Minimal mode that skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Keeps Bash, Read, and Edit tools. Useful for CI and scripts.
    example: claude --bare -p "Summarize this file" --allowedTools "Read"
    example_description: Runs a headless summary task with no project configuration loaded.

env_vars:
  - name: CLAUDE_CODE_ENABLE_AUTO_MODE
    effect: Set to 1 to make auto mode available on Amazon Bedrock, Google Cloud Vertex AI, Microsoft Foundry, and signed-in Claude apps gateway sessions. Auto mode is available by default on the Anthropic API. Requires v2.1.158+.
  - name: CLAUDE_CODE_MCP_ALLOWLIST_ENV
    effect: Set to 1 to spawn stdio MCP servers with only a safe baseline environment plus the server's configured env, rather than inheriting the user's full shell environment.
  - name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
    effect: Set to 1 to strip Anthropic and cloud-provider credentials from subprocess environments (Bash tool, hooks, MCP stdio servers). On Linux, also runs Bash subprocesses in an isolated PID namespace.
  - name: CLAUDE_CODE_SIMPLE
    effect: Set to 1 to run with a minimal system prompt and only Bash, file read, and file edit tools. Disables auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Equivalent to --bare.
  - name: CLAUDE_CODE_SAFE_MODE
    effect: Set to 1 to start in safe mode. Disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory. Permissions work normally.
  - name: ENABLE_CLAUDEAI_MCP_SERVERS
    effect: Set to false to disable claude.ai MCP connectors for the session. Same effect as the disableClaudeAiConnectors setting, but does not affect servers passed via --mcp-config.
  - name: CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST
    effect: Set by host platforms that embed Claude Code and manage model provider routing. When set, provider-selection, endpoint, and auth variables in settings files are ignored.

config_files:
  - os: all
    user: ~/.claude/settings.json
    repo: .claude/settings.json

precedence:
  - source: managed settings > CLI flags > environment variables > local project settings > shared project settings > user settings
    scope: [permissions]
    merge_strategy: none
    notes: "Previous prose summary: managed settings > CLI flags > environment variables (where they apply) > local project settings (.claude/settings.local.json) > shared project settings (.claude/settings.json) > user settings (~/.claude/settings.json). Deny rules from any scope override allow rules."

default_posture: "When nothing is configured, Claude Code starts in default permission mode: read-only tools (Read, Grep, Glob, LSP, etc.) run without approval, while Bash commands, file edits, Write, WebFetch, WebSearch, and other state-changing tools prompt for approval on first use."

agent_permissions:
  allowed: true
  fm_properties:
    - tools
    - disallowedTools
    - permissionMode

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--permission-mode bypassPermissions or --dangerously-skip-permissions; interactive sessions can also add it to the Shift+Tab cycle with --allow-dangerously-skip-permissions"

policy_engine:
  ergonomic: true
  provides_coverage: true
  gaps:
    - Auto mode classifier rules (autoMode.environment, allow, soft_deny, hard_deny) are prose evaluated by a model, not deterministic allow/ask/deny rules.
    - Protected-path circuit breakers are hard-coded and apply even when static policy would predict Allow.
    - PreToolUse hooks can block calls before policy evaluation; hook-based decisions are outside PolicyEngine's static rule model.
    - Subagent permissionMode can be overridden at runtime when the parent uses bypassPermissions, acceptEdits, or auto mode.
    - Managed-only administrative controls such as allowManagedPermissionRulesOnly and disableBypassPermissionsMode are policy enforcement knobs outside the user-facing permission rule surface.

changes: []

requires_claudine_update: false
---

# Claude Code Permissions

## Introduction to Claude Code Permissions

Claude Code uses a tiered permission system that balances power and safety. Read-only actions such as file reads, Grep, Glob, and LSP are allowed by default. Actions that can change state such as Bash commands, file edits, Write, WebFetch, and WebSearch require approval unless pre-approved by a permission rule or permission mode.

Permissions can be defined in three ways:

1. **Configuration files** in `settings.json` at user, project, local, or managed scope.
2. **CLI flags** passed at startup such as `--permission-mode`, `--allowedTools`, and `--disallowedTools`.
3. **In-session controls** such as `/permissions`, `/config`, and the Shift+Tab mode selector.

The permission system evaluates rules in this order: deny rules first, then ask rules, then allow rules, and finally the active permission mode. A matching deny rule always wins over an allow rule.

### Permission modes

Claude Code supports six permission modes. The mode acts as a baseline; allow/ask/deny rules can refine it.

| Mode | What runs without asking | Best for |
| :----- | :----- | :----- |
| `default` | Read-only tools only | Getting started, sensitive work |
| `acceptEdits` | Reads, file edits, and common filesystem commands (`mkdir`, `touch`, `mv`, `cp`, `rm`, `rmdir`, `sed`) in the working directory | Iterating on code you review after the fact |
| `plan` | Read-only tools only; file edits never auto-approve | Exploring before changing code |
| `auto` | Everything, routed through a background safety classifier | Long tasks with fewer prompts |
| `dontAsk` | Only pre-approved tools; everything else is denied | Locked-down CI and scripts |
| `bypassPermissions` | Everything (except explicit ask rules and root/home removal circuit breakers) | Isolated containers and VMs only |

### Permission rule syntax

Permission rules follow the form `Tool` or `Tool(specifier)`.

| Rule | Effect |
| :----- | :----- |
| `Bash` | Matches all Bash commands |
| `Bash(npm run *)` | Matches commands starting with `npm run ` |
| `Read(./.env)` | Matches reading `.env` in the current directory |
| `Edit(/src/**/*.ts)` | Matches edits under `<repo>/src/` |
| `WebFetch(domain:example.com)` | Matches fetches to `example.com` |
| `Agent(Explore)` | Matches the Explore subagent |
| `mcp__puppeteer__*` | Matches every tool from the `puppeteer` MCP server |

Rules live in the `permissions` object of `settings.json` under `allow`, `ask`, and `deny` arrays. Deny rules can use bare tool names to remove the tool from Claude's context, or scoped rules to block matching calls.

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--permission-mode <mode>` sets the session's permission mode.
- `--allowedTools <rules>` adds allow rules.
- `--disallowedTools <rules>` adds deny rules.
- `--dangerously-skip-permissions` and `--allow-dangerously-skip-permissions` control bypassPermissions mode.
- `--add-dir`, `--tools`, `--mcp-config`, `--strict-mcp-config`, and `--permission-prompt-tool` adjust scope, tool availability, MCP loading, and programmatic approval.

Precedence is documented in the frontmatter. The key points are:

- Managed settings cannot be overridden by any other source.
- CLI flags are temporary session overrides.
- Local project settings override project and user settings.
- Project settings override user settings.
- For permission rules specifically, a deny rule from any scope blocks the tool even if another scope allows it.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Claude Code starts in `default` mode with the posture described in the frontmatter: read-only tools are free, and state-changing tools prompt on first use.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for paths in the working directory and additional directories.
- `can_write(path)` → Ask for paths in the working directory; Deny for paths outside it until approved.
- `can_execute(command)` → Ask for Bash and PowerShell commands.
- `can_access_domain(domain)` → Ask for WebFetch/WebSearch.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask until approved or denied.
- `can_spawn_subagent(agent)` → Allow to spawn, but the subagent's own tool calls are checked independently.

This use case is ergonomic in PolicyEngine because the engine already models read, write, execute, network, MCP, and agent axes. No changes are required for PolicyEngine to describe it. The main limitation is that PolicyEngine returns static snapshots; the interactive approval prompt itself is a runtime UI concern, not a policy fact.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, use `dontAsk` mode combined with `permissions.allow` rules.

In `settings.json`:

```json
{
  "permissions": {
    "defaultMode": "dontAsk",
    "allow": ["Read", "Grep", "Glob"]
  }
}
```

With this configuration, only Read, Grep, and Glob run without a prompt. Bash, Edit, Write, WebFetch, and every other tool are denied unless you add them to `allow` or pass `--allowedTools` at startup.

CLI examples:

```bash
# Run tests in a locked-down CI invocation
claude -p --permission-mode dontAsk --allowedTools "Bash(npm test),Read" "run the test suite"

# Allow only read-only tools for a codebase exploration
claude --permission-mode dontAsk --allowedTools "Read,Grep,Glob" "explain the auth module"

# Add a temporary domain allowlist for one session
claude --permission-mode dontAsk \
  --allowedTools "Read,Grep,WebFetch(domain:docs.rs)" "research Rust docs"
```

In interactive sessions, you can still use `/permissions` to add allow rules on the fly, but `dontAsk` prevents prompts; it denies anything not pre-approved.

PolicyEngine can describe this use case by setting `SetApprovalMode(dontAsk)` and adding allow rules for the approved tool surface. It is ergonomic and provides coverage for the deterministic part of the policy. The gap is that PolicyEngine cannot force an interactive user to be asked; it can only report that the effective policy would deny the call. The actual ask-or-deny behavior is a runtime decision made by Claude Code's UI layer.

### YOLO

In Claude Code, YOLO mode is called `bypassPermissions`. A session can be put into this mode by:

- Starting with `--permission-mode bypassPermissions`.
- Starting with `--dangerously-skip-permissions` (equivalent to the above).
- Starting with `--allow-dangerously-skip-permissions`, which adds `bypassPermissions` to the interactive Shift+Tab cycle without activating it immediately.
- Setting `permissions.defaultMode` to `bypassPermissions` in a settings file.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags or when the default mode is set to `bypassPermissions`.
- **Non-interactive sessions**: yes, `claude -p --dangerously-skip-permissions` works.
- **Root/sudo on macOS and Linux**: no. Claude Code refuses to start in `bypassPermissions` mode as root or under sudo, with the error `--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons`. The check is skipped inside a recognized sandbox or dev container.

When in `bypassPermissions` mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits, Bash commands, WebFetch, WebSearch, MCP tool calls, and subagent spawns.
- **Still prompted**: explicit `ask` rules in configuration still force a prompt; removals targeting the filesystem root or home directory (`rm -rf /`, `rm -rf ~`) still prompt as a circuit breaker; writes to protected paths are allowed in v2.1.126+.
- **Not allowed**: it cannot override managed settings that disable the mode via `permissions.disableBypassPermissionsMode`.

### Root User

When Claude Code is started as root or under sudo on macOS or Linux, it behaves differently with regard to `bypassPermissions`:

- `bypassPermissions` mode is refused at startup unless the process is inside a recognized sandbox or dev container.
- Other permission modes (`default`, `acceptEdits`, `plan`, `auto`, `dontAsk`) work normally for root.
- YOLO/bypassPermissions is therefore not available to a root session outside a sandbox.

This is a hardcoded safety check, not a configurable policy rule.

### Configuring the Default

Default permissions are configured through `settings.json` files at three main scopes:

- **User scope**: `~/.claude/settings.json` applies across all projects.
- **Repo/project scope**: `.claude/settings.json` applies to everyone working in the repository and can be checked into version control.
- **Local scope**: `.claude/settings.local.json` applies only to you in this repository and is typically gitignored.

For the schema's `config_files` field, user scope is `~/.claude/settings.json` and repo scope is `.claude/settings.json`. Local overrides live in `.claude/settings.local.json`.

Examples that illustrate the grammar:

```json
// ~/.claude/settings.json — user-wide defaults
{
  "permissions": {
    "defaultMode": "acceptEdits",
    "allow": [
      "Bash(npm run *)",
      "Bash(git status *)",
      "WebFetch(domain:docs.rs)"
    ],
    "deny": [
      "Bash(curl *)",
      "Bash(wget *)",
      "Read(~/.ssh/**)"
    ]
  }
}
```

```json
// .claude/settings.json — repo-shared defaults
{
  "permissions": {
    "defaultMode": "default",
    "allow": [
      "Bash(npm run lint)",
      "Bash(npm run test *)"
    ],
    "deny": [
      "Read(./.env)",
      "Read(./secrets/**)"
    ]
  }
}
```

```json
// .claude/settings.local.json — personal repo overrides
{
  "permissions": {
    "allow": [
      "Bash(docker *)",
      "WebFetch(domain:localhost:*)"
    ]
  }
}
```

The `permissions` object also supports:

- `additionalDirectories` — directories treated like the working directory for read/edit permissions.
- `disableBypassPermissionsMode` — set to `"disable"` to prevent use of `bypassPermissions` mode.
- `disableAutoMode` — set to `"disable"` to prevent use of `auto` mode.

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows, repo denies.**

User `~/.claude/settings.json`:

```json
{
  "permissions": {
    "allow": ["Bash(curl *)"]
  }
}
```

Repo `.claude/settings.json`:

```json
{
  "permissions": {
    "deny": ["Bash(curl *)"]
  }
}
```

Result: `curl` is blocked in the repository because deny rules from any scope override allow rules.

**Example 2: user default mode, CLI override.**

User `~/.claude/settings.json`:

```json
{
  "permissions": {
    "defaultMode": "acceptEdits"
  }
}
```

CLI:

```bash
claude --permission-mode plan
```

Result: the session starts in `plan` mode. CLI flags override settings.

**Example 3: project allowlist, local addition.**

Repo `.claude/settings.json`:

```json
{
  "permissions": {
    "defaultMode": "dontAsk",
    "allow": ["Read", "Grep", "Bash(npm test)"]
  }
}
```

Local `.claude/settings.local.json`:

```json
{
  "permissions": {
    "allow": ["Bash(npm run build)"]
  }
}
```

Result: in this repository, `npm test` and `npm run build` are both allowed, along with Read and Grep, because allow rules merge across scopes.

## Tools and Permissions

Claude Code provides the following built-in tools. The "Permission Required" column indicates whether the tool prompts by default in `default` mode.

| Tool | Permission Required | Notes |
| :----- | :----- | :----- |
| `Agent` | No | Spawns subagents; subagent tool calls are checked independently. |
| `Artifact` | Yes | Publishes shareable artifacts. |
| `AskUserQuestion` | No | Gathers requirements. |
| `Bash` | Yes | Executes shell commands. Read-only commands such as `ls`, `cat`, `git status` run without prompting. |
| `CronCreate` / `CronDelete` / `CronList` | No | Session scheduling. |
| `Edit` | Yes | Targeted file edits. |
| `EnterPlanMode` | No | Switches to plan mode. |
| `EnterWorktree` | No | Creates/switches git worktrees. |
| `ExitPlanMode` | Yes | Presents plan for approval. |
| `ExitWorktree` | No | Returns to original directory. |
| `Glob` / `Grep` / `LSP` | No | File/content search and code intelligence. |
| `ListMcpResourcesTool` / `ReadMcpResourceTool` | No | MCP resource discovery/reading. |
| `Monitor` | Yes | Background watches; Bash rules apply to command sources. |
| `NotebookEdit` | Yes | Jupyter notebook edits. |
| `PowerShell` | Yes | PowerShell commands on Windows. |
| `PushNotification` | No | Desktop/phone notifications. |
| `Read` | No | Reads file contents. |
| `RemoteTrigger` | No | Manages claude.ai Routines. |
| `ReportFindings` | No | Code-review findings. |
| `ScheduleWakeup` | No | Self-paced `/loop` scheduling. |
| `SendMessage` | No | Agent-team/subagent messaging. |
| `SendUserFile` | No | Sends files to your device. |
| `ShareOnboardingGuide` | Yes | Uploads onboarding guide. |
| `Skill` | Yes | Executes a skill. |
| `TaskCreate` / `TaskGet` / `TaskList` / `TaskOutput` / `TaskStop` / `TaskUpdate` | No | Task list management. |
| `TodoWrite` | No | Session checklist (disabled by default in v2.1.142+). |
| `ToolSearch` / `WaitForMcpServers` | No | MCP server discovery/waiting. |
| `WebFetch` / `WebSearch` | Yes | Network requests. |
| `Workflow` | Yes | Dynamic multi-subagent workflows. |
| `Write` | Yes | Creates or overwrites files. |

Permissions map to tool calls via the rule syntax described above. An `Edit(...)` allow rule also grants read access to the same path. Bash permission rules support glob patterns and recognize common read-only commands. Read/Edit rules follow gitignore-style patterns with `//`, `~/`, `/`, and relative anchors. WebFetch rules use `domain:` prefixes.

## MCP and Permissions

MCP servers extend Claude Code with external tools. Once connected, their tools appear as `mcp__<server>__<tool>` and are governed by the same permission system as built-in tools.

Permission rules for MCP:

- `mcp__<server>` matches any tool from that server.
- `mcp__<server>__*` matches every tool from that server using wildcard syntax.
- `mcp__<server>__<tool>` matches a specific tool.
- `mcp__*` as a deny rule removes every MCP tool from Claude's context.

MCP servers can be configured at user scope (`~/.claude.json`), project scope (`.mcp.json`), or local scope (per-project entry in `~/.claude.json`). They can also be loaded via `--mcp-config` or bundled with plugins.

Administrators can make MCP safer through several mechanisms:

- **Managed MCP configuration**: deploy `managed-mcp.json` to define a fixed server set or disable MCP entirely.
- **Allowlists/denylists**: use `allowedMcpServers` and `deniedMcpServers` in managed settings, matching by `serverUrl`, `serverCommand`, or `serverName`.
- **Strict config**: use `--strict-mcp-config` to ignore all MCP configuration except what is passed via `--mcp-config`.
- **Environment scrubbing**: set `CLAUDE_CODE_MCP_ALLOWLIST_ENV=1` to limit the environment passed to stdio MCP servers, and `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` to strip credentials from subprocess environments.
- **Disable claude.ai connectors**: set `disableClaudeAiConnectors` to `true` or `ENABLE_CLAUDEAI_MCP_SERVERS=false`.
- **Tool-level permission rules**: add `deny` rules such as `mcp__filesystem__write_file` or `mcp__github__create_issue` to block specific high-risk operations while keeping the server connected.

When a configured MCP server is blocked by policy, it silently disappears from `/mcp` and `claude mcp list`; users see no warning that policy is the cause. In non-interactive mode with tool search enabled, Claude Code tells Claude that the server's tools are unavailable rather than pretending the server is not configured.
