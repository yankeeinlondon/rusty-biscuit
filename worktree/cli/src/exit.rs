//! Process exit codes shared by every `wt` command.
//!
//! 0, 1, and 2 mean the same everywhere (2 is clap's invalid-arguments code).
//! 3 and 4 are refusals that changed nothing, split because the remedy
//! differs: 3 needs a `--force-*` flag, 4 needs a different environment.

use worktree::WorktreeError;

pub const SUCCESS: i32 = 0;
pub const FAILURE: i32 = 1;
pub const REFUSED_TO_LOSE_WORK: i32 = 3;
pub const BLOCKED_BY_ENVIRONMENT: i32 = 4;

/// The exit code for a command that ended with `error`.
///
/// A prompt the caller cancelled is a choice, not a failure, so it exits 0.
pub fn exit_code(error: &WorktreeError) -> i32 {
    match error {
        WorktreeError::Cancelled => SUCCESS,
        WorktreeError::RefusedToLoseWork(_) => REFUSED_TO_LOSE_WORK,
        WorktreeError::BlockedByEnvironment(_) => BLOCKED_BY_ENVIRONMENT,
        _ => FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_and_cancel_map_to_their_codes() {
        assert_eq!(exit_code(&WorktreeError::Cancelled), 0);
        assert_eq!(
            exit_code(&WorktreeError::RefusedToLoseWork("dirty".into())),
            3
        );
        assert_eq!(
            exit_code(&WorktreeError::BlockedByEnvironment("no wrapper".into())),
            4
        );
    }

    #[test]
    fn every_other_error_is_a_failure() {
        let failures = [
            WorktreeError::NotInGitRepo,
            WorktreeError::WorktreeNotFound("x".into()),
            WorktreeError::GitCommand("boom".into()),
            WorktreeError::AmbiguousWorktree {
                name: "x".into(),
                candidates: Vec::new(),
            },
            WorktreeError::FromBranchNotFound("nope".into()),
        ];
        for error in failures {
            assert_eq!(exit_code(&error), 1, "{error:?}");
        }
    }
}
