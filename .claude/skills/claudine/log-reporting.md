# Log Reporting Design

## Contents

- Shipped Architecture Notes
- Data Foundation
- SQLite Aggregation Layer
- Metrics Catalog
- CLI Subcommand Design
- Implementation Phases
- Library / CLI Boundary
- File Organization
- Data Retention

Use heading search to jump to the listed subsystem.


Claudine's logging action captures a rich, structured audit trail of every agentic CLI session across all supported providers. This document defines a reporting system that transforms raw JSONL logs into actionable insights about developer-AI collaboration patterns.

- **IMPORTANT:** This file describes the functional and reporting requirements for a CLI command but we need to make sure we are always focused on the division of responsibilities between a CLI and its underlying library: a CLI is for reporting only, all business logic MUST go into the library!

## Shipped Architecture Notes

The implementation now follows a library-first reporting model:

- JSONL `EventMeta` logs under `~/.claudine/logs/` remain the canonical audit trail and source of truth.
- SQLite at `~/.claudine/logs/metrics.db` is a derived local index/cache that can be rebuilt from JSONL.
- The reporting schema starts with raw event rows plus a small `sessions` summary table; common rollups are handled by SQL views and query-time aggregation.
- Event identity is stable and idempotent via `(source_file, source_offset)`, so `claudine logs sync` can be rerun safely without double-counting.
- Session identity falls back conservatively: `provider + session_id`, then provider-specific fields such as transcript/thread paths, then `provider + source_file + source_offset`.
- Read-oriented `claudine logs` commands perform a best-effort sync before querying so users do not need to run `logs sync` manually first.
- Terminal output intentionally truncates prompts, commands, and error context; the reporting database stores structured payloads, but the CLI does not dump full tool payloads by default.

## Data Foundation

### Log Entry Schema (`EventMeta`)

Every event is serialized as a single-line JSON record with these fields:

| Field               | Type              | Description                                                      |
|---------------------|-------------------|------------------------------------------------------------------|
| `provider`          | `String`          | Which CLI fired the event (`claude`, `opencode`, `gemini`, etc)  |
| `event`             | `String`          | One of 16 normalized event types (see below)                     |
| `timestamp`         | `DateTime<Utc>`   | When the event occurred                                          |
| `session_id`        | `Option<String>`  | Session UUID (null for some OpenCode events)                     |
| `cwd`               | `Option<String>`  | Working directory at event time                                  |
| `tool_name`         | `Option<String>`  | Tool name for tool-related events                                |
| `tool_input`        | `Option<Value>`   | Structured tool arguments                                        |
| `tool_response`     | `Option<Value>`   | Structured tool output (after_tool only)                         |
| `error`             | `Option<String>`  | Error message (tool_error, turn_error)                           |
| `prompt`            | `Option<String>`  | User prompt text (before_prompt only)                            |
| `agent_type`        | `Option<String>`  | Subagent type (subagent_start/stop only)                         |
| `notification_type` | `Option<String>`  | Notification classification                                      |
| `notification_message` | `Option<String>` | Notification body text                                          |
| `extra`             | `HashMap`         | Provider-specific fields (model, permission_mode, transcript_path, tool_use_id, stop_hook_active) |
| `env`               | `EnvironmentContext` | Full environment snapshot (see below)                         |
| `env.claudine_pid`  | `Option<u32>`     | Claudine's own process ID, captured at wrapper startup          |
| `agent_pid`         | `Option<u32>`     | Immediate child PID after successful spawn; omitted when unavailable |

### Environment Context (embedded in every event)

| Section     | Fields                                                                                                    |
|-------------|-----------------------------------------------------------------------------------------------------------|
| `env.os`    | `os_type`, `name`, `version`, `kernel`, `hostname`, `linux_family`, `package_managers`                     |
| `env.hardware` | `arch`, `cpu`, `cores`, `memory_bytes`, `memory_available_bytes`                                       |
| `env.git`   | `repo_root`, `branch`, `is_dirty`, `staged_count`, `unstaged_count`, `untracked_count`, `head_sha`, `head_message`, `user_name`, `user_email`, `remote_name`, `remote_url`, `hosting_provider`, `repo_name`, `repo_org` |
| `env.repo`  | `is_monorepo`, `monorepo_standard`, `monorepo_orchestrators`, `monorepo_tool` (deprecated alias), `root`, `packages[]` |
| `primary_language` | Top-level string field                                                                              |
| wrapper package context | Top-level `package_area` and `package` fields populated from wrapper-provided `PACKAGE_AREA` / `PACKAGE` env vars when present |

### Event Types (16 normalized events)

| Event              | Slug               | Fired When                                        | Key Fields Present                          |
|--------------------|---------------------|---------------------------------------------------|---------------------------------------------|
| SessionStart       | `session_start`     | New CLI session begins                             | session_id, extra.model                     |
| SessionEnd         | `session_end`       | CLI session terminates                             | session_id                                  |
| BeforePrompt       | `before_prompt`     | User submits a prompt                              | prompt, extra.permission_mode               |
| BeforeTool         | `before_tool`       | Tool invocation starts                             | tool_name, tool_input, extra.tool_use_id    |
| AfterTool          | `after_tool`        | Tool execution completes                           | tool_name, tool_input, tool_response        |
| ToolError          | `tool_error`        | Tool execution fails                               | tool_name, tool_input, error                |
| PermissionRequest  | `permission_request`| CLI asks user for permission                       | tool_name, tool_input                       |
| TurnComplete       | `turn_complete`     | Model finishes responding                          | extra.stop_hook_active                      |
| TurnError          | `turn_error`        | Model response fails                               | error                                       |
| SubagentStart      | `subagent_start`    | Subagent spawned                                   | agent_type                                  |
| SubagentStop       | `subagent_stop`     | Subagent completes                                 | agent_type, extra.stop_hook_active          |
| BeforeModel        | `before_model`      | Model request about to be sent                     | (minimal)                                   |
| AfterModel         | `after_model`       | Model response received                            | (minimal — most context in provider)        |
| BeforeCompact      | `before_compact`    | Context window compression about to occur          | session_id                                  |
| Notification       | `notification`      | System notification (session resume, etc)           | notification_type, notification_message      |
| HumanInTheLoop     | `human_in_the_loop` | User interaction required (question, approval)     | tool_name, tool_input                       |

### Current Data Profile (observed from 15 days of production logs)

- **66,121 total events** across 15 days (Feb 20 – Mar 6, 2026)
- **~390 sessions** tracked
- **2 providers active**: Claude Code (43%), OpenCode (57%)
- **Daily volume**: 221 – 9,012 events/day (median ~4,000)
- **Session duration**: median 66 min, mean 310 min, max 57 hrs (long-running background)
- **Tool distribution**: Read (dominant), Bash, Edit, Grep, WebFetch, WebSearch, Write, Glob, Agent, TaskUpdate, TaskCreate, Skill
- **Error rate**: 131 tool_error + 12 turn_error across 66K events (~0.2%)
- **Subagent types observed**: rust-developer, rust-architect, Explore, feature-tester-rust
- **Log file sizes**: 2MB – 53MB/day (median ~19MB)

## SQLite Aggregation Layer

### Why SQLite

Raw JSONL logs are append-only and excellent for audit trails, but querying across days, sessions, or repos requires scanning potentially hundreds of megabytes. A SQLite database provides:

1. **Indexed queries** — sub-second lookups by date, session, provider, tool, repo
2. **Aggregation** — COUNT, SUM, AVG, GROUP BY without loading entire files
3. **Cross-day analysis** — trends, comparisons, rolling averages
4. **Space efficiency** — normalized schema avoids repeating env blocks
5. **Zero infrastructure** — single file, no server, ships with claudine

### Database Location

`~/.claudine/logs/metrics.db` — sibling to the JSONL files it indexes.

### Schema Design

```sql
-- Core dimension tables (deduplicated from env blocks)

CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,  -- session UUID
    provider    TEXT NOT NULL,
    model       TEXT,
    started_at  TEXT NOT NULL,     -- ISO 8601
    ended_at    TEXT,
    cwd         TEXT,
    repo_name   TEXT,
    repo_org    TEXT,
    branch      TEXT,
    is_monorepo INTEGER,
    primary_language TEXT,
    permission_mode TEXT,
    hostname    TEXT
);

CREATE TABLE daily_summaries (
    date          TEXT NOT NULL,     -- YYYY-MM-DD
    provider      TEXT NOT NULL,
    repo_name     TEXT,
    -- Session metrics
    session_count       INTEGER DEFAULT 0,
    total_turns         INTEGER DEFAULT 0,
    total_tool_calls    INTEGER DEFAULT 0,
    total_tool_errors   INTEGER DEFAULT 0,
    total_turn_errors   INTEGER DEFAULT 0,
    total_subagents     INTEGER DEFAULT 0,
    total_compactions   INTEGER DEFAULT 0,
    total_permission_requests INTEGER DEFAULT 0,
    total_human_in_loop INTEGER DEFAULT 0,
    -- Time metrics (seconds)
    total_session_duration_secs REAL DEFAULT 0,
    -- Memory snapshot (latest for the day)
    memory_available_bytes INTEGER,
    PRIMARY KEY (date, provider, repo_name)
);

CREATE TABLE tool_usage (
    date        TEXT NOT NULL,
    provider    TEXT NOT NULL,
    repo_name   TEXT,
    tool_name   TEXT NOT NULL,
    call_count  INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    PRIMARY KEY (date, provider, repo_name, tool_name)
);

CREATE TABLE subagent_usage (
    date          TEXT NOT NULL,
    provider      TEXT NOT NULL,
    repo_name     TEXT,
    agent_type    TEXT NOT NULL,
    spawn_count   INTEGER DEFAULT 0,
    PRIMARY KEY (date, provider, repo_name, agent_type)
);

CREATE TABLE git_activity (
    date            TEXT NOT NULL,
    repo_name       TEXT NOT NULL,
    repo_org        TEXT,
    -- Snapshot from first and last events of the day
    start_sha       TEXT,
    end_sha         TEXT,
    commits_observed INTEGER DEFAULT 0,  -- distinct head_sha values seen
    max_staged      INTEGER DEFAULT 0,
    max_unstaged    INTEGER DEFAULT 0,
    max_untracked   INTEGER DEFAULT 0,
    branches_used   TEXT,  -- JSON array of distinct branches
    PRIMARY KEY (date, repo_name)
);

-- Indexes
CREATE INDEX idx_sessions_date ON sessions(started_at);
CREATE INDEX idx_sessions_repo ON sessions(repo_name);
CREATE INDEX idx_daily_date ON daily_summaries(date);
CREATE INDEX idx_tool_date ON tool_usage(date);
```

### Ingestion Strategy

**Incremental ingestion** — track a `last_ingested` marker per JSONL file (file name + byte offset). On each `claudine logs --sync`, resume from the last position. This avoids re-reading gigabytes of already-processed data.

```sql
CREATE TABLE ingestion_state (
    file_name   TEXT PRIMARY KEY,  -- e.g., "2026-03-06.jsonl"
    byte_offset INTEGER NOT NULL,  -- resume position
    last_event_timestamp TEXT       -- for validation
);
```

**Ingestion runs** as a single-pass scan per JSONL file:

1. Open file, seek to stored byte_offset (or 0 for new files)
2. Parse each JSON line, extract fields
3. Upsert into dimension tables (sessions) and increment aggregate tables (daily_summaries, tool_usage, etc.)
4. Update ingestion_state with final byte position

## Metrics Catalog

### Intraday Metrics (single-day focus)

| Metric                        | Source Events                 | Description                                                       |
|-------------------------------|-------------------------------|-------------------------------------------------------------------|
| **Session count**             | session_start                 | How many coding sessions today                                    |
| **Active time**               | session_start → session_end   | Total session wall-clock time                                     |
| **Turns per session**         | turn_complete per session     | Number of user prompts answered per session                       |
| **Tool call volume**          | before_tool                   | Total tool invocations today                                      |
| **Tool breakdown**            | before_tool.tool_name         | Count per tool (Read, Edit, Bash, etc)                            |
| **Tool error rate**           | tool_error / before_tool      | Percentage of tool calls that fail, by tool                       |
| **Edit intensity**            | Edit tool_input               | Files edited, unique file count                                   |
| **Files read**                | Read tool_input.file_path     | Unique files read today                                           |
| **Bash commands**             | Bash tool_input.command       | Command count; description summaries                              |
| **Write volume**              | Write tool calls              | New files created today                                           |
| **Search activity**           | Grep + Glob + WebSearch       | Code search vs web research ratio                                 |
| **Subagent usage**            | subagent_start/stop           | Types spawned, count per type                                     |
| **Subagent duration**         | subagent_start → subagent_stop| Time spent in delegated work                                     |
| **Permission requests**       | permission_request            | How often the AI asked for permission; which tools triggered it   |
| **Human-in-the-loop**         | human_in_the_loop             | User questions asked; decisions made                              |
| **Context compactions**       | before_compact                | How often the context window overflowed                           |
| **Provider distribution**     | provider field                | Time spent in Claude vs OpenCode vs others                        |
| **Git commits observed**      | env.git.head_sha changes      | Commits that occurred during sessions                             |
| **Dirty state transitions**   | env.git.is_dirty changes      | When repo went dirty → clean (commit) or stayed dirty             |
| **Memory pressure**           | env.hardware.memory_available | Track if available memory drops during heavy sessions             |
| **Repo context switches**     | env.git.repo_name changes     | How often the user jumped between repos within a day              |
| **Model usage**               | extra.model (session_start)   | Which models were used (opus, sonnet, haiku)                      |
| **Permission mode profile**   | extra.permission_mode         | Distribution of bypassPermissions vs default vs plan vs acceptEdits |
| **Skill invocations**         | Skill tool calls              | Which skills were activated and how often                         |
| **Turn errors**               | turn_error                    | Model failures (separate from tool failures)                      |

### Interday Metrics (trends across days)

| Metric                        | Aggregation                   | Description                                                       |
|-------------------------------|-------------------------------|-------------------------------------------------------------------|
| **Daily activity trend**      | daily_summaries over time     | Events/day, sessions/day, tools/day over weeks                    |
| **Tool usage evolution**      | tool_usage over time          | Which tools are used more/less over time                          |
| **Error rate trend**          | tool_error rate over days     | Is error rate improving or worsening?                             |
| **Session duration trend**    | session durations over days   | Are sessions getting longer/shorter?                              |
| **Provider migration**        | provider distribution/day     | Shift between Claude and OpenCode usage                           |
| **Subagent adoption**         | subagent_usage over time      | Are subagents being used more? Which types?                       |
| **Repo activity heatmap**     | git_activity over time        | Which repos had the most activity per week                        |
| **Commit velocity**           | commits_observed per day      | How many commits per day across all repos                         |
| **Busiest hours**             | timestamp hour extraction     | What times of day see the most activity                           |
| **Day-of-week patterns**      | timestamp weekday extraction  | Weekday vs weekend usage patterns                                 |
| **Memory baseline**           | memory_available over days    | System health monitoring over time                                |
| **Permission mode shifts**    | permission_mode distribution  | Are users moving toward more permissive modes?                    |
| **Tool diversity index**      | unique tools per session      | Are sessions using more or fewer tool types over time?            |
| **Compaction frequency**      | before_compact per day        | Are context windows overflowing more often (complexity growth)?   |
| **Rolling averages**          | 7-day rolling window          | Smoothed trends for all key metrics                               |

### Derived / Composite Metrics

| Metric                        | Derivation                    | Description                                                       |
|-------------------------------|-------------------------------|-------------------------------------------------------------------|
| **Autonomy ratio**            | tool_calls / turns            | Higher = more autonomous (more tool calls per user prompt)        |
| **Research vs action ratio**  | (Read+Grep+Glob+WebSearch) / (Edit+Write+Bash) | Reading vs modifying                        |
| **Error recovery rate**       | tool_error followed by successful retry | How often errors self-resolve                    |
| **Delegation ratio**          | subagent tools / total tools  | What fraction of work is delegated to subagents                   |
| **Session efficiency**        | turns / session_duration      | Prompt throughput (higher = more productive sessions)             |
| **Context pressure index**    | compactions / session_count   | Average compactions per session (complexity indicator)             |

## CLI Subcommand Design

### `claudine logs [SUBCOMMAND]`

| Subcommand          | Description                                                          |
|---------------------|----------------------------------------------------------------------|
| `claudine logs`     | Quick daily summary for today (default)                              |
| `claudine logs today` | Same as bare `claudine logs`                                      |
| `claudine logs today errors` | Errors encountered today                                  |
| `claudine logs yesterday` | Quick daily summary for yesterday                              |
| `claudine logs yesterday errors` | Errors encountered yesterday                        |
| `claudine logs week` | 7-day rolling summary with sparklines                               |
| `claudine logs week errors` | Errors encountered during the rolling week                  |
| `claudine logs month` | 30-day rolling summary                                             |
| `claudine logs month errors` | Errors encountered during the rolling month                |
| `claudine logs sync` | Ingest new JSONL data into SQLite                                   |
| `claudine logs sessions [--date DATE]` | List sessions with duration, turns, tools    |
| `claudine logs tools [--date DATE] [--top N]` | Tool usage breakdown                 |
| `claudine logs errors [--date DATE]` | Error analysis with context                    |
| `claudine logs repos [--date DATE]` | Repository activity summary                     |
| `claudine logs trends [--days N]` | Multi-day trend analysis                          |

### Arguments

| Argument / Flag       | Description                                                          |
|-----------------------|----------------------------------------------------------------------|
| `--date DATE`         | Specific date (YYYY-MM-DD). Defaults to today.                       |
| `--from DATE`         | Start of date range.                                                 |
| `--to DATE`           | End of date range.                                                   |
| `--provider NAME`     | Filter to a specific provider.                                       |
| `--repo NAME`         | Filter to a specific repository.                                     |
| `--json`              | Output as JSON instead of terminal-formatted.                        |
| `--top N`             | Limit ranked lists to top N entries (default: 10).                   |

### Report Rendering

All reports follow existing claudine CLI patterns:

- **Header**: `<blue><b>Log Report: {title}</b></blue>` with separator
- **Tables**: Using `comfy-table` or `biscuit-terminal` table components
- **Sparklines**: For trend data, use Unicode block characters (▁▂▃▄▅▆▇█) to show 7-day or 30-day patterns inline
- **Badges**: Reuse `badges::USER_SCOPED` etc. pattern for status indicators
- **Colors**: Green for improvements, red for regressions, dim for secondary data

### Example Output: `claudine logs`

```
Log Report: Today (2026-03-06)
==============================

Sessions: 16  |  Turns: 22  |  Duration: 4h 12m
Providers: Claude (9), OpenCode (7)
Repos: rusty-biscuit (14), ai-consulting (2)

Tools                         Count   Errors
─────────────────────────────────────────────
Bash                            100        0
Read                             59        0
Edit                             36        0
Write                            28        0
WebSearch                        22        0
Grep                             14        0
Glob                              4        0
Skill                             4        0

Subagents: (none today)
Compactions: 0  |  Permission Requests: 0
```

### Example Output: `claudine logs trends --days 7`

```
Log Report: 7-Day Trends
=========================

         Events  Sessions  Turns  Errors
Mar 01    2,552       12     22       3   ▃
Mar 02    9,012       35     87      11   █
Mar 03    3,686       18     31       2   ▃
Mar 04    4,788       22     48       5   ▄
Mar 05    5,177       37     57       6   ▅
Mar 06    2,270       16     22       0   ▂

Provider Split: Claude ████████░░ 43%  OpenCode ██████████ 57%

Top Tools (7d):  Read 736 ▇  Bash 603 ▆  Edit 360 ▄  Grep 194 ▂
Error Hotspots:  Bash 53  Read 40  WebFetch 37
```

## Implementation Phases

### Phase 1: SQLite Foundation + `logs sync`

- Define schema and create `metrics.db`
- Implement incremental JSONL → SQLite ingestion
- The `logs sync` subcommand
- All logic in `claudine/lib/src/reporting/` module

### Phase 2: Basic Reporting (`logs`, `logs today`, `logs sessions`, `logs tools`, `logs errors`)

- Single-day queries against SQLite
- Terminal-formatted output via biscuit-terminal
- JSON output option

### Phase 3: Trend Analysis (`logs week`, `logs month`, `logs trends`)

- Multi-day aggregation queries
- Sparkline rendering
- Derived metrics (autonomy ratio, research vs action, etc)

### Phase 4: Repository & Git Analysis (`logs repos`)

- Git commit tracking (distinct SHA values)
- Branch activity
- Dirty state monitoring
- Repo switching patterns

### Phase 5: Advanced Analytics

- Session efficiency scoring
- Error recovery pattern detection
- Subagent cost/benefit analysis
- Hourly and day-of-week heatmaps
- Memory pressure correlation with session complexity

## Library / CLI Boundary

| Layer      | Responsibility                                                       |
|------------|----------------------------------------------------------------------|
| **Library** (`claudine/lib/src/reporting/`) | SQLite schema management, ingestion, queries, metric computation, data types |
| **CLI** (`claudine/cli/src/commands/logs.rs`) | Argument parsing, terminal rendering, output formatting |

The library exposes typed query results (e.g., `DailySummary`, `ToolBreakdown`, `SessionInfo`, `TrendData`) and the CLI renders them using biscuit-terminal components. No SQL or data logic in the CLI layer.

## File Organization

```
claudine/lib/src/reporting/
├── mod.rs              -- Public API re-exports
├── schema.rs           -- SQLite schema creation and migration
├── ingest.rs           -- JSONL → SQLite incremental ingestion
├── queries.rs          -- Query functions returning typed results
├── metrics.rs          -- Derived metric computation
└── types.rs            -- DailySummary, ToolBreakdown, SessionInfo, TrendData, etc.

claudine/cli/src/commands/
└── logs.rs             -- Subcommand args, dispatch, rendering
```

## Data Retention

- **JSONL files**: No automatic deletion — user controls retention
- **SQLite database**: Mirrors whatever JSONL files exist; `logs sync` can re-ingest if DB is deleted
- **Future**: Optional `--prune-before DATE` to drop old SQLite data and reclaim space
