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
agent_version: "1.25.0"
has_blocking_pre_tool_event: false
pre_tool_influence: "n/a"
pre_tool_actions: []
pre_tool_subagent: false
user_prompt_event: false
mcp_supported: true
mcp_docs: "https://block.github.io/goose/docs/getting-started/using-extensions/"
mcp_config_user: "~/.config/goose/config.yaml"
mcp_config_repo: "n/a"
mcp_event: false
mcp_event_name: "n/a"
mcp_event_modifiable: false
mcp_event_stop: false
has_completion_event: true
completion_event_blocking: false
completion_event_names:
    - "complete"
    - "waiting"
completion_loop_protection: true
has_subagent_events: true
hooks_fire_in_subagents: null
subagent_permissions_configurable: true
has_sandbox: true
detects_elevated_privileges: false
has_bypass_mode: true
last_updated: "2026-02-20"
body_hash: 10419402813387466384
other_events:
    GOOSE_STATUS_HOOK: "Fires on agent state transitions between 'waiting' and 'thinking'. Fire-and-forget only; exit code ignored, stdout/stderr suppressed. Cannot block or modify execution. Useful only for external monitoring (status indicators, notifications). Configured in ~/.config/goose/config.yaml."
    stream-json/message: "Emitted during goose run --output-format stream-json. Contains agent response or tool request/response content. Outbound-only, no blocking capability. Could be used by an external wrapper to log or audit tool calls after the fact."
    stream-json/notification: "Emitted during goose run for MCP extension logs, subagent tool requests, and task progress. Outbound-only, no blocking. Useful for detecting subagent activity and MCP usage in a monitoring wrapper."
    stream-json/complete: "Final event in goose run stream. Includes total_tokens. Outbound-only, non-blocking. Useful for post-execution auditing."
---

# Protecting Goose Agentic CLI

Goose (by [Block](https://block.github.io/goose/)) is an open-source, extensible AI agent available as both a CLI and desktop application. As of v1.25.0 (Feb 18, 2026), Goose's protection model differs significantly from agents like Claude Code or Gemini CLI. **Goose does not have a traditional pre/post lifecycle hook system that can block or modify tool calls.** Its protections rely on permission modes, file-level restrictions, extension allowlisting, Docker containerization, and macOS sandboxing rather than programmable event hooks.

## Event Hooks

### Pre-Tool Hooks

**Goose does not support pre-tool hooks.** There is no event that fires before a planned tool call where an external listener can inspect the call and block, modify, or approve it. This is a fundamental architectural difference from Claude Code (which has `PreToolUse` with exit-code-based allow/deny) and Gemini CLI (which has `BeforeTool`).

The closest mechanism Goose provides is the **permission mode system** (`GOOSE_MODE`), which controls whether tool calls require user approval:

| Mode | Behavior |
|------|----------|
| `auto` (default) | Full file modification, extension usage, and deletion without requiring approval |
| `approve` | All write operations and extension usage require human approval via Allow/Deny buttons |
| `smart_approve` | Risk assessment auto-approves low-risk actions; flags higher-risk actions for approval |
| `chat` | Conversation only; no file modifications or extension use |

Permission modes can be set in `~/.config/goose/config.yaml`:

```yaml
GOOSE_MODE: "smart_approve"
```

Or changed mid-session via the `/mode` slash command:

```
/mode approve
```

Or set as an environment variable:

```bash
export GOOSE_MODE=smart_approve
```

**Key limitation:** Permission modes are a built-in classification system, not a programmable hook. You cannot write custom logic to evaluate tool calls against patterns. The agent's LLM provider determines whether a tool is "read" or "write" as a best-effort classification. There is no way to inject a shell script, regex matcher, or external validator into the approval pipeline.

Additionally, Goose has a `permission.yaml` file (`~/.config/goose/permission.yaml`) for tool-level permissions configured via the `goose configure` command, and a runtime `tool_permissions.json` (`~/.config/goose/permissions/tool_permissions.json`) that auto-manages permission decisions. These operate at the tool-type level and cannot match on tool arguments or content.

([Permission Modes](https://block.github.io/goose/docs/guides/goose-permissions/), [Configuration Files](https://block.github.io/goose/docs/guides/config-files/))

### GOOSE_STATUS_HOOK (The Only True Hook)

The only actual hook mechanism in Goose is `GOOSE_STATUS_HOOK`, a shell command that fires on agent state transitions between `waiting` (idle, awaiting user input) and `thinking` (processing).

```yaml
# ~/.config/goose/config.yaml
GOOSE_STATUS_HOOK: "/path/to/my-status-handler.sh"
```

**This hook is entirely fire-and-forget:**

- Exit code: **ignored**
- stdout/stderr: **suppressed** (redirected to null)
- Effect on flow: **none** -- cannot block, modify, or influence the agent loop
- Execution: spawned on a separate thread with no ordering guarantee

Example handler:

```bash
#!/bin/bash
STATUS="$1"
case "$STATUS" in
  thinking)
    echo "$(date -Iseconds) THINKING" >> /tmp/goose-status.log
    ;;
  waiting)
    echo "$(date -Iseconds) WAITING" >> /tmp/goose-status.log
    ;;
esac
```

The `GOOSE_STATUS_HOOK` is useful only for external monitoring (e.g., updating a status indicator, sending notifications). It has zero protective capability.

([Source: run_status_hook](https://github.com/block/goose/blob/main/crates/goose-cli/src/session/output.rs))

### User Prompt Event

**Goose does not provide a user prompt event.** There is no hook or event that fires when a user submits a prompt and before the agent processes it. There is no mechanism to intercept, review, modify, or block user prompts programmatically.

The `GOOSE_STATUS_HOOK` fires `thinking` after user input is accepted, but this is a fire-and-forget notification with no access to the prompt content and no ability to influence processing.

### Other Events (Stream-JSON)

While Goose lacks traditional hooks, it does provide a **streaming JSON event feed** via `goose run --output-format stream-json`. These events are **outbound-only** (observe, never control):

| Event | Description | Blocking? |
|-------|-------------|-----------|
| `message` | Agent response or tool request/response content | No |
| `notification` | MCP extension or subagent log/progress notifications | No |
| `model_change` | Active model or mode changed | No |
| `error` | Error in the agent loop | No |
| `complete` | End of `goose run` execution (always last event) | No |

**None of these events support return values or blocking.** They are strictly telemetry. To filter events, you must parse the JSON stream in a consuming script.

**Important:** Stream-JSON mode only works with `goose run` (non-interactive task execution). Interactive `goose session` does not support structured event output.

([Running Tasks](https://block.github.io/goose/docs/guides/running-tasks/), [Source: StreamEvent](https://github.com/block/goose/blob/main/crates/goose-cli/src/session/mod.rs))

### Hook Configuration Format and Scope

| Aspect | Detail |
|--------|--------|
| Format | YAML (`config.yaml`) |
| User scope | `~/.config/goose/config.yaml` (macOS/Linux), `%APPDATA%\Block\goose\config\` (Windows) |
| Project scope | **Not supported** -- there is no project-scoped hook configuration |
| Enterprise/managed scope | No native enterprise config; use `GOOSE_ALLOWLIST` for extension control |
| Inline hooks in skills/agents | **Not supported** |

**Workaround for project-scoped hooks:** Use environment variables per project (e.g., via [direnv](https://direnv.net/)) or have the `GOOSE_STATUS_HOOK` script inspect the current working directory to apply project-specific logic.

### Hooks and Subagent Flows

The `GOOSE_STATUS_HOOK` fires at the session level for the main agent only. Subagents are isolated instances with their own `Agent::new()` context. There is **no documented evidence** that `GOOSE_STATUS_HOOK` fires inside subagent instances. The stream-json `notification` events do forward subagent tool request notifications and task execution progress, but these are outbound-only and cannot block execution.

### Summary: Event Hooks for Protection

**Goose has no blocking hook mechanism.** The permission mode system (`GOOSE_MODE`) provides coarse-grained approval gates but cannot be extended with custom logic. The `GOOSE_STATUS_HOOK` is fire-and-forget. The stream-JSON events are outbound telemetry. There is no way to programmatically evaluate and block individual tool calls based on custom patterns.

## Intercepting MCP Calls

### MCP Extension Configuration

Goose calls its MCP integrations "extensions." Extensions are configured in `~/.config/goose/config.yaml` under the `extensions` key:

```yaml
extensions:
  github:
    name: GitHub
    cmd: npx
    args: [-y, "@modelcontextprotocol/server-github"]
    enabled: true
    envs:
      GITHUB_PERSONAL_ACCESS_TOKEN: "<YOUR_TOKEN>"
    type: stdio
    timeout: 300

  my-remote:
    name: My Remote Extension
    type: sse
    uri: "https://example.com/mcp-endpoint"
    enabled: true
```

**Scopes:**

| Scope | Location | Supported? |
|-------|----------|------------|
| User | `~/.config/goose/config.yaml` | Yes |
| Project/repo | None | **Not supported** |
| Enterprise/managed | Via `GOOSE_ALLOWLIST` env var | Partial (allowlist only) |

There is no project-scoped MCP configuration. Extensions are always configured at the user level.

([Using Extensions](https://block.github.io/goose/docs/getting-started/using-extensions/), [Configuration Files](https://block.github.io/goose/docs/guides/config-files/))

### MCP Response Interception

**Goose does not provide an event to intercept MCP responses before they are fed into the agent's context.** MCP extension responses flow directly into the agent's processing pipeline. There is no hook to:

- Inspect MCP responses for secrets or prompt injections
- Modify MCP responses before the agent uses them
- Block or halt execution based on MCP response content

The stream-JSON `notification` event forwards MCP extension log and progress notifications, but these are **outbound-only** and provide reformatted log strings rather than raw structured tool responses.

### Environment Variables

Environment variables are passed to MCP extensions via the `envs` section in config.yaml:

```yaml
extensions:
  my-extension:
    envs:
      API_KEY: "my-secret-key"
      BASE_URL: "https://api.example.com"
```

They can also be set via the CLI pattern: `"VAR=value command arg1 arg2"`.

**Security note:** Goose's official documentation warns against storing sensitive information in plain-text config files. The system keyring is preferred; `secrets.yaml` is a fallback when keyring is unavailable.

### Local vs. Remote Extensions

| Type | Supported? | Configuration |
|------|------------|---------------|
| Local (stdio) | Yes | `type: stdio` with `cmd` and `args` |
| Remote (HTTP/SSE) | Yes | `type: sse` with `uri` |
| Streamable HTTP | Yes | Supported since v1.24.0+ |

Local MCP binaries do **not** require fully qualified paths if they are on the system `PATH`. The `GOOSE_SEARCH_PATHS` config can supplement the system PATH with additional directories.

### Authentication for MCP

Goose passes credentials to MCP servers via environment variables in the `envs` config section. There is no built-in OAuth flow, bearer token management, or API key rotation mechanism for MCP connections. Authentication is entirely the responsibility of the MCP server implementation and the environment variables passed to it.

### Extension Allowlisting (Enterprise Control)

Goose provides an **extension allowlist** for enterprise environments via the `GOOSE_ALLOWLIST` environment variable:

```bash
export GOOSE_ALLOWLIST=https://internal.example.com/goose-allowlist.yaml
```

The allowlist YAML file format:

```yaml
extensions:
  - id: github
    command: npx -y @modelcontextprotocol/server-github
  - id: fetch
    command: uvx mcp-server-fetch
```

**Behavior when enabled:**

1. Goose fetches the allowlist from the specified URL on startup (cached, re-fetched on restart)
2. During extension installation, Goose checks the MCP server's installation command against the allowlist
3. Extensions not matching the allowlist are **blocked** from installation
4. If `GOOSE_ALLOWLIST` is not set, no restrictions are applied

**Limitations:** The allowlist only controls _installation_ of new extensions. It does not restrict already-configured extensions in `config.yaml`, and it does not provide a deny-list mechanism. For strict control, administrators must both set the allowlist and pre-configure the `config.yaml` with approved extensions.

Additionally, Goose automatically scans extensions for malware before activation, blocking known malicious packages.

([Extension Allowlist](https://block.github.io/goose/docs/guides/allowlist/))

## Completion Gates

### Completion Events

Goose emits a `complete` event in stream-JSON mode when a `goose run` task finishes:

```json
{
  "type": "complete",
  "total_tokens": 1250
}
```

This is always the final event in the stream. However, this event is **outbound-only** and **non-blocking**. There is no mechanism to prevent the agent from stopping, inject feedback, or force the agent to continue working based on this event.

**There is no `turn_complete`-style hook** that fires when the agent considers its work done in interactive mode. The `GOOSE_STATUS_HOOK` fires `waiting` when the agent returns to idle, but this is fire-and-forget with no ability to block completion or inject instructions.

### Recipe Retry as a Completion Gate Substitute

While Goose lacks hook-based completion gates, it provides a **recipe retry mechanism** that serves a similar purpose for automated workflows:

```yaml
# recipe.yaml
title: "Build and test"
instructions: "Build the project and run all tests"
retry:
  max_retries: 3
  timeout_seconds: 300
  on_failure_timeout_seconds: 600
  on_failure: "make clean"
  checks:
    - type: shell
      command: "test $(cat test-results.txt | grep -c PASS) -gt 0"
```

**How it works:**

1. Recipe executes with provided instructions
2. All success checks (shell commands) run sequentially after completion
3. If any check fails and retries remain:
   - Execute the `on_failure` command (optional cleanup)
   - Reset agent message history to initial state
   - Increment retry counter and restart
4. Process terminates when all checks pass or max attempts reached

**Key properties:**

- Checks use shell commands that must exit with code 0 to succeed
- Only `type: shell` checks are currently supported
- `on_failure` runs a cleanup command before retry
- `max_retries` prevents infinite loops
- Environment variables `GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS` and `GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS` provide global overrides

This is the closest Goose gets to a completion gate: you can validate work output via shell commands and force the agent to retry if validation fails. However, this only works with `goose run --recipe`, not in interactive sessions, and the feedback mechanism is coarse (full restart vs. targeted continuation).

([Recipe Reference](https://block.github.io/goose/docs/guides/recipes/recipe-reference))

### Main Agent vs. Subagent Completion

The stream-JSON `complete` event fires for the main `goose run` task only. Subagent completion is signaled via `notification` events with `tasks_complete` subtype, but these are flattened into formatted log strings in stream-JSON mode rather than providing structured data. Neither event supports blocking.

### Completion Loop Protection

Since there is no blocking completion event, infinite loops from blocking completion are not a concern. The recipe retry mechanism has explicit `max_retries` protection.

## Subagents as Security Event?

### Subagent Architecture

Goose subagents are temporary, isolated instances created via `Agent::new()` with their own `ExtensionManager`, `ToolMonitor`, communication channels, and context. They are fully isolated from the parent session -- no shared conversation history, memory, or state.

Key constraints:

- Max 10 concurrent subagents (hard-coded)
- Default 5-minute timeout (configurable via natural language)
- Default 25 max turns (configurable via `GOOSE_SUBAGENT_MAX_TURNS`)
- Cannot spawn their own subagents (prevents recursion; enforced since v1.14.0)

([Subagents Guide](https://block.github.io/goose/docs/guides/subagents/))

### Detecting Subagent Creation via Events

Subagent creation **can** be detected via the stream-JSON event feed. When a subagent issues a tool call, a `notification` event is emitted with `subagent_tool_request` data. When subagent work completes, `tasks_complete` notifications appear. However, these are **outbound-only** observations -- there is no way to block subagent creation.

In Claudine's event model, Goose maps to `subagent_start` (via `subagent_tool_request` notifications) and `subagent_stop` (via `tasks_complete` notifications), both at `NonHook` support level -- meaning they require stream parsing or wrapper scripts to capture, and cannot be registered as config-file hooks.

### Hooks Inside Subagents

**Goose has no pre-tool or post-tool hooks at all**, so the question of whether hooks fire inside subagents is moot. The `GOOSE_STATUS_HOOK` fires at the main session level; there is no documented evidence it fires inside subagent instances.

The permission mode (`GOOSE_MODE`) applies at the session level. Subagents are **disabled** in `approve`, `smart_approve`, and `chat` modes -- they only operate in `auto` mode. This means that when subagents are active, they run with the least restrictive permission profile.

### Restricting Subagent Permissions

Subagent permissions can be partially controlled:

- **Extensions:** Subagents inherit extensions from the parent session by default. You can restrict extensions via natural language ("with only the developer extension") or via recipe `sub_recipes` YAML definitions.
- **Cannot enable new extensions:** Subagents can browse extensions for suggestions but cannot enable them, preventing modification of the parent session.
- **Cannot spawn nested subagents:** Enforced since v1.14.0.
- **Cannot manage scheduled tasks.**

However, there is **no way to force subagents into a stricter permission mode** (e.g., `approve` mode) independent of the parent session. If the parent is in `auto` mode (the only mode where subagents work), subagents also run in `auto` mode with full tool access within their inherited extensions.

### Limiting MCP in Subagents

There is no built-in mechanism to limit MCP servers to "read-only" variants within subagents. Extension restriction is the only lever: you can restrict _which_ extensions a subagent has access to, but you cannot modify the tools those extensions expose.

### Injecting Context into Subagents

Context can be injected into subagents at creation time via:

1. **Natural language instructions** in the creation prompt
2. **Recipe sub_recipes** with `values` for parameterized instructions
3. **Goosehints** (`.goosehints` files) which are loaded at the session level, though subagents as separate instances may or may not inherit these

## Escalated Privileges

### Root/Elevated Privilege Detection

**Goose does not automatically detect or warn about running as root or with elevated privileges.** There is no built-in check for `uid == 0` or equivalent. The official security documentation ([SECURITY.md](https://github.com/block/goose/blob/main/SECURITY.md)) recommends using a dedicated virtual machine or container with limited capabilities but does not enforce this at the application level.

### Detecting Elevated Privileges via Hooks

Since Goose has no blocking hook system, there is no way to programmatically detect and respond to elevated privileges via hooks. A workaround would be to check `$(id -u)` in a startup script before launching Goose, but this is entirely external to the agent.

### Sandboxing and Container Isolation

Goose provides **three levels of isolation**:

**1. macOS Sandbox (Desktop only, v1.25.0+):**

The Goose Desktop application now runs in a macOS sandbox, providing OS-level isolation. This is **not available for the CLI**.

**2. Docker Containerization:**

Goose supports running inside Docker containers via two mechanisms:

- **Full containerization:** Run the entire `goose` process inside a Docker container. This provides filesystem and network isolation at the container level.
- **Extension containerization:** The `--container` flag runs extensions inside a specified Docker container while Goose runs on the host:

```bash
goose session --container my-dev-container
goose run --container 4c76a1beed85 --text "run tests"
```

Extensions from `config.yaml` automatically run inside the specified container. Built-in extensions require Goose CLI installation inside the container.

**3. Docker Sandbox (via Docker Desktop):**

Docker Sandboxes run agents in microVMs with dedicated Docker daemons, providing hard security boundaries beyond standard containers. This is a Docker product feature, not a Goose-specific feature.

([Goose in Docker](https://block.github.io/goose/docs/tutorials/goose-in-docker/), [Docker Blog: Goose with Docker](https://www.docker.com/blog/building-ai-agents-with-goose-and-docker/))

### Filesystem Write Path Restrictions

Goose provides `.gooseignore` files to prevent access to specific files and directories:

| Location | Scope |
|----------|-------|
| `~/.config/goose/.gooseignore` | Global (all sessions) |
| `.gooseignore` (project root) | Project-specific |

Supported patterns: basic filenames, wildcards (`*.pdf`, `**/credentials.json`), directories (`backup/`), and negation (`!.env.example`).

**Default protections** (when no `.gooseignore` exists): `**/.env`, `/.env.*`, `/secrets.*`.

**Critical limitation:** `.gooseignore` only affects the **Developer extension's tools**. Other extensions are **not restricted** by these rules. Additionally, creating any `.gooseignore` file **disables the defaults** -- you must manually re-add sensitive patterns.

([Using .gooseignore](https://block.github.io/goose/docs/guides/using-gooseignore/))

### Network Access Restriction

Goose does not provide built-in network access restriction. When running in Docker, network isolation is controlled by Docker networking configuration (e.g., `--network none` for no network access). There is no Goose-native mechanism.

### Bypass Mode

**`GOOSE_MODE=auto` effectively bypasses all permission checks.** This is the default mode and the only mode in which subagents operate. While it is not labeled as a "bypass" or "dangerous" mode, it provides the agent with unrestricted file modification, deletion, extension usage, and shell command execution without human approval.

There are no safeguards specifically around `auto` mode beyond:

- `.gooseignore` file restrictions (Developer extension only)
- Extension allowlist (`GOOSE_ALLOWLIST`)
- Prompt injection detection (added in [PR #4237](https://github.com/block/goose/pull/4237), using ML-based detection)
- Zero-width character stripping (mitigates invisible prompt injection)

### Prompt Injection Detection

Block's internal red team [successfully compromised a developer's laptop](https://engineering.block.xyz/blog/how-we-red-teamed-our-own-ai-agent-) using prompt injection hidden in zero-width Unicode characters. Following this exercise, Goose implemented:

1. **Zero-width character stripping** when content is loaded
2. **Prompt injection detection** via ML-based analysis ([PR #4237](https://github.com/block/goose/pull/4237))
3. **BERT-based command injection detection** for tool calls
4. **Recipe transparency** -- recipes now visualize loaded instructions before execution

These protections are always-on and not configurable via hooks, but they add a layer of defense that other agents (including Claude Code) do not have at the application level.

## Sources

### Official Documentation

- [Goose Homepage](https://block.github.io/goose/)
- [Permission Modes](https://block.github.io/goose/docs/guides/goose-permissions/)
- [Configuration Files](https://block.github.io/goose/docs/guides/config-files/)
- [Using Extensions](https://block.github.io/goose/docs/getting-started/using-extensions/)
- [Extension Allowlist](https://block.github.io/goose/docs/guides/allowlist/)
- [Subagents Guide](https://block.github.io/goose/docs/guides/subagents/)
- [Running Tasks](https://block.github.io/goose/docs/guides/running-tasks/)
- [Using .gooseignore](https://block.github.io/goose/docs/guides/using-gooseignore/)
- [Recipe Reference](https://block.github.io/goose/docs/guides/recipes/recipe-reference)
- [Goose in Docker](https://block.github.io/goose/docs/tutorials/goose-in-docker/)
- [CLI Commands Reference](https://block.github.io/goose/docs/guides/goose-cli-commands/)

### GitHub Sources

- [Goose Repository](https://github.com/block/goose)
- [SECURITY.md](https://github.com/block/goose/blob/main/SECURITY.md)
- [run_status_hook implementation](https://github.com/block/goose/blob/main/crates/goose-cli/src/session/output.rs)
- [StreamEvent enum](https://github.com/block/goose/blob/main/crates/goose-cli/src/session/mod.rs)
- [Subagent handler](https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_handler.rs)
- [Task execution notifications](https://github.com/block/goose/blob/main/crates/goose/src/agents/subagent_execution_tool/notification_events.rs)
- [v1.25.0 Release Notes](https://github.com/block/goose/releases/tag/v1.25.0)

### Blog Posts

- [How We Red-Teamed Our Own AI Agent (Block Engineering Blog)](https://engineering.block.xyz/blog/how-we-red-teamed-our-own-ai-agent-)
- [Building AI agents with Goose and Docker](https://www.docker.com/blog/building-ai-agents-with-goose-and-docker/)
