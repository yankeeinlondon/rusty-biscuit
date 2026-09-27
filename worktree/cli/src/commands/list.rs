use std::io::IsTerminal as _;
use std::path::Path;
use std::time::{Duration, Instant};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::terminal_image::parse_width_spec;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::WorktreeError;
use worktree::pull_requests::{
    CachedPrs, LIST_DEADLINE, OpenPrSource, PrListing, SniffOpenPrSource, fetch_and_publish, origin_url,
    pr_store_path, select_cached, unix_now,
};
use worktree::remote_head::{CachedRemoteHead, remote_head_store_path, select_cached_head};
use worktree::worktree::{fill_worktree_statuses, parse_worktree_state};

use super::git_graph;
use super::list_table::{self, RemoteFacts, TableFacts};
use crate::perf;

/// Builds the open-PR source for an `origin` URL when a request is due.
pub type PrConnect = fn(&str) -> Box<dyn OpenPrSource>;

/// The production source: sniff's provider client for `origin`.
fn origin_pr_source(origin: &str) -> Box<dyn OpenPrSource> {
    Box::new(SniffOpenPrSource {
        remote_url: origin.to_string(),
        deadline: LIST_DEADLINE,
    })
}

/// Starts the background refresh (`wt internal-refresh`) for the repository
/// whose main checkout is the argument, without waiting for it.
pub type RefreshLaunch = fn(&Path);

/// How listing reaches the network: a foreground PR request on a PR miss, and
/// one background refresh for whatever is stale or missing. There is no
/// live-head seam, because listing never asks for the live head itself.
/// Tests replace both.
#[derive(Clone, Copy)]
pub struct ListSeams {
    pub connect: PrConnect,
    pub launch: RefreshLaunch,
}

const PRODUCTION_SEAMS: ListSeams = ListSeams {
    connect: origin_pr_source,
    launch: super::refresh_worker::launch,
};

/// Where the two stored answers live for one main checkout.
#[derive(Clone, Copy)]
struct Stores<'a> {
    prs: &'a Path,
    head: &'a Path,
}

/// What the remote stage hands the table.
#[derive(Default)]
struct RemoteAnswers {
    prs: PrListing,
    /// The stored live-head answer; `None` without an `origin`.
    head: Option<CachedRemoteHead>,
    origin_present: bool,
    /// The `pr gather` perf stage: the origin lookup, PR selection, and any
    /// foreground PR request.
    pr_gather: Duration,
    /// The `remote select` perf stage: live-head selection and the launch.
    remote_select: Duration,
}

/// The stored answers for the repository whose main checkout is `main`, with
/// at most one background refresh launched.
///
/// A stored PR answer for the current `origin` is shown at once. Only a PR
/// miss makes a request here, under [`LIST_DEADLINE`], and it settles before
/// the launch decision, so the worker's PR half never races it; a failed
/// request, or one during which `origin` changed, shows no badges. The
/// worker is launched once when the PR answer is stale, or when there is an
/// `origin` and the live-head answer is not fresh; a PR miss alone launches
/// nothing. The live head is never requested in the foreground.
fn gather_remote(stores: Stores<'_>, main: &Path, default_branch: &str, seams: ListSeams) -> RemoteAnswers {
    let t0 = Instant::now();
    let origin = origin_url(main);
    let (prs, pr_stale) = match select_cached(stores.prs, origin.as_deref(), unix_now()) {
        CachedPrs::Fresh(listing) => (listing, false),
        CachedPrs::Stale(listing) => (listing, true),
        CachedPrs::Miss => {
            let listing = origin
                .as_deref()
                .and_then(|origin| {
                    fetch_and_publish(stores.prs, main, origin, unix_now(), (seams.connect)(origin).as_ref())
                })
                .unwrap_or_default();
            (listing, false)
        }
    };
    let pr_gather = t0.elapsed();
    let t0 = Instant::now();
    let head = origin
        .as_deref()
        .map(|origin| select_cached_head(stores.head, Some(origin), Some(default_branch), unix_now()));
    let head_due = head.as_ref().is_some_and(|head| !matches!(head, CachedRemoteHead::Fresh(_)));
    if pr_stale || head_due {
        (seams.launch)(main);
    }
    RemoteAnswers {
        prs,
        head,
        origin_present: origin.is_some(),
        pr_gather,
        remote_select: t0.elapsed(),
    }
}

pub fn run(
    width_spec: Option<&str>,
    verbose: bool,
    perf: bool,
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

fn run_pipeline(
    width_spec: Option<&str>,
    verbose: bool,
    perf: bool,
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
    let needs_graph = image_support != ImageSupport::None;
    let gather_input = git_graph::GatherInput::from_list(&list);
    let needs_verbose = verbose && gather_input.has_verbose();
    let main_checkout = list.entries().first().map(|main| main.path.clone());
    let pr_store = main_checkout.as_deref().and_then(|main| pr_store_path(main).ok());
    let head_store = main_checkout.as_deref().and_then(|main| remote_head_store_path(main).ok());
    let default_branch = list.default_branch.clone();

    std::thread::scope(|scope| {
        // A PR request (only on a miss) runs beside the git work under its
        // own deadline, so the network never holds the table up for longer
        // than that.
        let remote_handle = scope.spawn(|| match (&pr_store, &head_store, &main_checkout) {
            (Some(prs), Some(head), Some(main)) => gather_remote(Stores { prs, head }, main, &default_branch, seams),
            _ => RemoteAnswers::default(),
        });
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

        let remote = remote_handle.join().expect("remote thread panicked");
        if perf {
            perf::record(&mut collector, "pr gather", remote.pr_gather);
            perf::record(&mut collector, "remote select", remote.remote_select);
        }
        let prs = remote.prs;

        let (graph_facts, verbose_data) = match graph_handle {
            Some(handle) => {
                let (data, elapsed) = handle.join().expect("graph gather thread panicked");
                if let Some(elapsed) = elapsed {
                    let stage = if needs_graph { "graph gather" } else { "verbose gather" };
                    perf::record(&mut collector, stage, elapsed);
                }
                data
            }
            None => (None, None),
        };

        let t0 = perf.then(Instant::now);
        let answer = match &remote.head {
            Some(CachedRemoteHead::Fresh(head) | CachedRemoteHead::Stale(head)) => Some(head),
            _ => None,
        };
        let tracking_ref = format!("origin/{}", list.default_branch);
        let remote_facts = remote.origin_present.then(|| RemoteFacts {
            default_branch: &list.default_branch,
            tracking_tip: list
                .caption
                .as_ref()
                .map(|caption| caption.tracking_sha.as_str())
                .or_else(|| list.refs().remote(&tracking_ref)),
            answer,
        });
        let facts = TableFacts::from_list(&list, &prs, remote_facts);
        eprint!("{}", list_table::render(&facts, terminal, unix_now()));
        if let Some(start) = t0 {
            perf::record(&mut collector, "table render", start.elapsed());
        }

        if let Some(facts) = graph_facts {
            let t0 = perf.then(Instant::now);
            let graph = facts.to_git_graph(&prs, parsed_width.clone());
            eprint!("{}", graph.render(&image_terminal(terminal)));
            if let Some(start) = t0 {
                perf::record(&mut collector, "graph image render (biscuit-terminal)", start.elapsed());
            }
        }

        if let Some(data) = verbose_data {
            let t0 = perf.then(Instant::now);
            render_verbose(&data, terminal);
            if let Some(start) = t0 {
                perf::record(&mut collector, "verbose render", start.elapsed());
            }
        }

        Ok(collector)
    })
}

fn render_verbose(data: &git_graph::VerboseData, terminal: &Terminal) {
    let default_branch = Prose::escape_text(&data.default_branch);
    let branch = Prose::escape_text(&data.branch);

    // Main section: the commit where the worktree branched from
    let main_heading = Prose::new(format!("<b><blue-500>{default_branch}</blue-500></b>"));
    eprintln!("{}", main_heading.render(terminal));

    if let Some(commit) = &data.merge_base {
        let mut main_list = UnorderedList::empty();
        main_list.add(Prose::new(git_graph::format_commit(commit)));
        eprintln!("{}", main_list.render(terminal));
    }

    // Worktree section: all commits since the branch point
    let branch_heading = Prose::new(format!("<b><yellow-500>{branch}</yellow-500></b>"));
    eprintln!("{}", branch_heading.render(terminal));

    if data.branch_commits.is_empty() {
        let mut empty_list = UnorderedList::empty();
        empty_list.add(Prose::new(
            "<dim>no commits since branching</dim>".to_string(),
        ));
        eprintln!("{}", empty_list.render(terminal));
    } else {
        let mut branch_list = UnorderedList::empty();
        for commit in &data.branch_commits {
            branch_list.add(Prose::new(git_graph::format_commit(commit)));
        }
        eprintln!("{}", branch_list.render(terminal));
    }
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
