//! The status report `wt remove` prints before asking or removing anything,
//! and the messages that follow it. Builders return Prose markup so tests can
//! read them without a terminal; [`render`] turns the report into text.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::remove::Inventory;
use worktree::remove::remote::{RemoteOnly, RemoteState};
use worktree::remove::safety::{BranchSafety, Commit, Evidence, PrLookup, Tier};

use crate::commands::dirty_tree;

/// Above this many entries a list becomes a count.
pub const LIST_LIMIT: usize = 10;

fn esc(text: &str) -> String {
    Prose::escape_text(text)
}

pub struct ReportInput<'a> {
    pub display_name: &'a str,
    pub path: &'a Path,
    pub branch: Option<&'a str>,
    pub inventory: &'a Inventory,
    /// `None` for a detached worktree.
    pub safety: Option<&'a BranchSafety>,
    pub has_origin: bool,
    /// Present with `--force-remote`.
    pub remote: Option<&'a RemoteState>,
}

/// The whole report, rendered. It ends without a blank line, so each question
/// can start after exactly one.
pub fn render(terminal: &Terminal, input: &ReportInput<'_>) -> String {
    let mut blocks = vec![Prose::new(heading_markup(input)).render(terminal)];
    blocks.push(Prose::new(files_markup(input.inventory)).render(terminal));
    match (input.branch, input.safety) {
        (Some(branch), Some(safety)) => {
            let mut block = Prose::new(format!("<b>Branch</b> <blue>{}</blue>", esc(branch)))
                .render(terminal);
            block.push('\n');
            block.push_str(&render_list(terminal, branch_facts(safety, input.has_origin)));
            blocks.push(block);
        }
        _ => blocks.push(
            Prose::new("<dim>Detached HEAD: there is no branch to delete.</dim>").render(terminal),
        ),
    }
    if let Some(remote) = input.remote {
        blocks.push(Prose::new(remote_markup(remote)).render(terminal));
    }
    blocks
        .iter()
        .map(|block| block.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_list(terminal: &Terminal, items: Vec<String>) -> String {
    let mut list = UnorderedList::empty();
    for item in items {
        list.add(Prose::new(item));
    }
    list.render(terminal)
}

pub fn heading_markup(input: &ReportInput<'_>) -> String {
    format!(
        "\n<b>Removing</b> worktree <blue>{}</blue> <dim>at {}</dim>",
        esc(input.display_name),
        esc(&input.path.display().to_string())
    )
}

/// Dirty files as a tree (or a count above [`LIST_LIMIT`]), then the
/// ignored entries grouped by top-level folder.
pub fn files_markup(inventory: &Inventory) -> String {
    let mut out = String::new();
    let dirty = inventory.dirty.len();
    if dirty == 0 && inventory.ignored.is_empty() {
        return "<dim>No uncommitted or ignored files.</dim>".to_string();
    }
    if dirty > LIST_LIMIT {
        out.push_str(&format!("<red><b>{dirty} uncommitted files</b></red>\n"));
    } else if dirty > 0 {
        out.push_str(&format!("<b>Uncommitted files</b> ({dirty}):\n"));
        let paths: Vec<PathBuf> = inventory.dirty.iter().map(|e| e.path.clone()).collect();
        out.push_str(&dirty_tree::render_markup(&paths));
    }
    if !inventory.ignored.is_empty() {
        if dirty > 0 {
            out.push('\n');
        }
        let groups = inventory.ignored_groups();
        let total = inventory.ignored.len();
        if groups.len() > LIST_LIMIT {
            out.push_str(&format!(
                "<red><b>{total} ignored entries</b></red>; removing the worktree deletes their contents\n"
            ));
        } else {
            let names: Vec<String> = groups
                .iter()
                .map(|group| {
                    let name = esc(&group.name);
                    if group.entries > 1 {
                        format!("<yellow>{name}</yellow> <dim>({} entries)</dim>", group.entries)
                    } else {
                        format!("<yellow>{name}</yellow>")
                    }
                })
                .collect();
            out.push_str(&format!(
                "<b>Ignored files</b>, whose contents removing the worktree deletes: {}\n",
                names.join(", ")
            ));
        }
    }
    out.trim_end().to_string()
}

/// The facts under the branch heading: its tier and evidence, ahead/behind,
/// its origin copy, its PR, and any evidence that did not count.
pub fn branch_facts(safety: &BranchSafety, has_origin: bool) -> Vec<String> {
    let mut facts = vec![tier_markup(safety)];
    if let (Some(target), Some((ahead, behind))) = (&safety.target, safety.ahead_behind) {
        let fetched = if target.reference.starts_with("origin/") {
            " <dim>(as of your last fetch)</dim>"
        } else {
            ""
        };
        facts.push(format!(
            "{ahead} ahead, {behind} behind <b>{}</b>{fetched}",
            esc(&target.reference)
        ));
    }
    if has_origin {
        facts.push(match &safety.origin_copy {
            None => "No copy on origin <dim>(as of your last fetch)</dim>".to_string(),
            Some(copy) => {
                let state = match (copy.missing_local, copy.extra_remote) {
                    (0, 0) => "matches your branch".to_string(),
                    (missing, 0) => format!("lacks {missing} of your commits"),
                    (0, extra) => format!("has {extra} commits your branch lacks"),
                    (missing, extra) => format!(
                        "lacks {missing} of your commits and has {extra} your branch lacks"
                    ),
                };
                format!(
                    "<b>{}</b> {state} <dim>(as of your last fetch)</dim>",
                    esc(&copy.reference)
                )
            }
        });
        facts.push(pr_markup(&safety.pr));
    }
    facts.extend(safety.notes.iter().map(|note| format!("<dim>{}</dim>", esc(note))));
    facts
}

fn pr_markup(pr: &PrLookup) -> String {
    match pr {
        PrLookup::Found(pr) => {
            let state = if pr.open { "open" } else { "merged" };
            let number = match &pr.url {
                Some(url) => format!("<a href=\"{}\">PR #{}</a>", esc(url), pr.number),
                None => format!("PR #{}", pr.number),
            };
            let target = pr
                .target_branch
                .as_deref()
                .map(|t| format!(" into <b>{}</b>", esc(t)))
                .unwrap_or_default();
            format!("{number} ({state}){target}")
        }
        PrLookup::NoneFound => "No open or merged PR".to_string(),
        PrLookup::Unavailable(reason) => {
            format!("<dim>PR status unavailable: {}</dim>", esc(reason))
        }
    }
}

pub fn tier_markup(safety: &BranchSafety) -> String {
    let (label, evidence) = match &safety.tier {
        Tier::Safe(evidence) => ("<green><b>Safe</b></green>", evidence),
        Tier::PrettySafe(evidence) => ("<green><b>Pretty safe</b></green>", evidence),
        Tier::NotSafe => {
            let what = match &safety.lost_commits {
                Ok(commits) if commits.len() == 1 => "1 commit exists".to_string(),
                Ok(commits) => format!("{} exist", commit_count(commits.len())),
                Err(_) => "its commits exist".to_string(),
            };
            return format!("<red><b>Not safe</b></red>: {what} nowhere else");
        }
        Tier::Unknown(reason) => {
            return format!(
                "<red><b>Not safe</b></red>: where its commits live could not be checked ({}), so it is treated as not safe",
                esc(reason)
            );
        }
    };
    let phrase = match evidence {
        Evidence::DefaultBranch(reference) => {
            let fetched = if reference.starts_with("origin/") {
                " <dim>(as of your last fetch)</dim>"
            } else {
                ""
            };
            format!("its commits are on <b>{}</b>{fetched}", esc(reference))
        }
        Evidence::PullRequest { number, merged, .. } => {
            let state = if *merged { "merged" } else { "open" };
            format!("it is the head of {state} PR #{number}")
        }
        Evidence::LocalBranch(branch) => format!(
            "your commits also live on <b>{}</b>, which is not merged into the default branch yet",
            esc(branch)
        ),
        Evidence::Tag(tag) => format!("your commits also live on tag <b>{}</b>", esc(tag)),
        Evidence::RemoteBranch(reference) => format!(
            "your commits also live on <b>{}</b> <dim>(checked on origin just now)</dim>",
            esc(reference)
        ),
    };
    format!("{label}: {phrase}")
}

pub(super) fn commit_count(count: usize) -> String {
    if count == 1 {
        "1 commit".to_string()
    } else {
        format!("{count} commits")
    }
}

/// The commits deleting the branch would lose: subjects, or a count
/// above [`LIST_LIMIT`].
pub fn lost_commits_markup(commits: &[Commit]) -> String {
    if commits.len() > LIST_LIMIT {
        return format!(
            "Deleting the branch would lose <red><b>{}</b></red>.",
            commit_count(commits.len())
        );
    }
    let mut out = format!(
        "Deleting the branch would lose {}:\n",
        commit_count(commits.len())
    );
    for commit in commits {
        out.push_str(&format!(
            "  <dim>{}</dim> {}\n",
            esc(&commit.short_sha),
            esc(&commit.subject)
        ));
    }
    out.trim_end().to_string()
}

/// What `--force-remote` will find on origin.
pub fn remote_markup(state: &RemoteState) -> String {
    match state {
        RemoteState::NoRemote => {
            "<b>Origin</b>: there is no origin remote, so no remote branch to delete.".to_string()
        }
        RemoteState::MultiplePushUrls { endpoints, .. } => format!(
            "<b>Origin</b>: <yellow>origin pushes to {} repositories</yellow> ({}), so <i>--force-remote</i> cannot delete from exactly one.",
            endpoints.len(),
            esc(&endpoints.join(", "))
        ),
        RemoteState::Absent {
            destination,
            endpoint,
        } => format!(
            "{} no matching remote branch exists (<b>{}</b>), so there is nothing to delete there.",
            origin_label(endpoint),
            esc(destination)
        ),
        RemoteState::Unavailable {
            destination,
            endpoint,
            reason,
        } => format!(
            "{} <yellow>origin could not be reached</yellow> ({}), so <b>origin/{}</b> cannot be deleted now.",
            origin_label(endpoint),
            esc(reason),
            esc(destination)
        ),
        RemoteState::Present {
            destination,
            endpoint,
            remote_only,
            ..
        } => {
            let label = origin_label(endpoint);
            let name = format!("<b>origin/{}</b>", esc(destination));
            match remote_only {
                RemoteOnly::Unknown => format!(
                    "{label} {name} will be deleted. It has commits that were never fetched here; their number and contents are unknown."
                ),
                RemoteOnly::Known(commits) if commits.is_empty() => format!(
                    "{label} {name} will be deleted; it has no commits your branch lacks."
                ),
                RemoteOnly::Known(commits) if commits.len() > LIST_LIMIT => format!(
                    "{label} {name} will be deleted, with <red><b>{}</b></red> that exist only there.",
                    commit_count(commits.len())
                ),
                RemoteOnly::Known(commits) => {
                    let mut out = format!(
                        "{label} {name} will be deleted, with {} that exist only there:\n",
                        commit_count(commits.len())
                    );
                    for commit in commits {
                        out.push_str(&format!(
                            "  <dim>{}</dim> {}\n",
                            esc(&commit.short_sha),
                            esc(&commit.subject)
                        ));
                    }
                    out.trim_end().to_string()
                }
            }
        }
    }
}

/// Names the push URL, since the report must describe the repository the
/// deletion modifies, which need not be the one `origin` fetches from.
fn origin_label(endpoint: &str) -> String {
    if endpoint.is_empty() {
        "<b>Origin</b>:".to_string()
    } else {
        format!("<b>Origin</b> <dim>({})</dim>:", esc(endpoint))
    }
}

#[cfg(test)]
mod tests {
    use worktree::default_target::DefaultTarget;
    use worktree::remove::safety::{OriginCopy, PullRequest};
    use worktree::remove::DirtyEntry;

    use super::*;

    fn dirty(count: usize) -> Inventory {
        Inventory {
            dirty: (0..count)
                .map(|i| DirtyEntry {
                    status: "??".into(),
                    path: PathBuf::from(format!("f{i:02}.rs")),
                    is_source: true,
                })
                .collect(),
            ignored: Vec::new(),
        }
    }

    fn commits(count: usize) -> Vec<Commit> {
        (0..count)
            .map(|i| Commit {
                short_sha: format!("abc{i:04}"),
                subject: format!("change <{i}>"),
            })
            .collect()
    }

    fn safety(tier: Tier) -> BranchSafety {
        BranchSafety {
            tier,
            notes: Vec::new(),
            lost_commits: Ok(commits(4)),
            target: Some(DefaultTarget {
                reference: "origin/main".into(),
                sha: "0".repeat(40),
                diverged: false,
            }),
            ahead_behind: Some((3, 12)),
            origin_copy: None,
            pr: PrLookup::NoneFound,
        }
    }

    #[test]
    fn up_to_ten_dirty_files_are_a_tree_and_more_are_a_bold_red_count() {
        let ten = files_markup(&dirty(10));
        assert!(ten.starts_with("<b>Uncommitted files</b> (10):"));
        assert!(ten.contains("<orange>f09.rs</orange>"));

        let eleven = files_markup(&dirty(11));
        assert_eq!(eleven, "<red><b>11 uncommitted files</b></red>");
    }

    #[test]
    fn ignored_entries_are_grouped_and_say_their_contents_are_deleted() {
        let inventory = Inventory {
            dirty: Vec::new(),
            ignored: vec![".env".into(), "notes.md".into(), "target/debug/".into(), "target/CACHEDIR.TAG".into()],
        };
        let markup = files_markup(&inventory);
        assert!(markup.contains("contents removing the worktree deletes"));
        assert!(markup.contains("<yellow>.env</yellow>, <yellow>notes.md</yellow>, <yellow>target/</yellow> <dim>(2 entries)</dim>"), "{markup}");

        let many = Inventory {
            dirty: Vec::new(),
            ignored: (0..11).map(|i| format!("dir{i}/")).collect(),
        };
        assert!(files_markup(&many).starts_with("<red><b>11 ignored entries</b></red>"));
        assert_eq!(files_markup(&Inventory::default()), "<dim>No uncommitted or ignored files.</dim>");
    }

    #[test]
    fn every_tier_names_its_evidence() {
        let cases = [
            (Tier::Safe(Evidence::DefaultBranch("main".into())), "its commits are on <b>main</b>"),
            (Tier::Safe(Evidence::DefaultBranch("origin/main".into())), "(as of your last fetch)"),
            (
                Tier::Safe(Evidence::PullRequest { number: 99, merged: true, url: None }),
                "head of merged PR #99",
            ),
            (Tier::PrettySafe(Evidence::LocalBranch("feat/theme".into())), "also live on <b>feat/theme</b>, which is not merged"),
            (Tier::PrettySafe(Evidence::Tag("v1".into())), "tag <b>v1</b>"),
            (Tier::PrettySafe(Evidence::RemoteBranch("origin/feat/x".into())), "<b>origin/feat/x</b> <dim>(checked on origin just now)"),
            (Tier::NotSafe, "<b>Not safe</b></red>: 4 commits exist nowhere else"),
            (Tier::Unknown("boom".into()), "could not be checked (boom)"),
        ];
        for (tier, needle) in cases {
            let markup = tier_markup(&safety(tier.clone()));
            assert!(markup.contains(needle), "{tier:?}: {markup}");
        }
        let mut one = safety(Tier::NotSafe);
        one.lost_commits = Ok(commits(1));
        assert!(tier_markup(&one).ends_with("1 commit exists nowhere else"));
    }

    #[test]
    fn branch_facts_show_ahead_behind_origin_copy_and_pr() {
        let mut facts_input = safety(Tier::NotSafe);
        facts_input.origin_copy = Some(OriginCopy {
            reference: "origin/feat/x".into(),
            missing_local: 2,
            extra_remote: 0,
        });
        facts_input.pr = PrLookup::Found(PullRequest {
            number: 99,
            url: Some("https://example.com/pr/99".into()),
            open: true,
            source_repo: "acme/widgets".into(),
            head_sha: None,
            target_branch: Some("main".into()),
        });
        facts_input.notes.push("origin/feat/x could not be verified: offline".into());
        let facts = branch_facts(&facts_input, true);
        assert!(facts[1].starts_with("3 ahead, 12 behind <b>origin/main</b> <dim>(as of your last fetch)"));
        assert!(facts[2].contains("<b>origin/feat/x</b> lacks 2 of your commits <dim>(as of your last fetch)"));
        assert!(facts[3].contains("<a href=\"https://example.com/pr/99\">PR #99</a> (open) into <b>main</b>"));
        assert!(facts[4].contains("could not be verified"));

        // No origin: neither the copy nor the PR is mentioned.
        assert_eq!(branch_facts(&safety(Tier::NotSafe), false).len(), 2);
    }

    #[test]
    fn lost_commits_list_escapes_subjects_and_caps_at_ten() {
        let ten = lost_commits_markup(&commits(10));
        assert!(ten.starts_with("Deleting the branch would lose 10 commits:"));
        assert!(ten.contains(r"change \<9\>"));
        assert_eq!(
            lost_commits_markup(&commits(11)),
            "Deleting the branch would lose <red><b>11 commits</b></red>."
        );
    }

    #[test]
    fn remote_states_read_plainly() {
        assert!(remote_markup(&RemoteState::NoRemote).contains("no origin remote"));
        assert!(remote_markup(&RemoteState::Absent {
            destination: "feat/x".into(),
            endpoint: "/srv/widgets.git".into(),
        })
        .contains("no matching remote branch"));
        let unknown = RemoteState::Present {
            destination: "feat/x".into(),
            endpoint: "/srv/widgets.git".into(),
            sha: "1".repeat(40),
            remote_only: RemoteOnly::Unknown,
        };
        assert!(remote_markup(&unknown).contains("number and contents are unknown"));
        let known = RemoteState::Present {
            destination: "feat/x".into(),
            endpoint: "/srv/widgets.git".into(),
            sha: "1".repeat(40),
            remote_only: RemoteOnly::Known(commits(2)),
        };
        assert!(remote_markup(&known).contains("with 2 commits that exist only there:"));
        let down = RemoteState::Unavailable {
            destination: "feat/x".into(),
            endpoint: "/srv/widgets.git".into(),
            reason: "timeout".into(),
        };
        assert!(remote_markup(&down).contains("could not be reached"));
        assert!(remote_markup(&down).starts_with("<b>Origin</b> <dim>(/srv/widgets.git)</dim>:"));
        let several = RemoteState::MultiplePushUrls {
            destination: "feat/x".into(),
            endpoints: vec!["/a.git".into(), "/b.git".into()],
        };
        assert!(remote_markup(&several).contains("pushes to 2 repositories"));
    }

    #[test]
    fn the_rendered_report_has_no_trailing_blank_line() {
        let terminal = Terminal::default();
        let inventory = dirty(2);
        let facts = safety(Tier::NotSafe);
        let input = ReportInput {
            display_name: "feat-x",
            path: Path::new("/tmp/feat-x"),
            branch: Some("feat/x"),
            inventory: &inventory,
            safety: Some(&facts),
            has_origin: false,
            remote: None,
        };
        let text = render(&terminal, &input);
        assert!(!text.ends_with('\n'), "{text:?}");
        assert!(text.contains("Uncommitted files"));
        assert!(text.contains("Not safe"));
    }
}
