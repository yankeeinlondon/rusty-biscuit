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

agent_version: "0.104.0"

has_blocking_pre_tool_event: false
pre_tool_influence: "n/a"
pre_tool_actions: []
pre_tool_subagent: false

user_prompt_event: false

other_events:
    notify: "Fires a fire-and-forget external command after each completed agent turn (agent-turn-complete). Non-blocking: stdout/stderr/stdin connected to /dev/null, exit code ignored. Cannot stop, modify, or steer the agent. Useful for logging, external audits, and notifications."
    jsonl_event_stream: "Available via `codex exec --json`. Streams structured JSONL events (thread.started, turn.started, turn.completed, turn.failed, item.*) to stdout. Read-only -- no way to send control signals back. An external orchestrator could parse and kill the process if dangerous patterns detected."
    otel_telemetry: "OpenTelemetry export (disabled by default). Events include codex.conversation_starts, codex.api_request, codex.user_prompt, codex.tool_decision, codex.tool_result. Observational only -- cannot block or modify execution. Useful for compliance logging and post-hoc auditing."

mcp_supported: true
mcp_docs: "https://developers.openai.com/codex/mcp/"
mcp_config_user: "~/.codex/config.toml"
mcp_config_repo: ".codex/config.toml"
mcp_event: false
mcp_event_name: "n/a"
mcp_event_modifiable: false
mcp_event_stop: false

has_completion_event: true
completion_event_blocking: false
completion_event_names:
    - "agent-turn-complete"
    - "turn.completed"
    - "turn.failed"
completion_loop_protection: false

has_subagent_events: false
hooks_fire_in_subagents: null
subagent_permissions_configurable: true

has_sandbox: true
detects_elevated_privileges: false
has_bypass_mode: true

last_updated: "2026-02-20"
body_hash: 17986279937849218661
---

# Protecting Codex CLI

> **CLI Version at time of research:** 0.104.0 (released 2026-02-18)
> **Repository:** https://github.com/openai/codex
> **Documentation:** https://developers.openai.com/codex/cli/

Codex CLI is OpenAI's open-source, Rust-based agentic coding CLI. Its security model is built on two layers: an OS-enforced **sandbox** (controlling what Codex can technically do) and an **approval policy** (controlling when Codex must ask before acting). Codex uses TOML configuration files at multiple scopes.

---

## Event Hooks

### Overview

Codex CLI's hook system is **severely limited** compared to agents like Claude Code. As of v0.104.0 (February 2026), the only user-facing hook is a **fire-and-forget `notify` command** that fires after agent turns complete. There is **no blocking pre-tool hook**, **no user-prompt hook**, and **no mechanism for external processes to approve, deny, or modify tool calls** through the hook system.

Multiple community PRs proposing comprehensive lifecycle hooks ([PR #11067](https://github.com/openai/codex/pull/11067), [PR #9796](https://github.com/openai/codex/pull/9796), [PR #2904](https://github.com/openai/codex/issues/2109)) have been **declined** by OpenAI maintainers, who have stated they are "actively working on designing a hooks system" internally. The feature request ([Issue #2109](https://github.com/openai/codex/issues/2109)) has 417 thumbs-up reactions and remains open.

Sources: [Codex config advanced](https://developers.openai.com/codex/config-advanced/), [Issue #2109](https://github.com/openai/codex/issues/2109), [PR #11067](https://github.com/openai/codex/pull/11067), [Discussion #2150](https://github.com/openai/codex/discussions/2150)

### Pre-Tool Hooks

**Not supported.** Codex CLI does not expose a pre-tool hook to user configuration. There is no way for an external process to intercept, approve, deny, or modify a tool call before it executes.

The internal `AfterToolUse` hook ([PR #11335](https://github.com/openai/codex/pull/11335), merged) exists in the Codex Rust runtime but is **not wired to user configuration**. The `HooksConfig` struct only accepts `legacy_notify_argv`; the `after_tool_use` hook vector is initialized empty. This internal hook fires **after** tool execution, not before, so it cannot prevent tool calls.

**Workaround -- Prefix Rules (Partial Mitigation):** While not a hook in the event-based sense, Codex's [rules system](https://developers.openai.com/codex/rules) provides a **static command-filtering layer** that can block or require approval for shell commands matching specific patterns. This is the closest Codex comes to pre-execution control:

```starlark
# ~/.codex/rules/default.rules
# Block all rm -rf commands
prefix_rule(
    pattern = ["rm", ["-rf", "-fr", "-r"]],
    decision = "forbidden",
    justification = "Recursive deletion is too dangerous",
)

# Require approval for git push --force
prefix_rule(
    pattern = ["git", "push", "--force"],
    decision = "prompt",
    justification = "Force pushing can destroy remote history",
)
```

Rules use [Starlark](https://github.com/bazelbuild/starlark) (a Python-like language) and are stored in `~/.codex/rules/` or enforced by administrators via `/etc/codex/requirements.toml`. When multiple rules match, the **most restrictive decision wins** (`forbidden` > `prompt` > `allow`).

**Limitations of prefix rules:**
- They only apply to **shell commands** (the `local_shell` tool), not MCP tool calls, file patches, or other tool types
- They are **static pattern matches** on command prefixes, not dynamic evaluations of tool arguments
- Admin rules in `requirements.toml` can only `prompt` or `forbidden` (never `allow`)
- Rules do not fire inside subagents (subagents run with non-interactive approvals; see [Subagents](#subagents-as-security-event) section)

Sources: [Codex rules](https://developers.openai.com/codex/rules), [Config reference](https://developers.openai.com/codex/config-reference/)

### User Prompt Event

**Not supported.** Codex CLI does not expose any event that fires when a user submits a prompt. There is no mechanism to intercept, review, or modify user prompts before the agent processes them.

The internal telemetry system emits a `codex.user_prompt` OpenTelemetry event, but this is an **observational telemetry signal**, not a hookable event. It is redacted by default (`log_user_prompt = false`) and cannot block or modify execution.

Sources: [Codex config advanced](https://developers.openai.com/codex/config-advanced/)

### Other Events

#### `notify` (AfterAgent / agent-turn-complete)

The only user-facing hook. Fires a fire-and-forget external command after each completed agent turn.

**Configuration:**

```toml
# ~/.codex/config.toml
notify = ["python3", "/path/to/my-hook.py"]
```

**Payload delivery:** JSON passed as the **last CLI argument** (not stdin):

```json
{
  "type": "agent-turn-complete",
  "thread-id": "b5f6c1c2-1111-2222-3333-444455556666",
  "turn-id": "12345",
  "cwd": "/Users/example/project",
  "input-messages": ["Rename foo to bar"],
  "last-assistant-message": "Done."
}
```

**Key characteristics:**
- **Fire-and-forget:** Codex does not wait for the command to finish, does not read its output, and does not check its exit code. stdout, stderr, and stdin are all connected to `/dev/null`.
- **Non-blocking:** The hook cannot stop, modify, or steer the agent in any way.
- **Single event type:** Only fires on `agent-turn-complete`. Does not fire on tool calls, approval requests, session start/end, or errors.
- **kebab-case payload:** Uses `thread-id`, `turn-id`, `input-messages`, `last-assistant-message` (not snake_case).
- **Defensive use:** Can be used to log completed turns, trigger external audits, or send notifications -- but cannot prevent any action.

#### AfterToolUse (Internal Only)

An internal hook that fires after tool execution. Merged via [PR #11335](https://github.com/openai/codex/pull/11335) but **not exposed to user configuration** as of v0.104.0. The internal hook supports three result types: `Success`, `FailedContinue`, and `FailedAbort` (which terminates the tool call pipeline). This infrastructure suggests OpenAI intends to expose user-configurable tool hooks in the future, but no timeline has been announced.

#### JSONL Event Stream (`codex exec --json`)

Running Codex in non-interactive mode with `--json` streams structured JSONL events to stdout. While not a traditional hook system, it provides an **external orchestration surface** for CI/CD pipelines:

```bash
codex exec --json "your task prompt" | jq
```

Event types include: `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.updated`, `item.completed`, and `error`. Item types include `command_execution`, `file_change`, `mcp_tool_call`, `agent_message`, and others.

This stream is **read-only** -- there is no way to send control signals back to Codex through it. However, an external orchestrator could parse the stream, detect dangerous patterns, and kill the Codex process if needed (a blunt but effective approach).

Sources: [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive/), [CLI reference](https://developers.openai.com/codex/cli/reference/)

#### OpenTelemetry Telemetry Export

Codex can export structured telemetry events via OpenTelemetry (disabled by default):

```toml
[otel]
exporter = "otlp-http"
environment = "prod"
log_user_prompt = false
```

Events include: `codex.conversation_starts`, `codex.api_request`, `codex.user_prompt`, `codex.tool_decision`, `codex.tool_result`. These are **observational only** and cannot block or modify execution. Useful for compliance logging and post-hoc auditing.

Sources: [Codex config advanced](https://developers.openai.com/codex/config-advanced/)

### Hook Configuration Summary

| Property | Value |
|----------|-------|
| Format | TOML (`config.toml`) |
| User scope | `~/.codex/config.toml` |
| Project scope | `.codex/config.toml` (trusted projects only) |
| Enterprise scope | `/etc/codex/requirements.toml` (admin-enforced) |
| Inline in skills | Not supported |
| Blocking hooks | Not supported |

### Subagent Coverage

The `notify` hook fires only for the **main agent's turn completions**. There is no documentation confirming that `notify` fires inside subagent threads. Subagents run with non-interactive approvals, meaning actions that would require approval simply fail rather than triggering a hook or prompt.

---

## Intercepting MCP Calls

### MCP Configuration

Codex CLI supports MCP servers and stores configuration in `config.toml`:

| Scope | Path | Notes |
|-------|------|-------|
| User | `~/.codex/config.toml` | Global for all projects |
| Project | `.codex/config.toml` | Trusted projects only |
| Enterprise | `/etc/codex/requirements.toml` | Admin-enforced allowlists |

**Configuration format:**

```toml
# STDIO server (local process)
[mcp_servers.my-local-server]
command = "npx"
args = ["-y", "@my/mcp-server"]
env = { API_KEY = "xxx" }
env_vars = ["HOME", "PATH"]
cwd = "/path/to/working/dir"
startup_timeout_sec = 10
tool_timeout_sec = 60
enabled = true
required = false
enabled_tools = ["read_file", "search"]
disabled_tools = ["delete_file"]

# Streamable HTTP server (remote)
[mcp_servers.my-remote-server]
url = "https://mcp.example.com/sse"
bearer_token_env_var = "MCP_TOKEN"
http_headers = { "X-Custom" = "value" }
env_http_headers = { "Authorization" = "MCP_AUTH_HEADER" }
```

Sources: [Codex MCP docs](https://developers.openai.com/codex/mcp/), [Config reference](https://developers.openai.com/codex/config-reference/)

### Transport Types

- **STDIO (local process):** Supported. Uses `command` + optional `args`. Does not require fully qualified paths (shell PATH resolution applies).
- **Streamable HTTP (remote):** Supported. Uses `url` field. Supports bearer tokens, static headers, and environment-sourced headers.

### Environment Variables

Environment variables are passed to STDIO MCP servers via two mechanisms:
- `env`: Explicit key-value pairs set directly in config
- `env_vars`: Whitelist of existing environment variables to forward from the Codex process

### Authentication

- **Bearer tokens:** Via `bearer_token_env_var` (reads token from an environment variable)
- **OAuth:** Via `codex mcp login <server-name>` for OAuth-supporting servers. Callback port configurable via `mcp_oauth_callback_port`.
- **Static headers:** Via `http_headers` (literal values) or `env_http_headers` (values from environment variables)
- **Credential storage:** Configurable via `mcp_oauth_credentials_store` (`auto`, `file`, `keyring`)

### Tool Allow/Deny Listing

Per-server tool filtering is supported:
- `enabled_tools`: Allow list (only these tools are exposed)
- `disabled_tools`: Deny list (applied after allow list)

### Enterprise MCP Allowlisting

Administrators can enforce MCP server identity matching via `/etc/codex/requirements.toml`:

```toml
[mcp_servers.approved-server]
identity.command = "npx -y @approved/mcp-server"  # STDIO allowlist
identity.url = "https://approved.example.com/mcp"  # HTTP allowlist
```

Only servers matching the identity are allowed. This provides enterprise-level MCP server allowlisting.

### Intercepting MCP Responses

**Not supported.** Codex CLI does not provide any hook or event that fires when an MCP server returns a response. There is no mechanism to inspect, modify, or block MCP responses before they are fed back into the agent's processing flow.

The internal `AfterToolUse` hook payload does include MCP tool calls (with `tool_kind: "mcp"`, `server`, `tool`, and `arguments` fields), but this hook fires **after** tool execution and is **not exposed to user configuration**.

The JSONL event stream (`codex exec --json`) does emit `mcp_tool_call` item events, but these are read-only and cannot be used to intercept or modify responses.

**Mitigation strategies:**
1. Use `enabled_tools` / `disabled_tools` per server to limit which MCP tools the agent can call
2. Use enterprise `requirements.toml` to restrict which MCP servers can be used
3. In `codex exec --json` mode, parse the JSONL stream externally and kill the process if dangerous MCP output is detected

Sources: [Codex MCP docs](https://developers.openai.com/codex/mcp/), [Config reference](https://developers.openai.com/codex/config-reference/)

---

## Completion Gates

### Available Completion Events

Codex provides one user-facing completion signal: the `notify` hook firing `agent-turn-complete` after each agent turn. In `codex exec --json` mode, `turn.completed` events are emitted in the JSONL stream.

| Event | Surface | Blocking | User-configurable |
|-------|---------|----------|-------------------|
| `agent-turn-complete` (notify) | External command | No | Yes |
| `turn.completed` (JSONL) | stdout stream | No | Only via `--json` flag |
| `turn.failed` (JSONL) | stdout stream | No | Only via `--json` flag |

### Can Completion Events Block?

**No.** The `notify` hook is fire-and-forget. Its exit code and output are ignored. There is no mechanism to force the agent to continue working, inject feedback, or prevent the agent from stopping.

### Infinite Loop Protection

Not applicable, since completion events cannot block.

### Running External Commands on Completion

The `notify` hook can run arbitrary external commands when a turn completes:

```toml
# Run tests after every agent turn
notify = ["bash", "-lc", "/path/to/post-turn-check.sh"]
```

However, the script cannot feed results back into the agent. It can only perform side effects (logging, notifications, external validation) without influencing the agent's behavior.

### Structured Output Validation

For non-interactive (`codex exec`) runs, the `--output-schema` flag provides a form of completion gate:

```bash
codex exec --json --output-schema schema.json "Analyze this repo"
```

Codex validates the final `agent_message` against the provided JSON Schema. If validation fails, Codex reports the failure. This ensures the **shape** of the final output is correct, though it cannot enforce semantic correctness.

### Subagent Completion

There is no documented distinction between main agent and subagent completion events. The `notify` hook documentation only mentions `agent-turn-complete` for the main agent. Subagent threads do not appear to trigger separate user-facing completion hooks.

### Feedback Injection

**Not supported.** There is no mechanism to inject feedback or instructions back into the agent after a completion event fires. The `notify` hook cannot return data to the agent.

**Workaround for CI/CD:** In `codex exec --json` mode, an external orchestrator can:
1. Parse the JSONL stream for `turn.completed` events
2. Evaluate the output
3. If unsatisfactory, start a **new** Codex invocation with additional context

This is a coarse-grained approach and does not constitute a true completion gate.

Sources: [Codex config advanced](https://developers.openai.com/codex/config-advanced/), [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive/)

---

## Subagents as Security Event?

### Multi-Agent Overview

Codex CLI supports multi-agent workflows as an **experimental** feature, requiring explicit opt-in:

```toml
# ~/.codex/config.toml
[features]
multi_agent = true
```

Or toggle at runtime via `/experimental` in the TUI. When enabled, Codex can spawn specialized sub-agents in parallel and collect their results.

Sources: [Multi-agents docs](https://developers.openai.com/codex/multi-agent)

### Can We Detect Subagent Creation via Events?

**No.** There is no hook or event that fires when a subagent is spawned. The `notify` hook only fires on `agent-turn-complete` and there is no `agent-spawned` or `subagent-created` event type. In `codex exec --json` mode, there is no documented JSONL event for subagent creation either.

### Do Hooks Fire Inside Subagents?

**Unknown / likely not.** The `notify` hook documentation only describes firing on the main agent's turn completion. There is no documentation confirming that `notify` fires inside subagent threads. Given that subagents run with non-interactive approvals (actions that would require approval simply fail), it is likely that the limited hook infrastructure does not extend to subagent threads.

### Can We Force Stricter Permissions on Subagents?

**Yes, partially.** Individual agent roles can override sandbox and model settings:

```toml
[agents.explorer]
description = "Read-only codebase exploration agent"
sandbox_mode = "read-only"
model = "gpt-4.1-mini"

[agents.worker]
description = "Implementation agent"
config_file = "worker-config.toml"
```

The `sandbox_mode` field on agent roles allows restricting subagents to `read-only` or other modes. However, subagents **inherit** the parent session's sandbox policy by default, and the override must be configured in advance -- it cannot be applied dynamically at spawn time.

### Can We Limit MCP Servers in Subagents?

**Only via role-level config overrides.** If an agent role specifies a `config_file`, that config file can define different `mcp_servers` settings. However, there is no per-subagent MCP restriction mechanism beyond this. There is no way to dynamically limit subagents to "read-only" MCP variants.

### Can We Reduce Tool Access for Subagents?

**Partially.** The `sandbox_mode` on agent roles can restrict filesystem and network access. Prefix rules apply at the session level but subagents run with non-interactive approvals, meaning any action that would trigger a `prompt` rule simply **fails** rather than prompting the user. This effectively makes `prompt` rules act as `forbidden` rules inside subagents -- which is a useful defensive property, but it is an implicit behavior rather than an explicit configuration.

### Can Context or Instructions Be Injected Into Subagents?

**Yes.** Agent roles support a `developer_instructions` field and a `description` field that provide guidance text. Additionally, a `config_file` can point to a full TOML config layer with custom instructions:

```toml
[agents.careful-worker]
description = "Worker that must not delete files"
developer_instructions = "Never use rm, never delete files. Always create backups before modifying."
sandbox_mode = "workspace-write"
```

Sources: [Multi-agents docs](https://developers.openai.com/codex/multi-agent), [Config reference](https://developers.openai.com/codex/config-reference/)

---

## Escalated Privileges

### Root / Elevated Privilege Detection

**Codex does not automatically detect or warn about running as root or with elevated privileges.** There is no startup check, warning banner, or configuration option that flags elevated execution.

When Codex runs under `sudo`, the `HOME` directory changes to `/root`, which can cause configuration loading failures since `~/.codex/config.toml` resolves to `/root/.codex/config.toml` instead of the original user's config. Several GitHub issues document problems with `sudo` execution ([Issue #6108](https://github.com/openai/codex/issues/6108), [Issue #7577](https://github.com/openai/codex/issues/7577)), but these are treated as bugs rather than deliberate security warnings.

### Sandbox and Isolation

Codex provides robust OS-enforced sandboxing:

| Platform | Technology | Description |
|----------|-----------|-------------|
| macOS | Seatbelt (`sandbox-exec`) | Profile-based process sandboxing |
| Linux | Landlock + seccomp | Filesystem restrictions + syscall filtering |
| Linux (alt) | Bubblewrap | Optional via `features.use_linux_sandbox_bwrap` |
| Windows | WSL / experimental native | Experimental support |
| Cloud | OpenAI containers | Isolated execution environments |

**Sandbox modes:**

| Mode | Filesystem | Network | Use case |
|------|-----------|---------|----------|
| `read-only` | Read only | Disabled | Safe exploration |
| `workspace-write` | Read/write in workspace | Disabled by default | Normal development |
| `danger-full-access` | Unrestricted | Unrestricted | Trusted environments only |

**Protected paths** (read-only regardless of sandbox mode): `.git`, `.agents/`, `.codex/`

Sources: [Codex security](https://developers.openai.com/codex/security)

### Filesystem Write Path Restrictions

**Yes.** The `workspace-write` sandbox restricts writes to the active workspace directory. Additional writable paths can be granted via:

- `--add-dir <path>` CLI flag (repeatable)
- `sandbox_workspace_write.writable_roots` in `config.toml`

Temporary directory access can be controlled via:
- `sandbox_workspace_write.exclude_slash_tmp` (removes `/tmp`)
- `sandbox_workspace_write.exclude_tmpdir_env_var` (removes `$TMPDIR`)

### Network Access Restrictions

**Yes.** Network access is **disabled by default** in `workspace-write` mode. Enable via:

```toml
[sandbox_workspace_write]
network_access = true
```

Web search has three modes: `disabled`, `cached` (pre-indexed results, reduces prompt injection risk), and `live`.

### Bypass Mode

**Yes.** Codex has two bypass mechanisms:

1. **`--dangerously-bypass-approvals-and-sandbox` (alias: `--yolo`):** Disables all sandboxing and approval checks. OpenAI's documentation warns to "only use inside an externally hardened environment."

2. **`--sandbox danger-full-access`:** Removes all technical restrictions while potentially keeping approval policy intact (though combining with `--ask-for-approval never` effectively disables everything).

3. **`--full-auto`:** A convenience alias for `--sandbox workspace-write --ask-for-approval on-request`. This is **not** a full bypass -- it still enforces the workspace-write sandbox and prompts for actions outside the workspace or requiring network access.

**Enterprise safeguards:** Administrators can restrict available sandbox modes and approval policies via `/etc/codex/requirements.toml`:

```toml
# /etc/codex/requirements.toml
allowed_sandbox_modes = ["read-only", "workspace-write"]
allowed_approval_policies = ["untrusted", "on-request"]
```

This prevents users from selecting `danger-full-access` or `never` approval policy on managed machines. macOS MDM support is available via the `com.openai.codex` preference domain.

### Security Advisory

A sandbox bypass vulnerability ([GHSA-w5fx-fh39-j5rw](https://github.com/openai/codex/security/advisories/GHSA-w5fx-fh39-j5rw)) was discovered where a bug in the path configuration logic allowed the model-generated `cwd` to be treated as the sandbox's writable root, enabling arbitrary file writes outside the intended workspace boundary. This has been patched, but it illustrates the importance of keeping Codex updated.

### Sandbox Testing

Codex provides sandbox testing commands:

```bash
codex sandbox macos [--full-auto] [COMMAND]
codex sandbox linux [COMMAND]
```

These allow verifying sandbox behavior without running full agent sessions.

Sources: [Codex security](https://developers.openai.com/codex/security), [CLI reference](https://developers.openai.com/codex/cli/reference/), [Config reference](https://developers.openai.com/codex/config-reference/)

---

## Summary Assessment

| Capability | Status | Notes |
|-----------|--------|-------|
| Blocking pre-tool hook | Not supported | Prefix rules provide static command filtering only |
| User prompt event | Not supported | OTel telemetry is observational only |
| Post-tool hook | Internal only | `AfterToolUse` exists but not user-configurable |
| Completion gate | Not supported | `notify` is fire-and-forget |
| MCP response interception | Not supported | No hook for MCP responses |
| Subagent event detection | Not supported | No spawn/creation event |
| Subagent permission control | Partial | Via agent role `sandbox_mode` and `config_file` |
| OS-enforced sandbox | Supported | Seatbelt (macOS), Landlock+seccomp (Linux) |
| Enterprise enforcement | Supported | `requirements.toml` restricts sandbox/approval/MCP |
| Bypass mode | `--yolo` / `danger-full-access` | Can be blocked by admin requirements |
| Elevated privilege detection | Not supported | No root/sudo warning |

Codex CLI compensates for its lack of hook-based protection through strong **OS-level sandboxing**, a **rules-based command filtering system**, and **enterprise enforcement** via `requirements.toml`. However, the absence of blocking pre-tool hooks means there is no way for external processes to dynamically evaluate and approve/deny tool calls at runtime. OpenAI has acknowledged the demand for a comprehensive hook system ([Issue #2109](https://github.com/openai/codex/issues/2109)) and stated they are "actively working on designing a hooks system," but no timeline or design details have been published.

---

## Sources

- [Codex CLI homepage](https://developers.openai.com/codex/cli/)
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference/)
- [Codex security](https://developers.openai.com/codex/security)
- [Codex config basics](https://developers.openai.com/codex/config-basic/)
- [Codex config advanced](https://developers.openai.com/codex/config-advanced/)
- [Codex config reference](https://developers.openai.com/codex/config-reference/)
- [Codex config sample](https://developers.openai.com/codex/config-sample/)
- [Codex rules](https://developers.openai.com/codex/rules)
- [Codex MCP docs](https://developers.openai.com/codex/mcp/)
- [Codex multi-agent docs](https://developers.openai.com/codex/multi-agent)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive/)
- [Codex changelog](https://developers.openai.com/codex/changelog/)
- [Codex GitHub repository](https://github.com/openai/codex)
- [Event hooks feature request (Issue #2109)](https://github.com/openai/codex/issues/2109)
- [Hooks discussion (#2150)](https://github.com/openai/codex/discussions/2150)
- [Comprehensive hooks PR (closed, #11067)](https://github.com/openai/codex/pull/11067)
- [AfterToolUse hook PR (merged, #11335)](https://github.com/openai/codex/pull/11335)
- [Sandbox bypass advisory (GHSA-w5fx-fh39-j5rw)](https://github.com/openai/codex/security/advisories/GHSA-w5fx-fh39-j5rw)
