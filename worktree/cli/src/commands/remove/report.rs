//! What `wt remove` prints before asking or removing anything: the
//! worktree, every warning [`facts::gather`](worktree::remove::facts::gather)
//! found (the lock first), and the branch's facts. Asked interactively, it is
//! the one checkout question. Builders return Prose markup so tests can read
//! them without a terminal; [`render`] turns the report into text.
//!
//! Process names, lock reasons, paths, and Git messages come from other
//! programs and the filesystem, so each is shown through [`visible`].

use std::path::Path;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::remove::CheckoutState;
use worktree::remove::facts::{Facts, LATE_CHANGES_NOTE, Warning, describe_process};
use worktree::remove::remote::{Reinterpretation, RemoteOnly, RemoteState};
use worktree::remove::safety::{BranchSafety, Commit, Evidence, PrLookup, Tier};

/// Above this many entries a list becomes a count.
pub const LIST_LIMIT: usize = 10;

/// Escape text for Prose markup, leaving the code spans it marks with
/// backticks literal.
fn esc(text: &str) -> String {
    Prose::escape_text_outside_code_spans(text)
}

/// `text` safe to show: control characters escaped (`\n`, `\u{1b}`), then
/// Prose markup escaped, so it renders as the literal text it is.
pub fn visible(text: &str) -> String {
    let shown: String = text
        .chars()
        .map(|c| if c.is_control() { c.escape_default().to_string() } else { c.to_string() })
        .collect();
    Prose::escape_text(&shown)
}

pub struct ReportInput<'a> {
    pub display_name: &'a str,
    pub path: &'a Path,
    /// The report is the checkout question (interactive, no `--force`).
    pub asking: bool,
    pub facts: &'a Facts,
    pub branch: Option<&'a str>,
    /// `None` for a detached worktree.
    pub safety: Option<&'a BranchSafety>,
    pub has_origin: bool,
    /// Present with `--force-remote`.
    pub remote: Option<&'a RemoteState>,
}

/// The whole report, rendered. It starts with a blank line and ends without
/// one, so each question can start after exactly one.
pub fn render(terminal: &Terminal, input: &ReportInput<'_>) -> String {
    // The markup builders lay their text out one line per `\n`.
    let lines = |markup: String| Prose::new(markup).with_line_breaks(LineBreaks::Hard).render(terminal);
    let mut heading = heading_markup(input);
    if matches!(input.facts.prepared, Some(CheckoutState::Repaired(_))) {
        heading.push_str(&format!("\n{RELINKED_MARKUP}"));
    }
    let mut blocks = vec![lines(heading)];
    let warnings = warning_markup(input.facts);
    if !warnings.is_empty() {
        let mut list = UnorderedList::empty().with_bullet("⚠ ");
        for warning in warnings {
            list.add(Prose::new(warning).with_line_breaks(LineBreaks::Hard));
        }
        blocks.push(list.render(terminal));
    }
    let notes: Vec<String> = input.facts.notes.iter().map(|note| format!("<dim>{}</dim>", visible(&note.to_string()))).collect();
    if !notes.is_empty() {
        blocks.push(render_list(terminal, notes));
    }
    if input.asking || !input.facts.warnings.is_empty() {
        blocks.push(lines(format!("<dim>{}</dim>", esc(LATE_CHANGES_NOTE))));
    }
    match (input.branch, input.safety) {
        (Some(branch), Some(safety)) => {
            let mut block = Prose::new(format!("<b>Branch</b> <blue>{}</blue>", esc(branch)))
                .render(terminal);
            block.push('\n');
            block.push_str(&render_list(terminal, branch_facts(safety, input.has_origin)));
            blocks.push(block);
        }
        (Some(_), None) => {}
        (None, _) => blocks.push(
            Prose::new("<dim>Detached HEAD: there is no branch to delete.</dim>").render(terminal),
        ),
    }
    if let Some(remote) = input.remote {
        blocks.push(lines(remote_markup(remote)));
    }
    let report = blocks
        .iter()
        .map(|block| block.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("\n{report}")
}

fn render_list(terminal: &Terminal, items: Vec<String>) -> String {
    let mut list = UnorderedList::empty();
    for item in items {
        list.add(Prose::new(item));
    }
    list.render(terminal)
}

pub fn heading_markup(input: &ReportInput<'_>) -> String {
    let name = esc(input.display_name);
    let path = visible(&input.path.display().to_string());
    if input.asking {
        format!("<b>Remove</b> worktree <blue>{name}</blue> <dim>({path})</dim>?")
    } else {
        format!("<b>Removing</b> worktree <blue>{name}</blue> <dim>at {path}</dim>")
    }
}

/// One markup item per warning, in [`Facts::warnings`] order (the lock
/// first). A warning that lists programs or mounts gives each its own line,
/// built from the parts so that no name can start a line of its own.
pub fn warning_markup(facts: &Facts) -> Vec<String> {
    facts
        .warnings
        .iter()
        .map(|warning| match warning {
            Warning::Locked { .. } => format!("<yellow><b>{}</b></yellow>", visible(&warning.to_string())),
            Warning::Processes { root, programs } => {
                let verb = if programs.len() == 1 { "program is" } else { "programs are" };
                let mut out = format!("<b>{} {verb} using files in this worktree:</b>", programs.len());
                for program in programs {
                    out.push_str(&format!("\n    {}", visible(&describe_process(program, root))));
                }
                out.push_str("\nDeleting now may make them fail.");
                out
            }
            Warning::NestedMounts { mounts } => {
                let what = if mounts.len() == 1 { "filesystem is" } else { "filesystems are" };
                let mut out = format!("<b>{} {what} mounted inside this worktree:</b>", mounts.len());
                for mount in mounts {
                    out.push_str(&format!(
                        "\n    {} (from {})",
                        visible(&mount.mount_point.display().to_string()),
                        visible(&mount.source)
                    ));
                }
                out.push_str("\nNothing on them is deleted, and the removal stays incomplete until they are unmounted.");
                out
            }
            other => visible(&with_one_line_reason(other).to_string()),
        })
        .collect()
}

/// `warning` with its Git or operating-system reason on one line; names and
/// paths are left as they are, for [`visible`] to escape.
fn with_one_line_reason(warning: &Warning) -> Warning {
    let line = |reason: &String| super::outcome::one_line(reason);
    match warning {
        Warning::RecordUnreadable { reason } => Warning::RecordUnreadable { reason: line(reason) },
        Warning::GitUnreadable { reason } => Warning::GitUnreadable { reason: line(reason) },
        Warning::Uninspectable { path, reason } => Warning::Uninspectable { path: path.clone(), reason: line(reason) },
        Warning::UsageIncomplete { reason } => Warning::UsageIncomplete { reason: line(reason) },
        Warning::Unchecked { reason } => Warning::Unchecked { reason: line(reason) },
        Warning::IncludedUnchecked { reason } => Warning::IncludedUnchecked { reason: line(reason) },
        Warning::StagedUnknown { reason } => Warning::StagedUnknown { reason: line(reason) },
        Warning::SubmoduleUnchecked { path, reason } => {
            Warning::SubmoduleUnchecked { path: path.clone(), reason: line(reason) }
        }
        other => other.clone(),
    }
}

/// The line under the heading for a worktree whose `.git` file was missing.
/// `git worktree repair` also restores other worktrees' broken links, so the
/// line does not claim to have touched only this one.
pub const RELINKED_MARKUP: &str = "<yellow>Its .git file was missing</yellow><dim>; wt restored it to check the \
    files (Git may restore other worktrees' broken links too).</dim>";

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
        RemoteState::ReinterpretedEndpoint {
            endpoint,
            by: Reinterpretation::Rewrite(rule),
            ..
        } => format!(
            "{} <yellow>git would rewrite this URL again</yellow> by <i>{}</i>, so <i>--force-remote</i> cannot tell which repository it would delete from.",
            origin_label(endpoint),
            esc(rule)
        ),
        RemoteState::ReinterpretedEndpoint {
            endpoint,
            by: Reinterpretation::RemoteName { source },
            ..
        } => format!(
            "{} <yellow>git would read this URL as the remote <b>{}</b></yellow> (defined by <i>{}</i>), so <i>--force-remote</i> cannot tell which repository it would delete from.",
            origin_label(endpoint),
            esc(endpoint),
            esc(source)
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
    use std::path::PathBuf;

    use worktree::remove::facts::{Note, Target};

    use super::*;

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
        let rewritten = RemoteState::ReinterpretedEndpoint {
            destination: "feat/x".into(),
            endpoint: "/a.git".into(),
            by: Reinterpretation::Rewrite("url./b.git.pushinsteadof=/a.git".into()),
        };
        assert!(remote_markup(&rewritten).contains("would rewrite this URL again"));
        assert!(remote_markup(&rewritten).contains("url./b.git.pushinsteadof=/a.git"));
        let named = RemoteState::ReinterpretedEndpoint {
            destination: "feat/x".into(),
            endpoint: "approved".into(),
            by: Reinterpretation::RemoteName {
                source: "remote.approved.pushurl".into(),
            },
        };
        assert!(remote_markup(&named).contains("as the remote <b>approved</b>"));
        assert!(remote_markup(&named).contains("remote.approved.pushurl"));
    }

    /// `text` without its styling (CSI and OSC sequences).
    fn plain(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\u{1b}' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn facts(warnings: Vec<Warning>) -> Facts {
        Facts {
            target: Target::Checkout { path: PathBuf::from("/tmp/feat-x") },
            admin: None,
            identities: Default::default(),
            prepared: None,
            warnings,
            notes: Vec::new(),
            blocker: None,
            inventory: None,
            usage: None,
        }
    }

    fn input<'a>(facts: &'a Facts, safety: &'a BranchSafety, asking: bool) -> ReportInput<'a> {
        ReportInput {
            display_name: "feat-x",
            path: Path::new("/tmp/feat-x"),
            asking,
            facts,
            branch: Some("feat/x"),
            safety: Some(safety),
            has_origin: false,
            remote: None,
        }
    }

    #[test]
    fn the_rendered_question_lists_warnings_lock_first_and_the_late_change_note() {
        let terminal = Terminal::default();
        let facts = facts(vec![
            Warning::Locked { reason: Some("on a USB disk".into()) },
            Warning::Uncommitted { files: vec!["src/config.rs".into(), "notes.md".into()] },
        ]);
        let safety = safety(Tier::NotSafe);
        let text = plain(&render(&terminal, &input(&facts, &safety, true)));
        assert!(!text.ends_with('\n'), "{text:?}");
        assert!(text.contains("worktree feat-x (/tmp/feat-x)?"), "{text}");
        let lock = text.find("with the reason: on a USB disk").expect("lock shown");
        let files = text.find("2 uncommitted files will be lost: src/config.rs, notes.md").expect("files shown");
        assert!(lock < files, "the lock comes first: {text}");
        assert_eq!(text.matches("⚠ ").count(), 2, "{text}");
        assert!(text.contains(LATE_CHANGES_NOTE), "{text}");
        assert!(text.contains("Not safe"), "{text}");
    }

    #[test]
    fn a_clean_report_without_a_question_has_no_warnings_or_late_change_note() {
        let terminal = Terminal::default();
        let facts = facts(Vec::new());
        let safety = safety(Tier::Safe(Evidence::DefaultBranch("main".into())));
        let text = plain(&render(&terminal, &input(&facts, &safety, false)));
        assert!(text.contains("Removing worktree feat-x at /tmp/feat-x"), "{text}");
        assert!(!text.contains('⚠'), "{text}");
        assert!(!text.contains(LATE_CHANGES_NOTE), "{text}");
    }

    /// A lock reason, a path, and a note come from other programs: control
    /// characters and Prose markup in them render as literal text.
    #[test]
    fn untrusted_text_renders_literally() {
        let terminal = Terminal::default();
        let mut facts = facts(vec![
            Warning::Locked { reason: Some("\u{1b}[31mred</b><i>x".into()) },
            Warning::Uncommitted { files: vec!["a\nb.rs".into()] },
            Warning::GitUnreadable { reason: "fatal: bad\nfatal: bad\nhint: x".into() },
        ]);
        facts.notes.push(Note::IncludedCheck { message: "<b>bold</b>\u{7}".into() });
        let safety = safety(Tier::NotSafe);
        let text = render(&terminal, &input(&facts, &safety, true));
        assert!(!text.contains("\u{1b}[31mred"), "the reason's escape sequence is never sent: {text:?}");
        assert!(text.contains("\\u{1b}[31mred</b><i>x"), "{text}");
        assert!(text.contains("a\\nb.rs"), "a file name is escaped, never folded: {text}");
        assert!(text.contains("fatal: bad; hint: x"), "Git's reason reads as one line: {text}");
        assert!(text.contains("<b>bold</b>\\u{7}"), "{text}");
    }

    #[test]
    fn the_heading_and_branch_section_are_shown_without_a_question() {
        let terminal = Terminal::default();
        let facts = facts(Vec::new());
        let markup = heading_markup(&input(&facts, &safety(Tier::NotSafe), false));
        assert!(markup.starts_with("<b>Removing</b>"), "{markup}");
        assert!(plain(&render(&terminal, &input(&facts, &safety(Tier::NotSafe), false))).contains("Branch feat/x"));
    }
}
