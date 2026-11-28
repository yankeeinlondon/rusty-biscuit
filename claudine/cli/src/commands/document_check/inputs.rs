//! The missing inputs a check reports: each prompt's own, prepared
//! passively, and each owning sequence document's, graded as one cohort.

use claudine::composition::{MissingProperty, SequenceCheck};
use darkmatter::markdown::compose::passive::DeferralReason;

use super::*;
use super::tasks::{MISSING_INPUT, missing_input_severity, site_trail};

/// One missing input, graded once the whole cohort is known.
pub(super) struct MissingRow {
    pub(super) subject: SubjectId,
    pub(super) trail: Trail,
    pub(super) property: MissingProperty,
    pub(super) declared_in: PathBuf,
    pub(super) hint: String,
}

impl Check {
    /// The missing inputs of a prompt task's document, prepared passively
    /// under the caller's setters and the task's `with:`.
    pub(super) fn prompt_inputs(&mut self, task: &PromptTask<'_>, rows: &mut Vec<MissingRow>) {
        if self.purpose == Purpose::Advisory {
            return;
        }
        // A `with:` value known only at the task's turn is not judged; the
        // document's other inputs still are.
        if !task.with_late.is_empty() {
            let keys: Vec<String> = task.with_late.iter().map(|key| format!("`{key}`")).collect();
            self.not_checked(task.subject, &format!("input {}", keys.join(", ")), "a `with:` value is known only when the task runs", None);
        }
        // A value that comes from a command stays as written; an input it
        // would supply still counts as given. A value the run writes before
        // the task is read as it will be then: known only at launch.
        let session = self.session();
        for name in &task.written {
            session.defer_written(name, DeferralReason::WrittenAtLaunch { name: name.clone() });
        }
        // A schema the run's initializer writes first decides the inputs
        // only at launch; the preview waits for it too.
        if let Some(reason) = self.schema_created_later(task) {
            self.schema_reported.insert((task.subject, task.prompt.resolved_path.clone()));
            self.not_checked(task.subject, "schema", &reason.describe(), None);
            return;
        }
        let Some(options) = self.passive_prepare_options(task, session) else { return };
        // Judged by the mode the task composes in, so an inline prompt's
        // completion-only required value may stay absent, as at its launch.
        let mode = if self.composes_inline(task) {
            composition::CompositionMode::InlineFrontmatterPrompt
        } else {
            composition::CompositionMode::ChainedDocument
        };
        match composition::sequence::document::prepare_sequence_document_values(task.prompt, options, mode) {
            Ok(_) => {}
            Err(CompositionError::MissingProperties { source_path, missing, .. }) => {
                // A value the task already waits for (its sequence's own
                // missing input) is reported once, by its owner.
                let waits = |name: &str| task.absent.iter().any(|(absent, _)| absent == name) || task.with_late.iter().any(|key| key == name);
                for property in missing.into_iter().filter(|property| !waits(&property.name)) {
                    if task.written.contains(&property.name) {
                        self.not_checked(
                            task.subject,
                            &format!("input `{}`", property.name),
                            "the run writes it before this task, so it is known only at launch",
                            None,
                        );
                        continue;
                    }
                    let hint = if self.mode == Mode::Sequence {
                        format!("add `{}` to the task's `with:`, or pass `{}=<value>` after the file", property.name, property.name)
                    } else {
                        format!("pass `{}=<value>` after the file, or give it a default", property.name)
                    };
                    rows.push(MissingRow {
                        subject: task.subject,
                        trail: task.trail.clone(),
                        hint,
                        property,
                        declared_in: source_path.clone(),
                    });
                }
            }
            // The schema's aggregate verdict is the preview's to report: it
            // runs next over the same document and gives each problem its
            // own typed, located row, where this copy would be a second,
            // unlocated one.
            Err(CompositionError::SchemaValidation { .. }) => {}
            // A `$schema` that cannot be prepared is this check's to report,
            // by the class the real launch check gives it (a slot by each
            // slot's own record); the preview meets it again.
            Err(
                error @ (CompositionError::SchemaLoad { .. }
                | CompositionError::SchemaParse { .. }
                | CompositionError::SchemaSlotFailed { .. }),
            ) => {
                self.schema_reported.insert((task.subject, task.prompt.resolved_path.clone()));
                self.push_error(task.subject, &error, task.trail.clone());
            }
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
            }
        }
    }

    /// Why `task`'s `$schema` is not checked, when a file reference it names
    /// (the value, or an entry of a list of them) is a file the root's
    /// `initialize` writes before this task: one that resolves to no file
    /// and whose candidate the run creates
    /// ([`PassiveSession::created_among`]), or an existing one it appends
    /// to ([`PassiveSession::initializer_changes`]).
    pub(super) fn schema_created_later(&self, task: &PromptTask<'_>) -> Option<DeferralReason> {
        let invocation = self.invocation.as_ref()?;
        let context = invocation.derive_composition_source(task.prompt).ok()?;
        let context = context.file_resolution_context().for_source(&task.prompt.resolved_path);
        let references: Vec<&str> = match task.prompt.markdown.frontmatter().as_map().get("$schema")? {
            Value::String(reference) => vec![reference.as_str()],
            Value::Array(entries) => entries.iter().filter_map(Value::as_str).collect(),
            _ => return None,
        };
        references.into_iter().find_map(|raw| {
            let reference = FileReference::new(raw).ok()?;
            let detailed = reference.resolve_detailed(&context);
            match detailed.matched_path() {
                Some(path) => self.observations.initializer_changes(path),
                None => self.observations.created_among(detailed.probed_paths()),
            }
        })
    }

    /// The missing inputs of every owning sequence document: the root's
    /// under the root, a referenced document's under the task that
    /// references it.
    ///
    /// A real run discovers inputs after the root's `initialize` fires and
    /// reads its writes as setters for every owner, so an input that stack
    /// may write is `not checked` rather than missing.
    pub(super) fn owner_inputs(&mut self, check: &SequenceCheck, rows: &mut Vec<MissingRow>) {
        if self.purpose == Purpose::Advisory {
            return;
        }
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
                    self.not_checked(subject, &format!("input `{}`", property.name), "the root's `initialize` writes it when it runs", None);
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
    pub(super) fn push_missing_rows(&mut self, rows: Vec<MissingRow>) {
        if rows.is_empty() {
            return;
        }
        let interactive = self.interactive_options();
        let every_shape = rows.iter().all(|row| row.property.interactive_shape.is_some());
        let severity = missing_input_severity(interactive.allowed(), every_shape);
        for row in rows {
            let note = match (row.property.type_label.as_deref(), row.property.description.as_deref()) {
                (Some(label), Some(description)) => format!(" ({label} — {description})"),
                (Some(note), None) | (None, Some(note)) => format!(" ({note})"),
                (None, None) => String::new(),
            };
            let message = match severity {
                Severity::Warning => format!("`{}`{note} has no value; an interactive run asks for it", row.property.name),
                _ => format!("`{}`{note} is required and has no value; a non-interactive run fails here", row.property.name),
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
