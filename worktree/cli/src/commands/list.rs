use std::io::IsTerminal as _;
use std::time::{Duration, Instant};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::terminal_image::parse_width_spec;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use sniff::remote::blocking::credential_env;
use worktree::WorktreeError;
use worktree::fast_forward::{FfRefusal, FfResult};
use worktree::list::{ListOptions, Listing, Observed, RemoteAnswers, WaitProgress};
use worktree::listing::CaptionState;
use worktree::live_remote::tracking_ref_changed_at;
use worktree::pull_requests::unix_now;
use worktree::remote_head::{
    ApiCondition, ApiNote, Attempt, CachedRemoteHead, CheckFailure, CredentialEvidence, Outcome, Phase, PrFailure,
    select_cached_head,
};
use worktree::timing::{Scope, Span, SpanList, Stage, Timings};

use super::git_graph;
use super::list_table::{
    self, CredentialCondition, CredentialLine, FfNotice, FfSuggestion, GraphOmissions, LastKnown, PrOutcome, RemoteFacts,
    RemoteStatus, Sections, TableFacts,
};
use crate::args::PerfFormat;
use crate::perf;

mod progress;

use progress::Progress;
use worktree::list::wait::{HeadEnd, PrEnd};
pub use worktree::list::wait::{FORCED_BUDGET, LaunchArgs, ORDINARY_BUDGET, WorkerHandle, WorkerLaunch};

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

/// The caption's head status: the followed attempt
/// ([`remote_status`]), or, for an `origin` replaced or removed during the
/// wait, the row a worker that noticed the change itself gives
/// (`couldn't check origin`). `None` when nothing was launched.
fn caption_status(remote: &RemoteAnswers, last: impl Fn() -> LastKnown) -> Option<RemoteStatus> {
    remote.waited.as_ref()?;
    Some(match remote.observed() {
        Some(observed) => remote_status(&observed.waited.head, last),
        None => RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: last() },
    })
}

/// What the status list and the notes say about this listing's requests.
#[derive(Debug, Default, PartialEq)]
struct RequestNotices {
    pr_outcome: Option<PrOutcome>,
    credential_line: Option<CredentialLine>,
    fallback_notice: Option<Vec<String>>,
}

/// Every request notice for `remote`, all behind the one
/// [`RemoteAnswers::observed`] guard: nothing without a wait or after
/// `origin` changed.
fn request_notices(remote: &RemoteAnswers) -> RequestNotices {
    let Some(observed) = remote.observed() else {
        return RequestNotices::default();
    };
    let attempt = followed_attempt(&observed.waited.head);
    RequestNotices {
        pr_outcome: Some(pr_outcome(&observed)),
        credential_line: credential_line(
            observed.origin,
            attempt,
            observed_pr_failure(&observed),
            observed_keyless(&observed),
        ),
        fallback_notice: fallback_notice(observed.origin, attempt),
    }
}

/// This run's PR half as the status list and badges present it (§5). An
/// ignored repository is ignored whatever the wait saw.
fn pr_outcome(observed: &Observed<'_>) -> PrOutcome {
    if observed.ignored {
        return PrOutcome::Ignored;
    }
    match observed.waited.prs {
        PrEnd::Published => PrOutcome::Published,
        PrEnd::Ignored => PrOutcome::Ignored,
        PrEnd::Unsupported => PrOutcome::Unsupported,
        PrEnd::Failed(_) => PrOutcome::Failed,
        PrEnd::Pending => PrOutcome::Pending,
    }
}

/// This run's PR failure, from its receipt.
fn observed_pr_failure<'a>(observed: &Observed<'a>) -> Option<&'a PrFailure> {
    match &observed.waited.prs {
        PrEnd::Failed(failure) => Some(failure),
        _ => None,
    }
}

/// Whether this listing observed a successful API answer sent without a key,
/// in either half: the followed head attempt's check, or the PR publication
/// the wait accepted.
///
/// Only evidence recorded by the worker for those exact results counts; a
/// cached answer, unknown credentials, an ignored repository, and a check
/// whose worker saw `origin` change never do. Evidence published after the
/// wait returned was never read.
fn observed_keyless(observed: &Observed<'_>) -> bool {
    if observed.ignored {
        return false;
    }
    let waited = observed.waited;
    let head = followed_attempt(&waited.head).is_some_and(|attempt| {
        attempt.credentials == CredentialEvidence::Anonymous
            && !matches!(attempt.outcome, Some(Outcome::Unavailable { .. }))
    });
    let prs = waited.prs == PrEnd::Published && waited.pr_credentials == CredentialEvidence::Anonymous;
    head || prs
}

/// Providers whose documentation gives an API key a higher rate limit than
/// an anonymous request on the hosts the blocking lookups ask: github.com,
/// gitlab.com, and bitbucket.org. Gitea and Forgejo set no such default, so
/// their notice promises only authentication.
fn keyed_limits_are_higher(provider: &str) -> bool {
    matches!(provider, "GitHub" | "GitLab" | "Bitbucket")
}

/// The one credentials line for what this run observed: the attempt's
/// confirmed API condition first, then this run's PR failure, from its
/// receipt, and only then the keyless notice when `keyless`
/// ([`observed_keyless`]).
fn credential_line(
    origin: &str,
    attempt: Option<&Attempt>,
    pr_failure: Option<&PrFailure>,
    keyless: bool,
) -> Option<CredentialLine> {
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
    .or_else(|| {
        keyless.then(|| {
            line(CredentialCondition::AnsweredWithoutKey { higher_limits: keyed_limits_are_higher(&env.provider) }, &None)
        })
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
        FfRefusal::UnavailableHolder(path) => FfNotice::UnavailableHolder(path.clone()),
        FfRefusal::Other => FfNotice::Failed,
    })
}

pub fn run(
    width_spec: Option<&str>,
    verbose: bool,
    perf: Option<PerfFormat>,
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
    let timings = run_pipeline(
        width_spec,
        verbose,
        perf.is_some(),
        flags,
        process_start,
        image_support,
        &terminal,
        PRODUCTION_SEAMS,
    )?;
    match (perf, timings) {
        (Some(PerfFormat::Human), Some(timings)) => eprint!("{}", perf::human_report(&timings)),
        (Some(PerfFormat::Json), Some(timings)) => eprint!("{}", perf::json_record(&timings)),
        _ => {}
    }
    Ok(())
}

/// Gathers the listing through [`worktree::list::gather`] from the current
/// directory, renders it, and writes it to stderr.
///
/// With `perf`, returns the [`Scope::Command`] timings: from `process_start`
/// to the listing's write, with the library's spans in place between
/// `startup` and the render stages. Reporting them is the caller's, so the
/// report never times itself.
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
) -> Result<Option<Timings>, WorktreeError> {
    let mut steps = perf.then(SpanList::sequential);
    if let Some(steps) = steps.as_mut() {
        steps.push(Span::new(Stage::Startup, process_start.elapsed()));
    }

    let parsed_width = width_spec.and_then(|s| parse_width_spec(s).ok());
    let options = ListOptions {
        refresh: flags.refresh,
        ignore_api: flags.ignore_api,
        fast_forward: flags.fast_forward,
        worker: Some(seams.launch),
        wait_budget: seams.wait_budget,
        forced_budget: seams.forced_budget,
        graph: image_support != ImageSupport::None,
        verbose,
        timings: perf,
    };
    let repo = std::env::current_dir()?;
    let mut progress = None;
    let listing = worktree::list::gather(&repo, &options, &mut |event| match event {
        WaitProgress::Started => progress = Some(Progress::on_stderr()),
        WaitProgress::Phase(phase) => {
            if let Some(progress) = &progress {
                progress.show(phase);
            }
        }
        WaitProgress::Finished => {
            if let Some(progress) = progress.take() {
                progress.finish();
            }
        }
    });
    if let Some(progress) = progress {
        progress.finish();
    }
    let Listing { list, remote, ff, graph: graph_facts, verbose: verbose_data, main_checkout, head_store, timings, .. } =
        listing?;
    if let (Some(steps), Some(library)) = (steps.as_mut(), timings) {
        // The library's top-level spans are sequential parts of this command
        // too; its own total is not a span of its own.
        for span in library.spans() {
            steps.push(span.clone());
        }
    }
    let default_branch = list.default_branch.clone();

    let now = unix_now();
    let status = perf::time(steps.as_mut(), Stage::CaptionStatus, || match (&remote.origin, &head_store, &main_checkout) {
        (Some(origin), Some(head_store), Some(main)) => caption_status(&remote, || {
            // The stored answer is bound to the `origin` this listing
            // launched for; after it changed, only the reflog still dates
            // anything true of this repository.
            let stored = if remote.origin_changed {
                CachedRemoteHead::Miss
            } else {
                select_cached_head(head_store, Some(origin), Some(&default_branch), now)
            };
            match stored {
                CachedRemoteHead::Fresh(head) | CachedRemoteHead::Stale(head) => {
                    LastKnown::Answer { checked_at: head.checked_at }
                }
                // The reflog runs only without a stored answer.
                CachedRemoteHead::Miss => match tracking_ref_changed_at(main, &default_branch) {
                    Some(at) => LastKnown::TrackingRefChanged { at },
                    None => LastKnown::Never,
                },
            }
        }),
        _ => None,
    });
    // Outside the timed step only because the facts borrow it.
    let tracking_ref = format!("origin/{}", list.default_branch);
    let tracking_tip = list
        .caption
        .as_ref()
        .map(|caption| caption.tracking_sha.clone())
        .or_else(|| list.refs().remote(&tracking_ref).map(str::to_string));
    let mut facts = perf::time(steps.as_mut(), Stage::DisplayFacts, || {
        let remote_facts = status.map(|status| RemoteFacts {
            default_branch: &list.default_branch,
            tracking_tip: tracking_tip.as_deref(),
            status,
        });
        let mut facts = TableFacts::from_list(&list, &remote.prs, remote_facts);
        let notices = request_notices(&remote);
        facts.pr_outcome = notices.pr_outcome;
        facts.credential_line = notices.credential_line;
        facts.fallback_notice = notices.fallback_notice;
        facts.timed_out = remote.waited.as_ref().is_some_and(|waited| waited.timed_out);
        facts.ff_notice = ff.as_ref().and_then(ff_notice);
        // §9: a failed check or fetch still leaves `--ff` a local tracking ref
        // to move to, and the caption keeps the reason. Only a render while
        // the worker may yet fetch (still checking or still pulling) is
        // excluded, since the comparison is about to change.
        facts.ff_suggestion = match (status, facts.caption.map(|caption| caption.state())) {
            (Some(RemoteStatus::StillChecking { .. } | RemoteStatus::StillPulling), _) => None,
            (Some(_), Some(CaptionState::Behind(behind))) if !flags.fast_forward => Some(FfSuggestion { behind }),
            _ => None,
        };
        facts
    });
    let table = perf::time(steps.as_mut(), Stage::TableRender, || list_table::render(&facts, terminal, now));
    let verbose_text =
        verbose_data.map(|data| perf::time(steps.as_mut(), Stage::VerboseRender, || render_verbose(&data, terminal)));
    let (status, preliminary_notes) = perf::time(steps.as_mut(), Stage::NotesRender, || {
        (list_table::render_status(&facts, terminal, now), list_table::render_notes(&facts, terminal))
    });
    // Everything but the graph is known now, so the graph gets the rows the
    // rest of the listing leaves (the notes before the graph adds its own).
    let (max_graph_rows, badges) = perf::time(steps.as_mut(), Stage::GraphBudget, || {
        let rows = list_table::graph_row_budget(
            terminal.height(),
            &[Some(table.as_str()), status.as_deref(), verbose_text.as_deref(), preliminary_notes.as_deref()],
        );
        (rows, facts.badges())
    });
    let graph = graph_facts.and_then(|graph_facts| {
        let graph = perf::time(steps.as_mut(), Stage::GraphRender, || {
            git_graph::to_git_graph(&graph_facts, badges, parsed_width.clone())
                .with_max_rows(max_graph_rows)
                .render_without_notes(&image_terminal(terminal))
        });
        graph.map(|graph| (graph, graph_facts))
    });
    let notes = perf::time(steps.as_mut(), Stage::FinalNotesRender, || {
        let graph = graph.map(|((image, plan), graph_facts)| {
            facts.graph_omissions = GraphOmissions {
                hidden_lanes: plan.hidden_lanes,
                omissions: plan.omissions,
                history_gap: graph_facts.incomplete,
                shallow: graph_facts.shallow,
                merged_elsewhere: graph_facts.merged_elsewhere,
                forked_off_line: graph_facts.forked_off_line,
            };
            image
        });
        (graph, list_table::render_notes(&facts, terminal))
    });
    let (graph, notes) = notes;
    perf::time(steps.as_mut(), Stage::WriteOutput, || {
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
    });

    Ok(steps.map(|steps| Timings::new(Scope::Command, process_start.elapsed(), steps)))
}

fn render_verbose(data: &worktree::graph::VerboseData, terminal: &Terminal) -> String {
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
