//! The declaration commands (`functions`, `types`, `symbols`, `classes`):
//! each asks the library for its fixed kind set narrowed by the record
//! selectors and only the capabilities it shows and filters on, analyzes the
//! files in parallel (the library applies the selectors once, after the
//! cache), orders the records, and writes them in the one output mode: the
//! terminal listing, outline or table, `--json`, `--json-full`, `--jsonl`,
//! `--count`, `--stats` or a terminal-independent export.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use tree_hugger::{
    AnalysisPass, Capability, ProgrammingLanguage, SchemaVersion, SymbolKindV2, SymbolQuery,
    SymbolQueryResult, SymbolRecord, SymbolScope, TreeHuggerError,
};

use crate::classes::{self, ClassSelection, ClassSummary};
use crate::order::{compare, portable_path_cmp, sort_file_records};
use crate::prelude::{PreludeFilter, resolve_prelude_symbols};
use crate::output::{ExportFormat, View};
use crate::markdown::MarkdownLayout;
use crate::prose::ProseLayout;
use crate::render::{
    FileListing, Layout, ListingOptions, ListingStream, NO_SYMBOLS, RenderOptions, Renderer, TerminalLayout,
    listing_order, neutralized,
};
use crate::report::{
    Entries, FileEntry, JsonLine, JsonReport, REPORT_SCHEMA_VERSION, ReportError, portable_relative,
    relativize_index, serialized, trim_record,
};
use crate::scan::{Analyzed, Analyzer, CacheStats, CacheUse, Flushed, in_file_order};
use crate::selection::Selection;
use crate::stats::Stats;
use crate::grouping::{Catalog, Group, Grouping};

/// Kinds `functions` lists.
pub(crate) const FUNCTION_KINDS: [SymbolKindV2; 3] = [
    SymbolKindV2::Function,
    SymbolKindV2::Method,
    SymbolKindV2::Constructor,
];

/// Kinds `types` lists.
pub(crate) const TYPE_KINDS: [SymbolKindV2; 6] = [
    SymbolKindV2::Type,
    SymbolKindV2::Class,
    SymbolKindV2::Interface,
    SymbolKindV2::Enum,
    SymbolKindV2::Trait,
    SymbolKindV2::TypeAlias,
];

/// The scopes `symbols` lists unless `--scope` or `--all-scopes` says
/// otherwise; the other commands list every scope.
pub(crate) const SYMBOLS_DEFAULT_SCOPES: [SymbolScope; 2] = [SymbolScope::Item, SymbolScope::Member];

/// Which declaration command runs.
#[derive(Debug, Clone)]
pub(crate) enum Declarations {
    Functions,
    Types,
    Symbols,
    Classes(Box<ClassSelection>),
}

impl Declarations {
    /// The command's fixed kind set narrowed by `selection`, the selection's
    /// predicates, and the capabilities `format` shows (see
    /// [`ReportFormat::capabilities`]); no listing runs references,
    /// semantics, structured docs or diagnostics. `classes` applies its
    /// predicates itself ([`ClassSelection`]), so its query carries none.
    pub(crate) fn query(&self, format: ReportFormat, selection: &Selection) -> SymbolQuery {
        let capabilities = format.capabilities();
        let (command_kinds, default_scopes): (Option<&[SymbolKindV2]>, Option<&[SymbolScope]>) = match self {
            Self::Functions => (Some(&FUNCTION_KINDS), None),
            Self::Types => (Some(&TYPE_KINDS), None),
            Self::Symbols => (None, Some(&SYMBOLS_DEFAULT_SCOPES)),
            Self::Classes(classes) => return classes.query(capabilities),
        };
        let mut query = SymbolQuery::new();
        if let Some(kinds) = selection.kinds_within(command_kinds) {
            query = query.kinds(kinds);
        }
        if let Some(scopes) = selection.scopes_within(default_scopes) {
            query = query.scopes(scopes);
        }
        query = selection.shape_predicates(selection.identity_predicates(query));
        for pattern in &selection.containers {
            query = query.in_container(pattern.clone());
        }
        query.capabilities(capabilities)
    }
}

/// How the result is written: the one active output mode.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReportFormat {
    Terminal { render: RenderOptions, view: View },
    /// The selected records of each file (`--json`).
    Json,
    /// Every file's complete index, diagnostics included (`--json-full`).
    JsonFull,
    /// One line per selected record (`--jsonl`).
    JsonLines,
    /// The number of selected records (`--count`).
    Count,
    /// `--stats`: as terminal tables written with the options, or as JSON
    /// (`None`).
    Stats(Option<RenderOptions>),
    /// A terminal-independent declaration export; `tree` is `--tree`
    /// (Markdown and Prose only).
    Export { format: ExportFormat, signatures: crate::output::SignatureMode, plain: bool, comments: bool, tree: bool },
}

impl ReportFormat {
    /// The capabilities this format shows beyond the declarations: records
    /// written as JSON always carry computed `is_test` and `is_deprecated`
    /// (so a `false` there means "not set", never "not computed"), `--stats`
    /// needs `has_docs`, and `--tree` nests by containment, which ctags
    /// also reads for its container fields.
    fn capabilities(self) -> Vec<Capability> {
        match self {
            Self::Json | Self::JsonFull | Self::JsonLines => vec![Capability::Flags],
            Self::Stats(_) => vec![Capability::Metrics],
            Self::Terminal { view: View::Tree, .. } => vec![Capability::Containment],
            Self::Export { tree: true, .. } | Self::Export { format: ExportFormat::Ctags, .. } => {
                vec![Capability::Containment]
            }
            Self::Terminal { .. } | Self::Export { .. } | Self::Count => Vec::new(),
        }
    }
}

/// One run of a declaration command.
pub(crate) struct DeclarationScan {
    pub(crate) command: Declarations,
    pub(crate) files: Vec<PathBuf>,
    /// The resolved scan root: JSON `root_dir` and declaration-path anchor.
    /// Stats package paths use the captured catalog root instead.
    pub(crate) scan_root: PathBuf,
    /// Anchor of terminal declaration paths and listing package sections.
    pub(crate) git_root: PathBuf,
    /// The record selectors, `--exported` included.
    pub(crate) selection: Selection,
    /// `--prelude`: keep only what the prelude module exports.
    pub(crate) prelude: bool,
    pub(crate) listing: ListingOptions,
    pub(crate) format: ReportFormat,
    pub(crate) verbose: bool,
}

/// One file's records after filtering, ready to report.
struct Prepared {
    result: SymbolQueryResult,
    /// `classes` only: the displayed containers.
    classes: Option<Vec<ClassSummary>>,
}

/// One file's outcome, as the scan delivers it in file order. A writer may
/// [`map`](Self::map) the prepared records into its own per-file form on the
/// worker, so delivery only writes.
struct FileOutcome<P = Prepared> {
    /// Portable scan-root-relative path.
    display: String,
    cache: Option<CacheUse>,
    prepared: Result<P, TreeHuggerError>,
}

impl<P> FileOutcome<P> {
    fn map<Q>(self, f: impl FnOnce(&str, P) -> Q) -> FileOutcome<Q> {
        let Self { display, cache, prepared } = self;
        let prepared = prepared.map(|prepared| f(&display, prepared));
        FileOutcome { display, cache, prepared }
    }
}

/// Runs `scan` and returns the process exit status: `0`, or `1` when a file
/// failed or stdout could not be written. A closed stdout (broken pipe) ends
/// the run quietly with `0`.
pub(crate) fn run(scan: DeclarationScan, analyzer: &Analyzer) -> Result<i32, TreeHuggerError> {
    let catalog = match Catalog::capture(scan.format, &scan.scan_root) {
        Ok(catalog) => catalog,
        Err(message) => {
            let _ = writeln!(io::stderr(), "error: {}", neutralized(&message));
            return Ok(1);
        }
    };

    // The scan root prefixes each displayed qualified name after the cache
    // lookup; it never enters an entry or an ID.
    let query = match scan.format {
        ReportFormat::JsonFull => SymbolQuery::complete_index(),
        format => scan.command.query(format, &scan.selection),
    }
    .scan_root(&scan.scan_root);
    let mut files = scan.files.clone();
    let mut stats = CacheStats::new(analyzer.uses_cache());
    let mut stderr = io::stderr();

    // Phase one of a prelude run: resolve the prelude against unrestricted
    // (every-kind) analyses, kept for the scan to narrow instead of parsing
    // again.
    let mut reusable: HashMap<PathBuf, Analyzed> = HashMap::new();
    let prelude = if scan.prelude {
        let wide = SymbolQuery::new().capabilities(query.required_capabilities());
        let filter = resolve_prelude_symbols(&scan.scan_root, analyzer, &wide, &mut reusable)?;
        for analyzed in reusable.values() {
            stats.record("prelude", &analyzed.cache, false, &mut stderr);
        }
        if matches!(scan.command, Declarations::Symbols) {
            for prelude_file in filter.exports_by_file.keys() {
                if !files.contains(prelude_file) {
                    files.push(prelude_file.clone());
                }
            }
        }
        Some(filter)
    } else {
        None
    };

    // Projected once per file: a path outside the scan root is resolved
    // through the filesystem.
    let mut keyed: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|file| (portable_relative(&file, &scan.scan_root), file))
        .collect();
    keyed.sort_by(|(left, _), (right, _)| portable_path_cmp(left, right));
    let mut files: Vec<PathBuf> = keyed.into_iter().map(|(_, file)| file).collect();
    files.dedup();

    // Each scanned file is listed once, so its reused analysis is taken, not
    // copied: the prelude filter above was its last other reader.
    let reusable = Mutex::new(reusable);
    let work = |file: &Path| -> FileOutcome {
        let display = portable_relative(file, &scan.scan_root);
        let reused = reusable.lock().unwrap_or_else(PoisonError::into_inner).remove(file);
        let analyzed = match reused {
            Some(analyzed) => analyzed
                .result
                .narrowed(&query)
                .map(|result| Analyzed {
                    result,
                    cache: CacheUse::Reused,
                })
                .ok_or(())
                .or_else(|()| analyzer.analyze(file, &query)),
            None => analyzer.analyze(file, &query),
        };
        match analyzed {
            Ok(analyzed) => FileOutcome {
                display,
                cache: Some(analyzed.cache),
                prepared: Ok(prepare(&scan, prelude.as_ref(), &query, analyzed.result)),
            },
            Err(error) => FileOutcome {
                display,
                cache: None,
                prepared: Err(error),
            },
        }
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut errors: Vec<ReportError> = Vec::new();
    let mut sink = Sink {
        out: &mut out,
        errors: &mut errors,
        stats: &mut stats,
        stderr: &mut stderr,
    };
    let written = match scan.format {
        ReportFormat::Json | ReportFormat::JsonFull => write_json(&scan, &files, work, &mut sink),
        ReportFormat::JsonLines => write_json_lines(&scan, &files, work, &mut sink),
        ReportFormat::Count | ReportFormat::Stats(_) => write_tally(&scan, &files, work, catalog.as_ref(), &mut sink),
        ReportFormat::Terminal { render, view } => {
            let renderer = Renderer::new(render, Some(scan.git_root.clone()));
            match view {
                View::List => write_listing(&scan, &renderer, &files, work, &mut sink, Presentation::Terminal),
                View::Table => write_table(&scan, &renderer, &files, work, &mut sink),
                View::Tree => write_tree(&scan, &renderer, &files, work, &mut sink, Presentation::Terminal),
            }
        }
        ReportFormat::Export {
            format: format @ (ExportFormat::Prose | ExportFormat::Markdown),
            signatures,
            plain,
            comments,
            tree,
        } => {
            let renderer = Renderer::for_export(comments, signatures, Some(scan.git_root.clone()));
            let presentation = match format {
                ExportFormat::Prose => Presentation::Prose { plain },
                _ => Presentation::Markdown,
            };
            if tree {
                write_tree(&scan, &renderer, &files, work, &mut sink, presentation)
            } else {
                write_listing(&scan, &renderer, &files, work, &mut sink, presentation)
            }
        }
        format @ ReportFormat::Export { .. } => write_export(&scan, &files, work, format, &mut sink),
    };

    if scan.verbose {
        let _ = writeln!(stderr, "{}", stats.summary());
    }
    Ok(write_status(written, errors.is_empty(), &mut stderr))
}

pub(crate) fn write_status(written: io::Result<()>, no_file_errors: bool, stderr: &mut dyn Write) -> i32 {
    match written {
        Ok(()) => i32::from(!no_file_errors),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot write output: {error}");
            1
        }
    }
}

/// The TSV and ctags exports: one file's flattened records at a time,
/// settled in file order.
fn write_export(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    format: ReportFormat,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let is_tsv = matches!(format, ReportFormat::Export { format: ExportFormat::Tsv, .. });
    let is_ctags = matches!(format, ReportFormat::Export { format: ExportFormat::Ctags, .. });
    if is_tsv {
        sink.out.write_all(crate::tsv::HEADER.as_bytes())?;
    }
    if is_ctags {
        sink.out.write_all(crate::ctags::HEADER.as_bytes())?;
    }
    let flow = in_file_order(files, work, |_, outcome, flush| {
        if let Some((display, prepared)) = sink.settle(outcome, scan.verbose) {
            let ReportFormat::Export { format, signatures, .. } = format else { unreachable!() };
            // Tags decide each path from the native record (see
            // `ctags::write_record`); TSV shows the projected one.
            let selected = if format == ExportFormat::Ctags {
                native_selected_records(scan, &display, prepared)
            } else {
                selected_records(scan, &display, prepared)
            };
            let rows = crate::export::flatten_selected(
                &selected.records,
                selected.members.as_deref(),
                scan.listing.sort,
                format != ExportFormat::Ctags,
            );
            if format == ExportFormat::Tsv
                && let Err(error) = crate::tsv::write_rows(&mut *sink.out, &rows, signatures)
            {
                return as_flow(Err(error));
            }
            if format == ExportFormat::Ctags {
                for record in &rows {
                    match crate::ctags::write_record(&mut *sink.out, record, &scan.scan_root) {
                        Ok(Some((path, line, reason))) => {
                            sink.errors.push(skipped_tag(path, line, reason, &mut *sink.stderr));
                        }
                        Ok(None) => {}
                        Err(error) => return as_flow(Err(error)),
                    }
                }
            }
        }
        as_flow(flush.apply(&mut *sink.out))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    sink.out.flush()
}

/// Warns on `stderr` that the tag declared at `path`:`line` was skipped.
/// The path may itself hold the controls ctags rejected, so the warning
/// shows them as visible escapes and stays one line.
fn skipped_tag(path: String, line: usize, reason: crate::ctags::Unsupported, stderr: &mut dyn Write) -> ReportError {
    let message = format!("ctags skipped {} in {path} at line {line}", reason.describe());
    let _ = writeln!(stderr, "warning: {}", neutralized(&message));
    ReportError { file: path, kind: "ctags_unsupported", message }
}

/// Where a writer puts its output, failures and cache counts.
struct Sink<'a> {
    out: &'a mut dyn Write,
    errors: &'a mut Vec<ReportError>,
    stats: &'a mut CacheStats,
    stderr: &'a mut dyn Write,
}

impl Sink<'_> {
    fn settle<P>(&mut self, outcome: FileOutcome<P>, verbose: bool) -> Option<(String, P)> {
        settle(outcome, self.errors, self.stats, verbose, self.stderr)
    }
}

/// Records `outcome`'s cache use and, for a failed file, reports the error
/// on stderr and in `errors`. Returns the prepared records of a good file.
fn settle<P>(
    outcome: FileOutcome<P>,
    errors: &mut Vec<ReportError>,
    stats: &mut CacheStats,
    verbose: bool,
    stderr: &mut dyn Write,
) -> Option<(String, P)> {
    if let Some(cache) = &outcome.cache {
        stats.record(&outcome.display, cache, verbose, stderr);
    }
    match outcome.prepared {
        Ok(prepared) => Some((outcome.display, prepared)),
        Err(error) => {
            let _ = writeln!(stderr, "error: {}", neutralized(&error.to_string()));
            errors.push(ReportError::new(outcome.display, &error));
            None
        }
    }
}

fn write_json(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let full = matches!(scan.format, ReportFormat::JsonFull);
    let entries = if full { Entries::Indexes } else { Entries::Files };
    let root_dir = biscuit_file::to_portable_string(&scan.scan_root);
    let Sink {
        out,
        errors,
        stats,
        stderr,
    } = sink;
    let mut report = JsonReport::begin(&mut **out, &root_dir, entries)?;

    // Each entry is projected, ordered and serialized on its worker; `None`
    // is a file with no selected record, which has no `--json` entry.
    let work = |file: &Path| {
        work(file).map(|display, prepared| -> io::Result<Option<Vec<u8>>> {
            if full {
                let mut index = prepared.result.index;
                relativize_index(&mut index, &scan.scan_root);
                sort_file_records(&mut index.symbols, display, scan.listing.sort);
                return serialized(&index).map(Some);
            }
            let selected = selected_records(scan, display, prepared);
            if selected.records.is_empty() {
                return Ok(None);
            }
            serialized(&FileEntry {
                file: display,
                language: selected.language,
                completed_passes: &selected.completed_passes,
                completed_capabilities: &selected.completed_capabilities,
                symbols: &selected.records,
                members: selected.members_by_id().as_ref(),
            })
            .map(Some)
        })
    };
    let flow = in_file_order(files, work, |_, outcome, flush| {
        let written = match settle(outcome, errors, stats, scan.verbose, &mut **stderr) {
            Some((_, Ok(Some(entry)))) => report.entry(&entry),
            Some((_, Err(error))) => Err(error),
            Some((_, Ok(None))) | None => Ok(()),
        };
        as_flow(written.and_then(|()| report.flush(flush)))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    report.finish(errors)
}

/// One file's selected records as `--json`, `--jsonl` and `--format table`
/// write them: ordered by
/// `--sort`, with portable paths and no source blobs.
struct SelectedRecords {
    language: ProgrammingLanguage,
    completed_passes: Vec<AnalysisPass>,
    completed_capabilities: Vec<Capability>,
    /// The records, or for `classes` the selected containers.
    records: Vec<SymbolRecord>,
    /// `classes` only: each container's displayed members, by position in
    /// `records`.
    members: Option<Vec<Vec<SymbolRecord>>>,
}

impl SelectedRecords {
    /// The `--json` class report's `members` map, keyed by container ID.
    fn members_by_id(&self) -> Option<BTreeMap<String, Vec<SymbolRecord>>> {
        self.members.as_ref().map(|members| {
            self.records
                .iter()
                .zip(members)
                .map(|(container, members)| (container.id.0.clone(), members.clone()))
                .collect()
        })
    }
}

fn selected_records(scan: &DeclarationScan, display: &str, prepared: Prepared) -> SelectedRecords {
    let mut selected = native_selected_records(scan, display, prepared);
    for record in selected.records.iter_mut().chain(selected.members.iter_mut().flatten().flatten()) {
        trim_record(record, &scan.scan_root);
    }
    selected
}

/// [`selected_records`] before `trim_record`: the same records in the same
/// order, still carrying their native paths. For a writer whose output must
/// lead back to the declaration file, which the display projection cannot
/// promise for a name that is not valid Unicode.
fn native_selected_records(scan: &DeclarationScan, display: &str, prepared: Prepared) -> SelectedRecords {
    let Prepared { result, classes } = prepared;
    let index = result.index;
    let (records, members) = match classes {
        Some(mut summaries) => {
            summaries.sort_by(|left, right| {
                compare(scan.listing.sort, (display, &left.container), (display, &right.container))
            });
            let mut containers = Vec::with_capacity(summaries.len());
            let mut members = Vec::with_capacity(summaries.len());
            for summary in summaries {
                members.push(summary.displayed_members());
                containers.push(summary.container);
            }
            (containers, Some(members))
        }
        None => {
            let mut records = index.symbols;
            sort_file_records(&mut records, display, scan.listing.sort);
            (records, None)
        }
    };
    SelectedRecords {
        language: index.language,
        completed_passes: index.completed_passes,
        completed_capabilities: result.completed_capabilities,
        records,
        members,
    }
}

/// Writes one JSON Lines envelope per selected record, files in path order
/// and records in `--sort` order within each file, flushing as
/// [`in_file_order`] asks. A failed file is reported on stderr only: a line
/// stream has no trailing `errors`.
fn write_json_lines(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    // Each file's lines are projected, ordered and serialized on its worker.
    let work = |file: &Path| {
        work(file).map(|display, prepared| -> io::Result<Vec<u8>> {
            let selected = selected_records(scan, display, prepared);
            let mut lines = Vec::new();
            for (position, record) in selected.records.iter().enumerate() {
                let line = JsonLine {
                    report_schema_version: REPORT_SCHEMA_VERSION,
                    schema_version: SchemaVersion::CURRENT,
                    file: display,
                    language: selected.language,
                    completed_passes: &selected.completed_passes,
                    completed_capabilities: &selected.completed_capabilities,
                    record,
                    members: selected.members.as_ref().map(|members| members[position].as_slice()),
                };
                line.write(&mut lines)?;
            }
            Ok(lines)
        })
    };
    let flow = in_file_order(files, work, |_, outcome, flush| {
        let written = match sink.settle(outcome, scan.verbose) {
            Some((_, lines)) => lines.and_then(|lines| sink.out.write_all(&lines)),
            None => Ok(()),
        };
        as_flow(written.and_then(|()| flush.apply(&mut *sink.out)))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    sink.out.flush()
}

struct Tally {
    count: u64,
    stats: Stats,
    groups: Vec<Group>,
}

fn tally(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    catalog: Option<&Catalog>,
    sink: &mut Sink<'_>,
) -> Tally {
    let mut count: u64 = 0;
    let mut stats = Stats::default();
    let mut grouping = catalog.map(Grouping::new);
    // Counted before anything is written, so no flush is due.
    let _ = in_file_order::<_, ()>(files, work, |file, outcome, flush| {
        let group = grouping.as_mut().map(|grouping| grouping.select_file(file));
        if let Some((_, prepared)) = sink.settle(outcome, scan.verbose) {
            let records = prepared.classes.iter().flatten().map(|summary| &summary.container)
                .chain(prepared.result.index.symbols.iter().filter(|_| prepared.classes.is_none()));
            let mut group = group;
            for record in records {
                count += 1;
                stats.add(record);
                if let Some(group) = group.as_deref_mut() {
                    group.add(record);
                }
            }
        }
        ControlFlow::Continue(flush.nothing_written())
    });
    Tally { count, stats, groups: grouping.map_or_else(Vec::new, Grouping::finish) }
}

/// Aggregates selected records (containers for classes) with only counters
/// retained. Native analysis paths attribute synthetic exports to their
/// identity file, even when their source locations point to another package.
fn write_tally(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    catalog: Option<&Catalog>,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let tally = tally(scan, files, work, catalog, sink);
    match scan.format {
        ReportFormat::Stats(None) => {
            let root_dir = biscuit_file::to_portable_string(&scan.scan_root);
            let package_root_dir = biscuit_file::to_portable_string(&catalog.expect("stats captures a catalog").root);
            tally.stats.write_json(&mut *sink.out, &root_dir, &package_root_dir, &tally.groups, sink.errors)
        }
        ReportFormat::Stats(Some(render)) => {
            let renderer = Renderer::new(render, None);
            tally.stats.write_grouped_terminal(&mut *sink.out, renderer.terminal(), !render.styled, &tally.groups)
        }
        _ => {
            writeln!(sink.out, "{}", tally.count)?;
            sink.out.flush()
        }
    }
}

/// How a listing, outline or `classes` report is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presentation {
    Terminal,
    /// `--format prose`; `plain` is global `--plain`.
    Prose { plain: bool },
    Markdown,
}

impl Presentation {
    fn layout<'a>(self, renderer: &'a Renderer, out: &'a mut dyn Write) -> Box<dyn Layout + 'a> {
        match self {
            Self::Terminal => Box::new(TerminalLayout::new(renderer, out)),
            Self::Prose { plain } => Box::new(ProseLayout::new(out, plain)),
            Self::Markdown => Box::new(MarkdownLayout::new(out)),
        }
    }
}

/// Writes the listing (or the `classes` report) as results arrive, flushing
/// as [`in_file_order`] asks, so a closed or failing stdout stops the scan
/// within one window.
fn write_listing(
    scan: &DeclarationScan,
    renderer: &Renderer,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
    presentation: Presentation,
) -> io::Result<()> {
    let Sink { out, errors, stats, stderr } = sink;
    let mut layout = presentation.layout(renderer, &mut **out);
    if matches!(scan.command, Declarations::Classes { .. }) {
        let mut wrote_any = false;
        let flow = in_file_order(files, work, |file, outcome, flush| {
                let mut written = Ok(());
            if let Some((_, prepared)) = settle(outcome, errors, stats, scan.verbose, &mut **stderr) {
                written = write_classes(scan, renderer, &mut *layout, file, prepared)
                    .map(|wrote| wrote_any |= wrote);
            }
            as_flow(written.and_then(|()| flush.apply(layout.writer())))
        });
        if let ControlFlow::Break(error) = flow {
            return Err(error);
        }
        // The terminal and Prose `classes` reports of an empty selection are
        // empty; a Markdown document says so.
        if !wrote_any && presentation == Presentation::Markdown {
            layout.note(0, NO_SYMBOLS)?;
        }
        return layout.writer().flush();
    }

    // Files are analyzed in listing order (package, directory, path), which
    // can differ from the scan order, so each unit is complete on arrival.
    let (ordered, packages) = listing_order(renderer, files, &scan.git_root, scan.listing);
    let (package_roots, ordered): (Vec<PathBuf>, Vec<PathBuf>) = ordered.into_iter().unzip();
    let per_package = scan.prelude;
    let mut stream = ListingStream::new(renderer, scan.listing, packages, per_package);
    // A unit of one file is rendered on the worker that analyzed it, unless
    // the layout carries state from one unit to the next (Markdown's blank
    // lines between blocks).
    let on_workers = stream.units_are_files() && presentation != Presentation::Markdown;
    let work = |file: &Path| {
        work(file).map(|_, prepared| {
            let index = prepared.result.index;
            let listing = FileListing::new(index.file, index.symbols);
            if !on_workers || listing.records.is_empty() {
                return listing;
            }
            let mut bytes = Vec::new();
            renderer
                .package_listing(&mut *presentation.layout(renderer, &mut bytes), &[&listing], scan.listing)
                .expect("rendering into memory cannot fail");
            FileListing::rendered(listing.file, bytes)
        })
    };
    let mut delivered = 0;
    let flow = in_file_order(&ordered, work, |_, outcome, flush| {
        let package_root = &package_roots[delivered];
        delivered += 1;
        let mut written = Ok(());
        if let Some((_, listing)) = settle(outcome, errors, stats, scan.verbose, &mut **stderr) {
            written = stream.push(&mut *layout, package_root, listing);
        }
        as_flow(written.and_then(|()| flush.apply(layout.writer())))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    stream.finish(&mut *layout)?;
    layout.writer().flush()
}

/// `--format table`: one table per file with a selection, files in path
/// order and rows in `--sort` order (for `classes`, each container followed
/// by its displayed members). A table per file keeps the output streaming.
fn write_table(
    scan: &DeclarationScan,
    renderer: &Renderer,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let mut wrote_any = false;
    let flow = in_file_order(files, work, |_, outcome, flush| {
        let mut written = Ok(());
        if let Some((display, prepared)) = sink.settle(outcome, scan.verbose) {
            // The Location link must reach the declaration file itself, so
            // the table projects each displayed path from the native record.
            let selected = native_selected_records(scan, &display, prepared);
            let mut rows: Vec<&SymbolRecord> = Vec::new();
            for (position, record) in selected.records.iter().enumerate() {
                rows.push(record);
                if let Some(members) = &selected.members {
                    rows.extend(&members[position]);
                }
            }
            wrote_any |= !rows.is_empty();
            written = renderer.table(&mut *sink.out, &rows, &scan.scan_root);
        }
        as_flow(written.and_then(|()| flush.apply(&mut *sink.out)))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    if !wrote_any {
        renderer.no_symbols(&mut *sink.out)?;
    }
    sink.out.flush()
}

/// `--tree`: each file's outline, files in path order; with
/// `--group-by-module`, a directory heading where the directory changes.
fn write_tree(
    scan: &DeclarationScan,
    renderer: &Renderer,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
    presentation: Presentation,
) -> io::Result<()> {
    let mut wrote_any = false;
    let mut module: Option<String> = None;
    let Sink { out, errors, stats, stderr } = sink;
    let mut layout = presentation.layout(renderer, &mut **out);
    let flow = in_file_order(files, work, |file, outcome, flush| {
        let mut written = Ok(());
        if let Some((_, prepared)) = settle(outcome, errors, stats, scan.verbose, &mut **stderr) {
            let name_prefix = prepared.result.name_prefix().to_string();
            let records = prepared.result.index.symbols;
            if scan.listing.group_by_module && !records.is_empty() {
                let directory = renderer.module_of(file);
                if module.as_ref() != Some(&directory) {
                    written = renderer.module_heading(&mut *layout, file);
                    module = Some(directory);
                }
            }
            written = written.and_then(|()| {
                let wrote = renderer.outline(
                    &mut *layout,
                    file,
                    &records,
                    &name_prefix,
                    scan.listing.sort,
                    scan.listing.group_by_module,
                )?;
                wrote_any |= wrote;
                Ok(())
            });
        }
        as_flow(written.and_then(|()| flush.apply(layout.writer())))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    if !wrote_any {
        layout.note(0, NO_SYMBOLS)?;
    }
    layout.writer().flush()
}


fn as_flow(result: io::Result<Flushed>) -> ControlFlow<io::Error, Flushed> {
    match result {
        Ok(flushed) => ControlFlow::Continue(flushed),
        Err(error) => ControlFlow::Break(error),
    }
}

/// Writes one file's `classes` report; a file without containers has none.
/// Returns whether anything was written.
fn write_classes(
    scan: &DeclarationScan,
    renderer: &Renderer,
    out: &mut dyn Layout,
    file: &Path,
    prepared: Prepared,
) -> io::Result<bool> {
    let Some(mut summaries) = prepared.classes.filter(|summaries| !summaries.is_empty()) else {
        return Ok(false);
    };
    let display = renderer.display(file);
    summaries.sort_by(|left, right| {
        compare(
            scan.listing.sort,
            (&display, &left.container),
            (&display, &right.container),
        )
    });
    renderer.classes(out, file, prepared.result.index.language, &summaries)?;
    Ok(true)
}

/// Applies `--prelude` to one file's result, whose records already passed
/// the query's selectors. `classes` summarizes the selected containers.
fn prepare(
    scan: &DeclarationScan,
    prelude: Option<&PreludeFilter>,
    query: &SymbolQuery,
    mut result: SymbolQueryResult,
) -> Prepared {
    if matches!(scan.format, ReportFormat::JsonFull) {
        // Symbol selectors are rejected with `--json-full`: the index is
        // complete.
        return Prepared {
            result,
            classes: None,
        };
    }

    if let Declarations::Classes(selection) = &scan.command {
        let mut summaries = classes::summarize(&result.index.symbols, selection);
        summaries.retain(|summary| in_prelude(prelude, &summary.container));
        return Prepared {
            result,
            classes: Some(summaries),
        };
    }

    let Some(filter) = prelude else {
        return Prepared {
            result,
            classes: None,
        };
    };
    let name_prefix = result.name_prefix().to_string();
    let index = &mut result.index;
    match filter.exports_by_file.get(&index.file) {
        // A prelude file lists its re-exports, not its own records, named
        // like the file's own records. They are synthetic, so the selectors
        // the library applied to the file's records apply to them here.
        Some(exports) if matches!(scan.command, Declarations::Symbols) => {
            index.symbols = exports.clone();
            for export in &mut index.symbols {
                export.identity.qualified_name.insert_str(0, &name_prefix);
            }
            index
                .symbols
                .retain(|export| query.selects(export.kind) && query.matches(export));
        }
        _ if matches!(scan.command, Declarations::Symbols) => {
            let names = if filter.exports_by_file.is_empty() {
                &filter.names
            } else {
                &filter.env_only_names
            };
            index.symbols.retain(|record| names.contains(&record.identity.name));
        }
        _ => index.symbols.retain(|record| in_prelude(prelude, record)),
    }
    Prepared {
        result,
        classes: None,
    }
}

/// Whether `record` is kept under `--prelude` (always, without it).
fn in_prelude(prelude: Option<&PreludeFilter>, record: &SymbolRecord) -> bool {
    prelude.is_none_or(|filter| filter.names.contains(&record.identity.name))
}

#[cfg(test)]
#[path = "declarations_grouping_tests.rs"]
mod grouping_tests;


#[cfg(test)]
mod export_write_tests {
    use super::*;

    #[test]
    fn exports_do_no_extra_analysis_beyond_equivalent_listings() {
        let scratch = tempfile::tempdir().expect("scratch");
        let file = scratch.path().join("sample.py");
        std::fs::write(&file, "class Sample:\n    def method(self, value):\n        if value:\n            return self.method(False)\n        return 0\n").expect("source");
        let selection = Selection::default();
        let analyzer = Analyzer::new(None, None, false);
        let measure = |query: &SymbolQuery| {
            let before = tree_hugger::work_counters();
            let result = analyzer.analyze(&file, query).expect("analyze").result;
            assert!(!result.index.symbols.is_empty(), "fixture must exercise declarations");
            (tree_hugger::work_counters().since(before), result)
        };
        for command in [
            Declarations::Functions,
            Declarations::Types,
            Declarations::Symbols,
            Declarations::Classes(Box::new(ClassSelection::new(&selection, Default::default()))),
        ] {
            let listing = command.query(ReportFormat::Count, &selection);
            let (flat_work, _) = measure(&listing);
            let owned = listing.clone().capabilities([Capability::Containment]);
            let (owned_work, _) = measure(&owned);
            for format in [ExportFormat::Tsv, ExportFormat::Markdown, ExportFormat::Prose, ExportFormat::Ctags] {
                for tree in [false, true] {
                    if tree && matches!(format, ExportFormat::Tsv | ExportFormat::Ctags) { continue; }
                    let containment = format == ExportFormat::Ctags || tree;
                    let query = command.query(ReportFormat::Export {
                        format,
                        signatures: crate::output::SignatureMode::Short,
                        plain: false,
                        comments: false,
                        tree,
                    }, &selection);
                    let expected_containment = containment || matches!(command, Declarations::Classes(_));
                    assert_eq!(query.required_capabilities(), if expected_containment {
                        vec![Capability::Declarations, Capability::Containment]
                    } else { vec![Capability::Declarations] }, "{command:?}, {format:?}, tree={tree}");
                    let (work, result) = measure(&query);
                    assert_eq!(work, if expected_containment { owned_work } else { flat_work },
                        "{command:?}, {format:?}, tree={tree}");
                    assert_eq!(work.parser_constructions, 1);
                    assert_eq!(work.definition_captures, 1);
                    assert_eq!(work.reference_captures, 0);
                    assert_eq!(work.occurrence_resolutions, 0);
                    assert_eq!(work.diagnostics_runs, 0);
                    assert_eq!(work.complexity_walks, 0);
                    assert!(!result.index.completed_passes.contains(&AnalysisPass::Semantic));
                }
            }
        }
    }

    #[test]
    fn skipped_tag_warning_shows_path_controls_as_one_visible_line() {
        let mut stderr = Vec::new();
        let error = skipped_tag(
            "a\n\n        b\u{9b}31m\u{1b}[0m\r.java".to_string(),
            3,
            crate::ctags::Unsupported::Control,
            &mut stderr,
        );
        let stderr = String::from_utf8(stderr).expect("utf-8");
        assert_eq!(
            stderr,
            "warning: ctags skipped unsupported control character in \
             a\\u{a}\\u{a}        b\\u{9b}31m\\u{1b}[0m\\u{d}.java at line 3\n"
        );
        assert_eq!(error.kind, "ctags_unsupported");
        assert_eq!(error.file, "a\n\n        b\u{9b}31m\u{1b}[0m\r.java");
    }

    #[test]
    fn export_write_errors_preserve_broken_pipe_and_other_failure_statuses() {
        let mut stderr = Vec::new();
        assert_eq!(
            write_status(Err(io::Error::from(io::ErrorKind::BrokenPipe)), true, &mut stderr),
            0
        );
        assert!(stderr.is_empty());

        assert_eq!(
            write_status(Err(io::Error::other("disk full")), true, &mut stderr),
            1
        );
        assert!(String::from_utf8(stderr).expect("stderr").contains("cannot write output: disk full"));
        assert_eq!(write_status(Ok(()), false, &mut Vec::new()), 1);
        assert_eq!(write_status(Ok(()), true, &mut Vec::new()), 0);
    }

    #[test]
    fn export_capabilities_add_only_requested_containment() {
        let flat = ReportFormat::Export {
            format: ExportFormat::Tsv,
            signatures: crate::output::SignatureMode::Short,
            plain: false,
            comments: false,
            tree: false,
        };
        let owned = ReportFormat::Export {
            format: ExportFormat::Ctags,
            signatures: crate::output::SignatureMode::Short,
            plain: false,
            comments: false,
            tree: false,
        };
        assert!(flat.capabilities().is_empty());
        assert_eq!(owned.capabilities(), [Capability::Containment]);
    }
}
