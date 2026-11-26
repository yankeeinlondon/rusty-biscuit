//! The sequence check: the library's recovering preflight walk
//! ([`check_sequence_tree`]), reported task by task.
//!
//! Every task the walk reached is its own subject, registered in authored
//! order (a container before its children) with its enclosing groups and
//! sequences in its trail. A task that failed to read reports that failure
//! and, for a prompt task, leaves its provider and inputs unchecked; a task
//! that was read gets its provider, model, inputs, and preview checks.

use std::collections::HashSet;

use claudine::composition::{
    CallerInputLayers, CheckFailure, CheckedTask, MissingProperty, RootList, SequenceCheck, SequenceCheckRequest,
    SiteFrame, TaskSite, check_sequence_tree,
};
use darkmatter::findings::TrailFrame;
use darkmatter::markdown::compose::passive::PassiveSession;

use super::*;

/// A missing required input.
pub(crate) const MISSING_INPUT: &str = "composition.missing_properties";

/// One missing input, graded once the whole cohort is known.
struct MissingRow {
    subject: SubjectId,
    trail: Trail,
    property: MissingProperty,
    declared_in: PathBuf,
    hint: String,
}

/// The severity of every missing input of a run: a warning when the real
/// run asks for them (it may collect, and every one has a shape it can ask
/// for), an error when it would refuse the batch.
pub(super) fn missing_input_severity(can_collect: bool, every_shape_supported: bool) -> Severity {
    if can_collect && every_shape_supported { Severity::Warning } else { Severity::Error }
}

/// The YAML path of an authored property such as `sequence[0].group.tasks[1]`.
pub(super) fn property_segments(property: &str) -> Vec<YamlPathSegment> {
    let mut segments = Vec::new();
    for part in property.split('.').filter(|part| !part.is_empty()) {
        let (key, mut rest) = part.split_once('[').map_or((part, ""), |(key, rest)| (key, rest));
        if !key.is_empty() {
            segments.push(YamlPathSegment::Key(key.to_string()));
        }
        while let Some((index, tail)) = rest.split_once(']') {
            if let Ok(index) = index.parse() {
                segments.push(YamlPathSegment::Index(index));
            }
            rest = tail.strip_prefix('[').unwrap_or(tail);
        }
    }
    segments
}

/// Whether `property` names an item of a `sequence` list (where `agent` and
/// `model` are step data), not a group leg.
fn is_sequence_item(property: &str) -> bool {
    property.rsplit('.').next().is_some_and(|last| last.starts_with("sequence["))
}

fn frame_noun(executable: Option<ExecutableField>) -> &'static str {
    match executable {
        Some(ExecutableField::Group) => "group",
        Some(ExecutableField::Sequence) => "sequence",
        _ => "task",
    }
}

/// The trail of a task at `site`: its enclosing tasks, then the task.
fn site_trail(site: &TaskSite) -> Trail {
    let frames = site
        .frames
        .iter()
        .map(|frame: &SiteFrame| TrailFrame::Host {
            host: frame_noun(frame.executable),
            label: frame.name.clone(),
            property: frame.property.clone(),
            source: None,
        })
        .chain(std::iter::once(TrailFrame::Host {
            host: "task",
            label: site.name.clone(),
            property: site.property.clone(),
            source: None,
        }))
        .collect();
    Trail::new(frames)
}

/// The selection hints `frontmatter` holds, when they name an `agent`.
fn hints_of(path: &Path, frontmatter: &Map<String, Value>) -> Option<(PathBuf, claudine::composition::EffectiveSelectionHints)> {
    crate::commands::wrap::sequence::providers::hints_from_map(frontmatter)
        .ok()
        .filter(claudine::composition::EffectiveSelectionHints::names_agent)
        .map(|hints| (path.to_path_buf(), hints))
}

impl Check {
    /// The root's selection hints under the caller's setters, as the real
    /// run's `collect_sites` reads them, when they name an `agent`.
    fn root_hints(&self) -> Option<(PathBuf, claudine::composition::EffectiveSelectionHints)> {
        let root = self.root_ref();
        let mut frontmatter: Map<String, Value> =
            root.markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        frontmatter.extend(self.setters.clone());
        hints_of(&root.resolved_path, &frontmatter)
    }

    /// A `prompt:` task's prediction and its enclosing sequence documents,
    /// nearest first, ending with the root, as `collect_sites` gathers them.
    fn task_hints(&self, task: &CheckedTask) -> selection::TaskHints {
        let documents: Vec<_> = task.site.frames.iter().rev()
            .filter_map(|frame| self.sequence_documents.get(frame.identity.as_str()))
            .collect();
        let mut enclosing: Vec<_> =
            documents.iter().filter_map(|(path, frontmatter)| hints_of(path, frontmatter)).collect();
        enclosing.extend(self.root_hints());
        let mut consulted: Vec<_> = documents.into_iter().map(|(path, _)| path.clone()).collect();
        consulted.push(self.root_ref().resolved_path.clone());
        selection::TaskHints::Task { prediction: task.provider.clone().map(Box::new), enclosing, consulted }
    }

    pub(super) fn check_sequence(&mut self) {
        let root = self.root_ref().clone();
        let mut removed = HashSet::new();
        self.check_schema_keys(&root.resolved_path, &mut removed);
        let frontmatter = root.markdown.frontmatter().as_map().clone();
        if let Some(Some(true)) =
            frontmatter.get("interactive").map(composition::parse_interactive_hint).and_then(Result::ok)
        {
            let subject = self.root_subject();
            let error = CompositionError::SequenceInteractiveRejected(root.resolved_path.clone());
            let source = self.locate(&[YamlPathSegment::Key("interactive".into())], false);
            let code = claudine::diagnostics::as_diagnostic(&error).map_or(INTERNAL_BUG, |known| known.code());
            self.sink.push(
                subject,
                Diagnostic::actionable(
                    code,
                    Severity::Error,
                    error.to_string(),
                    "remove `interactive: true`; a sequence always runs its tasks non-interactively",
                )
                .with_source(source),
            );
        }
        if let Some(other) = frontmatter.get("sequence").filter(|value| !value.is_array() && !value.is_string()) {
            let subject = self.root_subject();
            let source = self.locate(&[YamlPathSegment::Key("sequence".into())], true);
            self.sink.push(
                subject,
                Diagnostic::actionable(
                    SEQUENCE_SHAPE,
                    Severity::Error,
                    format!("`sequence` is {}, not a list of tasks or a file reference", json_kind(other)),
                    "write `sequence:` as a list of tasks",
                )
                .with_source(source),
            );
            self.sink.not_checked(subject, "tasks", "`sequence` is not a list", None);
            return;
        }

        let Some(invocation) = self.invocation.clone() else { return };
        let source_context = match invocation.derive_composition_source(&root) {
            Ok(context) => context,
            Err(error) => {
                let subject = self.root_subject();
                self.push_error(subject, &error, Trail::default());
                return;
            }
        };
        let compose_context = invocation.capture_launch_context(&ContextRequirements::for_document(&root.markdown));
        let layers = (!self.setters.is_empty()).then(|| {
            CallerInputLayers::from_caller_overrides(
                Some(Value::Object(self.setters.clone())),
                invocation.launch_file_resolution_context().clone(),
            )
        });
        let check = check_sequence_tree(SequenceCheckRequest {
            source: &root,
            compose_context,
            invocation: &invocation,
            source_context: &source_context,
            caller_layers: layers.as_ref(),
            launch_dir: self.launch_dir.as_deref(),
        });
        for file in &check.files {
            if *file != root.resolved_path {
                self.check_schema_keys(file, &mut removed);
            }
        }

        let root_subject = self.root_subject();
        let mut blocker = None;
        for failure in &check.root_failures {
            if let Some(id) = self.push_failure(root_subject, failure, Trail::default(), None, &removed) {
                blocker.get_or_insert(id);
            }
        }
        // A prompt task whose owner is missing an input waits for it, as the
        // real run's discovery leaves its reference pending.
        self.waiting_prompts = match &check.inventory {
            Ok(inventory) => inventory
                .prompts
                .iter()
                .filter(|prompt| inventory.owners.get(prompt.owner).is_some_and(|owner| owner.missing.is_some()))
                .map(|prompt| (prompt.source_path.clone(), prompt.property.clone()))
                .collect(),
            Err(_) => HashSet::new(),
        };
        self.sequence_documents = check.tasks.iter()
            .filter_map(|task| Some((task.site.identity.as_str().to_string(), task.sequence_document.clone()?)))
            .collect();
        let mut rows = Vec::new();
        match check.list {
            RootList::Body => {
                self.tasks = 1;
                if self.root_is_bodyless() {
                    let message = format!("{} has no `sequence` and no body to compose", self.root_label());
                    self.sink.push(
                        root_subject,
                        Diagnostic::actionable(
                            NO_EXECUTABLE,
                            Severity::Error,
                            message,
                            "add a `sequence:` list of tasks, or a body for the document to compose",
                        )
                        .with_source(DiagnosticSource::file_only(Some(root.resolved_path.clone()))),
                    );
                } else {
                    let task = PromptTask {
                        subject: root_subject,
                        trail: Trail::default(),
                        prompt: &root,
                        with_evaluated: Map::new(),
                        with_late: Vec::new(),
                        step_agent: None,
                        selection: selection::TaskHints::Body { document: self.root_hints() },
                        label: self.root_label(),
                        task_path: root.resolved_path.clone(),
                        seq: None,
                    };
                    self.check_prompt_task(&task);
                }
            }
            RootList::Command => {
                self.sink.not_checked(root_subject, "tasks", "the list comes from a shell command a dry run skips", None);
            }
            RootList::Unread => {
                self.sink.not_checked(root_subject, "tasks", "`sequence` could not be read", blocker);
            }
            RootList::Read => {
                for task in &check.tasks {
                    self.check_task(task, &removed, &mut rows);
                }
            }
        }
        self.owner_inputs(&check, &mut rows);
        self.push_missing_rows(rows);
        self.container_schemas(&check);
    }

    /// An inline container's `$schema` judges the values it sees at its
    /// turn, which a dry run never reaches: one `not checked` row each.
    fn container_schemas(&mut self, check: &SequenceCheck) {
        let Ok(inventory) = &check.inventory else { return };
        for container in inventory.inline_containers.iter().filter(|container| container.checks) {
            let task = check.tasks.iter().find(|task| {
                task.site.file == container.source_path && task.site.property == container.property
            });
            let subject = task
                .and_then(|task| self.task_subjects.get(task.site.identity.as_str()).copied())
                .unwrap_or_else(|| self.root_subject());
            self.sink.not_checked(subject, "`$schema`", "an inline container's schema judges the values at its turn", None);
        }
    }

    /// The key findings of Claudine's shipped schemas on `file` as
    /// authored: a removed or undeclared key (an error, which the parser
    /// would refuse at its own boundary, often before the rest is read) and
    /// near-miss advice on an open step (a warning). Each is reported once
    /// per owner and key. A task, group, catalog, or sequence YAML file is
    /// judged by its `kind`; the root by its triggers. Typed values are left
    /// to the parser and each prompt's own composition, since a templated
    /// value is not final here.
    ///
    /// `removed` collects each `(file, key)` reported as removed, so the
    /// parser's refusal of the same key is not reported a second time.
    pub(super) fn check_schema_keys(&mut self, file: &Path, removed: &mut HashSet<(PathBuf, String)>) {
        use simplified_schema::ValidationProblemCode;
        let is_root = self.root.as_ref().is_some_and(|root| root.resolved_path == file);
        let text = if is_root {
            self.root_ref().original_text.clone()
        } else {
            match std::fs::read_to_string(file) {
                Ok(text) => text,
                Err(_) => return,
            }
        };
        let Some(context) = self
            .invocation
            .as_ref()
            .and_then(|invocation| invocation.derive_source(file).ok())
            .map(|source| source.file_resolution_context().clone())
        else {
            return;
        };
        let schemas = claudine::composition::schema::bundle::with_claudine_registration(
            darkmatter::markdown::schemas::DarkmatterSchemas::new(context),
        );
        let report = if composition::is_yaml_source(file) {
            match schemas.validate_yaml_document(&text, file) {
                Ok(darkmatter::markdown::schemas::YamlOutcome::Validated(validation)) => validation.report,
                _ => return,
            }
        } else if is_root {
            match schemas.validate(&self.root_ref().markdown) {
                Ok(report) => report,
                Err(_) => return,
            }
        } else {
            return;
        };
        let keyed = report
            .problems
            .iter()
            .filter(|problem| matches!(problem.code, ValidationProblemCode::RemovedKey | ValidationProblemCode::UnknownKey))
            .map(|problem| (problem, Severity::Error))
            .chain(report.key_advice.iter().map(|advice| (advice, Severity::Warning)));
        let mut seen = HashSet::new();
        let mut subject = None;
        for (problem, severity) in keyed {
            if !seen.insert((problem.code.diagnostic_code(), problem.path.clone(), problem.offending_property.clone())) {
                continue;
            }
            if problem.code == ValidationProblemCode::RemovedKey
                && let Some(key) = &problem.offending_property
            {
                removed.insert((file.to_path_buf(), key.clone()));
            }
            let subject = *subject.get_or_insert_with(|| {
                if is_root {
                    self.root_subject()
                } else {
                    self.sink.register_subject(Subject::document(
                        shown(file, self.launch_dir.as_deref()),
                        Some(file.to_path_buf()),
                    ))
                }
            });
            let source = match (problem.line, problem.column) {
                (Some(line), column) => {
                    DiagnosticSource::at(Some(file.to_path_buf()), line as usize, column.unwrap_or(1) as usize)
                }
                (None, _) => DiagnosticSource::file_only(Some(file.to_path_buf())),
            };
            self.sink.push(
                subject,
                Diagnostic::projected(problem.code.diagnostic_code(), severity, problem.message.clone(), problem.hint.clone())
                    .with_source(source),
            );
        }
    }

    /// Records `failure` for `subject`: its reference finding (located at
    /// `source`) when it has one, else its projected error. A removed key the
    /// schema check already reported is not reported again (`None`).
    fn push_failure(
        &mut self,
        subject: SubjectId,
        failure: &CheckFailure,
        trail: Trail,
        source: Option<DiagnosticSource>,
        removed: &HashSet<(PathBuf, String)>,
    ) -> Option<FindingId> {
        if let CompositionError::RemovedGrammar { source_path, removed: key, .. } = &failure.error
            && removed.contains(&(source_path.clone(), key.clone()))
        {
            return None;
        }
        match &failure.finding {
            Some(finding) => {
                let finding = finding.clone().with_trail(trail);
                let finding = match source {
                    Some(source) => finding.with_source(source),
                    None => finding,
                };
                Some(self.sink.push(subject, finding))
            }
            None => self.push_error(subject, &failure.error, trail),
        }
    }

    /// The authored position of `property` in `file` (its key, a child
    /// `key`'s key, or with `value` that value); file-only when not proven.
    fn locate_in(&self, file: &Path, property: &str, key: Option<&str>, value: bool) -> DiagnosticSource {
        let mut path = property_segments(property);
        if let Some(key) = key {
            path.push(YamlPathSegment::Key(key.to_string()));
        }
        let is_root = self.root.as_ref().is_some_and(|root| root.resolved_path == file);
        if is_root {
            return self.locate(&path, value);
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            return DiagnosticSource::file_only(Some(file.to_path_buf()));
        };
        let (text, first_line) = frontmatter_of(file, &text);
        let offset = if value {
            locate_yaml_value(&text, &path).map(|location| location.span.start)
        } else {
            locate_yaml_key(&text, &path).map(|span| span.start)
        };
        match offset.and_then(|offset| darkmatter::markdown::compose::expression::one_based_position(&text, offset)) {
            Some((line, column)) => DiagnosticSource::at(Some(file.to_path_buf()), line + first_line - 1, column),
            None => DiagnosticSource::file_only(Some(file.to_path_buf())),
        }
    }

    /// The findings of one task the walk reached.
    fn check_task(&mut self, task: &CheckedTask, removed: &HashSet<(PathBuf, String)>, rows: &mut Vec<MissingRow>) {
        let site = &task.site;
        let container = matches!(task.executable, Some(ExecutableField::Group | ExecutableField::Sequence));
        if !container {
            self.tasks += 1;
        }
        // A nested task is labeled by the tasks it sits in, so occurrences
        // of one referenced file's tasks read apart.
        let label = site.frames.iter().map(|frame| frame.name.as_str()).chain([site.name.as_str()])
            .collect::<Vec<_>>().join(" → ");
        let subject = self.sink.register_subject(Subject::construct(
            "task",
            label,
            site.property.clone(),
            Some(site.file.clone()),
        ));
        self.task_subjects.insert(site.identity.as_str().to_string(), subject);
        let trail = site_trail(site);
        let step = task.item.as_object().cloned().unwrap_or_default();

        if is_sequence_item(&site.property) {
            for (key, noun) in [("agent", "provider"), ("model", "model")] {
                if let Some(value) = step.get(key) {
                    let written = scalar_text(value);
                    let mut key_trail = trail.frames().to_vec();
                    if let Some(TrailFrame::Host { property, .. }) = key_trail.last_mut() {
                        *property = format!("{}.{key}", site.property);
                    }
                    let finding = Diagnostic::actionable(
                        SELECTION_KEY_AS_DATA,
                        Severity::Warning,
                        format!("`{key}: {written}` is step data (read as `seq.{key}`), not the task's {noun}"),
                        format!("to choose the {noun}, move it into the task's inputs: `with: {{ {key}: {written} }}`"),
                    )
                    .with_source(self.locate_in(&site.file, &site.property, Some(key), false))
                    .with_trail(Trail::new(key_trail));
                    self.sink.push(subject, finding);
                }
            }
        }

        let executables: Vec<ExecutableField> = step.keys().filter_map(|key| ExecutableField::from_key(key)).collect();
        if executables.len() > 1 {
            let keys: Vec<String> = executables.iter().map(|field| format!("`{}`", field.key())).collect();
            self.sink.push(
                subject,
                Diagnostic::actionable(
                    MULTIPLE_EXECUTABLES,
                    Severity::Error,
                    format!("this task names more than one executable: {}", keys.join(", ")),
                    "keep exactly one executable per task, and move the others into their own tasks",
                )
                .with_source(self.locate_in(&site.file, &site.property, None, true))
                .with_trail(trail.clone()),
            );
            self.sink.not_checked(subject, "provider", "more than one executable", None);
            self.sink.not_checked(subject, "inputs", "more than one executable", None);
            return;
        }

        if !task.failures.is_empty() {
            let mut blocker = None;
            for failure in &task.failures {
                // A reference finding points at the reference as written,
                // when the task itself wrote it (not a task file it names).
                let written_here = failure.finding.as_ref()
                    .and_then(|finding| finding.reference_details.as_ref())
                    .and_then(|details| details.base.as_deref())
                    == Some(to_forward_slash_string(&site.file).as_str());
                let source = written_here.then(|| {
                    task.executable.map(|field| self.locate_in(&site.file, &site.property, Some(field.key()), true))
                }).flatten();
                // …and its trail ends at the executable's property.
                let failure_trail = match (&failure.finding, task.executable) {
                    (Some(_), Some(field)) => {
                        let mut frames = trail.frames().to_vec();
                        if let Some(TrailFrame::Host { property, .. }) = frames.last_mut() {
                            *property = format!("{}.{}", site.property, field.key());
                        }
                        Trail::new(frames)
                    }
                    _ => trail.clone(),
                };
                if let Some(id) = self.push_failure(subject, failure, failure_trail, source, removed) {
                    blocker.get_or_insert(id);
                }
            }
            // A task whose prompt was read (a refused lifecycle literal) is
            // still checked; one whose prompt was never reached is not.
            if task.prompt.is_none() {
                if task.executable == Some(ExecutableField::Prompt) {
                    let reason = if task.failures.iter().any(|failure| failure.finding.is_some()) {
                        "the prompt file could not be read"
                    } else {
                        "the task could not be read"
                    };
                    self.sink.not_checked(subject, "provider", reason, blocker);
                    self.sink.not_checked(subject, "inputs", reason, blocker);
                }
                return;
            }
        }

        let step_agent = step.get("agent").map(scalar_text);
        match (task.executable, &task.prompt) {
            (None, _) => self.check_body_task(task, subject, trail, step, step_agent, rows),
            (_, Some(prompt)) => {
                let Some(document) = self.load_document(subject, prompt, &trail) else { return };
                if self.waiting_prompts.contains(&(site.file.clone(), site.property.clone())) {
                    let prompt_task = PromptTask {
                        subject,
                        trail,
                        prompt: &document,
                        with_evaluated: Map::new(),
                        with_late: Vec::new(),
                        step_agent,
                        selection: self.task_hints(task),
                        label: site.name.clone(),
                        task_path: site.file.clone(),
                        seq: task.seq.clone(),
                    };
                    self.check_selection(&prompt_task);
                    let reason = "it waits for an input its sequence has no value for";
                    self.sink.not_checked(subject, "inputs", reason, None);
                    self.sink.not_checked(subject, "preview", reason, None);
                    return;
                }
                let prompt_task = PromptTask {
                    subject,
                    trail,
                    prompt: &document,
                    with_evaluated: task.with_evaluated.clone(),
                    with_late: task.with_late.clone(),
                    step_agent,
                    selection: self.task_hints(task),
                    label: site.name.clone(),
                    task_path: site.file.clone(),
                    seq: task.seq.clone(),
                };
                self.check_task_prompt(&prompt_task, rows);
            }
            (Some(ExecutableField::Shell), None) => {
                self.sink.not_checked(subject, "command", "a dry run runs no commands", None);
            }
            (Some(ExecutableField::SideEffect), None) => {
                self.sink.not_checked(subject, "side effect", "a dry run dispatches no side effects", None);
            }
            // A container's children are tasks of their own; a task file's
            // command or side effect is not run.
            (Some(_), None) => {}
        }
    }

    /// A task with no executable, which composes the body of the document
    /// that declares it.
    fn check_body_task(
        &mut self,
        task: &CheckedTask,
        subject: SubjectId,
        trail: Trail,
        step: Map<String, Value>,
        step_agent: Option<String>,
        rows: &mut Vec<MissingRow>,
    ) {
        let site = &task.site;
        let is_root = self.root.as_ref().is_some_and(|root| root.resolved_path == site.file);
        let document = if is_root {
            self.root_ref().clone()
        } else {
            match self.load_document(subject, &site.file, &trail) {
                Some(document) => document,
                None => return,
            }
        };
        let label = shown(&document.resolved_path, self.launch_dir.as_deref());
        // A string `prompt:` makes the step an inline composition of it,
        // which needs no body.
        let inline = matches!(document.markdown.frontmatter().as_map().get("prompt"), Some(Value::String(_)));
        let bodyless = !inline
            && (composition::is_yaml_source(&document.resolved_path)
                || darkmatter::markdown::render_tree::embedded::without_embedded_metadata(document.markdown.content())
                    .trim()
                    .is_empty());
        if bodyless {
            let why = if composition::is_yaml_source(&document.resolved_path) {
                "and a YAML file has no body"
            } else {
                "and its body is empty"
            };
            let source = if step.contains_key("name") {
                self.locate_in(&site.file, &site.property, Some("name"), false)
            } else {
                self.locate_in(&site.file, &site.property, None, true)
            };
            let blocker = self.sink.push(
                subject,
                Diagnostic::actionable(
                    NO_EXECUTABLE,
                    Severity::Error,
                    format!("this task has no executable, so it would compose the body of {label}, {why}"),
                    format!("add an executable such as `prompt: ./{}.md`", site.name),
                )
                .with_source(source)
                .with_trail(trail),
            );
            self.sink.not_checked(subject, "provider", "no executable", Some(blocker));
            self.sink.not_checked(subject, "inputs", "no executable", Some(blocker));
            return;
        }
        self.sink.push(
            subject,
            Diagnostic::projected(
                NO_EXECUTABLE,
                Severity::Info,
                if inline {
                    format!("this task has no executable, so it composes the `prompt` of {label}")
                } else {
                    format!("this task has no executable, so it composes the body of {label}")
                },
                None,
            )
            .with_trail(trail.clone()),
        );
        let with = step.get("with").and_then(Value::as_object).cloned().unwrap_or_default();
        // A body task's `with:` is not evaluated by the walk: a literal value
        // is data, and a template is known only at the task's turn.
        let (with_late, with_evaluated): (Vec<_>, Map<_, _>) = {
            let (late, known): (Vec<_>, Vec<_>) =
                with.iter().partition(|(_, value)| value.as_str().is_some_and(|text| text.contains("{{")));
            (late.into_iter().map(|(key, _)| key.clone()).collect(), known.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        };
        // A body step composes the document that declares it, whose `agent`
        // is its own: the root's under the caller's setters, a referenced
        // sequence document's as written.
        let own = if is_root {
            self.root_hints()
        } else {
            let frontmatter: Map<String, Value> = document.markdown.frontmatter().as_map().iter()
                .map(|(key, value)| (key.clone(), value.clone())).collect();
            hints_of(&document.resolved_path, &frontmatter)
        };
        let prompt_task = PromptTask {
            subject,
            trail,
            prompt: &document,
            with_evaluated,
            with_late,
            step_agent,
            selection: selection::TaskHints::Body { document: own },
            label: site.name.clone(),
            task_path: site.file.clone(),
            seq: task.seq.clone(),
        };
        // The root's own inputs are the root owner's, reported once.
        if is_root {
            self.check_prompt_task(&prompt_task);
        } else {
            self.check_task_prompt(&prompt_task, rows);
        }
    }

    /// The provider, model, inputs, and preview checks of a task's prompt.
    /// A missing input leaves the preview unchecked: it would show the
    /// prompt without that value.
    fn check_task_prompt(&mut self, task: &PromptTask<'_>, rows: &mut Vec<MissingRow>) {
        self.check_selection(task);
        let before = rows.len();
        self.prompt_inputs(task, rows);
        if rows.len() > before {
            self.sink.not_checked(task.subject, "preview", "an input has no value", None);
        } else {
            self.preview(task);
        }
    }

    /// Reads the document at `path`, recording why it could not be read.
    fn load_document(&mut self, subject: SubjectId, path: &Path, trail: &Trail) -> Option<ResolvedCompositionSource> {
        let invocation = self.invocation.clone()?;
        let raw = path.to_string_lossy().into_owned();
        match composition::resolve_composition_source_in_context(&raw, invocation.launch_file_resolution_context()) {
            Ok(document) => Some(document),
            Err(error) => {
                self.push_error(subject, &error, trail.clone());
                None
            }
        }
    }

    /// The missing inputs of a prompt task's document, prepared passively
    /// under the caller's setters and the task's `with:`.
    fn prompt_inputs(&mut self, task: &PromptTask<'_>, rows: &mut Vec<MissingRow>) {
        if !task.with_late.is_empty() {
            self.sink.not_checked(task.subject, "inputs", "a `with:` value is known only when the task runs", None);
            return;
        }
        // A value that comes from a command stays as written; an input it
        // would supply still counts as given.
        let Some(options) = self.passive_prepare_options(task, PassiveSession::new()) else { return };
        match composition::sequence::document::prepare_sequence_document_values(
            task.prompt,
            options,
            composition::CompositionMode::ChainedDocument,
        ) {
            Ok(_) => {}
            Err(CompositionError::MissingProperties { source_path, missing, .. }) => {
                for property in missing {
                    rows.push(MissingRow {
                        subject: task.subject,
                        trail: task.trail.clone(),
                        hint: format!("add `{}` to the task's `with:`, or pass `{}=<value>` after the file", property.name, property.name),
                        property,
                        declared_in: source_path.clone(),
                    });
                }
            }
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
            }
        }
    }

    /// The missing inputs of every owning sequence document: the root's
    /// under the root, a referenced document's under the task that
    /// references it.
    ///
    /// A real run discovers inputs after the root's `initialize` fires and
    /// reads its writes as setters for every owner, so an input that stack
    /// may write is `not checked` rather than missing.
    fn owner_inputs(&mut self, check: &SequenceCheck, rows: &mut Vec<MissingRow>) {
        let Ok(inventory) = &check.inventory else { return };
        let mut initialized = std::collections::HashSet::new();
        if let Some(initialize) = self.root_ref().markdown.frontmatter().as_map().get("initialize") {
            claudine::composition::collect_written_keys(initialize, &mut initialized);
        }
        for owner in &inventory.owners {
            let Some(entry) = &owner.missing else { continue };
            let (subject, trail) = match owner.role {
                claudine::composition::InputOwnerRole::Root | claudine::composition::InputOwnerRole::RootReference => {
                    (self.root_subject(), Trail::default())
                }
                claudine::composition::InputOwnerRole::Nested => {
                    let referenced_from = owner.referenced_from.as_deref();
                    let task = check.tasks.iter().find(|task| {
                        Some(task.site.file.as_path()) == referenced_from
                            && format!("{}.sequence", task.site.property) == owner.property
                    });
                    match task.and_then(|task| Some((self.task_subjects.get(task.site.identity.as_str()).copied()?, task))) {
                        Some((subject, task)) => (subject, site_trail(&task.site)),
                        None => (self.root_subject(), Trail::default()),
                    }
                }
            };
            for property in &entry.missing {
                if initialized.contains(&property.name) {
                    self.sink.not_checked(subject, &format!("input `{}`", property.name), "the root's `initialize` writes it when it runs", None);
                    continue;
                }
                rows.push(MissingRow {
                    subject,
                    trail: trail.clone(),
                    hint: format!("pass `{}=<value>` after the file, or give it a default", property.name),
                    property: property.clone(),
                    declared_in: entry.source_path.clone(),
                });
            }
        }
    }

    /// One `composition.missing_properties` row per missing input, graded
    /// by whether the real run would collect the whole cohort.
    fn push_missing_rows(&mut self, rows: Vec<MissingRow>) {
        if rows.is_empty() {
            return;
        }
        // As `resolve_interactive_options` does, an unreadable configuration
        // leaves the default (ask).
        let interactive = claudine::composition::schema::InteractiveSchemaOptions {
            prompt_for_missing: self.config.as_ref().is_none_or(|config| config.prompt_for_missing),
            stdin_is_tty: std::io::stdin().is_terminal(),
            stderr_is_tty: self.stderr_is_tty,
            silent: false,
        };
        let every_shape = rows.iter().all(|row| row.property.interactive_shape.is_some());
        let severity = missing_input_severity(interactive.allowed(), every_shape);
        for row in rows {
            let kind = row.property.type_label.as_deref().map(|label| format!(" ({label})")).unwrap_or_default();
            let message = match severity {
                Severity::Warning => format!("`{}`{kind} has no value; an interactive run asks for it", row.property.name),
                _ => format!("`{}`{kind} is required and has no value; a non-interactive run fails here", row.property.name),
            };
            self.sink.push(
                row.subject,
                Diagnostic::actionable(MISSING_INPUT, severity, message, row.hint)
                    .with_source(DiagnosticSource::file_only(Some(row.declared_in)))
                    .with_trail(row.trail),
            );
        }
    }
}
