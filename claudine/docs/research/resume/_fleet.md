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
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> resume is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The resume research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the resume research on **{{state.name}}** completed successfully"
---

## Skills

Use the 'claudine' skill.

## Scope

Research session resume and continuation support for **{{state.desc}}**. This topic
feeds Claudine's lifecycle `resume` control action, recovery behavior, and future
human-in-the-loop continuation.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so
the document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

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

## Research Deliverables

Write frontmatter that captures these facts directly:

- `created`, `last_updated`, `agent`, and `model` identify the research run. Preserve
  `created` on update; set `last_updated` to `{{ctx.today}}`.
- `docs` is the best official URL for session/resume behavior. If only CLI reference
  docs exist, use those and explain the gap.
- `support` classifies the provider's practical resume support:
  `first_class`, `partial`, `interactive_only`, `non_interactive_only`, `none`, or
  `unknown`.
- `continuity_model` states whether resume is transcript replay, server-side session
  continuation, live-process attach, checkpoint snapshot, mixed, absent, or unknown.
- `resume_modes` lists each surface where resume can happen: interactive CLI,
  non-interactive command, headless server, IDE, API, or unknown. Include whether it
  accepts a follow-up prompt and how the target session is selected.
- `session_id_capture` records where stable resume handles come from: stdout, stderr,
  JSON stream, log files, transcript/session files, CLI commands, interactive UI, hooks,
  or another surface.
- `resume_invocations` records exact commands, slash commands, API calls, or UI paths.
  Include examples for continue-latest and explicit-handle resume when both exist.
- `state_storage` records where resumable state lives on macOS, Linux, Windows, or all
  OSes. Include path, format, retention, and whether the format is stable enough for
  Claudine to read directly.
- `resume_scope` records how session lookup is scoped: current directory, project,
  repository, worktree, branch, all projects, remote server, or explicit handle only.
- `branching_checkpointing` records whether the provider can fork, branch, rewind,
  checkpoint, or duplicate a session without corrupting the original.
- `restored_state` records what survives resume: transcript, tool results, approvals,
  model choice, working directory, roots, environment, and permissions.
- `hitl_resume` records whether an interrupted question, permission prompt, or tool
  approval can be captured and answered later by Claudine.
- `interruption_recovery` records what happens after crash, Ctrl+C, process kill,
  terminal close, network failure, pending tool calls, and pending approvals.
- `observability` records stream events, hook events, logs, transcript fields, status
  commands, or APIs that reveal session IDs, lifecycle state, resumability, or failure.
- `quirks` records provider-specific traps and unsafe assumptions.
- `gaps` records claims that could not be verified from docs or local inspection.
- `changes` records update-mode changes only; first-run documents should use `[]`.
- `requires_claudine_update` is `true` only when the research implies a Claudine code or
  generated-metadata change. Explain that in `reason`.

Use `support: none` only when the provider clearly cannot continue any prior session.
Use `unknown` when the docs do not prove the answer.

## Quality Bar

A good answer is specific enough that Claudine can build wrapper logic from it without
guessing. Prefer verified commands, concrete file paths, exact field names, and explicit
limitations over broad statements.

Do:

- Distinguish interactive resume from non-interactive resume.
- Distinguish "continue the latest session" from "resume this exact session."
- Distinguish session names, IDs, numeric indexes, PR handles, and picker-only selection.
- Explain whether a new prompt can be supplied at resume time.
- Explain whether resume keeps writing to the same transcript or creates a new one.
- Explain whether resumed runs inherit or reset approvals, model choice, tools, roots,
  sandbox settings, and working directory.
- Explain whether concurrent resumes of the same session are safe, rejected, or can
  interleave transcript state.
- Explain whether branch/fork/checkpoint features preserve the original session.
- Explain whether direct parsing of session files is supported, discouraged, or unstable.
- Explain whether session storage and lookup differ across macOS, Linux, and Windows.
- Cite official documentation first; use local inspection only for facts not documented.

Avoid:

- Saying "supports resume" without saying which mode and selector.
- Treating a chat-history export as true resume unless the provider can continue from it.
- Treating memory files or project instructions as resume; those are context sources, not
  a prior session continuation mechanism.
- Assuming an interactive picker can be automated.
- Assuming local transcript replay is equivalent to live server-side continuation.
- Assuming approval state, current working directory, or model choice survives resume
  unless verified.
- Parsing undocumented session files as a stable integration path without calling out the
  risk.

## Examples of Useful Variance

These examples show the level of specificity expected. Do not copy them into provider
files unless verified for that provider.

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

## Research Questions

- What does the provider mean by a "session"? Is it local transcript history, remote
  server state, a live process, an IDE conversation, or a combination?
- Which resume surfaces exist: CLI flag, subcommand, slash command, TUI picker, IDE
  command, local server API, SDK, or undocumented file inspection?
- Can a caller continue the latest session without knowing a handle?
- Can a caller target an exact session by ID, name, index, branch, PR, worktree, or path?
- Can a non-interactive command send a follow-up prompt into a prior session and return
  structured output?
- Does resume work for sessions created in non-interactive mode, or only interactive
  sessions?
- How are session IDs, names, transcript paths, or run IDs emitted in stdout, stderr,
  JSON streams, hooks, logs, status commands, or local files?
- Where is resumable state stored on macOS, Linux, and Windows? Is the format documented
  and stable, or explicitly internal?
- How is session lookup scoped by current directory, project, repository, worktree,
  branch, all projects, or remote server?
- Can sessions be renamed, deleted, exported, listed, searched, shared, branched, forked,
  rewound, or checkpointed?
- What state survives resume: conversation transcript, tool results, plan history,
  approvals, model, permissions, sandbox, cwd, extra roots, attachments, MCP servers,
  environment variables, and pending tool calls?
- Can a pending user question, approval prompt, or permission request be captured and
  answered later? If so, what API/hook/event carries the question and what API/command
  submits the answer?
- What happens after crash, Ctrl+C, terminal close, process kill, provider error,
  rate-limit pause, context compaction, or network loss?
- Can the same session be resumed concurrently from multiple terminals or processes? If
  so, are messages serialized, interleaved, rejected, or forked?
- Which behavior matters for Claudine's lifecycle `resume`, `retry`, `proxy`, and future
  human-in-the-loop recovery?

## Body Structure

- `## Overview`
- `## Resume Semantics`
- `## Supported Modes`
- `## Session ID Capture`
- `## Resume Invocation`
- `## Session Lookup Scope`
- `## State Storage`
- `## Restored State`
- `## Branching and Checkpoints`
- `## Human-in-the-Loop Resume`
- `## Interruption Recovery`
- `## Observability`
- `## Quirks and Gaps`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
