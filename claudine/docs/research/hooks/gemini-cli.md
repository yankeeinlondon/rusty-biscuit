---
homepage: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
hooks: https://geminicli.com/docs/hooks/
---

# Gemini CLI Hooks and Events

## Scope

This document covers the hook and event system available in Gemini CLI (Google's
open-source agentic CLI). Gemini CLI provides 11 lifecycle hook events that
allow shell scripts to intercept, modify, or block actions at key points in
the agentic workflow. Sources are cited inline and collected in the Sources
section at the end.

**Home page:** https://github.com/google-gemini/gemini-cli

**Documentation:** https://geminicli.com/docs/

## Configuration

Hooks are defined in `settings.json` under a top-level `hooks` key. Multiple
configuration locations are supported, merged by priority (highest first).

### Settings file locations

| Priority | Location | Scope | Shareable |
|----------|----------|-------|-----------|
| 1 (highest) | `.gemini/settings.json` | Project/workspace | Yes (committed) |
| 2 | `~/.gemini/settings.json` | User (all projects) | No (local) |
| 3 | `/etc/gemini-cli/settings.json` | System (all users) | Admin-controlled |
| 4 (lowest) | Extension hooks | Per-extension | Yes |

Project settings override user settings; user settings override system settings.
Extensions provide the lowest precedence layer.

### Hook configuration schema

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "regex_or_exact_string (optional)",
        "sequential": false,
        "hooks": [
          {
            "name": "my-hook",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/my-script.sh",
            "timeout": 5000,
            "description": "Brief explanation of hook purpose"
          }
        ]
      }
    ]
  }
}
```

### Hook definition fields

Each entry under an event name is a **hook definition** containing a matcher and
an array of hook configurations.

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `matcher` | `string` | No | Regex (tool events) or exact string (lifecycle events) to filter when the hook fires. `"*"` or `""` matches all. |
| `sequential` | `boolean` | No | If `true`, hooks in this group run one after another. If `false` (default), they run in parallel. |
| `hooks` | `array` | **Yes** | Array of hook configurations. |

### Hook configuration fields

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `type` | `string` | **Yes** | Execution engine. Currently only `"command"` is supported. |
| `command` | `string` | **Yes** | Shell command to execute. Supports `$GEMINI_PROJECT_DIR` expansion. |
| `name` | `string` | No | Friendly name for identification in logs and `/hooks panel`. |
| `timeout` | `number` | No | Execution timeout in milliseconds (default: 60000). |
| `description` | `string` | No | Brief explanation shown in `/hooks panel` UI. |

### Environment variables

Hooks are executed with a sanitized environment. The following variables are
always available:

| Variable | Description |
|:---------|:------------|
| `GEMINI_PROJECT_DIR` | Absolute path to the project root |
| `GEMINI_SESSION_ID` | Unique ID for the current session |
| `GEMINI_CWD` | Current working directory |
| `CLAUDE_PROJECT_DIR` | Alias for `GEMINI_PROJECT_DIR` (compatibility) |

### Disabling hooks

Individual hooks can be disabled by name in settings without removing their
configuration:

```json
{
  "hooks": {
    "disabled": ["my-hook-name"]
  }
}
```

The `/hooks` CLI command provides interactive management:

- `/hooks panel` -- view all hooks, execution counts, timing
- `/hooks enable-all` / `/hooks disable-all` -- toggle all hooks
- `/hooks enable <name>` / `/hooks disable <name>` -- toggle individual hooks

### Environment variable redaction

Hooks inherit the CLI process environment, which may include secrets. Gemini CLI
provides a redaction system that filters variables matching sensitive patterns
(`KEY`, `TOKEN`, etc.). **Redaction is disabled by default.** Enable it in
settings:

```json
{
  "security": {
    "environmentVariableRedaction": {
      "enabled": true,
      "allowed": ["MY_REQUIRED_TOOL_KEY"]
    }
  }
}
```

## Matcher System

Matchers determine which invocations of an event trigger a hook.

### Tool events (`BeforeTool`, `AfterTool`)

Matchers are **regular expressions** compared against the tool name.

| Pattern | Matches |
|:--------|:--------|
| `write_file\|replace` | `write_file` or `replace` |
| `read_.*` | `read_file`, `read_many_files`, etc. |
| `mcp__github__.*` | All tools from the `github` MCP server |
| `run_shell_command` | Exact match on `run_shell_command` |
| `*` or `""` | All tools |

If the matcher is not valid regex, it falls back to exact string comparison.

**Built-in tool names:** `list_directory`, `read_file`, `read_many_files`,
`write_file`, `glob`, `search_file_content`, `replace`, `run_shell_command`,
`ask_user`, `save_memory`, `write_todos`, `activate_skill`,
`get_internal_docs`, `web_fetch`, `google_web_search`.

**MCP tool names** follow the pattern `mcp__<server_name>__<tool_name>`.

### Lifecycle events (`SessionStart`, `SessionEnd`, `PreCompress`)

Matchers are **exact strings** compared against the trigger/source value (e.g.,
`"startup"`, `"exit"`, `"auto"`).

### Events without matcher support

`BeforeAgent`, `AfterAgent`, `BeforeModel`, `AfterModel`,
`BeforeToolSelection`, `Notification` -- these fire on every occurrence
regardless of matcher value. Filter conditions must be checked inside the hook
script.

## Base Input Schema

All hooks receive these common fields via stdin as a JSON object:

```json
{
  "session_id": "string",
  "transcript_path": "string (absolute path to session transcript JSON)",
  "cwd": "string (current working directory)",
  "hook_event_name": "string (e.g. 'BeforeTool')",
  "timestamp": "string (ISO 8601)"
}
```

## Common Output Fields

Most hooks support these fields in their stdout JSON:

| Field | Type | Description |
|:------|:-----|:------------|
| `systemMessage` | `string` | Displayed immediately to the user in the terminal |
| `suppressOutput` | `boolean` | If `true`, hides hook metadata from logs/telemetry |
| `continue` | `boolean` | If `false`, stops the entire agent loop immediately |
| `stopReason` | `string` | Displayed to the user when `continue` is `false` |
| `decision` | `string` | `"allow"`, `"deny"` (alias `"block"`), or `"ask"` / `"approve"` |
| `reason` | `string` | Feedback message provided when decision is `"deny"` |
| `hookSpecificOutput` | `object` | Event-specific output fields (see each event) |

## Exit Codes

| Exit Code | Label | Behavior |
|:----------|:------|:---------|
| `0` | Success | stdout is parsed as JSON. **Preferred for all logic**, including intentional blocks (`{"decision": "deny"}`). |
| `2` | System Block | Action is blocked. stderr is used as the rejection reason. |
| Other | Warning | Non-fatal failure. A warning is shown but the interaction proceeds. |

### Exit code 2 behavior by event

| Event | Exit 2 effect |
|:------|:-------------|
| BeforeTool | Blocks tool execution; turn continues |
| AfterTool | Hides tool result; turn continues |
| BeforeAgent | Aborts turn; erases prompt from context |
| AfterAgent | Rejects response; triggers automatic retry using stderr as feedback |
| BeforeModel | Aborts turn; skips LLM call |
| AfterModel | Aborts turn; discards model output |
| SessionStart | Advisory only (startup is never blocked) |
| SessionEnd | Advisory only (not awaited) |
| PreCompress | Advisory only (async, cannot block) |
| Notification | Advisory only (cannot block) |
| BeforeToolSelection | Not supported (only toolConfig is applied) |

## Hook Events

### SessionStart

Fires on application startup, resuming a session, or after a `/clear` command.

**Input fields:**

| Field | Type | Values |
|:------|:-----|:-------|
| `source` | `string` | `"startup"`, `"resume"`, `"clear"` |

**Matcher values:** `"startup"`, `"resume"`, `"clear"` (exact string match).

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `hookSpecificOutput.additionalContext` | `string` | Interactive mode: injected as the first turn in history. Non-interactive mode: prepended to the user prompt. |
| `systemMessage` | `string` | Shown at the start of the session |

**Can block:** No. `continue` and `decision` are ignored. Startup is never
blocked.

**Gotchas:**
- None of the flow-control fields work on this event. If you need to gate
  access, use `BeforeAgent` instead.

---

### SessionEnd

Fires when the CLI exits or a session is cleared.

**Input fields:**

| Field | Type | Values |
|:------|:-----|:-------|
| `reason` | `string` | `"exit"`, `"clear"`, `"logout"`, `"prompt_input_exit"`, `"other"` |

**Matcher values:** `"exit"`, `"clear"`, `"logout"`, `"prompt_input_exit"`,
`"other"` (exact string match).

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `systemMessage` | `string` | Displayed to the user during shutdown |

**Can block:** No. The CLI **does not wait** for this hook to complete. All
flow-control fields are ignored.

**Gotchas:**
- **Best-effort only.** The CLI fires this hook and exits immediately. If you
  need to persist state reliably, save earlier (e.g., in `AfterAgent` or
  `AfterTool`), or keep durable state outside the hook lifecycle.

---

### BeforeAgent

Fires after a user submits a prompt, before the agent begins planning.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `prompt` | `string` | The original text submitted by the user |

**Matcher values:** None. Fires on every prompt.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `hookSpecificOutput.additionalContext` | `string` | Appended to the prompt for this turn only |
| `decision` | `"deny"` | Blocks the turn and discards the user message (does not appear in history) |
| `continue` | `false` | Blocks the turn but saves the message to history |
| `reason` | `string` | Required if denied or stopped |

**Can block:** Yes.

**Gotchas:**
- `decision: "deny"` erases the prompt from context entirely, while
  `continue: false` preserves it. Choose based on whether you want the LLM to
  "see" the rejected prompt later.

---

### AfterAgent

Fires once per turn after the model generates its final response. Primary use
case is response validation and automatic retries.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `prompt` | `string` | The user's original request |
| `prompt_response` | `string` | The final text generated by the agent |
| `stop_hook_active` | `boolean` | `true` if this hook is already running as part of a retry sequence |

**Matcher values:** None. Fires on every agent completion.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `decision` | `"deny"` / `"block"` | Rejects the response and forces a retry. `reason` becomes the new prompt for the retry. |
| `continue` | `false` | Stops the session without retrying |
| `reason` | `string` | Required for deny; sent to the agent as correction instructions |
| `hookSpecificOutput.clearContext` | `boolean` | If `true`, clears conversation history (LLM memory) while preserving UI display |

**Can block:** Yes (triggers retry).

**Gotchas:**
- **Infinite retry loops.** If your hook always returns `decision: "deny"`,
  the agent retries indefinitely. You **must** check `stop_hook_active` and
  allow stopping when it is `true`:
  ```bash
  INPUT=$(cat)
  if [ "$(echo "$INPUT" | jq -r '.stop_hook_active')" = "true" ]; then
    exit 0
  fi
  ```
- `clearContext` clears LLM memory but the UI still shows previous messages.
  This is useful between retries to prevent the LLM from repeating the same
  mistake.

---

### BeforeModel

Fires before sending a request to the LLM. Operates on a stable, SDK-agnostic
request format.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `llm_request` | `LLMRequest` | Stable request object (see Stable Model API below) |

**Matcher values:** None. Fires on every LLM call.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `hookSpecificOutput.llm_request` | `Partial<LLMRequest>` | Overrides parts of the outgoing request (e.g., model, temperature) |
| `hookSpecificOutput.llm_response` | `LLMResponse` | Synthetic response. If provided, the CLI skips the LLM call entirely and uses this as the response. |
| `decision` | `"deny"` | Blocks the request and aborts the turn |

**Can block:** Yes.

**Gotchas:**
- When providing a synthetic `llm_response`, the response must match the
  `LLMResponse` schema (see Stable Model API). Malformed responses cause
  unpredictable behavior.
- Multiple BeforeModel hooks use **field replacement** merging: later hooks
  override earlier hooks' fields.

---

### AfterModel

Fires immediately after an LLM response chunk is received. Used for real-time
redaction or PII filtering.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `llm_request` | `LLMRequest` | The original request |
| `llm_response` | `LLMResponse` | The model's response (or a single streaming chunk) |

**Matcher values:** None. Fires on every model response chunk.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `hookSpecificOutput.llm_response` | `Partial<LLMResponse>` | Replaces the model's response chunk |
| `decision` | `"deny"` | Discards the response chunk and blocks the turn |
| `continue` | `false` | Kills the entire agent loop |

**Can block:** Yes.

**Gotchas:**
- **Fires per streaming chunk**, not once per response. Heavy processing slows
  streaming and only affects the current chunk.
- Use `AfterAgent` for final-response validation. Keep `AfterModel` hooks
  lightweight or move work to caches.
- Multiple AfterModel hooks use **field replacement** merging (later overrides
  earlier).

---

### BeforeToolSelection

Fires before the LLM decides which tools to call. Used to filter the available
toolset or force specific tool modes.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `llm_request` | `LLMRequest` | Same format as BeforeModel |

**Matcher values:** None. Fires on every tool selection.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `hookSpecificOutput.toolConfig.mode` | `string` | `"AUTO"` (default), `"ANY"` (force at least one tool call), `"NONE"` (disable all tools) |
| `hookSpecificOutput.toolConfig.allowedFunctionNames` | `string[]` | Whitelist of tool names the LLM may call |

**Can block:** No. Does **not** support `decision`, `continue`, or
`systemMessage`.

**Aggregation strategy:** Multiple hooks' whitelists are **unioned** (combined).
Mode `"NONE"` from any hook overrides all others. Mode `"ANY"` overrides
`"AUTO"` when no `"NONE"` is present.

**Gotchas:**
- **Union aggregation can broaden access.** If you have multiple filtering
  hooks, the agent receives the union of all whitelisted tools. To restrict,
  centralize filtering in a single hook or use `mode: "NONE"` as a strict
  override.
- You cannot block turns or return `systemMessage` from this hook. Use
  `BeforeAgent` or `BeforeModel` if you need flow control.

---

### BeforeTool

Fires before a tool executes. Used for argument validation, security checks,
and parameter rewriting.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `tool_name` | `string` | Name of the tool being called (e.g., `write_file`, `run_shell_command`) |
| `tool_input` | `object` | Raw arguments generated by the model |
| `mcp_context` | `object` | Present only for MCP tools. Contains `server_name`, `tool_name`, and connection info (`command`/`args`/`cwd` for stdio, `url` for SSE/HTTP, `tcp` for WebSocket). |

**Matcher values:** Regex against tool name (e.g., `"write_file|replace"`,
`"mcp__github__.*"`).

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `decision` | `"deny"` / `"block"` | Prevents tool execution. `reason` is sent to the agent as a tool error. |
| `reason` | `string` | Required for deny. Becomes the error message the agent sees. |
| `hookSpecificOutput.tool_input` | `object` | Merges with and overrides the model's arguments before execution. |
| `continue` | `false` | Kills the entire agent loop |

**Can block:** Yes. Exit code 2 also blocks the tool but lets the turn
continue.

**Gotchas:**
- `hookSpecificOutput.tool_input` performs a shallow merge with the original
  arguments. You can override existing fields but cannot add fields that the
  tool does not expect.
- When multiple BeforeTool hooks match, they use **OR decision logic**: any
  single `"deny"` blocks the tool.

---

### AfterTool

Fires after a tool executes. Used for result auditing, context injection, or
hiding sensitive output from the agent.

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `tool_name` | `string` | Name of the tool |
| `tool_input` | `object` | Original arguments |
| `tool_response` | `object` | Result containing `llmContent`, `returnDisplay`, and optional `error` |
| `mcp_context` | `object` | Present only for MCP tools (same structure as BeforeTool) |

**Matcher values:** Regex against tool name.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `decision` | `"deny"` | Hides the real tool output from the agent |
| `reason` | `string` | Required for deny. Replaces the tool result sent to the model. |
| `hookSpecificOutput.additionalContext` | `string` | Appended to the tool result for the agent |
| `continue` | `false` | Kills the entire agent loop |

**Can block:** Partially. The tool has already executed and cannot be undone.
Blocking hides the result.

Exit code 2 hides the tool result and uses stderr as the replacement content.
The turn continues.

**Gotchas:**
- The tool has **already run**. `decision: "deny"` redacts the output but
  cannot reverse the action. Use `BeforeTool` for preventive validation.

---

### PreCompress

Fires before the CLI summarizes history to save tokens (context compression).

**Input fields:**

| Field | Type | Values |
|:------|:-----|:-------|
| `trigger` | `string` | `"auto"` (context window fills) or `"manual"` (`/compress` command) |

**Matcher values:** `"auto"`, `"manual"` (exact string match).

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `systemMessage` | `string` | Displayed to the user before compression |

**Can block:** No. Fired asynchronously. Cannot block or modify the
compression process. Flow-control fields are ignored.

**Gotchas:**
- Advisory only. If you need to save state before compression, this hook tells
  you it is about to happen but you cannot prevent it.

---

### Notification

Fires when the CLI emits a system alert (e.g., tool permissions).

**Input fields:**

| Field | Type | Description |
|:------|:-----|:------------|
| `notification_type` | `string` | Currently only `"ToolPermission"` |
| `message` | `string` | Summary of the alert |
| `details` | `object` | Alert-specific metadata (e.g., tool name, file path) |

**Matcher values:** None documented. Fires on every notification.

**Output fields:**

| Field | Type | Effect |
|:------|:-----|:-------|
| `systemMessage` | `string` | Displayed alongside the system alert |

**Can block:** No. Cannot block alerts or grant permissions. Flow-control
fields are ignored.

**Gotchas:**
- Observability only. To automate permission decisions, you would need a
  different mechanism (Gemini CLI does not yet support programmatic permission
  hooks like Claude Code's `PermissionRequest`).

## Stable Model API

Gemini CLI uses stable, SDK-agnostic structures for the `llm_request` and
`llm_response` fields in model hooks. These structures are versioned
independently from the underlying Gemini SDK.

### LLMRequest

```typescript
{
  model: string,
  messages: Array<{
    role: "user" | "model" | "system",
    content: string  // Non-text parts are filtered out
  }>,
  config?: {
    temperature?: number,
    maxOutputTokens?: number,
    topP?: number,
    topK?: number,
    stopSequences?: string[],
    candidateCount?: number,
    presencePenalty?: number,
    frequencyPenalty?: number
  },
  toolConfig?: {
    mode?: "AUTO" | "ANY" | "NONE",
    allowedFunctionNames?: string[]
  }
}
```

### LLMResponse

```typescript
{
  text?: string,
  candidates: Array<{
    content: {
      role: "model",
      parts: string[]
    },
    finishReason?: "STOP" | "MAX_TOKENS" | "SAFETY" | "RECITATION" | "OTHER",
    index?: number,
    safetyRatings?: Array<{
      category: string,
      probability: string,
      blocked?: boolean
    }>
  }>,
  usageMetadata?: {
    promptTokenCount?: number,
    candidatesTokenCount?: number,
    totalTokenCount?: number
  }
}
```

**Note:** The `LLMRequest` messages only contain text content. Non-text parts
(images, function calls, etc.) are intentionally filtered out to provide a
simplified, stable interface for hooks.

## Multi-Hook Aggregation

When multiple hooks match the same event, Gemini CLI aggregates their results
using event-specific strategies:

| Strategy | Events | Behavior |
|:---------|:-------|:---------|
| **OR decision** | BeforeTool, AfterTool, BeforeAgent, AfterAgent, SessionStart | Any single `"deny"` blocks. Messages and contexts are concatenated. Default decision is `"allow"`. |
| **Field replacement** | BeforeModel, AfterModel | Later hooks override earlier hooks' fields. |
| **Union** | BeforeToolSelection | `allowedFunctionNames` are unioned. `"NONE"` mode wins over all; `"ANY"` wins over `"AUTO"`. |
| **Simple merge** | SessionEnd, PreCompress, Notification | Later outputs override earlier. |

### Sequential vs. parallel execution

By default, hooks within a definition group run **in parallel**. Set
`"sequential": true` on the hook definition to run them one after another. If
**any** hook definition for an event has `sequential: true`, all hooks for that
event run sequentially.

## Security

### Project hook trust model

When you open a project with hooks defined in `.gemini/settings.json`:

1. **Detection:** Gemini CLI detects the hooks.
2. **Fingerprinting:** A unique identity is generated based on hook `name` and
   `command`.
3. **Warning:** If the identity has not been seen before, a warning is
   displayed.
4. **Execution:** The hook executes (unless the folder is untrusted).
5. **Trust:** The hook is marked as trusted for this project.

**Modification detection:** If the `command` string of a project hook changes
(e.g., via `git pull`), its identity changes. Gemini CLI treats it as a new
untrusted hook and warns again.

### Untrusted folders

Project hooks are **blocked entirely** in untrusted folders. The
`security.folderTrust.enabled` setting (default: `true`) controls this. When a
folder is not trusted, project-level hooks in `.gemini/settings.json` will not
execute.

## Gotchas

### 1. Strict JSON output on stdout

**Problem:** Any non-JSON text on stdout breaks parsing. If polluted, Gemini CLI
defaults to "allow" and treats the entire output as a `systemMessage`.

**Solution:** Log to stderr only (`echo "debug" >&2`). Use JSON validation
before printing. Guard shell profile output:

```bash
# ~/.zshrc or ~/.bashrc
if [[ $- == *i* ]]; then
  echo "Shell ready"  # Only in interactive shells
fi
```

### 2. Exit code 2 behavior varies by event

**Problem:** Exit code 2 blocks different things depending on the event (tool
vs. agent vs. retry trigger).

**Solution:** Prefer structured JSON with `decision`/`reason` for predictable
flow. Reserve exit code 2 for emergency blocks or script errors.

### 3. AfterModel fires per streaming chunk

**Problem:** AfterModel fires for every streaming chunk. Heavy processing slows
streaming and only affects the current chunk.

**Solution:** Use `AfterAgent` for final-response validation. Keep `AfterModel`
lightweight.

### 4. BeforeToolSelection does not support flow control

**Problem:** You cannot block turns or return `systemMessage` from this hook.
Only `toolConfig` is applied.

**Solution:** Use `BeforeAgent` or `BeforeModel` for flow control.

### 5. Union of tool allowlists broadens access

**Problem:** Multiple `BeforeToolSelection` hooks union `allowedFunctionNames`,
which can accidentally broaden tool access.

**Solution:** Centralize tool filtering in a single hook, or use
`mode: "NONE"` as a strict override.

### 6. SessionEnd is not awaited

**Problem:** SessionEnd fires on exit but the CLI does not wait for it to
complete.

**Solution:** Persist important state earlier in the lifecycle (e.g.,
`AfterAgent`, `AfterTool`).

### 7. PreCompress is advisory and async

**Problem:** PreCompress is fired asynchronously and cannot block or alter
compression.

**Solution:** Use it only for logging or notification. Save state before
compression triggers.

### 8. Project hook trust changes on command edits

**Problem:** Changing the hook command string (e.g., via `git pull`) makes the
hook untrusted again and triggers a new warning.

**Solution:** Pin commands and avoid modifying hook command strings unless
necessary.

### 9. Environment variable redaction is disabled by default

**Problem:** Hooks inherit the CLI environment and can see secrets like API
keys.

**Solution:** Enable `environmentVariableRedaction` in settings and allowlist
only required variables.

### 10. AfterAgent infinite retry loops

**Problem:** If your AfterAgent hook always returns `decision: "deny"`, the
agent retries indefinitely.

**Solution:** Always check `stop_hook_active` and exit 0 when it is `true`:

```bash
INPUT=$(cat)
if [ "$(echo "$INPUT" | jq -r '.stop_hook_active')" = "true" ]; then
  exit 0
fi
```

### 11. Slow hooks block the agent loop

**Problem:** Hooks run synchronously. Slow hooks delay the entire agent.

**Solution:** Keep hooks fast, cache expensive work, and use matchers to limit
execution to relevant events.

### 12. Only command type is supported

**Problem:** Unlike Claude Code, Gemini CLI does not support `prompt` or `agent`
hook types. Only `type: "command"` is available.

**Solution:** Implement complex logic in your script (Node.js, Python, bash).
If you need LLM-based validation, call an LLM API from within your command
hook script.

### 13. additionalContext is HTML-sanitized

**Problem:** The `additionalContext` field returned from hooks has `<` and `>`
escaped to `&lt;` and `&gt;` to prevent tag injection.

**Solution:** Do not rely on HTML/XML-like tags in `additionalContext` values.
Use plain text or markdown formatting instead.

## Quick Reference Table

| Event | Triggers | Can block | Matcher type | Key output fields |
|:------|:---------|:----------|:-------------|:------------------|
| SessionStart | Startup, resume, /clear | No | Exact string | `additionalContext` |
| SessionEnd | Exit, clear, logout | No | Exact string | `systemMessage` (best-effort) |
| BeforeAgent | User submits prompt | Yes | None | `decision`, `reason`, `additionalContext` |
| AfterAgent | Agent completes response | Yes (retry) | None | `decision`, `reason`, `clearContext` |
| BeforeModel | Before LLM call | Yes | None | `llm_request`, `llm_response` (synthetic) |
| AfterModel | After LLM chunk | Yes | None | `llm_response` (replacement) |
| BeforeToolSelection | Before tool choice | No | None | `toolConfig.mode`, `toolConfig.allowedFunctionNames` |
| BeforeTool | Before tool executes | Yes | Regex (tool name) | `decision`, `reason`, `tool_input` |
| AfterTool | After tool executes | Partial | Regex (tool name) | `decision`, `reason`, `additionalContext` |
| PreCompress | Before compression | No | Exact string | `systemMessage` |
| Notification | System alert | No | None | `systemMessage` |

## Sources

- Gemini CLI hooks overview: https://geminicli.com/docs/hooks/
- Gemini CLI hooks reference: https://geminicli.com/docs/hooks/reference
- Gemini CLI writing hooks guide: https://geminicli.com/docs/hooks/writing-hooks
- Gemini CLI hooks best practices: https://geminicli.com/docs/hooks/best-practices
- Gemini CLI GitHub repository: https://github.com/google-gemini/gemini-cli
- Hook types source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/types.ts
- Hook event handler source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookEventHandler.ts
- Hook aggregator source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookAggregator.ts
- Hook planner source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookPlanner.ts
- Hook translator source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookTranslator.ts
- Hook registry source: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookRegistry.ts
