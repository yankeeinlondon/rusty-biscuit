# Gemini CLI hooks (Agentic CLI)

This document summarizes the hook and event system in Gemini CLI, including
what each hook receives, what it can return, and how the return value affects
agent flow. The focus is the official Gemini CLI hook reference.

Sources

- https://geminicli.com/docs/hooks/
- https://geminicli.com/docs/hooks/reference/
- https://geminicli.com/docs/hooks/writing-hooks
- https://geminicli.com/docs/hooks/best-practices
- https://developers.googleblog.com/tailor-gemini-cli-to-your-workflow-with-hooks/

## Core mechanics

Hooks are scripts run synchronously at specific points in the agent loop. They
communicate over stdin/stdout with strict JSON.

Input (stdin)

- JSON object. Every hook gets a base schema:
    - session_id: string
    - transcript_path: string (absolute path to transcript JSON)
    - cwd: string
    - hook_event_name: string
    - timestamp: string (ISO 8601)

Output (stdout)

- JSON object only. Any non-JSON output on stdout is treated as failure.
- Common fields supported by most hooks:
    - systemMessage: string (shown to user)
    - suppressOutput: boolean (hide hook metadata from logs/telemetry)
    - continue: boolean (false stops agent loop)
    - stopReason: string (shown when continue=false)
    - decision: "allow" | "deny" ("block" is alias for "deny")
    - reason: string (required for "deny")

Exit codes

- 0: success; stdout parsed as JSON (preferred for all logic)
- 2: system block; action is blocked; stderr becomes the rejection reason
- other: warning; hook fails but flow continues

## Event-by-event reference

Each event below shows inputs, relevant output fields, expected return type, and
impact on agent flow. Return type is always a JSON object via stdout unless
explicitly noted.

### SessionStart
When it fires

- On startup, resume, or after /clear

Input fields

- source: "startup" | "resume" | "clear"

Output fields

- hookSpecificOutput.additionalContext: string
- systemMessage: string

Return type and effect

- JSON object; advisory only. continue/decision are ignored. Startup is never
  blocked. additionalContext is injected as the first turn (interactive) or
  prepended to the prompt (non-interactive).

### SessionEnd
When it fires

- On exit or session clear

Input fields

- reason: "exit" | "clear" | "logout" | "prompt_input_exit" | "other"

Output fields

- systemMessage: string

Return type and effect

- JSON object; best effort. CLI does not wait for completion and ignores
  flow-control fields.

### BeforeAgent
When it fires

- After user prompt, before planning

Input fields

- prompt: string

Output fields

- hookSpecificOutput.additionalContext: string (appended to prompt)
- decision: "deny" to block turn and discard prompt
- continue: false to stop loop but keep prompt in history
- reason: required for deny/continue=false

Return type and effect

- JSON object. Can block a turn, alter prompt context for this turn, or stop the
  loop.

### AfterAgent
When it fires

- After final response for the turn

Input fields

- prompt: string
- prompt_response: string
- stop_hook_active: boolean (already in retry sequence)

Output fields

- decision: "deny" to reject response and force retry
- reason: required for deny; becomes new prompt for retry
- continue: false to stop session without retry
- hookSpecificOutput.clearContext: boolean (clear history, keep UI display)

Return type and effect

- JSON object. Can force retry, stop session, or clear LLM memory between
  retries.

### BeforeModel
When it fires

- Before sending request to the LLM

Input fields

- llm_request: object (stable SDK-agnostic format)

Output fields

- hookSpecificOutput.llm_request: object (override request fields)
- hookSpecificOutput.llm_response: object (synthetic response, skips LLM call)
- decision: "deny" to block request

Return type and effect

- JSON object. Can rewrite model params, replace response, or block the turn.

### AfterModel
When it fires

- After each model response chunk (streaming)

Input fields

- llm_request: object
- llm_response: object (model response or chunk)

Output fields

- hookSpecificOutput.llm_response: object (replace response chunk)
- decision: "deny" to discard chunk and block turn
- continue: false to kill loop

Return type and effect

- JSON object. Runs per chunk; modifications affect only current chunk. Can
  block the turn.

### BeforeToolSelection
When it fires

- Before tool choice

Input fields

- llm_request: object

Output fields

- hookSpecificOutput.toolConfig.mode: "AUTO" | "ANY" | "NONE"
- hookSpecificOutput.toolConfig.allowedFunctionNames: string[]

Return type and effect

- JSON object. Only tool filtering is supported. decision/continue/systemMessage
  are not supported. Multiple hooks union their allowedFunctionNames; mode
  "NONE" overrides.

### BeforeTool
When it fires

- Before a tool executes

Input fields

- tool_name: string
- tool_input: object
- mcp_context: object (optional)

Output fields

- decision: "deny" to block tool
- reason: required for deny; sent to agent as tool error
- hookSpecificOutput.tool_input: object (merged and overrides tool args)
- continue: false to kill loop

Return type and effect

- JSON object. Can block tool, rewrite tool args, or stop loop. Exit code 2
  blocks tool but turn continues.

### AfterTool
When it fires

- After a tool executes

Input fields

- tool_name: string
- tool_input: object
- tool_response: object (llmContent, returnDisplay, optional error)
- mcp_context: object

Output fields

- decision: "deny" to hide real tool output
- reason: required for deny; replaces tool result sent to model
- hookSpecificOutput.additionalContext: string (appended to tool result)
- continue: false to kill loop

Return type and effect

- JSON object. Can redact or replace tool output, append context, or stop loop.
  Exit code 2 hides tool result but turn continues.

### PreCompress
When it fires

- Before context compression (summarization)

Input fields

- trigger: "auto" | "manual"

Output fields

- systemMessage: string

Return type and effect

- JSON object; advisory only. Fired asynchronously and cannot block or alter
  compression.

### Notification
When it fires

- On system alert (e.g., tool permissions)

Input fields

- notification_type: "ToolPermission"
- message: string
- details: object

Output fields

- systemMessage: string

Return type and effect

- JSON object; observability only. Cannot grant permissions or block alerts.

## Stable model API (hook-facing)

LLMRequest

- model: string
- messages: Array<{ role: "user" | "model" | "system", content: string }>
- config: object (generation params, e.g., temperature)
- toolConfig: { mode: string, allowedFunctionNames: string[] }

LLMResponse

- candidates: Array<{ content: { role: "model", parts: string[] }, finishReason: string }>
- usageMetadata: { totalTokenCount: number }

## Gotchas and common workarounds

Strict JSON output on stdout

- Gotcha: Any non-JSON text on stdout breaks parsing. If polluted, Gemini CLI
  defaults to allow and treats the output as systemMessage.
- Workaround: Log to stderr only. Use JSON validation (jq or JSON libs) before
  printing.

Exit code 2 behavior varies by event

- Gotcha: Exit code 2 blocks different things depending on the event (tool vs
  agent vs after-agent retry).
- Workaround: Prefer structured JSON with decision/reason for predictable flow,
  and reserve exit 2 for emergency blocks.

AfterModel is per chunk

- Gotcha: AfterModel fires for every streaming chunk; heavy processing can slow
  streaming and only affects the current chunk.
- Workaround: Use AfterAgent for final-response validation; keep AfterModel
  lightweight or move work to caches.

BeforeToolSelection does not support decision/continue

- Gotcha: You cannot block turns here or return systemMessage; only toolConfig
  is applied.
- Workaround: Use BeforeAgent or BeforeModel if you need flow control.

Union of tool allowlists

- Gotcha: Multiple BeforeToolSelection hooks union allowedFunctionNames, which
  can accidentally broaden tool access.
- Workaround: Centralize tool filtering or use mode="NONE" in a strict
  override hook.

SessionEnd and PreCompress are advisory

- Gotcha: SessionEnd is not awaited; PreCompress is async and cannot block.
- Workaround: If you must persist state, save earlier (AfterAgent) or keep
  durable state outside the hook lifecycle.

Project hook trust changes on command edits

- Gotcha: Changing the hook command string (e.g., git pull) makes it untrusted
  again and triggers a new warning.
- Workaround: Pin commands and avoid modifying hook command strings unless
  necessary.

Environment variable redaction is disabled by default

- Gotcha: Hooks inherit the CLI environment and can see secrets.
- Workaround: Enable environmentVariableRedaction and allowlist only required
  variables.

Performance of synchronous hooks

- Gotcha: Slow hooks block the agent loop.
- Workaround: Keep hooks fast, cache expensive work, and use matchers to limit
  execution to relevant events.
