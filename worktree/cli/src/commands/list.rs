use std::io::IsTerminal as _;
use std::path::Path;
use std::time::{Duration, Instant};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::terminal_image::parse_width_spec;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use sniff::remote::blocking::credential_env;
use worktree::WorktreeError;
use worktree::api_preference::{self, RepoIdentity};
use worktree::fast_forward::{FfRefusal, FfResult, fast_forward_default};
use worktree::listing::CaptionState;
use worktree::live_remote::tracking_ref_changed_at;
use worktree::pull_requests::{
    CachedPrs, LIST_DEADLINE, OpenPrSource, PrListing, SniffOpenPrSource, fetch_and_publish, origin_digest,
    origin_url, pr_lock_held, pr_store_path, select_cached, unix_now,
};
use worktree::remote_head::{
    ApiCondition, ApiNote, Attempt, CachedRemoteHead, CheckFailure, Outcome, Phase, PrFailure, PrStatus,
    remote_head_store_path, select_cached_head,
};
use worktree::worktree::{fill_worktree_statuses, parse_worktree_state};

use super::git_graph;
use super::list_table::{
    self, CredentialCondition, CredentialLine, FfNotice, FfSuggestion, LastKnown, RemoteFacts, RemoteStatus,
    Sections, TableFacts,
};
use crate::perf;

mod wait;

pub use wait::{FORCED_BUDGET, LaunchArgs, ORDINARY_BUDGET, WorkerHandle, WorkerLaunch};
use wait::{Progress, StoreEnv, WaitEnd, WaitRequest, Waited};

/// The flags that apply only to listing (spec §7–§9).
#[derive(Debug, Clone, Copy, Default)]
pub struct ListFlags {
    /// `-r`/`--refresh`: ignore both freshness windows and wait for the whole
    /// update and PR refresh.
    pub refresh: bool,
    /// `--ignore-api`: record this repository in `~/.wt.json` first.
    pub ignore_api: bool,
    /// `--ff`/`--fast-forward`: wait like `--refresh`, then fast-forward the
    /// local default branch.
    pub fast_forward: bool,
}

impl ListFlags {
    fn forced(self) -> bool {
        self.refresh || self.fast_forward
    }
}

/// Builds the open-PR source for an `origin` URL when a request is due.
pub type PrConnect = fn(&str) -> Box<dyn OpenPrSource>;

/// The production source: sniff's provider client for `origin`.
fn origin_pr_source(origin: &str) -> Box<dyn OpenPrSource> {
    Box::new(SniffOpenPrSource {
        remote_url: origin.to_string(),
        deadline: LIST_DEADLINE,
    })
}

/// How listing reaches the network: a foreground PR request on a PR miss, and
/// the worker it launches (or adopts) and waits for. Tests replace both, and
/// shorten the waits.
#[derive(Clone, Copy)]
pub struct ListSeams {
    pub connect: PrConnect,
    pub launch: WorkerLaunch,
    /// Ordinary listing's wait for the attempt.
    pub wait_budget: Duration,
    /// The `--refresh`/`--ff` wait.
    pub forced_budget: Duration,
}

const PRODUCTION_SEAMS: ListSeams = ListSeams {
    connect: origin_pr_source,
    launch: super::refresh_worker::launch,
    wait_budget: ORDINARY_BUDGET,
    forced_budget: FORCED_BUDGET,
};

/// Where the two stored answers live for one main checkout.
#[derive(Clone, Copy)]
struct Stores<'a> {
    prs: &'a Path,
    head: &'a Path,
}

/// What the remote stage hands the rest of the listing.
#[derive(Default)]
struct RemoteAnswers {
    /// The stored PR answer as of the end of the wait.
    prs: PrListing,
    /// This run's foreground PR request failure, for §5.
    pr_failure: Option<PrFailure>,
    origin: Option<String>,
    /// The repository is in `~/.wt.json`: no provider request, no badges.
    ignored: bool,
    /// `None` without an `origin`: nothing was launched.
    waited: Option<Waited>,
    /// The `pr gather` perf stage: the origin lookup, PR selection, and any
    /// foreground PR request.
    pr_gather: Duration,
    /// The `remote wait` perf stage: the launch and the wait.
    remote_wait: Duration,
}

/// The remote stage for the repository whose main checkout is `main`: the
/// stored PR answer (a request only on a miss), then one worker attempt,
/// launched or adopted, waited for.
///
/// The PR miss request settles before the launch, so the worker's PR half
/// finds its answer and never repeats it. `--ignore-api` records the
/// repository before anything asks the network. Without an `origin` nothing
/// is requested or launched, and stored answers are ignored.
fn gather_remote(
    stores: Stores<'_>,
    main: &Path,
    default_branch: &str,
    flags: ListFlags,
    seams: ListSeams,
) -> Result<RemoteAnswers, WorktreeError> {
    let t0 = Instant::now();
    let origin = origin_url(main);
    if flags.ignore_api {
        record_ignore_api(origin.as_deref())?;
    }
    let Some(origin) = origin else {
        return Ok(RemoteAnswers { pr_gather: t0.elapsed(), ..RemoteAnswers::default() });
    };
    let ignored = api_preference::preference_path()
        .is_some_and(|path| api_preference::load(&path).ignores_origin(&origin));
    let mut pr_failure = None;
    let mut prs = match select_cached(stores.prs, Some(&origin), unix_now()) {
        CachedPrs::Fresh(listing) | CachedPrs::Stale(listing) => listing,
        // A forced worker asks anyway; an ignored repository never asks.
        CachedPrs::Miss if ignored || flags.forced() => PrListing::default(),
        CachedPrs::Miss => {
            match fetch_and_publish(stores.prs, main, &origin, unix_now(), (seams.connect)(&origin).as_ref()) {
                Ok(listing) => listing.unwrap_or_default(),
                Err(failure) => {
                    pr_failure = Some(failure);
                    PrListing::default()
                }
            }
        }
    };
    let pr_gather = t0.elapsed();

    let t0 = Instant::now();
    let digest = origin_digest(&origin);
    let request = WaitRequest {
        main,
        origin_digest: &digest,
        branch: default_branch,
        force: flags.forced(),
        budget: if flags.forced() { seams.forced_budget } else { seams.wait_budget },
    };
    let env = StoreEnv::new(main, stores.head.to_path_buf(), stores.prs.to_path_buf(), origin.clone());
    let progress = Progress::on_stderr();
    let waited = wait::wait(request, &env, seams.launch, &mut |phase| progress.show(phase));
    progress.finish();
    let remote_wait = t0.elapsed();

    // The worker may have published a newer PR answer while we waited.
    if let CachedPrs::Fresh(listing) | CachedPrs::Stale(listing) = select_cached(stores.prs, Some(&origin), unix_now()) {
        prs = listing;
    }
    if ignored {
        prs = PrListing::default();
    }
    Ok(RemoteAnswers { prs, pr_failure, origin: Some(origin), ignored, waited: Some(waited), pr_gather, remote_wait })
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

/// The followed attempt as the wait left it, and what the caption says of it.
fn remote_status(end: &WaitEnd, last: impl Fn() -> LastKnown) -> RemoteStatus {
    let failed = |reason| RemoteStatus::CheckFailed { reason, last: last() };
    match end {
        WaitEnd::Finished { attempt, .. } => match attempt.outcome {
            Some(Outcome::InSync) => RemoteStatus::CheckedNow,
            Some(Outcome::Fetched) => RemoteStatus::Fetched,
            Some(Outcome::Absent) => RemoteStatus::Absent,
            Some(Outcome::FetchFailed { reason }) => RemoteStatus::FetchFailed { reason },
            Some(Outcome::CheckFailed { reason }) => failed(reason),
            Some(Outcome::Unavailable { .. }) | None => failed(CheckFailure::Other),
        },
        WaitEnd::TimedOut { last: Some(Attempt { phase: Phase::Fetching, .. }) } => RemoteStatus::StillPulling,
        WaitEnd::TimedOut { .. } => RemoteStatus::StillChecking { last: last() },
        WaitEnd::Unavailable => failed(CheckFailure::Other),
    }
}

/// This run's attempt, if it got far enough to record one.
fn followed_attempt(end: &WaitEnd) -> Option<&Attempt> {
    match end {
        WaitEnd::Finished { attempt, .. } => Some(attempt),
        WaitEnd::TimedOut { last } => last.as_ref(),
        WaitEnd::Unavailable => None,
    }
}

/// The §5 line for what this run observed: the attempt's API note first,
/// then a PR failure (the foreground request's, or the forced worker's).
fn credential_line(origin: &str, attempt: Option<&Attempt>, pr_failure: Option<&PrFailure>) -> Option<CredentialLine> {
    let env = credential_env(origin)?;
    let accepted = env.variables.join(" or ");
    let line = |condition, key: &Option<String>| CredentialLine {
        provider: env.provider.clone(),
        key: key.clone().unwrap_or_else(|| accepted.clone()),
        condition,
    };
    let from_note = attempt.and_then(|attempt| {
        let note = attempt.api.as_ref()?;
        let git_failed = matches!(attempt.outcome, Some(Outcome::CheckFailed { .. }));
        let condition = match note.condition {
            ApiCondition::CredentialsRequired | ApiCondition::NotFoundOrNotPermitted
                if note.key.is_none() && git_failed =>
            {
                CredentialCondition::NotVisible
            }
            ApiCondition::CredentialsRejected => CredentialCondition::Rejected,
            ApiCondition::CredentialsInsufficient => CredentialCondition::Insufficient,
            ApiCondition::RateLimited { authenticated } => CredentialCondition::RateLimited { authenticated },
            ApiCondition::CredentialsRequired | ApiCondition::NotFoundOrNotPermitted => return None,
        };
        Some(line(condition, &note.key))
    });
    from_note.or_else(|| match pr_failure? {
        PrFailure::CredentialsRejected { key } => Some(line(CredentialCondition::Rejected, key)),
        PrFailure::CredentialsInsufficient { key } => Some(line(CredentialCondition::Insufficient, key)),
        PrFailure::RateLimited { authenticated, key } => {
            Some(line(CredentialCondition::RateLimited { authenticated: *authenticated }, key))
        }
        PrFailure::CredentialsRequired | PrFailure::NotFoundOrNotPermitted | PrFailure::Other => None,
    })
}

/// §8: the attempt fell back to `ls-remote` because no key was set, and Git
/// answered.
fn fallback_notice(origin: &str, attempt: Option<&Attempt>) -> Option<Vec<String>> {
    let note: &ApiNote = attempt?.api.as_ref()?;
    let no_key = matches!(note.condition, ApiCondition::CredentialsRequired | ApiCondition::NotFoundOrNotPermitted)
        && note.key.is_none();
    (no_key && note.fallback_answered).then(|| credential_env(origin).map(|env| env.variables)).flatten()
}

fn ff_notice(result: &FfResult) -> Option<FfNotice> {
    let FfResult::Refused(refusal) = result else {
        return None;
    };
    Some(match refusal {
        FfRefusal::DirtyCheckout => FfNotice::DirtyCheckout,
        FfRefusal::Diverged => FfNotice::Diverged,
        FfRefusal::MissingLocal(reference) | FfRefusal::MissingTracking(reference) => {
            FfNotice::Missing(reference.clone())
        }
        FfRefusal::Changed => FfNotice::Changed,
        FfRefusal::Other => FfNotice::Failed,
    })
}

pub fn run(
    width_spec: Option<&str>,
    verbose: bool,
    perf: bool,
    flags: ListFlags,
    process_start: Instant,
) -> Result<(), WorktreeError> {
    let stderr_is_tty = std::io::stderr().is_terminal();
    let image_support = if stderr_is_tty {
        detect_image_support_from_env()
    } else {
        ImageSupport::None
    };
    let terminal = Terminal::default();
    let collector = run_pipeline(
        width_spec,
        verbose,
        perf,
        flags,
        process_start,
        image_support,
        &terminal,
        PRODUCTION_SEAMS,
    )?;
    if let Some(c) = collector {
        c.emit();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_pipeline(
    width_spec: Option<&str>,
    verbose: bool,
    perf: bool,
    flags: ListFlags,
    process_start: Instant,
    image_support: ImageSupport,
    terminal: &Terminal,
    seams: ListSeams,
) -> Result<Option<perf::PerfCollector>, WorktreeError> {
    let mut collector = if perf {
        Some(perf::PerfCollector::new(process_start))
    } else {
        None
    };

    if perf {
        perf::record(&mut collector, "pre-dispatch", process_start.elapsed());
    }

    let parsed_width = width_spec.and_then(|s| parse_width_spec(s).ok());
    let mut list = parse_worktree_state()?;
    let main_checkout = list.entries().first().map(|main| main.path.clone());
    let pr_store = main_checkout.as_deref().and_then(|main| pr_store_path(main).ok());
    let head_store = main_checkout.as_deref().and_then(|main| remote_head_store_path(main).ok());
    let default_branch = list.default_branch.clone();

    let mut remote = match (&pr_store, &head_store, &main_checkout) {
        (Some(prs), Some(head), Some(main)) => {
            gather_remote(Stores { prs, head }, main, &default_branch, flags, seams)?
        }
        _ => RemoteAnswers::default(),
    };
    if perf {
        perf::record(&mut collector, "pr gather", remote.pr_gather);
        perf::record(&mut collector, "remote wait", remote.remote_wait);
    }

    let ff = match (&main_checkout, flags.fast_forward) {
        (Some(main), true) => {
            let t0 = Instant::now();
            let result = fast_forward_default(main, &default_branch);
            perf::record(&mut collector, "fast-forward", t0.elapsed());
            Some(result)
        }
        _ => None,
    };
    // The fetch and the fast-forward move refs; everything below describes
    // the state they left.
    if remote.waited.is_some() || ff.is_some() {
        list.reread_refs();
    }

    let needs_graph = image_support != ImageSupport::None;
    let gather_input = git_graph::GatherInput::from_list(&list);
    let needs_verbose = verbose && gather_input.has_verbose();
    let (graph_facts, verbose_data) = std::thread::scope(|scope| {
        let graph_handle = (needs_graph || needs_verbose).then(|| {
            scope.spawn(|| {
                #[cfg(test)]
                tests::overlap::arrive(tests::overlap::Gather::Graph);
                let t0 = perf.then(Instant::now);
                let data = git_graph::gather(&gather_input, needs_graph, needs_verbose);
                (data, t0.map(|start| start.elapsed()))
            })
        });

        #[cfg(test)]
        tests::overlap::arrive(tests::overlap::Gather::List);
        let t0 = perf.then(Instant::now);
        fill_worktree_statuses(&mut list)?;
        if let Some(start) = t0 {
            perf::record(&mut collector, "list gather", start.elapsed());
        }

        Ok::<_, WorktreeError>(match graph_handle {
            Some(handle) => {
                let (data, elapsed) = handle.join().expect("graph gather thread panicked");
                if let Some(elapsed) = elapsed {
                    let stage = if needs_graph { "graph gather" } else { "verbose gather" };
                    perf::record(&mut collector, stage, elapsed);
                }
                data
            }
            None => (None, None),
        })
    })?;

    let t0 = perf.then(Instant::now);
    let now = unix_now();
    let tracking_ref = format!("origin/{}", list.default_branch);
    let tracking_tip = list
        .caption
        .as_ref()
        .map(|caption| caption.tracking_sha.clone())
        .or_else(|| list.refs().remote(&tracking_ref).map(str::to_string));
    let status = match (&remote.origin, &remote.waited, &head_store, &main_checkout) {
        (Some(origin), Some(waited), Some(head_store), Some(main)) => Some(remote_status(&waited.end, || {
            match select_cached_head(head_store, Some(origin), Some(&default_branch), now) {
                CachedRemoteHead::Fresh(head) | CachedRemoteHead::Stale(head) => {
                    LastKnown::Answer { checked_at: head.checked_at }
                }
                // The reflog runs only without a stored answer.
                CachedRemoteHead::Miss => match tracking_ref_changed_at(main, &default_branch) {
                    Some(at) => LastKnown::TrackingRefChanged { at },
                    None => LastKnown::Never,
                },
            }
        })),
        _ => None,
    };
    let remote_facts = status.map(|status| RemoteFacts {
        default_branch: &list.default_branch,
        tracking_tip: tracking_tip.as_deref(),
        status,
    });
    let unfinished = unfinished(&mut remote, pr_store.as_deref(), now);
    let mut facts = TableFacts::from_list(&list, &remote.prs, remote_facts);
    if let (Some(origin), Some(waited)) = (&remote.origin, &remote.waited) {
        let attempt = followed_attempt(&waited.end);
        let receipt_failure = match &waited.end {
            WaitEnd::Finished { receipt: Some(receipt), .. } => match &receipt.prs {
                PrStatus::Failed { failure } => Some(failure),
                _ => None,
            },
            _ => None,
        };
        facts.credential_line = credential_line(origin, attempt, remote.pr_failure.as_ref().or(receipt_failure));
        facts.fallback_notice = fallback_notice(origin, attempt);
    }
    facts.unfinished = unfinished;
    facts.ff_notice = ff.as_ref().and_then(ff_notice);
    // §9: a failed check or fetch still leaves `--ff` a local tracking ref to
    // move to, and the caption keeps the reason. Only a render while the
    // worker may yet fetch (still checking or still pulling) is excluded,
    // since the comparison is about to change.
    facts.ff_suggestion = match (status, facts.caption.map(|caption| caption.state())) {
        (Some(RemoteStatus::StillChecking { .. } | RemoteStatus::StillPulling), _) => None,
        (Some(_), Some(CaptionState::Behind(behind))) if !flags.fast_forward => Some(FfSuggestion { behind }),
        _ => None,
    };
    let table = list_table::render(&facts, terminal, now);
    if let Some(start) = t0 {
        perf::record(&mut collector, "table render", start.elapsed());
    }

    let graph = graph_facts.map(|facts| {
        let t0 = perf.then(Instant::now);
        let graph = facts.to_git_graph(&remote.prs, parsed_width.clone()).render(&image_terminal(terminal));
        if let Some(start) = t0 {
            perf::record(&mut collector, "graph image render (biscuit-terminal)", start.elapsed());
        }
        graph
    });
    let verbose_text = verbose_data.map(|data| {
        let t0 = perf.then(Instant::now);
        let text = render_verbose(&data, terminal);
        if let Some(start) = t0 {
            perf::record(&mut collector, "verbose render", start.elapsed());
        }
        text
    });
    let hint = list_table::render_hint(&facts, terminal);
    let notes = list_table::render_notes(&facts, terminal);
    eprint!(
        "{}",
        list_table::assemble(Sections {
            table: &table,
            graph: graph.as_deref(),
            hint: hint.as_deref(),
            verbose: verbose_text.as_deref(),
            notes: notes.as_deref(),
        })
    );

    Ok(collector)
}

/// §6: the wait ran out, or a PR refresh is still running at render time.
///
/// A PR refresh can be running only while the stored answer is not fresh.
/// While our worker lives it may be the one refreshing; once it has exited,
/// only another holder of the PR lock can be, and probing is then safe.
fn unfinished(remote: &mut RemoteAnswers, pr_store: Option<&Path>, now: u64) -> bool {
    let (Some(origin), Some(waited), Some(pr_store)) = (&remote.origin, &mut remote.waited, pr_store) else {
        return false;
    };
    if matches!(waited.end, WaitEnd::TimedOut { .. }) {
        return true;
    }
    let pr_pending =
        !remote.ignored && !matches!(select_cached(pr_store, Some(origin), now), CachedPrs::Fresh(_));
    let worker_running = waited.worker.as_mut().is_some_and(|worker| !worker.has_exited());
    pr_pending && (worker_running || pr_lock_held(pr_store))
}

fn render_verbose(data: &git_graph::VerboseData, terminal: &Terminal) -> String {
    let default_branch = Prose::escape_text(&data.default_branch);
    let branch = Prose::escape_text(&data.branch);
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };

    // Main section: the commit where the worktree branched from
    let main_heading = Prose::new(format!("<b><blue-500>{default_branch}</blue-500></b>"));
    line(main_heading.render(terminal));

    if let Some(commit) = &data.merge_base {
        let mut main_list = UnorderedList::empty();
        main_list.add(Prose::new(git_graph::format_commit(commit)));
        line(main_list.render(terminal));
    }

    // Worktree section: all commits since the branch point
    let branch_heading = Prose::new(format!("<b><yellow-500>{branch}</yellow-500></b>"));
    line(branch_heading.render(terminal));

    if data.branch_commits.is_empty() {
        let mut empty_list = UnorderedList::empty();
        empty_list.add(Prose::new(
            "<dim>no commits since branching</dim>".to_string(),
        ));
        line(empty_list.render(terminal));
    } else {
        let mut branch_list = UnorderedList::empty();
        for commit in &data.branch_commits {
            branch_list.add(Prose::new(git_graph::format_commit(commit)));
        }
        line(branch_list.render(terminal));
    }
    out
}

/// Build a Terminal suitable for rendering images to stderr.
///
/// `Terminal::default()` calls `is_tty()` which checks stdout. When the shell
/// wrapper captures stdout via `$()`, stdout is a pipe and image support is
/// suppressed. This builds a terminal that uses stderr for the TTY check and
/// detects image support from `$TERM_PROGRAM` env vars instead.
fn image_terminal(_base: &Terminal) -> Terminal {
    let stderr_is_tty = std::io::stderr().is_terminal();
    let img_support = if stderr_is_tty {
        detect_image_support_from_env()
    } else {
        ImageSupport::None
    };
    Terminal::builder()
        .is_tty(stderr_is_tty)
        .image_support(img_support)
        .build()
}

fn detect_image_support_from_env() -> ImageSupport {
    match std::env::var("TERM_PROGRAM").as_deref() {
        Ok("ghostty") | Ok("kitty") | Ok("WezTerm") | Ok("Warp") | Ok("WarpTerminal")
        | Ok("konsole") | Ok("wast") => ImageSupport::Kitty,
        Ok("iTerm.app") => ImageSupport::ITerm,
        _ => {
            if std::env::var("KITTY_WINDOW_ID").is_ok() {
                ImageSupport::Kitty
            } else {
                ImageSupport::None
            }
        }
    }
}

#[cfg(test)]
mod tests;
