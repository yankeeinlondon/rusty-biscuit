//! The passive check and preview behind `--dry-run` for `compose`,
//! `inline-compose`, and `sequence`.
//!
//! A dry run reports every problem it can observe without mutating anything
//! (see `claudine/docs/cli/sequence.md#dry-run`): it reads documents and the
//! read-only `PATH` snapshot, composes each prompt with Darkmatter's dry-run
//! options (no command runs, nothing is cached), and records each failure as a
//! finding in one [`FindingSink`] instead of stopping. Nothing here reaches the
//! standard error report: the human report goes to stderr, previews to stdout,
//! and `--json` replaces both with one envelope on stdout.
//!
//! It is dispatched from `main.rs` before Darkmatter's configuration load, the
//! wrapper, and `ensure_config_exists`, so it creates, migrates, and refreshes
//! nothing.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use biscuit_file::yaml::{YamlPathSegment, locate_yaml_key, locate_yaml_value};
use biscuit_file::{FileReference, FileResolutionContext, to_forward_slash_string};
use claudine::composition::{self, CompositionError, ExecutableField, ResolvedCompositionSource};
use claudine::invocation_context::InvocationContext;
use claudine::provider::{Provider, provider_info};
use darkmatter::findings::reference::{DocumentConstruct, ListingMemo, missing_document};
use darkmatter::findings::{
    Diagnostic, DiagnosticSource, EMPTY_BODY_CODE, EmptyBodyCause, FindingId, FindingReport, FindingSink, HumanLayout,
    NOT_CHECKED_CODE, Severity, Subject, SubjectId, Trail,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{CacheAccessMode, ContextRequirements};
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;
use darkmatter::markdown::MarkdownError;
use serde_json::{Map, Value};

use super::compose::SharedComposeArgs;
use crate::args::Commands;

mod outcome;
mod preview;
mod selection;
mod tasks;

use outcome::{Outcome, shown};

/// A step-level `agent:` or `model:` that a task reads as data, not as its
/// provider or model.
pub(crate) const SELECTION_KEY_AS_DATA: &str = "composition.selection_key_as_data";
/// A task with no executable over a document with no body.
pub(crate) const NO_EXECUTABLE: &str = "composition.no_executable";
/// A task naming more than one executable.
pub(crate) const MULTIPLE_EXECUTABLES: &str = "composition.multiple_executables";
/// A `sequence` value or item of the wrong shape.
pub(crate) const SEQUENCE_SHAPE: &str = "composition.sequence_shape";
/// No provider is chosen by any link of the selection chain.
pub(crate) const PROVIDER_UNRESOLVED: &str = "composition.provider_unresolved";
/// A provider name no provider answers to.
pub(crate) const PROVIDER_UNKNOWN: &str = "composition.provider_unknown";
/// A chosen provider whose binary is not on `PATH`.
pub(crate) const PROVIDER_NOT_INSTALLED: &str = "composition.provider_not_installed";
/// A provider that needs a model for this launch, with no model found.
pub(crate) const MODEL_REQUIRED: &str = "composition.model_required";
/// The command-line positionals do not parse.
pub(crate) const INVALID_ARGUMENTS: &str = "composition.invalid_arguments";
/// An error no projector classifies: a defect in Claudine.
pub(crate) const INTERNAL_BUG: &str = "internal.bug";

/// Every code this module emits itself, beside the Darkmatter finding codes
/// and the projected codes of the errors it catches.
#[cfg(test)]
pub(crate) const CHECK_CODES: &[&str] = &[
    SELECTION_KEY_AS_DATA,
    NO_EXECUTABLE,
    MULTIPLE_EXECUTABLES,
    SEQUENCE_SHAPE,
    PROVIDER_UNRESOLVED,
    PROVIDER_UNKNOWN,
    PROVIDER_NOT_INSTALLED,
    MODEL_REQUIRED,
    INVALID_ARGUMENTS,
    INTERNAL_BUG,
];

const BUG_HINT: &str = "this is a defect in Claudine; please report it with the command you ran";

/// Whether the passive check handles a document `--dry-run`.
///
/// Not yet in production: `--dry-run` keeps the task-runner preview until the
/// check walks the task tree through the library's preflight loader, because
/// today's preview validates dynamic sources, nested groups, task files, and
/// catalog entries that this walker reports as not checked. A
/// `test-fixtures` build turns it on with `CLAUDINE_TEST_PASSIVE_DRY_RUN`:
/// `1` for the human report, `json` for the JSON report the planned `--json`
/// flag will select.
pub(crate) fn enabled() -> Option<Format> {
    #[cfg(feature = "test-fixtures")]
    {
        match std::env::var_os("CLAUDINE_TEST_PASSIVE_DRY_RUN")?.to_str()? {
            "1" => Some(Format::Human),
            "json" => Some(Format::Json),
            _ => None,
        }
    }
    #[cfg(not(feature = "test-fixtures"))]
    {
        None
    }
}

/// Which report the passive check writes.
// Only the `test-fixtures` seam selects a format until `--dry-run` routes here.
#[cfg_attr(not(feature = "test-fixtures"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    /// The human report on stderr and the previews on stdout.
    Human,
    /// One JSON envelope on stdout and nothing else.
    Json,
}

/// The usage error for `--dry-run-shell` under the passive check, which runs
/// no command.
pub(crate) const DRY_RUN_SHELL_REMOVED: &str = "error: `--dry-run-shell` was removed. Dry runs skip commands.\n  hint: use `--dry-run` to check and preview, or run the command without it";

/// Which document command is being rehearsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Compose,
    InlineCompose,
    Sequence,
}

/// The dry-run invocation `command` asks for, if it is one.
pub(crate) fn dry_run_request(command: &Commands) -> Option<(Mode, &SharedComposeArgs, &[String])> {
    let (mode, shared, args) = match command {
        Commands::Compose(args) => (Mode::Compose, &args.shared, args.args.as_slice()),
        Commands::InlineCompose(args) => (Mode::InlineCompose, &args.shared, args.args.as_slice()),
        Commands::Sequence(args) => (Mode::Sequence, &args.shared, args.args.as_slice()),
        _ => return None,
    };
    shared.dry_run.then_some((mode, shared, args))
}

/// Runs the dry run and writes its output; returns the exit status: `2` when
/// any finding is an error or a warning, else `0`.
pub(crate) fn run(mode: Mode, format: Format, shared: &SharedComposeArgs, args: &[String]) -> i32 {
    if shared.dry_run_shell {
        crate::log::message(DRY_RUN_SHELL_REMOVED);
        return crate::args::EXIT_USAGE;
    }
    let stderr_is_tty = std::io::stderr().is_terminal();
    let mut check = Check::new(mode, shared.clone(), stderr_is_tty);
    check.run(args);
    let launch_dir = check.launch_dir.clone();
    let outcome = check.finish();

    if format == Format::Json {
        let envelope = outcome.envelope();
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
    } else {
        let human = outcome.human(stderr_is_tty, launch_dir.as_deref());
        let _ = std::io::stderr().lock().write_all(human.as_bytes());
        let mut stdout = std::io::stdout().lock();
        for preview in &outcome.previews {
            let _ = stdout.write_all(preview.as_bytes());
            if !preview.ends_with('\n') {
                let _ = stdout.write_all(b"\n");
            }
        }
    }
    if outcome.report.has_problems() { crate::args::EXIT_USAGE } else { 0 }
}

/// A prompt task the check composes: its document and the values it is
/// composed with.
struct PromptTask<'a> {
    subject: SubjectId,
    trail: Trail,
    prompt: &'a ResolvedCompositionSource,
    /// The task's `with:` overlay, empty outside a sequence.
    with: Map<String, Value>,
    /// The task's step-level `agent:` as written, which it reads as data.
    step_agent: Option<String>,
}

struct Check {
    mode: Mode,
    /// The command's options, with the caller tail owned once the root is
    /// read.
    shared: SharedComposeArgs,
    stderr_is_tty: bool,
    launch_dir: Option<PathBuf>,
    sink: FindingSink,
    memo: ListingMemo,
    root_subject: Option<SubjectId>,
    root: Option<ResolvedCompositionSource>,
    setters: Map<String, Value>,
    invocation: Option<InvocationContext>,
    previews: Vec<String>,
    tasks: usize,
    /// Each walked task's subject, by its identity.
    task_subjects: std::collections::HashMap<String, SubjectId>,
}

impl Check {
    fn new(mode: Mode, shared: SharedComposeArgs, stderr_is_tty: bool) -> Self {
        Self {
            mode,
            shared,
            stderr_is_tty,
            launch_dir: None,
            sink: FindingSink::new(),
            memo: ListingMemo::new(),
            root_subject: None,
            root: None,
            setters: Map::new(),
            invocation: None,
            previews: Vec::new(),
            tasks: 0,
            task_subjects: std::collections::HashMap::new(),
        }
    }

    fn finish(self) -> Outcome {
        Outcome {
            file: self.root.as_ref().map(|root| root.resolved_path.clone()),
            report: self.sink.finish(),
            previews: self.previews,
            tasks: self.tasks,
        }
    }

    fn root_subject(&mut self) -> SubjectId {
        if let Some(id) = self.root_subject {
            return id;
        }
        let (label, file) = match &self.root {
            Some(root) => (shown(&root.resolved_path, self.launch_dir.as_deref()), Some(root.resolved_path.clone())),
            None => ("command line".to_string(), None),
        };
        let id = self.sink.register_subject(Subject::document(label, file));
        self.root_subject = Some(id);
        id
    }

    fn run(&mut self, args: &[String]) {
        let parsed = match super::compose::parse_composition_positionals(args) {
            Ok(parsed) => parsed,
            Err(error) => {
                let subject = self.root_subject();
                self.sink.push(
                    subject,
                    Diagnostic::actionable(
                        INVALID_ARGUMENTS,
                        Severity::Error,
                        error.to_string(),
                        "write each setter as `key=value` after the file",
                    ),
                );
                return;
            }
        };
        let Some(file) = parsed.file_ref else {
            let subject = self.root_subject();
            self.sink.push(
                subject,
                Diagnostic::actionable(
                    INVALID_ARGUMENTS,
                    Severity::Error,
                    "no file reference was given",
                    "name the document to check, such as `claudine sequence plan.md --dry-run`",
                ),
            );
            return;
        };
        let invocation = match InvocationContext::capture(crate::request::snapshot()) {
            Ok(invocation) => invocation,
            Err(error) => {
                let subject = self.root_subject();
                self.push_error(subject, &error, Trail::default());
                return;
            }
        };
        let context = invocation.launch_file_resolution_context().clone();
        self.launch_dir = Some(invocation.launch_cwd().to_path_buf());
        self.invocation = Some(invocation);
        let Some(root) = self.load_root(&file, &context) else { return };
        // Arguments after the file are owned against the document as
        // authored, exactly as the real command owns them.
        let owned = match self.invocation.as_ref().map(|invocation| invocation.derive_composition_source(&root)) {
            Some(Ok(source)) => super::compose::ownership::own_caller_arguments(
                &mut self.shared,
                &root,
                source.file_resolution_context(),
                self.launch_dir.as_deref(),
            ),
            Some(Err(error)) => Err(error.into()),
            None => Ok(Vec::new()),
        };
        self.root = Some(root);
        let parsed = match owned
            .and_then(|owned| super::compose::parse_composition_positionals(&[args, owned.as_slice()].concat()))
        {
            Ok(parsed) => parsed,
            Err(error) => {
                let subject = self.root_subject();
                self.sink.push(
                    subject,
                    Diagnostic::actionable(
                        INVALID_ARGUMENTS,
                        Severity::Error,
                        error.to_string(),
                        "write each setter as `key=value`",
                    ),
                );
                return;
            }
        };
        match super::compose::merge_set_overrides(
            self.shared.set.as_deref(),
            parsed.shorthand_setters.clone(),
            parsed.positionals.clone(),
        ) {
            Ok(Some(Value::Object(values))) => self.setters = values,
            Ok(_) => {}
            Err(error) => {
                let subject = self.root_subject();
                self.sink.push(
                    subject,
                    Diagnostic::actionable(
                        INVALID_ARGUMENTS,
                        Severity::Error,
                        error.to_string(),
                        "pass `--set` a JSON object, and write setters as `key=value`",
                    ),
                );
                return;
            }
        }
        match self.mode {
            Mode::Sequence => self.check_sequence(),
            Mode::Compose | Mode::InlineCompose => self.check_single(),
        }
    }

    /// Reads the root document, recording why it could not be read.
    fn load_root(&mut self, file: &str, context: &FileResolutionContext) -> Option<ResolvedCompositionSource> {
        if let Ok(reference) = FileReference::new(file) {
            let detailed = reference.resolve_detailed(context);
            if let Some(finding) = missing_document(
                DocumentConstruct { construct: "file", noun: "document", launch_dir: self.launch_dir.as_deref() },
                &detailed,
                &mut self.memo,
            ) {
                let subject = self.root_subject();
                self.sink.push(subject, finding);
                return None;
            }
        }
        let loaded = match self.mode {
            Mode::Sequence => super::sequence::resolve_sequence_source(file, context),
            Mode::Compose | Mode::InlineCompose => composition::resolve_composition_source_in_context(file, context),
        };
        match loaded {
            Ok(root) => Some(root),
            Err(error) => {
                let subject = self.root_subject();
                self.push_error(subject, &error, Trail::default());
                None
            }
        }
    }

    /// Records `error` as findings for `subject`: through Darkmatter's
    /// projection, then Claudine's diagnostic code, else as `internal.bug`.
    fn push_error(
        &mut self,
        subject: SubjectId,
        error: &(dyn std::error::Error + 'static),
        trail: Trail,
    ) -> Option<FindingId> {
        let projected = darkmatter::markdown::compose::report_diagnostics::error_diagnostics(error);
        if !projected.is_empty() {
            let mut first = None;
            for diagnostic in projected {
                let diagnostic = if trail.is_empty() { diagnostic } else { diagnostic.with_trail(trail.clone()) };
                let id = self.sink.push(subject, diagnostic);
                first.get_or_insert(id);
            }
            return first;
        }
        let diagnostic = match claudine::diagnostics::as_diagnostic(error) {
            Some(known) => Diagnostic::projected(known.code(), Severity::Error, error.to_string(), None),
            None => Diagnostic::actionable(INTERNAL_BUG, Severity::Error, error.to_string(), BUG_HINT),
        };
        let diagnostic = if trail.is_empty() { diagnostic } else { diagnostic.with_trail(trail) };
        Some(self.sink.push(subject, diagnostic))
    }

    fn root_ref(&self) -> &ResolvedCompositionSource {
        self.root.as_ref().expect("the root is loaded before it is checked")
    }

    /// Frontmatter text of the root and the one-based line it starts on, for
    /// locating authored keys.
    fn root_frontmatter(&self) -> (String, usize) {
        let root = self.root_ref();
        frontmatter_of(&root.resolved_path, &root.original_text)
    }

    /// The authored position of the key (or, with `value`, the value) at
    /// `path` in the root's frontmatter; file-only when not proven.
    fn locate(&self, path: &[YamlPathSegment], value: bool) -> DiagnosticSource {
        let file = Some(self.root_ref().resolved_path.clone());
        let (text, first_line) = self.root_frontmatter();
        let offset = if value {
            locate_yaml_value(&text, path).map(|location| location.span.start)
        } else {
            locate_yaml_key(&text, path).map(|span| span.start)
        };
        match offset.and_then(|offset| darkmatter::markdown::compose::expression::one_based_position(&text, offset)) {
            Some((line, column)) => DiagnosticSource::at(file, line + first_line - 1, column),
            None => DiagnosticSource::file_only(file),
        }
    }

    /// Whether the root has no visible body to compose.
    fn root_is_bodyless(&self) -> bool {
        let root = self.root_ref();
        composition::is_yaml_source(&root.resolved_path)
            || darkmatter::markdown::render_tree::embedded::without_embedded_metadata(root.markdown.content())
                .trim()
                .is_empty()
    }

    fn root_label(&self) -> String {
        shown(&self.root_ref().resolved_path, self.launch_dir.as_deref())
    }

    fn check_single(&mut self) {
        self.tasks = 1;
        let subject = self.root_subject();
        let root = self.root_ref().clone();
        // The document's lifecycle and its literals, which its preparation
        // refuses before any provider starts.
        let frontmatter = Value::Object(
            root.markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        );
        if let Err(error) = composition::lifecycle::parse_lifecycle_config(&frontmatter, &root.resolved_path) {
            self.push_error(subject, &error, Trail::default());
        } else if let Err(error) = composition::validate_lifecycle_literals(&root.markdown, &root.resolved_path) {
            self.push_error(subject, &error, Trail::default());
        }
        let task = PromptTask { subject, trail: Trail::default(), prompt: &root, with: Map::new(), step_agent: None };
        self.check_prompt_task(&task);
    }

    /// The provider, model, and preview checks of one prompt task.
    fn check_prompt_task(&mut self, task: &PromptTask<'_>) {
        self.check_selection(task);
        self.preview(task);
    }

    /// The provider and model checks of one prompt task.
    fn check_selection(&mut self, task: &PromptTask<'_>) {
        match self.select_provider(task) {
            Some(provider) => self.check_model(task, provider),
            None => {
                self.sink.not_checked(task.subject, "model requirement", "no provider", None);
            }
        }
    }

}

/// The YAML text of `path`'s authored data (`text`): a YAML file whole, a
/// Markdown file's frontmatter, and the one-based line it starts on.
fn frontmatter_of(path: &Path, text: &str) -> (String, usize) {
    if composition::is_yaml_source(path) {
        return (text.to_string(), 1);
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else {
        return (String::new(), 1);
    };
    let end = rest.find("\n---").map_or(rest.len(), |index| index + 1);
    (rest[..end].to_string(), 2)
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| to_forward_slash_string(path), |name| name.to_string_lossy().into_owned())
}

/// A frontmatter value as an author wrote it: a string bare, anything else
/// as JSON.
fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "a mapping",
    }
}

#[cfg(test)]
mod tests;
