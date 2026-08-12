---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
support: first_class
continuity_model: server_session
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "kilo --continue / -c"
      - "kilo --session <id>"
      - "kilo --session <id> --fork"
      - "kilo --session <id> --cloud-fork"
      - "kilo attach <url> --continue"
      - "kilo attach <url> --session <id>"
      - "TUI /sessions, /resume, and /continue"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
    notes: "Interactive resume opens the TUI. Follow-up text is typed after the session opens; the root `--prompt` option is separate from documented `--continue` use and is not a reliable scriptable resume prompt."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "kilo run --continue <message>"
      - "kilo run --session <id> <message>"
      - "kilo run --session <id> --fork <message>"
      - "kilo run --session <id> --cloud-fork <message>"
      - "kilo run --attach <url> --session <id> <message>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Scriptable follow-up is first-class. `--format json` emits normalized event records with `sessionID`; default output is formatted text."
  - mode: headless_server
    supported: true
    mechanisms:
      - "kilo serve"
      - "kilo web"
      - "kilo daemon"
      - "GET /session"
      - "POST /session/{sessionID}/message"
      - "POST /session/{sessionID}/fork"
      - "POST /session/{sessionID}/revert"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - all_projects
    notes: "The local server exposes HTTP/SSE routes used by the TUI, `kilo run --attach`, Kilo Console, and editor clients."
  - mode: api
    supported: true
    mechanisms:
      - "@kilocode/sdk/v2 createKiloClient().session.prompt()"
      - "@kilocode/sdk/v2 createKiloClient().event.subscribe()"
      - "@kilocode/sdk/v2 createKiloClient().question.reply()"
      - "@kilocode/sdk/v2 createKiloClient().permission.reply()"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Current generated SDK samples use `createKiloClient` from `@kilocode/sdk`; older OpenCode SDK names should not be used for Kilo-specific integration."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code extension sessions"
      - "JetBrains extension sessions"
      - "Cloud Agents Recent Sessions"
      - "Remote Connections"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
    notes: "IDE and Cloud Agent resume are picker-oriented user surfaces. This document focuses on CLI/server semantics."
session_id_capture:
  - surface: json_stream
    field: sessionID
    format: "ses_<id>"
    notes: "`kilo run --format json` constructs every emitted record with `type`, `timestamp`, and `sessionID`; locally verified from current source, not by a live model run."
  - surface: cli_command
    field: id
    format: "ses_<id>"
    notes: "`kilo session list --format json --all` returned stable session IDs with `title`, `updated`, `created`, `projectId`, `directory`, and `project` fields on this host."
  - surface: session_file
    field: id
    format: "SQLite `session.id` primary key"
    notes: "Observed in `/Users/ken/.local/share/kilo/kilo.db`; related messages and parts use `session_id` foreign keys."
  - surface: stdout
    field: info.id
    format: "ses_<id>"
    notes: "`kilo export --sanitize <sessionID>` writes JSON with `info.id`, `messages[].info.sessionID`, and `parts[].sessionID`."
  - surface: interactive_ui
    field: title
    format: string
    notes: "The TUI session switcher is opened by `/sessions`, `/resume`, or `/continue`; it is a human picker, not a stable automation protocol."
resume_invocations:
  - mode: interactive
    invocation: "kilo --continue"
    accepts_prompt: false
    selection: latest
    notes: "Opens the most recent root session in the current workspace/project scope."
  - mode: interactive
    invocation: "kilo --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Opens the named local session in the TUI."
  - mode: interactive
    invocation: "kilo --session <session-id> --fork"
    accepts_prompt: false
    selection: id
    notes: "Forks the local session, then opens the fork."
  - mode: interactive
    invocation: "kilo --session <cloud-session-id> --cloud-fork"
    accepts_prompt: false
    selection: id
    notes: "Imports a cloud session through `kilo.cloud.session.import`, then opens the new local session."
  - mode: interactive
    invocation: "kilo attach <url> --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Attaches a TUI client to an explicit running Kilo server and opens the selected session."
  - mode: interactive
    invocation: "/sessions, /resume, or /continue"
    accepts_prompt: false
    selection: picker
    notes: "Slash aliases open the session switcher inside the TUI."
  - mode: non_interactive
    invocation: "kilo run --continue \"follow-up prompt\""
    accepts_prompt: true
    selection: latest
    notes: "Finds the latest root session through `session.list()` and sends the prompt."
  - mode: non_interactive
    invocation: "kilo run --session <session-id> \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Looks up the session by ID and sends the prompt with `client.session.prompt()`."
  - mode: non_interactive
    invocation: "kilo run --session <session-id> --fork \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Creates a fork with `session.fork`, then sends the prompt into the fork."
  - mode: non_interactive
    invocation: "kilo run --session <cloud-session-id> --cloud-fork \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Imports the cloud session locally, then sends the prompt to the imported session."
  - mode: non_interactive
    invocation: "kilo run --attach http://localhost:4096 --session <session-id> \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Uses a running server instead of the embedded server path; Basic auth can be supplied with `--username` and `--password`."
  - mode: headless_server
    invocation: "POST /session/{sessionID}/message"
    accepts_prompt: true
    selection: id
    notes: "Request body requires `parts`; optional fields include `model`, `agent`, `tools`, `format`, `system`, `variant`, and `noReply`."
  - mode: headless_server
    invocation: "POST /session/{sessionID}/fork"
    accepts_prompt: false
    selection: id
    notes: "Optional body field `messageID` forks at a specific message point."
  - mode: api
    invocation: "createKiloClient({ baseUrl }).session.prompt({ sessionID, parts: [...] })"
    accepts_prompt: true
    selection: id
    notes: "Kilo-native SDK call for programmatic follow-up."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/kilo/kilo.db"
    format: SQLite
    retention: "Local rows persist until deleted, uninstalled with data removal, or changed by internal migration/pruning."
    notes: "Verified on macOS with `kilo debug paths`; this host's real user data is under `/Users/ken/.local/share/kilo`, not `~/.kilo`."
  - location: local
    os: linux
    path: "~/.local/share/kilo/kilo.db"
    format: SQLite
    retention: "Local rows persist until deleted, uninstalled with data removal, or changed by internal migration/pruning."
    notes: "Inferred from Kilo's XDG-style path behavior and official Linux support; not locally verified."
  - location: local
    os: windows
    path: "%LOCALAPPDATA%\\kilo\\kilo.db"
    format: SQLite
    retention: "Local rows persist until deleted, uninstalled with data removal, or changed by internal migration/pruning."
    notes: "Inferred from common Windows app-data resolution for the same Kilo global path abstraction; not locally verified."
  - location: server
    path: "Kilo Cloud Agent / Kilo account"
    format: "cloud session record"
    retention: "Cloud Agent beta documentation says inactive sessions are deleted after 7 days; expired sessions remain accessible via the CLI."
    notes: "Cloud sessions can be imported locally with `--cloud-fork`; Remote Connections require the local CLI to keep running."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: false
  notes: "Default session listing is current project/runtime scoped and root-session filtered. `kilo session list --all` widens to all projects. Current source uses Kilo project-family/worktree-aware filters, but there is no documented git branch filter."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "kilo --session <id> --fork; kilo run --session <id> --fork <prompt>; POST /session/{sessionID}/fork"
  checkpoint_invocation: "POST /session/{sessionID}/revert; POST /session/{sessionID}/unrevert; TUI undo/redo surfaces"
  preserves_original: true
  notes: "Fork creates a new `session` row and copies message/part history with new IDs, remapping assistant parent IDs and compaction pointers. Revert records `session.revert` and writes session diffs; it is refused while the session is busy."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Transcript, reasoning, text, step, and tool parts are loaded from SQLite. Session rows can store agent, model, permission, directory, path, metadata, and revert state. `--model`, `--agent`, `--variant`, and `--dir` can override launch behavior; environment comes from the current process."
hitl_resume:
  supported: true
  question_capture: "GET /event or `client.event.subscribe()` emits `question.asked` and `permission.asked`; `GET /question` and `GET /permission` list pending requests."
  answer_injection: "`POST /question/{requestID}/reply`, `POST /question/{requestID}/reject`, SDK `question.reply()`, SDK `question.reject()`, SDK `permission.reply()`, or the deprecated `POST /session/{sessionID}/permissions/{permissionID}`."
  limitations: "Plain non-interactive `kilo run` denies question/plan/interactive-terminal permissions and auto-rejects permission prompts unless `--auto` or `--dangerously-skip-permissions` is used. For real HITL, use TUI, IDE, Remote Connections, or a server/SDK broker."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: true
  limitations: "Persisted transcript state survives crash, Ctrl+C, terminal close, and process kill. In-flight runners, pending questions, pending permissions, and background jobs are process memory; they remain answerable only while the owning server/runtime is alive. SessionRunState rejects concurrent busy operations with SessionBusyError for routes such as revert/delete, but multiple attached clients can observe the same session."
observability:
  stream_events:
    - "`kilo run --format json` emits records with `type`, `timestamp`, and `sessionID`."
    - "GET /event streams SSE events; SDK operationId is `event.subscribe`."
    - "GET /session/status returns per-session status values."
    - "GET /question and GET /permission expose pending HITL requests."
    - "`kilo session list --format json --all` exposes session IDs without parsing SQLite."
    - "`kilo export --sanitize <sessionID>` exposes stable transcript shape without sensitive text."
  hook_events:
    - "Kilo plugin/TUI event surfaces use session, message, part, question, permission, and status events."
  failure_modes:
    - "Session not found for missing explicit IDs."
    - "SessionBusyError for operations that require an idle session."
    - "QuestionRejectedError / Permission rejection when HITL requests are declined or disposed."
    - "Headless subagent permission denial to avoid unattended hangs."
  notes: "Logs live under the Kilo data log directory reported by `kilo debug paths`; logs identify run/server activity, while session identity is best captured from JSON streams, session list/export, or server events."
quirks:
  - "There is no `~/.kilo` session store on this host; Kilo uses XDG-style paths such as `~/.local/share/kilo/kilo.db`."
  - "The current source and installed help are Kilo-native; SDK examples use `createKiloClient` from `@kilocode/sdk`, not the old OpenCode client name."
  - "`kilo run` default output does not print a dedicated session-id banner; use `--format json`, `kilo session list --format json`, or `kilo export`."
  - "`kilo run --continue` selects the first root session returned by `session.list()` for the current scope; use `--session` when automation must target an exact session."
  - "`--cloud-fork` is mutually exclusive with `--fork` and `--continue`, and requires `--session`."
  - "`kilo run --replay` and `--replay-limit` apply only to `--interactive`; they are not non-interactive transcript replay controls."
  - "Plain non-interactive runs deny question, interactive_terminal, plan_enter, and plan_exit permissions, and auto-reject normal permission asks unless auto-approval flags are used."
  - "The SQLite schema and storage directories are internal; direct parsing is useful evidence but should not be Claudine's integration contract."
  - "Current local transcript sample had two sessions, four messages, and five parts; it included user text, assistant text, reasoning, and step-start parts but no completed tool-result part."
gaps:
  - "Windows storage path is inferred rather than locally observed."
  - "Exact cloud-session server retention beyond the Cloud Agent beta statement is not documented."
  - "Whether all per-session permission rules should be treated as restored approvals for Claudine automation needs provider testing with a real permission prompt."
  - "The concurrency behavior for two simultaneous prompt submissions to the same session is not fully documented, beyond busy-state guards in route-level operations."
  - "Live model execution of `kilo run --format json` was not performed; JSON `sessionID` emission was verified from current source."
changes:
  - "Updated research from Kilo CLI 7.3.45 help, current public source commit 419ff008ef180dd7076f679a89442883ba8f8d86, official docs, generated OpenAPI, and local Kilo state."
  - "Replaced OpenCode-inherited SDK wording with Kilo-native `@kilocode/sdk` / `createKiloClient` APIs."
  - "Confirmed real local state under `/Users/ken/.local/share/kilo/kilo.db`, with two observed `ses_...` sessions and transcript rows in `message` and `part` tables."
  - "Recorded current `kilo run` safety behavior: questions are denied, normal permission asks are auto-rejected, `--auto` and `--dangerously-skip-permissions` alter approval handling, and headless subagent asks are denied instead of hanging."
  - "Added current `--replay` / `--replay-limit` scope, cloud-fork validation, worktree-aware session lookup, and Kilo-native question/permission endpoints."
requires_claudine_update: true
reason: "Kilo remains on the research roster but is not a compiled Claudine provider. Claudine's future Kilo wrapper should use Kilo-native SDK/HTTP surfaces, capture `sessionID` from JSON or session-list output, avoid `kilo run` for HITL brokerage, and model current headless permission behavior."
---

# Kilo Code Session Resume

## Overview

Kilo Code has first-class resume support across the TUI, scriptable `kilo run`, attached/headless server mode, SDK/HTTP clients, IDE extensions, and Cloud Agent surfaces. Local CLI resume is not a loose chat-history export: it continues a persisted session record identified by a `ses_...` ID, loaded by an embedded, daemon, or explicit Kilo server and backed by a local SQLite database.

For Claudine, the main integration risk is not finding a resume command; it is targeting the right session and choosing the right surface. `kilo run --session <id> <prompt>` is suitable for simple non-interactive continuation, while human-in-the-loop continuation needs the server/SSE/SDK path because plain `kilo run` deliberately denies questions and auto-rejects permission prompts unless configured for auto-approval.

## Resume Semantics

A Kilo session is a persisted local runtime record in `kilo.db`, with transcript rows in `message` and `part` tables and auxiliary artifacts under the Kilo data directory. The server/runtime is responsible for loading that state, appending new user and assistant messages, publishing session events, and tracking busy/idle state. Cloud Agent sessions are separate cloud records that can be continued in Cloud Agents or imported locally with `--cloud-fork`.

The applicable resume patterns are continue latest, resume by handle, interactive picker, non-interactive follow-up, server-side/local-server session, branch/fork, checkpoint/revert, recovery resume, and human-in-the-loop resume through server events. Transcript export is observable and useful for validation, but an export is not itself the resume mechanism unless it is imported back into Kilo. Memory/config files and project instructions are context sources, not prior-session continuation.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `kilo --continue` | No | Latest root session |
| Interactive CLI | `kilo --session <id>` | No | Exact session ID |
| Interactive CLI | `kilo --session <id> --fork` | No | Exact session ID, then fork |
| Interactive CLI | `kilo --session <id> --cloud-fork` | No | Cloud session ID, then local import |
| Attached TUI | `kilo attach <url> --session <id>` | No | Exact session ID on running server |
| TUI picker | `/sessions`, `/resume`, `/continue` | No | Picker |
| Non-interactive CLI | `kilo run --continue <message>` | Yes | Latest root session |
| Non-interactive CLI | `kilo run --session <id> <message>` | Yes | Exact session ID |
| Non-interactive CLI | `kilo run --session <id> --fork <message>` | Yes | Exact session ID, then fork |
| Non-interactive CLI | `kilo run --attach <url> --session <id> <message>` | Yes | Exact session ID on running server |
| Headless server/API | `POST /session/{sessionID}/message` | Yes | Exact session ID |
| SDK | `createKiloClient().session.prompt()` | Yes | Exact session ID |
| IDE/Cloud | Extension sessions / Cloud Agents Recent Sessions | Human UI | Picker |

Sessions created by non-interactive `kilo run` are normal persisted sessions and can be resumed by ID or by `--continue` if they are the latest root session in scope. The TUI picker is human-oriented; Claudine should not attempt to automate it when a direct ID or API route is available.

## Session ID Capture

Stable session IDs use the `ses_...` shape. Reliable capture surfaces are:

- `kilo run --format json`: current source emits each JSON record with `type`, `timestamp`, and `sessionID`.
- `kilo session list --format json --all`: locally returned objects with `id`, `title`, `updated`, `created`, `projectId`, `directory`, and `project`.
- `kilo export --sanitize <sessionID>`: writes `info.id`, `messages[].info.sessionID`, and `parts[].sessionID`.
- Local SQLite: `session.id` is the primary key, and `message.session_id` / `part.session_id` link transcript rows.
- TUI: the session switcher shows a human list; use it for humans, not automation.

On this host, `kilo session list --format json --all` found two real sessions under `/Users/ken`: `ses_273de4d0cffewNgA2v0GJo0w5o` and `ses_273e3588dffefCDh7Mb6fA972k`. Both were project `global`, directory `/Users/ken`, and stored with Kilo session version `7.2.0`; the installed CLI is `7.3.45`.

## Resume Invocation

Continue latest interactively:

```bash
kilo --continue
```

Resume a specific local session interactively:

```bash
kilo --session ses_273de4d0cffewNgA2v0GJo0w5o
```

Fork while resuming:

```bash
kilo --session ses_273de4d0cffewNgA2v0GJo0w5o --fork
```

Import and continue a cloud session locally:

```bash
kilo --session ses_<cloud-session-id> --cloud-fork
```

Send a non-interactive follow-up:

```bash
kilo run --session ses_273de4d0cffewNgA2v0GJo0w5o "what is the next step?"
```

Continue latest non-interactively:

```bash
kilo run --continue "finish the task"
```

Attach to an explicit server and continue:

```bash
kilo run --attach http://localhost:4096 --session ses_273de4d0cffewNgA2v0GJo0w5o "next step"
```

Programmatic follow-up:

```typescript
import { createKiloClient } from "@kilocode/sdk"

const client = createKiloClient({ baseUrl: "http://localhost:4096" })
await client.session.prompt({
  sessionID: "ses_273de4d0cffewNgA2v0GJo0w5o",
  parts: [{ type: "text", text: "what is the next step?" }],
})
```

The current OpenAPI route for follow-up is `POST /session/{sessionID}/message`, not `POST /session/{sessionID}/prompt`. The request body requires `parts` and can also carry model, agent, tool, output-format, system-prompt, variant, and editor-context fields.

## Session Lookup Scope

Session lookup is project/runtime scoped by default. `kilo run --continue` calls `session.list()` and picks the first root session; current source filters by project and directory through Kilo's project-family/worktree-aware session filters. `kilo session list --all` widens lookup to all projects and includes project metadata in JSON output.

There is no documented git branch selector. Worktree awareness exists in source via Kilo project-family handling and session `path`, but branch filtering does not. Cloud-fork lookup is separate: it imports a cloud session by ID, then resumes the imported local session.

## State Storage

Kilo uses XDG-style data paths on this macOS host; there is no `~/.kilo` session store. `HOME=/Users/ken kilo debug paths` reported:

| Category | Path |
|----------|------|
| home | `/Users/ken` |
| data | `/Users/ken/.local/share/kilo` |
| log | `/Users/ken/.local/share/kilo/log` |
| repos | `/Users/ken/.local/share/kilo/repos` |
| cache | `/Users/ken/.cache/kilo` |
| config | `/Users/ken/.config/kilo` |
| state | `/Users/ken/.local/state/kilo` |
| tmp | `/var/folders/.../T/kilo` |

The local resumable state database is `/Users/ken/.local/share/kilo/kilo.db`. A separate session-export database, logs, `snapshot/`, `storage/session_diff/`, and `repos/` live beside it. The observed `storage/session_diff/ses_...json` files were empty arrays for the two sample sessions.

The observed SQLite tables included `session`, `message`, `part`, `session_message`, `permission`, `project`, `todo`, `event`, and migration/control tables. Important `session` columns include `id`, `project_id`, `parent_id`, `slug`, `directory`, `path`, `title`, `version`, `share_url`, summary fields, `revert`, `permission`, `workspace_id`, `agent`, `model`, cost/token fields, and timestamps. `message.data` stores message metadata as JSON; `part.data` stores text, reasoning, step, tool, and other part JSON.

The format is internal. Claudine can use local inspection for diagnostics, but should prefer CLI JSON, export/import, and HTTP/SDK APIs for integration.

## Restored State

Resume restores conversation transcript and message parts from SQLite. The local sample transcript contained user text parts, assistant reasoning/text parts, a `step-start` part with a snapshot hash, assistant `path.cwd` / `path.root`, model/provider metadata, and API error metadata in a failed session. Completed tool results were not present in the sample, but Kilo stores tool parts in the same `part` table and streams them as `message.part.updated`.

Session rows can store agent, model, permission rules, directory, relative path, metadata, summary, and revert state. Resume can override launch behavior with `--model`, `--agent`, `--variant`, and `--dir`; environment variables are from the current process. Pending in-memory runners, questions, permissions, and background jobs are not equivalent to transcript state and only remain pending while the owning runtime/server remains alive.

Resume appends to the same session unless `--fork` is used. Fork creates a new session row and copies message/part history with new IDs, leaving the original intact.

## Branching and Checkpoints

Kilo supports session forking and revert-style checkpoints.

- CLI fork: `kilo --session <id> --fork` or `kilo run --session <id> --fork <message>`.
- API fork: `POST /session/{sessionID}/fork`, with optional `messageID`.
- API revert: `POST /session/{sessionID}/revert`, with `messageID` and optional `partID`.
- API unrevert: `POST /session/{sessionID}/unrevert`.

Current source copies transcript rows into the fork with new message/part IDs, remaps assistant parent IDs and compaction tail pointers, and preserves/imports cumulative session diffs when present. Revert records `session.revert`, computes diffs, restores snapshots, and refuses to run while the session is busy.

Kilo also supports session list, delete, export, import, share, and picker-based switching. Shared session docs describe read-only share links and forked copies for collaborators.

## Human-in-the-Loop Resume

Kilo has server-level HITL primitives:

- `GET /event` / `client.event.subscribe()` for `question.asked`, `question.replied`, `question.rejected`, `permission.asked`, and permission reply events.
- `GET /question`, `POST /question/{requestID}/reply`, and `POST /question/{requestID}/reject`.
- `GET /permission` and SDK `permission.reply()`.
- Deprecated compatibility route `POST /session/{sessionID}/permissions/{permissionID}` with `response: "once" | "always" | "reject"`.

Plain `kilo run` is not the right HITL broker. In non-interactive mode it adds session permission rules denying `question`, `interactive_terminal`, `plan_enter`, and `plan_exit`; when permission asks occur, it auto-rejects them unless `--auto` or `--dangerously-skip-permissions` is set. Current source also marks unattended headless root sessions so child subagent permission asks fail instead of blocking forever. For Claudine-style "ask elsewhere, inject answer, continue same session", use an explicit server/SDK, TUI/IDE, Remote Connections, or Cloud Agent.

## Interruption Recovery

Persisted transcript state survives crash, Ctrl+C, terminal close, and process kill because session/message/part rows are written to SQLite. Resume with `--session` is safest after interruption; `--continue` can work but depends on current lookup scope and recency.

In-flight work is different. `SessionRunState` tracks busy runners in memory and cancels them when the owning runtime is disposed. Pending questions and permissions are also in memory; service finalizers reject/dismiss them when the instance is disposed. If the server remains alive, pending HITL can be answered later through server APIs. If the process is killed, the transcript remains resumable but the pending deferred question/approval does not remain live as the same in-memory request.

Concurrent client observation is normal because the TUI, attached clients, and SDK can subscribe to the same server events. Concurrent writes to the same session are not fully documented. Route-level operations such as revert guard against busy sessions with `SessionBusyError`; Claudine should serialize follow-up prompts per session unless Kilo documents stronger guarantees.

## Observability

Useful observability surfaces:

- `kilo debug paths`: authoritative local data/config/cache/log paths for the active HOME.
- `kilo session list --format json --all`: scriptable session ID discovery.
- `kilo export --sanitize <sessionID>`: safe transcript-shape inspection.
- `kilo run --format json`: final/stream event records with `sessionID`.
- `GET /event`: SSE stream for session, message, part, question, permission, network, and status events.
- `GET /session/status`: per-session lifecycle state.
- `GET /question` and `GET /permission`: pending HITL request lists.
- Kilo logs under the data log directory.

The local logs are useful for runtime diagnosis, but session IDs should be captured from CLI JSON, session list/export, or server events rather than log parsing.

## Quirks and Gaps

Quirks:

- No `~/.kilo` state existed on this host; the actual Kilo data was under `/Users/ken/.local/share/kilo`.
- The active tool session HOME (`/Users/ken/.claudine`) had its own empty Kilo database. Research used `/Users/ken` for real session evidence.
- Current Kilo SDK/docs use `createKiloClient`; older OpenCode names are stale for Kilo integration.
- `--cloud-fork` cannot be combined with `--fork` or `--continue`, and requires `--session`.
- `--replay` / `--replay-limit` are interactive-only; they are not a non-interactive transcript replay mechanism.
- Direct SQLite parsing is unsupported and may break across Kilo versions.
- `kilo run --continue` can pick the wrong session for automation if the current scope contains multiple roots; prefer explicit IDs.

Gaps:

- Windows storage path remains inferred.
- Exact cloud retention beyond the 7-day inactive Cloud Agent beta statement is not documented.
- Approval restoration needs a live permission-prompt test before Claudine treats `session.permission` as a fully durable approval state.
- Concurrent prompt-submission semantics are not documented beyond busy guards on some operations.
- Live model execution of `kilo run --format json` was not performed in this research run; source proves the emitted JSON shape.

## Claudine Integration Notes

For a future Kilo provider wrapper, Claudine should capture `sessionID` as early as possible from `kilo run --format json`, `kilo session list --format json --all`, or server session creation. Store explicit IDs for lifecycle `resume`; treat `--continue` as a human convenience or fallback only.

For simple lifecycle `resume`, invoke:

```bash
kilo run --session "$SESSION_ID" "$FOLLOW_UP"
```

For HITL resume, do not depend on plain `kilo run`. Start or attach to a server, subscribe to `event.subscribe`, capture `question.asked` / `permission.asked`, route the prompt to the human, then reply through question/permission APIs. Claudine should serialize prompts per session and handle `SessionBusyError` as "same session already running" rather than blindly retrying.

For lifecycle `retry`, a fork can preserve the original session while testing a corrective prompt. For `proxy`, Kilo's `kilo run --attach` and HTTP API are better fits than parsing TUI output. For recovery after crash or Ctrl+C, use explicit `--session` when a session ID was captured; use `--continue` only when Claudine can tolerate scope/recency ambiguity.

## Changelog

- 2026-07-03: Refreshed against Kilo CLI 7.3.45 help, official docs, current public source commit `419ff008ef180dd7076f679a89442883ba8f8d86`, generated OpenAPI, and local state under `/Users/ken/.local/share/kilo`.
- 2026-07-03: Replaced older OpenCode-inherited SDK wording with Kilo-native `@kilocode/sdk` / `createKiloClient` APIs and exact current routes.
- 2026-07-03: Added local evidence from `kilo.db`, sanitized export, session-list JSON, storage directories, and actual transcript row shapes.
- 2026-07-03: Added current non-interactive permission behavior, headless child-session denial, `--auto`, `--dangerously-skip-permissions`, and interactive-only replay flags.
- 2026-07-03: Clarified worktree-aware lookup, cloud-fork validation, HITL APIs, and interruption limits for in-memory pending requests.

## Sources

- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo Code CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code Sessions & Sharing](https://kilo.ai/docs/collaborate/sessions-sharing)
- [Kilo Code CLI Runtime Architecture](https://kilo.ai/docs/contributing/architecture/cli-runtime)
- [Kilo Code Cloud Agent docs](https://kilo.ai/docs/code-with-ai/platforms/cloud-agent)
- [Kilo Code GitHub repository](https://github.com/Kilo-Org/kilocode)
- Local inspection: `kilo --version` (`7.3.45`), `kilo --help`, `kilo run --help`, `kilo session list --help`, `kilo debug paths`, `kilo session list --format json --all`, `kilo export --sanitize <sessionID>`, and `/Users/ken/.local/share/kilo/kilo.db`.
- Local source inspection: `/tmp/kilocode-research` at commit `419ff008ef180dd7076f679a89442883ba8f8d86`, especially `packages/opencode/src/cli/cmd/run.ts`, `packages/opencode/src/cli/cmd/session.ts`, `packages/opencode/src/session/session.ts`, `packages/opencode/src/session/session.sql.ts`, `packages/opencode/src/question/index.ts`, `packages/opencode/src/permission/index.ts`, `packages/opencode/src/kilocode/cloud-session.ts`, and `packages/sdk/openapi.json`.
