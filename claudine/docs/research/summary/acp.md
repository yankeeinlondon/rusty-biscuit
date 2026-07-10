---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  The files found in @claudine/docs/research/acp/*.md all represent research done on how the ACP protocol is supported on the Agentic CLI providers that Claudine supports.

  Your task is to create a summary document that describes:

  1. What is ACP?
  2. Why is it important?
  3. How does an Agentic CLI leverage ACP?
  4. What is the current level of support for ACP across the supported providers in Claudine?
  5. How could Claudine benefit from using the ACP functionality provided by these Agentic CLI programs? What would be the downsides of using ACP versus simply wrapping the execution of these Agentic CLI's?

  Note: `json-rpc.md` in the research directory is protocol-level background rather than a provider document.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the protocol background and the first three provider research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/acp.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/acp.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: f53847b4265478cf-15a13f8956ef3f1c
last_updated: 2026-07-03
---
# ACP Support in Claudine's Agentic CLI Providers

## What is ACP?

ACP, the Agent Client Protocol, is a bidirectional JSON-RPC 2.0 protocol for connecting an agentic coding tool to a host client such as an editor, IDE, terminal UI, desktop app, or orchestration layer.

The common transport is newline-delimited JSON over stdio. A client sends requests such as `initialize`, `authenticate`, `session/new`, `session/load`, `session/prompt`, and `session/cancel`; the agent returns JSON-RPC results or errors and streams progress through notifications such as `session/update`.

ACP is not merely a prompt-in, text-out protocol. It models the lifecycle of a coding-agent session:

- capability negotiation through `initialize`
- authentication through `authenticate` and advertised auth methods
- session creation, loading, cancellation, and sometimes listing, resuming, closing, or forking
- streaming assistant text, reasoning, tool calls, plans, mode changes, command lists, and usage
- reverse requests where the agent asks the client to approve actions, read or write files, or run terminal commands
- optional MCP server injection at session startup
- provider-specific extension data through `_meta`, extension methods, and extension notifications

ACP is easiest to understand as the agentic-coding equivalent of LSP. LSP standardizes editor-to-language-tool interactions; ACP standardizes client-to-coding-agent interactions.

## Why ACP matters

ACP matters because the agentic CLI ecosystem is converging on similar capabilities but exposing them through incompatible native interfaces. Every provider can run prompts, stream output, call tools, request approval, edit files, and manage sessions, but each provider's raw CLI flags, logs, JSON formats, hook systems, config files, and permission semantics differ.

ACP provides a common shape for the parts Claudine most wants to normalize:

- session lifecycle
- streaming text, reasoning, and tool progress
- permission prompts
- filesystem and terminal delegation where actually implemented
- MCP server handoff
- model and mode selection
- authentication preflight
- cancellation behavior
- UI-oriented updates such as plans, usage, modes, config options, and available commands

The protocol also separates agent capability from host authority when reverse requests are used. If an agent asks the client to read a file, write a file, or run a command, the host can enforce path policy, command policy, approval UX, audit logging, sandboxing, timeouts, and process cleanup before responding.

That distinction is critical: ACP support does not automatically mean filesystem or terminal work is host-mediated. Some providers use ACP only as a session and streaming envelope while still executing file and shell tools inside the agent process. In those cases ACP improves observability and control flow, but it does not by itself give Claudine a filesystem or terminal security boundary.

## How an Agentic CLI leverages ACP

There are two broad ACP support models.

The first model is native ACP support. The provider's own binary exposes an ACP mode, usually over stdio. Examples in the research include `gemini --acp`, `goose acp`, `kilo acp`, `kimi acp`, `opencode acp`, and `qwen --acp`. The CLI process itself speaks JSON-RPC and maps its internal agent runtime to ACP methods and notifications.

The second model is adapter-based support. The provider's main CLI has no native ACP entry point, but a separate adapter process speaks ACP to the client and translates to the provider's internal SDK, app server, or private protocol. Claude Code, Codex, and Pi currently fall into this category.

A typical ACP run looks like this:

1. The client starts the agent or adapter as a subprocess with stdin/stdout piped.
2. The client sends `initialize` with its supported capabilities, such as filesystem, terminal, auth, and metadata extensions.
3. The agent replies with its protocol version, implementation identity, auth methods, and session capabilities.
4. The client authenticates if required, often through `authenticate` or an advertised terminal-auth flow.
5. The client creates or loads a session with `session/new` or `session/load`, passing an absolute `cwd` and optional `mcpServers`.
6. The client sends a user turn with `session/prompt`.
7. The agent streams `session/update` notifications for assistant text, reasoning, tool calls, tool updates, plans, commands, modes, config options, and usage.
8. When the agent needs host approval or host services, it may send reverse requests such as `session/request_permission`, `fs/read_text_file`, `fs/write_text_file`, or `terminal/create`.
9. The client responds to those reverse requests, applying its own policy and UI decisions.
10. The client can cancel the active turn with `session/cancel`, and newer schema support may also cancel in-flight requests with `$/cancel_request`.

For Claudine, the highest-value part of this flow is not the JSON-RPC framing itself. It is the fact that agent state, tool state, permissions, and streaming updates become typed protocol events instead of provider-specific stdout/stderr parsing.

## Current provider support

The ACP research shows broad but uneven support across Claudine's current provider set and adjacent researched providers.

| Provider    | ACP support                    | Launch path                                                                                         | Practical support level                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
|-------------|-------------------------------:|-----------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Claude Code | Adapter                        | `npx -y @agentclientprotocol/claude-agent-acp@latest`                                               | Strong session, streaming, auth, permissions, MCP, modes, media, plans, and extensions through an adapter. No native `claude` ACP mode. Current adapter does not delegate filesystem or terminal through ACP reverse requests; Claude's built-in tools perform those actions inside the agent runtime.                                                                                                                                                                         |
| Codex       | Adapter                        | `npx -y @agentclientprotocol/codex-acp`                                                             | Strong session, streaming, auth, modes, permissions, plans, media, and partial MCP. No native `codex` ACP mode. Files and commands are handled inside the Codex App Server, not through ACP filesystem or terminal reverse requests.                                                                                                                                                                                                                                           |
| Gemini CLI  | Native                         | `gemini --acp`                                                                                      | Strong native implementation. Supports session lifecycle, auth, modes, models, streaming, permissions, filesystem reverse requests, terminal reverse requests, MCP, media, plans, command updates, usage, and extensions. `--experimental-acp` remains as a deprecated alias.                                                                                                                                                                                                  |
| Goose       | Native                         | `goose acp`; also `goose serve` for HTTP/WebSocket ACP                                              | One of the strongest ACP implementations. Supports native stdio and HTTP/WebSocket transports, permissions, filesystem and terminal reverse requests, MCP, modes, custom Goose extensions, session persistence, and request-level cancellation. Plan updates are not emitted as the standard ACP `Plan` variant.                                                                                                                                                               |
| Kilo Code   | Native                         | `kilo acp --cwd /path/to/project`; `npx @kilocode/cli acp`                                          | Native ACP support in the primary Kilo CLI package. Supports session lifecycle, auth readiness, session load/list/resume/close/fork, streaming, permissions, modes/config options, MCP HTTP/SSE registration, image inputs, and usage updates. `session/cancel` is unsupported. No ACP terminal reverse requests are used. Ordinary file reads are internal; only proposed edit approval may call `fs/write_text_file`.                                                        |
| Kimi Code   | Native                         | `kimi acp`                                                                                          | Native ACP support in the TypeScript `kimi-code` CLI. Supports ACP v1, session lifecycle, auth, streaming, permissions, modes, MCP, media, and plans. Terminal reverse requests are not implemented. Filesystem delegation is version-sensitive: current docs describe `fs/read_text_file` and `fs/write_text_file`, but the inspected v0.14.0 binary did not advertise `fsCapabilities` and kept file work inside its local tool layer.                                       |
| OpenCode    | Native                         | `opencode acp`                                                                                      | Native implementation with strong session, streaming, permissions, modes, MCP, media, and config-option support. It does not issue terminal reverse requests and does not delegate generic file reads. It may use `fs/write_text_file` only in a narrow edit approval path.                                                                                                                                                                                                    |
| Pi          | Adapter                        | `pi-acp`; `npx -y @victor-software-house/pi-acp`; source-built `node /path/to/pi-acp/dist/index.js` | Adapter-based support. The primary `pi` CLI has Pi-specific RPC mode, not native ACP. The registry adapter provides session, streaming, auth, cancel, and basic session mapping. The Victor fork is more capable, with thought streaming, config/mode updates, optional `fs/read_text_file`, optional terminal delegation, and extension methods. No ACP permission bridge, no `fs/write_text_file`, no plan updates, and MCP params are accepted but not wired through to Pi. |
| Qwen Code   | Native                         | `qwen --acp`                                                                                        | Strong native implementation. Supports session lifecycle, auth, streaming, permissions, filesystem reverse requests, MCP, plans, media, and modes. Terminal methods exist at schema level, but normal shell execution remains inside Qwen's built-in shell tool rather than through ACP terminal reverse requests.                                                                                                                                                             |
| Roo Code    | Unknown from this research set | No ACP research document present                                                                    | Claudine supports Roo in its provider enum, but this ACP research set does not include a Roo provider document. Treat ACP support as unassessed here rather than unsupported.                                                                                                                                                                                                                                                                                                  |

The most important cross-provider pattern is this: ACP support does not imply ACP-controlled filesystem and terminal execution.

There are four practical tiers:

1. Full or near-full ACP delegation: Gemini and Goose can route filesystem and terminal work through client reverse requests when the client advertises those capabilities.
2. Partial delegation: Qwen supports filesystem delegation but usually keeps terminal execution inside the agent; Kimi filesystem delegation depends on version; Kilo and OpenCode have narrow optional write paths but keep most file and shell work internal.
3. Capability-gated adapter delegation: Pi's Victor adapter can delegate reads and terminal execution when the client advertises those capabilities, but Pi has no ACP permission bridge, writes remain local, MCP is not wired through, and the terminal bridge currently assumes `/bin/sh`.
4. ACP as adapter/session envelope: Claude and Codex provide useful ACP session, streaming, auth, and permission surfaces, but the current adapters do not hand filesystem or terminal execution to the client.

## How Claudine could benefit from ACP

Claudine today wraps provider CLIs and normalizes their lifecycle events, streaming formats, hooks, system prompt behavior, MCP injection, permissions, reporting, and composition workflows. ACP could become another execution backend for providers that support it.

The main benefits would be:

- Typed session lifecycle instead of provider-specific process scraping.
- More uniform streaming events across providers: assistant chunks, reasoning chunks, tool calls, tool updates, usage, plans, commands, and mode changes.
- First-class permission interception through `session/request_permission` where providers emit it.
- Cleaner cancellation through `session/cancel` and, where supported, request-level cancellation.
- Capability negotiation at runtime rather than hard-coded provider assumptions.
- Potentially cleaner MCP session injection through `session/new` `mcpServers`.
- Better UI and reporting foundations because tool calls, plans, usage, commands, and config changes arrive as structured events.
- A path toward desktop or TUI clients where Claudine acts as the ACP client and routes updates through its existing normalized event model.
- A stronger host-policy boundary for providers that actually delegate filesystem and terminal operations through ACP reverse requests.

ACP could fit Claudine particularly well in these areas:

- Provider wrappers: add an ACP launch mode beside the existing native CLI wrapping mode.
- Streaming normalization: map `session/update` variants into Claudine's existing semantic stream model.
- Permission engine: map `session/request_permission` into Claudine's `PolicyEngine` and protect-service checks.
- Filesystem policy: when `fs/read_text_file` and `fs/write_text_file` reverse requests appear, enforce project-root and allow/deny policy before responding.
- Terminal policy: when `terminal/*` reverse requests appear, route command creation through Claudine's shell audit, timeout, cancellation, and process-cleanup infrastructure.
- MCP mode: prefer ACP `mcpServers` injection where reliable instead of provider-specific shadow config or export flows.
- Reporting: persist structured ACP events directly rather than reverse-engineering them from logs.
- Session handling: use ACP `sessionId` values as durable correlation keys for concurrent sessions, resumed sessions, and reporting.

A few provider-specific examples show why this should be capability-gated rather than provider-name-gated.

Gemini and Goose are the clearest examples of ACP improving host control. If Claudine advertises filesystem and terminal capabilities to those agents, Claudine can become the authority that reads files, writes files, runs commands, enforces path policy, applies timeouts, and cleans up child processes.

Kimi is a useful native-provider caution. It supports `session/request_permission`, plan mode, `plan` updates, MCP injection, and strong session streaming, but terminal reverse requests are absent and filesystem delegation depends on the running version. Claudine should handle `fs/*` for forward compatibility while only treating it as a security boundary after `initialize` proves the binary advertises the capability.

Kilo is similar. Its ACP endpoint is native and useful for streaming, permission prompts, MCP registration, config options, image inputs, and usage updates. But Kilo reads files through its own local services, runs shell commands through its native tool path, and does not support `session/cancel`. Claudine could integrate Kilo ACP for typed UI and policy decisions, but hard host isolation would still need Kilo configuration, permission responses, and an outer process sandbox.

Pi shows the adapter version problem. Claudine could run Pi through `pi-acp` for structured streaming, session handling, usage updates, config/mode updates, and optional host-mediated reads or terminal commands. But it should not assume Pi sessions receive ACP permission prompts or MCP servers, and it should treat the registry adapter and Victor fork as materially different integrations.

## Downsides versus wrapping provider CLI execution

ACP is not a free replacement for Claudine's existing wrappers.

The first downside is uneven provider semantics. Claude and Codex have useful ACP adapters, but they do not expose filesystem or terminal reverse requests in the way a policy host might expect. Kilo, OpenCode, and Qwen support ACP natively but still run shell commands internally in normal use. Kimi supports native ACP and permissions, but terminal reverse requests are absent and filesystem delegation depends on the running version. Pi can optionally delegate reads and terminal execution through one adapter fork, but does not delegate writes, does not request ACP permissions, and does not wire MCP servers into Pi sessions. If Claudine assumed ACP meant host-mediated execution, it would overestimate its control.

The second downside is adapter dependency. Claude, Codex, and Pi rely on external adapter packages, often launched through `npx`. That adds version drift, package-manager dependency, startup overhead, network sensitivity, and another layer that can break independently of the provider CLI. Pi currently has at least two adapter lines with different capabilities: the ACP registry points at `svkozak/pi-acp`, while `@victor-software-house/pi-acp` is more featureful for filesystem and terminal delegation.

The third downside is protocol churn. The research references ACP protocol version 1, schema 1.1.0, SDK 1.0.x, older TypeScript SDK 0.x dependencies, deprecated flags, renamed packages, capability additions, and provider behavior changes within weeks of each other. Claudine would need explicit version detection, capability negotiation, and defensive fallbacks.

The fourth downside is loss of provider-native affordances. Claudine's current wrappers can pass provider-specific flags, config overlays, environment variables, stream formats, system prompt options, and permission modes directly. ACP tends to expose a common subset plus `_meta` extensions. Some provider-specific controls may be harder to reach, undocumented, or only available through extension fields.

The fifth downside is security ambiguity. Wrapping a CLI lets Claudine enforce policy at the process boundary it controls: launch args, env, cwd, sandbox mode, timeout, stdout/stderr parsing, and child termination. ACP can improve policy only when the agent delegates actions to the client. If the agent performs tool execution internally, Claudine still needs outer sandboxing and provider-specific permission configuration.

The sixth downside is implementation cost. A serious ACP client in Claudine would need to implement:

- JSON-RPC request/response correlation
- notification routing
- reverse request handlers
- permission UI and non-interactive policy decisions
- filesystem and terminal service implementations
- cancellation propagation and provider-specific cancellation fallback
- adapter and native ACP launch management
- schema-version compatibility
- provider-specific `_meta` handling
- robust stderr/stdout separation
- session cleanup and child-process cleanup across macOS, Windows, and Linux
- compatibility behavior for providers that advertise capabilities but only use them in narrow paths

The existing wrapper model remains simpler for one-shot composition and non-interactive execution. It also preserves provider-native behavior and avoids adopting a fast-moving protocol as the only path.

## Recommended Claudine position

Claudine should treat ACP as an additional provider execution strategy, not as an immediate replacement for wrapping provider CLIs.

The best near-term design is a hybrid model:

- Keep existing CLI wrappers as the stable baseline.
- Add ACP support provider-by-provider where it offers concrete advantages.
- Start with native, well-structured implementations: Gemini, Goose, Qwen, OpenCode, Kimi, and Kilo.
- Treat Claude, Codex, and Pi as adapter-backed integrations with useful streaming and session data, but provider-specific security and capability limits.
- For Gemini, prefer `gemini --acp`; treat `--experimental-acp` as deprecated compatibility only and avoid SDK presets that still shell through `npx` with the deprecated flag.
- For Kimi, negotiate ACP v1, handle `session/request_permission`, map `plan` updates and plan mode, pass `mcpServers: []` explicitly, implement filesystem reverse handlers defensively, and mark terminal reverse requests unsupported.
- For Kilo, implement `session/request_permission` before treating the integration as usable, treat `fs/write_text_file` as optional and policy-gated, and do not rely on ACP terminal methods or `session/cancel`.
- For Pi, prefer `pi-acp` only as an adapter launch target; do not treat `pi --mode rpc` as ACP, and evaluate the registry adapter and Victor fork separately because their reverse-request behavior differs.
- Capability-gate every reverse-request handler. Do not assume a provider will use `fs/*`, `terminal/*`, MCP, or `session/request_permission` just because ACP defines them.
- Route ACP permissions into Claudine's existing permission and protect layers when providers emit them.
- Preserve provider-specific wrapper paths for system prompt handling, MCP setup, auth setup, provider-native permissions, and fallback execution.
- Record ACP event streams into Claudine's existing reporting model so ACP can improve observability even before it replaces any execution path.
- Be conservative on Windows for adapter terminal delegation, especially Pi's Victor adapter while it launches `/bin/sh -c`.

The strategic value of ACP for Claudine is strongest when Claudine acts as a real host client: owning approval decisions, policy enforcement, event normalization, and UI/reporting state. The value is weaker when ACP is merely another subprocess protocol around a provider that still performs sensitive work internally.

A practical initial implementation should therefore focus on ACP as a typed streaming and permission backend first, then selectively enable filesystem and terminal delegation only for providers and versions that demonstrably issue those reverse requests.
