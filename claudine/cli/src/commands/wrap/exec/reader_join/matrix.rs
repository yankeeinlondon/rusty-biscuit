//! How a stalled or panicked reader combines with each way a run can end.
//!
//! One row per run outcome, one column per reader failure; the cell is the
//! exit code, `error_kind`, and warning the wrapper reports. No cell turns a
//! reader failure into success, and no cell lets the reader warning replace the
//! run's own cause.

use super::*;
use crate::commands::wrap::exec::termination::apply_early_termination_to_summary;
use claudine::stream::logs::EarlyTermination;

const BUDGET: ReaderBudget = ReaderBudget {
    pipe_cap: Duration::from_millis(20),
    pipe_settle: Duration::from_millis(100),
    drain_limit: Duration::from_secs(10),
};

/// A parser that has seen every line and already knows its verdict.
struct VerdictParser(Option<(&'static str, &'static str)>);

impl SemanticStreamParser for VerdictParser {
    fn feed_line(&mut self, _line: &str) {}

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        self.snapshot(exit_code)
    }

    fn snapshot(&self, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            exit_code,
            is_error: self.0.is_some(),
            error_kind: self.0.map(|(kind, _)| kind.to_string()),
            error_message: self.0.map(|(_, message)| message.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy)]
enum Run {
    ExitZeroWithResult,
    ExitZeroNoResult,
    ResultSaysFailureExitZero,
    NonzeroExit,
    Interrupted,
    ProviderTimeout,
}

impl Run {
    const ALL: [Run; 6] = [
        Run::ExitZeroWithResult,
        Run::ExitZeroNoResult,
        Run::ResultSaysFailureExitZero,
        Run::NonzeroExit,
        Run::Interrupted,
        Run::ProviderTimeout,
    ];

    fn exit_code(self) -> i32 {
        match self {
            Run::NonzeroExit => 2,
            Run::Interrupted => 130,
            Run::ProviderTimeout => 137,
            _ => 0,
        }
    }

    /// What the reader had published when it stalled in the sink work of a
    /// line: the summary of the parser that had parsed the result line.
    fn snapshot(self) -> ResultSnapshot {
        match self {
            Run::ExitZeroWithResult | Run::ResultSaysFailureExitZero => ResultSnapshot {
                summary: Some(VerdictParser(self.parser_verdict()).snapshot(0)),
                ..Default::default()
            },
            _ => ResultSnapshot::default(),
        }
    }

    /// The verdict a parser that read the whole stream reaches.
    fn parser_verdict(self) -> Option<(&'static str, &'static str)> {
        match self {
            Run::ExitZeroWithResult => None,
            Run::ResultSaysFailureExitZero => Some(("api_remote", "overloaded")),
            Run::ExitZeroNoResult => Some(("no_result", "no result line")),
            Run::NonzeroExit | Run::Interrupted | Run::ProviderTimeout => {
                Some(("exit_failure", "the agent failed"))
            }
        }
    }

    /// The wrapper applies a provider timeout after the reader is settled.
    fn finish(self, parser: Box<dyn SemanticStreamParser>) -> StreamExecutionSummary {
        let mut summary = parser.finish(self.exit_code());
        if let Run::ProviderTimeout = self {
            apply_early_termination_to_summary(
                &mut summary,
                &EarlyTermination::Timeout { message: "timed out".into() },
            );
        }
        summary
    }
}

fn slot_holding(parser: Option<VerdictParser>) -> ParserSlot {
    Arc::new(Mutex::new(parser.map(|p| Box::new(p) as Box<dyn SemanticStreamParser>)))
}

/// `(exit code, error kind, warning shown)`.
type Cell = (i32, Option<&'static str>, bool);

fn settle(run: Run, outcome: JoinOutcome<()>, slot: ParserSlot) -> (StreamExecutionSummary, bool) {
    let (parser, warning) = settle_parser(outcome, &slot, run.exit_code(), BUDGET, &run.snapshot());
    (run.finish(parser), warning.is_some())
}

fn assert_cell(label: &str, (summary, warned): (StreamExecutionSummary, bool), want: Cell) {
    let got: Cell = (summary.exit_code, summary.error_kind.as_deref().map(leak), warned);
    assert_eq!(got, want, "{label}");
    assert_eq!(summary.is_error, want.1.is_some(), "{label}: is_error");
}

fn leak(kind: &str) -> &'static str {
    ["claudine_completion_delayed", "interrupted", "parse_failure", "api_remote", "no_result", "exit_failure", "timeout"]
        .into_iter()
        .find(|known| *known == kind)
        .unwrap_or_else(|| panic!("unexpected error kind {kind}"))
}

#[test]
fn a_reader_stalled_before_handing_back_its_parser_never_turns_a_run_into_success() {
    let wanted: [Cell; 6] = [
        (0, None, true),
        (0, Some("claudine_completion_delayed"), true),
        (0, Some("api_remote"), true),
        (2, Some("exit_failure"), true),
        (130, Some("interrupted"), true),
        (1, Some("timeout"), true),
    ];
    for stall in [ReaderStall::Processing, ReaderStall::PipeOpen] {
        for (run, want) in Run::ALL.into_iter().zip(wanted) {
            let cell = settle(run, JoinOutcome::TimedOut(stall), slot_holding(None));
            let label = format!("{stall:?}, exit {}", run.exit_code());
            assert_cell(&label, cell, want);
        }
    }
}

#[test]
fn a_reader_stalled_only_in_its_final_render_reports_the_run_exactly_as_the_parser_saw_it() {
    let wanted: [Cell; 6] = [
        (0, None, true),
        (0, Some("no_result"), true),
        (0, Some("api_remote"), true),
        (2, Some("exit_failure"), true),
        (130, Some("exit_failure"), true),
        (1, Some("timeout"), true),
    ];
    for (run, want) in Run::ALL.into_iter().zip(wanted) {
        let slot = slot_holding(Some(VerdictParser(run.parser_verdict())));
        let cell = settle(run, JoinOutcome::TimedOut(ReaderStall::Processing), slot);
        assert_cell(&format!("exit {}", run.exit_code()), cell, want);
    }
}

#[test]
fn a_panicked_reader_is_a_parse_failure_unless_the_provider_already_timed_the_run_out() {
    for run in Run::ALL {
        let outcome = JoinOutcome::Panicked("boom".into());
        let (summary, warned) = settle(run, outcome, slot_holding(None));
        let kind = match run {
            Run::ProviderTimeout => "timeout",
            _ => "parse_failure",
        };
        assert_eq!(summary.error_kind.as_deref(), Some(kind), "exit {}", run.exit_code());
        assert!(summary.is_error);
        assert!(!warned, "the panic is the run's failure, not a warning beside a result");
    }
}

#[test]
fn a_reader_that_finished_in_time_changes_nothing() {
    for run in Run::ALL {
        let slot = slot_holding(Some(VerdictParser(run.parser_verdict())));
        let (summary, warned) = settle(run, JoinOutcome::Joined(()), slot);
        assert!(!warned);
        assert_eq!(summary.is_error, run.parser_verdict().is_some());
    }
}

/// The Codex case from the logs: a reader still handling a line (rendering)
/// well past the 5 s pipe cap, but finishing inside the 120 s drain limit.
/// The bounds are scaled down; only their ratio matters.
#[test]
fn a_slow_but_progressing_reader_is_joined_with_its_real_parser_and_no_warning() {
    let budget = ReaderBudget {
        pipe_cap: Duration::from_millis(50),
        pipe_settle: Duration::from_millis(20),
        drain_limit: Duration::from_secs(30),
    };
    let progress = ReaderProgress::default();
    let slot = slot_holding(None);
    let (reader_progress, reader_slot) = (progress.clone(), slot.clone());
    let handle = thread::spawn(move || {
        for _line in reader_progress.track(["result".to_string()].into_iter()) {
            // Longer than the pipe cap, as 6.5 s is longer than 5 s.
            thread::sleep(Duration::from_millis(400));
        }
        *reader_slot.lock().unwrap() = Some(Box::new(VerdictParser(None)));
    });

    let outcome = join_reader(handle, &progress, budget, Instant::now());

    assert!(matches!(outcome, JoinOutcome::Joined(())), "{outcome:?}");
    let (parser, warning) = settle_parser(outcome, &slot, 0, budget, &ResultSnapshot::default());
    assert_eq!(warning, None);
    assert!(!parser.finish(0).is_error);
}
