//! What removing a worktree's directory would delete: its dirty entries
//! (modified, staged, untracked) and its ignored entries.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WorktreeError;
use crate::git::git_from_bytes;

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
    /// For display only: names are decoded lossily.
    pub fn ignored_groups(&self) -> Vec<IgnoredGroup> {
        let mut groups: BTreeMap<String, usize> = BTreeMap::new();
        for entry in &self.ignored {
            let entry = entry.to_string_lossy();
            let name = match entry.split_once('/') {
                Some((first, _)) => format!("{first}/"),
                None => entry.into_owned(),
            };
            *groups.entry(name).or_default() += 1;
        }
        groups
            .into_iter()
            .map(|(name, entries)| IgnoredGroup { name, entries })
            .collect()
    }

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

    /// A BLAKE3 digest over each dirty path, its status, and its working
    /// content; over the set of ignored entries; and over every index entry's
    /// mode, object ID, stage, and path (`git ls-files --stage`).
    ///
    /// A dirty entry that is a directory (git lists an untracked nested
    /// repository or a modified submodule as one entry) contributes every
    /// path beneath it and each file's mode and content, without following
    /// symlinks.
    ///
    /// Two runs of `wt remove` compare this to prove nothing that removal
    /// would delete changed in between: an edit that keeps the status, a
    /// restaged version that keeps both the status and the working bytes, a
    /// new untracked file (including one inside such a directory), a changed
    /// symlink target, a regular file's executable bit (Unix only), or a new
    /// ignored entry all change it. Changes inside ignored entries do not,
    /// nor do timestamps or permission bits git ignores.
    ///
    /// Paths and symlink targets enter as their exact OS bytes
    /// ([`OsStr::as_encoded_bytes`]), never a lossy decoding, and every field
    /// is length-prefixed, so two distinct names never hash alike. The bytes
    /// are platform-specific: the digest is comparable only on one machine.
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
        // By bytes: `Path` ordering compares components and ignores a
        // trailing `/`.
        dirty.sort_by(|a, b| a.path.as_os_str().as_encoded_bytes().cmp(b.path.as_os_str().as_encoded_bytes()));
        let mut ignored: Vec<&PathBuf> = self.ignored.iter().collect();
        ignored.sort_by(|a, b| a.as_os_str().as_encoded_bytes().cmp(b.as_os_str().as_encoded_bytes()));

        let mut record = Record::default();
        for entry in dirty {
            let content = content_digest(&worktree.join(&entry.path))?;
            record
                .field(b"dirty")
                .field(entry.status.as_bytes())
                .field(entry.path.as_os_str().as_encoded_bytes())
                .field(&content);
        }
        for entry in ignored {
            record.field(b"ignored").field(entry.as_os_str().as_encoded_bytes());
        }
        // The whole index rather than a pathspec of the dirty paths, which
        // pathspec magic and command-line length limits make fragile.
        let index = git_from_bytes(base, worktree, &["ls-files", "--stage", "-z"])?;
        record.field(b"index").field(&index);
        Ok(record.digest())
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
    )?;
    Inventory::from_status_z(&output)
}

/// A path from git's `-z` output, byte for byte.
///
/// Windows paths are UTF-16 and git writes them as UTF-8, so bytes that are
/// not UTF-8 there name no file this process could address, and are refused
/// rather than decoded lossily into a different name.
#[cfg(unix)]
fn path_from_git(bytes: &[u8]) -> Result<PathBuf, WorktreeError> {
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// See the Unix variant.
#[cfg(not(unix))]
fn path_from_git(bytes: &[u8]) -> Result<PathBuf, WorktreeError> {
    String::from_utf8(bytes.to_vec()).map(PathBuf::from).map_err(|_| {
        WorktreeError::GitParse(format!(
            "git listed a path that is not UTF-8: {}",
            String::from_utf8_lossy(bytes)
        ))
    })
}

/// Hash input built from length-prefixed fields, so no field's bytes (a
/// filename may hold NUL-free but otherwise arbitrary bytes) can pass for a
/// separator or another field.
#[derive(Default)]
struct Record(Vec<u8>);

impl Record {
    fn field(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        self.0.extend_from_slice(bytes);
        self
    }

    /// The BLAKE3 digest of the fields so far, as hex.
    fn digest(&self) -> String {
        biscuit_hash::blake3_hash_bytes(&self.0)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

/// The working content at `path` as hash input, or `absent` when nothing is
/// there.
fn content_digest(path: &Path) -> Result<Vec<u8>, WorktreeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(b"absent".to_vec());
        }
        Err(error) => return Err(unreadable(path, error)),
    };
    if metadata.is_dir() {
        let mut listing = Record::default();
        // `.git` is walked too: a nested repository's commits and index are
        // lost with the directory, and the outer `git status` never lists them.
        // The outer status may refresh a submodule's own index; that refuses
        // a handoff spuriously, never accepts a change.
        list_directory(path, &[], &mut listing)?;
        return Ok(format!("dir:{}", listing.digest()).into_bytes());
    }
    entry_digest(path, &metadata)
}

/// A file's git mode and BLAKE3 digest, a symlink's exact target bytes
/// (never followed), or a marker for other file types, which have no content
/// to read.
fn entry_digest(path: &Path, metadata: &fs::Metadata) -> Result<Vec<u8>, WorktreeError> {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = fs::read_link(path).map_err(|error| unreadable(path, error))?;
        let mut digest = b"link:".to_vec();
        digest.extend_from_slice(target.as_os_str().as_encoded_bytes());
        return Ok(digest);
    }
    if !file_type.is_file() {
        return Ok(b"special".to_vec());
    }
    let mode = file_mode(metadata);
    fs::File::open(path)
        .and_then(|mut file| biscuit_hash::blake3_hash_reader(&mut file))
        .map(|digest| format!("file:{mode}:{digest}").into_bytes())
        .map_err(|error| unreadable(path, error))
}

/// The mode git would record for this regular file: `100755` when the owner
/// may execute it, as git decides, else `100644`. A chmod alone keeps a
/// modified file's status and bytes, so without this it would pass a handoff.
#[cfg(unix)]
fn file_mode(metadata: &fs::Metadata) -> &'static str {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o100 != 0 {
        "100755"
    } else {
        "100644"
    }
}

/// Windows has no executable bit; git takes the mode from the index there.
#[cfg(not(unix))]
fn file_mode(_metadata: &fs::Metadata) -> &'static str {
    "no-exec-bit"
}

/// Adds a (path, content) field pair per path under `dir`, in name order.
/// The path is the exact name bytes relative to the walk's root, joined by
/// `/`, so it does not depend on the absolute location.
fn list_directory(dir: &Path, prefix: &[u8], listing: &mut Record) -> Result<(), WorktreeError> {
    let mut children = fs::read_dir(dir)
        .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
        .map_err(|error| unreadable(dir, error))?;
    children.sort_by_key(|child| child.file_name());
    for child in children {
        let path = child.path();
        let mut relative = prefix.to_vec();
        relative.extend_from_slice(child.file_name().as_encoded_bytes());
        let metadata = fs::symlink_metadata(&path).map_err(|error| unreadable(&path, error))?;
        if metadata.is_dir() {
            relative.push(b'/');
            listing.field(&relative).field(b"dir");
            list_directory(&path, &relative, listing)?;
        } else {
            let digest = entry_digest(&path, &metadata)?;
            listing.field(&relative).field(&digest);
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
        assert!(inventory.has_source());
        assert!(inventory.needs_consent());
    }

    #[test]
    fn clean_output_needs_no_consent() {
        let inventory = Inventory::from_status_z(b"").unwrap();
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
            .map(PathBuf::from)
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
        let mut ignored: Vec<_> = inventory.ignored.iter().map(|p| p.to_string_lossy().into_owned()).collect();
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

    /// `\xff` and `\xfe` both decode lossily to U+FFFD, so only exact bytes
    /// tell these symlink targets and names apart.
    #[cfg(unix)]
    mod non_utf8_paths {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::symlink;

        use super::*;

        fn relink(link: &Path, target: &[u8]) {
            let _ = fs::remove_file(link);
            symlink(OsStr::from_bytes(target), link).unwrap();
        }

        fn print(repo: &TestRepo, wt: &Path) -> String {
            let base = repo.path();
            collect_inventory(&base, wt).unwrap().fingerprint(&base, wt).unwrap()
        }

        #[test]
        fn status_paths_keep_their_exact_bytes() {
            let inventory = Inventory::from_status_z(b"?? a-\xff\0!! b-\xfe/\0").unwrap();
            assert_eq!(inventory.dirty[0].path.as_os_str().as_bytes(), b"a-\xff");
            assert_eq!(inventory.ignored[0].as_os_str().as_bytes(), b"b-\xfe/");
        }

        #[test]
        fn fingerprint_changes_with_a_symlink_target_at_the_root() {
            let repo = TestRepo::new();
            let wt = repo.add_worktree("feat/x", "feat-x", "main");
            relink(&wt.join("link"), b"target-\xff");
            let approved = print(&repo, &wt);
            assert_eq!(approved, print(&repo, &wt), "stable when nothing changes");

            relink(&wt.join("link"), b"target-\xfe");
            assert_ne!(approved, print(&repo, &wt));
        }

        #[test]
        fn fingerprint_changes_with_a_symlink_target_inside_an_untracked_nested_repo() {
            let repo = TestRepo::new();
            let wt = worktree_with_nested_repo(&repo);
            relink(&wt.join("nested/link"), b"target-\xff");
            let approved = print(&repo, &wt);
            assert_eq!(approved, print(&repo, &wt), "stable when nothing changes");

            relink(&wt.join("nested/link"), b"target-\xfe");
            assert_ne!(approved, print(&repo, &wt));
        }

        /// Needs a filesystem that accepts names that are not UTF-8; macOS
        /// APFS refuses them, so there the test notes the skip and returns.
        #[test]
        fn fingerprint_changes_with_an_edit_to_a_non_utf8_file_or_a_lossy_equal_rename() {
            let repo = TestRepo::new();
            let wt = worktree_with_nested_repo(&repo);
            let file = wt.join(OsStr::from_bytes(b"notes-\xff"));
            if let Err(error) = fs::write(&file, "approved\n") {
                eprintln!("skipped: this filesystem refuses a non-UTF-8 file name ({error})");
                return;
            }
            let approved = print(&repo, &wt);

            fs::write(&file, "edited\n").unwrap();
            let edited = print(&repo, &wt);
            assert_ne!(approved, edited, "an edited root file");

            fs::write(wt.join(OsStr::from_bytes(b"nested/child-\xff")), "same\n").unwrap();
            let before_rename = print(&repo, &wt);
            fs::rename(
                wt.join(OsStr::from_bytes(b"nested/child-\xff")),
                wt.join(OsStr::from_bytes(b"nested/child-\xfe")),
            )
            .unwrap();
            assert_ne!(before_rename, print(&repo, &wt), "a nested child renamed");
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

    /// A chmod keeps a modified file's status (` M`), its bytes, and its
    /// index entry; only the working file's mode tells the states apart.
    #[cfg(unix)]
    mod executable_bit {
        use std::os::unix::fs::PermissionsExt;

        use super::*;

        fn set_mode(path: &Path, mode: u32) {
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        }

        fn print(repo: &TestRepo, wt: &Path) -> String {
            let base = repo.path();
            collect_inventory(&base, wt).unwrap().fingerprint(&base, wt).unwrap()
        }

        #[test]
        fn fingerprint_changes_when_only_a_modified_tracked_files_mode_changes() {
            let repo = TestRepo::new();
            repo.git(&["config", "core.filemode", "true"]);
            fs::write(repo.path().join("run.sh"), "echo one\n").unwrap();
            set_mode(&repo.path().join("run.sh"), 0o644);
            repo.git(&["add", "run.sh"]);
            repo.git(&["commit", "-q", "-m", "script"]);
            let wt = repo.add_worktree("feat/x", "feat-x", "main");
            let script = wt.join("run.sh");
            fs::write(&script, "echo edited\n").unwrap();
            set_mode(&script, 0o644);
            let approved = print(&repo, &wt);
            assert_eq!(approved, print(&repo, &wt), "stable when nothing changes");

            set_mode(&script, 0o755);
            assert_eq!(repo.git_in(&wt, &["status", "--porcelain=v1"]), "M run.sh");
            assert!(repo.git_in(&wt, &["diff", "--summary"]).contains("mode change 100644 => 100755"));
            assert_ne!(approved, print(&repo, &wt));
        }

        #[test]
        fn fingerprint_changes_when_only_a_mode_inside_an_untracked_nested_repo_changes() {
            let repo = TestRepo::new();
            let wt = worktree_with_nested_repo(&repo);
            let notes = wt.join("nested/notes");
            set_mode(&notes, 0o644);
            let approved = print(&repo, &wt);
            assert_eq!(approved, print(&repo, &wt), "stable when nothing changes");

            set_mode(&notes, 0o755);
            assert_ne!(approved, print(&repo, &wt));
        }

        /// Git ignores every permission bit but the owner's execute bit.
        #[test]
        fn fingerprint_ignores_permission_bits_git_does_not_record() {
            let repo = TestRepo::new();
            let wt = worktree_with_nested_repo(&repo);
            let notes = wt.join("nested/notes");
            set_mode(&notes, 0o644);
            let approved = print(&repo, &wt);

            set_mode(&notes, 0o600);
            assert_eq!(approved, print(&repo, &wt));
        }
    }
}
