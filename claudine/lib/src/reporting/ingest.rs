use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use biscuit_hash::xx_hash_bytes;
use chrono::{Local, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::error::Result;
use crate::events::EventMeta;

use super::paths;
use super::types::{DateRange, SyncFailure, SyncRequest, SyncSummary};

#[derive(Debug)]
struct IngestionStateRow {
    file_size: i64,
    fingerprint: String,
    byte_offset: i64,
}

#[derive(Debug, Default)]
struct FileSyncStats {
    rebuilt: bool,
    inserted: u64,
    skipped: u64,
    anonymous_session_fallbacks: u64,
    line_failures: Vec<SyncFailure>,
}

#[derive(Debug)]
struct PreparedEvent {
    source_file: String,
    source_offset: i64,
    source_date: String,
    timestamp: String,
    provider: String,
    event: String,
    session_key: String,
    session_id: Option<String>,
    cwd: Option<String>,
    repo_name: Option<String>,
    repo_org: Option<String>,
    branch: Option<String>,
    primary_language: Option<String>,
    package_area: Option<String>,
    package: Option<String>,
    tool_name: Option<String>,
    agent_type: Option<String>,
    notification_type: Option<String>,
    notification_message: Option<String>,
    error: Option<String>,
    model: Option<String>,
    permission_mode: Option<String>,
    head_sha: Option<String>,
    is_dirty: Option<i64>,
    memory_available_bytes: Option<i64>,
    hostname: Option<String>,
    prompt_text: Option<String>,
    tool_input_json: Option<String>,
    tool_response_json: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cost_usd: Option<f64>,
    extra_json: String,
    env_json: String,
    anonymous_session_fallback: bool,
    /// Whether this event came from an interactive (TTY) session.
    interactive: Option<bool>,
}

/// Sync JSONL event logs into the SQLite reporting index.
pub(crate) fn sync(
    conn: &mut Connection,
    logs_dir: &Path,
    request: SyncRequest,
) -> Result<SyncSummary> {
    let mut summary = SyncSummary::default();
    let files = discover_log_files(logs_dir, request)?;

    for file in files {
        summary.files_scanned += 1;
        match sync_file(conn, &file) {
            Ok(stats) => {
                if stats.rebuilt {
                    summary.files_rebuilt += 1;
                }
                summary.events_inserted += stats.inserted;
                summary.events_skipped += stats.skipped;
                summary.anonymous_session_fallbacks += stats.anonymous_session_fallbacks;
                summary.parse_failures += stats.line_failures.len() as u64;
                summary.failures.extend(stats.line_failures);
            }
            Err(failure) => {
                summary.parse_failures += 1;
                summary.failures.push(failure);
            }
        }
    }

    Ok(summary)
}

fn discover_log_files(logs_dir: &Path, request: SyncRequest) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !logs_dir.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(logs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }

        let include = match request {
            SyncRequest::All => true,
            SyncRequest::Date(date) => {
                paths::daily_log_date_from_path(&path).is_none_or(|file_date| file_date == date)
            }
            SyncRequest::Range(DateRange { from, to }) => paths::daily_log_date_from_path(&path)
                .is_none_or(|file_date| file_date >= from && file_date <= to),
        };

        if include {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn sync_file(
    conn: &mut Connection,
    path: &Path,
) -> std::result::Result<FileSyncStats, SyncFailure> {
    let source_file_str = path.display().to_string();

    macro_rules! fail {
        ($line:expr, $error:expr) => {
            SyncFailure {
                source_file: source_file_str.clone(),
                line_number: $line,
                message: $error.to_string(),
            }
        };
    }

    let metadata = std::fs::metadata(path).map_err(|e| fail!(0, e))?;
    let file_size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default();
    let fingerprint = file_fingerprint(path).map_err(|e| fail!(0, e))?;

    let state = load_state(conn, path).map_err(|e| fail!(0, e))?;

    let rebuild = state
        .as_ref()
        .is_some_and(|state| file_size < state.file_size || fingerprint != state.fingerprint);
    let start_offset = if rebuild {
        0
    } else {
        state
            .as_ref()
            .map(|state| state.byte_offset.min(file_size))
            .unwrap_or(0)
    };

    if start_offset == file_size && !rebuild {
        return Ok(FileSyncStats::default());
    }

    let mut stats = FileSyncStats {
        rebuilt: rebuild,
        ..FileSyncStats::default()
    };

    let tx = conn.transaction().map_err(|e| fail!(0, e))?;

    if rebuild {
        tx.execute(
            "DELETE FROM events WHERE source_file = ?1",
            params![source_file_str],
        )
        .map_err(|e| fail!(0, e))?;
    }

    let mut file = BufReader::new(File::open(path).map_err(|e| fail!(0, e))?);
    file.seek(SeekFrom::Start(start_offset as u64))
        .map_err(|e| fail!(0, e))?;

    let mut source_offset = start_offset as u64;
    let mut buffer = Vec::new();
    let mut line_number = count_lines_before(path, start_offset as u64).unwrap_or(0) + 1;
    let mut last_event_timestamp: Option<String> = None;

    loop {
        buffer.clear();
        let bytes_read = file
            .read_until(b'\n', &mut buffer)
            .map_err(|e| fail!(line_number, e))?;
        if bytes_read == 0 {
            break;
        }

        let raw_line = String::from_utf8_lossy(&buffer);
        let line = raw_line.trim_end_matches(['\n', '\r']);
        if line.trim().is_empty() {
            source_offset += bytes_read as u64;
            line_number += 1;
            continue;
        }

        // Treat malformed lines as per-line failures: skip the line, record
        // the failure, and continue ingesting the rest of the file.
        let meta: EventMeta = match serde_json::from_str(line) {
            Ok(meta) => meta,
            Err(error) => {
                stats.line_failures.push(SyncFailure {
                    source_file: source_file_str.clone(),
                    line_number,
                    message: error.to_string(),
                });
                source_offset += bytes_read as u64;
                line_number += 1;
                continue;
            }
        };

        let prepared = match prepare_event(path, source_offset as i64, meta) {
            Ok(prepared) => prepared,
            Err(error) => {
                stats.line_failures.push(SyncFailure {
                    source_file: source_file_str.clone(),
                    line_number,
                    message: error.to_string(),
                });
                source_offset += bytes_read as u64;
                line_number += 1;
                continue;
            }
        };

        let inserted = insert_event(&tx, &prepared).map_err(|e| fail!(line_number, e))?;

        if inserted {
            stats.inserted += 1;
            stats.anonymous_session_fallbacks += u64::from(prepared.anonymous_session_fallback);

            if !rebuild {
                upsert_session(&tx, &prepared).map_err(|e| fail!(line_number, e))?;
            }
        } else {
            stats.skipped += 1;
        }

        last_event_timestamp = Some(prepared.timestamp.clone());
        source_offset += bytes_read as u64;
        line_number += 1;
    }

    if rebuild {
        rebuild_sessions(&tx).map_err(|e| fail!(0, e))?;
    }

    save_state(
        &tx,
        path,
        file_size,
        modified_at,
        &fingerprint,
        source_offset as i64,
        last_event_timestamp.as_deref(),
    )
    .map_err(|e| fail!(0, e))?;

    tx.commit().map_err(|e| fail!(0, e))?;

    Ok(stats)
}

fn load_state(conn: &Connection, path: &Path) -> Result<Option<IngestionStateRow>> {
    conn.query_row(
        "SELECT file_size, fingerprint, byte_offset FROM ingestion_state WHERE source_file = ?1",
        params![path.display().to_string()],
        |row| {
            Ok(IngestionStateRow {
                file_size: row.get(0)?,
                fingerprint: row.get(1)?,
                byte_offset: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn file_fingerprint(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0_u8; 4096];
    let read = file.read(&mut buffer)?;
    Ok(format!("{:016x}", xx_hash_bytes(&buffer[..read])))
}

fn count_lines_before(path: &Path, upto: u64) -> Result<usize> {
    if upto == 0 {
        return Ok(0);
    }

    let mut file = File::open(path)?;
    let mut remaining = upto;
    let mut count = 0usize;
    let mut buffer = [0_u8; 8192];

    while remaining > 0 {
        let next = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = file.read(&mut buffer[..next])?;
        if read == 0 {
            break;
        }
        count += buffer[..read].iter().filter(|byte| **byte == b'\n').count();
        remaining -= read as u64;
    }

    Ok(count)
}

fn prepare_event(path: &Path, source_offset: i64, meta: EventMeta) -> Result<PreparedEvent> {
    let provider = meta.provider.as_slug().to_string();
    let source_file = path.display().to_string();
    let source_date = paths::daily_log_date_from_path(path)
        .unwrap_or_else(|| meta.timestamp.with_timezone(&Local).date_naive())
        .format("%Y-%m-%d")
        .to_string();
    let (session_key, anonymous_fallback) = session_key(&meta, &source_file, source_offset);

    let repo = meta.env.git.as_ref();
    let tool_input_json = meta
        .tool_input
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let tool_response_json = meta
        .tool_response
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    // Extract normalized token_usage and cost_usd from meta.extra
    let token_usage = meta.extra.get("token_usage");
    let input_tokens = token_usage
        .and_then(|v| v.get("input"))
        .and_then(Value::as_u64)
        .and_then(|v| i64::try_from(v).ok());
    let output_tokens = token_usage
        .and_then(|v| v.get("output"))
        .and_then(Value::as_u64)
        .and_then(|v| i64::try_from(v).ok());
    let total_tokens = token_usage
        .and_then(|v| v.get("total"))
        .and_then(Value::as_u64)
        .and_then(|v| i64::try_from(v).ok());
    let cache_read_tokens = token_usage
        .and_then(|v| v.get("cache_read"))
        .and_then(Value::as_u64)
        .and_then(|v| i64::try_from(v).ok());
    let cost_usd = meta.extra.get("cost_usd").and_then(Value::as_f64);

    let extra_json = serde_json::to_string(&meta.extra)?;
    let env_json = serde_json::to_string(&meta.env)?;

    let prepared = PreparedEvent {
        source_file,
        source_offset,
        source_date,
        timestamp: meta.timestamp.to_rfc3339(),
        provider,
        event: meta.event.as_slug().to_string(),
        session_key,
        session_id: meta.session_id.clone(),
        cwd: meta.cwd.clone(),
        repo_name: repo.and_then(|git| git.repo_name.clone()),
        repo_org: repo.and_then(|git| git.repo_org.clone()),
        branch: repo.and_then(|git| git.branch.clone()),
        primary_language: meta.env.primary_language.clone(),
        package_area: meta.env.package_area.clone(),
        package: meta.env.package.clone(),
        tool_name: meta.tool_name.clone(),
        agent_type: meta.agent_type.clone(),
        notification_type: meta.notification_type.clone(),
        notification_message: meta.notification_message.clone(),
        error: meta.error.clone(),
        model: extra_path_string(&meta.extra, &["model"]),
        permission_mode: extra_path_string(&meta.extra, &["permission_mode"]),
        head_sha: repo.and_then(|git| git.head_sha.clone()),
        is_dirty: repo.map(|git| i64::from(git.is_dirty)),
        memory_available_bytes: Some(
            i64::try_from(meta.env.hardware.memory_available_bytes).unwrap_or(i64::MAX),
        ),
        hostname: (!meta.env.os.hostname.is_empty()).then(|| meta.env.os.hostname.clone()),
        prompt_text: meta.prompt.clone(),
        tool_input_json,
        tool_response_json,
        input_tokens,
        output_tokens,
        total_tokens,
        cache_read_tokens,
        cost_usd,
        extra_json,
        env_json,
        anonymous_session_fallback: anonymous_fallback,
        interactive: meta
            .extra
            .get("interactive")
            .and_then(Value::as_str)
            .map(|v| v == "true"),
    };

    Ok(prepared)
}

fn session_key(meta: &EventMeta, source_file: &str, _source_offset: i64) -> (String, bool) {
    if let Some(session_id) = meta.session_id.as_deref()
        && !session_id.trim().is_empty()
    {
        return (format!("{}:{session_id}", meta.provider.as_slug()), false);
    }

    let fallback = [
        ["transcript_path"].as_slice(),
        ["thread_id"].as_slice(),
        ["thread-id"].as_slice(),
        ["session_path"].as_slice(),
        ["context_path"].as_slice(),
        ["wire_path"].as_slice(),
        ["conversation_id"].as_slice(),
        ["plugin_context", "session_id"].as_slice(),
        ["plugin_context", "sessionId"].as_slice(),
    ]
    .iter()
    .find_map(|path| extra_path_string(&meta.extra, path));

    if let Some(fallback) = fallback {
        return (format!("{}:{fallback}", meta.provider.as_slug()), false);
    }

    // No session identifier at all — group by provider + source file + date.
    // Using source_offset would make every single event its own "session",
    // massively inflating session counts.
    let date = paths::daily_log_date_from_path(std::path::Path::new(source_file))
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| {
            meta.timestamp
                .with_timezone(&chrono::Local)
                .date_naive()
                .format("%Y-%m-%d")
                .to_string()
        });
    (
        format!("{}:{source_file}:{date}", meta.provider.as_slug()),
        true,
    )
}

fn extra_path_string(
    extra: &std::collections::HashMap<String, Value>,
    path: &[&str],
) -> Option<String> {
    let mut current = extra.get(*path.first()?)?;
    for segment in &path[1..] {
        current = current.get(*segment)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn insert_event(tx: &Transaction<'_>, row: &PreparedEvent) -> Result<bool> {
    let inserted = tx.execute(
        r#"
        INSERT OR IGNORE INTO events (
            source_file, source_offset, source_date, timestamp, provider, event,
            session_key, session_id, cwd, repo_name, repo_org, branch, primary_language,
            package_area, package, tool_name, agent_type, notification_type,
            notification_message, error, model, permission_mode, head_sha, is_dirty,
            memory_available_bytes, hostname, prompt_text, tool_input_json,
            tool_response_json, input_tokens, output_tokens, total_tokens,
            cache_read_tokens, cost_usd, extra_json, env_json, anonymous_session
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25,
            ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?34, ?35, ?36, ?37
        )
        "#,
        params![
            row.source_file,
            row.source_offset,
            row.source_date,
            row.timestamp,
            row.provider,
            row.event,
            row.session_key,
            row.session_id,
            row.cwd,
            row.repo_name,
            row.repo_org,
            row.branch,
            row.primary_language,
            row.package_area,
            row.package,
            row.tool_name,
            row.agent_type,
            row.notification_type,
            row.notification_message,
            row.error,
            row.model,
            row.permission_mode,
            row.head_sha,
            row.is_dirty,
            row.memory_available_bytes,
            row.hostname,
            row.prompt_text,
            row.tool_input_json,
            row.tool_response_json,
            row.input_tokens,
            row.output_tokens,
            row.total_tokens,
            row.cache_read_tokens,
            row.cost_usd,
            row.extra_json,
            row.env_json,
            row.anonymous_session_fallback,
        ],
    )?;

    Ok(inserted > 0)
}

fn upsert_session(tx: &Transaction<'_>, row: &PreparedEvent) -> Result<()> {
    let turn_count = i64::from(row.event == "turn_complete");
    let tool_call_count = i64::from(row.event == "before_tool");
    let tool_error_count = i64::from(row.event == "tool_error");
    let turn_error_count = i64::from(row.event == "turn_error");
    let subagent_count = i64::from(row.event == "subagent_start");

    tx.execute(
        r#"
        INSERT INTO sessions (
            session_key, session_id, provider, started_at, ended_at, cwd, repo_name, repo_org,
            branch, package_area, package, model, permission_mode, hostname, primary_language,
            event_count, turn_count, tool_call_count, tool_error_count, turn_error_count,
            subagent_count, total_input_tokens, total_output_tokens, total_tokens,
            total_cache_read_tokens, total_cost_usd, interactive
        ) VALUES (
            ?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1,
            ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25
        )
        ON CONFLICT(session_key) DO UPDATE SET
            session_id = COALESCE(excluded.session_id, sessions.session_id),
            provider = excluded.provider,
            started_at = MIN(sessions.started_at, excluded.started_at),
            ended_at = MAX(sessions.ended_at, excluded.ended_at),
            cwd = COALESCE(excluded.cwd, sessions.cwd),
            repo_name = COALESCE(excluded.repo_name, sessions.repo_name),
            repo_org = COALESCE(excluded.repo_org, sessions.repo_org),
            branch = COALESCE(excluded.branch, sessions.branch),
            package_area = COALESCE(excluded.package_area, sessions.package_area),
            package = COALESCE(excluded.package, sessions.package),
            model = COALESCE(excluded.model, sessions.model),
            permission_mode = COALESCE(excluded.permission_mode, sessions.permission_mode),
            hostname = COALESCE(excluded.hostname, sessions.hostname),
            primary_language = COALESCE(excluded.primary_language, sessions.primary_language),
            interactive = COALESCE(excluded.interactive, sessions.interactive),
            event_count = sessions.event_count + 1,
            turn_count = sessions.turn_count + excluded.turn_count,
            tool_call_count = sessions.tool_call_count + excluded.tool_call_count,
            tool_error_count = sessions.tool_error_count + excluded.tool_error_count,
            turn_error_count = sessions.turn_error_count + excluded.turn_error_count,
            subagent_count = sessions.subagent_count + excluded.subagent_count,
            total_input_tokens = sessions.total_input_tokens + excluded.total_input_tokens,
            total_output_tokens = sessions.total_output_tokens + excluded.total_output_tokens,
            total_tokens = sessions.total_tokens + excluded.total_tokens,
            total_cache_read_tokens = sessions.total_cache_read_tokens + excluded.total_cache_read_tokens,
            total_cost_usd = sessions.total_cost_usd + excluded.total_cost_usd
        "#,
        params![
            row.session_key,
            row.session_id,
            row.provider,
            row.timestamp,
            row.cwd,
            row.repo_name,
            row.repo_org,
            row.branch,
            row.package_area,
            row.package,
            row.model,
            row.permission_mode,
            row.hostname,
            row.primary_language,
            turn_count,
            tool_call_count,
            tool_error_count,
            turn_error_count,
            subagent_count,
            row.input_tokens.unwrap_or(0),
            row.output_tokens.unwrap_or(0),
            row.total_tokens.unwrap_or(0),
            row.cache_read_tokens.unwrap_or(0),
            row.cost_usd.unwrap_or(0.0),
            row.interactive,
        ],
    )?;

    Ok(())
}

fn rebuild_sessions(tx: &Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM sessions", [])?;
    tx.execute_batch(
        r#"
        INSERT INTO sessions (
            session_key, session_id, provider, started_at, ended_at, cwd, repo_name, repo_org,
            branch, package_area, package, model, permission_mode, hostname, primary_language,
            event_count, turn_count, tool_call_count, tool_error_count, turn_error_count,
            subagent_count, total_input_tokens, total_output_tokens, total_tokens,
            total_cache_read_tokens, total_cost_usd, interactive
        )
        SELECT
            session_key,
            MIN(session_id),
            MIN(provider),
            MIN(timestamp),
            MAX(timestamp),
            MIN(cwd),
            MIN(repo_name),
            MIN(repo_org),
            MIN(branch),
            MIN(package_area),
            MIN(package),
            MAX(model),
            MAX(permission_mode),
            MIN(hostname),
            MIN(primary_language),
            COUNT(*),
            SUM(CASE WHEN event = 'turn_complete' THEN 1 ELSE 0 END),
            SUM(CASE WHEN event = 'before_tool' THEN 1 ELSE 0 END),
            SUM(CASE WHEN event = 'tool_error' THEN 1 ELSE 0 END),
            SUM(CASE WHEN event = 'turn_error' THEN 1 ELSE 0 END),
            SUM(CASE WHEN event = 'subagent_start' THEN 1 ELSE 0 END),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(total_tokens), 0),
            COALESCE(SUM(cache_read_tokens), 0),
            COALESCE(SUM(cost_usd), 0),
            MAX(CASE WHEN json_extract(extra_json, '$.interactive') = 'true' THEN 1
                     WHEN json_extract(extra_json, '$.interactive') = 'false' THEN 0
                     ELSE NULL END)
        FROM events
        GROUP BY session_key;
        "#,
    )?;
    Ok(())
}

fn save_state(
    tx: &Transaction<'_>,
    path: &Path,
    file_size: i64,
    modified_at: i64,
    fingerprint: &str,
    byte_offset: i64,
    last_event_timestamp: Option<&str>,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO ingestion_state (
            source_file, file_size, modified_at, fingerprint, byte_offset,
            last_event_timestamp, last_synced_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(source_file) DO UPDATE SET
            file_size = excluded.file_size,
            modified_at = excluded.modified_at,
            fingerprint = excluded.fingerprint,
            byte_offset = excluded.byte_offset,
            last_event_timestamp = excluded.last_event_timestamp,
            last_synced_at = excluded.last_synced_at
        "#,
        params![
            path.display().to_string(),
            file_size,
            modified_at,
            fingerprint,
            byte_offset,
            last_event_timestamp,
            Utc::now().to_rfc3339(),
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;
    use crate::events::{AgenticEvent, EventMeta, Provider};

    fn sample_meta() -> EventMeta {
        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::BeforeTool);
        meta.timestamp = Utc.with_ymd_and_hms(2026, 4, 3, 10, 0, 0).unwrap();
        meta
    }

    #[test]
    fn count_lines_before_counts_newlines_up_to_offset() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(&path, "one\ntwo\nthree\n").unwrap();

        let count = count_lines_before(&path, 8).unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn session_key_prefers_session_id_and_falls_back_to_extra_context() {
        let mut meta = sample_meta();
        meta.session_id = Some("session-123".to_string());
        assert_eq!(
            session_key(&meta, "/tmp/2026-04-03.jsonl", 0),
            ("claude:session-123".to_string(), false)
        );

        meta.session_id = None;
        meta.extra.insert(
            "plugin_context".to_string(),
            serde_json::json!({ "session_id": "plugin-456" }),
        );
        assert_eq!(
            session_key(&meta, "/tmp/2026-04-03.jsonl", 0),
            ("claude:plugin-456".to_string(), false)
        );
    }

    #[test]
    fn extra_path_string_reads_nested_values() {
        let mut extra = std::collections::HashMap::new();
        extra.insert(
            "plugin_context".to_string(),
            serde_json::json!({ "sessionId": "abc" }),
        );

        assert_eq!(
            extra_path_string(&extra, &["plugin_context", "sessionId"]),
            Some("abc".to_string())
        );
        assert_eq!(extra_path_string(&extra, &["missing"]), None);
    }

    fn open_store(dir: &tempfile::TempDir) -> (std::path::PathBuf, rusqlite::Connection) {
        let logs_dir = dir.path().join("logs");
        fs::create_dir_all(&logs_dir).unwrap();
        let db_path = dir.path().join("metrics.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        super::super::schema::initialize(&conn).unwrap();
        (logs_dir, conn)
    }

    /// Writing a single valid JSONL event and syncing inserts exactly one row.
    #[test]
    fn single_event_ingestion_inserts_one_row() {
        let dir = tempdir().unwrap();
        let (logs_dir, mut conn) = open_store(&dir);

        let meta = sample_meta();
        let log_path = logs_dir.join("2026-04-03.jsonl");
        fs::write(
            &log_path,
            format!("{}\n", serde_json::to_string(&meta).unwrap()),
        )
        .unwrap();

        let summary = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

        assert_eq!(summary.files_scanned, 1);
        assert_eq!(summary.events_inserted, 1);
        assert_eq!(summary.events_skipped, 0);
        assert_eq!(summary.parse_failures, 0);
    }

    /// Syncing the same file twice only inserts the new lines appended after
    /// the first sync (incremental ingestion by byte offset).
    ///
    /// The fingerprint covers the first 4096 bytes of the file, so the initial
    /// content must exceed that threshold to prevent a rebuild on append.
    #[test]
    fn incremental_ingestion_reads_only_new_lines() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let (logs_dir, mut conn) = open_store(&dir);

        // Build a padded event whose serialized form is > 4096 bytes so that
        // appending a second line does not change the fingerprint window.
        let make_meta = |session: &str, ts_hour: u32, extra_pad: Option<String>| {
            let mut m = sample_meta();
            m.session_id = Some(session.to_string());
            m.timestamp = Utc.with_ymd_and_hms(2026, 4, 3, ts_hour, 0, 0).unwrap();
            if let Some(pad) = extra_pad {
                m.extra
                    .insert("_pad".to_string(), serde_json::Value::String(pad));
            }
            m
        };

        // Pad the first event so serialized line1 is > 4096 bytes.
        let padding = "x".repeat(5000);
        let meta1 = make_meta("session-a", 10, Some(padding));
        let meta2 = make_meta("session-b", 11, None);

        let log_path = logs_dir.join("2026-04-03.jsonl");

        // First sync: one (large) event.
        let line1 = format!("{}\n", serde_json::to_string(&meta1).unwrap());
        assert!(line1.len() > 4096, "line1 must exceed fingerprint window");
        fs::write(&log_path, &line1).unwrap();
        let first = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();
        assert_eq!(first.events_inserted, 1, "first sync should insert 1 event");

        // Append a second event. Since the file already exceeds 4096 bytes, the
        // fingerprint (first 4096 bytes) is unchanged and no rebuild occurs.
        let line2 = format!("{}\n", serde_json::to_string(&meta2).unwrap());
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&log_path)
                .unwrap();
            file.write_all(line2.as_bytes()).unwrap();
        }
        let second = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

        assert_eq!(
            second.events_inserted, 1,
            "second sync should only insert the newly appended event"
        );
        assert_eq!(second.events_skipped, 0);
    }

    /// Malformed JSONL lines are recorded as parse failures but do not abort
    /// ingestion; valid lines around them are still inserted.
    #[test]
    fn malformed_line_is_recorded_as_failure_and_skipped() {
        let dir = tempdir().unwrap();
        let (logs_dir, mut conn) = open_store(&dir);

        let meta = sample_meta();
        let valid_line = serde_json::to_string(&meta).unwrap();
        let content = format!("{valid_line}\nnot-valid-json\n{valid_line}\n");
        let log_path = logs_dir.join("2026-04-03.jsonl");
        fs::write(&log_path, &content).unwrap();

        let summary = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

        assert_eq!(summary.parse_failures, 1, "one line should fail to parse");
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].line_number, 2);
        // The two valid events: one is inserted, the second is a duplicate
        // (same source_file + source_offset for duplicate detection via INSERT OR IGNORE).
        // Both valid lines are attempted; total processed = inserted + skipped.
        let total_processed = summary.events_inserted + summary.events_skipped;
        assert_eq!(total_processed, 2, "both valid lines should be processed");
    }

    /// An empty JSONL file does not produce errors and reports zero events.
    #[test]
    fn empty_file_produces_no_events() {
        let dir = tempdir().unwrap();
        let (logs_dir, mut conn) = open_store(&dir);

        let log_path = logs_dir.join("2026-04-03.jsonl");
        fs::write(&log_path, "").unwrap();

        let summary = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

        assert_eq!(summary.files_scanned, 1);
        assert_eq!(summary.events_inserted, 0);
        assert_eq!(summary.parse_failures, 0);
    }

    /// Non-JSONL files in the logs directory are silently ignored.
    #[test]
    fn non_jsonl_files_are_ignored() {
        let dir = tempdir().unwrap();
        let (logs_dir, mut conn) = open_store(&dir);

        fs::write(logs_dir.join("README.txt"), "not a log").unwrap();
        fs::write(logs_dir.join("metrics.db"), "also not a log").unwrap();

        let summary = sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

        assert_eq!(summary.files_scanned, 0);
        assert_eq!(summary.events_inserted, 0);
    }
}
