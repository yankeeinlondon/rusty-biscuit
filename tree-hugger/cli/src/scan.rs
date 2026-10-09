//! Analyzing the scanned files: in parallel on worker threads, inside a
//! bounded sliding window, with every result delivered in file order so
//! output never depends on scheduling.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::ops::ControlFlow;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, PoisonError, mpsc};
use std::thread;

use tree_hugger::cache::{CacheError, CacheLookup, CacheMiss, SymbolCache};
use tree_hugger::{
    ProgrammingLanguage, SourceFile, SymbolQuery, SymbolQueryResult, TreeHuggerError,
    WorkCounters, work_counters,
};

use crate::render::neutralized;

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

    /// Runs `query` over `file`, a file identity (see [`read`](Self::read)).
    /// The file is read and hashed once; it is
    /// parsed only when the cache cannot answer. A cache that cannot be read
    /// or written never fails the analysis.
    pub(crate) fn analyze(&self, file: &Path, query: &SymbolQuery) -> Result<Analyzed, TreeHuggerError> {
        self.analyze_source(self.read(file)?, query)
    }

    /// Reads `file` as [`analyze`](Self::analyze) does, for a caller that
    /// analyzes the same bytes more than once.
    ///
    /// `file` must be a [`file_identity`](tree_hugger::file_identity), as
    /// scanner output is: it is used verbatim, not resolved again.
    pub(crate) fn read(&self, file: &Path) -> Result<SourceFile, TreeHuggerError> {
        Ok(SourceFile::read_identified(file, self.language)?.experimental_semantics(self.experimental_semantics))
    }

    /// [`analyze`](Self::analyze) over a source already [`read`](Self::read).
    pub(crate) fn analyze_source(&self, source: SourceFile, query: &SymbolQuery) -> Result<Analyzed, TreeHuggerError> {
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

/// Runs `work` over `files` on worker threads and hands each result to
/// `deliver` in file order, stopping early when `deliver` breaks.
///
/// A sliding window bounds the work: a file is started only while it is
/// fewer than [`window_size`] files past the last delivery `deliver` was told
/// to flush, so at most one window of results is held and a failed write
/// (seen at a flush) leaves at most one window of started work. [`Flush`]
/// asks for a flush every window, before waiting for a result that has not
/// arrived, and after the last file. A panicking `work` is propagated when
/// its file is next to be delivered; no worker outlives the call.
pub(crate) fn in_file_order<T: Send, B>(
    files: &[PathBuf],
    work: impl Fn(&Path) -> T + Sync,
    mut deliver: impl FnMut(&Path, T, Flush) -> ControlFlow<B>,
) -> ControlFlow<B> {
    if files.is_empty() {
        return ControlFlow::Continue(());
    }
    let size = window_size();
    let workers = rayon::current_num_threads().clamp(1, files.len());
    let next = AtomicUsize::new(0);
    let window = Window {
        flushed: Mutex::new(0),
        advanced: Condvar::new(),
        stopped: AtomicBool::new(false),
        size,
    };
    let (sender, receiver) = mpsc::channel::<(usize, thread::Result<T>)>();
    thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let (next, window, work) = (&next, &window, &work);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= files.len() || !window.admit(index) {
                        break;
                    }
                    let result = panic::catch_unwind(AssertUnwindSafe(|| work(&files[index])));
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        let mut pending: BTreeMap<usize, thread::Result<T>> = BTreeMap::new();
        let mut head = 0;
        let mut unflushed = 0;
        let mut flow = ControlFlow::Continue(());
        let mut panicked = None;
        while head < files.len() {
            let Some(result) = pending.remove(&head) else {
                // Every worker sends each file it claims before it exits.
                let (index, result) = receiver.recv().expect("a worker result for every admitted file");
                pending.insert(index, result);
                continue;
            };
            let result = match result {
                Ok(result) => result,
                Err(payload) => {
                    panicked = Some(payload);
                    break;
                }
            };
            unflushed += 1;
            let flush = head + 1 == files.len() || unflushed >= size || !pending.contains_key(&(head + 1));
            if let ControlFlow::Break(value) = deliver(&files[head], result, Flush(flush)) {
                flow = ControlFlow::Break(value);
                break;
            }
            head += 1;
            if flush {
                unflushed = 0;
                window.advance(head);
            }
        }
        window.stop();
        drop(receiver);
        if let Some(payload) = panicked {
            panic::resume_unwind(payload);
        }
        flow
    })
}

/// Whether a delivery should flush its output, from [`in_file_order`]. The
/// window advances past a delivery only after one told to flush returns.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flush(bool);

impl Flush {
    /// Flushes `out` when asked to.
    pub(crate) fn apply(self, out: &mut dyn Write) -> io::Result<()> {
        if self.0 { out.flush() } else { Ok(()) }
    }
}

/// Files [`in_file_order`] may start beyond the last flushed delivery: four
/// per worker thread.
pub(crate) fn window_size() -> usize {
    4 * rayon::current_num_threads().max(1)
}

/// The admission state shared by [`in_file_order`]'s workers.
struct Window {
    /// Deliveries confirmed by a flush.
    flushed: Mutex<usize>,
    advanced: Condvar,
    stopped: AtomicBool,
    size: usize,
}

impl Window {
    /// Waits until `index` is inside the window; `false` once stopped.
    fn admit(&self, index: usize) -> bool {
        let mut flushed = self.flushed.lock().unwrap_or_else(PoisonError::into_inner);
        while index >= *flushed + self.size && !self.stopped.load(Ordering::Acquire) {
            flushed = self.advanced.wait(flushed).unwrap_or_else(PoisonError::into_inner);
        }
        !self.stopped.load(Ordering::Acquire)
    }

    fn advance(&self, flushed: usize) {
        *self.flushed.lock().unwrap_or_else(PoisonError::into_inner) = flushed;
        self.advanced.notify_all();
    }

    fn stop(&self) {
        let _guard = self.flushed.lock().unwrap_or_else(PoisonError::into_inner);
        self.stopped.store(true, Ordering::Release);
        self.advanced.notify_all();
    }
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
                            let _ = writeln!(err, "{}", neutralized(&format!("cache: {display}: {miss}")));
                        }
                    }
                }
                if verbose && let Some(error) = write_error {
                    let _ = writeln!(err, "{}", neutralized(&format!("cache: {display}: write failed: {error}")));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    fn files(count: usize) -> Vec<PathBuf> {
        (0..count).map(|index| PathBuf::from(format!("f{index:03}"))).collect()
    }

    fn index_of(file: &Path) -> usize {
        file.to_str().and_then(|name| name[1..].parse().ok()).expect("a test file name")
    }

    /// Runs `f` with `threads` workers.
    fn with_threads<R: Send>(threads: usize, f: impl FnOnce() -> R + Send) -> R {
        rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("pool").install(f)
    }

    #[test]
    fn every_file_is_delivered_once_in_file_order_with_any_worker_count() {
        let files = files(53);
        for threads in [1, 2, 8] {
            let (delivered, flushes) = with_threads(threads, || {
                let mut delivered = Vec::new();
                let mut flushes = Vec::new();
                let flow = in_file_order::<_, ()>(
                    &files,
                    // Uneven durations, so results arrive out of order.
                    |file| {
                        thread::sleep(Duration::from_micros(((index_of(file) * 7919) % 13) as u64 * 200));
                        index_of(file)
                    },
                    |file, result, flush| {
                        assert_eq!(index_of(file), result);
                        delivered.push(result);
                        flushes.push(flush.0);
                        ControlFlow::Continue(())
                    },
                );
                assert_eq!(flow, ControlFlow::Continue(()));
                (delivered, flushes)
            });
            assert_eq!(delivered, (0..files.len()).collect::<Vec<_>>(), "{threads} threads");
            assert_eq!(flushes.last(), Some(&true), "{threads} threads: the last delivery flushes");
            let window = 4 * threads;
            let mut unflushed = 0;
            for flush in flushes {
                unflushed += 1;
                assert!(unflushed <= window, "{threads} threads: a flush at least every window");
                if flush {
                    unflushed = 0;
                }
            }
        }
    }

    #[test]
    fn an_empty_input_delivers_nothing() {
        let flow = in_file_order::<(), ()>(&[], |_| panic!("no work"), |_, _, _| panic!("no delivery"));
        assert_eq!(flow, ControlFlow::Continue(()));
    }

    /// While the first file is still being analyzed, no file at or beyond
    /// one window is started, with one worker or several.
    #[test]
    fn a_slow_head_holds_the_window() {
        let files = files(40);
        for threads in [1, 2, 4] {
            let started = AtomicUsize::new(0);
            let started_at_head = with_threads(threads, || {
                let mut started_at_head = None;
                let _ = in_file_order::<_, ()>(
                    &files,
                    |file| {
                        started.fetch_add(1, Ordering::SeqCst);
                        if index_of(file) == 0 {
                            thread::sleep(Duration::from_millis(150));
                        }
                    },
                    |file, (), _| {
                        if index_of(file) == 0 {
                            started_at_head = Some(started.load(Ordering::SeqCst));
                        }
                        ControlFlow::Continue(())
                    },
                );
                started_at_head.expect("the head was delivered")
            });
            assert!(started_at_head <= 4 * threads, "{threads} threads: {started_at_head} started");
            assert_eq!(started.load(Ordering::SeqCst), files.len());
        }
    }

    /// A delivery that breaks at its first flush (a failed write) stops the
    /// scan with at most one window started, and every worker has returned.
    #[test]
    fn a_failed_write_stops_within_one_window() {
        let files = files(200);
        for threads in [1, 2, 8] {
            let started = AtomicUsize::new(0);
            let flow = with_threads(threads, || {
                in_file_order(
                    &files,
                    |_| {
                        started.fetch_add(1, Ordering::SeqCst);
                        thread::sleep(Duration::from_micros(300));
                    },
                    |_, (), flush| if flush.0 { ControlFlow::Break("broken pipe") } else { ControlFlow::Continue(()) },
                )
            });
            assert_eq!(flow, ControlFlow::Break("broken pipe"));
            let after_return = started.load(Ordering::SeqCst);
            assert!(after_return <= 4 * threads, "{threads} threads: {after_return} started");
            thread::sleep(Duration::from_millis(20));
            assert_eq!(started.load(Ordering::SeqCst), after_return, "no work runs after the call returns");
        }
    }

    /// A panicking job is propagated when its file is next, after every
    /// earlier file was delivered, instead of leaving a gap that blocks.
    #[test]
    fn a_panicking_job_is_propagated_in_file_order() {
        let files = files(30);
        for (threads, panicking) in [(1, 0), (4, 0), (4, 17)] {
            let mut delivered = Vec::new();
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
                with_threads(threads, || {
                    in_file_order::<_, ()>(
                        &files,
                        |file| {
                            assert_ne!(index_of(file), panicking, "job {panicking} fails");
                            index_of(file)
                        },
                        |_, result, _| {
                            delivered.push(result);
                            ControlFlow::Continue(())
                        },
                    )
                })
            }));
            let payload = outcome.expect_err("the panic propagates");
            let message = payload.downcast_ref::<String>().cloned().unwrap_or_default();
            assert!(message.contains(&format!("job {panicking} fails")), "{message}");
            assert_eq!(delivered, (0..panicking).collect::<Vec<_>>(), "{threads} threads");
        }
    }
}
