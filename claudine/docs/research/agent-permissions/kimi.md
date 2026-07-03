---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: yolo
    style: switch
    description: Auto-approve all tool calls for the session. The user remains reachable via AskUserQuestion unless AFK mode is also active. Aliases are --yes, -y, and --auto-approve.
    example: kimi --yolo -p "refactor the auth module"
    example_description: Runs a headless prompt with all approval prompts auto-approved.
  - param: afk
    style: switch
    description: Away-from-keyboard mode. Auto-approves all tool calls and auto-dismisses AskUserQuestion so the agent can run unattended.
    example: kimi --afk -p "run the full test suite"
    example_description: Runs non-interactively without stopping for approvals or clarifying questions.
  - param: auto
    style: switch
    description: Start the session in auto permission mode. Documented only briefly in the current CLI help; the exact classifier behavior is not covered in the public docs.
    example: kimi --auto -p "explore the codebase"
    example_description: Starts a non-interactive session in auto permission mode.
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
    example: "kimi --mcp-config '{\"mcpServers\": {\"test\": {\"url\": \"https://...\"}}}'"
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

env_vars:
  - name: KIMI_SHARE_DIR
    effect: "Overrides the default share directory path (default: ~/.kimi). Because config.toml, mcp.json, sessions, and hooks all live under this directory, changing it indirectly changes which permission configuration is loaded. No other environment variable directly sets a Kimi Code CLI permission mode or rule."

config_files:
  - os: all
    user: ~/.kimi/config.toml
    repo: ""
    notes: "No repo-scoped permission configuration file is loaded by default. KIMI_SHARE_DIR overrides the ~/.kimi base path. The config file can set default_yolo, default_plan_mode, and hooks. MCP servers are configured separately in ~/.kimi/mcp.json."

precedence:
  - source: environment variables
    scope: [config_location, provider, model]
    merge_strategy: none
    notes: "KIMI_SHARE_DIR changes where config.toml is loaded from. KIMI_* and OPENAI_* variables override provider/model fields. No env var directly sets permission modes, so this tier does not override CLI --yolo/--plan/--afk/--auto."
  - source: cli
    scope: [approval_mode, tool_visibility, mcp_loading]
    merge_strategy: none
    notes: "CLI flags are temporary session overrides. --yolo, --afk, --plan, and --auto select the session mode; --agent/--agent-file restrict the visible tool set; --mcp-config-file/--mcp-config load ad hoc MCP servers."
  - source: user_config
    scope: [approval_mode, hooks]
    merge_strategy: none
    notes: "~/.kimi/config.toml supplies default_yolo, default_plan_mode, and hooks. It is the only persisted permission-related config scope; there is no repo-scoped file."

default_posture: "When nothing is configured, Kimi Code CLI starts in interactive default mode: read-only tools (Glob, Grep, ReadFile, ReadMediaFile, TaskList, TaskOutput, SearchWeb, FetchURL, etc.) run without approval, while Shell, WriteFile, StrReplaceFile, TaskStop, ExitPlanMode, and MCP tool calls prompt for confirmation on each use."

cli_zero_permissions:
  supported: false
  invocation: "kimi --plan"
  mechanism: "Plan mode restricts the agent to read-only tools (Glob, Grep, ReadFile, etc.). A custom agent file can exclude additional tools, but there is no native flag to start with zero tools or a deny-all policy."
  limitations: "Read-only tools remain available in plan mode and cannot be disabled by CLI alone. There is no --tools '' or equivalent deny-all flag. The closest session-scoped lockdown combines --plan with --agent-file, but the agent file must be authored separately."

agent_permissions:
  allowed: true
  fm_properties:
    - tools
    - exclude_tools
    - subagents

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--yolo (and aliases --yes/-y/--auto-approve), the /yolo slash command, default_yolo in ~/.kimi/config.toml. --afk, --print, and --quiet also auto-approve tool calls, with --afk additionally dismissing AskUserQuestion."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Kimi Code CLI has no allow/ask/deny rule syntax; PolicyEngine's canonical rule model has no native rules to map to."
    - "There is no per-tool, per-path, or per-domain allow/deny configuration, only session-wide default/plan/yolo/afk/auto modes."
    - "Network access (SearchWeb, FetchURL) is gated by service configuration rather than permission rules."
    - "MCP server or tool allow/deny lists do not exist; all MCP tools share the same approval path."
    - "Subagent policy is expressed through tool lists in agent YAML, not through approval modes scoped to a subagent."
    - "PreToolUse hooks are external shell commands and therefore runtime policy outside PolicyEngine's static model."
    - "Only one user-scoped config file exists; there is no repo-scoped or local config precedence to model."
    - "Approve for this session is a runtime state mutation, not a persisted policy rule."
    - "The --auto permission mode is exposed in the CLI but not documented in enough detail to model accurately."

permission_entities:
  - entity: tool
    native_names: ["default agent tools", "okabe agent tools", "tools", "exclude_tools"]
    notes: "Permission is decided at the tool-name level by the active session mode. Individual tools can be hidden via --agent/--agent-file, but not allowed or denied statically."
  - entity: tool_group
    native_names: []
    notes: "No native tool-group permission entity exists. The default and okabe agents have fixed tool sets, and custom agent files list individual tools."
  - entity: command
    native_names: ["Shell"]
    notes: "All Shell commands are treated uniformly: they prompt in default/plan mode and are auto-approved in yolo/afk/print mode. There is no Bash(rm *)-style scoped command rule."
  - entity: path
    native_names: ["workspace", "additional_dirs", "sensitive file filter"]
    notes: "ReadFile and Grep reject sensitive files such as .env, SSH private keys, and cloud credentials. WriteFile and StrReplaceFile require absolute paths outside the working directory/additional directories."
  - entity: workspace
    native_names: ["--work-dir", "--add-dir", "/add-dir"]
    notes: "The working directory and added directories define the scope where relative paths resolve and where writes are allowed without absolute paths."
  - entity: mcp_server
    native_names: ["~/.kimi/mcp.json", "--mcp-config-file", "--mcp-config", "kimi mcp add"]
    notes: "MCP servers are loaded globally for the session. There is no allow/deny list; safety depends on which servers are configured."
  - entity: mcp_tool
    native_names: []
    notes: "MCP tools follow the same session-mode approval as built-in tools. There is no per-tool MCP filter except hiding the entire server by not loading it."
  - entity: mcp_resource
    native_names: []
    notes: "Resource access is governed by the connected MCP server and its approval path; no separate resource-level permission model was found."
  - entity: agent
    native_names: ["Agent", "coder", "explore", "plan"]
    notes: "The Agent tool spawns subagents. Subagent types are defined in agent YAML and have their own tool lists."
  - entity: subagent
    native_names: ["subagents", "tools", "exclude_tools"]
    notes: "Subagents inherit the session mode but can have a narrower tool set defined in their agent YAML."
  - entity: mode
    native_names: ["default", "plan", "yolo", "afk", "auto"]
    notes: "The session mode is the coarse permission control. default and plan prompt for state-changing tools; yolo and afk auto-approve; auto mode is available but not fully documented."
  - entity: approval_category
    native_names: []
    notes: "No approval categories exist. Every state-changing tool is prompted individually in default/plan mode."
  - entity: sandbox
    native_names: []
    notes: "No OS-enforced sandbox is provided. Shell commands run in a subprocess using the user's configured shell."
  - entity: hook
    native_names: ["PreToolUse", "PostToolUse", "SessionStart", "SessionEnd", "Stop", "StopFailure", "SubagentStart", "SubagentStop", "PreCompact", "PostCompact", "UserPromptSubmit", "Notification", "PostToolUseFailure"]
    notes: "PreToolUse hooks can block a tool call by exiting with code 2 or returning JSON with permissionDecision deny. Hooks are Beta and fail-open on timeout or crash."
  - entity: extension
    native_names: ["plugins"]
    notes: "Plugins (Beta) can add custom tools. They are installed via kimi plugin install and loaded for the session."
  - entity: slash_command
    native_names: ["/yolo", "/afk", "/plan"]
    notes: "Slash commands toggle yolo, afk, and plan modes at runtime."
  - entity: unknown
    native_names: []
    notes: "The --auto flag is exposed in current CLI help but its classifier semantics are not documented in the public docs."

approval_modes:
  - name: default
    effect: "Read-only tools run without approval; state-changing tools prompt on each use."
    interactive: true
    non_interactive: true
    aliases: ["default"]
  - name: plan
    effect: "Only read-only tools are allowed. The agent writes a plan and submits it for approval before execution."
    interactive: true
    non_interactive: true
    aliases: ["--plan", "/plan", "default_plan_mode"]
  - name: yolo
    effect: "All tool calls are auto-approved. AskUserQuestion still reaches the user unless AFK is also active."
    interactive: true
    non_interactive: true
    aliases: ["--yolo", "--yes", "-y", "--auto-approve", "/yolo", "default_yolo"]
  - name: afk
    effect: "All tool calls are auto-approved and AskUserQuestion is auto-dismissed, so the agent runs unattended."
    interactive: true
    non_interactive: true
    aliases: ["--afk", "/afk", "--print", "--quiet"]
  - name: auto
    effect: "Start in auto permission mode. Exact behavior is not documented in the public docs beyond the CLI help string."
    interactive: true
    non_interactive: true
    aliases: ["--auto"]

rule_model:
  decisions: ["allow", "deny"]
  syntax: "No native rule syntax exists. PreToolUse hooks receive JSON on stdin (tool_name, tool_input, etc.) and can block via exit code 2 or JSON { hookSpecificOutput: { permissionDecision: deny, permissionDecisionReason: ... } }."
  precedence: "No static rule precedence. Session mode decides the baseline; hooks run before the tool executes and can block."
  merge_semantics: "Only one persisted config file scope exists (~/.kimi/config.toml). CLI flags override config values for the session. Hooks are appended in config order and run in parallel per event."
  matcher_semantics: "Hooks match by regex on the tool name via the matcher field. There are no glob, prefix, or path-pattern matchers for permissions."
  default_decision: "In default and plan modes, read-only tools are allowed and everything else asks. In yolo/afk/print, everything is auto-approved."

tool_visibility:
  supported: true
  mechanisms:
    - "--agent selects the default or okabe built-in agent, each with a fixed tool list."
    - "--agent-file loads a custom YAML agent definition with tools and exclude_tools."
    - "Subagent YAML files can restrict the tool surface for that subagent."
  notes: "Tool visibility is independent of approval. A hidden tool is removed from the model context; visible tools still follow the active session mode."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: "None. Shell commands run as the user process with the user's filesystem permissions."
  network_control: "None. SearchWeb/FetchURL use configured services or local HTTP; Shell can make arbitrary network calls."
  notes: "Kimi Code CLI does not provide an OS-enforced sandbox. The only isolation is the workspace/additional-directory scope enforced by the file tools themselves."

trust_and_admin:
  folder_trust: "No folder or project trust dialog was observed. AGENTS.md files are discovered and merged hierarchically from the git project root to the working directory, but this is not gated by an explicit trust prompt."
  managed_policy: "No managed or admin policy layer was found in the current docs or config. There are no MDM, registry, or server-managed settings for permissions."
  safe_mode: "No safe mode or bare mode exists. The closest equivalent is to use --agent-file to narrow the tool set and --plan to restrict to read-only tools."
  notes: "Trust decisions reduce to which MCP servers and plugins the user has configured and whether the session is launched with an auto-approve mode."

mcp_permissions:
  supported: true
  server_filters:
    - "Load only configured servers from ~/.kimi/mcp.json."
    - "Use --mcp-config-file or --mcp-config to add servers for one session only."
    - "Remove servers with kimi mcp remove."
  tool_filters:
    - "No native per-tool MCP allow/deny list."
    - "PreToolUse hooks can inspect tool_name and block matching MCP calls."
  trust_model: "Servers are trusted by being present in ~/.kimi/mcp.json or passed via CLI. OAuth servers require authorization via kimi mcp auth and store tokens in ~/.kimi/mcp-oauth/."
  notes: "MCP tools follow the same session-mode approval as built-in tools. They run outside any OS sandbox. There is no response interception or sanitization layer beyond marking tool output in the prompt context."

headless_behavior: "In non-interactive --print/--quiet mode, AFK behavior is implied: tool calls are auto-approved and AskUserQuestion is auto-dismissed. --yolo can also be used with -p. Plan-mode transitions and questions auto-resolve without a user. There is no programmatic approval channel except PreToolUse hooks, which can still block. If a tool that requires approval is called without an auto-approve mode, the non-interactive run cannot prompt and will fail."

approval_persistence: "Allow for this session decisions are stored in the session state file (~/.kimi/sessions/<work-dir-hash>/<session-id>/state.json) and restored when the session is resumed. default_yolo and default_plan_mode persist in ~/.kimi/config.toml across sessions. Hooks persist in config.toml."

protected_paths:
  - ".env (and .env.local, .env.production, etc.; .env.example/.env.sample/.env.template are exempted)"
  - "SSH private keys (id_rsa, id_ed25519, id_ecdsa)"
  - "~/.aws/credentials"
  - "~/.gcp/credentials"

security_posture: "Kimi Code CLI's permission system is a client-side static policy engine with advisory prompts, not an OS-enforced sandbox. Session modes choose between prompting and auto-approval; PreToolUse hooks add a fail-open runtime guard; sensitive file filters provide hard-coded read protection. Defense-in-depth must come from the operating environment, not from Kimi Code CLI alone."

changes:
  - "Added the --auto permission-mode flag, which is exposed in current CLI help but not yet documented in detail."
  - "Corrected config precedence: environment variables override CLI flags, and CLI flags override ~/.kimi/config.toml (previously documented as CLI > config only)."
  - "Updated the YOLO/AFK split: --yolo now only bypasses approvals while keeping AskUserQuestion reachable; --afk/--print auto-dismiss AskUserQuestion. --print no longer uses yolo behavior."
  - "Added skip_afk_prompt_injection config key and noted that skip_yolo_prompt_injection is ignored."
  - "Documented the 13 Beta hook events and the fail-open PreToolUse blocking protocol (exit code 2 and JSON permissionDecision)."
  - "Confirmed there is no repo-scoped permission config file; configuration is user-scoped only, with KIMI_SHARE_DIR as the location override."
  - "Added agent/subagent permission entities and tool visibility mechanisms from agent YAML files."
  - "Recorded that no sandbox, folder trust, managed policy, or safe mode exists in the current Kimi Code CLI."
  - "Expanded MCP permission coverage: no tool/server filters, same approval path as built-in tools, OAuth token storage location."
  - "Added non-interactive behavior and approval persistence sections."
  - "Added protected paths list based on the sensitive file filter observed in current docs and source."

requires_claudine_update: true
reason: "Kimi Code CLI's permission surface remains coarse-grained, but the research surfaced a new --auto mode whose semantics are not yet fully documented, a corrected env > CLI > config precedence order, and independent yolo/afk flags. Claudine's PolicyEngine backend for Kimi needs to model default/plan/yolo/afk/auto modes, the hook-based blocking surface, agent-file tool visibility, and the single user-scoped config file, which will require code and metadata updates."
---

# Kimi Code CLI Permissions

## Introduction to Kimi Code CLI Permissions

Kimi Code CLI uses a coarse-grained, session-wide permission model. Instead of per-tool, per-path, or per-domain rules, the CLI chooses between interactive approval and auto-approve modes. Every tool call that can change state either prompts the user once, runs without prompting because the session is in yolo/afk/auto/print mode, or is blocked by a `PreToolUse` hook.

Permissions can be configured through:

1. **Configuration file** — `~/.kimi/config.toml` sets `default_yolo`, `default_plan_mode`, and `hooks`.
2. **CLI flags** — `--yolo`, `--afk`, `--auto`, `--plan`, `--print`, `--agent-file`, `--mcp-config-file`, etc.
3. **In-session controls** — `/yolo`, `/afk`, and `/plan` toggle modes at runtime.
4. **Hooks** — `PreToolUse` hooks can block or allow individual tool calls based on external shell commands.

### Permission modes

| Mode | Behavior | Best for |
| :--- | :--- | :--- |
| `default` | Read-only tools run freely; Shell, WriteFile, StrReplaceFile, TaskStop, ExitPlanMode, and MCP tool calls prompt. | Everyday interactive work. |
| `plan` | Only read-only tools are allowed. The agent writes a plan and submits it for approval before execution. | Designing changes before implementing them. |
| `yolo` | All tool calls are auto-approved. AskUserQuestion still reaches the user. | Trusted, isolated environments where the user is present. |
| `afk` | All tool calls are auto-approved and AskUserQuestion is auto-dismissed. | Unattended or CI-style runs. |
| `auto` | Start in auto permission mode. The public docs do not yet describe the classifier details. | Future auto-approve behavior. |

YOLO and AFK are independent flags. YOLO removes approval friction while keeping the user reachable; AFK means no user is present, so clarifying questions are also dismissed. `--print` and `--quiet` implicitly enable AFK, not YOLO.

### Configuration file fields

The permission-related fields in `~/.kimi/config.toml` are:

| Field | Type | Default | Effect |
| :--- | :--- | :--- | :--- |
| `default_yolo` | boolean | `false` | Start every new session in YOLO mode. |
| `default_plan_mode` | boolean | `false` | Start every new session in plan mode. |
| `skip_afk_prompt_injection` | boolean | `false` | Suppress the AFK-mode system reminder. Replaces the old `skip_yolo_prompt_injection` key, which is now ignored. |
| `hooks` | array | `[]` | Define lifecycle hooks, including `PreToolUse` hooks that can block tool calls. |

### CLI parameters and precedence

The CLI parameters that influence permissions are listed in the frontmatter. The effective precedence for permission-related settings is:

1. **Environment variables** — only `KIMI_SHARE_DIR` and provider/model overrides are relevant; none directly set a permission mode.
2. **CLI flags** — e.g., `--yolo`, `--plan`, `--afk`, `--auto`, `--agent-file`.
3. **Configuration file** (`~/.kimi/config.toml`) — supplies the defaults when no CLI flag is present.

No environment variable that directly changes Kimi Code CLI permission modes was identified in the documentation or source code.

### Permission policy vs tool visibility

Kimi Code CLI separately decides which tools are visible to the model and which visible tools are pre-approved:

- **Approval policy** (session mode) decides whether a visible tool call prompts or runs automatically.
- **Tool visibility** (`--agent`, `--agent-file`, `tools`/`exclude_tools` in agent YAML) decides which tools appear in the model context at all.

For example, `--agent-file ./readonly.yaml` can remove `WriteFile` from the context so the model cannot choose it, while `--yolo` would leave `WriteFile` visible but auto-approve it.

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

- **`default_plan_mode = true` or `--plan`** — restricts the agent to read-only tools at startup.
- **Custom agent file with a minimal `tools` list** — prevents the model from using removed tools, but the remaining tools still follow their normal approval rules.
- **`PreToolUse` hooks** — can block specific tool names or arguments, but hooks fail-open (a crashed hook allows the action).

Because Kimi lacks allow/deny rules, PolicyEngine cannot describe a whitelist for it without extending the engine. A `SetApprovalMode(dontAsk)` plus `allow` rules would not map to any native Kimi config, and there is no `deny` mechanism to fall back to.

The best CLI-only, session-scoped lockdown is `kimi --plan`, which confines the agent to read-only tools. To narrow further, combine it with `--agent-file ./readonly.yaml` that excludes remaining state-changing tools.

### YOLO

A Kimi Code CLI session can be put into YOLO mode in several ways:

- Start with `--yolo`, `--yes`, `-y`, or `--auto-approve`.
- Start with `--afk` (AFK implies auto-approve).
- Start with `--print` or `--quiet` (print mode implies AFK, which auto-approves).
- Toggle `/yolo` during an interactive session.
- Set `default_yolo = true` in `~/.kimi/config.toml`.

Availability:

- **Interactive sessions**: yes, via `--yolo` at startup or `/yolo` at runtime.
- **Non-interactive sessions**: yes, via `--yolo`, `--afk`, or `--print`/`--quiet` combined with `-p`.

When in YOLO mode (without AFK):

- **Allowed**: Shell commands, file writes/edits, MCP tool calls, task stops, and plan-mode transitions are auto-approved.
- **Still gated**: ReadFile and Grep still reject sensitive files such as `.env`, SSH private keys, and cloud credentials. `PreToolUse` hooks can still block actions.
- **Not allowed**: YOLO cannot bypass missing API keys, quota limits, invalid configuration, or hook blocks that return exit code `2`.

When AFK is also active, AskUserQuestion is auto-dismissed.

### Root User

Kimi Code CLI does **not** appear to treat root users differently from regular users. There are no documented root/sudo blocks for YOLO or AFK mode, and no source-code checks that disable auto-approve based on the effective UID. YOLO and AFK remain allowed when running as root.

### Configuring the Default

Default permissions are configured in a single user-scoped file:

- **User scope**: `~/.kimi/config.toml`
- **Repo scope**: none. Kimi Code CLI does not load a project-scoped permission configuration file by default.

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

When a user chooses **Allow for this session** in the approval panel, the decision is saved in the session's `state.json` and restored when the session is resumed.

### Native permission entities and rule grammar

Kimi Code CLI does not have a native allow/ask/deny rule grammar. The security controls are:

- **Session mode** (`default`, `plan`, `yolo`, `afk`, `auto`) — coarse baseline.
- **Tool visibility** (`--agent`, `--agent-file`, agent YAML `tools`/`exclude_tools`) — hides tools from the model.
- **Hooks** (`PreToolUse`) — regex-matched shell commands that can block calls.
- **Sensitive file filters** — hard-coded rejections in `ReadFile` and `Grep`.

Hook decisions are fail-open: a timeout, crash, or other non-zero/non-two exit code allows the action. A structured JSON output with `permissionDecision: deny` can also block.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Kimi Code CLI does **not** provide a separate sandbox mode. Shell commands run in a subprocess using the user's configured shell (bash on Unix-like systems, Git Bash `bash.exe` on Windows). There is no OS-level filesystem or network isolation. The workspace scope is enforced by the file tools, not by the operating system.

### Trust and administrative controls

- **Folder/project trust**: no explicit trust dialog was found. `AGENTS.md` files are discovered and merged hierarchically from the git project root down to the working directory, but this loading is not gated by a user trust decision.
- **Managed/admin policy**: no managed settings, MDM, registry, or server-managed policy layer exists in the current docs or config.
- **Safe mode**: no safe mode or bare mode exists. The closest session-scoped lockdown is `--plan` combined with a restrictive `--agent-file`.

### Protected paths

The tool layer protects a small set of sensitive paths even under permissive modes:

- `.env` files (`.env.example`, `.env.sample`, and `.env.template` are exempted).
- SSH private keys (`id_rsa`, `id_ed25519`, `id_ecdsa`).
- Cloud credential files (`~/.aws/credentials`, `~/.gcp/credentials`).

`ReadFile` rejects these outright; `Grep` filters them from results with a warning.

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

## Non-Interactive Behavior

In non-interactive `--print`/`--quiet` mode, AFK behavior is implied: tool calls are auto-approved and `AskUserQuestion` is auto-dismissed. You can also pass `--yolo` or `--afk` explicitly with `-p`.

Because interactive approval prompts cannot be displayed in headless mode:

- If a state-changing tool is called without an auto-approve mode, the run cannot prompt and will fail.
- `EnterPlanMode`, `ExitPlanMode`, and `AskUserQuestion` auto-resolve rather than hanging.
- There is no programmatic approval channel other than `PreToolUse` hooks, which can still block calls.

## Sources

- [Kimi Code CLI Docs](https://moonshotai.github.io/kimi-cli/)
- [`kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Slash Commands Reference](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html)
- [Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Config Overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.html)
- [Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Model Context Protocol](https://moonshotai.github.io/kimi-cli/en/customization/mcp.html)
- [`kimi mcp` Subcommand](https://moonshotai.github.io/kimi-cli/en/reference/kimi-mcp.html)
- [Hooks (Beta)](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Print Mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Interaction and Input](https://moonshotai.github.io/kimi-cli/en/guides/interaction.html)
- [Changelog](https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.html)
- [GitHub Repository](https://github.com/MoonshotAI/kimi-cli)

## Changelog

- 2026-07-02: Refreshed research against current Kimi Code CLI documentation and the installed `kimi` binary. Added the `--auto` flag, corrected config precedence to env > CLI > config, documented the yolo/afk split, expanded the Beta hooks section, added agent/subagent tool-visibility coverage, and added the new schema-required sections (sandboxing, trust/admin, MCP permissions, non-interactive behavior, protected paths, approval persistence, security posture, and changelog). Flagged Claudine updates as required.
