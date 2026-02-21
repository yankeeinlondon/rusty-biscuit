---
prompt: |-
    ## Focus

    This document covers how an Agentic CLI can be configured and run in a way that is both permissive enough to get work done efficiently while also being careful not to allow damaging actions to take place.

    **Identify which Agentic CLI this document covers from the document's filename and H1 heading.** All research must be specific to that CLI.

    The areas this research will focus on include:

    1. Event Hooks

        - PRE-TOOL:
            - One way to protect against damaging actions is to configure hooks that evaluate "pre-tool" calls and either _block_ or force _user approval_ before allowing commands which fit certain known patterns that indicate dangerous potential.
        - USER-PROMPT:
            - Some agents provide an event which allows all user prompts to be reviewed and modified before being recognized and processed by the Agent.
            - Where this is available, it is very helpful for scanning for dangerous patterns. The content tends to be less structured than what you'd get for a pre-tool call but it can still be helpful.
        - OTHER EVENTS:
            - While the pre-tool hook is the most common lifecycle event used for protection, the Agent may expose other events (e.g., post-tool, notification, session lifecycle, context compaction) that can also contribute to safety when used correctly.
            - Document any such events, what they trigger on, whether they can block execution, and how they might be used defensively.

        When considering Event Hooks as a protective measure, it is critical to understand and document:

        - Does this event fire not only for basic prompts but also in subagent/orchestrated flows?
            - If it doesn't always fire, specify exactly where it does and does not.
        - If an event fires but is not "blocking" (meaning listeners can STOP or MODIFY execution) then it is much more limited in its effectiveness.
            - Always describe whether event listeners can return a value to modify behavior and what behaviors they can "influence" or "guarantee".
        - What is the configuration format for hooks? (JSON, TOML, YAML, etc.)
        - Where are hooks configured? (user scope, project scope, managed/enterprise scope)
        - Can hooks be defined inline in skills, agents, or other component files?

    2. Intercepting MCP Calls

        MCP servers can be useful for gathering or synthesizing information but their responses are typically not checked by an Agent's event system and can contain secrets or embedded instructions to do something harmful.

        Research and document these specifics:

        - Where are MCP servers configured? At what scopes (user, project, enterprise)?
        - Are there any events the Agent provides to intercept the MCP response before it is fed back into the Agent's processing flow?
            - Does the event allow modification of the response before it's used?
            - Does the event allow stopping the Agentic flow if needed?
        - How are environment variables passed into the MCP server?
        - Do local MCP binaries require fully qualified paths?
        - Are local (stdio) MCP services allowed? Are remote (HTTP/SSE) MCP services allowed?
        - Does the Agent support any authentication regimes for MCP services (OAuth, API keys, bearer tokens)?
        - Can MCP servers be allow-listed or deny-listed at the enterprise/managed level?

    3. Completion Gates

        Attaching to events that mark the "completion" of a task gives an opportunity to:

        1. Run tests or other validation to verify that work is actually DONE and force the Agent to continue when it's not
        2. Scan for changes and look for "secrets" or sensitive data that shouldn't be in files, and take corrective action

        Research and document these specifics:

        - What events (if any) fire when the Agent considers its work complete? (e.g., stop events, task completion events, turn completion)
        - Can these completion events be blocked to force the Agent to continue working?
        - Is there a risk of infinite loops when blocking completion, and if so, how is that mitigated?
        - Can completion hooks run external commands (test suites, linters, secret scanners)?
        - Are there separate events for main agent completion vs. subagent completion?
        - Can completion hooks inject feedback or instructions back into the Agent?

    4. Subagents as a Security Event

        Having the ability to orchestrate and run concurrently via subagents is a feature, but because some Agent platforms provide a hook/event model that DOESN'T fire on subagents, we may need to treat the creation of a subagent as a security event.

        Through thorough research, determine whether the Agent supports event hooks not only from the originating/root-level tasks but throughout the flow and subagent process. If it does, document this clearly along with any quirks or gotchas. If it does not, explore how to mitigate this increased risk.

        - Can we detect a subagent's creation reliably via events?
        - Do pre-tool and post-tool hooks fire inside subagents the same as they do in the main agent?
        - Can we force a stricter permissions profile on a subagent when it runs?
        - Can we limit MCP servers to only "read-only" variants within subagents?
        - Can we reduce access to shell/filesystem tools for subagents?
        - Can context or instructions be injected into subagents at creation time?

    5. Escalated Privileges

        If the Agent is running as "root" or another user with escalated privileges, it has much greater potential for harm. Research how the Agent handles this:

        - Does the Agent automatically detect and warn about running as root or with elevated privileges?
        - Is there a way to detect and respond to elevated privileges via configuration or hooks/events?
        - Does the Agent provide sandboxing or container-based isolation?
        - Can filesystem write paths be restricted?
        - Can network access be restricted or controlled?
        - Is there a "dangerous" or "bypass permissions" mode, and if so, what safeguards exist around it?

    ## Task

    - Your task is to _update_ the research in the body of this file. If it's empty or contains only skeleton headings, create the content from scratch.
    - **Identify the target Agentic CLI from this document's filename and H1 heading.** All research must be specific to that CLI.
    - Be THOROUGH and SURE of your answer before updating the documentation.
        - If you find conflicting information from different sources, SAY THAT explicitly — this is as valuable as a definitive answer.
        - If a capability is NOT SUPPORTED by this CLI, state that clearly rather than omitting the section.
    - Always provide Markdown links to the sources you used for your research, inline or in a Sources section per topic.

    **IMPORTANT:** You must use the "claudine" skill when executing this task.
    **IMPORTANT:** Preserve all frontmatter properties that exist in this document. Your updates will only be to the BODY of this document.
closure: |-
    ## Task

    - Your task is to review the BODY of this document and extract key information into the frontmatter.
    - If any required property is not clearly answered in the body, do further research to reach a conclusive answer.
    - **If a property genuinely does not apply to this Agentic CLI** (e.g., it has no MCP support), set string values to `"n/a"`, booleans to `null`, and lists to `[]`.

    The frontmatter properties you MUST add/update are:

    ### General

    - `agent_version` - string: the latest version of the Agent software at the time of this research. If no version number is available, use the latest known release date.

    ### Event Hooks

    - `has_blocking_pre_tool_event` - boolean: whether the Agent has an event that fires before a planned tool call AND whose return value can influence whether that call proceeds.
    - `pre_tool_influence`
        - `"n/a"` if `has_blocking_pre_tool_event` is false
        - `"influence"` if the return value can influence but not deterministically guarantee the outcome
        - `"guarantee"` if the return value deterministically controls the outcome (e.g., an exit code or JSON response the Agent must obey)
    - `pre_tool_actions` - list of actions the pre-tool event listener can perform. Only include actions the CLI actually supports:
        - `stop` - block the current tool call; the agent receives feedback and continues working
        - `exit` - stop the agent's work entirely, propagating to any parent process/orchestrator
        - `ask-stop` - present the tool call to the user for approval; if denied, the tool call is blocked
        - `ask-exit` - present the tool call to the user for approval; if denied, the agent's work is stopped entirely
        - For every action listed, add a subsection in the BODY describing how it would be achieved. Include a code example in whatever language/format hooks are written in for this CLI (shell script, Python, JSON config, etc.). Note any nuances, exceptions, or gotchas.
    - `pre_tool_subagent` - boolean: whether pre-tool hooks fire inside subagents (not just the main agent).
    - `user_prompt_event` - boolean: whether the Agent provides an event where user prompts can be received before processing.
    - If `user_prompt_event` is true, also set:
        - `user_prompt_blocking_event` - boolean: whether the user prompt event can block execution or force user confirmation
        - `user_prompt_mutation_event` - boolean: whether the user prompt event allows mutating the prompt before the Agent processes it
        - `user_prompt_subagent` - boolean: whether the user prompt event fires inside subagents
    - `other_events` - key/value dictionary of other events useful for safety. Keys are event names, values describe: what the event triggers on, whether it supports blocking/return values, and how it could be used defensively. Omit the property entirely if there are no additional relevant events.

    ### Intercepting MCP Calls

    - `mcp_supported` - boolean: whether the Agent supports MCP servers at all.
    - `mcp_docs` - URL to the Agent's MCP documentation (or `"n/a"`)
    - `mcp_config_user` - filepath to user-scoped MCP configuration (or `"n/a"`)
    - `mcp_config_repo` - filepath to repo-scoped MCP configuration (or `"n/a"`)
    - `mcp_event` - boolean: whether the Agent has a hook event that gives access to MCP server responses
    - `mcp_event_name` - name of the event providing access to MCP responses (or `"n/a"`)
    - `mcp_event_modifiable` - boolean: whether the MCP response can be modified before the Agent uses it
    - `mcp_event_stop` - boolean: whether the MCP response event allows stopping execution or requiring permission

    ### Completion Gates

    - `has_completion_event` - boolean: whether the Agent fires an event when it considers work complete
    - `completion_event_blocking` - boolean: whether the completion event can prevent the Agent from stopping
    - `completion_event_names` - list of event names that relate to task/turn completion (or `[]`)
    - `completion_loop_protection` - boolean: whether the Agent has built-in protection against infinite loops from blocking completion

    ### Subagents

    - `has_subagent_events` - boolean: whether subagent creation/completion fires events
    - `hooks_fire_in_subagents` - boolean: whether pre-tool and post-tool hooks fire inside subagents the same as the main agent (or `null` if unknown)
    - `subagent_permissions_configurable` - boolean: whether subagent permissions can be restricted independently

    ### Escalated Privileges

    - `has_sandbox` - boolean: whether the Agent provides sandbox or container-based isolation
    - `detects_elevated_privileges` - boolean: whether the Agent detects and warns about running as root or elevated
    - `has_bypass_mode` - boolean: whether the Agent has a mode that bypasses permission checks entirely

    ### Metadata

    Before you finish, set:

    - `last_updated` to the current date in the format YYYY-MM-DD
    - `body_hash` to the xxHash value for this document's Markdown body content (not frontmatter)
        - Compute by printing the body content and piping to `bh` as STDIN
        - If the `bh` utility is not found in the executable path, leave this blank
agent_version: "v3.50.3 (Extension), v0.0.55 (CLI)"
has_blocking_pre_tool_event: true
pre_tool_influence: guarantee
pre_tool_actions:
    - stop
    - exit
    - ask-stop
pre_tool_subagent: true
user_prompt_event: false
other_events:
    stateChange: "Fires on any agent state transition (CLI ExtensionClient). Non-blocking, no return value. Use to monitor for unexpected states or detect when agent enters dangerous states."
    message: "Fires on each new conversation message including tool calls and results (CLI ExtensionClient). Non-blocking, no return value. Use for post-hoc audit logging of tool calls."
    tool_use: "Fires on tool invocation in NDJSON stream output (CLI stream-json). Non-blocking. Use to detect and log tool calls in CI/CD pipelines."
    tool_result: "Fires on tool execution outcome in NDJSON stream output (CLI stream-json). Non-blocking. Use to detect errors or unexpected results including MCP responses (subtype: mcp)."
    taskToolFailed: "Fires on tool execution failure (VS Code RooCodeAPI). Non-blocking. Use to alert on tool failures that might indicate dangerous operations."
    modeChanged: "Fires on mode switch (both CLI and VS Code). Non-blocking. Use to detect unexpected mode changes to modes with broader permissions."
    error: "Fires on processing errors (CLI ExtensionClient). Non-blocking. Use to catch errors and decide whether to cancel the task."
mcp_supported: true
mcp_docs: "https://docs.roocode.com/features/mcp/using-mcp-in-roo"
mcp_config_user: "mcp_settings.json"
mcp_config_repo: ".roo/mcp.json"
mcp_event: false
mcp_event_name: "n/a"
mcp_event_modifiable: false
mcp_event_stop: false
has_completion_event: true
completion_event_blocking: true
completion_event_names:
    - taskCompleted
    - taskDelegationCompleted
    - result
completion_loop_protection: true
has_subagent_events: true
hooks_fire_in_subagents: true
subagent_permissions_configurable: true
has_sandbox: false
detects_elevated_privileges: false
has_bypass_mode: true
last_updated: "2026-02-20"
body_hash: 2439901673214276288
---

# Protecting Roo Code

> **Agent version at time of research:** v3.50.3 (Extension), v0.0.55 (CLI)
> **Platform:** VS Code extension + standalone CLI (`roo`)

Roo Code is fundamentally different from shell-based agentic CLIs like Claude Code, Gemini CLI, or Codex CLI. It does **not** provide a declarative, shell-level hook system where external scripts receive JSON on stdin and return exit codes to block or modify tool calls. Instead, Roo Code's protection model is built on three pillars:

1. **Auto-approval permission gates** -- a multi-category approval system that requires explicit user consent (or pre-configured auto-approval) before tool execution
2. **Programmatic event surfaces** -- Node.js EventEmitter-based events accessible via the `ExtensionClient` API (CLI) or `RooCodeAPI` (VS Code extension), which are **observational only** and cannot block execution through return values
3. **Mode-based tool restrictions** -- per-mode tool group configuration with optional file regex patterns that constrain what tools are available in each operational mode

This architecture means that "protection" in Roo Code works differently than in hook-based CLIs. Rather than intercepting tool calls with external scripts, you configure permissions up front and respond to events programmatically.

## Event Hooks

### Pre-Tool Events

**Roo Code does not have a shell-level pre-tool hook.** There is no equivalent to Claude Code's `PreToolUse` event where an external script receives a JSON payload on stdin and can return an exit code to block or allow a tool call. This is the single most significant difference between Roo Code's protection model and that of hook-based CLIs.

#### What Roo Code offers instead: Approval Gates

Before any tool executes, Roo Code runs a multi-layered validation pipeline:

1. **RooIgnoreController** -- blocks access to files matching `.rooignore` patterns entirely (hard block, not overridable)
2. **Mode restrictions** -- `fileRegex` patterns in mode group definitions prevent tools from operating on certain file types
3. **Workspace boundary checks** -- rejects operations outside the workspace unless explicitly permitted
4. **Auto-approval rules** -- evaluates tool calls against allowlist/denylist configurations for commands, and per-category toggles for read/write/execute/browser/MCP operations

If auto-approval is **not** enabled for a given category, the agent pauses and presents the tool call to the user for manual approval or rejection. This is the closest equivalent to a "pre-tool hook" but it is an interactive UI gate, not a programmable hook.

#### Programmatic pre-tool interception (CLI `ExtensionClient`)

For the CLI, the `waitingForInput` event fires when the agent needs approval for a tool call. By listening to this event and inspecting the payload, you can programmatically approve or reject tool calls:

```typescript
import { ExtensionClient } from "@roo-code/cli";

const client = new ExtensionClient(/* config */);

client.on("waitingForInput", (event) => {
  if (event.ask === "tool") {
    const message = event.message;
    // Inspect message.text to determine what tool is being called
    // and decide whether to approve or reject
    if (isDangerous(message.text)) {
      client.reject(); // Block the tool call
    } else {
      client.approve(); // Allow the tool call
    }
  } else if (event.ask === "command") {
    // Command execution approval
    if (isUnsafeCommand(message.text)) {
      client.reject();
    } else {
      client.approve();
    }
  }
});
```

**Critical limitations:**

- The `waitingForInput` event only fires when auto-approval is **disabled** for the relevant category. If auto-approve is on (which is the CLI default), tool calls execute without ever emitting this event.
- To use this pattern for protection, you must run the CLI with `--require-approval` (`-a`), which disables auto-approval and forces every tool call through the approval gate.
- Event listener return values are **ignored** (standard Node.js EventEmitter behavior). You must call `client.approve()` or `client.reject()` explicitly; you cannot return a value to influence execution.
- The event only fires **on transitions**. If you attach a listener after the CLI is already waiting, you will miss the event. Workaround: call `getAgentState()` immediately after constructing the client and check `isWaitingForInput`.

#### Pre-Tool Action: `stop` (Block Tool Call, Agent Continues)

To block a specific tool call while allowing the agent to continue working on an alternative approach, use `client.reject()` inside a `waitingForInput` listener. The agent receives feedback that the action was rejected and attempts a different strategy.

```typescript
import { ExtensionClient } from "@roo-code/cli";

const client = new ExtensionClient(/* config */);

client.on("waitingForInput", (event) => {
  if (event.ask === "tool" || event.ask === "command") {
    const toolDescription = event.message?.text ?? "";
    if (matchesDangerousPattern(toolDescription)) {
      // Block this tool call; agent will try something else
      client.reject();
    } else {
      client.approve();
    }
  }
});
```

**Nuances:** This only works when `--require-approval` (`-a`) is set. With auto-approval enabled, the event never fires and tool calls proceed unchecked. After rejection, the agent receives a message indicating the tool call was denied and typically attempts an alternative approach.

#### Pre-Tool Action: `exit` (Stop Agent Entirely)

To halt the agent's work entirely (not just block one tool call), use `client.cancelTask()`. This terminates the current task and propagates to any parent process.

```typescript
client.on("waitingForInput", (event) => {
  if (event.ask === "tool" || event.ask === "command") {
    const toolDescription = event.message?.text ?? "";
    if (isCriticallyDangerous(toolDescription)) {
      // Stop the agent entirely -- no more work will be done
      client.cancelTask();
    } else {
      client.approve();
    }
  }
});
```

**Nuances:** `cancelTask()` is deterministic -- the agent cannot override it. The CLI process will exit. If running within an orchestrator (e.g., a parent Node.js process), the task completion event fires with `success: false`.

#### Pre-Tool Action: `ask-stop` (Present to User for Approval)

This is the **default behavior** when auto-approval is disabled (`--require-approval`). The agent pauses on every tool call and waits for input. In the VS Code extension, this presents a GUI approval dialog. In the CLI with `ExtensionClient`, you can implement a user-facing prompt:

```typescript
import { createInterface } from "readline";

const rl = createInterface({ input: process.stdin, output: process.stdout });

client.on("waitingForInput", (event) => {
  if (event.ask === "tool" || event.ask === "command") {
    const toolDescription = event.message?.text ?? "";
    rl.question(
      `Tool call: ${toolDescription}\nApprove? (y/n): `,
      (answer) => {
        if (answer.toLowerCase() === "y") {
          client.approve();
        } else {
          // User denied -- block the tool call, agent continues
          client.reject();
        }
      }
    );
  }
});
```

**Nuances:** Without the `ExtensionClient` API, the CLI's `--require-approval` flag causes the agent to pause and print the tool call to stdout, waiting for manual user input in the terminal. The programmatic version above gives you full control over the approval UX.

#### Influence vs. Guarantee

When using the `ExtensionClient` API with `--require-approval`:

- Calling `client.reject()` **deterministically blocks** the tool call. The agent receives feedback that the action was rejected and continues working on an alternative approach.
- Calling `client.approve()` **deterministically allows** the tool call to proceed.
- Calling `client.cancelTask()` **deterministically stops** the agent's work entirely.

These are **guarantees**, not influences -- the agent cannot override a `reject()` or `cancelTask()` call. However, this guarantee only holds when auto-approval is disabled. With auto-approval enabled, the programmatic surface is bypassed entirely.

#### Configuration format

There is no declarative configuration file for hooks. The `ExtensionClient` API is the only programmatic hook surface for the CLI. For the VS Code extension, the `RooCodeAPI` EventEmitter provides equivalent events.

Auto-approval settings are configured through:
- **VS Code extension:** GUI toggle panel (accessible via `Cmd+Alt+A` / `Ctrl+Alt+A`)
- **CLI:** `--require-approval` flag (disables auto-approval) or default behavior (auto-approves everything)

There is no enterprise/managed scope for hook configuration.

#### Can hooks be defined inline in skills or agents?

No. Skills (`.roo/skills/`) contain instructions and scripts but cannot define event hooks or modify the approval pipeline. Custom modes (agents) can restrict tool groups but cannot define pre-tool hooks.

### User Prompt Events

**Roo Code does not provide a pre-processing user prompt event.** There is no event that fires before the user's prompt is recognized and processed by the agent, and there is no mechanism to modify or block a user prompt before it reaches the LLM.

The closest equivalents are:

- **CLI `message` event**: Fires when a new message arrives in the conversation, including user messages. However, this is observational and fires **after** the message is processed, not before. The event payload is `ClineMessage` and listener return values are ignored.
- **VS Code `taskUserMessage` event**: Fires when a user sends a message, but only as a notification. It cannot block or modify the prompt.
- **Custom instructions** (`.roo/rules/`): These are injected into the system prompt at session start, shaping agent behavior. They cannot dynamically intercept individual user prompts.

### Other Events Useful for Safety

Roo Code exposes several events across its three event surfaces (CLI `ExtensionClient`, CLI structured output, VS Code `RooCodeAPI`) that can contribute to safety when used correctly. All of these are **observational only** -- listener return values are ignored.

| Event | Surface | Triggers On | Blocking? | Defensive Use |
|-------|---------|-------------|-----------|---------------|
| `stateChange` | CLI ExtensionClient | Any agent state transition | No | Monitor for unexpected state changes; detect when agent enters dangerous states |
| `waitingForInput` | CLI ExtensionClient | Agent needs user input | No (but triggers approval gate) | Primary programmatic hook for tool approval/rejection when `--require-approval` is set |
| `message` | CLI ExtensionClient | New conversation message | No | Inspect tool calls and results after they happen; log for audit |
| `tool_use` | CLI stream-json | Tool invocation | No | Parse NDJSON output to detect and log tool calls in CI/CD pipelines |
| `tool_result` | CLI stream-json | Tool execution outcome | No | Detect errors or unexpected results from tool calls |
| `taskToolFailed` | VS Code RooCodeAPI | Tool execution failure | No | Alert on tool failures that might indicate dangerous operations |
| `modeChanged` | Both | Mode switch | No | Detect unexpected mode changes (e.g., switching to a mode with broader permissions) |
| `error` | CLI ExtensionClient | Processing error | No | Catch and log errors; decide whether to cancel task |

**Key limitation across all events:** None of these events support shell-based hooks, matchers, regex filtering, or exit-code-based flow control. All filtering must be done in your event listener code by inspecting event payloads.

### Sources

- [Roo Code hooks research (claudine skill)](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/events.ts)
- [CLI message processor](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/message-processor.ts)
- [Auto-Approving Actions](https://docs.roocode.com/features/auto-approving-actions)
- [How Tools Work](https://docs.roocode.com/basic-usage/how-tools-work)
- [CLI README](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/README.md)
- [Extension API source](https://github.com/RooCodeInc/Roo-Code/blob/main/src/extension/api.ts)

## Intercepting MCP Calls

### MCP Support

Roo Code supports MCP (Model Context Protocol) servers with three transport types:

| Transport | Type | Description |
|-----------|------|-------------|
| **STDIO** | Local | Standard input/output; lower latency, no network exposure |
| **Streamable HTTP** | Remote | Modern HTTP POST/GET to a single MCP endpoint |
| **SSE** | Remote (legacy) | Server-Sent Events over HTTP/HTTPS |

### Configuration Scopes

| Scope | Location | Notes |
|-------|----------|-------|
| **Global (user)** | `mcp_settings.json` (in VS Code user settings directory) | Applies to all workspaces |
| **Project (repo)** | `.roo/mcp.json` | Project-specific; can be committed to version control |

Project-level configuration takes precedence when a server name exists in both global and project configs. There is **no enterprise/managed scope** for MCP configuration -- Roo Code does not have an organization-level policy enforcement mechanism for MCP servers.

### Environment Variables

Environment variables are passed to MCP servers via the `env` property in the server configuration:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["server.js"],
      "env": {
        "API_KEY": "${env:MY_API_KEY}"
      }
    }
  }
}
```

The `${env:VARIABLE_NAME}` syntax references system environment variables. The variable must exist in the system environment at runtime.

**Security concern:** There is a [known issue](https://github.com/RooCodeInc/Roo-Code/issues/2548) where environment variable references like `${env:MY_SECRET}` do not work consistently in `.roo/mcp.json`, forcing some users to store credentials in plaintext. This is a significant security risk when the file is committed to version control.

### Path Requirements

The `cwd` parameter defaults to the first workspace folder path or the main process's working directory when omitted. For STDIO servers, the `command` field should specify the command name (resolved via PATH) or a fully qualified path. Windows configurations may require shell wrappers (`cmd /c`).

### MCP Response Interception

**Roo Code does not provide any event to intercept MCP responses before they are fed into the agent's processing flow.** There is no equivalent to a "post-MCP" hook. MCP tool results flow directly back into the agent's conversation context without any opportunity for external inspection, modification, or blocking.

The only observational surface is:
- **CLI stream-json**: `tool_result` events with `subtype: "mcp"` appear in the NDJSON output stream, but these are read-only and fire after the result has already been processed.
- **VS Code `message` event**: MCP-related messages appear as `ClineMessage` objects with `say: "mcp_server_response"`, but again, this is observational only.

### Authentication

Roo Code supports the following authentication mechanisms for MCP servers:

- **Custom HTTP headers**: Via the optional `headers` object in Streamable HTTP and SSE configurations (e.g., `Authorization: Bearer <token>`, `X-API-Key: <key>`)
- **Environment variables**: For credential injection into STDIO servers
- **OAuth 2.1**: Support is [being implemented](https://github.com/RooCodeInc/Roo-Code/issues/8119) for HTTP MCP servers with RFC 9728 discovery and PKCE, but as of v3.50.3 this is not yet fully functional ([issue #7296](https://github.com/RooCodeInc/Roo-Code/issues/7296))

### Per-Server Tool Control

Roo Code provides granular tool-level control within each MCP server configuration:

- **`alwaysAllow`**: An array of tool names that are auto-approved without user confirmation
- **`disabledTools`**: An array of tool names that are completely disabled and unavailable to the agent
- **`disabled`**: A boolean that disables the entire server

These are **per-server** settings, not enterprise-level allow/deny lists. There is no centralized MCP server policy.

### Security Vulnerability History

A [security advisory](https://github.com/RooCodeInc/Roo-Code/security/advisories/GHSA-5x8h-m52g-5v54) was issued for a potential remote code execution vulnerability via MCP configuration: because `.roo/mcp.json` allows execution of arbitrary commands, an attacker could craft a prompt to write a malicious command to the configuration file. Roo Code addressed this by adding an additional opt-in configuration layer for auto-approving writes to `.roo/` protected files.

### Sources

- [Using MCP in Roo Code](https://docs.roocode.com/features/mcp/using-mcp-in-roo)
- [use_mcp_tool](https://docs.roocode.com/advanced-usage/available-tools/use-mcp-tool)
- [MCP OAuth issue #8119](https://github.com/RooCodeInc/Roo-Code/issues/8119)
- [MCP OAuth issue #7296](https://github.com/RooCodeInc/Roo-Code/issues/7296)
- [Security advisory GHSA-5x8h-m52g-5v54](https://github.com/RooCodeInc/Roo-Code/security/advisories/GHSA-5x8h-m52g-5v54)
- [Environment variable issue #2548](https://github.com/RooCodeInc/Roo-Code/issues/2548)

## Completion Gates

### Completion Events

Roo Code fires completion-related events across its event surfaces:

| Event | Surface | Payload | Description |
|-------|---------|---------|-------------|
| `taskCompleted` | CLI ExtensionClient | `{ success: boolean, stateInfo: AgentStateInfo, message?: ClineMessage }` | Fires when the agent determines a task has completed |
| `taskCompleted` | VS Code RooCodeAPI | `[taskId, tokenUsage, toolUsage, { isSubtask: boolean }]` | Task finished (includes subtask flag) |
| `result` | CLI stream-json | `{ type: "result", success: boolean, content: string, cost: {...} }` | Final task completion in JSON output |

### Can Completion Be Blocked?

**Partially.** The `attempt_completion` tool presents completion as an `ask`-type message (`completion_result`), meaning the agent pauses and waits for user feedback. At this point:

- **In the VS Code extension:** The user can provide feedback text instead of accepting the completion, which sends the agent back to work. This creates an iterative refinement cycle.
- **Via the CLI `ExtensionClient`:** You can listen for the `waitingForInput` event where `ask === "completion_result"` and call `client.respond(feedback)` to inject instructions that force the agent to continue:

```typescript
client.on("waitingForInput", (event) => {
  if (event.ask === "completion_result") {
    // Run external validation
    const testResult = runTests();
    const secretScan = scanForSecrets();

    if (!testResult.passed || secretScan.found) {
      // Reject completion and force the agent to continue
      client.respond(
        `Task is NOT complete. Issues found:\n${testResult.errors}\n${secretScan.report}\nPlease fix these issues.`
      );
    } else {
      // Accept completion
      client.respond("Looks good, thank you.");
    }
  }
});
```

This pattern **requires** that auto-approval for subtasks/completion is disabled (or that `--require-approval` is set in the CLI). When auto-approval is enabled, completion proceeds without pausing for feedback.

### Infinite Loop Protection

Roo Code has two built-in mechanisms to prevent infinite loops:

1. **`auto_approval_max_req_reached`**: When a "Max Requests" limit is configured in auto-approval settings, the agent pauses after reaching the specified number of consecutive API calls and requires the user to "Reset and Continue." This prevents runaway loops in auto-approval mode.

2. **`mistake_limit_reached`**: The agent tracks consecutive errors. When a threshold is reached, it emits this as an `ask` type, pausing execution and requiring user intervention.

Both of these are `ask` types that trigger the `waitingForInput` event, providing programmatic access for automated intervention.

### Separate Events for Main Agent vs. Subtask Completion

Yes. The VS Code `RooCodeAPI` surface provides distinct events:

- `taskCompleted` includes an `{ isSubtask: boolean }` flag, allowing listeners to distinguish main task completion from subtask completion
- `taskDelegationCompleted` fires specifically when a delegated subtask finishes, with `[parentTaskId, childTaskId, completionResultSummary]`

The CLI `ExtensionClient` `taskCompleted` event does not expose the subtask flag directly, but you can track subtask state by monitoring `stateChange` events and the task ID.

### Can Completion Hooks Run External Commands?

Not directly via hooks, because Roo Code does not have a shell-level hook system. However, via the `ExtensionClient` API, a `waitingForInput` listener can spawn external processes (test suites, linters, secret scanners) using Node.js `child_process` APIs before calling `client.respond()` or `client.approve()`.

### Sources

- [attempt_completion](https://docs.roocode.com/advanced-usage/available-tools/attempt-completion)
- [Auto-Approving Actions](https://docs.roocode.com/features/auto-approving-actions)
- [Rate Limits and Costs](https://docs.roocode.com/advanced-usage/rate-limits-costs)
- [CLI event map](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/events.ts)

## Subagents as Security Event?

### Subtask Architecture

Roo Code implements subagents through its **Boomerang Tasks** system. The Orchestrator mode spawns subtasks in specialized modes (Code, Architect, Debug, etc.) via the `new_task` tool. Each subtask:

- Runs in its own conversation context with separate history
- Does **not** inherit parent conversation history
- Receives instructions only via the `message` parameter
- Returns results only via `attempt_completion`'s `result` parameter

### Can We Detect Subtask Creation?

**Yes**, through multiple event surfaces:

| Event | Surface | Payload |
|-------|---------|---------|
| `taskSpawned` | VS Code RooCodeAPI | `[parentTaskId, childTaskId]` |
| `taskDelegated` | VS Code RooCodeAPI | `[parentTaskId, childTaskId]` |
| `taskPaused` | VS Code RooCodeAPI | `[taskId]` (parent task paused for subtask) |

The CLI `ExtensionClient` does not have dedicated subtask creation events, but subtask creation triggers a `waitingForInput` event with `ask === "tool"` (because `new_task` is itself a tool call that requires approval when auto-approval is disabled). You can detect this by inspecting the tool name in the message text.

### Do Pre-Tool Hooks Fire Inside Subtasks?

**There are no shell-level pre-tool hooks in Roo Code** (see Event Hooks section above). However, the approval gates and auto-approval settings apply globally -- they are not scoped per-task or per-subtask. This means:

- If auto-approval is **disabled**, tool calls in subtasks still require approval through the same `waitingForInput` mechanism
- If auto-approval is **enabled**, tool calls in subtasks are auto-approved just like in the parent task

The VS Code extension's approval UI applies to all tasks regardless of depth. The CLI's `ExtensionClient` events fire for all tasks in the process.

**Important caveat:** Subtask auto-approval is controlled by a **separate toggle** in the auto-approval settings. When "Subtasks" auto-approval is enabled, both subtask creation and subtask completion are auto-approved, bypassing the user confirmation gate entirely. This is a significant security consideration.

### Can We Force Stricter Permissions on Subtasks?

**Yes, partially.** Because subtasks run in specific modes, and each mode defines its allowed tool groups:

- The **Orchestrator mode** has no direct tool access (only `new_task`), so it cannot read files, write files, or run commands
- Subtasks can be spawned into **custom modes** with restricted `groups` and `fileRegex` patterns:

```yaml
# custom_modes.yaml
customModes:
  - slug: safe-code
    name: "Safe Code Mode"
    roleDefinition: "You write code but cannot execute commands."
    groups:
      - read
      - - edit
        - fileRegex: "\\.(ts|js|json)$"
          description: "Only TypeScript, JavaScript, and JSON files"
      # No 'command' group - no shell access
      # No 'mcp' group - no MCP server access
```

This allows restricting subtasks to read-only, write-only-certain-files, or no-command-execution modes. However:
- The Orchestrator must **choose** to spawn into the restricted mode -- it cannot be forced externally
- Custom instructions in `.roo/rules-orchestrator/` can guide the Orchestrator to prefer restricted modes for subtasks, but this is an instruction, not a guarantee

### Can We Limit MCP Servers in Subtasks?

**Partially.** The `mcp` tool group can be excluded from a custom mode's `groups` array, which would prevent all MCP access in that mode. However, there is no per-subtask MCP server configuration or read-only MCP mode. MCP server settings are global and project-scoped, not task-scoped.

### Can We Reduce Shell/Filesystem Access for Subtasks?

**Yes.** By spawning subtasks into modes that exclude the `command` tool group (for shell access) or restrict the `edit` group with `fileRegex` patterns (for filesystem writes). The `read` group can also be restricted with file patterns if needed.

### Can Context or Instructions Be Injected into Subtasks?

**Yes.** The `new_task` tool accepts a `message` parameter containing comprehensive instructions. Additionally:

- Mode-specific rules in `.roo/rules-{modeSlug}/` are automatically loaded when a subtask enters that mode
- `AGENTS.md` at the workspace root is loaded for all tasks (including subtasks)
- Global rules in `~/.roo/rules/` apply to all tasks

These instruction injection points allow embedding security guidelines, restrictions, and compliance requirements into every subtask.

### Sources

- [Boomerang Tasks](https://docs.roocode.com/features/boomerang-tasks)
- [new_task tool](https://docs.roocode.com/advanced-usage/available-tools/new-task)
- [Custom Modes](https://docs.roocode.com/features/custom-modes)
- [Auto-Approving Actions](https://docs.roocode.com/features/auto-approving-actions)
- [Extension API events](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/events.ts)

## Escalated Privileges

### Root/Elevated Privilege Detection

**Roo Code does not automatically detect or warn about running as root or with elevated privileges.** There is no built-in check for `uid === 0` (Linux/macOS) or Administrator status (Windows). A [bug report](https://github.com/RooCodeInc/Roo-Code/issues/5994) documented that Roo Code's browser-based tools run as root in VS Code Remote-SSH containers without any warning, causing Chromium sandbox failures. The team's response did not indicate plans to add privilege detection.

### Detecting Elevated Privileges via Configuration

There is no configuration option or hook/event to detect elevated privileges. As a workaround, you could:

1. Add a custom tool (`.roo/tools/check-privileges.ts`) that checks the process UID and returns a warning
2. Include custom instructions in `.roo/rules/` that tell the agent to check its privilege level before executing dangerous operations
3. Use the CLI `ExtensionClient` to check `process.getuid()` before starting the agent

None of these are built-in features -- they are all manual workarounds.

### Sandboxing and Container Isolation

**Roo Code does not provide built-in sandboxing or container-based isolation.** There is no equivalent to Codex CLI's sandbox or Claude Code's Docker sandbox support.

The protection mechanisms available are:

| Mechanism | Type | Scope |
|-----------|------|-------|
| `.rooignore` | File access control | Files matching patterns are blocked from all tools |
| Workspace boundary | Filesystem boundary | Operations outside workspace require explicit opt-in |
| Protected files | Write protection | `.roo/` directory and `.rooignore` require explicit opt-in to modify |
| Mode tool groups | Tool restriction | Per-mode tool availability with file regex patterns |
| Auto-approval gates | Approval workflow | Per-category permission toggles |

These are **application-level** protections, not OS-level sandboxing. They rely on Roo Code's own enforcement rather than kernel namespaces, containers, or filesystem permissions.

For true sandboxing, you must run Roo Code inside an external container (Docker, VM, etc.) and connect via VS Code Remote-SSH or Dev Containers. This is a deployment concern, not a built-in feature.

### Can Filesystem Write Paths Be Restricted?

**Yes**, through three mechanisms:

1. **`.rooignore`**: Files matching patterns are completely blocked from read and write operations
2. **Workspace boundary**: Write operations outside the workspace are blocked unless "Include files outside workspace" is enabled in auto-approval settings
3. **Mode `fileRegex` patterns**: The `edit` tool group can be restricted to specific file patterns per mode (e.g., `\\.md$` for markdown-only editing)

### Can Network Access Be Restricted?

**No.** Roo Code does not provide any mechanism to restrict network access. The agent can make HTTP requests via the browser tool, MCP servers can connect to arbitrary endpoints, and shell commands can access the network freely. Network restrictions must be enforced at the OS or container level.

### Bypass/Dangerous Mode

**The CLI auto-approves everything by default.** The `roo` CLI was designed for automation and runs with all auto-approval categories enabled unless `--require-approval` (`-a`) is specified. This is effectively a "bypass permissions" mode by default.

In the VS Code extension, auto-approval is **disabled** by default and must be toggled on per-category. There is no single "approve everything" button in the extension UI, though all individual toggles can be enabled.

Additionally, the command execution auto-approval includes dangerous-pattern blocking: even when commands are auto-approved, Roo Code blocks dangerous parameter substitutions like `${var@P}` and process substitutions automatically. This is a hardcoded safeguard that cannot be disabled.

### Sources

- [Auto-Approving Actions](https://docs.roocode.com/features/auto-approving-actions)
- [.rooignore](https://docs.roocode.com/features/rooignore)
- [Root privilege bug #5994](https://github.com/RooCodeInc/Roo-Code/issues/5994)
- [Custom Modes](https://docs.roocode.com/features/custom-modes)
- [FAQ](https://docs.roocode.com/faq)
- [Settings Management](https://docs.roocode.com/features/settings-management)
