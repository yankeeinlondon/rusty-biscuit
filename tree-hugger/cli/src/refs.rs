//! `hug refs`: the reference evidence of one selected declaration.
//!
//! Every scanned file is analyzed with [`Capability::Occurrences`] over every
//! kind and scope, so the selector sees locals too and the evidence is the
//! file's whole occurrence list. The selector must match exactly one
//! declaration across the scan before anything is written: several matches
//! are an error listing them, none is an empty report.
//!
//! Resolution is file-local, so only the selected declaration's own file can
//! hold an occurrence bound to it, and only its own body owns its calls. The
//! report is therefore read from that one file; a same-named call in another
//! file is never attached. User contract: `tree-hugger/docs/references.md`.

use std::collections::HashMap;
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tree_hugger::{
    Capability, Confidence, CoverageState, FileSymbolIndex, LimitReason, NamePattern, Occurrence, OccurrenceKind,
    ProgrammingLanguage, ReferenceCoverage, SchemaVersion, SymbolId, SymbolQuery, SymbolRecord,
    SymbolRef, TextSpan, TreeHuggerError,
};

use crate::order::portable_path_cmp;
use biscuit_terminal::prelude::{Prose, UnorderedList};

use crate::render::{RenderOptions, Renderer, label, literal, neutralized};
use crate::report::{REPORT_SCHEMA_VERSION, ReportError, portable_relative, trim_record};
use crate::scan::{Analyzer, CacheStats, in_file_order};

/// The file-local resolution boundary, stated by every report, empty or not.
pub(crate) const BOUNDARY: &str = "References are resolved within one file: only the declaration's own \
file is searched, so a caller in another file is never listed, even when it uses the same name.";

/// How the declaration is selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Selector {
    /// A name under the exact smart-case rules (a qualified one may start
    /// with its scan-root-relative file, `src/parser.rs#Parser::parse`).
    Name(String),
    /// A record ID, compared case-sensitively.
    Id(String),
}

/// Which occurrences are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    /// Incoming calls and usages (the default).
    References,
    /// Incoming calls only (`--callers`).
    Callers,
    /// Calls the declaration's own body makes (`--callees`).
    Callees,
}

/// How the report is written.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RefsFormat {
    Terminal(RenderOptions),
    Json,
}

/// One run of `hug refs`.
pub(crate) struct RefsScan {
    pub(crate) selector: Selector,
    pub(crate) direction: Direction,
    pub(crate) files: Vec<PathBuf>,
    /// JSON `root_dir`, the anchor of JSON paths and of qualified-name file
    /// prefixes.
    pub(crate) scan_root: PathBuf,
    /// Anchor of terminal paths.
    pub(crate) git_root: PathBuf,
    pub(crate) format: RefsFormat,
    pub(crate) verbose: bool,
}

/// Why a listed occurrence carries no target ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AbsentId {
    /// `Scoped` to an import of the owner: it names something outside the
    /// file.
    Import,
    /// `Scoped` to a binding that is no declaration record (a loop or
    /// pattern binding, for instance).
    BindingWithoutRecord,
    /// `NameOnly`: the candidates are suggestions, not a target.
    NameOnly,
    /// No supported binding and no same-named declaration in the file.
    Unresolved,
}

/// One listed occurrence. Owner and candidate qualified names carry the
/// report's file prefix, as declaration reports' records do; the target
/// keeps the name as written.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Row {
    #[serde(flatten)]
    pub(crate) occurrence: Occurrence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) absent_id: Option<AbsentId>,
}

/// The selected declaration's file and its listed evidence.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FileEvidence {
    /// Portable scan-root-relative path.
    pub(crate) file: String,
    pub(crate) language: ProgrammingLanguage,
    pub(crate) completed_capabilities: Vec<Capability>,
    /// Incoming: `Scoped` occurrences bound to the declaration's ID.
    /// Callees: every call the declaration's own body makes, whatever its
    /// confidence.
    pub(crate) occurrences: Vec<Row>,
    /// Incoming only: `NameOnly` occurrences that list the declaration among
    /// their candidates. Not proven to refer to it.
    pub(crate) name_only_suggestions: Vec<Row>,
    /// The coverage of every region owning a listed row, and for callees the
    /// declaration's own region, outer first.
    pub(crate) coverage: Vec<ReferenceCoverage>,
    /// Incoming only: how many regions of the file have limited coverage,
    /// where a reference to the declaration may be uncertain and so unlisted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limited_regions: Option<usize>,
}

/// What `hug refs` reports.
#[derive(Debug, Clone)]
pub(crate) struct RefsReport {
    /// The selected declaration, as analyzed (absolute file path, prefixed
    /// qualified name); `None` when nothing matched.
    pub(crate) declaration: Option<SymbolRecord>,
    pub(crate) evidence: Option<FileEvidence>,
}

/// A declaration the selector matched, with its file's analysis.
struct Match {
    display: String,
    record: SymbolRecord,
    index: FileSymbolIndex,
    completed_capabilities: Vec<Capability>,
}

/// Runs `scan` and returns the exit status: `0`; `1` when a file failed,
/// the selector is ambiguous, or stdout could not be written. A closed stdout
/// ends the run quietly with `0`.
pub(crate) fn run(scan: RefsScan, analyzer: &Analyzer) -> Result<i32, TreeHuggerError> {
    let query = SymbolQuery::new()
        .capabilities([Capability::Occurrences])
        .scan_root(&scan.scan_root);
    let name_filter = match &scan.selector {
        Selector::Name(name) => Some(SymbolQuery::new().name(NamePattern::exact(name))),
        Selector::Id(_) => None,
    };
    let selects = |record: &SymbolRecord| match (&scan.selector, &name_filter) {
        (_, Some(filter)) => filter.matches(record),
        (Selector::Id(id), None) => record.id.0 == *id,
        (Selector::Name(_), None) => unreachable!("a name selector has a filter"),
    };

    let mut files: Vec<(String, PathBuf)> = scan
        .files
        .iter()
        .map(|file| (portable_relative(file, &scan.scan_root), file.clone()))
        .collect();
    files.sort_by(|(left, _), (right, _)| portable_path_cmp(left, right));
    files.dedup_by(|left, right| left.1 == right.1);
    let paths: Vec<PathBuf> = files.iter().map(|(_, file)| file.clone()).collect();

    let mut stats = CacheStats::new(analyzer.uses_cache());
    let mut stderr = io::stderr();
    let mut errors: Vec<ReportError> = Vec::new();
    let mut matches: Vec<Match> = Vec::new();
    let _ = in_file_order::<_, ()>(
        &paths,
        |file| analyzer.analyze(file, &query),
        |file, analyzed| {
            let display = portable_relative(file, &scan.scan_root);
            match analyzed {
                Ok(analyzed) => {
                    stats.record(&display, &analyzed.cache, scan.verbose, &mut stderr);
                    let result = analyzed.result;
                    let found: Vec<SymbolRecord> =
                        result.index.symbols.iter().filter(|record| selects(record)).cloned().collect();
                    // Only a matching file's evidence is kept: no other file
                    // can hold an occurrence bound to the declaration.
                    for record in found {
                        matches.push(Match {
                            display: display.clone(),
                            record,
                            index: result.index.clone(),
                            completed_capabilities: result.completed_capabilities.clone(),
                        });
                    }
                }
                Err(error) => {
                    let _ = writeln!(stderr, "error: {error}");
                    errors.push(ReportError::new(display, &error));
                }
            }
            ControlFlow::Continue(())
        },
    );
    if scan.verbose {
        let _ = writeln!(stderr, "{}", stats.summary());
    }

    if matches.len() > 1 {
        let _ = write!(stderr, "{}", ambiguity_message(&scan.selector, &matches, &scan.git_root));
        return Ok(1);
    }
    let report = match matches.pop() {
        Some(found) => build_report(found, scan.direction),
        None => RefsReport {
            declaration: None,
            evidence: None,
        },
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let written = match scan.format {
        RefsFormat::Json => write_json(&mut out, &scan, report, &errors),
        RefsFormat::Terminal(options) => {
            let renderer = Renderer::new(options, Some(scan.git_root.clone()));
            write_terminal(&mut out, &renderer, &scan, &report)
        }
    }
    .and_then(|()| out.flush());
    match written {
        Ok(()) => Ok(if errors.is_empty() { 0 } else { 1 }),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(0),
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot write output: {error}");
            Ok(1)
        }
    }
}

/// The error for a selector matching several declarations: each one's
/// location, qualified name, kind and ID, in scan order.
fn ambiguity_message(selector: &Selector, matches: &[Match], git_root: &Path) -> String {
    let given = match selector {
        Selector::Name(name) => format!("`{name}`"),
        Selector::Id(id) => format!("ID `{id}`"),
    };
    let mut message = format!(
        "error: {given} names {} declarations; select one with `--id` or a file-prefixed qualified name:\n",
        matches.len()
    );
    for found in matches {
        let start = &found.record.source.declaration_span.start;
        message.push_str(&format!(
            "  {}:{}:{}  {} {}  {}\n",
            crate::display_path(&found.record.source.file_path, Some(git_root)),
            start.line,
            start.column,
            label(&found.record),
            neutralized(&found.record.identity.qualified_name),
            found.record.id.0,
        ));
    }
    message
}

/// Selects the listed rows of `found`'s file for `direction`.
fn build_report(found: Match, direction: Direction) -> RefsReport {
    let Match {
        display,
        record,
        index,
        completed_capabilities,
    } = found;
    let displayed: HashMap<&SymbolId, &str> = index
        .symbols
        .iter()
        .map(|symbol| (&symbol.id, symbol.identity.qualified_name.as_str()))
        .collect();
    let id = &record.id;
    let owned_by_declaration =
        |occurrence: &Occurrence| occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(id);

    let mut occurrences = Vec::new();
    let mut suggestions = Vec::new();
    for occurrence in &index.occurrences {
        match direction {
            Direction::References | Direction::Callers => {
                if direction == Direction::Callers && occurrence.kind != OccurrenceKind::Call {
                    continue;
                }
                if occurrence.confidence == Confidence::Scoped && occurrence.target.id.as_ref() == Some(id) {
                    occurrences.push(row(occurrence, None, &displayed));
                } else if occurrence.confidence == Confidence::NameOnly
                    && occurrence.candidates.iter().any(|candidate| candidate.id.as_ref() == Some(id))
                {
                    suggestions.push(row(occurrence, Some(AbsentId::NameOnly), &displayed));
                }
            }
            Direction::Callees => {
                if occurrence.kind != OccurrenceKind::Call || !owned_by_declaration(occurrence) {
                    continue;
                }
                let absent = match (occurrence.confidence, &occurrence.target.id) {
                    (Confidence::Scoped, Some(_)) => None,
                    (Confidence::Scoped, None) if occurrence.import_span.is_some() => {
                        Some(AbsentId::Import)
                    }
                    (Confidence::Scoped, None) => Some(AbsentId::BindingWithoutRecord),
                    (Confidence::NameOnly, _) => Some(AbsentId::NameOnly),
                    (Confidence::Unresolved, _) => Some(AbsentId::Unresolved),
                };
                occurrences.push(row(occurrence, absent, &displayed));
            }
        }
    }

    let coverage: Vec<ReferenceCoverage> = index
        .reference_coverage
        .iter()
        .filter(|region| {
            let owns = |listed: &Row| listed.occurrence.owner_span == region.span;
            occurrences.iter().chain(&suggestions).any(owns)
                || (direction == Direction::Callees
                    && region.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(id))
        })
        .map(|region| {
            let mut region = region.clone();
            if let Some(owner) = &mut region.owner {
                prefix_ref(owner, &displayed);
            }
            region
        })
        .collect();
    let limited_regions = (direction != Direction::Callees).then(|| {
        index
            .reference_coverage
            .iter()
            .filter(|region| region.state == CoverageState::Limited)
            .count()
    });

    RefsReport {
        declaration: Some(record),
        evidence: Some(FileEvidence {
            file: display,
            language: index.language,
            completed_capabilities,
            occurrences,
            name_only_suggestions: suggestions,
            coverage,
            limited_regions,
        }),
    }
}

fn row(occurrence: &Occurrence, absent_id: Option<AbsentId>, displayed: &HashMap<&SymbolId, &str>) -> Row {
    let mut occurrence = occurrence.clone();
    if let Some(owner) = &mut occurrence.owner {
        prefix_ref(owner, displayed);
    }
    for candidate in &mut occurrence.candidates {
        prefix_ref(candidate, displayed);
    }
    Row {
        occurrence,
        absent_id,
    }
}

/// Gives a declaration reference its record's displayed qualified name;
/// stored evidence keeps the in-file one.
fn prefix_ref(reference: &mut SymbolRef, displayed: &HashMap<&SymbolId, &str>) {
    if let Some(name) = reference.id.as_ref().and_then(|id| displayed.get(id)) {
        reference.qualified_name = Some((*name).to_string());
    }
}

/// The `--json` envelope; `errors` is always last.
#[derive(Serialize)]
struct Envelope<'a> {
    report_schema_version: u32,
    schema_version: SchemaVersion,
    root_dir: String,
    boundary: &'static str,
    selector: &'a Selector,
    direction: Direction,
    /// `null` when nothing matched.
    declaration: Option<SymbolRecord>,
    /// The declaration's file; empty when nothing matched.
    files: Vec<FileEvidence>,
    errors: &'a [ReportError],
}

fn write_json(out: &mut dyn Write, scan: &RefsScan, report: RefsReport, errors: &[ReportError]) -> io::Result<()> {
    let declaration = report.declaration.map(|mut record| {
        trim_record(&mut record, &scan.scan_root);
        record
    });
    let envelope = Envelope {
        report_schema_version: REPORT_SCHEMA_VERSION,
        schema_version: SchemaVersion::CURRENT,
        root_dir: biscuit_file::to_portable_string(&scan.scan_root),
        boundary: BOUNDARY,
        selector: &scan.selector,
        direction: scan.direction,
        declaration,
        files: report.evidence.into_iter().collect(),
        errors,
    };
    serde_json::to_writer(&mut *out, &envelope)?;
    out.write_all(b"\n")
}

fn write_terminal(out: &mut dyn Write, renderer: &Renderer, scan: &RefsScan, report: &RefsReport) -> io::Result<()> {
    let (Some(record), Some(evidence)) = (&report.declaration, &report.evidence) else {
        let given = match &scan.selector {
            Selector::Name(name) => format!("named <b>{}</b>", literal(name)),
            Selector::Id(id) => format!("with ID <b>{}</b>", literal(id)),
        };
        renderer.line(out, 0, &format!("No declaration {given} in the scanned files."))?;
        return renderer.line(out, 0, &format!("<dim>{}</dim>", literal(BOUNDARY)));
    };

    let heading = match scan.direction {
        Direction::References => "References to",
        Direction::Callers => "Callers of",
        Direction::Callees => "Callees of",
    };
    let start = &record.source.declaration_span.start;
    let location = format!("{}:{}:{}", renderer.display(&record.source.file_path), start.line, start.column);
    renderer.line(
        out,
        0,
        &format!(
            "<b>{heading}</b> {} <dim>{}</dim>",
            renderer.declaration(record, true),
            renderer.link(&record.source.file_path, start.line, &literal(&location)),
        ),
    )?;
    renderer.line(out, 0, &format!("<dim>{}</dim>", literal(BOUNDARY)))?;

    let file = &record.source.file_path;
    match scan.direction {
        Direction::References | Direction::Callers => {
            if evidence.occurrences.is_empty() && evidence.name_only_suggestions.is_empty() {
                let none = if scan.direction == Direction::Callers { "(no callers)" } else { "(no references)" };
                renderer.line(out, 0, &format!("<dim>{none}</dim>"))?;
            }
            if !evidence.occurrences.is_empty() {
                writeln!(out)?;
                renderer.line(out, 0, "<b>Established</b> <dim>(Scoped to this declaration)</dim>")?;
                write_groups(out, renderer, file, evidence, &evidence.occurrences)?;
            }
            if !evidence.name_only_suggestions.is_empty() {
                writeln!(out)?;
                renderer.line(
                    out,
                    0,
                    "<b>Name-only suggestions</b> <dim>(same name, not proven to refer to this declaration)</dim>",
                )?;
                write_groups(out, renderer, file, evidence, &evidence.name_only_suggestions)?;
            }
            if let Some(limited) = evidence.limited_regions.filter(|limited| *limited > 0) {
                writeln!(out)?;
                let regions = if limited == 1 { "region of this file has" } else { "regions of this file have" };
                renderer.line(
                    out,
                    0,
                    &format!(
                        "<dim>{limited} {regions} limited coverage; a reference there may be uncertain and is \
                         then not listed as established.</dim>"
                    ),
                )?;
            }
        }
        Direction::Callees => {
            if let Some(region) = evidence
                .coverage
                .iter()
                .find(|region| region.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(&record.id))
            {
                renderer.line(out, 0, &format!("<dim>coverage: {}</dim>", coverage_text(region)))?;
            }
            if evidence.occurrences.is_empty() {
                renderer.line(out, 0, "<dim>(no calls)</dim>")?;
            } else {
                writeln!(out)?;
                write_rows(out, renderer, file, &evidence.occurrences, 2)?;
            }
        }
    }
    Ok(())
}

/// `rows` grouped by owning region, in order of each region's first row,
/// every call site kept.
fn write_groups(
    out: &mut dyn Write,
    renderer: &Renderer,
    file: &Path,
    evidence: &FileEvidence,
    rows: &[Row],
) -> io::Result<()> {
    let mut groups: Vec<(&TextSpan, Vec<Row>)> = Vec::new();
    for listed in rows {
        let span = &listed.occurrence.owner_span;
        match groups.iter_mut().find(|(owner_span, _)| *owner_span == span) {
            Some((_, group)) => group.push(listed.clone()),
            None => groups.push((span, vec![listed.clone()])),
        }
    }
    for (span, group) in &groups {
        let (owner, place) = owner_heading(&group[0].occurrence, span, evidence);
        renderer.line(
            out,
            2,
            &format!("{owner} <dim>{}</dim>", renderer.link(file, span.start.line, &literal(&place))),
        )?;
        write_rows(out, renderer, file, group, 4)?;
    }
    Ok(())
}

/// The heading of an owning region and the location it links: the owner's
/// displayed qualified name at its start, else file-level or anonymous code
/// with its whole span.
fn owner_heading(occurrence: &Occurrence, span: &TextSpan, evidence: &FileEvidence) -> (String, String) {
    if let Some(owner) = &occurrence.owner {
        let name = format!("<b>{}</b>", literal(owner.qualified_name.as_deref().unwrap_or(&owner.name)));
        return (name, format!("{}:{}", span.start.line, span.start.column));
    }
    let whole_file = evidence
        .coverage
        .first()
        .is_some_and(|region| region.owner.is_none() && region.span == *span && span.start_byte == 0);
    let label = if whole_file { "file-level code" } else { "code without a declaration owner" };
    let range = format!(
        "[{}:{}–{}:{}]",
        span.start.line, span.start.column, span.end.line, span.end.column
    );
    (format!("<i>{label}</i>"), range)
}

/// One bullet per occurrence: kind, written target, location and, where it
/// has no target ID, why.
fn write_rows(out: &mut dyn Write, renderer: &Renderer, file: &Path, rows: &[Row], indent: usize) -> io::Result<()> {
    let mut list = UnorderedList::empty();
    for listed in rows {
        let occurrence = &listed.occurrence;
        let kind = match occurrence.kind {
            OccurrenceKind::Call => "Call",
            OccurrenceKind::Usage => "Usage",
        };
        let written = occurrence.target.qualified_name.as_deref().unwrap_or(&occurrence.target.name);
        let place = format!("{}:{}", occurrence.span.start.line, occurrence.span.start.column);
        let explanation = match listed.absent_id {
            None => String::new(),
            Some(AbsentId::Import) => " <dim>— Scoped to an import (outside this file, no ID)</dim>".to_string(),
            Some(AbsentId::BindingWithoutRecord) => {
                " <dim>— Scoped to a binding that is not a declaration record (no ID)</dim>".to_string()
            }
            Some(AbsentId::NameOnly) => {
                let candidates: Vec<String> = occurrence
                    .candidates
                    .iter()
                    .map(|candidate| literal(candidate.qualified_name.as_deref().unwrap_or(&candidate.name)))
                    .collect();
                format!(" <dim>— NameOnly, candidates: {}</dim>", candidates.join(", "))
            }
            Some(AbsentId::Unresolved) => {
                " <dim>— Unresolved (no binding or same-named declaration in this file)</dim>".to_string()
            }
        };
        list.add(Prose::new(format!(
            "{kind} <b>{}</b> <dim>{}</dim>{explanation}",
            literal(written),
            renderer.link(file, occurrence.span.start.line, &literal(&place)),
        )));
    }
    renderer.list(out, indent, &list)
}

fn coverage_text(region: &ReferenceCoverage) -> String {
    match region.state {
        CoverageState::Complete => "complete".to_string(),
        CoverageState::Limited => {
            let reasons: Vec<&str> = region.reasons.iter().map(|reason| reason_code(*reason)).collect();
            format!("limited ({})", reasons.join(", "))
        }
    }
}

/// The serialized code of `reason`, as JSON writes it.
fn reason_code(reason: LimitReason) -> &'static str {
    match reason {
        LimitReason::UnsupportedLanguage => "unsupported_language",
        LimitReason::UnsupportedBindingForm => "unsupported_binding_form",
        LimitReason::MissingCallCapture => "missing_call_capture",
        LimitReason::IndirectDispatch => "indirect_dispatch",
        LimitReason::UnexpandedMacro => "unexpanded_macro",
        LimitReason::OwnershipUnknown => "ownership_unknown",
        LimitReason::ParseError => "parse_error",
        LimitReason::EmptyReferenceQuery => "empty_reference_query",
    }
}
