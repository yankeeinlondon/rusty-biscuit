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
    /// Claudine's own process ID, captured once at wrapper startup.
    claudine_pid: Option<i64>,
    /// Immediate child PID returned by the wrapper spawn operation.
    agent_pid: Option<i64>,
    /// `extra["family_latest"]` alias stamp lifted from SessionEnd rows.
    family_latest_json: Option<String>,
    /// `extra["signals"]` observations exploded from SessionEnd rows.
    signals: Vec<PreparedSignal>,
}

/// One `session_signals` row exploded from a SessionEnd summary event.
#[derive(Debug)]
struct PreparedSignal {
    /// Position within the summary row's `signals` array; part of the
    /// primary key so re-ingesting a line stays idempotent.
    signal_index: i64,
    /// The signal's own `first_seen` when present, else the event
    /// timestamp — signals can predate the summary row by minutes.
    timestamp: String,
    kind: String,
    source: String,
    occurrences: i64,
    /// The full signal object, `event` payload included.
    payload_json: String,
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
        tx.execute(
            "DELETE FROM session_signals WHERE source_file = ?1",
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
            insert_session_signals(&tx, &prepared).map_err(|e| fail!(line_number, e))?;

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

    // Signals and the family_latest stamp are only written by the
    // SessionEnd summary builder; gate on the event so stray keys on
    // other rows can never fabricate signal history.
    let (family_latest_json, signals) = if meta.event == crate::events::AgenticEvent::SessionEnd {
        (
            meta.extra
                .get("family_latest")
                .filter(|value| value.is_object())
                .map(serde_json::to_string)
                .transpose()?,
            prepare_signals(&meta),
        )
    } else {
        (None, Vec::new())
    };

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
        claudine_pid: meta.env.claudine_pid.map(i64::from),
        agent_pid: meta.agent_pid.map(i64::from),
        family_latest_json,
        signals,
    };

    Ok(prepared)
}

/// Explode `extra["signals"]` into `session_signals` row material.
///
/// Tolerant by design: entries missing a string `kind` are skipped (the
/// full array still lives in `extra_json` for forensics), and missing
/// `source`/`occurrences`/`first_seen` fall back to `unknown`/`1`/the
/// event timestamp rather than failing the line.
fn prepare_signals(meta: &EventMeta) -> Vec<PreparedSignal> {
    let Some(entries) = meta.extra.get("signals").and_then(Value::as_array) else {
        return Vec::new();
    };

    entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let kind = entry.get("kind").and_then(Value::as_str)?.to_string();
            Some(PreparedSignal {
                signal_index: index as i64,
                timestamp: entry
                    .get("first_seen")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| meta.timestamp.to_rfc3339()),
                kind,
                source: entry
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                occurrences: entry
                    .get("occurrences")
                    .and_then(Value::as_i64)
                    .unwrap_or(1),
                payload_json: entry.to_string(),
            })
        })
        .collect()
}

/// Insert the prepared event's exploded signal rows (no-op when empty).
fn insert_session_signals(tx: &Transaction<'_>, row: &PreparedEvent) -> Result<()> {
    for signal in &row.signals {
        tx.execute(
            r#"
            INSERT OR IGNORE INTO session_signals (
                source_file, source_offset, signal_index, session_key, session_id,
                provider, timestamp, source_date, kind, source, occurrences, payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                row.source_file,
                row.source_offset,
                signal.signal_index,
                row.session_key,
                row.session_id,
                row.provider,
                signal.timestamp,
                row.source_date,
                signal.kind,
                signal.source,
                signal.occurrences,
                signal.payload_json,
            ],
        )?;
    }
    Ok(())
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
            cache_read_tokens, cost_usd, extra_json, env_json, anonymous_session,
            claudine_pid, agent_pid
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25,
            ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?34, ?35, ?36, ?37,
            ?38, ?39
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
            row.claudine_pid,
            row.agent_pid,
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
            total_cache_read_tokens, total_cost_usd, interactive, claudine_pid, agent_pid,
            family_latest
        ) VALUES (
            ?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7,
            ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1,
            ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25,
            ?26, ?27, ?28
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
            claudine_pid = COALESCE(excluded.claudine_pid, sessions.claudine_pid),
            agent_pid = COALESCE(excluded.agent_pid, sessions.agent_pid),
            family_latest = COALESCE(excluded.family_latest, sessions.family_latest),
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
            row.claudine_pid,
            row.agent_pid,
            row.family_latest_json,
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
            total_cache_read_tokens, total_cost_usd, interactive, claudine_pid, agent_pid,
            family_latest
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
                     ELSE NULL END),
            MAX(claudine_pid),
            MAX(agent_pid),
            -- json_extract yields the stamp object's JSON text; MAX picks
            -- the single SessionEnd stamp (NULL rows lose).
            MAX(CASE WHEN event = 'session_end'
                     THEN json_extract(extra_json, '$.family_latest')
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
mod tests;
