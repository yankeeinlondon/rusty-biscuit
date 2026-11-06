//! `md schema triggers` implementation.

use biscuit_terminal::components::list::OrderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use renderable::markdown::code_span;
use renderable::OutputOptions;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::command::{CommandValue, TriggersData, prose_document};
use crate::args::Cli;
use crate::commands::trigger_findings::{emit_warnings, finding_diagnostic};
use crate::io::{DocumentOutsideRepository, open_argument};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::output::stderr_terminal;
use crate::request::MdRequest;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, InvalidSchemasDir, InvalidTriggerFinding, SchemaSearchRoot, SchemaRootKind, SchemaRootState, TriggerTrace,
    trace_registry,
};
use std::path::Path;

/// Reports the five schema roots, shadowing, the triggers discovery skipped as
/// invalid, and condition-by-condition trigger results.
///
/// In a formatted output each skipped trigger's `dm.schema.invalid_trigger`
/// warning is written to stderr and stdout lists only its source; a data output
/// carries both.
pub fn run_triggers(file: &Path, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let skeleton = TriggersData {
        file: Some(file.display().to_string()),
        roots: Vec::new(),
        shadowed: Vec::new(),
        skipped: Vec::new(),
        triggers: Vec::new(),
    };
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::typed(&skeleton))?;
    let result = triggers_value(file, run.is_data(), request);
    run.finish(result)
}

fn triggers_value(file: &Path, is_data: bool, request: &MdRequest) -> Result<Completed<CommandValue<TriggersData>>> {
    // The argument resolves first, so a tree escape or malformed reference is
    // `InvalidReference` rather than a later missing-repository error.
    let opened = open_argument(file, request)?;
    // Legacy-spelling canonicalization: a verbatim `\\?\` result would gain a
    // path segment the gix-derived repository root lacks.
    let document_path = biscuit_file::canonicalize_simplified(opened.path())
        .unwrap_or_else(|_| opened.path().to_path_buf());
    let markdown = Markdown::try_from(document_path.as_path())?;
    let context = request.document_context(Some(opened.reference()), &document_path)?;
    if context.repository_root().is_none() {
        return Err(DocumentOutsideRepository { document: opened.path().to_path_buf() }.into());
    }
    let api = DarkmatterSchemas::new(context).with_trigger_discovery()?;
    let registry = api
        .trigger_registry()
        .expect("trigger discovery always installs a registry");
    let frontmatter = serde_json::Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    let trace = trace_registry(registry, &frontmatter, Some(&document_path));
    let findings = registry.findings().to_vec();

    let data = TriggersData::new(file.display().to_string(), &trace, &findings);
    let document = opened.path().to_path_buf();
    let prose = prose_document(&trace_markdown(&document, &trace, &findings));
    let diagnostics = findings.iter().map(finding_diagnostic).collect();
    // The warning itself (with its reason) goes to stderr like every other
    // command's; the trace names only which triggers were skipped, so the
    // warning is not delivered twice.
    if !is_data {
        emit_warnings(&findings, &stderr_terminal());
    }
    let value = CommandValue::new(data, prose)
        .with_terminal(move |options| trace_report(&document, &trace, &findings, options));
    Ok(Completed::success(value).with_diagnostics(diagnostics))
}

fn row(out: &mut String, terminal: &Terminal, content: String) {
    out.push_str(&Prose::new(content).render(terminal));
    out.push('\n');
}

fn trace_report(document: &Path, trace: &TriggerTrace, findings: &[InvalidTriggerFinding], options: &OutputOptions) -> String {
    let terminal = Terminal::from_output_options(options);
    let mut out = String::new();
    row(&mut out, &terminal, format!("<bold>Document:</bold> {}", escaped(document)));
    row(&mut out, &terminal, "<bold>Schema roots (search order):</bold>".to_string());
    let mut roots = OrderedList::empty();
    for root in trace.roots.entries() {
        roots.add(Prose::new(RootLine::of(root).prose()));
    }
    let roots = roots.render(&terminal);
    out.push_str(&roots);
    out.push('\n');

    row(&mut out, &terminal, "<bold>Shadowed envelopes:</bold>".to_string());
    if trace.shadowed.is_empty() {
        row(&mut out, &terminal, "- <dim>none</dim>".to_string());
    }
    for (path, winner) in &trace.shadowed {
        row(&mut out, &terminal, format!("- {} <dim>(shadowed by {})</dim>", escaped(path), escaped(winner)));
    }

    row(&mut out, &terminal, "<bold>Skipped invalid triggers:</bold>".to_string());
    if findings.is_empty() {
        row(&mut out, &terminal, "- <dim>none</dim>".to_string());
    }
    for finding in findings {
        row(&mut out, &terminal, format!(
            "- {} <dim>[{}]</dim>",
            Prose::escape_text(&finding.source().to_string()),
            finding.code()
        ));
    }

    row(&mut out, &terminal, "<bold>Triggers:</bold>".to_string());
    if trace.triggers.is_empty() {
        row(&mut out, &terminal, "- <dim>none</dim>".to_string());
    }
    for trigger in &trace.triggers {
        let status = if trigger.matched { "<green>matched</green>" } else { "<red>not matched</red>" };
        row(&mut out, &terminal, format!("- {} — {status}", Prose::escape_text(&trigger.source.to_string())));
        for condition in &trigger.conditions {
            let result = if condition.matched {
                "<green>matched</green>".to_string()
            } else {
                format!(
                    "<red>defeated</red>: {}",
                    Prose::escape_text_outside_code_spans(
                        condition.defeat.as_deref().unwrap_or("condition did not match")
                    )
                )
            };
            row(&mut out, &terminal, format!("  - condition {} — {result}", condition.index + 1));
        }
    }
    out
}

/// The trace as a Markdown document, for the Markdown and HTML outputs.
fn trace_markdown(document: &Path, trace: &TriggerTrace, findings: &[InvalidTriggerFinding]) -> String {
    let mut out = format!("**Document:** {}\n\n**Schema roots (search order):**\n\n", code_span(&document.display().to_string()));
    for (index, root) in trace.roots.entries().iter().enumerate() {
        let line = RootLine::of(root);
        let value = line.value.as_deref().map(|value| format!(" {}", code_span(value))).unwrap_or_default();
        let note = line.note.map(|note| format!(" {note}")).unwrap_or_default();
        out.push_str(&format!("{}. {}:{value}{note}\n", index + 1, line.label));
    }

    out.push_str("\n**Shadowed envelopes:**\n\n");
    if trace.shadowed.is_empty() {
        out.push_str("- none\n");
    }
    for (path, winner) in &trace.shadowed {
        out.push_str(&format!(
            "- {} (shadowed by {})\n",
            code_span(&path.display().to_string()),
            code_span(&winner.display().to_string())
        ));
    }

    out.push_str("\n**Skipped invalid triggers:**\n\n");
    if findings.is_empty() {
        out.push_str("- none\n");
    }
    for finding in findings {
        out.push_str(&format!("- {} {}\n", code_span(&finding.source().to_string()), code_span(finding.code())));
    }

    out.push_str("\n**Triggers:**\n\n");
    if trace.triggers.is_empty() {
        out.push_str("- none\n");
    }
    for trigger in &trace.triggers {
        let status = if trigger.matched { "matched" } else { "not matched" };
        out.push_str(&format!("- {} — {status}\n", code_span(&trigger.source.to_string())));
        for condition in &trigger.conditions {
            let result = if condition.matched {
                "matched".to_string()
            } else {
                format!("defeated: {}", condition.defeat.as_deref().unwrap_or("condition did not match"))
            };
            out.push_str(&format!("  - condition {} — {result}\n", condition.index + 1));
        }
    }
    out
}

/// What a formatted output says about one root.
struct RootLine {
    label: String,
    /// The folder, or an invalid variable's value.
    value: Option<String>,
    /// Why it is not searched, when it is not.
    note: Option<String>,
    invalid: bool,
}

impl RootLine {
    fn of(root: &SchemaSearchRoot) -> Self {
        let line = |value: Option<String>, note: Option<String>, invalid| Self { label: root.kind.label().to_string(), value, note, invalid };
        match &root.state {
            SchemaRootState::Searched(path) => line(Some(path.display().to_string()), None, false),
            SchemaRootState::Absent(path) => line(Some(path.display().to_string()), Some("(absent)".to_string()), false),
            SchemaRootState::Duplicate { path, of } => line(
                Some(path.display().to_string()),
                Some(format!("(same folder as the {}; searched there)", of.label())),
                false,
            ),
            SchemaRootState::NotApplicable => {
                let reason = match root.kind {
                    SchemaRootKind::Package => "none (the document is not in a package)",
                    SchemaRootKind::PackageArea => "none (the document is not in a package area)",
                    SchemaRootKind::Tree => "none",
                    SchemaRootKind::SchemasDir => "unset",
                    SchemaRootKind::Home => "none (no home directory)",
                };
                line(None, Some(reason.to_string()), false)
            }
            SchemaRootState::Invalid { value, reason } => {
                let reason = match reason {
                    InvalidSchemasDir::Empty => "invalid (empty)",
                    InvalidSchemasDir::Relative => "invalid (not an absolute path)",
                };
                line(Some(value.clone()), Some(reason.to_string()), true)
            }
        }
    }

    /// This line as Prose markup.
    fn prose(&self) -> String {
        let Self { label, value, note, invalid } = self;
        let value = match value {
            Some(value) if *invalid => format!(" {}", code_span(value)),
            Some(value) => format!(" {}", Prose::escape_text(value)),
            None => String::new(),
        };
        let note = match note {
            Some(note) if *invalid => format!(" <red>{note}</red>"),
            Some(note) => format!(" <dim>{note}</dim>"),
            None => String::new(),
        };
        format!("{label}:{value}{note}")
    }
}

fn escaped(path: &Path) -> String {
    Prose::escape_text(&path.display().to_string())
}
