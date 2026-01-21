mod tui;

use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveTime, TimeZone};
use clap::{ArgGroup, Parser};
use queue_lib::{parse_at_time, parse_delay};
use thiserror::Error;
use tokio::process::Command;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::tui::{run_app, App};

/// Queue commands for later execution.
///
/// Examples:
///   queue --tui                              # Launch interactive TUI
///   queue --at 7:00am "so-you-say 'hi'"
///   queue --in 15m "echo 'hi'"
///   queue --at 7:00am "echo 'good morning'" --fg
#[derive(Debug, Parser)]
#[command(name = "queue")]
#[command(about = "Queue commands for later execution", long_about = None)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .multiple(false)
        .args(["tui_mode", "at", "in_delay"])
))]
struct Cli {
    /// Launch the interactive TUI.
    #[arg(long = "tui", id = "tui_mode")]
    tui: bool,

    /// Schedule the command for the next occurrence of a time.
    #[arg(long, value_parser = parse_at_time, value_name = "TIME")]
    at: Option<NaiveTime>,

    /// Schedule the command to run after a delay.
    #[arg(long = "in", value_parser = parse_delay, value_name = "DELAY")]
    in_delay: Option<ChronoDuration>,

    /// Run the command in the foreground and wait for completion.
    #[arg(long)]
    fg: bool,

    /// Enable INFO-level logging.
    #[arg(long)]
    debug: bool,

    /// The raw shell command to schedule.
    #[arg(value_name = "COMMAND", required_unless_present = "tui_mode")]
    command: Option<String>,
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("failed to convert schedule to a duration")]
    InvalidSchedule,

    #[error("failed to start command: {0}")]
    CommandStart(#[from] std::io::Error),

    #[error("command exited with status {0}")]
    CommandFailed(ExitStatus),

    #[error("TUI error: {0}")]
    TuiError(std::io::Error),
}

fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();

    if cli.tui {
        run_tui()
    } else {
        init_tracing(cli.debug);
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
            .block_on(run(cli))
    }
}

/// Runs the TUI application with proper terminal setup and cleanup.
fn run_tui() -> Result<(), QueueError> {
    // Set up panic hook for terminal cleanup
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    // Initialize terminal
    let mut terminal = ratatui::init();

    // Create app and run
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    ratatui::restore();

    result.map_err(QueueError::TuiError)
}

fn init_tracing(debug: bool) {
    let filter = if debug { "info" } else { "warn" };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();
}

#[tracing::instrument(skip(cli))]
async fn run(cli: Cli) -> Result<(), QueueError> {
    // command is guaranteed to be Some when not in TUI mode (due to required_unless_present)
    let command = cli.command.as_ref().expect("command required in CLI mode");

    let scheduled_at = schedule_time(&cli)?;
    let now = Local::now();
    let wait_duration = scheduled_at.signed_duration_since(now);
    let wait_duration = wait_duration
        .to_std()
        .map_err(|_| QueueError::InvalidSchedule)?;

    info!(scheduled_at = %scheduled_at, "scheduled command");

    if cli.fg {
        if !wait_duration.is_zero() {
            info!(
                wait_seconds = wait_duration.as_secs(),
                "waiting for schedule"
            );
            tokio::time::sleep(wait_duration).await;
        }

        run_command_foreground(command).await
    } else {
        run_command_background(command, wait_duration)
    }
}

fn schedule_time(cli: &Cli) -> Result<DateTime<Local>, QueueError> {
    let now = Local::now();

    match (cli.at, cli.in_delay) {
        (Some(time), None) => schedule_for_time(now, time),
        (None, Some(delay)) => Ok(now + delay),
        _ => Err(QueueError::InvalidSchedule),
    }
}

fn schedule_for_time(now: DateTime<Local>, time: NaiveTime) -> Result<DateTime<Local>, QueueError> {
    let today = now.date_naive();
    let mut scheduled = today.and_time(time);

    if scheduled <= now.naive_local() {
        scheduled += ChronoDuration::days(1);
    }

    Local
        .from_local_datetime(&scheduled)
        .single()
        .ok_or(QueueError::InvalidSchedule)
}

#[tracing::instrument(skip(command), fields(command = %command))]
async fn run_command_foreground(command: &str) -> Result<(), QueueError> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = child.wait().await?;

    if status.success() {
        Ok(())
    } else {
        Err(QueueError::CommandFailed(status))
    }
}

#[tracing::instrument(skip(command), fields(command = %command))]
fn run_command_background(command: &str, wait_duration: Duration) -> Result<(), QueueError> {
    let shell_command = if wait_duration.is_zero() {
        format!("exec {command}")
    } else {
        let wait_seconds = wait_duration.as_secs_f64();
        format!("sleep {wait_seconds:.3}; exec {command}")
    };

    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(shell_command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    if let Some(pid) = child.id() {
        info!(pid, "spawned background command");
    } else {
        warn!("spawned background command without pid");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_requires_mode_argument() {
        // Neither --tui nor --at/--in provided
        let result = Cli::try_parse_from(["queue", "echo hi"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_rejects_multiple_modes() {
        let result = Cli::try_parse_from(["queue", "--at", "7:00am", "--in", "15m", "echo hi"]);
        assert!(result.is_err());

        let result = Cli::try_parse_from(["queue", "--tui", "--at", "7:00am"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_accepts_tui_mode() {
        let result = Cli::try_parse_from(["queue", "--tui"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.tui);
        assert!(cli.command.is_none());
    }

    #[test]
    fn clap_requires_command_for_schedule_mode() {
        let result = Cli::try_parse_from(["queue", "--at", "7:00am"]);
        assert!(result.is_err());

        let result = Cli::try_parse_from(["queue", "--in", "15m"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_accepts_schedule_with_command() {
        let result = Cli::try_parse_from(["queue", "--at", "7:00am", "echo hi"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(!cli.tui);
        assert_eq!(cli.command, Some("echo hi".to_string()));
    }
}
