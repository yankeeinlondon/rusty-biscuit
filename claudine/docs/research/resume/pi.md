---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://pi.dev/docs/latest/sessions
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "pi -c / --continue"
      - "pi -r / --resume"
      - "pi --session <path|id>"
      - "pi --fork <path|id>"
      - "/resume"
      - "/session"
      - "/tree"
      - "/fork"
      - "/clone"
      - "/import <file>"
      - "/name <name>"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
      - other
    notes: "Interactive resume reopens the TUI. Follow-up text is typed after the session is active. `other` covers file paths and branch/tree entry selection."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "pi -p --continue [message]"
      - "pi -p --session <path|id> [message]"
      - "pi --mode json --continue [message]"
      - "pi --mode json --session <path|id> [message]"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - other
    notes: "Print and JSON modes can append a prompt to the selected session. `--session` accepts a path or UUID prefix, but a cross-project UUID match can try to prompt for forking and is unsafe for unattended automation."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "Pi has RPC subprocess mode, not a persistent standalone headless server with authoritative session state."
  - mode: ide
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No official IDE resume surface was found."
  - mode: api
    supported: true
    mechanisms:
      - "pi --mode rpc --session <path|id>"
      - "RPC prompt"
      - "RPC switch_session"
      - "RPC follow_up / steer"
      - "RPC fork / clone / new_session"
      - "SessionManager.open(path)"
      - "SessionManager.continueRecent(cwd)"
      - "SessionManager.forkFrom(sourcePath, targetCwd)"
      - "AgentSessionRuntime.switchSession()"
      - "AgentSessionRuntime.fork()"
      - "AgentSessionRuntime.importFromJsonl()"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - other
    notes: "API resume is local-process transcript replay. RPC can load or switch sessions and then accept prompts, steering, or follow-up messages."
session_id_capture:
  - surface: json_stream
    field: id
    format: uuid
    notes: "`pi --mode json` emits the session header as the first JSONL line, including `type`, `version`, `id`, `timestamp`, and `cwd`."
  - surface: session_file
    field: filename
    format: "<timestamp>_<uuid>.jsonl"
    notes: "The same UUID appears in the first `session` JSONL record. Local files inspected on macOS used version 3 headers."
  - surface: interactive_ui
    field: session ID
    format: uuid
    notes: "`/session` shows the current session file and ID. The `/resume` picker is human-oriented and can search, rename, delete, and filter sessions."
  - surface: other
    field: sessionId
    format: uuid
    notes: "SDK `AgentSession.sessionId`, RPC `get_state`, and RPC `get_session_stats` expose the active session identifier."
resume_invocations:
  - mode: interactive
    invocation: "pi -c"
    accepts_prompt: false
    selection: latest
    notes: "Continue the most recent session for the current working directory."
  - mode: interactive
    invocation: "pi -r"
    accepts_prompt: false
    selection: picker
    notes: "Open the session picker for the current project; the picker can search, show paths, sort, filter named sessions, rename, and delete."
  - mode: interactive
    invocation: "pi --session <path|id>"
    accepts_prompt: false
    selection: id
    notes: "Open a specific JSONL file or UUID-prefix match in the TUI."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Browse previous sessions from inside the TUI."
  - mode: interactive
    invocation: "/tree"
    accepts_prompt: false
    selection: picker
    notes: "Move the active leaf to a previous entry in the same session tree and continue from there."
  - mode: interactive
    invocation: "/import <file>"
    accepts_prompt: false
    selection: other
    notes: "Copy a JSONL session into the current session directory and switch to it."
  - mode: non_interactive
    invocation: "pi -p --continue \"follow-up prompt\""
    accepts_prompt: true
    selection: latest
    notes: "Append a prompt to the most recent current-directory session and print the final answer."
  - mode: non_interactive
    invocation: "pi -p --session <path|id> \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Append a prompt to an explicit session selected by path or UUID prefix."
  - mode: non_interactive
    invocation: "pi --mode json --session <path|id> \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Append a prompt and emit JSONL events. The first line is the session header."
  - mode: api
    invocation: "pi --mode rpc --session <path|id>"
    accepts_prompt: true
    selection: id
    notes: "Start an RPC subprocess with an existing session loaded, then send `prompt`, `steer`, or `follow_up` commands."
  - mode: api
    invocation: "{\"type\":\"switch_session\",\"sessionPath\":\"/path/to/session.jsonl\"}"
    accepts_prompt: false
    selection: other
    notes: "Switch the active session in a running RPC process; send `prompt` afterward to continue."
  - mode: api
    invocation: "SessionManager.open('/path/to/session.jsonl')"
    accepts_prompt: false
    selection: other
    notes: "Open a persisted transcript through the SDK."
state_storage:
  - location: local
    os: macos
    path: "~/.pi/agent/sessions/--<cwd-with-separators-replaced-by-dashes>--/<timestamp>_<uuid>.jsonl"
    format: "JSONL, current header version 3"
    retention: "Manual/until deleted. No automatic retention policy was documented; `/resume` can delete via the picker."
    notes: "Locally verified under `/Users/ken/.pi/agent/sessions/`. This Claudine process also had `$HOME=/Users/ken/.claudine`, producing an empty overlay at `/Users/ken/.claudine/.pi/agent/sessions`; wrappers must avoid confusing process HOME overlays with the user's real Pi home."
  - location: local
    os: linux
    path: "~/.pi/agent/sessions/--<cwd-with-separators-replaced-by-dashes>--/<timestamp>_<uuid>.jsonl"
    format: "JSONL, current header version 3"
    retention: "Manual/until deleted. No automatic retention policy was documented; `/resume` can delete via the picker."
    notes: "Inferred from official docs and installed source using Node `os.homedir()` plus `.pi/agent/sessions`; not locally verified on Linux."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.pi\\agent\\sessions\\--<cwd-with-slashes-backslashes-and-colons-replaced-by-dashes>--\\<timestamp>_<uuid>.jsonl"
    format: "JSONL, current header version 3"
    retention: "Manual/until deleted. No automatic retention policy was documented; `/resume` can delete via the picker."
    notes: "Inferred from installed source using Node `os.homedir()` and platform path joins; not locally verified on Windows. A cwd such as `C:\\Users\\me\\repo` is encoded by replacing `:`, `/`, and `\\` with `-`."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Default lookup is cwd-scoped. `SessionManager.list(cwd)` and `/resume` target the current project, while source inspection shows `--session <uuid-prefix>` falls back to `SessionManager.listAll()` across projects if no local UUID prefix matches. There is no git branch or worktree metadata in the session header."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "CLI: pi --fork <path|id>; TUI: /tree, /fork, /clone; RPC: fork, clone; SDK: SessionManager.forkFrom(), createBranchedSession(), AgentSessionRuntime.fork()"
  checkpoint_invocation: ""
  preserves_original: true
  notes: "`/tree` branches in place within the same JSONL file by moving the active leaf. `/fork`, `/clone`, and `--fork` create a new session file and preserve the source. Labels can mark entries but are not checkpoints or rewinds."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: restored
  env: current_process
  notes: "Resume rebuilds context from the active JSONL branch. Stored entries include messages, tool results, model changes, thinking-level changes, compactions, branch summaries, labels, session_info names, and extension custom/custom_message entries. The header stores the original cwd; Pi recreates runtime services for that cwd when possible. CLI `--model`, `--provider`, and `--thinking` can override future turns. Current process environment, credentials, settings, extensions, tools, project trust, and context/system prompt files are recalculated at launch. Pi has no core approval system, though extensions can implement their own gates."
hitl_resume:
  supported: false
  question_capture: "RPC can emit live `extension_ui_request` records for extension dialogs, editor prompts, notifications, widgets, and related UI methods."
  answer_injection: "RPC clients answer live extension UI requests with `extension_ui_response` records on stdin; normal follow-up messages can also be queued with `follow_up` or `steer` while the process is alive."
  limitations: "No evidence that a pending extension UI request is persisted into the JSONL session file or can be answered after process exit. Interactive TUI prompts and permission-like extension gates are live-process state, not resumable transcript state."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: false
  limitations: "Completed entries already flushed to JSONL can be resumed after terminal close, Ctrl+C, crash, process kill, network loss, or provider error. Local transcripts show aborted/error assistant messages persisted with `stopReason` and `errorMessage`. In-progress tool calls, live extension UI requests, and queued steering/follow-up messages are process memory and were not observed as durable state. Concurrent writers to the same session file are not documented as safe."
observability:
  stream_events:
    - "JSON mode first line: session header with id, version, timestamp, cwd"
    - "agent_start / agent_end"
    - "turn_start / turn_end"
    - "message_start / message_update / message_end"
    - "tool_execution_start / tool_execution_update / tool_execution_end"
    - "queue_update"
    - "compaction_start / compaction_end"
    - "auto_retry_start / auto_retry_end"
    - "extension_error"
  hook_events:
    - "extension session_start"
    - "extension session_before_switch"
    - "extension session_before_fork"
    - "extension session_shutdown"
  failure_modes:
    - "assistant message stopReason=error with errorMessage in JSONL"
    - "assistant message stopReason=aborted with errorMessage in JSONL"
    - "RPC error responses for rejected commands or invalid session operations"
    - "auto_retry_start / auto_retry_end stream events for retryable failures"
  notes: "The JSONL session file is the durable observability source. RPC `get_state`, `get_session_stats`, `get_entries`, `get_tree`, `get_messages`, and `get_last_assistant_text` expose active session state while the process is alive."
quirks:
  - "`--session <uuid-prefix>` first searches the current project, then all projects. If a global match belongs to another cwd, installed 0.73.1 source prompts to fork, which can hang unattended runs."
  - "`--session <path>` preserves explicit paths and may create a new session at that path if the file does not exist."
  - "Session names are stored as `session_info` entries and improve picker display, but current CLI docs do not define direct resume-by-name."
  - "`/resume` is picker-only and should not be treated as scriptable selection."
  - "`/tree` creates alternate branches inside the same file; only `/fork`, `/clone`, or `--fork` creates a separate transcript."
  - "`--no-session` and `SessionManager.inMemory()` are intentionally not resumable."
  - "Direct JSONL parsing is documented for parsers, but wrappers still need to tolerate migrations and extension-defined entries."
  - "Runtime resources are recalculated on resume, so changing settings, system prompt files, extensions, tools, or project trust can change future behavior of an old session."
gaps:
  - "Exact behavior for simultaneous writes by two Pi processes to the same `.jsonl` file is undocumented."
  - "Whether any extension-provided permission gate can persist enough state for post-exit answer injection is undocumented and was not observed locally."
  - "Windows and Linux storage paths are source/doc inferred, not locally verified."
  - "The official docs document session names for display/search but not deterministic resume-by-name creation or selection."
  - "The exact durability boundary for a process killed mid-tool-call is not documented; local files only prove completed/aborted message records persist."
changes:
  - "Verified against Pi 0.73.1 installed locally and current pi.dev docs on 2026-07-03."
  - "Replaced single `os: all` storage with schema-compliant macOS, Linux, and Windows records."
  - "Added local evidence from `/Users/ken/.pi/agent/sessions`, including v3 JSONL headers, model/thinking entries, tool result entries, and persisted aborted/error assistant messages."
  - "Added the `$HOME=/Users/ken/.claudine` overlay caveat for Claudine wrappers inspecting Pi state."
  - "Added picker rename/delete/name filtering, session_info names, RPC live extension UI request limitations, and cross-project `--session` fork-prompt automation risk."
requires_claudine_update: true
reason: "Pi remains researched but not code-supported in Claudine. The verified local transcript model, session-id capture, per-OS storage paths, HOME-overlay caveat, and cross-project `--session` prompt risk need provider metadata and wrapper behavior before lifecycle `resume` can safely target Pi."
---

# Pi Session Resume Research

## Overview

Pi has first-class resume support backed by local JSONL transcript replay. A session is not a remote server allocation or a live model state; it is a versioned append-only file under the Pi agent sessions directory. Resume loads that file, rebuilds the active conversation branch, recreates runtime services for the session cwd, and appends future entries.

For Claudine, Pi is strong enough to support lifecycle `resume` once Pi becomes a code-supported provider, but automation must avoid treating all surfaces as equivalent. `pi -c` is current-directory latest-session resume, `pi --session <path|id>` targets a specific persisted transcript, `/resume` is a human picker, and RPC/SDK resume is local subprocess or in-process state. The main wrapper risks are HOME-overlay confusion, cross-project UUID matches that can prompt to fork, and the fact that live extension UI requests or queued messages are not persisted for later human-in-the-loop continuation.

## Resume Semantics

A Pi session is a local JSONL transcript. The first line is a `session` header with `type`, `version`, `id`, `timestamp`, and `cwd`; inspected local files use `version: 3`. Subsequent entries form a tree through `id` and `parentId`. The active leaf determines which branch is replayed into model context. Official session-format docs describe entries for messages, model changes, thinking-level changes, compactions, branch summaries, custom extension entries, custom extension messages, labels, and session metadata.

The applicable resume patterns are:

| Pattern | Applies | Continuity model |
|---|---:|---|
| Continue latest | Yes | `pi -c` / `--continue` loads the most recent valid JSONL session for the current cwd. |
| Resume by handle | Yes | `pi --session <path|id>` opens an explicit file path or UUID prefix. |
| Interactive picker | Yes | `pi -r` and `/resume` open a current-project picker. |
| Non-interactive follow-up | Yes | Print and JSON modes can append a prompt to `--continue` or `--session`. |
| Transcript replay | Yes | The session file is the durable state. |
| Server-side session | No | No separate remote authoritative session store was found. |
| Live-process attach | Partial | RPC clients talk to a running Pi subprocess, but that is process control rather than reattaching to a saved live process after exit. |
| Branch/fork/checkpoint | Partial | Branching and forking exist; named checkpoints/rewind do not. |
| Recovery resume | Partial | Completed persisted entries resume; pending live tool/UI state does not. |
| Human-in-the-loop resume | No | Live RPC UI requests can be answered while the process lives, but no durable post-exit pending question mechanism was found. |

Chat history export is not a resume mechanism unless Pi can load it as a session JSONL file. Context files such as `AGENTS.md`, `CLAUDE.md`, system prompt files, skills, settings, and extension files affect future launches, but they are context sources rather than prior-session continuation state.

## Supported Modes

| Mode | Entry point | Follow-up prompt at invocation | Selector |
|---|---|---:|---|
| Interactive | `pi -c` | No | Latest session for cwd |
| Interactive | `pi -r` | No | Picker |
| Interactive | `pi --session <path|id>` | No | File path or UUID prefix |
| Interactive | `/resume` | No | Picker |
| Interactive | `/tree` | No | Entry/tree selector inside active session |
| Interactive | `/fork`, `/clone`, `/import <file>` | No | Entry or file path |
| Non-interactive print | `pi -p --continue "prompt"` | Yes | Latest session for cwd |
| Non-interactive print | `pi -p --session <path|id> "prompt"` | Yes | File path or UUID prefix |
| Non-interactive JSON | `pi --mode json --session <path|id> "prompt"` | Yes | File path or UUID prefix |
| RPC | `pi --mode rpc --session <path|id>` then `prompt` | Yes | File path or UUID prefix |
| SDK | `SessionManager.open(path)` / `continueRecent(cwd)` | Yes, through `session.prompt()` | File path or latest cwd session |

Interactive resume and non-interactive follow-up are both supported, but the picker must be treated as human-only. Session names are first-class display metadata through `/name`, `--name`, `session_info` entries, and picker filters; current docs do not specify direct command-line resume by name. Non-interactive sessions are normal persisted sessions unless `--no-session` or `SessionManager.inMemory()` is used, so they can be resumed later.

## Session ID Capture

The stable session handle is a UUID. It appears in the JSONL header and in the filename:

```json
{"type":"session","version":3,"id":"019f26a9-8c2e-77e9-b9fa-035d8e903788","timestamp":"2026-07-03T06:27:53.518Z","cwd":"/Users/ken/.claudine/worktrees/rusty-biscuit/claudine"}
```

Observed local filenames use `<timestamp>_<uuid>.jsonl`, for example:

```text
/Users/ken/.pi/agent/sessions/--Users-ken-.claudine-worktrees-rusty-biscuit-claudine--/2026-07-03T06-27-53-518Z_019f26a9-8c2e-77e9-b9fa-035d8e903788.jsonl
```

`pi --mode json` emits the session header as its first JSONL line. Interactive `/session` shows the active session file, ID, message count, tokens, and cost. SDK and RPC expose the ID through `AgentSession.sessionId`, `get_state.sessionId`, and session stats.

The handle is available as soon as the session file/header is created. For durable automation, capture either the absolute session file path or the header UUID plus cwd. A bare UUID prefix is less safe because source inspection shows Pi searches the current project first, then all projects.

## Resume Invocation

Continue latest interactively:

```bash
pi -c
```

Browse current-project sessions interactively:

```bash
pi -r
```

Resume an explicit session in the TUI:

```bash
pi --session /Users/ken/.pi/agent/sessions/--Users-ken--/2026-05-02T11-24-41-165Z_019de86e-fd4c-720b-a89a-73893f5786b4.jsonl
pi --session 019de86e
```

Send a non-interactive follow-up and capture text output:

```bash
pi -p --session /path/to/session.jsonl "Continue from the prior findings and summarize the next step."
```

Send a non-interactive follow-up and capture structured events:

```bash
pi --mode json --session /path/to/session.jsonl "Continue from the prior findings."
```

Start RPC on an existing session and send a prompt:

```bash
pi --mode rpc --session /path/to/session.jsonl
```

```json
{"id":"req-1","type":"prompt","message":"Continue from the prior findings."}
```

Switch an RPC process to another session:

```json
{"id":"req-2","type":"switch_session","sessionPath":"/path/to/session.jsonl"}
```

The follow-up answer can be captured as plain text in print mode, as JSONL stream events in JSON mode, or as RPC events followed by `agent_end` / `get_last_assistant_text`.

## Session Lookup Scope

Default session storage is cwd-scoped. Pi encodes the working directory into the directory name below `~/.pi/agent/sessions/`, replacing path separators with dashes. Official docs describe `/resume` and `pi -r` as current-project selection.

Installed 0.73.1 source adds an important detail for `--session <id>`: it resolves UUID prefixes by checking `SessionManager.list(cwd, sessionDir)` first and `SessionManager.listAll()` second. If a global match is found in another project, Pi reports the source cwd and asks whether to fork into the current directory. That prompt is inappropriate for unattended Claudine runs.

Pi does not store git branch or worktree identity in the session header. A git worktree naturally receives a different encoded cwd if its filesystem path differs, but Pi does not appear to apply git-aware worktree grouping or branch filtering.

## State Storage

On this macOS host, real Pi sessions were found in `/Users/ken/.pi/agent/sessions/`, while the current Claudine process had `$HOME=/Users/ken/.claudine` and therefore an empty overlay at `/Users/ken/.claudine/.pi/agent/sessions/`. This matters for wrappers: inspect the intended user home or Pi agent directory, not blindly the wrapper's temporary HOME.

The installed source defines the default agent directory as `os.homedir()/.pi/agent`, with override environment variable `PI_CODING_AGENT_DIR`. The session directory defaults to `<agentDir>/sessions`, with overrides from `--session-dir`, `PI_CODING_AGENT_SESSION_DIR`, or settings `sessionDir`.

The JSONL format is documented and versioned. Existing v1/v2 sessions are migrated to the current version on load. Direct parsing is possible and documented, but an integration should still tolerate new entry types and extension-defined `custom` / `custom_message` records.

## Restored State

Resume restores the transcript branch, tool result messages, model changes, thinking-level changes, compaction summaries, branch summaries, custom messages, labels, and session metadata from JSONL. Local inspected transcripts include assistant records with `stopReason: "stop"`, `stopReason: "aborted"`, and `stopReason: "error"`, including `errorMessage` fields.

The original cwd is stored in the session header and used when Pi rebuilds runtime services. If the cwd is missing, installed source has explicit missing-cwd handling: interactive mode can ask the user whether to continue from a fallback cwd, while non-interactive mode exits with an error.

Model and thinking level are restored from the active branch unless launch flags override them. Future environment variables, API keys, project trust, settings, context files, system prompt files, extensions, tools, and resource discovery come from the current launch. Pi does not persist a core approval state because it has no built-in sandbox or approval system; extension-specific gates are outside the core resume contract.

Resume appends to the same transcript when opening an existing session. Forking and cloning create new transcript files.

## Branching and Checkpoints

Pi supports in-file branching and separate-file forks:

| Feature | Output | Notes |
|---|---|---|
| `/tree` | Same session file | Moves the active leaf to an earlier entry. Selecting a user/custom message lets the user edit and resubmit, creating a new branch. |
| `/fork` | New session file | Creates a new session from a previous user message. |
| `/clone` | New session file | Duplicates the current active branch. |
| `pi --fork <path|id>` | New session file | Forks a session selected by file path or UUID prefix. |
| Labels | Same session file | Marks entries, useful as bookmarks but not a true checkpoint restore feature. |

The `/resume` picker can search, show paths, sort, filter named sessions, rename, and delete. Deletion can use the `trash` CLI when available. Sessions can be exported to HTML or JSONL and shared as a private GitHub gist through `/share`, but sharing/exporting is not itself resume unless the JSONL is imported or opened.

No named rewind/checkpoint primitive was found. The closest resume-safe mechanism is tree navigation plus labels, or fork/clone for copy-preserving continuation.

## Human-in-the-Loop Resume

Pi supports live human interaction in interactive mode and, for extensions, through RPC `extension_ui_request` / `extension_ui_response`. RPC can also queue `steer` and `follow_up` messages while the process is alive.

No durable human-in-the-loop resume mechanism was verified. A pending extension dialog, approval-like tool gate, editor request, or notification is live process state. It was not observed in local session files, and the docs describe the response path as RPC stdin while the request is pending. Claudine should not assume it can stop Pi at a question, ask the user elsewhere, restart Pi later, and inject the answer into the same pending operation. It can only implement that pattern by keeping the RPC process alive or by translating the user's answer into a new prompt/follow-up after normal resume.

## Interruption Recovery

Because session files are append-only JSONL, completed persisted entries survive terminal close, Ctrl+C, process kill after flush, provider errors, and later launches. Local files show aborted and errored assistant messages persisted with `stopReason` and `errorMessage`, so a subsequent resume can continue after visible failure records.

The durable boundary is not identical to live recovery. In-progress tool calls, pending extension UI requests, process-memory queues, and partial output not yet appended are not guaranteed to survive. The installed session manager only writes the initial entries after an assistant message exists, then appends future entries; killing a process before that point can leave no session file for an otherwise started run.

Concurrent resumes of the same file are not documented as safe. Since persistence uses normal append/rewrite operations without a documented lock, Claudine should serialize access to a Pi transcript it owns.

## Observability

Useful resume observability surfaces:

| Surface | Resume-relevant data |
|---|---|
| JSON mode | First JSONL line is the session header; later events show lifecycle, messages, tools, queues, compaction, retry, and extension errors. |
| Session file | Durable header, entries, parent links, model/thinking changes, tool results, errors, labels, names, compactions, custom entries. |
| TUI `/session` | Current session file, ID, message count, tokens, and cost. |
| RPC `get_state` | `sessionFile`, `sessionId`, `sessionName`, model, thinking level, streaming state, queue counts. |
| RPC `get_entries` | Append-ordered entries plus `leafId`; supports a `since` cursor. |
| RPC `get_tree` | Tree nodes plus current `leafId`. |
| RPC `get_last_assistant_text` | Latest assistant text for final answer capture. |
| Extension events | `session_start`, `session_before_switch`, `session_before_fork`, and `session_shutdown` can observe or cancel session replacement. |

## Quirks and Gaps

Quirks:

- Cross-project `--session <uuid-prefix>` can become interactive by asking whether to fork into the current cwd.
- The current docs support names for display/search, but not direct command-line resume-by-name.
- `/tree` changes the active branch in the same file; it is not a separate fork unless `/fork` or `/clone` is used.
- `--no-session` and SDK `SessionManager.inMemory()` produce no resumable state.
- Pi recalculates launch resources, so an old session can behave differently after settings, extensions, tools, context files, trust, or system prompt files change.
- In this Claudine environment, `$HOME` points to `/Users/ken/.claudine`; real historical Pi sessions were under `/Users/ken/.pi`, so HOME overlays can hide the sessions Claudine wants.

Gaps:

- Simultaneous writes to the same JSONL transcript are undocumented.
- Windows and Linux storage paths were inferred from docs/source, not locally verified.
- Persistent post-exit extension UI answer injection was not found.
- The precise file state after killing Pi during an active tool call is not documented.
- Deterministic resume-by-session-name remains unsupported by the official docs as of this research.

## Claudine Integration Notes

For lifecycle `resume`, Claudine should model Pi as transcript replay with a durable local file handle. The safest handle is an absolute JSONL path captured from JSON mode, RPC state, `/session`, or filesystem discovery. UUID-prefix resume is useful for humans but weaker for automation because it can collide or match across projects.

`retry` can use ordinary new invocations when the desired behavior is rerunning a prompt, but `resume` should use `--session <absolute-path>` or an RPC process already switched to that file. Claudine should include `--session-dir` or `PI_CODING_AGENT_SESSION_DIR` only when it deliberately owns a separate storage root.

`proxy` and future human-in-the-loop recovery should prefer RPC when a live Pi process must remain answerable. If the process exits while waiting on an extension UI request, Claudine should treat the pending interaction as lost and resume by adding a new prompt rather than pretending to answer the original request.

For non-interactive wrappers, avoid `pi -r`, `/resume`, and cross-project UUID-prefix selection. Also avoid changing `$HOME` unless Claudine intentionally wants isolated Pi config and sessions; otherwise Pi will read and write a different `~/.pi/agent` tree.

## Changelog

- 2026-07-03: Refreshed against Pi 0.73.1 installed locally, current pi.dev docs, and real session files under `/Users/ken/.pi/agent/sessions`. Updated frontmatter to use `agent: codex`, `model: default`, and schema-compliant per-OS storage records.
- 2026-07-03: Added local JSONL evidence for v3 headers, model/thinking entries, tool results, aborted/error assistant records, and the Claudine `$HOME` overlay caveat.
- 2026-07-03: Added current session picker behavior, session names via `session_info`, RPC/SDK replacement APIs, live extension UI request limitations, and the cross-project `--session` fork prompt risk.

## Sources

- [Pi Sessions documentation](https://pi.dev/docs/latest/sessions)
- [Pi Session File Format documentation](https://pi.dev/docs/latest/session-format)
- [Pi Using Pi documentation](https://pi.dev/docs/latest/usage)
- [Pi JSON Event Stream Mode documentation](https://pi.dev/docs/latest/json)
- [Pi RPC Mode documentation](https://pi.dev/docs/latest/rpc)
- [Pi SDK documentation](https://pi.dev/docs/latest/sdk)
- [Pi Extensions documentation](https://pi.dev/docs/latest/extensions)
- Local installed Pi package source: `/Users/ken/.bun/install/global/node_modules/@mariozechner/pi-coding-agent/dist/main.js`
- Local installed Pi session manager source: `/Users/ken/.bun/install/global/node_modules/@mariozechner/pi-coding-agent/dist/core/session-manager.js`
- Local installed Pi runtime source: `/Users/ken/.bun/install/global/node_modules/@mariozechner/pi-coding-agent/dist/core/agent-session-runtime.js`
- Local inspected session directory: `/Users/ken/.pi/agent/sessions/`
