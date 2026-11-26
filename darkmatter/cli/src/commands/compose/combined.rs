//! The combined report of an `md compose` run that a reference failure
//! stopped.
//!
//! A reference failure stops `md compose` before anything runs. Before this
//! report existed, the run said only that; a document with a broken
//! reference *and* a wrong call, a schema violation, or a body that would
//! compose to nothing needed one run per problem. After a reference stop the
//! run now evaluates the checks that do not depend on the broken reference,
//! all passively (no command, effect, or approval):
//!
//! - the call check and the schema verdict, through one dry-run composition
//!   of the document in which every unresolved inclusion is left out;
//! - the prospective empty body, from that same composition: a block a
//!   known-false condition excludes is evidence even when its own reference
//!   is broken, while a construct the dry run left as written (a condition
//!   or value that waits on a command) never proves the body empty, so the
//!   check is reported `not checked` instead.
//!
//! When those checks add nothing, the run prints the reference report it
//! always printed. When they add a finding, the run prints one
//! [`FindingReport`] for the document instead: the reference findings first,
//! then the others, then the checks that could not run, and a closing tally.
//!
//! The stops this covers are a refused reference check
//! ([`ReferenceCheck::Refused`](super::ReferenceCheck::Refused)), an
//! inclusion target the condition-blind pre-flight walk could not resolve,
//! and a reference check that the call check or the schema verdict stopped
//! before it could list the document's references (it is repeated without
//! them so the broken references are still found).

use std::path::Path;

use color_eyre::eyre::Report;
use darkmatter::findings::{
    Diagnostic, DiagnosticSource, EMPTY_BODY_CODE, EmptyBodyCause, FindingCounts, FindingId, FindingReport, FindingSink,
    Severity, Subject, SubjectId,
};
use darkmatter::markdown::compose::passive::{Deferral, DeferredConstruct, PassiveSession};
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, reference_issue_diagnostic};
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeWarning};
use darkmatter::markdown::reference::types::ReferenceKind;
use darkmatter::markdown::reference::validate::{
    ReferenceIssue, ReferenceIssueCode, ReferenceSeverity, ReferenceValidationReport,
};
use darkmatter::markdown::{Markdown, MarkdownError};

use crate::args::policy::ResolvedPolicy;
use crate::output::report::ReportSink;
use crate::output::stderr_report;

/// The checks a call refusal leaves unchecked.
const AFTER_CALL_REFUSAL: &[&str] = &["schema", "empty body"];
/// The checks a schema refusal or a failed preparation leaves unchecked.
const AFTER_PREPARATION: &[&str] = &["empty body"];

/// One document's findings after a reference stop.
pub(crate) struct CombinedReport {
    report: FindingReport,
    label: String,
}

impl CombinedReport {
    /// The findings for stderr, with the closing tally, ending in a newline.
    pub(crate) fn render(&self, launch_dir: &Path) -> String {
        let presentation =
            self.report.presentation().with_launch_dir(launch_dir).with_summary(tally(&self.label, self.report.counts()));
        let mut rendered = stderr_report(&presentation);
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        rendered
    }

    /// Delivers the report: every record into a data output's
    /// `diagnostics`, in report order, or the rendered report on stderr.
    pub(crate) fn deliver(&self, sink: Option<&ReportSink>, launch_dir: &Path) {
        match sink {
            Some(sink) => sink.document_diagnostics(self.report.diagnostics().into_iter().cloned().collect()),
            None => eprint!("{}", self.render(launch_dir)),
        }
    }
}

/// `Checked notes.md: 1 error, 1 warning`, and the checks that did not run.
fn tally(label: &str, counts: FindingCounts) -> String {
    let mut tally = format!("Checked {label}: {}, {}", plural(counts.errors, "error"), plural(counts.warnings, "warning"));
    if counts.info > 0 {
        tally.push_str(&format!(", {} not run", plural(counts.info, "check")));
    }
    tally
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 { format!("1 {noun}") } else { format!("{count} {noun}s") }
}

/// The combined report after the reference check refused `md`: its error
/// issues and the warnings it reported, then the passive checks. `None` when
/// those checks add nothing, so the run prints the reference report alone.
pub(crate) fn after_refusal(
    md: &Markdown,
    request: &ComposeRequest,
    report: &ReferenceValidationReport,
    launch_dir: &Path,
) -> Option<CombinedReport> {
    let mut collector = Collector::new(md, launch_dir);
    collector.references(reported_issues(report));
    collector.passive_checks(md, request);
    collector.finish()
}

/// The combined report after the condition-blind pre-flight walk stopped at
/// an inclusion target it could not resolve (`error`). The references are
/// listed again with page blocks left in place, which finds the inclusion
/// inside a block whatever its condition; when that finds no refusal, the
/// stop's own record stands for it. `None` when `error` is no inclusion
/// failure, or when the passive checks add nothing.
pub(crate) fn after_inclusion_stop(
    md: &Markdown,
    request: &ComposeRequest,
    policy: &ResolvedPolicy,
    error: &Report,
    launch_dir: &Path,
) -> Option<CombinedReport> {
    let records = crate::output::report::failure_diagnostics(error);
    if !records.iter().any(|record| record.code.starts_with("dm.reference.")) {
        return None;
    }
    let mut collector = Collector::new(md, launch_dir);
    let blind = request
        .clone()
        .map_options(|options| options.disable(darkmatter::markdown::compose::ComposeOperation::PageBlocks));
    let refused: Vec<Diagnostic> = match super::validate(md, &blind) {
        Some(report) => report
            .issues
            .iter()
            .filter(|issue| {
                issue.kind == ReferenceKind::Transclusion
                    && issue.severity == ReferenceSeverity::Error
                    && !super::recovers_missing(policy, issue)
            })
            .map(reference_finding)
            .collect(),
        None => Vec::new(),
    };
    collector.references(if refused.is_empty() { records } else { refused });
    collector.passive_checks(md, request);
    collector.finish()
}

/// The combined report after the call check or the schema verdict stopped
/// the reference check itself (`error`): the check is repeated without them,
/// and when it then refuses the document, its findings lead, followed by the
/// stop's own records. `None` otherwise: the run reports `error` as before.
pub(crate) fn after_check_failure(
    md: &Markdown,
    request: &ComposeRequest,
    policy: &ResolvedPolicy,
    error: &Report,
    launch_dir: &Path,
) -> Option<CombinedReport> {
    let stop = Stop::of(error)?;
    let retry = request.clone().map_options(|options| {
        let options = options.with_deferred_schema_verdict(true);
        match stop {
            Stop::Call => options.without_call_check().with_dry_run(),
            Stop::Schema => options.with_dry_run(),
        }
    });
    let report = match super::check_references(md, &retry, policy) {
        Ok(super::ReferenceCheck::Refused(report)) => report,
        _ => return None,
    };
    let mut collector = Collector::new(md, launch_dir);
    collector.references(reported_issues(&report));
    let first = collector.push_all(crate::output::report::failure_diagnostics(error));
    let unchecked = match stop {
        Stop::Call => AFTER_CALL_REFUSAL,
        Stop::Schema => AFTER_PREPARATION,
    };
    collector.not_checked(unchecked, stop.reason(), first);
    collector.finish()
}

/// What stopped a check before the body was known.
#[derive(Clone, Copy)]
enum Stop {
    /// The call check proved a call wrong.
    Call,
    /// The frontmatter does not satisfy its schema.
    Schema,
}

impl Stop {
    fn of(error: &Report) -> Option<Self> {
        let markdown = error.chain().find_map(|cause| cause.downcast_ref::<MarkdownError>())?;
        Self::of_markdown(markdown)
    }

    fn of_markdown(error: &MarkdownError) -> Option<Self> {
        if error.rejected_call().is_some() {
            return Some(Self::Call);
        }
        matches!(error, MarkdownError::SchemaValidationFailed { .. }).then_some(Self::Schema)
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Call => "a call the check proves wrong stops the document first",
            Self::Schema => "the frontmatter does not satisfy its schema",
        }
    }
}

/// The issues a reference report shows: its errors and the warnings it
/// reports.
fn reported_issues(report: &ReferenceValidationReport) -> Vec<Diagnostic> {
    report
        .issues
        .iter()
        .filter(|issue| issue.severity == ReferenceSeverity::Error || issue.is_reported_warning())
        .map(reference_finding)
        .collect()
}

/// `issue` as a finding, worded as the reference report words its row
/// (`file not found in `::file``), with the next step for a missing target.
fn reference_finding(issue: &ReferenceIssue) -> Diagnostic {
    let mut finding = reference_issue_diagnostic(issue);
    if let Some(details) = &finding.reference_details
        && let (Some(reason), Some(construct)) = (&details.reason, &details.construct)
    {
        // A directive is code, as the reference report shows it.
        finding.message = if construct.starts_with("::") {
            format!("{reason} in `{construct}`")
        } else {
            format!("{reason} in {construct}")
        };
    }
    if matches!(issue.code, ReferenceIssueCode::MissingLocalTarget) && finding.hint.is_none() {
        let target = issue.reference_display.strip_prefix("./").unwrap_or(&issue.reference_display);
        finding.hint = Some(format!("create {target}, or correct the reference"));
    }
    finding
}

/// Builds one document's combined report.
struct Collector {
    sink: FindingSink,
    subject: SubjectId,
    label: String,
    document: Option<std::path::PathBuf>,
    /// Whether a check other than the reference check added a finding.
    contributed: bool,
}

impl Collector {
    fn new(md: &Markdown, launch_dir: &Path) -> Self {
        let document = md.source().as_ref().and_then(darkmatter::markdown::compose::ComposeSource::document_path);
        let label = match &document {
            Some(path) => match path.strip_prefix(launch_dir) {
                Ok(relative) if !relative.as_os_str().is_empty() => biscuit_file::to_forward_slash_string(relative),
                _ => biscuit_file::to_forward_slash_string(path),
            },
            None => "<stdin>".to_string(),
        };
        let mut sink = FindingSink::new();
        let subject = sink.register_subject(Subject::document(label.clone(), document.clone()));
        Self { sink, subject, label, document, contributed: false }
    }

    fn references(&mut self, records: Vec<Diagnostic>) {
        for record in records {
            self.sink.push(self.subject, record);
        }
    }

    /// Pushes `records` from another check; the first one's id.
    fn push_all(&mut self, records: Vec<Diagnostic>) -> Option<FindingId> {
        let mut first = None;
        for mut record in records {
            if record.source.is_none() {
                record.source = Some(DiagnosticSource::file_only(self.document.clone()));
            }
            let id = self.sink.push(self.subject, record);
            first.get_or_insert(id);
            self.contributed = true;
        }
        first
    }

    fn not_checked(&mut self, checks: &[&str], reason: &str, blocked_by: Option<FindingId>) {
        for check in checks {
            self.sink.not_checked(self.subject, check, reason, blocked_by);
        }
        self.contributed = true;
    }

    /// One dry-run composition of `md` under `request`, every unresolved
    /// inclusion left out, giving the call check, the schema verdict, and the
    /// prospective empty body.
    fn passive_checks(&mut self, md: &Markdown, request: &ComposeRequest) {
        let session = PassiveSession::new();
        let passive = super::with_reference_check_slot_session(request.clone().map_options(|options| {
            passive_options(options, session.clone())
        }));
        match md.compose_with_warnings(&passive) {
            Ok((composed, report)) => {
                let rendered = composed.content();
                let verdict = body_verdict(md.content(), rendered, &report, &session.deferrals());
                match verdict {
                    BodyVerdict::Visible | BodyVerdict::DependsOnReference => {}
                    BodyVerdict::Deferred(reason) => self.not_checked(AFTER_PREPARATION, &reason, None),
                    BodyVerdict::Empty(cause) => {
                        let finding = empty_body_finding(&cause, self.document.clone(), true);
                        self.sink.push(self.subject, finding);
                        self.contributed = true;
                    }
                }
            }
            Err(failure) => match Stop::of_markdown(&failure.error) {
                Some(stop) => {
                    let first = self.push_all(error_diagnostics(&failure.error));
                    let unchecked = match stop {
                        Stop::Call => AFTER_CALL_REFUSAL,
                        Stop::Schema => AFTER_PREPARATION,
                    };
                    self.not_checked(unchecked, stop.reason(), first);
                }
                // A failure of another family (an expression, a shell
                // command) is the composition's to report once the
                // references are fixed. The empty body is not checked, but
                // saying so alone would turn the reference report into a
                // combined one for a check nobody asked about.
                None => {
                    for check in AFTER_PREPARATION {
                        self.sink.not_checked(self.subject, check, "the document did not compose", None);
                    }
                }
            },
        }
    }

    fn finish(self) -> Option<CombinedReport> {
        self.contributed.then(|| CombinedReport { report: self.sink.finish(), label: self.label })
    }
}

/// The options of the passive composition: a dry run recording into
/// `session`, every unresolved inclusion left out (the reference findings
/// already report it), and no blocking policy, since the run already failed
/// and only the findings are wanted.
fn passive_options(options: ComposeOptions, session: PassiveSession) -> ComposeOptions {
    options
        .with_passive_session(session)
        .with_ignore_invalid_references(Some(true))
        .with_blocking_policy(BlockingPolicy::default())
}

/// What a passive composition proves about the body.
#[derive(Debug, PartialEq, Eq)]
enum BodyVerdict {
    /// Something visible survives.
    Visible,
    /// Nothing visible survives, but an inclusion left out for its broken
    /// reference might have contributed; the reference finding says so.
    DependsOnReference,
    /// Only constructs the dry run left as written are visible; `reason`
    /// says why the first was left.
    Deferred(String),
    /// The body is proven empty.
    Empty(EmptyBodyCause),
}

/// The verdict on `rendered`, the dry-run composition of a body authored as
/// `authored`.
fn body_verdict(
    authored: &str,
    rendered: &str,
    report: &darkmatter::markdown::compose::ComposeReport,
    deferrals: &[Deferral],
) -> BodyVerdict {
    if EmptyBodyCause::classify(authored, rendered).is_none() {
        let settled = without_deferred(rendered, deferrals);
        if EmptyBodyCause::classify(authored, &settled).is_none() {
            return BodyVerdict::Visible;
        }
        let reason = deferrals
            .iter()
            .find(|deferral| deferral.construct != DeferredConstruct::FrontmatterValue)
            .or_else(|| deferrals.first())
            .map_or_else(|| "a construct was left as written".to_string(), |deferral| deferral.reason.describe());
        return BodyVerdict::Deferred(reason);
    }
    if report.transclusions_skipped > 0 {
        return BodyVerdict::DependsOnReference;
    }
    EmptyBodyCause::classify_composed(authored, rendered, &report.excluded_blocks)
        .map_or(BodyVerdict::Visible, BodyVerdict::Empty)
}

/// `rendered` less what a dry run left as written: deferred `{{ … }}` spans,
/// `::shell` lines, and `::shell-block` regions, and every `::block` region
/// whose condition it could not decide, with the blocks nested in it.
fn without_deferred(rendered: &str, deferrals: &[Deferral]) -> String {
    let mut text = rendered.to_string();
    for deferral in deferrals {
        if matches!(
            deferral.construct,
            DeferredConstruct::Interpolation | DeferredConstruct::ShellDirective | DeferredConstruct::ShellBlock
        ) && !deferral.authored.is_empty()
        {
            text = text.replace(&deferral.authored, "");
        }
    }
    let conditions: Vec<&str> = deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::BlockCondition)
        .map(|deferral| deferral.authored.as_str())
        .collect();
    if conditions.is_empty() {
        return text;
    }
    let mut kept = Vec::new();
    let mut depth = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let opens = trimmed.starts_with("::block");
        if depth > 0 {
            if opens {
                depth += 1;
            } else if trimmed.starts_with("::end-block") {
                depth -= 1;
            }
            continue;
        }
        if opens && conditions.iter().any(|condition| trimmed.contains(condition)) {
            depth = 1;
            continue;
        }
        kept.push(line);
    }
    kept.join("\n")
}

/// The `dm.compose.empty_body` warning for `cause`, about `document`:
/// `prospective` when it is proven by a passive composition after a stop,
/// otherwise by the composition that ran. It sits at the first excluded
/// block when the cause is excluded blocks.
pub(crate) fn empty_body_finding(cause: &EmptyBodyCause, document: Option<std::path::PathBuf>, prospective: bool) -> Diagnostic {
    let lead = if prospective { "the body would be empty" } else { "the composed body is empty" };
    let source = match cause {
        EmptyBodyCause::ExcludedBlocks { blocks } if !blocks.is_empty() => {
            DiagnosticSource::at(document, blocks[0].line, 1)
        }
        _ => DiagnosticSource::file_only(document),
    };
    Diagnostic::actionable(EMPTY_BODY_CODE, Severity::Warning, format!("{lead}: the document {}", cause.phrase()), cause.hint())
        .with_source(source)
}

/// The `dm.compose.empty_body` warning of a composition that ran, when its
/// body is empty: the composed `rendered` body of `md`, with the blocks its
/// `report` left out.
pub(crate) fn empty_body_warning(
    md: &Markdown,
    rendered: &str,
    report: &darkmatter::markdown::compose::ComposeReport,
) -> Option<ComposeWarning> {
    let cause = EmptyBodyCause::classify_composed(md.content(), rendered, &report.excluded_blocks)?;
    let document = md.source().as_ref().and_then(darkmatter::markdown::compose::ComposeSource::document_path);
    let finding = empty_body_finding(&cause, document, false);
    let mut warning = ComposeWarning::new("empty-body", finding.message.clone());
    warning.code = Some(EMPTY_BODY_CODE.to_string());
    warning.diagnostic = Some(Box::new(finding));
    Some(warning)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::passive::DeferralReason;

    fn deferral(construct: DeferredConstruct, authored: &str) -> Deferral {
        Deferral {
            construct,
            reason: DeferralReason::DependsOn { name: "draft".to_string() },
            authored: authored.to_string(),
            key: None,
            line: None,
        }
    }

    #[test]
    fn a_deferred_block_and_its_nested_blocks_are_left_out() {
        let rendered = "::block when=\"draft\"\n::block when=\"x\"\ninner\n::end-block\nouter\n::end-block\nafter\n";
        let deferrals = [deferral(DeferredConstruct::BlockCondition, "draft")];
        assert_eq!(without_deferred(rendered, &deferrals), "after");
    }

    #[test]
    fn deferred_spans_and_shell_lines_are_left_out() {
        let rendered = "{{ name }}\n::shell git log\nkept";
        let deferrals = [
            deferral(DeferredConstruct::Interpolation, "{{ name }}"),
            deferral(DeferredConstruct::ShellDirective, "::shell git log"),
        ];
        assert_eq!(without_deferred(rendered, &deferrals).trim(), "kept");
    }

    #[test]
    fn a_body_of_only_deferred_constructs_is_not_checked() {
        let report = darkmatter::markdown::compose::ComposeReport::new();
        let deferrals = [deferral(DeferredConstruct::BlockCondition, "draft")];
        let verdict = body_verdict("::block when=\"draft\"\nx\n::end-block\n", "::block when=\"draft\"\n::end-block\n", &report, &deferrals);
        assert!(matches!(verdict, BodyVerdict::Deferred(_)), "{verdict:?}");
    }

    #[test]
    fn an_empty_body_that_left_out_an_inclusion_is_not_judged() {
        let mut report = darkmatter::markdown::compose::ComposeReport::new();
        report.transclusions_skipped = 1;
        assert_eq!(body_verdict("::file ./x.md\n", "\n", &report, &[]), BodyVerdict::DependsOnReference);
    }

    #[test]
    fn the_tally_counts_checks_that_did_not_run() {
        let counts = FindingCounts { errors: 1, warnings: 0, info: 2 };
        assert_eq!(tally("a.md", counts), "Checked a.md: 1 error, 0 warnings, 2 checks not run");
    }
}
