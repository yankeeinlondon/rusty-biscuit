//! `hug refs`: the reference evidence of one selected declaration.
//!
//! Two phases. First every scanned file is analyzed for its declarations
//! over every kind and scope, so the selector sees locals too. The selector
//! must match exactly one declaration across the scan before anything is
//! written: several matches are an error listing them, none is an empty
//! report. Then only the declaring file is analyzed with
//! [`Capability::Occurrences`], from the same bytes the first phase read, and
//! the evidence is that file's whole occurrence list. A scan of one file
//! asks for the evidence in the first phase and has no second.
//!
//! Resolution is file-local, so only the selected declaration's own file can
//! hold an occurrence bound to it, and only its own body owns its calls. The
//! report is therefore read from that one file; a same-named call in another
//! file is never attached. User contract: `tree-hugger/docs/references/hug-refs.md`.

use std::collections::HashMap;
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tree_hugger::{
    Capability, Confidence, CoverageState, FileSymbolIndex, LimitReason, NamePattern, Occurrence, OccurrenceKind,
    ProgrammingLanguage, ReferenceCoverage, SchemaVersion, SourceFile, SymbolId, SymbolQuery, SymbolRecord,
    SymbolRef, TextPoint, TextSpan, TreeHuggerError,
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

/// A declaration the selector matched in the first phase, as the ambiguity
/// error lists it. It names its file's retained state by position, so any
/// number of matches in one file share that file's one state.
struct Candidate {
    /// Position of its file in [`Selection::files`].
    file: usize,
    listed: Listed,
}

/// What the ambiguity error shows of one matched declaration.
struct Listed {
    start: TextPoint,
    label: String,
    qualified_name: String,
    id: SymbolId,
}

impl Listed {
    fn of(record: &SymbolRecord) -> Self {
        Self {
            start: record.source.declaration_span.start,
            label: label(record),
            qualified_name: record.identity.qualified_name.clone(),
            id: record.id.clone(),
        }
    }
}

/// A file holding at least one match, retained once.
struct MatchedFile {
    display: String,
    /// The native path its records carry.
    path: PathBuf,
    /// Its first match, the reported record when it is the only match.
    first: SymbolRecord,
    state: Phased,
}

/// What the first phase kept of a matching file. Not `Clone`: one state per
/// file is moved into the evidence phase, never copied per match.
enum Phased {
    /// The bytes its declarations were read from, for the evidence phase.
    Read(SourceFile),
    /// Its evidence analysis: a scan of one file runs the evidence query
    /// first, since that file is the only one that can declare.
    Analyzed {
        index: Box<FileSymbolIndex>,
        completed_capabilities: Vec<Capability>,
    },
}

/// The first phase's matches: every matched declaration in scan order, and
/// one retained state per file holding any of them.
#[derive(Default)]
struct Selection {
    files: Vec<MatchedFile>,
    candidates: Vec<Candidate>,
}

impl Selection {
    fn add(&mut self, file: MatchedFile, listed: Vec<Listed>) {
        let position = self.files.len();
        self.files.push(file);
        self.candidates
            .extend(listed.into_iter().map(|listed| Candidate { file: position, listed }));
    }

    /// The one matched declaration's file, or `None` when nothing matched.
    /// Call only once ambiguity is ruled out.
    fn into_unique(mut self) -> Option<MatchedFile> {
        debug_assert!(self.candidates.len() <= 1 && self.files.len() == self.candidates.len());
        self.files.pop()
    }
}

/// The first phase: every file's declarations (and, for a scan of one file,
/// its evidence), keeping a file only when the selector matched one of its
/// records. Failed files are reported to `stderr` and collected in `errors`.
fn select(
    scan: &RefsScan,
    paths: &[PathBuf],
    analyzer: &Analyzer,
    declarations: &SymbolQuery,
    stats: &mut CacheStats,
    stderr: &mut dyn Write,
    errors: &mut Vec<ReportError>,
) -> Selection {
    let name_filter = match &scan.selector {
        Selector::Name(name) => Some(SymbolQuery::new().name(NamePattern::exact(name))),
        Selector::Id(_) => None,
    };
    let selects = |record: &SymbolRecord| match (&scan.selector, &name_filter) {
        (_, Some(filter)) => filter.matches(record),
        (Selector::Id(id), None) => record.id.0 == *id,
        (Selector::Name(_), None) => unreachable!("a name selector has a filter"),
    };
    let one_file = paths.len() == 1;
    let mut selection = Selection::default();
    let _ = in_file_order::<_, ()>(
        paths,
        |file| {
            let (analyzed, source) = analyzer.analyze_retaining(analyzer.read(file)?, declarations)?;
            let result = analyzed.result;
            let mut found = result.index.symbols.iter().filter(|record| selects(record));
            let Some(first) = found.next() else {
                return Ok::<_, TreeHuggerError>((analyzed.cache, None));
            };
            let listed: Vec<Listed> = std::iter::once(first).chain(found).map(Listed::of).collect();
            let first = first.clone();
            // A scan of one file already holds its evidence; any other
            // matching file keeps the exact bytes it was read from.
            let state = if one_file {
                Phased::Analyzed {
                    index: Box::new(result.index),
                    completed_capabilities: result.completed_capabilities,
                }
            } else {
                Phased::Read(source)
            };
            let matched = MatchedFile {
                display: portable_relative(file, &scan.scan_root),
                path: first.source.file_path.clone(),
                first,
                state,
            };
            Ok((analyzed.cache, Some((matched, listed))))
        },
        // Nothing is written until every file is delivered.
        |file, analyzed, flush| {
            let display = portable_relative(file, &scan.scan_root);
            match analyzed {
                Ok((cache, matched)) => {
                    stats.record(&display, &cache, scan.verbose, &mut *stderr);
                    if let Some((matched, listed)) = matched {
                        selection.add(matched, listed);
                    }
                }
                Err(error) => {
                    let _ = writeln!(stderr, "error: {}", neutralized(&error.to_string()));
                    errors.push(ReportError::new(display, &error));
                }
            }
            ControlFlow::Continue(flush.nothing_written())
        },
    );
    selection
}

/// The selected declaration with its file's evidence analysis.
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
    let evidence = SymbolQuery::new()
        .capabilities([Capability::Occurrences])
        .scan_root(&scan.scan_root);

    let mut files: Vec<(String, PathBuf)> = scan
        .files
        .iter()
        .map(|file| (portable_relative(file, &scan.scan_root), file.clone()))
        .collect();
    files.sort_by(|(left, _), (right, _)| portable_path_cmp(left, right));
    files.dedup_by(|left, right| left.1 == right.1);
    let paths: Vec<PathBuf> = files.iter().map(|(_, file)| file.clone()).collect();
    let declarations =
        if paths.len() == 1 { evidence.clone() } else { SymbolQuery::new().scan_root(&scan.scan_root) };

    let mut stats = CacheStats::new(analyzer.uses_cache());
    let mut stderr = io::stderr();
    let mut errors: Vec<ReportError> = Vec::new();
    let selection = select(&scan, &paths, analyzer, &declarations, &mut stats, &mut stderr, &mut errors);

    if selection.candidates.len() > 1 {
        if scan.verbose {
            let _ = writeln!(stderr, "{}", stats.summary());
        }
        let _ = write!(stderr, "{}", ambiguity_message(&scan.selector, &selection, &scan.git_root));
        return Ok(1);
    }
    // Phase two: the evidence of the one declaring file, from the bytes
    // phase one selected the declaration in.
    let found = selection.into_unique().and_then(|matched| {
        let MatchedFile {
            display, first, state, ..
        } = matched;
        let (index, completed_capabilities) = match state {
            Phased::Analyzed { index, completed_capabilities } => (*index, completed_capabilities),
            Phased::Read(source) => match analyzer.analyze_source(source, &evidence) {
                Ok(analyzed) => {
                    stats.record(&display, &analyzed.cache, scan.verbose, &mut stderr);
                    (analyzed.result.index, analyzed.result.completed_capabilities)
                }
                Err(error) => {
                    let _ = writeln!(stderr, "error: {}", neutralized(&error.to_string()));
                    errors.push(ReportError::new(display, &error));
                    return None;
                }
            },
        };
        // The same bytes give the same IDs; the evidence analysis's own
        // record is the one reported.
        let record = index.symbols.iter().find(|symbol| symbol.id == first.id).cloned().unwrap_or(first);
        Some(Match {
            display,
            record,
            index,
            completed_capabilities,
        })
    });
    if scan.verbose {
        let _ = writeln!(stderr, "{}", stats.summary());
    }

    let report = match found {
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
fn ambiguity_message(selector: &Selector, selection: &Selection, git_root: &Path) -> String {
    let given = match selector {
        Selector::Name(name) => format!("`{name}`"),
        Selector::Id(id) => format!("ID `{id}`"),
    };
    let mut message = format!(
        "error: {given} names {} declarations; select one with `--id` or a file-prefixed qualified name:\n",
        selection.candidates.len()
    );
    for Candidate { file, listed } in &selection.candidates {
        let path = &selection.files[*file].path;
        message.push_str(&format!(
            "  {}:{}:{}  {} {}  {}\n",
            neutralized(&crate::display_path(path, Some(git_root))),
            listed.start.line,
            listed.start.column,
            listed.label,
            neutralized(&listed.qualified_name),
            listed.id.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `count` classes, each declaring a method `probe`.
    fn probes(count: usize) -> String {
        (0..count)
            .map(|class| format!("class C{class}:\n    def probe(self):\n        return self.value\n\n"))
            .collect()
    }

    fn scan_of(root: &Path, selector: Selector, names: &[&str]) -> RefsScan {
        RefsScan {
            selector,
            direction: Direction::References,
            files: names.iter().map(|name| tree_hugger::file_identity(&root.join(name))).collect(),
            scan_root: tree_hugger::file_identity(root),
            git_root: tree_hugger::file_identity(root),
            format: RefsFormat::Json,
            verbose: false,
        }
    }

    fn select_in(scan: &RefsScan) -> (Selection, Vec<ReportError>) {
        let analyzer = Analyzer::new(None, None, false);
        let declarations = SymbolQuery::new().scan_root(&scan.scan_root);
        let mut stats = CacheStats::new(false);
        let mut errors = Vec::new();
        let selection = select(scan, &scan.files, &analyzer, &declarations, &mut stats, &mut io::sink(), &mut errors);
        (selection, errors)
    }

    #[test]
    fn many_same_named_matches_in_one_file_share_its_one_analysis() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("sample.py"), probes(40)).expect("write sample");
        let scan = scan_of(dir.path(), Selector::Name("probe".into()), &["sample.py"]);

        let (selection, errors) = select_in(&scan);

        assert!(errors.is_empty());
        assert_eq!(selection.candidates.len(), 40);
        assert_eq!(selection.files.len(), 1, "one retained state for one matching file");
        assert!(selection.candidates.iter().all(|candidate| candidate.file == 0));
        assert!(matches!(selection.files[0].state, Phased::Analyzed { .. }));
        let lines: Vec<u32> = selection.candidates.iter().map(|candidate| candidate.listed.start.line).collect();
        assert_eq!(lines, (0..40).map(|class| class * 4 + 2).collect::<Vec<_>>(), "scan order");
    }

    #[test]
    fn only_matching_files_retain_one_byte_snapshot_each() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = probes(40);
        std::fs::write(dir.path().join("a.py"), &source).expect("write a");
        std::fs::write(dir.path().join("b.py"), &source).expect("write b");
        std::fs::write(dir.path().join("other.py"), "def unrelated():\n    return 1\n").expect("write other");
        let scan = scan_of(dir.path(), Selector::Name("probe".into()), &["a.py", "b.py", "other.py"]);

        let (selection, errors) = select_in(&scan);

        assert!(errors.is_empty());
        assert_eq!(selection.candidates.len(), 80);
        assert_eq!(selection.files.len(), 2, "one retained state per matching file, none for other.py");
        let displays: Vec<&str> = selection.files.iter().map(|file| file.display.as_str()).collect();
        assert_eq!(displays, ["a.py", "b.py"]);
        for (position, file) in selection.files.iter().enumerate() {
            let Phased::Read(snapshot) = &file.state else {
                panic!("a multi-file scan keeps the bytes it read");
            };
            assert_eq!(snapshot.source(), source);
            let in_file = selection.candidates.iter().filter(|candidate| candidate.file == position).count();
            assert_eq!(in_file, 40);
        }
    }

    #[test]
    fn an_id_selects_exactly_one_retained_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let source = probes(40);
        std::fs::write(dir.path().join("a.py"), &source).expect("write a");
        std::fs::write(dir.path().join("b.py"), &source).expect("write b");
        let by_name = scan_of(dir.path(), Selector::Name("probe".into()), &["a.py", "b.py"]);
        let (named, _) = select_in(&by_name);
        let wanted = named.candidates[57].listed.id.clone();
        let by_id = scan_of(dir.path(), Selector::Id(wanted.0.clone()), &["a.py", "b.py"]);

        let (selection, errors) = select_in(&by_id);

        assert!(errors.is_empty());
        assert_eq!(selection.candidates.len(), 1);
        let matched = selection.into_unique().expect("one match");
        assert_eq!(matched.display, "b.py");
        assert_eq!(matched.first.id, wanted);
    }
}
