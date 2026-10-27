//! What removing a worktree's directory would delete: its dirty entries, its
//! ignored entries, and the `.worktreeinclude` copies that differ from their
//! baseline.

#[cfg(unix)]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::WorktreeError;
use crate::git::git_from_bytes;
use super::included::IncludedAssessment;

/// One modified, staged, or untracked path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyEntry {
    /// The two porcelain status characters (`" M"`, `"A "`, `"??"`, ...).
    pub status: String,
    /// Repository-relative; the new path for a rename or copy.
    pub path: PathBuf,
    pub is_source: bool,
}

/// Everything in a worktree that exists nowhere else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    pub dirty: Vec<DirtyEntry>,
    /// `git status --ignored=matching` entries, repository-relative. A
    /// directory ends with `/` and is listed once only when a directory
    /// pattern matches it; otherwise its ignored children are listed.
    pub ignored: Vec<PathBuf>,
    pub included: IncludedAssessment,
}

impl Inventory {
    /// Parses `git status --porcelain=v1 -z --ignored=matching` output,
    /// keeping each path's exact bytes.
    ///
    /// ## Errors
    ///
    /// [`WorktreeError::GitParse`] for a path that is not UTF-8 on a platform
    /// whose paths are not bytes (Windows).
    pub fn from_status_z(output: &[u8]) -> Result<Self, WorktreeError> {
        let mut inventory = Self::default();
        let mut records = output.split(|&byte| byte == 0).filter(|record| !record.is_empty());
        while let Some(record) = records.next() {
            if record.len() < 4 {
                continue;
            }
            let (status, path) = (String::from_utf8_lossy(&record[..2]), path_from_git(&record[3..])?);
            if status.contains(['R', 'C']) {
                // `-z` puts the original path in the following record.
                records.next();
            }
            if status == "!!" {
                inventory.ignored.push(path);
                continue;
            }
            inventory.dirty.push(DirtyEntry {
                status: status.into_owned(),
                is_source: sniff::filesystem::path_kind::is_source_code_path(&path),
                path,
            });
        }
        Ok(inventory)
    }
}

/// Collects the inventory of the worktree at `worktree`, running git from
/// `base` (see [`crate::git::git_from`]).
///
/// Untracked files are listed individually (`-uall`), except that git lists
/// an untracked nested repository as one directory entry.
pub fn collect_inventory(base: &Path, worktree: &Path) -> Result<Inventory, WorktreeError> {
    let output = git_from_bytes(
        base,
        worktree,
        &[
            "-c",
            "core.untrackedCache=true",
            "-c",
            "core.quotePath=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignored=matching",
        ],
        None,
    )?;
    Inventory::from_status_z(&output)
}

/// A path from git's `-z` output, byte for byte.
///
/// Windows paths are UTF-16 and git writes them as UTF-8, so bytes that are
/// not UTF-8 there name no file this process could address, and are refused
/// rather than decoded lossily into a different name.
#[cfg(unix)]
pub(crate) fn path_from_git(bytes: &[u8]) -> Result<PathBuf, WorktreeError> {
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// See the Unix variant.
#[cfg(not(unix))]
pub(crate) fn path_from_git(bytes: &[u8]) -> Result<PathBuf, WorktreeError> {
    String::from_utf8(bytes.to_vec()).map(PathBuf::from).map_err(|_| {
        WorktreeError::GitParse(format!(
            "git listed a path that is not UTF-8: {}",
            String::from_utf8_lossy(bytes)
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn parses_every_status_kind_and_skips_rename_origins() {
        let output = " M src/lib.rs\0A  staged.md\0R  new.rs\0old.rs\0?? notes.txt\0!! target/\0!! .env\0";
        let inventory = Inventory::from_status_z(output.as_bytes()).unwrap();
        let paths: Vec<_> = inventory.dirty.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            ["src/lib.rs", "staged.md", "new.rs", "notes.txt"].map(PathBuf::from)
        );
        assert_eq!(inventory.dirty[2].status, "R ");
        assert!(inventory.dirty[0].is_source);
        assert!(!inventory.dirty[1].is_source);
        assert_eq!(inventory.ignored, ["target/", ".env"].map(PathBuf::from));
    }

    #[test]
    fn clean_output_lists_nothing() {
        let inventory = Inventory::from_status_z(b"").unwrap();
        assert_eq!(inventory, Inventory::default());
    }

    #[test]
    fn collects_dirty_and_ignored_entries_from_a_real_worktree() {
        let repo = TestRepo::new();
        let path = repo.path();
        fs::write(path.join(".gitignore"), "target/\n.env\n**/build/*\n").unwrap();
        repo.git(&["add", ".gitignore"]);
        repo.git(&["commit", "-q", "-m", "ignore"]);
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        fs::write(wt.join("README.md"), "changed\n").unwrap();
        fs::write(wt.join("staged.rs"), "fn a() {}\n").unwrap();
        repo.git_in(&wt, &["add", "staged.rs"]);
        fs::create_dir_all(wt.join("new/deep")).unwrap();
        fs::write(wt.join("new/deep/file.txt"), "x\n").unwrap();
        fs::write(wt.join(".env"), "SECRET=1\n").unwrap();
        fs::create_dir_all(wt.join("target/debug")).unwrap();
        fs::write(wt.join("target/debug/bin"), "x").unwrap();
        fs::create_dir_all(wt.join("build/out")).unwrap();
        fs::write(wt.join("build/out/a.o"), "x").unwrap();
        fs::write(wt.join("build/CACHE"), "x").unwrap();

        let inventory = collect_inventory(&path, &wt).unwrap();
        let mut dirty: Vec<_> = inventory
            .dirty
            .iter()
            .map(|e| (e.status.as_str(), e.path.to_string_lossy().into_owned()))
            .collect();
        dirty.sort();
        assert_eq!(
            dirty,
            vec![
                (" M", "README.md".to_string()),
                ("??", "new/deep/file.txt".to_string()),
                ("A ", "staged.rs".to_string()),
            ]
        );
        let mut ignored: Vec<_> = inventory.ignored.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        ignored.sort();
        // A directory pattern lists `target/` once; `**/build/*` lists children.
        assert_eq!(ignored, [".env", "build/CACHE", "build/out/", "target/"]);
    }

    /// `\xff` and `\xfe` both decode lossily to U+FFFD, so only exact bytes
    /// tell these names apart.
    #[cfg(unix)]
    mod non_utf8_paths {
        use std::os::unix::ffi::OsStrExt;

        use super::*;

        #[test]
        fn status_paths_keep_their_exact_bytes() {
            let inventory = Inventory::from_status_z(b"?? a-\xff\0!! b-\xfe/\0").unwrap();
            assert_eq!(inventory.dirty[0].path.as_os_str().as_bytes(), b"a-\xff");
            assert_eq!(inventory.ignored[0].as_os_str().as_bytes(), b"b-\xfe/");
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_status_path_that_is_not_utf8_is_refused_on_windows() {
        assert!(matches!(
            Inventory::from_status_z(b"?? a-\xff\0"),
            Err(WorktreeError::GitParse(_))
        ));
    }
}
