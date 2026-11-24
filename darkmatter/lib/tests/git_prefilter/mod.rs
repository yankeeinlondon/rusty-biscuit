//! Let Git rule out files a repository scan need not open.
//!
//! A test process's first open of each file costs about 1–2 ms on the native
//! Windows rig, and every new process pays it again, so a scan that opens
//! thousands of files takes seconds there; a single-threaded `git grep` reads
//! the same bytes several times faster. A scan asks
//! [`clean_tracked_files_without`] which files cannot hold its needles and
//! opens only the rest. `current_root_migration_guard`'s
//! `the_git_prefilter_skips_only_clean_tracked_files_without_a_needle` proves
//! the answer never skips a file that holds a needle.

use std::collections::HashSet;
use std::path::Path;

/// `/`-separated paths, relative to `root`, of tracked files whose
/// working-tree bytes Git reports free of every needle (matched as fixed
/// strings), so a scan for those needles need not open them; empty when Git
/// cannot answer (no repository, as in an extracted CI archive), which makes
/// the scan read every file.
///
/// Only entries `ls-files -v` tags `H` qualify: Git greps the index blob
/// rather than the working file for assume-unchanged and skip-worktree
/// entries, so those are read by the scan. The pathspecs only narrow both
/// commands; a scanned file they omit is read. A symlink is listed as itself,
/// never as the files below it, so a scan that follows one reads its files.
pub(crate) fn clean_tracked_files_without(root: &Path, needles: &[&str], pathspecs: &[&str]) -> HashSet<String> {
    let paths = |output: Vec<u8>| -> Vec<String> {
        output
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect()
    };
    let Some(listed) = git(root, &["ls-files", "-z", "-v", "--"], pathspecs) else {
        return HashSet::new();
    };
    // `grep.fullName` would print paths from the top of the repository
    // rather than relative to `root`, as `ls-files` does.
    let mut grep = vec!["-c", "grep.fullName=false", "grep", "--threads", "1", "--no-color", "--no-textconv", "-a", "-l", "-z", "-F"];
    for needle in needles {
        grep.extend(["-e", needle]);
    }
    grep.push("--");
    let Some(matching) = git(root, &grep, pathspecs) else {
        return HashSet::new();
    };
    let matching: HashSet<String> = paths(matching).into_iter().collect();
    paths(listed)
        .into_iter()
        .filter_map(|entry| entry.strip_prefix("H ").map(str::to_string))
        .filter(|path| !matching.contains(path))
        .collect()
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
