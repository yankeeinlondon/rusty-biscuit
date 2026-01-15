use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveTime, TimeZone};
use clap::{ArgGroup, Parser};
use thiserror::Error;
use tokio::process::Command;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

/// Queue commands for later execution.
///
/// Examples:
///   queue --at 7:00am "so-you-say 'hi'"
///   queue --in 15m "echo 'hi'"
///   queue --at 7:00am "echo 'good morning'" --fg
#[derive(Debug, Parser)]
#[command(name = "queue")]
#[command(about = "Queue commands for later execution", long_about = None)]
#[command(group(
    ArgGroup::new("schedule")
        .required(true)
        .multiple(false)
        .args(["at", "in_delay"])
))]
struct Cli {
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
    #[arg(value_name = "COMMAND")]
    command: String,
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("failed to convert schedule to a duration")]
    InvalidSchedule,

    #[error("failed to start command: {0}")]
    CommandStart(#[from] std::io::Error),

    #[error("command exited with status {0}")]
    CommandFailed(ExitStatus),
}

#[tokio::main]
async fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();
    init_tracing(cli.debug);
    run(cli).await
}

fn init_tracing(debug: bool) {
    let filter = if debug { "info" } else { "warn" };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();
}

#[tracing::instrument(skip(cli))]
async fn run(cli: Cli) -> Result<(), QueueError> {
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

        run_command_foreground(&cli.command).await
    } else {
        run_command_background(&cli.command, wait_duration)
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

fn parse_at_time(value: &str) -> Result<NaiveTime, String> {
    let normalized = value.trim().to_lowercase().replace(' ', "");

    if normalized.is_empty() {
        return Err("time cannot be empty".to_string());
    }

    let formats = ["%H:%M", "%I:%M%P", "%I%P"];

    for format in formats {
        if let Ok(time) = NaiveTime::parse_from_str(&normalized, format) {
            return Ok(time);
        }
    }

    Err("expected time like 7:00am or 19:30".to_string())
}

fn parse_delay(value: &str) -> Result<ChronoDuration, String> {
    let normalized = value.trim().to_lowercase().replace(' ', "");

    if normalized.is_empty() {
        return Err("delay cannot be empty".to_string());
    }

    let split_index = normalized
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or_else(|| normalized.len());
    let (amount, unit) = normalized.split_at(split_index);

    if amount.is_empty() {
        return Err("delay must start with a number".to_string());
    }

    let amount: i64 = amount
        .parse()
        .map_err(|_| "delay must be a number".to_string())?;

    if amount <= 0 {
        return Err("delay must be greater than zero".to_string());
    }

    let duration = match unit {
        "" | "m" => ChronoDuration::minutes(amount),
        "s" => ChronoDuration::seconds(amount),
        "h" => ChronoDuration::hours(amount),
        "d" => ChronoDuration::days(amount),
        _ => {
            return Err("delay units must be s, m, h, or d".to_string());
        }
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_at_time_accepts_12_hour_format() {
        let time = parse_at_time("7:00am").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(7, 0, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_accepts_24_hour_format() {
        let time = parse_at_time("19:30").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(19, 30, 0).expect("time"));
    }

    #[test]
    fn parse_delay_defaults_to_minutes() {
        let delay = parse_delay("15").expect("valid delay");
        assert_eq!(delay, ChronoDuration::minutes(15));
    }

    #[test]
    fn parse_delay_supports_seconds() {
        let delay = parse_delay("10s").expect("valid delay");
        assert_eq!(delay, ChronoDuration::seconds(10));
    }

    #[test]
    fn parse_delay_supports_hours() {
        let delay = parse_delay("2h").expect("valid delay");
        assert_eq!(delay, ChronoDuration::hours(2));
    }

    #[test]
    fn parse_delay_rejects_invalid_units() {
        assert!(parse_delay("1w").is_err());
    }

    #[test]
    fn clap_requires_schedule_argument() {
        let result = Cli::try_parse_from(["queue", "echo hi"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_rejects_multiple_schedules() {
        let result = Cli::try_parse_from(["queue", "--at", "7:00am", "--in", "15m", "echo hi"]);
        assert!(result.is_err());
    }
}
