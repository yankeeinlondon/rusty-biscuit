//! Git-resolved include membership and guarded path conversion.

pub mod copy;
pub mod rules;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use crate::error::WorktreeError;
use crate::git::{git_from_bytes, git_from_bytes_allow_no_match};

pub use rules::IncludeRules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind { File, Symlink, Other }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedEntry {
    pub path: Vec<u8>,
    pub kind: EntryKind,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IncludeSet {
    pub entries: Vec<IncludedEntry>,
    pub unsupported: Vec<(Vec<u8>, EntryKind)>,
}

pub fn relative_path(bytes: &[u8]) -> io::Result<PathBuf> {
    if bytes.is_empty() || bytes.contains(&0) || bytes.split(|byte| *byte == b'/').any(|part| part.is_empty() || part == b"." || part == b"..") {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid relative path"));
    }
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    };
    #[cfg(windows)]
    let path = PathBuf::from(std::str::from_utf8(bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?);
    if path.is_absolute() || !path.components().all(|part| matches!(part, std::path::Component::Normal(_))) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unsafe relative path"));
    }
    Ok(path)
}

pub fn guarded_kind(root: &Path, relative: &Path) -> io::Result<Option<EntryKind>> {
    let mut current = root.to_path_buf();
    let mut parts = relative.components().peekable();
    while let Some(component) = parts.next() {
        current.push(component);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if parts.peek().is_some() {
            if metadata.file_type().is_symlink() || is_reparse(&metadata) || !metadata.is_dir() {
                return Ok(None);
            }
            if fs::symlink_metadata(current.join(".git")).is_ok() { return Ok(None); }
        } else {
            if is_reparse(&metadata) { return Ok(None); }
            return Ok(Some(if metadata.file_type().is_symlink() {
                if fs::metadata(&current).is_ok_and(|target| target.is_dir()) { EntryKind::Other }
                else { EntryKind::Symlink }
            }
                else if metadata.is_file() { EntryKind::File } else { EntryKind::Other }));
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0 && !metadata.file_type().is_symlink()
}
#[cfg(not(windows))]
fn is_reparse(_: &fs::Metadata) -> bool { false }

pub fn resolve_include_set(base: &Path, worktree: &Path, rules_file: &Path) -> Result<IncludeSet, WorktreeError> {
    let rule = biscuit_file::canonicalize_simplified(rules_file)
        .map_err(|e| WorktreeError::IncludeSetDiscovery(e.to_string()))?;
    let rule_arg = format!("--exclude-from={}", rule.display());
    let candidates = git_from_bytes(base, worktree,
        &["ls-files", "--others", "--ignored", &rule_arg, "-z"], None)
        .map_err(|e| WorktreeError::IncludeSetDiscovery(e.to_string()))?;
    if candidates.is_empty() { return Ok(IncludeSet::default()); }
    let checked = git_from_bytes_allow_no_match(base, worktree,
        &["check-ignore", "--stdin", "-z", "--verbose", "--non-matching"], Some(&candidates))
        .map_err(|e| WorktreeError::IncludeSetDiscovery(e.to_string()))?;
    let paths: Vec<&[u8]> = candidates.split(|byte| *byte == 0).filter(|part| !part.is_empty()).collect();
    let fields: Vec<&[u8]> = checked.split(|byte| *byte == 0).collect();
    if fields.len() != paths.len() * 4 + 1 || fields.last() != Some(&b"".as_slice()) {
        return Err(WorktreeError::IncludeSetDiscovery("unexpected check-ignore output".into()));
    }
    let mut set = IncludeSet::default();
    let (rows, remainder) = fields[..fields.len() - 1].as_chunks::<4>();
    if !remainder.is_empty() { return Err(WorktreeError::IncludeSetDiscovery("incomplete check-ignore output".into())); }
    for (path, row) in paths.into_iter().zip(rows) {
        if row[3] != path { return Err(WorktreeError::IncludeSetDiscovery("check-ignore path mismatch".into())); }
        if row[0].is_empty() || row[2].starts_with(b"!") { continue; }
        if path.split(|byte| *byte == b'/').any(|part| part == b".git") { continue; }
        let relative = match relative_path(path) {
            Ok(relative) => relative,
            Err(_) => { set.unsupported.push((path.to_vec(), EntryKind::Other)); continue; }
        };
        match guarded_kind(worktree, &relative) {
            Ok(Some(kind @ (EntryKind::File | EntryKind::Symlink))) =>
                set.entries.push(IncludedEntry { path: path.to_vec(), kind }),
            Ok(Some(kind)) => set.unsupported.push((path.to_vec(), kind)),
            Ok(None) => {},
            Err(error) => return Err(WorktreeError::IncludeSetDiscovery(error.to_string())),
        }
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn rules_states_and_git_intersection() {
        let repo = TestRepo::new();
        assert_eq!(IncludeRules::locate(&repo.path()), IncludeRules::Missing);
        repo.write_include_rules(b"");
        assert_eq!(IncludeRules::locate(&repo.path()), IncludeRules::Empty);
        repo.write_include_rules(b"*.env\n!skip.env\n");
        assert!(matches!(IncludeRules::locate(&repo.path()), IncludeRules::Present(_)));
        fs::write(repo.path().join(".gitignore"), b"*.env\n").unwrap();
        repo.write_ignored("yes.env", b"secret");
        repo.write_ignored("skip.env", b"secret");
        repo.write_ignored("ordinary.txt", b"not ignored");
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert_eq!(set.entries.iter().map(|entry| entry.path.as_slice()).collect::<Vec<_>>(), vec![b"yes.env".as_slice()]);
        fs::remove_file(repo.path().join(".worktreeinclude")).unwrap();
        fs::create_dir(repo.path().join(".worktreeinclude")).unwrap();
        assert!(matches!(IncludeRules::locate(&repo.path()), IncludeRules::Indeterminate(_)));
    }

    #[test]
    fn global_nested_and_path_variants() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"**/*.env\n");
        repo.with_global_excludes(b"*.env\n");
        fs::create_dir(repo.path().join("nested")).unwrap();
        fs::write(repo.path().join("nested/.gitignore"), b"*.env\n").unwrap();
        repo.write_ignored("nested/space name.env", b"secret");
        repo.write_ignored("global.env", b"secret");
        #[cfg(unix)]
        repo.write_ignored("nested/new\nline.env", b"secret");
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert!(set.entries.iter().any(|entry| entry.path == b"nested/space name.env"));
        assert!(set.entries.iter().any(|entry| entry.path == b"global.env"));
        #[cfg(unix)]
        assert!(set.entries.iter().any(|entry| entry.path == b"nested/new\nline.env"));
    }

    #[test]
    fn include_pattern_without_standard_ignore_selects_nothing() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"*.env\n");
        repo.write_ignored("not-ignored.env", b"secret");
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert!(set.entries.is_empty());
    }

    #[test]
    fn excluded_parent_and_nested_repository_stay_out() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"secrets/\n!secrets/keep.env\nforeign/\n");
        fs::write(repo.path().join(".gitignore"), b"secrets/\nforeign/\n").unwrap();
        fs::create_dir(repo.path().join("secrets")).unwrap();
        repo.write_ignored("secrets/keep.env", b"secret");
        fs::create_dir(repo.path().join("foreign")).unwrap();
        fs::create_dir(repo.path().join("foreign/.git")).unwrap();
        repo.write_ignored("foreign/secret.env", b"secret");
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert_eq!(set.entries.iter().map(|entry| entry.path.as_slice()).collect::<Vec<_>>(),
            vec![b"secrets/keep.env".as_slice()]);
    }

    #[cfg(unix)]
    #[test]
    fn rules_symlink_must_resolve_inside_checkout() {
        use std::os::unix::fs::symlink;
        let repo = TestRepo::new();
        fs::write(repo.path().join("inside-rules"), b"*.env\n").unwrap();
        symlink("inside-rules", repo.path().join(".worktreeinclude")).unwrap();
        assert!(matches!(IncludeRules::locate(&repo.path()), IncludeRules::Present(_)));
        fs::remove_file(repo.path().join(".worktreeinclude")).unwrap();
        let outside = repo.cache_path();
        fs::write(&outside, b"*.env\n").unwrap();
        symlink(&outside, repo.path().join(".worktreeinclude")).unwrap();
        assert!(matches!(IncludeRules::locate(&repo.path()), IncludeRules::Indeterminate(_)));
    }

    #[test]
    fn linked_worktree_uses_base_rules_file_against_its_own_paths() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"*.env\n");
        let linked = repo.add_linked_worktree("feature");
        fs::write(linked.join(".gitignore"), b"*.env\n").unwrap();
        fs::write(linked.join("only-here.env"), b"linked").unwrap();
        let set = resolve_include_set(&repo.path(), &linked, &repo.path().join(".worktreeinclude")).unwrap();
        assert_eq!(set.entries.len(), 1);
        assert_eq!(set.entries[0].path, b"only-here.env");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn non_utf8_filename_keeps_its_git_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let repo = TestRepo::new();
        repo.write_include_rules(b"*.env\n");
        fs::write(repo.path().join(".gitignore"), b"*.env\n").unwrap();
        let name = std::ffi::OsStr::from_bytes(b"raw-\xff.env");
        fs::write(repo.path().join(name), b"secret").unwrap();
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert_eq!(set.entries[0].path, b"raw-\xff.env");
    }

    #[cfg(unix)]
    #[test]
    fn linked_directory_is_reported_but_not_selected() {
        use std::os::unix::fs::symlink;
        let repo = TestRepo::new();
        repo.write_include_rules(b"linked\n");
        fs::write(repo.path().join(".gitignore"), b"linked\n").unwrap();
        let outside = repo.cache_path();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.env"), b"secret").unwrap();
        symlink(&outside, repo.path().join("linked")).unwrap();
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert!(set.entries.is_empty());
        assert!(set.unsupported.iter().any(|(path, kind)| path == b"linked" && *kind == EntryKind::Other));
    }

    #[test]
    fn submodule_files_are_not_selected() {
        let repo = TestRepo::with_origin();
        repo.write_include_rules(b"sub/\n");
        let origin = repo.origin_path();
        repo.git(&["-c", "protocol.file.allow=always", "submodule", "add", "-q",
            origin.to_str().unwrap(), "sub"]);
        fs::write(repo.path().join(".gitignore"), b"sub/\n").unwrap();
        fs::write(repo.path().join("sub/secret.env"), b"secret").unwrap();
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert!(set.entries.is_empty(), "{set:?}");
    }

    #[test]
    fn nested_linked_worktree_files_are_not_selected() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"nested-wt/\n");
        let path = repo.path().join("nested-wt");
        repo.git(&["worktree", "add", "-q", "-b", "nested", path.to_str().unwrap(), "main"]);
        fs::write(repo.path().join(".gitignore"), b"nested-wt/\n").unwrap();
        fs::write(path.join("secret.env"), b"secret").unwrap();
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert!(set.entries.is_empty(), "{set:?}");
    }

    #[test]
    fn anchored_rule_and_non_utf8_rule_content_use_git_matching() {
        let repo = TestRepo::new();
        repo.write_include_rules(b"/.env\n# non-utf8: \xff\n");
        fs::write(repo.path().join(".gitignore"), b"*.env\n").unwrap();
        fs::create_dir(repo.path().join("nested")).unwrap();
        repo.write_ignored(".env", b"root");
        repo.write_ignored("nested/.env", b"nested");
        let set = resolve_include_set(&repo.path(), &repo.path(), &repo.path().join(".worktreeinclude")).unwrap();
        assert_eq!(set.entries.iter().map(|entry| entry.path.as_slice()).collect::<Vec<_>>(),
            vec![b".env".as_slice()]);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_rules_are_indeterminate() {
        use std::os::unix::fs::PermissionsExt;
        let repo = TestRepo::new();
        repo.write_include_rules(b"*.env\n");
        let path = repo.path().join(".worktreeinclude");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        assert!(matches!(IncludeRules::locate(&repo.path()), IncludeRules::Indeterminate(_)));
    }
}
