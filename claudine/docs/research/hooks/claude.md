---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/overview
hooks_docs: https://code.claude.com/docs/en/hooks

hooks:
  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, source (startup|resume|clear|compact), model, agent_type (optional)"
    return_contract: "Exit 0: stdout plain text or {additionalContext} injected into Claude's context. Env vars written to CLAUDE_ENV_FILE are exported into subsequent Bash calls. Exit 2: stderr shown to user, session still starts."
    notes: "Matcher: startup|resume|clear|compact. Fires on every compaction (source=compact) so expensive hooks should branch on source."
  - native_event: Setup
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, trigger (init|maintenance)"
    return_contract: "Same as SessionStart; designed for one-shot CI/scripted preparation via --init-only / --init / --maintenance."
    notes: "Matcher: init|maintenance. Designed for `--init-only` / `--init` / `--maintenance` in `-p` mode; MCP servers may not yet be connected."
  - native_event: UserPromptSubmit
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, prompt"
    return_contract: "Exit 0 + JSON {decision: 'block', reason, hookSpecificOutput.additionalContext}: blocks prompt, erases from context, reason shown to user. Exit 0 + plain stdout text or JSON {additionalContext}: injects context into Claude. Exit 2: blocks prompt and erases from context, stderr becomes Claude's feedback."
    notes: "Matcher field is silently ignored (no matcher support)."
  - native_event: UserPromptExpansion
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, command (skill/command name), expanded_prompt"
    return_contract: "Can block the expansion before the prompt reaches Claude. Exit 2 or {decision: 'block'} blocks."
    notes: "Matcher: command name (skill or slash command)."
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_use_id, tool_input"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, permissionDecision (allow|deny|ask|defer), permissionDecisionReason, updatedInput, additionalContext}}: blocks, modifies input, auto-approves, or shows permission dialog. Exit 2: blocks tool call, stderr becomes Claude's feedback. Top-level {decision, reason} deprecated — use hookSpecificOutput.permissionDecision."
    notes: "Matcher: tool name (regex or `|`/`-` separated list). Optional `if` field uses permission-rule syntax for tool-arg pre-filtering (v2.1.85+). updatedInput only modifies fields that exist in the tool schema."
  - native_event: PermissionRequest
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_input, permission_suggestions (optional)"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, decision: {behavior: allow|deny, updatedInput, updatedPermissions, message, interrupt}}}: grants or denies permission; allow may carry updatedPermissions (setMode session-only mode switch). Exit 2: denies permission, stderr becomes Claude's feedback."
    notes: "Matcher: tool name. Does NOT fire in non-interactive (`-p`) mode — permission dialog is bypassed; use PreToolUse for automated permission decisions in headless mode."
  - native_event: PermissionDenied
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_input"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, decision: {behavior: retry, message}}}: retry the denied tool call. Exit 0 + JSON {retry: true}: retry. Otherwise deny is final."
    notes: "Fires when auto-mode classifier denies a tool call. Matchable on tool name. Only `command`/`http`/`mcp_tool` types."
  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_use_id, tool_input, tool_response"
    return_contract: "Exit 0 + JSON {decision: 'block', reason, hookSpecificOutput: {hookEventName, additionalContext, updatedMCPToolOutput}}: tool has already run; cannot be undone. updatedMCPToolOutput replaces output for MCP tools only. Exit 2: stderr fed to Claude as feedback."
    notes: "Matcher: tool name. Tool already executed — `decision: block` warns Claude but cannot reverse."
  - native_event: PostToolUseFailure
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_name, tool_use_id, tool_input, error, is_interrupt (optional)"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, additionalContext}}: corrective context for Claude."
    notes: "Matcher: tool name."
  - native_event: PostToolBatch
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, tool_calls (array)"
    return_contract: "Information/context only; cannot block. Prompt hooks: `ok: false` ends the turn with `reason`."
    notes: "Fires after a full batch of parallel tool calls resolves. No matcher support."
  - native_event: Notification
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, message, title (optional), notification_type"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, additionalContext}}: external alerting (desktop notifications, Slack). Exit 2: stderr shown to user only."
    notes: "Matcher: permission_prompt|idle_prompt|auth_success|elicitation_dialog|elicitation_complete|elicitation_response|agent_needs_input|agent_completed."
  - native_event: MessageDisplay
    claudine_event: notification
    timing: around
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, message"
    return_contract: "Display-only while assistant message text streams. Lower default timeout (10s for command/http/mcp_tool)."
    notes: "No matcher support. Fires during streaming display."
  - native_event: SubagentStart
    claudine_event: subagent_start
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, agent_id, agent_type"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, additionalContext}}: context injected into the spawned subagent. Exit 2: stderr shown to user only."
    notes: "Matcher: agent type (general-purpose, Explore, Plan, custom, or plugin-scoped `plugin:reviewer`)."
  - native_event: SubagentStop
    claudine_event: subagent_stop
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, stop_hook_active, agent_id, agent_type, agent_transcript_path"
    return_contract: "Exit 0 + JSON {decision: 'block', reason}: prevents subagent from stopping, continues work. Exit 2: stderr becomes feedback. Stop hooks must check stop_hook_active to avoid infinite loops."
    notes: "Matcher: agent type (same values as SubagentStart)."
  - native_event: TaskCreated
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, task_id, task_subject, task_description (optional), task_status"
    return_contract: "Information/observation; cannot block."
    notes: "Fires when a task is being created via TaskCreate. No matcher support."
  - native_event: TaskCompleted
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, task_id, task_subject, task_description (optional), teammate_name (optional), team_name (optional)"
    return_contract: "Exit 2: prevents task from being marked completed, stderr becomes feedback. Prompt/agent hooks: ok=false keeps task open. Exit 0: task completes normally."
    notes: "No matcher support. Supports command, prompt, and agent hook types."
  - native_event: Stop
    claudine_event: finalize
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, stop_hook_active"
    return_contract: "Exit 0 + JSON {decision: 'block', reason}: prevents stopping, continues conversation. Exit 2: stderr becomes the reason for continuation. Prompt/agent hooks: ok=false keeps Claude working with `reason`."
    notes: "No matcher support. Hooks MUST check stop_hook_active to avoid infinite loops. Does not fire on user interrupts."
  - native_event: StopFailure
    claudine_event: failure
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, error_type, error_message (optional)"
    return_contract: "Output and exit code are ignored — observation only."
    notes: "Fires when turn ends due to API error. Matcher: rate_limit|overloaded|authentication_failed|oauth_org_not_allowed|billing_error|invalid_request|model_not_found|server_error|max_output_tokens|unknown."
  - native_event: TeammateIdle
    claudine_event: subagent_stop
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, teammate_name, team_name"
    return_contract: "Exit 2: prevents teammate from going idle, stderr becomes feedback. Command hooks only — no JSON decision control."
    notes: "No matcher support. Only `type: command` is allowed; prompt/agent hooks do not work."
  - native_event: InstructionsLoaded
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, file_path, load_reason"
    return_contract: "Observation only."
    notes: "Fires when CLAUDE.md or .claude/rules/*.md loads. Matcher: session_start|nested_traversal|path_glob_match|include|compact."
  - native_event: ConfigChange
    claudine_event: notification
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, source, file_path, change_type"
    return_contract: "Exit 2 or {decision: 'block'} prevents the change from taking effect; stderr or reason becomes the message. Default lets the change apply."
    notes: "Fires when a watched settings/skills file changes mid-session. Matcher: user_settings|project_settings|local_settings|policy_settings|skills."
  - native_event: CwdChanged
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, old_cwd, new_cwd"
    return_contract: "Observation only."
    notes: "No matcher support — fires on every directory change. Useful for direnv-style reactive env management via CLAUDE_ENV_FILE."
  - native_event: FileChanged
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, file_path, change_type"
    return_contract: "Observation only; writes back via `watchPaths` output field to extend the watch list."
    notes: "Matcher is a literal filename list (`.envrc|.env`) rather than regex; the same value seeds the watch list. FileChanged uses a narrower exact-match set (letters/digits/`_`/`|`)."
  - native_event: WorktreeCreate
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, worktree_path, branch"
    return_contract: "Observation only; can replace default git worktree behavior."
    notes: "No matcher support. Fires via --worktree or isolation: worktree."
  - native_event: WorktreeRemove
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, worktree_path"
    return_contract: "Observation only."
    notes: "No matcher support. Fires at session exit or when a subagent finishes."
  - native_event: PreCompact
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, trigger, custom_instructions"
    return_contract: "Information only; cannot block compaction."
    notes: "Matcher: manual|auto. custom_instructions carries `/compact` text for manual; empty string for auto."
  - native_event: PostCompact
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, trigger"
    return_contract: "Information only."
    notes: "Matcher: manual|auto. Post-compaction counterpart of PreCompact."
  - native_event: Elicitation
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, server_name, elicitation_id, message, requested_schema"
    return_contract: "Observation only; cannot block."
    notes: "Fires when MCP server requests user input. Matcher: MCP server name."
  - native_event: ElicitationResult
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, server_name, elicitation_id, action, content"
    return_contract: "Observation only."
    notes: "Fires after user responds to MCP elicitation. Matcher: MCP server name."
  - native_event: SessionEnd
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, permission_mode, hook_event_name, reason"
    return_contract: "Cleanup only; cannot prevent termination."
    notes: "Matcher: clear|resume|logout|prompt_input_exit|bypass_permissions_disabled|other."

config_files:
  - os: macos
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User scope; lowest priority. Can include `hooks`, `disableAllHooks`, `permissions`. File watcher reloads edits live."
  - os: linux
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User scope; lowest priority. Can include `hooks`, `disableAllHooks`, `permissions`. File watcher reloads edits live."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\settings.json"
    format: json
    notes: "User scope; lowest priority. Can include `hooks`, `disableAllHooks`, `permissions`. File watcher reloads edits live."
  - os: macos
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Project scope; medium priority. Committed to git and shared with collaborators. The `$schema` line points to https://json.schemastore.org/claude-code-settings.json."
  - os: linux
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Project scope; medium priority. Committed to git and shared with collaborators. The `$schema` line points to https://json.schemastore.org/claude-code-settings.json."
  - os: windows
    scope: repo
    path: ".claude\\settings.json"
    format: json
    notes: "Project scope; medium priority. Committed to git and shared with collaborators. The `$schema` line points to https://json.schemastore.org/claude-code-settings.json."
  - os: macos
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    notes: "Local scope; high priority (above project, below managed). Gitignored when Claude Code creates it. Hook additions here override project hooks."
  - os: linux
    scope: repo
    path: ".claude/settings.local.json"
    format: json
    notes: "Local scope; high priority (above project, below managed). Gitignored when Claude Code creates it. Hook additions here override project hooks."
  - os: windows
    scope: repo
    path: ".claude\\settings.local.json"
    format: json
    notes: "Local scope; high priority (above project, below managed). Gitignored when Claude Code creates it. Hook additions here override project hooks."
  - os: macos
    scope: managed
    path: "/Library/Application Support/ClaudeCode/managed-settings.json"
    format: json
    notes: "Managed scope; highest priority; cannot be overridden by user/project/local. Drop-in dir managed-settings.d/ also supported (systemd-style merge, sorted numerically)."
  - os: macos
    scope: managed
    path: "/Library/Application Support/ClaudeCode/managed-mcp.json"
    format: json
    notes: "Managed MCP configuration; takes exclusive control of MCP unless allowAllClaudeAiMcps is also set."
  - os: macos
    scope: managed
    path: "com.anthropic.claudecode (managed preferences domain)"
    format: other
    notes: "macOS plist delivered via MDM (Jamf, Iru/Kandji); nested settings as dictionaries, arrays as plist arrays."
  - os: linux
    scope: managed
    path: "/etc/claude-code/managed-settings.json"
    format: json
    notes: "Linux/WSL managed file scope; highest priority. Drop-in dir managed-settings.d/ supported."
  - os: windows
    scope: managed
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    format: json
    notes: "Windows managed file scope. v2.1.75 deprecated the legacy C:\\ProgramData\\ClaudeCode\\managed-settings.json path; must migrate."
  - os: windows
    scope: managed
    path: "HKLM\\SOFTWARE\\Policies\\ClaudeCode (Settings REG_SZ/REG_EXPAND_SZ)"
    format: other
    notes: "Windows registry delivery via Group Policy or Intune; contains JSON."
  - os: windows
    scope: managed
    path: "HKCU\\SOFTWARE\\Policies\\ClaudeCode"
    format: other
    notes: "Windows registry HKCU delivery; lowest policy priority, only used when no admin-level source exists."
  - os: macos
    scope: managed
    path: "Server-managed (claude.ai admin console or self-hosted Claude apps gateway)"
    format: json
    notes: "Server-side delivery at sign-in for cloud-managed policies."
  - os: linux
    scope: managed
    path: "Server-managed (claude.ai admin console or self-hosted Claude apps gateway)"
    format: json
    notes: "Server-side delivery at sign-in for cloud-managed policies."
  - os: windows
    scope: managed
    path: "Server-managed (claude.ai admin console or self-hosted Claude apps gateway)"
    format: json
    notes: "Server-side delivery at sign-in for cloud-managed policies."
  - os: macos
    scope: other
    path: "<plugin>/hooks/hooks.json"
    format: json
    notes: "Plugin-bundled hooks; loaded only when plugin is enabled (or force-enabled in managed settings enabledPlugins)."
  - os: linux
    scope: other
    path: "<plugin>/hooks/hooks.json"
    format: json
    notes: "Plugin-bundled hooks; loaded only when plugin is enabled (or force-enabled in managed settings enabledPlugins)."
  - os: windows
    scope: other
    path: "<plugin>\\hooks\\hooks.json"
    format: json
    notes: "Plugin-bundled hooks; loaded only when plugin is enabled (or force-enabled in managed settings enabledPlugins)."
  - os: macos
    scope: other
    path: "Skill or subagent YAML frontmatter (`hooks:`)"
    format: yaml
    notes: "Frontmatter-scoped hooks; live only while the skill or agent is active. Subagent `Stop` is automatically converted to `SubagentStop`."
  - os: linux
    scope: other
    path: "Skill or subagent YAML frontmatter (`hooks:`)"
    format: yaml
    notes: "Frontmatter-scoped hooks; live only while the skill or agent is active. Subagent `Stop` is automatically converted to `SubagentStop`."
  - os: windows
    scope: other
    path: "Skill or subagent YAML frontmatter (`hooks:`)"
    format: yaml
    notes: "Frontmatter-scoped hooks; live only while the skill or agent is active. Subagent `Stop` is automatically converted to `SubagentStop`."
  - os: macos
    scope: other
    path: "~/.claude.json"
    format: json
    notes: "OAuth session, per-project state (allowed tools, trust), and caches; not a hook-config location but holds MCP user/local config."
  - os: linux
    scope: other
    path: "~/.claude.json"
    format: json
    notes: "OAuth session, per-project state (allowed tools, trust), and caches; not a hook-config location but holds MCP user/local config."
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.claude.json"
    format: json
    notes: "OAuth session, per-project state (allowed tools, trust), and caches; not a hook-config location but holds MCP user/local config."

cli_params:
  - flag: "/hooks"
    description: "In-session menu listing every configured hook grouped by event with matchers, type, source file, and command/URL/prompt. Read-only — to add/modify remove, edit settings JSON."
    example: "/hooks"
  - flag: "--debug [categories]"
    description: "Enable debug output for hook execution; categories include `hooks` (e.g. `claude --debug \"api,hooks\"`)."
    example: "claude --debug \"hooks\""
  - flag: "--bare"
    description: "Minimal mode that skips auto-discovery of hooks (and skills, plugins, MCP servers, auto memory, CLAUDE.md); sets CLAUDE_CODE_SIMPLE."
    example: "claude --bare -p \"query\""
  - flag: "--disable-slash-commands"
    description: "Disable skills and commands for the session. Does not disable hooks."
    example: "claude --disable-slash-commands"
  - flag: "--permission-mode <mode>"
    description: "Set the active permission mode (`default|plan|acceptEdits|dontAsk|bypassPermissions|auto`); the chosen mode is reflected in hook payloads as `permission_mode` and gates which hooks fire."
    example: "claude --permission-mode bypassPermissions"
  - flag: "--dangerously-skip-permissions / --allow-dangerously-skip-permissions"
    description: "Skip all permission prompts (`bypassPermissions`); affects whether PermissionRequest hooks fire (they do NOT fire in non-interactive `-p` mode either way)."
    example: "claude --dangerously-skip-permissions"
  - flag: "--settings <path|json>"
    description: "Inline JSON or path whose keys override the same keys in settings.json files for this session, including any hook additions."
    example: "claude --settings ./settings.json"
  - flag: "--plugin-dir <path>"
    description: "Load a plugin from a local directory for this session; enables any hooks/hooks.json in the plugin."
    example: "claude --plugin-dir ./my-plugin"
  - flag: "--add-dir <path>"
    description: "Add additional working directories; grants file access (most .claude/ config is NOT discovered from these)."
    example: "claude --add-dir ../lib"
  - flag: "--init-only / --init / --maintenance (in -p mode)"
    description: "Trigger the `Setup` hook with `trigger: init` or `trigger: maintenance` for one-shot preparation in CI/scripted contexts."
    example: "claude -p --init \"prepare the build\""
  - flag: "disableAllHooks (settings key)"
    description: "Set in any settings file to disable all hooks. Hooks configured in managed settings still run unless disableAllHooks is also set there."
    example: "\"disableAllHooks\": true"
  - flag: "allowManagedHooksOnly (managed-settings only)"
    description: "Blocks user, project, and most plugin hooks; only managed hooks, SDK hooks, and plugins force-enabled via `enabledPlugins` remain."
    example: "\"allowManagedHooksOnly\": true"
  - flag: "allowedHttpHookUrls (managed-settings)"
    description: "Allowlist of URL patterns that HTTP hooks may target (`*` wildcard). Undefined = unrestricted, empty array = block all HTTP hooks. Arrays merge across settings sources."
    example: "\"allowedHttpHookUrls\": [\"https://hooks.example.com/*\"]"
  - flag: "--safe-mode"
    description: "Start with all customizations (CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, custom themes, keybindings) disabled. Admin-managed policy settings still apply. Sets CLAUDE_CODE_SAFE_MODE=1."
    example: "claude --safe-mode"
  - flag: "--include-hook-events"
    description: "Include all hook lifecycle events in the output stream. Only works with --output-format=stream-json."
    example: "claude -p --output-format=stream-json --include-hook-events \"query\""
  - flag: "--setting-sources <sources>"
    description: "Comma-separated list of setting sources to load (user, project, local). Use it to skip scopes that would normally contribute hooks."
    example: "claude --setting-sources user,project"

payload_fields:
  - native_event: SessionStart
    field: "source"
    type: string
    meaning: "Why the session started: startup | resume | clear | compact (drives SessionStart on every compaction)."
  - native_event: SessionStart
    field: "agent_type"
    type: string
    meaning: "Present only when the session was started with `claude --agent <name>`."
  - native_event: Setup
    field: "trigger"
    type: string
    meaning: "Which CLI flag triggered setup: init | maintenance."
  - native_event: UserPromptSubmit
    field: "prompt"
    type: string
    meaning: "The user-submitted text."
  - native_event: UserPromptExpansion
    field: "command"
    type: string
    meaning: "Skill or slash-command name whose prompt is being expanded."
  - native_event: UserPromptExpansion
    field: "expanded_prompt"
    type: string
    meaning: "The expanded prompt text about to be sent to Claude."
  - native_event: PreToolUse
    field: "tool_name"
    type: string
    meaning: "Tool name (matched against `matcher` for group filtering)."
  - native_event: PreToolUse
    field: "tool_use_id"
    type: string
    meaning: "Unique identifier for the tool call (used to correlate with PostToolUse)."
  - native_event: PreToolUse
    field: "tool_input"
    type: object
    meaning: "Arguments passed to the tool; schema varies per tool."
  - native_event: PreToolUse
    field: "permission_mode"
    type: string
    meaning: "Current permission mode: default | plan | acceptEdits | dontAsk | bypassPermissions."
  - native_event: PermissionRequest
    field: "permission_suggestions"
    type: array
    meaning: "Always-allow options the user would see in the dialog (e.g. {type: toolAlwaysAllow, tool: Bash})."
  - native_event: PostToolUse
    field: "tool_response"
    type: object
    meaning: "Result returned by the tool; schema varies per tool."
  - native_event: PostToolUseFailure
    field: "error"
    type: string
    meaning: "Description of what went wrong."
  - native_event: PostToolUseFailure
    field: "is_interrupt"
    type: boolean
    meaning: "Whether the failure was caused by user interruption."
  - native_event: Notification
    field: "notification_type"
    type: string
    meaning: "Type driving matcher filtering: permission_prompt | idle_prompt | auth_success | elicitation_dialog | elicitation_complete | elicitation_response | agent_needs_input | agent_completed."
  - native_event: SubagentStart
    field: "agent_id"
    type: string
    meaning: "Unique identifier for the subagent instance."
  - native_event: SubagentStart
    field: "agent_type"
    type: string
    meaning: "Agent type name driving matcher filtering (general-purpose, Explore, Plan, custom)."
  - native_event: SubagentStop
    field: "stop_hook_active"
    type: boolean
    meaning: "true when the subagent is already continuing due to a prior stop hook — must short-circuit to avoid infinite loops."
  - native_event: SubagentStop
    field: "agent_transcript_path"
    type: string
    meaning: "Path to the subagent's own transcript (nested `subagents/` folder)."
  - native_event: Stop
    field: "stop_hook_active"
    type: boolean
    meaning: "true when Claude is already continuing as a result of a stop hook."
  - native_event: TaskCreated
    field: "task_status"
    type: string
    meaning: "Initial status assigned to the new task (e.g. pending, in_progress)."
  - native_event: TaskCompleted
    field: "task_id"
    type: string
    meaning: "Identifier of the task being completed."
  - native_event: TaskCompleted
    field: "teammate_name"
    type: string
    meaning: "Optional; set when a teammate is finishing its turn with in-progress tasks."
  - native_event: PreCompact
    field: "custom_instructions"
    type: string
    meaning: "For manual trigger, the text passed to `/compact`; for auto, empty string."
  - native_event: SessionEnd
    field: "reason"
    type: string
    meaning: "Why the session ended: clear | resume | logout | prompt_input_exit | bypass_permissions_disabled | other."
  - native_event: CwdChanged
    field: "old_cwd / new_cwd"
    type: string
    meaning: "Previous and current working directory."
  - native_event: FileChanged
    field: "file_path"
    type: string
    meaning: "Path of the changed file (literal filename match)."
  - native_event: InstructionsLoaded
    field: "load_reason"
    type: string
    meaning: "Why the file loaded: session_start | nested_traversal | path_glob_match | include | compact."
  - native_event: (common)
    field: "session_id"
    type: string
    meaning: "Current session identifier (every event)."
  - native_event: (common)
    field: "transcript_path"
    type: string
    meaning: "Path to the conversation JSONL transcript (every event)."
  - native_event: (common)
    field: "cwd"
    type: string
    meaning: "Working directory when the hook fired (every event)."
  - native_event: (common)
    field: "permission_mode"
    type: string
    meaning: "Active permission mode (every event): default | plan | acceptEdits | dontAsk | bypassPermissions | auto."
  - native_event: (common)
    field: "hook_event_name"
    type: string
    meaning: "Name of the event that fired (every event)."

response_actions:
  - action: block
    native_value: "Exit 2 OR {decision: 'block', reason} (event-specific)"
    effect: "Stops the action; stderr or `reason` becomes feedback. Used by PreToolUse, PermissionRequest, UserPromptSubmit, Stop, SubagentStop, TeammateIdle, TaskCompleted. Some events cannot block (SessionStart, Setup, PostToolUse, Notification, SubagentStart, PreCompact, SessionEnd)."
  - action: deny
    native_value: "{hookSpecificOutput.permissionDecision: 'deny', permissionDecisionReason}"
    effect: "PreToolUse only: cancels tool call, reason fed to Claude. PermissionRequest: denies permission. PreToolUse `deny` overrides everything including `allow`."
  - action: allow
    native_value: "{hookSpecificOutput.permissionDecision: 'allow', permissionDecisionReason}"
    effect: "PreToolUse: skips interactive prompt but does NOT override deny/ask rules (managed denylist always wins). PermissionRequest: grants permission; may carry `updatedPermissions: [{type: setMode, mode, destination: session}]` to switch modes session-only."
  - action: ask
    native_value: "{hookSpecificOutput.permissionDecision: 'ask', permissionDecisionReason}"
    effect: "PreToolUse: shows the permission dialog to the user as normal (overrides an `allow` decision)."
  - action: other
    native_value: "{hookSpecificOutput.permissionDecision: 'defer'}"
    effect: "PreToolUse non-interactive `-p` mode only: exits the process with the tool call preserved so an Agent SDK wrapper can collect input and resume."
  - action: modify
    native_value: "{hookSpecificOutput.updatedInput: {...}}"
    effect: "PreToolUse: mutates tool input before execution (only fields that exist in the tool's schema). PermissionRequest: same on the allow path."
  - action: replace
    native_value: "{hookSpecificOutput.updatedMCPToolOutput: 'replacement'}"
    effect: "PostToolUse on MCP tools only: replaces the tool output that Claude sees."
  - action: continue
    native_value: "{continue: true|false, stopReason, suppressOutput, systemMessage}"
    effect: "Top-level continue flag (exit 0 only). `continue: false` stops Claude entirely regardless of event-specific decision fields; `systemMessage` shown to user; `stopReason` shown to user (not Claude) when continue=false; `suppressOutput` hides the hook's progress from the transcript."
  - action: other
    native_value: "{additionalContext: '...'}"
    effect: "Injects text into Claude's context (supported by SessionStart, Setup, UserPromptSubmit, UserPromptExpansion, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Notification, SubagentStart)."
  - action: other
    native_value: "{retry: true} or {hookSpecificOutput.decision.behavior: retry, message}"
    effect: "PermissionDenied only: tells the model it may retry the denied tool call."
  - action: other
    native_value: "{watchPaths: [...]} (FileChanged)"
    effect: "Extends the set of files being watched for future FileChanged events."
  - action: other
    native_value: "Plain-text stdout"
    effect: "UserPromptSubmit / UserPromptExpansion / SessionStart / Setup: added as Claude context when not valid JSON. Async command hooks: delivered on the next conversation turn as systemMessage/additionalContext."
  - action: other
    native_value: "{ok: true|false, reason: '...'}"
    effect: "Prompt/agent hooks only. `ok: false` is event-specific: Stop/SubagentStop keeps Claude working; PreToolUse denies tool call with reason; UserPromptSubmit/UserPromptExpansion/PostToolUse/PostToolBatch ends the turn with warning line."

execution:
  shell: "Default is `sh -c` on macOS/Linux, Git Bash on Windows, or PowerShell when Git Bash is unavailable. The `shell` field on command hooks can pin to `\"bash\"` or `\"powershell\"`. Exec form (`args` set) bypasses the shell entirely and spawns the executable directly; path placeholders are substituted as plain strings with no quoting. On Windows, exec form requires a real `.exe`; `.cmd`/`.bat` shims must be invoked via shell form or `node` directly."
  cwd: "Handers run in the current working directory of the Claude Code session (the `cwd` field in the payload)."
  env: "Claude Code's environment is exported to the spawned process. Path placeholders are also exported as env vars: `CLAUDE_PROJECT_DIR` (project root, set for stdio MCP servers and plugin LSP servers too), `CLAUDE_PLUGIN_ROOT` (plugin install dir), `CLAUDE_PLUGIN_DATA` (plugin persistent data dir). `CLAUDE_CODE_REMOTE` is `\"true\"` in remote web environments, unset in local CLI. `CLAUDE_CODE_BRIDGE_SESSION_ID` is set while the local session has an active Remote Control connection (v2.1.199+). `CLAUDE_CODE_SIMPLE` is set to `\"1\"` by `--bare`, which skips hooks (and skills, plugins, MCP, auto-memory, CLAUDE.md). `CLAUDE_CODE_SAFE_MODE` is set to `\"1\"` by `--safe-mode`, which disables hooks and other customizations while still honoring managed policy. `CLAUDE_ENV_FILE` is exposed ONLY in SessionStart hooks and is the file path Claude Code sources as a preamble to subsequent Bash commands (used to persist direnv-style env var changes)."
  timeout: "Default 600s for command/http/mcp_tool; 30s for prompt; 60s for agent. UserPromptSubmit lowers command/http/mcp_tool defaults to 30s. MessageDisplay lowers command/http/mcp_tool defaults to 10s. Async hooks share the same 10-minute default. Per-handler override via the `timeout` field (seconds). Permissions.json `permissions.hookTimeLimitMs` may also apply."
  stdin: "JSON event payload (one document). For command hooks only; http hooks receive the same JSON as the POST body."
  stdout: "On exit 0: parsed as JSON output (top-level fields continue/stopReason/suppressOutput/systemMessage; event-specific `hookSpecificOutput`). Plain text becomes Claude context for UserPromptSubmit/SessionStart/Setup/UserPromptExpansion; becomes systemMessage on async hooks. Empty/non-JSON exit-0 is treated as 'no decision' — for PreToolUse this does NOT approve the call (the normal permission flow still applies)."
  stderr: "On exit 2: becomes Claude's feedback (PreToolUse, PostToolUse, PostToolUseFailure, UserPromptSubmit, UserPromptExpansion, Stop, SubagentStop, PermissionRequest, TeammateIdle, TaskCompleted). On any other non-zero exit: shown in the transcript as `<hook name> hook error` with the first line of stderr; full stderr goes to the debug log. On exit 2 for non-blockable events (SessionStart, Setup, Notification, SubagentStart, etc.): stderr shown to user only."
  notes: "All matching hooks run in parallel; identical handlers are deduplicated automatically (command hooks dedup by `command`+`args`, HTTP hooks dedup by URL). For PreToolUse permission decisions, the most restrictive answer applies in the order deny, defer, ask, allow (text from additionalContext is concatenated from all hooks). One hook returning `deny` does NOT stop sibling hooks from running. Settings edits are watched live: most keys reload automatically; `model` and `outputStyle` apply on next restart. Snapshotting at session start is the legacy behavior — current watcher picks up edits to most keys including `hooks` without restart. HTTP hooks differ from command hooks: non-2xx/connection-failure/timeout produce non-blocking errors (action continues); to block, return 2xx with `decision: block` or `permissionDecision: deny`."

gaps:
  - "34 native events substantially exceed Claudine's 16 normalized events; mapping is many-to-one. Examples: PostToolUseFailure and PostToolBatch both map to `tool_result`; SessionStart, Setup, and InstructionsLoaded all map to `initialize`; Stop, StopFailure, and SessionEnd all map to `finalize`. Claudine needs a documented disambiguation strategy (likely an `event_kind` discriminator inside payload + a precedence rule)."
  - "Three hook handler types (command, prompt, agent) plus two newer ones (http, mcp_tool) — Claudine's adapter currently models only the shell-command handler. Prompt/agent (LLM-in-the-loop) and HTTP/MCP hook surfaces have no adapter equivalents."
  - "Per-event timing semantics differ (pre vs post vs around vs async). `MessageDisplay` is `around` (runs during streaming display); `Notification` and `FileChanged` are async (fire-and-forget post-event). Claudine's `prompt`/`tool_call`/`tool_result` mapping currently doesn't model `around` events."
  - "Matcher rule parsing is non-trivial: plain words/`|`/`,` are exact strings, anything else is a JS RegExp, plus per-event narrowing of the exact-match alphabet (FileChanged/StopFailure use only `[A-Za-z0-9_|]`). The `if` field uses a permission-rule syntax that is unrelated to matchers. Claudine's adapter needs both: matcher (group filter) and `if` (per-handler pre-filter)."
  - "SessionStart re-fires on every compaction with `source: compact`; current Claudine adapter cannot distinguish `initialize` (cold start) from `initialize` (post-compaction re-injection)."
  - "Hook payloads contain a per-event tool_input that varies in shape across tools (Bash vs Write vs Edit vs WebFetch vs Task). Claudine's `tool_call` payload schema would need a polymorphic `tool_input` field plus a tool-name discriminator."
  - "Top-level `decision`/`reason` for PreToolUse is deprecated in favor of `hookSpecificOutput.permissionDecision`. Claudine's adapter must model both the legacy and modern shapes, and choose per-version behavior."
  - "`permission_mode` in every payload gives Claudine a free signal for normalizing into its permission-event taxonomy. Currently the schema's `permission` event is a Claudine-side abstraction without an obvious Claude Code hook anchor — only PermissionRequest, PermissionDenied, and PreToolUse carry permission information."
  - "`stop_hook_active` / `is_interrupt` / `agent_transcript_path` / `permission_suggestions` / `load_reason` / `notification_type` are provider-specific discriminators that Claudine either maps into typed subfields or preserves as a `provider_extensions` blob."
  - "CLAUDE_ENV_FILE is a SessionStart-only mechanism with no equivalent on the other 7 providers; needs to be modeled as a Claude-specific session-side-effect (and wired into the Bash tool's preamble)."
  - "WorktreeCreate/WorktreeRemove, Elicitation/ElicitationResult, MessageDisplay, ConfigChange, FileChanged, CwdChanged, InstructionsLoaded, PreCompact/PostCompact are provider-specific lifecycle events without an obvious Claudine unified event. They either get a `provider_extensions` round-trip or fold into existing events with caveats."
  - "`Setup` is a hook event for one-shot CI preparation triggered by `--init-only`/`--init`/`--maintenance` in `-p` mode. There is no analogue on other providers."
  - "StopFailure outputs are ignored entirely (the hook is purely informational); Claudine's adapter needs to know not to dispatch any `finalize`-side actions from its output."
  - "`TeammateIdle` supports only `type: command` — prompt/agent hooks are silently rejected. Adapter must validate the hook type per event."
  - "PermissionRequest hooks do NOT fire in non-interactive (`-p`) mode. Claudine's permission-event adapter for headless runs must use PreToolUse instead."
  - "`allowedHttpHookUrls` is a managed-policy allowlist for HTTP hooks; not modelled in other providers."
  - "Existing research file `claude-code.md` (1232 lines) duplicates this content but is missing all schema fields (`$schema`, `created`, `last_updated`, `agent`, `model`, `requires_claudine_update`, `reason`). The schema migration is a separate cleanup task."
  - "Per-session hook-disable mechanisms (`--bare`/`CLAUDE_CODE_SIMPLE`, `--safe-mode`/`CLAUDE_CODE_SAFE_MODE`) and `--setting-sources` are not yet modeled as Claudine session gates; only the static `disableAllHooks` setting is represented."

changes:
  - "2026-07-03 — Refreshed against Claude Code v2.1.199 and official docs. Added `--safe-mode`, `--include-hook-events`, and `--setting-sources` CLI controls; split all `os: all` config_files records into per-OS (macOS/Linux/Windows) entries; mapped the native `defer` permissionDecision to `response_actions: other` because the schema action enum does not include `defer`; added `auto` to the `permission_mode` value set; documented `CLAUDE_CODE_SAFE_MODE` and `CLAUDE_CODE_SIMPLE` environment disables."
  - "Renamed target from `claude-code.md` to `claude.md` to match `claudine/docs/providers.yaml`."
  - "Added full SimplifiedSchema frontmatter (was missing on the existing `claude-code.md`)."
  - "Expanded event list from 14 to 34 (the 14 legacy events plus 20 new events documented in the current docs): Setup, UserPromptExpansion, PermissionDenied, PostToolBatch, MessageDisplay, TaskCreated, StopFailure, InstructionsLoaded, ConfigChange, CwdChanged, FileChanged, WorktreeCreate, WorktreeRemove, PostCompact, Elicitation, ElicitationResult."
  - "Added new hook types: http and mcp_tool (legacy research only covered command/prompt/agent)."
  - "Added `if` field (permission-rule pre-filter), exec form (`args`), and `shell` field (powershell pin) to handler schema."
  - "Added deferred permissionDecision value for headless `-p` mode."
  - "Added allowedHttpHookUrls, allowManagedHooksOnly, disableAllHooks, $schema URL, HKCU/HKLM/macOS plist delivery, drop-in managed-settings.d/ support to config_files."
  - "Added CLAUDE_ENV_FILE, CLAUDE_CODE_BRIDGE_SESSION_ID, CLAUDE_CODE_REMOTE to env section."
  - "Captured Auto-update re-snapshot vs file-watcher live reload semantics (model and outputStyle still need restart)."

requires_claudine_update: true
reason: "Claude Code's hook surface has expanded from 14 to 34 events across 5 handler types (command/prompt/agent/http/mcp_tool). Claudine's adapter needs: (a) a per-event many-to-one map into the 16 unified events with documented disambiguation (e.g. PostToolUseFailure→tool_result, SessionStart/Setup/InstructionsLoaded→initialize, Stop/StopFailure/SessionEnd→finalize, MessageDisplay→notification with around-timing, WorktreeCreate/Elicitation/ElicitationResult→tool_call/notification with provider_extensions); (b) per-handler-type validators (TeammateIdle only accepts command; SessionStart's CLAUDE_ENV_FILE is one-of-a-kind); (c) a polymorphic tool_input schema with tool_name discriminator (Bash vs Write vs Edit vs WebFetch vs Task vs MCP); (d) legacy PreToolUse top-level decision/reason vs modern hookSpecificOutput.permissionDecision support; (e) matcher-rule parser handling exact words (`Edit|Write`), comma-separated lists (v2.1.191+), hyphens in exact set (v2.1.195+), JS RegExp, and per-event narrowing (FileChanged/StopFailure drop to `[A-Za-z0-9_|]`); (f) `if` field permission-rule pre-filter; (g) HTTP-hook allowlist policy (allowedHttpHookUrls) and mcp_tool-only-on-connected-servers semantics; (h) `permission_mode`, `notification_type`, `load_reason`, `stop_hook_active`, `agent_transcript_path` as typed subfields on payloads; (i) async/around timing semantics (MessageDisplay is `around`; Notification/FileChanged/CwdChanged/ConfigChange/PostCompact are effectively async fire-and-forget); (j) per-session hook-disable toggles (`--bare`/`--safe-mode`/`--setting-sources` and the CLAUDE_CODE_SIMPLE/CLAUDE_CODE_SAFE_MODE env vars) that Claudine should surface when wrapping a Claude session. The legacy 14-event research in claude-code.md should be deleted once the adapter migrates to this 34-event map."
---

# Claude Code hooks and events

## Overview

Claude Code (Anthropic's agentic CLI) ships the most expressive hook system of any provider in Claudine's roster. As of v2.1.199 the documentation line lists **34 hook events** at the lifecycle, tool-call, subagent, configuration, worktree, compact, elicitation, and session-end boundaries, and supports **five hook handler types** per event (`command`, `http`, `mcp_tool`, `prompt`, `agent`).

A hook is a JSON object nested under a top-level `hooks` key in a settings file. Each event holds a list of *matcher groups*; each group has a `matcher` (regex, exact-string, or list) and an array of *handlers* that fire when the matcher matches. Identical handlers deduplicate automatically and all matching handlers run in parallel — one handler returning `deny` does **not** prevent siblings from running.

Hooks can **block**, **modify**, **allow/deny/ask/defer permissions**, **auto-approve**, **switch permission modes** (session-scoped), **inject context** into Claude's prompt, **persist environment variables** for subsequent Bash calls, **forward** event data to HTTP endpoints and MCP servers, and **route through prompt-based or agent-based LLM evaluation** instead of a deterministic shell command. They cannot reverse already-executed tool calls, terminate the session, or override managed-policy deny rules.

## Native Hooks

### Event inventory (34 events, grouped by cadence)

| Cadence | Event | Matcher target | Can block |
|---------|-------|----------------|-----------|
| Once per session | `SessionStart` | source (`startup\|resume\|clear\|compact`) | no |
| Once per session | `Setup` | trigger (`init\|maintenance`) | no |
| Once per turn | `UserPromptSubmit` | (none — fires on every prompt) | yes |
| Once per turn | `UserPromptExpansion` | command name (skill / slash command) | yes |
| Once per turn | `Stop` | (none) | yes |
| Once per turn | `StopFailure` | error type (`rate_limit\|overloaded\|...\|unknown`) | no (output ignored) |
| Per tool call | `PreToolUse` | tool name | yes |
| Per tool call | `PermissionRequest` | tool name | yes |
| Per tool call | `PermissionDenied` | tool name | yes |
| Per tool call | `PostToolUse` | tool name | no (already executed) |
| Per tool call | `PostToolUseFailure` | tool name | no (already failed) |
| Per batch | `PostToolBatch` | (none) | no |
| Async | `Notification` | notification_type | no |
| Around | `MessageDisplay` | (none) | no (display-only) |
| Subagent | `SubagentStart` | agent_type | no |
| Subagent | `SubagentStop` | agent_type | yes |
| Subagent | `TeammateIdle` | (none) | yes (exit 2 only) |
| Task list | `TaskCreated` | (none) | no |
| Task list | `TaskCompleted` | (none) | yes (exit 2 or prompt/agent `ok:false`) |
| Async | `ConfigChange` | source (`user_settings\|project_settings\|local_settings\|policy_settings\|skills`) | yes |
| Async | `CwdChanged` | (none) | no |
| Async | `FileChanged` | literal filenames (`.envrc\|.env`) | no |
| Async | `InstructionsLoaded` | load_reason (`session_start\|nested_traversal\|path_glob_match\|include\|compact`) | no |
| Worktree | `WorktreeCreate` | (none) | no |
| Worktree | `WorktreeRemove` | (none) | no |
| Compact | `PreCompact` | trigger (`manual\|auto`) | no |
| Compact | `PostCompact` | trigger (`manual\|auto`) | no |
| MCP | `Elicitation` | MCP server name | no |
| MCP | `ElicitationResult` | MCP server name | no |
| Once per session | `SessionEnd` | reason (`clear\|resume\|logout\|prompt_input_exit\|bypass_permissions_disabled\|other`) | no |

### Matcher resolution rules

| Matcher value | Evaluated as |
|---------------|--------------|
| `"*"`, `""`, or omitted | match all |
| Only `[A-Za-z0-9_\| -,]` (no hyphens before v2.1.195) | exact string, or `\|` / `,` separated list |
| Contains any other character | unanchored JavaScript regex (`RegExp.prototype.test`) |
| `FileChanged` and `StopFailure` | narrower exact set: `[A-Za-z0-9_\|]` only |

`Edit|Write` matches both tools; `Edit.*` matches `Edit` and `NotebookEdit` (wrap in `^…$` for whole-string match). MCP tools follow `mcp__<server>__<tool>` and need `mcp__memory__.*` to match a server.

The `if` field on individual handlers (v2.1.85+) is a separate permission-rule pre-filter (e.g. `Bash(git *)`); it runs after the group matcher and only applies to `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `PermissionDenied`.

### Handler types

| Type | Transport | Default timeout | Returns |
|------|-----------|-----------------|---------|
| `command` | shell | 600s (30s on `UserPromptSubmit`, 10s on `MessageDisplay`) | exit code + stdout JSON or plain text |
| `http` | POST request body | 600s | HTTP response body JSON (same shape as command) |
| `mcp_tool` | MCP server tool call | 600s | tool output (treated as command stdout) |
| `prompt` | single LLM call | 30s | `{ok: true|false, reason}` |
| `agent` | multi-turn agent (Read/Grep/Glob, up to 50 turns) | 60s | `{ok: true|false, reason}` |

All handlers accept `if`, `timeout`, `statusMessage`, and `once` (skills only). Command handlers additionally accept `command`, `args` (exec form), `async`, `asyncRewake`, and `shell`. HTTP handlers accept `url`, `headers`, `allowedEnvVars`. MCP-tool handlers accept `server`, `tool`, `input`. Prompt/agent handlers accept `prompt` and `model`.

### Per-event decision contract (current canonical shape)

| Event | Decision carrier | Blocking field | Mutation fields |
|-------|------------------|----------------|-----------------|
| `PreToolUse` | `hookSpecificOutput.permissionDecision` | `permissionDecision: deny` blocks; `allow`/`ask`/`defer` affect prompt flow | `updatedInput`, `additionalContext` |
| `PermissionRequest` | `hookSpecificOutput.decision.behavior` | `allow` / `deny` | `updatedInput`, `updatedPermissions` (setMode session), `message`, `interrupt` |
| `PermissionDenied` | `hookSpecificOutput.decision.behavior` or top-level `retry: true` | `retry` lets the model retry | `message` |
| `UserPromptSubmit`, `UserPromptExpansion`, `PostToolUse`, `PostToolUseFailure`, `SubagentStop`, `Stop` | top-level `decision: "block"` | block with `reason` | `additionalContext` |
| `PostToolUse` (MCP tools) | top-level `decision: "block"` | block with `reason` | `updatedMCPToolOutput`, `additionalContext` |
| `TeammateIdle`, `TaskCompleted` | exit code only | exit 2 blocks | n/a |
| `MessageDisplay`, `PostToolBatch`, `StopFailure`, `SessionStart`, `Setup`, `Notification`, `SubagentStart`, `PreCompact`, `PostCompact`, `CwdChanged`, `FileChanged`, `InstructionsLoaded`, `WorktreeCreate`, `WorktreeRemove`, `Elicitation`, `ElicitationResult`, `SessionEnd`, `ConfigChange` | (no decision control) | `ConfigChange` exit 2 / `{decision: "block"}` prevents the change | n/a |

Top-level `continue: false` overrides every event-specific decision and stops Claude entirely.

## Configuration

### Settings file locations and precedence

| Scope | macOS / Linux | Windows | Notes |
|-------|---------------|---------|-------|
| Managed (highest) | `/Library/Application Support/ClaudeCode/managed-settings.json` (macOS); `/etc/claude-code/managed-settings.json` (Linux/WSL) | `C:\Program Files\ClaudeCode\managed-settings.json` (v2.1.75+; `C:\ProgramData\…\managed-settings.json` deprecated) | Cannot be overridden by lower scopes |
| Managed — server-side | (n/a) | (n/a) | Claude.ai admin console or self-hosted Claude apps gateway |
| Managed — MDM plist | `com.anthropic.claudecode` managed preferences domain | n/a | Jamf, Iru/Kandji |
| Managed — registry | n/a | `HKLM\SOFTWARE\Policies\ClaudeCode` (REG_SZ `Settings` JSON), `HKCU\SOFTWARE\Policies\ClaudeCode` (lowest policy priority) | Group Policy, Intune |
| Local | `.claude/settings.local.json` | `.claude/settings.local.json` | Gitignored |
| Project | `.claude/settings.json` | `.claude/settings.json` | Committed |
| User (lowest) | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` | Local |
| Plugin | `<plugin>/hooks/hooks.json` | same | Loaded when plugin enabled |
| Skill / subagent frontmatter | `hooks:` block | same | Scoped to component lifetime |

`/hooks` (in-session) opens a read-only browser. The `disableAllHooks` setting disables all hooks; managed hooks still run unless `disableAllHooks` is set in managed settings. `allowManagedHooksOnly` (managed only) blocks user/project/plugin hooks (except plugins force-enabled in `enabledPlugins`). `allowedHttpHookUrls` (managed) is the URL allowlist for HTTP hooks. Per-session, `--bare` skips hooks and sets `CLAUDE_CODE_SIMPLE=1`; `--safe-mode` disables hooks and all other customizations while still honoring managed policy and sets `CLAUDE_CODE_SAFE_MODE=1`; `--setting-sources user,project,local` limits which settings files (and therefore which hooks) are loaded. `--include-hook-events` emits hook lifecycle events in `--output-format=stream-json` output.

A live file watcher reloads most settings edits including `hooks` mid-session; the `ConfigChange` hook fires for each detected change. `model` and `outputStyle` still apply on the next restart.

The `[JSON schema for settings.json](https://json.schemastore.org/claude-code-settings.json)` is published at [schemastore.org](https://json.schemastore.org/) — add `"$schema": "https://json.schemastore.org/claude-code-settings.json"` to your settings file for editor validation.

### Hook configuration shape

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "if": "Bash(git *)",
            "command": "${CLAUDE_PROJECT_DIR}/.claude/hooks/check-git.sh",
            "args": [],
            "timeout": 30,
            "shell": "bash",
            "async": false,
            "asyncRewake": false,
            "statusMessage": "Checking git policy",
            "once": false
          },
          {
            "type": "http",
            "url": "http://localhost:8080/hooks/pre-tool-use",
            "headers": { "Authorization": "Bearer $MY_TOKEN" },
            "allowedEnvVars": ["MY_TOKEN"]
          },
          {
            "type": "mcp_tool",
            "server": "my_server",
            "tool": "security_scan",
            "input": { "file_path": "${tool_input.file_path}" }
          },
          {
            "type": "prompt",
            "prompt": "Check safety: $ARGUMENTS",
            "model": "claude-haiku-4-5",
            "timeout": 30
          },
          {
            "type": "agent",
            "prompt": "Verify tests pass: $ARGUMENTS",
            "model": "claude-sonnet-5",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

### Path placeholders (exec and shell form)

- `${CLAUDE_PROJECT_DIR}` — project root (also exported as `CLAUDE_PROJECT_DIR` to stdio MCP servers and plugin LSP servers).
- `${CLAUDE_PLUGIN_ROOT}` — plugin install directory (changes on each plugin update).
- `${CLAUDE_PLUGIN_DATA}` — plugin persistent data directory.

In **exec form** (`args` set) the `command` is the executable and `args` is the argument vector: no shell tokenization, so spaces and `$` pass through verbatim. Prefer exec form when using path placeholders.

In **shell form** (`args` absent), the `command` is passed to `sh -c` on macOS/Linux, Git Bash on Windows, or PowerShell when Git Bash is unavailable (set `shell: "powershell"` to force PowerShell). Quote every placeholder in shell form.

## Payloads and Responses

### Common input fields (every event)

```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse"
}
```

`permission_mode` ∈ `default | plan | acceptEdits | dontAsk | bypassPermissions` (and `auto` from v2.1.111). Events add event-specific fields described in the schema frontmatter.

### Common output fields (exit 0)

```json
{
  "continue": true,
  "stopReason": "shown to user when continue=false (not Claude)",
  "suppressOutput": false,
  "systemMessage": "warning shown to user"
}
```

`continue: false` stops Claude entirely regardless of any event-specific decision.

### Per-event input highlights

- **SessionStart / Setup / UserPromptSubmit / UserPromptExpansion** — `prompt` / `source` / `command` fields drive matcher filtering and event identity.
- **PreToolUse / PostToolUse / PostToolUseFailure / PermissionRequest / PermissionDenied** — `tool_name` drives the group matcher; `tool_input` shape varies per tool (Bash: `{command, description?, timeout?, run_in_background?}`; Write: `{file_path, content}`; Edit: `{file_path, old_string, new_string, replace_all?}`; Read: `{file_path, offset?, limit?}`; Glob: `{pattern, path?}`; Grep: `{pattern, path?, glob?, output_mode?, -i?, multiline?}`; WebFetch: `{url, prompt}`; WebSearch: `{query, allowed_domains?, blocked_domains?}`; Task: `{prompt, description?, subagent_type?, model?}`; MCP: `{<server-specific>}`).
- **PostToolUse** adds `tool_use_id` (correlates with PreToolUse) and `tool_response` (tool-specific).
- **PostToolUseFailure** adds `error` (description) and `is_interrupt` (whether the failure was caused by user interruption).
- **Notification** adds `notification_type` and optional `title`; matches on the type.
- **SubagentStart** adds `agent_id`, `agent_type`; **SubagentStop** adds `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path` (subagent transcript in a nested `subagents/` folder).
- **TaskCreated** adds `task_id`, `task_subject`, `task_description?`, `task_status`; **TaskCompleted** adds `task_id`, `task_subject`, `task_description?`, `teammate_name?`, `team_name?`.
- **PreCompact / PostCompact** add `trigger` (`manual`/`auto`); PreCompact also adds `custom_instructions`.
- **SessionEnd** adds `reason` (`clear|resume|logout|prompt_input_exit|bypass_permissions_disabled|other`).
- **StopFailure** adds `error_type` (`rate_limit|overloaded|authentication_failed|oauth_org_not_allowed|billing_error|invalid_request|model_not_found|server_error|max_output_tokens|unknown`).
- **CwdChanged** adds `old_cwd`, `new_cwd`. **FileChanged** adds `file_path`, `change_type`. **InstructionsLoaded** adds `file_path`, `load_reason` (`session_start|nested_traversal|path_glob_match|include|compact`). **ConfigChange** adds `source` (`user_settings|project_settings|local_settings|policy_settings|skills`), `file_path`, `change_type`. **WorktreeCreate** adds `worktree_path`, `branch`. **WorktreeRemove** adds `worktree_path`. **Elicitation** adds `server_name`, `elicitation_id`, `message`, `requested_schema`. **ElicitationResult** adds `server_name`, `elicitation_id`, `action`, `content`.

### Per-event output highlights

- **PreToolUse** — `{hookSpecificOutput: {hookEventName, permissionDecision: allow|deny|ask|defer, permissionDecisionReason, updatedInput, additionalContext}}` (top-level `{decision, reason}` deprecated).
- **PermissionRequest** — `{hookSpecificOutput: {hookEventName, decision: {behavior: allow|deny, updatedInput, updatedPermissions: [{type: setMode, mode, destination: session}], message, interrupt}}}`.
- **PermissionDenied** — `{hookSpecificOutput: {hookEventName, decision: {behavior: retry, message}}}` or top-level `{retry: true}`.
- **UserPromptSubmit / UserPromptExpansion / PostToolUse / SubagentStop / Stop** — top-level `{decision: "block", reason}`; also accepts `{hookSpecificOutput: {hookEventName, additionalContext}}`. PostToolUse also accepts `updatedMCPToolOutput` for MCP tools.
- **PostToolUseFailure** — `{hookSpecificOutput: {hookEventName, additionalContext}}`.
- **TeammateIdle / TaskCompleted** — exit code only (`exit 2` blocks; stderr becomes feedback).
- **ConfigChange** — exit 2 or `{decision: "block"}` prevents the change from taking effect.
- **FileChanged** — also accepts `watchPaths` to extend the watch list.
- **Notification / SubagentStart / SessionStart / Setup / MessageDisplay / PostToolBatch / StopFailure / PreCompact / PostCompact / CwdChanged / InstructionsLoaded / WorktreeCreate / WorktreeRemove / Elicitation / ElicitationResult / SessionEnd** — output is informational only.
- **Prompt / agent hooks** — `{ok: true|false, reason: "..."}`. Effect varies by event (see Prompt and agent hooks below).

## Execution Semantics

### Shell, cwd, environment, timeout

- **Shell** — defaults to `sh -c` on macOS/Linux, Git Bash on Windows, PowerShell when Git Bash is unavailable. Pin via the `shell` field (`"bash"` or `"powershell"`).
- **Exec form** (`args` set) spawns the executable directly with no shell. Path placeholders are substituted as plain strings. On Windows, exec form requires a real `.exe` (`.cmd` / `.bat` shims from npm/npx/eslint must be invoked via `node` directly or in shell form).
- **cwd** — handlers run in the session's current working directory.
- **Environment** — Claude Code's env is exported; `CLAUDE_PROJECT_DIR`, `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` are always set; `CLAUDE_CODE_REMOTE` is `"true"` in web environments (unset locally); `CLAUDE_CODE_BRIDGE_SESSION_ID` is set while a Remote Control connection is active (v2.1.199+); `CLAUDE_CODE_SIMPLE` is set to `"1"` by `--bare` (hooks skipped); `CLAUDE_CODE_SAFE_MODE` is set to `"1"` by `--safe-mode` (hooks and other customizations disabled, managed policy still applies); `CLAUDE_ENV_FILE` is exposed **only** to `SessionStart` hooks and is the file Claude Code sources as a Bash preamble to persist direnv-style env-var changes.
- **Timeout** — default 600s for `command`/`http`/`mcp_tool`, 30s for `prompt`, 60s for `agent`. `UserPromptSubmit` lowers command/http/mcp_tool defaults to 30s. `MessageDisplay` lowers it to 10s. Async hooks share the 10-minute default. Per-handler `timeout` field overrides.

### Stdin / stdout / stderr

- **Stdin** — JSON event payload (one document).
- **Stdout** — exit 0: parsed as JSON output (top-level `continue`/`stopReason`/`suppressOutput`/`systemMessage`, plus event-specific `hookSpecificOutput`). Plain text becomes Claude context for `UserPromptSubmit`, `UserPromptExpansion`, `SessionStart`, `Setup`; becomes `systemMessage` on async hooks. Empty / non-JSON exit-0 is treated as **no decision** — for `PreToolUse` this does **not** approve the call (normal permission flow still applies).
- **Stderr** — exit 2: becomes Claude's feedback (PreToolUse, PostToolUse, PostToolUseFailure, UserPromptSubmit, UserPromptExpansion, Stop, SubagentStop, PermissionRequest, TeammateIdle, TaskCompleted). Any other non-zero exit: `<hook name> hook error` notice with the first line of stderr in the transcript, full stderr in the debug log. For non-blockable events (SessionStart, Setup, Notification, SubagentStart, etc.) exit 2 shows stderr to the user only.
- **HTTP hooks** — non-2xx, connection failures, and timeouts produce non-blocking errors (action continues). To block, return 2xx with `decision: block` or `permissionDecision: deny`.

### Async hooks

`async: true` on command hooks spawns the process in the background; Claude continues without waiting. `asyncRewake: true` (implies `async`) wakes Claude on exit code 2 — stderr (or stdout if stderr is empty) is shown as a system reminder so Claude can react to a long-running background failure. Async hooks cannot return decisions (`permissionDecision`, `continue`, `decision`) and have no deduplication across multiple fires. Output is delivered on the next interaction.

### Debug

`claude --debug "hooks"` enables hook-execution logging; look for `[DEBUG] Executing hooks for PreToolUse:Bash` and the matched handler details. `Ctrl+O` toggles verbose mode in the transcript.

## Claudine Mapping

The adapter must perform a many-to-one mapping from the 34 Claude Code events into Claudine's 16-event unified lifecycle, with provider-specific extensions preserved on the payload.

| Claude Code event | Claudine event | Notes |
|-------------------|----------------|-------|
| `SessionStart` | `initialize` | Carries `source` as `kind`; `compact` indicates post-compaction re-injection. |
| `Setup` | `initialize` | Provider-extension with `trigger` (`init`/`maintenance`); fires only on `--init-only`/`--init`/`--maintenance` in `-p` mode. |
| `InstructionsLoaded` | `initialize` | Provider-extension with `load_reason` (`session_start`/`nested_traversal`/`path_glob_match`/`include`/`compact`). |
| `UserPromptSubmit` | `prompt` | Blockable; preserves `prompt` text. |
| `UserPromptExpansion` | `prompt` | Provider-extension with `command` and `expanded_prompt`. |
| `Elicitation` | `prompt` | Provider-extension with `server_name`/`elicitation_id`/`requested_schema`. |
| `PreToolUse` | `tool_call` | Blockable; carries `tool_name` and polymorphic `tool_input`. Permission-decision variant of `permission`. |
| `TaskCreated` | `tool_call` | Provider-extension with `task_id`/`task_subject`/`task_status`. |
| `WorktreeCreate` | `tool_call` | Provider-extension with `worktree_path`/`branch`. |
| `PostToolUse` | `tool_result` | Post-event; can warn Claude but cannot reverse. |
| `PostToolUseFailure` | `tool_result` | Provider-extension with `error` and `is_interrupt`. |
| `PostToolBatch` | `tool_result` | Fires once per batch of parallel tool calls. |
| `TaskCompleted` | `tool_result` | Blockable (exit 2 / `ok:false`); carries `task_id`/`task_subject`. |
| `WorktreeRemove` | `tool_result` | Provider-extension with `worktree_path`. |
| `ElicitationResult` | `tool_result` | Provider-extension with `server_name`/`action`/`content`. |
| `Notification` | `notification` | Async; carries `notification_type` as `kind`. |
| `MessageDisplay` | `notification` | `around` timing; lower 10s default timeout; display-only. |
| `ConfigChange` | `notification` | Blockable (exit 2 / `decision: block`) on settings/skills file changes mid-session. |
| `CwdChanged` | `notification` | Async; carries `old_cwd`/`new_cwd`; primary entry point for direnv-style env management. |
| `FileChanged` | `notification` | Async; carries literal filenames via matcher. |
| `PreCompact` | `notification` | Pre-event; carries `trigger` and `custom_instructions`. |
| `PostCompact` | `notification` | Post-event counterpart of PreCompact. |
| `PermissionRequest` | `permission` | Does NOT fire in non-interactive `-p` mode; PermissionRequest is a UI-side permission dialog, PreToolUse is the headless permission gate. |
| `PermissionDenied` | `permission` | Auto-mode classifier denial; `{retry: true}` / `{decision.behavior: retry}` tells the model to retry. |
| `SubagentStart` | `subagent_start` | Carries `agent_id` and `agent_type`. |
| `SubagentStop` | `subagent_stop` | Blockable (exit 2 / `decision: block`); carries `stop_hook_active`, `agent_transcript_path`. |
| `TeammateIdle` | `subagent_stop` | Blockable via exit 2 only; command hooks only. |
| `Stop` | `finalize` | Blockable; checks `stop_hook_active` to avoid infinite loops. |
| `StopFailure` | `failure` | Output ignored; carries `error_type` as the typed discriminator. |
| `SessionEnd` | `finalize` | Post-event counterpart of Stop; carries `reason` as `kind`. |

Provider-specific discriminator fields should be exposed as typed subfields on the unified payload (not opaque blobs): `permission_mode` (every event), `notification_type`, `load_reason`, `agent_type`, `agent_transcript_path`, `stop_hook_active`, `error_type` (`StopFailure`), `is_interrupt` (`PostToolUseFailure`), `source` (`SessionStart`), `trigger` (`PreCompact`/`PostCompact`/`Setup`), `reason` (`SessionEnd`), `worktree_path` (`WorktreeCreate`/`WorktreeRemove`), `task_*` (`TaskCreated`/`TaskCompleted`), `server_name`/`elicitation_id`/`requested_schema` (`Elicitation`/`ElicitationResult`), `command`/`expanded_prompt` (`UserPromptExpansion`).

CLAUDE_ENV_FILE writes from `SessionStart` should be modeled as a `permission` / `tool_call` session-side-effect (a Bash preamble that persists env vars) rather than as a hook output.

## Gaps

1. **34→16 mapping** — many-to-one requires explicit disambiguation in the adapter; see the table above.
2. **Five handler types** — `command`, `http`, `mcp_tool`, `prompt`, `agent`; the existing Claudine adapter models only `command`. `prompt` and `agent` are LLM-in-the-loop and need a different side-effect surface; `http` and `mcp_tool` need network/MCP client abstractions.
3. **Timing semantics** — `around` (`MessageDisplay`) and `async` (`Notification`, `FileChanged`, `CwdChanged`, `ConfigChange`, `PostCompact`) are not modeled in the unified event timing taxonomy.
4. **Matcher / `if` rule parsing** — needs both regex/list matcher parsing and permission-rule pre-filter parsing per handler.
5. **Polymorphic tool_input** — needs a tool_name discriminator plus per-tool schemas for the 11 native tools plus MCP tools.
6. **PreToolUse legacy fields** — top-level `decision`/`reason` is deprecated; adapter must model both shapes.
7. **Permission event gating** — `PermissionRequest` is suppressed in `-p` mode; headless runs need `PreToolUse` as the permission gate instead.
8. **Per-event handler-type constraints** — `TeammateIdle` only accepts `type: command`; SessionStart's CLAUDE_ENV_FILE is unique to Claude; validate the handler type per event before dispatching.
9. **Managed policy gates** — `allowManagedHooksOnly`, `allowedHttpHookUrls`, `disableAllHooks`, and `allowAllClaudeAiMcps` have no analogue on other providers; model as Claude-specific session gates.
10. **Existing research file** — `claude-code.md` (1232 lines) duplicates this content but is missing all schema fields. It should be deleted once the adapter migrates to this 34-event map.
11. **Per-session hook-disable toggles** — `--bare`/`CLAUDE_CODE_SIMPLE`, `--safe-mode`/`CLAUDE_CODE_SAFE_MODE`, and `--setting-sources` are not yet modeled as Claudine session gates; only the static `disableAllHooks` setting is represented.

## Changelog

- **2026-07-03** — Refreshed against Claude Code v2.1.199 and official docs. Added `--safe-mode`, `--include-hook-events`, and `--setting-sources` as hook-affecting CLI controls; documented `CLAUDE_CODE_SAFE_MODE` and `CLAUDE_CODE_SIMPLE` environment disables; split all `os: all` config records into per-OS (macOS/Linux/Windows) entries; mapped native `permissionDecision: defer` to `response_actions: other` to comply with the schema action enum.
- **2026-07-02** — Initial research for `claude.md`. Expanded event list from 14 to 34; added `http` and `mcp_tool` handler types; added `if` field, exec form, `shell` field, `asyncRewake`, and `allowedEnvVars`. Documented `permissionDecision: defer` (headless `-p` mode), `allowedHttpHookUrls` policy, HKCU/HKLM/macOS plist delivery, and `managed-settings.d/` drop-in support. Captured per-handler-type timeouts (command 600s, prompt 30s, agent 60s; UserPromptSubmit 30s override; MessageDisplay 10s override). Recorded auto-update model/outputStyle restart-only behavior and live-watcher reload for everything else.

## Sources

- Hooks reference: <https://code.claude.com/docs/en/hooks>
- Hooks guide: <https://code.claude.com/docs/en/hooks-guide>
- Settings reference: <https://code.claude.com/docs/en/settings>
- CLI reference: <https://code.claude.com/docs/en/cli-reference>
- Environment variables: <https://code.claude.com/docs/en/env-vars>
- Permissions: <https://code.claude.com/docs/en/permissions>
- Subagents: <https://code.claude.com/docs/en/sub-agents>
- Plugins: <https://code.claude.com/docs/en/plugins>
- Skills: <https://code.claude.com/docs/en/skills>
- MCP: <https://code.claude.com/docs/en/mcp>
- Agent teams: <https://code.claude.com/docs/en/agent-teams>
- Headless / `-p` mode: <https://code.claude.com/docs/en/headless>
- Managed settings: <https://code.claude.com/docs/en/server-managed-settings>
- Managed MCP: <https://code.claude.com/docs/en/managed-mcp>
- Settings JSON schema: <https://json.schemastore.org/claude-code-settings.json>
- Permissions reference (managed-only): <https://code.claude.com/docs/en/permissions#managed-only-settings>
- File example (Bash command validator): <https://github.com/anthropics/claude-code/blob/main/examples/hooks/bash_command_validator_example.py>
- Awesome Claude Code (community hooks): <https://github.com/hesreallyhim/awesome-claude-code>
- Claude Code Hooks Mastery: <https://github.com/disler/claude-code-hooks-mastery>