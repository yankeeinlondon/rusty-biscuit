//! Test-only PTY harness that drives the validation failure reporter.
//!
//! This binary is intentionally not a user-facing CLI. It exists so the
//! integration test in `tests/validation_reporter_pty.rs` can spawn a process
//! under a real PTY and observe the exact ANSI/OSC-8 output that the
//! validation reporter emits, without needing to spin up a full
//! `claudine compose` flow with a stub provider on PATH.
//!
//! ## Usage
//!
//! ```text
//! validation_reporter_pty_harness <absolute-source-path>
//! ```
//!
//! The harness constructs a single failing `ValidationCheckOutcome` whose
//! `RuleSource.file` is the path supplied on the command line, then calls
//! [`claudine::harness::report::report_check_outcomes`] using a
//! `Terminal::new()` that auto-detects the surrounding PTY's capabilities.
//! The output goes to stderr — the same destination the production reporter
//! writes to.

use std::path::PathBuf;

use biscuit_terminal::terminal::Terminal;
use claudine::harness::report::report_check_outcomes;
use claudine::harness::{
    FailurePhase, RuleSource, ValidationCheckOutcome, ValidationEvent, ValidationPhaseReport,
    ValidationRuleId,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let source_path: PathBuf = args
        .next()
        .map(PathBuf::from)
        .expect("validation_reporter_pty_harness requires <absolute-source-path>");

    // Match what the real harness emits when a `file_exists` pre-check fails.
    let yaml_snippet = "  - file_exists: \"definitely-not-a-real-path-xyz.toml\"\n    message: \"{{source_file}} requires definitely-not-a-real-path-xyz.toml\"\n".to_string();

    let outcome = ValidationCheckOutcome {
        rule_id: ValidationRuleId(0),
        event: ValidationEvent::FileExists,
        subject_key: Some("definitely-not-a-real-path-xyz.toml".to_string()),
        passed: false,
        markup: "the file definitely-not-a-real-path-xyz.toml exists".to_string(),
        failure_message: Some(
            "file does not exist: /tmp/definitely-not-a-real-path-xyz.toml".to_string(),
        ),
        source: Some(RuleSource {
            file: source_path,
            line_range: Some(3..=5),
            yaml_snippet,
        }),
    };

    let report = ValidationPhaseReport {
        phase: FailurePhase::PreCheck,
        outcomes: vec![outcome],
    };

    // `Terminal::new()` performs real capability detection against the
    // attached PTY: it inspects `TERM`, `COLORTERM`, and TTY-ness. Under
    // expectrl this produces a TTY-attached, color-capable terminal, which
    // is exactly what we need to verify the SGR + OSC-8 emission path.
    let term = Terminal::new();
    report_check_outcomes(&report, &term);
}
