---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/resume/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local session files under {{state.user_dir}} when they exist.
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
              - message: "The provider **{{state.name}}** needs to update its research on **resume**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **resume** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Resume** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Resume** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Resume research on **{{state.name}}** failed to complete!"
    warn: "The Resume research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Resume Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research session resume and continuation support for **{{state.desc}}**. This topic
feeds Claudine's lifecycle `resume` control action, recovery behavior, and future
human-in-the-loop continuation.

**Boundary:** the sibling `non-interactive-sessions` topic owns non-interactive
invocation and follow-up mechanics — commands, output formats, stream parsing. This
topic owns session identity, persistence, and re-entry semantics. Record the
resume-relevant invocations here, but leave stream/format depth to that topic; it
reciprocally leaves resume semantics to this one.

## What Resume Means

For this research, **resume** means any provider-supported way to continue, reopen,
reattach to, branch from, or send a follow-up prompt into a prior agent session.
Do not assume that all providers mean the same thing by resume.

A provider may implement resume as one or more of these patterns:

- **Continue latest**: reopen the most recent session for the current directory or
  workspace, usually without the caller knowing a session ID.
- **Resume by handle**: resume a specific prior session by ID, name, index, PR, branch,
  worktree, or another provider-defined selector.
- **Interactive picker**: let a user browse, preview, search, rename, delete, or select
  sessions in a TUI or slash command.
- **Non-interactive follow-up**: send a new prompt into a prior session from a
  scriptable command and capture the answer.
- **Transcript replay**: reconstruct model context from saved local transcript/history
  records rather than reattaching to live model state.
- **Server-side session**: continue a session whose authoritative state is held by a
  provider service or local headless server.
- **Live-process attach**: connect a client to a still-running agent/server process.
- **Branch, fork, rewind, checkpoint**: resume from a prior point while preserving or
  copying the original session.
- **Recovery resume**: continue after terminal close, crash, Ctrl+C, process kill,
  network failure, approval interruption, or tool failure.
- **Human-in-the-loop resume**: stop on a user question or permission request, let
  Claudine ask the user elsewhere, then inject the answer and continue the same session.

The goal is not just "does it have a `resume` command?" The goal is to understand what
state is preserved, how a wrapper can target the right session, whether the behavior is
safe for automation, and where Claudine must compensate.

## Document Structure

The research deliverable is a prose document a maintainer can learn the provider's
resume behavior from; frontmatter is distilled from this body afterward, never invented
separately. A good body is specific enough that Claudine can build wrapper logic from
it without guessing: prefer verified commands, concrete file paths, exact field names,
and explicit limitations over broad statements. Write the body of `{{file}}` using
these sections:

- `## Overview` Section
    - One or two paragraphs: the provider's practical resume support level, the
      continuity model behind it, and the main risks for a wrapper
- `## Resume Semantics` Section
    - What the provider means by a "session": local transcript history, remote server
      state, a live process, an IDE conversation, or a combination
    - Which of the What-Resume-Means patterns above apply, and which continuity model
      backs them — local transcript replay is not equivalent to live server-side
      continuation, so say which one this is
    - Guard: a chat-history export is not resume unless the provider can continue
      from it; memory files and project instructions are context sources, not a prior
      session continuation mechanism
- `## Supported Modes` Section
    - Which resume surfaces exist: CLI flag, subcommand, slash command, TUI picker,
      IDE command, local server API, SDK
    - Distinguish interactive from non-interactive resume, and "continue the latest
      session" from "resume this exact session"; say whether each mode accepts a
      follow-up prompt
    - Distinguish session names, IDs, numeric indexes, PR handles, and picker-only
      selection — and do not assume an interactive picker can be automated
    - Say whether sessions created in non-interactive mode can themselves be resumed,
      or only interactive ones
- `## Session ID Capture` Section
    - How session IDs, names, transcript paths, or run IDs are emitted: stdout,
      stderr, JSON streams, hooks, logs, status commands, or local files
    - How early a handle becomes available and whether it is stable enough for a
      later resume
- `## Resume Invocation` Section
    - Exact commands, slash commands, API calls, or UI paths; include examples for
      continue-latest and explicit-handle resume when both exist
    - Whether a new prompt can be supplied at resume time and whether the follow-up
      answer can be captured as structured output
- `## Session Lookup Scope` Section
    - How session lookup is scoped: current directory, project, repository, worktree,
      branch, all projects, or remote server — and whether storage and lookup differ
      across macOS, Linux, and Windows
- `## State Storage` Section
    - Where resumable state lives on macOS, Linux, and Windows: path, format, and
      retention
    - Whether the format is documented and stable, or explicitly internal — if direct
      parsing is possible but unsupported, call out the risk instead of presenting it
      as an integration path
- `## Restored State` Section
    - What survives resume: conversation transcript, tool results, plan history,
      approvals, model, permissions, sandbox, cwd, extra roots, attachments, MCP
      servers, environment variables, and pending tool calls
    - Do not assume approval state, working directory, or model choice survives
      unless verified; say what is inherited, reset, or overridable at resume time
    - Whether resume keeps writing to the same transcript or creates a new one
- `## Branching and Checkpoints` Section
    - Whether sessions can be renamed, deleted, exported, listed, searched, shared,
      branched, forked, rewound, or checkpointed — and whether branch/fork/checkpoint
      features preserve the original session
- `## Human-in-the-Loop Resume` Section
    - Whether a pending user question, approval prompt, or permission request can be
      captured and answered later; which API/hook/event carries the question and
      which API/command submits the answer
- `## Interruption Recovery` Section
    - What happens after crash, Ctrl+C, terminal close, process kill, provider error,
      rate-limit pause, context compaction, or network loss — including pending tool
      calls and pending approvals
    - Whether concurrent resumes of the same session are safe, rejected, or can
      interleave transcript state
- `## Observability` Section
    - Stream events, hook events, logs, transcript fields, status commands, or APIs
      that reveal session IDs, lifecycle state, resumability, or failure
- `## Quirks and Gaps` Section
    - Provider-specific traps and unsafe assumptions; claims that could not be
      verified belong here as gaps rather than being silently dropped
- `## Claudine Integration Notes` Section
    - What the findings mean for Claudine's lifecycle `resume`, `retry`, `proxy`, and
      future human-in-the-loop recovery
- `## Changelog` Section (update runs only)
    - Summarize what changed since the prior research
- `## Sources`
    - add all useful resources you used as Markdown links; cite official
      documentation first and use local inspection for facts not documented

## Examples of Useful Variance

These examples show the level of specificity expected in the distilled frontmatter.
Do not copy them into provider files unless verified for that provider.

```yaml
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms: ["session picker", "resume by name"]
    accepts_followup_prompt: false
    selection_methods: [latest, id, name, picker, worktree, all_projects]
    notes: "Picker is human-oriented; direct ID/name resume is scriptable."
  - mode: non_interactive
    supported: true
    mechanisms: ["print-mode resume"]
    accepts_followup_prompt: true
    selection_methods: [id]
    notes: "Follow-up prompt can be sent to an existing transcript and returned as JSON."
```

```yaml
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Transcript is replayed; launch-time flags can override model and cwd."
```

```yaml
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: true
  notes: "Default lookup is current project; picker can widen to all projects."
```

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
    > host. Inspect the *actual* session files there — storage paths, file formats,
    > and what a real transcript contains — and prefer what you observe over what
    > documentation claims. Negative probes are evidence too — "no session files
    > exist for non-interactive runs" is a finding. Unanswered is not the same as
    > omitted: record `unknown` with a note rather than dropping a field.

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
    - `docs` - the best official URL for session/resume behavior; if only CLI
      reference docs exist, use those and explain the gap in the body
    - `support` - the classification from `## Overview`. Use `none` only when the
      provider clearly cannot continue any prior session; use `unknown` when the docs
      do not prove the answer
    - `continuity_model` - from `## Resume Semantics`
    - `resume_modes` - from `## Supported Modes`
    - `session_id_capture` - from `## Session ID Capture`
    - `resume_invocations` - from `## Resume Invocation`
    - `resume_scope` - from `## Session Lookup Scope`
    - `state_storage` - from `## State Storage`; one record per OS — storage paths
      must be recorded separately for macOS, Linux, and Windows (never one record
      for all OSes; Windows paths always differ)
    - `restored_state` - from `## Restored State`
    - `branching_checkpointing` - from `## Branching and Checkpoints`
    - `hitl_resume` - from `## Human-in-the-Loop Resume`
    - `interruption_recovery` - from `## Interruption Recovery`
    - `observability` - from `## Observability`
    - `quirks` and `gaps` - from `## Quirks and Gaps`
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on the changes discovered in your research.
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
