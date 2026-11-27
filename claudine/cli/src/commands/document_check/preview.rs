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
        let Some((session, prepared)) = self.prepare_preview(task, missing, false) else { return };
        let deferrals = session.deferrals();
        self.deferral_rows(task, &deferrals, &[]);
        match prepared {
            Ok(prepared) => {
                // What the real run would print as a warning (a tolerated
                // transclusion failure, for one) is a warning here too.
                // Skipped triggers are reported once per invocation elsewhere.
                // A child's failure the preparation tolerated past an
                // inclusion the walk refuses is that refusal, which stops a
                // real run before composition could warn.
                for warning in prepared.warnings.iter().filter(|warning| warning.invalid_trigger_source().is_none()) {
                    let mut diagnostic = warning_diagnostic(warning).with_trail(task.trail.clone());
                    if refused_by_walk(&diagnostic, &walked) {
                        continue;
                    }
                    if diagnostic.source.is_none() {
                        diagnostic = diagnostic.with_source(DiagnosticSource::file_only(Some(task.prompt.resolved_path.clone())));
                    }
                    self.sink.push(task.subject, diagnostic);
                }
                self.previews.push(marked(&prepared.prompt, &deferrals));
            }
            Err(CompositionError::ComposedBodyEmpty { cause, .. }) => {
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
            Err(error) => {
                if !reported_by_walk(&error, &walked) {
                    self.push_error_in(task.subject, &error, task.trail.clone(), Some(&task.prompt.resolved_path));
                }
                // A missing transclusion target is its own finding; the
                // content beside it is still previewed, the inclusion left
                // out. A failure the retry meets past that inclusion is its
                // own finding too; one it meets again is held once.
                match self.prepare_preview(task, missing, true) {
                    Some((retried, Ok(prepared))) => {
                        let retried = retried.deferrals();
                        self.deferral_rows(task, &retried, &deferrals);
                        self.previews.push(marked(&prepared.prompt, &retried));
                    }
                    Some((_, Err(CompositionError::ComposedBodyEmpty { .. }))) | None => {}
                    Some((_, Err(retried))) => {
                        if !reported_by_walk(&retried, &walked) {
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

    /// One finding for each inclusion of `task`'s prompt that does not
    /// resolve ([`composition::unresolved_inclusions`]), which it returns.
    /// A walk that cannot run leaves the preparation to report why.
    fn reference_rows(&mut self, task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) -> Vec<Diagnostic> {
        let Some(options) = self.passive_prepare_options(task, preview_session(task, missing)) else { return Vec::new() };
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

    /// One passive preparation of `task`'s prompt: a new session declaring
    /// its absent values, and the result. `None` once an error is recorded.
    fn prepare_preview(
        &mut self,
        task: &PromptTask<'_>,
        missing: &[(String, DeferralReason)],
        allow_missing_transclusions: bool,
    ) -> Option<(PassiveSession, Result<composition::PreparedComposition, CompositionError>)> {
        let session = preview_session(task, missing);
        let mut options = self.passive_prepare_options(task, session.clone())?;
        options.allow_missing_transclusions = allow_missing_transclusions;
        let prepared = if self.composes_inline(task) {
            composition::prepare_inline(task.prompt, options)
        } else {
            composition::prepare_direct(task.prompt, options)
        };
        Some((session, prepared))
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
        options.prepared_context =
            Some(epoch.capture_launch_context(&ContextRequirements::for_document(&task.prompt.markdown)));
        options.invocation_context = Some(invocation);
        options.document_epoch = Some(epoch);
        options.passive = Some(session);
        Some(options)
    }
}

/// A passive session declaring `task`'s absent values and `missing`.
fn preview_session(task: &PromptTask<'_>, missing: &[(String, DeferralReason)]) -> PassiveSession {
    let session = PassiveSession::new();
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
/// refused: the same code at the same authored line of the same file.
fn refused_by_walk(record: &Diagnostic, walked: &[Diagnostic]) -> bool {
    let at = |diagnostic: &Diagnostic| {
        diagnostic.source.as_ref().and_then(|source| Some((source.file.clone()?, source.line?)))
    };
    record.code.starts_with("dm.reference.")
        && at(record).is_some()
        && walked.iter().any(|row| row.severity == Severity::Error && row.code == record.code && at(row) == at(record))
}

/// The `not checked` row's check for what `deferral` left as written.
fn check_name(deferral: &Deferral) -> &'static str {
    match deferral.construct {
        DeferredConstruct::FrontmatterValue => "frontmatter value",
        DeferredConstruct::ShellDirective => "`::shell` directive",
        DeferredConstruct::ShellBlock => "`::shell-block`",
        DeferredConstruct::BlockCondition => "`::block` condition",
        DeferredConstruct::SchemaTemplate => "schema",
        DeferredConstruct::Transclusion => "transclusion",
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
            DeferredConstruct::ShellDirective | DeferredConstruct::ShellBlock | DeferredConstruct::Transclusion => {
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
