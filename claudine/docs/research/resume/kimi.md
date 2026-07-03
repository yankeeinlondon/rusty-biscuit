---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://www.kimi.com/code/docs/en/kimi-code-cli/guides/sessions.html
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "kimi --continue"
      - "kimi --session"
      - "kimi --session <id>"
      - "kimi --resume <id>"
      - "kimi -S <id>"
      - "/sessions slash command"
      - "/resume slash command"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
    notes: "Interactive resume reopens the selected local session in the TUI; follow-up prompts are typed after the session is loaded. --continue and --session are mutually exclusive."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "kimi --prompt <prompt> --session <id>"
      - "kimi --prompt <prompt> --continue"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "The current command reference documents --prompt as non-interactive mode and does not list --session/--continue as conflicts with it. In -p mode regular tool calls run under auto permission policy and human approval is not requested."
  - mode: headless_server
    supported: true
    mechanisms:
      - "kimi acp"
      - "kimi server run"
      - "kimi web"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
    notes: "ACP supports session/list, session/load, session/resume, session/prompt, and session/cancel. The local server/web UI exposes session management through a loopback daemon, but REST/WebSocket resume endpoint details were not inspected."
  - mode: ide
    supported: true
    mechanisms:
      - "ACP-compatible editors"
      - "Kimi Code for VS Code"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
    notes: "IDE clients use ACP or the dedicated VS Code integration and drive Kimi sessions through the editor UI."
  - mode: api
    supported: true
    mechanisms:
      - "ACP JSON-RPC"
      - "local server REST/WebSocket"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
    notes: "ACP resume is documented. The server publishes OpenAPI/AsyncAPI from a local loopback service, but endpoint details were left to API-specific research."
session_id_capture:
  - surface: stdout
    field: "resume command hint"
    format: "kimi -r <session-id>"
    notes: "Legacy docs say non-empty sessions print a resume hint after normal exit, Ctrl-C, /undo, /fork, session switch, and related exits. Current docs focus on explicit commands and do not emphasize this hint."
  - surface: session_file
    field: "session directory name"
    format: "session_<uuid> for current Kimi Code; uuid for legacy kimi-cli"
    notes: "Current Kimi Code stores sessions under ~/.kimi-code/sessions/<workDirKey>/<sessionId>/ and indexes them in session_index.jsonl. Local inspection found session IDs such as session_292400e4-dc10-4762-8542-7585ee220961."
  - surface: session_file
    field: "session_index.jsonl"
    format: "JSONL records with sessionId, sessionDir, workDir"
    notes: "Local ~/.kimi-code/session_index.jsonl contained 14 records with exactly these top-level keys."
  - surface: cli_command
    field: "sessionId"
    format: "session_<uuid> or provider-defined id"
    notes: "kimi export [sessionId] exports a specified session; without an ID it selects the most recent session for the current directory."
  - surface: interactive_ui
    field: "session title and metadata"
    format: "picker rows"
    notes: "The /sessions picker is human-oriented. Current docs do not state that raw IDs are visible in the picker."
  - surface: hook
    field: "session_id"
    format: "string"
    notes: "Hook stdin JSON includes hook_event_name, session_id, and cwd."
  - surface: json_stream
    field: "session id"
    format: "ACP JSON-RPC session identifiers"
    notes: "ACP exposes session/list, session/load, and session/resume. The exact response field names are ACP-specific and were not locally exercised."
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
    notes: "Opens an interactive session selector."
  - mode: interactive
    invocation: "kimi --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Resumes the specified session. Hidden aliases include --resume and -r; help also shows -S."
  - mode: interactive
    invocation: "/sessions"
    accepts_prompt: false
    selection: picker
    notes: "Slash-command alias /resume browses and resumes a previous session while the agent is idle."
  - mode: non_interactive
    invocation: "kimi -p \"<prompt>\" --session <session-id>"
    accepts_prompt: true
    selection: id
    notes: "Supported by current flag conflict rules by implication; not verified with a live model call in this research run."
  - mode: non_interactive
    invocation: "kimi -p \"<prompt>\" --continue"
    accepts_prompt: true
    selection: latest
    notes: "Supported by current flag conflict rules by implication; tool progress and resuming notices go to stderr, answer output to stdout."
  - mode: non_interactive
    invocation: "kimi -p \"<prompt>\" --session <session-id> --output-format stream-json"
    accepts_prompt: true
    selection: id
    notes: "Structured stdout is JSONL Assistant/Tool messages; thinking and progress remain off stdout."
  - mode: api
    invocation: "kimi acp: session/load"
    accepts_prompt: false
    selection: id
    notes: "Loads a session and replays history via session/update."
  - mode: api
    invocation: "kimi acp: session/resume"
    accepts_prompt: false
    selection: id
    notes: "Lightweight resume sibling that skips history replay."
  - mode: api
    invocation: "kimi acp: session/prompt"
    accepts_prompt: true
    selection: id
    notes: "Sends a follow-up prompt after session/new, session/load, or session/resume."
state_storage:
  - location: local
    os: macos
    path: "/Users/<name>/.kimi-code/sessions/<workDirKey>/<sessionId>/"
    format: "session_index.jsonl plus per-session state.json, agents/main/wire.jsonl, agents/<subagentId>/wire.jsonl, optional logs, tasks, cron, and plan files"
    retention: "manual deletion; no automatic retention policy documented"
    notes: "Set KIMI_CODE_HOME to relocate the data root. Local inspection on macOS confirmed ~/.kimi-code/session_index.jsonl and session directories with state.json, agents/main/wire.jsonl, optional agents/agent-N/wire.jsonl, and logs/kimi-code.log."
  - location: local
    os: linux
    path: "/home/<name>/.kimi-code/sessions/<workDirKey>/<sessionId>/"
    format: "session_index.jsonl plus per-session state.json and agents/*/wire.jsonl"
    retention: "manual deletion; no automatic retention policy documented"
    notes: "Path is documented, not independently verified in this macOS research run. Set KIMI_CODE_HOME to relocate the data root."
  - location: local
    os: windows
    path: "C:\\Users\\<name>\\.kimi-code\\sessions\\<workDirKey>\\<sessionId>\\"
    format: "session_index.jsonl plus per-session state.json and agents/*/wire.jsonl"
    retention: "manual deletion; no automatic retention policy documented"
    notes: "Path is documented, not independently verified in this macOS research run. Set KIMI_CODE_HOME to relocate the data root."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Sessions are grouped by working directory key, derived from the working directory path as wd_<slug>_<first-12-chars-of-sha256>. --continue selects the most recent session for the current directory. Legacy docs describe Ctrl-A in the picker to show all directories; current docs only say /sessions browses previous sessions."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "/fork"
  checkpoint_invocation: ""
  preserves_original: true
  notes: "/fork creates an independent session that does not affect the original. Current docs say saved /goal state is not copied to the fork. /compact compresses context but is not a checkpoint/rewind feature."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: restored
  env: current_process
  notes: "Current Kimi Code replays agents/*/wire.jsonl and restores state.json metadata. Current docs say --auto, --yolo, or --plan can override saved permission/plan mode on resume; --model can override the model for the launch. Environment variables are inherited from the current process. Local legacy ~/.kimi context.jsonl includes roles such as _system_prompt, user, assistant, tool, _usage, and _checkpoint."
hitl_resume:
  supported: false
  question_capture: "ACP can surface tool approval and prompt flow while the ACP process is alive; hooks can observe lifecycle events, but no documented persisted pending-question record was found."
  answer_injection: "ACP clients answer approvals/questions synchronously through the active protocol session. No documented command injects an answer later into a stopped CLI session."
  limitations: "In -p mode Kimi Code uses auto permission policy and does not request human approval. There is no documented defer-exit-and-resume-with-answer flow for user questions or approval prompts."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: false
  limitations: "The persisted wire event stream supports recovery/replay after process exit, terminal close, crash, or Ctrl-C. Current docs do not say that an in-flight tool call or approval resumes mid-call; treat recovery as session continuation from persisted transcript state, not live tool continuation. Concurrent resume behavior is undocumented."
observability:
  stream_events:
    - "agents/*/wire.jsonl event records"
    - "ACP session/list"
    - "ACP session/load"
    - "ACP session/resume"
    - "ACP session/prompt"
    - "non-interactive stream-json Assistant and Tool messages"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PostToolUse"
    - "Notification"
  failure_modes:
    - "diagnostic logs under ~/.kimi-code/logs/kimi-code.log"
    - "optional per-session logs/kimi-code.log"
    - "kimi server status"
    - "kimi export debug ZIP"
  notes: "Hook stdin includes hook_event_name, session_id, and cwd. Current local wire.jsonl records include metadata, system prompt, skills, model, mode, and user input event types; direct parsing is useful for research but not documented as a stable integration API."
quirks:
  - "Kimi CLI has migrated toward Kimi Code CLI: current docs and local 0.14.0 binary use ~/.kimi-code and KIMI_CODE_HOME, while legacy kimi-cli data remains under ~/.kimi with a .migrated-to-kimi-code marker."
  - "Current non-interactive mode is -p/--prompt; prior research and legacy docs used --print. The installed 0.14.0 help does not show --print."
  - "The installed 0.14.0 help output shows --session, --continue, --prompt, and --output-format, but the help stream appeared truncated after --skills-dir in this non-interactive shell; the online command reference was used for full flag coverage."
  - "Current session directories do not contain context.jsonl; the replay source is agents/*/wire.jsonl. Legacy ~/.kimi sessions do contain context.jsonl, wire.jsonl, and state.json."
  - "Directly editing files in the sessions directory is explicitly warned against because it may prevent restore."
  - "--continue and --session are mutually exclusive."
  - "In -p mode, regular tool calls run under auto permission policy and no human approval is requested."
  - "Kimi Code can override saved permission or plan mode on resume with --auto, --yolo, or --plan."
gaps:
  - "No live model call was run to confirm --prompt combined with --session or --continue, because that could consume credentials/network and execute tools. The command reference implies the combination by listing only other flag conflicts."
  - "Concurrent resumes of the same session from multiple terminals or clients are not documented."
  - "No documented persisted pending-approval or pending-question answer injection mechanism was found."
  - "Automatic retention or cleanup policy for session files is not documented."
  - "Linux and Windows storage paths were taken from official docs and not independently verified on those OSes."
  - "The local server REST/WebSocket resume endpoints were not inspected; ACP resume is documented and sufficient for this topic."
changes:
  - "2026-07-03: Updated research from legacy Kimi CLI semantics to current Kimi Code CLI docs and local 0.14.0 binary behavior."
  - "2026-07-03: Replaced legacy ~/.kimi-only storage model with current KIMI_CODE_HOME / ~/.kimi-code storage and documented legacy ~/.kimi migration residue separately."
  - "2026-07-03: Replaced --print resume examples with current -p/--prompt examples and noted that --print is absent from installed 0.14.0 help."
  - "2026-07-03: Added local inspection findings for session_index.jsonl, state.json keys, agents/*/wire.jsonl, and legacy context.jsonl roles."
  - "2026-07-03: Added ACP session/load versus session/resume distinction and current server/web UI surfaces."
requires_claudine_update: true
reason: "Claudine's Kimi resume metadata and wrapper assumptions should prefer current Kimi Code CLI behavior: -p/--prompt for non-interactive follow-up, ~/.kimi-code/KIMI_CODE_HOME for session discovery, agents/*/wire.jsonl replay state, and ACP session/load/session/resume semantics. Legacy ~/.kimi parsing alone is no longer sufficient."
---

# Kimi Code CLI Resume Research

## Overview

Kimi Code CLI has first-class resume support backed by local transcript replay. Current Kimi Code sessions are stored under `KIMI_CODE_HOME` (default `~/.kimi-code`) as per-session metadata plus an agent event stream; resume loads the selected session from disk and continues from that saved state. The main wrapper risk is product migration: legacy Kimi CLI data under `~/.kimi` still exists and uses `context.jsonl`, while current Kimi Code uses `~/.kimi-code`, `session_index.jsonl`, and `agents/*/wire.jsonl`.

For Claudine, Kimi resume is scriptable enough for lifecycle `resume`: the CLI supports continue-latest, explicit session ID, picker-based interactive resume, non-interactive follow-up with `-p/--prompt`, ACP session load/resume, and a local web/server mode. It is not a live-process attach or remote server-side continuation. Pending approvals and user questions should be treated as synchronous protocol interactions, not as durable human-in-the-loop checkpoints that can be answered after the process exits.

## Resume Semantics

A current Kimi Code "session" is a locally persisted record under `$KIMI_CODE_HOME/sessions/<workDirKey>/<sessionId>/`. Official docs describe `state.json` as metadata and `agents/*/wire.jsonl` as the agent event stream used for recovery and replay. Local inspection on this host confirmed that current Kimi Code session directories contain `state.json`, `agents/main/wire.jsonl`, optional subagent `agents/agent-N/wire.jsonl`, and optional `logs/kimi-code.log`.

The continuity model is transcript replay. Resume is not a remote server-side session, and it is not attach to a still-running model state. ACP and the local server can host active clients, but their resume behavior still loads local session records. Memory files, `AGENTS.md`, skills, plugins, MCP declarations, and config are context sources or launch configuration, not prior-session continuation mechanisms.

The applicable resume patterns are:

| Pattern | Supported | Continuity model |
|---------|-----------|------------------|
| Continue latest | Yes | Local transcript replay for current working directory |
| Resume by handle | Yes | Session ID |
| Interactive picker | Yes | TUI picker and slash command |
| Non-interactive follow-up | Yes | `-p/--prompt` plus `--session` or `--continue` |
| Transcript replay | Yes | `agents/*/wire.jsonl` in current Kimi Code; `context.jsonl` in legacy Kimi CLI |
| Server-side session | No | Local server reads local session storage |
| Live-process attach | No | Not documented as a resume mechanism |
| Branch/fork | Yes | `/fork` creates an independent session |
| Recovery resume | Partial | Session can be resumed after interruption, but in-flight calls are not documented as resumable |
| Human-in-the-loop resume | No | No documented persisted question/answer injection flow |

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `kimi --continue` | No | Latest session for cwd |
| Interactive CLI | `kimi --session` | No | Interactive picker |
| Interactive CLI | `kimi --session <id>` | No | Exact session ID |
| Interactive CLI | `/sessions` or `/resume` | No | Picker while agent is idle |
| Non-interactive CLI | `kimi -p "..." --session <id>` | Yes | Exact ID |
| Non-interactive CLI | `kimi -p "..." --continue` | Yes | Latest cwd session |
| ACP | `kimi acp` | Yes, through `session/prompt` | ID or list |
| Local server / Web UI | `kimi server run`, `kimi web` | Yes, through browser/API client | UI/API selection |
| IDE | ACP-compatible editors, Kimi Code for VS Code | Yes | IDE UI or session ID |

`--continue` and `--session` are mutually exclusive because both select an existing session. `--session` without an ID opens a picker; with an ID it opens that exact session. The current command reference documents `--prompt` as non-interactive mode and lists its conflicts as `--yolo`, `--auto`, and `--plan`, not `--session` or `--continue`; this research therefore records prompt-time resume as supported by documented flag composition, but the combination was not exercised with a live model call.

Sessions created in non-interactive mode appear to use the same local session machinery as interactive sessions. The current docs do not carve out a separate non-resumable run type, and local `~/.kimi-code/session_index.jsonl` records sessions independently of UI mode.

## Session ID Capture

Current Kimi Code keeps a top-level index at `$KIMI_CODE_HOME/session_index.jsonl`. Local inspection found 14 JSONL records, each with `sessionId`, `sessionDir`, and `workDir`. Current session IDs in local storage use the form `session_<uuid>`, for example `session_292400e4-dc10-4762-8542-7585ee220961`; legacy `~/.kimi` sessions used bare UUID directory names.

Session IDs can be captured through:

- `session_index.jsonl`: stable local index with session ID, session directory, and working directory.
- Session directory name: `$KIMI_CODE_HOME/sessions/<workDirKey>/<sessionId>/`.
- CLI export: `kimi export [sessionId]` accepts an explicit ID or selects the latest cwd session when omitted.
- Hooks: hook stdin JSON includes `session_id` and `cwd`.
- ACP: `session/list`, `session/load`, and `session/resume` use protocol-level session identifiers.
- Resume hint: legacy docs state non-empty sessions print `kimi -r <session-id>` on exit. Current docs still list `--resume`/`-r` as hidden aliases but focus on `--session`.

The earliest scriptable current handle is the session ID in `session_index.jsonl` after the session has been created and indexed. Hooks provide a cleaner event-time capture when configured. The TUI picker is not a stable automation surface.

## Resume Invocation

Continue the latest session for the current directory:

```sh
kimi --continue
```

Resume a specific session by ID:

```sh
kimi --session session_292400e4-dc10-4762-8542-7585ee220961
```

Open the interactive picker:

```sh
kimi --session
```

Switch sessions inside an idle TUI session:

```text
/sessions
```

Send a non-interactive follow-up to a known session:

```sh
kimi -p "Continue from the previous result and summarize the remaining risks." --session session_292400e4-dc10-4762-8542-7585ee220961
```

Send a non-interactive follow-up to the latest cwd session:

```sh
kimi -p "Continue and give the next concrete step." --continue
```

Capture structured non-interactive output:

```sh
kimi -p "List the changed files." --session session_292400e4-dc10-4762-8542-7585ee220961 --output-format stream-json
```

In `stream-json` mode, stdout contains JSONL Assistant and Tool messages. Thinking content, tool progress, and resume notices remain on stderr. Stream parsing details belong to the `non-interactive-sessions` topic.

ACP exposes the machine-oriented sequence:

```text
initialize
session/list
session/load or session/resume
session/prompt
session/cancel
```

`session/load` restores a session and replays history to the client. `session/resume` is a lighter sibling that skips history replay.

## Session Lookup Scope

Current Kimi Code groups sessions by working directory. The documented `workDirKey` format is `wd_<slug>_<first-12-chars-of-sha256>`, derived from the working directory path. Local directories confirmed that shape, for example `wd_claudine_50714871d8b0` and `wd_sniff_316962d80b01`.

`--continue` selects the most recent session for the current working directory. Explicit `--session <id>` targets a session by ID. Legacy docs say the interactive session picker can toggle between current-directory sessions and all directories with `Ctrl-A`; current Kimi Code docs only say `/sessions` browses previous sessions. There is no documented branch, PR, or git-worktree selector.

## State Storage

Current Kimi Code stores state under `KIMI_CODE_HOME`, defaulting to:

| OS | Default root | Session path |
|----|--------------|--------------|
| macOS | `/Users/<name>/.kimi-code` | `/Users/<name>/.kimi-code/sessions/<workDirKey>/<sessionId>/` |
| Linux | `/home/<name>/.kimi-code` | `/home/<name>/.kimi-code/sessions/<workDirKey>/<sessionId>/` |
| Windows | `C:\Users\<name>\.kimi-code` | `C:\Users\<name>\.kimi-code\sessions\<workDirKey>\<sessionId>\` |

Official current layout:

```text
$KIMI_CODE_HOME
├── config.toml
├── session_index.jsonl
└── sessions/
    └── <workDirKey>/
        └── <sessionId>/
            ├── state.json
            └── agents/
                ├── main/
                │   └── wire.jsonl
                └── <subagentId>/
                    └── wire.jsonl
```

The data-location docs also list optional `upcoming-goals.json`, `agents/main/plans/`, `logs/kimi-code.log`, `tasks/`, and `cron/` under a session directory. `session_index.jsonl` records `sessionId`, `sessionDir`, and `workDir`.

Local current Kimi Code inspection found:

- `/Users/ken/.kimi-code/session_index.jsonl` with 14 records.
- `/Users/ken/.kimi-code/sessions/wd_claudine_50714871d8b0/session_292400e4-dc10-4762-8542-7585ee220961/state.json`.
- `agents/main/wire.jsonl` in current sessions.
- Subagent streams such as `agents/agent-0/wire.jsonl`, `agents/agent-1/wire.jsonl`, and `agents/agent-2/wire.jsonl` in one observed session.
- `logs/kimi-code.log` in some session directories.

Current `state.json` files observed locally contained scalar paths such as `createdAt`, `updatedAt`, `title`, `lastPrompt`, `agents.main.homedir`, `agents.main.type`, and for subagents `agents.agent-0.parentAgentId`. Current `agents/main/wire.jsonl` records included top-level shapes such as `type`, `protocol_version`, `app_version`, `created_at`, `systemPrompt`, `names`, `modelAlias`, `thinkingLevel`, `mode`, and `input`.

Legacy Kimi CLI data remains under `/Users/ken/.kimi` on this host, with `/Users/ken/.kimi/.migrated-to-kimi-code` present. Legacy sessions use:

```text
~/.kimi/sessions/<work-dir-hash>/<uuid>/
├── context.jsonl
├── wire.jsonl
└── state.json
```

Observed legacy `context.jsonl` roles included `_system_prompt`, `_checkpoint`, `user`, `assistant`, `_usage`, and `tool`. This legacy format is real and may matter for migration, but current Kimi Code wrapper logic should prefer `~/.kimi-code` and `KIMI_CODE_HOME`.

The session file format is not documented as a stable integration API. Official docs explicitly warn not to manually edit files under `sessions/` because doing so may break restore.

## Restored State

Kimi Code restores conversation history and tool results by replaying `agents/*/wire.jsonl`. `state.json` restores metadata such as title, creation/update timestamps, `lastPrompt`, `forkedFrom`, and agent/subagent metadata. The current docs state that resuming can restore saved permission or plan mode, and that adding `--auto`, `--yolo`, or `--plan` at resume time overrides those saved modes. `--model` can override the model alias for the launch.

Do not assume the original environment survives. A resumed CLI process inherits the current process environment. Current docs resolve additional workspace roots from launch flags and config; persisted roots may exist in session state, but this run did not find a local state example with additional directory fields. Current working directory is part of lookup scope through `workDir`, but exact cwd restoration versus launch cwd should be treated as provider-managed and verified in wrapper tests.

Resume continues the selected local session. The docs do not explicitly state whether resume appends to the same `wire.jsonl` or creates a new stream, but local storage and ACP wording indicate that the session directory remains the authoritative record. Forking is the documented mechanism for creating an independent copy.

## Branching and Checkpoints

`/fork` creates an independent session from the current conversation so the user can explore a new direction without affecting the original. Current docs state that the two resulting sessions are independent and that a saved `/goal` is not copied to the fork.

Kimi Code also supports `/title <text>` and alias `/rename` for naming a session. `kimi export [sessionId]` creates a ZIP with all files in the session directory; `/export-debug-zip` does the same from the TUI, and `/export-md` exports a human-readable Markdown transcript. `kimi vis [sessionId]` opens a visualizer for a specific session or a home view of sessions.

No checkpoint, rewind, or branch-from-arbitrary-turn command was verified in current Kimi Code docs. `/compact` compresses context to save tokens, but it is not a user-addressable checkpoint.

## Human-in-the-Loop Resume

Kimi Code does not document a durable human-in-the-loop resume flow where a wrapper captures a pending question or approval, exits, asks the human elsewhere, then injects the answer into the same stopped session later.

ACP supports normal interactive agent flow, including session prompt, cancel, and tool approval while the ACP process is alive. That is useful for live HITL orchestration, but it is synchronous protocol control, not persisted defer-and-resume. In `-p/--prompt` mode, current docs say no human approval is requested: regular tool calls are handled under the `auto` permission policy and static deny rules remain in effect.

For Claudine, Kimi HITL should be modeled as live-process handling through ACP or the TUI, not as a post-exit lifecycle `resume` capability.

## Interruption Recovery

Because session state is local and replayable, a session can be resumed after terminal close, Ctrl-C, crash, or provider/process failure once enough state has been written. Legacy docs explicitly say a resume hint is printed for non-empty sessions after normal exit and Ctrl-C. Current Kimi Code docs describe persistent session storage and recovery/replay through `agents/*/wire.jsonl`.

This is not the same as resuming an in-flight tool call. The docs do not promise that a shell command, file edit, approval prompt, network request, or model call interrupted mid-turn will continue from the exact blocked point. Treat recovery as continuation from persisted transcript/event state. Pending tool and pending approval recovery should be considered unsupported unless wrapper tests prove otherwise for a specific version.

Concurrent resume of the same session is undocumented. Claudine should avoid launching two write-capable resumes for the same Kimi session unless it adds its own lock.

## Observability

Resume-relevant observability surfaces:

- `session_index.jsonl`: local session ID, directory, and working directory index.
- `state.json`: current session metadata and agent/subagent metadata.
- `agents/*/wire.jsonl`: event stream used for replay and recovery.
- Hooks: stdin JSON includes `hook_event_name`, `session_id`, and `cwd`; hook commands run in the session project directory.
- `kimi -p --output-format stream-json`: JSONL Assistant/Tool messages on stdout for non-interactive follow-up.
- ACP: `session/list`, `session/load`, `session/resume`, `session/prompt`, and `session/cancel`.
- Logs: global `~/.kimi-code/logs/kimi-code.log` and optional per-session `logs/kimi-code.log`.
- `kimi export`: ZIP of session directory, with optional inclusion of the global diagnostic log.
- `kimi server status`: local server installation/running status, with `--json` for automation.

Direct file parsing is useful for discovery and backup, but not documented as a stable API. Prefer CLI commands, hooks, ACP, or server APIs when integrating.

## Quirks and Gaps

Quirks:

- Current Kimi Code is not the same on disk as legacy Kimi CLI. Current data lives under `~/.kimi-code`; legacy migrated data can remain under `~/.kimi`.
- The installed local binary is `kimi` version `0.14.0` from `/Users/ken/.kimi-code/bin/kimi`.
- Current non-interactive mode is `-p/--prompt`; `--print` was not shown by installed `kimi --help`.
- Current session replay uses `agents/*/wire.jsonl`; legacy sessions use `context.jsonl` plus `wire.jsonl`.
- Current docs warn not to manually edit `sessions/` contents.
- `--continue` and `--session` are mutually exclusive.
- Non-interactive `-p` mode uses auto permission policy and does not request human approvals.
- Resume-time `--auto`, `--yolo`, and `--plan` can override saved permission/plan state.

Gaps:

- `kimi -p ... --session <id>` and `kimi -p ... --continue` were not run with a live model call in this research pass. Current docs imply the flag composition, but the body records that as an inference.
- Concurrent resume semantics are undocumented.
- No persisted pending HITL question/approval answer-injection path was found.
- Session retention policy is undocumented.
- Linux and Windows paths were taken from official docs, not observed on those OSes.
- Local server REST/WebSocket session APIs were not inspected; ACP was sufficient to verify API-level resume semantics.

## Claudine Integration Notes

For lifecycle `resume`, Claudine should treat Kimi as a local transcript replay provider with first-class session IDs. Prefer capturing the session ID from hooks or `session_index.jsonl`; fall back to resume hints where available. Current Kimi Code session IDs can include a `session_` prefix, so validators should not assume bare UUIDs.

For non-interactive follow-up, prefer:

```sh
kimi -p "<follow-up>" --session <session-id> --output-format stream-json
```

or:

```sh
kimi -p "<follow-up>" --continue --output-format stream-json
```

Claudine should not use legacy `--print` for current Kimi Code without version gating. It should discover data under `KIMI_CODE_HOME` or `~/.kimi-code`, not only `~/.kimi`. Legacy `~/.kimi` may still matter for migration or old installations, but current Kimi Code wrapper behavior should target `session_index.jsonl` and `sessions/<workDirKey>/<sessionId>/agents/*/wire.jsonl`.

For `retry`, restarting a failed Kimi run should be modeled as a new launch against the same or latest session, not as resuming an in-flight tool call. For `proxy`, ACP is the best documented protocol surface because it has explicit `session/load`, `session/resume`, `session/prompt`, and `session/cancel`. For future human-in-the-loop recovery, Claudine should handle Kimi questions and approvals while the ACP/TUI process is alive; it should not assume a stopped Kimi session can later receive a persisted answer.

## Changelog

- 2026-07-03: Refreshed against current Kimi Code docs, local `kimi` 0.14.0 help, and local session files under `/Users/ken/.kimi-code` and `/Users/ken/.kimi`.
- 2026-07-03: Replaced legacy `--print` examples with current `-p/--prompt` examples and recorded the flag-composition evidence boundary.
- 2026-07-03: Updated storage from legacy `~/.kimi/sessions/<hash>/<uuid>/{context.jsonl,wire.jsonl,state.json}` to current `~/.kimi-code/sessions/<workDirKey>/<sessionId>/{state.json,agents/*/wire.jsonl}` while preserving legacy migration notes.
- 2026-07-03: Added ACP `session/load` versus `session/resume`, server/web surfaces, and local `session_index.jsonl` findings.

## Sources

- [Kimi Code CLI - Sessions and context](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/sessions.html)
- [Kimi Code CLI - Data locations](https://www.kimi.com/code/docs/en/kimi-code-cli/configuration/data-locations.html)
- [Kimi Code CLI - `kimi` Command](https://www.kimi.com/code/docs/en/kimi-code-cli/reference/kimi-command.html)
- [Kimi Code CLI - `kimi acp` Subcommand](https://www.kimi.com/code/docs/en/kimi-code-cli/reference/kimi-acp.html)
- [Kimi Code CLI - Hooks](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html)
- [Legacy Kimi CLI - Sessions and Context](https://moonshotai.github.io/kimi-cli/en/guides/sessions.html)
- [MoonshotAI/kimi-cli GitHub repository](https://github.com/MoonshotAI/kimi-cli)
- Local inspection: `/Users/ken/.kimi-code/bin/kimi --version`, `/Users/ken/.kimi-code/session_index.jsonl`, `/Users/ken/.kimi-code/sessions/*`, and legacy `/Users/ken/.kimi/sessions/*`.
