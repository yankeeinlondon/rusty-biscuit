//! Command-line arguments.

use std::path::PathBuf;

use chrono::NaiveDate;
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Parser, Subcommand};
use content_policy::{Policy, PolicyOptions};

/// Check and renew Markdown content policies.
#[derive(Debug, Parser)]
#[command(name = "policy", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Evaluate a document's content policy and print the report.
    Check(CheckArgs),
    /// Plan, and with `--write` apply, a renewal of a document's baselines.
    Renew(RenewArgs),
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// The Markdown document to evaluate.
    pub document: PathBuf,

    /// Evaluate at 00:00 UTC on this date instead of now.
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date)]
    pub at: Option<NaiveDate>,

    /// Print only `true`, `false`, or `unknown`: whether any rule has
    /// confirmed a need for action.
    #[arg(long, conflicts_with = "json")]
    pub needs_action: bool,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct RenewArgs {
    /// The Markdown document to renew.
    pub document: PathBuf,

    /// The update date to record; defaults to today's UTC date and may not
    /// be in the future.
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date)]
    pub on: Option<NaiveDate>,

    /// Apply the planned edits to the file. Without it, only the preview is
    /// printed.
    #[arg(long)]
    pub write: bool,

    // Replaces "today's UTC date" so tests can renew on a fixed calendar.
    // Deliberately hidden and undocumented.
    #[arg(long, hide = true, value_parser = parse_date)]
    pub today: Option<NaiveDate>,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub output: OutputArgs,
}

/// Settings shared by every subcommand. Precedence is flag, then
/// environment variable, then the built-in value.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Frontmatter key holding the policy [default: content_policy]
    #[arg(long, value_name = "NAME", env = "CONTENT_POLICY_KEY", value_parser = NonEmptyStringValueParser::new())]
    pub key: Option<String>,

    /// Policy for documents that declare none: one rule, such as
    /// `TimeSensitive`, or a flow list starting with `[` [default: ValidFor(6mo)]
    #[arg(long, value_name = "POLICY", env = "CONTENT_POLICY_DEFAULT", value_parser = parse_policy)]
    pub default_policy: Option<Policy>,

    /// Property the `ValidFor(<duration>)` shorthand reads [default: last_updated]
    #[arg(long, value_name = "NAME", env = "CONTENT_POLICY_DATE_PROPERTY", value_parser = NonEmptyStringValueParser::new())]
    pub date_property: Option<String>,
}

impl ConfigArgs {
    pub fn options(&self) -> PolicyOptions {
        let mut options = PolicyOptions::default();
        if let Some(key) = &self.key {
            options = options.with_key(key);
        }
        if let Some(policy) = &self.default_policy {
            options = options.with_default_policy(policy.clone());
        }
        if let Some(name) = &self.date_property {
            options = options.with_date_property(name);
        }
        options
    }
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Print without colors or other styling.
    #[arg(long, conflicts_with = "json")]
    pub plain: bool,

    /// Print the result as JSON, the only content on stdout.
    #[arg(long)]
    pub json: bool,
}

/// Accepts exactly `YYYY-MM-DD`; chrono alone would also take `2026-9-1`.
fn parse_date(text: &str) -> Result<NaiveDate, String> {
    let shaped = text.len() == 10
        && text.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    shaped
        .then(|| NaiveDate::parse_from_str(text, "%Y-%m-%d").ok())
        .flatten()
        .ok_or_else(|| format!("`{text}` is not a calendar date in YYYY-MM-DD form"))
}

fn parse_policy(text: &str) -> Result<Policy, String> {
    Policy::from_text(text).map_err(|invalid| {
        invalid
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_must_be_exactly_yyyy_mm_dd() {
        assert_eq!(
            parse_date("2026-12-28"),
            Ok(NaiveDate::from_ymd_opt(2026, 12, 28).unwrap())
        );
        for text in ["2026-9-1", "2026-02-30", "2026-09-28T00:00:00Z", "", "20260928xx", "2026/09/28"] {
            assert!(parse_date(text).is_err(), "{text:?} should be rejected");
        }
    }
}
