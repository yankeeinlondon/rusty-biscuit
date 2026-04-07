---
title: "Codex CLI Non-Interactive / Structured Output: Developer Gotchas"
last_updated: 2026-04-06
---
# Codex CLI Non-Interactive / Structured Output: Developer Gotchas

Research into real-world developer experiences, pain points, and edge cases with OpenAI Codex CLI's `codex exec --json` non-interactive mode and structured output. All findings sourced from GitHub issues, community forums, and systematic flag/feature testing.

## Category 1: Event Stream Schema & Contract Issues

### 1.1 No Published JSON Schema for `--json` Output

There is no formal JSON Schema for the JSONL event stream emitted by `codex exec --json`. The event shapes are defined only implicitly by the Rust source code. Any consumer must reverse-engineer the format from observed output, which makes downstream parsers brittle.

**Impact:** Automation pipelines have no way to validate events against a contract. Breaking changes are discovered only at runtime.

**Source:** [#1673 - Provide JSON Schema for --json flag output](https://github.com/openai/codex/issues/1673) (open since Jul 2025)

### 1.2 Event Schema Drift Without Versioning

The `--json` output format has changed without documentation updates or versioning. Specifically, the field name `item_type` was renamed to `type`, and the value `assistant_message` was changed to `agent_message`. Parsers following documented examples fail silently because the field they match on no longer exists.

**Impact:** Machine-readable output formats are supposed to be contracts. Format changes without version indicators make it impossible to know which codex versions work with which schemas.

**Source:** [#4776 - JSON output mode docs are out of date](https://github.com/openai/codex/issues/4776) (closed)

### 1.3 Abandoned Items Emitted with Wrong Status

When a turn ends with shell commands still in-flight (no `ExecCommandEnd` received), `handle_task_complete` drains them and emits `ItemCompleted` events with `status: "completed"` and `exit_code: null`. This contradicts the normal path which only uses `"completed"` when `exit_code == 0`. A CI script checking `status == "completed"` to determine success would incorrectly treat abandoned commands as having succeeded.

Additionally, if `McpToolCallEnd` never arrives before the turn ends, `handle_task_complete` does not drain `running_mcp_tool_calls`. The JSONL stream has an unmatched `ItemStarted` with no `ItemCompleted`, breaking the lifecycle contract.

**Impact:** CI pipelines that rely on event status fields get false positives. Consumers tracking item lifecycle get incomplete traces.

**Source:** [#14691 - bug(exec --json): abandoned items emitted with wrong status when turn ends](https://github.com/openai/codex/issues/14691) (open)

### 1.4 Missing Model Name in JSONL Output

The `exec --json` JSONL output does not include which model was used. Consumers that need to log or bill by model must look it up elsewhere.

**Impact:** Multi-model pipelines cannot determine per-request model attribution from the event stream alone.

**Source:** [#14736 - Include model name in exec --json JSONL output](https://github.com/openai/codex/issues/14736) (open)

### 1.5 Rate Limits Always Null in Exec Mode

`codex exec` mode always yields `rate_limits: null` in rollout JSONL and in `TokenCount` events. The handler code exists to process rate limits, but the API server does not send `x-codex-*` response headers for exec-mode sessions, unlike VS Code/app-server mode.

**Impact:** Tooling built on exec mode (VS Code extensions, usage dashboards) cannot display real-time rate limit percentages.

**Source:** [#14728 - feat(exec): emit rate_limits in exec mode JSONL output](https://github.com/openai/codex/issues/14728) (open)

### 1.6 Reasoning Events Suppressed in Exec JSON Output

`codex exec --json` drops generic reasoning lifecycle events during long "thinking" stretches. The core event stream does emit `ItemStarted/ItemCompleted` reasoning events internally, but the exec JSON output filter discards them. Automation sees no progress signals and cannot distinguish "still thinking" from "actually hung."

**Impact:** Wrappers that enforce idle timeouts misclassify active reasoning as a stall. Discovered during real-world mounted-workspace integration testing.

**Source:** [#14462 - codex exec stalls in mounted workspace before any writes](https://github.com/openai/codex/issues/14462) (open, see comment by DeanoC)

### 1.7 Missing `PostToolUse` Hook for Exec-Session-Completed Tools

`PreToolUse` fires for shell tools started via `exec_command`, but `PostToolUse` is missing when the tool completes later through the session polling path (e.g., long-running commands using `write_stdin`). In one reproduced case, 7 `PreToolUse` events fired but only 2 matched `PostToolUse` events.

**Impact:** Consumers that rely on hooks to model tool lifecycle get incomplete traces -- tools appear to start but never finish, with missing duration and status.

**Source:** [#16246 - Hooks: PostToolUse is missing for tools that complete via exec session / polling path](https://github.com/openai/codex/issues/16246) (open)

## Category 2: Authentication & Billing Differences

### 2.1 Reasoning Items Missing with API Key Auth

When using API key authentication (`OPENAI_API_KEY`), `codex exec --json` does not emit `"type":"reasoning"` items. The same version with web login (`codex login`) does include reasoning items. Confirmed by an OpenAI collaborator as a bug, not intentional behavior, but unfixed for months.

**Impact:** Automation pipelines using API key auth get a materially different (incomplete) event stream compared to web-authenticated sessions. Any wrapper that processes reasoning events will see nothing.

**Source:** [#10746 - --json doesn't output reasoning items when using API Key auth](https://github.com/openai/codex/issues/10746) (open, confirmed bug)

### 2.2 OAuth Token Registration Fails in Automated Provisioning

The `codex auth login` flow requires an interactive terminal (TTY). In automated provisioning via SSH without TTY allocation, token registration hangs or silently fails. The `OPENAI_API_KEY` environment variable works as a workaround but triggers the reasoning-items-missing bug above.

**Impact:** Fully automated deployment pipelines cannot provision Codex auth non-interactively without accepting degraded event stream quality.

**Source:** [#15460 - Support OAuth token registration fails during automated provisioning](https://github.com/openai/codex/issues/15460) (open)

### 2.3 API Key Precedence is Non-Obvious

`OPENAI_API_KEY` is intentionally deprioritized below stored auth credentials. The correct environment variable for programmatic auth override is `CODEX_API_KEY`. This is not documented clearly and causes unexpected behavior where setting `OPENAI_API_KEY` has no effect.

**Impact:** Users expecting standard OpenAI SDK behavior (where `OPENAI_API_KEY` is authoritative) find their env var silently ignored.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Test 65)

## Category 3: Structured Output (`--output-schema`) Issues

### 3.1 Root Schema Type Violation

`codex_output_schema` defines the root schema as `type: "array"`, but the Structured Outputs API requires `type: "object"` at the root level. This causes a deterministic HTTP 400 error on every request, making the CLI completely unusable in CI/CD pipelines that use this feature.

**Impact:** Total blocker for CI pipelines using structured output with certain model/config combinations.

**Source:** [#16552 - codex_output_schema uses root type: "array" which violates Structured Outputs API requirement](https://github.com/openai/codex/issues/16552) (open)

### 3.2 `--output-schema` Not Supported with Chat Completions API

When a model provider override in `config.toml` forces the Chat Completions API path, `--output-schema` fails with `unsupported operation: output_schema is not supported for Chat Completions API`. This is not surfaced until runtime.

**Impact:** Users with custom model providers cannot use structured output at all.

**Source:** [#5360 - bug: output_schema is not supported for Chat Completions API](https://github.com/openai/codex/issues/5360) (closed)

### 3.3 `--output-schema` Silently Ignored When MCP/Tools Are Active

When `--json` and `--output-schema` are used together with MCP servers or tools, the backend silently drops the strict schema validation. The model produces malformed outputs: YAML-like bare objects, missing commas, unquoted keys, markdown wrappers. The CLI exits 0 as if the run succeeded.

**Impact:** Automated agentic workflows that depend on schema-validated output get unparseable results with no error indication.

**Source:** [#15451 - --json and --output-schema are silently ignored when tools/MCP servers are active](https://github.com/openai/codex/issues/15451) (closed, maintainer says "model behavior" not CLI bug)

### 3.4 `--output-schema` Missing from `exec resume`

The `--output-schema` flag is not supported when resuming an exec session via `codex exec resume`. There is no way to validate structured output on resumed sessions.

**Source:** [#14343 - Add --output-schema support to codex exec resume](https://github.com/openai/codex/issues/14343) (open)

## Category 4: Process Lifecycle & Hanging Issues

### 4.1 Exit Code Always 0 Even When Commands Fail

`codex exec` always exits with code 0, even when the requested shell command fails with a nonzero exit code. The failure is correctly recorded in the transcript (`status: "failed"`, `exit_code: 1`), but the outer process exit code is 0.

**Impact:** CI pipelines cannot rely on the process exit code to detect command failure. Must parse the JSONL stream to determine actual success/failure.

**Source:** [#15536 - codex exec exits 0 even when command_execution fails with nonzero exit code](https://github.com/openai/codex/issues/15536) (open)

### 4.2 Exec Hangs Indefinitely with `--image` Flag

`codex exec --json` hangs indefinitely when the `--image` flag is used. The CLI emits `thread.started` and then produces no further output. Root cause: a race condition where two consumers compete for the same async event channel. The background event-forwarding thread and the main image-upload-wait loop both call `conversation.next_event()`, and whichever gets the `TaskComplete` event first wins. If the background thread wins, the main thread waits forever.

**Impact:** Image-based workflows are completely unusable in non-interactive mode.

**Source:** [#5773 - codex exec --json hangs indefinitely when --image flag is used](https://github.com/openai/codex/issues/5773) (closed, fixed via PR #5891)

### 4.3 Exec Resume Hangs When MCP Helpers Start

`codex exec --json resume <session>` can hang indefinitely on macOS when MCP helper processes are configured. The resumed process starts, launches MCP helper processes, but then sits idle for hours without emitting any new events. The hang appears to occur during MCP `list_all_tools()` which awaits each client startup future indefinitely.

**Impact:** Resumed sessions with MCP servers configured may never complete. No timeout or error is surfaced.

**Source:** [#14470 - codex exec --json resume can hang indefinitely on macOS after MCP helpers start](https://github.com/openai/codex/issues/14470) (open)

### 4.4 Turn Completes Prematurely with Background Processes

When an agent starts a long-running process via `unified_exec` (e.g., `uv run train.py`), the turn completes immediately after the model's response without waiting for the process to finish. The `ResponseEvent::Completed` handler only checks `has_pending_input()` but does not account for running processes in `unified_exec_manager.process_store`.

**Impact:** Agent cannot observe or report on long-running commands. Autonomous experiment loops are impossible.

**Source:** [#14731 - Turn completes prematurely when unified_exec background processes are still running](https://github.com/openai/codex/issues/14731) (open)

### 4.5 Non-Interactive Mode Doesn't Close HTTP Connections

`codex exec` calls `std::process::exit()` throughout 13 error paths, terminating the process without properly closing HTTP connections. This bypasses `Drop` traits, causing session leaks on proxy servers that track concurrent connections.

**Impact:** Proxy servers accumulate stale sessions until TTL expires, potentially hitting connection limits.

**Source:** [#16443 - Non-interactive mode doesn't properly close HTTP connections on exit](https://github.com/openai/codex/issues/16443) (open, regression)

### 4.6 Exec Hangs Under systemd on WSL2

`codex exec` hangs when run under `systemd --user` on WSL2, but works in a normal shell. Likely related to terminal/PTY detection differences in the systemd service environment.

**Source:** [#15830 - codex exec hangs under systemd --user on WSL2](https://github.com/openai/codex/issues/15830) (open)

### 4.7 Exec Sandbox Panic Before Prompt Execution

`codex exec --sandbox workspace-write` can panic in `system-configuration` / `reqwest` / `opentelemetry-otlp` during initialization on macOS, before the prompt ever executes. The CLI crashes with thread panics rather than returning an error.

**Source:** [#16908 - codex exec --sandbox workspace-write panics before prompt execution](https://github.com/openai/codex/issues/16908) (open)

## Category 5: MCP Integration Issues

### 5.1 MCP Tool Calls Always Cancelled in Exec Mode (Regression)

All MCP tool calls are immediately cancelled in `codex exec` mode with `"user cancelled MCP tool call"` since version 0.117.0. The MCP elicitation feature became default-on between 0.116.0 and 0.118.0, and since exec mode doesn't support interactive input, MCP tool calls that hit the elicitation/approval path are auto-cancelled.

**Impact:** Total blocker for any exec-mode workflow using MCP tools. Workaround: pin to version 0.116.0.

**Source:** [#16685 - MCP tool calls always cancelled in exec mode with 'user cancelled MCP tool call'](https://github.com/openai/codex/issues/16685) (open, confirmed regression)

## Category 6: Sandbox & Permission Discrepancies

### 6.1 Exec `danger-full-access` More Restrictive Than Interactive

`codex exec` with `-a never -s danger-full-access` runs commands under materially different runtime restrictions than an interactive `codex` session with the same flags. Integration tests that pass in interactive mode fail in exec mode with `EPERM`, loopback binding, and Chromium sandbox errors on Linux/WSL2. Using `--dangerously-bypass-approvals-and-sandbox` works.

**Impact:** `danger-full-access` does not mean the same thing in exec mode as in interactive mode, and this difference is not documented.

**Source:** [#15696 - codex exec with danger-full-access still hits EPERM/loopback restrictions](https://github.com/openai/codex/issues/15696) (open)

### 6.2 Unicode Confusable Characters Bypass Exec Policy

Unicode confusable characters can bypass exec policy matching. For example, a command using a Cyrillic `a` instead of a Latin `a` could bypass a policy rule that blocks the Latin version.

**Source:** [#13095 - Unicode confusable characters can bypass exec policy matching](https://github.com/openai/codex/issues/13095) (open)

## Category 7: Output & Display Issues

### 7.1 Duplicate Output in Non-Interactive Mode

`codex exec` prints the final assistant message twice: once during `EventMsg::AgentMessage` handling (stderr) and again in `print_final_output()` (stdout). A fix was merged but the issue remained open as of April 2026.

**Impact:** Log parsing and output capture get duplicate content. Workaround: pipe stdout only.

**Source:** [#12566 - codex exec non interactive mode prints duplicate output](https://github.com/openai/codex/issues/12566) (open, fix in progress)

### 7.2 Repeated File Update Diffs in Non-Interactive Mode

Non-interactive mode repeatedly outputs the full cumulative diff of all previously changed files after each new file change, rather than showing only the incremental diff for the current step. Reproduced across multiple models, appears to be a logging/display issue rather than model behavior.

**Source:** [#6511 - repeated "file update:" in non-interactive mode](https://github.com/openai/codex/issues/6511) (open)

### 7.3 JSON/XML Truncation Breaks Parseability

When tool outputs return large JSON or XML responses exceeding the `tool_output_token_limit` (default 10,000 bytes), truncation treats structured data as plain text, breaking the JSON/XML structure. The truncated output cannot be parsed by standard parsers. Claude Code handles this correctly.

**Impact:** API responses and config files become unparseable after truncation.

**Source:** [#9504 - JSON and XML Structured Data Gets Truncated in Ways That Break Parseability](https://github.com/openai/codex/issues/9504) (open, "as designed")

## Category 8: Token Usage & Cost Issues

### 8.1 UnifiedExec Inflates Token Usage

The `ExecCommandToolOutput::response_text()` payload includes verbose wrapper fields (`Command`, `Chunk ID`, `Wall time`, `Process exited/running`, `Original token count`, `Output:`) in the model-visible transcript. Additionally, `write_stdin()` poll responses re-attach `session_command`, so long interactive sessions keep re-echoing the same command string. Even with 94.5% cache hit rates, individual turns can burn 70k uncached tokens.

**Impact:** Unexpectedly fast usage consumption in normal repo-inspection workflows.

**Source:** [#14750 - UnifiedExec appears to inflate uncached prompt suffixes](https://github.com/openai/codex/issues/14750) (open)

### 8.2 Session JSONL Bloat from Repeated Instructions

Codex CLI persists `turn_context.payload.user_instructions` (derived from `AGENTS.md`) verbatim on every turn even when unchanged. In a real session: 37MB JSONL file, 1,759 entries, 1 unique blob, 16MB of duplicated instruction text.

**Impact:** Long-lived sessions (Discord/Slack adapters) generate massive session files. Naive log tools that sum token snapshots massively overcount.

**Source:** [#10403 - Session JSONL bloat: repeated identical AGENTS-derived user_instructions per turn](https://github.com/openai/codex/issues/10403) (closed)

## Category 9: Flag & Configuration Surprises

### 9.1 `--full-auto` Overrides Explicit `--sandbox` Flags

`--full-auto` overrides explicit `--sandbox` flags in both directions. Even `--sandbox danger-full-access` gets downgraded to `workspace-write` when `--full-auto` is used. This non-obvious precedence can silently lock down intended escalations.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Tests 46-47)

### 9.2 Argument Ordering Matters but Isn't Documented

The `-a` (approval) and `--ask-for-approval` flags are GLOBAL flags that must precede `exec`. Placing them after `exec` produces unexpected argument errors. Similarly, `--search` is a global flag; the exec-mode workaround is `-c web_search=live`. The `-i` (image) flag must come AFTER the prompt, not before.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Tests 22, 24, 39)

### 9.3 `--ephemeral` + `resume` Creates a New Session Silently

Attempting to resume an ephemeral session (which was never persisted to disk) does not error. It silently creates a new session with a different thread ID, breaking session continuity without warning.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Test 48)

### 9.4 Approval Policy Silent Downgrade in Exec Mode

`--full-auto` is documented as setting `approval: on-request`, but exec mode silently downgrades this to `approval: never` because there is no interactive user to prompt. The stderr displays the downgraded value.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Test 14)

### 9.5 Stdin vs Argument Prompt Conflict

When both stdin and a positional prompt argument are provided, the CLI silently ignores stdin and uses only the argument. Stdin is only read when no prompt argument is provided or when `-` is explicitly used.

**Source:** [Codex CLI exec mode experiments gist](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770) (Test 5)

## Summary Statistics

| Category | Open Issues | Closed/Fixed | Total |
|----------|-------------|-------------|-------|
| Event Stream Schema & Contract | 6 | 1 | 7 |
| Auth & Billing Differences | 2 | 0 | 2 |
| Structured Output (`--output-schema`) | 2 | 2 | 4 |
| Process Lifecycle & Hanging | 5 | 1 | 6 |
| MCP Integration | 1 | 0 | 1 |
| Sandbox & Permission | 2 | 0 | 2 |
| Output & Display | 3 | 0 | 3 |
| Token Usage & Cost | 1 | 1 | 2 |
| Flag & Configuration | 5 (ungithub'd) | 0 | 5 |

## Key Takeaways for Claudine Integration

1. **Do not trust event `status` fields at face value.** Abandoned commands are marked `"completed"` when they should be `"failed"`. Parse `exit_code` explicitly.

2. **Do not rely on process exit code.** `codex exec` exits 0 even when inner commands fail. Parse the JSONL stream for `item.completed` events with `status: "failed"`.

3. **Expect incomplete event streams.** Reasoning events are dropped, `PostToolUse` may be missing, `ItemStarted` without matching `ItemCompleted` is possible.

4. **API key auth produces a different stream than web auth.** Reasoning items are missing with API key auth. This is a confirmed bug.

5. **Rate limit data is unavailable in exec mode.** `rate_limits` is always null. Build alternative usage tracking.

6. **MCP tools may be broken in newer versions.** Version 0.117.0+ has a regression that cancels all MCP tool calls in exec mode.

7. **`--output-schema` is unreliable with tools.** When MCP servers or tools are active, schema validation is silently dropped. Always validate output independently.

8. **Implement idle timeouts carefully.** Long reasoning stretches produce no events. Use generous timeouts or detect `turn.started` without `turn.completed` rather than event silence.

9. **Handle `--full-auto` flag precedence.** It overrides explicit `--sandbox` settings, potentially locking down access you intended to grant.

10. **The format is not versioned.** Field names, event types, and event content can change between CLI releases without notice.

## Sources

### GitHub Issues (openai/codex)

- [#1673 - Provide JSON Schema for --json flag output](https://github.com/openai/codex/issues/1673)
- [#2288 - CLI flag to save trajectory/output as JSON](https://github.com/openai/codex/issues/2288)
- [#4776 - JSON output mode docs are out of date](https://github.com/openai/codex/issues/4776)
- [#5360 - output_schema is not supported for Chat Completions API](https://github.com/openai/codex/issues/5360)
- [#5773 - codex exec --json hangs with --image flag](https://github.com/openai/codex/issues/5773)
- [#6511 - repeated "file update:" in non-interactive mode](https://github.com/openai/codex/issues/6511)
- [#9504 - JSON/XML truncation breaks parseability](https://github.com/openai/codex/issues/9504)
- [#10403 - Session JSONL bloat](https://github.com/openai/codex/issues/10403)
- [#10746 - --json doesn't output reasoning items with API Key auth](https://github.com/openai/codex/issues/10746)
- [#12566 - exec non-interactive mode prints duplicate output](https://github.com/openai/codex/issues/12566)
- [#13095 - Unicode confusable characters bypass exec policy](https://github.com/openai/codex/issues/13095)
- [#14343 - --output-schema support for exec resume](https://github.com/openai/codex/issues/14343)
- [#14462 - exec stalls in mounted workspace](https://github.com/openai/codex/issues/14462)
- [#14470 - exec --json resume hangs with MCP helpers](https://github.com/openai/codex/issues/14470)
- [#14691 - abandoned items emitted with wrong status](https://github.com/openai/codex/issues/14691)
- [#14728 - rate_limits always null in exec mode](https://github.com/openai/codex/issues/14728)
- [#14731 - turn completes prematurely with background processes](https://github.com/openai/codex/issues/14731)
- [#14736 - include model name in exec --json output](https://github.com/openai/codex/issues/14736)
- [#14750 - UnifiedExec inflates token usage](https://github.com/openai/codex/issues/14750)
- [#15451 - --json and --output-schema silently ignored with MCP](https://github.com/openai/codex/issues/15451)
- [#15460 - OAuth token registration fails in automation](https://github.com/openai/codex/issues/15460)
- [#15536 - exec exits 0 even when commands fail](https://github.com/openai/codex/issues/15536)
- [#15696 - exec danger-full-access more restrictive than interactive](https://github.com/openai/codex/issues/15696)
- [#15830 - exec hangs under systemd on WSL2](https://github.com/openai/codex/issues/15830)
- [#16246 - PostToolUse missing for exec session tools](https://github.com/openai/codex/issues/16246)
- [#16443 - non-interactive mode doesn't close HTTP connections](https://github.com/openai/codex/issues/16443)
- [#16552 - codex_output_schema root type violation](https://github.com/openai/codex/issues/16552)
- [#16685 - MCP tool calls always cancelled in exec mode](https://github.com/openai/codex/issues/16685)
- [#16908 - exec sandbox panic before prompt execution](https://github.com/openai/codex/issues/16908)

### Community & Documentation

- [Codex CLI exec mode experiments: 81 flag/feature tests](https://gist.github.com/alexfazio/359c17d84cb6a5af12bac88fa1db9770)
- [OpenAI Non-interactive mode docs](https://developers.openai.com/codex/noninteractive)
- [OpenAI Codex CLI features docs](https://developers.openai.com/codex/cli/features)
- [OpenAI Codex CLI reference docs](https://developers.openai.com/codex/cli/reference)
- [Hacker News: Codex CLI discussions](https://news.ycombinator.com/item?id=43708025)
- [OpenAI Community: Codex usage limits discussion](https://community.openai.com/t/codex-usage-after-the-limit-reset-update-single-prompt-eats-7-of-weekly-limits-plus-tier/1365284)
- [SmartScope: Fix Codex CLI Re-connecting Loop](https://smartscope.blog/en/generative-ai/chatgpt/codex-cli-reconnecting-issue-2025/)
