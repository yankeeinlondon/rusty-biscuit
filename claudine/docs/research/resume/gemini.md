---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://geminicli.com/docs/cli/session-management/
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "gemini --resume"
      - "gemini --resume latest"
      - "gemini --resume <index>"
      - "gemini --resume <session_id>"
      - "/resume session browser"
      - "/chat compatibility alias"
      - "/resume save|list|resume|delete <tag>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - index
      - picker
      - name
      - worktree
    notes: "Command-line resume accepts an optional positional query. The /resume browser is interactive and picker-oriented; manual chat checkpoints use names/tags. Worktree resume is supported by changing into the worktree and resuming by session ID."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "gemini -p '<prompt>' --resume"
      - "gemini -p '<prompt>' --resume latest"
      - "gemini -p '<prompt>' --resume <index>"
      - "gemini -p '<prompt>' --resume <session_id>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - index
    notes: "Headless mode is triggered by -p/--prompt or non-TTY execution. Follow-up output can be captured with --output-format json or stream-json."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No standalone local server resume surface is documented for normal CLI sessions."
  - mode: ide
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "The CLI has IDE integration commands, but no independently documented IDE resume API was verified for this research."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No public HTTP/SDK resume API is documented. Automation uses the CLI."
session_id_capture:
  - surface: json_stream
    field: session_id
    format: string
    notes: "Installed Gemini CLI 0.46.0 emits an init event with session_id and model when --output-format stream-json is active."
  - surface: hook
    field: session_id + transcript_path
    format: string + absolute path
    notes: "Hook stdin base input includes session_id, transcript_path, cwd, hook_event_name, and timestamp."
  - surface: session_file
    field: sessionId
    format: string
    notes: "Local transcript metadata records include sessionId, projectHash, startTime, lastUpdated, and kind. Current files are JSONL; older migrated files may be JSON."
  - surface: cli_command
    field: bracketed session ID
    format: "<index>. <preview> (<relative time>) [<session_id>]"
    notes: "gemini --list-sessions printed two sessions for this project, including full bracketed IDs."
  - surface: interactive_ui
    field: footer command
    format: "gemini --resume <session_id>"
    notes: "The installed TUI code prints a resume command in the exit footer; worktree sessions include a cd into the worktree before gemini --resume."
resume_invocations:
  - mode: both
    invocation: "gemini --resume"
    accepts_prompt: true
    selection: latest
    notes: "Bare --resume is coerced to latest by the installed CLI."
  - mode: both
    invocation: "gemini --resume latest"
    accepts_prompt: true
    selection: latest
    notes: "Loads the most recent session for the current project."
  - mode: both
    invocation: "gemini --resume 1"
    accepts_prompt: true
    selection: index
    notes: "The index is 1-based from gemini --list-sessions."
  - mode: both
    invocation: "gemini --resume a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    accepts_prompt: true
    selection: id
    notes: "Official docs and installed source support full session IDs."
  - mode: non_interactive
    invocation: "gemini -p 'continue with the next step' --resume <session_id> --output-format stream-json"
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up with structured JSONL lifecycle output."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the searchable Session Browser for the current project."
  - mode: interactive
    invocation: "/chat"
    accepts_prompt: false
    selection: picker
    notes: "Compatibility alias for the resume/chat checkpoint command family."
  - mode: interactive
    invocation: "/resume save <tag>"
    accepts_prompt: false
    selection: name
    notes: "Saves a named manual chat checkpoint."
  - mode: interactive
    invocation: "/resume resume <tag>"
    accepts_prompt: false
    selection: name
    notes: "Loads a named checkpoint as a branch point."
  - mode: interactive
    invocation: "/rewind"
    accepts_prompt: false
    selection: picker
    notes: "Interactive rewind can move conversation history, file changes, or both back to a prior interaction."
  - mode: interactive
    invocation: "/restore [tool_call_id]"
    accepts_prompt: false
    selection: other
    notes: "Automatic file checkpoint restore; requires general.checkpointing.enabled."
state_storage:
  - location: local
    os: macos
    path: "/Users/<user>/.gemini/tmp/<project_key>/chats/session-<timestamp>-<short_session_id>.jsonl"
    format: JSONL
    retention: "general.sessionRetention.enabled defaults true; maxAge defaults 30d; maxCount optional; minRetention defaults 1d."
    notes: "Observed on this host via ~/.gemini/tmp/claudine-2/chats/. Related per-project state includes .project_root, logs.json, tool-outputs/session-<session_id>/, plan/task files, and optional checkpoints/. Older local files may be .json."
  - location: local
    os: linux
    path: "/home/<user>/.gemini/tmp/<project_key>/chats/session-<timestamp>-<short_session_id>.jsonl"
    format: JSONL
    retention: "Same sessionRetention settings."
    notes: "Linux uses the same home-relative layout under ~/.gemini. The docs call <project_key> <project_hash>; current local storage can also use readable project keys with a .project_root marker."
  - location: local
    os: windows
    path: "C:\\Users\\<user>\\.gemini\\tmp\\<project_key>\\chats\\session-<timestamp>-<short_session_id>.jsonl"
    format: JSONL
    retention: "Same sessionRetention settings."
    notes: "Windows uses the user profile home directory. Worktree footer code quotes session IDs for PowerShell when needed."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: false
  branch_filtering: false
  notes: "Session lookup uses the current project temp directory. Official docs say switching directories switches session history. Worktree resume is documented as cd into the worktree, then gemini --resume <session_id>."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "/resume save <tag> then /resume resume <tag>"
  checkpoint_invocation: "/restore [tool_call_id] or /rewind"
  preserves_original: true
  notes: "Manual chat checkpoints are named branch points. Automatic file checkpoints are local shadow-Git snapshots plus checkpoint JSON under the project temp directory. Rewind is destructive for the current session history from the selected point."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: current_launch_dir
  env: current_process
  notes: "Resume converts saved messages to client history and calls resumeChat. Local transcripts preserve user/gemini turns, thoughts, token counts, tool calls, and tool results; separate tool output files remain on disk. Launch flags/settings still control model, approval mode, sandbox, MCP config, and environment. Resume must be launched in the intended project/worktree directory."
hitl_resume:
  supported: true
  question_capture: "BeforeTool hooks can match ask_user and receive tool_input.questions; hook base input also carries session_id and transcript_path."
  answer_injection: "No native deferred-answer API was found. A wrapper can deny/block ask_user, ask the human elsewhere, then run gemini -p '<answer>' --resume <session_id>."
  limitations: "This emulates continuation as a new user turn; it does not resume a suspended tool call. Notification hooks are advisory. ask_user itself is an interactive tool that pauses for answers in the TUI."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: false
  limitations: "Session files are written continuously and can be resumed after interruption. Pending approvals are not a documented persisted state; approval and sandbox behavior are recalculated from the resumed launch. Concurrent resume semantics are undocumented."
observability:
  stream_events:
    - init
    - message
    - tool_use
    - tool_result
    - error
    - result
  hook_events:
    - SessionStart
    - SessionEnd
    - BeforeAgent
    - AfterAgent
    - BeforeModel
    - AfterModel
    - BeforeToolSelection
    - BeforeTool
    - AfterTool
    - Notification
    - PreCompress
  failure_modes:
    - invalid session identifier
    - no sessions found
    - maxSessionTurns exceeded
    - headless general/API error
    - hook block or warning
  notes: "Useful surfaces are stream-json init, hook base input, gemini --list-sessions, local transcript metadata, logs.json, and the TUI exit footer."
quirks:
  - "The dedicated session docs moved from the older tutorial URL to /docs/cli/session-management/."
  - "Installed CLI help for --resume still mentions latest/index, but official docs and installed source also support full session IDs."
  - "Docs describe ~/.gemini/tmp/<project_hash>/, but this host's current files use readable project keys such as claudine-2 while transcript metadata still records a SHA-256 projectHash."
  - "Current local session files are JSONL; older local histories include .json files, and installed source scans both .json and .jsonl."
  - "The schema of local transcript files is internal. Direct parsing is useful evidence but should not be treated as a stable API."
  - "Manual chat checkpoints, automatic /restore checkpoints, and /rewind are related but distinct mechanisms."
  - "Memory files and GEMINI.md context are loaded into sessions but are not themselves resume."
gaps:
  - "Exact locking/concurrency behavior when two processes resume and append to the same session."
  - "Whether any 'always allow' approval choices persist outside normal settings/policy files."
  - "Whether MCP connection runtime state survives resume beyond reloading current settings and OAuth token files."
  - "Whether --session-file is safe for automation with arbitrary local transcript files; it is documented as loading JSON but the format is internal."
  - "Precise project key derivation for current readable directories versus metadata projectHash."
changes:
  - "2026-07-03: Updated docs URL to the current official session management page."
  - "2026-07-03: Verified installed Gemini CLI 0.46.0 and local ~/.gemini session files."
  - "2026-07-03: Corrected resume selectors to include full session IDs as first-class selectors."
  - "2026-07-03: Corrected state_storage to one schema-valid record per OS instead of os: all."
  - "2026-07-03: Added observed JSONL transcript shape, .json legacy compatibility, tool-outputs artifacts, and project key caveat."
  - "2026-07-03: Marked worktree resume as supported and documented as cd into worktree plus --resume <session_id>."
requires_claudine_update: true
reason: "Claudine should treat Gemini resume by full session ID as supported, capture stream-json session_id, store project cwd/worktree cwd for lookup, and avoid assuming ~/.gemini/tmp directories are always named by the metadata projectHash."
---

# Gemini CLI Resume Research

## Overview

Gemini CLI has first-class resume support backed by local transcript replay. Sessions are saved automatically while the CLI runs, can be resumed from the command line with `--resume`, and can be browsed through the interactive `/resume` Session Browser. The current official session docs describe resume by latest session, numeric index, and full session UUID; the installed Gemini CLI 0.46.0 source on this host implements the same selector set.

For Claudine, the main integration risks are lookup scope and state assumptions. Gemini session lookup is project-scoped and current-directory-sensitive, so a wrapper must preserve the launch project or worktree directory. Resume restores transcript context, tool calls, tool results, thoughts, and token records from local files, but launch-time settings still control model, approval, sandbox, MCP, and environment behavior. Treat local transcript parsing as diagnostic evidence, not as a stable integration API.

## Resume Semantics

A Gemini CLI session is a locally persisted conversation transcript plus related per-project artifacts. Official docs describe automatic saving of prompts, model responses, tool executions, token usage, and assistant thoughts/reasoning summaries. Local inspection on this host confirms that current session transcripts are JSONL files whose first record contains `sessionId`, `projectHash`, `startTime`, `lastUpdated`, and `kind`, followed by `$set` records and user/gemini message records. Gemini message records include fields such as `model`, `tokens`, `thoughts`, `toolCalls`, and tool result payloads.

Resume means transcript replay into a new CLI process, not live-process attach and not a remote server-side session. The installed CLI converts saved messages back into client history and calls `resumeChat`; new turns append to the resumed conversation. The applicable patterns are continue latest, resume by handle, interactive picker, non-interactive follow-up, transcript replay, manual chat checkpoints, rewind, and interruption recovery. Server-side session and live-process attach were not found.

Memory files such as `GEMINI.md`, private memory folders, and project instructions are context sources. They are not prior-session continuation mechanisms by themselves. A chat-history export is not resume unless Gemini CLI can load it through its session machinery, and the local transcript format is internal.

## Supported Modes

| Mode | Surface | Selector | Follow-up prompt | Automation fit |
|------|---------|----------|------------------|----------------|
| Interactive CLI | `gemini --resume`, `gemini -r` | latest, index, full session ID | Optional positional query | Scriptable launch, interactive TUI after launch |
| Interactive browser | `/resume` or `/chat` | picker | No | Human-oriented; do not automate as a stable API |
| Manual chat checkpoints | `/resume save`, `/resume list`, `/resume resume`, `/resume delete` | name/tag | No | Interactive command family |
| Non-interactive CLI | `gemini -p "<prompt>" --resume <selector>` | latest, index, full session ID | Yes | Best Claudine follow-up surface |
| Headless server/API | none verified | none | No | Unsupported |
| IDE | no resume-specific API verified | unknown | unknown | Out of scope unless future docs expose an API |

Sessions created in headless mode are normal sessions. The observed current-project `--list-sessions` output includes a recent session with a manually supplied non-UUID session ID, and installed source accepts alphanumeric, dash, and underscore session IDs for `--session-id`.

## Session ID Capture

The most automation-friendly handle is the session ID. It is available through several surfaces:

- `--output-format stream-json`: installed source emits an `init` event with `session_id` and `model` after any resumed chat history is loaded.
- Hooks: the hook base input schema includes `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp`.
- `gemini --list-sessions`: for this project it printed two entries as `index`, preview, relative age, and a bracketed session ID.
- Local transcript metadata: first JSONL record includes `sessionId`, `projectHash`, `startTime`, `lastUpdated`, and `kind`.
- TUI footer: installed source prints `gemini --resume <session_id>` on exit; worktree sessions include `cd <worktree> && gemini --resume <session_id>`.

The handle is available early in structured headless output through the `init` event. For a wrapper, that is preferable to polling local files. Hook capture is also stable enough when hooks are installed, but it couples resume support to hook configuration.

## Resume Invocation

Continue the latest session for the current project:

```bash
gemini --resume
gemini --resume latest
gemini -r latest "continue with the next step"
```

Resume an exact session:

```bash
gemini --list-sessions
gemini --resume 1
gemini --resume a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

Send a scriptable follow-up prompt and capture structured output:

```bash
gemini -p "continue with the next step" --resume a1b2c3d4-e5f6-7890-abcd-ef1234567890 --output-format stream-json
gemini -p "summarize the remaining work" --resume latest --output-format json
```

Resume from inside the TUI:

```text
/resume
/chat
```

Manual chat checkpoints:

```text
/resume save decision-point
/resume list
/resume resume decision-point
/resume delete decision-point
```

Checkpoint and rewind surfaces:

```text
/restore [tool_call_id]
/rewind
```

## Session Lookup Scope

Gemini session lookup is scoped to the current project. Official docs say sessions are stored under `~/.gemini/tmp/<project_hash>/chats/` and that switching directories switches session history. Local inspection in this workspace showed `~/.gemini/tmp/claudine-2/.project_root` containing `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`, and `gemini --list-sessions` in that directory listed only the two sessions for that project key.

Worktrees are explicitly resume-aware, but the lookup model is still directory-based: the docs say to `cd` into the worktree directory and run `gemini --resume <session_id>`. No all-projects resume flag or git-branch filter was verified.

## State Storage

Resumable state is local. On this macOS host, the effective shell home for this non-interactive session is `/Users/ken/.claudine`, whose `.gemini` entries symlink to `/Users/ken/.gemini`; the user-facing layout remains `~/.gemini`.

| OS | Primary transcript path | Format |
|----|-------------------------|--------|
| macOS | `/Users/<user>/.gemini/tmp/<project_key>/chats/session-<timestamp>-<short_session_id>.jsonl` | JSONL |
| Linux | `/home/<user>/.gemini/tmp/<project_key>/chats/session-<timestamp>-<short_session_id>.jsonl` | JSONL |
| Windows | `C:\Users\<user>\.gemini\tmp\<project_key>\chats\session-<timestamp>-<short_session_id>.jsonl` | JSONL |

Observed related paths for this project:

| Path | Purpose |
|------|---------|
| `~/.gemini/tmp/claudine-2/.project_root` | Project-root marker used for lookup and display |
| `~/.gemini/tmp/claudine-2/chats/session-2026-06-10T16-20-99967bca.jsonl` | Main session transcript |
| `~/.gemini/tmp/claudine-2/chats/session-2026-07-03T15-49-skills-l.jsonl` | Recent session with a custom non-UUID session ID |
| `~/.gemini/tmp/claudine-2/tool-outputs/session-99967bca-0948-4adf-b8db-da605e09b133/` | Large tool output artifacts |
| `~/.gemini/tmp/claudine-2/logs.json` | Per-project activity log records |
| `~/.gemini/history/<project_key>/` | Shadow Git storage for automatic file checkpoints when enabled |

The installed selector code scans both `.json` and `.jsonl`, and this host contains older `.json` session files in other project directories. Retention is controlled by `general.sessionRetention`: cleanup is enabled by default, `maxAge` defaults to `30d`, `maxCount` is optional, and `minRetention` defaults to `1d`.

## Restored State

Resume restores the conversation transcript and tool history by replaying saved messages into the Gemini client. Local transcript records preserve user turns, assistant turns, reasoning summaries/thoughts, model names for individual assistant records, token counts, tool calls, and tool results. Separate tool output files remain under the project temp directory and are referenced by transcript/tool records.

Resume does not prove that approval prompts, sandbox mode, model choice, MCP server runtime connection state, environment variables, or current working directory are restored from the original process. The installed code loads resumed message history, then uses the current launch configuration. Model selection is overridable with normal model flags/settings. Approval and sandbox behavior should be passed explicitly by Claudine when they matter. Resume must run from the original project or worktree directory so lookup and workspace context match.

Resume continues the selected session rather than creating a separate server-side session. The installed source deduplicates session list entries by `sessionId` and keeps the newest file for display, which matters because older local histories show multiple files with the same short session ID.

## Branching and Checkpoints

Gemini CLI has three separate continuation-adjacent mechanisms:

| Mechanism | Invocation | Preserves original? | Notes |
|-----------|------------|---------------------|-------|
| Manual chat checkpoint | `/resume save <tag>` then `/resume resume <tag>` | Yes | Named branch point inside a conversation |
| Automatic file checkpoint | `/restore [tool_call_id]` | Yes for the checkpoint; current files are reverted | Requires `general.checkpointing.enabled`; creates shadow Git snapshots under `~/.gemini/history/<project_key>` and checkpoint JSON under the project temp directory |
| Rewind | `/rewind` or Esc twice | No for current session history after the selected point | Interactive, destructive history/file rewind options |

The Session Browser also supports deleting sessions. Command-line deletion is available with `gemini --delete-session <index|id>`, and `/exit --delete` or `/quit --delete` deletes the current session history and temporary files on exit.

## Human-in-the-Loop Resume

Gemini CLI does not expose a native suspend-now-and-inject-answer-later API for pending questions or approvals. The closest supported primitive is hooks plus normal resume. A Claudine HITL broker can register a `BeforeTool` hook for `ask_user`, capture `tool_input.questions`, and return a denial/block so the tool does not execute. After collecting an answer elsewhere, Claudine can send it back as a new user turn with:

```bash
gemini -p "<answer>" --resume <session_id> --output-format stream-json
```

This is a continuation workaround, not a true pending-tool resume. The `ask_user` tool itself is explicitly interactive: it presents a dialog, pauses execution until the user answers or dismisses it, and returns answers to the model. Notification hooks are useful for observation but not for granting a pending approval.

## Interruption Recovery

Automatic session saving means crash, terminal close, Ctrl+C, and process kill generally leave a resumable transcript behind. Official docs describe automatic background saving even if a session is interrupted, and local JSONL files are written incrementally. Pending tool calls and tool results already written to the transcript are replayable context; separate tool output artifacts remain on disk until retention cleanup or deletion.

Pending approvals should not be treated as resumable state. If a process dies while a confirmation dialog is open, the next launch replays the transcript and uses current approval/sandbox settings. Network failure behavior is not documented as a mid-turn auto-resume feature. Concurrent resumes of the same session are not documented; a wrapper should serialize resumes per Gemini session ID to avoid interleaved appends.

## Observability

Resume-relevant observability surfaces are:

- `--output-format stream-json`: JSONL events include `init`, `message`, `tool_use`, `tool_result`, `error`, and `result`; `init` carries session metadata.
- Hook stdin: all hooks receive `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp`.
- `gemini --list-sessions`: project-scoped list with index, preview, age, and session ID.
- Local transcript metadata: `sessionId`, `projectHash`, `startTime`, `lastUpdated`, and `kind`.
- Local logs: `~/.gemini/tmp/<project_key>/logs.json` records activity with fields including `type`, `sessionId`, `timestamp`, `messageId`, and `message`.
- TUI footer: prints the command needed to resume the session, including worktree `cd` instructions when relevant.

## Quirks and Gaps

Quirks:

- The old tutorial URL still exists, but the best official session/resume reference is now `https://geminicli.com/docs/cli/session-management/`.
- Installed `gemini --help` for 0.46.0 is terse in this non-TTY environment and its `--resume` description mentions only latest/index, while official docs and installed source support full session IDs.
- Local storage on this host uses readable project keys such as `claudine-2`, not only the SHA-256-looking `projectHash` in transcript metadata.
- The local transcript format has changed over time: current files are JSONL, older files may be JSON, and source scans both.
- `--session-id` can start a new session with a non-UUID alphanumeric/dash/underscore ID; one local session uses `skills-list-session`.
- Manual chat checkpoints, automatic file checkpoints, and rewind are not interchangeable.

Gaps:

- Exact file locking and concurrency behavior for simultaneous resumes of one session.
- Whether any "always allow" approval decisions persist in settings or a separate cache.
- Whether MCP connection state survives beyond normal settings/token reload.
- Whether `--session-file` is a stable automation path for arbitrary transcript files.
- The exact algorithm mapping a project root to the observed project key directory.

## Claudine Integration Notes

For lifecycle `resume`, Claudine should store Gemini's `session_id` from the headless `init` event or hook payload and also store the launch cwd/worktree cwd. Resume should prefer:

```bash
gemini -p "<follow-up>" --resume <session_id> --output-format stream-json
```

Do not rely on numeric indexes unless Claudine just called `gemini --list-sessions` in the same project directory. Full session ID is the safest selector for exact-session resume. `latest` is useful for human convenience but unsafe for automation if multiple Gemini sessions run in the same project.

For `retry`, start a new Gemini process with the same cwd, model, approval, sandbox, MCP, and prompt settings; do not assume resume restores them. For `proxy`, hooks provide `session_id` and `transcript_path`, which are sufficient to correlate external user questions or blocks with a session. For future HITL recovery, intercept `ask_user` with `BeforeTool`, block/deny it, ask the human elsewhere, then resume by session ID with the answer as a new prompt. Claudine should serialize resume attempts per session ID.

## Changelog

- 2026-07-03: Updated the primary docs URL from the tutorial page to the current session management reference.
- 2026-07-03: Verified Gemini CLI 0.46.0 locally and inspected real `~/.gemini` session files for this workspace.
- 2026-07-03: Corrected explicit-handle resume from uncertain to supported for full session IDs.
- 2026-07-03: Corrected schema-invalid `state_storage.os: all` into separate macOS, Linux, and Windows records.
- 2026-07-03: Added observed JSONL transcript structure, `.json` legacy compatibility, separate tool output artifacts, and the project-key versus `projectHash` caveat.
- 2026-07-03: Added worktree-specific resume behavior and clarified that cwd/worktree cwd must be preserved by wrappers.

## Sources

- [Gemini CLI Session management](https://geminicli.com/docs/cli/session-management/)
- [Gemini CLI Headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI Hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI Hooks overview](https://geminicli.com/docs/hooks/)
- [Gemini CLI Command reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI Checkpointing](https://geminicli.com/docs/cli/checkpointing/)
- [Gemini CLI Rewind](https://geminicli.com/docs/cli/rewind/)
- [Gemini CLI Git worktrees](https://geminicli.com/docs/cli/git-worktrees/)
- [Gemini CLI Ask User tool](https://geminicli.com/docs/tools/ask-user/)
- [Gemini CLI repository](https://github.com/google-gemini/gemini-cli)
- Local inspection: Gemini CLI 0.46.0 installed at `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/`
- Local inspection: `~/.gemini/tmp/claudine-2/chats/session-2026-06-10T16-20-99967bca.jsonl`, `~/.gemini/tmp/claudine-2/chats/session-2026-07-03T15-49-skills-l.jsonl`, `~/.gemini/tmp/claudine-2/.project_root`, and `gemini --list-sessions`
