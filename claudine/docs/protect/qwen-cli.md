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
agent_version: "0.10.5"
has_blocking_pre_tool_event: true
pre_tool_influence: guarantee
pre_tool_actions:
    - stop
    - exit
    - ask-stop
    - ask-exit
pre_tool_subagent: false
user_prompt_event: false
other_events:
    SubagentHooks.preToolUse: "Internal only. Fires before subagent tool execution. Void return, fire-and-forget, not awaited, errors swallowed. Cannot block or modify. Not user-configurable. Useful only for instrumentation in custom builds."
    SubagentHooks.postToolUse: "Internal only. Fires after subagent tool execution. Void return, awaited but non-blocking. Not user-configurable. Useful only for instrumentation in custom builds."
    SubagentHooks.onStop: "Internal only. Fires when a subagent terminates. Void return, awaited but non-blocking. Provides terminateReason and summary fields. Not user-configurable."
    headless.stream-json: "Output-only line-delimited JSON events in headless mode (--output-format stream-json). Includes session_start, assistant messages with tool calls, and result events. No input channel; cannot block, modify, or feed back into agent. Useful for monitoring and post-hoc analysis only."
mcp_supported: true
mcp_docs: "https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server/"
mcp_config_user: "~/.qwen/settings.json"
mcp_config_repo: ".qwen/settings.json"
mcp_event: false
mcp_event_name: "n/a"
mcp_event_modifiable: false
mcp_event_stop: false
has_completion_event: false
completion_event_blocking: false
completion_event_names: []
completion_loop_protection: false
has_subagent_events: false
hooks_fire_in_subagents: null
subagent_permissions_configurable: true
has_sandbox: true
detects_elevated_privileges: false
has_bypass_mode: true
last_updated: "2026-02-20"
body_hash: 5596940524371119455
---

# Protecting Qwen CLI

> **Agent version:** 0.10.5 (February 2026)
> **CLI binary:** `qwen` (npm package: `@qwen-code/qwen-code`)
> **Homepage:** https://github.com/QwenLM/qwen-code
> **Documentation:** https://qwenlm.github.io/qwen-code-docs/

Qwen Code is an open-source agentic CLI from Alibaba, forked from Gemini CLI and optimized for Qwen3-Coder models. As of v0.10.5, Qwen Code **does not have a user-facing lifecycle hook system** comparable to Claude Code's `PreToolUse`/`PostToolUse`/`Stop` hooks in `settings.json`. The [roadmap lists hooks as "In Progress" (P2 priority)](https://github.com/QwenLM/qwen-code/issues/268), and maintainers have confirmed they are "still in development" ([issue #1708](https://github.com/QwenLM/qwen-code/issues/1708)). This document covers the protection surfaces that **do** exist today and their limitations.

---

## Event Hooks

### Pre-Tool: SDK `canUseTool` Callback (SDK-Only)

Qwen Code's only blocking pre-tool mechanism is the `canUseTool` callback, available exclusively through the [TypeScript SDK](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) (`@qwen-code/sdk`). It is **not** configurable via `settings.json`, CLI flags, or the extension system. You must write a Node.js/TypeScript program that invokes the SDK's `query()` function and provides the callback.

**Callback signature:**

```typescript
type CanUseTool = (
  toolName: string,
  input: Record<string, unknown>,
  options: {
    signal: AbortSignal;
    suggestions?: PermissionSuggestion[] | null;
  },
) => Promise<PermissionResult>;

type PermissionResult =
  | { behavior: 'allow'; updatedInput: Record<string, unknown> }
  | { behavior: 'deny'; message: string; interrupt?: boolean };
```

**How it works:**

- When the permission system requires confirmation for a tool call, the callback is invoked with the tool name, input arguments, and an abort signal.
- The callback **must** resolve within 60 seconds or the tool call is auto-denied. This timeout is not configurable.
- Returning `{ behavior: 'allow', updatedInput }` permits execution, optionally rewriting the tool's input arguments before they run.
- Returning `{ behavior: 'deny', message }` blocks the specific tool call. The denial `message` is surfaced to the model so it can adjust its approach.
- Returning `{ behavior: 'deny', message, interrupt: true }` halts the **entire session**, not just the current tool call.

**Influence level: Guarantee.** The callback's return value deterministically controls whether the tool executes. There is no probabilistic element -- `deny` always blocks, `allow` always proceeds.

**Critical limitation -- the callback is bypassed in several scenarios:**

| Condition | Callback invoked? |
|:----------|:------------------|
| `permissionMode: 'yolo'` | No -- all tools auto-approved |
| Tool in `allowedTools` list | No -- auto-approved |
| Tool in `excludeTools` list | No -- auto-denied before callback |
| `permissionMode: 'plan'` (non-read-only tool) | No -- auto-denied |
| `permissionMode: 'default'` (write tool) | **Yes** |
| `permissionMode: 'auto-edit'` (shell command) | **Yes** |

The priority chain is: `excludeTools` > `plan` mode > `yolo` mode > `allowedTools` > `canUseTool` callback > default deny.

[SDK documentation](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/) | [Hooks feature request (issue #268)](https://github.com/QwenLM/qwen-code/issues/268)

#### Action: `stop` (Block Current Tool Call)

To block a specific tool call while allowing the agent to continue working, return a `deny` response without the `interrupt` flag:

```typescript
import { QwenCode } from '@qwen-code/sdk';

const qwen = new QwenCode({
  canUseTool: async (toolName, input, { signal }) => {
    // Block destructive git operations
    if (toolName === 'run_shell_command') {
      const cmd = String(input.command ?? '');
      if (/git\s+(push\s+--force|reset\s+--hard|clean\s+-f)/.test(cmd)) {
        return {
          behavior: 'deny',
          message: `Blocked dangerous command: ${cmd}. Use a safer alternative.`,
        };
      }
    }
    // Allow everything else
    return { behavior: 'allow', updatedInput: input };
  },
});
```

**Nuances:**
- The denial message is fed back to the model as context, so it can attempt an alternative approach.
- The agent continues working after the denial -- only this specific tool call is blocked.
- If you want the model to receive guidance, make the `message` descriptive (e.g., "Use `git push` without `--force` instead").

#### Action: `exit` (Halt the Entire Session)

To stop the agent's work entirely, return a `deny` response with `interrupt: true`:

```typescript
canUseTool: async (toolName, input, { signal }) => {
  if (toolName === 'run_shell_command') {
    const cmd = String(input.command ?? '');
    if (/rm\s+-rf\s+\//.test(cmd)) {
      return {
        behavior: 'deny',
        message: 'CRITICAL: Attempted to delete root filesystem. Session terminated.',
        interrupt: true,
      };
    }
  }
  return { behavior: 'allow', updatedInput: input };
},
```

**Nuances:**
- `interrupt: true` halts the entire session immediately, propagating to any parent process.
- The SDK's `Query` instance also exposes an `interrupt()` method that can be called from external code to halt the session mid-execution.

#### Action: `ask-stop` and `ask-exit` (User Approval)

There is no built-in "ask the user" mechanism within the `canUseTool` callback itself. However, since the callback is async TypeScript code, you can implement user prompting manually:

```typescript
import * as readline from 'readline';

function askUser(question: string): Promise<boolean> {
  const rl = readline.createInterface({ input: process.stdin, output: process.stderr });
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.toLowerCase().startsWith('y'));
    });
  });
}

canUseTool: async (toolName, input, { signal }) => {
  if (toolName === 'write_file' || toolName === 'run_shell_command') {
    const desc = toolName === 'run_shell_command'
      ? `Shell: ${input.command}`
      : `Write: ${input.path}`;
    const approved = await askUser(`[APPROVAL] ${desc} — allow? (y/n): `);
    if (!approved) {
      return { behavior: 'deny', message: 'User denied this operation.' };
      // For ask-exit: add `interrupt: true` to halt the session
    }
  }
  return { behavior: 'allow', updatedInput: input };
},
```

**Nuances:**
- The 60-second timeout applies. If the user does not respond within 60 seconds, the tool call is auto-denied.
- This pattern blocks the SDK's event loop waiting for user input, which may cause issues in CI/headless contexts. Use only in interactive SDK consumers.

### Pre-Tool in Subagents

The `canUseTool` callback **does** apply to subagent tool calls, but only indirectly. Subagents execute within the same SDK `Query` instance, so the permission system (including `allowedTools`, `excludeTools`, and `permissionMode`) gates their tool calls through the same pipeline. However, the SDK documentation does not explicitly confirm that the `canUseTool` callback is invoked for subagent-initiated tool calls as opposed to being short-circuited by the subagent's own tool list.

**Conflicting information:** The internal `SubagentHooks` interface (documented in `packages/core/src/subagents/subagent-hooks.ts`) has its own `preToolUse` hook that fires for subagent tool calls, but this hook is notification-only (`void` return) and cannot block or modify execution. It is not exposed to users. This creates an asymmetry: the SDK callback might gate subagent tools at the permission level, but the internal pre-tool hook within subagents cannot block. The exact interaction between these two systems is not fully documented.

### User Prompt Event

**Not supported.** Qwen Code does not provide any event or callback that fires when a user submits a prompt, either in the CLI or the SDK. There is no equivalent to Claude Code's `UserPromptSubmit` hook. User prompts are processed directly without interception.

### No CLI-Level Hook System (settings.json)

As of v0.10.5, the `settings.json` schema does not include a `hooks` key. Adding one has no effect ([issue #1708](https://github.com/QwenLM/qwen-code/issues/1708)). The [Claude-to-Qwen extension converter](https://github.com/QwenLM/qwen-code) recognizes hooks in imported Claude extensions but logs a warning and drops them silently. When the hook system ships (roadmap P2), the configuration will likely follow the Claude/Gemini pattern with a `hooks` key in `settings.json`, but no schema is finalized.

**Configuration format:** JSON (`settings.json`)
**Configuration locations:**

| Scope | Path |
|:------|:-----|
| User | `~/.qwen/settings.json` |
| Project | `.qwen/settings.json` |

**Hooks cannot be defined inline** in skills, agents, commands, or extension manifests. The extension system (`qwen-extension.json`) supports MCP servers, commands, skills, and subagents but does not support lifecycle hook registration.

### Other Events Useful for Safety

#### Internal `SubagentHooks` (Not User-Configurable)

The `SubagentHooks` interface exists in Qwen Code's core package (`packages/core/src/subagents/subagent-hooks.ts`) but is **internal only** -- not exposed through settings, CLI, SDK, or extensions.

| Hook | Trigger | Return | Blocking? | Awaited? |
|:-----|:--------|:-------|:----------|:---------|
| `preToolUse` | Before subagent tool execution | `void` | No | No (fire-and-forget) |
| `postToolUse` | After subagent tool execution | `void` | No | Yes |
| `onStop` | Subagent terminates | `void` | No | Yes |

All three are notification-only. `preToolUse` errors are silently swallowed because the callback is not awaited. These hooks cannot block, modify, or control agent flow. They are useful only for instrumentation and telemetry within custom Qwen Code builds.

#### Headless Stream-JSON Events (Output-Only)

Running Qwen Code in headless mode with `--output-format stream-json` emits line-delimited JSON events that can be consumed by external processes:

| Event type | Description | Defensive use |
|:-----------|:------------|:--------------|
| `system` (subtype: `session_start`) | Session metadata | Audit session configuration |
| `assistant` | Full assistant message with tool calls | Post-hoc analysis of tool calls |
| `result` (subtype: `success` or error) | Final status with duration and usage | Detect failures, trigger alerts |

**Flow impact:** None. These are output-only events with no input channel. You cannot feed responses back into the agent or block execution based on stream events. They are useful only for **monitoring and post-hoc analysis**, not real-time protection.

#### Approval Modes as Passive Protection

While not an event system, the four [approval modes](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/) provide passive tool gating:

| Mode | File edits | Shell commands |
|:-----|:-----------|:---------------|
| `plan` | Blocked | Blocked |
| `default` | Manual approval | Manual approval |
| `auto-edit` | Auto-approved | Manual approval |
| `yolo` | Auto-approved | Auto-approved |

Combine with `tools.exclude` in `settings.json` to deny specific tools:

```json
{
  "tools": {
    "exclude": ["run_shell_command"]
  }
}
```

**Warning:** The documentation explicitly states that "command-specific restrictions in `tools.exclude` for `run_shell_command` are **not a security mechanism**" because they use simple string matching and can be easily bypassed. Use `tools.core` (an allowlist) for stronger restrictions.

---

## Intercepting MCP Calls

### MCP Server Configuration

Qwen Code fully supports MCP servers. Configuration lives in the `mcpServers` object within `settings.json` at two scopes:

| Scope | Path |
|:------|:-----|
| User | `~/.qwen/settings.json` |
| Project | `.qwen/settings.json` |

MCP servers can also be managed via CLI subcommands:

```bash
qwen mcp add <name> <command-or-url> [args..]
qwen mcp remove <name>
qwen mcp list
```

[MCP server documentation](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server/)

### Transport Types

All three MCP transport types are supported:

| Transport | Config key | Description |
|:----------|:-----------|:------------|
| Stdio | `command` | Spawns a subprocess communicating via stdin/stdout |
| SSE | `url` | Connects to a Server-Sent Events endpoint |
| Streamable HTTP | `httpUrl` | HTTP streaming endpoint |

### Environment Variables

Environment variables are passed to MCP servers via the `env` field in the server configuration. Variable interpolation is supported using `$VAR_NAME` or `${VAR_NAME}` syntax:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "npx",
      "args": ["-y", "@my/mcp-server"],
      "env": {
        "API_KEY": "${MY_API_KEY}",
        "DEBUG": "true"
      }
    }
  }
}
```

### Local Binary Paths

Local MCP binaries specified via `command` do **not** require fully qualified paths when the binary is available on `$PATH` (e.g., `"command": "npx"`). However, for binaries not on the system path, a fully qualified path is needed. The optional `cwd` field sets the working directory for the subprocess.

### Authentication

Qwen Code supports multiple authentication regimes for MCP servers:

- **OAuth 2.0:** Automatic discovery or manual configuration with `clientId`, `clientSecret`, `authorizationUrl`, `tokenUrl`, and `scopes`. Tokens are stored in `~/.qwen/mcp-oauth-tokens.json`.
- **Google Cloud credentials:** `authProviderType: 'google_credentials'` uses Application Default Credentials.
- **Service account impersonation:** `authProviderType: 'service_account_impersonation'` with `targetServiceAccount` and `targetAudience`.
- **API keys / Bearer tokens:** Passed via custom `headers` in the server configuration (e.g., `"Authorization": "Bearer <token>"`).

### Tool Filtering

Per-server tool filtering is available:

- **`includeTools`** (array): Allowlist -- only these tools from the server are exposed to the model.
- **`excludeTools`** (array): Blocklist -- these tools are hidden. `excludeTools` takes precedence over `includeTools`.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/safe/path"],
      "excludeTools": ["write_file", "delete_file"],
      "trust": false
    }
  }
}
```

### Trust Setting

The `trust` boolean on a server configuration bypasses all confirmation dialogs for that server's tools:

```bash
qwen mcp add --trust my-server npx @my/trusted-server
```

This is equivalent to adding the server's tools to `allowedTools`. Use with extreme caution as it bypasses the `canUseTool` callback entirely.

### Intercepting MCP Responses

**Not supported.** Qwen Code does not provide any event, hook, or callback that fires when an MCP server returns a response. MCP tool results are injected directly into the model context (split into `llmContent` for the model and `returnDisplay` for the user) without any interception point. There is no way to:

- Inspect an MCP response before the model processes it
- Modify an MCP response before it enters the context
- Block the agent flow based on MCP response content

**Workaround:** Build MCP servers that perform their own response sanitization internally. Alternatively, wrap untrusted MCP servers behind a proxy MCP server that inspects and filters responses before forwarding them.

### Enterprise/Managed Allow-listing

There is no documented enterprise or managed scope for MCP server configuration. MCP servers are configured at user and project scope only. There is no mechanism to enforce an organization-wide MCP server allowlist or denylist.

---

## Completion Gates

### Completion Events

**No user-facing completion events exist.** Qwen Code does not fire a hook or callback when the agent considers its work complete. There is no equivalent to Claude Code's `Stop` event or Gemini CLI's turn-completion hook.

The closest surfaces are:

1. **Internal `SubagentHooks.onStop`**: Fires when a subagent terminates. Notification-only (`void` return), cannot block the subagent from stopping, and is not user-configurable. Provides `terminateReason` and `summary` fields.

2. **Headless `StreamResult` event**: Emitted at the end of a headless session with `--output-format stream-json`. Contains `subtype: "success"` or an error description, `duration_ms`, and usage stats. This is output-only with no input channel -- you cannot feed instructions back into the agent.

### Blocking Completion

**Not possible.** Neither the internal `onStop` hook nor the headless `StreamResult` event supports blocking. There is no way to force the agent to continue working when it decides to stop.

### Running External Validation

**Not directly supported.** Since there are no completion hooks, you cannot attach test suites, linters, or secret scanners to a completion event. The workarounds are:

1. **Headless wrapper script:** Run `qwen` in headless mode, parse the stream-json output, run validation after the session ends, and re-invoke `qwen` with feedback if validation fails:

   ```bash
   #!/bin/bash
   qwen -o stream-json "Implement feature X" > session.jsonl
   # Run validation
   if ! npm test; then
     qwen -o stream-json "Tests failed. Fix the following errors: $(npm test 2>&1)"
   fi
   ```

2. **SDK loop:** Use the TypeScript SDK to run the agent, check results, and re-invoke with additional instructions programmatically.

### Infinite Loop Protection

Not applicable since completion cannot be blocked.

### Separate Events for Main vs. Subagent Completion

The internal `SubagentHooks.onStop` fires only for subagent termination. There is no equivalent event for the main agent's completion. The headless `StreamResult` event fires once at the end of the entire session, not per-turn or per-subagent.

### Injecting Feedback

**Not possible within a single session.** There is no mechanism to inject feedback or instructions back into the agent at completion time. The only approach is to start a new session (or use `--continue`) with additional context.

---

## Subagents as Security Event?

### Detecting Subagent Creation

**No user-facing event.** There is no event, hook, or callback that fires when a subagent is created. The internal `SubagentHooks` interface only has `preToolUse`, `postToolUse`, and `onStop` -- none of which fire at creation time. The headless stream-json output does not include a dedicated subagent-creation event type.

### Pre-Tool and Post-Tool Hooks in Subagents

The situation is nuanced:

- **Internal `SubagentHooks.preToolUse` / `postToolUse`**: These fire for every tool call within a subagent. However, they are notification-only (`void` return), fire-and-forget (for `preToolUse`), and not user-configurable. They **cannot** block or modify tool calls.

- **SDK `canUseTool` callback**: The permission system applies to the entire `Query` instance. Subagents run within the same instance, so the permission mode, `allowedTools`, and `excludeTools` settings apply. Whether the `canUseTool` callback itself is invoked for subagent tool calls is not explicitly documented. Given that subagents operate within the same permission pipeline, it is likely that the callback fires when a subagent tool call requires confirmation, but this is not confirmed.

### Restricting Subagent Permissions

**Yes, partially.** Subagent tool access is configured in the agent definition file's YAML frontmatter:

```yaml
---
name: doc-reviewer
description: Reviews documentation for accuracy
tools:
  - read_file
  - read_many_files
  - grep
---
You are a documentation reviewer. Only read and analyze files.
```

The `tools` array restricts which tools the subagent can access. This is an effective allowlist -- the subagent cannot use tools not listed in its configuration. However:

- This is an **agent-level** restriction, not a **permission-level** restriction. It controls tool visibility, not tool approval.
- Project-level `tools.core` restrictions in `.qwen/settings.json` override agent-level tool declarations, which can cause "tool not found" errors ([issue #792](https://github.com/QwenLM/qwen-code/issues/792)).
- There is no way to set a different `permissionMode` for a subagent (e.g., forcing `default` mode for subagents while the main agent runs in `auto-edit`).

### Limiting MCP Servers in Subagents

**Not directly configurable.** All MCP servers configured in `settings.json` are available to all agents and subagents. There is no per-subagent MCP server filtering. The `tools` array in a subagent's definition can exclude MCP tool names, but this requires knowing the exact tool names registered by each MCP server.

**Workaround:** Configure MCP servers with restrictive `includeTools` at the server level, or use separate `settings.json` profiles for different risk levels.

### Reducing Shell/Filesystem Access for Subagents

**Yes.** Omit `run_shell_command` and `write_file` from the subagent's `tools` array:

```yaml
---
name: analyzer
description: Reads and analyzes code without making changes
tools:
  - read_file
  - read_many_files
  - grep
  - list_directory
---
Analyze code structure. Do not modify any files.
```

This is effective but relies on the model respecting the tool list. The tool is simply not available to the subagent, so the model cannot call it.

### Injecting Context at Subagent Creation

**Yes.** The body content of the agent definition file serves as the subagent's system prompt. This is injected at creation time and persists for the subagent's entire execution. You can include security instructions:

```yaml
---
name: safe-coder
description: Writes code with security constraints
tools:
  - read_file
  - write_file
---
SECURITY CONSTRAINTS:
- Never write credentials, API keys, or secrets to files
- Never execute shell commands
- Always validate input before writing files
- Never modify files outside the project root
```

Additionally, the orchestrator (main agent) provides task context when delegating to a subagent.

---

## Escalated Privileges

### Root / Elevated Privilege Detection

**Not supported.** Qwen Code does not detect or warn about running as root or with elevated privileges. No documentation, source code references, or GitHub issues indicate this feature exists. Running `qwen` as root executes with full root permissions without any additional warnings or safeguards.

### Sandboxing

**Yes.** Qwen Code provides comprehensive [sandboxing](https://qwenlm.github.io/qwen-code-docs/en/users/features/sandbox/) inherited from its Gemini CLI heritage. Two isolation methods are available:

#### macOS Seatbelt (macOS only)

Lightweight sandboxing using `sandbox-exec` with configurable profiles:

| Profile | Writes | Network |
|:--------|:-------|:--------|
| `permissive-open` (default) | Restricted to project directory | Allowed |
| `permissive-closed` | Restricted to project directory | Blocked |
| `permissive-proxied` | Restricted to project directory | Via proxy |
| `restrictive-open` | More limited | Allowed |
| `restrictive-closed` | More limited | Blocked |
| `restrictive-proxied` | More limited | Via proxy |

Custom profiles can be defined in `.qwen/sandbox-macos-<profile_name>.sb`.

#### Docker/Podman (Cross-platform)

Complete process isolation in a container:

```bash
qwen -s -p "analyze this project"
qwen --sandbox --sandbox-image my-custom-image "review code"
```

The container mounts the workspace and `~/.qwen/` directory, so auth and settings persist. Custom Dockerfiles can be placed at `.qwen/sandbox.Dockerfile`.

**Enabling sandbox mode:**

| Method | Example |
|:-------|:--------|
| CLI flag | `qwen -s` or `qwen --sandbox` |
| Environment variable | `GEMINI_SANDBOX=true` (also `docker`, `podman`, `sandbox-exec`) |
| Settings | `tools.sandbox: true` in `settings.json` |

**Note:** Some environment variables retain the `GEMINI_*` prefix for backward compatibility with the Gemini CLI fork origin.

### Filesystem Write Path Restrictions

**Yes**, through sandboxing:

- **Seatbelt profiles** restrict writes outside the project directory.
- **Docker containers** limit filesystem access to the mounted workspace and `~/.qwen/`.
- **`tools.core`** allowlist can restrict tools to specific commands (e.g., `"run_shell_command(ls -l)"` permits only `ls -l`), though this is string-matching-based and not a security boundary.

### Network Access Restrictions

**Yes**, through sandboxing:

- **Seatbelt "closed" profiles** block all outbound network access.
- **Seatbelt "proxied" profiles** route traffic through a local proxy configured via `GEMINI_SANDBOX_PROXY_COMMAND`, enabling allowlist-style filtering.
- **Docker containers** can have network restrictions applied via `SANDBOX_FLAGS` (e.g., `--network=none`).

### Bypass Mode (YOLO)

**Yes.** YOLO mode (`--yolo` / `-y` / `--approval-mode yolo`) auto-approves all tool calls without any confirmation:

- File edits: auto-approved
- Shell commands: auto-approved
- `canUseTool` callback: **never invoked**
- MCP tools with `trust: false`: still require confirmation in the CLI, but trusted servers bypass this

**Safeguards around YOLO mode:**

1. The documentation warns: "Use YOLO Mode with caution: AI can execute any command with your terminal permissions."
2. YOLO can be combined with `--sandbox` to auto-approve within an isolated container, reducing risk.
3. `tools.exclude` still applies in YOLO mode (tools are denied before the approval check).
4. `tools.core` allowlist still restricts available tools in YOLO mode.
5. The mode is visually indicated in the TUI footer and can be toggled with Shift+Tab.

**Recommended safe configuration for automation:**

```bash
# YOLO inside a sandboxed container with network isolation
GEMINI_SANDBOX=docker SANDBOX_FLAGS="--network=none" qwen -y -p "refactor auth module"
```

---

## Summary of Protection Capabilities

| Capability | Supported? | Notes |
|:-----------|:-----------|:------|
| Blocking pre-tool hook (CLI) | No | Roadmap P2, not yet shipped |
| Blocking pre-tool hook (SDK) | **Yes** | `canUseTool` callback, 60s timeout |
| User prompt interception | No | No event exists |
| MCP response interception | No | Responses go directly to model |
| Completion gate (block stop) | No | No completion event exists |
| Subagent creation event | No | No event fires |
| Subagent tool restriction | **Yes** | Via `tools` array in agent definition |
| Sandbox isolation | **Yes** | Seatbelt (macOS) + Docker/Podman |
| Network restriction | **Yes** | Via sandbox profiles |
| Root detection | No | No warning or detection |
| YOLO bypass mode | **Yes** | `--yolo` bypasses all approvals |

## Sources

- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code documentation](https://qwenlm.github.io/qwen-code-docs/)
- [Qwen Code settings reference](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code SDK (TypeScript)](https://qwenlm.github.io/qwen-code-docs/en/developers/sdk-typescript/)
- [Qwen Code MCP server docs](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server/)
- [Qwen Code sandbox docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/sandbox/)
- [Qwen Code approval mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code subagents docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/)
- [Qwen Code headless mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Hook feature request (issue #268)](https://github.com/QwenLM/qwen-code/issues/268)
- [Hooks not working (issue #1708)](https://github.com/QwenLM/qwen-code/issues/1708)
- [Subagent tool restriction issue (#792)](https://github.com/QwenLM/qwen-code/issues/792)
- [npm: @qwen-code/qwen-code](https://www.npmjs.com/package/@qwen-code/qwen-code)
