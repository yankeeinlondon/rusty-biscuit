---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://moonshotai.github.io/kimi-cli/en/guides/sessions.html
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "kimi --continue"
      - "kimi --session (picker)"
      - "kimi --session <id>"
      - "/sessions slash command"
      - "/resume slash command"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
    notes: "Follow-up prompts are typed inside the resumed interactive session. The /sessions picker can toggle current-directory-only vs all directories with Ctrl-A."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "kimi --print --session <id>"
      - "kimi --print --continue"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Not explicitly documented as a combined invocation, but --print, --session, and --continue are global options and compose naturally. --print implies --afk."
  - mode: headless_server
    supported: true
    mechanisms:
      - "kimi acp"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Multi-session ACP server over stdio. Session creation/loading is JSON-RPC."
  - mode: ide
    supported: true
    mechanisms:
      - "ACP-compatible editors (Zed, JetBrains)"
      - "VS Code extension"
    accepts_followup_prompt: true
    selection_methods:
      - picker
      - id
    notes: "IDE clients connect via kimi acp or the dedicated extension."
  - mode: api
    supported: true
    mechanisms:
      - "ACP JSON-RPC"
      - "Wire protocol (--wire)"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Wire protocol exposes prompt/replay/steer/cancel; ACP exposes session lifecycle."
session_id_capture:
  - surface: stdout
    field: "resume command hint"
    format: "kimi -r <session-id>"
    notes: "Printed on exit for non-empty sessions after normal exit, Ctrl-C, /undo, /fork, /sessions switch, etc."
  - surface: session_file
    field: "session directory name"
    format: "<session-id>"
    notes: "Session data lives under ~/.kimi/sessions/<work-dir-hash>/<session-id>/."
  - surface: cli_command
    field: "sessionId"
    format: "uuid"
    notes: "kimi export [sessionId] accepts or defaults to the previous session for the cwd."
  - surface: interactive_ui
    field: "session title + last update"
    format: "text"
    notes: "/sessions and /resume show titles and timestamps; raw ID is hidden in the picker."
  - surface: hook
    field: session_id
    format: "uuid"
    notes: "SessionStart/SessionEnd hooks receive session_id and cwd in stdin JSON."
  - surface: json_stream
    field: session_id
    format: "uuid"
    notes: "Wire and ACP protocols carry session_id in lifecycle messages."
resume_invocations:
  - mode: interactive
    invocation: "kimi --continue"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recent session for the current working directory."
  - mode: interactive
    invocation: "kimi --session"
    accepts_prompt: false
    selection: picker
    notes: "Opens the interactive session picker (shell mode only)."
  - mode: interactive
    invocation: "kimi -r <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Aliases: --session, --resume, -S. Creates a new session if the ID does not exist."
  - mode: interactive
    invocation: "/sessions"
    accepts_prompt: false
    selection: picker
    notes: "Alias /resume. Lists sessions for the cwd; Ctrl-A toggles all directories."
  - mode: non_interactive
    invocation: "kimi --print --session <session-id> -p \"<prompt>\""
    accepts_prompt: true
    selection: id
    notes: "Likely valid composition; not explicitly documented. --print implies --afk."
  - mode: non_interactive
    invocation: "kimi --print --continue -p \"<prompt>\""
    accepts_prompt: true
    selection: latest
    notes: "Continues the latest cwd session with a new prompt in print mode."
  - mode: api
    invocation: "kimi acp (JSON-RPC session/load)"
    accepts_prompt: true
    selection: id
    notes: "ACP server loads or creates sessions by ID."
state_storage:
  - location: local
    os: all
    path: "~/.kimi/sessions/<work-dir-hash>/<session-id>/"
    format: "directory with context.jsonl, wire.jsonl, state.json, plus subagents/<id>/"
    retention: "not documented"
    notes: "Default share dir is ~/.kimi; override with KIMI_SHARE_DIR. context.jsonl is the conversation transcript; state.json stores approval/plan/additional-dirs/subagent state. Local inspection found an installed kimi-code binary using ~/.kimi-code/sessions/wd_<name>_<hash>/session_<uuid>/ instead, with a different state.json schema."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "--continue is cwd-scoped. /sessions defaults to cwd and can widen to all projects with Ctrl-A. Resuming by ID ignores scope."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "/fork"
  checkpoint_invocation: ""
  preserves_original: true
  notes: "/fork copies the full conversation history into a new session and switches to it. /undo also forks a new session with history before the selected turn, leaving the original intact. Web UI can fork from any assistant message. There is no rewind/checkpoint command."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: restored
  env: current_process
  notes: "Restores conversation context from context.jsonl, approval state (YOLO/AFK and allow-for-this-session), plan mode, additional directories, and subagent instance state. Model comes from config.toml and can be overridden with --model. Environment is taken from the launching process."
hitl_resume:
  supported: false
  question_capture: ""
  answer_injection: ""
  limitations: "AskUserQuestion is a tool call that expects an immediate answer. In --print mode --afk auto-dismisses questions. Wire/ACP expose QuestionRequest as a blocking JSON-RPC request. There is no documented defer-exit-and-resume-with-answer flow."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Resume hint is printed after Ctrl-C and normal exit. state.json uses atomic writes. Docs do not specify whether a tool call or approval request that was in flight at exit is resumed mid-turn or replayed from the start."
observability:
  stream_events:
    - "Wire TurnBegin/TurnEnd/StepBegin/StepRetry"
    - "Wire ApprovalRequest/QuestionRequest"
    - "ACP session lifecycle"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PostToolUse"
    - "PostToolUseFailure"
    - "Notification"
  failure_modes:
    - "SessionEnd reason"
    - "StopFailure hook"
  notes: "Hooks receive session_id and cwd. Wire replay reads wire.jsonl. The /sessions picker and /export command surface session metadata."
quirks:
  - "The official docs describe storage under ~/.kimi/, but the locally installed kimi-code 0.14.0 binary uses ~/.kimi-code/ with a different directory layout and state.json schema."
  - "kimi -r <id> creates a new session if the requested ID does not exist."
  - "Empty sessions do not print a resume hint on exit."
  - "The interactive session picker is shell-mode only."
  - "--print implicitly enables --afk, which auto-dismisses AskUserQuestion rather than pausing for human input."
  - "/clear and /reset clear conversation context but do not reset session state (approvals, subagents, additional directories)."
  - "Direct parsing of context.jsonl and wire.jsonl is not documented as a stable interface; use /export or the Wire/ACP protocols for interoperability."
gaps:
  - "Official docs do not explicitly confirm that --print and --session/--continue can be combined on one command line."
  - "Behavior when the same session is resumed concurrently from multiple terminals is not documented."
  - "No documented human-in-the-loop defer-and-resume mechanism for AskUserQuestion or approval prompts."
  - "Session retention/cleanup policy is not documented."
  - "Windows and Linux paths were not independently verified; docs state ~/.kimi and KIMI_SHARE_DIR."
changes:
  - "2026-07-02: Rewrote with schema-validated frontmatter based on current kimi-cli documentation and local inspection of both ~/.kimi and ~/.kimi-code installations."
requires_claudine_update: true
reason: "The previous kimi.md contained incorrect Claude-specific content. Accurate Kimi resume semantics, storage paths, and HITL limitations need to replace the stale provider metadata in Claudine."
---

## Overview

Kimi Code CLI's resume is a local-transcript-replay system. Each session writes conversation context, wire events, and runtime state to files under the share directory (default `~/.kimi/`). Resume loads those files, replays the conversation, and restores session-level state such as approval mode, plan mode, additional directories, and subagent instances. Resume can be triggered from the interactive shell, the non-interactive `--print` mode, the ACP server (`kimi acp`), or the Wire protocol (`--wire`).

## Resume Semantics

A Kimi "session" is a locally persisted conversation plus a `state.json` snapshot. Resume means reading the existing transcript and state and continuing to append turns to the same session. It is not a remote server-side continuation or a live-process attach. The same local files are authoritative for interactive, print-mode, ACP, and Wire clients.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `kimi --continue` | No | Latest session for cwd |
| Interactive CLI | `kimi --session` | No | Interactive picker |
| Interactive CLI | `kimi -r <session-id>` | No | Exact session ID |
| Interactive CLI | `/sessions` or `/resume` | No | Picker (Ctrl-A toggles all dirs) |
| Non-interactive CLI | `kimi --print --session <id> -p "…"` | Yes | Exact ID (likely valid composition) |
| Non-interactive CLI | `kimi --print --continue -p "…"` | Yes | Latest cwd session |
| Headless server | `kimi acp` | Yes | JSON-RPC session ID |
| Wire API | `kimi --wire` | Yes | Session passed at initialization |
| IDE | Zed/JetBrains via ACP, VS Code extension | Yes | Picker or ID |

The `--continue` and `--session` options are mutually exclusive. `--session` without an argument opens the picker; with an argument it resumes that exact session (or creates a new one if the ID is missing).

## Session ID Capture

Stable session identifiers are UUIDs. Capture surfaces include:

- **Resume hint on exit**: `To resume this session: kimi -r <session-id>` is printed for non-empty sessions after normal exit, `Ctrl-C`, `/undo`, `/fork`, and `/sessions` switch.
- **Session directory**: the `<session-id>` directory name under `~/.kimi/sessions/<work-dir-hash>/`.
- **Hooks**: `SessionStart`/`SessionEnd` receive `session_id` and `cwd` via stdin JSON.
- **Wire/ACP protocols**: lifecycle messages carry `session_id`.
- **`kimi export [sessionId]`**: accepts or defaults to the previous session for the cwd.

The interactive `/sessions` picker shows titles and timestamps, not raw IDs.

## Resume Invocation

Continue the latest session for the current directory:

```sh
kimi --continue
```

Resume a specific session by ID:

```sh
kimi -r session_abc123
```

Open the interactive picker:

```sh
kimi --session
```

Switch sessions inside an active shell session:

```text
/sessions
```

Non-interactive follow-up (composition of global flags; not explicitly documented):

```sh
kimi --print --session session_abc123 -p "next step"
```

## Session Lookup Scope

Session lookup is primarily working-directory scoped. `--continue` resumes the latest session for the current directory. `/sessions` defaults to sessions for the current directory and can be widened to all directories with `Ctrl-A`. Resuming by ID bypasses scope entirely. There is no documented git-branch or worktree filtering.

## State Storage

Resumable state is stored locally. The default share directory is `~/.kimi/` (override with `KIMI_SHARE_DIR`):

```
~/.kimi/
├── sessions/
│   └── <work-dir-hash>/
│       └── <session-id>/
│           ├── context.jsonl   # conversation transcript
│           ├── wire.jsonl      # wire event log
│           └── state.json      # approval, plan, subagents, additional dirs
```

- `context.jsonl`: full context in JSONL format, including system prompt, user messages, assistant responses, tool calls, and internal records.
- `wire.jsonl`: wire events for replay and title extraction.
- `state.json`: runtime state including title, approval decisions, plan mode, subagent instances, and additional directories.

The system prompt is frozen at session creation and reused on restore. `state.json` is written atomically to reduce corruption on crash.

**Local observation note**: the `kimi` binary installed on this host is `kimi-code` 0.14.0 and stores sessions under `~/.kimi-code/sessions/wd_<name>_<hash>/session_<uuid>/` with a different `state.json` schema. This appears to be the successor product described on the docs site as replacing Kimi Code CLI.

## Restored State

Resuming restores:

- Conversation transcript and tool results from `context.jsonl`.
- Approval state (`yolo`, `afk`, and per-session auto-approved operation types) from `state.json`.
- Plan mode status.
- Additional directories added via `--add-dir` or `/add-dir`.
- Subagent instance state and context history.

It does not restore the original process environment; the launching process environment is used. The model can be overridden at resume time with `--model`.

## Branching and Checkpoints

Branching is supported; checkpointing/rewinding is not.

- `/fork` copies the entire conversation history into a new session and switches to it, leaving the original unchanged.
- `/undo` selects a prior turn and forks a new session containing all history before that turn, also preserving the original.
- Web UI can fork from any assistant message.

There is no `/rewind` or checkpoint command that reverts within the same session.

## Human-in-the-Loop Resume

Kimi Code CLI does **not** provide a documented defer-and-resume flow for interrupted questions or approvals. `AskUserQuestion` is a tool call that expects an immediate answer. In `--print` mode, `--afk` is implied and auto-dismisses questions. Wire/ACP expose `QuestionRequest` and `ApprovalRequest` as blocking JSON-RPC requests that must be answered before the turn continues. Claudine cannot capture a pending question, exit, and later inject an answer through a documented resume path.

## Interruption Recovery

Sessions survive most interruptions because files are written locally:

- **Normal exit / Ctrl-C / /undo / /fork / /sessions switch**: a resume hint is printed.
- **Crash / working directory deletion**: the FAQ states a crash report with session ID and workDir is shown, and recovery is possible with `kimi -r <session-id>` from the correct directory.
- `state.json` uses atomic writes to reduce corruption.

The docs do not specify whether a tool call or approval request that was in flight at the moment of interruption is resumed mid-turn or replayed from the start.

## Observability

Relevant observability surfaces:

- **Hooks**: `SessionStart` and `SessionEnd` fire on resume with `session_id` and `cwd`; `StopFailure` fires on turn-end errors.
- **Wire protocol**: `event` and `request` messages expose session lifecycle, tool calls, approvals, and questions.
- **`kimi export`**: bundles the session directory into a ZIP archive.
- **`/sessions` and `/title`**: list and rename sessions.
- **`/debug`**: shows message/token counts and checkpoints.

## Quirks and Gaps

Quirks:

- The installed `kimi` binary may be the successor `kimi-code` product, which uses `~/.kimi-code/` and a different on-disk layout than the documented `~/.kimi/`.
- `kimi -r <id>` silently creates a new session if the ID does not exist.
- Empty sessions do not emit a resume hint.
- The interactive picker is only available in shell mode.
- `--print` implies `--afk`, so non-interactive runs auto-dismiss questions rather than pausing.
- `/clear` and `/reset` clear conversation context but leave session state intact.

Gaps:

- No explicit documentation confirming `--print --session` or `--print --continue` composition.
- No documented behavior for concurrent resumes of the same session.
- No documented defer-and-resume mechanism for HITL prompts.
- No documented retention or cleanup policy for session files.
- Windows and Linux paths were inferred from docs, not independently verified.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL recovery:

- Capture `session_id` from the resume hint printed on exit, from `SessionStart` hook JSON, or from the session directory name.
- Use `kimi --print --session <id> -p "<follow-up>"` for scriptable continuation if flag composition is confirmed in the target version.
- Use `kimi --continue` to resume the latest cwd session.
- Do not rely on parsing `context.jsonl` or `wire.jsonl` directly; use `/export`, the Wire protocol, or ACP for stable access.
- Do not assume HITL deferral is possible; plan to answer `QuestionRequest`/`ApprovalRequest` synchronously when wrapping Kimi via Wire/ACP.
- Be aware that the installed binary may be `kimi-code` (successor) with different storage paths than the documented `kimi-cli`.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter and rewrote body from current kimi-cli documentation plus local inspection.

## Sources

- [Kimi Code CLI — Sessions and Context](https://moonshotai.github.io/kimi-cli/en/guides/sessions.html)
- [Kimi Code CLI — Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Kimi Code CLI — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Kimi Code CLI — Slash Commands](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html)
- [Kimi Code CLI — Hooks](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [Kimi Code CLI — Print Mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Kimi Code CLI — Wire Mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Kimi Code CLI — Web UI](https://moonshotai.github.io/kimi-cli/en/reference/kimi-web.html)
- [Kimi Code CLI — FAQ](https://moonshotai.github.io/kimi-cli/en/faq.html)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Kimi Code CLI GitHub repository](https://github.com/MoonshotAI/kimi-cli)
