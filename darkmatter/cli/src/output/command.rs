//! One command's delivery: its target and width, its run-level diagnostics, and
//! the one thing it writes to stdout.
//!
//! A handler builds a [`CommandValue`](darkmatter::markdown::output::command::CommandValue)
//! and hands it to [`CommandRun::finish`]. For a data output the value, or a
//! failure's payload, is one value on stdout and stderr stays empty. For a
//! formatted output the value goes to stdout, and a failure with no value goes
//! back to `main`, which reports it on stderr.
//!
//! `outcome` and `diagnostics` are added here, by [`DataShape`], so a value can
//! never disagree with the exit status.

use biscuit_terminal::output::{OutputTarget, WidthRequest, render_output};
use color_eyre::eyre::{Report, Result};
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::markdown::compose::report_diagnostics::with_document_fallback;
use darkmatter::markdown::output::command::empty_document;
use renderable::CommandOutput;
use renderable::tree::Document;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

use super::report::{INTERRUPTED, ReportedExit, failure_diagnostics};
use super::{OutputEnvironment, ResolvedOutput, SERIALIZATION_CODE, output_diagnostic, resolve_output, stderr_warning, width_warning_diagnostic, write_stdout_text};

/// How a command's data carries its run status.
#[derive(Debug, Clone)]
pub enum DataShape {
    /// A shape new with `--output`: every value has `outcome` and `diagnostics`.
    /// The skeleton is the value for a failure that produced none, with
    /// whatever fields are already known and null for the rest.
    Typed(Value),
    /// An object older than `--output`: unchanged on a clean success, and it
    /// gains `outcome` and `diagnostics` when a run needs them.
    Object,
    /// A value older than `--output` whose fields belong to the user or may not
    /// be an object at all: unchanged on a clean success, otherwise the
    /// `value` of an `{ outcome, diagnostics, value }` object.
    Bare,
}

impl DataShape {
    /// [`DataShape::Typed`] with `skeleton`'s fields.
    #[must_use]
    pub fn typed(skeleton: impl Serialize) -> Self {
        Self::Typed(serde_json::to_value(skeleton).expect("a skeleton is JSON"))
    }

    fn status(&self, value: Option<Value>, success: bool, diagnostics: Vec<Diagnostic>) -> Value {
        let outcome = if success { "success" } else { "failure" };
        let diagnostics = serde_json::to_value(diagnostics).expect("a diagnostic is JSON");
        // `md clean`'s value already has a `diagnostics` field, whose rows are
        // positions in the document; its meaning is a published contract.
        let with_status = |mut object: Map<String, Value>| {
            object.insert("outcome".to_string(), Value::String(outcome.to_string()));
            let field = if object.contains_key("diagnostics") { "run_diagnostics" } else { "diagnostics" };
            object.insert(field.to_string(), diagnostics.clone());
            Value::Object(object)
        };
        let clean_success = success && diagnostics.as_array().is_some_and(Vec::is_empty);
        match self {
            Self::Typed(skeleton) => match value.unwrap_or_else(|| skeleton.clone()) {
                Value::Object(object) => with_status(object),
                other => json!({ "outcome": outcome, "diagnostics": diagnostics, "value": other }),
            },
            Self::Object => match value {
                Some(value) if clean_success => value,
                Some(Value::Object(object)) => with_status(object),
                other => json!({ "outcome": outcome, "diagnostics": diagnostics, "value": other }),
            },
            Self::Bare => match value {
                Some(value) if clean_success => value,
                other => json!({ "outcome": outcome, "diagnostics": diagnostics, "value": other }),
            },
        }
    }
}

/// Whether a value's text is printed as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextEnding {
    /// Text ends in exactly one newline; empty text stays empty.
    Terminated,
    /// A document's Markdown keeps its bytes.
    ExactMarkdown,
}

/// What a command produced.
pub struct Completed<T> {
    value: T,
    failed: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<T> Completed<T> {
    /// A successful run.
    #[must_use]
    pub fn success(value: T) -> Self {
        Self { value, failed: false, diagnostics: Vec::new() }
    }

    /// A run that finished with `value` and found the problem it reports: a
    /// report is the deliverable even when it reports failure.
    #[must_use]
    pub fn failure(value: T) -> Self {
        Self { value, failed: true, diagnostics: Vec::new() }
    }

    /// Adds `diagnostics` to the run's data output.
    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.diagnostics.extend(diagnostics);
        self
    }
}

/// One command's output selection, with what it has to say about the run.
#[derive(Debug)]
pub struct CommandRun {
    output: ResolvedOutput,
    shape: DataShape,
    ending: TextEnding,
    compact: bool,
    diagnostics: Vec<Diagnostic>,
    document: Option<PathBuf>,
}

impl CommandRun {
    /// Resolves `target` and `width`. A width warning goes to stderr now for a
    /// formatted output, and into the payload for a data output.
    ///
    /// ## Errors
    ///
    /// A usage error when a needed `OUTPUT_COLUMNS` is invalid or a percentage
    /// resolves outside the column range.
    pub fn begin(
        target: OutputTarget,
        width: Option<WidthRequest>,
        default_width: WidthRequest,
        verbosity: u8,
        shape: DataShape,
    ) -> Result<Self> {
        let output = resolve_output(target, width, default_width, verbosity, &OutputEnvironment::detect())?;
        let mut diagnostics = Vec::new();
        if let Some(warning) = &output.width_warning {
            if target.is_data() {
                diagnostics.push(width_warning_diagnostic(warning));
            } else {
                stderr_warning(warning);
            }
        }
        Ok(Self { output, shape, ending: TextEnding::Terminated, compact: false, diagnostics, document: None })
    }

    /// A document command: its Markdown keeps its exact bytes, and everything
    /// else ends as usual.
    #[must_use]
    pub fn exact_markdown(mut self) -> Self {
        self.ending = TextEnding::ExactMarkdown;
        self
    }

    /// `md get --compact`: JSON on one line, which is also JSON5.
    #[must_use]
    pub fn compact_json(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Names the one document this run works on, so a failure whose typed
    /// error holds no file still reports it.
    pub fn set_document(&mut self, document: &Path) {
        self.document = Some(document.to_path_buf());
    }

    /// Adds run-level diagnostics that reach a data output whatever the result.
    pub fn add_diagnostics(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// The resolved output.
    #[must_use]
    pub fn output(&self) -> &ResolvedOutput {
        &self.output
    }

    /// The selected target.
    #[must_use]
    pub fn target(&self) -> OutputTarget {
        self.output.target
    }

    /// Whether this run prints a data value.
    #[must_use]
    pub fn is_data(&self) -> bool {
        self.output.target.is_data()
    }

    /// Writes the one thing this run prints and returns how the command ends.
    ///
    /// A data output turns a failure into a payload. A formatted output leaves
    /// a failure with no value to `main`. An interrupted run and a usage error
    /// pass through unreported: no command result exists.
    ///
    /// ## Errors
    ///
    /// [`ReportedExit`] for a failed run; the original error where `main` still
    /// has to report it.
    pub fn finish<T: CommandOutput>(self, result: Result<Completed<T>>) -> Result<()> {
        match result {
            Err(error) if passes_through(&error) => Err(error),
            Err(error) if !self.is_data() => Err(error),
            Err(error) => {
                let mut diagnostics = self.diagnostics.clone();
                let mut failure = failure_diagnostics(&error);
                with_document_fallback(&mut failure, self.document.as_deref());
                diagnostics.extend(failure);
                self.write_data(None, false, diagnostics)
            }
            Ok(completed) if self.is_data() => {
                let value = serde_json::to_value(completed.value.as_data()).map_err(|error| {
                    color_eyre::eyre::eyre!("could not serialize the result: {error}")
                });
                let mut diagnostics = self.diagnostics.clone();
                diagnostics.extend(completed.diagnostics);
                match value {
                    Ok(value) => self.write_data(Some(value), !completed.failed, diagnostics),
                    Err(error) => {
                        diagnostics.extend(failure_diagnostics(&error));
                        self.write_data(None, false, diagnostics)
                    }
                }
            }
            Ok(completed) => {
                let rendered = render_output(&completed.value, self.target(), &self.output.options)
                    .map_err(|error| color_eyre::eyre::eyre!(error))?;
                self.write_text(rendered.text)?;
                if completed.failed { Err(ReportedExit(1).into()) } else { Ok(()) }
            }
        }
    }

    /// Writes this run's data value: `value`, or the failure payload.
    fn write_data(&self, value: Option<Value>, success: bool, diagnostics: Vec<Diagnostic>) -> Result<()> {
        let payload = self.shape.status(value, success, diagnostics);
        let mut failed = !success;
        if self.compact {
            let text = serde_json::to_string(&payload).map_err(|error| color_eyre::eyre::eyre!(error))?;
            self.write_text(text)?;
            return if failed { Err(ReportedExit(1).into()) } else { Ok(()) };
        }
        let text = match render_output(&DataValue(payload), self.target(), &self.output.options) {
            Ok(rendered) => rendered.text,
            Err(error) => {
                failed = true;
                let reduced = vec![output_diagnostic(SERIALIZATION_CODE, Severity::Error, &error.to_string())];
                let payload = self.shape.status(None, false, reduced);
                match render_output(&DataValue(payload), self.target(), &self.output.options) {
                    Ok(rendered) => rendered.text,
                    // The reduced value holds no data of the run's; nothing
                    // else can be said in this format.
                    Err(_) => return Err(ReportedExit(1).into()),
                }
            }
        };
        self.write_text(text)?;
        if failed { Err(ReportedExit(1).into()) } else { Ok(()) }
    }

    /// A closed stdout has nowhere to say so; the status reports it.
    fn write_text(&self, text: String) -> Result<()> {
        let exact = self.ending == TextEnding::ExactMarkdown
            && matches!(self.target(), OutputTarget::Markdown | OutputTarget::MarkdownPlus);
        let text = if exact { text } else { terminated(text) };
        write_stdout_text(&text).map_err(|_| ReportedExit(1).into())
    }
}

/// An interrupted run or a usage error has no result to report.
fn passes_through(error: &Report) -> bool {
    error.downcast_ref::<clap::Error>().is_some()
        || error.downcast_ref::<ReportedExit>().is_some_and(|exit| exit.0 == INTERRUPTED)
}

fn terminated(mut text: String) -> String {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// A finished data value on its way to a serializer.
struct DataValue(Value);

impl CommandOutput for DataValue {
    type Data = Value;

    fn as_prose(&self) -> Document {
        empty_document()
    }

    fn as_data(&self) -> Value {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning() -> Vec<Diagnostic> {
        vec![output_diagnostic("dm.test.warned", Severity::Warning, "warned")]
    }

    #[test]
    fn a_typed_value_always_has_its_status() {
        let shape = DataShape::typed(json!({ "file": null }));
        let value = shape.status(Some(json!({ "file": "a.md" })), true, Vec::new());
        assert_eq!(value, json!({ "file": "a.md", "outcome": "success", "diagnostics": [] }));
    }

    #[test]
    fn a_typed_failure_with_no_value_is_its_skeleton() {
        let shape = DataShape::typed(json!({ "file": "a.md", "code": null }));
        let value = shape.status(None, false, Vec::new());
        assert_eq!(value, json!({ "file": "a.md", "code": null, "outcome": "failure", "diagnostics": [] }));
    }

    #[test]
    fn an_older_object_is_unchanged_until_a_run_needs_its_status() {
        let shape = DataShape::Object;
        assert_eq!(shape.status(Some(json!({ "a": 1 })), true, Vec::new()), json!({ "a": 1 }));
        let failed = shape.status(Some(json!({ "a": 1 })), false, Vec::new());
        assert_eq!(failed, json!({ "a": 1, "outcome": "failure", "diagnostics": [] }));
        let warned = shape.status(Some(json!({ "a": 1 })), true, warning());
        assert_eq!(warned["outcome"], "success");
        assert_eq!(warned["diagnostics"][0]["code"], "dm.test.warned");
    }

    #[test]
    fn an_older_object_failing_with_no_value_has_a_null_value() {
        let value = DataShape::Object.status(None, false, Vec::new());
        assert_eq!(value, json!({ "outcome": "failure", "diagnostics": [], "value": null }));
    }

    #[test]
    fn a_bare_value_is_never_merged_with_status_fields() {
        let shape = DataShape::Bare;
        assert_eq!(shape.status(Some(json!("x")), true, Vec::new()), json!("x"));
        // The user's `outcome` property must stay theirs.
        let user = json!({ "outcome": "mine" });
        assert_eq!(shape.status(Some(user.clone()), true, Vec::new()), user);
        let warned = shape.status(Some(user.clone()), true, warning());
        assert_eq!(warned["value"], user);
        assert_eq!(warned["outcome"], "success");
    }

    #[test]
    fn a_field_named_diagnostics_keeps_its_meaning() {
        let value = DataShape::Object.status(Some(json!({ "diagnostics": ["own"] })), false, warning());
        assert_eq!(value["diagnostics"], json!(["own"]));
        assert_eq!(value["run_diagnostics"][0]["code"], "dm.test.warned");
    }

    #[test]
    fn text_gains_a_final_newline_only_when_it_has_text() {
        assert_eq!(terminated("a".to_string()), "a\n");
        assert_eq!(terminated("a\n".to_string()), "a\n");
        assert_eq!(terminated(String::new()), "");
    }
}
