use rusqlite::Connection;

use crate::error::Result;

/// Initialize the reporting schema if it does not exist yet.
pub(crate) fn initialize(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        INSERT INTO schema_meta (key, value)
        VALUES ('schema_version', '1')
        ON CONFLICT(key) DO UPDATE SET value = excluded.value;

        CREATE TABLE IF NOT EXISTS ingestion_state (
            source_file TEXT PRIMARY KEY,
            file_size INTEGER NOT NULL,
            modified_at INTEGER NOT NULL,
            fingerprint TEXT NOT NULL,
            byte_offset INTEGER NOT NULL,
            last_event_timestamp TEXT,
            last_synced_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            source_file TEXT NOT NULL,
            source_offset INTEGER NOT NULL,
            source_date TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            provider TEXT NOT NULL,
            event TEXT NOT NULL,
            session_key TEXT NOT NULL,
            session_id TEXT,
            cwd TEXT,
            repo_name TEXT,
            repo_org TEXT,
            branch TEXT,
            primary_language TEXT,
            tool_name TEXT,
            agent_type TEXT,
            notification_type TEXT,
            notification_message TEXT,
            error TEXT,
            model TEXT,
            permission_mode TEXT,
            head_sha TEXT,
            is_dirty INTEGER,
            memory_available_bytes INTEGER,
            hostname TEXT,
            prompt_text TEXT,
            tool_input_json TEXT,
            tool_response_json TEXT,
            extra_json TEXT NOT NULL,
            env_json TEXT NOT NULL,
            PRIMARY KEY (source_file, source_offset)
        );

        CREATE TABLE IF NOT EXISTS sessions (
            session_key TEXT PRIMARY KEY,
            session_id TEXT,
            provider TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT NOT NULL,
            cwd TEXT,
            repo_name TEXT,
            repo_org TEXT,
            branch TEXT,
            model TEXT,
            permission_mode TEXT,
            hostname TEXT,
            primary_language TEXT,
            event_count INTEGER NOT NULL DEFAULT 0,
            turn_count INTEGER NOT NULL DEFAULT 0,
            tool_call_count INTEGER NOT NULL DEFAULT 0,
            tool_error_count INTEGER NOT NULL DEFAULT 0,
            turn_error_count INTEGER NOT NULL DEFAULT 0,
            subagent_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_events_source_date ON events(source_date);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_provider_source_date ON events(provider, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_repo_source_date ON events(repo_name, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_session_timestamp ON events(session_key, timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_tool_source_date ON events(tool_name, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_event_source_date ON events(event, source_date);

        CREATE VIEW IF NOT EXISTS daily_event_totals_v AS
        SELECT
            source_date AS date,
            provider,
            repo_name,
            COUNT(*) AS event_count,
            COUNT(DISTINCT session_key) AS session_count,
            SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END) AS turn_count,
            SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END) AS tool_call_count,
            SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END) AS tool_error_count,
            SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END) AS turn_error_count
        FROM events
        GROUP BY source_date, provider, repo_name;

        CREATE VIEW IF NOT EXISTS daily_tool_usage_v AS
        SELECT
            source_date AS date,
            provider,
            repo_name,
            tool_name,
            SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END) AS call_count,
            SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END) AS error_count
        FROM events
        WHERE tool_name IS NOT NULL
        GROUP BY source_date, provider, repo_name, tool_name;

        CREATE VIEW IF NOT EXISTS daily_subagent_usage_v AS
        SELECT
            source_date AS date,
            provider,
            repo_name,
            agent_type,
            COUNT(*) AS spawn_count
        FROM events
        WHERE event = 'subagent_start' AND agent_type IS NOT NULL
        GROUP BY source_date, provider, repo_name, agent_type;

        CREATE VIEW IF NOT EXISTS daily_repo_activity_v AS
        SELECT
            source_date AS date,
            repo_name,
            repo_org,
            COUNT(*) AS event_count,
            COUNT(DISTINCT session_key) AS session_count,
            COUNT(DISTINCT branch) AS branch_count,
            COUNT(DISTINCT head_sha) AS head_sha_count
        FROM events
        WHERE repo_name IS NOT NULL
        GROUP BY source_date, repo_name, repo_org;
        "#,
    )?;

    Ok(())
}
