//! The `wt list` pipeline: everything a listing shows, gathered for one
//! repository while the refresh worker runs, and committed once.
//!
//! [`gather`] is the entry point. It runs every Git call in the repository
//! path it is given and never reads or changes the process's current
//! directory, so two listings of different repositories can run at once. It
//! prints nothing and makes no network request: the remote stage only
//! launches the worker the caller injects ([`ListOptions::worker`]) and
//! follows it through the stores ([`wait`]). Rendering is the caller's.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::WorktreeError;
use crate::api_preference::{self, RepoIdentity};
use crate::fast_forward::{FfResult, fast_forward_default};
use crate::git::calls;
use crate::graph::{self, GatherInput, GraphFacts, VerboseData};
use crate::listing::RefSnapshot;
use crate::pull_requests::{CachedPrs, PrListing, origin_digest, origin_url, pr_store_path, select_cached, unix_now};
use crate::remote_head::{Phase, remote_head_store_path};
use crate::worktree::{WorktreeList, parse_worktree_state_in};

pub mod wait;

use wait::{FORCED_BUDGET, ORDINARY_BUDGET, StoreEnv, WaitEnd, WaitRequest, WorkerLaunch};

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
    /// The origin recheck and stored PR answer read after the wait.
    pub pr_reread: Duration,
    /// The launch and the wait.
    pub remote_wait: Duration,
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
    /// The origin lookup, `--ignore-api` record, and preference read.
    pr_gather: Duration,
}

/// The remote stage's first step, run before any local gather: looks up
/// `origin` and, for `--ignore-api`, records the repository, so a failed
/// preference write ends the listing with no worker launched.
pub fn prepare_remote(main: &Path, options: &ListOptions) -> Result<RemotePlan, WorktreeError> {
    let t0 = Instant::now();
    let origin = origin_url(main);
    if options.ignore_api {
        record_ignore_api(origin.as_deref())?;
    }
    let ignored = origin.as_deref().is_some_and(|origin| {
        api_preference::preference_path().is_some_and(|path| api_preference::load(&path).ignores_origin(origin))
    });
    Ok(RemotePlan { origin, ignored, pr_gather: t0.elapsed() })
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
    let RemotePlan { origin, ignored, .. } = plan;
    let Some(origin) = origin else {
        return RemoteAnswers::default();
    };
    let Some(launch) = options.worker else {
        let prs = stored_prs(stores.prs, &origin, ignored);
        return RemoteAnswers { prs, origin: Some(origin), ignored, ..RemoteAnswers::default() };
    };

    let t0 = Instant::now();
    let digest = origin_digest(&origin);
    let request = WaitRequest {
        main,
        origin_digest: &digest,
        branch: default_branch,
        force: options.forced(),
        budget: if options.forced() { options.forced_budget } else { options.wait_budget },
    };
    let env = StoreEnv::new(stores.head.to_path_buf(), stores.prs.to_path_buf(), origin.clone());
    on_phase(WaitProgress::Started);
    let waited = wait::wait(request, &env, launch, &mut |phase| on_phase(WaitProgress::Phase(phase)));
    on_phase(WaitProgress::Finished);
    let remote_wait = t0.elapsed();

    // Read on every exit path: the worker may have published while we
    // waited, and an answer for an `origin` replaced meanwhile is not shown.
    let t0 = Instant::now();
    let origin_changed = origin_url(main).as_deref() != Some(origin.as_str());
    let prs = if origin_changed { PrListing::default() } else { stored_prs(stores.prs, &origin, ignored) };
    let pr_reread = t0.elapsed();
    RemoteAnswers { prs, origin: Some(origin), ignored, origin_changed, waited: Some(waited), pr_reread, remote_wait }
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

/// How long each step of [`gather`] took, for `wt list --perf`.
///
/// Concurrent work is measured as one group ([`Durations::group`]) whose
/// parts overlap; a part is never added to the group's total.
#[derive(Debug, Clone, Default)]
pub struct Durations {
    /// [`prepare_remote`]; `None` without a main checkout's stores.
    pub pr_gather: Option<Duration>,
    /// The concurrent region: the remote stage beside the local gathers.
    pub group: Duration,
    /// Dirtiness and the ref-dependent facts, inside [`Durations::group`].
    pub list_gather: Duration,
    /// The graph and verbose history, inside [`Durations::group`].
    pub history: Option<Duration>,
    pub fast_forward: Option<Duration>,
    pub regather: Option<RegatherDurations>,
    /// The second dirtiness read of a checkout `--ff` moved.
    pub checkout_refresh: Option<Duration>,
}

/// The parts of a regather, which overlap one another.
#[derive(Debug, Clone, Copy)]
pub struct RegatherDurations {
    pub total: Duration,
    pub ref_facts: Duration,
    pub history: Option<Duration>,
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
    pub durations: Durations,
}

/// Graph and verbose data from one snapshot, with how long the gather took.
type History = ((Option<GraphFacts>, Option<VerboseData>), Duration);

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
    let mut durations = Durations::default();
    let mut list = parse_worktree_state_in(repo)?;
    let main_checkout = list.entries().first().map(|main| main.path.clone());
    let pr_store = main_checkout.as_deref().and_then(|main| pr_store_path(main).ok());
    let head_store = main_checkout.as_deref().and_then(|main| remote_head_store_path(main).ok());
    let default_branch = list.default_branch.clone();
    let plan = match (&pr_store, &head_store, &main_checkout) {
        (Some(_), Some(_), Some(main)) => Some(prepare_remote(main, options)?),
        _ => None,
    };
    durations.pr_gather = plan.as_ref().map(|plan| plan.pr_gather);

    let needs_graph = options.graph;
    let initial = list.ref_snapshot().clone();
    let cache = list.load_comparison_cache();
    let first_input = GatherInput::from_list(&list, initial.tips());
    let needs_verbose = options.verbose && first_input.has_verbose();
    let needs_history = needs_graph || needs_verbose;

    // The group's span runs from before the first spawn until the wait, the
    // PR reread, and every local task have finished.
    let counted = calls::TaskHandle::current();
    let group_start = Instant::now();
    let (remote, (dirty, first_facts, list_elapsed), first_history) = std::thread::scope(|scope| {
        let history = needs_history.then(|| {
            scope.spawn(|| {
                counted.run(|| {
                    #[cfg(test)]
                    tests::overlap::arrive(tests::overlap::Gather::Graph);
                    let t0 = Instant::now();
                    let data = graph::gather(&first_input, needs_graph, needs_verbose);
                    #[cfg(test)]
                    tests::overlap::finished(tests::overlap::Gather::Graph);
                    (data, t0.elapsed())
                })
            })
        });
        let local = scope.spawn(|| {
            counted.run(|| {
                #[cfg(test)]
                tests::overlap::arrive(tests::overlap::Gather::List);
                let t0 = Instant::now();
                let (dirty, facts) = list.gather_local(initial.tips(), &cache);
                #[cfg(test)]
                tests::overlap::finished(tests::overlap::Gather::List);
                (dirty, facts, t0.elapsed())
            })
        });

        let remote = match (plan, &pr_store, &head_store, &main_checkout) {
            (Some(plan), Some(prs), Some(head), Some(main)) => {
                follow_remote(plan, Stores { prs, head }, main, &default_branch, options, on_phase)
            }
            _ => RemoteAnswers::default(),
        };
        #[cfg(test)]
        tests::overlap::remote_finished();
        let local = calls::joined(local.join().expect("list gather thread panicked"));
        let history: Option<History> =
            history.map(|handle| calls::joined(handle.join().expect("graph gather thread panicked")));
        (remote, local, history)
    });
    durations.group = group_start.elapsed();
    durations.list_gather = list_elapsed;
    durations.history = first_history.as_ref().map(|(_, elapsed)| *elapsed);

    let ff = match (&main_checkout, options.fast_forward) {
        (Some(main), true) => {
            let t0 = Instant::now();
            let result = fast_forward_default(main, &default_branch);
            durations.fast_forward = Some(t0.elapsed());
            Some(result)
        }
        _ => None,
    };

    // The fetch and the fast-forward move refs; everything shown describes
    // the state they left. Without either, the initial read is the final one.
    let reread = remote.waited.is_some() || ff.is_some();
    let accepted_refs = if reread { RefSnapshot::read_in(list.repo()) } else { initial.clone() };
    let regathered = reread && !initial.matches(&accepted_refs);
    let (facts, history) = if !regathered {
        (first_facts, first_history.map(|(data, _)| data))
    } else {
        let t0 = Instant::now();
        let final_input = GatherInput::from_list(&list, accepted_refs.tips());
        let ((facts, facts_elapsed), history) = std::thread::scope(|scope| {
            let history = needs_history.then(|| {
                scope.spawn(|| {
                    counted.run(|| {
                        let t0 = Instant::now();
                        (graph::gather(&final_input, needs_graph, needs_verbose), t0.elapsed())
                    })
                })
            });
            let t0 = Instant::now();
            let facts = list.gather_ref_facts(accepted_refs.tips(), &cache);
            let history = history.map(|handle| calls::joined(handle.join().expect("graph regather thread panicked")));
            ((facts, t0.elapsed()), history)
        });
        durations.regather = Some(RegatherDurations {
            total: t0.elapsed(),
            ref_facts: facts_elapsed,
            history: history.as_ref().map(|(_, elapsed)| *elapsed),
        });
        (facts, history.map(|(data, _)| data))
    };
    list.commit(accepted_refs, dirty, facts, cache);
    if let Some(FfResult::Moved { checkout: Some(checkout), .. }) = &ff {
        let t0 = Instant::now();
        list.refresh_dirty_status(checkout);
        durations.checkout_refresh = Some(t0.elapsed());
    }

    let (graph, verbose) = history.unwrap_or((None, None));
    Ok(Listing { list, remote, ff, graph, verbose, main_checkout, head_store, regathered, durations })
}

#[cfg(test)]
mod tests;
