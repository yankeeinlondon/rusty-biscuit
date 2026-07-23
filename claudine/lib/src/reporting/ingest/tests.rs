use std::fs;

use chrono::TimeZone;
use tempfile::tempdir;

use super::*;
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;
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

/// Phase 4 — `claudine_pid` and `agent_pid` are written to the events table.
#[test]
fn pid_fields_are_ingested_into_events() {
    let dir = tempdir().unwrap();
    let (logs_dir, mut conn) = open_store(&dir);

    let mut meta = sample_meta();
    meta.env.claudine_pid = Some(11_111);
    meta.agent_pid = Some(22_222);

    let log_path = logs_dir.join("2026-04-03.jsonl");
    fs::write(
        &log_path,
        format!("{}\n", serde_json::to_string(&meta).unwrap()),
    )
    .unwrap();

    sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

    let (claudine_pid, agent_pid): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT claudine_pid, agent_pid FROM events LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(claudine_pid, Some(11_111));
    assert_eq!(agent_pid, Some(22_222));
}

/// Phase 4 — missing `agent_pid` is stored as NULL, not fabricated.
#[test]
fn missing_agent_pid_is_null_in_database() {
    let dir = tempdir().unwrap();
    let (logs_dir, mut conn) = open_store(&dir);

    let mut meta = sample_meta();
    meta.env.claudine_pid = Some(11_111);
    meta.agent_pid = None;

    let log_path = logs_dir.join("2026-04-03.jsonl");
    fs::write(
        &log_path,
        format!("{}\n", serde_json::to_string(&meta).unwrap()),
    )
    .unwrap();

    sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

    let (claudine_pid, agent_pid): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT claudine_pid, agent_pid FROM events LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(claudine_pid, Some(11_111));
    assert_eq!(agent_pid, None);
}

/// Signals are only exploded from SessionEnd rows — a stray
/// `signals` key on any other event never fabricates signal history.
#[test]
fn signals_on_non_session_end_rows_are_ignored() {
    let dir = tempdir().unwrap();
    let (logs_dir, mut conn) = open_store(&dir);

    let mut meta = sample_meta();
    meta.extra.insert(
        "signals".to_string(),
        serde_json::json!([{
            "kind": "model_catalog_drift",
            "source": "wrapper",
            "occurrences": 1,
            "first_seen": "2026-04-03T10:00:00Z",
            "event": { "kind": "model_catalog_drift", "unexpected": [], "missing": [], "observed_via": "listing" }
        }]),
    );

    let log_path = logs_dir.join("2026-04-03.jsonl");
    fs::write(
        &log_path,
        format!("{}\n", serde_json::to_string(&meta).unwrap()),
    )
    .unwrap();

    sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM session_signals", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

/// Phase 4 — session aggregation surfaces the maximum PID values.
#[test]
fn pid_fields_are_aggregated_into_sessions() {
    let dir = tempdir().unwrap();
    let (logs_dir, mut conn) = open_store(&dir);

    let mut meta1 = sample_meta();
    meta1.session_id = Some("session-a".to_string());
    meta1.env.claudine_pid = Some(11_111);
    meta1.agent_pid = Some(22_222);

    let mut meta2 = sample_meta();
    meta2.session_id = Some("session-a".to_string());
    meta2.env.claudine_pid = Some(11_111);
    meta2.agent_pid = Some(33_333);

    let log_path = logs_dir.join("2026-04-03.jsonl");
    fs::write(
        &log_path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&meta1).unwrap(),
            serde_json::to_string(&meta2).unwrap()
        ),
    )
    .unwrap();

    sync(&mut conn, &logs_dir, crate::reporting::SyncRequest::All).unwrap();

    let (claudine_pid, agent_pid): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT claudine_pid, agent_pid FROM sessions LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(claudine_pid, Some(11_111));
    assert_eq!(agent_pid, Some(33_333));
}
