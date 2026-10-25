//! What `wt remove` does, as a pure function of the facts, the flags, and the
//! caller's answers; no terminal, git, or filesystem access.
//!
//! Order: the checkout question, then the branch question, then the remote
//! step. The worktree must go first because git cannot delete a branch that a
//! worktree has checked out.
//!
//! One question covers everything about the checkout (R28 of
//! `2026-10-22-confirmed-remove-always-deletes`): an interactive caller is
//! always asked it unless `--force` answered it, and a caller nobody can ask
//! is refused when there is anything to warn about.

use worktree::WorktreeError;

/// `--force` for the checkout; one flag each for the local and the remote
/// branch, which `--force` never implies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub force: bool,
    pub force_branch: bool,
    pub force_remote: bool,
}

/// The facts the decision depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// The question would show at least one warning.
    pub warned: bool,
    /// `None` for a detached worktree; otherwise whether the branch's tier is
    /// Safe or Pretty safe.
    pub branch_safe: Option<bool>,
    pub interactive: bool,
}

/// A question only an interactive caller is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Delete the checkout, with whatever the warnings listed? (default No)
    DeleteCheckout,
    /// Keep the Not safe branch (default) or delete it and lose its commits?
    DeleteUnsafeBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchStep {
    /// Delete: `approved` when a flag or an answer allowed it whatever the
    /// tier, otherwise because the tier is Safe or Pretty safe (and so it is
    /// rechecked by any later run that carries it out).
    Delete { approved: bool },
    /// Not safe and nobody could be asked: keep it and warn (exit 0).
    KeepWithWarning,
    /// The caller chose to keep it.
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actions {
    /// `None` for a detached worktree.
    pub branch: Option<BranchStep>,
    pub delete_remote: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Proceed(Actions),
    /// Warnings, no terminal to show them on, and no `--force`: nothing is
    /// removed; `wt` exits 3 ([`crate::exit::REFUSED_TO_LOSE_WORK`]).
    NeedsForce,
    /// The caller declined to delete the checkout; nothing removed, exit 0.
    Cancelled,
}

/// Decides every step. `ask` is called only when `situation.interactive`,
/// and returns `true` for "delete".
///
/// ## Errors
///
/// Whatever `ask` returns (for example, an interrupted prompt).
pub fn decide(
    situation: Situation,
    flags: Flags,
    ask: &mut dyn FnMut(Question) -> Result<bool, WorktreeError>,
) -> Result<Decision, WorktreeError> {
    if !flags.force {
        if situation.interactive {
            if !ask(Question::DeleteCheckout)? {
                return Ok(Decision::Cancelled);
            }
        } else if situation.warned {
            return Ok(Decision::NeedsForce);
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

    Ok(Decision::Proceed(Actions { branch, delete_remote: flags.force_remote }))
}

#[cfg(test)]
mod tests {
    use worktree::remove::safety::{Evidence, Tier};

    use super::*;

    fn tiers() -> Vec<Option<Tier>> {
        vec![
            None,
            Some(Tier::Safe(Evidence::DefaultBranch("main".into()))),
            Some(Tier::Safe(Evidence::PullRequest { number: 99, merged: true, url: None })),
            Some(Tier::PrettySafe(Evidence::Tag("v1".into()))),
            Some(Tier::PrettySafe(Evidence::RemoteBranch("origin/feat/x".into()))),
            Some(Tier::NotSafe),
            Some(Tier::Unknown("git failed".into())),
        ]
    }

    fn flag_subsets() -> Vec<Flags> {
        (0..8)
            .map(|bits| Flags { force: bits & 1 != 0, force_branch: bits & 2 != 0, force_remote: bits & 4 != 0 })
            .collect()
    }

    /// `None`: non-interactive. `Some((delete_checkout, delete_branch))`:
    /// the scripted answers.
    fn modes() -> Vec<Option<(bool, bool)>> {
        vec![None, Some((false, false)), Some((true, false)), Some((true, true))]
    }

    /// The full cross-product: tier × warned × flags × mode. Each case is
    /// checked against the rules stated independently of `decide`'s
    /// structure.
    #[test]
    fn policy_matrix_follows_the_rules() {
        let mut cases = 0;
        for tier in tiers() {
            for warned in [false, true] {
                for flags in flag_subsets() {
                    for mode in modes() {
                        cases += 1;
                        check_case(tier.as_ref(), warned, flags, mode);
                    }
                }
            }
        }
        assert_eq!(cases, 7 * 2 * 8 * 4);
    }

    fn check_case(tier: Option<&Tier>, warned: bool, flags: Flags, mode: Option<(bool, bool)>) {
        let situation = Situation { warned, branch_safe: tier.map(Tier::allows_deletion), interactive: mode.is_some() };
        let mut asked = Vec::new();
        let decision = decide(situation, flags, &mut |question| {
            asked.push(question);
            let (checkout, branch) = mode.expect("a non-interactive run never asks");
            Ok(match question {
                Question::DeleteCheckout => checkout,
                Question::DeleteUnsafeBranch => branch,
            })
        })
        .unwrap();
        let context = format!("tier={tier:?} warned={warned} flags={flags:?} mode={mode:?} -> {decision:?}");
        let safe = tier.is_some_and(Tier::allows_deletion);

        // One checkout question for every interactive run without --force,
        // whether or not anything is warned about.
        assert_eq!(asked.contains(&Question::DeleteCheckout), mode.is_some() && !flags.force, "{context}");
        assert!(asked.iter().filter(|question| **question == Question::DeleteCheckout).count() <= 1, "{context}");

        match decision {
            Decision::NeedsForce => {
                assert!(warned && !flags.force && mode.is_none(), "{context}");
            }
            Decision::Cancelled => {
                assert_eq!(mode.map(|m| m.0), Some(false), "{context}");
                assert!(!flags.force, "{context}");
                assert!(!asked.contains(&Question::DeleteUnsafeBranch), "{context}");
            }
            Decision::Proceed(actions) => {
                // --force-branch and --force-remote are never implied.
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
                assert_eq!(
                    asked.contains(&Question::DeleteUnsafeBranch),
                    tier.is_some() && !safe && !flags.force_branch && mode.is_some(),
                    "{context}"
                );
                if matches!(tier, Some(Tier::Unknown(_))) && !flags.force_branch {
                    assert_ne!(actions.branch, Some(BranchStep::Delete { approved: false }), "{context}");
                }
            }
        }
    }

    fn run(situation: Situation, flags: Flags) -> Decision {
        decide(situation, flags, &mut |_| panic!("not interactive")).unwrap()
    }

    const CLEAN_SAFE: Situation = Situation { warned: false, branch_safe: Some(true), interactive: false };

    #[test]
    fn a_clean_checkout_without_a_terminal_is_removed_without_force() {
        assert_eq!(
            run(CLEAN_SAFE, Flags::default()),
            Decision::Proceed(Actions { branch: Some(BranchStep::Delete { approved: false }), delete_remote: false })
        );
    }

    #[test]
    fn warnings_without_a_terminal_need_force_and_force_is_yes() {
        let warned = Situation { warned: true, ..CLEAN_SAFE };
        assert_eq!(run(warned, Flags::default()), Decision::NeedsForce);
        // --force-branch says nothing about the checkout.
        assert_eq!(run(warned, Flags { force_branch: true, ..Flags::default() }), Decision::NeedsForce);
        assert_eq!(
            run(warned, Flags { force: true, ..Flags::default() }),
            Decision::Proceed(Actions { branch: Some(BranchStep::Delete { approved: false }), delete_remote: false })
        );
    }

    #[test]
    fn force_alone_keeps_an_unsafe_branch() {
        let unsafe_branch = Situation { warned: true, branch_safe: Some(false), interactive: false };
        assert_eq!(
            run(unsafe_branch, Flags { force: true, ..Flags::default() }),
            Decision::Proceed(Actions { branch: Some(BranchStep::KeepWithWarning), delete_remote: false })
        );
    }

    #[test]
    fn an_interrupted_prompt_propagates() {
        let situation = Situation { warned: true, branch_safe: Some(true), interactive: true };
        let result = decide(situation, Flags::default(), &mut |_| Err(WorktreeError::Cancelled));
        assert!(matches!(result, Err(WorktreeError::Cancelled)));
    }
}
