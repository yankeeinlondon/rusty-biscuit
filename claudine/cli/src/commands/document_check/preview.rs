//! The passive preview of one prompt task.
//!
//! The prompt is prepared through the canonical preparer
//! ([`composition::prepare_direct`] or [`composition::prepare_inline`]) with a
//! passive session, so the preview is what the real command would compose,
//! except that work needing an effect is left as written. Each thing left
//! as written is its own `not checked` row, and the human preview marks it.
//! A warning the preparation reports, which a real run would print, is a
//! warning row.
//!
//! The prompt's inclusions are checked apart from the preparation, by the
//! condition-blind walk a real run's pre-flight makes
//! ([`composition::unresolved_inclusions`]), so a broken `::file`, `::code`,
//! `prologue:`, or `epilogue:` is reported even when the preparation fails
//! first (an expression such as `{{ 3 / 0 }}`) or a false `::block`
//! condition leaves it out. The preparation's own failure on the same
//! reference is not a second row. A preparation failure that names no file
//! names the prompt's, so the input check's located copy of the same
//! failure (an inline `prompt:` read as a frontmatter value) is held once.

use claudine::composition::CallerInputLayers;
use darkmatter::markdown::compose::passive::{Deferral, DeferralReason, DeferredConstruct, PassiveSession};
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, reference_issue_diagnostic, warning_diagnostic};
use darkmatter::markdown::reference::validate::ReferenceIssue;

use super::*;

/// Appended to a construct the preview shows as written.
const DEFERRED_MARK: &str = "[not evaluated in a dry run]";
/// Appended instead to a construct that reads a missing input.
const MISSING_MARK: &str = "[missing input]";

impl Check {
    /// Whether `task`'s prompt composes inline: always for
    /// `inline-compose`, never for `compose`, and for a sequence step when
    /// its document has a string `prompt:`, as the real run decides for its
    /// own document and its prompt files.
    pub(super) fn composes_inline(&self, task: &PromptTask<'_>) -> bool {
        match self.mode {
            Mode::InlineCompose => true,
            Mode::Compose => false,
            Mode::Sequence => {
                matches!(task.prompt.markdown.frontmatter().as_map().get("prompt"), Some(Value::String(_)))
            }
        }
    }

    /// Prepares the task's prompt passively and keeps its preview, or records
    /// why it could not be previewed. `missing` names inputs with no value,
    /// beside the task's own [`PromptTask::absent`]: they are findings of
    /// their own, so what reads one stays as written and marked, and the
    /// rest of the prompt composes.
    pub(super) fn preview(&mut self, task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) {
        if self.purpose == Purpose::Advisory {
            return;
        }
        // The condition-blind reference walk stands on its own: a
        // preparation that fails before it reaches an inclusion (or never
        // reaches one a false condition leaves out) does not hide it.
        let walked = self.reference_rows(task, missing);
        // A schema the run's initializer writes first leaves the
        // composition it judges unchecked too.
        if let Some(reason) = self.schema_created_later(task) {
            self.not_checked(task.subject, "preview", &reason.describe(), None);
            return;
        }
        let Some((session, prepared)) =
            self.prepare_preview(task, preview_session(self.session(), task, missing), false)
        else {
            return;
        };
        let deferrals = session.deferrals();
        self.deferral_rows(task, &deferrals, &[]);
        // A pass the run makes before any task starts reads a value the
        // preview left for launch as it is then; what that pass refuses
        // stops the run there. A preview that fails on its own already
        // fails what that pass composes.
        let stopped_before = prepared.is_ok() && self.earlier_rows(task, missing, &deferrals, &walked);
        match prepared {
            Ok(prepared) => {
                // A run its approval discovery stops never composes, so it
                // prints none of the composition's warnings.
                if !stopped_before {
                    self.warning_rows(task, &prepared, &walked);
                }
                self.previews.push(marked(&prepared.prompt, &deferrals));
            }
            Err(CompositionError::ComposedBodyEmpty { cause, .. }) => self.push_empty_body(task, &cause),
            Err(error) => {
                if !reported_by_walk(&error, &walked) && !self.schema_reported_for(task, &error) {
                    self.push_error_in(task.subject, &error, task.trail.clone(), Some(&task.prompt.resolved_path));
                }
                // A missing transclusion target is its own finding; the
                // content beside it is still previewed, the inclusion left
                // out. A failure the retry meets past that inclusion is its
                // own finding too; one it meets again is held once.
                match self.prepare_preview(task, preview_session(self.session(), task, missing), true) {
                    Some((retried, Ok(prepared))) => {
                        let retried = retried.deferrals();
                        self.deferral_rows(task, &retried, &deferrals);
                        self.warning_rows(task, &prepared, &walked);
                        self.previews.push(marked(&prepared.prompt, &retried));
                    }
                    Some((_, Err(CompositionError::ComposedBodyEmpty { .. }))) | None => {}
                    Some((_, Err(retried))) => {
                        if !reported_by_walk(&retried, &walked) && !self.schema_reported_for(task, &retried) {
                            self.push_error_in(
                                task.subject,
                                &retried,
                                task.trail.clone(),
                                Some(&task.prompt.resolved_path),
                            );
                        }
                    }
                }
            }
        }
    }

    /// The finding of `task`'s prompt composing to nothing.
    fn push_empty_body(&mut self, task: &PromptTask<'_>, cause: &darkmatter::findings::EmptyBodyCause) {
        self.sink.push(
            task.subject,
            Diagnostic::actionable(
                EMPTY_BODY_CODE,
                Severity::Error,
                format!("the prompt for this task is empty: {}", cause.phrase()),
                cause.hint(),
            )
            .with_source(DiagnosticSource::file_only(Some(task.prompt.resolved_path.clone())))
            .with_trail(task.trail.clone()),
        );
    }

    /// Whether `error` is the failure to prepare `task`'s `$schema` that
    /// the input check already reported for it.
    fn schema_reported_for(&self, task: &PromptTask<'_>, error: &CompositionError) -> bool {
        matches!(
            error.presented(),
            CompositionError::ComposeFailed(darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. })
                if problems.is_empty()
        ) && self.schema_reported.contains(&(task.subject, task.prompt.resolved_path.clone()))
    }

    /// A warning row for each warning of `prepared` (a tolerated
    /// transclusion failure, for one) that a real run would print, whichever
    /// preparation, first or retried, produced it. Skipped triggers are
    /// reported once per invocation elsewhere. A child's failure the
    /// preparation tolerated past an inclusion the walk refuses is that
    /// refusal, which stops a real run before composition could warn.
    fn warning_rows(&mut self, task: &PromptTask<'_>, prepared: &composition::PreparedComposition, walked: &[Diagnostic]) {
        for warning in prepared.warnings.iter().filter(|warning| warning.invalid_trigger_source().is_none()) {
            // The repository's guardrails file is one subject for every
            // inline prompt that reads it: one row under the root, at it.
            if warning.stage == composition::RETIRED_GUARDRAILS_STAGE
                && let Some(path) = &warning.path
            {
                let subject = self.root_subject();
                let diagnostic = warning_diagnostic(warning).with_source(DiagnosticSource::file_only(Some(path.clone())));
                self.sink.push(subject, diagnostic);
                continue;
            }
            let mut diagnostic = warning_diagnostic(warning).with_trail(task.trail.clone());
            if diagnostic.source.is_none() {
                diagnostic = diagnostic.with_source(DiagnosticSource::file_only(Some(task.prompt.resolved_path.clone())));
            }
            if refused_by_walk(&diagnostic, walked) {
                continue;
            }
            self.sink.push(task.subject, diagnostic);
        }
    }

    /// One finding for each inclusion of `task`'s prompt that does not
    /// resolve ([`composition::unresolved_inclusions`]), which it returns.
    /// A walk that cannot run leaves the preparation to report why.
    fn reference_rows(&mut self, task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) -> Vec<Diagnostic> {
        // A real run's pre-flight walk resolves every inclusion before any
        // task runs, so a value the run writes later is read as it is now,
        // and no step's `seq` position, loop pass, or group is bound yet:
        // the walk reads `seq` with no position, as no prompt has run.
        let Some(mut options) = self.passive_prepare_options(task, preflight_session(self.session(), task, missing)) else { return Vec::new() };
        options.sequence_binding = None;
        options.loop_values = None;
        options.lifecycle_group = None;
        let mode = if self.composes_inline(task) {
            composition::CompositionMode::InlineFrontmatterPrompt
        } else {
            composition::CompositionMode::ChainedDocument
        };
        let Ok(issues) = composition::unresolved_inclusions(task.prompt, &options, mode) else { return Vec::new() };
        let rows: Vec<Diagnostic> = issues.iter().map(reference_finding).collect();
        for row in &rows {
            self.sink.push(task.subject, row.clone().with_trail(task.trail.clone()));
        }
        rows
    }

    /// The refusal of the pass a real run makes of `task`'s prompt before
    /// any task starts ([`EarlierCheck`]), when the preview's `deferrals`
    /// left a value for launch that the pass reads as it is then, or, for a
    /// prompt's discovery, when the prompt reads a host root that pass
    /// leaves unbound. The walk's own refusals (`walked`) are not a second
    /// row. Whether the approval discovery of a `compose`,
    /// `inline-compose`, or root step refused, which stops the run before
    /// it composes.
    fn earlier_rows(
        &mut self,
        task: &PromptTask<'_>,
        missing: &[(String, DeferralReason)],
        deferrals: &[Deferral],
        walked: &[Diagnostic],
    ) -> bool {
        // The approval discovery a real `compose`, `inline-compose`, or root
        // step makes before it composes refuses a command or directive
        // whose shape depends on a value discovery does not evaluate (a
        // shell probe, `as_markdown`, a pending `$(…)`), whatever that value
        // is, so it is made whatever the preview deferred.
        if matches!(task.earlier, EarlierCheck::Approval | EarlierCheck::Probe { .. }) {
            let refusal = self
                .passive_prepare_options(task, preflight_session(self.session(), task, missing))
                .and_then(|options| composition::discover_document_shell(task.prompt, &options, self.mode_of(task)).err());
            if let Some(error) = refusal {
                self.push_refusal(task, error, walked);
                return true;
            }
        }
        let known_then: &[String] = match &task.earlier {
            EarlierCheck::None | EarlierCheck::Approval => return false,
            EarlierCheck::Discovery(_) => &[],
            EarlierCheck::Probe { written } => written,
        };
        // `loop` is the pass's own inside a looping root; only what the run
        // writes before the task is read differently.
        let reads_later_state = deferrals.iter().any(|deferral| {
            matches!(&deferral.reason, DeferralReason::WrittenAtLaunch { name }
                if !known_then.contains(name) && !(name == "loop" && matches!(task.earlier, EarlierCheck::Probe { .. })))
        });
        // Discovery binds no `seq`, `loop`, or `group` at all, so even a
        // member the preview reads statically (`seq.index`) reads `null`
        // there: a prompt reading one is walked whatever the preview deferred.
        let unbound_there =
            matches!(task.earlier, EarlierCheck::Discovery(_)) && composition::reads_sequence_late_root(&task.prompt.markdown);
        if !reads_later_state && !unbound_there {
            return false;
        }
        let refusal = match &task.earlier {
            EarlierCheck::None | EarlierCheck::Approval => return false,
            EarlierCheck::Discovery(reference) => {
                let Some(invocation) = self.invocation.clone() else { return false };
                let session = preflight_session(self.session(), task, missing);
                let epoch = invocation.begin_document_epoch();
                let graph_context = invocation.capture_launch_context(&ContextRequirements::for_document(&task.prompt.markdown));
                let mode = self.mode_of(task);
                match crate::commands::wrap::sequence::approval_compose_request(
                    &invocation,
                    &graph_context,
                    &epoch,
                    Some(invocation.launch_cwd()),
                    reference,
                    &task.prompt.markdown,
                ) {
                    Ok(request) => composition::discover_prompt_document(
                        &task.prompt.markdown,
                        &task.prompt.resolved_path,
                        mode,
                        &reference.with,
                        &request.map_options(|options| options.with_passive_session(session)),
                    )
                    .err(),
                    Err(error) => Some(error),
                }
            }
            EarlierCheck::Probe { written } => {
                let session = preflight_session(self.session(), task, missing);
                for name in written {
                    session.defer_written(name, DeferralReason::WrittenAtLaunch { name: name.clone() });
                }
                if task.in_loop {
                    session.defer_written("loop", DeferralReason::WrittenAtLaunch { name: "loop".to_string() });
                }
                let Some((_, prepared)) = self.prepare_preview(task, session, false) else { return false };
                prepared.err()
            }
        };
        if let Some(error) = refusal {
            self.push_refusal(task, error, walked);
        }
        false
    }

    /// The row of `error`, a refusal of a pass the run makes before `task`
    /// composes, unless the walk or the input check already reported it.
    fn push_refusal(&mut self, task: &PromptTask<'_>, error: CompositionError, walked: &[Diagnostic]) {
        match error {
            CompositionError::ComposedBodyEmpty { cause, .. } => self.push_empty_body(task, &cause),
            error if !reported_by_walk(&error, walked) && !self.schema_reported_for(task, &error) => {
                self.push_error_in(task.subject, &error, task.trail.clone(), Some(&task.prompt.resolved_path));
            }
            _ => {}
        }
    }

    /// The mode `task`'s prompt composes in.
    fn mode_of(&self, task: &PromptTask<'_>) -> composition::CompositionMode {
        if self.composes_inline(task) {
            composition::CompositionMode::InlineFrontmatterPrompt
        } else {
            composition::CompositionMode::ChainedDocument
        }
    }

    /// One passive preparation of `task`'s prompt recording into `session`,
    /// and the result. `None` once an error is recorded.
    fn prepare_preview(
        &mut self,
        task: &PromptTask<'_>,
        session: PassiveSession,
        allow_missing_transclusions: bool,
    ) -> Option<(PassiveSession, Result<composition::PreparedComposition, CompositionError>)> {
        let mut options = self.passive_prepare_options(task, session.clone())?;
        options.allow_missing_transclusions = allow_missing_transclusions;
        let inline = self.composes_inline(task);
        if !task.drops_invalid_optionals {
            let prepared =
                if inline { composition::prepare_inline(task.prompt, options) } else { composition::prepare_direct(task.prompt, options) };
            return Some((session, prepared));
        }
        // The real route's two drops, in its order: the pre-launch check's
        // of a literal value, then the composition's of what it still refuses.
        let invocation = self.invocation.clone()?;
        let (scrubbed, _, mut dropped) = composition::drop_invalid_optionals(
            task.prompt.clone(),
            options.set_overrides.clone(),
            options.file_ref_fallback_dir.as_deref(),
            &options.file_resolution_context,
            invocation.trigger_findings(),
        );
        // A caller's value stays for the composition to drop, which keeps the
        // caller records it was supplied with.
        dropped.retain(|drop| drop.source != composition::DroppedOptionalSource::Override);
        let mode = if inline {
            composition::CompositionMode::InlineFrontmatterPrompt
        } else {
            composition::CompositionMode::ChainedDocument
        };
        let (prepared, composed) = composition::prepare_passive_with_schema(&scrubbed, options, mode);
        dropped.extend(composed);
        self.dropped_rows(task, &dropped);
        Some((session, prepared))
    }

    /// A `composition.optional_dropped` warning for each of `dropped`, the
    /// warning the real run prints for it, once per document and property.
    pub(super) fn dropped_rows(&mut self, task: &PromptTask<'_>, dropped: &[composition::DroppedOptional]) {
        if self.purpose != Purpose::DryRun {
            return;
        }
        for drop in dropped {
            if !self.dropped_reported.insert((task.prompt.resolved_path.clone(), drop.property.clone())) {
                continue;
            }
            let row = dropped_row(self, &task.prompt.resolved_path, drop).with_trail(task.trail.clone());
            self.sink.push(task.subject, row);
        }
    }

    /// A `not checked` row for each of `deferrals` not in `reported`. A read
    /// of a missing input is that input's own finding, so the preview only
    /// marks it; an inclusion whose target reads one is still a reference
    /// the check could not make.
    fn deferral_rows(&mut self, task: &PromptTask<'_>, deferrals: &[Deferral], reported: &[Deferral]) {
        for deferral in deferrals {
            let marked_only = matches!(deferral.reason, DeferralReason::MissingInput { .. })
                && deferral.construct != DeferredConstruct::Transclusion;
            if marked_only || reported.contains(deferral) {
                continue;
            }
            self.not_checked(task.subject, check_name(deferral), &deferral.reason.describe(), None);
        }
    }

    /// The canonical preparation options of `task`'s prompt as a dry run
    /// recording into `session`: the task's evaluated `with:` values as data
    /// beneath the caller's setters, as the task's turn layers them. `None`
    /// once an error is recorded.
    pub(super) fn passive_prepare_options(
        &mut self,
        task: &PromptTask<'_>,
        session: PassiveSession,
    ) -> Option<composition::PrepareOptions> {
        let invocation = self.invocation.clone()?;
        let context = match invocation.derive_composition_source(task.prompt) {
            Ok(source) => source.file_resolution_context().clone(),
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
                return None;
            }
        };
        let mut overrides = task.with_evaluated.clone();
        let data_keys: std::collections::BTreeSet<String> =
            overrides.keys().filter(|key| !self.setters.contains_key(*key)).cloned().collect();
        overrides.extend(self.setters.clone());
        let layers = CallerInputLayers::from_caller_overrides(
            Some(Value::Object(overrides)),
            invocation.launch_file_resolution_context().clone(),
        );
        let epoch = invocation.begin_document_epoch();
        // The source repository anchors the document's own references, as
        // `compose/prep.rs` sets it for a real run.
        let source_repo_root = context.repository_root().map(std::path::Path::to_path_buf);
        // `apply_to` installs the layers' (launch) context; the document's
        // own references resolve from its source context, as a real run's
        // preparation resolves them, while the caller's setters keep their
        // launch-anchored records.
        let mut options = layers.apply_to(composition::PrepareOptions::new(context.clone()));
        options.file_resolution_context = context;
        options.source_repo_root = source_repo_root;
        options.file_ref_fallback_dir = Some(invocation.launch_cwd().to_path_buf());
        options.data_override_keys.extend(data_keys);
        options.sequence_binding = task.seq.clone();
        // A looping container's pass binds `loop`; its values are declared
        // written at launch (`preview_session`), so this placeholder is never
        // read.
        if task.in_loop {
            options.loop_values = Some(claudine::composition::LoopAmbient::new(1, true, false, Value::Null, 0));
        }
        // A group leg reads its group's `group`, which is in scope; it is
        // declared written at launch (`PromptTask::written`), so this empty
        // placeholder is never read. Outside a group it stays the scope
        // error the real run reports.
        if task.in_group {
            options.lifecycle_group = Some(Map::new());
        }
        options.prepared_context =
            Some(epoch.capture_launch_context(&ContextRequirements::for_document(&task.prompt.markdown)));
        options.invocation_context = Some(invocation);
        options.document_epoch = Some(epoch);
        options.passive = Some(session);
        Some(options)
    }
}

/// The warning row of `drop` in `document`, at the value the document wrote,
/// or naming the document alone for a caller's or a composed value.
pub(super) fn dropped_row(check: &Check, document: &Path, drop: &composition::DroppedOptional) -> Diagnostic {
    let source = if drop.source == composition::DroppedOptionalSource::Frontmatter {
        check.locate_in(document, "", Some(&drop.property), true)
    } else {
        DiagnosticSource::file_only(Some(document.to_path_buf()))
    };
    Diagnostic::actionable(
        OPTIONAL_DROPPED,
        Severity::Warning,
        drop.message(),
        format!("give `{}` a value its schema accepts, or remove it", drop.property),
    )
    .with_source(source)
}

/// `session` declaring `task`'s absent values and `missing`, and each value
/// the run writes before the task composes as known only at launch
/// ([`PromptTask::written`], and `loop` inside a loop).
fn preview_session(session: PassiveSession, task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) -> PassiveSession {
    let session = preflight_session(session, task, missing);
    for name in &task.written {
        session.defer_written(name, DeferralReason::WrittenAtLaunch { name: name.clone() });
    }
    // Each pass of a loop, the enclosing container's or the document's own,
    // reads its own `loop` values.
    if task.in_loop || task.prompt.markdown.frontmatter().as_map().get("loop").is_some_and(|value| !value.is_null()) {
        session.defer_written("loop", DeferralReason::WrittenAtLaunch { name: "loop".to_string() });
    }
    session
}

/// `session` declaring only `task`'s absent values and `missing`: the
/// values as they are before the run starts.
fn preflight_session(session: PassiveSession, task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) -> PassiveSession {
    for (name, reason) in task.absent.iter().chain(missing) {
        session.defer_absent(name, reason.clone());
    }
    // A `with:` value known only at the task's turn has none here.
    for key in &task.with_late {
        session.defer_absent(key, DeferralReason::DependsOn { name: key.clone() });
    }
    session
}

/// `issue` as a finding, worded as `md compose`'s reference rows are
/// (`file not found in `::file``).
fn reference_finding(issue: &ReferenceIssue) -> Diagnostic {
    let mut finding = reference_issue_diagnostic(issue);
    if let Some(details) = &finding.reference_details
        && let (Some(reason), Some(construct)) = (&details.reason, &details.construct)
    {
        finding.message = darkmatter::findings::reference::reference_message(reason, construct);
    }
    finding
}

/// Whether every record of the preparation's `error` is a reference failure
/// the walk already reported: the same code, the same reference resolved to
/// the same target, at the same authored line where both know one. The two
/// projections differ in the span they carry (and an inline prompt's
/// failure in whether it names the file), so the sink's own equivalence
/// would keep both.
fn reported_by_walk(error: &CompositionError, walked: &[Diagnostic]) -> bool {
    let cause = |diagnostic: &Diagnostic| {
        diagnostic
            .reference_details
            .as_ref()
            .map(|details| (details.reference.clone(), details.resolved.clone(), details.construct.clone()))
    };
    let line = |diagnostic: &Diagnostic| {
        diagnostic.source.as_ref().and_then(|source| source.line.map(|line| (source.file.clone(), line, source.column)))
    };
    let records = error_diagnostics(error.presented());
    !records.is_empty()
        && records.iter().all(|record| {
            record.code.starts_with("dm.reference.")
                && walked.iter().any(|row| {
                    row.code == record.code
                        && cause(row) == cause(record)
                        && match (line(row), line(record)) {
                            (Some(row), Some(record)) => row == record,
                            _ => true,
                        }
                })
        })
}

/// Whether the preparation's warning `record` is at a reference the walk
/// refused: the same code in the same file, at the same authored line where
/// both know one. A generated inline prompt places neither its refusal nor
/// the retry's warning for it on a line; one document copy tolerates all of
/// its invalid inclusions or none, so a refusal in the file is that warning's.
fn refused_by_walk(record: &Diagnostic, walked: &[Diagnostic]) -> bool {
    let file = |diagnostic: &Diagnostic| diagnostic.source.as_ref().and_then(|source| source.file.clone());
    let line = |diagnostic: &Diagnostic| diagnostic.source.as_ref().and_then(|source| source.line);
    record.code.starts_with("dm.reference.")
        && file(record).is_some()
        && walked.iter().any(|row| {
            row.severity == Severity::Error
                && row.code == record.code
                && file(row) == file(record)
                && match (line(row), line(record)) {
                    (Some(row), Some(record)) => row == record,
                    _ => true,
                }
        })
}

/// The `not checked` row's check for what `deferral` left as written.
fn check_name(deferral: &Deferral) -> &'static str {
    match deferral.construct {
        DeferredConstruct::FrontmatterValue => "frontmatter value",
        DeferredConstruct::ShellDirective => "`::shell` directive",
        DeferredConstruct::ShellBlock => "`::shell-block`",
        DeferredConstruct::BlockCondition => "`::block` condition",
        DeferredConstruct::SchemaTemplate => "schema",
        DeferredConstruct::SchemaCheck => "schema check",
        DeferredConstruct::Transclusion => "transclusion",
        DeferredConstruct::Collection => "`::map`/`::filter` collection",
        _ => "interpolation",
    }
}

/// The mark of what `deferral` left as written.
fn mark_for(deferral: &Deferral) -> &'static str {
    match deferral.reason {
        DeferralReason::MissingInput { .. } => MISSING_MARK,
        _ => DEFERRED_MARK,
    }
}

/// `prompt` with each construct a deferral left as written marked: a
/// `{{ … }}` span after the span, a directive after its line. A directive
/// left as written because its line holds such a span carries one mark, the
/// directive's.
fn marked(prompt: &str, deferrals: &[Deferral]) -> String {
    let directive_lines: Vec<(&str, &str)> = deferrals
        .iter()
        .filter_map(|deferral| match deferral.construct {
            DeferredConstruct::ShellDirective
            | DeferredConstruct::ShellBlock
            | DeferredConstruct::Transclusion
            | DeferredConstruct::Collection => {
                deferral.authored.lines().next().map(|line| (line.trim(), mark_for(deferral)))
            }
            _ => None,
        })
        .collect();
    let directive_mark = |line: &str| {
        let trimmed = line.trim();
        directive_lines.iter().find(|(directive, _)| *directive == trimmed).map(|(_, mark)| *mark)
    };
    let mut text = prompt.to_string();
    let mut spans: Vec<(&str, &str)> = deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::Interpolation)
        .map(|deferral| (deferral.authored.as_str(), mark_for(deferral)))
        .filter(|(span, _)| span.starts_with("{{"))
        .collect();
    // Longest first, so a span that contains another is marked once.
    spans.sort_by_key(|(span, _)| std::cmp::Reverse(span.len()));
    spans.dedup_by_key(|(span, _)| *span);
    for (span, mark) in spans {
        let mut out = String::with_capacity(text.len());
        let mut done = 0;
        let mut from = 0;
        while let Some(found) = text[from..].find(span) {
            let at = from + found;
            let end = at + span.len();
            let line_start = text[..at].rfind('\n').map_or(0, |newline| newline + 1);
            let line_end = text[end..].find('\n').map_or(text.len(), |newline| end + newline);
            let already = text[end..].starts_with(&format!(" {DEFERRED_MARK}")) || text[end..].starts_with(&format!(" {MISSING_MARK}"));
            if !already && directive_mark(&text[line_start..line_end]).is_none() {
                out.push_str(&text[done..end]);
                out.push(' ');
                out.push_str(mark);
                done = end;
            }
            from = end;
        }
        out.push_str(&text[done..]);
        text = out;
    }
    let conditions: Vec<(&str, &str)> = deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::BlockCondition)
        .map(|deferral| (deferral.authored.as_str(), mark_for(deferral)))
        .collect();
    if directive_lines.is_empty() && conditions.is_empty() {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (body, ending) = line.strip_suffix('\n').map_or((line, ""), |body| (body, "\n"));
        let trimmed = body.trim();
        let mark = directive_mark(body).or_else(|| {
            trimmed
                .starts_with("::block")
                .then(|| conditions.iter().find(|(condition, _)| trimmed.contains(condition)).map(|(_, mark)| *mark))
                .flatten()
        });
        out.push_str(body);
        if let Some(mark) = mark {
            out.push(' ');
            out.push_str(mark);
        }
        out.push_str(ending);
    }
    out
}

#[cfg(test)]
mod tests {
    use darkmatter::markdown::compose::passive::DeferralReason;

    use super::*;

    fn deferral(construct: DeferredConstruct, authored: &str) -> Deferral {
        Deferral {
            construct,
            reason: DeferralReason::Command { command: "git branch".into() },
            authored: authored.into(),
            key: None,
            line: None,
        }
    }

    #[test]
    fn each_construct_left_as_written_is_marked_and_nothing_else_is() {
        let prompt = "On {{ branch }} for main.\n::shell git log\n::file https://example.com/x.md\n::block when=\"branch == 'x'\"\nbody\n::end-block\nplain line\n";
        let deferrals = [
            deferral(DeferredConstruct::Interpolation, "{{ branch }}"),
            deferral(DeferredConstruct::ShellDirective, "::shell git log"),
            deferral(DeferredConstruct::Transclusion, "::file https://example.com/x.md"),
            deferral(DeferredConstruct::BlockCondition, "branch == 'x'"),
            deferral(DeferredConstruct::FrontmatterValue, "$(git branch)"),
        ];

        assert_eq!(
            marked(prompt, &deferrals),
            format!(
                "On {{{{ branch }}}} {DEFERRED_MARK} for main.\n::shell git log {DEFERRED_MARK}\n::file https://example.com/x.md {DEFERRED_MARK}\n::block when=\"branch == 'x'\" {DEFERRED_MARK}\nbody\n::end-block\nplain line\n"
            )
        );
        assert_eq!(marked(prompt, &[]), prompt);
    }

    #[test]
    fn a_collection_left_as_written_is_marked_on_its_opening_line() {
        let region = "::filter items \"i => length(seq.outputs) > 0\"\n- {{ i }}\n::end-filter\n";
        let prompt = format!("Before.\n{region}After.\n");
        let deferrals = [deferral(DeferredConstruct::Collection, region)];

        assert_eq!(
            marked(&prompt, &deferrals),
            format!(
                "Before.\n::filter items \"i => length(seq.outputs) > 0\" {DEFERRED_MARK}\n- {{{{ i }}}}\n::end-filter\nAfter.\n"
            )
        );
    }

    #[test]
    fn a_directive_whose_target_reads_a_missing_input_carries_one_mark() {
        let missing = |construct, authored: &str| Deferral {
            reason: DeferralReason::MissingInput { name: "topic".into() },
            ..deferral(construct, authored)
        };
        let prompt = "See {{ topic }}.\n::file ./{{ topic }}.md\n";
        let deferrals = [
            missing(DeferredConstruct::Interpolation, "{{ topic }}"),
            missing(DeferredConstruct::Transclusion, "::file ./{{ topic }}.md"),
        ];

        assert_eq!(
            marked(prompt, &deferrals),
            format!("See {{{{ topic }}}} {MISSING_MARK}.\n::file ./{{{{ topic }}}}.md {MISSING_MARK}\n")
        );
    }
}
