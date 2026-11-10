//! Output selection for one invocation: the resolved target and width, and the
//! stderr channel for what is not the deliverable.
//!
//! A deliverable's width comes from `--width`, `OUTPUT_COLUMNS`, and stdout's
//! terminal. Stderr has its own: it wraps at stderr's terminal columns and is
//! unlimited when stderr is not a terminal, whatever `--width` says.

pub mod command;
pub mod report;

use std::io::{IsTerminal as _, Write as _};

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::output::{
    OutputTarget, RenderedOutput, WidthRequest, render_output, resolve_stderr_width, resolve_width, stderr_terminal_columns,
};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use renderable::{CommandOutput, OutputOptions, TerminalProfile};

/// The code of the warning a fractional `--width` produces.
pub const FRACTIONAL_WIDTH_CODE: &str = "dm.output.fractional_width";
/// The code of a data output that could not be serialized.
pub const SERIALIZATION_CODE: &str = "dm.output.serialization_failed";
/// The code of an artifact the host could not open.
pub const OPEN_FAILED_CODE: &str = "dm.output.open_failed";

/// What the process knows about its output streams, read once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputEnvironment {
    /// `OUTPUT_COLUMNS`, unparsed: a bad value matters only when it is needed.
    pub output_columns: Option<String>,
    /// Whether stdout is a terminal.
    pub stdout_is_terminal: bool,
    /// Stdout's columns; known only when it is a terminal.
    pub stdout_columns: Option<u32>,
    /// Whether stderr is a terminal.
    pub stderr_is_terminal: bool,
    /// Stderr's columns; known only when it is a terminal.
    pub stderr_columns: Option<u32>,
    /// Whether stdin is a terminal: a render with no input then has nothing
    /// to read. Checked without reading stdin.
    pub stdin_is_terminal: bool,
}

impl OutputEnvironment {
    /// The running process's streams. `COLUMNS` outranks a measured width, on
    /// a stream that is a terminal only.
    #[must_use]
    pub fn detect() -> Self {
        let columns = std::env::var("COLUMNS").ok().and_then(|raw| raw.trim().parse::<u32>().ok()).filter(|&n| n > 0);
        let stdout_is_terminal = std::io::stdout().is_terminal();
        let stderr_is_terminal = std::io::stderr().is_terminal();
        Self {
            output_columns: std::env::var("OUTPUT_COLUMNS").ok(),
            stdout_is_terminal,
            stdout_columns: stdout_is_terminal.then(|| columns.or_else(|| Terminal::default().detected_columns())).flatten(),
            stderr_is_terminal,
            stderr_columns: stderr_is_terminal.then(|| columns.or_else(stderr_terminal_columns)).flatten(),
            stdin_is_terminal: std::io::stdin().is_terminal(),
        }
    }

    /// The options a stderr rendering uses for `terminal`'s capabilities.
    #[must_use]
    pub fn stderr_options(&self, terminal: &Terminal) -> OutputOptions {
        let profile = if self.stderr_is_terminal { terminal.output_profile() } else { TerminalProfile::plain() };
        OutputOptions::new(resolve_stderr_width(self.stderr_columns), profile)
    }

    /// A [`Terminal`] for components rendered to stderr: stderr's own columns
    /// and capabilities, with color and links off when it is not a terminal.
    #[must_use]
    pub fn stderr_terminal(&self) -> Terminal {
        let mut terminal = Terminal {
            is_tty: self.stderr_is_terminal,
            ..Terminal::default()
        };
        terminal.set_output_width(resolve_stderr_width(self.stderr_columns));
        if !self.stderr_is_terminal {
            terminal.color_depth = ColorDepth::None;
            terminal.osc_link_support = false;
            terminal.supports_italic = false;
        }
        terminal
    }
}

/// The [`Terminal`] for what this process renders to stderr.
#[must_use]
pub fn stderr_terminal() -> Terminal {
    OutputEnvironment::detect().stderr_terminal()
}

/// A target and width resolved for one invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedOutput {
    /// What was asked for.
    pub target: OutputTarget,
    /// The options every fold of the deliverable gets.
    pub options: OutputOptions,
    /// Whether `--width` was given; only then does it override a document's
    /// own `fixed-width:` and `unwrap:`.
    pub explicit_width: bool,
    /// The warning width resolution produced (a fractional `--width`).
    pub width_warning: Option<String>,
}

/// Resolves `target` and `--width`, with `default_width` when `--width` is
/// absent.
///
/// A data target never detects a terminal: only its width is read.
///
/// ## Errors
///
/// A usage error when a needed `OUTPUT_COLUMNS` is invalid or a percentage
/// resolves outside the column range.
pub fn resolve_output(
    target: OutputTarget,
    width: Option<WidthRequest>,
    default_width: WidthRequest,
    verbosity: u8,
    environment: &OutputEnvironment,
) -> std::result::Result<ResolvedOutput, clap::Error> {
    use clap::CommandFactory as _;
    let resolved = resolve_width(
        width.unwrap_or(default_width),
        environment.output_columns.as_deref(),
        environment.stdout_columns,
    )
    .map_err(|error| crate::args::Cli::command().error(clap::error::ErrorKind::ValueValidation, error.to_string()))?;
    let profile = if target.is_data() || !environment.stdout_is_terminal {
        TerminalProfile::plain()
    } else {
        Terminal::default().output_profile()
    };
    Ok(ResolvedOutput {
        target,
        options: OutputOptions::new(resolved.width, profile).with_verbosity(verbosity),
        explicit_width: width.is_some(),
        width_warning: resolved.warning,
    })
}

/// The warning for `text` as a data output's diagnostic.
#[must_use]
pub fn width_warning_diagnostic(text: &str) -> Diagnostic {
    output_diagnostic(FRACTIONAL_WIDTH_CODE, Severity::Warning, text)
}

/// A diagnostic with no source: the output machinery's own, about the run.
#[must_use]
pub fn output_diagnostic(code: &str, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: message.to_string(),
        source: None,
        expected_type: None,
        actual_type: None,
        span: None,
        iteration_path: Default::default(),
        reference_details: None,
        data_location: None,
        grants_needed: None,
    }
}

/// `text` as stderr can show it: a component may paint regardless of its
/// terminal, so off a terminal any escape it wrote is removed.
#[must_use]
pub fn stderr_text(text: String) -> String {
    if std::io::stderr().is_terminal() { text } else { biscuit_terminal::utils::escape_codes::strip_escape_codes(text) }
}

/// Writes a warning row to stderr.
pub fn stderr_warning(text: &str) {
    use biscuit_terminal::components::prose::{LineBreaks, Prose};
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::prelude::{Status, StatusState};
    let row = Status::from_prose(Prose::escape_text_outside_code_spans(text))
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Warning)
        .render(&stderr_terminal());
    eprintln!("{row}");
}

/// Renders `value` for `target` and writes it to stdout, which keeps a
/// Markdown or HTML document's final newline as `md` always has.
///
/// ## Errors
///
/// A render or serialization failure, before anything is written; a write
/// failure.
pub fn write_stdout<T: CommandOutput>(value: &T, target: OutputTarget, options: &OutputOptions) -> Result<()> {
    let RenderedOutput { text, .. } = render_output(value, target, options).map_err(|error| eyre!(error))?;
    write_stdout_text(&stdout_text(text, target))
}

/// `text` as `target` puts it on stdout: a Markdown or HTML document gains its
/// final newline, as `md` has always printed it, and any other text ends in
/// exactly one.
#[must_use]
pub fn stdout_text(mut text: String, target: OutputTarget) -> String {
    match target {
        OutputTarget::Markdown | OutputTarget::MarkdownPlus | OutputTarget::Html => text.push('\n'),
        _ => {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
        }
    }
    text
}

fn write_stdout_text(text: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|error| eyre!("could not write output: {error}"))
}

/// Renders `value` as `terminal` for stderr, by stderr's own detection.
#[must_use]
pub fn stderr_report<T: CommandOutput>(value: &T) -> String {
    let environment = OutputEnvironment::detect();
    let options = environment.stderr_options(&environment.stderr_terminal());
    match render_output(value, OutputTarget::Terminal, &options) {
        Ok(rendered) => rendered.text,
        Err(error) => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use renderable::Width;

    use super::*;

    fn piped() -> OutputEnvironment {
        OutputEnvironment::default()
    }

    #[test]
    fn the_width_precedence_is_flag_then_output_columns_then_terminal() {
        let env = OutputEnvironment {
            output_columns: Some("100".to_string()),
            stdout_is_terminal: true,
            stdout_columns: Some(60),
            ..piped()
        };
        let resolve = |width| resolve_output(OutputTarget::Json, width, WidthRequest::Auto, 0, &env).unwrap().options.width;
        assert_eq!(resolve(Some(WidthRequest::Columns(40))), Width::Columns(40));
        assert_eq!(resolve(None), Width::Columns(100));
        let terminal_only = OutputEnvironment { output_columns: None, ..env.clone() };
        let width = resolve_output(OutputTarget::Json, None, WidthRequest::Auto, 0, &terminal_only).unwrap().options.width;
        assert_eq!(width, Width::Columns(60));
    }

    #[test]
    fn nothing_known_is_unlimited() {
        let resolved = resolve_output(OutputTarget::Plain, None, WidthRequest::Auto, 0, &piped()).unwrap();
        assert_eq!(resolved.options.width, Width::Unlimited);
        assert!(!resolved.explicit_width);
    }

    #[test]
    fn an_explicit_unlimited_beats_a_known_terminal() {
        let env = OutputEnvironment { stdout_is_terminal: true, stdout_columns: Some(80), ..piped() };
        let resolved =
            resolve_output(OutputTarget::Json, Some(WidthRequest::Unlimited), WidthRequest::Auto, 0, &env).unwrap();
        assert_eq!(resolved.options.width, Width::Unlimited);
    }

    #[test]
    fn a_percentage_of_nothing_is_unlimited_and_of_output_columns_is_floored() {
        let percent = Some(WidthRequest::Percent(50.5));
        let width = resolve_output(OutputTarget::Json, percent, WidthRequest::Auto, 0, &piped()).unwrap().options.width;
        assert_eq!(width, Width::Unlimited);
        let env = OutputEnvironment { output_columns: Some("100".to_string()), ..piped() };
        let width = resolve_output(OutputTarget::Json, percent, WidthRequest::Auto, 0, &env).unwrap().options.width;
        assert_eq!(width, Width::Columns(50));
    }

    #[test]
    fn a_fractional_width_floors_and_warns() {
        let resolved =
            resolve_output(OutputTarget::Json, Some(WidthRequest::Fractional(72.9)), WidthRequest::Auto, 0, &piped()).unwrap();
        assert_eq!(resolved.options.width, Width::Columns(72));
        assert!(resolved.width_warning.is_some());
    }

    #[test]
    fn a_needed_invalid_output_columns_is_a_usage_error() {
        let env = OutputEnvironment { output_columns: Some("wide".to_string()), ..piped() };
        let error = resolve_output(OutputTarget::Json, None, WidthRequest::Auto, 0, &env).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        // An explicit width never reads it.
        assert!(resolve_output(OutputTarget::Json, Some(WidthRequest::Columns(9)), WidthRequest::Auto, 0, &env).is_ok());
    }

    #[test]
    fn stderr_ignores_the_deliverable_width_and_uses_its_own() {
        let env = OutputEnvironment { stderr_is_terminal: true, stderr_columns: Some(33), ..piped() };
        let term = env.stderr_terminal();
        assert_eq!(env.stderr_options(&term).width, Width::Columns(33));
        assert_eq!(term.width(), 33);
        let piped_env = piped();
        let term = piped_env.stderr_terminal();
        let options = piped_env.stderr_options(&term);
        assert_eq!(options.width, Width::Unlimited);
        assert!(options.terminal_profile.is_plain());
        assert_eq!(term.color_depth, ColorDepth::None);
    }

    #[test]
    fn a_markdown_document_gains_its_newline_and_data_ends_in_one() {
        assert_eq!(stdout_text("a\n".to_string(), OutputTarget::Markdown), "a\n\n");
        assert_eq!(stdout_text("{}".to_string(), OutputTarget::Json), "{}\n");
        assert_eq!(stdout_text("x = 1\n".to_string(), OutputTarget::Toml), "x = 1\n");
        assert_eq!(stdout_text("a".to_string(), OutputTarget::Plain), "a\n");
        assert_eq!(stdout_text(String::new(), OutputTarget::Plain), "");
    }
}
