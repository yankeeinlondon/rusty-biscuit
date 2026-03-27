---
homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/hooks-guide
hooks: https://code.claude.com/docs/en/hooks
---

# Claude Code hooks and events

## Home Page

<https://www.anthropic.com/claude-code>

## Documentation

- Hooks guide (quickstart and examples): <https://code.claude.com/docs/en/hooks-guide>
- Hooks reference (schemas, events, JSON formats): <https://code.claude.com/docs/en/hooks>
- Settings reference: <https://code.claude.com/docs/en/settings>

## Scope

This document covers the hook and event system available in Claude Code (Anthropic's agentic CLI). Claude Code provides 14 lifecycle hook events that allow shell scripts, LLM prompts, and agent-based validators to intercept, modify, or block actions at key points in the agentic workflow. Sources are cited inline.

## Configuration

Hooks are defined in JSON settings files under a top-level `hooks` key. Multiple configuration locations are supported, merged by priority. The configuration has three levels of nesting:

1. Choose a hook event to respond to (e.g., `PreToolUse`, `Stop`)
2. Add a matcher group to filter when it fires (e.g., "only for the Bash tool")
3. Define one or more hook handlers to run when matched

### Settings file locations

| Location | Scope | Shareable | Priority |
|----------|-------|-----------|----------|
| `~/.claude/settings.json` | All projects | No (local) | Low (user) |
| `.claude/settings.json` | Single project | Yes (committed) | Medium (project) |
| `.claude/settings.local.json` | Single project | No (gitignored) | High (local) |
| Plugin `hooks/hooks.json` | When plugin enabled | Yes | Medium (plugin) |
| Skill or agent frontmatter | While component active | Yes (defined in file) | Scoped to component |
| Managed policy settings (`managed-settings.json`) | Organization | Admin-controlled | Highest (can override all) |

Managed settings paths:
- macOS: `/Library/Application Support/ClaudeCode/managed-settings.json`
- Linux/WSL: `/etc/claude-code/managed-settings.json`
- Windows: `C:\Program Files\ClaudeCode\managed-settings.json`

Setting `"disableAllHooks": true` in any settings file disables all hooks. Enterprise administrators can set `"allowManagedHooksOnly": true` in managed settings to block user, project, and plugin hooks, allowing only managed and SDK hooks.

### Hook configuration schema

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "regex pattern (optional)",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/script.sh",
            "timeout": 600,
            "statusMessage": "Running validation",
            "async": false,
            "once": false
          },
          {
            "type": "prompt",
            "prompt": "Evaluate this: $ARGUMENTS",
            "model": "claude-haiku-4-5",
            "timeout": 30
          },
          {
            "type": "agent",
            "prompt": "Verify safety: $ARGUMENTS",
            "model": "claude-opus-4-5",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

### Hook handler types

1. **Command** (`type: "command"`): Executes a shell command. Receives JSON on stdin, returns JSON or plain text on stdout.
2. **Prompt** (`type: "prompt"`): Sends a single LLM call. The `$ARGUMENTS` placeholder is replaced with the hook's input JSON. Returns `{"ok": true|false, "reason": "..."}`. Supported events: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `UserPromptSubmit`, `Stop`, `SubagentStop`, `TaskCompleted`. Not supported by `TeammateIdle`.
3. **Agent** (`type: "agent"`): Spawns a multi-turn agent with tool access (Read, Grep, Glob, etc.). Same return format as prompt hooks but can perform file reads and searches (up to 50 turns). Same event support as prompt hooks.

### Handler fields

#### Common fields (all types)

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `type` | Yes | - | `"command"`, `"prompt"`, or `"agent"` |
| `timeout` | No | 600s / 30s / 60s | Seconds before canceling (command / prompt / agent) |
| `statusMessage` | No | - | Custom spinner message displayed during execution |
| `once` | No | false | Run only once per session (skills only, not agents) |

#### Command-specific fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `command` | Yes | - | Shell command to execute |
| `async` | No | false | Run in background without blocking |

#### Prompt and agent-specific fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `prompt` | Yes | - | Prompt text; `$ARGUMENTS` is replaced with input JSON. If `$ARGUMENTS` is absent, input JSON is appended |
| `model` | No | Fast model | LLM model to use |

All matching hooks run in parallel, and identical handlers are deduplicated automatically.

### Hooks in skills and agents

Hooks can be defined directly in skill and sub-agent YAML frontmatter, scoped to the component's lifecycle. All hook events are supported. For sub-agents, `Stop` hooks are automatically converted to `SubagentStop`.

```yaml
---
name: secure-operations
description: Perform operations with security checks
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/security-check.sh"
---
```

### Environment variables available in hooks

| Variable | Availability | Description |
|----------|-------------|-------------|
| `CLAUDE_PROJECT_DIR` | All hooks | Absolute path to project root |
| `CLAUDE_PLUGIN_ROOT` | Plugin hooks | Plugin directory |
| `CLAUDE_ENV_FILE` | SessionStart only | File path for persisting env vars to subsequent Bash calls |
| `CLAUDE_CODE_REMOTE` | All hooks | `"true"` in web/remote mode, unset in local CLI |

## Matcher system

Matchers are optional **regex patterns** that filter when hooks fire. They are case-sensitive. Omit `matcher`, set it to `"*"`, or set it to `""` to match all occurrences.

The matcher runs against a specific field from the JSON input that Claude Code sends to the hook on stdin. For tool events, that field is `tool_name`.

| Event type | Matches against | Example values |
|-----------|-----------------|----------|
| `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest` | Tool name | `Bash`, `Edit\|Write`, `mcp__.*`, `Notebook.*` |
| `SessionStart` | Startup source | `startup`, `resume`, `clear`, `compact` |
| `SessionEnd` | Exit reason | `clear`, `logout`, `prompt_input_exit`, `bypass_permissions_disabled`, `other` |
| `Notification` | Notification type | `permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog` |
| `SubagentStart`, `SubagentStop` | Agent type | `Bash`, `Explore`, `Plan`, or custom agent names |
| `PreCompact` | Trigger type | `manual`, `auto` |
| `UserPromptSubmit`, `Stop`, `TeammateIdle`, `TaskCompleted` | Not supported | Fires on every occurrence; matcher is silently ignored |

### MCP tool matching

MCP tools appear as regular tools in tool events and use the naming pattern `mcp__<server>__<tool>`:

- `mcp__memory__create_entities` -- Memory server's create entities tool
- `mcp__filesystem__read_file` -- Filesystem server's read file tool
- `mcp__github__search_repositories` -- GitHub server's search tool

Regex patterns for MCP tools:
- `mcp__memory__.*` -- all tools from the `memory` server
- `mcp__.*__write.*` -- any tool containing "write" from any server

## Exit codes

| Exit code | Behavior | JSON processed | Use for |
|-----------|----------|----------------|---------|
| `0` | Success, action proceeds | Yes | Allow action, optionally with JSON control |
| `2` | Blocking error, action prevented | No (stderr used instead) | Block action; stderr becomes Claude's feedback |
| Other | Non-blocking error | No | Unexpected error; stderr shown in verbose mode |

JSON output is **only** processed on exit 0. If you exit 2, any JSON on stdout is ignored. You must choose one approach per hook: either exit codes alone for signaling, or exit 0 and print JSON for structured control.

### Exit code 2 behavior by event

| Event | Can block | Exit 2 effect |
|-------|-----------|---------------|
| PreToolUse | Yes | Blocks tool call |
| PermissionRequest | Yes | Denies permission |
| UserPromptSubmit | Yes | Blocks and erases prompt from context |
| Stop | Yes | Prevents stopping, continues conversation |
| SubagentStop | Yes | Prevents subagent from stopping |
| TeammateIdle | Yes | Prevents teammate from going idle (continues working) |
| TaskCompleted | Yes | Prevents task from being marked as completed |
| PostToolUse | No | Shows stderr to Claude (action already ran) |
| PostToolUseFailure | No | Shows stderr to Claude |
| Notification | No | Shows stderr to user only |
| SubagentStart | No | Shows stderr to user only |
| SessionStart | No | Shows stderr to user only |
| SessionEnd | No | Shows stderr to user only |
| PreCompact | No | Shows stderr to user only |

## Common JSON output fields

All events support these top-level output fields on exit 0:

```json
{
  "continue": true,
  "stopReason": "Message shown to user when continue=false (not shown to Claude)",
  "suppressOutput": false,
  "systemMessage": "Warning shown to user"
}
```

Setting `"continue": false` stops Claude entirely, regardless of event-specific decision fields. It takes precedence over any other output.

### Decision control patterns

Different events use different decision patterns:

| Events | Decision pattern | Key fields |
|--------|-----------------|------------|
| `UserPromptSubmit`, `PostToolUse`, `PostToolUseFailure`, `Stop`, `SubagentStop` | Top-level `decision` | `decision: "block"`, `reason` |
| `TeammateIdle`, `TaskCompleted` | Exit code only | Exit 2 blocks; stderr becomes feedback |
| `PreToolUse` | `hookSpecificOutput` | `permissionDecision` (allow/deny/ask), `permissionDecisionReason` |
| `PermissionRequest` | `hookSpecificOutput` | `decision.behavior` (allow/deny) |

## Common input fields

All hook events receive these fields via stdin as JSON, in addition to event-specific fields:

| Field | Description |
|-------|-------------|
| `session_id` | Current session identifier |
| `transcript_path` | Path to conversation JSONL transcript |
| `cwd` | Current working directory when the hook was invoked |
| `permission_mode` | Current permission mode: `"default"`, `"plan"`, `"acceptEdits"`, `"dontAsk"`, or `"bypassPermissions"` |
| `hook_event_name` | Name of the event that fired |

## Hook events

### SessionStart

**Triggers:** New session, resumed session, `/clear` command, or post-compaction.

**Matcher values:** `startup`, `resume`, `clear`, `compact`

**Can block:** No

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "SessionStart",
  "source": "startup",
  "model": "claude-sonnet-4-6",
  "agent_type": "my-agent"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `source` | string | How the session started: `"startup"`, `"resume"`, `"clear"`, `"compact"` |
| `model` | string | The model identifier being used |
| `agent_type` | string (optional) | Present only when started with `claude --agent <name>` |

**Response:** Can inject context via stdout text or JSON `additionalContext`. The `CLAUDE_ENV_FILE` variable allows persisting environment variables to subsequent Bash commands:

```bash
#!/bin/bash
if [ -n "$CLAUDE_ENV_FILE" ]; then
  echo 'export NODE_ENV=production' >> "$CLAUDE_ENV_FILE"
fi
exit 0
```

To capture environment changes from setup commands:

```bash
#!/bin/bash
ENV_BEFORE=$(export -p | sort)
source ~/.nvm/nvm.sh
nvm use 20
if [ -n "$CLAUDE_ENV_FILE" ]; then
  ENV_AFTER=$(export -p | sort)
  comm -13 <(echo "$ENV_BEFORE") <(echo "$ENV_AFTER") >> "$CLAUDE_ENV_FILE"
fi
exit 0
```

**Gotchas:**
- `CLAUDE_ENV_FILE` is only available in SessionStart hooks; other hook types do not have access.
- Auto-compaction triggers SessionStart with `source: "compact"`, which can cause expensive hooks to run frequently. Use the `compact` matcher or check `source` inside the hook to skip expensive operations during compaction.

### UserPromptSubmit

**Triggers:** When the user submits a prompt, before Claude processes it.

**Matcher values:** None (fires on every prompt)

**Can block:** Yes

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "User's submitted text"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `prompt` | string | The text the user submitted |

**Response:**

Two ways to add context on exit 0:
- **Plain text stdout**: any non-JSON text is added as context.
- **JSON with `additionalContext`**: more structured control.

To block a prompt, return `decision: "block"`:

```json
{
  "decision": "block",
  "reason": "Explanation shown to the user (not added to context)",
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "Text added to Claude's context"
  }
}
```

| Field | Description |
|-------|-------------|
| `decision` | `"block"` prevents processing and erases from context. Omit to allow |
| `reason` | Shown to user when blocking. Not added to context |
| `additionalContext` | String added to Claude's context |

`exit 2` also blocks the prompt and erases it from context; stderr becomes feedback.

**Gotchas:**
- Does not support matchers. If you add a `matcher` field, it is silently ignored.

### PreToolUse

**Triggers:** After Claude creates tool parameters, before tool execution.

**Matchable tools:** `Bash`, `Edit`, `Write`, `Read`, `Glob`, `Grep`, `Task`, `WebFetch`, `WebSearch`, and MCP tools (`mcp__<server>__<tool>`).

**Can block:** Yes

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_use_id": "toolu_01ABC123...",
  "tool_input": {
    "command": "npm test",
    "description": "Run test suite",
    "timeout": 120000
  }
}
```

**Tool-specific `tool_input` shapes:**

| Tool | Key fields |
|------|-----------|
| Bash | `command` (string), `description` (string, optional), `timeout` (number, optional, ms), `run_in_background` (boolean, optional) |
| Write | `file_path` (string), `content` (string) |
| Edit | `file_path` (string), `old_string` (string), `new_string` (string), `replace_all` (boolean, optional) |
| Read | `file_path` (string), `offset` (number, optional), `limit` (number, optional) |
| Glob | `pattern` (string), `path` (string, optional, defaults to cwd) |
| Grep | `pattern` (string), `path` (string, optional), `glob` (string, optional), `output_mode` (string, optional), `-i` (boolean, optional), `multiline` (boolean, optional) |
| WebFetch | `url` (string), `prompt` (string) |
| WebSearch | `query` (string), `allowed_domains` (array, optional), `blocked_domains` (array, optional) |
| Task | `prompt` (string), `description` (string, optional), `subagent_type` (string, optional), `model` (string, optional) |

**Response:** Three-level permission decision system via `hookSpecificOutput`:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "Safe command approved by hook",
    "updatedInput": {
      "command": "modified command"
    },
    "additionalContext": "Context for Claude before tool executes"
  }
}
```

| Field | Description |
|-------|-------------|
| `permissionDecision` | `"allow"` bypasses permission system, `"deny"` prevents tool call, `"ask"` shows user the permission dialog |
| `permissionDecisionReason` | For `"allow"` and `"ask"`: shown to user but not Claude. For `"deny"`: shown to Claude |
| `updatedInput` | Modifies tool input before execution. Combine with `"allow"` to auto-approve or `"ask"` to show modified input |
| `additionalContext` | String added to Claude's context before tool executes |

`exit 2` blocks the tool call; stderr becomes Claude's feedback.

**Gotchas:**
- PreToolUse previously used top-level `decision` and `reason` fields. These are **deprecated**. Use `hookSpecificOutput.permissionDecision` and `hookSpecificOutput.permissionDecisionReason` instead. The deprecated values `"approve"` and `"block"` map to `"allow"` and `"deny"` respectively.
- `updatedInput` only works for fields that exist in the tool's input schema. Adding invented fields has no effect.

### PermissionRequest

**Triggers:** When a permission dialog is about to appear to the user.

**Matcher values:** Tool name (same as PreToolUse)

**Can block:** Yes

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PermissionRequest",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf node_modules",
    "description": "Remove node_modules directory"
  },
  "permission_suggestions": [
    { "type": "toolAlwaysAllow", "tool": "Bash" }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | The tool requesting permission |
| `tool_input` | object | Same structure as PreToolUse (no `tool_use_id`) |
| `permission_suggestions` | array (optional) | "Always allow" options the user would see in the dialog |

**Response:**
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow",
      "updatedInput": { "command": "npm run lint" },
      "updatedPermissions": [],
      "message": "Why denied (deny only)",
      "interrupt": false
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `behavior` | `"allow"` grants permission, `"deny"` denies it |
| `updatedInput` | For `"allow"` only: modifies tool input before execution |
| `updatedPermissions` | For `"allow"` only: applies permission rule updates (equivalent to user selecting "always allow") |
| `message` | For `"deny"` only: tells Claude why permission was denied |
| `interrupt` | For `"deny"` only: if `true`, stops Claude |

`exit 2` denies permission; stderr becomes Claude's feedback.

**Gotchas:**
- PermissionRequest hooks do **not** fire in non-interactive/headless mode (`claude -p`), because the permission dialog is bypassed entirely. Use PreToolUse hooks for automated permission decisions in non-interactive mode.
- The difference from PreToolUse: PermissionRequest fires only when a permission dialog is about to be shown, whereas PreToolUse fires before every tool execution regardless of permission status.

### PostToolUse

**Triggers:** Immediately after a tool completes successfully.

**Matcher values:** Tool name (same as PreToolUse)

**Can block:** No (action already ran; feedback only)

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PostToolUse",
  "tool_name": "Write",
  "tool_use_id": "toolu_01ABC123...",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_response": {
    "filePath": "/path/to/file.txt",
    "success": true
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tool_name` | string | The tool that executed |
| `tool_use_id` | string | Unique identifier for this tool call |
| `tool_input` | object | Arguments sent to the tool |
| `tool_response` | object | Result returned by the tool (schema varies by tool) |

**Response:**
```json
{
  "decision": "block",
  "reason": "Why the action should be reconsidered",
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "Context for Claude",
    "updatedMCPToolOutput": "replacement output (MCP tools only)"
  }
}
```

| Field | Description |
|-------|-------------|
| `decision` | `"block"` prompts Claude with the reason. Omit to proceed normally |
| `reason` | Explanation shown to Claude when blocking |
| `additionalContext` | Additional context for Claude |
| `updatedMCPToolOutput` | For MCP tools only: replaces tool output that Claude sees |

**Gotchas:**
- The tool has already executed. Returning `decision: "block"` tells Claude the action was problematic but **cannot reverse it**. Use PreToolUse for preventive validation.
- If a PostToolUse hook modifies files (e.g., running a formatter), Claude receives system reminders about those changes, consuming context tokens. Mitigate with `"async": true`, `"suppressOutput": true`, or deferring to commit hooks.

### PostToolUseFailure

**Triggers:** When a tool execution fails with an error.

**Matcher values:** Tool name (same as PreToolUse)

**Can block:** No (tool already failed)

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PostToolUseFailure",
  "tool_name": "Bash",
  "tool_use_id": "toolu_01ABC123...",
  "tool_input": {
    "command": "npm test",
    "description": "Run test suite"
  },
  "error": "Command exited with non-zero status code 1",
  "is_interrupt": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `error` | string | Description of what went wrong |
| `is_interrupt` | boolean (optional) | Whether the failure was caused by user interruption |

**Response:**
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUseFailure",
    "additionalContext": "Recovery suggestions for Claude"
  }
}
```

Provides corrective context to Claude about the failure.

### Notification

**Triggers:** When Claude Code sends a notification.

**Matcher values:** `permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog`

**Can block:** No

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "Notification",
  "message": "Claude needs your permission to use Bash",
  "title": "Permission needed",
  "notification_type": "permission_prompt"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `message` | string | Notification text |
| `title` | string (optional) | Notification title |
| `notification_type` | string | Which type fired (used for matcher filtering) |

**Response:** Can add `additionalContext` to the conversation. Primarily used for external alerting (desktop notifications, Slack, etc.).

### SubagentStart

**Triggers:** When a subagent is spawned via the Task tool.

**Matcher values:** Agent type names (`Bash`, `Explore`, `Plan`, or custom agent names from `.claude/agents/`)

**Can block:** No

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "SubagentStart",
  "agent_id": "agent-abc123",
  "agent_type": "Explore"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `agent_id` | string | Unique identifier for the subagent |
| `agent_type` | string | Agent name (used for matcher filtering) |

**Response:**
```json
{
  "hookSpecificOutput": {
    "hookEventName": "SubagentStart",
    "additionalContext": "Context injected into the subagent"
  }
}
```

Injects context or instructions into the newly spawned subagent.

### SubagentStop

**Triggers:** When a subagent finishes responding.

**Matcher values:** Agent type (same as SubagentStart)

**Can block:** Yes

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../abc123.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "SubagentStop",
  "stop_hook_active": false,
  "agent_id": "def456",
  "agent_type": "Explore",
  "agent_transcript_path": "~/.claude/projects/.../abc123/subagents/agent-def456.jsonl"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `stop_hook_active` | boolean | `true` when subagent is already continuing due to a stop hook |
| `agent_id` | string | Unique identifier for the subagent |
| `agent_type` | string | Agent name (used for matcher filtering) |
| `agent_transcript_path` | string | Path to the subagent's own transcript (nested `subagents/` folder) |

**Response:** Same as Stop hooks (see below).

**Gotchas:**
- **Must check `stop_hook_active`** to prevent infinite loops (see Stop gotchas below).

### Stop

**Triggers:** When the main Claude agent finishes responding. Does not fire on user interrupts.

**Matcher values:** None (fires on every stop)

**Can block:** Yes

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "Stop",
  "stop_hook_active": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `stop_hook_active` | boolean | `true` when Claude is already continuing as a result of a stop hook |

**Response:**
```json
{
  "decision": "block",
  "reason": "Why Claude should continue working"
}
```

| Field | Description |
|-------|-------------|
| `decision` | `"block"` prevents stopping. Omit to allow stop |
| `reason` | Required when `decision` is `"block"`. Tells Claude why it should continue |

`exit 2` also prevents stopping; stderr becomes the reason.

**Gotchas:**
- **Infinite loops:** Stop hooks that do not check `stop_hook_active` cause infinite loops where Claude never stops. Every Stop and SubagentStop hook **must** check this field and exit immediately when true:

```bash
#!/bin/bash
INPUT=$(cat)
if [ "$(echo "$INPUT" | jq -r '.stop_hook_active')" = "true" ]; then
  exit 0  # Always allow stopping on subsequent calls
fi
# Your logic here
```

### TeammateIdle

**Triggers:** When an agent team teammate is about to go idle after finishing its turn.

**Matcher values:** None (fires on every occurrence; does not support matchers)

**Can block:** Yes (exit 2 prevents teammate from going idle)

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "TeammateIdle",
  "teammate_name": "researcher",
  "team_name": "my-project"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `teammate_name` | string | Name of the teammate about to go idle |
| `team_name` | string | Name of the team |

**Response:** Uses exit codes only, **not** JSON decision control. Exit 2 blocks; stderr is fed back as feedback to the teammate.

```bash
#!/bin/bash
if [ ! -f "./dist/output.js" ]; then
  echo "Build artifact missing. Run the build before stopping." >&2
  exit 2
fi
exit 0
```

**Gotchas:**
- Does **not** support prompt-based or agent-based hooks (`type: "prompt"` / `type: "agent"`). Only `type: "command"` works.
- Uses exit code control only. JSON decision fields like `decision: "block"` have no effect.

### TaskCompleted

**Triggers:** When a task is being marked as completed. Fires in two situations:
1. When any agent explicitly marks a task as completed through the TaskUpdate tool.
2. When an agent team teammate finishes its turn with in-progress tasks.

**Matcher values:** None (fires on every occurrence; does not support matchers)

**Can block:** Yes (exit 2 prevents the task from being marked as completed)

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "TaskCompleted",
  "task_id": "task-001",
  "task_subject": "Implement user authentication",
  "task_description": "Add login and signup endpoints",
  "teammate_name": "implementer",
  "team_name": "my-project"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | string | Identifier of the task being completed |
| `task_subject` | string | Title of the task |
| `task_description` | string (optional) | Detailed description of the task |
| `teammate_name` | string (optional) | Name of the teammate completing the task |
| `team_name` | string (optional) | Name of the team |

**Response:** Uses exit codes only, **not** JSON decision control. Exit 2 blocks; stderr is fed back as feedback.

```bash
#!/bin/bash
INPUT=$(cat)
TASK_SUBJECT=$(echo "$INPUT" | jq -r '.task_subject')
if ! npm test 2>&1; then
  echo "Tests not passing. Fix failing tests before completing: $TASK_SUBJECT" >&2
  exit 2
fi
exit 0
```

**Gotchas:**
- Supports prompt-based and agent-based hooks (unlike `TeammateIdle`), but exit code control is the primary mechanism.

### PreCompact

**Triggers:** Before context compaction occurs.

**Matcher values:** `manual` (from `/compact` command), `auto` (when context window fills)

**Can block:** No

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "PreCompact",
  "trigger": "manual",
  "custom_instructions": ""
}
```

| Field | Type | Description |
|-------|------|-------------|
| `trigger` | string | `"manual"` or `"auto"` |
| `custom_instructions` | string | For `manual`, contains what the user passed to `/compact`. For `auto`, empty string |

**Response:** Information only. Useful for logging when compaction occurs.

### SessionEnd

**Triggers:** When the session terminates.

**Matcher values:** `clear`, `logout`, `prompt_input_exit`, `bypass_permissions_disabled`, `other`

**Can block:** No

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "SessionEnd",
  "reason": "prompt_input_exit"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `reason` | string | Why the session ended (used for matcher filtering) |

**Response:** Cleanup only. Cannot prevent session termination.

## Async hooks

Command hooks support `"async": true` for non-blocking background execution.

```json
{
  "type": "command",
  "command": "long-running-task.sh",
  "async": true,
  "timeout": 300
}
```

### How async hooks work

1. Hook process spawns in the background.
2. Claude immediately continues without waiting.
3. When the hook completes, any `systemMessage` or `additionalContext` is delivered on the next conversation turn.

### Limitations

- Cannot block tool calls or return decisions (action already proceeded).
- Cannot return `decision`, `permissionDecision`, `continue`, or other control fields.
- Only `type: "command"` supports `async`; prompt and agent hooks cannot run asynchronously.
- No deduplication across multiple fires of the same async hook.
- Output is delivered on the next interaction (waits if session is idle).
- If not specified, async hooks use the same 10-minute default timeout as sync hooks.

### Valid use cases

Background test runs, logging, notifications, CI/CD triggers, cleanup tasks.

## Gotchas

### 1. Infinite Stop/SubagentStop loops

**Problem:** Stop hooks that don't check `stop_hook_active` cause infinite loops where Claude never stops.

**Solution:** Every Stop and SubagentStop hook must check this field and exit immediately when true:

```bash
#!/bin/bash
INPUT=$(cat)
if [ "$(echo "$INPUT" | jq -r '.stop_hook_active')" = "true" ]; then
  exit 0  # Always allow stopping on subsequent calls
fi
# Your logic here
```

### 2. Shell profile output breaks JSON parsing

**Problem:** If `~/.zshrc` or `~/.bashrc` contains unconditional `echo` statements, they run when the hook's shell spawns and corrupt the JSON output.

**Symptoms:** "JSON validation failed" errors despite valid JSON in your hook script.

**Solution:** Guard profile output with an interactive-shell check:

```bash
# ~/.zshrc
if [[ $- == *i* ]]; then
  echo "Shell ready"  # Only in interactive shells
fi
```

### 3. Context window noise from file-modifying hooks

**Problem:** If a PostToolUse hook modifies files (e.g., running a formatter), Claude receives system reminders about those changes, consuming context tokens.

**Workarounds:**
- Format on commit, not on every edit.
- Use `"suppressOutput": true` in the hook's JSON output.
- Use `"async": true` so the modification happens after Claude has moved on.
- Defer formatting to end of session.

### 4. Settings changes require review or restart

**Problem:** Editing a settings file while Claude Code is running does not take effect immediately. Claude Code captures a snapshot of hooks at startup and uses it throughout the session. This prevents malicious or accidental modifications from taking effect mid-session.

**Solution:** Hooks added through the `/hooks` menu take effect immediately. If hooks are modified externally, Claude Code warns you and requires review in the `/hooks` menu before changes apply. Alternatively, restart the session.

### 5. Matchers are case-sensitive regex

**Problem:** `"matcher": "bash"` won't match the `Bash` tool.

**Solution:** Use exact case (`"Bash"`) or a case-insensitive pattern (`"[Bb]ash"`).

### 6. No matcher support for UserPromptSubmit, Stop, TeammateIdle, and TaskCompleted

**Problem:** You cannot filter these hooks to fire only under certain conditions using matchers. Any `matcher` field on these events is silently ignored.

**Solution:** Check conditions inside your hook script using the JSON input fields.

### 7. Async hooks cannot control behavior

**Problem:** Setting `async: true` while expecting to block a tool call silently ignores decision fields.

**Solution:** Use synchronous hooks for any decision-making. Reserve async for fire-and-forget work.

### 8. PostToolUse cannot undo actions

**Problem:** PostToolUse fires after the tool has already executed. Returning `decision: "block"` tells Claude the action was problematic but cannot reverse it.

**Solution:** Use PreToolUse for preventive validation. Use PostToolUse for logging and post-hoc feedback.

### 9. PermissionRequest hooks don't fire in non-interactive mode

**Problem:** Using `claude -p` (non-interactive/headless mode) bypasses the permission dialog entirely, so PermissionRequest hooks never fire.

**Solution:** Use PreToolUse hooks for automated permission decisions in non-interactive mode.

### 10. SessionStart fires on every compaction

**Problem:** Auto-compaction triggers SessionStart with `source: "compact"`, which can cause expensive hooks to run frequently.

**Solution:** Use the `compact` matcher to separate compaction behavior from startup, or check `source` inside the hook:

```bash
#!/bin/bash
INPUT=$(cat)
SOURCE=$(echo "$INPUT" | jq -r '.source')
if [ "$SOURCE" = "compact" ]; then
  exit 0  # Skip expensive operations during compaction
fi
# Expensive startup logic here
```

### 11. Tool input modification is limited to existing fields

**Problem:** `updatedInput` in PreToolUse only works for fields that exist in the tool's input schema. Adding invented fields has no effect.

**Solution:** Only modify documented fields. For Bash: `command`, `description`, `timeout`. For Write: `file_path`, `content`. Etc.

### 12. Multiple hooks don't guarantee execution order

**Problem:** When multiple hooks match the same event, all matching hooks run in parallel. Context from each hook may appear in unpredictable order.

**Solution:** Don't rely on context ordering. Use unique identifiers. Consolidate related logic into a single hook if order matters.

### 13. TeammateIdle does not support prompt or agent hooks

**Problem:** Setting `type: "prompt"` or `type: "agent"` on a TeammateIdle hook will not work.

**Solution:** Use `type: "command"` with exit code control for TeammateIdle hooks.

### 14. PreToolUse deprecated fields

**Problem:** Older examples may show top-level `decision` and `reason` fields for PreToolUse hooks. These are deprecated.

**Solution:** Use `hookSpecificOutput.permissionDecision` (`"allow"`, `"deny"`, `"ask"`) and `hookSpecificOutput.permissionDecisionReason` instead. The deprecated values `"approve"` and `"block"` map to `"allow"` and `"deny"` respectively. Other events like PostToolUse and Stop continue to use top-level `decision` and `reason`.

## Debugging hooks

### Enable debug output

```bash
claude --debug
```

Look for lines like:
```
[DEBUG] Executing hooks for PreToolUse:Bash
[DEBUG] Getting matching hook commands for PreToolUse with query: Bash
[DEBUG] Found 1 hook matchers in settings
[DEBUG] Matched 1 hooks for query "Bash"
[DEBUG] Found 1 hook commands to execute
[DEBUG] Executing hook command: <Your command> with timeout 600000ms
[DEBUG] Hook command completed with status 0: <Your stdout>
```

### Toggle verbose mode

Press `Ctrl+O` in Claude Code to see hook progress in the transcript.

### Test hooks manually

```bash
echo '{
  "session_id": "test",
  "tool_name": "Bash",
  "tool_input": {"command": "npm test"},
  "hook_event_name": "PreToolUse"
}' | ./your-hook.sh

echo $?  # Check exit code
```

## Reference examples

### PreToolUse: Block dangerous commands

```bash
#!/bin/bash
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

BLOCKED=("rm -rf /" "drop table" "DELETE FROM" "truncate")
for pattern in "${BLOCKED[@]}"; do
  if [[ "$COMMAND" == *"$pattern"* ]]; then
    echo "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"Blocked dangerous pattern: $pattern\"}}"
    exit 0
  fi
done

exit 0
```

### PostToolUse: Auto-format after writes

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_input.file_path' | xargs npx prettier --write",
            "async": true,
            "timeout": 120
          }
        ]
      }
    ]
  }
}
```

### Stop: Verify tests pass before stopping

```bash
#!/bin/bash
INPUT=$(cat)

if [ "$(echo "$INPUT" | jq -r '.stop_hook_active')" = "true" ]; then
  exit 0
fi

if npm test > /dev/null 2>&1; then
  exit 0
else
  echo '{"decision": "block", "reason": "Tests are failing. Please fix them before stopping."}'
  exit 0
fi
```

### Notification: Desktop alerts on macOS

```bash
#!/bin/bash
INPUT=$(cat)
NOTIF_TYPE=$(echo "$INPUT" | jq -r '.notification_type')

case "$NOTIF_TYPE" in
  permission_prompt)
    osascript -e 'display notification "Claude needs permission" with title "Claude Code"'
    ;;
  idle_prompt)
    osascript -e 'display notification "Claude is waiting for input" with title "Claude Code"'
    ;;
esac

exit 0
```

### SessionStart: Inject project context

```bash
#!/bin/bash
INPUT=$(cat)
SOURCE=$(echo "$INPUT" | jq -r '.source')

if [ "$SOURCE" = "startup" ] || [ "$SOURCE" = "compact" ]; then
  RECENT=$(git log --oneline -5 2>/dev/null || echo "No git repo")
  echo "{\"additionalContext\": \"Recent commits:\\n$RECENT\"}"
fi

exit 0
```

### Prompt-based Stop hook: Multi-criteria check

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "You are evaluating whether Claude should stop working. Context: $ARGUMENTS\n\nAnalyze the conversation and determine if:\n1. All user-requested tasks are complete\n2. Any errors need to be addressed\n3. Follow-up work is needed\n\nRespond with JSON: {\"ok\": true} to allow stopping, or {\"ok\": false, \"reason\": \"your explanation\"} to continue working.",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

### Agent-based Stop hook: Verify tests

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "agent",
            "prompt": "Verify that all unit tests pass. Run the test suite and check the results. $ARGUMENTS",
            "timeout": 120
          }
        ]
      }
    ]
  }
}
```

## Quick reference table

| Event | Triggers | Can block | Supports matchers | Key output fields |
|-------|----------|-----------|-------------------|-------------------|
| SessionStart | Session begin/resume/clear/compact | No | Yes (source) | `additionalContext`, env vars via `CLAUDE_ENV_FILE` |
| UserPromptSubmit | User submits prompt | Yes | No | `decision`, `reason`, `additionalContext` |
| PreToolUse | Before tool execution | Yes | Yes (tool name) | `permissionDecision`, `updatedInput`, `additionalContext` |
| PermissionRequest | Permission dialog appears | Yes | Yes (tool name) | `decision.behavior`, `updatedInput`, `updatedPermissions` |
| PostToolUse | After successful tool execution | No | Yes (tool name) | `decision`, `reason`, `updatedMCPToolOutput` |
| PostToolUseFailure | After failed tool execution | No | Yes (tool name) | `additionalContext` |
| Notification | Notification sent | No | Yes (type) | `additionalContext` |
| SubagentStart | Subagent spawned | No | Yes (agent type) | `additionalContext` |
| SubagentStop | Subagent finished | Yes | Yes (agent type) | `decision`, `reason` |
| Stop | Main agent finished | Yes | No | `decision`, `reason` |
| TeammateIdle | Teammate about to go idle | Yes | No | Exit code only |
| TaskCompleted | Task marked as completed | Yes | No | Exit code only |
| PreCompact | Before compaction | No | Yes (trigger) | Information only |
| SessionEnd | Session terminates | No | Yes (reason) | Cleanup only |

## Sources

- Hooks reference: <https://code.claude.com/docs/en/hooks>
- Hooks guide: <https://code.claude.com/docs/en/hooks-guide>
- Settings reference: <https://code.claude.com/docs/en/settings>
- Bash command validator example: <https://github.com/anthropics/claude-code/blob/main/examples/hooks/bash_command_validator_example.py>
- Awesome Claude Code (community hooks): <https://github.com/hesreallyhim/awesome-claude-code>
- Claude Code Hooks Mastery: <https://github.com/disler/claude-code-hooks-mastery>
