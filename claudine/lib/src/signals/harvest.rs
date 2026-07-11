//! Opt-in harvest of unmatched error/warning-class signal payloads
//! (design `signal-detection.md` §Harvest, v1 scope).
//!
//! When harvesting is enabled (`harvest_unmatched` in
//! `~/.claudine/config.json`, or the `CLAUDINE_HARVEST` env override), the
//! per-run [`SignalHub`] buffers every payload that fired ZERO detection
//! records (declarative and bespoke) and passes the deliberately minimal
//! [`is_error_or_warning_class`] test. Candidates are scrubbed through
//! [`crate::protect::scrub`] before they ever enter the buffer, and the
//! wrapper appends them to `~/.claudine/harvest/<provider>/<YYYY-MM-DD>.jsonl`
//! at end of run via [`flush_hub`].
//!
//! Harvested files are candidate evidence only. Promotion into
//! `docs/research/signals/fixtures/` is a HUMAN-REVIEWED process step: a
//! promoted fixture enters `provenance.yaml` as provenance class `capture`
//! (see `docs/research/signals/fixtures/README.md`, curation rules 4-5).
//! The promotion is a documented process fact — never automated.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use claudine_catalog_types::SignalSource;
use serde::Serialize;
use serde_json::Value;

use super::SignalHub;

/// Per-run candidate cap. Beyond this the buffer stops growing and each
/// further candidate only bumps the dropped counter — a run that produces
/// hundreds of unmatched errors is telling us about ONE gap, not hundreds.
pub const MAX_HARVEST_ENTRIES_PER_RUN: usize = 100;

/// Harvest files whose date-stamped name is older than this many days are
/// deleted on every flush.
pub const HARVEST_MAX_AGE_DAYS: u64 = 30;

/// Whole-tree size cap for `~/.claudine/harvest/`. When exceeded after age
/// pruning, oldest files are deleted first until the tree fits.
pub const HARVEST_MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024;

/// One scrubbed unmatched-payload capture, serialized as a JSONL line.
#[derive(Debug, Clone, Serialize)]
pub struct HarvestCandidate {
    /// RFC 3339 capture timestamp (UTC).
    pub captured_at: String,
    /// Provider slug the observing hub was compiled for.
    pub provider: &'static str,
    /// Snake-case [`SignalSource`] name of the observing channel.
    pub source: &'static str,
    /// The scrubbed payload (see [`crate::protect::scrub`]).
    pub payload: Value,
}

/// The hub-owned per-run buffer. Exists only when harvesting was opted in,
/// so the disabled path never evaluates the predicate or clones payloads.
#[derive(Debug)]
pub(super) struct HarvestBuffer {
    provider: &'static str,
    entries: Vec<HarvestCandidate>,
    dropped: usize,
}

impl HarvestBuffer {
    pub(super) fn new(provider: &'static str) -> Self {
        Self {
            provider,
            entries: Vec::new(),
            dropped: 0,
        }
    }

    /// Buffer `payload` if it is error/warning-class; scrubbing happens
    /// here so raw secrets never sit in the buffer.
    pub(super) fn consider(&mut self, source: SignalSource, payload: &Value) {
        if !is_error_or_warning_class(source, payload) {
            return;
        }
        if self.entries.len() >= MAX_HARVEST_ENTRIES_PER_RUN {
            self.dropped += 1;
            return;
        }
        let mut scrubbed = payload.clone();
        crate::protect::scrub::scrub_json_value(&mut scrubbed);
        self.entries.push(HarvestCandidate {
            captured_at: chrono::Utc::now().to_rfc3339(),
            provider: self.provider,
            source: source.into(),
            payload: scrubbed,
        });
    }

    /// Take everything buffered so far, leaving harvesting enabled with an
    /// empty buffer.
    pub(super) fn take_batch(&mut self) -> HarvestBatch {
        HarvestBatch {
            provider: self.provider,
            entries: std::mem::take(&mut self.entries),
            dropped: std::mem::take(&mut self.dropped),
        }
    }
}

/// One run's harvested candidates, handed from the hub to persistence.
#[derive(Debug)]
pub struct HarvestBatch {
    /// Provider slug (the `~/.claudine/harvest/<provider>/` subdirectory).
    pub provider: &'static str,
    /// Buffered candidates, in observation order.
    pub entries: Vec<HarvestCandidate>,
    /// Candidates discarded after [`MAX_HARVEST_ENTRIES_PER_RUN`] filled up.
    pub dropped: usize,
}

/// Deliberately minimal v1 error/warning-class test. A JSON object
/// qualifies when ANY of:
///
/// - a top-level `type`/`subtype`/`level`/`severity`/`status` string
///   contains `error`, `warn`, or `fatal` (case-insensitive);
/// - `is_error == true`;
/// - a top-level `error` key is present (whatever its value);
/// - for [`SignalSource::Exit`] payloads, `exit_code != 0`.
///
/// Nothing else — no recursive shape heuristics. The looser
/// shape-recognizer is explicitly future work, not v1.
pub fn is_error_or_warning_class(source: SignalSource, payload: &Value) -> bool {
    let Some(map) = payload.as_object() else {
        return false;
    };
    if source == SignalSource::Exit
        && map
            .get("exit_code")
            .and_then(Value::as_i64)
            .is_some_and(|code| code != 0)
    {
        return true;
    }
    if map.get("is_error").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    if map.contains_key("error") {
        return true;
    }
    ["type", "subtype", "level", "severity", "status"]
        .iter()
        .any(|key| {
            map.get(*key).and_then(Value::as_str).is_some_and(|text| {
                let lower = text.to_ascii_lowercase();
                ["error", "warn", "fatal"]
                    .iter()
                    .any(|marker| lower.contains(marker))
            })
        })
}

/// The `CLAUDINE_HARVEST` env override: `1`/`true` forces harvesting on,
/// `0`/`false` forces it off (winning over `harvest_unmatched` in the user
/// config either way). Unset or unparsable values defer to config.
pub fn env_override() -> Option<bool> {
    parse_env_flag(&std::env::var("CLAUDINE_HARVEST").ok()?)
}

fn parse_env_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// End-of-run persistence: take the hub's harvest buffer and append it
/// under `~/.claudine/harvest/`, then enforce retention over the whole
/// tree. No-op when harvesting is disabled or nothing was captured.
///
/// Harvest must never break a run: every failure path is a `warn!`, never
/// an error to the caller.
pub fn flush_hub(hub: &SignalHub) {
    let Some(batch) = hub.take_harvest() else {
        return;
    };
    if batch.entries.is_empty() {
        return;
    }
    let base_dir = match crate::reporting::paths::claudine_home_dir() {
        Ok(home) => home.join("harvest"),
        Err(error) => {
            tracing::warn!(%error, "harvest: cannot resolve ~/.claudine; discarding candidates");
            return;
        }
    };
    let today = chrono::Local::now().date_naive();
    match persist_batch(&batch, &base_dir, today) {
        Ok(path) => tracing::debug!(
            harvested = batch.entries.len(),
            dropped = batch.dropped,
            path = %path.display(),
            "harvest: unmatched candidates persisted"
        ),
        Err(error) => {
            tracing::warn!(%error, "harvest: failed to persist candidates");
            return;
        }
    }
    if let Err(error) = enforce_retention(&base_dir, today) {
        tracing::warn!(%error, "harvest: retention enforcement failed");
    }
}

/// Append `batch` as JSONL to `<base_dir>/<provider>/<date>.jsonl`,
/// creating directories as needed. Returns the file written.
pub fn persist_batch(
    batch: &HarvestBatch,
    base_dir: &Path,
    date: NaiveDate,
) -> std::io::Result<PathBuf> {
    use std::io::Write;

    let dir = base_dir.join(batch.provider);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    for candidate in &batch.entries {
        let line = serde_json::to_string(candidate).map_err(std::io::Error::other)?;
        writeln!(file, "{line}")?;
    }
    Ok(path)
}

/// Enforce the pinned retention policy over the harvest tree: age pruning
/// ([`HARVEST_MAX_AGE_DAYS`]) first, then oldest-first size pruning down to
/// [`HARVEST_MAX_TOTAL_BYTES`].
pub fn enforce_retention(base_dir: &Path, today: NaiveDate) -> std::io::Result<()> {
    enforce_retention_with(
        base_dir,
        today,
        HARVEST_MAX_AGE_DAYS,
        HARVEST_MAX_TOTAL_BYTES,
    )
}

/// [`enforce_retention`] with injectable limits (the testing seam).
fn enforce_retention_with(
    base_dir: &Path,
    today: NaiveDate,
    max_age_days: u64,
    max_total_bytes: u64,
) -> std::io::Result<()> {
    if !base_dir.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    collect_harvest_files(base_dir, &mut files)?;

    let cutoff = today - chrono::Days::new(max_age_days);
    files.retain(|file| {
        if file.date.is_some_and(|date| date < cutoff) {
            if let Err(error) = std::fs::remove_file(&file.path) {
                tracing::warn!(%error, path = %file.path.display(), "harvest: age prune failed");
            }
            false
        } else {
            true
        }
    });

    let mut total: u64 = files.iter().map(|file| file.len).sum();
    if total <= max_total_bytes {
        return Ok(());
    }
    // Undated files sort last (NaiveDate::MAX) so an unexpected stray file
    // is never the first thing deleted.
    files.sort_by(|a, b| {
        (a.date.unwrap_or(NaiveDate::MAX), &a.path)
            .cmp(&(b.date.unwrap_or(NaiveDate::MAX), &b.path))
    });
    for file in files {
        if total <= max_total_bytes {
            break;
        }
        match std::fs::remove_file(&file.path) {
            Ok(()) => total = total.saturating_sub(file.len),
            Err(error) => {
                tracing::warn!(%error, path = %file.path.display(), "harvest: size prune failed");
            }
        }
    }
    Ok(())
}

struct HarvestFile {
    path: PathBuf,
    len: u64,
    /// Parsed from the `YYYY-MM-DD.jsonl` filename; `None` for stray files.
    date: Option<NaiveDate>,
}

fn collect_harvest_files(dir: &Path, out: &mut Vec<HarvestFile>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_harvest_files(&path, out)?;
        } else {
            out.push(HarvestFile {
                date: crate::reporting::paths::daily_log_date_from_path(&path),
                len: metadata.len(),
                path,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use claudine_catalog_types::SignalSource;
    use serde_json::json;

    use super::super::{bespoke::exit_source_payload, detection_table};
    use super::*;

    // ---------------------------------------------------------------------
    // Predicate
    // ---------------------------------------------------------------------

    #[test]
    fn predicate_fires_on_each_class_key() {
        for key in ["type", "subtype", "level", "severity", "status"] {
            for marker in ["error", "Warning", "FATAL", "tool_error"] {
                let payload = json!({ key: marker });
                assert!(
                    is_error_or_warning_class(SignalSource::Stream, &payload),
                    "{key}={marker} should qualify"
                );
            }
        }
    }

    #[test]
    fn predicate_fires_on_is_error_true_and_error_key() {
        assert!(is_error_or_warning_class(
            SignalSource::Stream,
            &json!({ "type": "result", "is_error": true })
        ));
        assert!(is_error_or_warning_class(
            SignalSource::Stream,
            &json!({ "type": "assistant", "error": { "code": 42 } })
        ));
    }

    #[test]
    fn predicate_fires_on_nonzero_exit_only_for_exit_source() {
        let payload = exit_source_payload(53, "", "boom");
        assert!(is_error_or_warning_class(SignalSource::Exit, &payload));
        assert!(
            !is_error_or_warning_class(SignalSource::Stream, &payload),
            "exit_code rule is Exit-source-only"
        );
        assert!(!is_error_or_warning_class(
            SignalSource::Exit,
            &exit_source_payload(0, "", "")
        ));
    }

    #[test]
    fn predicate_ignores_benign_payloads() {
        for payload in [
            json!({ "type": "result", "is_error": false, "usage": { "input_tokens": 5 } }),
            json!({ "type": "assistant", "message": "hello" }),
            json!({ "level": "info", "message": "starting" }),
            json!("bare string"),
            json!(42),
        ] {
            assert!(
                !is_error_or_warning_class(SignalSource::Stream, &payload),
                "should not qualify: {payload}"
            );
        }
    }

    #[test]
    fn env_flag_parsing() {
        assert_eq!(parse_env_flag("1"), Some(true));
        assert_eq!(parse_env_flag("true"), Some(true));
        assert_eq!(parse_env_flag("TRUE"), Some(true));
        assert_eq!(parse_env_flag("0"), Some(false));
        assert_eq!(parse_env_flag("false"), Some(false));
        assert_eq!(parse_env_flag("yes"), None);
        assert_eq!(parse_env_flag(""), None);
    }

    // ---------------------------------------------------------------------
    // Persistence
    // ---------------------------------------------------------------------

    fn candidate(n: u64) -> HarvestCandidate {
        HarvestCandidate {
            captured_at: "2026-07-06T00:00:00+00:00".to_string(),
            provider: "claude",
            source: "stream",
            payload: json!({ "level": "error", "n": n }),
        }
    }

    #[test]
    fn persist_batch_appends_jsonl_under_provider_and_date() {
        let tmp = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let batch = HarvestBatch {
            provider: "claude",
            entries: vec![candidate(1), candidate(2)],
            dropped: 0,
        };
        let path = persist_batch(&batch, tmp.path(), date).unwrap();
        assert!(path.ends_with("claude/2026-07-06.jsonl"), "path: {path:?}");

        // Second flush on the same day appends rather than truncating.
        persist_batch(&batch, tmp.path(), date).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        let first: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["provider"], "claude");
        assert_eq!(first["source"], "stream");
        assert_eq!(first["payload"]["n"], 1);
        assert!(first["captured_at"].is_string());
    }

    // ---------------------------------------------------------------------
    // Retention
    // ---------------------------------------------------------------------

    fn write_file(base: &Path, provider: &str, name: &str, bytes: usize) -> PathBuf {
        let dir = base.join(provider);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, vec![b'x'; bytes]).unwrap();
        path
    }

    #[test]
    fn retention_prunes_by_age_then_size_oldest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let ancient = write_file(tmp.path(), "claude", "2026-05-01.jsonl", 10);
        let old = write_file(tmp.path(), "codex", "2026-07-01.jsonl", 60);
        let mid = write_file(tmp.path(), "codex", "2026-07-03.jsonl", 60);
        let new = write_file(tmp.path(), "claude", "2026-07-06.jsonl", 60);

        // Age prune removes `ancient` (older than 30 days). The remaining
        // 180 bytes exceed the 150-byte budget, so the oldest survivor
        // (`old`) is pruned; deleting it lands under budget.
        enforce_retention_with(tmp.path(), today, 30, 150).unwrap();

        assert!(!ancient.exists(), "age-pruned");
        assert!(!old.exists(), "size-pruned oldest-first");
        assert!(mid.exists());
        assert!(new.exists());
    }

    #[test]
    fn retention_is_a_noop_under_budget_and_on_missing_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let recent = write_file(tmp.path(), "claude", "2026-07-05.jsonl", 10);
        enforce_retention_with(tmp.path(), today, 30, 1000).unwrap();
        assert!(recent.exists());

        enforce_retention_with(&tmp.path().join("does-not-exist"), today, 30, 1000).unwrap();
    }

    // ---------------------------------------------------------------------
    // Hub-to-disk integration (the drain-helper-level smoke test)
    // ---------------------------------------------------------------------

    #[test]
    fn unmatched_error_payload_flows_from_hub_to_jsonl() {
        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        hub.enable_harvest();
        // No claude record matches a bare `level` payload, so this is
        // unmatched; `level: error` makes it harvest-eligible.
        hub.observe_json(
            SignalSource::Stream,
            &json!({ "level": "error", "message": "unrecognized failure shape" }),
        );

        let batch = hub.take_harvest().expect("harvest enabled");
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.dropped, 0);

        let tmp = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let path = persist_batch(&batch, tmp.path(), date).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(line["provider"], "claude");
        assert_eq!(line["payload"]["message"], "unrecognized failure shape");
    }
}
