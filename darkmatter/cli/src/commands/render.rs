//! `md render` (implicit and explicit) subcommand implementation.

use crate::args::{Cli, OutputFormat};
use crate::artifact::{
    emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact, markdown_plus_artifact,
    open_output_artifact,
};
use crate::io::load_markdown;
use crate::render::render_terminal_output;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use tracing::{debug, instrument};

/// Shared render logic for both implicit (no subcommand) and explicit `render` subcommand.
#[instrument(skip_all, fields(command = "render"))]
pub fn run_render(
    input: Option<&PathBuf>,
    output: OutputFormat,
    show: bool,
    indent: Option<usize>,
    cli: &Cli,
) -> Result<()> {
    debug!("rendering document");
    let mut md = load_markdown(input)?;

    // Apply cleanup with the specified or default indentation
    let indent_size = indent.unwrap_or(darkmatter::markdown::cleanup::DEFAULT_INDENT);
    md.cleanup_with_indent(indent_size);

    // Only terminal rendering needs a real `Terminal`; MarkdownPlus and HTML
    // construct their own optimistic terminal inside the artifact builders so
    // they can resolve a single theme from the same terminal that renders the
    // page. Markdown, JSON, and non-TTY auto paths never resolve a theme.
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                let term = Terminal::new();
                render_terminal_output(
                    &md, input, cli, term,
                )?;
                if show {
                    open_output_artifact(&markdown_artifact(&md))?;
                }
            } else {
                emit_or_show_artifact(markdown_artifact(&md), show)?;
            }
        }
        OutputFormat::Markdown => {
            emit_or_show_artifact(markdown_artifact(&md), show)?;
        }
        OutputFormat::MarkdownPlus => {
            let artifact = markdown_plus_artifact(&md, cli, input,
            )?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&md, cli, input,
            )?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&md)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}
