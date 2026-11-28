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
    CallerInputLayers, CheckFailure, CheckedTask, OwnerFailure, RootList, SequenceCheck, SequenceCheckRequest,
    SiteFrame, TaskSite, check_sequence_tree,
};
use darkmatter::findings::TrailFrame;
use darkmatter::markdown::compose::passive::DeferralReason;

use super::*;
use super::inputs::MissingRow;
use super::keys::{KeyVerdict, key_owners, pointer_segments};

/// A missing required input.
pub(crate) const MISSING_INPUT: &str = "composition.missing_properties";

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
pub(super) fn site_trail(site: &TaskSite) -> Trail {
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
        // The root's own keys are the root's; a verdict inside its list
        // waits for the task it belongs to.
        let (mut pending, own): (Vec<_>, Vec<_>) = self
            .judge_keys(&root.resolved_path)
            .into_iter()
            .partition(|verdict| verdict.owner.len() > 1 && verdict.owner[0] == "sequence");
        let root_subject = self.root_subject();
        self.push_key_rows(root_subject, &Trail::default(), &own.iter().collect::<Vec<_>>(), false);
        if self.purpose != Purpose::Advisory {
            self.sequence_supplied_files();
        }
        let frontmatter = root.markdown.frontmatter().as_map().clone();
        if let Some(Some(true)) =
            frontmatter.get("interactive").map(composition::parse_interactive_hint).and_then(Result::ok)
        {
            let subject = self.root_subject();
            let error = CompositionError::SequenceInteractiveRejected {
                path: root.resolved_path.clone(),
                site: Box::new(claudine::composition::SequenceSite::at(&root.resolved_path, "interactive")),
            };
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
            self.not_checked(subject, "tasks", "`sequence` is not a list", None);
            return;
        }

        let Some(invocation) = self.invocation.clone() else {
            self.push_key_rows(root_subject, &Trail::default(), &pending.iter().collect::<Vec<_>>(), false);
            return;
        };
        let source_context = match invocation.derive_composition_source(&root) {
            Ok(context) => context,
            Err(error) => {
                self.push_key_rows(root_subject, &Trail::default(), &pending.iter().collect::<Vec<_>>(), false);
                self.push_error(root_subject, &error, Trail::default());
                return;
            }
        };
        // The real run drops the root's invalid optional values, with a
        // warning, before it reads any task (`commands/sequence.rs`).
        if self.purpose == Purpose::DryRun {
            let (_, _, dropped) = composition::drop_invalid_optionals(
                root.clone(),
                (!self.setters.is_empty()).then(|| Value::Object(self.setters.clone())),
                Some(invocation.launch_cwd()),
                source_context.file_resolution_context(),
                invocation.trigger_findings(),
            );
            for drop in &dropped {
                if self.dropped_reported.insert((root.resolved_path.clone(), drop.property.clone())) {
                    let row = super::preview::dropped_row(self, &root.resolved_path, drop);
                    self.sink.push(root_subject, row);
                }
            }
        }
        let compose_context = invocation.capture_launch_context(&ContextRequirements::for_document(&root.markdown));
        let layers = (!self.setters.is_empty()).then(|| {
            CallerInputLayers::from_caller_overrides(
                Some(Value::Object(self.setters.clone())),
                invocation.launch_file_resolution_context().clone(),
            )
        });
        let mut check = check_sequence_tree(SequenceCheckRequest {
            source: &root,
            compose_context,
            invocation: &invocation,
            source_context: &source_context,
            caller_layers: layers.as_ref(),
            launch_dir: self.launch_dir.as_deref(),
            session: Some(&self.observations),
        });
        // Every task composing a prompt is checked against the walk's read.
        self.loaded.extend(std::mem::take(&mut check.prompts));
        // Each loaded document is judged once, however many tasks use it,
        // and each verdict goes to every occurrence it belongs to.
        for file in &check.files {
            if *file != root.resolved_path {
                pending.extend(self.judge_keys(file));
            }
        }
        let mut task_keys: Vec<Vec<&KeyVerdict>> = check.tasks.iter().map(|_| Vec::new()).collect();
        let mut root_keys = Vec::new();
        for verdict in &pending {
            let owners = key_owners(&check.tasks, verdict);
            if owners.is_empty() && verdict.file != root.resolved_path {
                continue;
            }
            // Known before any failure is pushed, so a root failure for the
            // same key is held back too.
            if verdict.refusal {
                self.key_problem_paths.entry(verdict.file.clone()).or_default().insert(verdict.instance_path.clone());
                if let Some(key) = &verdict.key {
                    self.keyed.entry((verdict.file.clone(), key.clone())).or_default();
                }
            }
            if owners.is_empty() {
                root_keys.push(verdict);
            }
            for index in owners {
                task_keys[index].push(verdict);
            }
        }
        self.push_key_rows(root_subject, &Trail::default(), &root_keys, false);

        let mut blocker = None;
        for failure in &check.root_failures {
            if let Some(id) = self.push_failure(root_subject, failure, Trail::default(), None) {
                blocker.get_or_insert(id);
            }
        }
        // The root's supplied files are judged on their own, before the walk
        // (`sequence_supplied_files`); the same verdict is not a second row.
        let supplied_only = |owner: &&OwnerFailure| matches!(
            &owner.failure.error,
            CompositionError::ComposeFailed(darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. })
                if !problems.is_empty() && problems.iter().all(|problem| problem.caller_file.is_some())
        ) && owner.document == root.resolved_path;
        for owner in check.owner_failures.iter().filter(|owner| owner.task.is_none()).filter(|owner| !supplied_only(owner)) {
            if let Some(id) = self.push_owner_failure(root_subject, owner, Trail::default()) {
                self.owner_blocks.push((None, id, owner.document.clone()));
            }
        }
        // A prompt task whose owner is missing an input waits for it, as the
        // real run's discovery leaves its reference pending.
        self.waiting_prompts = match &check.inventory {
            Ok(inventory) => inventory
                .prompts
                .iter()
                .filter_map(|prompt| {
                    let entry = inventory.owners.get(prompt.owner)?.missing.as_ref()?;
                    let names = entry.missing.iter().map(|property| property.name.clone()).collect();
                    Some(((prompt.source_path.clone(), prompt.property.clone()), names))
                })
                .collect(),
            Err(_) => std::collections::HashMap::new(),
        };
        // A referenced sequence document's missing inputs, which a body task
        // composing it reads; its owner reports them.
        self.owner_missing = match &check.inventory {
            Ok(inventory) => inventory
                .owners
                .iter()
                .filter(|owner| owner.role != claudine::composition::InputOwnerRole::Root)
                .filter_map(|owner| owner.missing.as_ref())
                .map(|entry| (entry.source_path.clone(), entry.missing.iter().map(|property| property.name.clone()).collect()))
                .collect(),
            Err(_) => std::collections::HashMap::new(),
        };
        // The root's own missing inputs, which its body reads: a value its
        // `initialize` writes is known only when the run starts.
        self.root_absent = match &check.inventory {
            Ok(inventory) => {
                let mut initialized = HashSet::new();
                if let Some(initialize) = frontmatter.get("initialize") {
                    claudine::composition::collect_written_keys(initialize, &mut initialized);
                }
                inventory
                    .owners
                    .iter()
                    .filter(|owner| owner.role == claudine::composition::InputOwnerRole::Root)
                    .filter_map(|owner| owner.missing.as_ref())
                    .flat_map(|entry| entry.missing.iter())
                    .map(|property| {
                        let name = property.name.clone();
                        let reason = if initialized.contains(&name) {
                            DeferralReason::DependsOn { name: name.clone() }
                        } else {
                            DeferralReason::MissingInput { name: name.clone() }
                        };
                        (name, reason)
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        };
        self.runtime_written = check.runtime_written.clone();
        self.looping = check.tasks.iter()
            .filter(|task| self.in_loop(task, &check.tasks))
            .map(|task| task.site.identity.as_str().to_string())
            .collect();
        self.sequence_documents = check.tasks.iter()
            .filter_map(|task| Some((task.site.identity.as_str().to_string(), task.sequence_document.clone()?)))
            .collect();
        let mut rows = Vec::new();
        match check.list {
            RootList::Command => {
                self.not_checked(root_subject, "tasks", "the list comes from a shell command a dry run skips", None);
            }
            // The root failure already said why the list was not decided.
            RootList::Deferred => {}
            RootList::Unread => {
                let reason = if frontmatter.contains_key("sequence") {
                    "`sequence` could not be read"
                } else {
                    "the file has no `sequence`"
                };
                self.not_checked(root_subject, "tasks", reason, blocker);
            }
            RootList::Read => {
                for (task, keys) in check.tasks.iter().zip(&task_keys) {
                    self.check_owned_task(task, keys, &check, &mut rows);
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
            self.not_checked(subject, "`$schema`", "an inline container's schema judges the values at its turn", None);
        }
    }

    /// Records `failure` for `subject`: its reference finding (located at
    /// `source`) when it has one, else its projected error. A key refusal a
    /// schema-key row already reported is not reported again; the row
    /// stands for it (its id when `subject` holds it, else `None`).
    fn push_failure(
        &mut self,
        subject: SubjectId,
        failure: &CheckFailure,
        trail: Trail,
        source: Option<DiagnosticSource>,
    ) -> Option<FindingId> {
        // The same refusal is the same file and key: the parser and the
        // schema spell the enclosing property differently (`tasks[i]` in a
        // real run's root, `sequence[i]` here).
        let key = match &failure.error {
            CompositionError::RemovedGrammar { source_path, removed, .. } => Some((source_path.clone(), removed.clone())),
            CompositionError::SequenceUnknownTaskField { key, site, .. } => site.file.clone().map(|file| (file, key.clone())),
            _ => None,
        };
        if let Some(rows) = key.and_then(|key| self.keyed.get(&key)) {
            return rows.iter().find(|(held, _)| *held == subject).map(|(_, id)| *id);
        }
        if let CompositionError::SchemaValidation { source_path, problems, .. } = &failure.error
            && !problems.is_empty()
            && self.key_problem_paths.get(source_path).is_some_and(|paths| problems.iter().all(|path| paths.contains(path)))
        {
            return None;
        }
        // An expression a dry run could not evaluate without an effect is
        // not a defect: what depends on it is not checked.
        // A reference to a file the run creates before this point (a root
        // `initialize` `ensure_file`) is not missing either.
        if let Some(reason) = failure.error.deferral_reason().or(failure.pending.as_ref()) {
            let check = if subject == self.root_subject() { "tasks" } else { "task" };
            self.not_checked(subject, check, &reason.describe(), None);
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
    pub(super) fn locate_in(&self, file: &Path, property: &str, key: Option<&str>, value: bool) -> DiagnosticSource {
        let mut path = property_segments(property);
        if let Some(key) = key {
            path.push(YamlPathSegment::Key(key.to_string()));
        }
        let is_root = self.root.as_ref().is_some_and(|root| root.resolved_path == file);
        if is_root {
            return self.locate(&path, value);
        }
        let located = composition::read_local_text(file).ok().and_then(|text| locate_authored(file, &text, &path, value));
        match located {
            Some((line, column)) => DiagnosticSource::at(Some(file.to_path_buf()), line, column),
            None => DiagnosticSource::file_only(Some(file.to_path_buf())),
        }
    }

    /// The findings of one task the walk reached.
    /// [`Self::check_task`], knowing which failed sequence document, if any,
    /// owns the task: the real run stops at that document's own verdict, so
    /// what reads its values (inputs, preview) is not checked.
    fn check_owned_task(&mut self, task: &CheckedTask, keys: &[&KeyVerdict], check: &SequenceCheck, rows: &mut Vec<MissingRow>) {
        let identity = task.site.identity.as_str();
        self.owner_blocked = self.owner_blocks.iter()
            .find(|(owner, _, _)| owner.as_deref().is_none_or(|owner| identity.starts_with(&format!("{owner}.sequence["))))
            .map(|(_, id, document)| (*id, document.clone()));
        self.check_task(task, keys, rows);
        self.owner_blocked = None;
        // The task's own document failed: its subject exists now.
        for owner in check.owner_failures.iter().filter(|owner| owner.task.as_ref().is_some_and(|own| own.as_str() == identity)) {
            let Some(&subject) = self.task_subjects.get(identity) else { continue };
            let trail = site_trail(&task.site);
            if let Some(id) = self.push_owner_failure(subject, owner, trail) {
                self.owner_blocks.push((Some(identity.to_string()), id, owner.document.clone()));
            }
        }
    }

    /// A sequence document's own preparation failure, located at the first
    /// value its `$schema` refused when it names one.
    fn push_owner_failure(&mut self, subject: SubjectId, owner: &OwnerFailure, trail: Trail) -> Option<FindingId> {
        let CompositionError::SchemaValidation { source_path, problems, .. } = &owner.failure.error else {
            return self.push_failure(subject, &owner.failure, trail, None);
        };
        let Some(first) = problems.first().filter(|pointer| pointer.starts_with('/')) else {
            return self.push_failure(subject, &owner.failure, trail, None);
        };
        let path: Vec<YamlPathSegment> = pointer_segments(first).into_iter().map(YamlPathSegment::Key).collect();
        let source = match composition::read_local_text(source_path).ok().and_then(|text| locate_authored(source_path, &text, &path, true)) {
            Some((line, column)) => DiagnosticSource::at(Some(source_path.clone()), line, column),
            None => DiagnosticSource::file_only(Some(source_path.clone())),
        };
        let code = claudine::diagnostics::as_diagnostic(&owner.failure.error).map_or(INTERNAL_BUG, |known| known.code());
        let hint = owner.failure.error.sequence_hint();
        let diagnostic = Diagnostic::projected(code, Severity::Error, owner.failure.error.to_string(), hint).with_source(source);
        Some(self.sink.push(subject, if trail.is_empty() { diagnostic } else { diagnostic.with_trail(trail) }))
    }

    fn check_task(&mut self, task: &CheckedTask, keys: &[&KeyVerdict], rows: &mut Vec<MissingRow>) {
        let site = &task.site;
        // A data row selected by another file is located where it was
        // written; it resolves from the file that selected it.
        let (authored_file, authored_property) = site.authored();
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
        self.subject_trails.insert(subject, trail.clone());
        // A verdict on an object this task's file authors names its key in
        // the trail; one on a file the task's reference read names the task.
        let (inside, read): (Vec<&KeyVerdict>, Vec<&KeyVerdict>) =
            keys.iter().partition(|verdict| verdict.file == site.file);
        self.push_key_rows(subject, &trail, &inside, true);
        self.push_key_rows(subject, &trail, &read, false);
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
                    .with_source(self.locate_in(authored_file, authored_property, Some(key), false))
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
                .with_source(self.locate_in(authored_file, authored_property, None, true))
                .with_trail(trail.clone()),
            );
            self.not_checked(subject, "provider", "more than one executable", None);
            self.not_checked(subject, "inputs", "more than one executable", None);
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
                    task.executable.map(|field| self.locate_in(authored_file, authored_property, Some(field.key()), true))
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
                if let Some(id) = self.push_failure(subject, failure, failure_trail, source) {
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
                    self.not_checked(subject, "provider", reason, blocker);
                    self.not_checked(subject, "inputs", reason, blocker);
                }
                return;
            }
        }

        let step_agent = step.get("agent").map(scalar_text);
        match (task.executable, &task.prompt) {
            (None, _) => self.check_body_task(task, subject, trail, step, step_agent, rows),
            (_, Some(prompt)) => {
                let Some(document) = self.load_document(subject, prompt, &trail) else { return };
                // An input its sequence is missing reaches the task as that
                // run's answer: what reads it, or a `with:` value derived
                // from it, stays as written; the rest is checked.
                let absent: Vec<_> = self
                    .waiting_prompts
                    .get(&(site.file.clone(), site.property.clone()))
                    .into_iter()
                    .flatten()
                    .map(|name| (name.clone(), DeferralReason::MissingInput { name: name.clone() }))
                    .collect();
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
                    written: self.written_before(task),
                    in_loop: self.looping.contains(task.site.identity.as_str()),
                    in_group: task.in_group(),
                    seq: task.seq.clone(),
                    absent,
                    // A prompt task's barrier composes its document before
                    // the run drops anything, and refuses an invalid value.
                    drops_invalid_optionals: false,
                    earlier: task.reference.clone().map_or(EarlierCheck::None, EarlierCheck::Discovery),
                };
                self.check_task_prompt(&prompt_task, rows);
            }
            (Some(ExecutableField::Shell), None) => {
                self.not_checked(subject, "command", "a dry run runs no commands", None);
            }
            (Some(ExecutableField::SideEffect), None) => {
                self.not_checked(subject, "side effect", "a dry run dispatches no side effects", None);
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
            let (authored_file, authored_property) = site.authored();
            let source = if step.contains_key("name") {
                self.locate_in(authored_file, authored_property, Some("name"), false)
            } else {
                self.locate_in(authored_file, authored_property, None, true)
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
            self.not_checked(subject, "provider", "no executable", Some(blocker));
            self.not_checked(subject, "inputs", "no executable", Some(blocker));
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
            written: self.written_before(task),
            in_loop: self.looping.contains(task.site.identity.as_str()),
            in_group: task.in_group(),
            seq: task.seq.clone(),
            // A root step composes the root as a direct composition does; a
            // nested body step runs through a prompt task's barrier.
            drops_invalid_optionals: is_root && task.site.frames.is_empty(),
            earlier: if is_root && task.site.frames.is_empty() {
                EarlierCheck::Probe { written: self.probe_written(&document) }
            } else {
                EarlierCheck::None
            },
            // The document's own inputs are its owner's, reported once.
            absent: if is_root {
                self.root_absent.clone()
            } else {
                self.owner_missing
                    .get(&document.resolved_path)
                    .into_iter()
                    .flatten()
                    .map(|name| (name.clone(), DeferralReason::MissingInput { name: name.clone() }))
                    .collect()
            },
        };
        if is_root {
            self.check_prompt_task(&prompt_task);
        } else {
            self.check_task_prompt(&prompt_task, rows);
        }
    }

    /// What `task`'s prompt reads that the run writes before the task
    /// composes: every key the graph's lifecycle may write (the graph's one
    /// rule, which also moves provider prediction to launch), the whole
    /// `group` root inside a group (bound at the group's start, from the
    /// `variables` it rereads then), and, for a sequence step, the
    /// `seq` members only its turn knows. The step's position, name, and
    /// neighbors stay known, and so do the outputs of a first step outside
    /// any loop: none yet.
    fn written_before(&self, task: &CheckedTask) -> Vec<String> {
        let mut written = self.runtime_written.clone();
        if task.in_group() {
            written.push("group".to_string());
        }
        if let Some(seq) = &task.seq {
            let first = seq.get("index").and_then(Value::as_u64) == Some(1)
                && !self.looping.contains(task.site.identity.as_str());
            written.extend(
                claudine::composition::LAUNCH_TIME_SEQ_MEMBERS
                    .iter()
                    .filter(|member| !(first && **member == "seq.outputs"))
                    .map(|member| (*member).to_string()),
            );
        }
        written
    }

    /// What Phase 1c's probe of a root step reads as known only at launch:
    /// the keys the root's `initialize` writes, which a dry run does not
    /// make, and the run token. The step's position and its empty outputs
    /// are the probe's own.
    fn probe_written(&self, root: &ResolvedCompositionSource) -> Vec<String> {
        let mut written = std::collections::HashSet::new();
        if let Some(initialize) = root.markdown.frontmatter().as_map().get("initialize") {
            claudine::composition::sequence::preflight::providers::collect_written_keys(initialize, &mut written);
        }
        written.insert("seq.run_id".to_string());
        let mut written: Vec<String> = written.into_iter().collect();
        written.sort();
        written
    }

    /// Whether `task` runs inside a looping container: the root, an inline
    /// container or group object, or a referenced sequence document that
    /// declares `loop:`. Its `loop.*` values are its pass's.
    fn in_loop(&self, task: &CheckedTask, tasks: &[CheckedTask]) -> bool {
        let loops = |value: &Value| value.get("loop").is_some_and(|value| !value.is_null());
        self.root.as_ref().is_some_and(|root| root.markdown.frontmatter().as_map().get("loop").is_some_and(|value| !value.is_null()))
            || task.site.frames.iter().any(|frame| {
                tasks.iter().find(|enclosing| enclosing.site.identity == frame.identity).is_some_and(|enclosing| {
                    loops(&enclosing.item)
                        || enclosing.item.get("group").is_some_and(loops)
                        || enclosing.sequence_document.as_ref().is_some_and(|(_, frontmatter)| {
                            frontmatter.get("loop").is_some_and(|value| !value.is_null())
                        })
                })
            })
    }

    /// The provider, model, inputs, and preview checks of a task's prompt.
    /// A missing input is its own finding; the preview keeps what reads it
    /// as written and composes the rest.
    pub(super) fn check_task_prompt(&mut self, task: &PromptTask<'_>, rows: &mut Vec<MissingRow>) {
        self.check_selection(task);
        if let Some((blocker, document)) = self.owner_blocked.clone() {
            let reason = self.blocked_reason(blocker, &document);
            self.not_checked(task.subject, "inputs and preview", &reason, Some(blocker));
            return;
        }
        let before = rows.len();
        self.prompt_inputs(task, rows);
        let missing: Vec<_> = rows[before..]
            .iter()
            .map(|row| (row.property.name.clone(), DeferralReason::MissingInput { name: row.property.name.clone() }))
            .collect();
        self.preview(task, &missing);
    }

    /// The document at `path`: the walk's read of a prompt, else this
    /// check's one read of the file, recording why it could not be read at
    /// each occurrence that needs it.
    fn load_document(&mut self, subject: SubjectId, path: &Path, trail: &Trail) -> Option<ResolvedCompositionSource> {
        if let Some(document) = self.loaded.get(path) {
            return Some(document.clone());
        }
        let invocation = self.invocation.clone()?;
        let raw = path.to_string_lossy().into_owned();
        match composition::resolve_composition_source_in_context(&raw, invocation.launch_file_resolution_context()) {
            Ok(document) => {
                self.loaded.insert(path.to_path_buf(), document.clone());
                Some(document)
            }
            Err(error) => {
                self.push_error(subject, &error, trail.clone());
                None
            }
        }
    }
}
