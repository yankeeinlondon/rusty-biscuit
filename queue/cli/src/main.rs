mod tui;

use chrono::{Duration as ChronoDuration, Local, NaiveTime, TimeZone, Utc};
use clap::Parser;
use queue_lib::{parse_at_time, parse_delay, ExecutionTarget, ScheduledTask};
use thiserror::Error;

use crate::tui::{run_app, App};

/// Queue commands for later execution with an interactive TUI.
///
/// All invocations open the TUI. Use --at or --in to pre-schedule a task.
///
/// Examples:
///   queue                                    # Open TUI
///   queue --at 7:00am "echo 'good morning'"  # Open TUI with pre-scheduled task
///   queue --in 15m "echo 'reminder'"         # Open TUI with task in 15 minutes
#[derive(Debug, Parser)]
#[command(name = "queue")]
#[command(version)]
#[command(about = "Queue commands for later execution with an interactive TUI")]
struct Cli {
    /// Schedule the command for the next occurrence of a time.
    #[arg(long, value_parser = parse_at_time, value_name = "TIME", conflicts_with = "in_delay")]
    at: Option<NaiveTime>,

    /// Schedule the command to run after a delay (e.g., 15m, 2h30m).
    #[arg(long = "in", value_parser = parse_delay, value_name = "DELAY", conflicts_with = "at")]
    in_delay: Option<ChronoDuration>,

    /// Enable debug logging to ~/.queue-debug.log.
    #[arg(long)]
    debug: bool,

    /// The shell command to schedule (required with --at or --in).
    #[arg(value_name = "COMMAND", required_if_eq_any = [("at", ""), ("in_delay", "")])]
    command: Option<String>,
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("TUI error: {0}")]
    TuiError(std::io::Error),

    #[error("debug log error: {0}")]
    DebugLogError(std::io::Error),
}

fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();

    // Set up debug logging if requested
    if cli.debug {
        init_debug_logging()?;
    }

    // Build the initial task if --at or --in was provided
    let initial_task = build_initial_task(&cli);

    run_tui(initial_task)
}

/// Builds an initial task from CLI arguments.
fn build_initial_task(cli: &Cli) -> Option<ScheduledTask> {
    let command = cli.command.as_ref()?;
    let now = Local::now();

    let scheduled_at = match (cli.at, cli.in_delay) {
        (Some(time), None) => {
            // Schedule for the specified time
            let today = now.date_naive();
            let mut scheduled = today.and_time(time);

            // If the time is in the past, schedule for tomorrow
            if scheduled <= now.naive_local() {
                scheduled += ChronoDuration::days(1);
            }

            Local
                .from_local_datetime(&scheduled)
                .single()
                .map(|dt| dt.with_timezone(&Utc))
        }
        (None, Some(delay)) => {
            // Schedule after the delay
            Some((now + delay).with_timezone(&Utc))
        }
        _ => None,
    }?;

    Some(ScheduledTask::new(
        1,
        command.clone(),
        scheduled_at,
        ExecutionTarget::default(),
    ))
}

/// Initializes debug logging to ~/.queue-debug.log.
fn init_debug_logging() -> Result<(), QueueError> {
    use std::fs::OpenOptions;
    use tracing_subscriber::EnvFilter;

    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let log_path = home.join(".queue-debug.log");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(QueueError::DebugLogError)?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .with_writer(file)
        .with_ansi(false)
        .init();

    Ok(())
}

/// Runs the TUI application with proper terminal setup and cleanup.
fn run_tui(initial_task: Option<ScheduledTask>) -> Result<(), QueueError> {
    // Build a tokio runtime for the executor
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Enter the runtime context for the executor to spawn tasks
    let _guard = runtime.enter();

    // Set up panic hook for terminal cleanup
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    // Initialize terminal
    let mut terminal = ratatui::init();

    // Create app with executor
    let mut app = App::new().with_executor();

    // Add initial task if provided
    if let Some(task) = initial_task {
        app.schedule_task(task);
    }

    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    ratatui::restore();

    result.map_err(QueueError::TuiError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_accepts_no_args() {
        // Plain `queue` opens TUI with no pre-scheduled task
        let result = Cli::try_parse_from(["queue"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.at.is_none());
        assert!(cli.in_delay.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn clap_rejects_conflicting_schedule_modes() {
        // Cannot specify both --at and --in
        let result = Cli::try_parse_from(["queue", "--at", "7:00am", "--in", "15m", "echo hi"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_accepts_at_with_command() {
        let result = Cli::try_parse_from(["queue", "--at", "7:00am", "echo hi"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.at.is_some());
        assert_eq!(cli.command, Some("echo hi".to_string()));
    }

    #[test]
    fn clap_accepts_in_with_command() {
        let result = Cli::try_parse_from(["queue", "--in", "15m", "echo hi"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.in_delay.is_some());
        assert_eq!(cli.command, Some("echo hi".to_string()));
    }

    #[test]
    fn clap_accepts_debug_flag() {
        let result = Cli::try_parse_from(["queue", "--debug"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.debug);
    }

    #[test]
    fn build_initial_task_returns_none_without_schedule() {
        let cli = Cli::try_parse_from(["queue"]).unwrap();
        assert!(build_initial_task(&cli).is_none());
    }

    #[test]
    fn build_initial_task_returns_task_with_at() {
        let cli = Cli::try_parse_from(["queue", "--at", "7:00am", "echo hello"]).unwrap();
        let task = build_initial_task(&cli);
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.command, "echo hello");
    }

    #[test]
    fn build_initial_task_returns_task_with_in() {
        let cli = Cli::try_parse_from(["queue", "--in", "15m", "echo hello"]).unwrap();
        let task = build_initial_task(&cli);
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.command, "echo hello");
    }
}
