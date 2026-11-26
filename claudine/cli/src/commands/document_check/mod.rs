//! The passive check and preview behind `--dry-run` for `compose`,
//! `inline-compose`, and `sequence`.
//!
//! A dry run reports every problem it can observe without mutating anything
//! (see `claudine/docs/cli/sequence.md#dry-run`): it reads documents and the
//! read-only `PATH` snapshot, prepares each prompt through the canonical
//! preparer in its passive mode (no command runs, nothing is cached, nothing
//! asks; what needs one of those is left as written and reported as not
//! checked), and records each failure as a finding in one [`FindingSink`]
//! instead of stopping. Nothing here reaches the
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
use darkmatter::findings::reference::{DocumentCheck, DocumentConstruct, ListingMemo, check_document};
use darkmatter::findings::{
    Diagnostic, DiagnosticSource, EMPTY_BODY_CODE, FindingId, FindingReport, FindingSink, HumanLayout,
    NOT_CHECKED_CODE, Severity, Subject, SubjectId, Trail,
};
use darkmatter::markdown::compose::ContextRequirements;
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

const BUG_HINT: &str = "this is a defect in Claudine; please report it with the command you ran";

/// Which report the passive check writes.
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

/// Whether `command` is a document command spelling the removed
/// `--dry-run-shell`, with or without `--dry-run`.
pub(crate) fn asks_for_dry_run_shell(command: &Commands) -> bool {
    match command {
        Commands::Compose(args) => args.shared.dry_run_shell,
        Commands::InlineCompose(args) => args.shared.dry_run_shell,
        Commands::Sequence(args) => args.shared.dry_run_shell,
        _ => false,
    }
}

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
pub(crate) fn run(mode: Mode, shared: &SharedComposeArgs, args: &[String]) -> i32 {
    let format = if shared.json { Format::Json } else { Format::Human };
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
    /// The `with:` values known before the run, evaluated as the task's turn
    /// will see them.
    with_evaluated: Map<String, Value>,
    /// The `with:` keys known only at the task's turn.
    with_late: Vec<String>,
    /// The task's step-level `agent:` as written, which it reads as data.
    step_agent: Option<String>,
    /// Where its provider hints come from.
    selection: selection::TaskHints,
    /// The label provider errors name the task by.
    label: String,
    /// The document that wrote the task.
    task_path: PathBuf,
    /// The `seq` binding a sequence step's document reads at its turn.
    seq: Option<Value>,
}

/// What a [`Check`] is run for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Purpose {
    /// `--dry-run`: every check, and the previews.
    DryRun,
    /// A real sequence stopped at preflight: every check, no preview kept.
    Failure,
    /// A real sequence passed preflight: only the checks no prompt or
    /// review answers (the authored keys and the walk), so no finding
    /// depends on an answer the run has already collected.
    Advisory,
}

struct Check {
    mode: Mode,
    purpose: Purpose,
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
    /// The user configuration, read passively; `None` when it is invalid
    /// (the finding is recorded) or not yet read.
    config: Option<claudine::config::claudine_config::ClaudineConfig>,
    selection_env: Option<selection::SelectionEnv>,
    previews: Vec<String>,
    tasks: usize,
    /// Each walked task's subject, by its identity.
    task_subjects: std::collections::HashMap<String, SubjectId>,
    /// Each `sequence:` task's sequence document and frontmatter, by its
    /// identity.
    sequence_documents: std::collections::HashMap<String, (PathBuf, Map<String, Value>)>,
    /// The `(file, property)` of each prompt task whose owner is missing an
    /// input.
    waiting_prompts: std::collections::HashSet<(PathBuf, String)>,
    /// Whether the root misses an input its `initialize` does not write, so
    /// a body task of the root waits for it as a prompt task would.
    root_waits: bool,
    /// The tasks whose `agent` value provider selection already reported
    /// as unusable; their document's schema verdict on that same `agent`
    /// is not reported again.
    agent_reported: std::collections::HashSet<SubjectId>,
}

impl Check {
    fn new(mode: Mode, shared: SharedComposeArgs, stderr_is_tty: bool) -> Self {
        Self {
            mode,
            purpose: Purpose::DryRun,
            shared,
            stderr_is_tty,
            launch_dir: None,
            sink: FindingSink::new(),
            memo: ListingMemo::new(),
            root_subject: None,
            root: None,
            setters: Map::new(),
            invocation: None,
            config: None,
            selection_env: None,
            previews: Vec::new(),
            tasks: 0,
            task_subjects: std::collections::HashMap::new(),
            sequence_documents: std::collections::HashMap::new(),
            waiting_prompts: std::collections::HashSet::new(),
            root_waits: false,
            agent_reported: std::collections::HashSet::new(),
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
        self.read_config(&invocation);
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
            let construct =
                DocumentConstruct { construct: "file", noun: "document", launch_dir: self.launch_dir.as_deref() };
            // The root argument is the user's own request: no read permission
            // governs it, so every location is admitted.
            let admit_all = |_: &std::path::Path| Ok::<(), std::convert::Infallible>(());
            match check_document(&reference, context, construct, &mut self.memo, admit_all) {
                DocumentCheck::Missing(finding) => {
                    let subject = self.root_subject();
                    self.sink.push(subject, *finding);
                    return None;
                }
                DocumentCheck::Refused(never) => match never {},
                DocumentCheck::Found(_) | DocumentCheck::Failed(_) => {}
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
    /// Darkmatter's catch-all `dm.compose.failure` record means it classified
    /// nothing, so it is never reported. A file-reference
    /// failure carries its class (`failure`), as a real run's `failure:` row,
    /// and a sequence error its block's hint
    /// ([`CompositionError::sequence_hint`]) at its site's file. A schema
    /// verdict only on an `agent` value selection already reported for the
    /// subject is not reported (`None`).
    fn push_error(
        &mut self,
        subject: SubjectId,
        error: &(dyn std::error::Error + 'static),
        trail: Trail,
    ) -> Option<FindingId> {
        use darkmatter::markdown::compose::report_diagnostics::{COMPOSE_FAILURE_CODE, error_diagnostics};
        if self.agent_reported.contains(&subject)
            && let Some(CompositionError::SchemaValidation { problems, .. }) = error.downcast_ref::<CompositionError>()
            && !problems.is_empty()
            && problems.iter().all(|problem| problem.trim_start_matches('/') == "agent")
        {
            // Selection's row names the `agent` value and its fix; the
            // schema's verdict on the same value says less.
            return None;
        }
        let projected = error_diagnostics(error);
        let unclassified = matches!(projected.as_slice(), [only] if only.code == COMPOSE_FAILURE_CODE);
        // Darkmatter's catch-all is never the answer: Claudine's own code
        // speaks for an error it types, and an error neither crate types is
        // `internal.bug`.
        if !projected.is_empty() && !unclassified {
            let mut first = None;
            for diagnostic in projected {
                let diagnostic = if trail.is_empty() { diagnostic } else { diagnostic.with_trail(trail.clone()) };
                let id = self.sink.push(subject, diagnostic);
                first.get_or_insert(id);
            }
            return first;
        }
        let composition = error.downcast_ref::<CompositionError>();
        let mut diagnostic = match claudine::diagnostics::as_diagnostic(error) {
            // A sequence error's row says what its block says: the same
            // hint, at its file.
            Some(known) => {
                let hint = composition.and_then(CompositionError::sequence_hint);
                let diagnostic = Diagnostic::projected(known.code(), Severity::Error, error.to_string(), hint);
                match composition.and_then(CompositionError::sequence_site).and_then(|site| site.file.clone()) {
                    Some(file) => diagnostic.with_source(DiagnosticSource::file_only(Some(file))),
                    None => diagnostic,
                }
            }
            None => Diagnostic::actionable(INTERNAL_BUG, Severity::Error, error.to_string(), BUG_HINT),
        };
        if let Some(failure) = error.downcast_ref::<CompositionError>().and_then(CompositionError::resolution_failure) {
            diagnostic.reference_details = Some(darkmatter::findings::ReferenceDetails {
                failure: Some(darkmatter::markdown::errors::resolution_failure_name(failure).to_string()),
                ..Default::default()
            });
        }
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
        let task = PromptTask {
            subject,
            trail: Trail::default(),
            prompt: &root,
            with_evaluated: Map::new(),
            with_late: Vec::new(),
            step_agent: None,
            selection: selection::TaskHints::Document,
            label: self.root_label(),
            task_path: root.resolved_path.clone(),
            seq: None,
        };
        self.check_prompt_task(&task);
    }

    /// The provider, model, and preview checks of one prompt task.
    fn check_prompt_task(&mut self, task: &PromptTask<'_>) {
        self.check_selection(task);
        self.preview(task);
    }

    /// The provider and model checks of one prompt task.
    fn check_selection(&mut self, task: &PromptTask<'_>) {
        if self.purpose == Purpose::Advisory {
            return;
        }
        match self.select_provider(task) {
            Some(target) => self.check_model(task, &target),
            None => {
                self.sink.not_checked(task.subject, "model requirement", "no provider", None);
            }
        }
    }

    /// Reads the user configuration passively (nothing is created,
    /// converted, or backed up), recording an invalid one as a root finding.
    /// Its notes (a missing file read as its defaults, a legacy file read as
    /// its conversion) are context, not findings.
    fn read_config(&mut self, invocation: &InvocationContext) {
        let repository = invocation.launch_repository();
        match claudine::dispatch::loader::load_claudine_config_passive(None, repository.repo_root()) {
            Ok(loaded) => self.config = Some(loaded.config),
            Err(error) => {
                let subject = self.root_subject();
                let source = config_source(&error);
                let finding = match claudine::diagnostics::as_diagnostic(&error) {
                    Some(known) => Diagnostic::actionable(
                        known.code(),
                        Severity::Error,
                        error.to_string(),
                        "fix the configuration file; a real run refuses it before it starts",
                    ),
                    None => Diagnostic::actionable(INTERNAL_BUG, Severity::Error, error.to_string(), BUG_HINT),
                };
                self.sink.push(subject, finding.with_source(source));
            }
        }
    }

}

/// The state a real sequence run is in where its passive report is taken.
pub(crate) struct RunState<'a> {
    pub(crate) root: &'a ResolvedCompositionSource,
    pub(crate) invocation: &'a InvocationContext,
    pub(crate) shared: &'a SharedComposeArgs,
    /// The caller's values, then every runtime write so far (the root's
    /// `initialize` included), as the run's next preparation reads them.
    pub(crate) values: Map<String, Value>,
}

/// The passive check of a real sequence that stopped at preflight, taken
/// where it stopped: its errors and warnings, or `None` when it observed
/// neither. No preview is composed into the report and nothing runs.
pub(crate) fn preflight_failure_report(state: &RunState<'_>) -> Option<FindingReport> {
    let report = run_for(Purpose::Failure, state);
    let report = retain_severities(&report, &[Severity::Error, Severity::Warning]);
    report.has_problems().then_some(report)
}

/// The warnings a real sequence that passed preflight has not shown: the
/// key advice and step-data warnings of every file its walk read. `None`
/// when there are none.
pub(crate) fn preflight_advisories(state: &RunState<'_>) -> Option<FindingReport> {
    let report = run_for(Purpose::Advisory, state);
    let report = retain_severities(&report, &[Severity::Warning]);
    report.has_problems().then_some(report)
}

fn run_for(purpose: Purpose, state: &RunState<'_>) -> FindingReport {
    let mut check = Check::new(Mode::Sequence, state.shared.clone(), std::io::stderr().is_terminal());
    check.purpose = purpose;
    check.launch_dir = Some(state.invocation.launch_cwd().to_path_buf());
    check.read_config(state.invocation);
    check.invocation = Some(state.invocation.clone());
    check.root = Some(state.root.clone());
    check.setters = state.values.clone();
    check.check_sequence();
    check.sink.finish()
}

/// `report` with only the findings of a `keep` severity, and only the
/// subjects that still hold one.
fn retain_severities(report: &FindingReport, keep: &[Severity]) -> FindingReport {
    let mut sink = FindingSink::new();
    let mut subjects = std::collections::HashMap::new();
    for entry in report.entries() {
        if !keep.contains(&entry.diagnostic().severity) {
            continue;
        }
        let Some(subject) = report.subject(entry.subject()) else { continue };
        let id = *subjects.entry(entry.subject()).or_insert_with(|| sink.register_subject(subject.clone()));
        sink.push(id, entry.diagnostic().clone());
    }
    sink.finish()
}

/// A real run's preflight report for a person: a heading, the findings,
/// and the failure that stopped the run (`stopped`; `None` for advisories
/// on a run that goes on) when no finding already states it. Ends in a
/// newline.
pub(crate) fn render_preflight_report(
    report: &FindingReport,
    stopped: Option<&str>,
    term: &biscuit_terminal::terminal::Terminal,
    launch_dir: Option<&Path>,
) -> String {
    let layout = HumanLayout { width: Some(term.width()), launch_dir: launch_dir.map(Path::to_path_buf) };
    let counts = report.counts();
    let found = format!("{} and {}", outcome::plural(counts.errors, "error"), outcome::plural(counts.warnings, "warning"));
    let title = match stopped {
        Some(_) => format!("Sequence preflight found {found}; no task ran"),
        None => format!("Sequence preflight found {found}; the sequence runs"),
    };
    let mut out = format!("{title}\n\n");
    let body = report.render_human(term, &layout);
    if !body.is_empty() {
        out.push_str(&body);
        out.push('\n');
    }
    // The failure is named only when no row already says it (the failure's
    // text, or its text under a wrapper's `…: ` prefix): the first row can
    // belong to another task than the one the run stopped at.
    if let Some(stopped) = stopped {
        let stopped = claudine::harness::concise_message(stopped);
        let words = |text: &str| text.split_whitespace().map(str::to_string).collect::<Vec<_>>();
        let failure = words(&stopped);
        let said = report.diagnostics().iter().any(|diagnostic| {
            let row = words(&diagnostic.message);
            !row.is_empty() && failure.ends_with(&row) && (row.len() == failure.len() || failure[failure.len() - row.len() - 1].ends_with(':'))
        });
        if !said {
            out.push_str(&format!("\nThe run stopped at: {stopped}\n"));
        }
    }
    out
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

/// The first located problem of a configuration error, else its file.
fn config_source(error: &claudine::error::ClaudineError) -> DiagnosticSource {
    use claudine::error::ClaudineError;
    match error {
        ClaudineError::ConfigInvalid { problems } | ClaudineError::ConfigConversion { problems } => {
            match problems.first() {
                Some(problem) => match problem.line {
                    Some(line) => DiagnosticSource::at(problem.file.clone(), line, problem.column.unwrap_or(1)),
                    None => DiagnosticSource::file_only(problem.file.clone()),
                },
                None => DiagnosticSource::file_only(None),
            }
        }
        _ => DiagnosticSource::file_only(None),
    }
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
