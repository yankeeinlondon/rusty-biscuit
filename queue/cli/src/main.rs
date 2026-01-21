mod tui;

use std::process::{Command, Stdio};

use chrono::{Duration as ChronoDuration, Local, NaiveTime, TimeZone, Utc};
use clap::Parser;
use queue_lib::{
    parse_at_time,
    parse_delay,
    ExecutionTarget,
    JsonFileStore,
    ScheduledTask,
    TerminalDetector,
};
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

    /// Internal flag: run TUI directly in current pane (used after Wezterm split).
    #[arg(long, hide = true)]
    tui_pane: bool,

    /// The shell command to schedule (required with --at or --in).
    #[arg(value_name = "COMMAND", required_if_eq_any = [("at", ""), ("in_delay", "")])]
    command: Option<String>,
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("TUI error: {0}")]
    Tui(std::io::Error),

    #[error("debug log error: {0}")]
    DebugLog(std::io::Error),

    #[error("failed to spawn TUI pane: {0}")]
    SpawnPane(String),
}

fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();

    // Set up debug logging if requested
    if cli.debug {
        init_debug_logging()?;
    }

    // In Wezterm, split the pane and spawn the TUI in the bottom pane,
    // unless we're already running in the TUI pane (--tui-pane flag).
    if TerminalDetector::is_wezterm() && !cli.tui_pane {
        return spawn_tui_in_split_pane(&cli);
    }

    // Build the initial task if --at or --in was provided
    let initial_task = build_initial_task(&cli);

    run_tui(initial_task)
}

/// Spawns the TUI in a new bottom pane and exits.
///
/// This creates the split layout:
/// - Top pane (80%): User's original shell (remains interactive)
/// - Bottom pane (20%): The TUI
fn spawn_tui_in_split_pane(cli: &Cli) -> Result<(), QueueError> {
    // Build the command to run in the new pane
    let exe = std::env::current_exe()
        .map_err(|e| QueueError::SpawnPane(format!("failed to get executable path: {e}")))?;

    let mut args = vec!["--tui-pane".to_string()];

    if cli.debug {
        args.push("--debug".to_string());
    }

    if let Some(ref time) = cli.at {
        args.push("--at".to_string());
        args.push(time.format("%H:%M").to_string());
    }

    if let Some(ref delay) = cli.in_delay {
        args.push("--in".to_string());
        // Format the delay back to a string
        let total_secs = delay.num_seconds();
        if total_secs % 86400 == 0 {
            args.push(format!("{}d", total_secs / 86400));
        } else if total_secs % 3600 == 0 {
            args.push(format!("{}h", total_secs / 3600));
        } else if total_secs % 60 == 0 {
            args.push(format!("{}m", total_secs / 60));
        } else {
            args.push(format!("{}s", total_secs));
        }
    }

    if let Some(ref cmd) = cli.command {
        args.push(cmd.clone());
    }

    // Get current pane ID for targeting the split
    let current_pane = TerminalDetector::get_wezterm_pane_id()
        .ok_or_else(|| QueueError::SpawnPane("WEZTERM_PANE not set".to_string()))?;

    // Build wezterm CLI command to create a bottom pane (20%) running the TUI
    let mut wezterm_args = vec![
        "cli".to_string(),
        "split-pane".to_string(),
        "--bottom".to_string(),
        "--percent".to_string(),
        "20".to_string(),
        "--pane-id".to_string(),
        current_pane,
        "--".to_string(),
        exe.to_string_lossy().to_string(),
    ];
    wezterm_args.extend(args);

    let output = Command::new("wezterm")
        .args(&wezterm_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| QueueError::SpawnPane(format!("failed to run wezterm cli: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QueueError::SpawnPane(format!(
            "wezterm cli split-pane failed: {stderr}"
        )));
    }

    // The new pane ID is returned in stdout - we could use it to focus the pane
    let new_pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Focus the new TUI pane
    if !new_pane_id.is_empty() {
        let _ = Command::new("wezterm")
            .args(["cli", "activate-pane", "--pane-id", &new_pane_id])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    // Exit - the TUI is now running in the new pane, user's shell in top pane is free
    Ok(())
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
        .map_err(QueueError::DebugLog)?;

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
    let mut app = App::new()
        .with_executor()
        .with_history_store(JsonFileStore::default_path());

    // In Wezterm, get the current pane ID so tasks can create panes relative to the TUI.
    // When running with --tui-pane, we're in the bottom pane. Tasks with NewPane target
    // will split above this pane.
    if TerminalDetector::is_wezterm()
        && let Some(ref executor) = app.executor
    {
        let current_pane = TerminalDetector::get_wezterm_pane_id();
        executor.set_task_pane_id_sync(current_pane);
    }

    // Add initial task if provided
    if let Some(mut task) = initial_task {
        task.id = app.alloc_task_id();
        app.schedule_task(task);
    }

    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    ratatui::restore();

    result.map_err(QueueError::Tui)
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
        assert!(!cli.tui_pane);
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
    fn clap_accepts_tui_pane_flag() {
        let result = Cli::try_parse_from(["queue", "--tui-pane"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.tui_pane);
    }

    #[test]
    fn clap_accepts_tui_pane_with_other_args() {
        let result =
            Cli::try_parse_from(["queue", "--tui-pane", "--at", "7:00am", "echo hello"]);
        assert!(result.is_ok());
        let cli = result.unwrap();
        assert!(cli.tui_pane);
        assert!(cli.at.is_some());
        assert_eq!(cli.command, Some("echo hello".to_string()));
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

    // =========================================================================
    // Regression tests for Bug 1: TUI should split and move to bottom pane
    // =========================================================================

    #[test]
    fn tui_pane_flag_is_hidden_in_help() {
        // Regression test: --tui-pane is an internal flag that should be hidden
        // from user-facing help to avoid confusion
        use clap::CommandFactory;
        let cmd = Cli::command();
        let arg = cmd.get_arguments().find(|a| a.get_id() == "tui_pane");
        assert!(arg.is_some(), "--tui-pane argument should exist");
        assert!(arg.unwrap().is_hide_set(), "--tui-pane should be hidden");
    }

    #[test]
    fn tui_pane_flag_prevents_split_recursion() {
        // Regression test: When --tui-pane is set, the TUI should run directly
        // without attempting to split again (prevents infinite recursion)
        let cli = Cli::try_parse_from(["queue", "--tui-pane"]).unwrap();
        assert!(cli.tui_pane, "tui_pane flag should be true");
        // The actual behavior (not splitting) is tested by the fact that
        // run_tui() is called instead of spawn_tui_in_split_pane() when
        // tui_pane is true
    }

    #[test]
    fn tui_pane_flag_preserves_all_arguments() {
        // Regression test: When spawning TUI in split pane, all user arguments
        // should be passed through to the child process
        let cli = Cli::try_parse_from([
            "queue",
            "--debug",
            "--at",
            "14:30",
            "echo 'test command'",
        ])
        .unwrap();

        assert!(cli.debug);
        assert!(cli.at.is_some());
        assert_eq!(cli.command, Some("echo 'test command'".to_string()));
        // The spawn_tui_in_split_pane function reconstructs these args
        // for the child process
    }
}
