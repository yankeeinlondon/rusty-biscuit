use clap::{Subcommand, ValueHint};
use std::ffi::OsString;
use std::time::{Duration, Instant};

/// Subcommands under `sniff filesystem`.
#[derive(Subcommand, Debug, Clone)]
pub enum FilesystemSubcommand {
    /// Report processes using a file or directory tree (open handles,
    /// working directories, watch registrations, loaded modules)
    #[command(after_help = QUERY_AFTER_HELP)]
    Query(FilesystemQueryArgs),
}

/// Arguments for `sniff filesystem query`.
#[derive(clap::Args, Debug, Clone)]
pub struct FilesystemQueryArgs {
    /// File or directory to query; a relative path resolves against the
    /// invocation directory (`--base` does not apply). File references
    /// (`@`, `&`, `^`, `~`, `vault:`) resolve to a local file or directory
    #[arg(value_name = "PATH", value_hint = ValueHint::AnyPath)]
    pub path: OsString,

    /// Inspect the named target alone, not a directory's descendants
    #[arg(long)]
    pub target_only: bool,

    /// Stop scheduling discovery work after this many milliseconds (default
    /// 2000). A system call already running can overrun it, so this is not a
    /// limit on when the command returns
    #[arg(long, value_name = "MS", value_parser = parse_timeout, allow_hyphen_values = true)]
    pub timeout: Option<Duration>,
}

const QUERY_AFTER_HELP: &str = "\
A match shows usage; it does not prove that deleting the target will fail.
No matches does not prove that nothing is using the target: read the coverage.

Exit codes: 0 for a usable report (including partial or empty), 1 when no
discovery mechanism could run or the target could not be resolved, 2 for
invalid arguments.";

/// Accepts a positive whole number of milliseconds that can form a deadline.
pub(crate) fn parse_timeout(value: &str) -> Result<Duration, String> {
    let millis: u64 = value
        .parse()
        .map_err(|_| format!("`{value}` is not a whole number of milliseconds"))?;
    if millis == 0 {
        return Err("the timeout must be greater than zero".to_string());
    }
    let duration = Duration::from_millis(millis);
    if Instant::now().checked_add(duration).is_none() {
        return Err(format!("{millis}ms is too large to form a deadline"));
    }
    Ok(duration)
}
