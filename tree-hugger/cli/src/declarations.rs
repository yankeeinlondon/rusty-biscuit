//! The declaration commands (`functions`, `types`, `symbols`, `classes`):
//! each asks the library for its fixed kind set narrowed by the record
//! selectors and only the capabilities it shows and filters on, analyzes the
//! files in parallel (the library applies the selectors once, after the
//! cache), orders the records, and writes them in the one output mode: the
//! terminal listing, outline or table, `--json`, `--json-full`, `--jsonl`,
//! `--count` or `--stats`.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use tree_hugger::{
    AnalysisPass, Capability, ProgrammingLanguage, SchemaVersion, SymbolKindV2, SymbolQuery,
    SymbolQueryResult, SymbolRecord, SymbolScope, TreeHuggerError,
};

use crate::classes::{self, ClassSelection, ClassSummary};
use crate::order::{compare, portable_path_cmp, sort_file_records};
use crate::prelude::{PreludeFilter, resolve_prelude_symbols};
use crate::output::View;
use crate::render::{FileListing, ListingOptions, ListingStream, RenderOptions, Renderer, listing_order};
use crate::report::{
    Entries, FileEntry, JsonLine, JsonReport, REPORT_SCHEMA_VERSION, ReportError, portable_relative,
    relativize_index, trim_record,
};
use crate::scan::{Analyzed, Analyzer, CacheStats, CacheUse, chunk_size, flush_at_chunk_end, in_file_order};
use crate::selection::Selection;
use crate::stats::Stats;

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
}

impl ReportFormat {
    /// The capabilities this format shows beyond the declarations: records
    /// written as JSON always carry computed `is_test` and `is_deprecated`
    /// (so a `false` there means "not set", never "not computed"), `--stats`
    /// needs `has_docs`, and `--tree` nests by containment.
    fn capabilities(self) -> Vec<Capability> {
        match self {
            Self::Json | Self::JsonFull | Self::JsonLines => vec![Capability::Flags],
            Self::Stats(_) => vec![Capability::Metrics],
            Self::Terminal { view: View::Tree, .. } => vec![Capability::Containment],
            Self::Terminal { .. } | Self::Count => Vec::new(),
        }
    }
}

/// One run of a declaration command.
pub(crate) struct DeclarationScan {
    pub(crate) command: Declarations,
    pub(crate) files: Vec<PathBuf>,
    /// The resolved scan root: JSON `root_dir`, and the anchor of every JSON
    /// path.
    pub(crate) scan_root: PathBuf,
    /// Anchor of terminal paths and of package grouping.
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

/// One file's outcome, as the scan delivers it in file order.
struct FileOutcome {
    /// Portable scan-root-relative path.
    display: String,
    cache: Option<CacheUse>,
    prepared: Result<Prepared, TreeHuggerError>,
}

/// Runs `scan` and returns the process exit status: `0`, or `1` when a file
/// failed or stdout could not be written. A closed stdout (broken pipe) ends
/// the run quietly with `0`.
pub(crate) fn run(scan: DeclarationScan, analyzer: &Analyzer) -> Result<i32, TreeHuggerError> {
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

    let work = |file: &Path| -> FileOutcome {
        let display = portable_relative(file, &scan.scan_root);
        let analyzed = match reusable.get(file) {
            Some(analyzed) => analyzed
                .result
                .clone()
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
        ReportFormat::Count | ReportFormat::Stats(_) => write_tally(&scan, &files, work, &mut sink),
        ReportFormat::Terminal { render, view } => {
            let renderer = Renderer::new(render, Some(scan.git_root.clone()));
            match view {
                View::List => write_terminal(&scan, &renderer, &files, work, &mut sink),
                View::Table => write_table(&scan, &renderer, &files, work, &mut sink),
                View::Tree => write_tree(&scan, &renderer, &files, work, &mut sink),
            }
        }
    };

    if scan.verbose {
        let _ = writeln!(stderr, "{}", stats.summary());
    }
    match written {
        Ok(()) => Ok(if errors.is_empty() { 0 } else { 1 }),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(0),
        Err(error) => {
            let _ = writeln!(stderr, "error: cannot write output: {error}");
            Ok(1)
        }
    }
}

/// Where a writer puts its output, failures and cache counts.
struct Sink<'a> {
    out: &'a mut dyn Write,
    errors: &'a mut Vec<ReportError>,
    stats: &'a mut CacheStats,
    stderr: &'a mut dyn Write,
}

impl Sink<'_> {
    fn settle(&mut self, outcome: FileOutcome, verbose: bool) -> Option<(String, Prepared)> {
        settle(outcome, self.errors, self.stats, verbose, self.stderr)
    }
}

/// Records `outcome`'s cache use and, for a failed file, reports the error
/// on stderr and in `errors`. Returns the prepared records of a good file.
fn settle(
    outcome: FileOutcome,
    errors: &mut Vec<ReportError>,
    stats: &mut CacheStats,
    verbose: bool,
    stderr: &mut dyn Write,
) -> Option<(String, Prepared)> {
    if let Some(cache) = &outcome.cache {
        stats.record(&outcome.display, cache, verbose, stderr);
    }
    match outcome.prepared {
        Ok(prepared) => Some((outcome.display, prepared)),
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
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

    let chunk = chunk_size();
    let mut delivered = 0;
    let flow = in_file_order(files, work, |_, outcome| {
        delivered += 1;
        let Some((display, prepared)) = settle(outcome, errors, stats, scan.verbose, &mut **stderr) else {
            return as_flow(report.flush_at_chunk_end(delivered, chunk, files.len()));
        };
        let written = if full {
            let mut index = prepared.result.index;
            relativize_index(&mut index, &scan.scan_root);
            sort_file_records(&mut index.symbols, &display, scan.listing.sort);
            report.entry(&index)
        } else {
            let selected = selected_records(scan, &display, prepared);
            if selected.records.is_empty() {
                Ok(())
            } else {
                report.entry(&FileEntry {
                    file: &display,
                    language: selected.language,
                    completed_passes: &selected.completed_passes,
                    completed_capabilities: &selected.completed_capabilities,
                    symbols: &selected.records,
                    members: selected.members_by_id().as_ref(),
                })
            }
        };
        as_flow(written.and_then(|()| report.flush_at_chunk_end(delivered, chunk, files.len())))
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
                let mut displayed = summary.displayed_members();
                for record in &mut displayed {
                    trim_record(record, &scan.scan_root);
                }
                members.push(displayed);
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
    let mut records = records;
    for record in &mut records {
        trim_record(record, &scan.scan_root);
    }
    SelectedRecords {
        language: index.language,
        completed_passes: index.completed_passes,
        completed_capabilities: result.completed_capabilities,
        records,
        members,
    }
}

/// Writes one JSON Lines envelope per selected record, files in path order
/// and records in `--sort` order within each file, flushing after each
/// chunk. A failed file is reported on stderr only: a line stream has no
/// trailing `errors`.
fn write_json_lines(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let chunk = chunk_size();
    let mut delivered = 0;
    let flow = in_file_order(files, work, |_, outcome| {
        delivered += 1;
        let mut written = Ok(());
        if let Some((display, prepared)) = sink.settle(outcome, scan.verbose) {
            let selected = selected_records(scan, &display, prepared);
            for (position, record) in selected.records.iter().enumerate() {
                let line = JsonLine {
                    report_schema_version: REPORT_SCHEMA_VERSION,
                    schema_version: SchemaVersion::CURRENT,
                    file: &display,
                    language: selected.language,
                    completed_passes: &selected.completed_passes,
                    completed_capabilities: &selected.completed_capabilities,
                    record,
                    members: selected.members.as_ref().map(|members| members[position].as_slice()),
                };
                written = line.write(&mut *sink.out);
                if written.is_err() {
                    break;
                }
            }
        }
        as_flow(written.and_then(|()| flush_at_chunk_end(&mut *sink.out, delivered, chunk, files.len())))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    sink.out.flush()
}

/// `--count` and `--stats`: aggregates the selected records (for `classes`,
/// the selected containers) and writes the total or the stats once every
/// file is analyzed. Only the counters are held.
fn write_tally(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let mut count: u64 = 0;
    let mut stats = Stats::default();
    let _ = in_file_order::<_, ()>(files, work, |_, outcome| {
        if let Some((_, prepared)) = sink.settle(outcome, scan.verbose) {
            let records: Vec<&SymbolRecord> = match &prepared.classes {
                Some(summaries) => summaries.iter().map(|summary| &summary.container).collect(),
                None => prepared.result.index.symbols.iter().collect(),
            };
            count += records.len() as u64;
            for record in records {
                stats.add(record);
            }
        }
        ControlFlow::Continue(())
    });
    match scan.format {
        ReportFormat::Stats(None) => {
            let root_dir = biscuit_file::to_portable_string(&scan.scan_root);
            stats.write_json(&mut *sink.out, &root_dir, sink.errors)
        }
        ReportFormat::Stats(Some(render)) => {
            let renderer = Renderer::new(render, None);
            stats.write_terminal(&mut *sink.out, renderer.terminal(), !render.styled)
        }
        _ => {
            writeln!(sink.out, "{count}")?;
            sink.out.flush()
        }
    }
}

/// Writes the terminal report as results arrive, flushing after each chunk,
/// so a closed or failing stdout stops the scan before the next chunk.
fn write_terminal(
    scan: &DeclarationScan,
    renderer: &Renderer,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    sink: &mut Sink<'_>,
) -> io::Result<()> {
    let chunk = chunk_size();
    let mut delivered = 0;
    if matches!(scan.command, Declarations::Classes { .. }) {
        let flow = in_file_order(files, work, |file, outcome| {
            delivered += 1;
            let mut written = Ok(());
            if let Some((_, prepared)) = sink.settle(outcome, scan.verbose) {
                written = write_classes(scan, renderer, &mut *sink.out, file, prepared);
            }
            as_flow(written.and_then(|()| flush_at_chunk_end(&mut *sink.out, delivered, chunk, files.len())))
        });
        if let ControlFlow::Break(error) = flow {
            return Err(error);
        }
        return sink.out.flush();
    }

    // Files are analyzed in listing order (package, directory, path), which
    // can differ from the scan order, so each unit is complete on arrival.
    let (ordered, packages) = listing_order(renderer, files, &scan.git_root, scan.listing);
    let (package_roots, ordered): (Vec<PathBuf>, Vec<PathBuf>) = ordered.into_iter().unzip();
    let per_package = scan.prelude;
    let mut stream = ListingStream::new(renderer, scan.listing, packages, per_package);
    let flow = in_file_order(&ordered, work, |_, outcome| {
        let package_root = &package_roots[delivered];
        delivered += 1;
        let mut written = Ok(());
        if let Some((_, prepared)) = sink.settle(outcome, scan.verbose) {
            let index = prepared.result.index;
            let listing = FileListing {
                file: index.file,
                records: index.symbols,
            };
            written = stream.push(&mut *sink.out, package_root, listing);
        }
        as_flow(written.and_then(|()| flush_at_chunk_end(&mut *sink.out, delivered, chunk, ordered.len())))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    stream.finish(&mut *sink.out)?;
    sink.out.flush()
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
    let chunk = chunk_size();
    let mut delivered = 0;
    let mut wrote_any = false;
    let flow = in_file_order(files, work, |_, outcome| {
        delivered += 1;
        let mut written = Ok(());
        if let Some((display, prepared)) = sink.settle(outcome, scan.verbose) {
            let selected = selected_records(scan, &display, prepared);
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
        as_flow(written.and_then(|()| flush_at_chunk_end(&mut *sink.out, delivered, chunk, files.len())))
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
) -> io::Result<()> {
    let chunk = chunk_size();
    let mut delivered = 0;
    let mut wrote_any = false;
    let mut module: Option<String> = None;
    let flow = in_file_order(files, work, |file, outcome| {
        delivered += 1;
        let mut written = Ok(());
        if let Some((_, prepared)) = sink.settle(outcome, scan.verbose) {
            let name_prefix = prepared.result.name_prefix().to_string();
            let records = prepared.result.index.symbols;
            if scan.listing.group_by_module && !records.is_empty() {
                let directory = renderer.module_of(file);
                if module.as_ref() != Some(&directory) {
                    written = renderer.module_heading(&mut *sink.out, file);
                    module = Some(directory);
                }
            }
            written = written.and_then(|()| {
                let wrote = renderer.outline(
                    &mut *sink.out,
                    file,
                    &records,
                    &name_prefix,
                    scan.listing.sort,
                )?;
                wrote_any |= wrote;
                Ok(())
            });
        }
        as_flow(written.and_then(|()| flush_at_chunk_end(&mut *sink.out, delivered, chunk, files.len())))
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    if !wrote_any {
        renderer.no_symbols(&mut *sink.out)?;
    }
    sink.out.flush()
}

fn as_flow(result: io::Result<()>) -> ControlFlow<io::Error> {
    match result {
        Ok(()) => ControlFlow::Continue(()),
        Err(error) => ControlFlow::Break(error),
    }
}

/// Writes one file's `classes` report; a file without containers has none.
fn write_classes(
    scan: &DeclarationScan,
    renderer: &Renderer,
    out: &mut dyn Write,
    file: &Path,
    prepared: Prepared,
) -> io::Result<()> {
    let Some(mut summaries) = prepared.classes.filter(|summaries| !summaries.is_empty()) else {
        return Ok(());
    };
    let display = renderer.display(file);
    summaries.sort_by(|left, right| {
        compare(
            scan.listing.sort,
            (&display, &left.container),
            (&display, &right.container),
        )
    });
    renderer.classes(out, file, prepared.result.index.language, &summaries)
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
