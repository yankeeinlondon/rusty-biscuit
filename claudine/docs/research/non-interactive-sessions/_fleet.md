---
sequence: "@claudine/docs/providers.yaml"
operation: "research"
file: "{{ctx.repo_root}}/claudine/docs/research/non-interactive-sessions/{{state.file}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **non-interactive sessions**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **non-interactive sessions** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Non-Interactive Sessions** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Non-Interactive Sessions** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Non-Interactive Sessions research on **{{state.name}}** failed to complete!"
    warn: "The Non-Interactive Sessions research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Non-Interactive Session Research on {{state.name}}

## Scope

Research how **{{state.desc}}** behaves when it runs as a **non-interactive agent
session**.

When an agent runs non-interactively, Claudine is not just trying to get the final answer
as text. Claudine is acting as a wrapper around an autonomous process. It needs to see
what is happening while the run is still active: which model is being used, which tools
are called, whether a file was changed, whether the agent hit a quota or auth failure,
whether a permission prompt would block automation, how many tokens were consumed, and
whether the final state was success, failure, cancellation, or something ambiguous.

Plain Markdown output is useful for a human, but it is weak input for a wrapper. The
highest-value non-interactive mode is one that emits structured events, usually JSON,
JSONL/NDJSON, SSE, or a bidirectional protocol. This structured output lets Claudine
parse live progress, classify errors, render clean terminal status, build reports, and
drive lifecycle behavior without scraping prose.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but do not treat the schema as the research instructions. The
instructions below define the facts that matter and the quality bar for this topic.

## What Non-Interactive Means

For this research, **non-interactive** means a mode intended for automation, scripting,
CI, wrapper execution, or API-style use where the provider should not require a human TTY
mid-run. Providers use different names for this mode: print mode, headless mode, exec,
run, batch, wire mode, app-server mode, or SDK mode.

Most agentic CLIs provide multiple output formats for non-interactive sessions. Your
first task is to enumerate the output formats provided by **{{state.name}}** and decide
which one Claudine should prefer.

The most valuable format is usually structured data that can be parsed while the agent is
still running. Choose the best format for Claudine and explain why it is the best choice.
Do not stop at "it is structured." Consider:

- **API style:** many providers expose both a request/reply style interaction and a
  streaming response. The streaming response is almost always more valuable because
  Claudine can show progress, classify failures, and react before the process exits.
- **Multiple streams:** some providers expose more than one useful stream, such as a
  response stream plus a log, telemetry, hook, or server-event stream. When that exists,
  explain whether Claudine should parse the response stream, the secondary stream, or
  both to provide useful situational awareness to the caller.
- **Schema:** after identifying the right structured stream, determine how formally it is
  defined. If there is an official schema, name the schema language. If there is no
  formal specification, look for a strong informal specification: source-code types, typed
  SDKs, generated clients, third-party typed libraries, reverse-engineered schemas, or
  well-maintained examples. Be creative, because good schema evidence is highly valuable
  for Claudine's parser.
- **Configuration:** identify the CLI switch or switches used to select the output
  format. Then look for other surfaces that influence output: environment variables,
  user config files, repo config files, managed/system config, command aliases, profiles,
  debug/verbose flags, or logging settings.

When researching configuration, always consider the common **user** and **repo** scopes
that often combine to create the effective configuration. Be clear about which scope
wins, whether scopes merge or replace each other, and how that affects the output Claudine
will see.

After selecting the best output format, research the metadata it exposes:

- session identity, cwd/project/worktree, provider version, model, auth source,
  permission mode, sandbox mode, MCP servers, tools, roots, and terminal status
- tool calls, tool results, command execution, file changes, plans, reasoning, subagents,
  token usage, cost, rate limits, quota caps, auth failures, and permission denials
- stdout/stderr/stdin behavior, framing, event ordering, correlation fields, terminal
  events, unknown event behavior, and failure/exit-code semantics

## Calibration: What Good Research Looks Like

Use these known patterns to calibrate the level of specificity expected. They are not
answers for **{{state.desc}}** unless you verify them for this provider.

- Claude-style print mode is not just "JSON output": it can require a print/headless flag
  plus an output-format flag, emits NDJSON on stdout, uses top-level `type`, has `system`
  and `result` subtypes, and exposes richer metadata only with extra flags such as verbose
  or hook-event inclusion.
- Codex-style exec mode may have a flattened event stream whose authoritative schema is
  in Rust source rather than public JSON Schema. A broader app-server schema is useful
  context but not necessarily the CLI stream schema.
- Gemini/Qwen-style `stream-json` can look complete while dropping internal events or
  only exposing some fields in buffered `json` mode. Research must say which fields are
  present in the streaming mode Claudine would actually parse.
- Kimi-style wire mode is a bidirectional JSON-RPC line protocol, not a simple output
  format. The client may need to answer requests, approve tools, or send cancellation
  messages; stdout is not merely "events from the agent."
- OpenCode-style JSON runs may emit tool events only after completion, omit a terminal
  `session.complete` event, or require process exit to infer completion. That changes how
  Claudine renders live progress.
- Goose-style streams can mix top-level snake_case event names with nested camelCase
  content names. These casing and nesting differences are parser requirements, not trivia.
- Roo-style streams may expose unusually direct operational signals such as plan caps,
  auth kind, permission denial, and cost. If a provider has such events, capture the exact
  payload fields instead of summarizing them.

A strong answer says things like: "tool calls are only visible after completion; join
results by `tool_id`; final usage is in `result.stats`, but model identity in `init.model`
may be an alias; stderr is diagnostics only; permission prompts fail rather than hang."

A weak answer says things like: "supports JSON output," "has tool events," "shows errors,"
or "can be used in CI" without naming commands, formats, event types, fields, and failure
behavior.

## Research Output Model

Write the research as useful prose first. The body of `{{file}}` should explain how
**{{state.name}}** works, why one output format is better for Claudine than the others,
what the important caveats are, and how the facts connect. Tables are welcome where they
help compare formats, events, fields, or commands, but do not reduce the document to a
field dump.

Then distill the key operational facts into frontmatter so Claudine can use them for
structured operations. The frontmatter should be lifted from the prose, not invented
separately. If you cannot explain a field clearly in the body, the structured value is
probably not ready yet.

The prose and frontmatter serve different use cases:

- **Body prose** is for human maintainers. It should explain tradeoffs, context,
  provider-specific behavior, edge cases, and why Claudine should choose one output
  format or parsing strategy over another.
- **Frontmatter** is for machines. It should contain concise, normalized facts that can
  feed wrappers, parser generation, provider metadata, reports, and drift checks.

This means every important frontmatter claim should have corresponding prose somewhere in
the body, with source links or observed evidence. Conversely, if the prose identifies a
wrapper/parser-significant fact, capture the distilled form in frontmatter.

## Frontmatter Distillation

After writing the body, set these frontmatter properties:

- `created` set to "{{ctx.today}}" when the file does not define this property; otherwise
  leave unchanged
- `last_updated` set to "{{ctx.today}}"
- `agent` set to "{{env.AGENT}}"
- `model` set to "{{env.MODEL || 'default'}}"
- `docs`: primary official documentation for the exact mode Claudine would use. Prefer a
  headless/print/exec/run/SDK/wire-mode page over generic CLI docs.
- `invocation`: every scriptable launch form. Include the full command shape, whether the
  prompt can come from argv or stdin, and whether the command starts a fresh session,
  resumes a session, or talks to a long-running server.
- `output_formats`: every output mode and the exact CLI value that selects it. For each
  one, say whether it is text, single JSON, JSONL/NDJSON, SSE, JSON-RPC lines, or other;
  whether it streams; what behavior changes when that format is selected; and whether
  Claudine should prefer it.
- `schema_sources`: where the stream shape is actually defined. Distinguish public docs,
  JSON Schema, OpenAPI, TypeScript SDK types, Rust Serde structs, Python/Pydantic models,
  generated SDK types, examples, and reverse-engineered community schemas.
- `cli_params`: non-interactive and parser-relevant switches: output format, input
  format, verbose/debug, thinking/reasoning, partial deltas, hook events, JSON-schema
  validation, max turns, budgets, cwd, model, permission mode, yolo/approval bypass,
  MCP, resume, files, images, and config overrides.
- `config_files`: user, repo, system, or managed config files that can influence
  non-interactive output format, logging, verbosity, color, model, tools, permissions, or
  stream shape. Include OS-specific paths, file format, scope, effect, and merge/override
  notes. One record per OS — file paths must be recorded separately for macOS, Linux,
  and Windows (never one record for all OSes; Windows paths always differ).
- `env_vars`: only variables that change non-interactive behavior, auth source,
  structured output, buffering, color, logging, tool approval, or provider config.
- `io_contract`: what Claudine can safely assume about stdout, stderr, stdin, and event
  framing. Say whether stdout is parse-only, mixed, or text; whether stderr carries
  useful lifecycle data; and whether stdin is prompt text or a bidirectional protocol.
- `stream_contract`: the parser contract: discriminator path, nested subtype paths,
  event ordering, join/correlation fields, terminal event, partial-message behavior,
  schema-version markers, and unknown-event policy.
- `session_metadata`: the exact fields that reveal session ID, cwd, model, provider,
  auth source, version, MCP servers, permission mode, sandbox, roots, or project. Say
  whether each field is always present or requires a flag.
- `stream_events`: list the concrete event names and categories. Include both top-level
  events and important subtypes; do not collapse them into "message" or "tool event" if
  the provider exposes a richer union.
- `tools`: for each built-in/provider-native tool family, say whether call start, input,
  progress, result, error, stdout/stderr, file changes, attachments, metadata, and final
  status are visible.
- `completion`: how Claudine identifies final success, final failure, final answer text,
  cancellation, interruption, usage, and cost. Say whether process exit code is reliable
  or only advisory.
- `blocking_behavior`: what happens without a TTY when the provider needs permission,
  approval, auth, MCP OAuth, elicitation, or a user answer. Use concrete behavior:
  auto-deny, auto-approve, fail, hang, empty answer, callback/prompt tool, or configurable.
- `subagents`: whether subagents can run non-interactively; whether their start/stop,
  tool calls, results, model, session IDs, and errors are visible in the parent stream;
  and whether the caller can inject non-interactive instructions into their prompts.
- `use_cases`: detection records for Claudine's normalized operational signals. Include
  event names, field paths, units, timezone/window details, near-miss distinctions, and
  hook/log parity.
- `headless_constraints`: constraints that can break automation, such as required yolo
  flags, unsupported stdin, model required in non-TTY, OAuth prompts, TUI-only pickers,
  no terminal event, noisy stdout, or undocumented stream drift.
- `quirks`: provider-specific parser footguns, surprising behavior, version drift, or
  unsafe assumptions that should be visible to future wrapper/parser work.
- `gaps`: facts that could not be verified from docs, source, local inspection, or
  captured examples. Missing data is useful; do not hide it.
- `claudine_strategy`: the recommended command and flags Claudine should use, flags it
  should avoid, parser notes, wrapper notes, and which stream or streams Claudine should
  parse.
- `data_format`: the primary structured format Claudine should use for this provider.
- `changes`: update-mode changelog entries only; first-run documents should use `[]`.
- `requires_claudine_update`: `true` only when the findings imply a Claudine code or
  generated-metadata change. Explain that in `reason`.

Use `unknown` when current evidence does not prove the answer. Do not invent a stable
schema from examples unless you clearly label it as inferred in both the body and
frontmatter.

## Quality Bar

A good non-interactive research file lets both a human maintainer and Claudine's
structured tooling understand the provider without guessing. Prefer clear explanatory
prose supported by exact commands, exact event names, exact field paths, documented
schema locations, and concrete examples.

Do:

- Distinguish text output, single-result JSON, streaming JSON/NDJSON, SSE, and
  bidirectional protocols.
- Distinguish provider documentation from source-code types, generated SDK types,
  reverse-engineered examples, and community schemas.
- Say which stream field is the discriminator, such as `type`, `event`, `method`, or a
  nested subtype.
- Explain whether events are complete snapshots or deltas.
- Explain how tool calls and tool results are correlated.
- Explain whether tool start/progress is visible before completion or only after the
  result is known.
- Explain how final success and failure are represented and whether process exit code can
  be trusted.
- Explain whether stdout is safe to parse line-by-line or contains banners, markdown,
  progress bars, ANSI escapes, or logs.
- Explain whether stderr is ignorable, useful, structured, or necessary for lifecycle
  classification.
- Explain what information only appears when additional flags are enabled.
- Explain whether enabling structured output changes provider behavior, disables color,
  hides thinking, changes permission handling, or suppresses prompts.
- Explain whether provider config can set the output format persistently, or whether the
  flag must be supplied every run.
- Explain whether the stream exposes session ID early enough for resume/recovery.
- Explain whether model identity is an alias, provider/model ID, resolved backend model,
  or unavailable.
- Explain token and cost units precisely, including whether usage is per-step, per-turn,
  or session-total.
- Explain whether timestamps are Unix seconds, Unix milliseconds, ISO-8601, local time,
  UTC, or unspecified.
- Explain whether non-interactive runs can hang on permissions, questions, auth prompts,
  MCP OAuth, or tool confirmations.
- Use prose to connect facts that are separate in frontmatter. For example, explain why
  a stream is best even if a prettier single JSON result exists, or why a useful field is
  unreliable because it only appears under a verbose flag.
- Keep prose and frontmatter in sync. If a field says `exit_code_reliable: false`, the
  body should explain what terminal event or stream record Claudine should trust instead.
- Cite official docs first, then source code, then observed local behavior, then
  community reports. Clearly label inferred facts.

Avoid:

- Writing a thin prose body that merely restates frontmatter fields.
- Saying "JSON output" without saying single object, JSONL/NDJSON, SSE, or protocol
  envelope.
- Saying "supports tools" without saying whether tool input, result, errors, progress,
  and metadata are visible in the stream.
- Treating pretty-printed human text as structured output.
- Assuming stderr is noise.
- Assuming exit code maps cleanly to agent success or failure.
- Assuming a schema is formal because documentation shows examples.
- Assuming fields are stable when docs or source say the format is internal.
- Omitting fields that are absent. Absence is useful metadata; record it in `gaps`,
  `notes`, or the appropriate frontmatter field.

## Required Body Sections

Write the body using these sections. Add subsections where useful. Each section should
explain the provider behavior in prose before, or alongside, any tables. The point is not
to mirror frontmatter; the point is to make the research understandable and reviewable.

- `## Summary`
- `## Non-Interactive Entry Points`
- `## Output Formats`
- `## Schema Sources`
- `## IO Contract`
- `## Stream Contract`
- `## Session Metadata`
- `## Event Families`
- `## Tools`
- `## Completion and Exit Status`
- `## Blocking Behavior`
- `## Subagents`
- `## Use Case Detection`
- `## Headless Constraints`
- `## Timeline`
- `## Quirks and Gaps`
- `## Claudine Integration Notes`
- `## Changelog` when updating an existing document
- `## Sources`

`## Summary` should be at the top of the body and should answer in one or two paragraphs
whether Claudine can run this provider non-interactively with structured output, which
format should be used, and what the main parser/wrapper risks are.

`## Output Formats` should compare every available format and explicitly recommend the
format Claudine should parse. Explain the recommendation in prose, including the tradeoff
between request/reply output, streaming output, and any secondary logging or event
streams.

`## Schema Sources` should explain the confidence level behind the stream shape. If the
best source is a TypeScript union, Rust enum, Pydantic model, generated SDK, or observed
example rather than an official schema, explain why it is still useful and what risk
remains.

`## Claudine Integration Notes` should be a practical maintainer-oriented synthesis:
recommended command, required flags, streams to parse, streams to ignore, parser hazards,
blocking behavior, and known gaps.

## Research Questions

### Entry Points and Invocation

- What commands or modes run the provider without opening a TUI?
- Can the prompt be passed as an argv argument, stdin, file, JSON message, SDK call, or
  protocol request?
- Can the run include attachments, images, file references, extra roots, cwd, model,
  system prompt, MCP servers, tools, or agent/subagent selection?
- Which flags are required for automation to avoid prompts or hangs?
- Which flags conflict with structured output or non-interactive mode?
- Can the output format be configured persistently, or only per invocation?
- If config files influence output format or stream shape, which files exist at user,
  repo, system, or managed scopes? How are those scopes merged, overridden, or gated by
  project trust?

### Output Formats and Schema Sources

- What output formats are documented, and what exact CLI values select them?
- Which one should Claudine prefer for live parsing?
- Does the provider expose a request/reply mode, a streaming response mode, a logging
  stream, a hook stream, a server-events stream, or a bidirectional protocol? Which stream
  or streams should Claudine parse, and why?
- Is the format single JSON, JSONL, NDJSON, SSE, JSON-RPC lines, text, or something else?
- Is each line/event independently parseable?
- Does the provider publish a formal schema? If so, where and in what language?
- If there is no formal schema, is the source code type definition authoritative enough?
- Are SDK types equivalent to CLI output, broader than CLI output, narrower than CLI
  output, or stale?
- Is there schema versioning, protocol versioning, or release-note history for stream
  changes?

### IO and Stream Contract

- Is stdout machine-readable only when structured output is enabled?
- What goes to stderr: logs, warnings, progress, JSON events, auth messages, tool output,
  or debug data?
- Does the stream use a top-level discriminator? Are there nested subtypes?
- Are assistant messages streamed as partial deltas, completed blocks, or both?
- What event marks session start? What event marks terminal success or failure?
- How are tool calls correlated with results?
- Are events ordered strongly enough for a single-pass parser?
- What should Claudine do with unknown event types?
- Are timestamps present? If so, what unit and timezone do they use?

### Session and Runtime Metadata

- Is a session ID emitted? How early? Is it stable enough for logs or resume?
- Is cwd, project root, git branch, or workspace path emitted?
- Is model identity emitted? Is it a requested alias or resolved backend model?
- Is provider identity emitted for aggregator CLIs?
- Is auth source or auth kind emitted without leaking secrets?
- Is CLI/provider version emitted?
- Are MCP servers, tools, permission mode, sandbox mode, and roots emitted?
- Which of these fields require verbose/debug flags?

### Tools, Files, and Subagents

- Which built-in tools can run in non-interactive mode?
- Are MCP tools represented differently from native tools?
- Is a tool call visible before execution, after completion, or both?
- Is tool input visible? Is it redacted?
- Is tool output visible? Is it summarized, truncated, or raw?
- Are file changes represented as dedicated events or only as tool results?
- Are command execution exit codes and stdout/stderr represented structurally?
- Are subagent start/stop events visible?
- Are nested subagent tool calls visible to the parent stream?
- Can the caller inject instructions into subagent prompts for non-interactive behavior?

### Completion, Failure, and Blocking

- How does the provider represent normal completion?
- How does it represent model errors, provider errors, auth errors, rate limits, context
  overflow, budget limits, max-turn limits, cancellation, and user interruption?
- Is the process exit code reliable, or must Claudine parse terminal events?
- Can a run hang waiting for a user question, permission approval, MCP OAuth flow, or tool
  confirmation?
- Does the provider auto-deny, auto-approve, fail, ask through a programmable tool, or
  block indefinitely when no human input is available?
- Can permissions be preconfigured so a non-interactive run is deterministic?

### Use Case Detection

For each use case, say whether it is detectable, which event(s) identify it, which fields
to extract, how to distinguish it from nearby events, whether units/timezones are clear,
and whether an equivalent hook/log event exists.

- `plan_cap_approaching`: plan or quota approaching a cap; extract remaining quantity,
  unit, threshold, reset time, and cap window where possible.
- `plan_capped`: plan or quota exhausted; extract reset time, window, upgrade URL, or
  provider-specific remediation where possible.
- `no_funds`: insufficient balance, credits, billing, or quota funds.
- `auth`: invalid auth, missing auth, expired auth, or auth kind detected.
- `permission_read_denied`: read attempt denied; extract path, tool, reason, and policy.
- `permission_write_denied`: write/edit/delete attempt denied; extract path, tool,
  reason, and policy.
- `tokens_consumed`: token usage at session, turn, step, tool, or model granularity.
- `model_used`: requested model and resolved model/provider where available.
- `model_fallback`: fallback from requested model to another model.
- `human_in_loop`: question, approval, confirmation, or elicitation attempted in a
  non-interactive run.
- `session_resumable`: session ID or transcript reference that can be used for resume.
- `subagent_prompt_injection`: ability to steer subagents away from interactive behavior.

## Example Metadata Shape

Use this as an example of the level of specificity expected. Do not copy it unless it is
verified for the provider being researched.

```yaml
invocation:
  - command: "provider exec --json \"fix the tests\""
    stdin_support: true
    prompt_arg: "PROMPT or - for stdin"
    notes: "Use exec for CI; TUI is not started."
output_formats:
  - name: "stream-json"
    cli_value: "stream-json"
    stream: true
    format: ndjson
    description: "One JSON event per line on stdout."
    side_effects: "Disables ANSI styling and moves logs to stderr."
config_files:
  - os: macos
    scope: user
    path: "~/.provider/config.toml"
    format: toml
    effect: "Can set default output format and logging verbosity."
    notes: "CLI flags override this file for a single run; add Linux and Windows records with their platform-specific paths."
  - os: linux
    scope: user
    path: "~/.config/provider/config.toml"
    format: toml
    effect: "Can set default output format and logging verbosity."
    notes: "Example XDG location; verify against provider docs."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\config.toml"
    format: toml
    effect: "Can set default output format and logging verbosity."
    notes: "Example Windows location; verify against provider docs."
  - os: macos
    scope: repo
    path: ".provider/config.toml"
    format: toml
    effect: "Can override project-local output and tool settings."
    notes: "Repo-relative paths still need separate OS records; add Linux and Windows records explicitly."
io_contract:
  stdout: structured_only
  stderr: diagnostics_only
  stdin: prompt
  framing: ndjson
  noise_handling: "Ignore stderr unless process exits without a terminal event."
  notes: "Each stdout line is independently parseable JSON."
stream_contract:
  discriminator: "type"
  event_ordering: "session_start before tool events; result is terminal"
  correlation_fields: ["session_id", "tool_call_id"]
  terminal_event: "result"
  partial_message_events: true
  unknown_event_policy: "skip and log at trace"
  notes: "Tool progress events are optional."
```

## Sources and Evidence

Use current official documentation and local inspection where available. If source code is
the real schema, cite the exact file path, package, crate, or repository URL. If a claim
comes from local execution, include a small redacted example in the body or describe the
command used and the event shape observed. Do not include secrets, full home-directory
paths, API keys, private repository names, or user prompt content.

The final Markdown must be idiomatic CommonMark + GFM. Use Markdown tables for structured
comparisons and Mermaid diagrams only when they clarify event flow.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
