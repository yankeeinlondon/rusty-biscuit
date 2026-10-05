//! The declaration commands (`functions`, `types`, `symbols`, `classes`):
//! each asks the library for its fixed kind set and only the capabilities it
//! shows, analyzes the files in parallel, filters and orders the records,
//! and writes the JSON report or the terminal listing.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use tree_hugger::{
    Capability, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolQueryResult, SymbolRecord,
    TreeHuggerError,
};

use crate::classes::{self, ClassSummary, MemberFilter};
use crate::order::{compare, portable_path_cmp, sort_file_records};
use crate::prelude::{PreludeFilter, resolve_prelude_symbols};
use crate::render::{FileListing, ListingOptions, Renderer};
use crate::report::{
    Entries, FileEntry, JsonReport, ReportError, portable_relative, relativize_index, trim_record,
};
use crate::scan::{Analyzed, Analyzer, CacheStats, CacheUse, in_file_order};
use crate::scanner::matches_symbol_filters;

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

/// Which declaration command runs.
#[derive(Debug, Clone)]
pub(crate) enum Declarations {
    Functions,
    Types,
    Symbols,
    Classes {
        /// `--name`: keep containers whose name contains this.
        name_filter: Option<String>,
        members: MemberFilter,
    },
}

impl Declarations {
    /// The command's fixed kind set and the capabilities it shows: only
    /// `classes` needs containment, and no listing runs references,
    /// semantics, structured docs or diagnostics.
    pub(crate) fn query(&self) -> SymbolQuery {
        match self {
            Self::Functions => SymbolQuery::new().kinds(FUNCTION_KINDS),
            Self::Types => SymbolQuery::new().kinds(TYPE_KINDS),
            Self::Symbols => SymbolQuery::new(),
            Self::Classes { .. } => SymbolQuery::new()
                .kinds(classes::CONTAINER_KINDS.into_iter().chain(classes::MEMBER_KINDS))
                .capabilities([Capability::Containment]),
        }
    }
}

/// How the result is written.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReportFormat {
    Terminal { styled: bool, show_comments: bool },
    /// The selected records of each file (`--json`).
    Json,
    /// Every file's complete index, diagnostics included (`--json-full`).
    JsonFull,
}

/// Which declarations a run keeps, decided before the name filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Membership {
    All,
    /// `--exported`.
    Exported,
    /// `--prelude`.
    Prelude,
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
    pub(crate) membership: Membership,
    pub(crate) include_globs: Vec<String>,
    pub(crate) exclude_globs: Vec<String>,
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
    let query = match scan.format {
        ReportFormat::JsonFull => SymbolQuery::complete_index(),
        _ => scan.command.query(),
    };
    let mut files = scan.files.clone();
    let mut stats = CacheStats::new(analyzer.uses_cache());
    let mut stderr = io::stderr();

    // Phase one of a prelude run: resolve the prelude against unrestricted
    // (every-kind) analyses, kept for the scan to narrow instead of parsing
    // again.
    let mut reusable: HashMap<PathBuf, Analyzed> = HashMap::new();
    let prelude = if scan.membership == Membership::Prelude {
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

    files.sort_by(|left, right| {
        portable_path_cmp(
            &portable_relative(left, &scan.scan_root),
            &portable_relative(right, &scan.scan_root),
        )
    });
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
                prepared: Ok(prepare(&scan, prelude.as_ref(), analyzed.result)),
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
    let written = match scan.format {
        ReportFormat::Json | ReportFormat::JsonFull => {
            write_json(&scan, &files, work, &mut out, &mut errors, &mut stats, &mut stderr)
        }
        ReportFormat::Terminal {
            styled,
            show_comments,
        } => {
            let renderer = Renderer::new(styled, show_comments, Some(scan.git_root.clone()));
            write_terminal(&scan, &renderer, &files, work, &mut out, &mut errors, &mut stats, &mut stderr)
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

#[allow(clippy::too_many_arguments)]
fn write_json(
    scan: &DeclarationScan,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    out: &mut dyn Write,
    errors: &mut Vec<ReportError>,
    stats: &mut CacheStats,
    stderr: &mut dyn Write,
) -> io::Result<()> {
    let full = matches!(scan.format, ReportFormat::JsonFull);
    let entries = if full { Entries::Indexes } else { Entries::Files };
    let root_dir = biscuit_file::to_portable_string(&scan.scan_root);
    let mut report = JsonReport::begin(&mut *out, &root_dir, entries)?;

    let flow = in_file_order(files, work, |_, outcome| {
        let Some((display, prepared)) = settle(outcome, errors, stats, scan.verbose, stderr) else {
            return ControlFlow::Continue(());
        };
        let written = if full {
            let mut index = prepared.result.index;
            relativize_index(&mut index, &scan.scan_root);
            report.entry(&index)
        } else {
            json_file_entry(scan, &mut report, &display, prepared)
        };
        match written {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => ControlFlow::Break(error),
        }
    });
    if let ControlFlow::Break(error) = flow {
        return Err(error);
    }
    report.finish(errors)
}

/// Writes one file's `--json` entry; a file with nothing selected has none.
fn json_file_entry(
    scan: &DeclarationScan,
    report: &mut JsonReport<&mut dyn Write>,
    display: &str,
    prepared: Prepared,
) -> io::Result<()> {
    let Prepared { result, classes } = prepared;
    let index = result.index;
    let (mut symbols, members) = match classes {
        Some(summaries) => {
            let mut members = BTreeMap::new();
            let mut containers = Vec::with_capacity(summaries.len());
            for summary in summaries {
                let mut displayed = summary.displayed_members();
                for record in &mut displayed {
                    trim_record(record, &scan.scan_root);
                }
                members.insert(summary.container.id.0.clone(), displayed);
                containers.push(summary.container);
            }
            (containers, Some(members))
        }
        None => (index.symbols, None),
    };
    if symbols.is_empty() {
        return Ok(());
    }
    sort_file_records(&mut symbols, display, scan.listing.sort);
    for record in &mut symbols {
        trim_record(record, &scan.scan_root);
    }
    report.entry(&FileEntry {
        file: display,
        language: index.language,
        completed_passes: &index.completed_passes,
        completed_capabilities: &result.completed_capabilities,
        symbols: &symbols,
        members: members.as_ref(),
    })
}

#[allow(clippy::too_many_arguments)]
fn write_terminal(
    scan: &DeclarationScan,
    renderer: &Renderer,
    files: &[PathBuf],
    work: impl Fn(&Path) -> FileOutcome + Sync,
    out: &mut dyn Write,
    errors: &mut Vec<ReportError>,
    stats: &mut CacheStats,
    stderr: &mut dyn Write,
) -> io::Result<()> {
    let mut listings: Vec<FileListing> = Vec::new();
    let mut class_files: Vec<(PathBuf, ProgrammingLanguage, Vec<ClassSummary>)> = Vec::new();
    let _ = in_file_order::<_, ()>(files, work, |file, outcome| {
        if let Some((_, prepared)) = settle(outcome, errors, stats, scan.verbose, stderr) {
            let index = prepared.result.index;
            match prepared.classes {
                Some(summaries) if !summaries.is_empty() => {
                    class_files.push((file.to_path_buf(), index.language, summaries));
                }
                Some(_) => {}
                None => listings.push(FileListing {
                    file: index.file,
                    records: index.symbols,
                }),
            }
        }
        ControlFlow::Continue(())
    });

    if matches!(scan.command, Declarations::Classes { .. }) {
        for (file, language, mut summaries) in class_files {
            let display = renderer.display(&file);
            summaries.sort_by(|left, right| {
                compare(
                    scan.listing.sort,
                    (&display, &left.container),
                    (&display, &right.container),
                )
            });
            renderer.classes(out, &file, language, &summaries)?;
        }
    } else {
        renderer.listing(out, &listings, &scan.git_root, scan.listing)?;
    }
    out.flush()
}

/// Applies membership (`--exported`, `--prelude`), then the name filters,
/// to one file's result. `classes` summarizes first and filters containers.
fn prepare(scan: &DeclarationScan, prelude: Option<&PreludeFilter>, mut result: SymbolQueryResult) -> Prepared {
    if matches!(scan.format, ReportFormat::JsonFull) {
        // Symbol selectors are rejected with `--json-full`: the index is
        // complete.
        return Prepared {
            result,
            classes: None,
        };
    }
    let keep_name = |name: &str| {
        (scan.include_globs.is_empty() || matches_symbol_filters(name, &scan.include_globs))
            && (scan.exclude_globs.is_empty() || !matches_symbol_filters(name, &scan.exclude_globs))
    };

    if let Declarations::Classes {
        name_filter,
        members,
    } = &scan.command
    {
        let mut summaries = classes::summarize(&result.index.symbols, *members);
        summaries.retain(|summary| {
            let container = &summary.container;
            let name = container.identity.name.as_str();
            name_filter.as_deref().is_none_or(|filter| name.contains(filter))
                && is_member(scan.membership, prelude, container)
                && keep_name(name)
        });
        return Prepared {
            result,
            classes: Some(summaries),
        };
    }

    let index = &mut result.index;
    match (scan.membership, prelude) {
        (Membership::Prelude, Some(filter)) if matches!(scan.command, Declarations::Symbols) => {
            match filter.exports_by_file.get(&index.file) {
                // A prelude file lists its re-exports, not its own records.
                Some(exports) => index.symbols = exports.clone(),
                None => {
                    let names = if filter.exports_by_file.is_empty() {
                        &filter.names
                    } else {
                        &filter.env_only_names
                    };
                    index.symbols.retain(|record| names.contains(&record.identity.name));
                }
            }
        }
        (membership, prelude) => index
            .symbols
            .retain(|record| is_member(membership, prelude, record)),
    }
    index.symbols.retain(|record| keep_name(&record.identity.name));
    Prepared {
        result,
        classes: None,
    }
}

fn is_member(membership: Membership, prelude: Option<&PreludeFilter>, record: &SymbolRecord) -> bool {
    match membership {
        Membership::All => true,
        Membership::Exported => record.visibility.is_exported,
        Membership::Prelude => prelude.is_some_and(|filter| filter.names.contains(&record.identity.name)),
    }
}
