//! Top-level `claudine sequence <file>` command.

use clap::Args;
use claudine::composition::{self, SequenceExecutionOptions};
use color_eyre::eyre::{Result, eyre};

use super::compose::SharedComposeArgs;
use crate::log;

/// Run a Markdown document as a serial sequence of composition steps.
#[derive(Debug, Clone, Args)]
pub struct SequenceArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference to the sequence document.
    #[arg(value_name = "FILE")]
    pub file: String,

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
pub fn run_sequence(args: SequenceArgs, verbose: u8) -> Result<()> {
    let code = match run_sequence_inner(args, verbose) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };
    std::process::exit(code);
}

fn run_sequence_inner(args: SequenceArgs, verbose: u8) -> Result<i32> {
    let SequenceArgs {
        shared,
        file,
        fail_fast,
    } = args;

    let source = composition::resolve_composition_source(&file).map_err(|e| eyre!("{e}"))?;

    let plan = composition::resolve_sequence_plan(&source)
        .map_err(|e| eyre!("{e}"))?
        .ok_or_else(|| {
            eyre!(
                "file '{}' does not define a `sequence` frontmatter property",
                file
            )
        })?;

    let set_overrides = super::compose::parse_set_json(shared.set.as_deref())?;

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
