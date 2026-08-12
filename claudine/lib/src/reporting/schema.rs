use rusqlite::{Connection, OptionalExtension};

use crate::error::Result;

const SCHEMA_VERSION: &str = "6";

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
            package_area TEXT,
            package TEXT,
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
            input_tokens INTEGER,
            output_tokens INTEGER,
            total_tokens INTEGER,
            cache_read_tokens INTEGER,
            cost_usd REAL,
            claudine_pid INTEGER,
            agent_pid INTEGER,
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
            package_area TEXT,
            package TEXT,
            model TEXT,
            permission_mode TEXT,
            hostname TEXT,
            primary_language TEXT,
            event_count INTEGER NOT NULL DEFAULT 0,
            turn_count INTEGER NOT NULL DEFAULT 0,
            tool_call_count INTEGER NOT NULL DEFAULT 0,
            tool_error_count INTEGER NOT NULL DEFAULT 0,
            turn_error_count INTEGER NOT NULL DEFAULT 0,
            subagent_count INTEGER NOT NULL DEFAULT 0,
            total_input_tokens INTEGER NOT NULL DEFAULT 0,
            total_output_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
            total_cost_usd REAL NOT NULL DEFAULT 0,
            claudine_pid INTEGER,
            agent_pid INTEGER,
            family_latest TEXT
        );

        -- One row per signal observation exploded from a SessionEnd
        -- summary row's extra["signals"]; keyed by source position so
        -- re-ingest is idempotent alongside the events table.
        CREATE TABLE IF NOT EXISTS session_signals (
            source_file TEXT NOT NULL,
            source_offset INTEGER NOT NULL,
            signal_index INTEGER NOT NULL,
            session_key TEXT NOT NULL,
            session_id TEXT,
            provider TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source_date TEXT NOT NULL,
            kind TEXT NOT NULL,
            source TEXT NOT NULL,
            occurrences INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY (source_file, source_offset, signal_index)
        );
        "#,
    )?;

    let previous_version = schema_version(conn)?;
    let prev = previous_version.as_deref().unwrap_or("0");
    if prev < "2" {
        migrate_to_v2(conn)?;
    }
    if prev < "3" {
        migrate_to_v3(conn)?;
    }
    if prev < "4" {
        migrate_to_v4(conn)?;
    }
    if prev < "5" {
        migrate_to_v5(conn)?;
    }
    if prev < "6" {
        migrate_to_v6(conn)?;
    }

    conn.execute(
        r#"
        INSERT INTO schema_meta (key, value)
        VALUES ('schema_version', ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        [SCHEMA_VERSION],
    )?;

    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_events_source_date ON events(source_date);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_provider_source_date ON events(provider, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_repo_source_date ON events(repo_name, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_package_area_source_date ON events(package_area, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_package_source_date ON events(package, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_session_timestamp ON events(session_key, timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_tool_source_date ON events(tool_name, source_date);
        CREATE INDEX IF NOT EXISTS idx_events_event_source_date ON events(event, source_date);
        CREATE INDEX IF NOT EXISTS idx_session_signals_kind_source_date ON session_signals(kind, source_date);

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

fn schema_version(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM schema_meta WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    ensure_column(conn, "events", "package_area", "TEXT")?;
    ensure_column(conn, "events", "package", "TEXT")?;
    ensure_column(conn, "sessions", "package_area", "TEXT")?;
    ensure_column(conn, "sessions", "package", "TEXT")?;

    // The reporting database is a derived cache of JSONL logs, so clearing the
    // indexed tables is the safest way to backfill newly-added columns.
    conn.execute("DELETE FROM ingestion_state", [])?;
    conn.execute("DELETE FROM sessions", [])?;
    conn.execute("DELETE FROM events", [])?;

    Ok(())
}

fn migrate_to_v3(conn: &Connection) -> Result<()> {
    // Add token/cost columns to events
    ensure_column(conn, "events", "input_tokens", "INTEGER")?;
    ensure_column(conn, "events", "output_tokens", "INTEGER")?;
    ensure_column(conn, "events", "total_tokens", "INTEGER")?;
    ensure_column(conn, "events", "cache_read_tokens", "INTEGER")?;
    ensure_column(conn, "events", "cost_usd", "REAL")?;

    // Add token/cost accumulators to sessions
    ensure_column(conn, "sessions", "total_input_tokens", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "sessions", "total_output_tokens", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "sessions", "total_tokens", "INTEGER DEFAULT 0")?;
    ensure_column(
        conn,
        "sessions",
        "total_cache_read_tokens",
        "INTEGER DEFAULT 0",
    )?;
    ensure_column(conn, "sessions", "total_cost_usd", "REAL DEFAULT 0")?;

    // Re-ingest to backfill the new columns from extra_json
    conn.execute("DELETE FROM ingestion_state", [])?;
    conn.execute("DELETE FROM sessions", [])?;
    conn.execute("DELETE FROM events", [])?;

    Ok(())
}

fn migrate_to_v4(conn: &Connection) -> Result<()> {
    // Track whether a session was interactive (TTY) or headless (CI, piped).
    ensure_column(conn, "sessions", "interactive", "INTEGER")?;
    // Track whether a session used the anonymous fallback key (no provider session_id).
    ensure_column(conn, "events", "anonymous_session", "INTEGER DEFAULT 0")?;

    // Re-ingest to backfill
    conn.execute("DELETE FROM ingestion_state", [])?;
    conn.execute("DELETE FROM sessions", [])?;
    conn.execute("DELETE FROM events", [])?;

    Ok(())
}

fn migrate_to_v5(conn: &Connection) -> Result<()> {
    // Capture Claudine wrapper PIDs for correlation with system telemetry.
    ensure_column(conn, "events", "claudine_pid", "INTEGER")?;
    ensure_column(conn, "events", "agent_pid", "INTEGER")?;
    ensure_column(conn, "sessions", "claudine_pid", "INTEGER")?;
    ensure_column(conn, "sessions", "agent_pid", "INTEGER")?;

    // The reporting database is a derived cache of JSONL logs, so clearing the
    // indexed tables is the safest way to backfill newly-added columns.
    conn.execute("DELETE FROM ingestion_state", [])?;
    conn.execute("DELETE FROM sessions", [])?;
    conn.execute("DELETE FROM events", [])?;

    Ok(())
}

fn migrate_to_v6(conn: &Connection) -> Result<()> {
    // Model-catalog reporting: lifted `family_latest` alias stamp on
    // sessions; the session_signals table itself is created by the base
    // CREATE TABLE IF NOT EXISTS batch, which runs before migrations.
    ensure_column(conn, "sessions", "family_latest", "TEXT")?;

    // The reporting database is a derived cache of JSONL logs, so clearing the
    // indexed tables is the safest way to backfill newly-added columns.
    conn.execute("DELETE FROM ingestion_state", [])?;
    conn.execute("DELETE FROM sessions", [])?;
    conn.execute("DELETE FROM events", [])?;
    conn.execute("DELETE FROM session_signals", [])?;

    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    validate_identifier(table, "table")?;
    validate_identifier(column, "column")?;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn validate_identifier(ident: &str, kind: &str) -> Result<()> {
    if ident.is_empty() {
        return Err(crate::error::ClaudineError::ConfigValidation(format!(
            "SQL {kind} identifier must not be empty"
        )));
    }
    if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(crate::error::ClaudineError::ConfigValidation(format!(
            "SQL {kind} identifier `{ident}` contains invalid characters (only [a-zA-Z0-9_] allowed)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;

    #[test]
    fn initialize_migrates_v1_reporting_db_to_v2() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT INTO schema_meta (key, value) VALUES ('schema_version', '1');

            CREATE TABLE ingestion_state (
                source_file TEXT PRIMARY KEY,
                file_size INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                fingerprint TEXT NOT NULL,
                byte_offset INTEGER NOT NULL,
                last_event_timestamp TEXT,
                last_synced_at TEXT NOT NULL
            );
            INSERT INTO ingestion_state (
                source_file, file_size, modified_at, fingerprint, byte_offset, last_event_timestamp, last_synced_at
            ) VALUES ('/tmp/2026-03-07.jsonl', 10, 11, 'abcd', 12, NULL, '2026-03-07T00:00:00Z');

            CREATE TABLE events (
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
            INSERT INTO events (
                source_file, source_offset, source_date, timestamp, provider, event, session_key,
                session_id, cwd, repo_name, repo_org, branch, primary_language, tool_name,
                agent_type, notification_type, notification_message, error, model,
                permission_mode, head_sha, is_dirty, memory_available_bytes, hostname,
                prompt_text, tool_input_json, tool_response_json, extra_json, env_json
            ) VALUES (
                '/tmp/2026-03-07.jsonl', 1, '2026-03-07', '2026-03-07T00:00:00Z', 'claude',
                'session_start', 'claude:s-1', 's-1', '/tmp/repo', 'rusty-biscuit', 'ken',
                'main', 'Rust', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0,
                'host', NULL, NULL, NULL, '{}', '{}'
            );

            CREATE TABLE sessions (
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
            INSERT INTO sessions (
                session_key, session_id, provider, started_at, ended_at, cwd, repo_name, repo_org,
                branch, model, permission_mode, hostname, primary_language, event_count,
                turn_count, tool_call_count, tool_error_count, turn_error_count, subagent_count
            ) VALUES (
                'claude:s-1', 's-1', 'claude', '2026-03-07T00:00:00Z', '2026-03-07T00:01:00Z',
                '/tmp/repo', 'rusty-biscuit', 'ken', 'main', NULL, NULL, 'host', 'Rust',
                1, 0, 0, 0, 0, 0
            );
            "#,
        )
        .unwrap();

        initialize(&conn).unwrap();

        let version: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        let event_columns = table_columns(&conn, "events");
        assert!(event_columns.contains(&"package_area".to_string()));
        assert!(event_columns.contains(&"package".to_string()));

        let session_columns = table_columns(&conn, "sessions");
        assert!(session_columns.contains(&"package_area".to_string()));
        assert!(session_columns.contains(&"package".to_string()));
        // v4 columns
        assert!(event_columns.contains(&"anonymous_session".to_string()));
        assert!(session_columns.contains(&"interactive".to_string()));
        // v5 columns
        assert!(event_columns.contains(&"claudine_pid".to_string()));
        assert!(event_columns.contains(&"agent_pid".to_string()));
        assert!(session_columns.contains(&"claudine_pid".to_string()));
        assert!(session_columns.contains(&"agent_pid".to_string()));
        // v6: lifted alias stamp + the signals table
        assert!(session_columns.contains(&"family_latest".to_string()));
        let signal_columns = table_columns(&conn, "session_signals");
        assert!(signal_columns.contains(&"kind".to_string()));
        assert!(signal_columns.contains(&"payload_json".to_string()));

        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap();
        let session_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        let ingestion_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ingestion_state", [], |row| row.get(0))
            .unwrap();

        assert_eq!(event_count, 0);
        assert_eq!(session_count, 0);
        assert_eq!(ingestion_count, 0);
    }

    fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();

        rows.map(|row| row.unwrap()).collect()
    }
}
