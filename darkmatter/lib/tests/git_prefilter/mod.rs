//! Let Git rule out files a repository scan need not open.
//!
//! A test process's first open of each file costs about 1–2 ms on the native
//! Windows rig, and every new process pays it again, so a scan that opens
//! thousands of files takes seconds there; a single-threaded `git grep` reads
//! the same bytes several times faster. A scan asks
//! [`clean_tracked_files_without`] which files cannot hold its needles and
//! opens only the rest. `current_root_migration_guard`'s
//! `the_git_prefilter_skips_only_clean_tracked_files_without_a_needle` proves
//! the answer never skips a file that holds a needle, and
//! `the_git_prefilter_never_skips_a_file_link` that it never skips a link.

use std::collections::HashSet;
use std::fs::FileType;
use std::path::Path;

/// Tracked files whose working-tree bytes Git reports free of every needle;
/// [`CleanTracked::skips`] is the only way to consult them.
#[derive(Debug, Default)]
pub(crate) struct CleanTracked(HashSet<String>);

impl CleanTracked {
    /// Whether a scan may leave the entry keyed `key` (`/`-separated, relative
    /// to the root given to [`clean_tracked_files_without`]) unopened. `kind`
    /// must be the entry's no-follow type as the walk saw it
    /// (`DirEntry::file_type`, free on every OS): Git's working-tree grep
    /// never reads through a link, so its negative answer holds only for a
    /// regular file, and a link (tracked, or replacing a tracked file) is
    /// read. `symlink_metadata` per file would cost about as much as the open
    /// this saves on native Windows.
    pub(crate) fn skips(&self, key: &str, kind: FileType) -> bool {
        kind.is_file() && self.0.contains(key)
    }
}

/// Tracked files below `root` whose working-tree bytes Git reports free of
/// every needle (matched as fixed strings), so a scan for those needles need
/// not open them; empty when Git cannot answer (no repository, as in an
/// extracted CI archive), which makes the scan read every file.
///
/// Only regular-file index entries (mode `100644` or `100755`) that
/// `ls-files -v` tags `H` qualify: Git greps the index blob rather than the
/// working file for assume-unchanged and skip-worktree entries, and never
/// greps a tracked link (including one checked out as a plain file where
/// `core.symlinks` is off), so those are read by the scan. A tracked file the
/// working tree has replaced with a link is excluded by
/// [`CleanTracked::skips`]. The pathspecs only narrow both commands; a scanned
/// file they omit is read. A directory link is listed as itself, never as the
/// files below it, so a scan that follows one reads its files.
pub(crate) fn clean_tracked_files_without(root: &Path, needles: &[&str], pathspecs: &[&str]) -> CleanTracked {
    let paths = |output: Vec<u8>| -> Vec<String> {
        output
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect()
    };
    let Some(listed) = git(root, &["ls-files", "-z", "-v", "-s", "--"], pathspecs) else {
        return CleanTracked::default();
    };
    // `grep.fullName` would print paths from the top of the repository
    // rather than relative to `root`, as `ls-files` does.
    let mut grep = vec!["-c", "grep.fullName=false", "grep", "--threads", "1", "--no-color", "--no-textconv", "-a", "-l", "-z", "-F"];
    for needle in needles {
        grep.extend(["-e", needle]);
    }
    grep.push("--");
    let Some(matching) = git(root, &grep, pathspecs) else {
        return CleanTracked::default();
    };
    let matching: HashSet<String> = paths(matching).into_iter().collect();
    // Each entry reads `<tag> <mode> <object> <stage>\t<path>`.
    let clean = paths(listed)
        .into_iter()
        .filter_map(|entry| {
            let (fields, path) = entry.split_once('\t')?;
            let mut fields = fields.split(' ');
            let regular = fields.next() == Some("H") && matches!(fields.next(), Some("100644" | "100755"));
            (regular && !matching.contains(path)).then(|| path.to_string())
        })
        .collect();
    CleanTracked(clean)
}

/// Stdout of `git <args> <pathspecs>` run in `root`, or `None` unless it exits
/// 0 or, for `grep`, 1 (no match). Inherited repository variables are
/// dropped so a hook's `GIT_DIR` cannot point the command elsewhere.
fn git(root: &Path, args: &[&str], pathspecs: &[&str]) -> Option<Vec<u8>> {
    let output = std::process::Command::new("git")
        .args(args)
        .args(pathspecs)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    match output.status.code() {
        Some(0) => Some(output.stdout),
        Some(1) if args.contains(&"grep") => Some(Vec::new()),
        _ => None,
    }
}
