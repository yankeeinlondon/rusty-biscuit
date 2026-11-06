//! `md render` (implicit and explicit) subcommand implementation.

use crate::args::policy::{self, PolicyCommand};
use crate::args::{Cli, Feature, RenderOptions};
use crate::artifact::{OutputArtifact, html_page, open_output_artifact};
use crate::io::load_document;
use crate::output::report::ReportSink;
use crate::output::{OutputEnvironment, ResolvedOutput, resolve_output, stderr_terminal, stderr_warning, write_stdout};
use crate::render::terminal_page;
use crate::request::MdRequest;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::rules::{Indentation, Leaf, MarkdownRuleOverrides};
use darkmatter::markdown::output::command::{CommandValue, DocumentOutput, document_markdown, empty_document};
use renderable::Width;
use std::path::PathBuf;
use tracing::{debug, instrument};

/// Shared render logic for both implicit (no subcommand) and explicit `render` subcommand.
#[instrument(skip_all, fields(command = "render"))]
pub fn run_render(options: &RenderOptions, cli: &Cli, request: &MdRequest) -> Result<()> {
    let policy = policy::resolve_for(PolicyCommand::Render, &[], &options.deny, &[], None)?;
    let target = options.output.unwrap_or(OutputTarget::Terminal);
    let output = source_markdown_width(
        resolve_output(target, cli.width, WidthRequest::Auto, cli.verbose, &OutputEnvironment::detect())?,
    );
    let render = Render {
        options,
        cli,
        request,
        strict_style: policy.denies("style"),
        blocking: policy.blocking_policy(),
        output: &output,
    };
    if target.is_data() {
        let file = options.input.as_ref().filter(|path| path.to_str() != Some("-")).map(|path| path.display().to_string());
        let sink = ReportSink::new(&output, file, options.show, false);
        let result = render.run(Some(&sink));
        return sink.finish(result);
    }
    if let Some(warning) = &output.width_warning {
        stderr_warning(warning);
    }
    render.run(None)
}

/// A document's Markdown output is its source text, so with no `--width` and no
/// known width it stays as authored rather than having its paragraphs joined.
fn source_markdown_width(mut output: ResolvedOutput) -> ResolvedOutput {
    let prints_source = matches!(output.target, OutputTarget::Markdown | OutputTarget::MarkdownPlus) || output.target.is_data();
    if prints_source && !output.explicit_width && output.options.width == Width::Unlimited {
        output.options.width = Width::Source;
    }
    output
}

struct Render<'a> {
    options: &'a RenderOptions,
    cli: &'a Cli,
    request: &'a MdRequest,
    strict_style: bool,
    blocking: darkmatter::markdown::compose::policy::BlockingPolicy,
    output: &'a ResolvedOutput,
}

impl Render<'_> {
    fn run(&self, sink: Option<&ReportSink>) -> Result<()> {
        debug!("rendering document");
        let (mut md, resolved) = load_document(self.options.input.as_ref(), self.request)?;
        // Page-relative assets (images, `style:` files) resolve beside the file
        // the argument opened, not beside its spelling (`@doc.md` has no parent).
        let input = resolved.as_ref();

        // Before cleanup: a rewritten body has no authored position to report.
        let link_warnings = md.link_title_warnings();
        if let Some(sink) = sink {
            sink.warnings(&link_warnings);
        }
        super::compose::blocked_diagnostics(&self.blocking, &link_warnings, &[])?;
        if sink.is_none() {
            for warning in &link_warnings {
                eprintln!("{}", warning_row(warning));
            }
        }

        // Guarded cleanup under the default rules, with `--indent` (which only
        // parses 2, 4, or 8) as an explicit override.
        md.cleanup_with(&MarkdownRuleOverrides {
            indentation: self.options.indent.and_then(Indentation::from_spaces).map_or(Leaf::Absent, Leaf::Value),
            ..MarkdownRuleOverrides::default()
        });

        if !self.strict_style {
            crate::render::stop_if_style_code_denied(&md, &self.blocking)?;
        }

        if let Some(sink) = sink {
            sink.diagnostics(crate::render::style_diagnostics(&md, self.strict_style)?);
            sink.document(document_markdown(&md, true, self.output.options.width));
            return Ok(());
        }
        self.write(&md, input)
    }

    fn write(&self, md: &Markdown, input: Option<&PathBuf>) -> Result<()> {
        let features: &[Feature] = &self.options.features;
        let target = self.output.target;
        let page = match target {
            OutputTarget::Terminal | OutputTarget::Plain => {
                terminal_page(md, input, self.cli, &Terminal::default(), self.strict_style, features)?
            }
            OutputTarget::Html | OutputTarget::MarkdownPlus => html_page(md, self.cli, input, self.strict_style, features)?,
            // The source text needs no page; its style is never read.
            _ => darkmatter::layout::DarkmatterPage::new(&Terminal::new_optimistic(120)),
        };
        // `md render --max-width` is this subcommand's own flag; a root flag
        // reached the page through the style claims above.
        let page = match self.options.max_width {
            Some(max_width) => page.with_max_width(u32::from(max_width)),
            None => page,
        };
        let document = DocumentOutput::prepare(md, &page, target, &self.output.options)
            .wrap_err_with(|| format!("Failed to render {target} output"))?;

        match target {
            OutputTarget::Terminal | OutputTarget::Plain => {
                write_stdout(&document, target, &self.output.options)?;
                // A terminal rendering has no file form: `--show` opens the source.
                if self.options.show {
                    open_output_artifact(&OutputArtifact {
                        content: document.markdown().to_string(),
                        extension: "md",
                        label: "markdown",
                    })?;
                }
            }
            _ if self.options.show => {
                let rendered = biscuit_terminal::output::render_output(&document, target, &self.output.options)
                    .map_err(|error| color_eyre::eyre::eyre!(error))?;
                open_output_artifact(&OutputArtifact {
                    content: rendered.text,
                    extension: target.extension(),
                    label: target.as_str(),
                })?;
            }
            _ => write_stdout(&document, target, &self.output.options)?,
        }
        Ok(())
    }
}

/// A document command's value: `data` behind the data family, and `text`, which
/// keeps its exact bytes, as the Markdown. Every other formatted output renders
/// `document` through its page, which happens now so a failure surfaces here.
///
/// ## Errors
///
/// The page could not be built or rendered.
pub(crate) fn document_value<D>(
    data: D,
    document: &Markdown,
    text: String,
    input: Option<&PathBuf>,
    output: &ResolvedOutput,
    cli: &Cli,
) -> Result<CommandValue<D>> {
    let target = output.target;
    let page = match target {
        OutputTarget::Terminal | OutputTarget::Plain => terminal_page(document, input, cli, &Terminal::default(), false, &[])?,
        OutputTarget::Html | OutputTarget::MarkdownPlus => html_page(document, cli, input, false, &[])?,
        OutputTarget::Markdown => return Ok(CommandValue::new(data, empty_document()).with_markdown(text)),
        OutputTarget::Json | OutputTarget::Json5 | OutputTarget::Yaml | OutputTarget::Toml => {
            return Ok(CommandValue::new(data, empty_document()));
        }
    };
    let prepared = DocumentOutput::prepare(document, &page, target, &output.options)
        .wrap_err_with(|| format!("Failed to render {target} output"))?;
    let prose = renderable::CommandOutput::as_prose(&prepared);
    let rendered = biscuit_terminal::output::render_output(&prepared, target, &output.options)
        .map_err(|error| color_eyre::eyre::eyre!(error))?
        .text;
    let value = CommandValue::new(data, prose);
    Ok(match target {
        OutputTarget::Html => value.with_html(rendered),
        OutputTarget::Terminal | OutputTarget::Plain => value.with_terminal(move |_| rendered.clone()),
        _ => value,
    })
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
        .render(&stderr_terminal())
}
