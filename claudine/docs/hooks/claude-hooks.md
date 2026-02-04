# Claude Code hooks and events

## Scope

This document covers the hook and event system available in Claude Code (Anthropic's agentic CLI). Claude Code provides 12 lifecycle hook events that allow shell scripts, LLM prompts, and agent-based validators to intercept, modify, or block actions at key points in the agentic workflow. Sources are cited inline.

## Configuration

Hooks are defined in JSON settings files under a top-level `hooks` key. Multiple configuration locations are supported, merged by priority.

### Settings file locations

| Location | Scope | Shareable | Priority |
|----------|-------|-----------|----------|
| `~/.claude/settings.json` | All projects | No (local) | Low (global) |
| `.claude/settings.json` | Single project | Yes (committed) | High (project) |
| `.claude/settings.local.json` | Single project | No (gitignored) | Highest (local) |
| Plugin `hooks/hooks.json` | When plugin enabled | Yes | Medium |
| Managed policies | Organization | Admin-controlled | Can override all |

(https://code.claude.com/docs/en/hooks)

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
2. **Prompt** (`type: "prompt"`): Sends a single LLM call. The `$ARGUMENTS` placeholder is replaced with the hook's input JSON. Returns `{"ok": true|false, "reason": "..."}`.
3. **Agent** (`type: "agent"`): Spawns a multi-turn agent with tool access (Read, Grep, Glob, etc.). Same return format as prompt hooks but can perform file reads and searches.

### Handler fields

| Field | Required | Default | Applies to | Description |
|-------|----------|---------|------------|-------------|
| `type` | Yes | - | All | `"command"`, `"prompt"`, or `"agent"` |
| `command` | Yes | - | command | Shell command to execute |
| `prompt` | Yes | - | prompt, agent | Prompt text; `$ARGUMENTS` is replaced with input JSON |
| `model` | No | Fast model | prompt, agent | LLM model to use |
| `timeout` | No | 600s / 30s / 60s | All | Seconds before canceling |
| `statusMessage` | No | - | command | Custom spinner message displayed during execution |
| `async` | No | false | command only | Run in background without blocking |
| `once` | No | false | skills only | Run only once per session |

(https://code.claude.com/docs/en/hooks)

### Matcher system

Matchers are **regex patterns** that filter when hooks fire. They are case-sensitive.

| Event type | Matches against | Examples |
|-----------|-----------------|----------|
| Tool events (PreToolUse, PostToolUse, etc.) | Tool name | `Bash`, `Edit\|Write`, `mcp__.*` |
| SessionStart | Startup source | `startup`, `resume`, `clear`, `compact` |
| SessionEnd | Exit reason | `clear`, `logout`, `prompt_input_exit`, `other` |
| Notification | Notification type | `permission_prompt`, `idle_prompt` |
| SubagentStart/Stop | Agent type | `Explore`, `Bash`, `Plan` |
| PreCompact | Trigger type | `manual`, `auto` |
| UserPromptSubmit, Stop | Not supported | Fires on every occurrence |

MCP tools use the naming pattern `mcp__<server>__<tool>` (e.g., `mcp__github__search_repositories`). You can match all tools from a server with `mcp__github__.*`.

### Environment variables available in hooks

| Variable | Availability | Description |
|----------|-------------|-------------|
| `CLAUDE_PROJECT_DIR` | All hooks | Absolute path to project root |
| `CLAUDE_PLUGIN_ROOT` | Plugin hooks | Plugin directory |
| `CLAUDE_ENV_FILE` | SessionStart only | File path for persisting env vars to future Bash calls |
| `CLAUDE_CODE_REMOTE` | All hooks | `"true"` in web mode, unset in CLI |

## Exit codes

| Exit code | Behavior | JSON processed | Use for |
|-----------|----------|----------------|---------|
| `0` | Success, action proceeds | Yes | Allow action, optionally with JSON control |
| `1` | Non-blocking error | No | Unexpected error, logged in verbose mode |
| `2` | Blocking error, action prevented | No | Block action; stderr becomes Claude's feedback |
| Other | Non-blocking error | No | Any unexpected state |

### Exit code 2 behavior by event

| Event | Blockable | Exit 2 effect |
|-------|-----------|---------------|
| PreToolUse | Yes | Blocks tool call |
| PermissionRequest | Yes | Denies permission |
| UserPromptSubmit | Yes | Blocks and erases prompt from context |
| Stop | Yes | Prevents stopping |
| SubagentStop | Yes | Prevents subagent from stopping |
| PostToolUse | Partially | Shows stderr to Claude (action already ran) |
| PostToolUseFailure | No | Shows stderr to Claude |
| Notification | No | Shows stderr to user only |
| SubagentStart | No | Shows stderr to user only |
| SessionStart | No | Shows stderr to user only |
| SessionEnd | No | Shows stderr to user only |
| PreCompact | No | Shows stderr to user only |

(https://code.claude.com/docs/en/hooks)

## Common JSON output fields

All events support these top-level output fields:

```json
{
  "continue": true,
  "stopReason": "Message shown when continue=false",
  "suppressOutput": false,
  "systemMessage": "Warning shown to user",
  "additionalContext": "Text injected into Claude's context",
  "hookSpecificOutput": {}
}
```

## Hook events

### SessionStart

**Triggers:** New session, resumed session, `/clear` command, or post-compaction.

**Input schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/current/working/directory",
  "permission_mode": "default",
  "hook_event_name": "SessionStart",
  "source": "startup|resume|clear|compact",
  "model": "claude-opus-4-5-20251101"
}
```

**Matcher values:** `startup`, `resume`, `clear`, `compact`

**Can block:** No

**Return type effect:** Can inject context via stdout or JSON `additionalContext`. The `CLAUDE_ENV_FILE` variable allows persisting environment variables to subsequent Bash commands:

```bash
#!/bin/bash
if [ -n "$CLAUDE_ENV_FILE" ]; then
  echo 'export NODE_ENV=production' >> "$CLAUDE_ENV_FILE"
fi
exit 0
```

### UserPromptSubmit

**Triggers:** When the user submits a prompt, before Claude processes it.

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

**Matcher values:** None (fires on every prompt)

**Can block:** Yes

**Return type effect:**
- `exit 0` with plain text stdout: text is added to Claude's context.
- `exit 0` with JSON `decision: "block"`: blocks the prompt.
- `exit 2`: blocks the prompt and erases it from context; stderr becomes feedback.

**Output schema:**
```json
{
  "decision": "block",
  "reason": "Explanation for blocking",
  "additionalContext": "Text added to Claude's context"
}
```

### PreToolUse

**Triggers:** After Claude creates tool parameters, before tool execution.

**Matchable tools:** `Bash`, `Write`, `Edit`, `Read`, `Glob`, `Grep`, `Task`, `WebFetch`, `WebSearch`, and MCP tools (`mcp__<server>__<tool>`).

**Input schema (varies by tool):**
```json
{
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
| Bash | `command`, `description`, `timeout`, `run_in_background` |
| Write | `file_path`, `content` |
| Edit | `file_path`, `old_string`, `new_string`, `replace_all` |
| Read | `file_path`, `offset`, `limit` |
| Glob | `pattern`, `path` |
| Grep | `pattern`, `path`, `glob`, `output_mode`, `-i`, `multiline` |
| WebFetch | `url`, `prompt` |
| WebSearch | `query`, `allowed_domains`, `blocked_domains` |
| Task | `prompt`, `description`, `subagent_type`, `model` |

**Can block:** Yes

**Return type effect:** Three-level permission decision system:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow|deny|ask",
    "permissionDecisionReason": "Why this decision",
    "updatedInput": {
      "command": "modified command"
    },
    "additionalContext": "Context for Claude"
  }
}
```

| Decision | Effect |
|----------|--------|
| `allow` | Bypasses permission system, proceeds without user prompt |
| `deny` | Cancels tool call, sends reason to Claude |
| `ask` | Shows user the permission dialog as normal |

The `updatedInput` field can modify tool parameters before execution (e.g., rewriting a command, changing a file path).

`exit 2` blocks the tool call; stderr becomes Claude's feedback.

### PermissionRequest

**Triggers:** When a permission dialog is about to appear to the user.

**Input schema:**
```json
{
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

**Can block:** Yes

**Return type effect:**
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow|deny",
      "updatedInput": {},
      "updatedPermissions": [],
      "message": "Why denied"
    }
  }
}
```

`exit 2` denies permission; stderr becomes Claude's feedback.

### PostToolUse

**Triggers:** Immediately after a tool completes successfully.

**Input schema:**
```json
{
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

**Can block:** Partially (cannot undo the action)

**Return type effect:**
```json
{
  "decision": "block",
  "reason": "Why the action should be reconsidered",
  "additionalContext": "Context for Claude",
  "updatedMCPToolOutput": "replacement output (MCP tools only)"
}
```

- `exit 0`: proceed normally.
- `exit 2` or `decision: "block"`: prompts Claude with the reason, but the tool has already executed and cannot be undone.
- `updatedMCPToolOutput`: replaces the tool output that Claude sees (MCP tools only).

### PostToolUseFailure

**Triggers:** When a tool execution fails with an error.

**Input schema:**
```json
{
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

**Can block:** No (tool already failed)

**Return type effect:**
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

**Input schema:**
```json
{
  "hook_event_name": "Notification",
  "message": "Claude needs your permission to use Bash",
  "title": "Permission needed",
  "notification_type": "permission_prompt"
}
```

**Can block:** No

**Return type effect:** Can add `additionalContext` to the conversation. Primarily used for external alerting (desktop notifications, Slack, etc.).

### SubagentStart

**Triggers:** When a subagent is spawned via the Task tool.

**Matcher values:** Agent type names (`Bash`, `Explore`, `Plan`, or custom agent names)

**Input schema:**
```json
{
  "hook_event_name": "SubagentStart",
  "agent_id": "agent-abc123",
  "agent_type": "Explore"
}
```

**Can block:** No

**Return type effect:**
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

**Input schema:**
```json
{
  "hook_event_name": "SubagentStop",
  "stop_hook_active": false,
  "agent_id": "def456",
  "agent_type": "Explore",
  "agent_transcript_path": "~/.claude/projects/.../subagents/agent-def456.jsonl"
}
```

**Can block:** Yes

**Return type effect:**
```json
{
  "decision": "block",
  "reason": "Why subagent should continue"
}
```

- `exit 0`: allow subagent to stop.
- `exit 2` or `decision: "block"`: prevents stopping, sends reason as the subagent's next instruction.
- **Must check `stop_hook_active`** to prevent infinite loops (see Gotchas).

### Stop

**Triggers:** When the main Claude agent finishes responding. Does not fire on user interrupts.

**Input schema:**
```json
{
  "hook_event_name": "Stop",
  "stop_hook_active": false
}
```

**Matcher values:** None (fires on every stop)

**Can block:** Yes

**Return type effect:**
```json
{
  "decision": "block",
  "reason": "Why Claude should continue working"
}
```

- `exit 0`: allow Claude to stop.
- `exit 2` or `decision: "block"`: prevent stopping, continue working.
- **Must check `stop_hook_active`** to prevent infinite loops (see Gotchas).

### PreCompact

**Triggers:** Before context compaction occurs.

**Matcher values:** `manual` (from `/compact` command), `auto` (when context window fills)

**Input schema:**
```json
{
  "hook_event_name": "PreCompact",
  "trigger": "manual|auto",
  "custom_instructions": ""
}
```

**Can block:** No

**Return type effect:** Information only. Useful for logging when compaction occurs.

### SessionEnd

**Triggers:** When the session terminates.

**Matcher values:** `clear`, `logout`, `prompt_input_exit`, `bypass_permissions_disabled`, `other`

**Input schema:**
```json
{
  "hook_event_name": "SessionEnd",
  "reason": "prompt_input_exit"
}
```

**Can block:** No

**Return type effect:** Cleanup only. Cannot prevent session termination.

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

- Cannot block tool calls (action already proceeded).
- Cannot return `decision`, `permissionDecision`, or other control fields.
- Only `type: "command"` supports `async`; prompt and agent hooks cannot run asynchronously.
- No deduplication across multiple fires of the same hook.
- Output is delivered on the next interaction (waits if session is idle).

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

(https://github.com/anthropics/claude-code/issues/10205)

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
- Use `suppressOutput: true` in the hook's JSON output.
- Use `async: true` so the modification happens after Claude has moved on.
- Defer formatting to end of session.

### 4. Settings changes require session restart

**Problem:** Editing a settings file while Claude Code is running does not take effect immediately. Hook configuration is captured at startup.

**Solution:** Use the `/hooks` interactive menu to review changes, or restart the session.

### 5. Matchers are case-sensitive regex

**Problem:** `"matcher": "bash"` won't match the `Bash` tool.

**Solution:** Use exact case (`"Bash"`) or a case-insensitive pattern (`"[Bb]ash"`).

### 6. No matcher support for UserPromptSubmit and Stop

**Problem:** You cannot filter these hooks to fire only under certain conditions using matchers.

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

**Problem:** When multiple hooks match the same event, context from each hook may appear in unpredictable order.

**Solution:** Don't rely on context ordering. Use unique identifiers. Consolidate related logic into a single hook if order matters.

## Debugging hooks

### Enable debug output

```bash
claude --debug
```

Look for lines like:
```
[DEBUG] Executing hooks for PreToolUse:Bash
[DEBUG] Found 1 hook matchers in settings
[DEBUG] Hook command completed with status 0: output here
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
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "npx prettier --write \"$(echo $ARGUMENTS | jq -r '.tool_input.file_path')\"",
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

## Quick reference table

| Event | Triggers | Can block | Supports matchers | Key output fields |
|-------|----------|-----------|-------------------|-------------------|
| SessionStart | Session begin/resume/clear/compact | No | Yes | `additionalContext`, env vars via `CLAUDE_ENV_FILE` |
| UserPromptSubmit | User submits prompt | Yes | No | `decision`, `reason`, `additionalContext` |
| PreToolUse | Before tool execution | Yes | Yes (tool name) | `permissionDecision`, `updatedInput`, `additionalContext` |
| PermissionRequest | Permission dialog appears | Yes | Yes (tool name) | `decision.behavior`, `updatedInput`, `updatedPermissions` |
| PostToolUse | After successful tool execution | Partially | Yes (tool name) | `decision`, `reason`, `updatedMCPToolOutput` |
| PostToolUseFailure | After failed tool execution | No | Yes (tool name) | `additionalContext` |
| Notification | Notification sent | No | Yes (type) | `additionalContext` |
| SubagentStart | Subagent spawned | No | Yes (agent type) | `additionalContext` |
| SubagentStop | Subagent finished | Yes | Yes (agent type) | `decision`, `reason` |
| Stop | Main agent finished | Yes | No | `decision`, `reason` |
| PreCompact | Before compaction | No | Yes (trigger) | Information only |
| SessionEnd | Session terminates | No | Yes (reason) | Cleanup only |

## Sources

- Claude Code hooks reference: https://code.claude.com/docs/en/hooks
- Claude Code hooks guide: https://code.claude.com/docs/en/hooks-guide
- Claude Code settings reference: https://code.claude.com/docs/en/settings
- Awesome Claude Code (community hooks): https://github.com/hesreallyhim/awesome-claude-code
- Claude Code Hooks Mastery: https://github.com/disler/claude-code-hooks-mastery
- DataCamp hooks tutorial: https://www.datacamp.com/tutorial/claude-code-hooks
- Async hooks discussion: https://medium.com/@joe.njenga/claude-code-async-hooks-upgrade-makes-workflows-3x-faster-i-tested-it-in-seconds-ef5836f2bd34
