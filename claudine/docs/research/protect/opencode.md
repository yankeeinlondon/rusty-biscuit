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
agent_version: "v1.2.10"
has_blocking_pre_tool_event: true
pre_tool_influence: guarantee
pre_tool_actions:
    - stop
    - ask-stop
pre_tool_subagent: true
user_prompt_event: true
user_prompt_blocking_event: false
user_prompt_mutation_event: true
user_prompt_subagent: false
other_events:
    tool.execute.after: "Fires after a tool call completes. Cannot block or retry, but can modify the output shown to the agent. Useful for redacting sensitive data from tool results."
    tool.definition: "Fires when tool definitions are assembled for the LLM. Can modify tool descriptions and parameter schemas to guide the LLM away from dangerous patterns. Influence-based, not a guarantee."
    permission.ask: "Fires when the permission system evaluates a tool call to 'ask'. Can programmatically set the decision to 'allow', 'deny', or leave as 'ask' for user prompting. Blocking/guaranteed."
    shell.env: "Fires on every shell invocation. Can inject environment variables into all shell executions. Useful for PATH restrictions or safety flags. Non-blocking but mutates environment."
    experimental.chat.system.transform: "Modifies the system prompt before sending to the LLM. Can inject persistent safety instructions. Influence-based. Experimental -- may change in future versions."
    event: "Fire-and-forget bus observer receiving 40+ event types. Cannot affect agent flow. Useful for logging, monitoring, and alerting."
mcp_supported: true
mcp_docs: "https://opencode.ai/docs/mcp-servers"
mcp_config_user: "~/.config/opencode/opencode.json"
mcp_config_repo: ".opencode/opencode.json"
mcp_event: true
mcp_event_name: "tool.execute.after"
mcp_event_modifiable: true
mcp_event_stop: false
has_completion_event: true
completion_event_blocking: false
completion_event_names:
    - session.status
    - session.idle
completion_loop_protection: true
has_subagent_events: true
hooks_fire_in_subagents: null
subagent_permissions_configurable: true
has_sandbox: false
detects_elevated_privileges: false
has_bypass_mode: false
last_updated: "2026-02-20"
body_hash: 3870497985973441061
---

# Protecting OpenCode CLI

> **Agent version at time of research:** v1.2.10 (February 20, 2026)
>
> OpenCode is an open-source AI coding agent by [Anomaly](https://github.com/anomalyco). It uses a TypeScript/JavaScript plugin system for hooks rather than shell-based hooks. Plugins are `.ts` or `.js` files that export async functions returning hook objects. This is a fundamentally different model from shell-invocation-based agents like Claude Code.

## Event Hooks

OpenCode provides a rich [plugin system](https://opencode.ai/docs/plugins) that exposes 16 hook entry points. Plugins are TypeScript or JavaScript modules that export an async function returning a hooks object. Each hook follows an **input/output mutation pattern**: the hook receives a read-only `input` and a mutable `output`, mutates `output` in place, and returns `Promise<void>`.

### Configuration Format and Scopes

**Format:** TypeScript (`.ts`) or JavaScript (`.js`) files, plus npm packages referenced in config JSON.

**Plugin loading locations (lowest to highest precedence):**

| Source | Scope | Path |
|--------|-------|------|
| Global config | User | Listed in `~/.config/opencode/opencode.json` under `"plugin"` |
| Project config | Project | Listed in `opencode.json` or `.opencode/opencode.json` under `"plugin"` |
| Global directory | User | `~/.config/opencode/plugins/*.{ts,js}` |
| Project directory | Project | `.opencode/plugins/*.{ts,js}` |

Plugins **cannot** be defined inline in skills, agents, or other component files. They must be standalone files in the `plugins/` directories or referenced as npm packages in config. However, plugin files can import from a project's `package.json` dependencies (add `package.json` to `.opencode/` and OpenCode runs `bun install` at startup).

There is no managed/enterprise-scoped plugin directory. Enterprise configs can reference plugin npm packages in the managed config directory (`/Library/Application Support/opencode` on macOS, `/etc/opencode` on Linux, `%ProgramData%\opencode` on Windows), but this is done through the `"plugin"` array in config JSON, not through a dedicated enterprise plugin directory.

### PRE-TOOL: `tool.execute.before`

The [`tool.execute.before`](https://opencode.ai/docs/plugins) hook fires before every tool call executes. It provides **guaranteed blocking** capability.

**Signature:**

```typescript
"tool.execute.before"?: (
  input: { tool: string; sessionID: string; callID: string },
  output: { args: any }
) => Promise<void>
```

**Flow control mechanisms:**

1. **Mutate `output.args`** to change tool arguments before execution
2. **Throw an error** to block the tool call entirely; the error message is returned to the agent as feedback

The blocking mechanism is deterministic: if the hook throws, the tool call is guaranteed not to execute. The error message becomes the tool's "result" from the agent's perspective, allowing the agent to adapt its behavior.

**Important:** This hook fires for **all** tool calls including the `task` tool (which spawns subagents). By intercepting `input.tool === "task"`, you can gate subagent creation.

#### Action: `stop` -- Block a Tool Call

Throwing an error in `tool.execute.before` blocks the current tool call. The error message is fed back to the agent, which continues working with that feedback. This is a guaranteed block.

```typescript
// .opencode/plugins/protect.ts
import type { Plugin } from "@opencode-ai/plugin"

const DANGEROUS_PATTERNS = [
  /rm\s+(-rf?|--recursive)\s+\//,
  /chmod\s+777/,
  />\s*\/etc\//,
  /mkfs\./,
  /dd\s+if=/,
]

export const ProtectPlugin: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash") {
        const cmd = output.args.command ?? ""
        for (const pattern of DANGEROUS_PATTERNS) {
          if (pattern.test(cmd)) {
            // Throwing blocks the tool call; the message goes to the agent
            throw new Error(
              `Blocked dangerous command: "${cmd}" matched pattern ${pattern}. ` +
              `Please use a safer alternative.`
            )
          }
        }
      }
    },
  }
}
```

**Gotchas:**
- The `args` shape varies by tool. For `bash`, the command is in `output.args.command`. For `edit`, the file path is in `output.args.filePath`. There is no compile-time type narrowing based on tool name.
- The error message appears to the agent as if the tool failed, so the agent may retry with a modified approach. This is generally the desired behavior for safety hooks.

#### Action: `exit` -- Stop the Agent Entirely

OpenCode does not provide a direct "exit the agent" mechanism from within a plugin hook. However, you can simulate this by throwing an error and then terminating the OpenCode process externally:

```typescript
// .opencode/plugins/hard-stop.ts
import type { Plugin } from "@opencode-ai/plugin"

export const HardStopPlugin: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command?.includes("rm -rf /")) {
        // Block the tool call
        throw new Error("CRITICAL: Attempted destructive command. Agent halted.")
        // Note: the agent will continue working after receiving this error.
        // To truly halt, you would need an external process monitor.
      }
    },
  }
}
```

**Limitation:** Throwing an error blocks the tool call but does NOT stop the agent from continuing to work. The agent receives the error as feedback and may attempt alternative approaches. There is no plugin API to force the agent to terminate. To achieve a true "exit," you would need an external watchdog process that monitors for specific error patterns and kills the OpenCode process.

#### Action: `ask-stop` -- Prompt User Before Blocking

OpenCode provides the `permission.ask` hook and the broader permission system for user-approval workflows. The most effective pattern combines the permission system with the `permission.ask` hook:

```jsonc
// opencode.json -- set bash permission to "ask"
{
  "permission": {
    "bash": "ask"
  }
}
```

```typescript
// .opencode/plugins/smart-permissions.ts
import type { Plugin } from "@opencode-ai/plugin"

const SAFE_COMMANDS = [/^git\s/, /^ls\s/, /^cat\s/, /^echo\s/]

export const SmartPermissions: Plugin = async () => {
  return {
    "permission.ask": async (input, output) => {
      if (input.permission === "bash") {
        const cmd = input.metadata?.command ?? ""
        // Auto-approve known safe commands
        if (SAFE_COMMANDS.some(p => p.test(cmd))) {
          output.status = "allow"
          return
        }
        // Everything else stays as "ask" -- user is prompted
      }
    },
  }
}
```

**Gotchas:**
- The `permission.ask` hook **only fires** when the permission system evaluates a rule to `"ask"`. If you set a tool to `"allow"` or `"deny"` in config, this hook never fires for that tool.
- The user is presented with three choices: `once` (approve this one time), `always` (whitelist matching patterns for the session), or `reject` (deny the request).

**Alternative approach using `tool.execute.before`:**

You can also implement user prompting directly in `tool.execute.before` using the plugin context's `$` (BunShell) to run an interactive prompt, but this is more fragile and not the recommended approach:

```typescript
// Less recommended -- uses external prompting
"tool.execute.before": async (input, output) => {
  if (input.tool === "bash" && isDangerous(output.args.command)) {
    // This would require a way to prompt the user,
    // which is not natively provided in tool hooks.
    // The permission.ask hook is the correct approach.
    throw new Error("Dangerous command detected. Use permission.ask for approval workflows.")
  }
}
```

#### Action: `ask-exit` -- Prompt User Then Exit

This action is **not directly supported** by OpenCode. There is no mechanism in the plugin system to both prompt the user and then terminate the agent based on the response. The closest approximation is:

1. Set the tool's permission to `"ask"` in config
2. If the user rejects, the tool call is blocked (throws `RejectedError`)
3. The agent receives the rejection and continues working (does not exit)

To achieve a true "exit on rejection," you would need an external process monitor.

### USER-PROMPT: `chat.message`

The [`chat.message`](https://opencode.ai/docs/plugins) hook fires when a new user message is being prepared for the LLM. It allows **mutation** of the message content and parts, but it **cannot block** the message from being processed.

**Signature:**

```typescript
"chat.message"?: (
  input: {
    sessionID: string
    agent?: string
    model?: { providerID: string; modelID: string }
    messageID?: string
    variant?: string
  },
  output: { message: UserMessage; parts: Part[] }
) => Promise<void>
```

**Flow control:** Mutation only. You can modify the user message or inject/remove parts, but you cannot block the message or throw to prevent it from being sent. This makes it useful for:

- Injecting safety instructions into every user message
- Stripping potentially dangerous prompt injection patterns
- Adding context about restricted operations

```typescript
// .opencode/plugins/prompt-guard.ts
import type { Plugin } from "@opencode-ai/plugin"

export const PromptGuard: Plugin = async () => {
  return {
    "chat.message": async (input, output) => {
      // Inject safety reminder into every message
      output.parts.push({
        type: "text",
        text: "\n[SYSTEM SAFETY NOTE: Do not execute destructive commands without confirmation.]",
      } as any)
    },
  }
}
```

**Subagent behavior:** The `chat.message` hook fires when a user message is composed. For subagents (invoked via the `task` tool), the "user message" is the task prompt injected by the parent agent. **The documentation does not explicitly state whether `chat.message` fires for subagent task prompts.** Based on the architecture (subagents run as isolated child sessions), it is likely that plugins loaded globally would fire in subagent sessions too, but this is not confirmed.

### OTHER EVENTS: Additional Safety-Relevant Hooks

#### `tool.execute.after` -- Post-Tool Inspection

Fires after a tool call completes successfully. Cannot block or retry, but can **modify the output** shown to the agent. Useful for redacting sensitive data from tool results.

```typescript
"tool.execute.after": async (input, output) => {
  // Redact secrets from bash output
  output.output = output.output
    .replace(/(?:API_KEY|SECRET|TOKEN|PASSWORD)=\S+/gi, "$1=***REDACTED***")
}
```

**Limitation:** There is no `tool.execute.error` hook. Tool failures can only be observed via the fire-and-forget `event` bus (`session.error` event type).

#### `tool.definition` -- Shape Tool Availability

Fires when tool definitions are assembled for the LLM. Can modify tool descriptions and parameter schemas. This is a powerful defensive hook because it can guide the LLM away from dangerous patterns at the instruction level:

```typescript
"tool.definition": async (input, output) => {
  if (input.toolID === "bash") {
    output.description += `
IMPORTANT RESTRICTIONS:
- Never use sudo or run as root
- Never modify files in /etc, /usr, or /var
- Never use rm -rf on directories outside the project
- Always use git for file operations when possible`
  }
}
```

**Limitation:** This is an influence-based approach (modifying what the LLM "knows" about the tool), not a guarantee. The LLM may still attempt restricted operations.

#### `permission.ask` -- Programmatic Permission Decisions

Fires when the permission system evaluates a tool call to `"ask"`. Can programmatically set the decision to `"allow"`, `"deny"`, or leave as `"ask"` for user prompting. See the `ask-stop` section above for detailed examples.

#### `shell.env` -- Environment Variable Injection

Fires on every shell invocation. Can inject environment variables into all shell executions (bash tool, PTY sessions, subprocesses). Useful for setting `PATH` restrictions or injecting safety-related environment:

```typescript
"shell.env": async (input, output) => {
  // Restrict PATH to known-safe locations
  output.env.PATH = "/usr/local/bin:/usr/bin:/bin"
  // Set a safety flag that scripts can check
  output.env.OPENCODE_RESTRICTED = "true"
}
```

#### `experimental.chat.system.transform` -- System Prompt Injection

Modifies the system prompt before sending to the LLM. Can inject persistent safety instructions:

```typescript
"experimental.chat.system.transform": async (input, output) => {
  output.system.push(`<safety-rules>
You MUST NOT:
- Execute destructive file operations without explicit user confirmation
- Access files outside the project directory
- Use sudo or attempt privilege escalation
- Expose API keys, tokens, or credentials in output
</safety-rules>`)
}
```

**Safety mechanism:** If the `system` array is emptied entirely, OpenCode restores the original system prompt. This prevents accidental removal of essential system instructions.

**Limitation:** This hook is marked **experimental** and may change in future versions.

#### `event` -- Fire-and-Forget Bus Observer

The catch-all `event` hook receives every system bus event (40+ types). It is **fire-and-forget** and cannot affect agent flow. Useful for logging, monitoring, and alerting:

```typescript
event: async ({ event }) => {
  if (event.type === "file.edited") {
    // Log all file edits for audit
    console.error(`[AUDIT] File edited: ${event.properties.file}`)
  }
  if (event.type === "session.error") {
    // Alert on errors
    console.error(`[ALERT] Session error: ${JSON.stringify(event.properties)}`)
  }
}
```

### Sources

- [OpenCode Plugin Documentation](https://opencode.ai/docs/plugins)
- [OpenCode Permissions Documentation](https://opencode.ai/docs/permissions)
- [OpenCode Configuration Documentation](https://opencode.ai/docs/config)
- [Plugin type definitions (source)](https://github.com/anomalyco/opencode/tree/dev/packages/plugin/src/index.ts)

## Intercepting MCP Calls

OpenCode supports [MCP (Model Context Protocol) servers](https://opencode.ai/docs/mcp-servers) for extending agent capabilities. MCP servers are configured in `opencode.json` under the `"mcp"` key.

### MCP Configuration Scopes

| Scope | Location | Notes |
|-------|----------|-------|
| User | `~/.config/opencode/opencode.json` under `"mcp"` | Global user-level servers |
| Project | `opencode.json` or `.opencode/opencode.json` under `"mcp"` | Per-project servers |
| Organization | Remote `.well-known/opencode` endpoint | Organizational defaults fetched at startup |
| Enterprise/Managed | `/Library/Application Support/opencode/opencode.json` (macOS), `/etc/opencode/opencode.json` (Linux), `%ProgramData%\opencode\opencode.json` (Windows) | Admin-controlled, highest priority |

Configuration files are **merged**, not replaced. Later sources override only conflicting keys.

### Transport Types

**Local (stdio) servers:** Supported via `"type": "local"` with a command array.

```jsonc
{
  "mcp": {
    "my-server": {
      "type": "local",
      "command": ["npx", "-y", "@my-org/mcp-server"],
      "environment": {
        "API_KEY": "secret"
      }
    }
  }
}
```

Local MCP binaries do **not** require fully qualified paths. OpenCode uses the `command` array where the first element is resolved via `PATH`. However, for security-sensitive deployments, using fully qualified paths is recommended.

**Remote (HTTP) servers:** Supported via `"type": "remote"` with a URL.

```jsonc
{
  "mcp": {
    "remote-server": {
      "type": "remote",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer {env:MCP_API_KEY}"
      }
    }
  }
}
```

### Environment Variables

Environment variables for local MCP servers are passed via the `"environment"` object in the server config. The `{env:VARIABLE_NAME}` syntax is supported for referencing system environment variables in remote server headers and OAuth configs.

### Authentication

OpenCode supports multiple authentication mechanisms for MCP servers:

- **OAuth:** Automatic OAuth via Dynamic Client Registration (RFC 7591). Detects 401 responses and triggers authentication flows. Tokens stored in `~/.local/share/opencode/mcp-auth.json`. OAuth can be disabled with `"oauth": false`.
- **API keys / Bearer tokens:** Via custom headers using `"headers": {"Authorization": "Bearer {env:API_KEY}"}`.
- **CLI management:** `opencode mcp auth`, `opencode mcp logout`, `opencode mcp debug` commands.

### Intercepting MCP Responses

**OpenCode does NOT provide a dedicated hook or event for intercepting MCP server responses before they are fed back into the agent's processing flow.**

MCP tool calls are processed through the normal tool execution pipeline, which means:

- `tool.execute.before` fires before an MCP tool is called (the tool name includes the MCP server prefix)
- `tool.execute.after` fires after an MCP tool completes, and the `output.output` can be modified

This means you **can** intercept and modify MCP responses via `tool.execute.after`, and you **can** block MCP tool calls via `tool.execute.before`. However, there is no MCP-specific event that gives access to the raw MCP protocol response before it is processed into the tool result format.

```typescript
"tool.execute.after": async (input, output) => {
  // MCP tools are prefixed with the server name
  if (input.tool.startsWith("my-mcp-server_")) {
    // Scan and redact MCP response content
    if (output.output.includes("SECRET")) {
      output.output = "[REDACTED: MCP response contained sensitive data]"
    }
  }
}
```

### Enterprise-Level MCP Controls

The enterprise/managed config can define MCP servers that take highest precedence. Organizations can provide default MCP servers via the `.well-known/opencode` remote config endpoint. **There is no documented mechanism for explicit MCP allow-listing or deny-listing at the enterprise level.** The managed config can set `"enabled": false` on specific servers to disable them, and organizations can control which servers are available by defining them in managed config. However, project-level configs can add additional MCP servers that are not controlled by the enterprise config.

Additionally, per-agent tool management using glob patterns (e.g., `"my-mcp*": false`) can disable MCP tools within specific agent configurations.

### Sources

- [OpenCode MCP Servers Documentation](https://opencode.ai/docs/mcp-servers)
- [OpenCode Configuration Documentation](https://opencode.ai/docs/config)
- [OpenCode Enterprise Documentation](https://opencode.ai/docs/enterprise)

## Completion Gates

### Completion Events

OpenCode fires the following events when the agent considers its work complete:

1. **`session.idle`** (bus event): Fires when a session becomes idle after completing work. This is a **deprecated** event; `session.status` is preferred.
2. **`session.status`** (bus event): Fires when session status changes, including transitions to idle/complete states.

Both of these are **fire-and-forget bus events** accessible only through the catch-all `event` hook. They **cannot be blocked** -- there is no mechanism to prevent the agent from stopping or to force it to continue working.

### Can Completion Be Blocked?

**No.** OpenCode's completion events are informational notifications on the event bus. The `event` hook returns `Promise<void>` and cannot influence agent flow. There is no equivalent to Claude Code's `Stop` hook that can return instructions to force the agent to continue.

### Running External Commands on Completion

While you cannot block completion, you can run external commands in response to completion events:

```typescript
// .opencode/plugins/completion-gate.ts
import type { Plugin } from "@opencode-ai/plugin"
import { execSync } from "child_process"

export const CompletionGate: Plugin = async ({ $ }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.status" && event.properties.status === "idle") {
        try {
          // Run tests or secret scanner
          execSync("npm test", { timeout: 60000 })
          execSync("git secrets --scan", { timeout: 30000 })
        } catch (error) {
          // Cannot force the agent to continue -- only log the failure
          console.error(`[COMPLETION GATE] Validation failed: ${error}`)
          // Could send a notification, write to a file, etc.
        }
      }
    },
  }
}
```

### Main Agent vs. Subagent Completion

There are no separate events for main agent completion vs. subagent completion. The `session.status` and `session.idle` events fire for both. Since subagents run as isolated child sessions, each subagent session will produce its own idle/status events.

### Injecting Feedback on Completion

**Not possible via hooks.** Since completion events are fire-and-forget, there is no mechanism to inject feedback or instructions back into the agent. The agent has already decided it is done.

**Workaround:** For non-interactive/CI usage via `opencode run`, you could implement a wrapper script that:
1. Runs `opencode run` with the initial prompt
2. Checks the output/changes for validation
3. If validation fails, runs `opencode run` again with a follow-up prompt

This is external orchestration, not a built-in feature.

### Loop Protection

Since completion events cannot be blocked, the infinite loop problem does not arise. OpenCode does have a built-in `doom_loop` permission that triggers after 3 repetitions of identical tool calls (defaulting to `"ask"`), which provides protection against the agent getting stuck in a loop during execution (not at completion).

### Sources

- [OpenCode Plugin Documentation](https://opencode.ai/docs/plugins)
- [Plugin type definitions (source)](https://github.com/anomalyco/opencode/tree/dev/packages/plugin/src/index.ts)
- [Bus event system (source)](https://github.com/anomalyco/opencode/tree/dev/packages/opencode/src/bus)

## Subagents as Security Event?

### Detecting Subagent Creation

Yes, subagent creation can be detected reliably. OpenCode spawns subagents via the `task` tool. Since `tool.execute.before` fires for **all** tool calls including `task`, you can intercept subagent creation:

```typescript
"tool.execute.before": async (input, output) => {
  if (input.tool === "task") {
    const targetAgent = output.args.agent ?? "unknown"
    const prompt = output.args.prompt ?? ""
    console.error(`[SECURITY] Subagent creation: agent=${targetAgent}, prompt=${prompt}`)

    // Optionally block certain subagent invocations
    if (targetAgent === "untrusted-agent") {
      throw new Error("Blocked: untrusted subagent invocation")
    }
  }
}
```

### Do Hooks Fire Inside Subagents?

**This is not explicitly documented.** OpenCode's plugin documentation does not distinguish between main agent and subagent contexts for hook execution. Based on the architecture:

- Subagents run as **isolated child sessions** with independent context
- Plugins are loaded globally (from `~/.config/opencode/plugins/`) and per-project (from `.opencode/plugins/`)
- Since plugins are loaded at the server/process level (not per-session), it is **likely** that `tool.execute.before` and other hooks fire inside subagent sessions too

However, this has not been definitively confirmed in documentation or testing. **Treat this as "likely yes" with medium confidence.** The Claudine codebase models this mapping as supported (OpenCode has `Hook` support level for `before_tool` and `after_tool`), suggesting hooks fire in both contexts.

### Subagent Permission Controls

**Yes.** OpenCode provides granular control over subagent permissions:

1. **Task tool permissions:** Control which subagents can be invoked:

```jsonc
{
  "permission": {
    "task": {
      "*": "deny",
      "code-reviewer": "allow",
      "deploy-*": "ask"
    }
  }
}
```

2. **Per-agent tool permissions:** Each agent definition (`.opencode/agents/*.md`) can specify its own `tools` and `permission` fields in frontmatter:

```yaml
---
description: Read-only code reviewer
mode: subagent
tools:
  write: false
  edit: false
  bash: false
permission:
  read: allow
  glob: allow
  grep: allow
  task: deny
---
```

3. **Per-agent MCP controls:** Agent definitions can disable specific MCP tools using glob patterns in the `tools` field. This effectively limits MCP access per subagent, but there is no "read-only MCP" mode -- you either enable or disable MCP tools entirely.

### Reducing Shell/Filesystem Access for Subagents

Yes, this is supported through the agent definition frontmatter:

```yaml
---
description: Safe research agent
mode: subagent
tools:
  bash: false
  edit: false
  write: false
  external_directory: false
permission:
  read: allow
  glob: allow
  grep: allow
---
```

### Injecting Context into Subagents

Partially supported:

- The **task prompt** is the primary mechanism for passing context from parent to subagent
- Agent definitions can include a `prompt` field for a custom system prompt
- The `experimental.chat.system.transform` hook can inject system-level context that applies to all sessions (including subagent sessions, if plugins fire there)
- There is no mechanism to inject per-invocation context into a specific subagent session beyond what the parent includes in the task prompt

### Sources

- [OpenCode Agents Documentation](https://opencode.ai/docs/agents)
- [OpenCode Permissions Documentation](https://opencode.ai/docs/permissions)
- [OpenCode Plugin Documentation](https://opencode.ai/docs/plugins)

## Escalated Privileges

### Root/Elevated Privilege Detection

**OpenCode does NOT automatically detect or warn about running as root or with elevated privileges.** There is no built-in check for `uid === 0`, no warning banner, and no configuration option to enforce non-root execution.

### Detection via Hooks

You can implement privilege detection in a plugin:

```typescript
// .opencode/plugins/privilege-check.ts
import type { Plugin } from "@opencode-ai/plugin"
import { execSync } from "child_process"

export const PrivilegeCheck: Plugin = async () => {
  // Check at plugin load time (startup)
  const uid = process.getuid?.()
  if (uid === 0) {
    console.error("[WARNING] OpenCode is running as root! This is dangerous.")
    // Could also inject a system prompt warning via experimental.chat.system.transform
  }

  return {
    "experimental.chat.system.transform": async (input, output) => {
      if (process.getuid?.() === 0) {
        output.system.push(
          "WARNING: You are running as root. Exercise extreme caution with all file and system operations."
        )
      }
    },
  }
}
```

### Sandbox / Container-Based Isolation

**OpenCode does NOT provide built-in sandboxing or container-based isolation.** There is no `--sandbox` flag, no Docker integration for isolated execution, and no filesystem jail. All tools execute with the full permissions of the user running OpenCode.

### Filesystem Write Path Restrictions

**Not natively supported.** OpenCode does not provide a configuration option to restrict filesystem write paths. However, you can implement path restrictions via the `tool.execute.before` hook:

```typescript
"tool.execute.before": async (input, output) => {
  const ALLOWED_PATHS = ["/home/user/project", "/tmp"]

  if (input.tool === "edit" || input.tool === "write") {
    const filePath = output.args.filePath ?? ""
    const isAllowed = ALLOWED_PATHS.some(p => filePath.startsWith(p))
    if (!isAllowed) {
      throw new Error(`Blocked: writing to ${filePath} is outside allowed paths`)
    }
  }
}
```

Additionally, OpenCode has a built-in `external_directory` permission (defaults to `"ask"`) that prompts the user when the agent attempts to access files outside the project workspace.

### Network Access Restrictions

**Not natively supported.** OpenCode does not provide configuration options to restrict network access. Shell commands, MCP servers, and LLM API calls all execute with the user's full network access. You could restrict network access at the OS level (firewall rules, network namespaces) or via the `shell.env` hook to set restrictive proxy environment variables.

### Bypass Permission Mode

**OpenCode does NOT have a "yolo" or "dangerously skip permissions" mode.** There is no CLI flag or configuration option that bypasses the permission system entirely. The closest configuration is setting all permissions to `"allow"`:

```jsonc
{
  "permission": {
    "*": "allow"
  }
}
```

This permits all operations without user prompting, but it is an explicit configuration choice, not a bypass mode. There is no `--force`, `--trust`, or `--dangerously-skip-permissions` CLI flag.

The `--permission` CLI flag (and `OPENCODE_PERMISSION` env var) allows inline JSON permission configuration, but it still uses the normal `allow`/`ask`/`deny` system -- it does not bypass it.

### Sources

- [OpenCode CLI Documentation](https://opencode.ai/docs/cli)
- [OpenCode Permissions Documentation](https://opencode.ai/docs/permissions)
- [OpenCode Configuration Documentation](https://opencode.ai/docs/config)
- [OpenCode Enterprise Documentation](https://opencode.ai/docs/enterprise)
