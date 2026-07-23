//! CLI performance timing and no-results handling.
//!
//! This module provides `CliPerf` for tracking command execution time and
//! `handle_no_results` for uniform no-results exit behaviour. Extracted to a
//! leaf module to break the `commands` → `output` → `commands` cycle.

use sniff::PerformanceReport;
use sniff::performance::PerformanceCollector;
use std::sync::Arc;

use crate::output;

/// Request-scoped CLI performance timing and work collection.
///
/// When `--perf` is active, detection and any later command-specific work can
/// share one collector and end-to-end clock. Rich terminal commands emit to
/// stdout; scriptable text commands emit to stderr so stdout stays clean for
/// shell pipelines.
pub(crate) struct CliPerf {
    start: Option<std::time::Instant>,
    collector: Option<Arc<PerformanceCollector>>,
    plain: bool,
}

impl CliPerf {
    pub fn new(enabled: bool, plain: bool) -> Self {
        Self {
            start: enabled.then(std::time::Instant::now),
            collector: enabled.then(PerformanceCollector::new_shared),
            plain,
        }
    }

    /// Run work inside this command's request-scoped collector.
    pub fn collect<T>(&self, f: impl FnOnce() -> T) -> T {
        match self.collector.as_ref() {
            Some(collector) => sniff::performance::with_current_collector(
                Some(Arc::clone(collector)),
                f,
            ),
            None => f(),
        }
    }

    pub fn emit_stdout(&self, detailed: Option<&PerformanceReport>) {
        self.emit(detailed, false);
    }

    pub fn emit_stderr(&self, detailed: Option<&PerformanceReport>) {
        self.emit(detailed, true);
    }

    /// Emit perf output, routing to stderr when JSON has been printed to
    /// stdout so the JSON payload stays machine-parseable.
    pub fn emit_for_json(&self, detailed: Option<&PerformanceReport>) {
        self.emit(detailed, true)
    }

    /// Snapshot the request collector with the elapsed CLI timing.
    ///
    /// Returns `None` when `--perf` was not enabled.
    pub fn build_report(&self) -> Option<PerformanceReport> {
        let start = self.start?;
        Some(
            self.collector
                .as_ref()
                .expect("enabled CLI timing always has a collector")
                .snapshot(start.elapsed()),
        )
    }

    fn emit(&self, detailed: Option<&PerformanceReport>, to_stderr: bool) {
        let Some(report) = self.build_report() else {
            return;
        };
        let report = detailed.cloned().unwrap_or(report);
        let text = output::render_performance_section(&report);
        if to_stderr {
            output::emit_stderr(&text, self.plain);
        } else {
            output::emit_text(&text, self.plain);
        }
    }
}

/// Handle the no-results exit behavior for file-list and blast-radius commands.
///
/// - Default: exit 1 with no output.
/// - `--no-error`: exit 0 with no output.
/// - `--on-error <msg>`: render message to stderr.
/// - `--on-error` + `--no-error`: render message to stdout, exit 0.
pub(crate) fn handle_no_results(
    no_error: bool,
    on_error: &Option<String>,
    plain: bool,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;

    if let Some(msg) = on_error {
        let terminal = Terminal::default();
        let rendered = Prose::new(msg).render(&terminal);
        let text = if plain {
            biscuit_terminal::prelude::strip_escape_codes(&rendered)
        } else {
            rendered
        };
        if no_error {
            println!("{text}");
        } else {
            eprintln!("{text}");
        }
    }

    perf.emit_stderr(None);
    if no_error {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
