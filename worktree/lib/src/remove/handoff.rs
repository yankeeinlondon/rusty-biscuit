//! The short-lived record that carries a move-first removal across the shell
//! wrapper's `cd`.
//!
//! The first `wt remove` run asks every question, then stores what it saw and
//! what the caller approved under a random token and prints
//! `remove-handoff:<token>`. After a successful `cd`, the wrapper runs
//! `wt remove --handoff <token>`, which consumes the record (deleting it before
//! anything else, so it cannot be replayed), re-reads the same state, and
//! removes only if nothing changed and the caller has left the worktree.
//!
//! Records sit beside the comparison cache as
//! `<repo hash>.handoff-<token>.json` and expire after [`HANDOFF_TTL`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file};
use crate::error::WorktreeError;

pub const HANDOFF_FORMAT_VERSION: u32 = 2;

/// How long a record stays valid.
pub const HANDOFF_TTL: Duration = Duration::from_secs(60);

/// Everything the second run re-reads and compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffState {
    /// The main worktree, canonical.
    pub repo: PathBuf,
    /// The worktree being removed, canonical.
    pub target: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    /// [`crate::remove::Inventory::fingerprint`] of the worktree.
    pub fingerprint: String,
    /// Where the wrapper moves the caller, canonical.
    pub landing: PathBuf,
}

/// What the first run decided to do to the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchAction {
    /// Approved by `--force-branch` or an answer: delete whatever the tier.
    Delete,
    /// Safe or Pretty safe: delete only if still so.
    DeleteIfSafe,
    Keep,
}

/// The remote deletion `--force-remote` approved, and what the report showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteApproval {
    /// The branch name on origin; `None` when there is no `origin` remote.
    /// The second run refuses if its freshly computed destination differs,
    /// since a different branch can share `observed_sha`.
    pub destination: Option<String>,
    /// Origin's resolved push URL, which the report observed and the deletion
    /// pushes to. The second run refuses if it differs, since another
    /// repository can hold the same branch at the same commit.
    pub endpoint: Option<String>,
    /// The live head the report showed; the lease is taken against it.
    pub observed_sha: Option<String>,
}

/// The caller's approvals from the first run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approvals {
    /// Discard the dirty and ignored entries (`--force-worktree` or an answer).
    pub discard_files: bool,
    /// `None` for a detached worktree.
    pub branch: Option<BranchAction>,
    pub remote: Option<RemoteApproval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffRecord {
    pub format_version: u32,
    pub state: HandoffState,
    pub approvals: Approvals,
    /// Seconds since the Unix epoch.
    pub created_at: u64,
}

/// Why a token did not yield a record. Each leaves the worktree untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffError {
    /// No record for the token (never written, already used, or malformed).
    Missing,
    Expired,
}

/// Why the second run refuses a valid record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffRefusal {
    /// Plain-words names of each field that changed between the runs.
    Changed(Vec<&'static str>),
    /// The caller's current directory is still inside the worktree.
    InsideTarget,
}

impl HandoffRecord {
    pub fn new(state: HandoffState, approvals: Approvals, created_at: u64) -> Self {
        Self {
            format_version: HANDOFF_FORMAT_VERSION,
            state,
            approvals,
            created_at,
        }
    }
}

/// 128 bits from the OS random source, as 32 lowercase hex characters.
pub fn new_token() -> Result<String, WorktreeError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| WorktreeError::Io(std::io::Error::other(format!("no OS randomness: {e}"))))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn is_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The record file for `token`. A malformed token (which could otherwise
/// reach outside the cache directory) is [`HandoffError::Missing`].
pub fn handoff_path(repo_root: &Path, token: &str) -> Result<PathBuf, HandoffError> {
    if !is_token(token) {
        return Err(HandoffError::Missing);
    }
    repo_cache_file(repo_root, &format!("handoff-{token}.json")).map_err(|_| HandoffError::Missing)
}

pub fn write_record(path: &Path, record: &HandoffRecord) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, &serde_json::to_vec_pretty(record)?)
}

/// Reads and deletes the record at `path`; `now` is seconds since the Unix
/// epoch.
///
/// The file is deleted before the record is judged, so a token works at most
/// once whatever the outcome.
pub fn consume(path: &Path, now: u64) -> Result<HandoffRecord, HandoffError> {
    let bytes = fs::read(path).map_err(|_| HandoffError::Missing)?;
    fs::remove_file(path).map_err(|_| HandoffError::Missing)?;
    let record: HandoffRecord = serde_json::from_slice(&bytes).map_err(|_| HandoffError::Missing)?;
    if record.format_version != HANDOFF_FORMAT_VERSION {
        return Err(HandoffError::Missing);
    }
    // A record from the future is as suspect as an old one.
    let age = now.checked_sub(record.created_at).ok_or(HandoffError::Expired)?;
    if age > HANDOFF_TTL.as_secs() {
        return Err(HandoffError::Expired);
    }
    Ok(record)
}

/// Requires `cwd` to be outside the target, then compares the stored state
/// with `fresh`. Paths are compared canonical, so spellings that name the same
/// directory (a symlinked temp dir, Windows `\\?\` prefixes) agree.
pub fn verify(record: &HandoffRecord, fresh: &HandoffState, cwd: &Path) -> Result<(), HandoffRefusal> {
    let stored = &record.state;
    // Checked first: a caller who never left also fails the landing check,
    // but the remedy is to move, not to start again.
    if is_within(cwd, &stored.target) {
        return Err(HandoffRefusal::InsideTarget);
    }
    let mut changed = Vec::new();
    if !same_path(&stored.repo, &fresh.repo) {
        changed.push("repository");
    }
    if !same_path(&stored.target, &fresh.target) {
        changed.push("worktree path");
    }
    if stored.head != fresh.head {
        changed.push("checked-out commit");
    }
    if stored.branch != fresh.branch {
        changed.push("branch");
    }
    if stored.fingerprint != fresh.fingerprint {
        changed.push("uncommitted or ignored files");
    }
    if !same_path(&stored.landing, &fresh.landing) {
        changed.push("landing directory");
    }
    if changed.is_empty() {
        Ok(())
    } else {
        Err(HandoffRefusal::Changed(changed))
    }
}

/// `path` canonical, without Windows' verbatim prefix; unchanged when it
/// cannot be resolved (for example, it no longer exists).
pub fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

fn same_path(a: &Path, b: &Path) -> bool {
    canonical(a) == canonical(b)
}

/// Whether `path` is `dir` or inside it, compared canonical and by whole
/// components (so `/wt/feat-x2` is not inside `/wt/feat-x`).
pub fn is_within(path: &Path, dir: &Path) -> bool {
    canonical(path).starts_with(canonical(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(dir: &Path) -> HandoffState {
        let target = dir.join("wt");
        let landing = dir.join("base");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&landing).unwrap();
        HandoffState {
            repo: landing.clone(),
            target,
            head: "1111111111111111111111111111111111111111".into(),
            branch: Some("feat/x".into()),
            fingerprint: "f".repeat(64),
            landing,
        }
    }

    fn approvals() -> Approvals {
        Approvals {
            discard_files: true,
            branch: Some(BranchAction::DeleteIfSafe),
            remote: Some(RemoteApproval {
                destination: Some("feat/x".into()),
                endpoint: Some("/srv/git/widgets.git".into()),
                observed_sha: Some("2".repeat(40)),
            }),
        }
    }

    #[test]
    fn tokens_are_32_random_hex_characters() {
        let first = new_token().unwrap();
        let second = new_token().unwrap();
        assert!(is_token(&first) && is_token(&second));
        assert_ne!(first, second);
    }

    #[test]
    fn a_malformed_token_never_names_a_file() {
        let dir = tempfile::tempdir().unwrap();
        for token in ["", "../../etc/passwd", &"A".repeat(32), &"a".repeat(31), &"a".repeat(33)] {
            assert_eq!(handoff_path(dir.path(), token), Err(HandoffError::Missing), "{token:?}");
        }
        let path = handoff_path(dir.path(), &"a".repeat(32)).unwrap();
        let cache = crate::cache::cache_path(dir.path()).unwrap();
        assert_eq!(path.parent(), cache.parent());
    }

    #[test]
    fn round_trip_then_replay_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache").join("h.json");
        let record = HandoffRecord::new(state(dir.path()), approvals(), 1_000);

        write_record(&path, &record).unwrap();
        // A second write/read keeps every field.
        let first = consume(&path, 1_010).unwrap();
        write_record(&path, &first).unwrap();
        let second = consume(&path, 1_020).unwrap();
        assert_eq!(first, record);
        assert_eq!(second, record);

        // Consumed: the same token cannot be used again.
        assert_eq!(consume(&path, 1_020), Err(HandoffError::Missing));
        assert!(!path.exists());
    }

    #[test]
    fn an_expired_or_future_record_is_refused_and_still_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        let record = HandoffRecord::new(state(dir.path()), approvals(), 1_000);

        write_record(&path, &record).unwrap();
        assert!(consume(&path, 1_060).is_ok(), "exactly the TTL is still valid");

        write_record(&path, &record).unwrap();
        assert_eq!(consume(&path, 1_061), Err(HandoffError::Expired));
        assert!(!path.exists());

        write_record(&path, &record).unwrap();
        assert_eq!(consume(&path, 999), Err(HandoffError::Expired));
    }

    #[test]
    fn a_corrupt_or_other_version_record_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        fs::write(&path, b"{not json").unwrap();
        assert_eq!(consume(&path, 0), Err(HandoffError::Missing));

        let mut record = HandoffRecord::new(state(dir.path()), approvals(), 0);
        record.format_version += 1;
        write_record(&path, &record).unwrap();
        assert_eq!(consume(&path, 0), Err(HandoffError::Missing));
    }

    #[test]
    fn every_changed_field_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let stored = state(dir.path());
        let record = HandoffRecord::new(stored.clone(), approvals(), 0);
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let outside = stored.landing.clone();

        assert_eq!(verify(&record, &stored, &outside), Ok(()));

        type Change<'a> = Box<dyn Fn(&mut HandoffState) + 'a>;
        let cases: Vec<(&str, Change)> = vec![
            ("repository", Box::new(|s| s.repo = elsewhere.clone())),
            ("worktree path", Box::new(|s| s.target = elsewhere.clone())),
            ("checked-out commit", Box::new(|s| s.head = "3".repeat(40))),
            ("branch", Box::new(|s| s.branch = Some("feat/y".into()))),
            ("branch", Box::new(|s| s.branch = None)),
            ("uncommitted or ignored files", Box::new(|s| s.fingerprint = "0".repeat(64))),
            ("landing directory", Box::new(|s| s.landing = elsewhere.clone())),
        ];
        for (field, change) in cases {
            let mut fresh = stored.clone();
            change(&mut fresh);
            assert_eq!(
                verify(&record, &fresh, &outside),
                Err(HandoffRefusal::Changed(vec![field])),
                "{field}"
            );
        }
    }

    #[test]
    fn a_caller_still_inside_the_target_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let stored = state(dir.path());
        let record = HandoffRecord::new(stored.clone(), approvals(), 0);
        let inside = stored.target.join("src");
        fs::create_dir_all(&inside).unwrap();

        assert_eq!(verify(&record, &stored, &stored.target), Err(HandoffRefusal::InsideTarget));
        assert_eq!(verify(&record, &stored, &inside), Err(HandoffRefusal::InsideTarget));

        // A sibling whose name extends the target's is outside it.
        let sibling = dir.path().join("wt2");
        fs::create_dir_all(&sibling).unwrap();
        assert_eq!(verify(&record, &stored, &sibling), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn paths_compare_canonical() {
        let dir = tempfile::tempdir().unwrap();
        let stored = state(dir.path());
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&stored.target, &link).unwrap();
        assert!(is_within(&link.join("."), &stored.target));
        let mut fresh = stored.clone();
        fresh.target = link;
        let record = HandoffRecord::new(stored.clone(), approvals(), 0);
        assert_eq!(verify(&record, &fresh, &stored.landing), Ok(()));
    }
}
