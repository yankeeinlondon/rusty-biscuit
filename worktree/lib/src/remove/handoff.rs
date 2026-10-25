//! The short-lived record that carries a move-first removal across the shell
//! wrapper's `cd`.
//!
//! The first `wt remove` run asks every question and saves the approval as
//! an operation's compact record, then stores that operation's id under a
//! random token and prints `remove-handoff:<token>`. After a successful
//! `cd`, the wrapper runs `wt remove --handoff <token>`, which consumes the
//! handoff (deleting it before anything else, so it cannot be replayed),
//! checks that the caller has left the worktree, and finishes the operation.
//! No content of the checkout is compared between the runs: the record also
//! carries the dirty paths the question showed, only so the second run's
//! final line can name uncommitted files the question did not (R14).
//!
//! Records sit beside the comparison cache as
//! `<repo hash>.handoff-<token>.json` and expire after
//! [`OPERATION_HANDOFF_TTL`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};
use sniff::filesystem::query::NativeString;

use crate::cache::{atomic_write, repo_cache_file};
use crate::error::WorktreeError;
use super::recovery::OperationId;

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

/// 128 bits from the OS random source, as 32 lowercase hex characters.
pub fn new_token() -> Result<String, WorktreeError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| WorktreeError::Io(std::io::Error::other(format!("no OS randomness: {e}"))))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn is_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The [`OperationHandoff::format_version`] this build writes. A record of
/// any other version (including an earlier `wt`'s content-comparing format,
/// version 4, and version 5, which did not carry the question's dirty
/// paths) is [`HandoffError::Missing`] to [`consume_operation`].
pub const OPERATION_HANDOFF_VERSION: u32 = 6;

/// How long an [`OperationHandoff`] stays valid (R18).
pub const OPERATION_HANDOFF_TTL: Duration = Duration::from_secs(10 * 60);

/// The handoff of a confirmed removal: the first run saved its approval as
/// the compact record of `operation` before printing the token, and the
/// second run finishes that operation. Nothing here is compared with the
/// checkout's content; the record's own identities bind the checkout and
/// the repository, and its branch steps keep their protections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationHandoff {
    pub format_version: u32,
    pub operation: OperationId,
    /// The main worktree, canonical.
    pub repo: PathBuf,
    /// The worktree being removed, canonical; the caller must have left it.
    pub target: PathBuf,
    /// Seconds since the Unix epoch.
    pub created_at: u64,
    /// The checkout-relative dirty paths the question listed, one per file
    /// it showed; `null` when it showed no inventory (Git could not read the
    /// checkout), so the second run names no late change. Required: a
    /// record without the field, or with any element that is not a path,
    /// is [`HandoffError::Missing`].
    #[serde(deserialize_with = "required")]
    pub shown_dirty: Option<Vec<NativeString>>,
}

/// Reads an `Option` field that must be present (`null` is `None`); serde
/// would otherwise read a missing one as `None`.
fn required<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<NativeString>>, D::Error> {
    Option::deserialize(deserializer)
}

impl OperationHandoff {
    /// `shown` is the question's dirty paths, as
    /// [`late::shown`](super::lifecycle::late::shown) reads them.
    pub fn new(operation: OperationId, repo: PathBuf, target: PathBuf, created_at: u64, shown: Option<Vec<PathBuf>>) -> Self {
        let shown_dirty = shown.map(|paths| paths.into_iter().map(NativeString::from).collect());
        Self { format_version: OPERATION_HANDOFF_VERSION, operation, repo, target, created_at, shown_dirty }
    }

    /// [`Self::shown_dirty`] as paths, for the late-change comparison.
    pub fn shown(&self) -> Option<Vec<PathBuf>> {
        self.shown_dirty.as_ref().map(|paths| paths.iter().map(|path| path.as_path().to_path_buf()).collect())
    }
}

pub fn write_operation(path: &Path, handoff: &OperationHandoff) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, &serde_json::to_vec_pretty(handoff)?)
}

/// Reads and deletes the handoff at `path`; `now` is seconds since the Unix
/// epoch. The file is deleted before it is judged, so a token works at most
/// once whatever the outcome.
pub fn consume_operation(path: &Path, now: u64) -> Result<OperationHandoff, HandoffError> {
    let bytes = fs::read(path).map_err(|_| HandoffError::Missing)?;
    fs::remove_file(path).map_err(|_| HandoffError::Missing)?;
    let handoff: OperationHandoff = serde_json::from_slice(&bytes).map_err(|_| HandoffError::Missing)?;
    if handoff.format_version != OPERATION_HANDOFF_VERSION {
        return Err(HandoffError::Missing);
    }
    // A record from the future is as suspect as an old one.
    let age = now.checked_sub(handoff.created_at).ok_or(HandoffError::Expired)?;
    if age > OPERATION_HANDOFF_TTL.as_secs() {
        return Err(HandoffError::Expired);
    }
    Ok(handoff)
}

/// The second run's own checks before it finishes the operation: the
/// caller is outside the target, and `repo` is the repository the first
/// run approved in. The checkout's and the worktree record's identities are
/// checked against the operation's record when it is resumed.
pub fn verify_operation(handoff: &OperationHandoff, repo: &Path, cwd: &Path) -> Result<(), HandoffRefusal> {
    if is_within(cwd, &handoff.target) {
        return Err(HandoffRefusal::InsideTarget);
    }
    if !same_path(&handoff.repo, repo) {
        return Err(HandoffRefusal::Changed(vec!["repository"]));
    }
    Ok(())
}

/// The record file for `token`. A malformed token (which could otherwise
/// reach outside the cache directory) is [`HandoffError::Missing`].
pub fn handoff_path(repo_root: &Path, token: &str) -> Result<PathBuf, HandoffError> {
    if !is_token(token) {
        return Err(HandoffError::Missing);
    }
    repo_cache_file(repo_root, &format!("handoff-{token}.json")).map_err(|_| HandoffError::Missing)
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

    fn operation_handoff(dir: &Path, created_at: u64) -> OperationHandoff {
        let target = dir.join("wt");
        fs::create_dir_all(&target).unwrap();
        OperationHandoff::new(OperationId::parse(&"c".repeat(32)).unwrap(), dir.join("base"), target, created_at, Some(vec!["src/a.rs".into()]))
    }

    /// R18 / N9: the token carries the operation id, survives a second
    /// write and read unchanged, and works once.
    #[test]
    fn an_operation_handoff_round_trips_then_replay_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache").join("h.json");
        let handoff = operation_handoff(dir.path(), 1_000);

        write_operation(&path, &handoff).unwrap();
        let first = consume_operation(&path, 1_010).unwrap();
        write_operation(&path, &first).unwrap();
        let second = consume_operation(&path, 1_020).unwrap();

        assert_eq!(first, handoff);
        assert_eq!(second, handoff);
        assert_eq!(consume_operation(&path, 1_020), Err(HandoffError::Missing));
        assert!(!path.exists());
    }

    /// R18: ten minutes, not one; a record from the future is refused too,
    /// and either way the file is gone.
    #[test]
    fn an_operation_handoff_expires_after_ten_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        let handoff = operation_handoff(dir.path(), 1_000);

        write_operation(&path, &handoff).unwrap();
        assert!(consume_operation(&path, 1_061).is_ok(), "past the old one-minute limit");
        write_operation(&path, &handoff).unwrap();
        assert!(consume_operation(&path, 1_600).is_ok(), "exactly ten minutes is still valid");

        write_operation(&path, &handoff).unwrap();
        assert_eq!(consume_operation(&path, 1_601), Err(HandoffError::Expired));
        assert!(!path.exists());
        write_operation(&path, &handoff).unwrap();
        assert_eq!(consume_operation(&path, 999), Err(HandoffError::Expired));
    }

    /// The content-comparing format, another version, a malformed id, or an
    /// unknown field is never read as an operation handoff.
    #[test]
    fn only_the_operation_format_is_an_operation_handoff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        let earlier = serde_json::json!({"format_version": 4, "state": {}, "approvals": {}, "created_at": 0});
        fs::write(&path, serde_json::to_vec(&earlier).unwrap()).unwrap();
        assert_eq!(consume_operation(&path, 0), Err(HandoffError::Missing));

        let mut other = operation_handoff(dir.path(), 0);
        other.format_version = 4;
        write_operation(&path, &other).unwrap();
        assert_eq!(consume_operation(&path, 0), Err(HandoffError::Missing));

        let valid = serde_json::to_value(operation_handoff(dir.path(), 0)).unwrap();
        for (key, value) in [("operation", serde_json::json!("../../etc")), ("extra", serde_json::json!(1))] {
            let mut edited = valid.clone();
            edited[key] = value;
            fs::write(&path, serde_json::to_vec(&edited).unwrap()).unwrap();
            assert_eq!(consume_operation(&path, 0), Err(HandoffError::Missing), "{key}");
        }
    }

    /// R14: the question's dirty paths are required. `null` (no inventory
    /// was shown) and `[]` (none were dirty) are distinct valid values;
    /// absent, another type, or any element that is not a path is
    /// [`HandoffError::Missing`].
    #[test]
    fn the_questions_dirty_paths_have_one_outcome_per_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        let valid = serde_json::to_value(operation_handoff(dir.path(), 0)).unwrap();
        let consume = |edit: &dyn Fn(&mut serde_json::Value)| {
            let mut edited = valid.clone();
            edit(&mut edited);
            fs::write(&path, serde_json::to_vec(&edited).unwrap()).unwrap();
            consume_operation(&path, 0)
        };
        let set = |value: serde_json::Value| move |handoff: &mut serde_json::Value| handoff["shown_dirty"] = value.clone();

        assert_eq!(consume(&|_| {}).unwrap().shown(), Some(vec![PathBuf::from("src/a.rs")]));
        assert_eq!(consume(&set(serde_json::json!(null))).unwrap().shown(), None);
        assert_eq!(consume(&set(serde_json::json!([]))).unwrap().shown(), Some(Vec::new()));
        let absent = |handoff: &mut serde_json::Value| {
            handoff.as_object_mut().unwrap().remove("shown_dirty");
        };
        assert_eq!(consume(&absent), Err(HandoffError::Missing), "absent");
        for invalid in [
            serde_json::json!("src/a.rs"),
            serde_json::json!(1),
            serde_json::json!({"display": "src/a.rs"}),
            serde_json::json!([{"display": "src/a.rs"}, 1]),
            serde_json::json!(["src/a.rs", "src/b.rs"]),
            serde_json::json!([null]),
        ] {
            assert_eq!(consume(&set(invalid.clone())), Err(HandoffError::Missing), "{invalid}");
        }
    }

    /// A dirty path that is not UTF-8 survives the handoff byte for byte.
    #[cfg(unix)]
    #[test]
    fn a_non_utf8_dirty_path_round_trips() {
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.json");
        let name = PathBuf::from(std::ffi::OsStr::from_bytes(b"caf\xe9.txt"));
        let mut handoff = operation_handoff(dir.path(), 0);
        handoff.shown_dirty = Some(vec![name.clone().into()]);

        write_operation(&path, &handoff).unwrap();

        assert_eq!(consume_operation(&path, 0).unwrap().shown(), Some(vec![name]));
    }

    /// No content is compared: only where the caller stands and which
    /// repository it runs in.
    #[test]
    fn the_second_run_checks_the_caller_and_the_repository_only() {
        let dir = tempfile::tempdir().unwrap();
        let handoff = operation_handoff(dir.path(), 0);
        let repo = dir.path().join("base");
        fs::create_dir_all(&repo).unwrap();

        let inside = handoff.target.join("src");
        fs::create_dir_all(&inside).unwrap();
        assert_eq!(verify_operation(&handoff, &repo, &repo), Ok(()));
        assert_eq!(verify_operation(&handoff, &repo, &inside), Err(HandoffRefusal::InsideTarget));
        let other = dir.path().join("other");
        fs::create_dir_all(&other).unwrap();
        assert_eq!(verify_operation(&handoff, &other, &repo), Err(HandoffRefusal::Changed(vec!["repository"])));
    }

    /// Spellings that name the same directory (a symlinked temp dir, Windows
    /// `\\?\` prefixes) agree.
    #[cfg(unix)]
    #[test]
    fn paths_compare_canonical() {
        let dir = tempfile::tempdir().unwrap();
        let handoff = operation_handoff(dir.path(), 0);
        let repo = dir.path().join("base");
        fs::create_dir_all(&repo).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&handoff.target, &link).unwrap();
        let repo_link = dir.path().join("base-link");
        std::os::unix::fs::symlink(&repo, &repo_link).unwrap();

        assert!(is_within(&link.join("."), &handoff.target));
        assert_eq!(verify_operation(&handoff, &repo_link, &repo), Ok(()));
        assert_eq!(verify_operation(&handoff, &repo, &link), Err(HandoffRefusal::InsideTarget));
    }
}
