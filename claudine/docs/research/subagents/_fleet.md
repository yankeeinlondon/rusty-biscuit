---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/subagents/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local agent folders under {{state.user_dir}} when they exist.
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
              - message: "The provider **{{state.name}}** needs to update its research on **subagents**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **subagents** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Subagents** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Subagents** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Subagents research on **{{state.name}}** failed to complete!"
    warn: "The Subagents research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Subagent Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research user-defined **agent, subagent, mode, persona, and worker definitions** for
**{{state.desc}}**. This topic feeds Claudine's agent linking module and future
lifecycle `resume`/`proxy` behavior, so the research must cover both static definition
files and runtime delegation semantics.

For this topic, a subagent-like resource is any durable definition that can change who
does work inside a session: named subagents, agents, modes, personas, roles, workers,
specialists, tool-scoped agents, extension-provided agents, or documented conventions
that provide equivalent delegation. Do not count a one-off prompt, an ordinary model
selection flag, or a built-in mode alone unless users can define their own equivalent.
Likewise, do not re-document lifecycle-event semantics here — the hooks topic owns
those; record only *which* events expose agent lifecycle.

Cross-topic ownership: the hooks topic owns event semantics (this topic records only
which stream/hook events expose agent lifecycle); the plugins topic's
`packaged_resources` records containment only — agent definitions packaged inside
plugins still have their semantics documented here.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Research Deliverables

Write prose specific enough that Claudine can link definitions and reason about runtime
delegation without guessing. Prefer exact paths, metadata keys, invocation examples,
stream events, and inheritance rules over broad statements.

In the body, cover:

- What the provider calls the feature, or the closest equivalent when there is no named
  "subagent" feature.
- Which scopes exist: user, repo/project, workspace, system, extension/plugin, or other.
- The exact storage locations on macOS, Linux, and Windows. Use separate OS records for
  every filesystem path; do not use `os: all`.
- The definition format: file names, frontmatter/config keys, required fields,
  optional fields, body format, prompt text, model/tool/permission metadata, and
  examples.
- Runtime behavior: how an agent is invoked or selected, whether delegation is automatic
  or explicit, whether multiple agents can run, whether parent and child contexts are
  isolated or shared, and what state returns to the parent.
- Inheritance: model, tools, MCP servers, roots, sandbox, approvals, permissions,
  environment, memory, max turns, and prompt context.
- Observability: whether starts/stops are visible in streams, hooks, logs, transcripts,
  session IDs, or status APIs.
- CLI flags, environment variables, config files, safe mode, trust, profiles, or
  extensions that affect agent definition loading.
- Portability: which definitions can be linked as-is, which need provider-specific
  rewrites, and which are unsafe or meaningless outside the provider.
- Claudine integration notes: what the agent linker should do, what lifecycle `proxy`
  or `resume` can rely on, and whether code or generated-metadata changes are needed.

## Frontmatter Contract

Read `./_schema.yaml` before writing. It is the machine-validated contract. Populate
frontmatter as follows:

- `$schema` - set to the string `./_schema.yaml`.
- `created` - first-run date, `{{ctx.today}}`. Preserve the existing value on update.
- `last_updated` - set to `{{ctx.today}}`.
- `agent` - set to `{{env.AGENT}}`.
- `model` - set to `{{env.MODEL || 'default'}}`.
- `homepage` - provider homepage URL, when useful for identification.
- `docs` - best general official documentation URL for this provider's CLI/config.
- `subagent_docs` - best official URL specifically covering subagents, agents, modes,
  personas, or the nearest equivalent. Omit only when no such page exists and explain
  that gap in the body.
- `support` - one of:
  - `first_class`: the provider has named, documented user-defined agents/subagents.
  - `partial`: definitions exist but with major limits such as one scope, no delegation
    API, unstable format, or no automatic discovery.
  - `convention_only`: there is no formal feature, but documented reusable persona or
    mode files can act as agent definitions Claudine could link.
  - `none`: user-defined agents/subagents or equivalents are clearly absent.
  - `unknown`: current sources do not prove the answer.
- `locations` - one record per definition storage location: `os`, `scope`, `path`, and
  optional `notes`. Use template paths like `~/.provider/agents` or `.provider/agents`.
- `format` - summarize the definition artifact:
  - `file_names`: accepted names or glob patterns such as `*.md`, `AGENTS.md`, or
    `agents/*.json`.
  - `frontmatter`: whether frontmatter is recognized.
  - `required_fields`: metadata keys required by the provider.
  - `optional_fields`: recognized metadata keys such as model, tools, color,
    description, permissions, or max turns.
  - `body_format`: `markdown`, `yaml`, `json`, `toml`, `text`, `other`, or `unknown`.
  - `notes`: include examples, directory layout, extension behavior, or undocumented
    constraints.
- `runtime` - describe delegation behavior:
  - `invocation`: slash command, automatic router, tool call, CLI flag, config selector,
    IDE UI, or "none documented"; include examples.
  - `parent_child_context`: what context the child receives and what returns to the
    parent.
  - `permissions_inheritance`: whether approvals, sandbox, policy, and trust inherit,
    narrow, widen, or reset.
  - `model_inheritance`: whether the child inherits the parent model or declares its
    own.
  - `tool_inheritance`: whether tools/MCP servers inherit, narrow, widen, or reset.
  - `max_turns`: documented turn/iteration limits or "none documented".
  - `notes`: concurrency, nested delegation, failure behavior, or selection quirks.
- `observability` - describe lifecycle visibility:
  - `stream_events`: event names or JSON fields emitted when agents start/stop.
  - `hook_events`: hook names that expose agent lifecycle, if any.
  - `session_ids`: true if child agents have stable IDs or transcript/session handles.
  - `notes`: log locations, transcript fields, status APIs, or absence of visibility.
- `portability` - Claudine's linking classification:
  - `portable`: true only when a definition can be linked/copied to another provider
    with no semantic rewrite beyond path placement.
  - `non_portable_assets`: provider-specific metadata, tool names, MCP references,
    permission keys, prompts, scripts, or attachments.
  - `rewrite_needed`: true when content or metadata must be transformed.
  - `notes`: describe the exact rewrite or why no safe rewrite exists.
- `cli_params` - every CLI flag/subcommand that affects agent definition discovery,
  selection, profiles, extensions, trust, safe mode, model selection, permissions, or
  disabling. Use `[]` only after checking docs and `--help`.
- `env_vars` - environment variables that influence agent paths, config roots, profiles,
  trust, extensions, models, permissions, or disabling. Use `[]` only when verified
  absent.
- `changes` - on first run, `[]`; on update, concise strings describing changes since
  the previous research. Do not use old research as proof for current facts.
- `requires_claudine_update` - `true` only when Claudine code, schemas, generated
  metadata, or linking/lifecycle rules should change because of the research.
- `reason` - required when `requires_claudine_update` is true; otherwise a short
  explanation is still useful.

## Useful Examples

These examples show the expected specificity. Do not copy them unless verified for
{{state.name}}.

```yaml
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.provider/agents"
    notes: "User agents are available in all workspaces on macOS."
  - os: linux
    scope: user
    path: "~/.config/provider/agents"
    notes: "Example Linux/XDG location; verify exact provider behavior."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\agents"
    notes: "Example Windows location; verify exact provider behavior."
  - os: macos
    scope: repo
    path: ".provider/agents"
    notes: "Repo agents require folder trust; add Linux and Windows records explicitly."
format:
  file_names: ["*.md"]
  frontmatter: true
  required_fields: ["name", "description"]
  optional_fields: ["model", "tools", "max_turns"]
  body_format: markdown
  notes: "Body becomes the agent's system prompt."
```

```yaml
runtime:
  invocation: "The main agent delegates with the Task tool using subagent_type: reviewer."
  parent_child_context: "Child receives the task prompt plus selected transcript context; final response returns to parent."
  permissions_inheritance: "Child inherits session approval mode but may narrow tools via definition metadata."
  model_inheritance: "Definition model overrides parent model when set; otherwise parent model is used."
  tool_inheritance: "Definition tools are an allowlist; omitted tools inherit provider defaults."
  max_turns: "Optional max_turns metadata limits child iterations."
  notes: "Nested delegation is not documented."
observability:
  stream_events: ["subagent_start", "subagent_stop"]
  hook_events: []
  session_ids: true
  notes: "Transcript includes child agent name and ID."
```

## Research Questions

- Does the provider support user-defined agents, subagents, modes, personas, workers, or
  equivalent resources?
- Where are definitions stored by OS and scope?
- What file names, metadata, and body formats are recognized?
- How are agents invoked, selected, delegated to, disabled, trusted, or overridden?
- What context, permissions, model, tool, MCP, sandbox, and turn-limit inheritance
  applies?
- Are agent starts/stops visible in streams, hooks, logs, transcripts, or session IDs?
- Can Claudine target a specific subagent from a wrapper or lifecycle action, or is
  selection only model-driven/interactive?
- Which definitions are portable across providers, and which need rewriting?

## Body Structure

- `## Overview` — what the provider calls the feature and how complete the support is.
- `## Locations` — exact template paths per OS and scope, noting which were observed
  locally versus documented only.
- `## Definition Format` — file names, metadata keys, body format, and a small real
  example of a definition.
- `## Runtime Behavior` — how delegation is triggered, what context and permissions the
  child inherits, what returns to the parent, and concurrency/nesting limits.
- `## Observability` — which stream events, hook events, logs, transcripts, or session
  IDs make agent starts/stops visible to a wrapper.
- `## Portability` — which definitions link as-is, which need rewriting, and why.
- `## Claudine Linking Notes` — what the agent linker and lifecycle `proxy`/`resume` can
  rely on for this provider.
- `## Changelog` when `update` is true
- `## Sources`

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`.

    > Prior research may be stale. Use it to preserve useful topics and write the
    > changelog, not as proof of current behavior.

::end-block
- Research the current behavior using official documentation first, then source code,
  release notes, `--help`, and local inspection where useful.
- Inspect `{{state.user_dir}}` when it exists and the provider stores agent definitions
  there. State what you observed, including when no local config/resources exist.
::block when="update"
- Update `{{file}}` with current research and add a `## Changelog` entry.
::end-block
::block when="!update"
- Write and save the new research document to `{{file}}`.
::end-block
- Set all frontmatter required by `./_schema.yaml`.
- Cite sources as Markdown links in `## Sources`.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done when `{{file}}` has been saved with complete prose research, all
frontmatter fields populated appropriately, `$schema: ./_schema.yaml`, and
`md schema validate '{{file}}'` returns `true`.

- You do not need to run tests or lints.
- This task has no code modifications.
