//! Opt-in raw NDJSON stream capture for post-mortem analysis of provider
//! stalls (notably OpenCode silence after subagent completion).
//!
//! Activated when the `CLAUDINE_RAW_STREAM_DIR` environment variable points
//! at a writable directory. While active, every line fed to the semantic
//! stream parser is mirrored to a per-spawn NDJSON file alongside a
//! sidecar `.meta.json` describing the run. Each captured line is wrapped
//! as `{"ts_ms": <elapsed-since-spawn>, "raw": "<original-line>"}` so a
//! reviewer can replay both content and timing without losing NDJSON
//! validity. Capture errors are debug-traced and never propagated — the
//! goal is diagnostic visibility, not a new failure mode.
//!
//! ## Examples
//!
//! ```bash
//! CLAUDINE_RAW_STREAM_DIR=~/claudine-traces \
//!   claudine opencode "say hi"
//! # later
//! jq -c '.ts_ms,.raw' ~/claudine-traces/opencode-20260510T184215-12345.ndjson
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Local;
use serde_json::json;

use claudine::provider::Provider;

/// Returns the configured raw-stream capture directory, if any. Trims and
/// strips a surrounding pair of quotes for ergonomics when users paste a
/// path with quotes.
fn capture_dir_from_env() -> Option<PathBuf> {
    let raw = std::env::var("CLAUDINE_RAW_STREAM_DIR").ok()?;
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }
    let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
        biscuit_file::home_dir().map(|h| h.join(rest))?
    } else {
        PathBuf::from(trimmed)
    };
    Some(expanded)
}

/// Per-spawn raw stream capture handle. Owned by the stdout reader thread.
pub(crate) struct StreamCapture {
    writer: BufWriter<File>,
    started_at: Instant,
    path: PathBuf,
    observation: Option<(super::super::run_scope::RunScope, super::super::run_scope::retention::CaptureFacts)>,
}

impl StreamCapture {
    /// Open a capture file for this child if `CLAUDINE_RAW_STREAM_DIR` is
    /// set and the directory is writable. Returns `None` otherwise.
    ///
    /// `started_at` should be the same `Instant` the spawn function uses
    /// as its wall-clock reference so the embedded `ts_ms` values align
    /// with the rest of the telemetry.
    pub(crate) fn open(provider: Option<Provider>, pid: u32, started_at: Instant) -> Option<Self> {
        let dir = capture_dir_from_env()?;
        if let Err(err) = fs::create_dir_all(&dir) {
            tracing::debug!(
                target: "claudine::stream_capture",
                error = %err,
                dir = %dir.display(),
                "failed to create raw stream capture directory; capture disabled"
            );
            return None;
        }

        let provider_slug = provider
            .map(|p| p.as_slug().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let timestamp = Local::now().format("%Y%m%dT%H%M%S");
        let filename = format!("{provider_slug}-{timestamp}-{pid}.ndjson");
        let path = dir.join(filename);

        let file = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => f,
            Err(err) => {
                tracing::debug!(
                    target: "claudine::stream_capture",
                    error = %err,
                    path = %path.display(),
                    "failed to open raw stream capture file; capture disabled"
                );
                return None;
            }
        };

        write_meta_sidecar(&path, provider_slug.as_str(), pid);

        tracing::debug!(
            target: "claudine::stream_capture",
            path = %path.display(),
            "raw stream capture enabled"
        );

        Some(Self {
            writer: BufWriter::new(file),
            started_at,
            path,
            observation: None,
        })
    }

    pub(crate) fn observe_with(&mut self, scope: super::super::run_scope::RunScope) {
        let facts = super::super::run_scope::retention::CaptureFacts {
            path: self.path.to_string_lossy().into_owned(),
            // The envelope omits original delimiters; flush cannot prove byte coverage.
            omitted: true,
            ..Default::default()
        };
        scope.capture(facts.clone());
        self.observation = Some((scope, facts));
    }

    pub(crate) fn reached_eof(&mut self) {
        if let Some((scope, facts)) = self.observation.as_mut() {
            facts.eof = true;
            scope.capture(facts.clone());
        }
    }

    /// Write a single raw line to the capture file, prefixed with elapsed
    /// milliseconds since spawn. Whitespace-only lines are skipped to
    /// match the byte-heartbeat semantics in spawn.rs.
    pub(crate) fn record_line(&mut self, line: &str, line_at: Instant) {
        if line.chars().all(char::is_whitespace) {
            return;
        }
        let ts_ms = line_at
            .saturating_duration_since(self.started_at)
            .as_millis() as u64;
        let entry = json!({ "ts_ms": ts_ms, "raw": line });
        let mut serialized = match serde_json::to_string(&entry) {
            Ok(s) => s,
            Err(err) => {
                tracing::debug!(
                    target: "claudine::stream_capture",
                    error = %err,
                    "failed to serialize raw stream capture entry"
                );
                return;
            }
        };
        serialized.push('\n');
        if let Err(err) = self.writer.write_all(serialized.as_bytes()) {
            if let Some((scope, facts)) = self.observation.as_mut() {
                facts.failed = true;
                scope.capture(facts.clone());
            }
            tracing::debug!(
                target: "claudine::stream_capture",
                error = %err,
                path = %self.path.display(),
                "failed to write raw stream capture entry"
            );
        }
    }
}

impl Drop for StreamCapture {
    fn drop(&mut self) {
        let flushed = self.writer.flush();
        if let Some((scope, facts)) = self.observation.as_mut() {
            facts.flushed = flushed.is_ok();
            facts.failed |= flushed.is_err();
            scope.capture(facts.clone());
        }
        if let Err(err) = flushed {
            tracing::debug!(
                target: "claudine::stream_capture",
                error = %err,
                path = %self.path.display(),
                "failed to flush raw stream capture on drop"
            );
        }
    }
}

fn write_meta_sidecar(ndjson_path: &Path, provider_slug: &str, pid: u32) {
    let meta_path = ndjson_path.with_extension("meta.json");
    let meta = json!({
        "provider": provider_slug,
        "pid": pid,
        "started_at": Local::now().to_rfc3339(),
        "claudine_version": env!("CARGO_PKG_VERSION"),
    });
    if let Err(err) = fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    ) {
        tracing::debug!(
            target: "claudine::stream_capture",
            error = %err,
            path = %meta_path.display(),
            "failed to write raw stream capture meta sidecar"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn open_returns_none_when_env_var_unset() {
        // SAFETY: tests are serialized via `serial_test`.
        unsafe { std::env::remove_var("CLAUDINE_RAW_STREAM_DIR") };
        let capture = StreamCapture::open(Some(Provider::OpenCode), 1234, Instant::now());
        assert!(capture.is_none(), "capture must be disabled by default");
    }

    #[test]
    #[serial]
    fn capture_writes_ndjson_with_elapsed_ms() {
        let dir = tempdir().unwrap();
        // SAFETY: single-threaded test path; we restore the env var after.
        let prev = std::env::var("CLAUDINE_RAW_STREAM_DIR").ok();
        unsafe {
            std::env::set_var("CLAUDINE_RAW_STREAM_DIR", dir.path());
        }

        let started_at = Instant::now();
        let mut capture =
            StreamCapture::open(Some(Provider::OpenCode), 42, started_at).expect("capture opens");
        capture.record_line(r#"{"type":"step_start","sessionID":"ses_1"}"#, started_at);
        capture.record_line(
            r#"{"type":"text","text":"hello"}"#,
            started_at + Duration::from_millis(125),
        );
        // Whitespace-only line is skipped.
        capture.record_line("   ", started_at + Duration::from_millis(200));
        drop(capture);

        unsafe {
            match prev {
                Some(v) => std::env::set_var("CLAUDINE_RAW_STREAM_DIR", v),
                None => std::env::remove_var("CLAUDINE_RAW_STREAM_DIR"),
            }
        }

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        let ndjson = entries
            .iter()
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("ndjson"))
            .expect("ndjson capture file");
        let body = fs::read_to_string(ndjson).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2, "whitespace line must be skipped: {body:?}");
        assert!(
            lines[0].contains("\"ts_ms\":0") && lines[0].contains("step_start"),
            "first entry: {}",
            lines[0]
        );
        assert!(
            lines[1].contains("\"ts_ms\":125") && lines[1].contains("hello"),
            "second entry: {}",
            lines[1]
        );

        let meta = entries
            .iter()
            .find(|p| {
                p.file_name().and_then(|n| n.to_str())
                    == Some(
                        ndjson
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap()
                            .replace(".ndjson", ".meta.json")
                            .as_str(),
                    )
            })
            .or_else(|| {
                entries
                    .iter()
                    .find(|p| p.to_string_lossy().ends_with(".meta.json"))
            })
            .expect("meta sidecar");
        let meta_body = fs::read_to_string(meta).unwrap();
        assert!(
            meta_body.contains("\"provider\""),
            "meta has provider: {meta_body}"
        );
        assert!(
            meta_body.contains("\"pid\": 42"),
            "meta has pid: {meta_body}"
        );
    }

    #[test]
    #[serial]
    fn existing_capture_is_visible_without_flushing_the_reader() {
        let dir = tempdir().unwrap();
        let _env = test_toolkit::EnvGuard::set_safe("CLAUDINE_RAW_STREAM_DIR", dir.path().to_str().unwrap());
        let scope = super::super::super::run_scope::RunScope::default();
        let mut capture = StreamCapture::open(Some(Provider::Codex), 43, Instant::now()).unwrap();
        capture.observe_with(scope.clone());
        scope.retain_raw(&vec![b'x'; super::super::super::run_scope::retention::INLINE_LIMIT + 1]);
        capture.record_line("{}", Instant::now());
        let frozen = scope.freeze();
        assert!(Path::new(frozen.retained.raw_output_path.as_deref().unwrap()).exists());
        assert_eq!(frozen.retained.raw_output_complete, Some(false));
        capture.reached_eof();
        drop(capture);
        assert_eq!(scope.freeze(), frozen);
    }

    #[test]
    #[serial]
    fn failed_capture_flush_never_upgrades_raw_completeness() {
        let dir = tempdir().unwrap();
        let _env = test_toolkit::EnvGuard::set_safe("CLAUDINE_RAW_STREAM_DIR", dir.path().to_str().unwrap());
        let scope = super::super::super::run_scope::RunScope::default();
        let mut capture = StreamCapture::open(Some(Provider::Codex), 44, Instant::now()).unwrap();
        capture.writer = BufWriter::new(File::open(&capture.path).unwrap());
        capture.observe_with(scope.clone());
        scope.retain_raw(&vec![b'x'; super::super::super::run_scope::retention::INLINE_LIMIT + 1]);
        capture.record_line("{}", Instant::now());
        capture.reached_eof();
        drop(capture);
        let frozen = scope.freeze();
        assert!(frozen.retained.raw_output_path.is_some());
        assert_eq!(frozen.retained.raw_output_complete, Some(false));
    }

}
