//! Human projection of a captured `filesystem query` report.
//!
//! Rendering only reads the report: it never queries again, filters
//! evidence, or infers what the library did not report. JSON is the
//! serialized report itself; this view may shorten long lists and always
//! says how much it left out.

use std::fmt::Write;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::WordWrap;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::query::{
    Coverage, CoverageStatus, Evidence, EvidenceKind, Limitation, LimitationExample, Mechanism,
    NativeString, Outcome, PathUsageReport, ProcessRecord, TargetKind,
};

/// Processes shown before the rest are counted as omitted.
pub const MAX_PROCESSES_SHOWN: usize = 50;

/// Evidence lines shown per process before the rest are counted as omitted.
pub const MAX_EVIDENCE_SHOWN: usize = 20;

/// Width used when stdout is not a terminal: wide enough that no line
/// wraps, because a path split across lines cannot be recovered by a reader
/// or a pipe.
pub const UNWRAPPED_WIDTH: u32 = 4096;

/// How to present a report.
pub struct View<'a> {
    pub term: &'a Terminal,
    pub width: u32,
    /// The file reference the caller typed, when it resolved to a path
    /// other than its own spelling. The report records only the path.
    pub reference: Option<&'a str>,
}

/// Renders the report for a terminal; `--plain` strips the styling afterward.
pub fn render_path_usage(report: &PathUsageReport, view: &View) -> String {
    let mut out = String::new();
    render_target(&mut out, report, view);
    render_processes(&mut out, report, view);
    render_coverage(&mut out, report, view);
    render_report_limitations(&mut out, report, view);
    render_summary(&mut out, report, view);
    out
}

/// Literal text made safe for Prose: control characters (terminal escapes,
/// newlines) and bidirectional overrides become visible `\u{..}`-style
/// spellings, then Prose markup characters are escaped.
pub(crate) fn literal(text: &str) -> String {
    Prose::escape_text(&neutralize(text))
}

/// Control characters and bidirectional overrides spelled visibly, so a
/// path or name cannot drive the terminal.
pub(crate) fn neutralize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || is_bidi_control(c) => {
                write!(out, "\\u{{{:x}}}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out
}

fn is_bidi_control(ch: char) -> bool {
    matches!(ch, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn native(value: &NativeString) -> String {
    literal(&value.display())
}

/// Wrap only at spaces: the default policy also breaks at `-` and `/`, which
/// would split a path across lines.
fn wrap_at_spaces() -> WordWrap {
    WordWrap::BespokeProse(None, vec![' '], None)
}

fn block(out: &mut String, markup: &str, view: &View) {
    let prose = Prose::new(markup).with_word_wrap(wrap_at_spaces());
    draw(out, &prose, view);
}

fn list(out: &mut String, items: Vec<String>, bullet: &str, view: &View) {
    if items.is_empty() {
        return;
    }
    let mut rendered = UnorderedList::empty().with_bullet(bullet);
    for item in items {
        let mut item = InlineProse::new(item);
        item.layout_mut().word_wrap = wrap_at_spaces();
        rendered.add(item);
    }
    draw(out, &rendered, view);
}

fn draw(out: &mut String, component: &impl TerminalRenderable, view: &View) {
    out.push_str(&component.render_in_width(view.term, view.width));
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

fn render_target(out: &mut String, report: &PathUsageReport, view: &View) {
    let target = &report.target;
    block(
        out,
        &format!(
            "<b>Filesystem usage:</b> {}",
            native(&target.resolved)
        ),
        view,
    );
    let mut details = Vec::new();
    if let Some(reference) = view.reference {
        details.push(format!("Reference: {}", literal(reference)));
    }
    if target.requested != target.resolved {
        details.push(format!("Requested as: {}", native(&target.requested)));
    }
    let scope = match (target.kind, target.recursive) {
        (TargetKind::File, _) => "file",
        (TargetKind::Directory, true) => "directory and its descendants",
        (TargetKind::Directory, false) => "directory only (target-only)",
    };
    details.push(format!("Scope: {scope}"));
    details.push(format!(
        "Budget: {} ms, used {} ms{}",
        report.budget_ms,
        report.elapsed_us / 1_000,
        if report.budget_exhausted {
            " (exhausted)"
        } else {
            ""
        }
    ));
    list(out, details, "  ", view);
    writeln!(out).unwrap();
}

fn render_processes(out: &mut String, report: &PathUsageReport, view: &View) {
    if report.processes.is_empty() {
        block(out, "No processes were found using this path.", view);
        writeln!(out).unwrap();
        return;
    }
    for process in report.processes.iter().take(MAX_PROCESSES_SHOWN) {
        render_process(out, process, view);
        writeln!(out).unwrap();
    }
    let omitted = report.processes.len().saturating_sub(MAX_PROCESSES_SHOWN);
    if omitted > 0 {
        block(
            out,
            &format!(
                "<i>{omitted} more {} omitted from this view; --json shows every process.</i>",
                plural(omitted as u64, "process", "processes")
            ),
            view,
        );
        writeln!(out).unwrap();
    }
}

fn render_process(out: &mut String, process: &ProcessRecord, view: &View) {
    let name = process
        .name
        .as_ref()
        .map(native)
        .unwrap_or_else(|| "<dim>name unavailable</dim>".to_string());
    block(out, &format!("<b>PID {}</b>  {name}", process.pid), view);

    let mut items = Vec::new();
    if let Some(executable) = &process.executable {
        items.push(format!("Executable: {}", native(executable)));
    }
    let mut identity = Vec::new();
    if let Some(user) = &process.user {
        identity.push(format!("user {}", literal(user)));
    }
    if let Some(started) = &process.start_time {
        identity.push(format!("started {}", started.format("%Y-%m-%d %H:%M:%S UTC")));
    }
    if process.identity_uncertain {
        identity.push("identity uncertain: no lifetime key ties this record to one process".into());
    }
    if !identity.is_empty() {
        items.push(format!("<dim>{}</dim>", identity.join(", ")));
    }
    for evidence in process.evidence.iter().take(MAX_EVIDENCE_SHOWN) {
        items.push(evidence_line(evidence));
    }
    let omitted = process.evidence.len().saturating_sub(MAX_EVIDENCE_SHOWN);
    if omitted > 0 {
        items.push(format!(
            "<i>{omitted} more evidence {} omitted from this view</i>",
            plural(omitted as u64, "record", "records")
        ));
    }
    list(out, items, "  ", view);
}

fn evidence_line(evidence: &Evidence) -> String {
    let label = match evidence.kind {
        EvidenceKind::WatchRegistration => "Watch registration",
        EvidenceKind::OpenHandle => "Open handle",
        EvidenceKind::WorkingDirectory => "Working directory",
        EvidenceKind::LoadedModule => "Loaded module",
    };
    let mut attributes = Vec::new();
    if evidence.mechanism != default_mechanism(evidence.kind) {
        attributes.push(mechanism_label(evidence.mechanism).to_string());
    }
    if let Some(access) = evidence.access {
        attributes.push(
            match (access.read, access.write) {
                (true, true) => "read/write",
                (true, false) => "read",
                (false, true) => "write",
                (false, false) => "no read/write",
            }
            .to_string(),
        );
    }
    if evidence.event_only == Some(true) {
        attributes.push("event-only".to_string());
    }
    if let Some(descriptor) = evidence.descriptor {
        attributes.push(format!("descriptor {descriptor}"));
    }
    if let Some(watch) = &evidence.watch {
        if let Some(id) = watch.watch_id {
            attributes.push(format!("watch {id}"));
        }
        if let Some(mask) = watch.mask {
            attributes.push(format!("mask {mask:#x}"));
        }
        match watch.recursive {
            Some(true) => attributes.push("recursive".to_string()),
            Some(false) => attributes.push("not recursive".to_string()),
            None => {}
        }
    }
    let attributes = if attributes.is_empty() {
        String::new()
    } else {
        format!(" <dim>({})</dim>", attributes.join(", "))
    };
    let paths = evidence
        .matched_paths
        .iter()
        .map(native)
        .collect::<Vec<_>>()
        .join(", ");
    let mut line = format!("{label}{attributes}: {paths}");
    if let Some(observed) = &evidence.observed_path
        && !evidence.matched_paths.contains(observed)
    {
        write!(line, " <dim>(observed as {})</dim>", native(observed)).unwrap();
    }
    line
}

/// The mechanism an evidence kind is normally found by; only a different
/// one is worth naming.
fn default_mechanism(kind: EvidenceKind) -> Mechanism {
    match kind {
        EvidenceKind::WatchRegistration => Mechanism::Inotify,
        EvidenceKind::OpenHandle => Mechanism::OpenHandles,
        EvidenceKind::WorkingDirectory => Mechanism::WorkingDirectories,
        EvidenceKind::LoadedModule => Mechanism::LoadedModules,
    }
}

fn mechanism_label(mechanism: Mechanism) -> &'static str {
    match mechanism {
        Mechanism::ProcessEnumeration => "Process enumeration",
        Mechanism::TreeIdentity => "Tree identity",
        Mechanism::OpenHandles => "Open handles",
        Mechanism::WorkingDirectories => "Working directories",
        Mechanism::Inotify => "inotify",
        Mechanism::Fanotify => "fanotify",
        Mechanism::Fsevents => "FSEvents",
        Mechanism::LoadedModules => "Loaded modules",
        Mechanism::DirectoryChangeSubscriptions => "Directory change subscriptions",
        Mechanism::Polling => "Polling watchers",
    }
}

fn status_label(status: CoverageStatus) -> &'static str {
    match status {
        CoverageStatus::Complete => "complete",
        CoverageStatus::Partial => "<b>partial</b>",
        CoverageStatus::Unsupported => "unsupported",
        CoverageStatus::Failed => "<b>failed</b>",
        CoverageStatus::NotAttempted => "<b>not attempted</b>",
    }
}

fn render_coverage(out: &mut String, report: &PathUsageReport, view: &View) {
    block(out, "<b>Coverage</b>", view);
    for coverage in &report.coverage {
        list(out, vec![coverage_line(coverage)], "  ", view);
        let limitations = coverage.limitations.iter().map(limitation_line).collect();
        list(out, limitations, "    - ", view);
    }
    writeln!(out).unwrap();
}

fn coverage_line(coverage: &Coverage) -> String {
    let mut line = format!(
        "{}: {}",
        mechanism_label(coverage.mechanism),
        status_label(coverage.status)
    );
    match (coverage.attempted, coverage.succeeded) {
        (Some(attempted), Some(succeeded)) => {
            write!(line, " <dim>({succeeded} of {attempted} succeeded)</dim>").unwrap();
        }
        (Some(attempted), None) => write!(line, " <dim>({attempted} attempted)</dim>").unwrap(),
        _ => {}
    }
    if let Some(reason) = &coverage.reason {
        write!(line, ". {}", literal(reason)).unwrap();
    }
    if let Some(truncation) = &coverage.truncation {
        write!(
            line,
            ". Evidence capped at {} per process and {} in total; {} omitted",
            truncation.per_process_limit, truncation.total_limit, truncation.omitted
        )
        .unwrap();
    }
    line
}

fn limitation_line(limitation: &Limitation) -> String {
    let mut line = literal(&limitation.message);
    if limitation.count > 1 {
        write!(line, " <dim>({} cases)</dim>", limitation.count).unwrap();
    }
    let examples: Vec<String> = limitation.examples.iter().map(example_text).collect();
    if !examples.is_empty() {
        write!(line, " <dim>e.g. {}</dim>", examples.join("; ")).unwrap();
    }
    line
}

fn example_text(example: &LimitationExample) -> String {
    let mut parts = Vec::new();
    if let Some(pid) = example.pid {
        parts.push(format!("PID {pid}"));
    }
    if let Some(path) = &example.path {
        parts.push(native(path));
    }
    if let Some(detail) = &example.detail {
        parts.push(literal(detail));
    }
    parts.join(": ")
}

fn render_report_limitations(out: &mut String, report: &PathUsageReport, view: &View) {
    if report.limitations.is_empty() {
        return;
    }
    block(out, "<b>Limitations</b>", view);
    let items = report.limitations.iter().map(limitation_line).collect();
    list(out, items, "  - ", view);
    writeln!(out).unwrap();
}

fn render_summary(out: &mut String, report: &PathUsageReport, view: &View) {
    let gaps = report.coverage.iter().any(|c| {
        matches!(
            c.status,
            CoverageStatus::Partial | CoverageStatus::Failed | CoverageStatus::NotAttempted
        )
    });
    let headline = match report.outcome {
        Outcome::Usable if gaps => "Discovery is partial.",
        Outcome::Usable => "Discovery finished for the processes this account can inspect.",
        Outcome::Unavailable => "<b>Discovery was unavailable:</b> no mechanism could inspect a process.",
        Outcome::Unsupported => "<b>Discovery is unsupported here:</b> no mechanism can inventory usage on this system.",
    };
    block(out, headline, view);
    let mut notes = Vec::new();
    if report.budget_exhausted {
        notes.push(format!(
            "The {} ms budget ran out before every mechanism finished.",
            report.budget_ms
        ));
    }
    if report
        .coverage
        .iter()
        .any(|c| c.status == CoverageStatus::Unsupported)
    {
        notes.push("Mechanisms marked unsupported cannot be inventoried on this system.".into());
    }
    if report.processes.is_empty() {
        notes.push("No matches does not establish that the target is unused.".into());
    } else {
        notes.push("A match shows usage; it does not prove that deleting the target will fail.".into());
    }
    list(out, notes, "  ", view);
}

fn plural<'a>(count: u64, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}
