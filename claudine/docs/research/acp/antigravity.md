---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
docs: https://antigravity.google/product/antigravity-cli
acp_docs: https://github.com/shubzkothekar/antigravity-acp#readme
repo: https://github.com/google-antigravity/antigravity-cli
support: adapter
launch_modes:
  - command: agy-acp
    args: []
    transport: stdio
    adapter: antigravity-acp
    notes: "Community adapter published as npm package `antigravity-acp`; speaks newline-delimited ACP JSON-RPC over stdio and spawns the official `agy` CLI as a child process."
  - command: bun
    args: ["run", "index.ts"]
    transport: stdio
    adapter: antigravity-acp
    notes: "Source checkout launch path; Bun is required. The adapter also offers compiled single-file binaries named `agy-acp-<platform>`."
protocol_versions:
  - "ACP protocolVersion 1 (adapter initialize response observed in source)"
  - "@agentclientprotocol/sdk ^1.0.0 (adapter package.json)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Adapter implements initialize and returns protocolVersion 1, agentInfo, loadSession, prompt embeddedContext, session list/delete/resume/close/additionalDirectories, and auth logout."
  - capability: authenticate
    support: partial
    notes: "Adapter advertises Google Sign In but authenticate only validates the method id; OAuth remains owned by `agy`."
  - capability: session_new
    support: supported
    notes: "Accepts cwd and additionalDirectories, creates a persisted adapter session binding, and returns config options."
  - capability: session_load
    support: supported
    notes: "Replays conversation history from `agy` SQLite databases."
  - capability: session_prompt
    support: supported
    notes: "Spawns `agy -p` per turn and streams translated DB rows as session/update notifications."
  - capability: session_cancel
    support: supported
    notes: "Adapter handles session/cancel by sending SIGINT to child `agy` on Unix-like systems or kill on Windows."
  - capability: session_modes
    support: partial
    notes: "Adapter exposes model and mode config options. Modes are standard, plan, and bypassPermissions; only bypass maps to `--dangerously-skip-permissions`."
  - capability: streaming
    support: supported
    notes: "Adapter polls conversation SQLite rows every 200 ms and emits agent_message_chunk, tool_call, user_message_chunk during replay, session_info_update, available_commands_update, and config_option_update."
  - capability: permissions
    support: partial
    notes: "No ACP session/request_permission reverse request is used. Native `agy` handles permission prompts internally; in non-interactive print mode this may block or fail unless bypass mode is selected."
  - capability: fs_read
    support: unsupported
    notes: "Adapter does not call ACP fs/read_text_file. `agy` reads files itself inside its own workspace/add-dir model."
  - capability: fs_write
    support: unsupported
    notes: "Adapter does not call ACP fs/write_text_file. `agy` writes files itself after its own permission flow."
  - capability: terminal
    support: unsupported
    notes: "Adapter does not call ACP terminal/create, terminal/output, terminal/wait_for_exit, terminal/kill, or terminal/release."
  - capability: mcp
    support: partial
    notes: "Adapter exposes empty MCP resources/prompts/tools compatibility handlers; Antigravity CLI itself has MCP features, but the adapter does not proxy MCP servers through ACP session setup."
  - capability: media
    support: partial
    notes: "Prompt flattening supports text, resource_link, and embedded text resources. Image/audio ACP content was not verified."
  - capability: plans
    support: partial
    notes: "Plan mode is implemented as prompt injection and generated `plan.md` rows are rendered as user-facing text, not a first-class ACP plan update."
  - capability: extensions
    support: partial
    notes: "Adapter implements newer session list/delete/resume/close methods and compatibility `resources/list`, `prompts/list`, and `tools/list`, but no vendor extension namespace was documented."
reverse_requests:
  - method: session/update
    purpose: other
    client_must_handle: true
    notes: "Adapter sends all streaming output as client notifications; a useful UI must handle these updates."
  - method: session/request_permission
    purpose: permission
    client_must_handle: false
    notes: "Wrapper class exists but no adapter path calls it; permission decisions remain inside `agy`."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Unsupported by adapter."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Unsupported by adapter."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Unsupported by adapter."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Unsupported by adapter."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Unsupported by adapter."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Unsupported by adapter."
permission_model:
  mechanism: "Antigravity-native permissions, surfaced indirectly as tool_call content; adapter mode can pass `--dangerously-skip-permissions`."
  timeout: "Unknown for native prompts; adapter child process waits for `agy` to exit."
  default_policy: "Native `agy` 1.1.0 defaults to request-review mode for file writes; adapter default mode does not bypass permissions."
  approval_values: ["standard", "plan", "bypassPermissions"]
  notes: "A headless ACP client cannot answer native `agy` permission prompts through ACP. For reliable non-interactive adapter runs, use a trusted workspace plus a mode that will not prompt, or accept that `agy -p` may fail or block."
filesystem_model:
  read_methods: []
  write_methods: []
  path_base: "Adapter passes absolute or cwd-derived workspace paths to `agy` via `--add-dir`; translated tool locations use absolute paths, with project-relative display names where possible."
  sandboxing: "`agy` owns filesystem access. The adapter does not enforce a client-side filesystem sandbox and does not proxy filesystem operations through ACP reverse requests."
  notes: "Read/update rows are decoded from SQLite after `agy` has already performed the operation. File locations use 1-based line numbers where Antigravity records line ranges."
terminal_model:
  supported: false
  methods: []
  shell: "Owned by `agy`; not exposed through ACP terminal requests."
  cwd: "Adapter spawns `agy` with the session cwd and passes `--add-dir <cwd>`."
  streaming: "Command activity is rendered after SQLite polling as tool_call updates, not live terminal/output notifications."
  cancellation: "session/cancel sends SIGINT to the `agy` child on Unix-like systems and kill on Windows."
  notes: "A Claudine host policy engine cannot approve or deny individual host command executions through ACP with this adapter; it can only gate adapter launch, args, cwd, env, and whether bypass mode is allowed."
streaming_model:
  update_methods: ["session/update"]
  text_events: ["agent_message_chunk", "user_message_chunk"]
  tool_events: ["tool_call"]
  plan_events: ["tool_call kind=think", "tool_call kind=edit for plan.md text", "config_option_update mode=plan"]
  error_events: ["tool_call rawOutput", "tool_call content error block", "JSON-RPC internalError when `agy` fails before streaming"]
  notes: "Adapter polls `~/.gemini/antigravity-cli/conversations/*.db` and translates step rows. It emits appended text slices for live rows and buffered text during replay."
auth_setup:
  required: true
  mechanisms: ["Google OAuth handled by `agy`", "system keyring", "browser sign-in or SSH authorization URL", "enterprise GCP project onboarding when applicable"]
  headless_notes: "Run `agy` interactively at least once to complete onboarding, trust workspaces, and store credentials before using the adapter headlessly."
  notes: "Local settings inspected at `/Users/ken/.gemini/antigravity-cli/settings.json` show onboarding-complete cache and trustedWorkspaces; no ACP-specific auth artifact was found."
env_vars:
  - name: AGY_BIN
    effect: "Adapter uses this path for the `agy` binary and skips auto-download."
  - name: AGY_SKIP_DOWNLOAD
    effect: "Set to `1` to prevent adapter auto-download of `agy`."
  - name: AGY_EXTRA_ARGS
    effect: "Whitespace-split extra args appended to every spawned `agy` invocation."
  - name: AGY_CONVERSATIONS_DIR
    effect: "Overrides the conversation SQLite directory polled by the adapter."
  - name: AGY_CLI_CMD_OUTPUT_PERCENTAGE
    effect: "Native `agy` TUI setting for command output height; documented in 1.0.11 changelog, not adapter-specific."
  - name: AGY_CLI_HIDE_ACCOUNT_INFO
    effect: "Native `agy` TUI setting to hide account info; documented in 1.0.2 changelog, not adapter-specific."
  - name: AGY_CLI_DISABLE_LATEX
    effect: "Native `agy` setting to disable LaTeX rendering; documented in 1.0.4 changelog, not adapter-specific."
rust_client:
  crate: agent-client-protocol
  connection_type: "ClientSideConnection over child-process stdio to `agy-acp`"
  localset_required: false
  reverse_request_handlers: ["session/update", "optional session/request_permission stub", "optional fs and terminal stubs returning unsupported"]
  desktop_streaming_pattern: "Read ACP notifications on a Tokio task and forward normalized UI events over tokio::sync::mpsc to the desktop event loop."
  notes: "The adapter already speaks standard ACP over stdio, so Rust should use the official crate for JSON-RPC framing and schema types. Native `agy` cannot be used directly without a lower-level proprietary adapter because it has no ACP mode."
compatibility:
  - client: Zed
    status: partial
    issue: "zed-industries/zed#57221 requests Antigravity CLI ACP registry support; maintainers say support depends on Google or registry-compatible ACP."
    workaround: "Use terminal threads or the community `antigravity-acp` adapter, accepting the ToS risk and registry PR uncertainty."
  - client: ACP Registry
    status: partial
    issue: "agentclientprotocol/registry#414 is open for adding `antigravity-acp`; comments raise Google ToS concerns."
    workaround: "Install/launch adapter manually until registry status is resolved."
  - client: OpenAB
    status: partial
    issue: "google-antigravity/antigravity-cli#31 describes an OpenAB workaround that retargets Gemini CLI ACP to Antigravity backend endpoints."
    workaround: "Prefer official `agy --acp` if Google ships it; otherwise treat backend retargeting as a stopgap with additional risk."
recent_changes:
  - date: "2026-07-08"
    version: "Antigravity CLI 1.1.0"
    change: "Latest release; adds public mode cycling and request-review default behavior for file writes."
    impact: "Adapter clients must account for native permission review behavior because ACP permission prompts are not proxied."
  - date: "2026-06-29"
    version: "antigravity-acp 1.0.0"
    change: "Initial adapter release bridging `agy` to ACP with live streaming, history replay, session management, model/mode config, and standalone binaries."
    impact: "Creates adapter-based ACP support despite native `agy` lacking ACP."
  - date: "2026-05-20"
    version: "Antigravity CLI issue #31"
    change: "Feature request opened for native `agy --acp` stdio JSON-RPC mode."
    impact: "Confirms native ACP was absent at `agy` 1.0.0 and remains requested."
quirks:
  - "Installed `agy` 1.1.0 rejects `--acp` and `--experimental-acp`; `agy acp` is not a subcommand and falls into TUI startup, which fails without `/dev/tty`."
  - "`~/.antigravity` exists on this host but contains IDE extension data, not CLI ACP/auth artifacts. CLI state is under `~/.gemini/antigravity-cli`."
  - "The adapter is not a true tool-hosting ACP implementation: `agy` performs reads, writes, command execution, and permissions itself; the adapter translates completed or in-progress SQLite rows into ACP updates."
  - "Adapter plan mode is prompt injection, not an ACP plan capability."
  - "Adapter streams by polling SQLite, so update latency and completeness depend on Antigravity's private DB schema remaining compatible."
  - "Google FAQ/ToS concerns are material: third-party software driving Antigravity accounts may violate terms even when the adapter only spawns the official CLI."
gaps:
  - "No official Google ACP documentation or native ACP mode was found for Antigravity CLI."
  - "No initialize handshake was run against `antigravity-acp` locally because Bun/package installation was not required for this research and could trigger adapter postinstall downloads."
  - "Exact ACP SDK 1.0.0 schema release semantics were inferred from adapter package.json and source, not verified by a live wire capture."
  - "Native `agy` permission prompt behavior under `agy -p` with the adapter was not exercised to avoid interactive prompts and account-affecting actions."
  - "Image/audio media behavior through the adapter remains unknown."
changes: []
requires_claudine_update: true
reason: "Claudine would need new ACP client launch support for adapter binaries, stdio JSON-RPC lifecycle management, capability negotiation, session/update routing, auth preflight checks for `agy`, and policy gates around adapter args such as bypassPermissions."
---

# Antigravity ACP Support Research

## Overview

Antigravity CLI does not currently provide native ACP support in the primary `agy` binary. The installed local binary is `/Users/ken/.local/bin/agy` version `1.1.0`; its help lists TUI, print, prompt-interactive, conversation, model, mode, sandbox, update, plugin, models, and changelog surfaces, but no ACP flag or subcommand. Local negative probes on 2026-07-08 found:

| Probe | Result |
| --- | --- |
| `agy --acp` | Rejected with `flags provided but not defined: -acp`. |
| `agy --experimental-acp` | Rejected with `flags provided but not defined: -experimental-acp`. |
| `agy acp` | Treated like normal TUI startup and failed in this non-interactive shell because Bubble Tea could not open `/dev/tty`. |
| Official repo source scan | No `acp`, `Agent Client`, JSON-RPC, or `session/` ACP method implementation was found in the public `google-antigravity/antigravity-cli` repository clone, except unrelated text. |

ACP support is therefore **adapter-based**. The maintained adapter found for this run is [`antigravity-acp`](https://github.com/shubzkothekar/antigravity-acp), an unofficial Bun/TypeScript ACP server. It translates between:

- ACP over stdio: newline-delimited JSON-RPC handled with `@agentclientprotocol/sdk`.
- Antigravity CLI process and state: child `agy` invocations using print mode (`-p`) plus read-only polling of Antigravity's conversation SQLite databases.

The adapter is not native Google support. Its own README documents Terms of Service risk, and the ACP Registry PR discussion repeats the concern that using third-party software to drive an Antigravity login may violate Google terms. The practical classification for Claudine is still `adapter`, because a bridge process exists and is maintained, but it should be treated as policy-sensitive and not equivalent to official `agy --acp`.

Local artifact inspection found two relevant state roots:

- `/Users/ken/.antigravity`: present, but it contains IDE extension data; no ACP/auth/config evidence for `agy` was found there.
- `/Users/ken/.gemini/antigravity-cli`: active CLI state, including `settings.json`, onboarding cache, project cache, conversation SQLite databases, logs, built-in skills, keybindings, and history.

The local `settings.json` records `enableTelemetry: false`, model `Gemini 3.1 Pro (High)`, and trusted workspaces under the rusty-biscuit worktrees. Local onboarding cache says consumer onboarding is complete. No ACP-specific auth artifact was found.

## Launching ACP

Native launch is unavailable:

```bash
agy --acp
```

does not launch ACP on `agy` 1.1.0. It exits with an unknown flag error.

The adapter launch is:

```bash
agy-acp
```

or from source:

```bash
bun run index.ts
```

Both adapter launch modes speak ACP over stdio using newline-delimited JSON-RPC. The adapter's `runAcp()` wires `Bun.stdin.stream()` and a Bun stdout writer through the SDK `ndJsonStream`, then registers agent-side request handlers.

The adapter launches the provider binary itself. It resolves `agy` in this order:

1. An `agy` binary next to the adapter executable or in source `bin/`.
2. `$AGY_BIN`.
3. `agy` on `$PATH`.
4. Auto-download from `google-antigravity/antigravity-cli` unless disabled.

For each prompt turn, the adapter builds a command like:

```bash
agy --add-dir <cwd> [--add-dir <extra>] [AGY_EXTRA_ARGS...] [--conversation <id>] [--model <model>] [-p <prompt>]
```

If the adapter mode is `bypassPermissions`, it also passes:

```bash
--dangerously-skip-permissions
```

The child process runs with stdin ignored, stdout ignored, and stderr piped. Streaming does not come from `agy` stdout; the adapter polls the conversation database while the child runs.

## Protocol and Capabilities

The adapter source returns:

```json
{
  "protocolVersion": 1,
  "agentInfo": { "name": "Antigravity", "version": "1.0.0" }
}
```

The package depends on `@agentclientprotocol/sdk` `^1.0.0`. This verifies ACP protocol major version 1 from source, not from a live handshake.

Capability support:

| Capability | Support | Evidence and Notes |
| --- | --- | --- |
| `initialize` | Supported | Returns protocol version 1, agent info, capabilities, and auth method metadata. |
| `authenticate` | Partial | Validates method id and returns success; actual OAuth is owned by `agy`. |
| `logout` | Partial | No-op in adapter; `agy` owns credential state. |
| `session/new` | Supported | Creates adapter session binding with `cwd` and `additionalDirectories`. |
| `session/load` | Supported | Replays conversation rows from SQLite into ACP updates. |
| `session/resume` | Supported | Re-attaches an ACP client and re-sends command/config notifications. |
| `session/list` | Supported | Lists persisted adapter sessions from `~/.agy-acp/sessions.json`. |
| `session/delete` | Supported | Deletes adapter session binding. |
| `session/close` | Supported | Cancels any child and evicts in-memory session. |
| `session/prompt` | Supported | Spawns `agy -p`, polls DB rows, and returns `end_turn` or `cancelled`. |
| `session/cancel` | Supported | Sends SIGINT on Unix-like systems; kills on Windows. |
| `session/set_config_option` | Supported | Sets model or mode in adapter session state. |
| Streaming | Supported | Emits `session/update` notifications. |
| Permissions | Partial | Native `agy` handles permissions; adapter only decodes permission rows into tool content. |
| Filesystem reverse requests | Unsupported | No ACP `fs/*` requests are sent. |
| Terminal reverse requests | Unsupported | No ACP `terminal/*` requests are sent. |
| MCP | Partial | Native `agy` has MCP features, but adapter's `resources/list`, `prompts/list`, and `tools/list` return empty arrays. |
| Media | Unknown/partial | Adapter flattens text blocks, resource links, and embedded text resources. Image/audio behavior was not verified. |
| Plans | Partial | Mode option plus prompt injection; plan artifacts are rendered as tool calls/text. |
| Extensions | Partial | Implements newer session management methods and compatibility list methods, but no documented vendor extension namespace. |

The adapter's `promptCapabilities` advertise `embeddedContext: true`. It does not advertise client-side filesystem or terminal delegation, because it is not asking the ACP client to perform those operations.

## Reverse Requests

The only client-bound method the adapter clearly uses is:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "<adapter-session-id>",
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": { "type": "text", "text": "<delta>" }
    }
  }
}
```

The adapter source includes an `AcpClient.requestPermission()` wrapper around `session/request_permission`, but the prompt adapter path does not call it. Permission records from Antigravity's SQLite `permissions` column are decoded after the fact and appended to `tool_call` content as prose such as permission category and target.

Required before the adapter is usable:

- `session/update` notification handling.
- Prompt response handling for `session/prompt`.
- Capability/config update handling if the UI wants model and mode selectors.

Capability-gated or currently unnecessary:

- `session/request_permission`: should exist as a defensive stub in a generic Claudine ACP client, but this adapter does not rely on it.
- `fs/read_text_file` and `fs/write_text_file`: unsupported.
- `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`: unsupported.

Example tool update shape emitted by adapter source:

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "a4u6fsq8",
  "title": "echo \"Hello, World!\"",
  "kind": "execute",
  "status": "completed",
  "content": [
    {
      "type": "content",
      "content": { "type": "text", "text": "```\necho \"Hello, World!\"\n```" }
    }
  ],
  "locations": [{ "path": "/Users/user/Desktop" }]
}
```

The payload is an adapter-rendered representation of an `agy` tool row, not a client request to execute the command.

## Permissions, Filesystem, and Terminal

The client should treat this adapter as an agent that already has local process access through `agy`. ACP reverse requests are not the enforcement point.

Permission handling:

- Do not expect `session/request_permission`.
- Gate the adapter launch itself through Claudine policy: command path, env, cwd, added directories, and whether `bypassPermissions` is permitted.
- Avoid enabling adapter `bypassPermissions` unless the workspace and user policy allow native `agy --dangerously-skip-permissions`.
- If running in standard mode, expect native `agy` permission behavior. In a headless `-p` child, native prompts may block or fail because the ACP client cannot answer them through ACP.

Filesystem handling:

- `session/new.cwd` becomes the child process cwd and `--add-dir <cwd>`.
- `additionalDirectories` become repeated `--add-dir` arguments.
- The adapter does not enforce sandboxing; it trusts `agy` workspace and permission logic.
- Tool locations use absolute filesystem paths. Display titles may be project-relative when a path is under `cwd`.
- File line numbers are 1-based where Antigravity records line ranges.
- Local CLI conversations are under `~/.gemini/antigravity-cli/conversations/*.db`; the adapter default follows `os.homedir()`, so Claudine launch environments with a rewritten `HOME` must set `AGY_CONVERSATIONS_DIR` if they need the real user's Antigravity state.

Terminal handling:

- The adapter does not expose ACP terminal handles.
- Command execution is performed by `agy` as part of its own tool runtime.
- Command output is later rendered as `tool_call` updates, often as code blocks or error blocks.
- `session/cancel` is process-level cancellation of the `agy` child, not terminal-handle cancellation.

Process lifecycle responsibilities for Claudine:

- Spawn the adapter as the ACP agent process.
- Keep stdout exclusively reserved for JSON-RPC.
- Read stderr for adapter and child diagnostics.
- Kill the adapter process tree on session teardown.
- On cancellation, send `session/cancel` first; if the adapter does not exit when expected, terminate the adapter child process under Claudine's normal process policy.

## Streaming and UI Integration

The adapter emits these `session/update` variants:

| Update | Meaning | UI Routing |
| --- | --- | --- |
| `agent_message_chunk` | Assistant text delta during live polling or buffered text during replay. | Append to the active assistant message stream. |
| `user_message_chunk` | Replayed user turns from history. | Render as historical user messages during `session/load`. |
| `tool_call` | Read, edit, search, fetch, execute, subagent, question, task, permission, and error activity. | Route to tool timeline; use `kind`, `status`, `locations`, `content`, `rawInput`, and `rawOutput`. |
| `session_info_update` | Conversation title changes. | Update session title/sidebar metadata. |
| `available_commands_update` | Slash-command-like commands (`goal`, `schedule`, `grill-me`, `teamwork-preview`, `learn`). | Populate command palette if supported. |
| `config_option_update` | Model and mode selector updates. | Update UI config controls. |

There is no first-class ACP `plan` update. The adapter uses:

- `config_option_update` for the `plan` mode.
- Prompt injection to tell `agy` not to mutate files.
- `tool_call kind=think` for title/thought blocks.
- `tool_call kind=edit` or text content for generated `plan.md` artifacts.

The UI event loop should treat ACP reading as an async stream:

1. A JSON-RPC task reads adapter stdout and decodes responses/notifications.
2. Responses resolve request futures.
3. `session/update` notifications become internal Claudine events.
4. Tool and text events are sent to terminal/browser/desktop renderers without blocking the protocol reader.

## Authentication and Setup

Antigravity CLI authenticates outside ACP. The official README says the CLI uses the system keyring and falls back to Google Sign-In when no active session exists. Local/desktop use opens a browser; remote/SSH use prints an authorization URL. Sign-out is a CLI slash command (`/logout`) in the interactive surface.

Before headless adapter use:

- Install `agy`.
- Run `agy` interactively at least once.
- Complete Google OAuth and onboarding.
- Trust the intended workspace.
- Verify `agy -p "..."` can run in that workspace without interactive setup prompts.

Local files inspected:

- `/Users/ken/.gemini/antigravity-cli/settings.json`: telemetry disabled, selected model, trusted workspaces.
- `/Users/ken/.gemini/antigravity-cli/cache/onboarding.json`: consumer onboarding complete.
- `/Users/ken/.gemini/antigravity-cli/cache/last_conversations.json`: workspace-to-conversation mapping.
- `/Users/ken/.gemini/antigravity-cli/conversations/*.db`: Antigravity conversation databases used by the adapter model.
- `/Users/ken/.antigravity/extensions`: IDE extension data, not CLI ACP state.

Environment variables:

| Variable | Owner | Effect |
| --- | --- | --- |
| `AGY_BIN` | Adapter | Explicit `agy` path; skips auto-download. |
| `AGY_SKIP_DOWNLOAD` | Adapter | Set to `1` to skip adapter auto-download. |
| `AGY_EXTRA_ARGS` | Adapter | Whitespace-split args forwarded to each `agy` child. |
| `AGY_CONVERSATIONS_DIR` | Adapter | Overrides SQLite conversation directory. |
| `AGY_CLI_CMD_OUTPUT_PERCENTAGE` | Native `agy` | TUI command-output height percentage. |
| `AGY_CLI_HIDE_ACCOUNT_INFO` | Native `agy` | Hide account info in the TUI header. |
| `AGY_CLI_DISABLE_LATEX` | Native `agy` | Disable LaTeX rendering. |

## Compatibility, Quirks, and Workarounds

Known issues and compatibility notes:

| Client/Surface | Status | Issue | Workaround |
| --- | --- | --- | --- |
| Zed | Partial | Zed discussion #57221 says Antigravity needs ACP support or registry-compatible adapter support. Zed maintainers prefer ACP over bespoke SDK integration. | Use terminal threads or manually configure the community adapter if policy allows. |
| ACP Registry | Partial | Registry PR #414 for `antigravity-acp` remains open and includes ToS objections. | Manual adapter launch until registry status is settled. |
| OpenAB | Partial | Antigravity CLI issue #31 describes an OpenAB workaround retargeting Gemini CLI ACP to Antigravity backend endpoints. | Treat as stopgap; prefer native `agy --acp` if Google ships it. |
| Generic ACP clients | Partial | Adapter does not implement ACP fs/terminal reverse requests or true permission prompts. | Build UI around `session/update` and host-policy-gate the adapter process. |

Quirks:

- The adapter depends on a private Antigravity SQLite schema. Source comments say permission/error shapes were reverse-engineered from real `agy` DBs.
- Streaming is polling-based, not provider stdout streaming.
- The adapter ignores `agy` stdout and only captures stderr for diagnostics.
- `HOME` matters. The adapter defaults to `os.homedir()/.gemini/antigravity-cli/conversations`; Claudine's wrapped HOME/shadow HOME patterns can accidentally point it at an empty state directory.
- Native `agy` has a `--sandbox` flag, but the adapter does not expose a first-class ACP sandbox capability. Claudine can pass it through `AGY_EXTRA_ARGS` or wrapper configuration if desired.
- `agy` 1.1.0 request-review mode pauses before file writes in the native TUI, but that approval loop is not bridged to ACP.
- Google ToS/FAQ concerns are not theoretical: the adapter README and related GitHub issues document account suspension risk for third-party access patterns.

## Recent Changes

| Date | Component | Change | ACP Impact |
| --- | --- | --- | --- |
| 2026-07-08 | Antigravity CLI 1.1.0 | Latest release. Adds public execution mode cycling and request-review default behavior for file writes. | Native ACP still absent in local probe; adapter clients must account for native permission behavior. |
| 2026-07-01 | `antigravity-acp` issue #3 / ACP Registry PR #414 | ToS risk was raised and documented. | Claudine should treat adapter launch as policy-sensitive. |
| 2026-06-29 | `antigravity-acp` 1.0.0 | Initial adapter release with stdio ACP, live streaming, replay, sessions, model/mode config, and standalone binaries. | Establishes adapter-based support. |
| 2026-05-20 | `google-antigravity/antigravity-cli` issue #31 | Feature request asks Google to add `agy --acp`. | Confirms native support gap and desired method surface. |

## Rust Client Example

For `antigravity-acp`, use the official `agent-client-protocol` Rust crate when possible. The adapter is already a normal ACP stdio server, so a Rust client does not need to parse Antigravity SQLite rows or proprietary output.

Sketch:

```rust
use std::process::Stdio;

use agent_client_protocol::{Client, ClientSideConnection};
use tokio::process::Command;

async fn launch_antigravity_acp() -> anyhow::Result<()> {
    let mut child = Command::new("agy-acp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("AGY_BIN", "/Users/ken/.local/bin/agy")
        .spawn()?;

    let stdin = child.stdin.take().expect("adapter stdin");
    let stdout = child.stdout.take().expect("adapter stdout");

    // Exact constructor names vary by crate release. The intended shape is:
    // create a ClientSideConnection over stdout/stdin, initialize, create a
    // session, then send session/prompt requests while handling Client callbacks.
    let _transport = (stdout, stdin);

    Ok(())
}
```

If the exact SDK connection helpers do not match the crate version in Claudine, use `agent-client-protocol-schema` for typed messages plus a small newline-delimited JSON-RPC stdio transport. Do not drive native `agy` directly as ACP; it has no ACP mode.

## Rust Reverse Request Handling

A robust Rust ACP client for this adapter should implement the client role even for methods the adapter does not currently call:

```rust
enum UiEvent {
    AssistantText { session_id: String, text: String },
    ToolCall { session_id: String, title: String, kind: String },
    SessionTitle { session_id: String, title: Option<String> },
    ConfigChanged { session_id: String },
}

async fn handle_session_update(update: serde_json::Value, tx: tokio::sync::mpsc::Sender<UiEvent>) {
    let session_id = update["sessionId"].as_str().unwrap_or_default().to_owned();
    let body = &update["update"];

    match body["sessionUpdate"].as_str() {
        Some("agent_message_chunk") => {
            let text = body["content"]["text"].as_str().unwrap_or_default().to_owned();
            let _ = tx.send(UiEvent::AssistantText { session_id, text }).await;
        }
        Some("tool_call") => {
            let title = body["title"].as_str().unwrap_or("Tool").to_owned();
            let kind = body["kind"].as_str().unwrap_or("other").to_owned();
            let _ = tx.send(UiEvent::ToolCall { session_id, title, kind }).await;
        }
        Some("session_info_update") => {
            let title = body["title"].as_str().map(str::to_owned);
            let _ = tx.send(UiEvent::SessionTitle { session_id, title }).await;
        }
        Some("config_option_update") | Some("available_commands_update") => {
            let _ = tx.send(UiEvent::ConfigChanged { session_id }).await;
        }
        _ => {}
    }
}
```

For `session/request_permission`, `fs/*`, and `terminal/*`, return an explicit unsupported JSON-RPC error unless future adapter versions advertise and call them. That keeps the client honest and prevents accidental host access outside Claudine policy.

## Rust Host Command Handling

With this adapter, ACP host command handling is mostly launch policy, not reverse-request execution:

1. Decide whether `agy-acp` may run for the workspace.
2. Resolve the adapter binary and `AGY_BIN`.
3. Set `cwd` and `AGY_CONVERSATIONS_DIR` deliberately.
4. Decide whether `AGY_EXTRA_ARGS` may include `--sandbox`.
5. Reject or require explicit user policy for `bypassPermissions`.
6. Kill the adapter process tree on timeout, cancellation, or UI shutdown.

If a future Antigravity-native ACP mode adds `terminal/create`, then Claudine should run commands through its normal host terminal policy:

- Normalize cwd against allowed workspace roots.
- Reject commands outside policy before spawning.
- Stream stdout/stderr back as ACP terminal output responses.
- Track terminal ids and release/kill them on cancellation.
- Use platform-specific process groups or job objects for macOS/Linux/Windows parity.

For current `antigravity-acp`, command tool calls should be displayed as telemetry only. The command has already been delegated to `agy`.

## Rust Desktop Streaming Bridge

For a Tauri or iced app, keep ACP protocol IO off the UI thread:

```rust
#[derive(Debug, Clone)]
enum DesktopEvent {
    TextDelta { session_id: String, text: String },
    ToolUpdate { session_id: String, title: String, status: String },
    Title { session_id: String, title: Option<String> },
    Error { message: String },
}

fn spawn_bridge(mut rx: tokio::sync::mpsc::Receiver<DesktopEvent>) {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            // Tauri: app_handle.emit("acp://event", event)
            // iced: forward into a Subscription stream
            let _ = event;
        }
    });
}
```

Backpressure policy matters because `agent_message_chunk` can arrive frequently during polling. Prefer a bounded channel and coalesce text deltas per session before repainting. Tool updates should remain ordered relative to text updates from the protocol task, because the adapter's translator emits ordered rows from the SQLite step index.

## Claudine Integration Notes

Adding Antigravity ACP support to Claudine would require adapter-aware ACP infrastructure:

- Launch detection: native `agy` should be recorded as no native ACP mode until a future version proves otherwise. Detect `agy-acp` or configured adapter binaries separately.
- Auth preflight: verify `agy --version`, `agy -p` readiness, onboarding/trusted workspace state, and conversation directory path. Avoid triggering interactive OAuth in a headless run.
- Capability negotiation: initialize the ACP adapter and record protocol version 1 plus the adapter's partial capabilities.
- Reverse-request routing: implement `session/update` as required; implement unsupported stubs for permission, fs, and terminal requests.
- Streaming bridge: normalize `agent_message_chunk`, `tool_call`, `session_info_update`, command updates, and config updates into Claudine UI events.
- Policy enforcement: gate `cwd`, `additionalDirectories`, `AGY_BIN`, `AGY_EXTRA_ARGS`, `AGY_CONVERSATIONS_DIR`, `--sandbox`, and `bypassPermissions`.
- Process lifecycle: manage adapter stdio, stderr diagnostics, cancellation, and child cleanup across macOS, Linux, and Windows.
- Terms and safety: surface that the adapter is unofficial and may violate Google terms when used with an Antigravity OAuth account.

The strategic design point is that Claudine should treat this as an ACP adapter profile, not an Antigravity-native ACP profile. If Google later ships `agy --acp`, that should be a separate launch mode with its own verified capabilities.

## Changelog

[]

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI releases](https://github.com/google-antigravity/antigravity-cli/releases)
- [Feature request: add ACP stdio JSON-RPC mode to Antigravity CLI](https://github.com/google-antigravity/antigravity-cli/issues/31)
- [Antigravity ACP adapter repository](https://github.com/shubzkothekar/antigravity-acp)
- [Antigravity ACP adapter README](https://github.com/shubzkothekar/antigravity-acp/blob/main/README.md)
- [Antigravity ACP adapter changelog](https://github.com/shubzkothekar/antigravity-acp/blob/main/CHANGELOG.md)
- [Antigravity ACP adapter package metadata](https://github.com/shubzkothekar/antigravity-acp/blob/main/package.json)
- [Document Google ToS risk in antigravity-acp](https://github.com/shubzkothekar/antigravity-acp/issues/3)
- [ACP Registry PR for antigravity-acp](https://github.com/agentclientprotocol/registry/pull/414)
- [Zed discussion: Add Antigravity CLI to ACP Registry](https://github.com/zed-industries/zed/discussions/57221)
- [Google AI Developers Forum: lack of ACP support in Antigravity CLI](https://discuss.ai.google.dev/t/ux-issues-and-lack-of-acp-support-in-antigravity-cli/169445)
- Local inspection: `/Users/ken/.local/bin/agy --version`, `/Users/ken/.local/bin/agy --help`, `/Users/ken/.local/bin/agy --acp`, `/Users/ken/.local/bin/agy --experimental-acp`, `/Users/ken/.local/bin/agy acp`
- Local inspection: `/Users/ken/.antigravity`, `/Users/ken/.gemini/antigravity-cli`, `/Applications/Antigravity.app`, `/Applications/Antigravity IDE.app`
- Local source inspection: `/tmp/antigravity-acp-research` cloned from `https://github.com/shubzkothekar/antigravity-acp`
- Local source inspection: `/tmp/antigravity-cli-research` cloned from `https://github.com/google-antigravity/antigravity-cli`
