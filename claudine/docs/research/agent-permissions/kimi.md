---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: yolo
    style: switch
    description: "Auto-approve regular tool actions for the session. Hidden aliases are --yes, -y, and --auto-approve. The current kimi-cli docs/source keep AskUserQuestion reachable in yolo mode."
    example: "kimi --yolo"
    example_description: "Starts an interactive session where regular tool approvals are skipped."
  - param: afk
    style: switch
    description: "Away-from-keyboard mode. Auto-approves tool calls and auto-dismisses AskUserQuestion because no user is expected to be present."
    example: "kimi --afk"
    example_description: "Starts an unattended interactive shell session."
  - param: plan
    style: switch
    description: "Starts or resumes the session in plan mode. Plan mode adds workflow/tool guidance and plan-file handling; in current source it is not an OS sandbox and Shell is not hard-blocked."
    example: "kimi --plan"
    example_description: "Starts in planning posture before implementation."
  - param: print
    style: switch
    description: "Runs the print UI, which is non-interactive and applies an invocation-only AFK overlay so approvals and questions do not block."
    example: "kimi --print --prompt \"summarize the repo\""
    example_description: "Runs one prompt non-interactively with runtime AFK behavior."
  - param: prompt
    style: switch
    description: "Provides a prompt from the CLI using --prompt, -p, --command, or -c. With --print it creates a headless one-shot run; without --print it pre-fills/starts shell UI behavior."
    example: "kimi --print -p \"inspect the failing test\""
    example_description: "Runs a headless prompt."
  - param: input-format
    style: switch
    description: "Print-mode input format, text or stream-json. Only valid with --print."
    example: "cat prompt.txt | kimi --print --input-format text"
    example_description: "Reads non-interactive prompt input from stdin."
  - param: output-format
    style: switch
    description: "Print-mode output format, text or stream-json. Only valid with --print."
    example: "kimi --print --output-format stream-json -p \"run diagnostics\""
    example_description: "Emits JSONL stream output for a headless run."
  - param: final-message-only
    style: switch
    description: "Print UI option that suppresses intermediate output and prints only the final assistant message."
    example: "kimi --print --final-message-only -p \"write a summary\""
    example_description: "Returns only the final text from a non-interactive run."
  - param: quiet
    style: switch
    description: "Alias for --print --output-format text --final-message-only. It inherits print mode's runtime AFK behavior."
    example: "kimi --quiet -p \"draft a commit message\""
    example_description: "Runs a quiet non-interactive prompt."
  - param: config-file
    style: switch
    description: "Loads a TOML or JSON configuration file instead of the default ~/.kimi/config.toml. Can set default_yolo, default_plan_mode, hooks, MCP client timeout, providers, services, and skill behavior."
    example: "kimi --config-file ./kimi.lockdown.toml"
    example_description: "Starts with an alternate session-scoped config file."
  - param: config
    style: switch
    description: "Loads inline TOML or JSON config content. Mutually exclusive with --config-file and useful for temporary policy overlays."
    example: "kimi --config 'default_yolo = false\ndefault_plan_mode = true'"
    example_description: "Starts one run with plan mode enabled without editing user config."
  - param: work-dir
    style: switch
    description: "Sets the working directory for the session. This changes the default filesystem context and where session history is grouped."
    example: "kimi --work-dir /path/to/repo"
    example_description: "Runs with a specific project root."
  - param: add-dir
    style: switch
    description: "Adds an additional readable workspace directory to session state. Can be repeated; directories under the work dir are skipped because they are already in scope."
    example: "kimi --add-dir ../shared --add-dir ../docs"
    example_description: "Expands the workspace roots for this session."
  - param: agent
    style: switch
    description: "Selects the built-in default or okabe agent. Agent selection changes the visible built-in tool set."
    example: "kimi --agent okabe"
    example_description: "Uses the okabe agent, which includes the experimental SendDMail tool."
  - param: agent-file
    style: switch
    description: "Loads a custom agent YAML file. The agent file defines visible tools and subagent definitions."
    example: "kimi --agent-file ./agents/readonly.yaml"
    example_description: "Starts with a custom tool surface."
  - param: mcp-config-file
    style: switch
    description: "Loads one or more MCP config JSON files for this invocation. If no MCP config is supplied, the default ~/.kimi/mcp.json is loaded when it exists."
    example: "kimi --mcp-config-file ./mcp.safe.json"
    example_description: "Adds only the MCP servers declared in the provided file plus any default behavior described by the CLI."
  - param: mcp-config
    style: switch
    description: "Loads one or more inline MCP config JSON strings."
    example: "kimi --mcp-config '{\"mcpServers\":{\"docs\":{\"url\":\"https://example.test/mcp\"}}}'"
    example_description: "Adds an ad hoc MCP server without mutating ~/.kimi/mcp.json."
  - param: skills-dir
    style: switch
    description: "Overrides default skill discovery with one or more custom skills directories. Skills can influence model behavior and workflow but are not approval rules."
    example: "kimi --skills-dir ./skills"
    example_description: "Restricts auto-discovered skills to an explicit directory set."
  - param: acp
    style: switch
    description: "Deprecated top-level flag for ACP server mode; current docs prefer the kimi acp subcommand. ACP clients receive programmatic permission requests."
    example: "kimi --acp"
    example_description: "Runs Kimi as an ACP server for an editor/client."
  - param: wire
    style: switch
    description: "Runs the experimental Wire server. Wire hook subscriptions can participate in hook decisions, including PreToolUse blocking."
    example: "kimi --wire"
    example_description: "Runs the experimental structured protocol server."

env_vars:
  - name: KIMI_SHARE_DIR
    effect: "Changes the runtime data directory from ~/.kimi to the provided path; config.toml, mcp.json, sessions, credentials, logs, and plan files move with it. This indirectly changes which permission defaults, hooks, MCP servers, and session approvals are loaded."
    effect_category: state_home_relocation
  - name: KIMI_BASE_URL
    effect: "Overrides the configured base_url for kimi-type providers. It does not grant or deny tools, but it can redirect model traffic."
    effect_category: none
  - name: KIMI_API_KEY
    effect: "Overrides the configured API key for kimi-type providers. It does not grant tool permissions, but it can enable provider access in CI or wrappers."
    effect_category: credential
  - name: KIMI_MODEL_NAME
    effect: "Overrides the provider model identifier. No direct permission effect."
    effect_category: none
  - name: KIMI_MODEL_MAX_CONTEXT_SIZE
    effect: "Overrides the model context size. No direct permission effect."
    effect_category: none
  - name: KIMI_MODEL_CAPABILITIES
    effect: "Overrides model capabilities such as thinking, image_in, or video_in. This can affect whether media-reading features are usable but is not an approval rule."
    effect_category: none
  - name: KIMI_MODEL_TEMPERATURE
    effect: "Generation parameter override. No direct permission effect."
    effect_category: none
  - name: KIMI_MODEL_TOP_P
    effect: "Generation parameter override. No direct permission effect."
    effect_category: none
  - name: KIMI_MODEL_MAX_TOKENS
    effect: "Generation parameter override. No direct permission effect."
    effect_category: none
  - name: KIMI_MODEL_THINKING_KEEP
    effect: "Controls Moonshot preserved-thinking request behavior when thinking is enabled. No direct tool permission effect."
    effect_category: none
  - name: OPENAI_BASE_URL
    effect: "Overrides base_url for OpenAI-compatible providers. No direct tool permission effect."
    effect_category: none
  - name: OPENAI_API_KEY
    effect: "Overrides API key for OpenAI-compatible providers. No direct tool permission effect."
    effect_category: credential
  - name: KIMI_CLI_GIT_BASH_PATH
    effect: "On Windows, points Shell execution at a specific Git Bash bash.exe. This changes the command execution backend but not the approval policy."
    effect_category: none

config_files:
  - os: macos
    user: ".kimi/config.toml"
    repo: ""
    notes: "Relative to the user's home directory unless KIMI_SHARE_DIR is set. MCP servers live in .kimi/mcp.json; session approvals live under .kimi/sessions/<work-dir-hash>/<session-id>/state.json. No repo-scoped permission config file is loaded by default."
  - os: linux
    user: ".kimi/config.toml"
    repo: ""
    notes: "Same layout as macOS. KIMI_SHARE_DIR relocates the whole runtime data directory. No repo-scoped permission config file is loaded by default."
  - os: windows
    user: ".kimi/config.toml"
    repo: ""
    notes: "Relative to the Windows user home directory. Shell uses Git Bash and can be redirected with KIMI_CLI_GIT_BASH_PATH. No repo-scoped permission config file is loaded by default."

precedence:
  - source: runtime_session_state
    scope: [approval_mode, workspace]
    merge_strategy: none
    notes: "When resuming a session, state.json restores yolo, afk, auto_approve_actions, plan_mode, and additional_dirs. CLI --plan and --add-dir can change the resumed state for the session."
  - source: cli
    scope: [approval_mode, tool_visibility, mcp, workspace, other, config_loading]
    merge_strategy: none
    notes: "CLI flags are temporary invocation controls. --config and --config-file replace the default config source. --agent/--agent-file replace the agent tool surface. --mcp-config-file/--mcp-config add invocation MCP configs; ~/.kimi/mcp.json is loaded only when no MCP config file is provided."
  - source: env
    scope: [config_loading, provider_model, security_controls]
    merge_strategy: none
    notes: "KIMI_SHARE_DIR changes where config/session/MCP data is read. Provider/model environment variables override provider/model fields after config loading. No environment variable directly selects yolo, afk, plan, or hook rules."
  - source: user_config
    scope: [approval_mode, hooks, mcp, customization_resources, skills]
    merge_strategy: none
    notes: "~/.kimi/config.toml supplies default_yolo, default_plan_mode, hooks, services, and extra_skill_dirs. It is the only persisted config scope for permission defaults."

default_posture: "With no CLI flags, relevant environment variables, config defaults, or resumed session state, Kimi CLI starts in default interactive mode. Read/search/fetch-style tools are visible and usually run without approval; write/edit/shell/task-stop/plan-exit/MCP actions request approval, and approvals are client-side guardrails rather than an OS sandbox."

cli_zero_permissions:
  supported: false
  invocation: "kimi --config 'default_yolo = false\ndefault_plan_mode = true' --agent-file ./no-tools.yaml"
  mechanism: "The closest posture combines a temporary config that disables yolo and starts plan mode with a custom agent file that removes tools. Kimi CLI has no native --no-tools flag, empty tool allowlist flag, deny-all approval mode, or CLI rule grammar."
  limitations: "This is not complete CLI-only lockdown because Claudine would have to provide a custom agent file. Plan mode is not a sandbox and does not hard-block every state-changing tool in source. Additional permissions cannot be added back as allow rules from the CLI; they can only be made visible through the agent file, MCP config, session mode, or interactive approval."

agent_permissions:
  allowed: true
  fm_properties:
    - tools
    - subagents
    - extend

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--yolo/--yes/-y/--auto-approve, /yolo, and default_yolo in ~/.kimi/config.toml. --afk and --print also auto-approve, but AFK additionally auto-dismisses user questions. Source and docs for the target repo do not expose the successor standalone --auto mode."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Kimi CLI does not have a native static allow/ask/deny rule grammar comparable to Claude permissions."
    - "The important controls are session modes, per-session auto_approve_actions, agent YAML tool visibility, MCP loading, and fail-open hooks; these do not map cleanly to PolicyEngine's current static rule model."
    - "Plan mode is partly prompt/tool workflow state, not a strict permission mode or sandbox."
    - "There is no CLI-only no-tools/deny-all posture for wrappers to compose from."
    - "Hook blocking can inspect arbitrary JSON and shell out, but it is runtime and fail-open."
    - "ACP has a programmatic approval channel with allow-once/allow-always/reject, which PolicyEngine does not currently model as provider-native runtime approval transport."
    - "MCP has server loading controls but no server/tool/resource allow-deny policy."
    - "Hard-coded sensitive-file filters and workspace guidance are tool-layer behavior, not policy rules."

permission_entities:
  - entity: tool
    native_names: ["Agent", "AskUserQuestion", "SetTodoList", "Shell", "TaskList", "TaskOutput", "TaskStop", "ReadFile", "ReadMediaFile", "Glob", "Grep", "WriteFile", "StrReplaceFile", "SearchWeb", "FetchURL", "EnterPlanMode", "ExitPlanMode", "SendDMail"]
    notes: "Approval is requested by tool action names. Tool visibility comes from the selected agent YAML and hidden-tool state."
  - entity: command
    native_names: ["Shell", "Bash"]
    notes: "Shell commands run through the host shell backend; on Windows this is Git Bash. Shell asks unless yolo/afk/print or an approve-for-session action applies."
  - entity: path
    native_names: ["KIMI_WORK_DIR", "additional_dirs", "sensitive file filters"]
    notes: "File tools are oriented around the work dir and session additional directories. The system prompt tells the model not to access outside the work dir unless instructed, but this is advisory."
  - entity: workspace
    native_names: ["--work-dir", "--add-dir", "/add-dir", "additional_dirs"]
    notes: "Workspace roots affect context and relative path handling; additional_dirs persist in session state."
  - entity: mcp_server
    native_names: ["~/.kimi/mcp.json", "--mcp-config-file", "--mcp-config", "kimi mcp add/remove/auth/reset-auth"]
    notes: "Servers are trusted by being configured or passed at launch. No server allow/deny rules were found."
  - entity: mcp_tool
    native_names: ["MCPTool", "server tool names"]
    notes: "MCP tools are added to the toolset after server discovery and follow the same approval runtime as other tools."
  - entity: mcp_resource
    native_names: []
    notes: "No separate resource-level permission model was found."
  - entity: agent
    native_names: ["--agent", "--agent-file", "default", "okabe"]
    notes: "Agent YAML defines the root tool list and available subagents."
  - entity: subagent
    native_names: ["coder", "explore", "plan", "subagents"]
    notes: "Subagents are session objects with their own context; approvals are coordinated through the root approval runtime."
  - entity: mode
    native_names: ["default", "yolo", "afk", "plan", "print"]
    notes: "Modes are coarse runtime state. Yolo and afk affect approval; plan affects workflow; print applies runtime AFK."
  - entity: approval_category
    native_names: ["approve", "approve_for_session", "reject"]
    notes: "Interactive and ACP approval choices are once, for this session/action, or reject."
  - entity: hook
    native_names: ["PreToolUse", "PostToolUse", "PostToolUseFailure", "UserPromptSubmit", "Stop", "StopFailure", "SessionStart", "SessionEnd", "SubagentStart", "SubagentStop", "PreCompact", "PostCompact", "Notification"]
    notes: "Hook matcher is a regex over the event target such as tool name. Any blocking hook result blocks; hook engine errors and timeouts fail open."
  - entity: extension
    native_names: ["plugin", "kimi plugin"]
    notes: "Plugins are Beta and can add behavior/tools; they are adjacent to permission modeling but not an approval rule system."
  - entity: slash_command
    native_names: ["/yolo", "/afk", "/plan", "/add-dir", "/hooks", "/config"]
    notes: "Slash commands mutate runtime/session state in interactive shell mode."
  - entity: sandbox
    native_names: []
    notes: "No OS-enforced sandbox mode exists in the target repo."

approval_modes:
  - name: default
    effect: "Visible read/search/fetch tools can run; state-changing tool actions ask through the approval runtime."
    interactive: true
    non_interactive: false
    aliases: ["default"]
  - name: yolo
    effect: "Auto-approves regular tool actions while keeping AskUserQuestion reachable. Current plan-tool descriptions and tests indicate ExitPlanMode still presents plan approval under yolo."
    interactive: true
    non_interactive: true
    aliases: ["--yolo", "--yes", "-y", "--auto-approve", "/yolo", "default_yolo"]
  - name: afk
    effect: "Auto-approves tool calls and auto-dismisses AskUserQuestion; used when no user is present."
    interactive: true
    non_interactive: true
    aliases: ["--afk", "/afk", "--print runtime overlay"]
  - name: plan
    effect: "Starts or keeps the session in plan workflow state. The model is instructed to plan first; plan file writes are auto-approved; plan exit asks unless AFK applies."
    interactive: true
    non_interactive: false
    aliases: ["--plan", "/plan", "default_plan_mode", "Shift-Tab"]
  - name: print
    effect: "Non-interactive UI mode with invocation-only AFK overlay; approvals and questions auto-resolve instead of prompting."
    interactive: false
    non_interactive: true
    aliases: ["--print", "--quiet"]
  - name: acp
    effect: "Programmatic mode where approval requests are sent to the ACP client as permission requests."
    interactive: false
    non_interactive: true
    aliases: ["kimi acp", "--acp"]

rule_model:
  decisions: ["approve", "approve_for_session", "reject", "allow", "block", "deny"]
  syntax: "There is no static permission rule grammar. Hooks are configured as [[hooks]] with event, matcher regex, command, and timeout; a hook blocks by exit code 2 or JSON stdout hookSpecificOutput.permissionDecision = deny."
  precedence: "Runtime auto-approval (yolo/afk/auto_approve_actions) skips approval prompts, but PreToolUse hooks still run before tool execution. Any hook block wins; hook errors/timeouts fail open. Reject from approval denies that call."
  merge_semantics: "There is one user config file. CLI --config/--config-file replaces the loaded config source for the invocation. MCP inline/file configs are appended to loaded MCP configs; session state persists approval/action and workspace state."
  matcher_semantics: "Hook matcher uses Python regular expressions against the hook target, such as a tool name. No glob/path/command-pattern permission matcher exists."
  default_decision: "Default mode asks when a tool calls Approval.request; yolo/afk/print approve those actions automatically; no matching hook means no hook opinion."

tool_visibility:
  supported: true
  mechanisms:
    - "--agent selects the built-in default or okabe YAML."
    - "--agent-file loads a custom root agent YAML with tools and subagents."
    - "Agent YAML tools lists determine which built-in tools are visible."
    - "The runtime toolset has hidden-tool support for dynamic tool visibility."
    - "--skills-dir restricts skill discovery, which changes behavioral context but not tools directly."
  notes: "Tool visibility is separate from approval. Removing a tool from YAML prevents model selection; leaving it visible means the active approval mode and hooks decide execution."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: "No OS sandbox. File tools enforce their own path handling and sensitive-file filters; Shell runs with the launching user's filesystem permissions."
  network_control: "No network sandbox. SearchWeb/FetchURL use configured services; Shell can run network commands allowed by the host."
  notes: "The system prompt explicitly says the operating environment is not in a sandbox. Windows Shell uses Git Bash, and KIMI_CLI_GIT_BASH_PATH can override the Git Bash executable."

trust_and_admin:
  folder_trust: "No explicit project/folder trust prompt or trusted-workspace database was found for the target repo. AGENTS.md and skills can be loaded from project locations, but this is not guarded by a native trust decision."
  managed_policy: "No managed/admin policy layer, MDM setting, registry policy, or centrally enforced permission policy was found."
  safe_mode: "No safe-mode flag was found. The closest restrictions are --config with safe defaults, --plan, --agent-file, --skills-dir, and not loading untrusted MCP/plugin assets."
  notes: "The successor standalone Kimi Code documentation has a different ~/.kimi-code home and default_permission_mode/rules examples; this document targets the requested kimi-cli repo/site and records that drift in the changelog."

mcp_permissions:
  supported: true
  server_filters:
    - "Use ~/.kimi/mcp.json as the default global server file."
    - "Use --mcp-config-file and --mcp-config for invocation-specific server definitions."
    - "Use kimi mcp add/remove/auth/reset-auth to manage persisted servers and OAuth tokens."
  tool_filters:
    - "No native MCP server/tool allow-deny filter was found."
    - "PreToolUse hooks can match the emitted tool name and block specific MCP calls at runtime."
  trust_model: "A server is trusted by being configured or passed at launch. OAuth tokens live under ~/.kimi/mcp-oauth. MCP tools run as external MCP server/client operations outside any Kimi OS sandbox."
  notes: "MCP tool output receives a larger tool-result budget than built-in tools in source, but no response interception/sanitization permission layer was found."

headless_behavior: "Print mode is the non-interactive path and sets runtime AFK, so approval prompts and AskUserQuestion auto-resolve instead of waiting for a terminal. ACP mode is the exception: it has a programmatic approval channel and forwards permission requests to the client with approve once, approve for this session, and reject options."

approval_persistence: "Approve-for-session decisions are stored as action strings in session state under approval.auto_approve_actions and are restored with that session. yolo, persisted afk, plan_mode, and additional_dirs also persist in state.json; print-mode runtime AFK does not persist."

protected_paths:
  - ".env and similar sensitive dotenv files are filtered by file tools; examples/templates are treated more permissively."
  - "SSH private key names such as id_rsa, id_ed25519, and id_ecdsa are treated as sensitive by the file layer."
  - "Cloud credential files such as ~/.aws/credentials and ~/.gcp/credentials are treated as sensitive by the file layer."
  - "~/.kimi/credentials and ~/.kimi/mcp-oauth contain OAuth credentials/tokens; docs state credential files are written with 0600 permissions."

security_posture: "Kimi CLI combines advisory model instructions, client-side approval prompts, session state, static agent tool visibility, fail-open hook checks, and tool-layer sensitive-file filters. It is not an OS-enforced sandbox or centrally managed policy engine; strong isolation must come from the host OS, container, VM, or wrapper."

changes:
  - "Refreshed the document against the current MoonshotAI/kimi-cli repository HEAD (pyproject 1.48.0), current moonshotai.github.io/kimi-cli docs, and the locally installed standalone Kimi binary."
  - "Fixed schema compliance by splitting config_files into macOS, Linux, and Windows records and setting agent/model/last_updated as requested."
  - "Corrected target-repo CLI flags: current kimi-cli source/docs include --yolo, --afk, --plan, --print, --config, --config-file, --agent, --agent-file, --mcp-config-file, --mcp-config, --skills-dir, --acp, and --wire; --auto appears only in the local successor standalone binary/newer Kimi Code docs, not the requested kimi-cli source."
  - "Updated plan-mode semantics: it is workflow/prompt state, not a strict sandbox; source tests say Shell is not hard-blocked in plan mode, plan-file writes are auto-approved, and ExitPlanMode still asks under yolo but auto-approves under afk."
  - "Added ACP approval behavior: Kimi forwards approval requests to ACP clients with approve once, approve for this session, and reject choices."
  - "Confirmed no local ~/.kimi config files existed to inspect on this host; the default directory existed only after inspection commands."
  - "Reframed PolicyEngine fit around runtime modes, session auto_approve_actions, agent YAML visibility, MCP loading, and fail-open hooks rather than a nonexistent static allow/ask/deny config grammar."
  - "Added successor drift notes for Kimi Code standalone ~/.kimi-code/default_permission_mode/permission.rules so future refreshes do not accidentally mix provider generations."

requires_claudine_update: true
reason: "Claudine needs provider metadata/code changes to model Kimi accurately: no CLI-only zero-permissions baseline, yolo versus afk split, plan-mode limitations, ACP approval transport, session-persisted auto_approve_actions, agent YAML tool visibility, and fail-open hook blocking are not covered by a simple static PolicyEngine rule backend."
---

# Kimi Code CLI Permissions and Security Controls

## Introduction to Kimi Code CLI Permissions

Kimi CLI, as represented by the requested `MoonshotAI/kimi-cli` repository and `moonshotai.github.io/kimi-cli` documentation, uses a coarse runtime approval system rather than a static allow/ask/deny policy file. The main controls are session modes (`default`, `yolo`, `afk`, `plan`, `print`), session approval state, agent YAML tool visibility, MCP server loading, and Beta hooks.

Configuration can define permission-adjacent defaults in `~/.kimi/config.toml`: `default_yolo`, `default_plan_mode`, `hooks`, MCP client timeout, services, providers, and skill discovery. MCP server definitions live separately in `~/.kimi/mcp.json`, and session decisions persist in `~/.kimi/sessions/<work-dir-hash>/<session-id>/state.json`.

No local Kimi config file was available to inspect on this host. `find ~/.kimi` initially returned no files; source inspection confirms `get_share_dir()` creates `~/.kimi` when invoked.

Environment variables do not directly set yolo/afk/plan. `KIMI_SHARE_DIR` changes which config, MCP, session, and credential files are loaded. Provider/model variables such as `KIMI_API_KEY`, `KIMI_BASE_URL`, and `OPENAI_API_KEY` override model/provider connection data but do not grant tool permissions. `KIMI_CLI_GIT_BASH_PATH` affects the Windows Shell backend.

CLI switches have the strongest per-invocation influence over approval mode, config source, tool visibility, MCP loading, workspace roots, and headless behavior. `--config` and `--config-file` replace the default config source for that run. `--agent` and `--agent-file` replace the visible root agent tool surface. `--mcp-config-file` and `--mcp-config` add invocation MCP server definitions; if no MCP config file is passed, the default `~/.kimi/mcp.json` is loaded when present.

Permission/approval policy is distinct from tool visibility:

- **Approval policy** decides whether a visible tool call asks, auto-approves, or rejects at runtime.
- **Tool visibility** decides whether the model can see/select a tool at all, primarily through built-in or custom agent YAML.

For example, `--agent-file ./readonly.yaml` can remove `WriteFile` from the model context, while `--yolo` leaves tools visible but bypasses regular approval prompts.

## Permissions Use Cases

### Default

With no relevant environment variables, config defaults, CLI switches, or resumed session state, Kimi starts in default interactive mode. Read/search/fetch-style tools are visible and generally run without approval. State-changing actions such as Shell, WriteFile, StrReplaceFile, TaskStop, plan exit, and MCP tools request approval through the approval runtime.

A PolicyEngine approximation would be:

- `can_read(path)` -> allow, subject to tool-layer sensitive-file filters.
- `can_write(path)` -> ask.
- `can_execute(command)` -> ask.
- `can_use_mcp_tool(server, tool)` -> ask.
- `can_spawn_subagent(agent)` -> allow, with the subagent's tool calls routed through the shared approval runtime.

This is not ergonomic for PolicyEngine because Kimi does not expose native static rules. PolicyEngine could approximate the posture but would miss session-persisted action approvals, hooks, sensitive-file filters, and the fact that plan mode is not a strict permission boundary.

### Whitelisting

Kimi does not support true whitelisting where the default is "no permissions" and every allowed operation is declared through CLI/config rules. There is no `--no-tools`, empty tool allowlist, or deny-all approval mode.

The closest pieces are:

- `--config 'default_yolo = false\ndefault_plan_mode = true'` to start from non-yolo plan posture.
- `--agent-file ./minimal.yaml` to remove tools from the model context.
- `PreToolUse` hooks to block selected tool names or inputs at runtime.

The best session-scoped wrapper posture is therefore not fully CLI-only unless Claudine supplies a generated temporary agent file:

```bash
kimi --config 'default_yolo = false
default_plan_mode = true' --agent-file ./no-tools.yaml
```

Additional permissions cannot be added back as native rules. Claudine would have to generate a different agent file, add MCP configs, or rely on interactive approvals. PolicyEngine cannot faithfully express this because it has no Kimi backend concept for "visible tool only" plus runtime hook guards.

### YOLO

Ways to enter auto-approval:

- `--yolo`, `--yes`, `-y`, or `--auto-approve`.
- `/yolo` in the interactive shell.
- `default_yolo = true` in `~/.kimi/config.toml`.
- `--afk` or `/afk`, which also auto-dismisses questions.
- `--print` or `--quiet`, which apply runtime AFK for non-interactive execution.

YOLO is available interactively and non-interactively, but `--print` is the intended headless mode. In yolo, regular tool approval prompts are bypassed and AskUserQuestion remains reachable. AFK means no user is expected, so AskUserQuestion auto-dismisses. Current plan-tool source says `ExitPlanMode` still presents plan approval under yolo; AFK auto-approves plan exit.

YOLO does not bypass PreToolUse hook blocks, missing credentials, provider errors, host OS permissions, or tool-layer sensitive-file checks.

### Root User

No source or documentation check was found that disables yolo/afk for root or sudo. Because Kimi has no OS sandbox, running as root increases the blast radius of Shell and file tools. YOLO still appears allowed.

### Configuring the Default

User scope:

```toml
# ~/.kimi/config.toml
default_yolo = false
default_plan_mode = true
skip_afk_prompt_injection = false
```

Repo scope: none for permission config. Project `AGENTS.md` and project skills can influence instructions, but they are not native permission files.

Hook grammar:

```toml
[[hooks]]
event = "PreToolUse"
matcher = "Shell"
command = "./.kimi/hooks/check-shell.sh"
timeout = 10
```

A hook receives JSON on stdin. Exit code `2` blocks. Exit code `0` with JSON output can also block:

```json
{
  "hookSpecificOutput": {
    "permissionDecision": "deny",
    "permissionDecisionReason": "Blocked by policy"
  }
}
```

Timeouts, crashes, invalid regexes, and hook engine errors fail open.

### Extending the Base

Example 1: user config starts yolo, CLI starts plan mode.

```toml
default_yolo = true
```

```bash
kimi --plan
```

Result: the invocation requests plan mode, but yolo may still be restored from config/session state. For strict wrappers, pass a temporary config with `default_yolo = false`.

Example 2: temporary config disables yolo and starts plan mode.

```bash
kimi --config 'default_yolo = false
default_plan_mode = true'
```

Result: no user config mutation, but not a no-tools posture.

Example 3: custom agent file narrows visible tools.

```yaml
version: 1
agent:
  extend: default
  tools:
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:Glob"
    - "kimi_cli.tools.file:Grep"
```

```bash
kimi --agent-file ./readonly.yaml
```

Result: only listed tools are visible from that agent definition; approval mode still applies to visible approval-requesting tools.

Example 4: PreToolUse hook blocks a command family even under yolo.

```toml
[[hooks]]
event = "PreToolUse"
matcher = "Shell"
command = "./.kimi/hooks/no-rm-rf.sh"
timeout = 5
```

Result: a hook returning exit code `2` blocks the tool call before execution.

## Tools and Permissions

Default agent tools observed in source:

| Tool | Default-mode approval | Notes |
| --- | --- | --- |
| `Agent` | Usually no direct approval | Spawns/resumes subagent instances; child tool approvals go through root runtime. |
| `AskUserQuestion` | No approval prompt | In AFK/print it auto-dismisses. |
| `SetTodoList` | No approval prompt | Updates session task list. |
| `Shell` | Asks | Runs host shell command; Windows uses Git Bash. |
| `TaskList` | No approval prompt | Lists background tasks. |
| `TaskOutput` | No approval prompt | Reads task output. |
| `TaskStop` | Asks | Stops background task. |
| `ReadFile` | No approval prompt | Sensitive-file filters apply. |
| `ReadMediaFile` | No approval prompt | Requires model/media support. |
| `Glob` | No approval prompt | File discovery. |
| `Grep` | No approval prompt | Sensitive files are skipped. |
| `WriteFile` | Asks | Plan-file writes can be auto-approved in plan mode. |
| `StrReplaceFile` | Asks | Plan-file edits can be auto-approved in plan mode. |
| `SearchWeb` | No approval prompt | Depends on service/provider configuration. |
| `FetchURL` | No approval prompt | Network is not sandboxed by Kimi. |
| `EnterPlanMode` | Asks or auto-approves by mode | Yolo auto-approves entering plan. |
| `ExitPlanMode` | Asks | Still asks under yolo; AFK auto-approves. |
| `SendDMail` | Asks | Present in okabe agent. |

Native permission entities are session modes, approval actions, tool names, hook events/matchers, workspace roots, MCP servers/tools, agent YAML tool lists, and subagent definitions. There is no command glob grammar like `Bash(rm *)` in the target repo's persisted permission config; such patterns can only be implemented by hooks or, in successor docs, a different permission rule system.

Approval persistence is per session and per action string. Choosing "approve for this session" adds the action to `approval.auto_approve_actions` in `state.json`.

## Sandboxing, Trust, and Administrative Controls

Kimi has no separate sandbox mode. The source system prompt explicitly says the operating environment is not sandboxed. Shell commands run with the launching user's OS permissions. File tools and model instructions encourage work inside the working directory, but this is not an OS-enforced boundary.

No folder trust gate, managed/admin policy layer, or safe mode was found. Project `AGENTS.md` and skills can influence behavior; hooks and MCP servers can add powerful local behavior; none is gated by a native trust decision in the target repo.

Protected/sensitive paths are tool-layer filters, not sandbox rules. The docs/source identify dotenv files, SSH private keys, and cloud credentials as protected read/search targets. Credential stores under `~/.kimi/credentials` and MCP OAuth tokens under `~/.kimi/mcp-oauth` are sensitive provider-reserved data; docs state credential files are written with user-only file permissions.

Security posture: Kimi is a combination of advisory prompts, client-side approvals, persisted session state, hook checks, and hard-coded tool filters. It is not an OS sandbox.

## MCP and Permissions

MCP servers are loaded from `~/.kimi/mcp.json`, `--mcp-config-file`, or `--mcp-config`. Persisted server management is through `kimi mcp add`, `remove`, `auth`, and `reset-auth`.

MCP tools follow the same approval runtime as built-in tools. There are no native server/tool/resource allow-deny lists. To make MCP safer:

- Load only trusted servers.
- Prefer invocation-scoped `--mcp-config-file` for wrapper-controlled sessions.
- Avoid yolo/afk when connecting untrusted MCP servers.
- Use `PreToolUse` hooks to block risky MCP tool names.
- Keep OAuth tokens under `~/.kimi/mcp-oauth` minimal and reset them when no longer needed.

MCP servers/tools run outside any Kimi OS sandbox. No response sanitization or resource-level policy layer was found.

## Non-Interactive Behavior

`--print` is non-interactive and applies runtime AFK, so tool approvals and AskUserQuestion auto-resolve rather than hanging. `--quiet` inherits this behavior. In ordinary shell mode without yolo/afk, approval prompts require an interactive UI.

ACP mode is different: Kimi forwards approval requests to the ACP client via `session/request_permission` with approve once, approve for this session, and reject options. If that programmatic path fails, source rejects the request rather than hanging.

## Sources

- [Kimi CLI repository](https://github.com/MoonshotAI/kimi-cli)
- [Kimi command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Config files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Environment variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Data locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Hooks (Beta)](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Changelog](https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.html)
- [Successor Kimi Code config docs](https://www.kimi.com/code/docs/en/kimi-code-cli/configuration/config-files.html)

## Changelog

- 2026-07-03: Refreshed against current docs/source and local installation; fixed frontmatter schema issues; corrected plan/yolo/afk/headless semantics; added ACP approval behavior, hook fail-open details, tool visibility separation, and successor Kimi Code drift notes.
- 2026-07-02: Earlier merged-topic research added broad permissions/security-control coverage.
