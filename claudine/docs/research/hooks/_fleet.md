---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/hooks/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local config examples under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **hooks**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **hooks** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Hooks** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Hooks** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Hooks research on **{{state.name}}** failed to complete!"
    warn: "The Hooks research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Hooks Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research the hook/event system for **{{state.desc}}**. This topic feeds Claudine's
unified lifecycle event model, provider adapters, hook installation, and blocking
behavior, so the goal is to understand which native events exist, when they fire, what
payloads they carry, and what a hook's response can make the provider do.

Boundaries against sibling topics:

- This topic owns **event semantics**: the native event inventory, timing, payload
  shapes, and return/response contracts.
- Plugin `hooks.json` files and skill-scoped hooks are **containers** documented by the
  `plugins` and `skills` topics; this topic documents the event model those containers
  hook into. Record container file locations in `config_files` when they configure
  hooks, but leave container packaging and discovery to those topics.
- The `subagents` topic records only *which* events expose agent lifecycle; the full
  semantics of those events live here.
- The `mcp` topic owns MCP server configuration, security, and notification surfaces;
  when MCP activity fires lifecycle events, document the event semantics here and leave
  server config to `mcp`.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Document Structure

Ask "which hook mechanisms exist and how do they work", never "does {{state.name}}
support hooks". Write prose specific enough that Claudine can build a provider adapter
from it without guessing: exact event names, exact payload fields, exact return values.
Write the body with these H2 sections:

- `## Overview`
    - What the hook system is: the handler kinds it supports (shell commands, HTTP
      endpoints, LLM evaluators, or other), where it sits in the provider lifecycle,
      and a capability summary — can hooks block, mutate, or only observe?
- `## Native Hooks`
    - The full native event inventory — one entry per provider-native event, never one
      per Claudine event
    - Classify each event's timing precisely:
        - `pre` — runs before the provider action and may affect whether it happens
        - `post` — runs after the provider action
        - `around` — wraps an action or can observe both request and result
        - `async` — fire-and-forget or background notification
        - `unknown` — the hook is documented but its timing is unclear
    - Whether each event can block, allow, deny, mutate, replace, or stop execution,
      and any matcher/filter mechanism that scopes when a hook fires
- `## Configuration`
    - Where hooks are configured or installed, by OS and scope. State macOS, Linux,
      and Windows paths separately — never claim one path covers all OSes; Windows
      path syntax alone makes that ambiguous
    - CLI switches and commands that install, list, test, or disable hooks
    - Environment variables that disable hooks entirely, redirect the config roots
      hooks are loaded from, or otherwise alter hook execution. The schema has no
      dedicated env-var field, so document these controls here in prose and reflect
      execution-relevant ones in the `execution` capture — do not invent frontmatter
      keys beyond the schema
- `## Payloads and Responses`
    - The payload shape for each meaningful event: use dot paths for nested fields
      (e.g. `tool_input.command`) and cover every field useful for routing, security
      decisions, display, or correlation
    - The response contract per event: which exit code allows, which blocks, and where
      stderr goes (shown to the user versus fed back to the model); JSON/stdout
      response shapes; and whether a hook can rewrite the pending action
    - Record provider-native return values verbatim — when the provider uses process
      exit codes rather than JSON responses, the exit code *is* the native value
- `## Execution Semantics`
    - Shell selection, working directory, environment, default timeouts (per event
      where they differ), stdin/stdout/stderr handling, sequential versus parallel
      execution, async/background hooks, and platform caveats
- `## Claudine Mapping`
    - Map each native event to the closest Claudine lifecycle event; use `unknown`
      when the timing is known but the Claudine mapping is unclear, and `none` when a
      native hook has no useful lifecycle equivalent
    - Call out many-to-one collisions and the provider-specific payload fields
      Claudine must preserve on the unified payload
- `## Gaps`
    - Claims that could not be verified, missing provider docs, or behavior Claudine
      cannot model yet
- `## Changelog`
    - only when `update` is true
- `## Sources`
    - all useful resources you used in your research, as Markdown links; every
      mechanism claim needs a URL or an observed-on-host reference

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** agentic CLIs change rapidly, so assume the prior research is out of
    > date. You are reading it primarily to report changes into the `## Changelog`
    > section. Never substitute information from the old research for doing your own
    > up-to-date research.

::end-block
- Perform research on the topic using official documentation first, then source code,
  release notes, `--help`, and local inspection

    > **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
    > host. Inspect the *actual* hook configurations there — settings files, hook
    > scripts, installed hook containers — and prefer what you observe over what
    > documentation claims. Negative probes are evidence too ("the real settings file
    > has no hooks key" is a finding). Unanswered is not the same as omitted — record
    > `unknown` with a note rather than dropping a field, and never invent behavior
    > to complete the exercise.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`, with the body organized per the Document
  Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative field contract, expressed as a `SimpleSchema`, and
    > `md schema validate` will enforce it against everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `homepage`, `docs`, `hooks_docs` - primary URLs for the product, general docs,
      and hook-specific docs; prefer official docs and source files over third-party
      summaries. Omit `hooks_docs` only when no such page exists, and explain the gap
      in the body
    - `hooks` - one record per native event from `## Native Hooks`: `native_event`,
      `claudine_event` (from `## Claudine Mapping`), `timing`, `blocking`,
      `payload_schema`, `return_contract`, and `notes`
    - `config_files` - one record per OS + scope + path from `## Configuration`; use
      template paths (e.g. `~/.provider/settings.json`), and never a single record
      claiming to cover all OSes
    - `cli_params` - the hook-affecting flags/commands from `## Configuration` only —
      no unrelated general flags
    - `payload_fields` - the dot-path field records from `## Payloads and Responses`;
      one record per meaningful field
    - `response_actions` - one record per thing a hook response can make the provider
      do, carrying the provider-native value (exit code or JSON) and its actual
      effect, from `## Payloads and Responses`
    - `execution` - one object for the provider's general execution model from
      `## Execution Semantics`; put event-specific caveats in `hooks[].notes`
    - `gaps` - the unverified claims listed in `## Gaps`
    ::block when="update"
    - `changes` - a list of string descriptions summarizing the changes discovered
      since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe
      Claudine code or generated metadata must change because of your research —
      not merely because documentation changed
        - if `true`, you must also set the `reason` frontmatter property to describe
          why

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, organized per the Document Structure
2. all Frontmatter properties set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all
   Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
