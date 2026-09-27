use std::io::IsTerminal as _;
use std::time::Instant;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::terminal_image::parse_width_spec;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::WorktreeError;
use worktree::pull_requests::{
    OpenPrSource, PrListing, SniffOpenPrSource, open_pull_requests, pr_store_path, unix_now,
};
use worktree::worktree::{fill_worktree_statuses, parse_worktree_state};

use super::git_graph;
use super::list_table::{self, TableFacts};
use crate::perf;

/// Builds the open-PR source when a request is due; `None` shows no badges.
pub type PrConnect = fn() -> Option<Box<dyn OpenPrSource>>;

/// The production source: sniff's provider client for `origin`.
fn origin_pr_source() -> Option<Box<dyn OpenPrSource>> {
    SniffOpenPrSource::for_origin().map(|source| Box::new(source) as Box<dyn OpenPrSource>)
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
        origin_pr_source,
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
    pr_connect: PrConnect,
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
    let pr_store = list.entries().first().and_then(|main| pr_store_path(&main.path).ok());

    std::thread::scope(|scope| {
        // The PR request runs beside the git work under its own deadline, so
        // the network never holds the table up for longer than that.
        let pr_handle = scope.spawn(|| {
            let t0 = perf.then(Instant::now);
            let prs = match &pr_store {
                Some(store) => open_pull_requests(store, unix_now(), pr_connect),
                None => PrListing::default(),
            };
            (prs, t0.map(|start| start.elapsed()))
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

        let (prs, pr_elapsed) = pr_handle.join().expect("PR thread panicked");
        if let Some(elapsed) = pr_elapsed {
            perf::record(&mut collector, "pr gather", elapsed);
        }

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
        let facts = TableFacts::from_list(&list, &prs);
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
