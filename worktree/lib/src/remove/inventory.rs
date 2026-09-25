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

    /// A BLAKE3 digest over each dirty path, its status, and its content, and
    /// over the set of ignored entries.
    ///
    /// Two runs of `wt remove` compare this to prove nothing that removal
    /// would delete changed in between: an edit that keeps the status, a new
    /// untracked file, or a new ignored entry all change it.
    pub fn fingerprint(&self, worktree: &Path) -> String {
        let mut dirty: Vec<&DirtyEntry> = self.dirty.iter().collect();
        dirty.sort_by(|a, b| a.path.cmp(&b.path));
        let mut ignored: Vec<&String> = self.ignored.iter().collect();
        ignored.sort();

        let mut text = String::new();
        for entry in dirty {
            let content = content_digest(&worktree.join(&entry.path));
            text.push_str(&format!(
                "dirty\0{}\0{}\0{content}\n",
                entry.status,
                entry.path.to_string_lossy()
            ));
        }
        for entry in ignored {
            text.push_str(&format!("ignored\0{entry}\n"));
        }
        biscuit_hash::blake3_hash(&text)
    }
}

/// Collects the inventory of the worktree at `worktree`, running git from
/// `base` (see [`crate::git::git_from`]).
///
/// Untracked files are listed individually (`-uall`), so the count covers
/// every path.
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

fn content_digest(path: &Path) -> String {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return "absent".to_string();
    };
    if metadata.file_type().is_symlink() {
        return fs::read_link(path)
            .map(|target| format!("link:{}", target.to_string_lossy()))
            .unwrap_or_else(|_| "link".to_string());
    }
    if metadata.is_dir() {
        return "dir".to_string();
    }
    fs::File::open(path)
        .and_then(|mut file| biscuit_hash::blake3_hash_reader(&mut file))
        .unwrap_or_else(|_| "unreadable".to_string())
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
        let print = || collect_inventory(&base, &wt).unwrap().fingerprint(&wt);

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
}
