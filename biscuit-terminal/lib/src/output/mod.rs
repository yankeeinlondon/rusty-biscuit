//! Shared output selection and dispatch for command results.
//!
//! A command result implements [`renderable::CommandOutput`]: it has a
//! render-tree source (`as_prose`) behind the **formatted** family and a
//! serializable source (`as_data`) behind the **data** family.
//! [`write_output`] folds such a value to one [`OutputTarget`]:
//!
//! | Target | Produced by |
//! |---|---|
//! | `terminal` | the value's terminal override, else the terminal fold, with the profile in the options; a profile that is not a color TTY degrades to `plain` |
//! | `plain` | the `terminal` path with [`TerminalProfile::plain`] |
//! | `markdown`, `markdown-plus` | the override, else the Markdown fold at the options' width |
//! | `html` | the override, else the browser fold |
//! | `json`, `json5`, `yaml`, `toml` | `as_data()` serialized; see [`serialize_data`] |
//!
//! Output is rendered completely before anything is written, so a render or
//! serialization failure never leaves partial output behind.
//!
//! Width selection lives in [`width`]: [`parse_width_request`] reads a
//! `--width` value and [`resolve_width`] turns it into a
//! [`renderable::Width`].
//!
//! [`TerminalProfile::plain`]: renderable::TerminalProfile::plain

mod data;
mod profile;
pub mod width;

use std::fmt;
use std::io::Write;
use std::str::FromStr;

use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_document_html,
    render_markdown_document,
};
use renderable::tree::{Diagnostic, RenderError, RenderStrictness};
use renderable::{CommandOutput, OutputOptions};

pub use data::serialize_data;
pub use profile::terminal_target_options;
pub use width::{
    ResolvedWidth, WidthError, WidthRequest, parse_width_request, resolve_stderr_width,
    resolve_width, stderr_terminal_columns,
};

/// The two output families. A command accepts whole families, never part of
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFamily {
    /// `terminal`, `plain`, `markdown`, `markdown-plus`, `html`: rendered from
    /// `as_prose()`.
    Formatted,
    /// `json`, `json5`, `yaml`, `toml`: serialized from `as_data()`.
    Data,
}

/// Every output a command can be asked for with `--output`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputTarget {
    /// Styled text for a terminal, degraded to what it supports.
    Terminal,
    /// Text with every style removed; never writes an escape byte.
    Plain,
    /// Markdown that is ergonomic to author and to read.
    Markdown,
    /// Markdown that uses inline HTML for richer presentation.
    MarkdownPlus,
    /// HTML (`browser` is an accepted alias).
    Html,
    /// Pretty-printed JSON.
    Json,
    /// JSON5.
    Json5,
    /// YAML.
    Yaml,
    /// TOML.
    Toml,
}

impl OutputTarget {
    /// Every target, formatted family first.
    pub const ALL: [Self; 9] = [
        Self::Terminal,
        Self::Plain,
        Self::Markdown,
        Self::MarkdownPlus,
        Self::Html,
        Self::Json,
        Self::Json5,
        Self::Yaml,
        Self::Toml,
    ];

    /// The canonical `--output` spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Plain => "plain",
            Self::Markdown => "markdown",
            Self::MarkdownPlus => "markdown-plus",
            Self::Html => "html",
            Self::Json => "json",
            Self::Json5 => "json5",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }

    /// The family this target belongs to.
    #[must_use]
    pub const fn family(self) -> OutputFamily {
        match self {
            Self::Terminal | Self::Plain | Self::Markdown | Self::MarkdownPlus | Self::Html => {
                OutputFamily::Formatted
            }
            Self::Json | Self::Json5 | Self::Yaml | Self::Toml => OutputFamily::Data,
        }
    }

    /// Whether this target serializes `as_data()`.
    #[must_use]
    pub const fn is_data(self) -> bool {
        matches!(self.family(), OutputFamily::Data)
    }

    /// The file extension an artifact of this target is saved with.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Terminal | Self::Plain => "txt",
            Self::Markdown | Self::MarkdownPlus => "md",
            Self::Html => "html",
            Self::Json => "json",
            Self::Json5 => "json5",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
        }
    }
}

impl fmt::Display for OutputTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An `--output` value that names no target.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not an output target")]
pub struct UnknownOutputTarget(pub String);

impl FromStr for OutputTarget {
    type Err = UnknownOutputTarget;

    /// Parses a canonical spelling, or `browser` for [`OutputTarget::Html`].
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw == "browser" {
            return Ok(Self::Html);
        }
        Self::ALL
            .into_iter()
            .find(|target| target.as_str() == raw)
            .ok_or_else(|| UnknownOutputTarget(raw.to_owned()))
    }
}

/// Why an output could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    /// A formatted fold failed.
    #[error("could not render {target} output: {source}")]
    Render {
        /// The target being rendered.
        target: OutputTarget,
        /// The fold's error.
        #[source]
        source: RenderError,
    },
    /// The data could not be serialized or converted to the target format.
    /// `path` is a JSON-pointer-style path to the offending value, when one
    /// value is to blame (`""` is the root).
    #[error("could not write {target} output{}: {message}", path.as_deref().map(|p| format!(" at `{p}`")).unwrap_or_default())]
    Serialize {
        /// The data target.
        target: OutputTarget,
        /// The path to the offending value, when known.
        path: Option<String>,
        /// What went wrong.
        message: String,
    },
    /// Writing the finished output failed.
    #[error("could not write output: {0}")]
    Io(#[from] std::io::Error),
}

/// A fully rendered output, ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedOutput {
    /// The complete output text.
    pub text: String,
    /// Non-fatal diagnostics the generic folds reported. An override reports
    /// none here.
    pub diagnostics: Vec<Diagnostic>,
}

/// Renders `value` for `target` without writing it.
///
/// ## Errors
///
/// [`OutputError::Render`] when a generic fold fails, and
/// [`OutputError::Serialize`] when a data target cannot represent the data.
pub fn render_output<T: CommandOutput + ?Sized>(
    value: &T,
    target: OutputTarget,
    options: &OutputOptions,
) -> Result<RenderedOutput, OutputError> {
    let render_error = |source| OutputError::Render { target, source };
    let formatted = |text: String| RenderedOutput {
        text,
        diagnostics: Vec::new(),
    };
    match target {
        OutputTarget::Terminal | OutputTarget::Plain => {
            let options = terminal_target_options(target, options);
            if let Some(text) = value.render_terminal(&options) {
                return Ok(formatted(text));
            }
            let rendered = crate::render_tree::render_terminal_output(
                &value.as_prose(),
                &options,
                RenderStrictness::Warn,
            )
            .map_err(render_error)?;
            Ok(RenderedOutput {
                text: rendered.output,
                diagnostics: rendered.diagnostics,
            })
        }
        OutputTarget::Markdown | OutputTarget::MarkdownPlus => {
            let (dialect, overridden) = if target == OutputTarget::Markdown {
                (MarkdownDialect::Markdown, value.render_markdown(options))
            } else {
                (MarkdownDialect::MarkdownPlus, value.render_markdown_plus(options))
            };
            if let Some(text) = overridden {
                return Ok(formatted(text));
            }
            let rendered = render_markdown_document(
                &value.as_prose(),
                &MarkdownRenderOptions {
                    dialect,
                    width: options.width,
                    ..MarkdownRenderOptions::default()
                },
            )
            .map_err(render_error)?;
            Ok(RenderedOutput {
                text: rendered.output,
                diagnostics: rendered.diagnostics,
            })
        }
        OutputTarget::Html => {
            if let Some(text) = value.render_html(options) {
                return Ok(formatted(text));
            }
            let rendered = render_browser_document_html(
                &value.as_prose(),
                &BrowserRenderOptions {
                    width: options.width,
                    ..BrowserRenderOptions::default()
                },
            )
            .map_err(render_error)?;
            Ok(RenderedOutput {
                text: rendered.output,
                diagnostics: rendered.diagnostics,
            })
        }
        OutputTarget::Json | OutputTarget::Json5 | OutputTarget::Yaml | OutputTarget::Toml => {
            Ok(formatted(serialize_data(&value.as_data(), target)?))
        }
    }
}

/// Folds `value` to `target` and writes the complete result to `out`.
///
/// Nothing is written unless rendering succeeded; then the whole output is
/// written at once. Diagnostics from the generic folds are dropped here; a
/// caller that reports them uses [`render_output`].
///
/// ## Errors
///
/// The errors of [`render_output`], and [`OutputError::Io`] when writing or
/// flushing `out` fails.
pub fn write_output<T: CommandOutput>(
    value: &T,
    target: OutputTarget,
    options: &OutputOptions,
    out: &mut dyn Write,
) -> Result<(), OutputError> {
    let rendered = render_output(value, target, options)?;
    out.write_all(rendered.text.as_bytes())?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_target_round_trips_through_its_spelling() {
        for target in OutputTarget::ALL {
            assert_eq!(target.as_str().parse::<OutputTarget>(), Ok(target));
        }
        assert_eq!("browser".parse::<OutputTarget>(), Ok(OutputTarget::Html));
    }

    #[test]
    fn removed_spellings_are_not_targets() {
        for raw in ["auto", "ast", "text", "report-json", "json-schema", "Terminal", ""] {
            assert_eq!(
                raw.parse::<OutputTarget>(),
                Err(UnknownOutputTarget(raw.to_owned())),
                "{raw}"
            );
        }
    }

    #[test]
    fn families_partition_the_targets() {
        let data: Vec<_> = OutputTarget::ALL.into_iter().filter(|t| t.is_data()).collect();
        assert_eq!(
            data,
            [OutputTarget::Json, OutputTarget::Json5, OutputTarget::Yaml, OutputTarget::Toml]
        );
    }
}
