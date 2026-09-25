//! What removing a worktree's directory would delete: its dirty entries
//! (modified, staged, untracked) and its ignored entries.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WorktreeError;
use crate::git::git_from_raw;

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
    pub ignored: Vec<String>,
}

/// One line of the grouped ignored-entry display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredGroup {
    /// The first path component; a directory ends with `/`.
    pub name: String,
    /// How many ignored entries sit under `name` (1 for a lone file).
    pub entries: usize,
}

impl Inventory {
    /// Whether removing the directory needs consent: any dirty or ignored
    /// entry. Ignore rules keep files out of version control; they do not
    /// make their contents disposable.
    pub fn needs_consent(&self) -> bool {
        !self.dirty.is_empty() || !self.ignored.is_empty()
    }

    pub fn has_source(&self) -> bool {
        self.dirty.iter().any(|entry| entry.is_source)
    }

    /// The ignored entries grouped by their first path component, in path
    /// order: `target/` with 6 entries rather than six `target/...` lines.
    pub fn ignored_groups(&self) -> Vec<IgnoredGroup> {
        let mut groups: BTreeMap<String, usize> = BTreeMap::new();
        for entry in &self.ignored {
            let name = match entry.split_once('/') {
                Some((first, _)) => format!("{first}/"),
                None => entry.clone(),
            };
            *groups.entry(name).or_default() += 1;
        }
        groups
            .into_iter()
            .map(|(name, entries)| IgnoredGroup { name, entries })
            .collect()
    }

    /// Parses `git status --porcelain=v1 -z --ignored=matching` output.
    pub fn from_status_z(output: &str) -> Self {
        let mut inventory = Self::default();
        let mut records = output.split('\0').filter(|record| !record.is_empty());
        while let Some(record) = records.next() {
            if record.len() < 4 {
                continue;
            }
            let (status, path) = (&record[..2], &record[3..]);
            if status.contains(['R', 'C']) {
                // `-z` puts the original path in the following record.
                records.next();
            }
            if status == "!!" {
                inventory.ignored.push(path.to_string());
                continue;
            }
            let path = PathBuf::from(path);
            inventory.dirty.push(DirtyEntry {
                status: status.to_string(),
                is_source: sniff::filesystem::path_kind::is_source_code_path(&path),
                path,
            });
        }
        inventory
    }

    /// A BLAKE3 digest over each dirty path, its status, and its working
    /// content; over the set of ignored entries; and over every index entry's
    /// mode, object ID, stage, and path (`git ls-files --stage`).
    ///
    /// A dirty entry that is a directory (git lists an untracked nested
    /// repository or a modified submodule as one entry) contributes every
    /// path beneath it and each file's content, without following symlinks.
    ///
    /// Two runs of `wt remove` compare this to prove nothing that removal
    /// would delete changed in between: an edit that keeps the status, a
    /// restaged version that keeps both the status and the working bytes, a
    /// new untracked file (including one inside such a directory), or a new
    /// ignored entry all change it. Changes inside ignored entries do not.
    ///
    /// ## Errors
    ///
    /// - [`WorktreeError::Io`] when a path under a dirty entry cannot be
    ///   read; the digest would not cover it. A dirty path that no longer
    ///   exists is not an error.
    /// - [`WorktreeError::GitCommand`] when git cannot list the index; a
    ///   digest without it could not see staged work.
    pub fn fingerprint(&self, base: &Path, worktree: &Path) -> Result<String, WorktreeError> {
        let mut dirty: Vec<&DirtyEntry> = self.dirty.iter().collect();
        dirty.sort_by(|a, b| a.path.cmp(&b.path));
        let mut ignored: Vec<&String> = self.ignored.iter().collect();
        ignored.sort();

        let mut text = String::new();
        for entry in dirty {
            let content = content_digest(&worktree.join(&entry.path))?;
            text.push_str(&format!(
                "dirty\0{}\0{}\0{content}\n",
                entry.status,
                entry.path.to_string_lossy()
            ));
        }
        for entry in ignored {
            text.push_str(&format!("ignored\0{entry}\n"));
        }
        // The whole index rather than a pathspec of the dirty paths, which
        // pathspec magic and command-line length limits make fragile.
        let index = git_from_raw(base, worktree, &["ls-files", "--stage", "-z"])?;
        text.push_str("index\0");
        text.push_str(&index);
        Ok(biscuit_hash::blake3_hash(&text))
    }
}

/// Collects the inventory of the worktree at `worktree`, running git from
/// `base` (see [`crate::git::git_from`]).
///
/// Untracked files are listed individually (`-uall`), except that git lists
/// an untracked nested repository as one directory entry.
pub fn collect_inventory(base: &Path, worktree: &Path) -> Result<Inventory, WorktreeError> {
    let output = git_from_raw(
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
    )?;
    Ok(Inventory::from_status_z(&output))
}

/// The working content at `path`, or `absent` when nothing is there.
fn content_digest(path: &Path) -> Result<String, WorktreeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok("absent".to_string());
        }
        Err(error) => return Err(unreadable(path, error)),
    };
    if metadata.is_dir() {
        let mut listing = String::new();
        // `.git` is walked too: a nested repository's commits and index are
        // lost with the directory, and the outer `git status` never lists them.
        // The outer status may refresh a submodule's own index; that refuses
        // a handoff spuriously, never accepts a change.
        list_directory(path, "", &mut listing)?;
        return Ok(format!("dir:{}", biscuit_hash::blake3_hash(&listing)));
    }
    entry_digest(path, &metadata)
}

/// A file's BLAKE3 digest, a symlink's target (never followed), or a marker
/// for other file types, which have no content to read.
fn entry_digest(path: &Path, metadata: &fs::Metadata) -> Result<String, WorktreeError> {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = fs::read_link(path).map_err(|error| unreadable(path, error))?;
        return Ok(format!("link:{}", target.to_string_lossy()));
    }
    if !file_type.is_file() {
        return Ok("special".to_string());
    }
    fs::File::open(path)
        .and_then(|mut file| biscuit_hash::blake3_hash_reader(&mut file))
        .map(|digest| format!("file:{digest}"))
        .map_err(|error| unreadable(path, error))
}

/// Appends one line per path under `dir`, in name order, keyed by its
/// `/`-separated path relative to the walk's root so every OS hashes the
/// same text.
fn list_directory(dir: &Path, prefix: &str, listing: &mut String) -> Result<(), WorktreeError> {
    let mut children = fs::read_dir(dir)
        .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
        .map_err(|error| unreadable(dir, error))?;
    children.sort_by_key(|child| child.file_name());
    for child in children {
        let path = child.path();
        let relative = format!("{prefix}{}", child.file_name().to_string_lossy());
        let metadata = fs::symlink_metadata(&path).map_err(|error| unreadable(&path, error))?;
        if metadata.is_dir() {
            listing.push_str(&format!("{relative}/\0dir\n"));
            list_directory(&path, &format!("{relative}/"), listing)?;
        } else {
            let digest = entry_digest(&path, &metadata)?;
            listing.push_str(&format!("{relative}\0{digest}\n"));
        }
    }
    Ok(())
}

fn unreadable(path: &Path, error: std::io::Error) -> WorktreeError {
    WorktreeError::Io(std::io::Error::new(
        error.kind(),
        format!("cannot read {}: {error}", path.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn parses_every_status_kind_and_skips_rename_origins() {
        let output = " M src/lib.rs\0A  staged.md\0R  new.rs\0old.rs\0?? notes.txt\0!! target/\0!! .env\0";
        let inventory = Inventory::from_status_z(output);
        let paths: Vec<_> = inventory.dirty.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            ["src/lib.rs", "staged.md", "new.rs", "notes.txt"].map(PathBuf::from)
        );
        assert_eq!(inventory.dirty[2].status, "R ");
        assert!(inventory.dirty[0].is_source);
        assert!(!inventory.dirty[1].is_source);
        assert_eq!(inventory.ignored, ["target/", ".env"]);
        assert!(inventory.has_source());
        assert!(inventory.needs_consent());
    }

    #[test]
    fn clean_output_needs_no_consent() {
        let inventory = Inventory::from_status_z("");
        assert!(!inventory.needs_consent());
        assert!(!inventory.has_source());
    }

    #[test]
    fn ignored_entries_group_by_first_component() {
        let inventory = Inventory {
            dirty: Vec::new(),
            ignored: [
                ".env",
                "target/debug/",
                "target/CACHEDIR.TAG",
                "target/nextest/",
                "notes.md",
                "logs/",
            ]
            .map(String::from)
            .to_vec(),
        };
        assert_eq!(
            inventory.ignored_groups(),
            vec![
                IgnoredGroup { name: ".env".into(), entries: 1 },
                IgnoredGroup { name: "logs/".into(), entries: 1 },
                IgnoredGroup { name: "notes.md".into(), entries: 1 },
                IgnoredGroup { name: "target/".into(), entries: 3 },
            ]
        );
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
        let mut ignored = inventory.ignored.clone();
        ignored.sort();
        // A directory pattern lists `target/` once; `**/build/*` lists children.
        assert_eq!(ignored, [".env", "build/CACHE", "build/out/", "target/"]);
        let groups: Vec<_> = inventory.ignored_groups().into_iter().map(|g| (g.name, g.entries)).collect();
        assert_eq!(
            groups,
            vec![(".env".into(), 1), ("build/".into(), 2), ("target/".into(), 1)]
        );
    }

    #[test]
    fn fingerprint_changes_with_content_status_new_paths_and_ignored_entries() {
        let repo = TestRepo::new();
        fs::write(repo.path().join(".gitignore"), "*.log\n").unwrap();
        repo.git(&["add", ".gitignore"]);
        repo.git(&["commit", "-q", "-m", "ignore"]);
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        fs::write(wt.join("README.md"), "one\n").unwrap();
        let base = repo.path();
        let print = || collect_inventory(&base, &wt).unwrap().fingerprint(&base, &wt).unwrap();

        let first = print();
        assert_eq!(first, print(), "stable when nothing changes");

        // A content-only edit keeps the status (` M`) but changes the digest.
        fs::write(wt.join("README.md"), "two\n").unwrap();
        let content_edit = print();
        assert_ne!(first, content_edit);

        // A status change on the same path.
        repo.git_in(&wt, &["add", "README.md"]);
        let staged = print();
        assert_ne!(content_edit, staged);

        // A new untracked file.
        fs::write(wt.join("new.txt"), "x\n").unwrap();
        let untracked = print();
        assert_ne!(staged, untracked);

        // A new ignored entry.
        fs::write(wt.join("debug.log"), "x\n").unwrap();
        assert_ne!(untracked, print());
    }

    /// A worktree whose only dirty entry is an untracked nested repository,
    /// which git lists as one `?? nested/` line.
    fn worktree_with_nested_repo(repo: &TestRepo) -> PathBuf {
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        fs::create_dir(wt.join("nested")).unwrap();
        repo.git_in(&wt.join("nested"), &["init", "-q", "-b", "main"]);
        fs::write(wt.join("nested/notes"), "approved content\n").unwrap();
        let inventory = collect_inventory(&repo.path(), &wt).unwrap();
        let dirty: Vec<_> = inventory
            .dirty
            .iter()
            .map(|e| (e.status.as_str(), e.path.to_string_lossy().into_owned()))
            .collect();
        assert_eq!(dirty, [("??", "nested/".to_string())]);
        wt
    }

    #[test]
    fn fingerprint_covers_edits_and_new_files_inside_an_untracked_nested_repo() {
        let repo = TestRepo::new();
        let wt = worktree_with_nested_repo(&repo);
        let base = repo.path();
        let print = || collect_inventory(&base, &wt).unwrap().fingerprint(&base, &wt).unwrap();

        let approved = print();
        assert_eq!(approved, print(), "stable when nothing changes");

        fs::write(wt.join("nested/notes"), "new work after approval\n").unwrap();
        let edited = print();
        assert_ne!(approved, edited, "an edited child");

        fs::write(wt.join("nested/new-file"), "new work\n").unwrap();
        assert_ne!(edited, print(), "a new child");
    }

    #[cfg(unix)]
    #[test]
    fn fingerprint_fails_when_a_path_inside_a_dirty_directory_is_unreadable() {
        use std::os::unix::fs::PermissionsExt;

        let repo = TestRepo::new();
        let wt = worktree_with_nested_repo(&repo);
        let base = repo.path();
        let secret = wt.join("nested/notes");
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::File::open(&secret).is_ok() {
            // Root reads it regardless of the mode bits.
            return;
        }

        let result = collect_inventory(&base, &wt).unwrap().fingerprint(&base, &wt);
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o644)).unwrap();
        match result {
            Err(WorktreeError::Io(error)) => assert!(error.to_string().contains("notes"), "{error}"),
            other => panic!("expected an I/O error naming the file, got {other:?}"),
        }
    }

    /// Restaging keeps the status (`MM`) and the working bytes; only the
    /// index entry's object ID differs.
    #[test]
    fn fingerprint_changes_when_only_the_staged_version_changes() {
        let repo = TestRepo::new();
        let wt = repo.add_worktree("feat/x", "feat-x", "main");
        let base = repo.path();
        let stage_then_edit = |staged: &str| {
            fs::write(wt.join("README.md"), staged).unwrap();
            repo.git_in(&wt, &["add", "README.md"]);
            fs::write(wt.join("README.md"), "working copy\n").unwrap();
            let inventory = collect_inventory(&base, &wt).unwrap();
            let statuses: Vec<_> = inventory.dirty.iter().map(|e| e.status.as_str()).collect();
            assert_eq!(statuses, ["MM"]);
            inventory.fingerprint(&base, &wt).unwrap()
        };

        let before = stage_then_edit("staged before\n");
        let after = stage_then_edit("new staged work\n");
        assert_ne!(before, after);
    }
}
