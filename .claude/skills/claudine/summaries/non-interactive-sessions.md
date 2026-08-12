# Non-Interactive Session Contracts

Claudine’s provider wrappers depend on a simple but demanding contract: launch an agent without a human UI, deliver a prompt, select a parser-safe output format, supervise progress while it runs, and decide why it ended. Without that contract, a wrapper cannot reliably distinguish a successful answer from a stuck approval prompt, an authentication failure, a quota cap, a tool denial, a transport crash, or a provider stream change.

The contract matters because Claudine is not just invoking CLIs as subprocesses. It is normalizing provider behavior into a shared execution model: prompt delivery, session identity, tool activity, assistant output, usage, failures, timeout handling, and recovery decisions. Human-oriented output is insufficient for that job. Claudine needs structured stdout when a structured mode is selected, diagnostic or explicitly classified stderr, recognizable terminal events when a provider emits them, and a clear policy for runs that exit without a terminal stream event.

## Comparison Axes

A useful Claudine-facing contract has five parts.

First is invocation shape: the exact argv that forces non-interactive behavior, how prompt text is supplied, whether stdin is prompt text or a protocol, and how resume works.

Second is output format selection: whether the provider has human text, single final JSON, streaming JSON, schema-constrained final output, or a separate bidirectional protocol.

Third is the stream and event contract: framing, discriminator fields, event order, correlation identifiers, terminal events, unknown-event policy, and whether assistant/tool output arrives as deltas or completed items.

Fourth is exit semantics: whether a semantic terminal event exists, whether process exit is reliable, and how to classify runs that exit without a terminal stream event.

Fifth is operational-condition detectability: whether auth failures, caps, billing failures, permission denials, token usage, model identity, fallback, subagents, and human-in-loop hazards are visible as structured fields or only as text and exit status.

## Provider Contracts

Claude Code is closest to the ideal wrapper contract. The preferred invocation is:

```sh
claude -p "PROMPT" --output-format stream-json --verbose
```

`stream-json` is newline-delimited JSON on stdout, with `type` as the top-level discriminator and `type=result` as the semantic terminal event. It can expose session initialization, assistant/user messages, tool results, permission denials, API retries, rate-limit events, auth status, model fallback, thinking-token telemetry, hook events, usage, and final result metadata. Its main cost is parser complexity: some events are opt-in, plugin records can precede `system/init`, prompt suggestions can arrive after `result`, and there is no standalone versioned JSON Schema. Claudine should treat `result` as semantic completion, use process exit as a consistency check, and preserve unknown events for drift analysis.

Codex CLI has a clean but compact one-shot stream:

```sh
codex exec --json "PROMPT"
```

Stdout is JSONL. The stream starts with `thread.started`, then `turn.started`, item lifecycle records, and finally `turn.completed` or `turn.failed`. Tool and content details are flattened into `item.started`, `item.updated`, and `item.completed` with nested `item.type` values such as `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, and `error`. Codex is strong on turn lifecycle, file changes, tool calls, final assistant text, and usage, but weak on launch metadata: cwd, model, provider, auth kind, version, MCP inventory, sandbox mode, and approval policy are not emitted in exec JSONL. Claudine must capture those wrapper-side and classify auth/cap/billing failures from stderr, exit status, and error items when the stream is incomplete.

Gemini CLI supports headless streaming through:

```sh
gemini --output-format stream-json --prompt "PROMPT"
```

Its stream is a public projection of an internal event bus. Stdout is one JSON object per line, with events such as `init`, `message`, `tool_use`, `tool_result`, `error`, and `result`. Assistant text arrives as `message` deltas, and tool calls/results correlate through `tool_id`. `init` gives session/model metadata, and `result.stats` carries usage. The projection omits internal usage updates, session updates, tool progress, elicitation, subagent start, and custom events. Claudine should parse it as a stable public progress stream, not a full audit trail, and fall back to exit code and stderr when fatal errors bypass `result`.

Goose runs headlessly through `goose run`; Claudine’s safest form is:

```sh
goose run --quiet --output-format stream-json --name <claudine-run-id> -i -
```

`--quiet` is required for parse-safe stdout because otherwise Goose can print a human session banner before JSON. The stream has top-level `message`, `notification`, `error`, and `complete` events. Nested message content uses camelCase tags such as `text`, `toolRequest`, `toolResponse`, `actionRequired`, and `systemNotification`. Goose has no init event and does not emit session id, cwd, version, permission mode, configured extensions, or requested provider/model at stream start. `complete` carries token counts but no status or final answer, and an `error` can be followed by `complete`; Claudine should treat prior `error` as tainting the run. Generate a stable `--name`, record launch metadata wrapper-side, join tools by content id, and treat setup/auth/keyring failures before JSON as stderr/exit classifications.

Kimi Code’s strongest contract is not print JSONL but Wire mode:

```sh
kimi --wire --work-dir <repo> --afk
```

Wire is bidirectional JSON-RPC over newline-delimited stdin/stdout. Claudine must send `initialize`, then a `prompt` request, and treat the JSON-RPC response to that prompt as the turn terminal outcome. Events and requests are typed under `params.type`; tool calls/results, status updates, approvals, questions, hooks, external tool requests, and subagent events are visible in the protocol. Print mode exists:

```sh
kimi --print -p "PROMPT" --output-format stream-json
```

but it is a lossy projection that buffers deltas and drops many control events. Wire gives strong supervision but makes Claudine a protocol peer: it must answer or reject blocking `ApprovalRequest`, `QuestionRequest`, `HookRequest`, and `ToolCallRequest` according to policy. `--afk` is the safest unattended posture when human questions should be dismissed. Wire still omits some launch facts, such as stable resolved model/provider/auth metadata, so wrapper-side launch records remain necessary.

OpenCode’s subprocess contract is dual-source:

```sh
opencode run --format json --print-logs --log-level INFO -- "PROMPT"
```

Stdout NDJSON contains `step_start`, `text`, `reasoning`, `tool_use`, `step_finish`, and `error`. It emits completed text/reasoning blocks, terminal tool states, per-step tokens/cost, and errors. It does not emit user prompts, permission events, tool starts, resolved model identity, or a terminal success event. OpenCode consumes internal idle status but does not forward it to stdout, so process exit plus accumulated stdout/stderr evidence is normal completion classification. With `--print-logs --log-level INFO`, stderr carries parser-relevant lifecycle records: session creation, child-session lifecycle, LLM provider/model, step loop heartbeats, permission evaluations, HTTP/retry activity, auth failures, API failures, and provider limits. For OpenCode, selected stderr logs are part of the wrapper-grade contract, not disposable noise.

Qwen CLI’s preferred one-shot form is:

```sh
qwen -p "PROMPT" --output-format stream-json --include-partial-messages
```

`stream-json` emits one complete JSON object per stdout line. `--include-partial-messages` adds `stream_event` records for message starts, content deltas, tool input deltas, MCP progress, and active-goal updates. Normal completion is `type=result`, usually with `subtype=success` and `is_error=false`, but documented failures can exit with stderr only: max session turns can exit `53`, wall-clock budget aborts `55`, and interrupts `130`. Qwen has useful init/session metadata, but docs and source differ on `system/session_start` versus `system/init`; Claudine should accept both. Tool calls/results join by IDs, task/subagent events provide high-level lifecycle, and final `permission_denials` plus tool errors help classify denials. Auth, no-funds, quota, cost, file changes, and fallback remain partly heuristic.

## Researched Future Providers

Kilo Code and Pi are researched in the non-interactive session corpus but are not currently compiled Claudine providers. Their contracts matter for future adapter design.

Kilo’s likely first integration surface is:

```sh
kilo run --auto --format json --dir <cwd> "PROMPT"
```

Stdout is NDJSON with `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. It has session IDs, completed text, completed/errored tools, usage/cost/model metadata on step finish, and generic errors, but no terminal success event, no tool-call starts, no deltas, no forwarded permission ask/reply events, and no nested subagent stream. A mid-stream provider error can emit `error` and still exit `0`, so structured `error` outranks process exit. Success is inferred from exit `0` plus absence of errors.

Pi’s likely first integration surface is:

```sh
pi --mode json "PROMPT"
```

For deterministic runs, the researched form disables project-local resources unless explicitly wanted:

```sh
pi --mode json --no-approve --no-extensions --no-skills --no-prompt-templates --no-context-files "PROMPT"
```

Pi JSON mode emits a session header and live `AgentSessionEvent` records such as assistant message lifecycle, tool execution start/update/end, compaction, retry, turn end, and terminal `agent_end`. `tool_execution_update.partialResult` is accumulated output, not a guaranteed delta. Success requires inspecting `agent_end`, final assistant stop reason, retry records, process exit, and stderr. Pi has no built-in permission approval system comparable to several other CLIs, so side effects must be constrained with tool/resource flags and external sandboxing.

## Cross-Provider Differences

“Streaming JSON” does not mean one thing.

Claude is a rich semantic stream. It can drive many operational classifications directly from typed fields.

Codex is a clean turn/item stream. It is easy to parse, but it omits many launch and account facts.

Gemini is a public projection. It is useful for progress and final usage, but not a complete internal event bus.

Goose is a tool-centric conversation stream. It is parseable only with `--quiet`, has no init envelope, and its `complete` event is usage-oriented rather than success-oriented.

Kimi Wire is a protocol, not a passive stream. It gives strong visibility only if Claudine participates as a JSON-RPC peer.

OpenCode is a filtered stdout stream plus a classified stderr bridge. It has no stdout terminal success event.

Qwen is JSONL with useful partial-message support, but some terminal failures intentionally bypass final `result`.

Kilo and Pi reinforce the same design lesson for future adapters: some providers use process exit as part of ordinary completion, and some have richer protocol/server modes that should be separate adapters rather than squeezed into one-shot stdout parsers.

Terminal semantics vary accordingly. Claude and Gemini use `result` when emitted. Codex uses `turn.completed`/`turn.failed`. Goose uses `complete`, but that does not prove success. Kimi Wire uses the JSON-RPC response to `prompt`. OpenCode and Kilo have no stdout terminal success event. Pi uses `agent_end` but still requires final-message inspection.

Operational-condition detection follows the same gradient. Typed provider fields are highest confidence. Wrapper-captured metadata fills launch gaps. Stderr and exit status explain failures that occur before or outside the structured stream. Text classification is necessary for provider-specific quota, billing, auth, and no-funds messages, but those detections should be labeled lower confidence.

## Parser Implications

Claudine’s parsers should treat each provider stream as a provider-specific contract, then normalize into Claudine’s shared lifecycle model. There is no universal event schema underneath these CLIs.

Shared invariants:

- stdout parser input must be structured-only in the selected machine-readable mode.
- stderr is diagnostic evidence unless a provider contract explicitly promotes selected structured stderr records.
- unknown events should be tolerated, logged, and preserved for drift analysis.
- terminal stream or protocol events are preferred over process exit when the provider emits them.
- non-zero exit, signal termination, missing terminal events, and malformed/no stream output must remain distinct evidence.
- wrapper-captured launch metadata must fill gaps the provider stream does not expose.
- richer protocol modes such as ACP, app-server, HTTP/SSE, Wire, or RPC need separate adapter logic from one-shot subprocess JSON parsers.

Provider-specific parser posture:

- Claude: parse `type`/`subtype`, treat `result` as terminal, correlate tools and denials by ID, and preserve richer operational events.
- Codex: parse top-level `type` plus nested `item.type`, treat `turn.completed`/`turn.failed` as terminal, collect final assistant text from completed agent messages, and augment metadata from wrapper context.
- Gemini: parse top-level `type`, concatenate `message` deltas, join tools by `tool_id`, and use exit/stderr when fatal errors bypass `result`.
- Goose: require `--quiet`, parse top-level `type` plus nested `message.content[].type`, join `toolRequest`/`toolResponse` by id, treat `complete` as loop teardown/usage, and taint runs with any `error`.
- Kimi: implement a JSON-RPC loop for Wire, parse `method` and `params.type`, answer blocking requests by policy, recurse into subagent events, and treat prompt responses as turn terminals.
- OpenCode: parse stdout NDJSON by top-level `type`, keep embedded `part.type` separate, treat `tool_use` as terminal tool state, sum usage/cost from `step_finish`, and parse selected stderr logs as lifecycle evidence.
- Qwen: parse `type`, `subtype`, and `event.type`; normalize both `system/init` and `system/session_start`; reconstruct partials when enabled; join tool IDs; and classify missing-result exits from exit code and stderr.
- Kilo: parse top-level `type`, group by `sessionID`, collect completed `text`, sum `step_finish`, classify any `error` as failure even if exit is `0`, and infer success from process exit plus absence of structured errors.
- Pi: parse top-level `type` plus nested assistant event type, treat `agent_end` as terminal, join tools by `toolCallId`, treat tool updates as accumulated snapshots, and inspect final stop reasons and retry records.

## Signal Detection Point of View

Planned Claudine signal detection should be layered.

The first layer is structured event detection. Use provider-native typed fields wherever they exist: Claude rate-limit/auth/permission events, Codex `turn.failed` and item records, Gemini `result.stats` and `tool_result.error`, Goose tool responses and `complete` usage, Kimi `StatusUpdate` and approval/request events, OpenCode `error`/`tool_use`/`step_finish`, Qwen `result.usage` and permission/task records, and future Kilo/Pi tool/usage/terminal records.

The second layer is provider-promoted auxiliary streams. OpenCode is the clearest current example: selected structured stderr logs are necessary lifecycle evidence. Other providers should keep stderr primarily diagnostic unless their contract explicitly promotes it.

The third layer is wrapper metadata. Claudine should record argv, cwd, prompt delivery mode, selected provider/model flags, sandbox and approval settings, profile/config isolation, relevant environment-derived auth source when safe, provider version, session/resume identifiers, and provider-specific controls such as Goose `--name`, Kimi Wire capabilities, OpenCode printed-log mode, Qwen budgets, and Pi/Kilo resource or permission flags.

The fourth layer is diagnostic fallback. Stderr, process exit code, process signal, malformed stdout, no structured output, and missing-terminal-event classification should refine failures without polluting the structured stdout parser.

The fifth layer is best-effort text classification. Use it only where typed fields are absent: quota, billing, no-funds, auth, model fallback, and provider-specific cap messages. These detections should be lower confidence than structured detections.

The practical conclusion is that Claudine should not wait for a perfect universal protocol. The providers already expose enough headless structure to support reliable wrapping, but only if Claudine respects their differences. Claude can drive rich signal detection directly from the stream. Codex needs stream parsing plus wrapper-side context capture. Gemini needs explicit acknowledgement that its public stream is a projection. Goose needs strict stdout purity and conservative terminal classification. Kimi needs a real JSON-RPC peer. OpenCode needs stdout NDJSON plus a classified stderr bridge. Qwen needs partial-message parsing plus disciplined exit/stderr handling. Future Kilo and Pi adapters should model their terminal and permission gaps explicitly rather than forcing them into another provider’s shape.

A dependable non-interactive contract is therefore not just “can the CLI run without a TTY?” It is the full operational envelope: deterministic launch, parser-safe stdout, diagnostic or classified stderr, recognizable terminal events when present, reliable exit handling when they are absent, and enough typed signal to decide what happened. Claudine’s stream parsers and signal detectors should make that envelope explicit for each provider and normalize only after the provider-specific evidence has been faithfully captured.
