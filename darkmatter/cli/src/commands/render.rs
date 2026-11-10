//! `md render` (implicit and explicit) subcommand implementation.
//!
//! Both spellings reach [`dispatch_render`], which prepares the options,
//! resolves the output once, and hands [`run_render`] only prepared values.
//! The render then reads the document, prepares its effective style (the
//! document's `style:` under each `--style` file and the setters) before
//! anything is printed or opened, and writes one target.

use crate::args::{Cli, PreparedRenderOptions, RenderOptions, setter_hint};
use crate::artifact::{OutputArtifact, html_page, open_output_artifact};
use crate::commands::trigger_findings::{emit_warnings, finding_diagnostic};
use crate::io::{DocumentArgumentError, OpenedArgument, load_opened_document};
use crate::output::report::{ClassifiedFailure, ReportSink, ReportedExit};
use crate::output::{
    OutputEnvironment, ResolvedOutput, output_diagnostic, resolve_output, stderr_terminal, stderr_warning, write_stdout,
};
use crate::render::{PageInputs, PagePresentation, document_style, terminal_page};
use crate::render_style::{
    RenderStyle, StyleWarningRecord, document_style_layer, prepare_read_render_style, read_cli_style_inputs,
};
use crate::request::MdRequest;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use clap::CommandFactory as _;
use color_eyre::eyre::{Context, Report, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::block::scan_inline_hr_warnings;
use darkmatter::markdown::cleanup::rules::{Indentation, Leaf, MarkdownRuleOverrides};
use darkmatter::markdown::compose::expression::{Diagnostic, DiagnosticSource, Severity};
use darkmatter::markdown::compose::policy::{BlockingPolicy, STYLE_DEPRECATED_CODE};
use darkmatter::markdown::compose::report_diagnostics::INPUT_READ_CODE;
use darkmatter::markdown::output::command::{CommandValue, DocumentOutput, document_markdown, empty_document};
use darkmatter::style::{StylePreparationPolicy, StyleWarning, StyleWarningKind};
use renderable::Width;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

/// Prepares `options` (the root's for `md`, the subcommand's for
/// `md render`), resolves the output once in `environment`, and renders.
/// `cli` supplies only the shared controls (`--width`, `-v`).
///
/// With no input and a terminal on stdin there is nothing to read: a
/// formatted output writes the root help to stderr and a data output its one
/// failure value, both with exit 1.
///
/// ## Errors
///
/// A usage error (exit 2) from preparation or output resolution;
/// [`ReportedExit`] or the failure itself otherwise.
#[instrument(skip_all, fields(command = "render"))]
pub fn dispatch_render(options: &RenderOptions, cli: &Cli, request: &MdRequest, environment: &OutputEnvironment) -> Result<()> {
    let prepared = PreparedRenderOptions::prepare(options)?;
    let target = prepared.output.unwrap_or(OutputTarget::Terminal);
    let output = source_markdown_width(resolve_output(target, cli.width, WidthRequest::Auto, cli.verbose, environment)?);
    if prepared.input.is_none() && environment.stdin_is_terminal {
        return no_input(&prepared, &output);
    }
    run_render(&prepared, &output, request)
}

/// Renders the prepared invocation to `output`.
///
/// ## Errors
///
/// [`ReportedExit`] after a data output's failure value; the failure itself
/// for a formatted output.
pub fn run_render(options: &PreparedRenderOptions, output: &ResolvedOutput, request: &MdRequest) -> Result<()> {
    let render = Render {
        options,
        request,
        deny_style: options.policy.denies("style"),
        blocking: options.policy.blocking_policy(),
        output,
    };
    if output.target.is_data() {
        let sink = ReportSink::new(output, options.input_file(), options.show, false);
        let result = render.run(Some(&sink));
        return sink.finish(result);
    }
    if let Some(warning) = &output.width_warning {
        stderr_warning(warning);
    }
    render.run(None)
}

/// The no-input outcome of [`dispatch_render`].
fn no_input(options: &PreparedRenderOptions, output: &ResolvedOutput) -> Result<()> {
    if output.target.is_data() {
        let message = "no input: give a Markdown file, `-` for stdin, or pipe the document";
        let failure = ClassifiedFailure::report(eyre!(message), vec![output_diagnostic(INPUT_READ_CODE, Severity::Error, message)]);
        return ReportSink::new(output, None, options.show, false).finish(Err(failure));
    }
    eprint!("{}", Cli::command().render_help());
    Err(ReportedExit(1).into())
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

/// Whether `target` builds a page, and so reads the document's style.
fn builds_page(target: OutputTarget) -> bool {
    matches!(target, OutputTarget::Terminal | OutputTarget::Plain | OutputTarget::Html | OutputTarget::MarkdownPlus)
}

struct Render<'a> {
    options: &'a PreparedRenderOptions,
    request: &'a MdRequest,
    /// `--deny style`.
    deny_style: bool,
    blocking: BlockingPolicy,
    output: &'a ResolvedOutput,
}

impl Render<'_> {
    fn run(&self, sink: Option<&ReportSink>) -> Result<()> {
        debug!("rendering document");
        let input = self.options.input.as_ref();
        let (mut md, opened) = load_opened_document(input, self.request).map_err(|error| with_setter_hint(error, input))?;
        // Page-relative assets (images, `style:` files) resolve beside the file
        // the argument opened, not beside its spelling (`@doc.md` has no parent).
        let resolved = opened.as_ref().map(|opened| opened.path().to_path_buf());
        if let Some(sink) = sink {
            sink.composes(md.source().as_ref());
        }

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

        // Also before cleanup, so the document's style keeps its positions;
        // and before any output, so a bad style prints and opens nothing.
        let style = self.prepare_style(&md, opened.as_ref(), sink)?;

        // Guarded cleanup under the default rules, with `--indent` (which only
        // parses 2, 4, or 8) as an explicit override. The style never enters it.
        md.cleanup_with(&MarkdownRuleOverrides {
            indentation: self.options.indent.and_then(Indentation::from_spaces).map_or(Leaf::Absent, Leaf::Value),
            ..MarkdownRuleOverrides::default()
        });

        if let Some(sink) = sink {
            sink.document(document_markdown(&md, true, self.output.options.width));
            return Ok(());
        }
        self.write(&md, resolved.as_deref(), opened.as_ref(), &style)
    }

    /// The effective style, its trigger findings and warnings reported and
    /// its denied warnings refused. A failure still reports every warning
    /// the inputs that were read raised, except a denied one, which is the
    /// failure's own error record.
    fn prepare_style(&self, md: &Markdown, opened: Option<&OpenedArgument>, sink: Option<&ReportSink>) -> Result<RenderStyle> {
        let options = self.options;
        let read = read_cli_style_inputs(md, opened, &options.styles, &options.setters, self.request);
        match sink {
            Some(sink) => sink.diagnostics(read.inputs.trigger_findings.iter().map(finding_diagnostic).collect()),
            None => emit_warnings(&read.inputs.trigger_findings, &stderr_terminal()),
        }
        let context = match opened {
            Some(opened) => opened.document_context(self.request)?,
            None => self.request.launch_context()?.clone(),
        };
        let policy = StylePreparationPolicy::render().with_deny_style(self.deny_style);
        let page = builds_page(self.output.target);
        let layer = document_style_layer(md, opened.map(OpenedArgument::path), context);
        let style = match prepare_read_render_style(read, layer, policy, page) {
            Ok(style) => style,
            Err(failure) => {
                Self::report_style_warnings(failure.warnings.iter(), sink, page);
                return Err(failure.error);
            }
        };

        let document = opened.map(|opened| PathBuf::from(biscuit_file::to_portable_string(opened.path())));
        let inline_rules: Vec<Diagnostic> =
            scan_inline_hr_warnings(md.content()).iter().map(|warning| inline_rule_record(warning, document.as_deref())).collect();
        let denied = self.stop_if_denied(&style, &inline_rules);
        let reported = style.warnings.iter().filter(|warning| denied.is_ok() || !self.blocks(&warning.record));
        Self::report_style_warnings(reported, sink, page);
        denied?;
        Ok(style)
    }

    /// Reports `warnings` to the data output's `sink`, or on stderr.
    fn report_style_warnings<'w>(warnings: impl Iterator<Item = &'w StyleWarningRecord>, sink: Option<&ReportSink>, page: bool) {
        for warning in warnings {
            match sink {
                Some(sink) => sink.diagnostics(vec![warning.record.clone()]),
                // A formatted source output builds no page, so it shows only
                // the CLI layers' warnings.
                None if page || !warning.from_document => stderr_warning(&warning.record.message),
                None => {}
            }
        }
    }

    /// Whether the policy makes the style warning `record` an error.
    fn blocks(&self, record: &Diagnostic) -> bool {
        self.deny_style || self.blocking.blocks(Some(&record.code), Severity::Warning)
    }

    /// Fails when the policy blocks a style warning: `--deny style` blocks the
    /// body's inline horizontal-rule styles (preparation already refused the
    /// rest), and `--deny <code>` each warning with that code. The failure's
    /// records are the blocked warnings, as errors; the caller reports the rest.
    fn stop_if_denied(&self, style: &RenderStyle, inline_rules: &[Diagnostic]) -> Result<()> {
        let mut records: Vec<Diagnostic> = if self.deny_style {
            inline_rules.to_vec()
        } else {
            style.warnings.iter().map(|warning| warning.record.clone()).chain(inline_rules.iter().cloned()).collect()
        };
        let Some(first) = records.iter().find(|record| self.blocks(record)).cloned() else {
            return Ok(());
        };
        records.retain(|record| self.blocks(record));
        for record in &mut records {
            record.severity = Severity::Error;
        }
        let denied = if self.deny_style { "style" } else { first.code.as_str() };
        Err(ClassifiedFailure::located(eyre!("the style was rejected by `--deny {denied}`: {}", first.message), records))
    }

    fn write(&self, md: &Markdown, input: Option<&Path>, opened: Option<&OpenedArgument>, style: &RenderStyle) -> Result<()> {
        let target = self.output.target;
        let presentation = PagePresentation {
            style: &style.style,
            features: &self.options.features,
            mermaid: self.options.mermaid,
        };
        let page = match target {
            OutputTarget::Terminal | OutputTarget::Plain => {
                let inputs = PageInputs::for_opened(self.request, opened)?;
                terminal_page(input, &inputs, &presentation, &Terminal::from_output_options(&self.output.options))?
            }
            OutputTarget::Html | OutputTarget::MarkdownPlus => {
                let inputs = PageInputs::for_opened(self.request, opened)?;
                html_page(&inputs, &presentation)?
            }
            // The source text needs no page; its style is never read.
            _ => darkmatter::layout::DarkmatterPage::new(&Terminal::new_optimistic(120)),
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

/// `error`, when it is an input that did not resolve and looks like a
/// misspelled setter, with the setter spelling suggested.
fn with_setter_hint(error: Report, input: Option<&PathBuf>) -> Report {
    let Some(hint) = input.and_then(|input| setter_hint(input.as_os_str())) else {
        return error;
    };
    match error.downcast::<DocumentArgumentError>() {
        Ok(argument) => Report::new(argument.with_setter_hint(hint)),
        Err(error) => error,
    }
}

/// A body's inline horizontal-rule style (`--- { style: waves }`) as a
/// deprecation record in `document`.
fn inline_rule_record(warning: &StyleWarning, document: Option<&Path>) -> Diagnostic {
    let message = match &warning.kind {
        StyleWarningKind::Deprecated { replacement } => format!("deprecated style key `{}`; use `{replacement}`", warning.path),
        _ => format!("deprecated style key `{}`", warning.path),
    };
    let mut record = output_diagnostic(STYLE_DEPRECATED_CODE, Severity::Warning, &message);
    record.source = document.map(|document| DiagnosticSource::file_only(Some(document.to_path_buf())));
    record
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
    request: &MdRequest,
) -> Result<CommandValue<D>> {
    let target = output.target;
    match target {
        OutputTarget::Markdown => return Ok(CommandValue::new(data, empty_document()).with_markdown(text)),
        OutputTarget::Json | OutputTarget::Json5 | OutputTarget::Yaml | OutputTarget::Toml => {
            return Ok(CommandValue::new(data, empty_document()));
        }
        _ => {}
    }
    // The document's own style only: no render override reaches an edit.
    let inputs = PageInputs::for_resolved(request, input.map(PathBuf::as_path))?;
    let style = document_style(document, input.map(PathBuf::as_path), inputs.document_context.clone())?;
    for diagnostic in style.diagnostics() {
        if let Some(message) = diagnostic.message() {
            stderr_warning(&message);
        }
    }
    let presentation = PagePresentation { style: &style, features: &[], mermaid: false };
    let page = match target {
        OutputTarget::Html | OutputTarget::MarkdownPlus => html_page(&inputs, &presentation)?,
        _ => terminal_page(input.map(PathBuf::as_path), &inputs, &presentation, &Terminal::from_output_options(&output.options))?,
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
