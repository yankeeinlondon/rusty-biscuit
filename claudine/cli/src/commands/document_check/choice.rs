//! File choices a real run would ask the caller to make.
//!
//! Two real routes recover a file reference that names no file by asking:
//! the root argument's picker (a bare name, `compose plan`) and the supplied
//! eager-file chooser (`spec=partial` against a `file(eager; match(…))`
//! input of `compose` and `inline-compose`). The dry run asks nothing: it
//! runs the same read-only search each route runs before showing anything,
//! and reports a pending choice (a warning) where the route would ask and
//! the search found files, and the route's own refusal (an error) where it
//! would not ask or found nothing. A sequence asks for neither a supplied
//! file nor a task's input, so those keep their refusal.

use claudine::composition::schema::supplied::unresolved_supplied_files;
use darkmatter::findings::ReferenceDetails;

use super::*;
use crate::completion::operation_file;
use crate::completion::scopes::{ComposeMode, ScopeContext};

/// How many candidates a pending-choice row lists before counting the rest.
const LISTED: usize = 5;

impl Check {
    /// Reports a root reference that names no file the way the real
    /// command recovers it, when its picker would take it: `true` once that
    /// is recorded. `missing` is the reference finding a hard miss reports.
    pub(super) fn root_choice(&mut self, file: &str, context: &FileResolutionContext, missing: &Diagnostic) -> bool {
        let loaded = match self.mode {
            Mode::Sequence => super::super::sequence::resolve_sequence_source(file, context).map(|_| ()),
            Mode::Compose | Mode::InlineCompose => {
                composition::resolve_composition_source_in_context(file, context).map(|_| ())
            }
        };
        let Err(no_match) = loaded else { return false };
        if !operation_file::attempts_autocomplete(file, &no_match) {
            return false;
        }
        let subject = self.root_subject();
        let searched = if operation_file::picker_can_ask(self.stdin_is_tty, self.stderr_is_tty) {
            operation_file::candidate_references(file, self.compose_mode(), &ScopeContext::discover())
        } else {
            Err(CompositionError::AutocompleteNotInteractive)
        };
        match searched {
            Ok(candidates) => {
                let row = pending_choice(
                    format!("`{file}` names no file, so an interactive run asks which file to use"),
                    &candidates,
                    "pass the file's path to run without the choice",
                );
                let row = match &missing.reference_details {
                    Some(details) => row.with_reference_details(ReferenceDetails { suggestions: None, ..details.clone() }),
                    None => row,
                };
                self.sink.push(subject, row);
            }
            // The run's own refusal: the miss, and why the picker cannot
            // recover it here.
            Err(refusal) => {
                self.sink.push(subject, missing.clone().with_note(format!("a real run stops here: {refusal}")));
            }
        }
        true
    }

    /// The supplied eager-file partials of a `compose` or `inline-compose`
    /// root that the real run's chooser would offer files for, each reported
    /// as a pending choice; returns their properties, which the rest of the
    /// check reads as absent. A partial the run would not ask about (no
    /// terminal, `--silent`, prompting off, no candidate) is left to the
    /// preparation, which reports the run's schema refusal.
    pub(super) fn supplied_file_choices(&mut self) -> Vec<String> {
        let Some(invocation) = self.invocation.clone() else { return Vec::new() };
        let root = self.root_ref().clone();
        let Ok(source) = invocation.derive_composition_source(&root) else { return Vec::new() };
        if !self.interactive_options().allowed() {
            return Vec::new();
        }
        let records = super::super::compose::caller_input_records(
            &self.shared,
            Some(Value::Object(self.setters.clone())),
            invocation.launch_file_resolution_context().clone(),
        );
        let mut chosen = Vec::new();
        for pending in unresolved_supplied_files(&root, &records, source.file_resolution_context()) {
            let CompositionError::UnresolvedFileReference { property, provided, alternatives, .. } = &pending.error
            else {
                continue;
            };
            let Some(record) = records.get(property) else { continue };
            let launch_dir = self.launch_dir.clone();
            let candidates: Vec<String> =
                crate::commands::schema_interactive::supplied_file_candidates(provided, alternatives, record.origin())
                    .iter()
                    .map(|path| shown(path, launch_dir.as_deref()))
                    .collect();
            if candidates.is_empty() {
                continue;
            }
            let subject = self.root_subject();
            let row = pending_choice(
                format!("`{property}={provided}` names no file, so an interactive run asks which file to use"),
                &candidates,
                &format!("pass the file's path, such as `{property}={}`, to run without the choice", candidates[0]),
            )
            .with_source(DiagnosticSource::file_only(Some(root.resolved_path.clone())));
            self.sink.push(subject, row);
            if !chosen.contains(property) {
                chosen.push(property.clone());
            }
        }
        for property in &chosen {
            self.setters.remove(property);
        }
        chosen
    }

    /// The real sequence run's refusal of each supplied eager-file value of
    /// the root that names no file: it asks for none of them.
    pub(super) fn sequence_supplied_files(&mut self) {
        let Some(invocation) = self.invocation.clone() else { return };
        let root = self.root_ref().clone();
        let Ok(source) = invocation.derive_composition_source(&root) else { return };
        let records = super::super::compose::caller_input_records(
            &self.shared,
            Some(Value::Object(self.setters.clone())),
            invocation.launch_file_resolution_context().clone(),
        );
        for pending in unresolved_supplied_files(&root, &records, source.file_resolution_context()) {
            let subject = self.root_subject();
            let code = claudine::diagnostics::as_diagnostic(&pending.error).map_or(INTERNAL_BUG, |known| known.code());
            let property = match &pending.error {
                CompositionError::UnresolvedFileReference { property, .. } => property.clone(),
                _ => continue,
            };
            self.sink.push(
                subject,
                Diagnostic::actionable(
                    code,
                    Severity::Error,
                    pending.error.to_string(),
                    format!("pass `{property}=` the file's path; a sequence does not ask which file to use"),
                )
                .with_source(DiagnosticSource::file_only(Some(root.resolved_path.clone()))),
            );
        }
    }

    /// The real run's interactive-input policy, read from the terminal
    /// state the check observed.
    pub(super) fn interactive_options(&self) -> claudine::composition::schema::InteractiveSchemaOptions {
        // As `resolve_interactive_options` does, an unreadable configuration
        // leaves the default (ask).
        claudine::composition::schema::InteractiveSchemaOptions {
            prompt_for_missing: self.config.as_ref().is_none_or(|config| config.prompt_for_missing),
            stdin_is_tty: self.stdin_is_tty,
            stderr_is_tty: self.stderr_is_tty,
            silent: self.shared.silent,
        }
    }

    fn compose_mode(&self) -> ComposeMode {
        match self.mode {
            Mode::Compose => ComposeMode::Compose,
            Mode::InlineCompose => ComposeMode::InlineCompose,
            Mode::Sequence => ComposeMode::Sequence,
        }
    }
}

/// A `composition.file_choice_pending` warning listing `candidates`.
fn pending_choice(message: String, candidates: &[String], hint: &str) -> Diagnostic {
    let count = match candidates.len() {
        1 => "1 file matches".to_string(),
        n => format!("{n} files match"),
    };
    let mut row = Diagnostic::actionable(FILE_CHOICE_PENDING, Severity::Warning, format!("{message}; {count}"), hint);
    for candidate in candidates.iter().take(LISTED) {
        row = row.with_note(candidate.clone());
    }
    if candidates.len() > LISTED {
        row = row.with_note(format!("… and {} more", candidates.len() - LISTED));
    }
    row
}
