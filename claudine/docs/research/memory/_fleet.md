---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/memory/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local memory stores under {{state.user_dir}} when they exist.
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
              - message: "The provider **{{state.name}}** needs to update its research on **memory**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** has research for **memory** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Memory** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Memory** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Memory research on **{{state.name}}** failed to complete!"
    warn: "The Memory research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Memory Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Survey the **memory** offering of **{{state.desc}}** as it exists today. Claudine will
eventually add its own memory system; before that design process starts, we need a
structured landscape survey of what each provider ALREADY offers as "memory". This
topic is **design input** for that future system — it is deliberately **not** wired to
catalog codegen, and no generated `ProviderInfo` field consumes it.

Prefer provider source code over documentation whenever public source exists; use
official documentation otherwise. Always distinguish **shipped** behavior from
**announced/roadmap** behavior — record announced features in the body and in `gaps`,
never in the structured frontmatter as if they shipped.

**Boundary:** the sibling `system-prompt` topic owns context-file loading — which
instruction files a provider auto-loads (`CLAUDE.md`, `AGENTS.md`, and friends) and how
they layer into the effective prompt. The catalog's `memory_files` field belongs to
that surface and is out of scope here. This topic owns memory as a **persistent
knowledge store**: what is written back over time, where it lives, who writes it, and
when it re-enters context. Where the two overlap (a memory file that is also an
auto-loaded context file), reference the system-prompt research rather than duplicating
it, and keep this document focused on the store-and-recall semantics.

## What Memory Means

For this research, **memory** means any provider-supported way to persist knowledge
across turns or sessions and bring it back into model context later. Do not assume all
providers mean the same thing by memory. A provider may implement one or more of:

- **Model-written auto memory**: the model decides to record facts/preferences during
  or after a session without the user curating them.
- **User-curated memory files**: files the user writes or approves that the provider
  treats as durable memory (distinct from ordinary project instructions).
- **Session-scoped memory**: state that persists within one session or conversation
  but not beyond it.
- **Project-scoped memory**: stores keyed to a repository/project directory.
- **Semantic or vector memory**: embedding-backed retrieval over past content.
- **Extension/plugin memory**: memory shipped as an optional extension rather than
  core behavior (record it, but say so).

Known examples of the kind of thing to find (verify — do not copy blindly): Claude
Code's auto-memory `MEMORY.md` directories, Codex's `~/.codex/memories_1.sqlite`,
Goose's memory extension storage, and Qwen's `memory/MEMORY.md`.

The goal is not just "does it have a memory feature?" The goal is to understand the
mechanism well enough that a future Claudine memory design can learn from it: what is
stored, in what format, who writes it, when it loads, what the user can control, and
where the sharp edges are.

## Document Structure

The research deliverable is a prose document a maintainer can learn the provider's
memory behavior from; frontmatter is distilled from this body afterward, never invented
separately. Prefer verified paths, observed file contents, exact command/flag names,
and explicit limitations over broad statements. Write the body of `{{file}}` using
these sections:

- `## Overview` Section
    - One or two paragraphs: what the provider ships as memory today, which of the
      What-Memory-Means patterns apply, and how mature/central the feature is
- `## Memory Mechanisms` Section
    - One subsection per distinct mechanism; name it with the provider-native term
    - For each: what it stores, its scope (global/project/session), and whether it is
      core behavior or an optional extension
- `## Storage` Section
    - Where each store lives on macOS, Linux, and Windows: path, format (sqlite,
      jsonl, markdown, ...), and scope
    - Inspect actual stores under `{{state.user_dir}}` when they exist — observed
      shapes beat documented ones; "no memory store exists on this host" is a finding
    - Whether the format is documented/stable or explicitly internal
- `## Write Model` Section
    - Who writes memory (model, user, system) and what triggers a write
    - Whether writes are visible to the user when they happen, and whether they
      require confirmation
- `## Load Model` Section
    - When memory enters context: session start, on demand, retrieval-based
    - How the provider resolves which store(s) to load (global vs project vs session)
- `## User Controls` Section
    - How a user can enable, disable, edit, clear, or inspect memory — flags,
      commands, settings, or direct file edits
- `## System Prompt Interaction` Section
    - How memory content relates to the system prompt and context files; reference
      the system-prompt topic's research for the loading mechanics instead of
      re-documenting them
- `## Limits and Expiry` Section
    - Size caps, pruning, retention, and how memory interacts with context
      compaction
- `## Portability` Section
    - Whether the store can be exported, shared between machines/users, or migrated
- `## Shipped vs Announced` Section
    - Anything announced, experimental, or flag-gated that is not yet default
      behavior
- `## Quirks and Gaps` Section
    - Provider-specific traps and unsafe assumptions; claims that could not be
      verified belong here as gaps rather than being silently dropped
- `## Claudine Memory Design Notes` Section
    - What this provider's approach suggests for a future Claudine-owned memory
      system — explicitly speculative and non-binding
- `## Changelog` Section (update runs only)
    - Summarize what changed since the prior research
- `## Sources`
    - add all useful resources you used as Markdown links; cite provider source code
      and official documentation first

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLIs change is rapid and therefore you
    > should assume that the prior research is out of date. You are reading this
    > primarily to be able to effectively report the changes into the `## Changelog`
    > section of the document. Critically, you should never substitute information in
    > the old research for doing your own (up-to-date) research.

::end-block
- Perform research on the topic

    > **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
    > host. Inspect the *actual* memory stores there — storage paths, file formats,
    > and what a real store contains — and prefer what you observe over what
    > documentation claims. Negative probes are evidence too. Unanswered is not the
    > same as omitted: record `unknown` or a `gaps` entry with a note rather than
    > dropping a field.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save the research to `{{file}}`, following the Document Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative field contract, and `md schema validate` will enforce it against
    > everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `docs` - the best official URL for the provider's memory feature(s); if none
      exists, omit it and record that in `gaps`
    - `memory_kinds` - from `## Memory Mechanisms`; one record per distinct mechanism
      with the provider-native `name`, a `kind` classification, and a `description`
    - `storage` - from `## Storage`; one record per OS/location with `path`, `format`,
      and `scope`. Use `os: all` only when the path is verified identical across
      operating systems
    - `write_model` - from `## Write Model`; one record per writer with `writer`,
      a free-form `trigger`, and `user_visible`
    - `load_model` - from `## Load Model`; a prose `summary` plus `timing` and
      `scope_resolution` when the provider distinguishes them
    - `user_controls` - from `## User Controls`; one record per control with
      `control`, `mechanism`, and `notes`
    - `system_prompt_interaction` - prose distilled from `## System Prompt
      Interaction`
    - `limits_and_expiry` - prose distilled from `## Limits and Expiry`
    - `portability` - prose distilled from `## Portability`
    - `claudine_notes` - prose distilled from `## Claudine Memory Design Notes`
    - `gaps` - from `## Quirks and Gaps`; unverified claims and missing evidence
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe
      there will be required code changes to **Claudine** based on your research.
      This topic is design input, not codegen-wired, so `false` is the expected
      answer unless you found something that breaks Claudine's current wrapper
      behavior.
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, following the Document Structure
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
