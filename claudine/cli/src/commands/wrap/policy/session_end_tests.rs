//! The session-end record a structured run stores: written once, after the
//! final drain, with the run's reader warnings and its delivery loss.

use super::*;
use crate::commands::wrap::output_worker::tests::{Gate, wedge_worker};
use crate::commands::wrap::section::SectionStream;
use crate::commands::wrap::stream_io::StreamOutput;
use claudine::events::{AgenticEvent, EventMeta};
use claudine::provider::Provider;
use claudine::stream::summary::StreamExecutionSummary;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The warning `settle_parser` gives a stdout reader cut off after the
/// result, built from the production cause text.
pub(crate) fn reader_timeout_warning() -> String {
    use crate::commands::wrap::exec::reader_join::{
        JoinOutcome, ReaderBudget, ReaderStall, ReaderStream, reader_failure,
    };
    let failure = reader_failure(
        ReaderStream::Output,
        &JoinOutcome::<()>::TimedOut(ReaderStall::Processing),
        ReaderBudget::default(),
    )
    .unwrap();
    format!("{}. The run's result is kept; its output may be incomplete", failure.message)
}

/// A private home for the record, so the test reads only its own rows.
pub(crate) struct RecordHome {
    dir: tempfile::TempDir,
    _home: test_toolkit::EnvGuard,
    _profile: test_toolkit::EnvGuard,
}

impl RecordHome {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        Self {
            _home: test_toolkit::EnvGuard::set_safe("HOME", &path),
            _profile: test_toolkit::EnvGuard::set_safe("USERPROFILE", &path),
            dir,
        }
    }

    /// Every `session_end` row stored under this home.
    pub(crate) fn session_ends(&self) -> Vec<EventMeta> {
        let logs = self.dir.path().join(".claudine").join("logs");
        let Ok(entries) = std::fs::read_dir(&logs) else {
            return Vec::new();
        };
        entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .flat_map(|path| {
                std::fs::read_to_string(path)
                    .unwrap()
                    .lines()
                    .map(|line| serde_json::from_str::<EventMeta>(line).unwrap())
                    .collect::<Vec<_>>()
            })
            .filter(|meta| meta.event == AgenticEvent::SessionEnd)
            .collect()
    }

    /// The one `session_end` row; fails when there is not exactly one.
    pub(crate) fn only_session_end(&self) -> EventMeta {
        let mut records = self.session_ends();
        assert_eq!(records.len(), 1, "expected one session_end, got {records:#?}");
        records.remove(0)
    }
}

/// A section stream over `gate`, standing in for the run's terminal.
pub(crate) fn section_stream_over(gate: &Gate) -> (Arc<StreamOutput>, SectionStream) {
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    let stream = SectionStream::new(Arc::clone(&output));
    (output, stream)
}

/// A successful Claude run with enough detail to render a trailer.
pub(crate) fn successful_summary() -> StreamExecutionSummary {
    StreamExecutionSummary {
        provider: Provider::Claude,
        exit_code: 0,
        duration_ms: Some(1_000),
        tool_calls: Some(2),
        assistant_text: "the answer\n".into(),
        ..Default::default()
    }
}

/// Publish `summary` exactly as both structured call sites do.
pub(crate) fn publish(
    summary: &StreamExecutionSummary,
    section_stream: &SectionStream,
    reader_warnings: &[String],
) {
    let profile = super::super::profile::profile_for_provider(Provider::Claude).unwrap();
    emit_stream_summary(
        summary,
        claudine::harness::ProcessTermination::Completed,
        profile,
        &EnvironmentContext::default(),
        Verbosity::Normal,
        false,
        &StructuredSummaryDetails::default(),
        Some(section_stream),
        Some(4242),
        &[],
        None,
        reader_warnings,
    );
}

/// Opens the gate when the test ends, so an abandoned worker is released even
/// when an assertion fails.
pub(crate) struct ReleaseOnDrop(pub(crate) Gate);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// A run whose reader timed out after the result still succeeds, and its one
/// stored record carries the warning text, not just a counter.
#[test]
fn a_successful_run_whose_reader_timed_out_stores_the_warning_on_its_session_end() {
    let home = RecordHome::new();
    let (gate, _entered) = Gate::new(true);
    let (_output, section_stream) = section_stream_over(&gate);

    let warning = reader_timeout_warning();
    publish(&successful_summary(), &section_stream, std::slice::from_ref(&warning));

    let record = home.only_session_end();
    assert_eq!(record.extra["exit_code"], 0, "{:?}", record.extra);
    let incomplete = &record.extra["output_incomplete"];
    assert_eq!(
        incomplete["reader_warnings"],
        serde_json::json!([warning]),
        "{incomplete}"
    );
    assert_eq!(incomplete["stalled"], false, "{incomplete}");
    assert_eq!(incomplete["dropped_frames"], 0, "{incomplete}");
}

/// A terminal that stopped accepting output before the trailer, with nothing
/// waiting on it until then, is found by the final drain, and the record
/// written after it says so.
#[test]
fn a_stall_found_by_the_final_drain_is_on_the_session_end_record() {
    let home = RecordHome::new();
    let (gate, entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let (output, section_stream) = section_stream_over(&gate);
    wedge_worker(&output, &entered);
    output.set_drain_deadline(Instant::now() + Duration::from_millis(50));

    let started = Instant::now();
    publish(&successful_summary(), &section_stream, &[]);

    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    let record = home.only_session_end();
    let incomplete = &record.extra["output_incomplete"];
    assert_eq!(incomplete["stalled"], true, "{:?}", record.extra);
    assert_eq!(incomplete["reader_warnings"], serde_json::json!([]), "{incomplete}");
}

/// Complete output adds nothing to the record.
#[test]
fn a_run_with_complete_output_stores_no_output_incomplete() {
    let home = RecordHome::new();
    let (gate, _entered) = Gate::new(true);
    let (_output, section_stream) = section_stream_over(&gate);

    publish(&successful_summary(), &section_stream, &[]);

    let record = home.only_session_end();
    assert!(
        !record.extra.contains_key("output_incomplete"),
        "{:?}",
        record.extra
    );
    let trailer: String = gate
        .written()
        .into_iter()
        .map(|(_, bytes)| String::from_utf8(bytes).unwrap())
        .collect();
    assert!(trailer.contains("tool calls"), "the trailer was drained first: {trailer:?}");
}
