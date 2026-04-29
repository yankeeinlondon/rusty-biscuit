# Better Non-Interactive Reporting

## Claude Code Suggestions

### Additional Reporting Opportunities

- **Billing Mode & Auth Source Reporting**
    - **WHAT:** Report the `apiKeySource` from the `init` event. Distinguish between subscription-based auth ("none") and API key-based auth ("ANTHROPIC_API_KEY").
    - **How:** Present as a subtle "Auth: [Subscription|API Key]" badge in the session header. Use dimmed colors for the value to keep it low-noise.
    - **STDOUT or STDERR:** STDERR (Metadata).
    - **Examples:** `{"type":"system","subtype":"init","apiKeySource":"none"}` → `Auth: Subscription`; `{"type":"system","subtype":"init","apiKeySource":"ANTHROPIC_API_KEY"}` → `Auth: API Key`.
    - **Future Enhancements:**
        - What data is _missing_? The specific subscription tier (Pro, Max, Team) is not currently exposed in the stream.
        - Is this missing data available? Not currently available through hooks or stream events; would require an external account API call.

- **Classified Error Reporting via `api_retry`**
    - **WHAT:** Capture and report the `error` enum and `error_status` from `system/api_retry` events. This allows distinguishing between `billing_error`, `rate_limit`, `authentication_failed`, and `server_error`.
    - **How:** Use high-contrast error banners. For `billing_error`, suggest specific actions like "Check credit balance at anthropic.com/settings/plans".
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"subtype":"api_retry","error":"billing_error","error_status":402}` → `[Error] 402 Payment Required: Credit balance is too low.`
    - **Future Enhancements:**
        - What data is _missing_? The specific amount of credits needed vs remaining is missing.
        - Is this missing data available? No; requires account management API.

- **Predictive Rate Limit Feedback**
    - **WHAT:** Report `status` (`approaching_limit` vs `limited`) and `resetsAt` from the `rate_limit_event`.
    - **How:** A warning bar that appears when `status` is `approaching_limit`. If `limited`, show a countdown timer until `resetsAt`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"rate_limit_info":{"status":"limited","resetsAt":1712000000}}` → `[Warning] Rate limited. Resets in 4m 12s.`
    - **Future Enhancements:**
        - What data is _missing_? The exact token count remaining in the current window.
        - Is this missing data available? No.

- **Real-time "Thinking" and Progress Indicators**
    - **WHAT:** Use `thinking_delta` and `tool_progress` to provide a "live" feel to the non-interactive session.
    - **How:** Use a transient progress line that updates with the thinking text or current tool activity.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Analyzing code..."}}` → `Thinking: Analyzing code...`
    - **Future Enhancements:**
        - What data is _missing_? Estimated time to completion for long-running tools like `Bash` or `WebFetch`.
        - Is this missing data available? No, but `tool_progress` provides periodic heartbeats for some tools.

- **File Persistence Confirmation**
    - **WHAT:** Report the `files_persisted` event to confirm which files were actually written to disk.
    - **How:** A checklist style output: `[Saved] src/main.rs`.
    - **STDOUT or STDERR:** STDOUT (as it represents a state change in the workspace).
    - **Examples:** `{"type":"system","subtype":"files_persisted","files":["src/main.rs"]}` → `Saved: src/main.rs`

- **Subagent Activity Tracking**
    - **WHAT:** Report `task_started`, `task_progress`, and `task_notification` events for subagents spawned via the `Agent` tool.
    - **How:** Use indented blocks or "Subagent > [Task Status]" prefixes to differentiate from the main agent.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"system","subtype":"task_started","task_subject":"Fixing tests"}` → `Subagent: Fixing tests...`

### Current Problems

- **Outdated Rate Limit Schema:** `ClaudeStreamParser::handle_rate_limit` expects `is_throttled` and `retry_after_ms` at the top level, but modern Claude Code wraps these in a `rate_limit_info` object with `status` and `resetsAt`.
- **Ignored `api_retry` Events:** The parser currently skips `system/api_retry` messages, missing out on early error classification that could prevent unnecessary retries or provide better UX for billing/auth failures.
- **Metadata Loss in `init`:** Fields like `apiKeySource`, `claude_code_version`, and `permissionMode` are not captured or stored in the `StreamExecutionSummary`, limiting diagnostics.
- **Opaque Tool Results:** Some tool results (especially large outputs) are just captured as strings; we don't nicely format or truncate them for progress reporting.

### Other Improvements

- **Better Type Safety via Structured Deserialization:**
    - Transition `ClaudeStreamParser` from manual `serde_json::Value` traversal to using strongly-typed structs with `#[derive(Deserialize)]`. The research document points to a formal `SDKMessage` union that we should model in Rust.
- **Ergonomic Programmatic Experience:**
    - Expose `apiKeySource` and `modelUsage` more explicitly in `StreamExecutionSummary` to allow Claudine wrappers to make smarter decisions about session resumption or model switching.
- **Clearer UX for User:**
    - Implement "Cost Awareness" by showing the `total_cost_usd` from the `result` event prominently at the end of every successful session.
- **Test Coverage Improvements:**
    - **Missing Tests:** There are no tests for `api_retry` paths or the newer `rate_limit_info` structure.
    - **Recommendation:** Add unit tests to `claudine/lib/src/stream/claude.rs` that simulate billing failures and rate limit resets using the NDJSON format discovered in research.

## Codex CLI Suggestions

### Additional Reporting Opportunities

- **Live Checklist (Todo List) Progress**
    - **WHAT:** Report updates from `item.updated` events where the item type is `todo_list`. This includes the text of each task and its `completed` status.
    - **How:** Present as a live-updating checklist in the terminal. Use `[ ]` for pending tasks and `[x]` for completed tasks. When a task is completed, it should be visually checked off in real-time.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"item.updated","item":{"type":"todo_list","items":[{"text":"inspect repo","completed":true},{"text":"write summary","completed":false}]}}` → `[x] inspect repo\n[ ] write summary`.
    - **Future Enhancements:**
        - What data is _missing_? The `todo_list` doesn't include an estimated time or priority for tasks.
        - Is this missing data available? No; the model generates the list without these metadata fields.

- **Subagent (Collab) Lifecycle Tracking**
    - **WHAT:** Report `item.started` and `item.completed` for `collab_tool_call` items, including the subagent's `prompt`.
    - **How:** Use a "Subagent Spawned" header followed by the prompt (possibly truncated if very long). When the subagent completes, report its success or failure.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"item.started","item":{"type":"collab_tool_call","prompt":"Fix the tests in src/lib.rs"}}` → `Spawned Subagent: Fix the tests in src/lib.rs...`
    - **Future Enhancements:**
        - What data is _missing_? The name or specialized role of the subagent is not explicitly tagged in the `exec` projection.
        - Is this missing data available? It might be available in the `agents_states` field, but it requires further parsing of the app-server state.

- **Reasoning & Plan Transparency**
    - **WHAT:** Capture `item.updated` for `reasoning` and `plan_update` items to show the model's intermediate thoughts.
    - **How:** A dedicated "Thinking" or "Plan" section that updates live. Use dimmed text to distinguish reasoning from actual output or tool calls.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"item.updated","item":{"type":"reasoning","text":"I need to check the dependencies first."}}` → `Reasoning: I need to check the dependencies first.`
    - **Future Enhancements:**
        - What data is _missing_? Reasoning is sometimes missing entirely under API-key auth due to provider-side policy.
        - Is this missing data available? No; it is suppressed at the source.

- **Tool Execution Previews**
    - **WHAT:** Report the `aggregated_output` and `exit_code` from completed `command_execution` items.
    - **How:** Show a summary of the command's outcome: `[Shell] 'cargo test' exited with 0 (143 lines of output)`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"item.completed","item":{"type":"command_execution","command":"ls","aggregated_output":"file1\nfile2","exit_code":0}}` → `[Shell] 'ls' succeeded. Output: file1, file2`.
    - **Future Enhancements:**
        - What data is _missing_? Real-time streaming of stdout/stderr *during* tool execution is not currently supported in the `exec --json` projection; it only provides the aggregate at the end.
        - Is this missing data available? No; the CLI captures the full output before emitting the JSON event.

- **Token Usage & Cost Analysis**
    - **WHAT:** Report `input_tokens`, `cached_input_tokens`, and `output_tokens` from `turn.completed`.
    - **How:** A summary at the end of the session showing total tokens and an estimated cost if the model/price is known.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"usage":{"input_tokens":1000,"cached_input_tokens":500,"output_tokens":200}}` → `Tokens: 1,200 (500 cached)`.
    - **Future Enhancements:**
        - What data is _missing_? The actual model name is missing from the stream, making cost estimation difficult without external context.
        - Is this missing data available? Yes, via `codex exec` start arguments or `PrePrompt` hooks, but not in the JSONL stream itself.

### Current Problems

- **Item Type Mismatches:** `CodexStreamParser` currently uses `command_exec` and `patch_apply`, while the research indicates the actual event types are `command_execution` and `file_change`. This results in tool lifecycle events being ignored.
- **Missing Tool Types:** Neither the `CodexAdapter` nor the `CodexStreamParser` currently recognize `collab_tool_call`, `web_search`, or `todo_list` as tool/item types, leading to "UnknownEvent" errors or silent drops.
- **Double Counting Logic Risk:** The `capture_codex_usage` logic assumes `input_tokens` already includes `cached_input_tokens` based on research, but if provider behavior changes to make these mutually exclusive, the `total` calculation will be incorrect.
- **Fallback Assistant Text:** The parser accumulates `agent_message` text as a fallback, but since the authoritative text comes from a separate file, any discrepancy between the stream and the file is currently unhandled.

### Other Improvements

- **Strongly Typed Event Models:**
    - Replace the manual `Value` traversal in `CodexStreamParser` with a versioned Rust enum that mirrors the `exec_events.rs` source from the official Codex repository.
- **Enhanced `StreamEventSink` Protocol:**
    - Add `on_checklist_update` and `on_reasoning` methods to the `StreamEventSink` trait to allow the TUI and other reporters to handle these Codex-specific signals elegantly.
- **Improved UX for Subagents:**
    - When a `collab_tool_call` is detected, Claudine could automatically create a nested reporting context to clearly show that the subsequent events are happening inside a sub-agent.
- **Test Coverage Improvements:**
    - **Missing Tests:** There are no tests for `todo_list` updates, `collab_tool_call` (subagents), or `web_search` in the current suite.
    - **Recommendation:** Add integration-style tests to `claudine/lib/src/stream/codex.rs` using the JSONL samples from the research document to ensure all 9 item types are correctly dispatched.

## Gemini CLI Suggestions

### Additional Reporting Opportunities

- **Per-Model Token Attribution**
    - **WHAT:** Report the detailed breakdown from `result.stats.models` (added in March 2026). This identifies which specific models (e.g., `gemini-1.5-flash`, `gemini-2.0-pro-exp`) were used and their respective token counts.
    - **How:** Display as a summary list or a small table at the end of the session: `Model: gemini-1.5-flash (420 tokens)`, `Model: gemini-2.0-pro (1,200 tokens)`.
    - **STDOUT or STDERR:** STDERR (Metadata).
    - **Examples:** `{"stats":{"models":{"gemini-1.5-flash":{"input_tokens":100,"output_tokens":50}}}}` → `Flash: 150 tokens`.
    - **Future Enhancements:**
        - What data is _missing_? The exact "reason" for a model switch (e.g., fallback due to error vs. strategic routing) is not explicitly stated in the stream.
        - Is this missing data available? Not in the stream; potentially available in `llm_request` metadata from `BeforeModel` hooks.

- **Cache Efficiency Metrics**
    - **WHAT:** Report the `cached` and `input` token counts from the `result.stats` object.
    - **How:** Present as a "Cache Efficiency" badge or percentage: `Cache Hit: 85% (12k tokens saved)`. Use green/dimmed styling for high efficiency.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"stats":{"cached":800,"input":200}}` → `Cache Hit: 80% (800 cached)`.
    - **Future Enhancements:**
        - What data is _missing_? The actual "cost saved" in USD is not reported.
        - Is this missing data available? No; requires external pricing tables mapped to model versions.

- **Tool Execution Timing**
    - **WHAT:** Calculate and report the duration for each tool by comparing the `timestamp` between `tool_use` and its corresponding `tool_result`.
    - **How:** Show the duration next to tool completion in the progress indicator: `[Tool] read_file finished in 140ms`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `tool_use` at `12:00:01`, `tool_result` at `12:00:03` → `Tool: 2s`.
    - **Future Enhancements:**
        - What data is _missing_? Breakdown of "local execution time" vs "provider round-trip time".
        - Is this missing data available? No; the stream only provides the aggregate event timing.

- **Granular Error Classification**
    - **WHAT:** Capture and report specific error types from `tool_result.error.type` and `result.error.type` (e.g., `path_not_in_workspace`, `permission_denied`, `FatalTurnLimitedError`).
    - **How:** Use specific icons or color-coded labels for different error categories rather than just a generic "Error" message.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"error":{"type":"path_not_in_workspace"}}` → `[Access Denied] Path is outside workspace boundary.`
    - **Future Enhancements:**
        - What data is _missing_? Recommendations for fixing the error (e.g., "Add path to workspace settings").
        - Is this missing data available? Not in the stream, but Claudine can map known error types to suggestions.

### Current Problems

- **Ambiguous Tool "Success" on Cancellation:** Gemini CLI sometimes reports `status: "success"` for tool calls that were cancelled or timed out. Claudine needs to correlate with empty `output` or subsequent `error` messages to avoid false positives.
- **Fragile Tool Correlation:** The `tool_result` event does not repeat the `tool_name`, requiring the parser to maintain a `tool_id` map. If a stream is interrupted or IDs are reused (though unlikely), the correlation between input and output breaks.
- **Opaque `ask_user` Denials:** In headless mode, `ask_user` calls are automatically denied. This often leaves the model in a loop trying to clarify its task. Claudine should detect frequent `ask_user` denials and suggest a clearer prompt or a sub-agent strategy.
- **Metadata Timing:** The `init` event is emitted early, but concrete model names (beyond aliases like `auto`) are only confirmed at the very end in `result.stats.models`.

### Other Improvements

- **Enhanced `tool_uses` Modeling:**
    - Replace the current `HashMap<String, (Option<String>, Option<Value>)>` with a structured `PendingToolCall` struct that includes timing and name to improve type safety and maintainability.
- **Multi-Model Usage Support:**
    - Update `StreamExecutionSummary` to include a `models_usage` map instead of just a single `model` string, properly reflecting Gemini CLI's routing and fallback behavior.
- **Semantic Error Mapping:**
    - Implement a mapping layer that converts Gemini-specific error strings (e.g., `TerminalQuotaError`) into Claudine's unified error hierarchy for consistent reporting across providers.
- **Test Coverage Improvements:**
    - **Missing Tests:** There are no tests for the `result.stats.models` structure (March 2026 update) or for the `severity: "warning"` branch in `handle_error`.
    - **Recommendation:** Add unit tests to `claudine/lib/src/stream/gemini.rs` that use a real `result` payload containing per-model stats and multiple `tool_result` events.

## Goose Suggestions

### Additional Reporting Opportunities

- **Subagent Activity Observation**
    - **WHAT:** Capture and report `notification_type: "subagent_tool_request"` and `notification_type: "tasks_complete"`. These events signal the lifecycle of spawned subagents.
    - **How:** Display with an indented "Subagent >" prefix and specialized icons (e.g., `🧑‍💼`). Use dimmed colors for subagent logs to distinguish them from the primary agent's output.
    - **STDOUT or STDERR:** STDERR (Monitoring).
    - **Examples:** `{"type":"notification","notification_type":"subagent_tool_request"}` → `🧑‍💼 Subagent: Starting task...`; `{"type":"notification","notification_type":"tasks_complete"}` → `🧑‍💼 Subagent: Finished.`
    #### Future Enhancements
    - **What data is _missing_?** The specific subagent ID and its unique system prompt are not currently exposed in the top-level notification.
    - **Is this missing data available?** Partial data is available in nested `message` events, but requires stateful tracking to correlate with the notification.

- **Live Log and Progress Reporting**
    - **WHAT:** Report `notification` events that include `message`, `progress`, and `total`. These are used by MCP extensions and built-in tools to provide status updates.
    - **How:** Use a transient progress bar for events with `progress` and `total`. For message-only notifications, show a "live" log line that gets replaced by the next update.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"notification","progress":0.5,"total":1.0,"message":"Analyzing..."}` → `Analyzing... [||||    ] 50%`.
    #### Future Enhancements
    - **What data is _missing_?** The specific tool or extension emitting the progress is often missing unless manually included in the `message` string.
    - **Is this missing data available?** Yes, the `extension_id` is present in the notification payload and should be surfaced.

- **Tool Execution Transparency**
    - **WHAT:** Extract and report `toolRequest` and `toolResponse` items from nested `message.content[]` arrays.
    - **How:** Present as `[Tool] name(args)` for requests and `[Tool] result` for responses. For large outputs, show a summary (e.g., "142 lines of output").
    - **STDOUT or STDERR:** STDERR (Execution details).
    - **Examples:** `toolRequest` for `shell` with `ls -la` → `🔧 [shell] ls -la`.
    #### Future Enhancements
    - **What data is _missing_?** Precise execution duration and per-tool token cost.
    - **Is this missing data available?** Duration can be calculated by Claudine; token cost is not currently available per-tool in the stream.

- **Quota and Billing Awareness**
    - **WHAT:** Monitor `systemNotification` with `notificationType: "creditsExhausted"` and extract the `top_up_url`.
    - **How:** Display a high-visibility warning banner: `[Quota] Credits exhausted. Resend message after topping up at [URL]`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"message","message":{"content":[{"type":"systemNotification","notificationType":"creditsExhausted","data":{"top_up_url":"..."}}]}}` → `[Error] Payment Required. Top up: https://...`
    #### Future Enhancements
    - **What data is _missing_?** Low-balance warnings *before* exhaustion and current balance amount.
    - **Is this missing data available?** Not currently exposed in the structured stream; potentially available in log-style notifications via heuristics.

- **Final Session Metrics Extraction**
    - **WHAT:** Capture `complete.total_tokens` at the end of the `stream-json` feed.
    - **How:** Show a "Session Summary" footer with the total token count and an estimated session duration.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"complete","total_tokens":1234}` → `Session Complete. Total Tokens: 1,234`.
    #### Future Enhancements
    - **What data is _missing_?** Model/Provider name used for the session.
    - **Is this missing data available?** No, Goose does not report the model in the stream; it must be tracked from CLI launch arguments.

### Current Problems

- **Complete Absence of Stream Parser:** Claudine currently lacks a `GooseStreamParser`, meaning `goose run --output-format stream-json` is not actually being parsed.
- **Incorrect Fallback Parser:** `create_parser` defaults to `ClaudeStreamParser` for Goose, which will fail to handle Goose's specific NDJSON envelope and nested `camelCase` fields.
- **Protocol Misconfiguration:** `stream_protocol_for` returns `None` for Goose, preventing the system from even attempting structured parsing.
- **Inconsistent Case Sensitivity:** Goose uses `snake_case` for the outer event envelope (e.g., `total_tokens`) but `camelCase` for nested message content (e.g., `toolRequest`), which requires careful deserialization logic.
- **Flattened Notification Payload:** Unlike other providers that nest payloads, Goose flattens notification fields (message, progress, total) directly into the event object, which the current `GooseAdapter` only partially handles.

### Other Improvements

- **Better Type Safety:**
    - Implement a `GooseStreamParser` using a comprehensive `StreamEvent` enum that mirrors the `crates/goose-cli/src/session/mod.rs` source.
    - Use `#[serde(tag = "type")]` and `#[serde(rename_all = "snake_case")]` for the outer envelope, with a separate `MessageContent` enum using `camelCase`.
- **More Ergonomic Programmatic Experience:**
    - Update `StreamEventSink` to include an `on_subagent_event` method to handle Goose's subagent notifications natively.
    - Expose `total_tokens` directly in `StreamExecutionSummary` for Goose sessions.
- **Clearer UX for the User:**
    - Automatically detect and surface the `top_up_url` when a `creditsExhausted` notification is received, rather than just printing a generic error.
    - Provide a "Thinking..." indicator for `systemNotification` events of type `thinkingMessage`.
- **Test Coverage Enhancements:**
    - **Zero Coverage:** There are currently no tests for Goose's structured stream output.
    - **Recommendation:** Add a full suite of unit tests in `claudine/lib/src/stream/goose.rs` using the v1.29.0 NDJSON snippets identified in research, covering all event types: `message`, `notification`, `error`, and `complete`.

## Kimi Code Suggestions

### Additional Reporting Opportunities

- **Live Plan & Todo List Updates**
    - **WHAT:** Capture and report the `PlanDisplay` event, which contains the agent's current goal and a structured todo list with completion statuses.
    - **How:** Present as a "Goal: [Text]" header followed by a checklist. Use `[x]` for completed tasks and `[ ]` for pending ones. Update the list in-place if the terminal supports it, or append a "Progress Update" block.
    - **STDOUT or STDERR:** STDERR (Progress).
    - **Examples:** `{"type":"PlanDisplay","payload":{"goal":"Refactor auth","todo":[{"text":"Check files","done":true},{"text":"Apply fix","done":false}]}}` → `Goal: Refactor auth\n[x] Check files\n[ ] Apply fix`.
    #### Future Enhancements
    - **What data is _missing_?** Estimated time per task and priority levels are not currently included in the payload.
    - **Is this missing data available?** No; the model does not currently generate these metadata fields in the planning phase.

- **Subagent Activity & Lifecycle Monitoring**
    - **WHAT:** Report `SubagentEvent` notifications, including subagent creation (`start`), prompt summaries, and completion status.
    - **How:** Use an indented "Subagent >" prefix or a specialized agent icon (e.g., 🤖). Differentiate subagent output from the main agent using dimmed colors.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"SubagentEvent","payload":{"agent_id":"coder_1","event":"start","prompt":"Fix the failing test"}} ` → `🤖 Subagent (coder_1): Starting task: Fix the failing test...`.
    #### Future Enhancements
    - **What data is _missing_?** Direct token usage attribution for the subagent is sometimes missing from the parent stream's `SubagentEvent`.
    - **Is this missing data available?** It is often available in nested `StatusUpdate` events within the subagent's own stream, which Claudine would need to correlate.

- **Real-time Execution Notifications**
    - **WHAT:** Capture the `Notification` event type which carries non-content messages such as "Indexing complete" or "Search results found."
    - **How:** Display as a subtle info line: `i [Kimi] Indexing complete`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"Notification","payload":{"message":"Starting web search..."}}` → `Note: Starting web search...`.
    #### Future Enhancements
    - **What data is _missing_?** Notification severity (e.g., info vs warning) is not explicitly typed.
    - **Is this missing data available?** No; it must be inferred from the message text or context.

- **Turn Boundaries & Token Snapshots**
    - **WHAT:** Use `TurnBegin` and `TurnEnd` (Wire mode) and `StatusUpdate` to report the start of new reasoning cycles and incremental token counts.
    - **How:** A separator line or "Turn [N]" marker. Display a running total of tokens in a status bar.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"TurnBegin","payload":{"turn_id":2}}` → `--- Turn 2 ---`.
    #### Future Enhancements
    - **What data is _missing_?** The specific model name used for the individual turn is not in the stream.
    - **Is this missing data available?** No, it requires external configuration tracking.

- **Human-in-the-Loop Request Visibility**
    - **WHAT:** Detect `ApprovalRequest` and `QuestionRequest` (Wire mode). Even in non-interactive (`--yolo`) modes where these are auto-denied, reporting them tells the user *why* the agent is stuck or taking a certain path.
    - **How:** Highlight as `[Input Requested]` or `[Approval Gated]`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"QuestionRequest","payload":{"questions":[{"header":"Mode","question":"Proceed?"}]}}` → `[Request] Kimi asked: Proceed? (Auto-dismissed: yolo mode)`.
    #### Future Enhancements
    - **What data is _missing_?** The reasoning behind *why* the model decided to ask the question vs. proceeding.
    - **Is this missing data available?** Usually found in the preceding `assistant` message content.

### Current Problems

- **Wire Mode Incompatibility:** The current `KimiStreamParser` expects a flat `type` field at the top level, which matches `stream-json` but fails on the recommended `--wire` format (which wraps events in a JSON-RPC 2.0 envelope).
- **Missing Event Handlers:** `PlanDisplay`, `SubagentEvent`, and `Notification` events are currently ignored by the parser, leading to "silent" periods during complex multi-step tasks.
- **Inconsistent Token Fields:** The parser checks for `usage` and `token_usage` but doesn't handle the protocol 1.8 shift where some usage data is nested deeper in `payload` objects.
- **Error Mapping:** Does not currently distinguish between `AUTH_EXPIRED` and general provider errors, which prevents Claudine from suggesting a re-login.

### Other Improvements

- **Better Type Safety:**
    - Transition `KimiStreamParser` to use a versioned enum for Kimi events, properly modeling both the JSON-RPC envelope and the internal `type/payload` structure used in Wire mode.
- **Ergonomic Programmatic Experience:**
    - Implement a `WireProxy` that can handle capability negotiation during `initialize`, allowing Claudine to selectively enable or disable "structured question" support based on the execution environment.
- **Clearer UX for User:**
    - Map Kimi's `PlanDisplay` to Claudine's unified "Live Checklist" UI component so that planning looks consistent regardless of whether the user is running Kimi, Codex, or Claude.
- **Test Coverage:**
    - **Current Gap:** No tests exist for `PlanDisplay`, `SubagentEvent`, or JSON-RPC parsing in the Kimi suite.
    - **Recommendation:** Add integration tests in `claudine/lib/src/stream/kimi.rs` using raw JSON-RPC blobs recorded from `kimi --wire` sessions.

## OpenCode CLI Suggestions

### Additional Reporting Opportunities

- **Token and Cost Accounting per Step**
    - **WHAT:** Extract `tokens` (input, output, reasoning, cache read/write) and `cost` from `step_finish` events.
    - **How:** Display a running total or a per-step breakdown. Use a "Usage: [Input]/[Output] tokens ($[Cost])" format at the end of each step or as a final summary.
    - **STDOUT or STDERR:** STDERR (Metadata).
    - **Examples:** `{"type": "step_finish", "part": {"tokens": {"input": 1024, "output": 220}, "cost": 0.00123}}` → `Step usage: 1244 tokens ($0.00123)`.
    #### Future Enhancements
    - **What data is _missing_?** A final session-total event is missing; Claudine must aggregate these manually.
    - **Is this missing data available?** No, must be summed from `step_finish` records.

- **Reasoning and Thinking Visibility**
    - **WHAT:** Capture `reasoning` events when `--thinking` is enabled.
    - **How:** Display reasoning text in a dimmed or "Thought" block to distinguish it from the final response.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type": "reasoning", "part": {"text": "I should check the package.json first."}}` → `Thinking: I should check the package.json first.`
    #### Future Enhancements
    - **What data is _missing_?** Real-time streaming of reasoning tokens (delta) is not currently supported; only the final reasoning part is emitted.
    - **Is this missing data available?** No; the CLI emits the full part once finished.

- **Subagent Session Discovery**
    - **WHAT:** Extract `sessionId`, `providerID`, and `modelID` from the `metadata` of completed `task` tool results.
    - **How:** Report that a subagent was used: `[Subagent] ses_abc (anthropic/claude-3-5-sonnet)`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type": "tool_use", "part": {"tool": "task", "state": {"metadata": {"sessionId": "ses_123", "model": {"providerID": "anthropic", "modelID": "claude-3.5-sonnet"}}}}} ` → `Spawned Subagent: ses_123 (anthropic/claude-3.5-sonnet)`.
    #### Future Enhancements
    - **What data is _missing_?** Live activity of the child session is not visible in the parent's stdout stream.
    - **Is this missing data available?** Only via direct hook integration or by monitoring the child session's event stream separately.

- **Granular Tool Failure Context**
    - **WHAT:** Distinguish between different types of tool errors (e.g., permission denied vs. execution failed) using the `error` string in `tool_use` events.
    - **How:** Map common error strings to user-friendly labels like `[Permission Denied]` or `[Bash Error]`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type": "tool_use", "part": {"state": {"status": "error", "error": "...prevented by rule"}}} ` → `[Permission Denied] write .env`.
    #### Future Enhancements
    - **What data is _missing_?** Rich permission objects (showing which rule was triggered) are missing from stdout.
    - **Is this missing data available?** Yes, through the hook layer (`permission.asked`/`replied`).

- **Model Identification (Turn-based)**
    - **WHAT:** Capture model and provider info from `message.updated` events (via hooks) or `task` tool results.
    - **How:** Display the active model in the session header.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `message.updated` with `providerID: "minimax", modelID: "M2.5"` → `Model: minimax/M2.5`.
    #### Future Enhancements
    - **What data is _missing_?** The parent session model is not explicitly sent to stdout JSON.
    - **Is this missing data available?** Yes, via `message.updated` events in the hook layer.

### Current Problems

- **Filtered Stdout Stream:** `opencode run --format json` omits many useful events (like `session.status`, `permission.asked`, `question.asked`) that are available in the hook layer.
- **No Completion Event:** Callers must rely on process exit to know when a session is finished; there's no `session.complete` JSON record.
- **Post-hoc Tool Reporting:** `tool_use` is only emitted *after* the tool finishes, making it impossible to show "Running [tool]..." status from the JSON stream alone.
- **Undocumented Envelope:** The exact shape of the NDJSON envelope is not formally specified in the OpenCode provider docs.

### Other Improvements

- **Unified Error Handling:**
    - The `extract_error` function in `OpenCodeAdapter` already handles several structured formats (SDK, legacy, flat). It should be expanded to handle `ProviderAuthError` and `APIError` specifics more deeply if they start including more metadata.
- **Type Safety:**
    - Use the inferred `RunJsonEvent` type (from research) to create a strongly-typed Rust enum for deserializing stdout lines.
- **Ergonomics:**
    - Aggregate token usage across `step_finish` events automatically in the `StreamExecutionSummary` to provide a clean "Total Cost" field.
- **Test Coverage:**
    - **Current Gap:** Tests for `reasoning` and `tool_use` (with various statuses/metadata) appear light.
    - **Recommendation:** Add unit tests to `claudine/lib/src/adapters/opencode.rs` that simulate reasoning parts and failed tool calls with specific error strings.

## Qwen CLI Suggestions

### Additional Reporting Opportunities

- **Thinking Blocks and Streaming Deltas**
    - **WHAT:** Capture and report `thinking` content blocks from `assistant` messages and `content_block_delta` events with type `thinking_delta`.
    - **How:** Display as `Thinking: [content]` in dimmed or italicized text. For streaming sessions, update the thinking line in real-time to provide immediate feedback that the model is processing.
    - **STDOUT or STDERR:** STDERR (Reasoning/Progress).
    - **Examples:** `{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Analyzing repository structure..."}}` → `Thinking: Analyzing repository structure...`
    #### Future Enhancements
    - **What data is _missing_?** The total duration spent in the "thinking" state vs. "generating" state is not explicitly summarized.
    - **Is this missing data available?** Not in the stream, but Claudine can calculate this by tracking the time between `content_block_start` and `content_block_stop` for thinking blocks.

- **MCP Tool Progress Indicators**
    - **WHAT:** Capture `tool_progress` events from `stream_event`. This is a unique feature of the Qwen protocol for MCP extensions.
    - **How:** Show a progress bar or percentage next to the active tool: `🔧 [mcp-tool] Progress: 45%`.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"stream_event","event":{"type":"tool_progress","progress":0.45}}` → `[mcp-tool] 45% [====>    ]`
    #### Future Enhancements
    - **What data is _missing_?** The specific tool or MCP extension ID is not always bundled in the `tool_progress` event; it relies on the context of the most recent `tool_use`.
    - **Is this missing data available?** Usually, yes, through stateful tracking of the active `tool_use_id`.

- **Post-Execution Permission Denials**
    - **WHAT:** Capture the `permission_denials` array from the final `result` message at the end of a session.
    - **How:** List each denied tool call with its attempted input: `[Denied] write_file (path: /etc/hosts)`. This provides a clear "post-mortem" of why certain tasks failed.
    - **STDOUT or STDERR:** STDERR.
    - **Examples:** `{"type":"result","permission_denials":[{"tool_name":"write_file","tool_input":{"path":"/etc/hosts"}}]} ` → `Permission Denied: write_file for /etc/hosts`
    #### Future Enhancements
    - **What data is _missing_?** The specific reason for the denial (e.g., "User explicitly denied" vs "Path outside workspace").
    - **Is this missing data available?** No; the `blocked_path` field in the protocol is currently `null` in the CLI implementation.

- **Session Metadata and Model Identity**
    - **WHAT:** Report the `model` and `session_id` from the `init` or `system (subtype: init)` event.
    - **How:** Use a header-style display: `Qwen | Model: qwen-coder-plus | Session: qw-123`.
    - **STDOUT or STDERR:** STDERR.
    #### Future Enhancements
    - **What data is _missing_?** The authentication method used (OAuth vs API Key) is not explicitly tagged in the stream.
    - **Is this missing data available?** Only indirectly through `auth_success` notifications in the hook layer.

- **Streaming Turn Counters and Usage Stats**
    - **WHAT:** Report `num_turns` and the richer `stats` object from the final `result` or `summary` message.
    - **How:** Display as a session summary: `Session ended after 5 turns. Total tokens: 1,500.`
    - **STDOUT or STDERR:** STDERR.
    #### Future Enhancements
    - **What data is _missing_?** Cost in USD is often missing unless using specific providers that populate it.
    - **Is this missing data available?** Yes, it is sometimes present in `stats` but requires manual aggregation for `stream-json` mode.

### Current Problems

- **Missing Partial Stream Support:** `QwenStreamParser` currently ignores `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_stop`, and `tool_progress`. This results in "chunky" updates rather than the fluid streaming experience Qwen supports.
- **Incomplete Usage Capture:** The parser focuses on `usage` and `token_usage` but misses the `stats` object emitted in buffered `json` mode, which contains more granular telemetry.
- **Ambiguous Event Identification:** The parser relies on a mix of `role` and `type` fields to identify assistant messages, which works but lacks the robustness of a strictly-typed protocol model.
- **Hook/Event Mapping Gaps:** The `QwenAdapter` (used for hooks) does not yet map `ask_user_question` tool calls to a unified `QuestionRequest` event, causing them to degrade into generic tool failures in non-interactive mode.

### Other Improvements

- **Better Type Safety via Protocol Enums:**
    - Transition `QwenStreamParser` to use a comprehensive `NDJSON` event enum that mirrors the `@qwen-code/sdk` TypeScript protocol, reducing reliance on manual `Value` traversal.
- **Enhanced `StreamExecutionSummary`:**
    - Update the summary to include a `permission_denials` list, allowing Claudine wrappers to provide better diagnostic reports after a session fails due to policy blocks.
- **Improved UX for Reasoning:**
    - Enable the display of `thinking` blocks by default to eliminate "dead air" during complex reasoning steps, making the agent feel more responsive.
- **Test Coverage Enhancements:**
    - **Missing Tests:** There are no tests for `thinking_delta`, `tool_progress`, or the `permission_denials` array in the current suite.
    - **Recommendation:** Add unit tests to `claudine/lib/src/stream/qwen.rs` using raw NDJSON snippets for partial streaming and multi-turn sessions.

## Roo Code Suggestions

### Additional Reporting Opportunities

- **Cost and Usage Visibility**
    - **WHAT:** Capture `cost` events emitted per turn and the `final_result` event for session totals. Report `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, and `total_cost_usd`.
    - **How:** Display a running cost in the status bar (e.g., `Cost: $0.012`) and a detailed token breakdown at the end of the session. Use green or dimmed text for cache hits to highlight efficiency.
    - **STDOUT or STDERR:** STDERR (Metadata).
    - **Examples:** `{"type": "cost", "data": {"total_cost_usd": 0.005}}` → `Cost: $0.005`.
    - #### Future Enhancements
        - What data is _missing_? Per-tool token usage is not currently broken down.
        - Is this missing data available? No; cost tracking is currently aggregated at the turn level.

- **"Thinking" Transparency**
    - **WHAT:** Capture `thinking` events that provide insight into the agent's internal reasoning process.
    - **How:** Display as `Thinking: [text]` in dimmed or italicized text. This helps the user understand what the agent is doing during long pauses between tool calls.
    - **STDOUT or STDERR:** STDERR (Reasoning).
    - **Examples:** `{"type": "thinking", "data": {"text": "I need to check the source..."}}` → `Thinking: I need to check the source...`.
    - #### Future Enhancements
        - What data is _missing_? Structured reasoning (e.g., plan vs. critique) is not differentiated in the thinking text.
        - Is this missing data available? No; it is currently a flat string.

- **Quota and Funding Alerts**
    - **WHAT:** Monitor `plan_cap_approaching`, `plan_capped`, and `insufficient_funds` events.
    - **How:** Use high-contrast warning banners. If `plan_capped`, show the `upgrade_url` and `reset_timestamp`. If `insufficient_funds`, show the `provider_name` and `current_balance`.
    - **STDOUT or STDERR:** STDERR (Actionable alerts).
    - **Examples:** `{"type": "insufficient_funds", "data": {"current_balance": 0.05}}` → `[Error] Insufficient funds ($0.05). Please top up.`.
    - #### Future Enhancements
        - What data is _missing_? Automatic calculation of "how many more turns" are possible with remaining funds.
        - Is this missing data available? No; depends on fluctuating token costs.

- **Permission Denials (EACCES and Whitelist)**
    - **WHAT:** Capture `permission_denied` events, including the `operation` (read/write), `path`, and `reason`.
    - **How:** Clearly report why an action was blocked (e.g., `[Access Denied] write to /etc/hosts (system_whitelist_blocked)`).
    - **STDOUT or STDERR:** STDERR.
    - #### Future Enhancements
        - What data is _missing_? Information on how to request permission or modify the whitelist.
        - Is this missing data available? No; but Claudine can map these to local config advice.

- **Detailed Tool Visibility (MCP and Native)**
    - **WHAT:** Extract `name`, `input`, and `content` (result) from `tool_use` and `tool_result` events. For `mcp_call`, report the `server_name` and `tool_name`.
    - **How:** Format as `🔧 [tool_name] input` and `✅ [tool_name] result`. For large results, show a summary or file path.
    - **STDOUT or STDERR:** STDERR (Execution details).
    - #### Future Enhancements
        - What data is _missing_? Execution duration for individual tool calls.
        - Is this missing data available? Yes; can be calculated by comparing timestamps of `tool_use` and `tool_result`.

### Current Problems

- **Outdated Event Mapping:** The `RooAdapter` currently maps `WaitingForInput`, `TaskCompleted`, and `Error`. However, the modern Roo Code NDJSON stream uses `type` fields like `thinking`, `tool_use`, `tool_result`, `cost`, and `final_result`. The current mapping seems better suited for older "hook" styles or is incomplete for the modern CLI stream.
- **Missing NDJSON Parser:** There is no dedicated `RooStreamParser` that handles the `{"type": "...", "data": {...}}` structure. `RooAdapter` attempts a generic parse but doesn't handle the nested `data` object patterns seen in the CLI stream.
- **Inconsistent Field Access:** `RooAdapter` looks for `event_name`, `type`, or `event`, but the modern schema is strictly `type`.
- **Metadata fields:** `roo.rs` captures `tool_name`, `tool_input`, etc. at the top level of the JSON, but in the CLI stream these are nested inside the `data` object.

### Other Improvements

- **Better Type Safety:**
    - Implement a `RooStreamParser` with a strongly-typed `RooEvent` enum that mirrors the `@roo-code/types` schema. This replaces the manual `Value` traversal in the current adapter.
- **Ergonomics:**
    - Move `cost` tracking from `meta.extra` to a first-class `Usage` field in `StreamExecutionSummary` to allow for unified cost reporting across all providers.
    - Capture and surface the `model` and `provider` from the `init` event to provide better session context.
- **UX:**
    - Provide a progress indicator for the `thinking` event to reduce "dead air" in non-interactive sessions.
    - Automatically suggest checking `.claudine/config.json` or whitelist files when `permission_denied` is received.
- **Test Coverage:**
    - **Current Gap:** Tests in `roo.rs` only cover `WaitingForInput` and `can_block`.
    - **Recommendation:** Add unit tests for the full NDJSON stream lifecycle, including `init`, `thinking`, `tool_use`, `tool_result`, `cost`, and `final_result`.
