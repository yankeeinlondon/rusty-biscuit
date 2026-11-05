//! `md render` (implicit and explicit) subcommand implementation.

use crate::args::policy::{self, PolicyCommand};
use crate::args::{Cli, OutputFormat, RenderOptions};
use crate::artifact::{
    emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact, markdown_plus_artifact,
    open_output_artifact,
};
use crate::io::load_document;
use crate::render::render_terminal_output;
use crate::request::MdRequest;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::cleanup::rules::{Indentation, Leaf, MarkdownRuleOverrides};
use std::io::{self, IsTerminal};
use tracing::{debug, instrument};

/// Shared render logic for both implicit (no subcommand) and explicit `render` subcommand.
#[instrument(skip_all, fields(command = "render"))]
pub fn run_render(
    options: &RenderOptions,
    cli: &Cli,
    request: &MdRequest,
) -> Result<()> {
    let RenderOptions { input, output, show, indent, deny, features } = options;
    let show = *show;
    let policy = policy::resolve_for(PolicyCommand::Render, &[], deny, &[], None)?;
    let strict_style = policy.denies("style");
    debug!("rendering document");
    let (mut md, resolved) = load_document(input.as_ref(), request)?;
    // Page-relative assets (images, `style:` files) resolve beside the file
    // the argument opened, not beside its spelling (`@doc.md` has no parent).
    let input = resolved.as_ref();

    // Before cleanup: a rewritten body has no authored position to report.
    let link_warnings = md.link_title_warnings();
    super::compose::blocked_diagnostics(&policy.blocking_policy(), &link_warnings, &[])?;
    for warning in &link_warnings {
        eprintln!("{}", warning_row(warning));
    }

    // Guarded cleanup under the default rules, with `--indent` (which only
    // parses 2, 4, or 8) as an explicit override.
    md.cleanup_with(&MarkdownRuleOverrides {
        indentation: indent.and_then(Indentation::from_spaces).map_or(Leaf::Absent, Leaf::Value),
        ..MarkdownRuleOverrides::default()
    });

    // Only terminal rendering needs a real `Terminal`; MarkdownPlus and HTML
    // construct their own optimistic terminal inside the artifact builders so
    // they can resolve a single theme from the same terminal that renders the
    // page. Markdown, JSON, and non-TTY auto paths never resolve a theme.
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                let term = Terminal::new();
                render_terminal_output(&md, input, cli, term, strict_style, features)?;
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
            let artifact = markdown_plus_artifact(&md, cli, input, strict_style)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&md, cli, input, strict_style, features)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&md)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}

fn warning_row(warning: &darkmatter::markdown::compose::ComposeWarning) -> String {
    use biscuit_terminal::components::prose::{LineBreaks, Prose};
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::prelude::{Status, StatusState};
    let location = warning.path.as_ref().zip(warning.line_number).map_or_else(String::new, |(path, line)| {
        format!("{}:{line}: ", biscuit_file::to_portable_string(path))
    });
    Status::from_prose(format!("{location}{}", Prose::escape_text_outside_code_spans(&warning.message)))
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Warning)
        .render(&Terminal::new())
}
