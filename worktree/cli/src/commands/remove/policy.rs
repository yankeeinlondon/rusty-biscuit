//! What `wt remove` does, as a pure function of the facts, the flags, and the
//! caller's answers; no terminal, git, or filesystem access.
//!
//! Order: the worktree question, then the branch question, then the remote
//! step. The worktree must go first because git cannot delete a branch that a
//! worktree has checked out.

use worktree::WorktreeError;

/// The three `--force-*` flags, one per object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub force_worktree: bool,
    pub force_branch: bool,
    pub force_remote: bool,
}

/// The facts the decision depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// The worktree has dirty or ignored entries.
    pub needs_consent: bool,
    /// `None` for a detached worktree; otherwise whether the branch's tier is
    /// Safe or Pretty safe.
    pub branch_safe: Option<bool>,
    pub interactive: bool,
}

/// A question only an interactive caller is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Discard the listed files and remove the worktree? (default No)
    DiscardFiles,
    /// Keep the Not safe branch (default) or delete it and lose its commits?
    DeleteUnsafeBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchStep {
    /// Delete: `approved` when a flag or an answer allowed it whatever the
    /// tier, otherwise because the tier is Safe or Pretty safe.
    Delete { approved: bool },
    /// Not safe and nobody could be asked: keep it and warn (exit 0).
    KeepWithWarning,
    /// The caller chose to keep it.
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actions {
    /// Pass `--force` to `git worktree remove`.
    pub discard_files: bool,
    /// `None` for a detached worktree.
    pub branch: Option<BranchStep>,
    pub delete_remote: bool,
}

/// Why nothing was removed; `wt` exits 3 ([`crate::exit::REFUSED_TO_LOSE_WORK`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Dirty or ignored entries, no terminal, no `--force-worktree`.
    FilesNeedForce,
    /// `--force-branch` on a worktree with files, without `--force-worktree`.
    ForceBranchNeedsWorktree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Proceed(Actions),
    Refuse(Refusal),
    /// The caller declined to discard the files; nothing removed, exit 0.
    Cancelled,
}

/// Decides every step. `ask` is called only when `situation.interactive`,
/// and returns `true` for "discard" or "delete".
///
/// ## Errors
///
/// Whatever `ask` returns (for example, an interrupted prompt).
pub fn decide(
    situation: Situation,
    flags: Flags,
    ask: &mut dyn FnMut(Question) -> Result<bool, WorktreeError>,
) -> Result<Decision, WorktreeError> {
    if situation.needs_consent && !flags.force_worktree {
        if !situation.interactive {
            return Ok(Decision::Refuse(if flags.force_branch {
                Refusal::ForceBranchNeedsWorktree
            } else {
                Refusal::FilesNeedForce
            }));
        }
        if !ask(Question::DiscardFiles)? {
            return Ok(Decision::Cancelled);
        }
    }

    let branch = match situation.branch_safe {
        None => None,
        Some(_) if flags.force_branch => Some(BranchStep::Delete { approved: true }),
        Some(true) => Some(BranchStep::Delete { approved: false }),
        Some(false) if !situation.interactive => Some(BranchStep::KeepWithWarning),
        Some(false) => Some(if ask(Question::DeleteUnsafeBranch)? {
            BranchStep::Delete { approved: true }
        } else {
            BranchStep::Keep
        }),
    };

    Ok(Decision::Proceed(Actions {
        discard_files: situation.needs_consent,
        branch,
        delete_remote: flags.force_remote,
    }))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use worktree::remove::safety::{Evidence, Tier};
    use worktree::remove::{DirtyEntry, Inventory};

    use super::*;

    /// The worktree contents the matrix covers.
    fn inventories() -> Vec<(&'static str, Inventory)> {
        let dirty = |path: &str| DirtyEntry {
            status: " M".into(),
            path: PathBuf::from(path),
            is_source: path.ends_with(".rs"),
        };
        vec![
            ("clean", Inventory::default()),
            (
                "dirty",
                Inventory {
                    dirty: vec![dirty("src/lib.rs"), dirty("notes.md"), dirty("a.txt")],
                    ignored: Vec::new(),
                },
            ),
            (
                "ignored .env",
                Inventory {
                    dirty: Vec::new(),
                    ignored: vec![".env".into()],
                },
            ),
            (
                "ignored target/",
                Inventory {
                    dirty: Vec::new(),
                    ignored: vec!["target/".into()],
                },
            ),
        ]
    }

    fn tiers() -> Vec<Option<Tier>> {
        vec![
            None,
            Some(Tier::Safe(Evidence::DefaultBranch("main".into()))),
            Some(Tier::Safe(Evidence::PullRequest {
                number: 99,
                merged: true,
                url: None,
            })),
            Some(Tier::PrettySafe(Evidence::Tag("v1".into()))),
            Some(Tier::PrettySafe(Evidence::RemoteBranch("origin/feat/x".into()))),
            Some(Tier::NotSafe),
            Some(Tier::Unknown("git failed".into())),
        ]
    }

    fn flag_subsets() -> Vec<Flags> {
        (0..8)
            .map(|bits| Flags {
                force_worktree: bits & 1 != 0,
                force_branch: bits & 2 != 0,
                force_remote: bits & 4 != 0,
            })
            .collect()
    }

    /// `None`: non-interactive. `Some((discard, delete))`: the scripted answers.
    fn modes() -> Vec<Option<(bool, bool)>> {
        vec![
            None,
            Some((false, false)),
            Some((true, false)),
            Some((true, true)),
        ]
    }

    /// The full cross-product: tier × worktree contents × flags × mode. Each
    /// case is checked against the rules of item 3 stated independently of
    /// `decide`'s structure.
    #[test]
    fn policy_matrix_follows_the_rules() {
        let mut cases = 0;
        for tier in tiers() {
            for (label, inventory) in inventories() {
                for flags in flag_subsets() {
                    for mode in modes() {
                        cases += 1;
                        check_case(tier.as_ref(), label, &inventory, flags, mode);
                    }
                }
            }
        }
        assert_eq!(cases, 7 * 4 * 8 * 4);
    }

    fn check_case(
        tier: Option<&Tier>,
        label: &str,
        inventory: &Inventory,
        flags: Flags,
        mode: Option<(bool, bool)>,
    ) {
        let situation = Situation {
            needs_consent: inventory.needs_consent(),
            branch_safe: tier.map(Tier::allows_deletion),
            interactive: mode.is_some(),
        };
        let mut asked = Vec::new();
        let decision = decide(situation, flags, &mut |question| {
            asked.push(question);
            let (discard, delete) = mode.expect("a non-interactive run never asks");
            Ok(match question {
                Question::DiscardFiles => discard,
                Question::DeleteUnsafeBranch => delete,
            })
        })
        .unwrap();
        let context = format!("tier={tier:?} contents={label} flags={flags:?} mode={mode:?} -> {decision:?}");

        let safe = tier.is_some_and(Tier::allows_deletion);
        let consent = inventory.needs_consent();

        // The files question is asked exactly when files would be lost, no
        // flag covers them, and someone can answer.
        assert_eq!(
            asked.contains(&Question::DiscardFiles),
            consent && !flags.force_worktree && mode.is_some(),
            "{context}"
        );

        match decision {
            Decision::Refuse(refusal) => {
                assert!(consent && !flags.force_worktree && mode.is_none(), "{context}");
                let expected = if flags.force_branch {
                    Refusal::ForceBranchNeedsWorktree
                } else {
                    Refusal::FilesNeedForce
                };
                assert_eq!(refusal, expected, "{context}");
            }
            Decision::Cancelled => {
                assert_eq!(mode.map(|m| m.0), Some(false), "{context}");
                assert!(consent && !flags.force_worktree, "{context}");
                assert!(!asked.contains(&Question::DeleteUnsafeBranch), "{context}");
            }
            Decision::Proceed(actions) => {
                assert_eq!(actions.discard_files, consent, "{context}");
                assert_eq!(actions.delete_remote, flags.force_remote, "{context}");
                let expected_branch = match tier {
                    None => None,
                    Some(_) if flags.force_branch => Some(BranchStep::Delete { approved: true }),
                    Some(_) if safe => Some(BranchStep::Delete { approved: false }),
                    Some(_) => Some(match mode {
                        None => BranchStep::KeepWithWarning,
                        Some((_, true)) => BranchStep::Delete { approved: true },
                        Some((_, false)) => BranchStep::Keep,
                    }),
                };
                assert_eq!(actions.branch, expected_branch, "{context}");
                // The branch question only for an unsafe branch without a flag.
                assert_eq!(
                    asked.contains(&Question::DeleteUnsafeBranch),
                    tier.is_some() && !safe && !flags.force_branch && mode.is_some(),
                    "{context}"
                );
                // An Unknown tier is never treated as safe.
                if matches!(tier, Some(Tier::Unknown(_))) && !flags.force_branch {
                    assert_ne!(actions.branch, Some(BranchStep::Delete { approved: false }), "{context}");
                }
            }
        }
    }

    fn run(situation: Situation, flags: Flags) -> Decision {
        decide(situation, flags, &mut |_| panic!("not interactive")).unwrap()
    }

    const CLEAN_SAFE: Situation = Situation {
        needs_consent: false,
        branch_safe: Some(true),
        interactive: false,
    };

    /// The rows of the spec's Examples table.
    #[test]
    fn spec_examples() {
        // `wt remove old-cleanup`: clean, merged into main.
        assert_eq!(
            run(CLEAN_SAFE, Flags::default()),
            Decision::Proceed(Actions {
                discard_files: false,
                branch: Some(BranchStep::Delete { approved: false }),
                delete_remote: false,
            })
        );
        // `wt remove spike-parser` (no terminal): clean, commits nowhere else.
        let spike = Situation {
            branch_safe: Some(false),
            ..CLEAN_SAFE
        };
        assert_eq!(
            run(spike, Flags::default()),
            Decision::Proceed(Actions {
                discard_files: false,
                branch: Some(BranchStep::KeepWithWarning),
                delete_remote: false,
            })
        );
        // `... --force-branch`.
        let force_branch = Flags {
            force_branch: true,
            ..Flags::default()
        };
        assert_eq!(
            run(spike, force_branch),
            Decision::Proceed(Actions {
                discard_files: false,
                branch: Some(BranchStep::Delete { approved: true }),
                delete_remote: false,
            })
        );
        // `wt remove fix-wt-ux` (no terminal): 3 dirty files.
        let dirty = Situation {
            needs_consent: true,
            ..CLEAN_SAFE
        };
        assert_eq!(run(dirty, Flags::default()), Decision::Refuse(Refusal::FilesNeedForce));
        assert_eq!(
            run(dirty, force_branch),
            Decision::Refuse(Refusal::ForceBranchNeedsWorktree)
        );
        // `... --force-worktree --force-branch --force-remote`.
        let all = Flags {
            force_worktree: true,
            force_branch: true,
            force_remote: true,
        };
        assert_eq!(
            run(dirty, all),
            Decision::Proceed(Actions {
                discard_files: true,
                branch: Some(BranchStep::Delete { approved: true }),
                delete_remote: true,
            })
        );
    }

    #[test]
    fn an_interrupted_prompt_propagates() {
        let situation = Situation {
            needs_consent: true,
            branch_safe: Some(true),
            interactive: true,
        };
        let result = decide(situation, Flags::default(), &mut |_| Err(WorktreeError::Cancelled));
        assert!(matches!(result, Err(WorktreeError::Cancelled)));
    }
}
