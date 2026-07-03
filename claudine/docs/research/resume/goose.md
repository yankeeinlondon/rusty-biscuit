---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://goose-docs.ai/docs/guides/sessions/session-management
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "goose session --resume"
      - "goose session --resume --name <name>"
      - "goose session --resume --session-id <id>"
      - "goose session --resume --fork"
      - "goose session --resume --edit"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - name
    notes: "No built-in picker for resume; omitting identifiers resumes the most recently updated session globally. --edit opens the conversation YAML in $EDITOR and can truncate history; --fork copies the original."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "goose run --resume -t \"follow-up prompt\""
      - "goose run --resume --name <name> -t \"follow-up prompt\""
      - "goose run --resume --session-id <id> -t \"follow-up prompt\""
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
    notes: "run requires -i, -t, or --recipe. --resume loads the prior session and the supplied text is appended as a new user message."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "goosed and goose serve exist for ACP/desktop, but there is no documented standalone CLI/API surface for programmatic session resume."
  - mode: ide
    supported: true
    mechanisms:
      - "ACP clients (Zed, VS Code) resume via session history"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
    notes: "ACP sessions are saved to the same SQLite store; this research focuses on CLI behavior."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No direct public HTTP API for session resume; ACP JSON-RPC is the closest programmatic surface."
session_id_capture:
  - surface: stdout
    field: "session_id"
    format: "YYYYMMDD_<N>"
    notes: "Printed in the ASCII-art session header at start unless --quiet is used. Not emitted inside --output-format json or stream-json events."
  - surface: cli_command
    field: "id"
    format: "YYYYMMDD_<N>"
    notes: "goose session list --format json returns Session objects with id, name, working_dir, created_at, updated_at."
  - surface: session_file
    field: "id"
    format: "YYYYMMDD_<N>"
    notes: "Primary key in the local SQLite sessions table."
  - surface: log_file
    field: "AGENT_SESSION_ID"
    format: "YYYYMMDD_<N>"
    notes: "Set in the environment of shell tool subprocesses and stdio extensions; not exposed to the wrapper process."
resume_invocations:
  - mode: interactive
    invocation: "goose session --resume"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recently updated session globally."
  - mode: interactive
    invocation: "goose session --resume --name <name>"
    accepts_prompt: false
    selection: name
    notes: "Names match the user-provided or AI-generated session name; --name also accepts a session ID."
  - mode: interactive
    invocation: "goose session --resume --session-id <id>"
    accepts_prompt: false
    selection: id
    notes: "Exact YYYYMMDD_<N> ID."
  - mode: interactive
    invocation: "goose session --resume --fork [--name <name>|--session-id <id>]"
    accepts_prompt: false
    selection: latest
    notes: "Creates a new session copy before resuming; the original is left intact."
  - mode: interactive
    invocation: "goose session --resume --edit [--fork]"
    accepts_prompt: false
    selection: id
    notes: "Opens conversation YAML in $VISUAL/$EDITOR/vi; save to truncate or rewrite history, then continue."
  - mode: non_interactive
    invocation: "goose run --resume -t \"follow-up prompt\""
    accepts_prompt: true
    selection: latest
    notes: "Appends the text as a new user message to the latest session."
  - mode: non_interactive
    invocation: "goose run --resume --name <name> -t \"follow-up prompt\""
    accepts_prompt: true
    selection: name
    notes: "Resumes the named session and appends the prompt."
  - mode: non_interactive
    invocation: "goose run --resume --session-id <id> -t \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Resumes the exact session and appends the prompt."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/goose/sessions/sessions.db"
    format: SQLite
    retention: "Indefinite until deleted with goose session remove; system logs are cleaned after two weeks."
    notes: "sessions table stores metadata, provider_name, model_config_json, extension_data, recipe_json, goose_mode, working_dir; messages table stores the conversation."
  - location: local
    os: linux
    path: "~/.local/share/goose/sessions/sessions.db"
    format: SQLite
    retention: "Indefinite until deleted; system logs cleaned after two weeks."
    notes: "Same schema as macOS."
  - location: local
    os: windows
    path: "%APPDATA%\\Block\\goose\\data\\sessions\\sessions.db"
    format: SQLite
    retention: "Indefinite until deleted; system logs cleaned after two weeks."
    notes: "Same schema as Unix. Legacy individual .jsonl files under the sessions directory are imported once and then ignored."
resume_scope:
  project_scoped: false
  cwd_scoped: false
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Sessions are global across working directories. goose session list -w <path> can filter by working_dir for discovery, but resume without an identifier always picks the most recently updated session globally."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "goose session --resume --fork [--name <name>|--session-id <id>]"
  checkpoint_invocation: "n/a"
  preserves_original: true
  notes: "Fork duplicates the full session. --edit can truncate the original conversation in place. There is no named rewind/checkpoint command beyond /compact (summarization)."
restored_state:
  transcript: true
  tool_results: true
  approvals: cleared
  model: restored
  cwd: configurable
  env: current_process
  notes: "Provider, model config, enabled extensions, recipe, and goose_mode are read from the sessions table. CLI --provider/--model can override model. Working directory is only switched back in interactive mode with a confirmation prompt; non-interactive runs stay in the launching directory. Environment is never stored."
hitl_resume:
  supported: false
  question_capture: "n/a"
  answer_injection: "n/a"
  limitations: "Goose has no defer/return-channel hook. In non-interactive mode, approve/smart_approve modes error out when a tool confirmation is needed, and MCP elicitation errors out. Automation must use GOOSE_MODE=auto to avoid blocking prompts."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: false
  limitations: "Messages are persisted to SQLite as they are produced, so the session can be resumed after most interruptions. An interrupted assistant turn may be incomplete. In-flight tool calls and mid-turn approvals are not stored."
observability:
  stream_events:
    - "message"
    - "notification"
    - "model_change"
    - "error"
    - "complete"
  hook_events:
    - "GOOSE_STATUS_HOOK (waiting/thinking)"
  failure_modes:
    - "Tool approval required in non-interactive mode"
    - "Elicitation requested in non-interactive mode"
  notes: "stream-json is only available for goose run. The status hook is fire-and-forget with suppressed stdout/stderr and no return channel. Session IDs are not embedded in stream-json events; capture them from the session list or start header."
quirks:
  - "Session IDs use YYYYMMDD_<N> (e.g. 20251108_2), not UUIDs."
  - "--resume without an identifier resumes the most recently updated session globally, not the latest session in the current directory."
  - "--name also matches session IDs, so a name lookup can accidentally resolve to an ID."
  - "Non-interactive resumed runs cannot answer tool confirmations or MCP elicitation forms; use GOOSE_MODE=auto for headless automation."
  - "Working directory is not automatically restored in non-interactive resume; the wrapper must cd first if desired."
  - "--output-format stream-json does not include the session_id field; do not rely on it for stable handles."
  - "GOOSE_DISABLE_SESSION_NAMING avoids the AI-generated name call and keeps the default 'CLI Session' name."
  - "The SQLite schema is internal and may change between releases; direct SQL is possible but not a supported integration surface."
gaps:
  - "No documented concurrency guarantees when the same session is resumed from multiple processes."
  - "Whether MCP OAuth tokens, connection state, and extension runtime state survive resume is not documented."
  - "Exact behavior when resuming a session that was interrupted mid-assistant-turn is not documented."
  - "Whether goose run --resume supports --recipe follow-up or resuming from a recipe session is not documented."
  - "No documented standalone headless API for session resume outside ACP clients."
changes:
  - "2026-07-02: Replaced free-form frontmatter with schema-validated fields and corrected continuity/HITL claims."
  - "2026-07-02: Documented SQLite session storage, OS-specific paths, and restored-state semantics."
  - "2026-07-02: Clarified that Goose has no defer/answer-injection hook and that non-interactive approve/smart_approve modes fail on tool confirmation."
  - "2026-07-02: Added resume-scope notes (global, not cwd-scoped) and branching via --fork."
requires_claudine_update: true
reason: "The schema migration and corrected findings should update Claudine's Goose adapter: resume must use YYYYMMDD_<N> handles captured from goose session list or the start header, HITL must rely on GOOSE_MODE=auto because there is no defer hook, and cwd/model restore semantics need to be accounted for."
---

## Overview

Goose CLI stores every session in a local SQLite database and provides first-class resume for both interactive and non-interactive use. A session is a persisted conversation plus metadata (provider, model, extensions, mode, working directory). Resume loads that persisted state into a new process; there is no live-process attach or remote session server for CLI wrappers.

## Resume Semantics

Resume means "load a previously persisted session from SQLite and continue appending messages to it." The same database is shared by Goose CLI, Goose Desktop, and ACP clients. A resumed session keeps the same `YYYYMMDD_<N>` ID unless it is forked, in which case a new session record is created with copied history.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `goose session --resume` | No | Latest globally |
| Interactive CLI | `goose session --resume --name <name>` | No | Exact name |
| Interactive CLI | `goose session --resume --session-id <id>` | No | Exact ID |
| Interactive CLI | `goose session --resume --fork` | No | Latest / name / ID |
| Non-interactive CLI | `goose run --resume -t "..."` | Yes | Latest globally |
| Non-interactive CLI | `goose run --resume --name <name> -t "..."` | Yes | Exact name |
| Non-interactive CLI | `goose run --resume --session-id <id> -t "..."` | Yes | Exact ID |

`goose run --resume` also accepts `--instructions` and `--recipe`, but the interaction between `--resume` and `--recipe` is not explicitly documented.

## Session ID Capture

Stable handles are `YYYYMMDD_<N>` strings. Capture surfaces:

- **Start header**: printed to stdout when a session starts (suppressed by `--quiet`).
- **`goose session list --format json`**: returns an array of `Session` objects with `id`, `name`, `working_dir`, `created_at`, `updated_at`.
- **SQLite primary key**: the `id` column in `sessions`.
- **`AGENT_SESSION_ID`**: set in shell tool and stdio extension subprocesses, not in the wrapper process.

Stream-json events do **not** contain `session_id`, so wrappers must capture the ID before or after the run.

## Resume Invocation

Continue the latest session interactively:

```bash
goose session --resume
```

Resume a specific session by name or ID:

```bash
goose session --resume --name react-migration
goose session --resume --session-id 20251108_2
```

Fork while resuming:

```bash
goose session --resume --fork --name react-migration
goose session --resume --fork --session-id 20251108_2
```

Send a follow-up prompt into a prior non-interactive session:

```bash
goose run --resume --session-id 20251108_2 -t "run the tests now"
```

## Session Lookup Scope

Sessions are global. `goose session -r` without an identifier resumes the most recently updated session across all working directories. Use `goose session list -w <path>` to filter discovery by working directory, but resume itself does not scope to the current project, repository, worktree, or branch.

## State Storage

Resumable state is local SQLite.

| OS | Path |
|----|------|
| macOS / Linux | `~/.local/share/goose/sessions/sessions.db` |
| Windows | `%APPDATA%\Block\goose\data\sessions\sessions.db` |

The `sessions` table holds metadata, provider, model config, extension data, recipe, goose mode, and working directory. The `messages` table holds the full conversation including tool requests and responses. Legacy `.jsonl` files remain on disk after migration but are no longer managed.

## Restored State

Resuming restores:

- The full conversation transcript and tool results from the `messages` table.
- Provider and model configuration from the session record, unless overridden by `--provider` / `--model`.
- Enabled extensions from the saved `extension_data`.
- Recipe configuration if the session was recipe-based.
- `goose_mode` from the saved session.

Resuming does **not** restore:

- The environment of the original session.
- The working directory, in non-interactive mode (interactive mode asks whether to switch back).
- Individual tool approvals; each tool call is gated fresh by the active mode.

## Branching and Checkpoints

Branching is supported via `--fork` on resume; it creates a new session record with copied history and leaves the original intact. Checkpoints/rewind are not supported as named operations. `--edit` can truncate the conversation in place by editing the YAML in an external editor.

## Human-in-the-Loop Resume

Goose does **not** support Claudine-style HITL resume. There is no hook to defer a tool call or question, capture it, and later inject an answer. In non-interactive mode:

- `approve` and `smart_approve` modes fail when a tool confirmation is required.
- MCP elicitation forms fail because there is no interactive terminal.

For automation, set `GOOSE_MODE=auto` so that tool calls proceed without blocking.

## Interruption Recovery

Because messages are written to SQLite as they are produced, sessions survive:

- Crash, terminal close, or process kill.
- `Ctrl+C` during a response.

There is no documented recovery of:

- An in-flight assistant turn that was interrupted before a message was persisted.
- Pending tool calls or mid-turn approvals.

## Observability

Relevant surfaces for resume-aware wrappers:

- **`--output-format stream-json`**: emits `message`, `notification`, `model_change`, `error`, and `complete` events. No `session_id` in these events.
- **`--output-format json`**: returns final `messages` and `metadata` after `goose run` completes.
- **`GOOSE_STATUS_HOOK`**: fire-and-forget shell hook receiving `waiting` or `thinking`; cannot block or return data.
- **`goose session list --format json`**: the stable way to enumerate and select sessions.

## Quirks and Gaps

Quirks:

- Session IDs are `YYYYMMDD_<N>`, not UUIDs.
- `--resume` without identifiers is global, not cwd-scoped.
- `--name` on resume also matches session IDs.
- Non-interactive resume cannot handle confirmations or elicitation; use `auto` mode.
- Working directory is only restored interactively.
- Direct SQLite queries are possible but the schema is internal.

Gaps:

- Concurrent resume behavior is undocumented.
- Survival of MCP OAuth / connection state on resume is undocumented.
- Mid-turn interruption recovery semantics are undocumented.
- Resume with `--recipe` is undocumented.
- No documented standalone API for programmatic resume.

## Claudine Integration Notes

- Capture the session ID from the start header or from `goose session list --format json`; do not rely on stream-json events.
- Use `goose run --resume --session-id <id> -t "..."` for non-interactive continuation.
- Do not implement HITL defer/answer injection for Goose; instead ensure `GOOSE_MODE=auto` for headless runs.
- When resuming after failure, be aware that cwd is not restored automatically in non-interactive mode; cd to the original `working_dir` first if the task depends on it.
- Treat the SQLite database as read-only and unstable; use `goose session export` for stable backups.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter and corrected HITL/storage claims.
- 2026-07-02: Added SQLite paths, OS-specific storage, restored-state semantics, and `--fork` branching.
- 2026-07-02: Documented that Goose has no defer hook and that `approve`/`smart_approve` modes fail in non-interactive runs.

## Sources

- [Goose session management guide](https://goose-docs.ai/docs/guides/sessions/session-management)
- [Goose CLI commands reference](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Goose logging and session storage](https://goose-docs.ai/docs/guides/logs)
- [Running tasks with Goose](https://goose-docs.ai/docs/guides/running-tasks)
- [Goose permission modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions)
- [Goose environment variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Goose hooks and event stream research](/claudine/docs/research/hooks/goose.md)
- [Goose source: `crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Goose source: `crates/goose-cli/src/session/builder.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/builder.rs)
- [Goose source: `crates/goose/src/session/session_manager.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/session/session_manager.rs)
