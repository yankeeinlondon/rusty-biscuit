//! Content identity of a working tree: `HEAD` plus the bytes of every path Git
//! reports as changed. See [`GitRepo::working_state`].

use std::fmt::Write as _;
use std::hash::Hasher as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use biscuit_hash::XxHasher;

use super::types::{ChangedBase, GitRepo};
use crate::Result;
use crate::performance::{self, counters};

/// A working tree's identity: two observations of the same commit with the
/// same uncommitted content have the same [`id`](Self::id), whatever their
/// file timestamps, and any content difference changes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingState {
    /// The `HEAD` commit as full hex, or `None` on an unborn branch.
    pub head: Option<String>,
    /// Every path [`GitRepo::changed_paths`] reports against `HEAD`
    /// (staged, unstaged, and nonignored untracked), sorted by path bytes.
    pub changed: Vec<ChangedContent>,
    /// 16 hex digits of xxHash over `head` and `changed`.
    ///
    /// Paths enter the hash in Git's slash-separated spelling, so the same
    /// state yields the same id on every OS (line-ending conversion aside:
    /// the worktree bytes are hashed as they are on disk).
    pub id: String,
}

/// One changed path and what is on disk there now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedContent {
    /// Repository-relative path, as [`GitRepo::changed_paths`] returns it.
    pub path: PathBuf,
    pub content: PathContent,
}

/// What a changed path holds in the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathContent {
    /// Nothing: the path was deleted (or renamed away).
    Absent,
    /// A regular file, by the xxHash of its bytes.
    File(u64),
    /// A symbolic link, by the xxHash of its target path; never followed.
    Symlink(u64),
    /// A directory, such as a submodule checkout; its contents are not read.
    Directory,
}

impl PathContent {
    /// The stable spelling that enters [`WorkingState::id`].
    pub fn token(self) -> String {
        match self {
            Self::Absent => "absent".into(),
            Self::File(hash) => format!("file:{hash:016x}"),
            Self::Symlink(hash) => format!("symlink:{hash:016x}"),
            Self::Directory => "directory".into(),
        }
    }
}

impl GitRepo {
    /// The working tree's [`WorkingState`]: `HEAD` plus a content hash of
    /// every changed path.
    ///
    /// The path set is [`Self::changed_paths`] with [`ChangedBase::Omitted`]
    /// (one status walk), so ignored files never count and every error it
    /// returns — a bare repository, merge conflicts, an unreadable index — is
    /// returned here too. Each changed path is then read once from disk; the
    /// work is counted as [`counters::GIT_WORKING_STATE_PATHS_HASHED`].
    pub fn working_state(&self) -> Result<WorkingState> {
        let paths = self.changed_paths(ChangedBase::Omitted)?;
        let head = self
            .with_cached_gix(super::discovery::head_id_opt)?
            .map(|id| id.to_string());
        let root = self.repo_root();
        let changed = paths
            .into_iter()
            .map(|path| {
                let content = path_content(&root.join(&path))?;
                Ok(ChangedContent { path, content })
            })
            .collect::<Result<Vec<_>>>()?;
        let id = state_id(head.as_deref(), &changed);
        Ok(WorkingState { head, changed, id })
    }
}

/// The id of `head` plus `changed`; see [`WorkingState::id`].
pub(crate) fn state_id(head: Option<&str>, changed: &[ChangedContent]) -> String {
    let mut hasher = XxHasher::new();
    let _ = writeln!(hasher, "head {}", head.unwrap_or("unborn"));
    for entry in changed {
        hasher.write(&gix::path::into_bstr(entry.path.as_path()));
        // NUL cannot occur in a Git path, so it separates path from content
        // without ambiguity.
        let _ = writeln!(hasher, "\0{}", entry.content.token());
    }
    format!("{:016x}", hasher.finish())
}

fn path_content(path: &Path) -> Result<PathContent> {
    performance::increment_counter(counters::GIT_WORKING_STATE_PATHS_HASHED, 1);
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(PathContent::Absent),
        Err(err) => return Err(err.into()),
    };
    if metadata.is_symlink() {
        let target = std::fs::read_link(path)?;
        let mut hasher = XxHasher::new();
        hasher.write(&gix::path::into_bstr(target.as_path()));
        return Ok(PathContent::Symlink(hasher.finish()));
    }
    if metadata.is_dir() {
        return Ok(PathContent::Directory);
    }

    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        // Deleted between the status walk and this read.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(PathContent::Absent),
        Err(err) => return Err(err.into()),
    };
    let mut hasher = XxHasher::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
        total += read as u64;
    }
    performance::increment_counter(counters::FS_BYTES_READ, total);
    Ok(PathContent::File(hasher.finish()))
}
