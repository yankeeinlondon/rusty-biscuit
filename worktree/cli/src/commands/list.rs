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
use worktree::pull_requests::{CachedPrs, PrListing, origin_digest, origin_url, pr_store_path, select_cached, unix_now};
use worktree::remote_head::{
    ApiCondition, ApiNote, Attempt, CachedRemoteHead, CheckFailure, Outcome, Phase, PrFailure,
    remote_head_store_path, select_cached_head,
};
use worktree::worktree::{fill_worktree_statuses, parse_worktree_state};

use super::git_graph;
use super::list_table::{
    self, CredentialCondition, CredentialLine, FfNotice, FfSuggestion, LastKnown, PrOutcome, RemoteFacts,
    RemoteStatus, Sections, TableFacts,
};
use crate::perf;

mod wait;

pub use wait::{FORCED_BUDGET, LaunchArgs, ORDINARY_BUDGET, WorkerHandle, WorkerLaunch};
use wait::{HeadEnd, PrEnd, Progress, StoreEnv, WaitEnd, WaitRequest};

/// The flags that apply only to listing (spec §7–§9).
#[derive(Debug, Clone, Copy, Default)]
pub struct ListFlags {
    /// `-r`/`--refresh`: wait for the whole update and PR refresh.
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

/// How listing reaches the network: only through the worker it launches and
/// waits for. Tests replace the launch and shorten the waits.
#[derive(Clone, Copy)]
pub struct ListSeams {
    pub launch: WorkerLaunch,
    /// Ordinary listing's wait for both halves.
    pub wait_budget: Duration,
    /// The `--refresh`/`--ff` wait.
    pub forced_budget: Duration,
}

const PRODUCTION_SEAMS: ListSeams = ListSeams {
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
    origin: Option<String>,
    /// The repository is in `~/.wt.json`: no provider request, no badges.
    ignored: bool,
    /// `origin` changed or disappeared during the wait: the PR answer and
    /// this run's PR result belong to the old one and are not shown.
    origin_changed: bool,
    /// `None` without an `origin`: nothing was launched.
    waited: Option<WaitEnd>,
    /// The `pr gather` perf stage: the origin lookup, and the origin recheck
    /// and stored PR answer read after the wait.
    pr_gather: Duration,
    /// The `remote wait` perf stage: the launch and the wait.
    remote_wait: Duration,
}

/// The remote stage for the repository whose main checkout is `main`: one
/// worker launched and waited for (both halves, or the head attempt it
/// adopts), then the stored PR answer as the wait left it.
///
/// The worker is the only PR writer; nothing here asks the network.
/// `--ignore-api` records the repository before the launch. Without an
/// `origin` nothing is launched, and stored answers are ignored.
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
    let mut pr_gather = t0.elapsed();

    let t0 = Instant::now();
    let digest = origin_digest(&origin);
    let request = WaitRequest {
        main,
        origin_digest: &digest,
        branch: default_branch,
        force: flags.forced(),
        budget: if flags.forced() { seams.forced_budget } else { seams.wait_budget },
    };
    let env = StoreEnv::new(stores.head.to_path_buf(), stores.prs.to_path_buf(), origin.clone());
    let progress = Progress::on_stderr();
    let waited = wait::wait(request, &env, seams.launch, &mut |phase| progress.show(phase));
    progress.finish();
    let remote_wait = t0.elapsed();

    // Read on every exit path: the worker may have published while we
    // waited, and an answer for an `origin` replaced meanwhile is not shown.
    let t0 = Instant::now();
    let origin_changed = origin_url(main).as_deref() != Some(origin.as_str());
    let prs = match select_cached(stores.prs, Some(&origin), unix_now()) {
        CachedPrs::Fresh(listing) | CachedPrs::Stale(listing) if !ignored && !origin_changed => listing,
        _ => PrListing::default(),
    };
    pr_gather += t0.elapsed();
    Ok(RemoteAnswers { prs, origin: Some(origin), ignored, origin_changed, waited: Some(waited), pr_gather, remote_wait })
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

/// The followed head attempt as the wait left it, and what the caption says
/// of it; the PR half never changes it.
fn remote_status(head: &HeadEnd, last: impl Fn() -> LastKnown) -> RemoteStatus {
    let failed = |reason| RemoteStatus::CheckFailed { reason, last: last() };
    match head {
        HeadEnd::Finished(attempt) => match attempt.outcome {
            Some(Outcome::InSync) => RemoteStatus::CheckedNow,
            Some(Outcome::Fetched) => RemoteStatus::Fetched,
            Some(Outcome::Absent) => RemoteStatus::Absent,
            Some(Outcome::FetchFailed { reason }) => RemoteStatus::FetchFailed { reason },
            Some(Outcome::CheckFailed { reason }) => failed(reason),
            Some(Outcome::Unavailable { .. }) | None => failed(CheckFailure::Other),
        },
        HeadEnd::Running { last: Some(Attempt { phase: Phase::Fetching, .. }) } => RemoteStatus::StillPulling,
        HeadEnd::Running { .. } => RemoteStatus::StillChecking { last: last() },
        HeadEnd::Unavailable => failed(CheckFailure::Other),
    }
}

/// This run's head attempt, if it got far enough to record one.
fn followed_attempt(head: &HeadEnd) -> Option<&Attempt> {
    match head {
        HeadEnd::Finished(attempt) => Some(attempt),
        HeadEnd::Running { last } => last.as_ref(),
        HeadEnd::Unavailable => None,
    }
}

/// This run's PR half as the status list and badges present it (§5). An
/// ignored repository is ignored whatever the wait saw, and a changed
/// `origin` leaves nothing to say about the old one.
fn pr_outcome(remote: &RemoteAnswers) -> Option<PrOutcome> {
    let end = remote.waited.as_ref()?;
    if remote.origin_changed {
        return None;
    }
    if remote.ignored {
        return Some(PrOutcome::Ignored);
    }
    Some(match end.prs {
        PrEnd::Published => PrOutcome::Published,
        PrEnd::Ignored => PrOutcome::Ignored,
        PrEnd::Unsupported => PrOutcome::Unsupported,
        PrEnd::Failed(_) => PrOutcome::Failed,
        PrEnd::Pending => PrOutcome::Pending,
    })
}

/// This run's PR failure, from its receipt, unless it was about an `origin`
/// replaced during the wait.
fn observed_pr_failure(remote: &RemoteAnswers) -> Option<&PrFailure> {
    match &remote.waited.as_ref()?.prs {
        PrEnd::Failed(failure) if !remote.origin_changed => Some(failure),
        _ => None,
    }
}

/// The §5 line for what this run observed: the attempt's API note first,
/// then this run's PR failure, from its receipt.
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

    let remote = match (&pr_store, &head_store, &main_checkout) {
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
        (Some(origin), Some(waited), Some(head_store), Some(main)) => Some(remote_status(&waited.head, || {
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
    let mut facts = TableFacts::from_list(&list, &remote.prs, remote_facts);
    facts.pr_outcome = pr_outcome(&remote);
    if let (Some(origin), Some(waited)) = (&remote.origin, &remote.waited) {
        let attempt = followed_attempt(&waited.head);
        facts.credential_line = credential_line(origin, attempt, observed_pr_failure(&remote));
        facts.fallback_notice = fallback_notice(origin, attempt);
        facts.timed_out = waited.timed_out;
    }
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

    let badges = facts.badges();
    let graph = graph_facts.map(|graph_facts| {
        let t0 = perf.then(Instant::now);
        let graph = graph_facts.to_git_graph(badges, parsed_width.clone()).render(&image_terminal(terminal));
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
    let status = list_table::render_status(&facts, terminal, now);
    let notes = list_table::render_notes(&facts, terminal);
    eprint!(
        "{}",
        list_table::assemble(Sections {
            table: &table,
            graph: graph.as_deref(),
            status: status.as_deref(),
            verbose: verbose_text.as_deref(),
            notes: notes.as_deref(),
        })
    );

    Ok(collector)
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
