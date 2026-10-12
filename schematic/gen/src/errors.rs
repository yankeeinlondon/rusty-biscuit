//! Error types for the schematic generator.

use thiserror::Error;

use crate::diagnostics::{Output, ValidationDiagnostic};

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Failed to parse API definition
    #[error("Failed to parse API definition: {0}")]
    ParseError(String),

    /// Failed to generate code
    #[error("Code generation failed: {0}")]
    CodeGenError(String),

    /// Failed to write output file
    #[error("Failed to write output file '{path}': {source}")]
    WriteError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Output directory does not exist
    #[error("Output directory does not exist: {0}")]
    OutputDirNotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// One output's validator reported blocking diagnostics.
    ///
    /// Nothing of that output was written. The rendered message lists every
    /// diagnostic, warnings included, one per line.
    #[error(
        "validation failed for output {output} ({} error(s)):{}",
        error_count(diagnostics),
        render_lines(diagnostics)
    )]
    Validation {
        /// The output that was not written.
        output: Output,
        /// Every diagnostic its validator reported.
        diagnostics: Vec<ValidationDiagnostic>,
    },

    /// An existing output root cannot be updated safely. Nothing was changed.
    #[error("refusing to update {root}; nothing was changed:{}\n{remedy}", bullet_lines(problems))]
    UnsafeOutput {
        /// The output root that was left untouched.
        root: String,
        /// Every reason found, one per offending path.
        problems: Vec<String>,
        /// What the user can do about it.
        remedy: String,
    },

    /// A partial run would replace shared runtime files that retained,
    /// unregenerated modules were generated against. Nothing was changed.
    #[error(
        "refusing a partial update of {root}; nothing was changed: the shared runtime \
         file(s) {} would change while module(s) {} are retained without being \
         regenerated. Run full generation (`generate --api all`) instead",
        changed.join(", "),
        retained.join(", ")
    )]
    RequiresFullGeneration {
        /// The output root that was left untouched.
        root: String,
        /// Shared runtime files whose new content differs from the existing one.
        changed: Vec<String>,
        /// Module groups that would have been kept as they are.
        retained: Vec<String>,
    },

    /// A write or deletion failed after publication had started.
    ///
    /// Every check ran before the first mutation, so this is an I/O failure.
    /// The output root is partly updated: `completed` mutations happened,
    /// `failed` and `pending` did not.
    #[error(
        "publishing {root} stopped at `{failed}`: {source}\n  completed ({}):{}\n  not done ({}):{}\n\
         The output is only partly updated. Fix the cause and run the command again; \
         if a partial run then refuses the output, run full generation (`generate --api all`)",
        completed.len(),
        indented_lines(completed),
        pending.len() + 1,
        indented_lines(&not_done(failed, pending))
    )]
    PublicationFailed {
        /// The output root being published.
        root: String,
        /// The mutation that failed.
        failed: String,
        /// Mutations that were applied, in order.
        completed: Vec<String>,
        /// Mutations after the failed one, never attempted.
        pending: Vec<String>,
        /// The underlying write or deletion error.
        #[source]
        source: Box<GeneratorError>,
    },

    /// At least one requested output failed; the others were still attempted.
    #[error(
        "generation failed for output(s) {}{}",
        join_outputs(failed),
        succeeded_suffix(succeeded)
    )]
    OutputsFailed {
        /// Outputs that were written (or would be, in a dry run).
        succeeded: Vec<Output>,
        /// Outputs that failed validation or writing.
        failed: Vec<Output>,
    },
}

fn error_count(diagnostics: &[ValidationDiagnostic]) -> usize {
    diagnostics.iter().filter(|d| d.is_error()).count()
}

fn render_lines(diagnostics: &[ValidationDiagnostic]) -> String {
    diagnostics.iter().map(|d| format!("\n  {d}")).collect()
}

fn bullet_lines(lines: &[String]) -> String {
    lines.iter().map(|line| format!("\n  - {line}")).collect()
}

fn indented_lines(lines: &[String]) -> String {
    lines.iter().map(|line| format!("\n    {line}")).collect()
}

fn not_done(failed: &str, pending: &[String]) -> Vec<String> {
    std::iter::once(failed.to_string())
        .chain(pending.iter().cloned())
        .collect()
}

fn join_outputs(outputs: &[Output]) -> String {
    outputs
        .iter()
        .map(|output| output.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn succeeded_suffix(succeeded: &[Output]) -> String {
    if succeeded.is_empty() {
        String::new()
    } else {
        format!(" (succeeded: {})", join_outputs(succeeded))
    }
}
