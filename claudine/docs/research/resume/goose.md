---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://goose-docs.ai/docs/guides/sessions/session-management/
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "goose session --resume"
      - "goose session --resume --name <name>"
      - "goose session --resume --session-id <id>"
      - "goose session --resume --path <file>"
      - "goose session --resume --fork"
      - "goose session --resume --edit"
      - "goose project"
      - "goose projects"
      - "goose term init --name <name> plus @goose"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - name
      - picker
      - worktree
      - other
    notes: "Interactive resume is first-class. Session resume can target latest, name, ID, or a legacy/export path. Project and terminal integration add human-oriented continuation surfaces; project selection is directory/project-based and terminal named sessions are keyed by the shell integration's session name."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "goose run --resume -t \"follow-up prompt\""
      - "goose run --resume --name <name> -t \"follow-up prompt\""
      - "goose run --resume --session-id <id> -t \"follow-up prompt\""
      - "goose run --resume --path <file> -t \"follow-up prompt\""
      - "goose term run <prompt>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
      - other
    notes: "Headless run accepts a new prompt from --text, --instructions, stdin, or recipe. --no-session is mutually exclusive with --resume. Terminal integration can send prompts into the terminal-associated persistent session, but it is a separate shell integration surface."
  - mode: ide
    supported: true
    mechanisms:
      - "Goose Desktop sidebar"
      - "Session History Resume"
      - "Session History New Window"
      - "ACP clients backed by Goose session storage"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
      - id
      - name
    notes: "Desktop and CLI use the same session database. Desktop can switch active sessions, resume from history, duplicate, export, import, rename, and delete sessions."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "goose serve/goosed and ACP exist, but no documented stable headless HTTP or JSON-RPC API was found for Claudine-style programmatic session resume."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No public SDK or REST API for direct resume was verified. ACP is an IDE/client protocol surface rather than a documented general resume API."
session_id_capture:
  - surface: stdout
    field: "session_id"
    format: "YYYYMMDD_<COUNT>"
    notes: "The CLI displays the session ID in the session info header unless quiet output suppresses non-response text."
  - surface: cli_command
    field: "id"
    format: "JSON array of Session records"
    notes: "goose session list --format json returns sessions with id, name, working_dir, timestamps, metadata, and message-count-derived fields."
  - surface: session_file
    field: "sessions.id"
    format: "SQLite primary key"
    notes: "The sessions table primary key is generated as YYYYMMDD_N by create_session. Messages reference it through messages.session_id."
  - surface: other
    field: "AGENT_SESSION_ID"
    format: "environment variable"
    notes: "Terminal integration exposes the active terminal session ID through AGENT_SESSION_ID; shell tools/extensions may also receive a session-scoped ID."
  - surface: log_file
    field: "session identifiers"
    format: "implementation logs"
    notes: "The logging guide says CLI logs include session identifiers, but logs are operational observability rather than the preferred handle-capture surface."
resume_invocations:
  - mode: interactive
    invocation: "goose session --resume"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recently active visible user/scheduled session."
  - mode: interactive
    invocation: "goose session --resume --name <name>"
    accepts_prompt: false
    selection: name
    notes: "Name matching also accepts an exact session ID through the current get_or_create_session_id implementation."
  - mode: interactive
    invocation: "goose session --resume --session-id <id>"
    accepts_prompt: false
    selection: id
    notes: "Best explicit-handle resume path for wrappers."
  - mode: interactive
    invocation: "goose session --resume --path ./session.json"
    accepts_prompt: false
    selection: other
    notes: "Documented for exported sessions and legacy JSONL. Current CLI identifier handling extracts the file stem as the session ID; import/export paths are separate commands."
  - mode: interactive
    invocation: "goose session --resume --fork [--name <name>|--session-id <id>|--path <file>] [--history]"
    accepts_prompt: false
    selection: other
    notes: "Copies the selected session and resumes the copy; --history prints previous messages after resuming."
  - mode: interactive
    invocation: "goose session --resume --session-id <id> --edit [--fork] [--history]"
    accepts_prompt: false
    selection: id
    notes: "Opens the conversation in $VISUAL, $EDITOR, or vi as YAML; saved edits replace or fork history before continuing."
  - mode: non_interactive
    invocation: "goose run --resume --session-id <id> -t \"follow-up prompt\" --output-format json"
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up. Structured result is available after completion; stream-json emits events but not session_id."
  - mode: non_interactive
    invocation: "goose run --resume --name <name> -t \"follow-up prompt\""
    accepts_prompt: true
    selection: name
    notes: "Resolves name or ID to a stored session before appending the prompt."
  - mode: non_interactive
    invocation: "goose run --resume -t \"follow-up prompt\""
    accepts_prompt: true
    selection: latest
    notes: "Convenient but less deterministic; implementation resolves latest before build_session, but saved provider/model restoration is most reliable when an explicit session_id is supplied."
  - mode: interactive
    invocation: "goose project"
    accepts_prompt: false
    selection: other
    notes: "Interactive project manager can resume the most recent tracked project with its associated session or start fresh in that directory."
  - mode: interactive
    invocation: "goose projects"
    accepts_prompt: false
    selection: picker
    notes: "Interactive project picker; not safe to treat as an automatable session selector."
  - mode: non_interactive
    invocation: "goose term run <prompt>"
    accepts_prompt: true
    selection: other
    notes: "Sends a prompt to the terminal-integrated persistent session selected by AGENT_SESSION_ID or the shell integration's named-session setup."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/goose/sessions/sessions.db"
    format: "SQLite, current source schema version 14"
    retention: "Indefinite until removed; CLI/server logs are cleaned after about two weeks."
    notes: "Official docs group macOS with Unix-like paths. Source uses etcetera app strategy with Block/goose compatibility and GOOSE_PATH_ROOT for tests, so wrappers should prefer goose info/docs over hard-coded parsing. No Goose session database existed on this host under the documented or Block Application Support paths."
  - location: local
    os: linux
    path: "~/.local/share/goose/sessions/sessions.db"
    format: "SQLite, current source schema version 14"
    retention: "Indefinite until removed; CLI/server logs are cleaned after about two weeks."
    notes: "Legacy JSONL files under the sessions directory are imported during migration and then left unmanaged."
  - location: local
    os: windows
    path: "%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db"
    format: "SQLite, current source schema version 14"
    retention: "Indefinite until removed; CLI/server logs are cleaned after about two weeks."
    notes: "Windows uses the Block/goose data directory documented by Goose. Console and shell behavior differ, but session schema is shared."
resume_scope:
  project_scoped: false
  cwd_scoped: false
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Core session resume is global over visible user/scheduled sessions. session list can filter by working_dir substring, and Projects store directory-to-session metadata in projects.json, but plain --resume is not repository-, branch-, or worktree-scoped."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "goose session --resume --fork [--name <name>|--session-id <id>|--path <file>]"
  checkpoint_invocation: "n/a"
  preserves_original: true
  notes: "CLI fork creates a new session ID and copies conversation, extension data, schedule/recipe/user values, project ID, provider/model config, and goose mode. The inspected CLI copy path did not copy accumulated usage/cost fields. Desktop has duplicate and edited-message fork concepts; import creates a new ID."
restored_state:
  transcript: true
  tool_results: true
  approvals: cleared
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Conversation messages, tool requests/responses, extension data, recipe data, provider, model config, project ID, and goose_mode are stored. Explicit --session-id resume restores saved provider/model unless CLI flags override them; latest/name resume resolves through CLI selection and should not be assumed equivalent for model restoration without verification. Interactive resume asks before switching to the saved working directory; headless resume warns and stays in the current launch directory."
hitl_resume:
  supported: false
  question_capture: "ActionRequired messages can appear for tool confirmation or MCP elicitation during a live run, and stream-json can emit message events containing action-required content."
  answer_injection: "No documented CLI/API accepts a later answer into a suspended session. Live interactive code answers by calling internal agent handlers inside the same process."
  limitations: "Headless approve/smart_approve fails on tool confirmation; MCP elicitation fails in non-interactive mode. There is no durable pending-question queue or resume-with-answer command."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: false
  limitations: "Persisted messages can be resumed after process loss. On stream errors or Ctrl+C, CLI code attempts to remove or patch interrupted messages around the most recent user message, so an interrupted turn may be incomplete or rewritten. Concurrent resume/write behavior is undocumented."
observability:
  stream_events:
    - "message"
    - "notification"
    - "error"
    - "complete"
  hook_events:
    - "GOOSE_STATUS_HOOK waiting/thinking"
    - "internal HookEvent::SessionEnd"
  failure_modes:
    - "Cannot resume - no previous sessions found"
    - "Cannot resume session <id> - no such session exists"
    - "No session found with name <name>"
    - "Tool approval required in non-interactive mode"
    - "Elicitation requested but no interactive terminal is available"
    - "Working directory differs from session; staying in current directory"
  notes: "goose run --output-format json emits final messages and metadata. stream-json emits event lines as work occurs but no stable session_id field. Session listing, export, diagnostics, and logs are the useful resume observability surfaces."
quirks:
  - "The project moved from Block-hosted branding to AAIF/Linux Foundation branding, but source paths and Windows directories still preserve Block/goose compatibility."
  - "Goose was not installed on this host according to sniff software agents, and no local Goose session database or transcript files were found under documented macOS/XDG paths."
  - "Session IDs are date counters such as 20260213_9, not UUIDs."
  - "Plain --resume selects the latest visible user/scheduled session globally, not the latest session for the current repository."
  - "--name matching in current CLI code also matches session IDs."
  - "Legacy --path is documented for exported JSON and JSONL, but current identifier code extracts a file stem as a session ID; import/export commands are the safer migration surface."
  - "Explicit --session-id is safer than latest/name resume for wrappers because saved provider/model configuration is read before provider resolution only for explicit IDs in the inspected builder code."
  - "GOOSE_DISABLE_SESSION_NAMING avoids an extra model call and keeps the default CLI Session name for headless workflows."
  - "The SQLite schema is observable but internal; direct SQL is useful for diagnostics, not a stable integration contract."
gaps:
  - "No real Goose transcript existed on this host to inspect; local evidence is limited to negative filesystem and program-detection probes plus upstream source/docs."
  - "No documented concurrency guarantee for resuming the same session from multiple processes."
  - "No verified public API for resume outside CLI/Desktop/ACP client behavior."
  - "Exact recovery semantics for an interrupted assistant turn depend on internal message cleanup and were not verified against a live run."
  - "Whether OAuth/session connection state for every extension survives resume is not documented."
  - "The docs say exported JSON can be resumed with --path, but source-level path handling appears ID-stem based; live behavior with exported JSON was not verifiable without an installed Goose binary."
changes:
  - "2026-07-03: Verified current AAIF Goose docs and source instead of relying on the prior Block-era research."
  - "2026-07-03: Added project and terminal-integration continuation surfaces, including projects.json and AGENT_SESSION_ID behavior."
  - "2026-07-03: Updated storage details to current SQLite schema version 14 and added project_id, archived_at, usage/cost, message_count, and last-message fields."
  - "2026-07-03: Recorded negative local evidence: Goose is not installed on this host and no session database/transcripts were found under documented paths."
  - "2026-07-03: Corrected wrapper guidance around explicit --session-id resume versus latest/name resume for saved provider/model restoration."
  - "2026-07-03: Added legacy --path, --history, export/import/diagnostics, Desktop duplicate/import/export, and source-observed fork limitations."
requires_claudine_update: true
reason: "Claudine should treat Goose resume as first-class but prefer explicit --session-id handles, account for project/terminal continuation surfaces, avoid relying on stream-json for session IDs, and update Goose metadata for current SQLite schema/project fields and non-HITL limitations."
---

# Goose CLI Resume Research

## Overview

Goose CLI has first-class resume support for both interactive and non-interactive sessions. The practical continuity model is local transcript replay from Goose's SQLite session store: a new process loads a saved conversation and session metadata, then appends new messages. CLI wrappers can resume the latest session, a named session, an exact session ID, a legacy/export path, a project-associated session, or a terminal-integrated named session.

The main automation risks are selector ambiguity and partial state restoration. `goose run --resume --session-id <id> -t "..."` is the safest wrapper primitive. `--resume` without an explicit handle is global, not cwd-scoped; `--name` also matches IDs in current source; stream JSON does not include a session ID; and headless runs cannot suspend on approvals or MCP elicitation for a later human answer.

## Resume Semantics

Goose's persistent session is a local record with metadata plus conversation messages. Current source defines a `Session` with fields including `id`, `working_dir`, `name`, `user_set_name`, `session_type`, `created_at`, `updated_at`, `extension_data`, usage/cost fields, `schedule_id`, `recipe`, `user_recipe_values`, `conversation`, `message_count`, `last_message_at`, `provider_name`, `model_config`, `goose_mode`, `archived_at`, `project_id`, and `last_message_snippet`. Messages live in a separate SQLite `messages` table and are loaded into a `Conversation` on resume.

The applicable resume patterns are continue-latest, resume-by-handle, interactive picker, non-interactive follow-up, transcript replay, branch/fork, recovery resume, Desktop/IDE continuation, project continuation, and terminal named-session continuation. This is not live-process attach for the CLI: a resumed CLI session reconstructs context from stored messages and metadata. It is not a server-side session in the sense of an authoritative cloud session. Goose has local server/ACP components, but no stable public resume API was verified for Claudine to call directly.

Chat-history export is not itself resume unless Goose can import or load it into a continued session. Goose can export sessions as JSON/YAML/Markdown, import JSON through Desktop/CLI import paths, and documents `--path` for exported JSON or legacy JSONL resume. Memory, context files, `.goosehints`, and project instructions are context sources, not prior-session continuation mechanisms.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector | Automation fit |
|------|-------------|------------------|----------|----------------|
| Interactive CLI | `goose session --resume` | No | Latest globally | Human-oriented |
| Interactive CLI | `goose session --resume --name <name>` | No | Name, also ID in source | Scriptable selector, interactive session |
| Interactive CLI | `goose session --resume --session-id <id>` | No | Exact ID | Best interactive handle |
| Interactive CLI | `goose session --resume --path <file>` | No | Legacy/export path | Ambiguous; verify live before relying on it |
| Interactive CLI | `goose session --resume --fork` | No | Latest, name, ID, path | Human-oriented branch |
| Interactive CLI | `goose session --resume --edit` | No | Latest, name, ID, path | Opens editor; not automation-safe |
| Non-interactive CLI | `goose run --resume --session-id <id> -t "..."` | Yes | Exact ID | Best headless primitive |
| Non-interactive CLI | `goose run --resume --name <name> -t "..."` | Yes | Name, also ID in source | Scriptable with collision risk |
| Non-interactive CLI | `goose run --resume -t "..."` | Yes | Latest globally | Scriptable but unsafe if multiple sessions exist |
| Desktop | Sidebar active session / Session History | Typed after selection | Picker | Human-oriented |
| Projects | `goose project`, `goose projects` | No direct prompt at selection | Project picker/latest | Human-oriented |
| Terminal integration | `goose term init --name <name>`, `goose term run <prompt>` | Yes | Terminal/named session | Shell-oriented, not the core `run --resume` path |

Sessions created by non-interactive `goose run` are stored unless `--no-session` is used. `--no-session` is mutually exclusive with `--resume`, `--name`, and `--path`, so runs intended for later continuation must not use `--no-session`.

## Session ID Capture

Goose session IDs use `YYYYMMDD_<COUNT>`, such as `20260213_9`. Useful capture surfaces are:

- The CLI session header, unless `--quiet` suppresses non-response output.
- `goose session list --format json`, which emits stored session records.
- `goose session export --session-id <id> --format json`, which emits the full session backup.
- `goose session diagnostics --session-id <id>`, which emits a diagnostics bundle containing session data and logs.
- The SQLite `sessions.id` primary key.
- `AGENT_SESSION_ID` in terminal integration and shell/tool contexts.

`goose run --output-format stream-json` emits `message`, `notification`, `error`, and `complete` events, but current source does not include a session ID in those event variants. A Claudine wrapper should capture the handle before launching, from the start header, or by listing sessions after launch with a tight correlation strategy.

## Resume Invocation

Continue the latest session interactively:

```bash
goose session --resume
```

Resume an exact session interactively:

```bash
goose session --resume --session-id 20260213_9
```

Resume by name:

```bash
goose session --resume --name react-migration
```

Show previous messages when resuming:

```bash
goose session --resume --session-id 20260213_9 --history
```

Fork and continue a copy:

```bash
goose session --resume --fork --session-id 20260213_9
```

Edit history before continuing:

```bash
goose session --resume --session-id 20260213_9 --edit
goose session --resume --session-id 20260213_9 --fork --edit --history
```

Send a scriptable follow-up prompt and capture structured output:

```bash
goose run --resume --session-id 20260213_9 -t "Run the tests and summarize failures" --output-format json
goose run --resume --session-id 20260213_9 -t "Continue" --output-format stream-json
```

Project and terminal continuation:

```bash
goose project
goose projects
goose term init zsh --name auth-bug
goose term run "what was the solution we discussed?"
```

## Session Lookup Scope

Core session lookup is global over visible user/scheduled sessions. `goose session --resume` and `goose run --resume` without an identifier select the first result from `SessionManager::list_sessions()`, which lists user and scheduled sessions and orders them by activity in current implementation. It is not scoped to the current directory, repository, worktree, or branch.

Discovery can be narrowed by `goose session list -w <path>`, which filters sessions by a working-directory substring. Goose Projects add a separate directory-to-session layer: the docs describe `~/.local/share/goose/projects.json` with path, last accessed time, last instruction, and associated session ID. That makes project continuation available, but it is a separate project manager surface, not a change to plain session resume scope.

## State Storage

Goose stores resumable session state locally. Official docs list these locations:

| OS | Session records | Command history | CLI/server logs |
|----|-----------------|-----------------|-----------------|
| macOS | `~/.local/share/goose/sessions/sessions.db` | `~/.config/goose/history.txt` | `~/.local/state/goose/logs/` |
| Linux | `~/.local/share/goose/sessions/sessions.db` | `~/.config/goose/history.txt` | `~/.local/state/goose/logs/` |
| Windows | `%APPDATA%\Block\goose\data\sessions\sessions.db` | `%APPDATA%\Block\goose\data\history.txt` | `%APPDATA%\Block\goose\data\logs\` |

Current source uses an `etcetera` app strategy with `Block/goose` kept for backward compatibility and a `GOOSE_PATH_ROOT` override for tests. The source constant `CURRENT_SCHEMA_VERSION` is `14`. The SQLite schema includes `schema_version`, `sessions`, `messages`, and additional inventory/thread tables. The session table stores provider/model config, extension data, recipe values, working directory, mode, project ID, archive time, and usage/cost counters. The messages table stores `message_id`, `session_id`, role, content JSON, timestamps, token count, and metadata JSON.

Local inspection on this macOS host found no installed Goose binary through `sniff software agents`, no `goose` on `PATH`, and no session database or transcript files under the documented XDG paths or the checked `~/Library/Application Support/Block/goose` / `~/Library/Application Support/Goose` paths. That is negative evidence: there were no real local transcript rows to inspect.

The database is useful evidence, but direct parsing should be treated as unsupported. Prefer `goose session list --format json`, `goose session export`, and `goose session diagnostics` when possible.

## Restored State

Resume restores the transcript and tool results because the conversation is reloaded from `messages`. It also stores and can restore session metadata: provider name, model config, extension data, recipe data, user recipe values, `goose_mode`, working directory, project ID, schedule ID, and usage fields.

Restoration is not uniform across selectors. In the inspected `build_session` path, saved provider/model config is read before provider resolution only when `session_config.resume` and `session_config.session_id` are set. A wrapper should therefore prefer `--session-id` over latest/name resume if preserving saved provider/model is important. CLI `--provider` and `--model` can override the saved provider/model. Recipe settings and current config can also participate in provider/model resolution.

Interactive resume checks whether the current directory differs from the saved working directory and asks whether to switch back. Non-interactive resume does not change directories; it prints a warning and stays in the launch directory. Environment variables are from the current process. Individual approvals are not stored for later replay; tool confirmations are evaluated again under the active Goose mode. Resume continues writing to the same session ID unless `--fork`, duplicate, or import creates a new one.

## Branching and Checkpoints

Goose supports branching-like behavior through fork, duplicate, edit, import, and export:

| Feature | Surface | Preserves original | Notes |
|---------|---------|--------------------|-------|
| Fork | `goose session --resume --fork ...` | Yes | Creates a new session ID and copies conversation plus key metadata. |
| Edit | `goose session --resume --edit ...` | No, unless combined with `--fork` | Opens YAML conversation in an editor and replaces/truncates saved messages. |
| Desktop duplicate | Session History duplicate action | Yes | Docs describe a full copy visible at the top of the session list. |
| Import | Desktop import or `goose session import <file>` | Yes | Creates a new session ID rather than overwriting. |
| Export | `goose session export --format json|yaml|markdown` | n/a | JSON/YAML are complete backups; Markdown is for reading/sharing. |
| Diagnostics | `goose session diagnostics --session-id <id>` | n/a | Captures session data, config files, logs, and system info for debugging. |

There is no named checkpoint, rewind, or live branch graph for CLI wrappers. Goose does have context compaction and message truncation internals, but those are not a general checkpoint API.

## Human-in-the-Loop Resume

Goose does not provide Claudine-style human-in-the-loop resume. During a live interactive process, action-required messages can ask for tool confirmation or MCP elicitation. The CLI answers those by calling internal agent handlers in the same process.

In headless mode, Goose cannot later capture a question, stop, let another system ask a user, and inject the answer into the same suspended turn. Current source returns an error if `approve` or `smart_approve` mode requires a tool confirmation in non-interactive mode. It also errors when MCP elicitation needs an interactive terminal. Use `GOOSE_MODE=auto` only when the automation policy allows tools to proceed without confirmation; otherwise Claudine must fail or proxy to a provider with a real pause/answer protocol.

## Interruption Recovery

Because sessions are persisted locally, Goose can resume after terminal close, process death, crash, provider error, or Ctrl+C as long as enough messages were saved. On errors and cancellation, current CLI code calls interruption cleanup that removes or patches messages around the most recent user message, so a resumed session may continue from the last clean user turn rather than from an in-flight tool call.

Pending tool calls and pending approval prompts are not durable resume points. A killed process does not leave a public "waiting for approval" handle. Concurrent resumes of the same session are not documented; the SQLite code uses transactions for writes, but no high-level interleaving or locking guarantee was found for multiple live Goose processes appending to the same conversation.

## Observability

Relevant observability surfaces:

- `goose session list --format json` for session IDs, names, paths, timestamps, metadata, and activity.
- `goose session export --format json` for full session content.
- `goose session diagnostics` for session data plus logs/config.
- `goose info` for configuration, session storage, and logs according to the CLI docs.
- `goose run --output-format json` for final messages and metadata after a run.
- `goose run --output-format stream-json` for `message`, `notification`, `error`, and `complete` lines.
- `GOOSE_STATUS_HOOK` for fire-and-forget `waiting`/`thinking` status.
- `AGENT_SESSION_ID` for terminal integration.
- CLI logs under the documented log directory; docs say logs include session identifiers, timestamps, tool invocations, and responses.

Stream-json is useful for progress, but not for session-handle capture. Hooks are not a blocking request/response channel.

## Quirks and Gaps

Quirks:

- Goose is now documented under AAIF branding, while the repository and data paths retain Block/goose compatibility.
- Session IDs are date counters, not UUIDs.
- Plain `--resume` is global and can pick an unrelated latest session.
- `--name` matching currently also matches exact session IDs.
- Explicit `--session-id` is safer than latest/name resume for saved provider/model restoration.
- `--path` is documented for exported JSON and legacy JSONL, but source-level identifier handling extracts a file stem as an ID; import/export are safer for migration.
- Non-interactive resume cannot handle approvals or elicitation.
- Working directory is not restored in headless mode.
- Direct SQLite parsing is possible but not a stable integration path.

Gaps:

- No real local Goose transcripts existed on this host to inspect.
- No public concurrency guarantee was found for simultaneous resumes of one session.
- No public resume API was verified outside CLI/Desktop/ACP client behavior.
- Live behavior of `--path` with exported JSON could not be tested without an installed Goose binary.
- Exact survival of every extension's OAuth/session state after resume is not documented.
- Mid-turn recovery after a provider/network failure is source-observable but not fully documented as a stable contract.

## Claudine Integration Notes

Claudine should model Goose resume as first-class transcript replay with local SQLite persistence. The preferred lifecycle `resume` command is:

```bash
goose run --resume --session-id "$SESSION_ID" -t "$FOLLOW_UP" --output-format stream-json
```

For exact restoration, capture and store `SESSION_ID` early. Do not rely on `stream-json` to reveal it. Avoid latest-session resume except as a human convenience. If Claudine wants cwd continuity, it should launch Goose from the saved or intended working directory because Goose headless mode will not `cd` back automatically.

`retry` can re-enter by sending a new prompt to the same session if the previous run persisted a usable state, but it must account for partially cleaned interrupted turns. `proxy` can route to Goose for normal follow-up, but not for deferred approval/HITL. Future human-in-the-loop continuation should mark Goose unsupported unless Goose adds a stable pause/answer API. For policy-sensitive automation, use `GOOSE_MODE=auto` only when Claudine's own policy has already decided auto-execution is allowed; otherwise fail closed instead of trying to answer Goose prompts later.

Project and terminal integration are useful human workflows but should not replace explicit session IDs in Claudine's provider wrapper. They may become optional discovery hints: `projects.json` can associate directories with sessions, and `AGENT_SESSION_ID` can identify an active terminal-integrated session, but both are secondary to `goose session list --format json` and explicit `--session-id`.

## Changelog

- 2026-07-03: Refreshed against current AAIF Goose docs and upstream source.
- 2026-07-03: Added project and terminal-integration continuation surfaces.
- 2026-07-03: Updated storage details to SQLite schema version 14 and added current session fields.
- 2026-07-03: Recorded negative local evidence: Goose is not installed and no local session DB/transcripts were present on this host.
- 2026-07-03: Corrected wrapper guidance to prefer explicit `--session-id` because saved provider/model restoration is more reliable than latest/name resume in inspected source.
- 2026-07-03: Added `--path`, `--history`, export/import/diagnostics, Desktop duplicate/import/export, and source-observed fork limitations.
- 2026-07-02: Converted to schema-validated frontmatter and corrected HITL/storage claims.
- 2026-07-02: Added SQLite paths, OS-specific storage, restored-state semantics, and `--fork` branching.
- 2026-07-02: Documented that Goose has no defer hook and that `approve`/`smart_approve` modes fail in non-interactive runs.

## Sources

- [Goose Session Management](https://goose-docs.ai/docs/guides/sessions/session-management/)
- [Goose CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Goose Logging System](https://goose-docs.ai/docs/guides/logs/)
- [Running Tasks with Goose](https://goose-docs.ai/docs/guides/running-tasks/)
- [Managing Projects](https://goose-docs.ai/docs/guides/managing-projects/)
- [Terminal Integration](https://goose-docs.ai/docs/guides/terminal-integration/)
- [Goose Environment Variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [Goose Permissions](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/)
- [Goose source: `crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Goose source: `crates/goose-cli/src/session/builder.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/builder.rs)
- [Goose source: `crates/goose-cli/src/session/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs)
- [Goose source: `crates/goose-cli/src/commands/session.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/commands/session.rs)
- [Goose source: `crates/goose/src/session/session_manager.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/session/session_manager.rs)
- [Goose source: `crates/goose/src/config/paths.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
