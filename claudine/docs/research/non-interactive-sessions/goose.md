---
schema: https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json
schema_type: open-api
data_format: json / ndjson
docs: https://goose-docs.ai/docs/guides/running-tasks/
created: 2026-04-10
last_updated: 2026-04-10
---

# Goose CLI Structured Output in Non-Interactive Sessions

## Summary

Goose currently exposes two machine-readable output modes for `goose run`: batch `json` and streaming `stream-json`. The batch mode emits one JSON document after the run completes. The streaming mode emits one JSON object per line as the run progresses, so it is functionally NDJSON even though the CLI flag is named `stream-json`.

For Claudine, the important distinction is that Goose documents the modes, but does not publish a dedicated formal schema for the full `goose run` wire format. The best provider-owned formal schema I found is Goose's OpenAPI document for the desktop/server API plus its generated TypeScript types. Those formalize the nested `Message`, `MessageContent`, `ActionRequiredData`, and `SystemNotificationContent` objects that also appear inside CLI output, but they do not define the outer `stream-json` event envelope. The outer envelope is currently defined only in source by Rust `serde` types such as `StreamEvent`, `JsonOutput`, and `JsonMetadata`.

The current source-backed stream contract is narrower than some older notes and examples imply:

- Current top-level stream events are `message`, `notification`, `error`, and `complete`.
- I did not find a current top-level `model_change` event in Goose mainline.
- `stream-json` uses `snake_case` at the outer envelope, while nested `message.content[]` items use `camelCase`.
- `notification` payloads are flattened into the top level of the event object.
- Several high-value conditions Claudine may care about, especially auth failures, permissions failures, and some subagent activity, are still exposed only as nested message content or plain-text log/error strings rather than dedicated event types.

The practical integration guidance is:

- Prefer `goose run --output-format stream-json` for live orchestration.
- Parse the outer event envelope first, then parse nested Goose `Message` objects exactly.
- Treat `complete.total_tokens` as the only stable built-in session token total in the stream.
- Expect some cases to require heuristics over `toolResponse.toolResult.error`, `systemNotification.msg`, or `error.error`.
- Treat human-in-the-loop prompts in headless mode as a current sharp edge, not a cleanly modeled workflow.

## Schema

### What Goose publishes today

Goose does not currently publish a standalone schema artifact for the complete CLI wire format of:

- `goose run --output-format json`
- `goose run --output-format stream-json`

I did not find:

- A dedicated JSON Schema for the CLI output
- An AsyncAPI document for the streaming event feed
- An OpenAPI path or component that models the outer `stream-json` envelope
- A provider-published TypeScript definition specifically for the CLI stream events

### Best formal definitions I found

The best provider-owned formal definitions are split across three layers:

| Surface | Source | Schema language | Scope | Notes |
| --- | --- | --- | --- | --- |
| Nested message types | [OpenAPI document](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json) | OpenAPI 3 | `Message`, `MessageContent`, `ActionRequiredData`, `SystemNotificationContent`, `SystemNotificationType`, and related types | This is the best formal schema Goose publishes, but it does not define the outer CLI `stream-json` event envelope. |
| Generated consumer types | [Generated TypeScript types](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/api/types.gen.ts) | TypeScript types generated from OpenAPI | Same nested message and action-required shapes | Useful as a formalized consumer view of the provider schema. |
| Outer CLI event envelope | [`crates/goose-cli/src/session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs) | Rust `serde` enums/structs | `StreamEvent`, `NotificationData`, `JsonOutput`, `JsonMetadata` | This is source-backed and authoritative, but not published as a schema artifact. |

### Closest thing to a full current schema

Current `stream-json` top-level events are defined in source as:

```json
{"type":"message","message":{...}}
{"type":"notification","extension_id":"developer","message":"..."}
{"type":"notification","extension_id":"developer","progress":0.5,"total":1.0,"message":"..."}
{"type":"error","error":"..."}
{"type":"complete","total_tokens":1234}
```

Important current details:

- Outer `type` values are `snake_case`.
- `notification` uses `#[serde(flatten)]`, so the log/progress payload is not nested.
- `complete.total_tokens` is optional.
- There is no current source-backed `model_change` variant in `StreamEvent`.

Current batch `json` mode emits one final JSON object:

```json
{
  "messages": [
    {
      "id": null,
      "role": "assistant",
      "created": 1743999999,
      "content": [],
      "metadata": {
        "userVisible": true,
        "agentVisible": true
      }
    }
  ],
  "metadata": {
    "total_tokens": 1234,
    "status": "completed"
  }
}
```

### Nested message schema details that matter

The nested Goose message schema is formalized much better than the outer CLI envelope.

Examples from the current provider-owned OpenAPI and TypeScript types:

- `MessageContent.type` values include `text`, `image`, `toolRequest`, `toolResponse`, `toolConfirmationRequest`, `actionRequired`, `frontendToolRequest`, `thinking`, `redactedThinking`, and `systemNotification`
- `ActionRequiredData.actionType` values include `toolConfirmation`, `elicitation`, and `elicitationResponse`
- `SystemNotificationType` currently enumerates `thinkingMessage`, `inlineMessage`, and `creditsExhausted`

That means Goose does have a formal provider schema for the nested message payloads Claudine will likely parse, but not for the complete top-level streaming wire format.

### What I checked

I looked in all of the following places before concluding that Goose does not publish a full CLI stream schema:

- Official docs:
  - [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
  - [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
  - [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
  - [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/)
  - [goose Permission Modes](https://goose-docs.ai/docs/guides/goose-permissions/)
  - [Customizing Prompt Templates](https://goose-docs.ai/docs/guides/prompt-templates/)
- Current Goose source:
  - [`crates/goose-cli/src/session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
  - [`crates/goose/src/conversation/message.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)
  - [`ui/desktop/openapi.json`](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json)
  - [`ui/desktop/src/api/types.gen.ts`](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/api/types.gen.ts)
- Release history:
  - [v1.25.0](https://github.com/aaif-goose/goose/releases/tag/v1.25.0)
  - [v1.26.0](https://github.com/aaif-goose/goose/releases/tag/v1.26.0)
  - [v1.27.0](https://github.com/aaif-goose/goose/releases/tag/v1.27.0)
  - [v1.29.0](https://github.com/aaif-goose/goose/releases/tag/v1.29.0)
  - [v1.17.0](https://github.com/aaif-goose/goose/releases/tag/v1.17.0)
  - [v1.0.30](https://github.com/aaif-goose/goose/releases/tag/v1.0.30)
- Broader ecosystem searches targeted at Vercel AI SDK, LangChain, and Goose-related GitHub discussions

I did not find a respected third-party project that formalizes Goose's CLI stream schema either.

## Documentation

### Official documentation

The most useful official documentation for structured non-interactive output is:

- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
  - Best direct explanation of `goose run`
  - Explicitly documents `--output-format json` and `--output-format stream-json`
- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
  - Enumerates the `run` flags
  - Confirms the available output formats are `text`, `json`, and `stream-json`
- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
  - Best explanation of headless behavior and non-interactive limitations
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation/)
  - Explains the structured user-input request model Goose uses
- [goose Permission Modes](https://goose-docs.ai/docs/guides/goose-permissions/)
  - Important context for tool approval requests
- [Customizing Prompt Templates](https://goose-docs.ai/docs/guides/prompt-templates/)
  - Important for subagent prompt injection via `subagent_system.md`
- [Smart Context Management](https://goose-docs.ai/docs/guides/sessions/smart-context-management/)
  - Important for token-limit behavior and credit-balance messaging

### Official code and schema references

When the prose docs are vague, the current authoritative references are:

- [`StreamEvent`, `NotificationData`, `JsonOutput`, and `JsonMetadata`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [`Message`, `MessageContent`, `ActionRequiredData`, and `SystemNotificationContent`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)
- [OpenAPI document](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json)
- [Generated TypeScript consumer types](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/api/types.gen.ts)

### Article-style documentation and release writeups

The highest-signal article-style resources I found are mostly Goose's own blog/tutorial material:

- [Previewing Goose v1.0 Beta](https://goose-docs.ai/blog/2024/12/06/previewing-goose-v10-beta)
  - Earliest clear public article-style mention I found for headless mode
- [Using goose in Headless Mode for Automation](https://goose-docs.ai/docs/tutorials/headless-goose/)
  - Tutorial, but it reads like a practical operational article
- [goose v1.25.0 is here](https://github.com/aaif-goose/goose/releases/tag/v1.25.0)
  - Explicitly calls out `stream-json` usage in Goose's provider integrations
- [I had Goose Build its Own Secure Recipe Scanner](https://goose-docs.ai/blog/2025/08/25/goose-became-its-own-watchdog)
  - A real-world headless automation case study

### Broader ecosystem coverage

I did not find a strong set of independent third-party articles focused specifically on Goose's structured output schema. Coverage is much stronger for:

- Headless automation as a workflow
- Goose recipes and GitHub Actions
- Provider integration behavior

For the schema itself, the official docs and current source are much more useful than community writeups.

## CLI

### Available output formats

Goose currently accepts these output formats for `goose run`:

| CLI value | Returns | Best use |
| --- | --- | --- |
| `text` | Human-readable terminal output | Manual runs |
| `json` | One final JSON object after the run completes | Batch automation, CI, logs |
| `stream-json` | One JSON object per line during execution | Live orchestration, progress monitoring, wrappers like Claudine |

### Syntax

The current CLI syntax is:

```bash
goose run --output-format <text|json|stream-json> ...
```

Examples:

```bash
goose run --output-format json -t "summarize this repo"
```

```bash
goose run --output-format stream-json -t "summarize this repo"
```

Structured output can be combined with the usual non-interactive inputs:

- `-t, --text`
- `-i, --instructions <FILE>`
- `-i -` for stdin
- `--recipe ...`

### Behavioral differences and side effects

`text`

- Not a stable machine contract
- Best for humans

`json`

- Emits a single final JSON object
- Includes the full saved conversation messages plus `metadata.total_tokens`
- Good for batch post-processing

`stream-json`

- Emits line-delimited JSON, not one single JSON document
- Is the only built-in format suitable for real-time wrappers
- Emits `error` as JSON rather than plain stderr
- Ends with a `complete` event

Important current side effects:

- `stream-json` is NDJSON in practice, even though the CLI flag is not named `ndjson`
- `json` gives you the final conversation state, not an event log
- `notification` payloads are flattened
- Rich MCP or subagent notification payloads are sometimes reduced to formatted strings
- Current Goose output does not include a stable structured model/provider event
- Human-in-the-loop requests are not cleanly represented in headless mode today

## Gotchas

### No published full CLI schema

Goose documents the formats but does not publish a complete machine-readable schema for the outer CLI wire format. The best formal schema is partial and covers the nested message objects, not the full stream envelope.

Sources:

- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
- [OpenAPI document](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json)
- [`StreamEvent` source](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### Outer events are `snake_case`; nested message content is `camelCase`

This is a real parser footgun:

- Outer event: `{"type":"complete"}`
- Nested message content: `{"type":"toolRequest"}`
- Nested action discriminator: `actionType`
- Nested notification discriminator: `notificationType`

Sources:

- [`StreamEvent` source](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [`MessageContent` source](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs)

### `notification` events flatten their payload

Current source emits:

```json
{"type":"notification","extension_id":"developer","message":"..."}
```

and:

```json
{"type":"notification","extension_id":"developer","progress":0.5,"total":1.0,"message":"..."}
```

not:

```json
{"type":"notification","log":{"message":"..."}}
```

Source:

- [`NotificationData` and `#[serde(flatten)]`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)

### Stream consumers lose some MCP and subagent structure

Current Goose CLI turns several richer MCP notifications into formatted strings before printing them to `stream-json`. This affects subagent tool-call visibility and task-execution updates.

Sources:

- [`handle_mcp_notification`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [Issue: stream subagent output to CLI terminal](https://github.com/aaif-goose/goose/issues/6178)
- [PR: restore subagent tool call notifications after summon refactor](https://github.com/aaif-goose/goose/pull/7243)

### Headless human-in-the-loop behavior is still sharp-edged

Current `process_agent_response` logic still routes tool confirmations and elicitation requests through interactive handlers before normal stream emission. There is an open PR specifically to stop headless sessions from hanging when approvals are required.

Sources:

- [`process_agent_response` logic in `session/mod.rs`](https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs)
- [Open PR: prevent session hang when tool approval required in headless mode](https://github.com/aaif-goose/goose/pull/7915)

### Developer extension docs lag the current code

The official Developer MCP page still documents an older surface (`text_editor`, `analyze`, `screen_capture`, `image_processor`) while current source and recent Goose research show the developer built-in has shifted toward a different tool surface. For Claudine, source should win when the docs and code disagree.

Sources:

- [Developer Extension docs](https://goose-docs.ai/docs/mcp/developer-mcp/)
- [`bundled-extensions.json`](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/components/settings/extensions/bundled-extensions.json)
- Current Goose source

### Streaming tool payloads still have active bug reports

The stream layer continues to see parser-edge bugs for large or fragmented tool-call payloads.

Source:

- [Issue: Tool call JSON parse failures with large payloads and streaming fragments](https://github.com/aaif-goose/goose/issues/8272)

## Timeline

This timeline focuses on structured-output-adjacent milestones that matter for non-interactive automation.

| Date | Version | Event | Why it matters |
| --- | --- | --- | --- |
| 2025-06-27 | [v1.0.30](https://github.com/aaif-goose/goose/releases/tag/v1.0.30) | Subagents shipped | Subagent observability later becomes a key stream consumer requirement. |
| 2025-12-18 | [v1.17.0](https://github.com/aaif-goose/goose/releases/tag/v1.17.0) | MCP elicitation support shipped | Introduces structured user-input requests modeled as `actionRequired` content. |
| 2026-02-18 | [v1.25.0](https://github.com/aaif-goose/goose/releases/tag/v1.25.0) | Release notes explicitly mention `stream-json` in provider integration work | Confirms `stream-json` is a first-class integration surface, not just a side feature. |
| 2026-02-26 | [v1.26.0](https://github.com/aaif-goose/goose/releases/tag/v1.26.0) | Release notes mention low-balance detection and subagent tool-call stream work | Important for Claudine's monitoring story around funds and subagents. |
| 2026-03-05 | [v1.27.0](https://github.com/aaif-goose/goose/releases/tag/v1.27.0) | Developer `shell` tool gains structured `{stdout, stderr}` output schema | A meaningful improvement in machine-readability of tool results. |

Context note:

- On 2026-04-07, Goose announced its move to the Agentic AI Foundation. Older references to `block.github.io/goose` and `github.com/block/goose` now redirect to `goose-docs.ai` and `github.com/aaif-goose/goose`.

## Tools

### Built-in extensions available out of the box

The current bundled built-ins are declared in Goose's bundled extension manifest:

| Extension | Bundled | Enabled by default | Primary role |
| --- | --- | ---: | --- |
| `developer` | yes | yes | Development and file/system work |
| `computercontroller` | yes | no | Browser and desktop automation |
| `autovisualiser` | yes | no | Charts and diagrams |
| `memory` | yes | no | Persistent preference/memory storage |
| `tutorial` | yes | no | Guided tutorials |

Sources:

- [`bundled-extensions.json`](https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/components/settings/extensions/bundled-extensions.json)
- [Developer MCP docs](https://goose-docs.ai/docs/mcp/developer-mcp/)
- [Computer Controller MCP docs](https://goose-docs.ai/docs/mcp/computer-controller-mcp/)
- [Auto Visualiser MCP docs](https://goose-docs.ai/docs/mcp/autovisualiser-mcp/)
- [Memory MCP docs](https://goose-docs.ai/docs/mcp/memory-mcp/)
- [Tutorial MCP docs](https://goose-docs.ai/docs/mcp/tutorial-mcp/)

### Built-in tool inventory

This is the current out-of-the-box built-in tool inventory I could verify:

| Extension | Tool names | Verification basis |
| --- | --- | --- |
| `developer` | `write`, `edit`, `shell`, `tree` | Current source in `platform_extensions/developer/mod.rs` |
| `computercontroller` | `web_scrape`, `automation_script`, `computer_control`, `xlsx_tool`, `docx_tool`, `pdf_tool`, `cache` | Current source in `goose-mcp/src/computercontroller/mod.rs` |
| `autovisualiser` | `render_sankey`, `render_radar`, `render_donut`, `render_treemap`, `render_chord`, `render_map`, `render_mermaid`, `show_chart` | Current source in `goose-mcp/src/autovisualiser/mod.rs` |
| `memory` | `remember_memory`, `retrieve_memories`, `remove_memory_category`, `remove_specific_memory` | Official docs plus current repo references |
| `tutorial` | `load_tutorial` | Current source in `goose-mcp/src/tutorial/mod.rs` |

Notes:

- The Developer MCP docs still describe an older surface, so I treated current source as authoritative there.
- The built-in inventory above is about bundled extensions and their tools, not third-party MCP servers the user may add later.

### What the stream exposes around tool calls

For normal tool usage, Goose surfaces tool activity inside `message` events:

1. A `message` event contains one or more `toolRequest` content items
2. A later `message` event contains the matching `toolResponse` content items
3. The `id` ties the request and response together

This means the stream usually exposes:

- Tool name
- Tool arguments
- Success vs error
- Tool output
- Some provider metadata

What it does not guarantee:

- A dedicated before/after top-level tool event type
- Stable cost metadata
- Full-fidelity forwarding of every MCP notification related to the tool

### What the stream exposes around notifications

In addition to `toolRequest` and `toolResponse`, Goose emits top-level `notification` events for MCP logging/progress.

These usually expose:

- `extension_id`
- `message` for log-style notifications
- `progress`
- `total`
- optional `message` for progress notifications

What it may lose:

- Original structured notification objects for subagent and task-execution events

### Example: Developer `shell`

Possible request:

```json
{
  "type": "message",
  "message": {
    "role": "assistant",
    "content": [
      {
        "type": "toolRequest",
        "id": "call_1",
        "toolCall": {
          "status": "success",
          "value": {
            "name": "shell",
            "arguments": {
              "command": "cargo test -p claudine"
            }
          }
        }
      }
    ]
  }
}
```

Possible response:

```json
{
  "type": "message",
  "message": {
    "role": "user",
    "content": [
      {
        "type": "toolResponse",
        "id": "call_1",
        "toolResult": {
          "status": "success",
          "value": {
            "content": [
              {
                "type": "text",
                "text": "..."
              }
            ]
          }
        }
      }
    ]
  }
}
```

Since v1.27.0, Goose also documents a structured `{stdout, stderr}` shell return path in the Developer tooling story, which is relevant if Claudine wants better shell-output extraction.

### Example: Credits exhausted

Possible structured signal:

```json
{
  "type": "message",
  "message": {
    "role": "assistant",
    "content": [
      {
        "type": "systemNotification",
        "notificationType": "creditsExhausted",
        "msg": "Please add credits to your account, then resend your message to continue.",
        "data": {
          "top_up_url": "https://..."
        }
      }
    ]
  }
}
```

The top-up URL is extractable when the provider supplies it.

### Example: Subagent tool notification

Possible surfaced stream event:

```json
{
  "type": "notification",
  "extension_id": "developer",
  "message": "[subagent:3] shell | developer"
}
```

The CLI currently exposes a formatted log string, not the original rich notification payload, for this case.

## Use Cases

Before drilling into the cases: Goose has exactly one true hook today, `GOOSE_STATUS_HOOK`, and it only receives a single positional status argument such as `thinking` or `waiting`. None of the cases below have hook parity with the structured stream. Unless otherwise noted, the answer to "is this also exposed as a hook?" is "no".

### Plan Cap Approaching

I did not find a stable, source-backed structured event for "plan cap approaching" or "usage cap approaching".

What I found instead:

- Goose docs say it can warn when credits are running low or exhausted
- The current formal `SystemNotificationType` enum only contains `thinkingMessage`, `inlineMessage`, and `creditsExhausted`
- The only explicit credit-related structured notification I found in current source is `creditsExhausted`

Practical conclusion:

- There is no current source-backed event type dedicated to "approaching cap"
- If Goose does surface a low-balance warning before exhaustion, it is not presently formalized as its own stable structured enum
- The best fallback is heuristic text matching over a `systemNotification.msg` or `error.error`, but that is inference, not a schema guarantee
- I found no way to extract remaining percentage, token count, or reset-window time from the current stream

Hook parity:

- No. `GOOSE_STATUS_HOOK` does not expose balance or cap information.

### Plan Capped

The best current structured signal is:

- top-level event type: `message`
- nested content type: `systemNotification`
- nested `notificationType`: `creditsExhausted`

Current source path:

- `ProviderError::CreditsExhausted` is turned into an assistant `systemNotification`
- `notification.data.top_up_url` may be present

What can be extracted:

- A stable classification: `creditsExhausted`
- A human-readable message in `msg`
- An optional `top_up_url`

What cannot be extracted from the current stream:

- Remaining balance
- Remaining tokens
- Reset-window time
- A reliable distinction between "subscription cap hit" and "credit wallet empty"

Hook parity:

- No. The stream includes the useful data; the hook does not.

### No Funds

For Goose, "no funds" and "plan capped" currently collapse into the same structured concept when the provider reports HTTP 402 or equivalent: `creditsExhausted`.

Relevant current mapping:

- `402 Payment Required` maps to `ProviderError::CreditsExhausted`
- That becomes a `systemNotification` with `notificationType: "creditsExhausted"`

Practical conclusion:

- The stable structured signal is the same one used for plan exhaustion: `creditsExhausted`
- Goose does not currently expose a separate structured variant for "no funds" versus "plan quota exhausted"
- If you need to distinguish them, you would be forced into provider-specific heuristics over the message text or out-of-band billing metadata

Hook parity:

- No.

### Auth

The stable signal I found for authentication failures is:

- top-level event type: `error`
- payload field: `error`

Current provider behavior:

- `401` and `403` map to `ProviderError::Authentication`
- `handle_agent_error` emits the provider error as a plain string in the `error` event for `stream-json`

Practical conclusion:

- You can often detect auth failures by matching `error.error` text such as `Authentication failed`
- I did not find a stable structured field that tells you the auth kind used by the user, such as API key vs subscription vs device flow vs OAuth
- Auth kind is effectively out-of-band configuration state, not part of the stream contract

Hook parity:

- No.

### Permissions: Can't Read File

I did not find a dedicated "file read permission denied" event type.

Current best detection path:

- top-level event type: `message`
- nested content type: `toolResponse`
- nested `toolResult.status == "error"`
- error text containing clues such as `Permission denied`, `os error 13`, `Access is denied`, or provider-specific equivalents

How to identify the path:

- Prefer the preceding `toolRequest` arguments when available
- Fall back to path extraction from `toolResult.error` or from structured shell output if the tool is `shell`

How to distinguish from similar cases:

- Look at the tool name and arguments in the immediately preceding `toolRequest`
- Read-like cases typically come from tree/list/read/view commands, or from shell commands that attempted to read
- Goose does not emit a dedicated reason code beyond the error string

Hook parity:

- No.

### Permissions: Can't Write File

I also did not find a dedicated "file write permission denied" event type.

Current best detection path:

- top-level event type: `message`
- nested content type: `toolResponse`
- nested `toolResult.status == "error"`
- error text describing the denial

How to identify the path:

- Prefer the preceding `toolRequest` arguments for `write`, `edit`, or shell-based file modifications
- Fall back to parsing the error text

How to distinguish from similar cases:

- Write-like tools and shell commands are the main discriminator
- In interactive approval modes, a request may first appear as `actionRequired` with `actionType: "toolConfirmation"`, but that is a request for permission, not evidence that the underlying filesystem blocked the write
- In headless runs, this area is currently especially sharp-edged because approval handling is not cleanly modeled

Hook parity:

- No.

### Tokens Consumed

Current stable overall token totals:

- `stream-json`: `complete.total_tokens`
- `json`: `metadata.total_tokens`

What I did not find as a stable built-in contract:

- Per-turn token usage in the CLI stream
- A machine-readable cost basis in `json` or `stream-json`
- Structured dollar-cost reporting in the non-interactive output formats

Practical conclusion:

- Goose exposes a useful session-level token total
- Claudine should not expect reliable per-turn usage or cost data from the CLI structured output today

Hook parity:

- No.

### Model Used

I did not find a stable current stream event that announces the active model/provider.

Important current source finding:

- The current `StreamEvent` enum does not include `model_change`

Practical conclusion:

- There is no current source-backed structured model-selection event in `stream-json`
- The reliable way to know the model is out-of-band:
  - the caller's own `--provider` / `--model` arguments
  - environment variables
  - recipe metadata
  - the session/config state you launched Goose with
- Older notes that mention `model_change` should be treated as stale unless you pin to an older Goose build that still emitted it

Hook parity:

- No.

### Human in the Loop

Goose does have structured message types for human-in-the-loop requests:

- `message.content[].type == "actionRequired"`
- `actionType == "toolConfirmation"`
- `actionType == "elicitation"`
- `actionType == "elicitationResponse"`

Those shapes are formalized in the provider-owned OpenAPI and TypeScript types.

However, there is a major practical caveat for non-interactive sessions:

- Current CLI session processing intercepts tool confirmations and elicitation requests before the normal stream-emission path
- There is an open PR specifically to stop headless sessions from hanging when approval is required

Practical conclusion:

- Goose conceptually models these requests as structured data
- But in current headless CLI behavior, you should not assume you will receive them cleanly as `stream-json` events
- For subagents, I found no dedicated structured subagent-specific human-input event in the stream; subagent activity is mostly surfaced as notifications or nested messages, and some of that gets flattened to log strings

If you do receive the raw message before interception, you can extract:

- For tool approval:
  - `data.actionType == "toolConfirmation"`
  - `id`
  - `toolName`
  - `arguments`
  - optional `prompt`
- For elicitation:
  - `data.actionType == "elicitation"`
  - `id`
  - `message`
  - `requested_schema`

Hook parity:

- No. `GOOSE_STATUS_HOOK` does not expose these requests or their payloads.

### Injecting into Subagent Prompt

Yes, but not through the structured stream itself.

What Goose supports today:

- A customizable prompt template named [`subagent_system.md`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/prompts/subagent_system.md)
- A prompt-template override mechanism under `~/.config/goose/prompts/`
- Runtime rendering of subagent prompt context in `build_subagent_prompt`

Practical conclusion:

- If you control Goose's config, you can inject persistent instructions into all subagents by overriding `subagent_system.md`
- That is the cleanest current way to warn subagents that they are in non-interactive mode and should not ask for input
- I did not find a dedicated per-run CLI flag that injects extra context only into spawned subagents
- This is configuration-time prompt customization, not an event-stream feature

Hook parity:

- No. This is a prompt-template/configuration surface, not a hook.

## Sources

- Goose docs:
  - https://goose-docs.ai/docs/guides/goose-cli-commands/
  - https://goose-docs.ai/docs/guides/running-tasks/
  - https://goose-docs.ai/docs/tutorials/headless-goose/
  - https://goose-docs.ai/docs/guides/mcp-elicitation/
  - https://goose-docs.ai/docs/guides/goose-permissions/
  - https://goose-docs.ai/docs/guides/prompt-templates/
  - https://goose-docs.ai/docs/guides/sessions/smart-context-management/
  - https://goose-docs.ai/docs/guides/handling-llm-rate-limits-with-goose/
- Goose schema/source:
  - https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/openapi.json
  - https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/api/types.gen.ts
  - https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/mod.rs
  - https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/cli.rs
  - https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-cli/src/session/output.rs
  - https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/conversation/message.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/agent.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/subagent_handler.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/subagent_execution_tool/notification_events.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/agents/platform_extensions/developer/mod.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/computercontroller/mod.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/autovisualiser/mod.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose-mcp/src/tutorial/mod.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/crates/goose/src/providers/openai_compatible.rs
- https://raw.githubusercontent.com/aaif-goose/goose/main/ui/desktop/src/components/settings/extensions/bundled-extensions.json
- Goose releases and article-style resources:
  - https://github.com/aaif-goose/goose/releases/tag/v1.29.0
  - https://github.com/aaif-goose/goose/releases/tag/v1.27.0
  - https://github.com/aaif-goose/goose/releases/tag/v1.26.0
  - https://github.com/aaif-goose/goose/releases/tag/v1.25.0
  - https://github.com/aaif-goose/goose/releases/tag/v1.17.0
  - https://github.com/aaif-goose/goose/releases/tag/v1.0.30
  - https://goose-docs.ai/blog/2024/12/06/previewing-goose-v10-beta
  - https://goose-docs.ai/blog/2025/08/25/goose-became-its-own-watchdog
- Relevant issues and PRs:
  - https://github.com/aaif-goose/goose/issues/6178
  - https://github.com/aaif-goose/goose/pull/7243
  - https://github.com/aaif-goose/goose/pull/7915
  - https://github.com/aaif-goose/goose/issues/8272
