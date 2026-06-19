---
schema: ""
schema_type: ""
data_format: ndjson
docs: https://opencode.ai/docs/cli/
last_updated: 2026-05-12
---

# OpenCode CLI Structured Output in Non-Interactive Sessions

## ⚠ The "DONE-only" NDJSON Rule (Read First)

OpenCode's `opencode run --format json` stream is **"DONE-only"** for tool
and subagent lifecycle:

- **No `tool_start`.** `tool_use` is emitted *only* after the tool reaches
  `completed` / `error`. There is no paired request-side event on the wire.
- **No native `task_started`.** The variant exists in the SDK but current
  OpenCode releases do not emit it.
- **No reliable session-complete event.** Token/cost data lives on
  `step_finish`; the closing turn often goes silent on stdout for minutes.
- **Often no `init` payload.** The run's primary provider/model identity
  is frequently absent from stdout entirely.

**Consumers cannot rely on stdout NDJSON alone** to observe an OpenCode
run. The structured stderr stream (`--print-logs --log-level INFO`) is
the second mandatory source for tool/subagent lifecycle, primary
provider/model identity, and progress signal during long synthesis
turns. Claudine treats this as a **Dual-Source Contract** and promotes
selected stderr log records to first-class `SemanticEvent`s; see
[`opencode-event-sources.md`](../../../../.claude/skills/claudine/opencode-event-sources.md)
for the full mapping table.

## Summary

As of 2026-04-06, OpenCode's non-interactive structured output is the `opencode run --format json` mode documented at <https://opencode.ai/docs/cli/> and implemented in [`packages/opencode/src/cli/cmd/run.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts). It writes one JSON object per line to stdout, so the wire format is newline-delimited JSON rather than a single JSON document.

The stream is useful, but lightly specified. OpenCode does publish formal schemas for the underlying session/message/event model through an official OpenAPI 3.1.1 document and generated TypeScript types, but it does **not** publish a formal schema for the exact `opencode run --format json` envelope itself. The current CLI implementation emits a narrow subset of session activity as custom event records: `tool_use`, `step_start`, `step_finish`, `text`, `reasoning`, and `error`. That is enough for automation, but it is notably less expressive than the raw bus event system available to plugins.

For automation, the practical split is:

- Use `step_finish` to collect token and cost data.
- Use `tool_use` to inspect completed or failed tool calls.
- Use plugin hooks or the SDK event stream, not `opencode run --format json`, when you need model identity, permission prompts, user questions, or raw session lifecycle events.

The biggest gaps today are the lack of a formal CLI stream schema, the lack of a dedicated session-complete event, and the fact that important human-in-the-loop signals are available to hooks but not surfaced in the CLI JSON stream.

Beyond the NDJSON stream on stdout, OpenCode also writes a parallel diagnostic stream to **stderr**. That stream has two layers: a structured logger (off by default, opted into with `--print-logs` and tuned via `--log-level`) that emits `LEVEL  YYYY-MM-DDTHH:MM:SS +Nms key=value ... message` lines, and a smaller "human chrome" channel written through `UI.println`/`UI.error` that emits ANSI-styled banners, share URLs, warnings, and human-readable error messages. The structured layer is the **richest source of internal lifecycle metadata that OpenCode currently exposes** — for example, parent-vs-subagent session distinction (`mode=primary` vs `mode=subagent`), provider/model identity per LLM call, permission evaluations, retry-classified errors, and HTTP request spans. None of this is documented officially; the schema below was reconstructed from source plus real captures against opencode 1.14.48.

## Schema

### Bottom line

There does not appear to be a provider-published formal schema for the **exact** output of `opencode run --format json`.

What OpenCode **does** publish officially is:

| Artifact | Schema language | URL | What it covers |
| --- | --- | --- | --- |
| Server API spec | OpenAPI 3.1.1 | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Official schema for server routes, raw bus events, messages, parts, permission requests, question requests, and related types |
| Generated SDK types | TypeScript | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Generated client types such as `Part`, `ToolPart`, `StepFinishPart`, `EventSessionError`, `PermissionRequest`, and `QuestionRequest` |
| Internal source-of-truth validators | TypeScript + Zod | <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts> | The runtime Zod definitions from which many message/part shapes are derived |

Those official schemas are extremely useful because the CLI JSON stream embeds official `part` and `error` objects inside its envelope. The problem is that the **envelope** is only described by implementation code in `run.ts`, not by OpenAPI, JSON Schema, or a published TypeScript type.

### Best available formal schema

The best official formal schema I found is OpenCode's OpenAPI 3.1.1 document:

- URL: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- Schema language: `open-api`
- Scope: underlying OpenCode session/message/event model, **not** the exact `opencode run --format json` line format

Important relevant components inside that spec include:

- `Part`
- `ToolPart`
- `StepStartPart`
- `StepFinishPart`
- `SnapshotPart`
- `Event.message.part.updated`
- `Event.session.error`
- `Event.permission.asked`
- `Event.question.asked`

### Informal schema for the CLI NDJSON stream

The following TypeScript-style shape is an **inference** from the current CLI implementation, not an official provider schema:

```ts
type RunJsonEvent =
  | {
      type: "tool_use"
      timestamp: number
      sessionID: string
      part: ToolPart
    }
  | {
      type: "step_start"
      timestamp: number
      sessionID: string
      part: StepStartPart
    }
  | {
      type: "step_finish"
      timestamp: number
      sessionID: string
      part: StepFinishPart
    }
  | {
      type: "text"
      timestamp: number
      sessionID: string
      part: TextPart
    }
  | {
      type: "reasoning"
      timestamp: number
      sessionID: string
      part: ReasoningPart
    }
  | {
      type: "error"
      timestamp: number
      sessionID: string
      error:
        | ProviderAuthError
        | UnknownError
        | MessageOutputLengthError
        | MessageAbortedError
        | StructuredOutputError
        | ContextOverflowError
        | ApiError
    }
```

Important implementation details from the current source:

- `timestamp` is produced with `Date.now()`, so it is an epoch-millisecond number.
- `tool_use` is emitted only when a tool part reaches `completed` or `error`.
- `reasoning` is emitted only when `--thinking` is enabled and the reasoning part is complete.
- There is no `session.complete` or equivalent terminal JSON event in the current source.

### Places checked for a formal schema

I looked in all of the following places before concluding that the exact CLI envelope is undocumented:

- Official CLI docs: <https://opencode.ai/docs/cli/>
- Official SDK docs: <https://opencode.ai/docs/sdk/>
- Official server docs: <https://opencode.ai/docs/server/>
- Official plugins docs: <https://opencode.ai/docs/plugins/>
- Official tools docs: <https://opencode.ai/docs/tools/>
- Official repo OpenAPI spec: <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json>
- Official generated TypeScript types: <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts>
- Internal runtime schema source: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts>
- Current CLI implementation: <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/cmd/run.ts>
- Community harness docs that consume this stream, including Cub and Harness

The result is clear: OpenCode formally specifies the **embedded data model**, but not the exact `run --format json` NDJSON wrapper.

## Documentation

### Official documentation

| Topic | URL | Notes |
| --- | --- | --- |
| CLI non-interactive mode | <https://opencode.ai/docs/cli/> | Documents `opencode run` and the `--format` flag |
| SDK structured output | <https://opencode.ai/docs/sdk/> | Documents `format: { type: "json_schema", schema, retryCount }` for `session.prompt()` |
| Plugin events and hooks | <https://opencode.ai/docs/plugins/> | Documents raw event names such as `session.error`, `permission.asked`, and `question.asked` plus `tool.execute.before/after` |
| Built-in tools | <https://opencode.ai/docs/tools/> | Documents most public built-ins and their permission behavior |
| Server routes | <https://opencode.ai/docs/server/> | Useful because the published SDK/server schemas are closer to the raw internal model than the CLI docs are |
| OpenAPI spec | <https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/sdk/openapi.json> | Best formal source for event/message/part shapes |
| Generated TypeScript types | <https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/v2/gen/types.gen.ts> | Best readable source for payload shapes without reading raw OpenAPI |

### Third-party documentation and articles

Long-form blog coverage of OpenCode's JSON stream is still sparse. The most useful third-party writeups I found were documentation from tools that integrate OpenCode as a subprocess:

| Source | URL | Why it matters |
| --- | --- | --- |
| Cub harness docs | <https://docs.cub.tools/docs/guide/harnesses/opencode/> | Explains how another tool consumes `opencode run --format json`, especially `step_finish` for token accounting |
| Harness docs | <https://www.harness.lol/docs> | Describes wrapping OpenCode's native stream into a provider-agnostic NDJSON format |
| Cupcake OpenCode reference | <https://cupcake.eqtylab.io/reference/harnesses/opencode/> | Documents how OpenCode plugin hooks are mapped into a policy harness |

These are valuable integration references, but they should be treated as secondary sources. They occasionally simplify or overgeneralize current OpenCode behavior.

## CLI

### Available output formats for non-interactive `run`

For `opencode run`, the current CLI enumerates exactly two output formats:

| Format | Meaning |
| --- | --- |
| `default` | Human-oriented formatted output |
| `json` | Machine-oriented NDJSON stream of custom event objects |

The CLI syntax is:

```bash
opencode run --format default "your prompt"
opencode run --format json "your prompt"
```

The current source defines that flag in `packages/opencode/src/cli/cmd/run.ts` with:

- `choices: ["default", "json"]`
- `default: "default"`

### What `json` actually means

Despite the docs calling this "raw JSON events", the implementation is not the full raw bus event stream. The CLI subscribes to the internal event bus, selects a handful of signals, then re-emits them as its own NDJSON records.

Current emitted event types:

- `tool_use`
- `step_start`
- `step_finish`
- `text`
- `reasoning`
- `error`

### Side effects and behavior changes when `--format json` is used

| Behavior | Effect |
| --- | --- |
| Stdout becomes NDJSON | The primary result channel is machine-readable line output, not prose |
| Human status output moves to stderr | Share URLs, warnings, and formatted UI messages still go to stderr because `UI.println()` writes there |
| No dedicated completion event | Callers must infer completion from process exit and the last received events |
| Tool start visibility is incomplete | The current formatter does not emit a generic "tool started" event for most tools |
| Reasoning is opt-in | `reasoning` records only appear when `--thinking` is supplied |
| Permission prompts are not exposed as JSON | `permission.asked` is handled internally and not emitted as structured stdout |

### Related but different `--format` flags

OpenCode also uses `--format json` on some non-streaming commands such as session/model listing, but those return regular JSON documents or tables rather than the NDJSON event stream used by `opencode run`.

## Logging (STDERR Stream)

OpenCode emits diagnostic output to **stderr** in non-interactive `run` mode. This is distinct from the NDJSON event stream on stdout described above and is the only place where many internal lifecycle signals are exposed — provider/model selection per call, session creation with `parentID` lineage, permission evaluations, retry-classified API errors, and HTTP span timings. There is no official documentation for this stream; the schema and examples here were reconstructed from source against opencode 1.14.48.

### Three distinct stderr producers

OpenCode `run` writes to stderr from exactly three sources, all of which can interleave in a single capture:

| Producer | Source | Activation | Format |
| --- | --- | --- | --- |
| Structured logger | [`packages/core/src/util/log.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/core/src/util/log.ts) | `--print-logs` flag (off by default — writes go to a log file instead) | Plain-text `LEVEL  TIMESTAMP +Nms key=val ... message\n` |
| UI helpers | [`packages/opencode/src/cli/ui.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/cli/ui.ts) | Always on; gated per call by `--format` | ANSI-colored prose with no timestamp; bare bytes via `process.stderr.write` |
| Direct stderr writes | [`packages/opencode/src/index.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/index.ts) (~lines 127–155, 232–247) | First-run DB migration; top-level fatal fallback | Plain text; on TTY uses cursor and progress glyphs (`\x1b[?25l`, `■`); on pipe uses `sqlite-migration:N\n` lines |

The structured logger is always created — when `--print-logs` is omitted, the same stream is written to a file under OpenCode's data dir instead of being redirected to stderr.

### Activating the structured logger

```bash
opencode run --format json --print-logs --log-level INFO  "your prompt"   # default ~280 lines for a small session
opencode run --format json --print-logs --log-level DEBUG "your prompt"   # adds verbose config-loading paths
opencode run --format json --print-logs --log-level WARN  "your prompt"   # only WARN/ERROR (typically duplicate-skill notes)
opencode run --format json --print-logs --log-level ERROR "your prompt"   # only ERROR (silent on success)
```

Source wiring (`packages/opencode/src/index.ts` ~lines 75–104):

```ts
.option("print-logs", { describe: "print logs to stderr", type: "boolean" })
.option("log-level",  { type: "string", choices: ["DEBUG", "INFO", "WARN", "ERROR"] })
.middleware(async (opts) => {
  await Log.init({
    print: process.argv.includes("--print-logs"),
    dev:   Installation.isLocal(),
    level: opts.logLevel ?? (Installation.isLocal() ? "DEBUG" : "INFO"),
  })
  Log.Default.info("opencode", { version, args, process_role, run_id })
})
```

Default level is `INFO` for installed builds, `DEBUG` for `dev` checkouts. The first line written to the stream is always the `opencode` boot banner with `version`, `args`, `process_role`, and `run_id` — useful for parsers as a deterministic stream anchor.

### Log line format

Source: `build()` in `packages/core/src/util/log.ts` (lines ~113–139).

The wire format is:

```
LEVEL  YYYY-MM-DDTHH:MM:SS +Nms key=value [key=value ...] [free-form message]\n
```

Notable details:

- **Level token** is one of `DEBUG`, `INFO `, `WARN `, `ERROR` — every level is padded to **five characters** by trailing spaces so the prefix is fixed width. Followed by a single space (so the `LEVEL ` prefix is always six characters before the timestamp).
- **Timestamp** is `new Date().toISOString().split(".")[0]` — second precision only, **no milliseconds, no trailing `Z`** (e.g. `2026-05-12T20:00:12`).
- **Delta** is `+Nms` since the previous log line written by this process (not since the timestamped second).
- **Tags** are space-joined `key=value` pairs from logger tags plus per-call extras. Object values are JSON-stringified inline (no internal newlines, but values may contain spaces if the inner JSON contains a space-bearing string). `Error` values are formatted via `formatError()` which concatenates `message + " Caused by: " + cause.message`.
- **Free-form message** follows the tags and may be any prose (e.g. `created`, `loop`, `exiting loop`, `Sent HTTP response`, `stream`, `stream error`). It is always last on the line.
- A single `\n` terminates the line; values **never** contain bare newlines because object serialization inlines and string messages are trusted.

Conservative parse regex (matches all four levels, captures level/timestamp/delta/rest):

```
^(DEBUG|INFO |WARN |ERROR) (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) \+(\d+)ms (.*)\n
```

### Tags catalog

The `service` tag is the primary discriminator. Observed services in a single non-interactive run against opencode 1.14.48:

| Service | What it logs |
| --- | --- |
| `default` | Boot banner (`opencode`), `creating instance`, `bootstrapping`, HTTP request spans (`http.method`, `http.url`, `http.status`, `logSpan.http.span.N`), instance disposal |
| `file` | File subsystem init |
| `project` | `directory=... fromDirectory` |
| `db` | `opening database`, `applying migrations` (with `count` and `mode`) |
| `config` | `loading` per config path; `DEBUG` adds `loading config from <path>` for `.opencode/*.json[c]` |
| `plugin` | `loading internal plugin` (with `name=<obfuscated>`); `loading plugin` for external plugins (`path=file://...`) |
| `bus` | `subscribing`/`publishing`/`unsubscribing` for every event type (very noisy — see Gotchas below) |
| `lsp` | `all LSPs are disabled` or per-LSP startup |
| `format` | `all formatters are disabled` or per-formatter startup |
| `file.watcher` | `init` plus `platform=<os>` and `backend=<fs-events|inotify|...>` |
| `session` | **Session-created records** with full metadata (see use cases) |
| `session.prompt` | Per-step loop markers (`step=N loop`, `exiting loop`), `status=started/completed resolveTools` |
| `session.processor` | Per-message processing (`messageID=...`) |
| `provider` | Per-provider availability (`providerID=X found`), provider init spans |
| `tool.registry` | Per-tool registration (`status=started/completed duration=N <toolName>`) — see warning about meaning below |
| `shell-tool` | `shell=/bin/zsh shell tool using shell` |
| `skill` | `init` plus `WARN duplicate skill name` for every skill collision |
| `llm` | **Per-LLM-call provider/model identification** (see use cases) |
| `permission` | **Permission evaluation results** (see use cases) |
| `server` | Internal API server lifecycle and request errors (with stack traces for fatals) |
| `share-next` | Share subscriber failures (carries Effect `Cause` tree) |
| `image` | (Source-only — image attachment processing) |
| `acp.session` | (Source-only — ACP integration) |

Other recurring tag keys: `session.id`, `messageID`, `providerID`, `modelID`, `agent`, `mode`, `small`, `step`, `duration`, `cause`, `error`, `logSpan.<name>.span.<N>`.

### What `tool.registry status=started/completed` does NOT mean

This is the easiest log line to misread. `service=tool.registry status=started <name>` is logged **only** for tool **registration** at session startup, not for tool **invocation**. The duration on the completed line is the time to register the tool definition (typically `duration=0` or `duration=1`). Actual tool calls do not produce a `service=tool.registry` line. The only stderr signal for an actual tool invocation in INFO/DEBUG modes is the `service=session.prompt` loop transitions and any `service=permission ... evaluated` lines — the real per-call payload lives on stdout as `tool_use` events.

### Default-format vs JSON-format stderr behavior

| Behavior | `--format default` | `--format json` |
| --- | --- | --- |
| `> agent · modelID` start banner | Yes (once per session) | Suppressed |
| Tool call inline lines (icon + title) | Yes | Suppressed (moves to stdout as `tool_use`) |
| Completed text output | Stderr if stdout is a TTY; stdout otherwise | Suppressed (moves to stdout as `text` parts) |
| Share URL line `~ <url>` | Yes | Yes (still printed via `UI.println`) |
| Permission auto-reject warning `! permission requested: ...; auto-rejecting` | Yes | Yes |
| Agent-not-found fallback `! agent "X" not found. Falling back to default agent` | Yes | Yes |
| `UI.error("...")` for fatals (e.g. `Error: Model not found: ...`) | Yes | **Yes** — `UI.error` fires after `emit("error", ...)` writes to stdout (`emit` returns false when `args.format !== "json"` was the original intent, but the model-not-found path still hits `UI.error` via the top-level catch in `index.ts`) |
| Structured logger output | Same in both formats (gated by `--print-logs`) | Same in both formats |

### Stripping the structured-logger frame

The structured logger uses bare ASCII; the UI helpers use ANSI SGR escapes (`\x1b[91m\x1b[1m` danger-bold, `\x1b[94m\x1b[1m` info-bold, `\x1b[93m\x1b[1m` warning-bold, `\x1b[90m` dim, `\x1b[0m` reset). A captured stream can be split into "structured" and "UI" by matching the level token at column 0 — any line that does not start with one of `DEBUG`, `INFO `, `WARN `, `ERROR` followed by a space is a UI helper or direct write, and should typically be SGR-stripped before further parsing.

### Informal schema for the structured-log stream

```ts
type OpenCodeStderrLogLine = {
  level: "DEBUG" | "INFO" | "WARN" | "ERROR"
  timestamp: string          // "YYYY-MM-DDTHH:MM:SS", no millis, no Z
  delta_ms: number           // ms since previous log line written by this process
  service?: string           // e.g. "default" | "session" | "session.prompt" | "llm" | "tool.registry" | "permission" | "server" | "share-next" | "config" | "db" | ...
  tags: Record<string, string | number | boolean | object>
                             // free-form key=value pairs; object values are inline-JSON
  message: string            // free-form trailing prose, e.g. "created" | "loop" | "exiting loop" | "stream" | "stream error" | "Sent HTTP response"
}
```

Important tags carried by specific services:

- `service=default` boot line: `version`, `args` (JSON array), `process_role`, `run_id` (UUID), trailing message `opencode`.
- `service=session` created line: `id` (session ID), `slug`, `version`, `projectID`, `directory`, `path`, `parentID?` (only for subagent child sessions), `title`, `permission` (inline JSON array), `time` (inline JSON object), trailing message `created`.
- `service=session.prompt`: `session.id`, `step`, `logSpan.http.span.N`, trailing message `loop` / `exiting loop` / `resolveTools` / status pairs.
- `service=llm`: `providerID`, `modelID`, `session.id`, `small` (boolean), `agent`, `mode` (`primary` | `subagent`), trailing message `stream`. On failure: trailing message `stream error` and an additional `error=<JSON>` tag with the full AI SDK error object.
- `service=permission`: `permission` (type, e.g. `task`, `read`, `write`), `pattern`, `action` (inline JSON), trailing message `evaluated`.
- `service=server` error: `error=<ClassName>`, `cause=<ClassName>: <message>\n    at <stack>...`, trailing message `failed`.
- `service=share-next` error: `type=<bus event type>`, `cause=<Effect Cause JSON>`, trailing message `share subscriber failed`.
- `service=default` HTTP request: `http.method`, `http.url`, `http.status`, `logSpan.http.span.<N>` (duration in ms), trailing message `Sent HTTP response`.

### Real-capture examples

These lines are from a captured `--format json --print-logs --log-level INFO` run against opencode 1.14.48. Long inline-JSON fields are abbreviated with `…`.

Boot banner:

```
INFO  2026-05-12T20:00:11 +97ms service=default version=1.14.48 args=["run","--format","json","--print-logs","--log-level","INFO","say hello in one word"] process_role=main run_id=48277674-19e5-40b6-b2b5-efa7577f08ea opencode
```

Session created (primary):

```
INFO  2026-05-12T20:00:12 +20ms service=session id=ses_1e23972b3ffe8QLhzuFpWS5bzd slug=happy-panda version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test title=New session - 2026-05-12T20:00:12.108Z permission=[{"permission":"question","pattern":"*","action":"deny"},{"permission":"plan_enter","pattern":"*","action":"deny"},{"permission":"plan_exit","pattern":"*","action":"deny"}] time={"created":1778616012108,"updated":1778616012108} created
```

LLM call (primary, title generation):

```
INFO  2026-05-12T20:00:12 +4ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd small=true agent=title mode=primary stream
```

LLM call (primary, real work):

```
INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd small=false agent=build mode=primary stream
```

Step loop:

```
INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd step=0 logSpan.http.span.4=55ms loop
INFO  2026-05-12T20:00:19 +0ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd step=1 logSpan.http.span.4=7436ms loop
INFO  2026-05-12T20:00:19 +1ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd logSpan.http.span.4=7437ms exiting loop
```

Provider rate-limit error from the Kimi backend (note the trailing `stream error` message and the massive inline `error=` payload that contains the full request body **and** the parsed retry classification):

```
ERROR 2026-05-12T20:02:20 +1967ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e237a304ffeqwr10bXJSRYGHJ small=false agent=build mode=primary error={"error":{"name":"AI_APICallError","url":"https://api.kimi.com/coding/v1/messages","requestBodyValues":{…},"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"},\"type\":\"error\"}","isRetryable":true,"data":{"type":"error","error":{"type":"rate_limit_error","message":"The engine is currently overloaded, please try again later"}}}} stream error
```

Bad-model fatal (server + share-next both emit, then JSON stdout emits `{"type":"error",...}`):

```
ERROR 2026-05-12T20:05:54 +15ms service=server error=ProviderModelNotFoundError cause=ProviderModelNotFoundError: ProviderModelNotFoundError
    at <anonymous> (/$bunfs/root/chunk-71tjptxz.js:519:65509)
    at Provider.getModel (/$bunfs/root/chunk-jxv846wz.js:1900:1166)
    ...stack frames... failed
INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/ses_1e2343b5cffeGOb3bcdTjvh1wZ/message http.status=500 logSpan.http.span.4=99ms Sent HTTP response
ERROR 2026-05-12T20:05:54 +0ms service=share-next type=message.updated cause={"_id":"Cause","failures":[{"_tag":"Die","defect":{"name":"ProviderModelNotFoundError","data":{"providerID":"nonexistent","modelID":"bogus-model","suggestions":[]}}}]} share subscriber failed
```

Bad-agent fallback (UI helper, **not** a structured log line — ANSI bytes shown verbatim):

```
^[[93m^[[1m! ^[[0m agent "nonexistent-agent" not found. Falling back to default agent
```

### Things that are NOT on stderr (even with `--log-level DEBUG`)

- **Token totals or per-message token usage** — token counts only appear in the JSON stdout `step_finish` part. No `service=llm` log line includes `tokens.input` / `tokens.output` / `cost` fields.
- **Live tool invocation arguments and results** — those are only in stdout `tool_use` events. The stderr stream tells you when the parent session looped (`step=N loop`) and when a child session was created, but not which arguments the model passed to a tool.
- **Rate-limit headroom or quota status** — there is no `service=quota` or `service=billing` stream. Cap-related signals only appear as provider error payloads inside the `error=` tag on `service=llm` ERROR lines, with provider-specific text.
- **"Approaching plan cap" warnings** — no first-class signal, same as on stdout.
- **Permission requests in interactive sense** — `service=permission ... evaluated` only logs decisions, not pending asks; in non-interactive `run` the engine pre-denies `question` / `plan_enter` / `plan_exit`, and other asks become UI-helper warnings (`! permission requested: ...; auto-rejecting`).

### Gotchas

- **Bus chatter dominates INFO output.** In a non-trivial session, the majority of INFO lines are `service=bus type=message.part.delta publishing` and `service=bus type=message.part.updated publishing`. A 207-line stderr capture for a single tool call had ~150 of those. Filter `service=bus` aggressively unless you are debugging the bus itself.
- **The `error=` payload can be tens of kilobytes per line.** AI SDK `AI_APICallError` objects include `requestBodyValues` with the full system prompt and message history. Parsers must allow very long single lines (no synthetic line wrap) and budget memory accordingly.
- **Exit code is `0` even on fatal-looking errors.** A `ProviderModelNotFoundError` produces stderr `ERROR` lines, a stdout `{"type":"error",...}` event, and exits `0`. Do not use exit code as a success signal — inspect the JSON stream and/or look for ERROR-level lines.
- **DEBUG adds very little above INFO in installed builds.** In 1.14.48, the only INFO→DEBUG delta observed was two extra `service=config loading config from <path>` lines. Most of the verbose detail OpenCode developers see is gated on `Installation.isLocal()` (dev checkouts), not on the `--log-level` flag.
- **Timestamps have second precision only.** Sub-second ordering must use the `+Nms` delta.
- **First-run DB migration prints non-log lines.** A fresh install emits `sqlite-migration:N\n` lines (or a TTY progress bar) before any structured log appears. Treat these as a one-shot pre-amble.

### Logging Use Cases

These are the answers to the practical questions a wrapper needs to ask of the stderr stream, with the exact log lines that surface each signal.

#### 1. Subagent created and returned

Triggered when the parent agent calls the `task` tool. Sequence observed on stderr (all `INFO`):

```
service=permission permission=task pattern=<subagent-type> action={...} evaluated
service=session id=ses_<CHILD> slug=<slug> version=<ver> projectID=<id> directory=<cwd> path=<rel> parentID=ses_<PARENT> title=<title> permission=[...] time={...} created
service=session.prompt session.id=ses_<CHILD> step=0 logSpan.http.span.4=<Nms> loop
service=session.prompt status=started resolveTools
service=tool.registry status=started <tool> ... (per registered child tool)
service=session.prompt status=completed duration=<N> resolveTools
service=session.processor session.id=ses_<CHILD> messageID=msg_<...> process
service=llm providerID=<X> modelID=<Y> session.id=ses_<CHILD> small=false agent=<subagent-name> mode=subagent stream
service=session.prompt session.id=ses_<CHILD> step=1 logSpan.http.span.4=<Nms> loop
service=session.prompt session.id=ses_<CHILD> logSpan.http.span.4=<Nms> exiting loop
service=session.prompt session.id=ses_<PARENT> step=N logSpan.http.span.4=<Nms> loop   ← parent resumes
```

The three definitive signals that distinguish a subagent from the primary session:

| Signal | Where | Discriminator |
| --- | --- | --- |
| Child session creation | `service=session ... created` | Has `parentID=ses_<PARENT>`; primary sessions never carry `parentID` |
| LLM call mode | `service=llm ... mode=subagent stream` | Primary sessions emit `mode=primary` |
| Agent identity | `service=llm ... agent=<name>` | `agent=build` is the primary work agent; `agent=title` is the auxiliary title generator; anything else (e.g. `agent=general`) is a subagent |

There is no explicit `subagent stopped` log line — completion is signaled by `service=session.prompt session.id=ses_<CHILD> ... exiting loop` followed immediately by the parent's next `loop` line. The corresponding JSON `tool_use` event for the `task` tool on stdout includes `metadata.sessionId` matching the child's session ID, which lets you cross-reference the two streams.

Real captured example (Claude Haiku 4.5, `general` subagent):

```
INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated
INFO  2026-05-12T20:05:26 +1ms   service=session id=ses_1e234a70dffeOCARJZRL9dhpHT slug=lucky-orchid version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test parentID=ses_1e234af48ffeViMPs5pMk6UhYk title=Count letters in 'banana' (@general subagent) permission=[…] time={…} created
INFO  2026-05-12T20:05:26 +1ms   service=llm providerID=opencode modelID=claude-haiku-4-5 session.id=ses_1e234a70dffeOCARJZRL9dhpHT small=false agent=general mode=subagent stream
INFO  2026-05-12T20:05:27 +1ms   service=session.prompt session.id=ses_1e234a70dffeOCARJZRL9dhpHT logSpan.http.span.4=3519ms exiting loop
INFO  2026-05-12T20:05:27 +0ms   service=session.prompt session.id=ses_1e234af48ffeViMPs5pMk6UhYk step=1 logSpan.http.span.4=3590ms loop
```

#### 2. Tool calling

The structured stderr stream does **not** log per-call tool invocations at INFO or DEBUG. `service=tool.registry` lines log only the tool **registration phase** at session bootstrap (with `duration=0`/`duration=1`), not the calls themselves. To observe actual tool calls you must either:

- Read the **stdout NDJSON** `tool_use` events (the only place `tool`, `callID`, `input`, `output`, `metadata`, and `state.status` are exposed). This is the authoritative tool-call channel.
- Or use the **plugin/hook layer** described earlier in this document — `tool.execute.before/after` plus raw `event` subscriptions are much richer than anything that lands in stderr.

What stderr **does** give you adjacent to tool calling:

| Signal | Meaning |
| --- | --- |
| `service=session.prompt ... step=N loop` | The parent went into another LLM step — usually because a tool result was added to the conversation |
| `service=permission permission=<type> pattern=<arg> action={...} evaluated` | A permission for a tool argument was evaluated (allow/deny/auto-allow) |
| UI helper line `! permission requested: <type> (<patterns>); auto-rejecting` | Non-interactive auto-reject for a tool that asked for permission (e.g. `bash`, `write`, `task`, etc.) |
| `service=session.processor session.id=<X> messageID=<Y> process` | A new assistant message is being processed — usually follows a tool result |

So in practice: stderr tells you that a step boundary happened and that a permission was decided; stdout tells you *which tool* and *with what arguments*.

#### 3. Provider rate-cap reached and approaching

**Cap reached.** The stderr signal is an `ERROR` line on `service=llm` whose `error=` tag contains the AI SDK's classified error. The error JSON is the same structure that downstream emits as a `session.error` bus event and as the JSON-stdout `{"type":"error", ...}` envelope, but stderr exposes the **full raw provider response** including the original retry classification.

Concrete pattern for an Anthropic-style rate cap:

```
ERROR <ts> +<N>ms service=llm providerID=<P> modelID=<M> session.id=<S> small=<bool> agent=<A> mode=<primary|subagent> error={"error":{"name":"AI_APICallError","url":"<endpoint>","requestBodyValues":{…},"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"<vendor message>\"},…}","isRetryable":true|false,"data":{"type":"error","error":{"type":"<rate_limit_error|insufficient_quota|usage_not_included>","message":"<vendor message>"}}}} stream error
```

Fields a wrapper should inspect from inside the `error=` payload:

- `error.name` — `AI_APICallError`, `ProviderAuthError`, or other AI SDK class name
- `error.isRetryable` — boolean; `true` for rate-limit-style errors, useful to distinguish "wait and retry" from "stop"
- `error.data.error.type` — `rate_limit_error` (engine overloaded / per-window rate cap), `insufficient_quota` (OpenAI-style billing cap), `usage_not_included` (subscription/plan cap)
- `error.data.error.message` — provider-supplied human text; OpenCode does not normalize this further

Observed at 2026-05-12 against Kimi:

```
ERROR 2026-05-12T20:02:20 +1967ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e237a304ffeqwr10bXJSRYGHJ small=false agent=build mode=primary error={…"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"},\"type\":\"error\"}","isRetryable":true,…} stream error
```

OpenCode retries internally, so on a sustained cap you typically see this line **repeated** (the capture above had 6 retries before giving up). The retry classifier lives in [`packages/opencode/src/session/retry.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/retry.ts) and emits no separate stderr text — counting `ERROR ... service=llm ... stream error` lines is the cheapest way for a wrapper to detect "we are being rate-limited" before the request finally fails.

**Cap approaching.** OpenCode does **not** surface a "cap approaching" signal anywhere — neither as a stderr log line nor as a JSON event. Provider-side soft warnings (e.g. Anthropic's `X-RateLimit-Remaining` headers, OpenAI usage warnings) are not propagated. The only way a wrapper can preview an upcoming cap is to track its own `step_finish.cost` totals from the JSON stream and apply a local threshold.

#### 4. Token usage

Not on stderr. The `service=llm` lines log model identity and stream success/failure, but never carry token counts. To get tokens, parse the JSON stdout `step_finish.part.tokens` (`input`, `output`, `reasoning`, `cache.read`, `cache.write`) and `step_finish.part.cost`. Each step emits one such record; summing them is the canonical way to get per-session totals because OpenCode does not emit a terminal session-summary event.

Adjacent stderr signals that help correlate token use to wall-clock time:

- `service=session.prompt ... step=N loop` — marks the start of a step
- `service=default http.method=POST http.url=/session/<id>/message http.status=200 logSpan.http.span.4=<Nms> Sent HTTP response` — marks the end of the prompt's HTTP request

#### 5. Cleanest way to extract provider and model

The single best line for provider/model identification is `service=llm`:

```
INFO  <ts> +<N>ms service=llm providerID=<P> modelID=<M> session.id=<S> small=<bool> agent=<A> mode=<primary|subagent> stream
```

Recommended extractor for Claudine:

- Match each stderr line against a regex that captures the structured prefix and the trailing `service=llm ... stream` shape.
- Project the four fields the wrapper needs: `providerID`, `modelID`, `agent`, `mode`.
- Treat the **first** `service=llm ... small=false mode=primary stream` line as the authoritative primary provider/model for the session. Earlier `small=true agent=title` lines reflect the auxiliary title-generation call (usually a smaller, faster model — e.g. `gpt-5-nano` even when the main agent runs on `claude-haiku-4-5`) and should be reported separately if reported at all.
- For subagents, capture every `mode=subagent` line and pair it with the most recent `service=session ... parentID=... created` line to attribute the call to its child session ID.

Suggested Rust extractor (illustrative, not authoritative):

```rust
// Per-line regex; fields are unordered in the source but stable in practice.
// Allowing flexible ordering avoids breakage on logger refactors.
const RE: &str = r"^(?P<level>DEBUG|INFO |WARN |ERROR) (?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) \+(?P<delta>\d+)ms (?P<body>.*)$";

fn extract_llm_call(body: &str) -> Option<LlmCall> {
    if !body.starts_with("service=llm ") { return None; }
    if !body.ends_with(" stream") && !body.ends_with(" stream error") { return None; }
    let mut out = LlmCall::default();
    for kv in body.split(' ') {
        let (k, v) = kv.split_once('=')?;
        match k {
            "providerID" => out.provider = v.to_string(),
            "modelID"    => out.model    = v.to_string(),
            "agent"      => out.agent    = v.to_string(),
            "mode"       => out.mode     = v.to_string(),  // "primary" | "subagent"
            "small"      => out.small    = v == "true",
            "session.id" => out.session  = v.to_string(),
            _ => {}
        }
    }
    Some(out)
}
```

Why not parse the JSON stdout for this? The `step_finish` part on stdout includes `cost` and `tokens` but **no `providerID` / `modelID` field** — those identifiers are surfaced only as part of the `task` tool's child-session `metadata.model` (for subagents) and never at the top of the parent stream. The stderr `service=llm` line is OpenCode's only reliable top-of-stream announcement of "this call is being routed to provider X model Y".

#### 6. Other useful signals for a wrapper

| Need | Stderr signal |
| --- | --- |
| Session ID assigned | `service=session id=ses_<X> ... created` (first match wins for primary session) |
| Project / repo OpenCode thinks it's in | `service=project directory=<path> fromDirectory` |
| Permissions OpenCode actually enforces in this session | `permission=[...]` inline JSON inside the session-created line — non-interactive sessions show `question`/`plan_enter`/`plan_exit` denied by default; subagents add `task` deny and several `repo_*` denies |
| HTTP timings end-to-end | `service=default http.method=... http.url=... http.status=... logSpan.http.span.N=<Nms> Sent HTTP response` (one per request, including `/session`, `/config`, `/event`, `/session/<id>/message`) |
| Configuration files loaded | `service=config path=<path> loading` — useful to detect that the wrapper's intended config override was picked up |
| Plugins loaded | `service=plugin name=<obfuscated> loading internal plugin` (built-ins) and `service=plugin path=file://<...> loading plugin` (external — readable path) |

## Tools

### Built-in tools currently available out of the box

The official tools page documents these public built-ins:

- `bash`
- `edit`
- `write`
- `read`
- `grep`
- `glob`
- `list`
- `lsp` (experimental)
- `apply_patch`
- `skill`
- `todowrite`
- `webfetch`
- `websearch`
- `question`

The current source-level registry also includes:

- `task`
- `codesearch`
- `batch` (experimental)
- `plan_exit` (experimental CLI plan mode)
- `invalid` (internal fallback, not a user-facing tool)

This is a good example of why the source is the stronger reference than the public tools page for integration work.

### What the CLI JSON stream exposes for tool calls

Current `opencode run --format json` behavior is asymmetric:

| Phase | CLI JSON visibility | Hook visibility |
| --- | --- | --- |
| Before a tool runs | No general structured stdout event today | `tool.execute.before` and `event` hook both see it |
| While a tool is running | Usually no structured stdout event; `task` gets special pretty-print handling only in default mode | `tool.execute.before`, internal part updates, and plugin `event` hook are richer |
| After success | Yes, as `tool_use` with `part.type = "tool"` and `part.state.status = "completed"` | `tool.execute.after` and `event` hook both see it |
| After failure | Yes, as `tool_use` with `part.state.status = "error"` | `tool.execute.after` may still observe the post-call state; raw `event` hook is the safer universal source |

The important payload sits inside `part.state`:

- `input`: the tool arguments the model used
- `title`: the tool title shown by OpenCode
- `output`: textual tool result
- `metadata`: tool-specific structured metadata
- `time`: start/end timestamps
- `attachments`: optional attached files or images

### Tool metadata examples

#### `bash`

`tool_use.part.state.metadata` includes:

- `output`: preview of command output
- `exit`: exit code
- `description`: the tool description shown to the agent

#### `read`

`tool_use.part.state.metadata` includes:

- `preview`: preview of the loaded content
- `truncated`: whether the read was truncated
- `loaded`: referenced system-reminder file paths that were injected

#### `write`

`tool_use.part.state.metadata` includes:

- `diagnostics`: LSP diagnostics after the write
- `filepath`: absolute file path
- `exists`: whether the file already existed

#### `edit`

`tool_use.part.state.metadata` includes:

- `diagnostics`
- `diff`
- `filediff` with before/after content and addition/deletion counts

#### `task`

`tool_use.part.state.metadata` includes:

- `sessionId`: child session ID for the subagent run
- `model`: `{ providerID, modelID }` used by the child agent

#### `step_finish`

Strictly speaking this is not a tool call, but it is the most important structured "after" record for accounting. It includes:

- `cost`
- `tokens.input`
- `tokens.output`
- `tokens.reasoning`
- `tokens.cache.read`
- `tokens.cache.write`
- `reason`

### Example NDJSON shapes

These are representative examples reconstructed from the current source and official part schemas.

#### Completed tool call

```json
{
  "type": "tool_use",
  "timestamp": 1775490000000,
  "sessionID": "ses_123",
  "part": {
    "type": "tool",
    "tool": "read",
    "callID": "call_1",
    "state": {
      "status": "completed",
      "input": {
        "filePath": "/repo/src/lib.rs",
        "offset": 1,
        "limit": 200
      },
      "title": "src/lib.rs",
      "output": "<path>/repo/src/lib.rs</path> ...",
      "metadata": {
        "preview": "pub fn example() { ... }",
        "truncated": false,
        "loaded": []
      },
      "time": {
        "start": 1775490000100,
        "end": 1775490000325
      }
    }
  }
}
```

#### Step accounting event

```json
{
  "type": "step_finish",
  "timestamp": 1775490001000,
  "sessionID": "ses_123",
  "part": {
    "type": "step-finish",
    "reason": "tool-calls",
    "cost": 0.00123,
    "tokens": {
      "input": 1024,
      "output": 220,
      "reasoning": 0,
      "cache": {
        "read": 0,
        "write": 0
      }
    }
  }
}
```

#### Failed tool call caused by permissions

```json
{
  "type": "tool_use",
  "timestamp": 1775490002000,
  "sessionID": "ses_123",
  "part": {
    "type": "tool",
    "tool": "write",
    "state": {
      "status": "error",
      "input": {
        "filePath": "/repo/.env",
        "content": "SECRET=..."
      },
      "error": "Tool execution failed: The user has specified a rule which prevents you from using this specific tool call."
    }
  }
}
```

## Use Cases

### Plan Cap Approaching

| Question | Answer |
| --- | --- |
| CLI JSON event type | No dedicated event type |
| Best fallback | Possibly `error` or raw `session.status` retry messages, but only if the provider surfaces a useful string |
| How to distinguish | Only by provider-specific message text; there is no normalized "cap approaching" field |
| Remaining budget extractable? | No structured field today |
| Reset window extractable? | No structured field today |
| Hook exposure | No dedicated hook event either; the best hook fallback is raw `event` subscriptions for `session.error` or `session.status` |
| Stream vs hook parity | Hooks are slightly richer because they preserve the raw event type and payload; the CLI JSON formatter does not surface retry/status events at all |

Assessment: OpenCode does not currently formalize "plan cap approaching" as a first-class signal. If a provider emits a warning in plain text, callers must pattern-match on vendor-specific strings.

### Plan Capped

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `error` |
| Best raw event | `session.error` with `APIError` or `ProviderAuthError`, depending provider behavior |
| How to distinguish | Inspect `error.name`, then inspect `error.data.message` and, when present, `error.data.responseBody` |
| Remaining budget extractable? | Not in a normalized field |
| Reset window extractable? | Not in a normalized field |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Similar payload, but the hook sees the raw `session.error` object instead of the CLI envelope |

Provider-specific notes:

- OpenAI-style `insufficient_quota` is normalized by OpenCode to an `APIError` message that says quota was exceeded and billing should be checked.
- OpenAI-style `usage_not_included` is normalized to an upgrade message for Codex access.

That means "capped" is detectable only as a generic provider error, not as a standardized plan-window object.

### No Funds

| Question | Answer |
| --- | --- |
| CLI JSON event type | `error` |
| Best raw event | `session.error` with `APIError` |
| How to distinguish | Check `error.name === "APIError"` and inspect the provider-specific message or response body |
| Normalized fields? | Only partially; OpenCode normalizes some OpenAI-style quota errors into friendlier text |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Very close; the hook gets the original raw event, the CLI wraps it |

Assessment: "no funds" is not a dedicated OpenCode event. It is a subclass of provider error handling.

### Auth

| Question | Answer |
| --- | --- |
| CLI JSON event type | `error` when auth fails |
| Best raw event | `session.error` with `ProviderAuthError` |
| How to distinguish | `error.name === "ProviderAuthError"` |
| Can auth kind be detected? | Not from the current `opencode run --format json` stream |
| Hook exposure | Yes, via `event` hook on `session.error` |
| Stream vs hook parity | Same fundamental signal, but neither includes "API key vs OAuth vs subscription" as a structured field |

Important nuance: older or abandoned community proposals exposed fields like `apiKeySource`, but the current `run` implementation does not.

### Permissions: Can't Read File

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `tool_use` with `part.tool === "read"` and `part.state.status === "error"` |
| Full path available? | Yes, via `part.state.input.filePath` for the attempted read |
| Reason available? | Sometimes. Explicit deny usually becomes a generic permission-denied error string. Ask-mode rejection becomes a rejection/corrected message |
| How to distinguish | Check `tool === "read"`, inspect `state.error`, and inspect `state.input.filePath` |
| Hook exposure | Yes. `event` hook sees `permission.asked` for ask-mode reads; `tool.execute.before` sees raw args even earlier |
| Stream vs hook parity | Hooks are richer. CLI JSON only shows the post-failure tool record; hooks can expose the permission request itself |

Important source-level detail:

- The read tool asks for permission `read`.
- For reads, the raw permission request uses `patterns: [absolute_file_path]`.
- Read permission metadata is empty, so the file path comes primarily from `patterns` or `tool.state.input.filePath`.

### Permissions: Can't Write File

| Question | Answer |
| --- | --- |
| CLI JSON event type | Usually `tool_use` with `tool` equal to `write`, `edit`, or `apply_patch`, and `state.status === "error"` |
| Full path available? | Usually yes |
| Reason available? | Sometimes; explicit deny becomes a generic permission-denied error string |
| How to distinguish | Inspect `tool`, `state.input`, and `state.error` |
| Hook exposure | Yes. `permission.asked` plus `tool.execute.before/after` are both relevant |
| Stream vs hook parity | Hooks are significantly richer for write-like tools because metadata includes diffs and per-file details |

Important tool differences:

- `write`: `state.input.filePath` gives the absolute path; permission metadata includes `filepath` and `diff`.
- `edit`: `state.input.filePath` gives the absolute path; permission metadata includes `filepath` and `diff`.
- `apply_patch`: the path may be embedded in `patchText`, but permission metadata includes relative paths, total diff, and per-file details.

### Tokens Consumed

| Question | Answer |
| --- | --- |
| CLI JSON event type | `step_finish` |
| Session total available directly? | No dedicated final total event in the current run formatter |
| Granular data available? | Yes, per model step |
| Cost basis available? | Yes, `part.cost` |
| Hook exposure | Yes, via `message.part.updated` for `step-finish`, and `message.updated` for assistant-turn totals |
| Stream vs hook parity | Hooks are richer because `message.updated` exposes final assistant-turn `cost` and `tokens` in addition to per-step accounting |

The best current automation strategy is to sum every `step_finish.part.tokens` and `step_finish.part.cost` record you observe.

### Model Used

| Question | Answer |
| --- | --- |
| CLI JSON event type | No dedicated event in the current `run --format json` stream |
| Can model be detected reliably from CLI JSON? | Not generally |
| Hook exposure | Yes, via `message.updated` |
| Raw hook fields | Assistant messages expose `providerID` and `modelID`; user messages expose `model.providerID` and `model.modelID` |
| Stream vs hook parity | Hooks are much better; the CLI formatter drops model identity entirely |

The default human formatter prints `agent · modelID` to stderr when the assistant starts, but that is not structured stdout data.

### Human in the Loop

| Question | Answer |
| --- | --- |
| Can the CLI JSON stream detect prompts/questions? | Not today |
| Can the CLI JSON stream detect permission prompts? | Not as structured stdout; `run` handles them internally and writes warnings to stderr |
| Hook exposure | Yes, strongly |
| Relevant hook events | `question.asked`, `question.replied`, `question.rejected`, `permission.asked`, `permission.replied` |
| Stream vs hook parity | Not close. Hooks expose full request payloads; CLI JSON exposes none of them |

Current non-interactive behavior nuances:

- `opencode run` creates sessions with `question`, `plan_enter`, and `plan_exit` denied.
- That suppresses built-in primary-agent question/plan approval flows in non-interactive mode.
- Built-in subagents also inherit `question: deny` by default from the agent defaults.
- Custom agents or plugins can still create human-in-the-loop scenarios that the CLI JSON stream will not surface cleanly.

For subagents specifically:

- A `task` tool result can tell you that a child session existed and which model it used.
- It does **not** expose live child-session `question.asked` or `permission.asked` events on stdout.
- The plugin event bus is the only reliable structured source for those.

### Injecting into Subagent Prompt

| Question | Answer |
| --- | --- |
| First-class CLI support? | No |
| Task-tool prompt field? | Yes, but it is controlled by the parent model when it calls `task` |
| Caller-side runtime append field? | No dedicated flag or JSON-stream mechanism |
| Hook workaround? | Partially, via system/message transformation hooks |
| Stream vs hook parity | The CLI stream gives no help here; hooks are the only structured interception point |

What is possible today:

- The parent agent can include any text it wants in the `task` tool's `prompt`.
- Agent definitions in `.opencode/agents/*.md` can inject persistent subagent instructions.
- Global/project instructions and plugin hooks such as `experimental.chat.system.transform` and `chat.message` can modify session context more broadly.

What is **not** available today:

- A dedicated non-interactive CLI flag like "append this extra string to every subagent prompt".
- A structured child-prompt injection field surfaced by `opencode run --format json`.

## Gotchas

### No formal schema for the exact CLI NDJSON envelope

The official OpenAPI and TypeScript types are good, but they stop at the underlying message/event model. If your parser depends on the exact `run --format json` envelope, you are coding against implementation behavior in `run.ts`.

### "Raw JSON events" is slightly misleading

The CLI docs say `json` means raw JSON events. In practice, `opencode run --format json` emits a **filtered and reformatted** stream, not the full raw event bus. Critical signals like `message.updated`, `session.status`, `permission.asked`, and `question.asked` are omitted from stdout.

### No session-complete event

Community requests such as <https://github.com/anomalyco/opencode/issues/17221> explicitly ask for a terminal event containing the session ID, but the current formatter still does not emit one. Callers have to infer completion from process exit and the last observed events.

### Tool start visibility is incomplete

Open PR <https://github.com/anomalyco/opencode/pull/18249> exists specifically because current JSON mode does not provide good generic "tool started" visibility. If you need live progress bars or "tool is running" UX, the current stdout stream is weaker than the hook layer.

### Docs and SDK examples have drifted

Issue <https://github.com/anomalyco/opencode/issues/14875> reports that the SDK docs showed structured output under `structured_output`, while the current model stores it under `structured`. That kind of drift matters when you are generating parsers or adapters automatically.

### Structured output and reasoning-model interactions have been brittle

Issue <https://github.com/anomalyco/opencode/issues/15226> documents a real failure mode where `toolChoice: "required"` for structured output collided with thinking-enabled models. Open PR <https://github.com/anomalyco/opencode/pull/18450> exists to move toward native provider JSON-schema support.

### Third-party harness docs can be outdated

Some third-party documentation is useful but lags the source. For example:

- Cub's OpenCode harness page assumes `opencode run` behaves like an auto-approve autonomous harness.
- Cupcake's OpenCode reference simplifies plugin behavior into a smaller event model than OpenCode actually exposes.

Treat those docs as integration notes, not as the source of truth.

## Timeline

The dates below focus on structured output in non-interactive or machine-consumable workflows.

| Date | Event | Why it matters |
| --- | --- | --- |
| 2025-06-29 | PR #533 proposed an earlier `run --print` mode with `json` and `stream-json` outputs: <https://github.com/anomalyco/opencode/pull/533> | Useful historical context: OpenCode experimented with richer machine-readable non-interactive output before the current `--format json` shape |
| 2025-10-31 | PR #3638 updated docs/source alignment for `opencode run --format json`: <https://github.com/anomalyco/opencode/pull/3638> | This is the earliest clear upstream evidence I found that the current `run --format json` mode was established and documented as "raw JSON events" |
| 2025-12-16 | Feature request #5639 opened for SDK structured outputs: <https://github.com/anomalyco/opencode/issues/5639> | Marks explicit demand for schema-constrained structured output beyond plain JSON streaming |
| 2026-02-12 | PR #8161 merged and shipped in release `v1.1.60`: <https://github.com/anomalyco/opencode/pull/8161>, <https://github.com/anomalyco/opencode/releases/tag/v1.1.60> | Official introduction of Claude Agent SDK-style structured outputs in the OpenCode SDK |
| 2026-02-12 | Issue #13342 reported docs drift immediately after the structured-output merge: <https://github.com/anomalyco/opencode/issues/13342> | Shows that consumers had to track naming/details closely because documentation changed quickly |
| 2026-02-24 | Issue #14875 reported the docs still used the wrong structured-output field name: <https://github.com/anomalyco/opencode/issues/14875> | Confirms that even after rollout, the documentation for structured output remained easy to misread |
| 2026-02-26 | Issue #15226 documented a structured-output failure with thinking-enabled models: <https://github.com/anomalyco/opencode/issues/15226> | Highlights a real integration gotcha for machine-reliant callers |
| 2026-03-19 | PR #18249 proposed emitting running `tool_use` events in JSON mode: <https://github.com/anomalyco/opencode/pull/18249> | Important for observability: it shows current JSON mode is still seen as incomplete by downstream integrators |
| 2026-03-20 | PR #18450 proposed moving structured output to native `Output.object()` support: <https://github.com/anomalyco/opencode/pull/18450> | Signals likely future change in how structured output is implemented and possibly how failures look |
| 2026-04-06 | Release `v1.3.16` fixed output token totals when reasoning tokens are separated: <https://github.com/anomalyco/opencode/releases/tag/v1.3.16> | Important for budget/accounting consumers who sum token usage from structured output |

