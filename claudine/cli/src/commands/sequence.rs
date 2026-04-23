//! Top-level `claudine sequence <file>` command.

use clap::Args;
use claudine::composition::{self, SequenceExecutionOptions};
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::compose::SharedComposeArgs;

/// Run a Markdown document as a serial sequence of composition steps.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys; reserved
/// per-step overlay keys still win over both.
#[derive(Debug, Clone, Args)]
pub struct SequenceArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,

    /// Override the document's fail-fast behavior for this run.
    /// Accepts: true, false, 1, 0, yes, no.
    #[arg(long = "fail-fast", value_name = "BOOL", value_parser = parse_boolish)]
    pub fail_fast: Option<bool>,
}

fn parse_boolish(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(format!(
            "invalid boolean value '{s}'; expected true/false, 1/0, or yes/no"
        )),
    }
}

/// Entry point for `claudine sequence`.
///
/// Errors returned here bubble up to the top-level walker in `main.rs`,
/// which renders darkmatter `BlockError` reports for typed Markdown
/// failures and falls back to `color_eyre` otherwise.
pub fn run_sequence(
    args: SequenceArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = run_sequence_inner(args, verbose, startup_timings)?;
    std::process::exit(code);
}

fn run_sequence_inner(
    args: SequenceArgs,
    verbose: u8,
    _startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let SequenceArgs {
        shared,
        args,
        fail_fast,
    } = args;

    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    // Validate `--step-timeout` once at entry; the parsed value is threaded
    // to each step through [`super::wrap::sequence::execute_sequence`] via
    // [`SharedComposeArgs::step_timeout_secs`].
    shared.step_timeout_secs()?;

    let parsed = super::compose::parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

    let source = composition::resolve_composition_source(&file)?;

    let _sequence_span = info_span!(
        "sequence",
        file = %source.resolved_path.display(),
        fail_fast = ?fail_fast,
    )
    .entered();

    let plan = composition::resolve_sequence_plan(&source)?
        .ok_or_else(|| {
            eyre!(
                "file '{}' does not define a `sequence` frontmatter property",
                file
            )
        })?;

    let set_overrides =
        super::compose::merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;

    let execution_options = SequenceExecutionOptions {
        fail_fast_override: fail_fast,
    };

    super::wrap::sequence::execute_sequence(
        &source,
        plan,
        &shared,
        set_overrides,
        execution_options,
        verbose,
    )
}
