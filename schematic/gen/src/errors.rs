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
