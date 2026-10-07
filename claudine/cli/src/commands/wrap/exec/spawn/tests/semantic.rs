//! Structured-stream spawn mode: the agent's stderr passthrough is delivered
//! by the output worker.

use super::*;
use crate::commands::wrap::output_worker::tests::Gate;
use crate::commands::wrap::output_worker::{Drained, Stream};
use crate::commands::wrap::stream_io::StreamOutput;
use claudine::signals::SignalHub;
use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::summary::StreamExecutionSummary;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::super::timeouts::TimeoutConfig;

/// A parser that ignores the stream and reports a plain exit.
struct ExitOnly;

impl SemanticStreamParser for ExitOnly {
    fn feed_line(&mut self, _line: &str) {}

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        self.snapshot(exit_code)
    }

    fn snapshot(&self, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            exit_code,
            ..Default::default()
        }
    }
}

fn run_with_stderr(output: &Arc<StreamOutput>) -> StreamExecutionSummary {
    let (binary, args) = test_shell_command("echo passthrough >&2", "1>&2 echo passthrough");
    let mut child_spawned = false;
    run_child_stream_semantic(
        &binary,
        &args,
        &minimal_env(),
        &test_cwd(),
        TimeoutConfig::default(),
        &[],
        false,
        false,
        None,
        Box::new(|_, _, _| Box::new(ExitOnly)),
        &mut child_spawned,
        Default::default(),
        Arc::clone(output),
        None,
        None,
        None,
        None,
        None,
        Arc::new(SignalHub::without_table()),
        None,
        None,
    )
    .expect("spawning the platform shell fixture must succeed")
    .data
}

/// Opens the gate when the test ends, so an abandoned worker is released even
/// when an assertion fails.
struct ReleaseOnDrop(Gate);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// Unsuppressed provider stderr reaches the terminal as a stderr frame.
#[test]
fn semantic_stderr_passthrough_is_delivered_by_the_output_worker() {
    let (gate, _entered) = Gate::new(true);
    let output = StreamOutput::with_sink(Box::new(gate.clone()));

    let summary = run_with_stderr(&output);

    assert_eq!(summary.exit_code, 0);
    assert_eq!(output.drain(Instant::now() + Duration::from_secs(10)), Drained::Complete);
    let stderr: String = gate
        .written()
        .into_iter()
        .filter(|(stream, _)| *stream == Stream::Stderr)
        .map(|(_, bytes)| String::from_utf8(bytes).unwrap())
        .collect();
    assert_eq!(stderr.trim_end(), "passthrough");
}

/// On a terminal an earlier run found stalled, the next run's passthrough is
/// refused at once: the run completes without touching the terminal.
#[test]
fn semantic_stderr_passthrough_after_a_stall_is_refused_without_blocking() {
    let (gate, _entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    output.emit_stderr_line("the line the terminal never took");
    assert_eq!(output.drain(Instant::now() + Duration::from_millis(50)), Drained::Disabled);
    let rejected = output.loss().rejected_frames;

    let started = Instant::now();
    let summary = run_with_stderr(&output);

    assert_eq!(summary.exit_code, 0);
    assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
    assert!(output.loss().rejected_frames > rejected, "the passthrough was refused");
    assert!(
        gate.written()
            .iter()
            .all(|(_, bytes)| !String::from_utf8_lossy(bytes).contains("passthrough")),
        "{:?}",
        gate.written()
    );
}
