---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Every Agentic CLI writes logs of its sessions — conversation transcripts, tool calls, token usage, errors — but each provider chooses its own log surfaces, file locations, record formats, retention behavior, and time semantics (units and zones). Claudine wraps these providers and also writes its own JSONL logs with a `claudine logs` reporting layer, so understanding each provider's logging is essential for session reconstruction, usage analysis, and debugging wrapped runs.

  ## Task

  Your task is to report on agent logging across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why provider logs matter to a wrapper like Claudine (debugging, usage/cost accounting, session reconstruction, auditing)
  - and then shift its focus to how each provider's logging differs: where logs live, what record types exist, what formats they use, and any time-semantics gotchas (units, zones, precision)
  - close with a point of view on how Claudine could leverage provider logs alongside its own JSONL/SQLite reporting

  As background material we have agent-logging research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/agent-logging/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/agent-logging.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/agent-logging.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: 554da4c67eb6c57d-53183e3a6013ac1c
last_updated: 2026-07-03
---
# Agent Logging Across Claudine Providers

Provider logs matter to Claudine because a wrapper only sees part of a run. Claudine can observe the process it launches, normalize lifecycle events, stream provider output, and write its own JSONL/SQLite reporting data, but each provider also leaves behind native evidence: transcripts, state databases, debug logs, telemetry files, prompt history, subagent records, tool-output spill files, and desktop-side logs. Those native records are essential for four wrapper-grade jobs:

- **Debugging:** Provider logs preserve failures that may happen before, after, or outside Claudine's stream parser: auth failures, hook loading, MCP startup, telemetry errors, daemon/server failures, desktop sidecar problems, and provider retries.
- **Usage and cost accounting:** Some providers emit token and cost data in live streams, some write it into transcript messages, some denormalize it into SQLite, and some put request-level details in separate LLM request logs. Claudine should not assume one common usage surface.
- **Session reconstruction:** A wrapped run may need to be reconstructed after process death, compaction, resume, subagent spawn, branch/fork creation, or background task completion. Provider transcripts and indexes often contain the canonical session graph.
- **Auditing:** Native logs are the provider's own record of prompts, tool calls, permission decisions, model responses, retries, file snapshots, and sometimes cloud-share/export events. Claudine's logs are the wrapper's view; provider logs are corroborating evidence.

The main conclusion from the research is that "agent logs" are not one category. Each provider chooses different storage roots, formats, schemas, retention behavior, and timestamp conventions. Claudine should treat provider logs as provider-specific evidence sources that enrich its normalized ledger, not as interchangeable JSONL files.

## Provider Comparison

| Provider    | Primary Conversation Store                                                         | Operational Logs                                             | Native SQLite                   | Notable Time Gotchas                                                                                                  |
|-------------|------------------------------------------------------------------------------------|--------------------------------------------------------------|---------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| Claude Code | `~/.claude/projects/{sanitized_cwd}/{session_id}.jsonl` plus subagent JSONL        | Desktop text logs under `~/Library/Logs/Claude/`             | No                              | Transcript timestamps are ISO-8601 UTC; `history.jsonl` uses Unix millis; desktop logs use local time                 |
| Codex CLI   | `~/.codex/sessions/YYYY/MM/DD/rollout-{local_ts}-{session_id}.jsonl`               | `logs_2.sqlite`, optional text logs, login log, desktop logs | Yes                             | Filename timestamp is local; JSONL event timestamps are UTC; SQLite has Unix seconds plus nanosecond component fields |
| Gemini CLI  | `~/.gemini/tmp/{project_id}/chats/session-{utc_ts}-{id}.jsonl`                     | Optional debug/telemetry output; Antigravity desktop logs    | CLI no; Antigravity desktop yes | Session filenames and transcript timestamps are UTC; Antigravity Go logs use local time                               |
| Goose       | Shared `sessions.db`                                                               | JSON tracing logs and rotating `llm_request.0..9.jsonl`      | Yes                             | DB timestamps are UTC; CLI/server log filenames use local time; tracing JSON timestamps are effectively UTC           |
| Kimi Code   | `~/.kimi/sessions/{md5_cwd}/{session_id}/wire.jsonl` plus context/state/task files | `~/.kimi/logs/kimi.log` and rotated prior-run logs           | No                              | Wire timestamps are Unix seconds UTC; Loguru text logs and filenames use local time                                   |
| OpenCode    | `opencode.db` tables `session`, `message`, `part`                                  | `log/opencode.log` logfmt                                    | Yes                             | DB times are Unix millis UTC; current logfmt timestamps are ISO-8601 UTC; legacy log filenames/prefixes are local     |
| Kilo Code   | Current CLI: `~/.local/share/kilo/kilo.db`; IDE extension: Roo-style task JSON     | CLI per-process logs under `~/.local/share/kilo/log/`        | CLI yes; IDE extension no       | CLI DB times are Unix millis UTC; CLI log filenames/prefixes are UTC; prompt history has no timestamp                 |
| Pi          | `~/.pi/agent/sessions/--{sanitized_cwd}--/{ISO8601Z}_{session_id}.jsonl`           | Opt-in stdout NDJSON via `pi --mode json` / `--mode rpc`     | No                              | Transcript entry timestamps are ISO-8601 UTC; nested message timestamps are Unix millis; live events lack timestamps  |
| Qwen CLI    | `~/.qwen/projects/{sanitized_cwd}/chats/{session_id}.jsonl`                        | `~/.qwen/debug/{session_id}.txt`, optional OTel output       | No                              | Transcript/debug timestamps are ISO-8601 UTC; filenames are UUIDs with no embedded time; tip history uses Unix millis |

Roo is part of Claudine's provider roster, but there is no current `agent-logging/roo*.md` research document in the provided directory. That should remain an explicit coverage gap rather than being inferred from adjacent providers. Pi and Kilo are included because the agent-logging research roster includes them, even where the compiled provider set may trail the research set.

## Claude Code

Claude Code is file-based. Its primary audit trail is a per-session JSONL transcript under `~/.claude/projects/{sanitized_cwd}/`. Subagents get their own JSONL files under a sibling session directory, and prompt recall lives in a global `~/.claude/history.jsonl`.

The important distinction is between **on-disk transcript JSONL** and **stream-json SDK output**. They overlap, but they are not the same schema. On-disk transcripts include interactive bookkeeping such as `mode`, `permission-mode`, `ai-title`, `last-prompt`, `queue-operation`, attachments, and `file-history-snapshot`. Stream-json output carries live SDK events such as `result`, rate-limit events, tool progress, and auth/status events that are not written as the transcript's terminal record. Usage is found inline in assistant message usage, not as a separate transcript result event.

Claude has no native SQLite database. It uses JSONL, JSON metadata, file-history snapshots, task lock directories, shell snapshots, and desktop text logs. Timestamps are mixed: transcript records are ISO-8601 UTC, `history.jsonl` and live session metadata use Unix millis, and desktop logs/statusline-script output can be local time.

For Claudine, Claude logs are useful for reconstructing interactive-only events and subagent sidechains that may not appear in the live wrapper stream. Claudine should not treat the SDK stream schema as the on-disk transcript schema.

## Codex CLI

Codex is the most multi-surface provider in the current research. It writes per-session rollout JSONL under a date-sharded tree, global history/index files, several SQLite databases, desktop logs, update metadata, shell snapshots, and optional text tracing.

The rollout transcript is a `{timestamp,type,payload}` JSONL envelope with top-level types such as `session_meta`, `turn_context`, `response_item`, `event_msg`, and `compacted`. Its `event_msg` vocabulary includes task lifecycle, token counts, user/agent messages, command completion, web search completion, compaction, review mode, collaboration/subagent events, errors, and thread-name updates. Subagent sessions are represented as additional rollout files and can be correlated through state database edges and parent thread IDs.

Codex also writes SQLite stores:

- `logs_2.sqlite` for Rust tracing logs.
- `state_5.sqlite` for threads, dynamic tools, spawn edges, jobs, remote control, and backfill state.
- `goals_1.sqlite` for thread goals and budget/usage status.
- `memories_1.sqlite` for memory pipeline state.
- `sqlite/codex-dev.db` for app-server and desktop features such as inbox items and automations.

These databases use WAL mode where noted, so Claudine must never copy or symlink them while Codex is live. Timestamp semantics vary sharply: rollout filenames embed local time, rollout JSON fields are UTC, history uses Unix seconds, SQLite tracing uses seconds plus a nanosecond component, and some state tables include both seconds and millis mirrors.

For Claudine, Codex provider logs can enrich usage/cost accounting, thread and subagent graphs, compaction history, and error classification. The wrapper should keep its normalized JSONL/SQLite layer as the canonical Claudine view, but a Codex-specific ingest path could add rich historical context.

## Gemini CLI

Gemini CLI is mostly flat-file based. The current transcript format is append-only JSONL under `~/.gemini/tmp/{project_id}/chats/`. A transcript starts with a header line, then message lines, interleaved with `$set` patch records that update metadata such as `lastUpdated`. Older sessions may still exist as single pretty-printed JSON objects.

The per-project `logs.json` is not a full session log; it indexes user messages only. Tool outputs may spill to `tool-outputs/session-{session_id}/`, and plans can be written under a session `plans/` directory. The current Node CLI does not use SQLite, but the Antigravity desktop counterpart does: it writes SQLite conversation databases, protobuf state, and Go glog-style text logs under separate Antigravity paths.

Gemini's main time model is cleaner than several providers: transcript filename timestamps, header `startTime`, message timestamps, thoughts, tool calls, and `logs.json` entries are ISO-8601 UTC. Antigravity desktop logs use local time.

For Claudine, the biggest parsing gotcha is the JSONL shape: line 1 is a header, `$set` lines are patches, user content is an array of parts, and Gemini response content is a string. A naive "every line is a message" parser will be wrong.

## Goose

Goose is DB-centric for conversations and tracing-centric for operations. The authoritative conversation store is a shared `sessions.db` used by both CLI and Desktop. It contains sessions, messages, threads, schema versioning, and provider inventory tables. The database is WAL-mode SQLite with an explicit `schema_version` table.

Operational logs are JSONL tracing files split by component and local-date directory: CLI logs, server logs, and a rotating LLM request log family. The LLM request logs keep the ten most recent completed requests as `llm_request.0.jsonl` through `llm_request.9.jsonl`; in-flight requests are written to temporary UUID-named files before being rotated into place. Desktop has its own plain text `main.log`, but conversation content is in the shared DB.

Goose time semantics cross storage boundaries: DB timestamps are UTC, message created timestamps are Unix seconds with reader tolerance for millis, CLI/server log filenames and date directories use local time, and tracing JSON timestamps are UTC-like system-time output.

For Claudine, Goose's provider logs are excellent for post-run session and message reconstruction, but they require SQLite-aware ingestion with WAL discipline. The current Claudine live integration does not require immediate Goose log ingestion, but future historical reporting should read via a safe snapshot/read-only strategy.

## Kimi Code

Kimi is file-based and built around its Wire protocol. The primary session record is `wire.jsonl` under `~/.kimi/sessions/{md5_cwd}/{session_id}/`. The first line is metadata with a protocol version, then event records such as turn boundaries, steps, content parts, tool calls/results, notifications, compaction events, status updates, subagent events, plans, hooks, MCP loading events, approvals, and background task markers.

Kimi also writes an LLM context file, compaction snapshots, session state, subagent directories, plan files, task runtime directories, user history, app logs, and telemetry fallback files. User history is keyed by MD5 of cwd and contains prompt content only, with no timestamp and no session ID. Background task directories are especially important: they contain `spec.json`, `runtime.json`, `control.json`, `consumer.json`, and live `output.log` files.

Kimi uses Unix seconds UTC for Wire records, task state, telemetry fallback, and session state timestamps. Its Loguru app log line prefixes and rotated filenames use local time. Protocol versioning is explicit: observed sessions used protocol `1.9`, while current source defines `1.10`, which matters for strict parsers.

For Claudine, Kimi logs are useful because the persisted `wire.jsonl` is a richer historical audit trail than a generic text log. But Claudine should handle Wire protocol versions deliberately and should distinguish persisted Event records from live Wire Request records that are not stored as top-level transcript entries.

## OpenCode

OpenCode is strongly SQLite-centric. Conversation content is not stored as JSONL files; it lives in `opencode.db`, primarily in `session`, `message`, and `part` tables. Subagents are child sessions in the same database. The `part.data` JSON discriminator includes values such as `tool`, `step-start`, `step-finish`, `reasoning`, `text`, `patch`, `file`, `snapshot`, `compaction`, `subtask`, and `agent`.

The session table now denormalizes cost and token columns, which makes historical usage accounting more direct than parsing every message JSON blob. The same database also holds non-transcript state such as accounts, credentials, permissions, todos, projects, workspaces, event streams, shares, and migrations. It runs in WAL mode and must be treated as live-locked.

OpenCode's diagnostic log changed format. The current `log/opencode.log` is a single append-only logfmt file with ISO-8601 UTC timestamps, levels, run IDs, messages, and key/value context. Older Effect-style per-invocation logs may still exist and use local timestamps. The desktop app drives the same CLI server sidecar and shares the same DB/log footprint, with separate UI state files outside the main CLI data store.

For Claudine, OpenCode is the clearest case where historical provider reporting requires a SQLite reader. JSONL-only ingestion will miss the real transcript, cost, token, and subagent data.

## Kilo Code

Kilo is a special case because its current CLI and IDE extension come from different lineages. The current `kilo` CLI is an OpenCode fork and writes OpenCode-style SQLite state under XDG paths such as `~/.local/share/kilo`, even on macOS. The VS Code and JetBrains extensions are Roo Code forks and use Roo-style per-task JSON files under IDE `globalStorage`. Legacy `@kilocode/cli` data may also exist under `~/.kilocode/cli`.

For the current CLI, the authoritative conversation store is `~/.local/share/kilo/kilo.db`, a WAL-mode SQLite database. Sessions live in the `session` table, turns in `message`, and content blocks in `part`; subagents are child sessions with `session.parent_id` set, not separate transcript files. The same database also stores projects, workspaces, todos, permissions, accounts, events, and migration metadata. A separate WAL-mode `session-export.db` queues session-share events for Kilo Cloud.

Kilo's app logs are per-process text files under `~/.local/share/kilo/log/{YYYY-MM-DDTHHMMSS}.log`. Lines include a level, UTC timestamp, elapsed-millis marker, `service=` tag, and sometimes internal bus event names such as `session.created`, `session.turn.open`, `message.part.updated`, `permission.asked`, and `command.executed`. Prompt history lives in `~/.local/state/kilo/prompt-history.jsonl`, but it has no timestamp and no session ID.

The IDE extension surface is completely different: Roo-style task directories contain `api_conversation_history.json`, `ui_messages.json`, task metadata, checkpoints, and an `_index.json`. Its UI messages use Roo's `ask`/`say` model rather than the CLI database schema.

For Claudine, Kilo should not be inferred as "just OpenCode" or "just Roo." Supporting it requires a distinct provider identity. The current CLI can reuse OpenCode-shaped SQLite ingestion concepts, while the IDE extension follows the Roo task-file pattern. Any reader must keep those two products separate.

## Pi

Pi is file-based and deliberately minimal. Its primary audit trail is an append-only JSONL transcript under `~/.pi/agent/sessions/--{sanitized_cwd}--/`. Each file starts with a `session` header and then stores tree-linked entries using `id` and `parentId`, which lets Pi represent in-place branching without creating separate branch files. `/fork` and `/clone` create new files with a `parentSession` pointer; ordinary `/tree` branching stays in the same file.

Pi has an important split between persisted transcripts and live events. The transcript stores durable records such as `message`, `model_change`, `thinking_level_change`, `compaction`, `branch_summary`, `custom`, `custom_message`, `label`, and `session_info`. The live stream from `pi --mode json`, `pi --mode rpc`, or `AgentSession.subscribe()` adds transient lifecycle events such as `agent_start`, `turn_start`, `message_update`, `tool_execution_update`, `queue_update`, `compaction_start`, and `auto_retry_start`, but Pi does not persist that stream itself.

There is no native SQLite database. Pi persists transcripts as JSONL and config/state as JSON under `~/.pi/agent/`, with timestamped `{settings,models}.{unix_seconds}.bak` backups on config mutation. Session listing is filesystem scanning, not index lookup.

Pi's main timestamp gotcha is that transcript entries have two timestamp conventions: entry-level `timestamp` fields are ISO-8601 UTC strings, while nested `message.timestamp` fields are Unix epoch milliseconds. Transcript filenames also embed an ISO-8601 UTC timestamp with colons replaced by dashes. Live `AgentSessionEvent` records have no per-event timestamp; only the initial session header carries time.

For Claudine, Pi would be a clean JSONL evidence adapter for historical transcript reconstruction, but live observability would require capturing stdout NDJSON during the wrapped run. A post-run parser should preserve the transcript tree shape instead of flattening entries into a simple chronological list.

## Qwen CLI

Qwen is file-based and resembles Gemini in some areas, but its current logging surface has its own structure. Primary transcripts are JSONL files under `~/.qwen/projects/{sanitized_cwd}/chats/`. Each record is a linked-list-style object with fields such as `uuid`, `parentUuid`, `sessionId`, `timestamp`, `type`, `cwd`, `version`, `gitBranch`, and optional system payloads. Observed transcript types include `system`, `user`, `assistant`, `tool`, and `result`.

Qwen also writes a per-project `logs.json`, per-session structured debug text logs under `~/.qwen/debug/`, optional OpenTelemetry output, OTel diagnostics under `~/.qwen/log/`, project metadata, file-history snapshots, settings, memory files, tip history, and statusline payloads. There is no native SQLite.

The debug log is a valuable diagnostic surface: lines carry ISO-8601 UTC timestamps, levels, and component tags such as preconnect, theme detection, skill managers, hook registries, command loaders, and message bus components. OpenTelemetry adds another layer when enabled, including logs, metrics, and distributed-tracing spans. Transcript and debug timestamps are UTC; filenames are UUIDs without embedded time; tip history uses Unix millis.

For Claudine, Qwen's `result` stream event remains the best live carrier for duration, usage, error state, and final text, while the transcript and debug files provide post-run audit/debug evidence. Claudine should not search for a native Qwen database.

## Cross-Provider Patterns

The providers fall into several storage families:

- **File-transcript providers:** Claude, Gemini, Kimi, Pi, and Qwen primarily use JSONL/JSON/text files for transcripts and state.
- **SQLite-primary providers:** Goose, OpenCode, and the current Kilo CLI store authoritative conversation history in SQLite.
- **Hybrid providers:** Codex uses JSONL rollouts for session transcript history plus several SQLite databases for tracing, goals, state, memory, and app-server features.
- **Split-lineage providers:** Kilo combines an OpenCode-fork CLI with a Roo-fork IDE extension, so one provider brand can expose incompatible storage models.

The record models are equally diverse:

- Claude uses transcript `type`, attachment subtypes, system subtypes, and message content block types.
- Codex uses a top-level envelope plus nested payload types for response items and event messages.
- Gemini uses line shape: header, `$set` patch, user message, or Gemini message.
- Goose uses relational tables plus JSON message content blocks.
- Kimi uses Wire protocol message types and payload type discriminators.
- OpenCode uses SQL tables with JSON `message.data` and `part.data` discriminators.
- Kilo CLI uses OpenCode-shaped SQL tables with JSON `message.data` and `part.data`; Kilo IDE extensions use Roo-style `ask`/`say` task JSON.
- Pi uses top-level transcript `type`, nested `message.role`, content block `type`, and live `AgentSessionEvent` discriminators.
- Qwen uses transcript `type`, system `subtype`, debug levels/components, and OTel event names.

The time model is not portable. Claudine should normalize all provider timestamps into a single internal UTC representation, but it must preserve source metadata: field name, unit, zone assumption, and confidence. Local-time filenames are especially dangerous because they look like ISO timestamps but lack an offset. Provider logs contain all of these timestamp forms:

- ISO-8601 UTC strings.
- ISO-like local strings with no offset.
- Unix seconds.
- Unix millis.
- Unix seconds plus nanosecond side fields.
- UUID filenames with no timestamp.
- Timestamp-free records whose order only comes from file order or sequence.
- Monotonic event sequence numbers that are not clocks.

Retention also differs. Claude, Kimi, Pi, Qwen, and OpenCode generally accumulate core logs indefinitely unless manually removed. Gemini has session retention settings. Goose cleans old date-subdirectory tracing logs and keeps only ten LLM request logs. Codex date-shards session rollouts but does not make date sharding equivalent to retention. Kilo archives sessions in place with a timestamp field rather than moving them to a different store.

## Point of View for Claudine

Claudine should keep its own JSONL/SQLite reporting layer as the canonical wrapper ledger. Provider logs should be treated as **evidence adapters** that can enrich, corroborate, and backfill Claudine records.

A practical architecture would separate three layers:

1. **Claudine run ledger:** What Claudine launched, which provider/model was used, wrapper timing, lifecycle events, normalized tool events, process termination, errors, and Claudine's own usage estimates.
2. **Provider evidence index:** Provider-native sessions, transcripts, DB rows, debug logs, subagent edges, branch trees, compaction records, retries, and cost/token fields, normalized into provider-specific typed records with source pointers.
3. **Reporting views:** Joined SQLite views that answer user questions: sessions by repo, tool usage, provider errors, token/cost trends, subagent activity, compaction frequency, retry causes, and mismatches between Claudine-observed and provider-native outcomes.

The ingestion rules should be conservative:

- Use provider logs to enrich historical reporting, not to replace live wrapper events.
- Do not assume JSONL. Some providers are SQLite-first.
- Do not copy live WAL databases. Use read-only connections or safe snapshots.
- Preserve raw provider record pointers so a report can trace a metric back to its original file/table/line.
- Normalize timestamps, but store the original value and the interpretation used.
- Preserve provider-native graph structure, including parent/child session edges, subagent edges, fork/clone lineage, and Pi-style transcript `parentId` trees.
- Keep schemas provider-versioned. Several providers have recently changed transcript shape, event vocabulary, database schema, or log format.
- Treat desktop logs as separate unless the provider explicitly shares the same backend store.
- Treat provider brand and product surface separately when they diverge. Kilo's CLI and IDE extension should be modeled as distinct evidence surfaces under one provider, not collapsed into one schema.

The high-value near-term wins are clear: parse Claude and Qwen transcript/debug files as supplemental audit trails; teach Gemini ingestion about header and `$set` JSONL records; add a Pi transcript reader that preserves tree-linked branching and handles the ISO-vs-millis timestamp split; add SQLite readers for OpenCode and Kilo CLI and, later, Goose; use Codex rollout events and state databases for richer thread/subagent/cost reconstruction; and relax/version Kimi Wire parsing so protocol evolution does not break wrapped runs.

The strategic direction is for `claudine logs` to become a federated observability layer: Claudine's own normalized JSONL/SQLite reports remain stable and provider-agnostic, while provider-specific evidence readers add depth where available. That gives Claudine the best of both worlds: a consistent reporting surface for users and enough native detail to debug, audit, and reconstruct wrapped agent sessions accurately.
