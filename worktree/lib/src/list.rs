//! The `wt list` pipeline: everything a listing shows, gathered for one
//! repository while the refresh worker runs, and committed once.
//!
//! [`gather`] is the entry point. It runs every Git call in the repository
//! path it is given and never reads or changes the process's current
//! directory, so two listings of different repositories can run at once. It
//! prints nothing and makes no network request: the remote stage only
//! launches the worker the caller injects ([`ListOptions::worker`]) and
//! follows it through the stores ([`wait`]). Rendering is the caller's.
//!
//! With [`ListOptions::timings`] the listing also reports where its time went
//! ([`Listing::timings`], a [`Scope::Library`] document from entry through the
//! checkout refresh). Each launched worker is then asked to measure itself,
//! and its reports travel beside the spans as diagnostics
//! ([`Timings::worker_reports`]), never inside them. Without it, no clock is
//! read for timing, no `git` count scope is opened, and no worker is asked
//! for timings, so a listing does the same work either way.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::WorktreeError;
use crate::api_preference::{self, RepoIdentity};
use crate::cache::Cache;
use crate::fast_forward::{FfResult, fast_forward_default};
use crate::git::calls;
use crate::graph::{self, GatherInput, GraphFacts, VerboseData};
use crate::listing::{RefSnapshot, RefTips};
use crate::pull_requests::{CachedPrs, PrListing, origin_digest, origin_url, pr_store_path, select_cached, unix_now};
use crate::remote_head::{Phase, remote_head_store_path};
use crate::timing::{self, ChildrenKind, Scope, Span, SpanList, Stage, Timings, WorkerReport, WorkerReportStatus};
use crate::worktree::{DirtyStatus, RefFacts, WorktreeList, gather_dirtiness, parse_worktree_state_in};

pub mod wait;

use wait::{FORCED_BUDGET, HeadEnd, ORDINARY_BUDGET, StoreEnv, WaitEnd, WaitRequest, WorkerLaunch};

/// What one listing gathers and how it reaches the network.
#[derive(Clone, Copy)]
pub struct ListOptions {
    /// `--refresh`: wait for the whole update and PR refresh.
    pub refresh: bool,
    /// `--ignore-api`: record this repository in `~/.wt.json` first.
    pub ignore_api: bool,
    /// `--ff`: wait like [`ListOptions::refresh`], then fast-forward the
    /// local default branch.
    pub fast_forward: bool,
    /// Starts the refresh worker. `None` reads the stored remote answers
    /// without launching or waiting for anything.
    pub worker: Option<WorkerLaunch>,
    /// The ordinary wait for both halves.
    pub wait_budget: Duration,
    /// The `refresh`/`fast_forward` wait.
    pub forced_budget: Duration,
    /// Gather the history the graph is drawn from.
    pub graph: bool,
    /// Gather the current branch's commit details (`-v`).
    pub verbose: bool,
    /// Measure the listing into [`Listing::timings`].
    pub timings: bool,
}

impl Default for ListOptions {
    /// An ordinary listing with no worker, no graph, and no verbose details.
    fn default() -> Self {
        Self {
            refresh: false,
            ignore_api: false,
            fast_forward: false,
            worker: None,
            wait_budget: ORDINARY_BUDGET,
            forced_budget: FORCED_BUDGET,
            graph: false,
            verbose: false,
            timings: false,
        }
    }
}

impl ListOptions {
    fn forced(&self) -> bool {
        self.refresh || self.fast_forward
    }
}

/// What the remote stage reports to the caller's progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitProgress {
    /// The worker is about to be launched and waited for.
    Started,
    /// The followed head attempt entered a phase (see [`wait::wait`]).
    Phase(Phase),
    /// The wait has returned; nothing more is reported.
    Finished,
}

/// Where the two stored answers live for one main checkout.
#[derive(Debug, Clone, Copy)]
pub struct Stores<'a> {
    pub prs: &'a Path,
    pub head: &'a Path,
}

/// What the remote stage hands the rest of the listing.
#[derive(Debug, Default)]
pub struct RemoteAnswers {
    /// The stored PR answer as of the end of the wait.
    pub prs: PrListing,
    pub origin: Option<String>,
    /// The repository is in `~/.wt.json`: no provider request, no badges.
    pub ignored: bool,
    /// `origin` changed or disappeared during the wait: the PR answer and
    /// both halves' results belong to the old one and are not shown
    /// ([`RemoteAnswers::observed`]).
    pub origin_changed: bool,
    /// `None` when nothing was launched: no `origin`, or no worker.
    pub waited: Option<WaitEnd>,
}

/// This listing's own results from both halves, for the repository `origin`
/// still names.
///
/// Only [`RemoteAnswers::observed`] makes one, so every presentation of
/// request evidence that reads through it (the caption's head status, the PR
/// item, credential and fallback notices) cannot describe an `origin` that
/// was replaced or removed during the wait.
#[derive(Debug, Clone, Copy)]
pub struct Observed<'a> {
    pub origin: &'a str,
    pub waited: &'a WaitEnd,
    pub ignored: bool,
}

impl RemoteAnswers {
    /// The repository-identity guard: `None` without a wait, and when
    /// `origin` changed or disappeared during it.
    pub fn observed(&self) -> Option<Observed<'_>> {
        if self.origin_changed {
            return None;
        }
        Some(Observed { origin: self.origin.as_deref()?, waited: self.waited.as_ref()?, ignored: self.ignored })
    }
}

/// What the remote stage settled before any speculative local work.
#[derive(Debug)]
pub struct RemotePlan {
    /// `None`: nothing is launched and stored answers are ignored.
    origin: Option<String>,
    /// The repository is in `~/.wt.json`.
    ignored: bool,
}

/// The remote stage's first step, run before any local gather: looks up
/// `origin` and, for `--ignore-api`, records the repository, so a failed
/// preference write ends the listing with no worker launched.
pub fn prepare_remote(main: &Path, options: &ListOptions) -> Result<RemotePlan, WorktreeError> {
    let origin = origin_url(main);
    if options.ignore_api {
        record_ignore_api(origin.as_deref())?;
    }
    let ignored = origin.as_deref().is_some_and(|origin| {
        api_preference::preference_path().is_some_and(|path| api_preference::load(&path).ignores_origin(origin))
    });
    Ok(RemotePlan { origin, ignored })
}

/// The remote stage for the repository whose main checkout is `main`: one
/// worker launched and waited for (both halves, or the head attempt it
/// adopts), then the stored PR answer as the wait left it.
///
/// The worker is the only PR writer; nothing here asks the network. Without
/// an `origin` nothing is launched, and stored answers are ignored. Without
/// a worker ([`ListOptions::worker`]) the stored answers are read as they
/// are. `on_phase` sees [`WaitProgress::Started`] before a launch and
/// [`WaitProgress::Finished`] once the wait returns.
pub fn follow_remote(
    plan: RemotePlan,
    stores: Stores<'_>,
    main: &Path,
    default_branch: &str,
    options: &ListOptions,
    on_phase: &mut dyn FnMut(WaitProgress),
) -> RemoteAnswers {
    follow_remote_steps(plan, stores, main, default_branch, options, on_phase, None)
}

/// [`follow_remote`], recording [`Stage::RefreshWorker`] and
/// [`Stage::PrCacheRead`] into `steps` when it is given.
fn follow_remote_steps(
    plan: RemotePlan,
    stores: Stores<'_>,
    main: &Path,
    default_branch: &str,
    options: &ListOptions,
    on_phase: &mut dyn FnMut(WaitProgress),
    mut steps: Option<&mut SpanList>,
) -> RemoteAnswers {
    let RemotePlan { origin, ignored } = plan;
    let Some(origin) = origin else {
        return RemoteAnswers::default();
    };
    let Some(launch) = options.worker else {
        let prs = timing::record(steps, Stage::PrCacheRead, || stored_prs(stores.prs, &origin, ignored));
        return RemoteAnswers { prs, origin: Some(origin), ignored, ..RemoteAnswers::default() };
    };

    let started = steps.is_some().then(Instant::now);
    let digest = origin_digest(&origin);
    let request = WaitRequest {
        main,
        origin_digest: &digest,
        branch: default_branch,
        force: options.forced(),
        budget: if options.forced() { options.forced_budget } else { options.wait_budget },
        timings: started.is_some(),
    };
    let env = StoreEnv::new(stores.head.to_path_buf(), stores.prs.to_path_buf(), origin.clone());
    on_phase(WaitProgress::Started);
    let mut launches = LaunchClock::new(launch, started.is_some());
    let wait_started = started.map(|_| Instant::now());
    let waited = wait::wait(request, &env, |main: &Path, args: &wait::LaunchArgs| launches.launch(main, args), &mut |phase| {
        on_phase(WaitProgress::Phase(phase))
    });
    let wait_elapsed = wait_started.map(|at| at.elapsed());
    on_phase(WaitProgress::Finished);
    if let (Some(steps), Some(started), Some(wait_elapsed)) = (steps.as_deref_mut(), started, wait_elapsed) {
        steps.push(launches.span(started.elapsed(), wait_elapsed));
    }

    // Read on every exit path: the worker may have published while we
    // waited, and an answer for an `origin` replaced meanwhile is not shown.
    let (prs, origin_changed) = timing::record(steps, Stage::PrCacheRead, || {
        let origin_changed = origin_url(main).as_deref() != Some(origin.as_str());
        let prs = if origin_changed { PrListing::default() } else { stored_prs(stores.prs, &origin, ignored) };
        (prs, origin_changed)
    });
    RemoteAnswers { prs, origin: Some(origin), ignored, origin_changed, waited: Some(waited) }
}

/// The worker launches of one wait, timed on the foreground's monotonic
/// clock when `timed`.
struct LaunchClock {
    launch: WorkerLaunch,
    timed: bool,
    launches: usize,
    elapsed: Duration,
}

impl LaunchClock {
    fn new(launch: WorkerLaunch, timed: bool) -> Self {
        Self { launch, timed, launches: 0, elapsed: Duration::ZERO }
    }

    fn launch(&mut self, main: &Path, args: &wait::LaunchArgs) -> std::io::Result<wait::WorkerHandle> {
        if !self.timed {
            return (self.launch)(main, args);
        }
        let started = Instant::now();
        let handle = (self.launch)(main, args);
        self.elapsed += started.elapsed();
        self.launches += 1;
        handle
    }

    /// [`Stage::RefreshWorker`] over `total`: the launches (every attempt,
    /// a retry or a failed spawn included) and the rest of the `wait`
    /// elapsed time, which is following the stores. The worker's own work
    /// runs in another process, so the span has no `git` count.
    fn span(&self, total: Duration, wait: Duration) -> Span {
        let mut children = SpanList::sequential();
        if self.launches > 0 {
            children.push(Span::new(Stage::WorkerLaunch, self.elapsed));
        }
        children.push(Span::new(Stage::WorkerWait, wait.saturating_sub(self.elapsed)));
        Span::new(Stage::RefreshWorker, total).with_children(children)
    }
}

/// The stored PR answer for `origin`, fresh or stale; nothing for an ignored
/// repository.
fn stored_prs(store: &Path, origin: &str, ignored: bool) -> PrListing {
    match select_cached(store, Some(origin), unix_now()) {
        CachedPrs::Fresh(listing) | CachedPrs::Stale(listing) if !ignored => listing,
        _ => PrListing::default(),
    }
}

/// `--ignore-api`: adds `origin`'s repository to `~/.wt.json`.
fn record_ignore_api(origin: Option<&str>) -> Result<(), WorktreeError> {
    let origin = origin.ok_or_else(|| WorktreeError::NoRepositoryIdentity("this repository has no origin".into()))?;
    let identity = RepoIdentity::from_origin(origin)
        .ok_or_else(|| WorktreeError::NoRepositoryIdentity("origin is a local path".into()))?;
    let path = api_preference::preference_path()
        .ok_or_else(|| WorktreeError::NoRepositoryIdentity("the home directory is unknown".into()))?;
    api_preference::add(&path, &identity)
}

/// The accepted results of one listing, ready to render once.
pub struct Listing {
    /// Committed: dirtiness, caption, target, tree, and comparisons all
    /// describe the accepted ref snapshot ([`WorktreeList::ref_snapshot`]).
    pub list: WorktreeList,
    pub remote: RemoteAnswers,
    pub ff: Option<FfResult>,
    /// Gathered from the same accepted snapshot as `list`.
    pub graph: Option<GraphFacts>,
    pub verbose: Option<VerboseData>,
    pub main_checkout: Option<PathBuf>,
    pub head_store: Option<PathBuf>,
    /// The first gather was discarded and the ref-dependent facts, graph,
    /// and verbose data were gathered again from the final ref read.
    pub regathered: bool,
    /// Where the time went; `Some` only with [`ListOptions::timings`].
    pub timings: Option<Timings>,
}

/// Graph and verbose data from one snapshot.
type History = (Option<GraphFacts>, Option<VerboseData>);

/// Everything `wt list` shows for the repository containing `repo`,
/// gathered while the refresh worker runs.
///
/// After the cheap parse step (which captures the initial ref snapshot), the
/// remote wait runs on this thread while scoped threads gather dirtiness,
/// the ref-dependent facts, and graph and verbose history from that
/// snapshot. They are joined before `--ff` runs. When remote work was
/// followed or a fast-forward attempted, refs are read again: the first
/// gather is accepted only when both reads succeeded with equal tips;
/// otherwise the ref-dependent facts, graph, and verbose data are gathered
/// once more from the final read ([`Listing::regathered`]). Dirtiness is
/// kept, except that a checkout `--ff` moved is measured again. The gathers'
/// persistent effects (cache save, fork-record and copy-record pruning)
/// happen once, in [`WorktreeList::commit`] of the accepted results.
///
/// A wait budget limits only the wait: a longer local gather is waited for.
/// `on_phase` is called on this thread only, during the wait.
pub fn gather(
    repo: &Path,
    options: &ListOptions,
    on_phase: &mut dyn FnMut(WaitProgress),
) -> Result<Listing, WorktreeError> {
    let started = options.timings.then(Instant::now);
    let mut root = options.timings.then(SpanList::sequential);
    let (mut list, main_checkout, pr_store, head_store) = timing::record(root.as_mut(), Stage::ReadWorktrees, || {
        let list = parse_worktree_state_in(repo)?;
        let main_checkout = list.entries().first().map(|main| main.path.clone());
        let pr_store = main_checkout.as_deref().and_then(|main| pr_store_path(main).ok());
        let head_store = main_checkout.as_deref().and_then(|main| remote_head_store_path(main).ok());
        Ok::<_, WorktreeError>((list, main_checkout, pr_store, head_store))
    })?;
    let default_branch = list.default_branch.clone();
    let plan = match (&pr_store, &head_store, &main_checkout) {
        (Some(_), Some(_), Some(main)) => {
            Some(timing::record(root.as_mut(), Stage::OriginLookup, || prepare_remote(main, options))?)
        }
        _ => None,
    };

    let needs_graph = options.graph;
    let (initial, cache, first_input) = timing::record(root.as_mut(), Stage::PrepareLocal, || {
        let initial = list.ref_snapshot().clone();
        let cache = list.load_comparison_cache();
        let first_input = GatherInput::from_list(&list, initial.tips());
        (initial, cache, first_input)
    });
    let needs_verbose = options.verbose && first_input.has_verbose();
    let needs_history = needs_graph || needs_verbose;
    let timed = options.timings;

    // The region's span runs from before the first spawn until the wait, the
    // PR reread, and every local task have finished. Its count scope is open
    // before the tasks take their handle, so they count into it.
    let region_scope = timed.then(calls::CallScope::enter);
    let region_started = timed.then(Instant::now);
    let counted = calls::TaskHandle::current();
    let mut region = timed.then(SpanList::concurrent);
    let (remote, (dirty, first_facts, local_span), first_history) = std::thread::scope(|scope| {
        let history = needs_history.then(|| {
            scope.spawn(|| {
                counted.run(|| {
                    #[cfg(test)]
                    tests::overlap::arrive(tests::overlap::Gather::Graph);
                    let data = gather_history(&first_input, needs_graph, needs_verbose, timed);
                    #[cfg(test)]
                    tests::overlap::finished(tests::overlap::Gather::Graph);
                    data
                })
            })
        });
        let local = scope.spawn(|| {
            counted.run(|| {
                #[cfg(test)]
                tests::overlap::arrive(tests::overlap::Gather::List);
                let local = gather_local(&list, initial.tips(), &cache, timed);
                #[cfg(test)]
                tests::overlap::finished(tests::overlap::Gather::List);
                local
            })
        });

        let remote = match (plan, &pr_store, &head_store, &main_checkout) {
            (Some(plan), Some(prs), Some(head), Some(main)) => {
                follow_remote_steps(plan, Stores { prs, head }, main, &default_branch, options, on_phase, region.as_mut())
            }
            _ => RemoteAnswers::default(),
        };
        #[cfg(test)]
        tests::overlap::remote_finished();
        let local = calls::joined(local.join().expect("list gather thread panicked"));
        let history = history.map(|handle| calls::joined(handle.join().expect("graph gather thread panicked")));
        (remote, local, history)
    });
    if let (Some(root), Some(mut region), Some(region_started), Some(region_scope)) =
        (root.as_mut(), region.take(), region_started, region_scope)
    {
        let elapsed = region_started.elapsed();
        region.push(local_span.expect("a timed local gather has a span"));
        if let Some((_, Some(span))) = &first_history {
            region.push(span.clone());
        }
        // Only a wait makes this a remote region, not merely an `origin`.
        let stage = if remote.waited.is_some() { Stage::RemoteAndLocal } else { Stage::LocalReads };
        root.push(Span::new(stage, elapsed).with_git_calls(region_scope.finish()).with_children(region));
    }
    let first_history = first_history.map(|(data, _)| data);

    let ff = match (&main_checkout, options.fast_forward) {
        (Some(main), true) => {
            Some(timing::record(root.as_mut(), Stage::FastForward, || fast_forward_default(main, &default_branch)))
        }
        _ => None,
    };

    // The fetch and the fast-forward move refs; everything shown describes
    // the state they left. Without either, the initial read is the final one.
    let reread = remote.waited.is_some() || ff.is_some();
    let accepted_refs = if reread {
        timing::record(root.as_mut(), Stage::RefReread, || RefSnapshot::read_in(list.repo()))
    } else {
        initial.clone()
    };
    let regathered = reread && !initial.matches(&accepted_refs);
    let (facts, history) = if !regathered {
        (first_facts, first_history)
    } else {
        timing::record_parent(root.as_mut(), Stage::Regather, ChildrenKind::Sequential, |steps| {
            regather(&list, accepted_refs.tips(), &cache, needs_graph, needs_verbose, steps)
        })
    };
    timing::record(root.as_mut(), Stage::Commit, || list.commit(accepted_refs, dirty, facts, cache));
    if let Some(FfResult::Moved { checkout: Some(checkout), .. }) = &ff {
        timing::record(root.as_mut(), Stage::CheckoutRefresh, || list.refresh_dirty_status(checkout));
    }

    let timings = root.zip(started).map(|(root, started)| {
        let timings = Timings::new(Scope::Library, started.elapsed(), root);
        match worker_reports(&remote) {
            Some((reports, status)) => timings.with_worker_reports(reports, status),
            None => timings,
        }
    });
    let (graph, verbose) = history.unwrap_or((None, None));
    Ok(Listing { list, remote, ff, graph, verbose, main_checkout, head_store, regathered, timings })
}

/// The worker reports of a timed listing's wait and their summary; `None`
/// when no worker was followed. A changed `origin` suppresses every report,
/// as it suppresses the wait's results ([`RemoteAnswers::observed`]).
fn worker_reports(remote: &RemoteAnswers) -> Option<(Vec<WorkerReport>, WorkerReportStatus)> {
    let waited = remote.waited.as_ref()?;
    if remote.origin_changed {
        return Some((Vec::new(), WorkerReportStatus::OriginChanged));
    }
    let followed = match &waited.head {
        HeadEnd::Finished(attempt) | HeadEnd::Running { last: Some(attempt) } => Some(attempt.id.as_str()),
        HeadEnd::Running { last: None } | HeadEnd::Unavailable => None,
    };
    // Every launch has an entry, so a followed attempt with none is adopted.
    let adopted = followed.is_some_and(|id| waited.worker_reports.iter().all(|report| report.attempt_id != id));
    let reports = waited.worker_reports.clone();
    let status = timing::summarize_worker_reports(&reports, adopted, false);
    Some((reports, status))
}

/// Graph and verbose history from `input`, with its span when `timed`.
fn gather_history(input: &GatherInput, needs_graph: bool, needs_verbose: bool, timed: bool) -> (History, Option<Span>) {
    if timed {
        let (data, span) = graph::gather_timed(input, needs_graph, needs_verbose);
        (data, Some(span))
    } else {
        (graph::gather(input, needs_graph, needs_verbose), None)
    }
}

/// Dirtiness and the ref-dependent facts for `refs`, concurrently, as
/// [`WorktreeList::gather_local`] does; when `timed`, also the
/// [`Stage::LocalGather`] span with each side as a concurrent child.
fn gather_local(
    list: &WorktreeList,
    refs: &RefTips,
    cache: &Mutex<Cache>,
    timed: bool,
) -> (Vec<DirtyStatus>, RefFacts, Option<Span>) {
    if !timed {
        let (dirty, facts) = list.gather_local(refs, cache);
        return (dirty, facts, None);
    }
    let ((dirty, facts, sides), span) = timing::measure(Stage::LocalGather, || {
        let counted = calls::TaskHandle::current();
        std::thread::scope(|scope| {
            let dirty = scope.spawn(|| {
                counted.run(|| timing::measure(Stage::WorktreeStatus, || gather_dirtiness(list.entries())))
            });
            let (facts, comparisons) = timing::measure(Stage::BranchComparisons, || list.gather_ref_facts(refs, cache));
            let (dirty, status) = calls::joined(dirty.join().expect("dirtiness thread panicked"));
            let mut sides = SpanList::concurrent();
            sides.push(status);
            sides.push(comparisons);
            (dirty, facts, sides)
        })
    });
    (dirty, facts, Some(span.with_children(sides)))
}

/// The ref-dependent facts, graph, and verbose data gathered again from
/// `refs`: the input first, then the facts beside the history. Dirtiness is
/// never read again. When `steps` is given, records [`Stage::PrepareLocal`]
/// and a [`Stage::LocalReads`] group of [`Stage::BranchComparisons`] beside
/// the history span into it.
fn regather(
    list: &WorktreeList,
    refs: &RefTips,
    cache: &Mutex<Cache>,
    needs_graph: bool,
    needs_verbose: bool,
    mut steps: Option<&mut SpanList>,
) -> (RefFacts, Option<History>) {
    let needs_history = needs_graph || needs_verbose;
    let timed = steps.is_some();
    let input = timing::record(steps.as_deref_mut(), Stage::PrepareLocal, || GatherInput::from_list(list, refs));
    timing::record_parent(steps, Stage::LocalReads, ChildrenKind::Concurrent, |mut group| {
        let counted = calls::TaskHandle::current();
        let (facts, history) = std::thread::scope(|scope| {
            let history = needs_history.then(|| {
                scope.spawn(|| counted.run(|| gather_history(&input, needs_graph, needs_verbose, timed)))
            });
            let facts =
                timing::record(group.as_deref_mut(), Stage::BranchComparisons, || list.gather_ref_facts(refs, cache));
            let history = history.map(|handle| calls::joined(handle.join().expect("graph regather thread panicked")));
            (facts, history)
        });
        let history = history.map(|(data, span)| {
            if let (Some(group), Some(span)) = (group, span) {
                group.push(span);
            }
            data
        });
        (facts, history)
    })
}

#[cfg(test)]
mod tests;
