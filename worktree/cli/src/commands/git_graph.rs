use worktree::git::git_command;

/// Returns commit short SHAs in oldest-first order (up to `max`),
/// reachable from `target` but not from `exclude`.
fn commits_since(target: &str, exclude: &str, max: usize) -> Vec<String> {
    let max_str = max.to_string();
    match git_command(&[
        "log",
        "--format=%h",
        "--max-count",
        &max_str,
        target,
        "--not",
        exclude,
    ]) {
        Ok(output) if !output.is_empty() => {
            let mut v: Vec<String> = output.lines().map(|s| s.to_string()).collect();
            v.reverse();
            v
        }
        _ => vec![],
    }
}

/// Returns up to `max` ancestor commits ending at `tip`, oldest first.
fn ancestor_commits(tip: &str, max: usize) -> Vec<String> {
    let max_str = max.to_string();
    match git_command(&["log", "--format=%h", "--max-count", &max_str, tip]) {
        Ok(output) if !output.is_empty() => {
            let mut v: Vec<String> = output.lines().map(|s| s.to_string()).collect();
            v.reverse();
            v
        }
        _ => vec![],
    }
}

fn get_merge_base(a: &str, b: &str) -> Option<String> {
    git_command(&["merge-base", a, b])
        .ok()
        .filter(|s| !s.is_empty())
}

fn short_sha(full: &str) -> String {
    git_command(&["rev-parse", "--short", full])
        .unwrap_or_else(|_| full[..7.min(full.len())].to_string())
}

/// Build Mermaid gitGraph instructions for the 2-branch case
/// (current worktree branch vs. the default/main branch).
pub fn worktree_graph(current_branch: &str, default_branch: &str) -> Option<String> {
    let base_sha = get_merge_base(default_branch, current_branch)?;

    // Up to 3 context commits ending at the merge-base (oldest first)
    let context = ancestor_commits(&base_sha, 3);
    // Commits on the feature branch since divergence
    let branch_commits = commits_since(current_branch, &base_sha, 20);
    // Commits on main since divergence
    let main_after = commits_since(default_branch, &base_sha, 20);

    let mut lines = vec!["gitGraph".to_string()];
    for sha in &context {
        lines.push(format!("    commit id: \"{sha}\""));
    }
    if !branch_commits.is_empty() {
        lines.push(format!("    branch {current_branch}"));
        lines.push(format!("    checkout {current_branch}"));
        for sha in &branch_commits {
            lines.push(format!("    commit id: \"{sha}\""));
        }
        lines.push(format!("    checkout {default_branch}"));
    }
    for sha in &main_after {
        lines.push(format!("    commit id: \"{sha}\""));
    }

    Some(lines.join("\n"))
}

struct BranchData {
    name: String,
    /// Index of the merge-base commit in the main commit list
    merge_base_idx: usize,
    commits: Vec<String>,
}

/// Build Mermaid gitGraph instructions for the base (main) case,
/// showing up to 10 recent main commits and all active worktree branches.
pub fn base_graph(branch_names: &[String], default_branch: &str) -> Option<String> {
    // Last 10 commits on main, oldest first
    let main_commits = ancestor_commits(default_branch, 10);
    if main_commits.is_empty() {
        return None;
    }

    let mut branch_data: Vec<BranchData> = Vec::new();
    for branch in branch_names {
        if branch == default_branch {
            continue;
        }
        let Some(base_sha) = get_merge_base(default_branch, branch) else {
            continue;
        };
        let base_short = short_sha(&base_sha);
        // If the merge-base is outside our window, anchor it at position 0
        let merge_base_idx = main_commits
            .iter()
            .position(|s| s == &base_short)
            .unwrap_or(0);
        let commits = commits_since(branch, &base_sha, 10);
        branch_data.push(BranchData {
            name: branch.clone(),
            merge_base_idx,
            commits,
        });
    }

    // Sort so branches appear in the order their divergence point appears in main
    branch_data.sort_by_key(|b| b.merge_base_idx);

    let mut lines = vec!["gitGraph".to_string()];
    let mut branch_iter = branch_data.iter().peekable();

    for (i, sha) in main_commits.iter().enumerate() {
        lines.push(format!("    commit id: \"{sha}\""));
        // After each main commit, insert any branches that diverge here
        while branch_iter
            .peek()
            .map_or(false, |b| b.merge_base_idx == i)
        {
            let info = branch_iter.next().unwrap();
            if info.commits.is_empty() {
                continue;
            }
            lines.push(format!("    branch {}", info.name));
            lines.push(format!("    checkout {}", info.name));
            for c in &info.commits {
                lines.push(format!("    commit id: \"{c}\""));
            }
            lines.push(format!("    checkout {default_branch}"));
        }
    }

    Some(lines.join("\n"))
}
