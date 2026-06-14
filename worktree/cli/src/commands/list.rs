use std::io::IsTerminal as _;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::terminal_image::{ImageWidth, parse_width_spec};
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use worktree::WorktreeError;
use worktree::worktree::{DirtyStatus, WorktreeStatus, list_worktrees};

use super::git_graph;

const MIN_GRAPH_TERMINAL_WIDTH: u32 = 80;

pub fn run(width_spec: Option<&str>, verbose: bool) -> Result<(), WorktreeError> {
    let list = list_worktrees()?;
    let statuses = &list.statuses;
    let terminal = Terminal::default();

    let table = build_status_table(statuses, &terminal);
    eprintln!("\n{}", table.render(&terminal));

    // Detect image support from the real stderr TTY + env. Width and verbose
    // gating live inside `gather_extras` so the same decision is exercised by
    // tests with a forced terminal width.
    let stderr_is_tty = std::io::stderr().is_terminal();
    let image_support = if stderr_is_tty {
        detect_image_support_from_env()
    } else {
        ImageSupport::None
    };
    let parsed_width = width_spec.and_then(|s| parse_width_spec(s).ok());

    let (needs_graph, needs_verbose, current_branch_data, base_graph_data) = gather_extras(
        &list.default_branch,
        statuses,
        &terminal,
        image_support,
        &parsed_width,
        verbose,
    );

    // Generate graph instructions and render when image support is present.
    if needs_graph {
        let instructions = graph_instructions(
            statuses,
            &list.default_branch,
            current_branch_data.as_ref(),
            base_graph_data.as_ref(),
        );
        if let Some(instructions) = instructions {
            let commit_count = instructions
                .lines()
                .filter(|l| l.contains("commit id:"))
                .count();

            let graph_width = parsed_width
                .unwrap_or_else(|| default_graph_width(commit_count, terminal.width()));

            let fits = match &graph_width {
                ImageWidth::Percent(_) | ImageWidth::Fill => true,
                ImageWidth::Characters(_) => terminal.width() >= MIN_GRAPH_TERMINAL_WIDTH,
            };

            if fits {
                let img_term = image_terminal(&terminal);
                let diagram = MermaidDiagram::new(instructions).with_width(graph_width);
                eprint!("{}", diagram.render(&img_term));
            }
        }
    }

    if needs_verbose {
        render_verbose(statuses, &list.default_branch, current_branch_data.as_ref(), &terminal);
    }

    Ok(())
}

/// Decide whether graph/verbose data is needed for this run and gather it.
///
/// `gather_data` performs the actual git work; this wrapper owns the image
/// support + width gating so the decision is exercisable from tests with a
/// forced terminal width. The real stderr-TTY and env detection stays in
/// `run` and is passed in here.
fn gather_extras(
    default_branch: &str,
    statuses: &[WorktreeStatus],
    terminal: &Terminal,
    image_support: ImageSupport,
    parsed_width: &Option<ImageWidth>,
    verbose: bool,
) -> (
    bool,
    bool,
    Option<git_graph::BranchGraphData>,
    Option<git_graph::BaseGraphData>,
) {
    let needs_graph = graph_eligible(image_support, parsed_width, terminal.width());
    let current = statuses.iter().find(|s| s.entry.is_current);
    let needs_verbose = verbose && current.is_some_and(|c| !c.entry.is_main);

    let (current_branch_data, base_graph_data) =
        gather_data(default_branch, statuses, needs_graph, needs_verbose);

    (
        needs_graph,
        needs_verbose,
        current_branch_data,
        base_graph_data,
    )
}

/// Whether graph data may render on this terminal.
///
/// Character widths and the default (no `--width`) case both require
/// `terminal_width >= MIN_GRAPH_TERMINAL_WIDTH`. On a narrower terminal the
/// later `fits` check discards a character-width result, so gathering it
/// would pay the `git_graph` subprocess cost for nothing. Percentage and fill
/// widths stay eligible whenever image support is present.
fn graph_eligible(
    image_support: ImageSupport,
    parsed_width: &Option<ImageWidth>,
    terminal_width: u32,
) -> bool {
    image_support != ImageSupport::None
        && match parsed_width {
            Some(ImageWidth::Percent(_)) | Some(ImageWidth::Fill) => true,
            _ => terminal_width >= MIN_GRAPH_TERMINAL_WIDTH,
        }
}

/// Gather graph and verbose data in a single orchestration pass.
///
/// This is the sole boundary at which [`git_graph::gather_branch`] and
/// [`git_graph::gather_base_graph`] are invoked during a `wt list` run. Both
/// graph and verbose rendering consume the result; neither issues its own git
/// calls. When both graph and verbose data are needed for the current branch,
/// [`git_graph::gather_branch`] is called exactly once — the single
/// `merge-base` it resolves is shared across both surfaces.
fn gather_data(
    default_branch: &str,
    statuses: &[WorktreeStatus],
    needs_graph: bool,
    needs_verbose: bool,
) -> (Option<git_graph::BranchGraphData>, Option<git_graph::BaseGraphData>) {
    let current = statuses.iter().find(|s| s.entry.is_current);
    let current_branch = current
        .and_then(|c| c.entry.branch.as_deref())
        .unwrap_or("HEAD");

    if needs_graph {
        if let Some(current) = current {
            if current.entry.is_main {
                let branch_names: Vec<String> = statuses
                    .iter()
                    .filter_map(|s| s.entry.branch.clone())
                    .collect();
                let base = git_graph::gather_base_graph(default_branch, &branch_names);
                return (None, base);
            } else {
                let branch =
                    git_graph::gather_branch(default_branch, current_branch, needs_verbose);
                return (branch, None);
            }
        }
    } else if needs_verbose {
        let branch = git_graph::gather_branch(default_branch, current_branch, true);
        return (branch, None);
    }

    (None, None)
}

fn graph_instructions(
    statuses: &[WorktreeStatus],
    _default_branch: &str,
    current_branch_data: Option<&git_graph::BranchGraphData>,
    base_graph_data: Option<&git_graph::BaseGraphData>,
) -> Option<String> {
    let current = statuses.iter().find(|s| s.entry.is_current)?;

    if current.entry.is_main {
        base_graph_data.and_then(git_graph::base_graph)
    } else {
        current_branch_data.and_then(git_graph::worktree_graph)
    }
}

/// Choose a default graph width based on how many commits are in the graph.
fn default_graph_width(commit_count: usize, terminal_width: u32) -> ImageWidth {
    if commit_count <= 4 {
        ImageWidth::Characters(60)
    } else if commit_count <= 8 {
        ImageWidth::Characters(80)
    } else if commit_count <= 15 {
        if terminal_width > 120 {
            ImageWidth::Characters(120)
        } else {
            ImageWidth::Percent(100.0)
        }
    } else if terminal_width >= 160 {
        ImageWidth::Characters(160)
    } else {
        ImageWidth::Percent(100.0)
    }
}

fn render_verbose(
    statuses: &[WorktreeStatus],
    default_branch: &str,
    branch_data: Option<&git_graph::BranchGraphData>,
    terminal: &Terminal,
) {
    let current = statuses.iter().find(|s| s.entry.is_current);
    let Some(current) = current else {
        return;
    };
    if current.entry.is_main {
        return;
    }
    let branch = current.entry.branch.as_deref().unwrap_or("HEAD");
    let Some(data) = branch_data else {
        return;
    };

    // Main section: the commit where the worktree branched from
    let main_heading = Prose::new(format!("<b><blue-500>{default_branch}</blue-500></b>"));
    eprintln!("{}", main_heading.render(terminal));

    if let Some(commit) = git_graph::merge_base_commit(data) {
        let mut main_list = UnorderedList::empty();
        main_list.add(Prose::new(git_graph::format_commit(commit)));
        eprintln!("{}", main_list.render(terminal));
    }

    // Worktree section: all commits since the branch point
    let branch_heading = Prose::new(format!("<b><yellow-500>{branch}</yellow-500></b>"));
    eprintln!("{}", branch_heading.render(terminal));

    let branch_commits = git_graph::branch_commits_detail(data);
    if branch_commits.is_empty() {
        let mut empty_list = UnorderedList::empty();
        empty_list.add(Prose::new(
            "<dim>no commits since branching</dim>".to_string(),
        ));
        eprintln!("{}", empty_list.render(terminal));
    } else {
        let mut branch_list = UnorderedList::empty();
        for commit in branch_commits {
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

/// Build the worktree-status table with one row per worktree.
fn build_status_table(statuses: &[WorktreeStatus], terminal: &Terminal) -> Table {
    let columns = vec![
        TableColumn::new("Worktree").with_alignment(Alignment::Center),
        TableColumn::new("Worktree Name"),
        TableColumn::new("Branch"),
        TableColumn::new("Merge").with_alignment(Alignment::Center),
        TableColumn::new("Commits").with_alignment(Alignment::Right),
    ];

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

    for status in statuses {
        table.add_row(status_row(status, terminal));
    }

    table
}

fn status_row(status: &WorktreeStatus, terminal: &Terminal) -> Vec<TableCellContent> {
    vec![
        prose_cell(&dirty_badge(status.dirty), terminal),
        prose_cell(&wt_name_markup(status), terminal),
        prose_cell(&branch_markup(status), terminal),
        prose_cell(&merge_markup(status), terminal),
        prose_cell(&commits_markup(status), terminal),
    ]
}

fn prose_cell(markup: &str, terminal: &Terminal) -> TableCellContent {
    Prose::new(markup.to_string()).render(terminal).into()
}

fn wt_name_markup(status: &WorktreeStatus) -> String {
    let name = status
        .entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(unknown)".to_string());
    if status.entry.is_main {
        let branch = status.entry.branch.as_deref().unwrap_or("HEAD");
        return format!("<dim>{branch}::(<i>{name}</i>)</dim>");
    }
    if status.entry.is_current {
        format!("<b>{name}</b>")
    } else {
        format!("<dim>{name}</dim>")
    }
}

fn branch_markup(status: &WorktreeStatus) -> String {
    let branch = status.entry.branch.as_deref().unwrap_or("(detached)");
    if status.entry.is_current {
        format!("<b>{branch}</b>")
    } else {
        format!("<dim>{branch}</dim>")
    }
}

fn merge_markup(status: &WorktreeStatus) -> String {
    if status.entry.is_main {
        return String::new();
    }
    match (status.entry.is_current, status.is_clean) {
        (true, true) => "<green>clean</green>".to_string(),
        (true, false) => "<red>conflict</red>".to_string(),
        (false, true) => "<dim>clean</dim>".to_string(),
        (false, false) => "<dim>conflict</dim>".to_string(),
    }
}

fn commits_markup(status: &WorktreeStatus) -> String {
    if status.entry.is_main || (status.ahead == 0 && status.behind == 0) {
        return String::new();
    }
    let mut parts = Vec::new();
    if status.ahead > 0 {
        parts.push(if status.entry.is_current {
            format!("<green>+{}</green>", status.ahead)
        } else {
            format!("<dim>+{}</dim>", status.ahead)
        });
    }
    if status.behind > 0 {
        parts.push(if status.entry.is_current {
            format!("<yellow>-{}</yellow>", status.behind)
        } else {
            format!("<dim>-{}</dim>", status.behind)
        });
    }
    parts.join(" ")
}

/// Render a colored badge for a worktree's working-tree dirtiness.
///
/// All three badges have the same visible width (` Clean ` / ` Dirty `) so the
/// name column lines up across rows.
fn dirty_badge(status: DirtyStatus) -> String {
    match status {
        DirtyStatus::Clean => {
            "<bg-green-900><green-300><b> Clean </b></green-300></bg-green-900>".to_string()
        }
        DirtyStatus::DirtyNonSource => {
            "<bg-yellow-900><yellow-300><b> Dirty </b></yellow-300></bg-yellow-900>".to_string()
        }
        DirtyStatus::DirtySource => {
            "<bg-red-900><red-300><b> Dirty </b></red-300></bg-red-900>".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use biscuit_terminal::components::terminal_image::ImageWidth;
    use biscuit_terminal::discovery::detection::ImageSupport;
    use biscuit_terminal::terminal::Terminal;
    use worktree::git::recorder;
    use worktree::worktree::{DirtyStatus, WorktreeEntry, WorktreeStatus, list_worktrees};

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .expect("git should be installed");
        assert!(status.success(), "git {:?} failed in {:?}", args, repo);
    }

    struct DirGuard {
        old: PathBuf,
    }

    impl DirGuard {
        fn enter(dir: &Path) -> Self {
            let old = std::env::current_dir().expect("get cwd");
            std::env::set_current_dir(dir).expect("set cwd");
            DirGuard { old }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old);
        }
    }

    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        // Suppress background/detached git work so nextest leak detection
        // sees no lingering child processes after the test returns.
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);

        fs::write(path.join("file.txt"), "1\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 1"]);

        dir
    }

    /// Create a temp repo with `main` (2 commits) and `feature-a` (1 commit
    /// since divergence), checked out on `main`. Used by `gather_data` tests
    /// that need real branch data without the overhead of linked worktrees.
    fn temp_repo_with_feature_branch() -> tempfile::TempDir {
        let dir = temp_repo();
        let path = dir.path();

        fs::write(path.join("file.txt"), "2\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 2"]);

        run_git(path, &["checkout", "-b", "feature-a"]);
        fs::write(path.join("a.txt"), "a\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "feature a"]);
        run_git(path, &["checkout", "main"]);

        dir
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_resolves_default_branch_once() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());

        recorder::start_recording();
        let list = list_worktrees().expect("list_worktrees should succeed in temp repo");
        let calls = recorder::finish_recording();

        let symbolic_ref_count = recorder::count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("symbolic-ref")
        });
        assert_eq!(
            symbolic_ref_count, 1,
            "expected exactly one symbolic-ref call, got {calls:?}"
        );
        assert!(!list.default_branch.is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn run_skips_graph_git_calls_when_image_unavailable() {
        let repo = temp_repo();
        let _dir = DirGuard::enter(repo.path());

        let old_term = std::env::var("TERM_PROGRAM").ok();
        let old_kitty = std::env::var("KITTY_WINDOW_ID").ok();
        unsafe {
            std::env::remove_var("TERM_PROGRAM");
            std::env::remove_var("KITTY_WINDOW_ID");
        }

        recorder::start_recording();
        let result = super::run(None, false);
        let calls = recorder::finish_recording();

        unsafe {
            match old_term {
                Some(v) => std::env::set_var("TERM_PROGRAM", v),
                None => std::env::remove_var("TERM_PROGRAM"),
            }
            match old_kitty {
                Some(v) => std::env::set_var("KITTY_WINDOW_ID", v),
                None => std::env::remove_var("KITTY_WINDOW_ID"),
            }
        }

        assert!(result.is_ok(), "run should succeed: {result:?}");
        let graph_calls = recorder::count_matching(&calls, |args| {
            matches!(
                args.first().map(String::as_str),
                Some("merge-base") | Some("log")
            )
        });
        assert_eq!(
            graph_calls, 0,
            "expected zero graph-path git calls when image support is unavailable, got {calls:?}"
        );
    }

    /// Regression guard for the narrow image-terminal gating decision. A
    /// character width or the default (no `--width`) case must be gated by
    /// `MIN_GRAPH_TERMINAL_WIDTH`; percentage and fill widths stay eligible
    /// regardless of width, and no image support is never eligible.
    #[test]
    fn graph_eligible_gates_default_and_character_widths_on_terminal_width() {
        let narrow = super::MIN_GRAPH_TERMINAL_WIDTH - 1;
        let wide = super::MIN_GRAPH_TERMINAL_WIDTH;

        // No image support -> never eligible, regardless of width.
        assert!(!super::graph_eligible(ImageSupport::None, &None, wide));

        // Image support + percentage / fill -> eligible even on narrow terminals.
        assert!(super::graph_eligible(
            ImageSupport::Kitty,
            &Some(ImageWidth::Percent(0.5)),
            narrow
        ));
        assert!(super::graph_eligible(
            ImageSupport::Kitty,
            &Some(ImageWidth::Fill),
            narrow
        ));

        // Image support + explicit character width -> gated by the minimum.
        assert!(!super::graph_eligible(
            ImageSupport::Kitty,
            &Some(ImageWidth::Characters(60)),
            narrow
        ));
        assert!(super::graph_eligible(
            ImageSupport::Kitty,
            &Some(ImageWidth::Characters(60)),
            wide
        ));

        // Image support + default (no `--width`) -> gated by the minimum. This
        // is the review-5 gap: a narrow image-capable terminal with the default
        // width must not gather graph data only to discard it later.
        assert!(!super::graph_eligible(ImageSupport::Kitty, &None, narrow));
        assert!(super::graph_eligible(ImageSupport::Kitty, &None, wide));
    }

    /// Recorder-backed regression for the narrow image-terminal case: on an
    /// image-capable terminal below `MIN_GRAPH_TERMINAL_WIDTH` with the
    /// default width, `gather_extras` must issue zero graph-only git calls.
    /// Without the early skip, `gather_data` would run `merge-base` / `log`
    /// only for the later `fits` check to discard the result.
    #[test]
    #[serial_test::serial]
    fn narrow_image_terminal_skips_graph_gather() {
        let repo = temp_repo();
        let _dir = DirGuard::enter(repo.path());

        // Resolve the real worktree set outside the recorder window so only the
        // gather path is observed.
        let list = list_worktrees().expect("list_worktrees should succeed");

        // 40 columns is below MIN_GRAPH_TERMINAL_WIDTH; image support is forced
        // on, so the only thing that can suppress the gather is the width gate.
        let narrow = Terminal::builder().width(40).build();

        recorder::start_recording();
        let (needs_graph, _needs_verbose, current, base) = super::gather_extras(
            &list.default_branch,
            &list.statuses,
            &narrow,
            ImageSupport::Kitty,
            &None,
            false,
        );
        let calls = recorder::finish_recording();

        assert!(
            !needs_graph,
            "narrow image terminal with default width should not gather graph data"
        );
        assert!(current.is_none() && base.is_none());

        let graph_calls = recorder::count_matching(&calls, |args| {
            matches!(
                args.first().map(String::as_str),
                Some("merge-base") | Some("log")
            )
        });
        assert_eq!(
            graph_calls, 0,
            "expected zero graph-path git calls on a narrow image terminal, got {calls:?}"
        );
    }

    /// Regression guard for R6/R9: when the image-capable graph path and
    /// verbose are both needed for the current feature branch, the
    /// orchestration must issue exactly one `merge-base` call — shared across
    /// graph and verbose surfaces.
    ///
    /// This tests the `gather_data` orchestration boundary directly.
    /// `gather_data` is the sole function from which `gather_branch` is
    /// invoked during `run()`, so a future regression that calls
    /// `gather_branch` twice would have to bypass this boundary. The test uses
    /// a crafted `statuses` Vec to simulate a non-main current worktree without
    /// needing linked worktrees in the fixture.
    #[test]
    #[serial_test::serial]
    fn gather_data_shares_one_merge_base_for_graph_and_verbose() {
        let repo = temp_repo_with_feature_branch();
        let _guard = DirGuard::enter(repo.path());

        let statuses = vec![
            WorktreeStatus {
                entry: WorktreeEntry {
                    path: PathBuf::from("/fake/main"),
                    branch: Some("main".to_string()),
                    is_main: true,
                    is_current: false,
                },
                is_clean: true,
                dirty: DirtyStatus::Clean,
                ahead: 0,
                behind: 0,
            },
            WorktreeStatus {
                entry: WorktreeEntry {
                    path: PathBuf::from("/fake/feature-a"),
                    branch: Some("feature-a".to_string()),
                    is_main: false,
                    is_current: true,
                },
                is_clean: true,
                dirty: DirtyStatus::Clean,
                ahead: 1,
                behind: 0,
            },
        ];

        recorder::start_recording();
        let (current_branch_data, base_graph_data) =
            super::gather_data("main", &statuses, true, true);
        let calls = recorder::finish_recording();

        let merge_base_count = recorder::count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("merge-base")
        });
        assert_eq!(
            merge_base_count, 1,
            "expected exactly one merge-base for graph+verbose gather, got {calls:?}"
        );

        let short_sha_count = recorder::count_matching(&calls, |args| {
            args.len() >= 2 && args[0] == "rev-parse" && args[1] == "--short"
        });
        assert_eq!(
            short_sha_count, 0,
            "expected zero rev-parse --short calls, got {calls:?}"
        );

        assert!(
            current_branch_data.is_some(),
            "current branch data should be gathered for a feature branch"
        );
        assert!(
            base_graph_data.is_none(),
            "base graph should not be gathered for a feature branch"
        );
    }

    /// Complement to the graph+verbose test: when neither graph nor verbose is
    /// needed, `gather_data` must issue zero graph/verbose git calls. This
    /// confirms the orchestration boundary gates correctly on the `needs_*`
    /// flags.
    #[test]
    #[serial_test::serial]
    fn gather_data_skips_git_calls_when_nothing_needed() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());

        recorder::start_recording();
        let (current, base) = super::gather_data("main", &[], false, false);
        let calls = recorder::finish_recording();

        assert!(current.is_none());
        assert!(base.is_none());

        let graph_calls = recorder::count_matching(&calls, |args| {
            matches!(
                args.first().map(String::as_str),
                Some("merge-base") | Some("log")
            )
        });
        assert_eq!(
            graph_calls, 0,
            "expected zero git calls when neither graph nor verbose is needed, got {calls:?}"
        );
    }

    /// Reproducible subprocess-count and wall-clock measurement for the
    /// optimized `wt list` paths. Run with `--nocapture` to see timing.
    ///
    /// Asserts the subprocess-count bounds the optimization guarantees for the
    /// `list_worktrees()` and `gather_base_graph()` paths in the ambient
    /// `rusty-biscuit` checkout (see `sniff/fixes/_completed/2026-04-21-performance/spec.md`).
    /// The image-terminal `wt list -v` data-gather path (`gather_branch` with
    /// verbose) has its binding SLA + count guard in
    /// `git_graph::tests::gather_branch_uses_one_merge_base_and_no_short_sha`,
    /// which runs on a controlled fixture so it always asserts regardless of
    /// the ambient checkout; here that path is observed ambiently only.
    /// Rasterization (Mermaid -> SVG -> PNG) lives outside this package and is
    /// excluded: this test never invokes `MermaidDiagram::render`. The
    /// full-command SLA is covered by `perf_full_command_non_image_meets_sla`.
    #[test]
    #[serial_test::serial]
    fn perf_subprocess_counts_meet_sla() {
        use std::time::Instant;

        let list = list_worktrees().expect("list_worktrees should succeed");
        let default_branch = list.default_branch.clone();
        let current_branch = list
            .statuses
            .iter()
            .find(|s| s.entry.is_current)
            .and_then(|c| c.entry.branch.as_deref())
            .unwrap_or("HEAD")
            .to_string();
        let is_main = list
            .statuses
            .iter()
            .find(|s| s.entry.is_current)
            .is_some_and(|c| c.entry.is_main);
        let branch_names: Vec<String> = list
            .statuses
            .iter()
            .filter_map(|s| s.entry.branch.clone())
            .collect();
        drop(list);

        recorder::start_recording();
        let t0 = Instant::now();
        let _ = list_worktrees();
        let list_elapsed = t0.elapsed();
        let list_calls = recorder::finish_recording();

        let symbolic_ref_count = recorder::count_matching(&list_calls, |args| {
            args.first().map(String::as_str) == Some("symbolic-ref")
        });
        assert_eq!(
            symbolic_ref_count, 1,
            "list_worktrees should resolve default branch exactly once, got {list_calls:?}"
        );
        eprintln!(
            "list_worktrees: {list_elapsed:.2?}, {} git calls",
            list_calls.len()
        );

        // Ambient observability for the current branch's graph+verbose gather
        // path, printed when the ambient checkout is a feature branch. This is
        // not the binding SLA guard: that lives in
        // `git_graph::tests::gather_branch_uses_one_merge_base_and_no_short_sha`,
        // which runs on a controlled fixture so it always asserts (this ambient
        // branch is naturally skipped on a main checkout, where it would assert
        // nothing).
        if !is_main {
            let t0 = Instant::now();
            let _ = super::git_graph::gather_branch(&default_branch, &current_branch, true);
            eprintln!("gather_branch (ambient, verbose): {:.2?}", t0.elapsed());
        }

        if !branch_names.is_empty() {
            recorder::start_recording();
            let t0 = Instant::now();
            let base_data = super::git_graph::gather_base_graph(&default_branch, &branch_names);
            let base_elapsed = t0.elapsed();
            let base_calls = recorder::finish_recording();

            if let Some(data) = &base_data {
                let mb_count = recorder::count_matching(&base_calls, |args| {
                    args.first().map(String::as_str) == Some("merge-base")
                });
                assert_eq!(
                    mb_count,
                    data.branches.len(),
                    "gather_base_graph should issue one merge-base per branch, got {base_calls:?}"
                );

                let log_count = recorder::count_matching(&base_calls, |args| {
                    args.first().map(String::as_str) == Some("log")
                });
                assert_eq!(
                    log_count,
                    1 + data.branches.len(),
                    "gather_base_graph should issue one main log plus one per branch, got {base_calls:?}"
                );
            }
            eprintln!(
                "gather_base_graph: {base_elapsed:.2?}, {} git calls",
                base_calls.len()
            );
        }

        // list_worktrees wall-clock is printed above for observability. The
        // full-command SLA (which subsumes this piece) is asserted by
        // `perf_full_command_non_image_meets_sla` in the integration tests.
    }
}
