//! Test-only binary that renders a fixture `git-status` report through the real
//! [`render_git_section`] path.
//!
//! Level 2 tests run this inside a real terminal and capture the pane to verify
//! that the user-observable terminal styling survives the display pipeline:
//! double-underline section headers (`<uu>`, degrading to straight underline on
//! terminals without double-underline support), blue OSC8 worktree hyperlinks
//! (degrading to a `[label](url)` fallback), and the exact blank-row layout —
//! none of which a non-TTY Level 1 string assertion can observe.
//!
//! A Case A scenario (running inside a linked worktree) is used so the report
//! exercises the `main:` location link, the current-worktree link, and all
//! three section headers (Status, Worktrees, Meta).

use std::collections::HashMap;
use std::path::PathBuf;

use sniff::filesystem::git::{GitConfig, GitInfo, RepoStatus, WorktreeInfo};
use sniff_cli::output::render_git_section;

fn worktree(
    branch: &str,
    path: &str,
    ahead: usize,
    behind: usize,
    is_current: bool,
) -> WorktreeInfo {
    WorktreeInfo {
        branch: branch.to_string(),
        filepath: PathBuf::from(path),
        sha: "abc1234".to_string(),
        dirty: false,
        ahead,
        behind,
        base_branch: "main".to_string(),
        has_conflicts: false,
        merged: false,
        changed_files: 0,
        is_current,
    }
}

fn main() {
    let mut worktrees = HashMap::new();
    worktrees.insert(
        "feature/login".to_string(),
        worktree("feature/login", "/tmp/demo/login-fix", 2, 1, true),
    );
    worktrees.insert(
        "hotfix".to_string(),
        worktree("hotfix", "/tmp/demo/hotfix", 0, 0, false),
    );

    let git = GitInfo {
        repo_root: PathBuf::from("/tmp/demo/login-fix"),
        org: None,
        repo: None,
        current_branch: Some("feature/login".to_string()),
        head_id: None,
        branches: vec![],
        in_worktree: true,
        base_repo_root: Some(PathBuf::from("/tmp/demo/project")),
        recent: vec![],
        status: Some(RepoStatus::default()),
        remotes: vec![],
        worktrees,
        config: GitConfig::default(),
        tracking: vec![],
        file_changes: vec![],
    };

    // `render_git_section` constructs its own `Terminal::default()` internally,
    // so capability detection happens inside the real terminal at runtime.
    print!("{}", render_git_section(&git, 10, 0, false, None, None));
}
