//! Analyzing the scanned files: in parallel on rayon workers, in bounded
//! chunks, with every result delivered in file order so output never depends
//! on scheduling.

use std::io::Write;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tree_hugger::cache::{CacheError, CacheLookup, CacheMiss, SymbolCache};
use tree_hugger::{
    ProgrammingLanguage, SourceFile, SymbolQuery, SymbolQueryResult, TreeHuggerError,
    WorkCounters, work_counters,
};

/// Reads and analyzes one file, through the persistent cache when enabled.
///
/// Shared by every worker: each call reads, hashes and (when needed) parses
/// its own file, so no parser or tree crosses threads.
pub(crate) struct Analyzer {
    cache: Option<SymbolCache>,
    language: Option<ProgrammingLanguage>,
    experimental_semantics: bool,
}

/// How the persistent cache took part in one file's analysis.
#[derive(Debug)]
pub(crate) enum CacheUse {
    /// `--no-cache`.
    Disabled,
    /// Answered from a result this run already computed (prelude reuse).
    Reused,
    /// The cache was consulted.
    Consulted {
        lookup: CacheLookup,
        write_error: Option<CacheError>,
    },
}

/// One file's analysis.
#[derive(Debug)]
pub(crate) struct Analyzed {
    pub(crate) result: SymbolQueryResult,
    pub(crate) cache: CacheUse,
}

impl Analyzer {
    pub(crate) fn new(
        cache: Option<SymbolCache>,
        language: Option<ProgrammingLanguage>,
        experimental_semantics: bool,
    ) -> Self {
        Self {
            cache,
            language,
            experimental_semantics,
        }
    }

    pub(crate) fn uses_cache(&self) -> bool {
        self.cache.is_some()
    }

    /// Runs `query` over `file`. The file is read and hashed once; it is
    /// parsed only when the cache cannot answer. A cache that cannot be read
    /// or written never fails the analysis.
    pub(crate) fn analyze(&self, file: &Path, query: &SymbolQuery) -> Result<Analyzed, TreeHuggerError> {
        let source = SourceFile::read_with_language(file, self.language)?
            .experimental_semantics(self.experimental_semantics);
        match &self.cache {
            Some(cache) => {
                let cached = cache.query(source, query)?;
                Ok(Analyzed {
                    result: cached.result,
                    cache: CacheUse::Consulted {
                        lookup: cached.lookup,
                        write_error: cached.write_error,
                    },
                })
            }
            None => Ok(Analyzed {
                result: source.query(query)?,
                cache: CacheUse::Disabled,
            }),
        }
    }
}

/// Runs `work` over `files` on rayon workers and hands each result to
/// `deliver` in file order, stopping early when `deliver` breaks.
///
/// Files are processed in chunks of four per worker thread, and a chunk is
/// delivered before the next starts, so at most one chunk of results is held
/// at a time.
pub(crate) fn in_file_order<T: Send, B>(
    files: &[PathBuf],
    work: impl Fn(&Path) -> T + Sync,
    mut deliver: impl FnMut(&Path, T) -> ControlFlow<B>,
) -> ControlFlow<B> {
    let chunk_size = 4 * rayon::current_num_threads().max(1);
    for chunk in files.chunks(chunk_size) {
        let results: Vec<T> = chunk.par_iter().map(|file| work(file)).collect();
        for (file, result) in chunk.iter().zip(results) {
            deliver(file, result)?;
        }
    }
    ControlFlow::Continue(())
}

/// What the persistent cache did over a run, for `--verbose`.
#[derive(Debug, Default)]
pub(crate) struct CacheStats {
    enabled: bool,
    hits: usize,
    partial_hits: usize,
    misses: usize,
    reused: usize,
    started: WorkCounters,
}

impl CacheStats {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            started: work_counters(),
            ..Self::default()
        }
    }

    /// Counts one file's cache use, and with `verbose` writes any
    /// recoverable cache failure (an unusable entry, a failed write) for
    /// `display` to `err`.
    pub(crate) fn record(&mut self, display: &str, cache: &CacheUse, verbose: bool, err: &mut dyn Write) {
        match cache {
            CacheUse::Disabled => {}
            CacheUse::Reused => self.reused += 1,
            CacheUse::Consulted {
                lookup,
                write_error,
            } => {
                match lookup {
                    CacheLookup::Hit => self.hits += 1,
                    CacheLookup::WidenedCapabilities { .. } | CacheLookup::WidenedKinds => {
                        self.partial_hits += 1
                    }
                    CacheLookup::Miss(miss) => {
                        self.misses += 1;
                        if verbose && *miss != CacheMiss::Absent {
                            let _ = writeln!(err, "cache: {display}: {miss}");
                        }
                    }
                }
                if verbose && let Some(error) = write_error {
                    let _ = writeln!(err, "cache: {display}: write failed: {error}");
                }
            }
        }
    }

    /// The stats line: persistent-cache hits, partial hits and misses, and
    /// the parsing and analysis work the run did.
    pub(crate) fn summary(&self) -> String {
        let work = work_counters().since(self.started);
        let cache = if self.enabled {
            format!(
                "{} hits, {} partial hits, {} misses",
                self.hits, self.partial_hits, self.misses
            )
        } else {
            "disabled".to_string()
        };
        let reused = if self.reused > 0 {
            format!(", {} reused in this run", self.reused)
        } else {
            String::new()
        };
        format!(
            "cache: {cache}{reused}; parsed {} files, {} definition captures, {} diagnostics runs",
            work.parser_constructions, work.definition_captures, work.diagnostics_runs
        )
    }
}
