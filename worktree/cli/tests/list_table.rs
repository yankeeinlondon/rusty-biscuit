//! The `wt list` table rendered from listing facts: caption variants, every
//! cell kind, badge placement, the legend, row emphasis, and the PR age line.
//!
//! The tree comes from the library's real `build_tree`; only the git answers
//! (comparisons, dirty state, PRs) are scripted.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use biscuit_terminal::discovery::detection::{ColorDepth, ColorMode};
use biscuit_terminal::terminal::Terminal;
use worktree::default_target::DefaultTarget;
use worktree::fork_origin::{ForkOrigin, ForkOriginStore};
use worktree::listing::{
    BranchComparisons, Caption, Comparison, ParentComparison, TreeRow, build_tree,
};
use worktree::pull_requests::{OpenPullRequest, PrListing};
use worktree::worktree::{DirtyStatus, WorktreeEntry, WorktreeStatus};
use worktree_cli::commands::list_table::{self, TableFacts};

const NOW: u64 = 1_790_000_000;

fn terminal_at(width: u32, color: bool) -> Terminal {
    let builder = Terminal::builder().osc_link_support(color).is_tty(color).width(width);
    if color {
        builder.color_depth(ColorDepth::TrueColor).color_mode(ColorMode::Dark).build()
    } else {
        builder.color_depth(ColorDepth::None).build()
    }
}

fn plain_terminal() -> Terminal {
    terminal_at(120, false)
}

fn color_terminal() -> Terminal {
    terminal_at(120, true)
}

/// The two tips are the same commit.
const EQUAL_TIPS: Comparison = Comparison { ahead: 0, behind: 0, is_clean: true };
/// Nothing to merge; the target has moved on.
const NOTHING_TO_MERGE: Comparison = Comparison { ahead: 0, behind: 4, is_clean: true };
const AHEAD_ONLY: Comparison = Comparison { ahead: 3, behind: 0, is_clean: true };
const CLEAN: Comparison = Comparison { ahead: 2, behind: 1, is_clean: true };
const CONFLICTS: Comparison = Comparison { ahead: 1, behind: 3, is_clean: false };

/// The spec's example repository, standing in `fix-wt-ux`.
struct Example {
    statuses: Vec<WorktreeStatus>,
    tree: Vec<TreeRow>,
    comparisons: HashMap<String, BranchComparisons>,
    target: DefaultTarget,
    caption: Caption,
    prs: PrListing,
}

fn status(dir: &str, branch: Option<&str>, is_main: bool, is_current: bool, dirty: DirtyStatus) -> WorktreeStatus {
    WorktreeStatus {
        entry: WorktreeEntry {
            path: PathBuf::from("/code/wts").join(dir),
            branch: branch.map(str::to_string),
            head_sha: Some("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678".to_string()),
            is_main,
            is_current,
        },
        dirty,
    }
}

fn pr(number: u64, repo: &str, branch: &str, target: &str) -> OpenPullRequest {
    OpenPullRequest {
        number,
        url: Some(format!("https://github.com/owner/repo/pull/{number}")),
        source_repo: Some(repo.to_string()),
        source_branch: branch.to_string(),
        target_branch: target.to_string(),
    }
}

impl Example {
    fn new() -> Self {
        let statuses = vec![
            status("rusty-biscuit", Some("main"), true, false, DirtyStatus::Clean),
            status("fix-wt-ux", Some("fix/wt-ux"), false, true, DirtyStatus::DirtySource),
            status("feat-theme", Some("feat/theme"), false, false, DirtyStatus::DirtySource),
            status("feat-dark-fixes", Some("feat/dark-fixes"), false, false, DirtyStatus::Clean),
            status("old-cleanup", Some("chore/old-cleanup"), false, false, DirtyStatus::Clean),
            status("spike-parser", Some("spike/parser"), false, false, DirtyStatus::DirtyNonSource),
            status("release-prep", Some("release/prep"), false, false, DirtyStatus::Clean),
            status("bisect", None, false, false, DirtyStatus::DirtyNonSource),
        ];
        let local: BTreeMap<String, String> = ["main", "fix/wt-ux", "feat/theme", "feat/dark-fixes", "chore/old-cleanup", "spike/parser", "release/prep"]
            .iter()
            .map(|name| (name.to_string(), "0".repeat(40)))
            .collect();
        let mut forks = ForkOriginStore::default();
        for (created_at, (branch, parent)) in [
            ("fix/wt-ux", "main"),
            ("feat/theme", "main"),
            ("feat/dark-fixes", "feat/theme"),
            ("chore/old-cleanup", "main"),
            ("spike/parser", "experiments"),
        ]
        .into_iter()
        .enumerate()
        {
            forks.insert(
                branch,
                ForkOrigin {
                    base_branch: parent.to_string(),
                    base_sha: "0".repeat(40),
                    created_at: created_at as u64,
                },
            );
        }
        let entries: Vec<WorktreeEntry> = statuses.iter().map(|s| s.entry.clone()).collect();
        let tree = build_tree(&entries, "main", &local, &forks);

        let target_only = |comparison| BranchComparisons {
            target: Some(comparison),
            parent: ParentComparison::NotApplicable,
        };
        let comparisons = HashMap::from([
            ("fix/wt-ux".to_string(), target_only(CLEAN)),
            ("feat/theme".to_string(), target_only(AHEAD_ONLY)),
            (
                "feat/dark-fixes".to_string(),
                BranchComparisons {
                    target: Some(CLEAN),
                    parent: ParentComparison::Compared(Some(CONFLICTS)),
                },
            ),
            ("chore/old-cleanup".to_string(), target_only(NOTHING_TO_MERGE)),
            (
                "spike/parser".to_string(),
                BranchComparisons {
                    target: Some(CONFLICTS),
                    parent: ParentComparison::Deleted,
                },
            ),
            ("release/prep".to_string(), target_only(EQUAL_TIPS)),
        ]);

        Self {
            statuses,
            tree,
            comparisons,
            target: DefaultTarget {
                reference: "origin/main".to_string(),
                sha: "f".repeat(40),
                diverged: false,
            },
            caption: Caption {
                local: "main".to_string(),
                remote: "origin/main".to_string(),
                tracking_sha: "f".repeat(40),
                ahead: 0,
                behind: 7,
            },
            prs: PrListing {
                source_repo: Some("owner/repo".to_string()),
                pull_requests: vec![
                    pr(99, "owner/repo", "fix/wt-ux", "main"),
                    pr(104, "owner/repo", "feat/dark-fixes", "feat/theme"),
                    pr(7, "owner/repo", "release/prep", "release/2"),
                    // A fork's same-named branch must not get a badge.
                    pr(120, "someone/fork", "feat/theme", "main"),
                ],
                fetched_at: Some(NOW),
            },
        }
    }

    fn facts(&self) -> TableFacts<'_> {
        TableFacts {
            default_branch: "main",
            target: Some(&self.target),
            caption: Some(&self.caption),
            tree: &self.tree,
            statuses: &self.statuses,
            comparisons: &self.comparisons,
            prs: &self.prs,
        }
    }
}

fn plain(example: &Example) -> String {
    list_table::render(&example.facts(), &plain_terminal(), NOW)
}

/// The row whose Branch cell contains `needle`.
fn row<'a>(rendered: &'a str, needle: &str) -> &'a str {
    rendered
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row contains {needle:?} in:\n{rendered}"))
}

#[test]
fn the_spec_example_renders_as_ruled() {
    insta::assert_snapshot!(plain(&Example::new()));
}

#[test]
fn the_target_header_names_a_remote_or_local_target() {
    let example = Example::new();
    let rendered = plain(&example);
    assert!(row(&rendered, "Worktree").contains("->  origin/main "), "{rendered}");

    let local = DefaultTarget {
        reference: "main".to_string(),
        ..example.target.clone()
    };
    let facts = TableFacts {
        target: Some(&local),
        ..example.facts()
    };
    let rendered = list_table::render(&facts, &plain_terminal(), NOW);
    assert!(row(&rendered, "Worktree").contains("->  main "), "{rendered}");

    // In color the header carries the remote badge's background, the local
    // badge's in the Branch column.
    let colored = list_table::render(&example.facts(), &color_terminal(), NOW);
    let header = row(&colored, "Worktree");
    let remote_badge = "\u{1b}[48;2;93;14;192m";
    let local_badge = "\u{1b}[48;2;25;60;184m";
    assert!(header.contains(remote_badge), "{header:?}");
    assert!(row(&colored, "base repo").contains(local_badge));
}

#[test]
fn caption_variants_read_as_ruled() {
    let caption = |ahead, behind| {
        let mut example = Example::new();
        example.caption.ahead = ahead;
        example.caption.behind = behind;
        let rendered = plain(&example);
        rendered.lines().nth(1).unwrap().trim().to_string()
    };
    assert_eq!(caption(0, 7), "main  is 7 commits behind  origin/main");
    assert_eq!(caption(0, 1), "main  is 1 commit behind  origin/main");
    assert_eq!(caption(3, 0), "main  is 3 commits ahead of  origin/main");
    assert_eq!(caption(0, 0), "main  is in sync with  origin/main");
    assert_eq!(
        caption(3, 7),
        "main  has diverged from  origin/main : 3 commits ahead, 7 commits behind"
    );

    let example = Example::new();
    let facts = TableFacts {
        caption: None,
        ..example.facts()
    };
    let rendered = list_table::render(&facts, &plain_terminal(), NOW);
    assert!(!rendered.contains("behind"), "no remote, no caption:\n{rendered}");
    assert!(rendered.starts_with("\n┌"), "{rendered:?}");
}

#[test]
fn only_the_caption_count_is_colored_yellow() {
    let example = Example::new();
    let colored = list_table::render(&example.facts(), &color_terminal(), NOW);
    let caption = colored.lines().nth(1).unwrap();
    assert!(caption.contains("\u{1b}[33m7 commits\u{1b}[0m"), "{caption:?}");
    assert!(caption.contains(" is "), "{caption:?}");
    assert!(!caption.contains("\u{1b}[33m is"), "{caption:?}");
}

#[test]
fn every_cell_kind_renders_as_ruled() {
    let rendered = plain(&Example::new());

    let base = row(&rendered, "base repo");
    assert!(base.contains("○ base repo"), "{base}");
    assert_eq!(base.matches('—').count(), 2, "the default row has no answers: {base}");

    let current = row(&rendered, "fix-wt-ux");
    assert!(current.contains("● fix-wt-ux"));
    assert!(current.contains("├─ fix/wt-ux"));
    assert!(current.contains("clean"));

    let dark = row(&rendered, "feat/dark-fixes");
    assert!(dark.contains("│  └─ feat/dark-fixes"), "{dark}");
    assert!(dark.contains("conflicts"), "{dark}");

    let old_cleanup = row(&rendered, "chore/old-cleanup");
    assert!(old_cleanup.contains("clean -4"), "{old_cleanup}");
    assert!(!rendered.contains("already in"), "{rendered}");

    let deleted = row(&rendered, "experiments");
    assert!(deleted.contains("experiments (deleted)"));
    assert!(!deleted.contains('○') && !deleted.contains('●'), "no worktree: {deleted}");
    assert!(!deleted.contains('—'), "empty target cells: {deleted}");

    let orphan = row(&rendered, "spike/parser");
    assert!(orphan.contains("└┄ spike/parser"), "{orphan}");
    assert!(orphan.contains("parent deleted"), "{orphan}");
    assert!(orphan.contains("conflicts"), "{orphan}");

    let detached = row(&rendered, "detached @");
    assert!(detached.contains("detached @ a1b2c3d"));
    assert_eq!(detached.matches('—').count(), 2);
}

#[test]
fn an_unknown_comparison_is_a_question_mark_not_an_answer() {
    let mut example = Example::new();
    example.comparisons.insert(
        "fix/wt-ux".to_string(),
        BranchComparisons {
            target: None,
            parent: ParentComparison::Compared(None),
        },
    );
    let rendered = plain(&example);
    let current = row(&rendered, "fix-wt-ux");
    assert!(current.contains('?'), "{current}");
    assert!(!current.contains("clean"), "{current}");
}

#[test]
fn pr_badges_follow_the_prs_target_and_skip_forks() {
    let rendered = plain(&Example::new());

    let current = row(&rendered, "fix/wt-ux");
    assert!(current.contains("clean +2 -1  PR #99"), "badge after the default column's cell: {current}");

    let dark = row(&rendered, "feat/dark-fixes");
    assert!(dark.contains("conflicts +1 -3  PR #104"), "badge after the parent cell: {dark}");
    assert!(!dark.contains("PR #104 →"), "{dark}");

    let release = row(&rendered, "release/prep");
    assert!(release.contains("release/prep  PR #7 → release/2"), "beside the branch: {release}");

    assert!(!rendered.contains("#120"), "a fork's PR must not match: {rendered}");
}

#[test]
fn pr_badges_link_through_osc_8_and_print_no_url_without_it() {
    let colored = list_table::render(&Example::new().facts(), &color_terminal(), NOW);
    assert!(
        colored.contains("\u{1b}]8;;https://github.com/owner/repo/pull/99\u{1b}\\"),
        "{colored:?}"
    );

    // Without OSC 8 the badge stays, the URL does not, and the table still
    // fits a 100-column terminal.
    let narrow = Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(false)
        .width(100)
        .build();
    let rendered = list_table::render(&Example::new().facts(), &narrow, NOW);
    assert!(rendered.contains("PR #99"), "{rendered}");
    assert!(!rendered.contains("https://"), "{rendered}");
    assert!(!rendered.contains("could not be rendered"), "{rendered}");
}

#[test]
fn styles_follow_the_design() {
    let colored = list_table::render(&Example::new().facts(), &color_terminal(), NOW);

    // The current row carries the subtle highlight; others do not.
    let highlight = "\u{1b}[48;2;38;42;54m";
    assert!(row(&colored, "fix-wt-ux").contains(highlight));
    assert!(!row(&colored, "feat-theme").contains(highlight));
    // The current worktree's name is bold.
    assert!(row(&colored, "fix-wt-ux").contains("\u{1b}[1mfix-wt-ux"));

    // A conflicting child has a red connector and a red cell.
    let dark = row(&colored, "feat/dark-fixes");
    assert!(dark.contains("\u{1b}[31m└─"), "{dark:?}");
    assert!(dark.contains("\u{1b}[31mconflicts"), "{dark:?}");

    // A deleted parent is struck through.
    assert!(row(&colored, "experiments").contains("\u{1b}[9mexperiments"));

    // Dots: yellow for other files, the `conflicts` red for source files.
    assert!(row(&colored, "spike-parser").contains("\u{1b}[33m●"));
    assert!(row(&colored, "feat-theme").contains("\u{1b}[31m●"));
    assert!(!colored.contains("\u{1b}[38;2;255;165;0m"), "no orange left: {colored:?}");
}

#[test]
fn the_legend_explains_both_columns() {
    let rendered = plain(&Example::new());
    let lines: Vec<&str> = rendered.lines().collect();
    let legend = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Worktree   "))
        .expect("legend");
    assert_eq!(
        lines[legend].trim_end(),
        " Worktree   ○ clean    ● uncommitted files    ● uncommitted source files"
    );
    assert_eq!(
        lines[legend + 1].trim_end(),
        " Branch     └─ merges cleanly into parent    └─ conflicts with parent    └┄ parent deleted"
    );
}

#[test]
fn the_pr_age_line_appears_once_the_badges_are_60_seconds_old() {
    let with = |fetched_at: Option<u64>| {
        let mut example = Example::new();
        example.prs.fetched_at = fetched_at;
        plain(&example).lines().last().unwrap().trim().to_string()
    };
    assert_eq!(with(Some(NOW - (12 * 60 + 30))), "PRs as of 12 min ago");
    assert_eq!(with(Some(NOW - 60)), "PRs as of 1 min ago");
    assert_eq!(with(Some(NOW - 3 * 3600)), "PRs as of 3 h ago");
    assert_eq!(with(Some(NOW - 5 * 86_400)), "PRs as of 5 days ago");
    assert!(with(Some(NOW - 59)).starts_with("Branch"), "fresh badges need no age line");
    assert!(with(Some(NOW + 60)).starts_with("Branch"), "a future fetch time has no age");
    assert!(with(None).starts_with("Branch"), "no answer has no age");
}

#[test]
fn a_stored_empty_answer_shows_no_badges_but_keeps_its_age() {
    let mut example = Example::new();
    example.prs.pull_requests.clear();
    example.prs.fetched_at = Some(NOW - 5 * 60);
    let rendered = plain(&example);
    assert!(!rendered.contains("PR #"), "{rendered}");
    assert_eq!(rendered.lines().last().unwrap().trim(), "PRs as of 5 min ago");

    // An unavailable first answer (nothing stored, the request failed) is
    // not an empty answer: no badges and no age.
    example.prs = Default::default();
    let rendered = plain(&example);
    assert!(!rendered.contains("PR #") && !rendered.contains("PRs as of"), "{rendered}");
}

fn plain_at(width: u32, example: &Example) -> String {
    list_table::render(&example.facts(), &terminal_at(width, false), NOW)
}

/// The target and parent cells of a table line, trimmed. They are the last two
/// columns, counted from the right because a Branch cell's tree guide is also
/// a `│`.
fn target_cells(line: &str) -> (String, String) {
    let cells: Vec<&str> = line.split('│').map(str::trim).collect();
    let n = cells.len();
    (cells[n - 3].to_string(), cells[n - 2].to_string())
}

/// The line after the one containing `needle`, where a wrapped cell continues.
fn next_line<'a>(rendered: &'a str, needle: &str) -> &'a str {
    let mut lines = rendered.lines();
    lines.find(|line| line.contains(needle)).expect("row");
    lines.next().expect("a following line")
}

#[test]
fn the_table_at_99_columns_shows_no_counts() {
    insta::assert_snapshot!(plain_at(99, &Example::new()));
}

#[test]
fn the_table_at_100_columns_shows_counts() {
    insta::assert_snapshot!(plain_at(100, &Example::new()));
}

#[test]
fn counts_follow_the_state_word_and_precede_the_badge_from_100_columns() {
    let rendered = plain_at(100, &Example::new());
    let cells = |needle| target_cells(row(&rendered, needle));

    // Still the four columns: five vertical rules on the header and a row.
    assert_eq!(row(&rendered, "Worktree  ").matches('│').count(), 5, "{rendered}");
    assert_eq!(row(&rendered, "fix/wt-ux").matches('│').count(), 5, "{rendered}");

    assert_eq!(cells("fix/wt-ux"), ("clean +2 -1  PR #99".into(), "—".into()));
    assert_eq!(cells("feat/theme"), ("clean +3".into(), "—".into()));
    assert_eq!(cells("chore/old-cleanup"), ("clean -4".into(), "—".into()));
    assert_eq!(cells("release/prep"), ("clean".into(), "—".into()), "equal tips show no count");
    // At exactly 100 the parent cell's badge wraps under its counts; the
    // gate promises the counts, not that every row fits on one line.
    assert_eq!(cells("feat/dark-fixes"), ("clean +2 -1".into(), "conflicts +1 -3".into()));
    assert_eq!(target_cells(next_line(&rendered, "feat/dark-fixes")).1, "PR #104");
    assert_eq!(cells("spike/parser"), ("conflicts +1 -3".into(), "parent deleted".into()));
    assert_eq!(cells("base repo"), ("—".into(), "—".into()));
    assert_eq!(cells("detached @"), ("—".into(), "—".into()));
    assert_eq!(cells("experiments"), (String::new(), String::new()));
}

#[test]
fn up_to_99_columns_cells_keep_state_and_badges_without_counts() {
    let rendered = plain_at(99, &Example::new());
    let cells = |needle| target_cells(row(&rendered, needle));
    assert_eq!(cells("fix/wt-ux").0, "clean  PR #99", "{rendered}");
    assert_eq!(cells("feat/theme").0, "clean");
    assert_eq!(cells("chore/old-cleanup").0, "clean");
    assert_eq!(cells("release/prep").0, "clean");
    assert_eq!(cells("feat/dark-fixes"), ("clean".into(), "conflicts  PR #104".into()));
    assert_eq!(cells("spike/parser"), ("conflicts".into(), "parent deleted".into()));

    // Narrower terminals wrap badges but never gain counts.
    for width in [99, 80] {
        let rendered = plain_at(width, &Example::new());
        for token in ["+1", "+2", "+3", "-1", "-3", "-4"] {
            assert!(!rendered.contains(token), "no {token} at {width}:\n{rendered}");
        }
    }
}

#[test]
fn an_unknown_comparison_never_gets_counts() {
    let mut example = Example::new();
    example.comparisons.insert(
        "fix/wt-ux".to_string(),
        BranchComparisons {
            target: None,
            parent: ParentComparison::Compared(None),
        },
    );
    let rendered = plain_at(100, &example);
    assert_eq!(target_cells(row(&rendered, "fix/wt-ux")), ("?  PR #99".into(), "?".into()));
}

#[test]
fn counts_are_dim_green_and_dim_red_in_color() {
    let colored = list_table::render(&Example::new().facts(), &terminal_at(100, true), NOW);
    let current = row(&colored, "fix-wt-ux");
    assert!(current.contains("\u{1b}[2m\u{1b}[32m+2"), "{current:?}");
    assert!(current.contains("\u{1b}[2m\u{1b}[31m-1"), "{current:?}");
    let ahead = current.find("+2").unwrap();
    let behind = current.find("-1").unwrap();
    let badge = current.find("PR #99").unwrap();
    assert!(ahead < behind && behind < badge, "{current:?}");

    let dark = row(&colored, "feat/dark-fixes");
    assert!(dark.contains("\u{1b}[31mconflicts"), "{dark:?}");
    assert!(dark.contains("\u{1b}[2m\u{1b}[32m+1"), "{dark:?}");
    assert!(dark.contains("\u{1b}[2m\u{1b}[31m-3"), "{dark:?}");

    let narrow = list_table::render(&Example::new().facts(), &terminal_at(99, true), NOW);
    assert!(!narrow.contains("\u{1b}[32m"), "no green count at 99: {narrow:?}");
}

#[test]
fn no_color_counts_read_as_plain_text() {
    let rendered = plain_at(100, &Example::new());
    assert!(!rendered.contains('\u{1b}'), "{rendered:?}");
    assert!(row(&rendered, "fix/wt-ux").contains("clean +2 -1"), "{rendered}");
}
