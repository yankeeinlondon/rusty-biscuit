# Non-Interactive Sessions

Claudine wraps agentic CLIs in both interactive and non-interactive modes. Non-interactive sessions unlock structured streaming, the typed semantic event pipeline, token/cost tracking, the harness validation system, timeouts, and programmatic recovery — none of which are available in interactive mode.

A session becomes non-interactive when:

- A prompt is provided (positional argument, stdin, or composition file)
- `claudine compose` or `claudine inline-compose` is used and the resolved session mode is non-interactive (the default — see below)

`compose` and `inline-compose` default to non-interactive but can be switched to an interactive provider session by `-i` / `--interactive` or an `interactive: true` frontmatter property (resolution precedence: `--no-interactive` > `--interactive` > frontmatter > default). `claudine sequence` is always non-interactive automation and rejects `interactive: true` frontmatter. See [Composition — The `--interactive` and `--no-interactive` Flags](composition.md#the---interactive-and---no-interactive-flags).

## Information Density Contract

Non-interactive sessions are blind from the caller's perspective: there is no TTY to query, no interactive feedback loop, and the wrapped provider may take minutes or hours to complete. Claudine compensates by surfacing **as much information as possible**, consistent across providers wherever the providers themselves are consistent. The guiding rule:

- Anything the provider tells us about a tool call, a thought, a warning, or an error reaches the user.
- Repetitive content is collapsed at most once (e.g. the silent-extension allowlist).
- Format is consistent across providers; field availability is not, because providers themselves vary.

## Structured JSON Streaming

In non-interactive mode, Claudine instructs the wrapped provider to emit structured JSON output instead of plain text. Each provider has its own protocol and flags:

| Provider | Protocol | Flag(s) Applied |
|----------|----------|-----------------|
| Claude | StreamJson | `--print --output-format stream-json` |
| Codex | Jsonl | `exec` entrypoint + `--json` |
| Gemini | StreamJson | `--output-format stream-json` |
| Kimi Code | WireJsonRpc | `--wire` (JSON-RPC 2.0 over stdin/stdout) |
| OpenCode | Ndjson | `run` entrypoint + `--format json` |
| Qwen Code | StreamJson | _(automatic)_ |
| Goose | — | No structured streaming support |

Non-interactive runs also force a safety appendix into the effective system prompt. Claudine looks for `<repo-root>/.claudine/non-interactive.md`, then `~/.claudine/non-interactive.md`, and otherwise falls back to a built-in warning that tells the provider not to ask permission questions or request user input, and to avoid commands that would require an interactive TTY or follow-up stdin input.

### Parser Architecture

Each provider has a typed protocol module under [`claudine/lib/src/stream/protocol/`](../../lib/src/stream/protocol/) (one file per provider) plus a semantic parser under [`claudine/lib/src/stream/`](../../lib/src/stream/) (e.g. `claude_semantic.rs`, `opencode_semantic.rs`).

- **Protocol modules** define a serde-derived `*Event` enum tagged on `"type"`. Every field is `#[serde(default)]`, so format evolution never breaks deserialization. Unknown event types fail typed deserialization and are routed to a fallback arm that emits a `SemanticEvent::ProviderExtension` so nothing is dropped.
- **Semantic parsers** implement `SemanticStreamParser`. Each line is parsed first to `serde_json::Value` (preserves the malformed-line warning path), then to the provider-specific tagged enum. Successful parses dispatch to handler methods that translate provider events into provider-agnostic [`SemanticEvent`](../../lib/src/stream/semantic.rs) variants.

The `SemanticEvent` model is the cross-provider contract:

| Variant | Meaning |
|---------|---------|
| `SessionStart` | Session ID + model identified |
| `TurnStart` / `TurnComplete` | Turn boundaries; carries token usage / cost / duration on completion |
| `OutputText` | Assistant response text (deltas) |
| `Reasoning` | Thinking / reasoning text (deltas) |
| `ToolCall` / `ToolResult` | Tool invocation request / response |
| `SubagentStart` / `SubagentStop` | Subagent lifecycle |
| `FileChange` | Provider-reported file mutation |
| `PlanUpdate` | Step-of-plan progress |
| `Info` / `Warning` / `Error` | Diagnostic events; `Error` carries a `SemanticErrorKind` and a `terminal: bool` |
| `PermissionRequest` | Agent asked for a permission decision |
| `ProviderExtension` | Catch-all for parseable but not-yet-typed kinds |

### LiveSemanticSink

The CLI-side consumer is [`LiveSemanticSink`](../../cli/src/commands/wrap/live_semantic_sink.rs). For every `SemanticEvent` it receives, it does five things in order:

1. Updates [`LiveMetricsState`](../../lib/src/stream/progress.rs) so the prompt-scoped timing monitor (and the step-silence detector behind `step_timeout`) can observe the activity clock.
2. Updates the cached session ID and model from envelope events.
3. Updates the structured-summary tool-name rollup.
4. Forwards `OutputText` to the stdout text renderer and `Reasoning` to the stderr `BlockQuote` renderer.
5. Renders a status / block to stderr, dispatches the event to claudine's hook pipeline, and writes a JSONL row to the semantic event log.

Every emission flows through a [`SectionTracker`](../../cli/src/commands/wrap/section.rs) that enforces the 9-section model documented below, so callers never have to reason about blank-line spacing.

## The 9-Section Model

`LiveSemanticSink` enforces a strict ordering on stderr output — every line is tagged with one of nine sections, and the tracker inserts exactly one blank line between section transitions:

| # | Section | Content |
|---|---------|---------|
| 1 | `Execution` | The execution header (provider, mode, source) — first thing emitted |
| 2 | `Env` | Environment details (working dir, repo, git state) when verbose |
| 3 | `SystemPrompt` | The effective system prompt rendered as a `BlockQuote` (`▌ ` border) |
| 4 | `AgentPrompt` | The composed prompt rendered as a `BlockQuote` (`▌ ` border) |
| 5 | `SessionAndModel` | The session ID + model line (always emitted; survives `--quiet` and `--silent`) |
| 6 | `Thinking` | Reasoning prose rendered as a `BlockQuote` (gray `▌ ` border, dim italic) |
| 7 | `ToolUseAndEvents` | Tool calls, tool results, subagents, info/warning lines, error blocks |
| 8 | `FinalStdout` | The agent's final response, on stdout (markdown-rendered when TTY, raw otherwise) |
| 9 | `TrailerMetadata` | The summary line + verbose details |

Within a section, consecutive blank lines collapse to one.

Section interleaving is provider-dependent. Claude and Codex keep `FinalStdout` strictly at the end of the turn, so `ToolUseAndEvents` never re-opens once stdout starts. For Claude specifically, assistant prose that shares an envelope with a `tool_use` is reclassified to `Reasoning` instead of `OutputText`; this keeps "Let me investigate..." style tool-preface narration in `Section::Thinking` and avoids opening `FinalStdout` mid-run. OpenCode (and any provider that emits true assistant text mid-turn) interleaves `FinalStdout` with `ToolUseAndEvents` — the sink re-enters `FinalStdout` on each `OutputText` event and returns to `ToolUseAndEvents` on the next tool call.

Because interleaving is supported, the transition separator rule has two suppression conditions that together guarantee no consecutive blank lines in the combined stdout+stderr output:

1. **Visual blank row.** `LiveSemanticSink` maintains a unified `at_blank_row` flag that tracks whether the last emission to *either* stream left the output at a visually blank row. The flag is set when a blank stderr line is written (section separator or explicit blank) or when stdout text ending with `\n\n` is forwarded. It is cleared when non-blank content is written to either stream. When the flag is true, the automatic section-transition separator is suppressed because injecting another blank would produce consecutive blank lines.

2. **Leading newline in OutputText.** When the `OutputText` payload itself starts with `\n`, the text provides its own visual break. The separator into `FinalStdout` is suppressed so the text's own newline creates the gap rather than a redundant stderr blank.

These two rules work together with the `SectionTracker`'s same-section blank dedup to ensure that tool transitions are tightly packed (single newline), section changes have exactly one blank line, and no combination of stream output and injected separators ever produces `\n\n\n` (two consecutive visual blank lines).

## Tool Call Rendering

Every `ToolCall` and `ToolResult` produces a single line in `Section::ToolUseAndEvents` via [`ToolCallDisplay`](../../lib/src/stream/tool_display.rs):

```text
→ Bash(bash ls -la)
← Bash(successful, bash ls -la)

→ Read(/etc/hosts)
← Read(successful, /etc/hosts)

→ Bash(bash git status)
← Bash(error)

→ Task(Investigate hanging behavior in StreamTextRenderer)
← Task(successful)
```

Format rules:

- **Outgoing call:** `→ {DisplayName}({summary})`. The summary is dim-italic.
- **Incoming result:** `← {DisplayName}({slot})`. The slot resolves status and summary independently and renders both when both are available, so a successful tool result reads symmetrically with the outgoing call. The status word (`successful` / `pending` dim-italic, `error` red+bold) appears first; a derived summary follows when one can be extracted from the cached input or the result output.
- **Summary precedence:** `extract_tool_summary` runs against `extra["input"]` first (the cached request-side input from the paired tool call), then falls back to `output` when no input-derived summary is available. File tools surface the requested path; shell tools surface the executed command. Tools that `extract_tool_summary` does not know about render as the bare status (`← Read(successful)`) rather than fabricating content.
- **Shell tools** (`Bash`, `bash`, `run_command`) prepend the canonical shell name to the command (`bash ls -la`) so the user can reason about how the line would actually execute.
- **Task** prefers `description → subject → prompt → task` for its summary in that order.
- **Display name humanization:** known tools pass through unchanged (`Bash`, `Read`, …); MCP-shape tools (`mcp__server__tool`) become `Server Tool`; unknown tools are title-cased on `_` boundaries.
- **User-controlled content** (commands, paths, URLs, raw JSON fallbacks) is escaped so stray `<`, `>`, `{`, or `\` cannot be interpreted as prose markup.

The full input/output JSON is **never** dumped verbatim for known tools. The raw event remains available in the JSONL semantic log.

### OpenCode Phase Markers

OpenCode emits `step_start` and `step_finish` wire events to bracket each turn-internal phase. The OpenCode semantic parser still maps these to `SemanticEvent::Info { extra["step_phase"] }` so they continue to flow through `LiveMetrics` and the JSONL log, but the live sink suppresses any OpenCode `Info` event that carries `extra["step_phase"]` from stderr — they were visually noisy and added blank-line gaps around real tool lines without contributing user-visible meaning. Other providers' `Info` events are unaffected.

## Thinking (Reasoning) Rendering

Every `SemanticEvent::Reasoning` is rendered into `Section::Thinking` as a `BlockQuote` with the wider `▌ ` border (matching System Prompt and Agent Prompt) and the default gray `Tailwind::Gray500` accent:

```text
▌ The user is asking about when the NFL draft is in 2026. This is a factual
▌ question that requires web search since I don't have real-time information
▌ about specific dates for future events.
```

Provider coverage:

| Provider | Reasoning Source |
|----------|------------------|
| Claude | `content_block_delta` with `delta.type = "thinking_delta"` |
| Codex | `item.completed` / `item.updated` with `item.type = "reasoning"` |
| OpenCode | Top-level `{"type":"reasoning","text":"…"}` (also resolves nested `part.text`) |
| Kimi | `ContentPart` event with `payload.type = "think"` (per-token deltas concatenated into a thinking block at `TurnEnd` or when the part type changes) |
| Gemini | _not yet surfaced_ — `thinkingConfig.includeThoughts` controls model-side emission but no dedicated stream-json event has been observed in this repo's research or fixtures |
| Qwen | _not yet surfaced_ — when `enable_thinking` is on, the model emits `<think>…</think>` blocks inline inside normal message content rather than as a separate event |

Goose does not surface reasoning in its stream protocol; nothing is rendered for it. Adding a typed `Reasoning` variant for Gemini or Qwen requires first observing the wire format; contributions welcome.

Claude also promotes assistant prose from mixed `assistant` envelopes that contain both `text` and `tool_use` into `Reasoning`. Those "tool preface" lines are planning narration, not final answer text, and rendering them in `Section::Thinking` avoids stray stdout paragraphs and extra section separators during tool-heavy turns.

## Warning Rendering

Warnings render as a single `Status::Warning` line (yellow icon) in `Section::ToolUseAndEvents`:

```text
⚠ rate limited; retrying in 30s
```

Two suppressions are applied:

- `Malformed JSON on line N` warnings (which a permissive provider may emit for non-JSON lines mixed into its structured output) are not rendered to stderr but are still dispatched and logged.
- The Claude rate-limit warning is suppressed when the user is on a Subscription (no `ANTHROPIC_API_KEY`); dispatch and JSONL log still fire.

## Error Rendering

`SemanticEvent::Error` carries a typed `SemanticErrorKind`:

| Kind | Border Color | Label |
|------|--------------|-------|
| `Configuration` | `Tailwind::Orange700` | `Configuration Error` |
| `AgentNative` | `Tailwind::Red700` | `Agent Error` |
| `ApiRemote` | `Tailwind::Red700` | `API Error` |
| `Interrupted` | `Tailwind::Yellow700` | `Interrupted` |
| `Unknown` | `Tailwind::Red700` | `Agent Error` |

Each error renders as a `BlockQuote` with `▌ ` border in the matching color, in `Section::ToolUseAndEvents`:

```text
▌ Agent Error (opencode)
▌ Tool execution failed: command terminated with non-zero exit
```

The same recipe (`with_border("▌ ").with_left_block_color(...)`) is used by [`AgentErrorReport`](../../cli/src/output/error_report.rs) for end-of-run exit-code formatting; `SemanticErrorKind` maps directly onto `AgentErrorCategory` via `From` so a typed live error can be promoted to a richer end-of-run report when desired.

`terminal: bool` is independent of `kind` — `terminal: true` errors map to `AgenticEvent::TurnError` for hook dispatch; non-terminal errors map to `AgenticEvent::Notification`. The kind remains purely classificatory regardless.

### Provider Error Classification

| Provider | Source signal | Mapped kind |
|----------|---------------|-------------|
| Claude | `error.type = "overloaded_error"` / 5xx | `ApiRemote` |
| Claude | `error.type = "invalid_request_error"` | `Configuration` |
| Codex | `error.type = "usage_limit_reached"` | `ApiRemote` |
| Codex | `error.type = "auth"` / `account` | `Configuration` |
| OpenCode | `error_type` literal table | `AgentNative` (default) |
| Gemini / Qwen | regex on `error.message` | best-effort |
| Kimi | JSON-RPC `error.code` from `kimi_cli/wire/jsonrpc.py::ErrorCodes` (`-32004 AUTH_EXPIRED` → `Configuration`, `-32003 CHAT_PROVIDER_ERROR` → `ApiRemote`, `-32601`/`-32602` → `Configuration`, others → `AgentNative`); message-keyword fallback covers rate/quota/billing → `ApiRemote`, auth/api-key/permission → `Configuration`, interrupt/cancel → `Interrupted` |
| Any | SIGINT / SIGTERM exit | `Interrupted` |

Anything unmatched defaults to `Unknown`, which renders as a red `Agent Error` block.

## Output Streaming and Idle Flush

Assistant text reaches stdout through [`StreamTextRenderer`](../../cli/src/commands/wrap/exec.rs). It accumulates lines into a block buffer and renders them through Darkmatter at boundaries (paragraph break, code-fence close, stream-safe list item). Partial trailing lines stream raw the moment they arrive, so the user sees progress as the agent types.

When a streamed fragment is rendered as its own Markdown document, Darkmatter may add a trailing blank line that was not present in the provider's source text. `StreamTextRenderer` trims those synthetic trailing blank lines back to the provider-authored newline count before writing to stdout, which keeps standalone headings and list items from turning into loose, double-spaced output during streaming.

### Sentence-Level Early Flush

For non-fenced prose, the renderer additionally flushes when the buffered block has grown past the `SENTENCE_FLUSH_MIN_BYTES` threshold (200 bytes today) **and** the latest line ends with sentence-terminating punctuation (`.`, `!`, `?`, optionally followed by a closing quote / bracket / parenthesis). This keeps long single-paragraph monologues streaming in roughly sentence-sized chunks instead of waiting for a blank-line boundary, while short responses (`"OK."`) and lines without terminators stay buffered. Code fences and list items are excluded — those branches return earlier in `process_line`.

### Idle Flush

Block buffers are also flushed when a dedicated **flush-if-idle ticker** observes the renderer has been quiet longer than the silence window (fixed **30 s**). This guarantees that a final paragraph emitted by a slow-to-close provider never sits invisible in the buffer waiting for an EOF that may never arrive.

The ticker runs independently from the prompt-scoped timing monitor (described in the next section). On every 30-second wake-up it acquires the renderer lock, calls `flush_if_idle(Duration::from_secs(30))`, and releases the lock. There is no `Status::Info` line emitted from this ticker — its sole purpose is stdout correctness.

### Prompt-Scoped Timing Surface

Composition runs emit a prompt-scoped timing header anchored on the prompt's start time, plus two optional fire-once warnings configured via harness frontmatter (`timeout_warn`, `step_timeout_warn`). See the "Timing Surface" section of [`composition.md`](composition.md) for the full grammar, duration rendering rules, and frontmatter contract.

Wrapper passthrough runs with no prompt file skip the header entirely; their only timing output (beyond the structured event sink) is whatever the provider prints.

### OpenCode Silent-Stall Recovery

OpenCode has an additional recovery path because its structured stdout stream is not a reliable process-lifecycle contract:

- Some successful runs end on `step_finish.part.reason = "stop"` and then never exit cleanly.
- Some subagent-heavy runs go completely silent after the last visible `Task(...)` completion even though the parent process remains alive. OpenCode dispatches the `task` tool as an ordinary tool, so the parent stream sees `tool_use` / `tool_result` for each subagent (not `task_started` / `task_completed`); after every dispatched task returns, the parent sometimes never emits a final `step_finish` with `reason = "stop"`.

To avoid indefinite hangs in those cases, Claudine's OpenCode wait loop polls `LiveMetricsState` while it waits for process exit. The hang-recovery rule fires when **all** of these hold:

- At least one `step_finish` event has been observed (i.e. `provider_status` is not `None`). This guarantees the parser has crossed at least one step boundary, which rules out slow-startup false positives.
- No tools and no subagents are in flight (`in_flight` and `in_flight_subagents` are both empty).
- The stream has been silent for the recovery window (default **120 s**).

When all conditions hold, Claudine SIGTERMs the hung process and treats the run as successful (`CompletedButHung`). The synthesized message names the last observed `step_finish.reason` (`stop` or `tool-calls`) so operators can distinguish a clean-finish hang from a parallel-tool-dispatch hang.

## Timing Threads

Each structured-stream run spawns up to two dedicated timing threads:

1. **Flush-if-idle ticker** (always on for structured runs): wakes every 30 s and surfaces any buffered markdown that has been idle for 30 s. Emits no status line itself.
2. **Prompt-scoped timing monitor** (composition runs only): anchored on the prompt's start time, emits a `⏱️ {HH:MM} {TZ} running the <prompt> prompt[ for <duration>]` header at `t=0` and every 10 minutes thereafter. The same thread also watches for the fire-once `timeout_warn` and `step_timeout_warn` thresholds and emits Status WARN lines when those cross. Full grammar is documented in [`composition.md`](composition.md).

The legacy heartbeat tick (`240s · 10 done`) and the stall-threshold stderr warning (`no provider activity in 2m — …`) were removed as part of the 2026-04-19 timing revamp; the prompt-scoped surface replaces them.

## Agent Session ID

When a non-interactive session starts, the agent returns a session ID in its structured JSON output. Claudine extracts this and emits it to stderr immediately — regardless of `--quiet` or `--silent` — because it is essential operational tracking info:

```text
- OpenCode session ID ses_abc123de · minimax/MiniMax-M2.7-highspeed
```

The session ID is truncated to 12 characters in the display but stored in full for logging and resume.

### Extraction per Provider

Most providers emit a dedicated `init` or `session_start` event at the beginning of the stream. OpenCode is an exception: the session ID arrives in the first `step_start` payload as `sessionID` (camelCase). The OpenCode parser synthesizes a `SessionStart` event when it first sees this.

### Distinction from CLAUDINE_SESSION_ID

| | Agent Session ID | CLAUDINE_SESSION_ID |
|---|---|---|
| **Source** | Provider's API response | UUID generated by Claudine wrapper |
| **Purpose** | Resume capability, provider-side tracking | Harness handlers, log correlation |
| **Availability** | Only in structured streaming (non-interactive) | Always (set as env var for child process) |
| **Visibility** | Emitted to stderr at session start | Available via `$CLAUDINE_SESSION_ID` env var |

## Token Usage and Cost Reporting

At the end of a non-interactive session, Claudine displays a summary line in `Section::TrailerMetadata`:

```text
✓ 12.3s · 1.2K input tokens · 567 output tokens · 89 cached tokens · $0.0042 · 3 tool calls
```

The summary is built from `StreamExecutionSummary`, which accumulates across all turns:

- **Duration** — wall-clock time from process start to exit
- **Token usage** — input, output, cache read (provider-dependent)
- **Cost** — USD cost reported by the provider (when available)
- **Tool calls** — count of tool invocations observed in the stream
- **Turns** — number of agent turns

### Verbosity Levels

| Mode | Session ID | Warnings | Errors | Summary Line | Verbose Details |
|------|-----------|----------|--------|--------------|-----------------|
| Normal | Always | Yes | Yes | Full | With `-v` only |
| `--quiet` | Always | Yes | Yes | Compact | No |
| `--silent` | Always | No | Yes | No | No |

**Normal summary:** `✓ 12.3s · 1K input tokens · 567 output tokens · $0.0042 · 3 tool calls`

**Quiet summary:** `✓ 12.3s · 1K→567 tokens · $0.0042`

Errors render at every verbosity level (including `--silent`) because suppressing them would defeat their purpose.

### Verbose Mode (`-v`)

When `-v` is used, an additional details line follows the summary:

```text
  session: ses_abc123def456 · model: claude-sonnet-4 · turns: 3 · tools used: Read, Edit, Bash
```

This includes the full (non-truncated) session ID, model name, turn count, per-tool invocation names, stop reason, and rate-limit info if the provider was throttled.

## Prompt Delivery

Non-interactive sessions use two delivery mechanisms depending on the provider:

**stdin pipe** — the prompt is written to the child's stdin, then the pipe is closed (EOF). This avoids `ENAMETOOLONG` errors when composed prompts exceed OS argument length limits. Used by: Claude.

**Positional/flag arguments** — the prompt is passed as a CLI argument. Some providers convert positionals to flags (e.g., Gemini converts to `--prompt`). Used by: Codex, Gemini, Goose, OpenCode.

**Wire JSON-RPC `prompt` request** — the prompt is delivered as the `params.user_input` of a JSON-RPC `prompt` request after the `initialize` handshake completes. Stdin remains open for the duration of the turn so Claudine can answer Kimi's `ApprovalRequest`/`QuestionRequest`/`ToolCallRequest`/`HookRequest` envelopes inline. Used by: Kimi Code (non-interactive only; interactive `kimi` runs continue to use the native `--prompt` flag).

Even when the prompt is delivered via CLI args, structured non-interactive runs do not inherit the caller's stdin. Claudine closes stdin for those child processes unless it is actively seeding prompt content, which prevents providers from lingering on an open terminal after the non-interactive task is already complete.

## Output Noise Filtering

Some providers emit debug/diagnostic lines mixed into their structured output. Claudine filters these in non-interactive mode using per-provider prefix lists:

- **stdout noise** — lines matching configured prefixes are silently dropped (e.g., Gemini's `[LocalAgentExecutor]` lines)
- **stderr noise** — similar filtering for stderr
- **Conditional stderr suppression** — some providers buffer all stderr and only surface it on non-zero exit (`suppress_structured_stderr_on_success`)

### OpenCode default-mode TUI leakage

When wrapping OpenCode with `--format json`, OpenCode still writes its default-mode TUI formatter output to stderr. Claudine suppresses those lines via the OpenCode wrapper's `stderr_noise_prefixes`:

| Prefix | Meaning |
|--------|---------|
| `✱ ` | Status bullet used for Glob/Grep/Read lines |
| `$ ` | Bare shell command echo lines |
| `> build ` | Session banner |
| `████ ` | Subheader marker |

The helper lives in [`claudine/cli/src/commands/wrap/profile.rs::opencode_default_tui_noise_prefixes()`](../../cli/src/commands/wrap/profile.rs). Add new entries there as future OpenCode releases surface additional formatter prefixes.

## Status Line Rendering on STDERR

Two rules protect the rendering surface from low-signal noise:

1. **No raw-JSON tails.** Both `summarize_provider_payload` (for `ProviderExtension` events) and `summarize_input` (for tool calls) walk a curated set of text locations — top-level string fields (`message`, `status`, `name`, `path`, `text`, `content`, `error.message`, `title`, `description`, …), nested content arrays (`message.content[*].text`, `item.content.parts[*].text`, …), and finally the first non-empty top-level string value. When none of those resolve to readable text, the sink renders only `provider/kind` (no ` · <payload>` tail) rather than a truncated JSON blob. The fidelity contract still holds because the full raw event is written to the JSONL semantic log regardless of how it renders.

2. **Silent-kind allowlist.** Some extension kinds are high-volume or fully redundant with other typed events. They are named in `SILENT_PROVIDER_EXTENSION_KINDS` and produce **no** stderr line at all (dispatch and JSONL logging still happen). Current entries:

    | Provider | Kind | Reason |
    |----------|------|--------|
    | Claude | `stream_event` | Partial assistant token deltas — already surfaced via `OutputText` |
    | Claude | `system/hook_started` / `system/hook_response` / `system/hook_progress` | Hook lifecycle already surfaced as semantic events |

    Add new entries in the same file rather than inventing new heuristics — explicit listing keeps the suppression visible and reviewable.

### Gemini user-prompt echo

Gemini replays the operator's own prompt back into the stream as a `message` event with `role: "user"` (and occasionally `role: "system"`). Since those never carry new information, the Gemini parser ([`claudine/lib/src/stream/gemini_semantic.rs`](../../lib/src/stream/gemini_semantic.rs)) drops them silently instead of emitting a `ProviderExtension` that the sink would then render as a status line. Assistant-role messages continue to route through `OutputText` as normal.

## Composition Workflows

`claudine compose` and `claudine inline-compose` are non-interactive entry points that add Markdown composition before provider execution:

**Compose** (`claudine compose <file-ref>`) — resolves a Markdown file through Darkmatter's composition engine (frontmatter variable substitution, `::shell` directives, `@` file references), then sends the composed content as a non-interactive prompt. No file mutations.

**Inline compose** (`claudine inline-compose <file-ref>`) — extracts the `prompt` frontmatter property, composes it, sends to the provider, then atomically replaces the document body with the provider's response. The original frontmatter is preserved and `last_updated` is set.

Both commands flow through the same unified pipeline: `execute_composition_request` builds a `HarnessPromptState` and calls `run_harness_loop()` with `HarnessPromptMode::Compose` for direct compose or `HarnessPromptMode::Inline` for inline compose. The loop re-parses the harness plan each attempt; bare documents (no harness frontmatter) yield the empty plan. The loop handles structured streaming, captured/non-structured fallback, summary emission, and inline closure through a single code path.

## Harness System

The harness wraps non-interactive prompts with timeout enforcement, shell-audit pre-flight, runtime attempt classification, and lifecycle recovery infrastructure. Gating, verification, and recovery are expressed through the prompt's [lifecycle stack](lifecycle.md) — `when:` guards plus the `error` / `skip` / `proxy` / `retry` / `resume` / `defer` lifecycle actions — not a separate validation/handler DSL.

### Gating, Verification, and Recovery

- **Gating** (does this run even start?) lives in the `initialize`/`start` stacks: guard with `when:` and `Skip`, `Proxy`, or `Error` out before the agent is invoked.
- **Verification** (did the run do what it claimed?) lives in the `success`/`finalize` stacks: raise an `Error` lifecycle action when a `when:` contract is unmet.
- **Recovery** (what to do on failure) lives in the `failure`/`blocked` stacks via `retry`, `resume`, or `proxy` — and, because flow control is universal, in any other event's stack too.

The retired `pre_checks` / `post_checks` / `handle_*` / `handle` / `deviate` frontmatter keys now reject with a typed `RemovedValidationKey` diagnostic; see [Composition — Migrating from the Retired Harness DSL](composition.md#migrating-from-the-retired-harness-dsl).

### Timeout

The harness supports two independent timeouts, both parsed from human-readable
strings (`30s`, `5m`, `2h`):

| Property | Frontmatter | CLI flag | Semantics |
|----------|-------------|----------|-----------|
| Wall clock | `timeout` | `--timeout <DURATION>` | Deadline for total runtime like 30s, 5m, 2h. Enforced by the watchdog ticker. |
| Step silence | `step_timeout` | `--step-timeout <DURATION>` | Deadline for silence between stream events. Resets on every `SemanticEvent`; fires when `last_event_at` is older than the budget. |

At either deadline, Claudine sends SIGTERM to the child; after a 5-second
grace period, SIGKILL. Both timeouts surface as the same timeout failure and
route to the `failure` lifecycle event, where a `Retry` or `Resume` action
can recover either case.

**Wall-clock precedence.** When both budgets expire in the same poll, the
wall-clock timeout wins — the loop checks it first, and the step-silence
branch skips when `early_termination.is_some()`.

**Streaming-only.** `--step-timeout` / `step_timeout` is enforced only when
the provider runs in structured-stream mode. Capture-mode and passthrough
runs (notably Goose) emit a warning and ignore the field.

**Interactive restriction.** Both `--timeout` and `--step-timeout` are
restricted to non-interactive mode — combining either with a session that
resolves to interactive (via `--interactive` or `interactive: true`
frontmatter) is a hard error. The conflict is checked against the resolved
session mode, and the diagnostic names the source (`--interactive` vs
`frontmatter`).

**CLI precedence.** CLI flags override frontmatter. On `compose`,
`inline-compose`, and `sequence`, the `--step-timeout DURATION` flag uses
the same duration parser as the frontmatter property and applies uniformly
across all sequence steps.
