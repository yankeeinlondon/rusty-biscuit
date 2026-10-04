//! The `wt list` output rendered from listing facts: caption variants, the
//! credentials line, every cell kind, badge placement, the legend, row
//! emphasis, the status list (PR age line and refresh hint), the closing notes, and the
//! order of every section.
//!
//! The tree comes from the library's real `build_tree`; only the git answers
//! (comparisons, dirty state, PRs) are scripted.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use biscuit_terminal::discovery::detection::{ColorDepth, ColorMode};
use biscuit_terminal::terminal::Terminal;
use worktree::availability::{Availability, OtherCondition};
use worktree::default_target::DefaultTarget;
use worktree::fork_origin::{ForkOrigin, ForkOriginStore};
use worktree::listing::{
    BranchComparisons, Caption, Comparison, ParentComparison, TreeRow, build_tree,
};
use worktree::pull_requests::{OpenPullRequest, PrListing};
use worktree::remote_head::{CheckFailure, FetchFailure};
use worktree::worktree::{DirtyStatus, WorktreeEntry, WorktreeStatus};
use worktree_cli::commands::list_table::{
    self, CredentialCondition, CredentialLine, FfNotice, FfSuggestion, ForkedOffLine, GraphOmission, GraphOmissions, LastKnown, MergedElsewhere, PrOutcome, RemoteFacts,
    RemoteStatus, Sections, TableFacts,
};

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
            prunable: None,
        },
        dirty,
        availability: Availability::Healthy,
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
            remote: Some(RemoteFacts {
                default_branch: "main",
                tracking_tip: Some(&self.caption.tracking_sha),
                status: RemoteStatus::CheckedNow,
            }),
            credential_line: None,
            pr_outcome: Some(PrOutcome::Published),
            timed_out: false,
            ff_suggestion: None,
            ff_notice: None,
            fallback_notice: None,
            graph_omissions: GraphOmissions::default(),
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
        let facts = example.facts();
        let rendered = list_table::render(&facts, &plain_terminal(), NOW);
        rendered.lines().nth(1).unwrap().trim().to_string()
    };
    // A check this run made that found the tracking ref current adds no suffix.
    assert_eq!(caption(0, 7), "main  is 7 commits behind  origin/main");
    assert_eq!(caption(0, 1), "main  is 1 commit behind  origin/main");
    assert_eq!(caption(3, 0), "main  is 3 commits ahead of  origin/main");
    assert_eq!(caption(0, 0), "main  is in sync with  origin/main");
    assert_eq!(
        caption(3, 7),
        "main  has diverged from  origin/main  (3 commits ahead, 7 commits behind)"
    );

    let example = Example::new();
    let facts = TableFacts {
        caption: None,
        remote: None,
        ..example.facts()
    };
    let rendered = list_table::render(&facts, &plain_terminal(), NOW);
    assert!(!rendered.contains("behind"), "no remote, no caption:\n{rendered}");
    assert!(!rendered.contains("origin ("), "no remote, no observation:\n{rendered}");
    assert!(rendered.starts_with("\n┌"), "{rendered:?}");
}

#[test]
fn only_the_caption_count_is_colored_yellow() {
    let example = Example::new();
    let colored = list_table::render(&example.facts(), &color_terminal(), NOW);
    let caption = colored.lines().nth(1).unwrap();
    assert!(caption.contains("\u{1b}[33m7 commits\u{1b}[0m"), "{caption:?}");
    assert_eq!(caption.matches("\u{1b}[33m").count(), 1, "the suffix has no yellow: {caption:?}");
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
    let column = |line: &str, glyph: char, nth: usize| line.chars().enumerate().filter(|(_, c)| *c == glyph).nth(nth).map(|(i, _)| i);
    assert_eq!(
        column(lines[legend + 1], '└', 1),
        column(lines[legend], '●', 1),
        "the conflicts elbow sits under the source-files dot"
    );
    assert_eq!(
        lines[legend + 1].trim_end(),
        " Branch     └─ merges cleanly into parent     └─ conflicts with parent      └┄ parent deleted"
    );
}

#[test]
fn a_narrow_table_is_as_wide_as_the_legend() {
    let example = Example::new();
    let facts = TableFacts { tree: &example.tree[..1], ..example.facts() };
    let rendered = list_table::render(&facts, &plain_terminal(), NOW);
    let width = |line: &str| line.trim_end().chars().count();
    let branch_legend = rendered.lines().find(|line| line.starts_with(" Branch ")).expect("legend");
    let table_lines: Vec<&str> = rendered.lines().filter(|line| line.starts_with(['┌', '│', '├', '└'])).collect();
    assert_eq!(table_lines.len(), 5, "{rendered}");
    for line in table_lines {
        assert_eq!(width(line), width(branch_legend), "{line:?} in:\n{rendered}");
    }
}

/// The status list alone for `example` with this run's PR half `outcome`.
fn pr_status(example: &Example, outcome: PrOutcome) -> Option<String> {
    let facts = TableFacts { pr_outcome: Some(outcome), ..example.facts() };
    list_table::render_status(&facts, &plain_terminal(), NOW).map(|status| status.trim().to_string())
}

#[test]
fn the_pr_age_line_appears_once_a_pending_refreshs_badges_are_60_seconds_old() {
    let with = |fetched_at: Option<u64>| {
        let mut example = Example::new();
        example.prs.fetched_at = fetched_at;
        assert!(!plain(&example).contains("PRs as of"), "the age line is not beneath the legend");
        pr_status(&example, PrOutcome::Pending)
    };
    assert_eq!(with(Some(NOW - (12 * 60 + 30))).as_deref(), Some("- PRs as of 12 min ago"));
    assert_eq!(with(Some(NOW - 60)).as_deref(), Some("- PRs as of 1 min ago"));
    assert_eq!(with(Some(NOW - 3 * 3600)).as_deref(), Some("- PRs as of 3 h ago"));
    assert_eq!(with(Some(NOW - 5 * 86_400)).as_deref(), Some("- PRs as of 5 days ago"));
    assert_eq!(with(Some(NOW - 59)), None, "fresh badges need no age line");
    assert_eq!(with(Some(NOW + 60)), None, "a future fetch time has no age");
    assert_eq!(with(None), None, "no answer has no age");
}

#[test]
fn a_published_answer_has_no_status_item_at_any_age() {
    for fetched_at in [Some(NOW), Some(NOW - 5 * 86_400), None] {
        let mut example = Example::new();
        example.prs.fetched_at = fetched_at;
        assert_eq!(pr_status(&example, PrOutcome::Published), None, "{fetched_at:?}");
    }
}

#[test]
fn a_failed_refresh_dates_the_stored_answer_at_any_age() {
    let with = |fetched_at: Option<u64>| {
        let mut example = Example::new();
        example.prs.fetched_at = fetched_at;
        pr_status(&example, PrOutcome::Failed)
    };
    assert_eq!(with(Some(NOW - 10)).as_deref(), Some("- PRs as of less than 1 min ago (couldn't refresh)"));
    assert_eq!(with(Some(NOW)).as_deref(), Some("- PRs as of less than 1 min ago (couldn't refresh)"));
    assert_eq!(with(Some(NOW - 59)).as_deref(), Some("- PRs as of less than 1 min ago (couldn't refresh)"));
    assert_eq!(with(Some(NOW - 60)).as_deref(), Some("- PRs as of 1 min ago (couldn't refresh)"));
    assert_eq!(with(Some(NOW - 3 * 3600)).as_deref(), Some("- PRs as of 3 h ago (couldn't refresh)"));
    assert_eq!(with(Some(NOW - 5 * 86_400)).as_deref(), Some("- PRs as of 5 days ago (couldn't refresh)"));
    assert_eq!(with(None).as_deref(), Some("- couldn't get open PRs"), "nothing stored");
}

#[test]
fn a_stored_empty_answer_shows_no_badges_but_keeps_its_age() {
    let mut example = Example::new();
    example.prs.pull_requests.clear();
    example.prs.fetched_at = Some(NOW - 5 * 60);
    let rendered = plain(&example);
    assert!(!rendered.contains("PR #"), "{rendered}");
    assert_eq!(pr_status(&example, PrOutcome::Pending).as_deref(), Some("- PRs as of 5 min ago"));
    assert_eq!(pr_status(&example, PrOutcome::Failed).as_deref(), Some("- PRs as of 5 min ago (couldn't refresh)"));

    // Nothing stored is not an empty answer: no badges, no age, and a
    // failure says there are no PRs to show.
    example.prs = Default::default();
    let rendered = plain(&example);
    assert!(!rendered.contains("PR #"), "{rendered}");
    assert_eq!(pr_status(&example, PrOutcome::Pending), None);
    assert_eq!(pr_status(&example, PrOutcome::Failed).as_deref(), Some("- couldn't get open PRs"));
}

#[test]
fn an_ignored_or_unsupported_repository_shows_no_badges_even_when_an_answer_is_stored() {
    let example = Example::new();
    assert!(plain(&example).contains("PR #99"), "control: the stored answer has badges");
    for outcome in [PrOutcome::Ignored, PrOutcome::Unsupported] {
        let facts = TableFacts { pr_outcome: Some(outcome), ..example.facts() };
        let rendered = list_table::render(&facts, &plain_terminal(), NOW);
        assert!(!rendered.contains("PR #"), "{outcome:?}: {rendered}");
        assert_eq!(facts.badges().pull_requests.len(), 0, "{outcome:?}: the graph's tags too");
        assert_eq!(list_table::render_status(&facts, &plain_terminal(), NOW), None, "{outcome:?}");
    }
}

/// Every row of the PR presentation table: the badges the table shows and
/// the status list, for each PR outcome and stored answer.
#[test]
fn pr_presentation_snapshot_every_row() {
    let example = Example::new();
    let answer = |fetched_at: Option<u64>| PrListing { fetched_at, ..example.prs.clone() };
    let empty = |fetched_at: u64| PrListing { pull_requests: Vec::new(), fetched_at: Some(fetched_at), ..example.prs.clone() };
    let fresh = answer(Some(NOW - 2));
    let young = answer(Some(NOW - 10));
    let old = answer(Some(NOW - (12 * 60 + 30)));
    let empty_old = empty(NOW - 5 * 60);
    let empty_fresh = empty(NOW - 2);
    let nothing = PrListing::default();
    let rows: Vec<(&str, Option<PrOutcome>, &PrListing, bool)> = vec![
        ("published within the wait", Some(PrOutcome::Published), &fresh, false),
        ("published within the wait, head still running", Some(PrOutcome::Published), &fresh, true),
        ("published an empty answer", Some(PrOutcome::Published), &empty_fresh, false),
        ("ignored, an old answer stored", Some(PrOutcome::Ignored), &old, false),
        ("unsupported origin", Some(PrOutcome::Unsupported), &nothing, false),
        ("still running at 3 s, answer 12 min old", Some(PrOutcome::Pending), &old, true),
        ("still running at 3 s, answer 10 s old", Some(PrOutcome::Pending), &young, true),
        ("still running at 3 s, empty answer 5 min old", Some(PrOutcome::Pending), &empty_old, true),
        ("still running at 3 s, nothing stored", Some(PrOutcome::Pending), &nothing, true),
        ("failed, answer 10 s old", Some(PrOutcome::Failed), &young, false),
        ("failed, answer 12 min old", Some(PrOutcome::Failed), &old, false),
        ("failed, empty answer 5 min old", Some(PrOutcome::Failed), &empty_old, false),
        ("failed, nothing stored", Some(PrOutcome::Failed), &nothing, false),
        ("failed, head still running at 3 s", Some(PrOutcome::Failed), &young, true),
        ("no origin", None, &nothing, false),
    ];
    let cases = rows
        .into_iter()
        .map(|(label, pr_outcome, prs, timed_out)| {
            let facts = TableFacts { prs, pr_outcome, timed_out, ..example.facts() };
            let table = list_table::render(&facts, &terminal_at(400, false), NOW);
            let mut badges: Vec<&str> = table.match_indices("PR #").map(|(at, _)| {
                let rest = &table[at..];
                &rest[..rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == ' ' || c == '#')).unwrap_or(rest.len())]
            })
            .map(str::trim)
            .collect();
            badges.sort_unstable();
            let status = list_table::render_status(&facts, &terminal_at(400, false), NOW).unwrap_or_else(|| "(none)\n".into());
            let badges = if badges.is_empty() { "(none)".to_string() } else { badges.join(", ") };
            (label.to_string(), format!("badges: {badges}\n{status}"))
        })
        .collect();
    insta::assert_snapshot!("pr_presentation_every_row", labeled(cases));
}

#[test]
fn the_pr_status_item_is_dim_and_precedes_the_hint() {
    let mut example = Example::new();
    example.prs.fetched_at = None;
    let facts = TableFacts { pr_outcome: Some(PrOutcome::Failed), timed_out: true, ..example.facts() };
    let colored = list_table::render_status(&facts, &color_terminal(), NOW).expect("status");
    let item = colored.find("couldn't get open PRs").expect("the item");
    let hint = colored.find("running this command again").expect("the hint");
    assert!(item < hint, "the PR item comes first: {colored:?}");
    for text in ["couldn't get open PRs", "running this command again"] {
        assert!(colored.contains(&format!("\u{1b}[2m{text}")), "{text:?} is dim: {colored:?}");
    }
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

/// The caption paragraph (and any credentials line), unwrapped, from a
/// render of `facts` at `now`; empty when the render has none.
fn paragraph(facts: &TableFacts<'_>, now: u64) -> String {
    let rendered = list_table::render(facts, &terminal_at(400, false), now);
    if rendered.lines().nth(1).is_some_and(|line| line.starts_with('┌')) {
        return String::new();
    }
    rendered
        .lines()
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

const TRACKING_TIP: &str = "ffffffffffffffffffffffffffffffffffffffff";

fn comparison(ahead: usize, behind: usize) -> Caption {
    Caption {
        local: "main".to_string(),
        remote: "origin/main".to_string(),
        tracking_sha: TRACKING_TIP.to_string(),
        ahead,
        behind,
    }
}

/// The caption for `caption` (none: no comparison) and `status`, with the
/// tracking ref present or not.
fn caption_for(caption: Option<&Caption>, tracking: bool, status: RemoteStatus) -> String {
    caption_on("main", caption, tracking, status, NOW)
}

fn caption_on(default: &str, caption: Option<&Caption>, tracking: bool, status: RemoteStatus, now: u64) -> String {
    let example = Example::new();
    let facts = TableFacts {
        caption,
        remote: Some(RemoteFacts {
            default_branch: default,
            tracking_tip: tracking.then_some(TRACKING_TIP),
            status,
        }),
        ..example.facts()
    };
    paragraph(&facts, now)
}

fn labeled(cases: Vec<(String, String)>) -> String {
    cases
        .into_iter()
        .map(|(label, text)| format!("## {label}\n{text}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

const TWO_HOURS: u64 = 2 * 3600;
const THREE_HOURS: u64 = 3 * 3600;

/// One status per §4 row, labeled.
fn every_row() -> Vec<(&'static str, RemoteStatus)> {
    vec![
        ("checked this run, no variance", RemoteStatus::CheckedNow),
        ("variance; the fetch succeeded", RemoteStatus::Fetched),
        ("variance; the fetch failed", RemoteStatus::FetchFailed { reason: FetchFailure::Timeout }),
        (
            "still checking at 3 s",
            RemoteStatus::StillChecking { last: LastKnown::Answer { checked_at: NOW - TWO_HOURS } },
        ),
        ("still pulling at 3 s", RemoteStatus::StillPulling),
        (
            "check failed; an earlier answer exists",
            RemoteStatus::CheckFailed {
                reason: CheckFailure::Other,
                last: LastKnown::Answer { checked_at: NOW - TWO_HOURS },
            },
        ),
        (
            "check failed; no earlier answer; the tracking ref has a reflog",
            RemoteStatus::CheckFailed {
                reason: CheckFailure::Other,
                last: LastKnown::TrackingRefChanged { at: NOW - THREE_HOURS },
            },
        ),
        (
            "check failed; nothing known",
            RemoteStatus::CheckFailed { reason: CheckFailure::Other, last: LastKnown::Never },
        ),
        ("branch absent on origin, tracking ref still exists", RemoteStatus::Absent),
    ]
}

#[test]
fn the_caption_is_one_sentence_with_a_dim_italic_suffix() {
    let example = Example::new();
    let facts = TableFacts {
        remote: Some(RemoteFacts {
            default_branch: "main",
            tracking_tip: Some(TRACKING_TIP),
            status: RemoteStatus::Fetched,
        }),
        ..example.facts()
    };
    let colored = list_table::render(&facts, &color_terminal(), NOW);
    let caption = colored.lines().nth(1).unwrap();
    let suffix_at = caption.find("(updated from origin just now)").expect("suffix");
    let before = &caption[..suffix_at];
    let styles = &before[before.rfind("origin/main").expect("badge")..];
    assert!(styles.contains("\u{1b}[2m") && styles.contains("\u{1b}[3m"), "dim italic suffix: {caption:?}");
    assert!(!caption[..caption.find("origin/main").unwrap()].contains("\u{1b}[3m"), "the comparison is not italic");

    for (label, status) in every_row() {
        let text = caption_for(Some(&comparison(0, 3)), true, status);
        assert_eq!(text.matches(". ").count(), 0, "one sentence ({label}): {text}");
        assert!(!text.contains("tracking ref origin/main."), "{label}: {text}");
        assert!(!text.contains("local tracking ref  origin"), "no 'local tracking ref' ({label}): {text}");
    }
}

#[test]
fn only_rows_that_could_not_update_the_ref_call_it_local() {
    for (label, status) in every_row() {
        let text = caption_for(Some(&comparison(0, 3)), true, status);
        let local = matches!(status, RemoteStatus::FetchFailed { .. } | RemoteStatus::StillPulling);
        assert_eq!(text.contains("behind local  origin/main"), local, "{label}: {text}");
    }
}

#[test]
fn an_absent_branch_never_reads_as_in_sync_with_origin() {
    let text = caption_for(Some(&comparison(0, 0)), true, RemoteStatus::Absent);
    assert_eq!(
        text,
        "main  is in sync with  origin/main  (main was absent on origin when checked just now; origin/main is a local tracking ref)"
    );
    // Pruned: only the observation is left.
    assert_eq!(caption_for(None, false, RemoteStatus::Absent), "main  was absent on origin when checked just now");
}

#[test]
fn a_long_caption_wraps_between_words_within_the_terminal() {
    let example = Example::new();
    let facts = TableFacts {
        remote: Some(RemoteFacts {
            default_branch: "main",
            tracking_tip: Some(TRACKING_TIP),
            status: RemoteStatus::StillChecking { last: LastKnown::Answer { checked_at: NOW - TWO_HOURS } },
        }),
        ..example.facts()
    };
    let rendered = list_table::render(&facts, &terminal_at(60, false), NOW);
    let lines: Vec<&str> = rendered.lines().skip(1).take_while(|line| !line.trim().is_empty()).collect();

    assert!(lines.len() > 1, "{rendered}");
    for line in &lines {
        assert!(line.chars().count() <= 60, "{line:?} is wider than the terminal");
        assert!(line.starts_with(' '), "continuation lines keep the caption's indent: {line:?}");
    }
    let joined = lines.iter().map(|line| line.trim()).collect::<Vec<_>>().join(" ");
    assert_eq!(joined, paragraph(&facts, NOW), "no word was split");
}

#[test]
fn without_an_origin_leftover_tracking_refs_show_no_caption() {
    let example = Example::new();
    let facts = TableFacts { caption: None, remote: None, ..example.facts() };
    let rendered = list_table::render(&facts, &plain_terminal(), NOW);
    assert!(rendered.starts_with("\n┌"), "{rendered:?}");
    assert_eq!(paragraph(&facts, NOW), "");
}

#[test]
fn ages_use_the_pr_age_units() {
    for (seconds, text) in [
        (0, "less than 1 min"),
        (59, "less than 1 min"),
        (60, "1 min"),
        (3599, "59 min"),
        (3600, "1 h"),
        (2 * 86_400 - 60, "47 h"),
        (2 * 86_400, "2 days"),
    ] {
        assert_eq!(list_table::age_text(seconds), text, "{seconds} s");
    }
}

// Caption snapshots (acceptance 5). Each snapshot concatenates labeled cases
// of one group, rendered plain and unwrapped, so a reviewer reads one file per
// group.

#[test]
fn caption_snapshot_every_row_in_every_comparison_state() {
    let states = [
        ("behind", comparison(0, 3)),
        ("ahead", comparison(3, 0)),
        ("in sync", comparison(0, 0)),
        ("diverged", comparison(3, 2)),
    ];
    let mut cases = Vec::new();
    for (row, status) in every_row() {
        for (state, caption) in &states {
            cases.push((format!("{row} / {state}"), caption_for(Some(caption), true, status)));
        }
    }
    insta::assert_snapshot!("caption_every_row_in_every_comparison_state", labeled(cases));
}

#[test]
fn caption_snapshot_reasons() {
    let behind = comparison(0, 3);
    let earlier = LastKnown::Answer { checked_at: NOW - TWO_HOURS };
    let mut cases = Vec::new();
    for (name, reason) in
        [("timeout", CheckFailure::Timeout), ("credentials", CheckFailure::Credentials), ("other", CheckFailure::Other)]
    {
        cases.push((
            format!("check failed: {name}"),
            caption_for(Some(&behind), true, RemoteStatus::CheckFailed { reason, last: earlier }),
        ));
    }
    for (name, reason) in [("timeout", FetchFailure::Timeout), ("other", FetchFailure::Other)] {
        cases.push((format!("fetch failed: {name}"), caption_for(Some(&behind), true, RemoteStatus::FetchFailed { reason })));
    }
    for (name, last) in [
        ("an earlier answer", earlier),
        ("the reflog", LastKnown::TrackingRefChanged { at: NOW - THREE_HOURS }),
        ("nothing", LastKnown::Never),
    ] {
        cases.push((
            format!("still checking, last known from {name}"),
            caption_for(Some(&behind), true, RemoteStatus::StillChecking { last }),
        ));
    }
    insta::assert_snapshot!("caption_reasons", labeled(cases));
}

#[test]
fn caption_snapshot_missing_refs_and_failed_comparison() {
    let mut cases = Vec::new();
    for (row, status) in every_row() {
        // Without a local default (or when git could not compare) the
        // tracking tip is still in the ref snapshot.
        cases.push((format!("no comparison, tracking ref present / {row}"), caption_for(None, true, status)));
        cases.push((format!("no tracking ref / {row}"), caption_for(None, false, status)));
    }
    insta::assert_snapshot!("caption_missing_refs_and_failed_comparison", labeled(cases));
}

#[test]
fn caption_snapshot_trunk_default_branch() {
    let caption = Caption {
        local: "trunk".to_string(),
        remote: "origin/trunk".to_string(),
        tracking_sha: TRACKING_TIP.to_string(),
        ahead: 2,
        behind: 5,
    };
    let cases = every_row()
        .into_iter()
        .map(|(row, status)| (row.to_string(), caption_on("trunk", Some(&caption), true, status, NOW)))
        .chain([(
            "pruned, absent".to_string(),
            caption_on("trunk", None, false, RemoteStatus::Absent, NOW),
        )])
        .collect();
    insta::assert_snapshot!("caption_trunk_default_branch", labeled(cases));
}

#[test]
fn caption_snapshot_age_boundaries_and_future_timestamps() {
    let behind = comparison(0, 3);
    let at = |last| {
        caption_for(Some(&behind), true, RemoteStatus::CheckFailed { reason: CheckFailure::Other, last })
    };
    let mut cases = Vec::new();
    for (label, age) in [
        ("0 s", 0),
        ("59 s", 59),
        ("60 s", 60),
        ("3599 s", 3599),
        ("3600 s", 3600),
        ("2 days - 1 min", 2 * 86_400 - 60),
        ("2 days", 2 * 86_400),
    ] {
        cases.push((format!("answer {label}"), at(LastKnown::Answer { checked_at: NOW - age })));
    }
    cases.push(("answer future-dated by 1 s".into(), at(LastKnown::Answer { checked_at: NOW + 1 })));
    cases.push(("reflog 3 days".into(), at(LastKnown::TrackingRefChanged { at: NOW - 3 * 86_400 })));
    cases.push(("reflog future-dated by 1 day".into(), at(LastKnown::TrackingRefChanged { at: NOW + 86_400 })));
    insta::assert_snapshot!("caption_age_boundaries_and_future_timestamps", labeled(cases));
}

// Credentials line (§5).

const PROVIDERS: [(&str, &str); 4] = [
    ("GitHub", "GH_TOKEN or GITHUB_TOKEN"),
    ("GitLab", "GITLAB_TOKEN or GITLAB_PRIVATE_TOKEN"),
    ("Gitea", "GITEA_TOKEN or FORGEJO_TOKEN or CODEBERG_TOKEN"),
    ("Bitbucket", "BITBUCKET_TOKEN"),
];

fn with_credentials(line: CredentialLine) -> String {
    let example = Example::new();
    let facts = TableFacts { credential_line: Some(line), ..example.facts() };
    let notes = list_table::render_notes(&facts, &terminal_at(400, false)).expect("notes");
    notes.trim().strip_prefix("- ").expect("one note").to_string()
}

#[test]
fn credential_lines_snapshot_every_condition_for_every_provider() {
    let conditions = [
        ("no key, not visible, Git failed too", CredentialCondition::NotVisible, None),
        ("the key was not accepted", CredentialCondition::Rejected, Some("used")),
        ("the key lacks rights", CredentialCondition::Insufficient, Some("used")),
        ("rate limited without a key", CredentialCondition::RateLimited { authenticated: false }, None),
        ("rate limited with a key", CredentialCondition::RateLimited { authenticated: true }, Some("used")),
        (
            "answered without a key, higher limits with one",
            CredentialCondition::AnsweredWithoutKey { higher_limits: true },
            None,
        ),
        (
            "answered without a key, no established limit",
            CredentialCondition::AnsweredWithoutKey { higher_limits: false },
            None,
        ),
    ];
    let mut cases = Vec::new();
    for (provider, accepted) in PROVIDERS {
        for (label, condition, used) in conditions {
            let key = match used {
                Some(_) => accepted.split(" or ").next().unwrap().to_string(),
                None => accepted.to_string(),
            };
            let line = CredentialLine { provider: provider.to_string(), key, condition };
            cases.push((format!("{provider}: {label}"), with_credentials(line)));
        }
    }
    insta::assert_snapshot!("credential_lines_every_condition_for_every_provider", labeled(cases));
}

/// Every §5 line, warning or keyless notice, is an undimmed closing note
/// after the `--ff` suggestion; nothing about credentials is printed above
/// the table.
#[test]
fn the_credentials_line_is_a_closing_note() {
    let example = Example::new();
    for condition in [CredentialCondition::Rejected, CredentialCondition::AnsweredWithoutKey { higher_limits: true }] {
        let line = CredentialLine { provider: "GitHub".into(), key: "GITHUB_TOKEN".into(), condition };
        let facts = TableFacts {
            credential_line: Some(line),
            ff_suggestion: Some(FfSuggestion { behind: 3 }),
            ..example.facts()
        };
        let table = list_table::render(&facts, &color_terminal(), NOW);
        assert!(!table.contains("GITHUB_TOKEN"), "{table}");
        let notes = list_table::render_notes(&facts, &terminal_at(400, true)).expect("notes");
        let items: Vec<&str> = notes.lines().collect();
        assert_eq!(items.len(), 2, "{notes:?}");
        assert!(items[0].contains("fast-forward"), "{notes:?}");
        assert!(items[1].starts_with(" - ") && items[1].contains("GITHUB_TOKEN"), "{notes:?}");
        assert!(!items[1].contains("\u{1b}[2m"), "not dim: {notes:?}");
    }
}

// The hint (§6), the closing notes (§8, §9, `--ff`), and the order of all
// sections.

#[test]
fn the_hint_appears_only_when_the_wait_timed_out() {
    let example = Example::new();
    assert_eq!(list_table::render_status(&example.facts(), &plain_terminal(), NOW), None);
    let facts = TableFacts { timed_out: true, ..example.facts() };
    let hint = list_table::render_status(&facts, &plain_terminal(), NOW).expect("hint");
    let words = hint.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(words, format!("- {}", list_table::REFRESH_HINT), "wrapped, never split: {hint:?}");
    let colored = list_table::render_status(&facts, &color_terminal(), NOW).expect("hint");
    assert!(colored.contains("\u{1b}[2m"), "dim: {colored:?}");
}

#[test]
fn closing_notes_snapshot() {
    let example = Example::new();
    let notes = |facts: TableFacts<'_>| {
        list_table::render_notes(&facts, &terminal_at(400, false)).unwrap_or_else(|| "(none)".into())
    };
    let keys = || Some(vec!["GH_TOKEN".to_string(), "GITHUB_TOKEN".to_string()]);
    let mut cases = vec![
        ("nothing to say".to_string(), notes(example.facts())),
        (
            "fast-forward suggestion".to_string(),
            notes(TableFacts { ff_suggestion: Some(FfSuggestion { behind: 3 }), ..example.facts() }),
        ),
        (
            "fast-forward suggestion, one commit".to_string(),
            notes(TableFacts { ff_suggestion: Some(FfSuggestion { behind: 1 }), ..example.facts() }),
        ),
        ("fallback notice".to_string(), notes(TableFacts { fallback_notice: keys(), ..example.facts() })),
        (
            "graph omissions".to_string(),
            notes(TableFacts {
                graph_omissions: GraphOmissions {
                    hidden_lanes: 3,
                    omissions: vec![
                        GraphOmission::UnconnectedLane {
                            branch: "fix/globs".into(),
                            fork: Some("e23d0c2b3a00fa6369b00619374eaa9c9664ff40".into()),
                        },
                        GraphOmission::UnconnectedLane { branch: "feat/far".into(), fork: Some("abcdef0123456789".into()) },
                        GraphOmission::UnconnectedLane { branch: "feat/lost".into(), fork: None },
                        GraphOmission::Merge { branch: "feat/m".into(), destination: "8888888abc".into() },
                        GraphOmission::Tag("v0.1.0".into()),
                    ],
                    forked_off_line: vec![ForkedOffLine {
                        branch: "fix/globs".into(),
                        fork: "e23d0c2b3a00fa6369b00619374eaa9c9664ff40".into(),
                        into: "origin/main".into(),
                        merge: "21debbcc5c98000f5b9a331be59e7ef24cdde66f".into(),
                    }],
                    merged_elsewhere: vec![
                        MergedElsewhere {
                            branch: "feat/schema".into(),
                            into: "origin/main".into(),
                            through: Some("fix/skill".into()),
                        },
                        MergedElsewhere { branch: "feat/old".into(), into: "main".into(), through: None },
                    ],
                    ..GraphOmissions::default()
                },
                ..example.facts()
            }),
        ),
        (
            "one hidden lane".to_string(),
            notes(TableFacts {
                graph_omissions: GraphOmissions { hidden_lanes: 1, ..GraphOmissions::default() },
                ..example.facts()
            }),
        ),
        (
            "incomplete in a shallow clone".to_string(),
            notes(TableFacts {
                graph_omissions: GraphOmissions {
                    history_gap: true,
                    shallow: true,
                    omissions: vec![GraphOmission::UnconnectedLane { branch: "feat/lost".into(), fork: None }],
                    ..GraphOmissions::default()
                },
                ..example.facts()
            }),
        ),
        (
            "a history gap in a complete clone".to_string(),
            notes(TableFacts {
                graph_omissions: GraphOmissions { history_gap: true, ..GraphOmissions::default() },
                ..example.facts()
            }),
        ),
        (
            "shallow but complete".to_string(),
            notes(TableFacts {
                graph_omissions: GraphOmissions { shallow: true, ..GraphOmissions::default() },
                ..example.facts()
            }),
        ),
        (
            "suggestion before the notice".to_string(),
            notes(TableFacts {
                ff_suggestion: Some(FfSuggestion { behind: 3 }),
                fallback_notice: keys(),
                ..example.facts()
            }),
        ),
    ];
    for (label, notice) in [
        ("dirty checkout", FfNotice::DirtyCheckout),
        ("diverged", FfNotice::Diverged),
        ("missing local branch", FfNotice::Missing("main".into())),
        ("missing tracking ref", FfNotice::Missing("origin/main".into())),
        ("changed while waiting", FfNotice::Changed),
        ("git failed", FfNotice::Failed),
        ("holder unavailable", FfNotice::UnavailableHolder(PathBuf::from("/code/wts/main-checkout"))),
    ] {
        cases.push((format!("--ff refused: {label}"), notes(TableFacts { ff_notice: Some(notice), ..example.facts() })));
    }
    insta::assert_snapshot!("closing_notes", labeled(cases));
}

#[test]
fn commands_in_the_closing_notes_are_in_reverse_video() {
    let example = Example::new();
    let facts = TableFacts {
        ff_suggestion: Some(FfSuggestion { behind: 3 }),
        fallback_notice: Some(vec!["GH_TOKEN".to_string()]),
        ..example.facts()
    };
    let colored = list_table::render_notes(&facts, &terminal_at(400, true)).expect("notes");
    for command in [" wt --ff ", " --ignore-api "] {
        assert!(colored.contains(&format!("\u{1b}[7m{command}")), "{command:?} in reverse video: {colored:?}");
    }
}

#[test]
fn output_order_snapshot() {
    let example = Example::new();
    let terminal = terminal_at(400, false);
    let facts = TableFacts {
        pr_outcome: Some(PrOutcome::Pending),
        timed_out: true,
        ff_suggestion: Some(FfSuggestion { behind: 7 }),
        fallback_notice: Some(vec!["GH_TOKEN".to_string(), "GITHUB_TOKEN".to_string()]),
        credential_line: Some(CredentialLine {
            provider: "GitHub".into(),
            key: "GITHUB_TOKEN".into(),
            condition: CredentialCondition::RateLimited { authenticated: true },
        }),
        prs: &PrListing { fetched_at: Some(NOW - 600), ..example.prs.clone() },
        ..example.facts()
    };
    let table = list_table::render(&facts, &terminal, NOW);
    let status = list_table::render_status(&facts, &terminal, NOW);
    let notes = list_table::render_notes(&facts, &terminal);
    let graph = "<the git graph>\n";
    let verbose = "<the verbose section>\n";
    let assemble = |graph: Option<&str>, verbose: Option<&str>| {
        list_table::assemble(Sections {
            table: &table,
            graph,
            status: status.as_deref(),
            verbose,
            notes: notes.as_deref(),
        })
    };
    insta::assert_snapshot!(
        "output_order",
        labeled(vec![
            ("with a graph".into(), assemble(Some(graph), None)),
            ("without a graph".into(), assemble(None, None)),
            ("with a graph and --verbose".into(), assemble(Some(graph), Some(verbose))),
            ("without a graph, with --verbose".into(), assemble(None, Some(verbose))),
        ])
    );
}

/// Worktrees Git can't read, beside healthy ones: one row per availability,
/// plus a readable row whose status check failed. `lhg-before` has the shape
/// of the observed case: detached at a recorded HEAD, its `.git` file gone.
struct Unavailable {
    statuses: Vec<WorktreeStatus>,
    tree: Vec<TreeRow>,
    comparisons: HashMap<String, BranchComparisons>,
    target: DefaultTarget,
    prs: PrListing,
}

const GIT_REASON: &str = "gitdir file points to non-existent location";

fn unavailable(dir: &str, branch: Option<&str>, availability: Availability, reason: &str) -> WorktreeStatus {
    let mut status = status(dir, branch, false, false, DirtyStatus::Unknown);
    status.entry.prunable = Some(reason.to_string());
    status.availability = availability;
    status
}

impl Unavailable {
    fn new() -> Self {
        Self::with(vec![
            status("rusty-biscuit", Some("main"), true, true, DirtyStatus::Clean),
            status("feat-ok", Some("feat/ok"), false, false, DirtyStatus::Clean),
            unavailable("lhg-before", None, Availability::Unlinked, GIT_REASON),
            unavailable("feat-gone", Some("feat/gone"), Availability::Missing, GIT_REASON),
            unavailable("feat-link", Some("feat/link"), Availability::Other(OtherCondition::Link), GIT_REASON),
            unavailable("feat-file", Some("feat/file"), Availability::Other(OtherCondition::NotADirectory), ""),
            unavailable(
                "feat-locked",
                Some("feat/locked"),
                Availability::Other(OtherCondition::PathUninspectable("Permission denied (os error 13)".into())),
                GIT_REASON,
            ),
            unavailable(
                "feat-gitdir",
                Some("feat/gitdir"),
                Availability::Other(OtherCondition::GitEntryUninspectable("Permission denied (os error 13)".into())),
                GIT_REASON,
            ),
            unavailable("feat-broken", Some("feat/broken"), Availability::Other(OtherCondition::GitEntryPresent), GIT_REASON),
            status("feat-unknown", Some("feat/unknown"), false, false, DirtyStatus::Unknown),
        ])
    }

    fn with(statuses: Vec<WorktreeStatus>) -> Self {
        let local: BTreeMap<String, String> = statuses
            .iter()
            .filter_map(|status| status.entry.branch.clone())
            .map(|branch| (branch, "0".repeat(40)))
            .collect();
        let entries: Vec<WorktreeEntry> = statuses.iter().map(|s| s.entry.clone()).collect();
        let tree = build_tree(&entries, "main", &local, &ForkOriginStore::default());
        let comparisons = local
            .keys()
            .filter(|branch| *branch != "main")
            .map(|branch| {
                (branch.clone(), BranchComparisons { target: Some(CLEAN), parent: ParentComparison::NotApplicable })
            })
            .collect();
        Self {
            statuses,
            tree,
            comparisons,
            target: DefaultTarget { reference: "main".to_string(), sha: "f".repeat(40), diverged: false },
            prs: PrListing { source_repo: None, pull_requests: Vec::new(), fetched_at: None },
        }
    }

    fn facts(&self) -> TableFacts<'_> {
        TableFacts {
            default_branch: "main",
            target: Some(&self.target),
            caption: None,
            tree: &self.tree,
            statuses: &self.statuses,
            comparisons: &self.comparisons,
            prs: &self.prs,
            remote: None,
            credential_line: None,
            pr_outcome: None,
            timed_out: false,
            ff_suggestion: None,
            ff_notice: None,
            fallback_notice: None,
            graph_omissions: GraphOmissions::default(),
        }
    }

    fn table(&self) -> String {
        list_table::render(&self.facts(), &plain_terminal(), NOW)
    }

    fn notes(&self) -> String {
        list_table::render_notes(&self.facts(), &terminal_at(400, false)).expect("notes")
    }
}

/// The Worktree cell of the row for directory `dir`.
fn worktree_cell(rendered: &str, dir: &str) -> String {
    let line = rendered
        .lines()
        .find(|line| line.split('│').nth(1).is_some_and(|cell| cell.trim().ends_with(dir)))
        .unwrap_or_else(|| panic!("no row for {dir:?} in:\n{rendered}"));
    line.split('│').nth(1).unwrap().trim().to_string()
}

#[test]
fn unavailable_and_unknown_rows_snapshot() {
    insta::assert_snapshot!("unavailable_and_unknown_rows", Unavailable::new().table());
}

#[test]
fn a_row_git_cannot_read_is_a_cross_never_clean() {
    let rendered = Unavailable::new().table();
    for dir in ["lhg-before", "feat-gone", "feat-link", "feat-file", "feat-locked", "feat-gitdir", "feat-broken"] {
        assert_eq!(worktree_cell(&rendered, dir), format!("✕ {dir}"), "in:\n{rendered}");
    }
    assert_eq!(worktree_cell(&rendered, "feat-unknown"), "? feat-unknown");
    assert_eq!(worktree_cell(&rendered, "feat-ok"), "○ feat-ok");
}

#[test]
fn unavailability_outranks_any_dirtiness() {
    for dirty in [DirtyStatus::Clean, DirtyStatus::DirtyNonSource, DirtyStatus::DirtySource, DirtyStatus::Unknown] {
        let mut row = unavailable("feat-gone", Some("feat/gone"), Availability::Missing, GIT_REASON);
        row.dirty = dirty;
        let example = Unavailable::with(vec![status("repo", Some("main"), true, true, DirtyStatus::Clean), row]);
        let rendered = example.table();
        assert_eq!(worktree_cell(&rendered, "feat-gone"), "✕ feat-gone", "{dirty:?}");
        assert!(!rendered.contains("couldn't check"), "a ✕ row adds no ? legend entry: {dirty:?}");
    }
}

#[test]
fn the_cross_is_red_and_the_question_mark_dim() {
    let example = Unavailable::new();
    let colored = list_table::render(&example.facts(), &color_terminal(), NOW);
    assert!(row(&colored, "lhg-before").contains("\u{1b}[31m✕"), "{colored:?}");
    assert!(row(&colored, "feat-unknown").contains("\u{1b}[2m?"), "{colored:?}");
}

#[test]
fn legend_entries_appear_only_for_markers_in_the_table() {
    let base = || status("repo", Some("main"), true, true, DirtyStatus::Clean);
    let legend = |statuses: Vec<WorktreeStatus>| {
        let rendered = Unavailable::with(statuses).table();
        let start = rendered.lines().position(|line| line.starts_with(" Worktree ")).expect("legend");
        rendered.lines().skip(start).map(str::trim_end).collect::<Vec<_>>().join("\n")
    };
    let healthy = " Worktree   ○ clean    ● uncommitted files    ● uncommitted source files";
    let branch = " Branch     └─ merges cleanly into parent     └─ conflicts with parent      └┄ parent deleted";

    assert_eq!(legend(vec![base()]), format!("{healthy}\n{branch}"));
    assert_eq!(
        legend(vec![base(), unavailable("gone", Some("gone"), Availability::Missing, GIT_REASON)]),
        format!("{healthy}    ✕ git can't read worktree\n{branch}")
    );
    assert_eq!(
        legend(vec![base(), status("odd", Some("odd"), false, false, DirtyStatus::Unknown)]),
        format!("{healthy}    ? couldn't check\n{branch}")
    );
    assert_eq!(
        legend(vec![
            base(),
            status("odd", Some("odd"), false, false, DirtyStatus::Unknown),
            unavailable("gone", Some("gone"), Availability::Unlinked, GIT_REASON),
        ]),
        format!("{healthy}    ✕ git can't read worktree    ? couldn't check\n{branch}")
    );
    let with_mark = format!("{healthy}    ✕ git can't read worktree");
    let column = |line: &str, needle: &str| line[..line.find(needle).unwrap()].chars().count();
    assert_eq!(column(branch, "└┄"), column(&with_mark, "✕"), "the parent-deleted elbow sits under the ✕");
}

/// Git reads a checkout through a link that replaced it and does not mark it
/// `prunable`; the row is `✕`, and nothing claims Git can't read it.
#[test]
fn a_readable_replacement_link_is_unavailable_without_claiming_git_cant_read_it() {
    let replaced = || {
        let mut row = unavailable("feat-link", Some("feat/link"), Availability::Other(OtherCondition::Link), "");
        row.entry.prunable = None;
        row
    };
    let base = || status("repo", Some("main"), true, true, DirtyStatus::Clean);
    let legend_line = |statuses: Vec<WorktreeStatus>| {
        let rendered = Unavailable::with(statuses).table();
        let worktree = rendered.lines().find(|line| line.starts_with(" Worktree "))?;
        worktree.find('✕').map(|at| worktree[at..].trim().to_string())
    };

    let table = Unavailable::with(vec![base(), replaced()]).table();
    assert!(table.lines().any(|line| line.contains("✕ feat-link")), "{table}");
    assert_eq!(legend_line(vec![base(), replaced()]).as_deref(), Some("✕ its path is a link"));
    assert_eq!(
        legend_line(vec![base(), replaced(), unavailable("gone", Some("gone"), Availability::Missing, GIT_REASON)]).as_deref(),
        Some("✕ git can't read worktree, or its path is a link")
    );
    let note = note_for(replaced(), vec![]);
    let words = note.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.contains("feat-link: /code/wts/feat-link is a link, which may have replaced the original checkout."),
        "{words}"
    );
    assert!(!words.contains("Git can't read"), "{words}");
}

#[test]
fn a_narrow_table_is_as_wide_as_its_widest_legend_line() {
    let example = Unavailable::with(vec![
        status("repo", Some("main"), true, true, DirtyStatus::Clean),
        status("odd", Some("odd"), false, false, DirtyStatus::Unknown),
        unavailable("gone", Some("gone"), Availability::Missing, GIT_REASON),
    ]);
    // Wide enough for the Worktree legend line with both marker entries.
    let rendered = list_table::render(&example.facts(), &terminal_at(160, false), NOW);
    let width = |line: &str| line.trim_end().chars().count();
    let legend_start = rendered.lines().position(|line| line.starts_with(" Worktree ")).expect("legend");
    let widest = rendered.lines().skip(legend_start).map(width).max().unwrap();
    let table_lines: Vec<&str> = rendered.lines().filter(|line| line.starts_with(['┌', '│', '├', '└'])).collect();
    assert_eq!(table_lines.len(), 7, "{rendered}");
    for line in table_lines {
        assert_eq!(width(line), widest, "{line:?} in:\n{rendered}");
    }
}

#[test]
fn comparisons_of_unavailable_rows_keep_their_meaning() {
    let rendered = Unavailable::new().table();
    let cells = |dir: &str| -> Vec<String> {
        let line = rendered.lines().find(|line| line.contains(dir)).expect("row");
        line.split('│').skip(3).map(|cell| cell.trim().to_string()).collect()
    };
    let healthy = cells("feat-ok");
    assert!(healthy[0].starts_with("clean"), "{rendered}");
    for dir in ["lhg-before", "feat-gone", "feat-link", "feat-unknown"] {
        let expected = if dir == "lhg-before" { vec!["—".to_string(), "—".to_string(), String::new()] } else { healthy.clone() };
        assert_eq!(cells(dir), expected, "comparisons come from refs alone: {dir}");
    }
}

#[test]
fn unavailable_notes_snapshot() {
    insta::assert_snapshot!("unavailable_notes", Unavailable::new().notes());
}

#[test]
fn unavailable_notes_follow_the_existing_notes_in_table_row_order() {
    let example = Unavailable::new();
    let facts = TableFacts {
        ff_notice: Some(FfNotice::Diverged),
        fallback_notice: Some(vec!["GH_TOKEN".to_string()]),
        ..example.facts()
    };
    let notes = list_table::render_notes(&facts, &terminal_at(400, false)).expect("notes");
    let starts: Vec<&str> = notes
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("- "))
        .map(|line| line.split([':', ' ']).next().unwrap())
        .collect();
    let table = example.table();
    let row_order: Vec<&str> = table
        .lines()
        .filter_map(|line| line.split('│').nth(1))
        .map(str::trim)
        .filter_map(|cell| cell.strip_prefix("✕ "))
        .collect();
    assert_eq!(row_order.len(), 7, "{table}");
    let mut expected = vec!["main", "Git", "Set"];
    expected.extend(row_order);
    assert_eq!(starts, expected, "{notes}");
}

#[test]
fn unavailable_notes_are_not_dim() {
    let example = Unavailable::new();
    let colored = list_table::render_notes(&example.facts(), &terminal_at(400, true)).expect("notes");
    assert!(!colored.contains("\u{1b}[2m"), "styled like the other notes: {colored:?}");
    assert!(colored.contains("\u{1b}[7m wt remove lhg-before "), "commands in reverse video: {colored:?}");
}

#[test]
fn healthy_listings_have_no_unavailable_notes() {
    let example = Example::new();
    assert_eq!(list_table::render_notes(&example.facts(), &terminal_at(400, false)), None);
    let example = Unavailable::with(vec![
        status("repo", Some("main"), true, true, DirtyStatus::Clean),
        status("odd", Some("odd"), false, false, DirtyStatus::Unknown),
    ]);
    assert_eq!(list_table::render_notes(&example.facts(), &terminal_at(400, false)), None, "? needs no note");
}

/// One note, for a single unavailable worktree at `path` beside the base.
fn note_for(entry: WorktreeStatus, others: Vec<WorktreeStatus>) -> String {
    let mut statuses = vec![status("repo", Some("main"), true, true, DirtyStatus::Clean)];
    statuses.extend(others);
    statuses.push(entry);
    Unavailable::with(statuses).notes()
}

fn at(path: &str, branch: Option<&str>, availability: Availability, reason: &str) -> WorktreeStatus {
    let mut status = unavailable("x", branch, availability, reason);
    status.entry.path = PathBuf::from(path);
    status
}

#[test]
fn notes_quote_names_and_paths_for_every_shell_or_suggest_no_command() {
    let unlinked = |path: &str, branch: Option<&str>| note_for(at(path, branch, Availability::Unlinked, GIT_REASON), vec![]);
    insta::assert_snapshot!(
        "unavailable_note_quoting",
        labeled(vec![
            ("plain".into(), unlinked("/code/wts/plain", Some("feat/plain"))),
            ("spaces in the path".into(), unlinked("/code/wts/my work", None)),
            ("a single quote in the path".into(), unlinked("/code/wts/it's", None)),
            ("a PowerShell quote in the path".into(), unlinked("/code/wts/it\u{2019}s", None)),
            ("shell metacharacters".into(), unlinked("/code/wts/$(rm -rf ~);x", None)),
            ("a leading dash".into(), unlinked("/code/wts/-n", None)),
        ])
    );
}

#[test]
fn notes_never_suggest_a_name_that_selects_another_worktree() {
    // Two worktrees share the basename `dup`, and the target's branch is
    // another worktree's directory name.
    let target = at("/code/a/dup", Some("other"), Availability::Missing, GIT_REASON);
    let others = vec![
        {
            let mut status = status("dup", Some("feat/dup"), false, false, DirtyStatus::Clean);
            status.entry.path = PathBuf::from("/code/b/dup");
            status
        },
        status("other", Some("feat/other"), false, false, DirtyStatus::Clean),
    ];
    let note = note_for(target, others);
    assert!(!note.contains("wt remove dup") && !note.contains("wt remove other"), "{note}");
    assert!(note.contains("No name selects it for wt remove"), "{note}");
    assert!(note.contains("dup also matches /code/b/dup"), "{note}");
    assert!(note.contains("other also matches /code/wts/other"), "{note}");

    // The branch is preferred when it alone selects the row.
    let note = note_for(at("/code/a/dup", Some("feat/unique"), Availability::Missing, GIT_REASON), vec![]);
    assert!(note.contains("wt remove feat/unique"), "{note}");
    // A detached row falls back to its directory name.
    let note = note_for(at("/code/a/lone", None, Availability::Missing, GIT_REASON), vec![]);
    assert!(note.contains("wt remove lone"), "{note}");
}

/// Paths and commands in the notes are copied by users, so a narrow wrap
/// never breaks them: one that fits moves to the next line whole, and one
/// longer than a line gets a line of its own for the terminal to soft-wrap,
/// never a forced break with an inserted `-`.
#[test]
fn notes_keep_paths_and_commands_whole_at_a_narrow_width() {
    let width = 60;
    let long_dir = format!("/private/var/folders/xy/{}/T/.tmpAbCdEf", "q".repeat(40));
    let long = |name: &str| format!("{long_dir}/{name}");
    let mut base = status("repo", Some("main"), true, true, DirtyStatus::Clean);
    base.entry.path = PathBuf::from(long("main-repo"));
    let mut link = at(&long("wt-moved"), Some("moved"), Availability::Other(OtherCondition::Link), "");
    link.entry.prunable = None;
    let short = at("/code/wts/feat-short-name", None, Availability::Unlinked, GIT_REASON);
    let rows = vec![
        base,
        at(&long("wt-unlinked"), Some("unlinked"), Availability::Unlinked, GIT_REASON),
        link,
        short,
        at("/code/wts/wt-gone", Some("feature/a-rather-long-branch-name"), Availability::Missing, GIT_REASON),
    ];
    let example = Unavailable::with(rows);
    let facts = TableFacts {
        ff_notice: Some(FfNotice::UnavailableHolder(PathBuf::from(long("holder-checkout")))),
        ..example.facts()
    };

    let notes = list_table::render_notes(&facts, &terminal_at(width, false)).expect("notes");

    for copyable in [
        long("wt-moved"),
        long("holder-checkout"),
        "wt remove unlinked".to_string(),
        "wt remove feat-short-name".to_string(),
        "wt remove feature/a-rather-long-branch-name".to_string(),
    ] {
        assert!(notes.contains(&copyable), "{copyable:?} whole in:\n{notes}");
    }
    for line in notes.lines() {
        let fits = line.chars().count() <= width as usize;
        assert!(fits || line.contains(&long_dir), "only a line holding an overlong run is wider: {line:?}\n{notes}");
    }
    // The wrap only moved text to new lines: nothing, such as an inserted
    // `-`, was added to it.
    let unwrapped = list_table::render_notes(&facts, &terminal_at(1000, false)).expect("notes");
    let squeezed = |text: &str| text.split_whitespace().collect::<String>();
    assert_eq!(squeezed(&notes), squeezed(&unwrapped), "{notes}");
    assert!(!notes.chars().any(|ch| ('\u{FDD0}'..='\u{FDD4}').contains(&ch)), "no stand-in survives: {notes:?}");
}

#[test]
fn names_paths_and_reasons_never_become_markup() {
    let hostile = "<red>boom</red>";
    let mut row = at(&format!("/code/wts/{hostile}"), None, Availability::Other(OtherCondition::Link), hostile);
    row.entry.branch = Some(format!("feat/{hostile}"));
    let example = Unavailable::with(vec![status("repo", Some("main"), true, true, DirtyStatus::Clean), row]);

    let notes = list_table::render_notes(&example.facts(), &terminal_at(400, true)).expect("notes");
    assert!(notes.contains(hostile), "shown literally: {notes:?}");
    assert!(!notes.contains("\u{1b}[31mboom"), "never styled red: {notes:?}");
    let table = list_table::render(&example.facts(), &color_terminal(), NOW);
    assert!(table.contains(hostile), "{table:?}");
}

/// One external field projected into the closing notes, and the literal
/// text the notes must show for it.
struct Projection {
    name: &'static str,
    example: Unavailable,
    default_branch: String,
    ff_notice: Option<FfNotice>,
    fallback_notice: Option<Vec<String>>,
    shown: Vec<String>,
}

fn projections(long_dir: &str, odd: &str) -> Vec<Projection> {
    let base = || status("repo", Some("main"), true, true, DirtyStatus::Clean);
    let detached = |path: String, availability: Availability, reason: &str| at(&path, None, availability, reason);
    let projection = |name, rows: Vec<WorktreeStatus>, shown: Vec<String>| Projection {
        name,
        example: Unavailable::with([vec![base()], rows].concat()),
        default_branch: "main".to_string(),
        ff_notice: None,
        fallback_notice: None,
        shown,
    };
    let mut link = detached(format!("{long_dir}/link{odd}"), Availability::Other(OtherCondition::Link), "");
    link.entry.prunable = None;
    let mut competing = status("dup", Some("feat/dup"), false, false, DirtyStatus::Clean);
    competing.entry.path = PathBuf::from(format!("{long_dir}/b{odd}/dup"));
    let other = |condition: OtherCondition, reason: &str| {
        vec![detached("/code/wts/x".to_string(), Availability::Other(condition), reason)]
    };
    let ff = |name, default_branch: String, notice: Option<FfNotice>, keys: Option<Vec<String>>, shown| Projection {
        default_branch,
        ff_notice: notice,
        fallback_notice: keys,
        ..projection(name, vec![], shown)
    };
    vec![
        projection(
            "missing-entry removal command and worktree label",
            vec![detached(format!("/code/wts/target{odd}"), Availability::Missing, GIT_REASON)],
            vec![format!("- target{odd}:"), format!("wt remove 'target{odd}'")],
        ),
        projection(
            "unlinked removal command and worktree label",
            vec![detached(format!("{long_dir}/unlinked{odd}"), Availability::Unlinked, GIT_REASON)],
            vec![format!("- unlinked{odd}:"), format!("wt remove 'unlinked{odd}'")],
        ),
        projection("other-unavailable observed path", vec![link], vec![format!("{long_dir}/link{odd}")]),
        projection(
            "name-conflict competing path",
            vec![competing, at("/code/a/dup", None, Availability::Missing, GIT_REASON)],
            vec![format!("{long_dir}/b{odd}/dup")],
        ),
        ff(
            "fast-forward refusal holder path",
            "main".to_string(),
            Some(FfNotice::UnavailableHolder(PathBuf::from(format!("{long_dir}/holder{odd}")))),
            None,
            vec![format!("{long_dir}/holder{odd}")],
        ),
        projection(
            "Git reason",
            other(OtherCondition::GitEntryPresent, &format!("gone{odd}")),
            vec![format!("reports: gone{odd}")],
        ),
        projection(
            "path inspection error",
            other(OtherCondition::PathUninspectable(format!("denied{odd}")), GIT_REASON),
            vec![format!("(denied{odd})")],
        ),
        projection(
            ".git inspection error",
            other(OtherCondition::GitEntryUninspectable(format!("denied{odd}")), GIT_REASON),
            vec![format!("(denied{odd})")],
        ),
        ff(
            "default-branch note",
            format!("main{odd}"),
            Some(FfNotice::Diverged),
            None,
            vec![format!("main{odd}"), format!("origin/main{odd},")],
        ),
        ff(
            "missing-ref note",
            "main".to_string(),
            Some(FfNotice::Missing(format!("origin/main{odd}"))),
            None,
            vec![format!("fast-forwarded: origin/main{odd}")],
        ),
        ff(
            "fallback-key note",
            "main".to_string(),
            None,
            Some(vec![format!("GITHUB_TOKEN{odd}"), format!("GH{odd}")]),
            vec![format!("GITHUB_TOKEN{odd}"), format!("GH{odd}")],
        ),
    ]
}

/// Every external field the closing notes show, holding ordinary markup or
/// that markup behind any of U+FDD0..U+FDD4 (characters an earlier renderer
/// used as its own delimiters), renders literally at a wide and a narrow
/// width: markup is never styled, and copyable paths and commands are never
/// broken.
#[test]
fn every_external_field_in_the_notes_is_shown_literally() {
    let long_dir = format!("/private/var/folders/xy/{}/T", "q".repeat(40));
    for injected in ["", "\u{FDD0}", "\u{FDD1}", "\u{FDD2}", "\u{FDD3}", "\u{FDD4}"] {
        let odd = format!("{injected}<red>INJECTED");
        for width in [400, 60] {
            for case in projections(&long_dir, &odd) {
                let facts = TableFacts {
                    default_branch: &case.default_branch,
                    ff_notice: case.ff_notice.clone(),
                    fallback_notice: case.fallback_notice.clone(),
                    ..case.example.facts()
                };
                let colored = list_table::render_notes(&facts, &terminal_at(width, true)).expect("notes");
                let notes = biscuit_test_harness::strip_ansi(&colored);
                let context = format!("{} with {injected:?} at {width} columns:\n{notes}", case.name);
                for shown in &case.shown {
                    assert!(notes.contains(shown.as_str()), "{shown:?} whole in {context}");
                }
                if injected.is_empty() {
                    assert!(!notes.chars().any(|ch| ('\u{FDD0}'..='\u{FDD4}').contains(&ch)), "{context}");
                }
                for line in notes.lines() {
                    let fits = line.chars().count() <= width as usize;
                    assert!(fits || line.contains(&long_dir), "only an overlong run is wider: {line:?} in {context}");
                }
            }
        }
    }
}

#[test]
fn a_note_claims_a_missing_git_file_only_for_an_unlinked_row() {
    let notes = Unavailable::new().notes();
    let claims: Vec<&str> = notes.lines().filter(|line| line.contains(".git file is missing")).collect();
    assert_eq!(claims.len(), 1, "{notes}");
    assert!(claims[0].contains("lhg-before"), "{notes}");
}

/// An unavailable worktree's note is styled like every other note, its
/// command badge exactly like the `wt --ff` badge.
#[test]
fn an_unavailable_note_is_styled_like_the_other_notes() {
    let example = Unavailable::with(vec![
        status("repo", Some("main"), true, true, DirtyStatus::Clean),
        at("/code/wts/plain", Some("feat/plain"), Availability::Unlinked, GIT_REASON),
    ]);
    let facts = TableFacts { ff_suggestion: Some(FfSuggestion { behind: 3 }), ..example.facts() };
    let notes = list_table::render_notes(&facts, &terminal_at(400, true)).expect("notes");
    assert!(notes.contains("\u{1b}[7m wt --ff \u{1b}[0m"), "{notes:?}");
    assert!(notes.contains("\u{1b}[7m wt remove feat/plain \u{1b}[0m"), "{notes:?}");
    assert!(!notes.contains("\u{1b}[2m"), "nothing is dim: {notes:?}");
}

/// The graph gets the rows the rest of the listing leaves on screen, less
/// four for its own notes and the prompt, and never fewer than the minimum.
#[test]
fn the_graph_gets_the_rows_the_rest_of_the_listing_leaves() {
    let table = "line\n".repeat(20);
    let notes = " - a\n - b\n";
    // 80 rows - (20 + 2 + 1 blank) - 4 reserved.
    assert_eq!(list_table::graph_row_budget(80, &[Some(&table), None, None, Some(notes)]), 53);
    assert_eq!(list_table::graph_row_budget(80, &[Some(&table), Some("status\n"), None, None]), 54);
    assert_eq!(list_table::graph_row_budget(30, &[Some(&table), None, None, Some(notes)]), list_table::GRAPH_MIN_ROWS);
}
