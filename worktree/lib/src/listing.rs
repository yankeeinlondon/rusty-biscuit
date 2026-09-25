//! The facts behind the `wt list` table: ref tips, branch comparisons, the
//! caption, and the fork tree.
//!
//! The design is item 5 of the worktree fix `2026-09-24-ux-improvements`.
//! Every column after Branch answers one question: what happens if this row's
//! branch merges into the column's target?

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;

use crate::cache::{CACHE_FORMAT_VERSION, Cache, CacheKey, CacheValue};
use crate::fork_origin::ForkOriginStore;
use crate::git::git_command;
use crate::worktree::WorktreeEntry;

/// Local and remote-tracking branch tips, read with one `git for-each-ref`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefTips {
    /// Local branch name to full SHA.
    pub local: BTreeMap<String, String>,
    /// Remote-tracking name (`origin/main`) to full SHA. Symbolic `HEAD`
    /// entries are left out.
    pub remote: BTreeMap<String, String>,
}

impl RefTips {
    /// The `--format` [`RefTips::parse`] reads.
    pub const FORMAT: &'static str = "--format=%(objectname) %(refname)";

    /// Reads every local and remote-tracking branch tip in the current
    /// repository. `None` when git fails, so a failure is never mistaken for
    /// a repository without branches.
    pub fn read() -> Option<Self> {
        git_command(&["for-each-ref", Self::FORMAT, "refs/heads", "refs/remotes"])
            .ok()
            .map(|output| Self::parse(&output))
    }

    /// Parses `<sha> <full refname>` lines; other lines are skipped.
    pub fn parse(output: &str) -> Self {
        let mut tips = Self::default();
        for line in output.lines() {
            let Some((sha, refname)) = line.trim().split_once(' ') else {
                continue;
            };
            if let Some(branch) = refname.strip_prefix("refs/heads/") {
                tips.local.insert(branch.to_string(), sha.to_string());
            } else if let Some(remote) = refname.strip_prefix("refs/remotes/")
                && !remote.ends_with("/HEAD")
            {
                tips.remote.insert(remote.to_string(), sha.to_string());
            }
        }
        tips
    }

    pub fn local(&self, branch: &str) -> Option<&str> {
        self.local.get(branch).map(String::as_str)
    }

    pub fn remote(&self, name: &str) -> Option<&str> {
        self.remote.get(name).map(String::as_str)
    }
}

/// One branch measured against one target tip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    /// Commits on the branch that the target lacks.
    pub ahead: usize,
    /// Commits on the target that the branch lacks.
    pub behind: usize,
    /// Merging the branch into the target would not conflict.
    pub is_clean: bool,
}

/// What merging a branch into a target would do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeState {
    /// Every commit on the branch is already on the target.
    AlreadyIn,
    /// The branch has commits the target lacks, and they merge cleanly.
    Clean,
    Conflicts,
}

impl Comparison {
    pub fn merge_state(&self) -> MergeState {
        if self.ahead == 0 {
            MergeState::AlreadyIn
        } else if self.is_clean {
            MergeState::Clean
        } else {
            MergeState::Conflicts
        }
    }
}

/// Compares `branch_sha` against `target_sha` through the SHA-pair cache.
///
/// On a miss, `rev-list --left-right --count` and a speculative
/// `merge-tree --write-tree` run in parallel; the merge result is discarded
/// when one side contains the other, since such a merge cannot conflict.
/// `None` when `rev-list` fails; a failure is never cached.
pub fn compare_cached(cache: &Mutex<Cache>, target_sha: &str, branch_sha: &str) -> Option<Comparison> {
    let key = CacheKey {
        target_tip_sha: target_sha.to_string(),
        branch_tip_sha: branch_sha.to_string(),
        version: CACHE_FORMAT_VERSION,
    };
    if let Some(value) = cache.lock().expect("cache mutex poisoned").get(&key).copied() {
        return Some(Comparison {
            ahead: value.ahead,
            behind: value.behind,
            is_clean: value.is_clean,
        });
    }

    let comparison = compare_live(target_sha, branch_sha)?;
    cache.lock().expect("cache mutex poisoned").put(
        key,
        CacheValue {
            ahead: comparison.ahead,
            behind: comparison.behind,
            is_clean: comparison.is_clean,
        },
    );
    Some(comparison)
}

fn compare_live(target_sha: &str, branch_sha: &str) -> Option<Comparison> {
    std::thread::scope(|scope| {
        let counts = scope.spawn(|| ahead_behind(target_sha, branch_sha));
        let merge = scope.spawn(|| {
            git_command(&["merge-tree", "--write-tree", target_sha, branch_sha]).is_ok()
        });
        let (ahead, behind) = counts.join().expect("rev-list thread panicked")?;
        let speculative_is_clean = merge.join().expect("merge-tree thread panicked");
        Some(Comparison {
            ahead,
            behind,
            is_clean: ahead == 0 || behind == 0 || speculative_is_clean,
        })
    })
}

/// `(ahead, behind)` of `branch` relative to `target`.
fn ahead_behind(target: &str, branch: &str) -> Option<(usize, usize)> {
    let range = format!("{target}...{branch}");
    let output = git_command(&["rev-list", "--left-right", "--count", &range]).ok()?;
    let mut parts = output.split_whitespace();
    let behind = parts.next()?.parse().ok()?;
    let ahead = parts.next()?.parse().ok()?;
    Some((ahead, behind))
}

/// The local default branch measured against `origin/<default>`, the one line
/// above the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption {
    /// The local default branch, e.g. `main`.
    pub local: String,
    /// Its origin peer, e.g. `origin/main`.
    pub remote: String,
    /// Local commits the remote lacks.
    pub ahead: usize,
    /// Remote commits the local branch lacks.
    pub behind: usize,
}

/// How [`Caption`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionState {
    InSync,
    Ahead(usize),
    Behind(usize),
    Diverged { ahead: usize, behind: usize },
}

impl Caption {
    pub fn state(&self) -> CaptionState {
        match (self.ahead, self.behind) {
            (0, 0) => CaptionState::InSync,
            (ahead, 0) => CaptionState::Ahead(ahead),
            (0, behind) => CaptionState::Behind(behind),
            (ahead, behind) => CaptionState::Diverged { ahead, behind },
        }
    }
}

/// A row's `-> parent` answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentComparison {
    /// No fork-origin record, or the parent is the default branch (the
    /// `-> {default}` column already answers it).
    NotApplicable,
    /// The recorded parent branch no longer exists.
    Deleted,
    /// Measured against the parent's tip. `None` when git failed.
    Compared(Option<Comparison>),
}

/// The comparisons for one branch in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchComparisons {
    /// Against the default-branch target. `None` on the default branch
    /// itself, without a target, or when git failed.
    pub target: Option<Comparison>,
    pub parent: ParentComparison,
}

/// What a tree row names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    Branch { name: String, is_default: bool },
    /// A recorded fork parent that no longer exists.
    DeletedBranch(String),
    /// A detached worktree, by its HEAD's first seven hex characters.
    Detached(String),
}

/// One row of the Branch column's tree, in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub node: TreeNode,
    /// Index into the listing's worktree entries; `None` for a parent row
    /// that has no worktree.
    pub worktree: Option<usize>,
    /// The tree parent's branch name; `None` at the root level.
    pub parent: Option<String>,
    /// The tree parent is a deleted branch.
    pub parent_deleted: bool,
    /// For each ancestor below the root, outermost first: whether it has
    /// later siblings, so its vertical guide continues past this row. Empty at
    /// depths 0 and 1.
    pub guides: Vec<bool>,
    /// 0 at the root level.
    pub depth: usize,
    /// The last of its siblings.
    pub last: bool,
}

impl TreeRow {
    /// The branch this row names, when it names an existing one.
    pub fn branch(&self) -> Option<&str> {
        match &self.node {
            TreeNode::Branch { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// Builds the Branch column's tree from the worktrees and fork-origin records.
///
/// - The default branch is the first root when it exists locally or a record
///   names it.
/// - A branch sits under its recorded parent. A parent without a worktree
///   still gets a row; a deleted parent gets a [`TreeNode::DeletedBranch`]
///   root. A branch without a record, or caught in a record cycle, is a root.
/// - Siblings follow the records' creation time, then name. Roots after the
///   default branch follow worktree order, then deleted parents by name, then
///   detached worktrees.
/// - Two worktrees on one branch give that branch two rows.
pub fn build_tree(
    entries: &[WorktreeEntry],
    default_branch: &str,
    local_branches: &BTreeMap<String, String>,
    forks: &ForkOriginStore,
) -> Vec<TreeRow> {
    let exists = |branch: &str| local_branches.contains_key(branch);

    let mut worktrees: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        if let Some(branch) = entry.branch.as_deref() {
            worktrees.entry(branch).or_default().push(index);
        }
    }

    // Every branch with a worktree, plus each recorded ancestor.
    let mut parent_of: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut pending: Vec<String> = worktrees.keys().map(|b| b.to_string()).collect();
    if exists(default_branch) {
        pending.push(default_branch.to_string());
    }
    while let Some(branch) = pending.pop() {
        if parent_of.contains_key(&branch) {
            continue;
        }
        let parent = if branch == default_branch || !exists(&branch) {
            None
        } else {
            forks
                .get(&branch)
                .map(|origin| origin.base_branch.clone())
                .filter(|parent| *parent != branch)
        };
        if let Some(parent) = &parent {
            pending.push(parent.clone());
        }
        parent_of.insert(branch, parent);
    }

    // A record cycle would hide its branches from every root; cut it where
    // the walk first returns to its start.
    let names: Vec<String> = parent_of.keys().cloned().collect();
    for name in &names {
        let mut seen = HashSet::from([name.clone()]);
        let mut cursor = parent_of[name].clone();
        while let Some(current) = cursor {
            if !seen.insert(current.clone()) {
                if current == *name {
                    parent_of.insert(name.clone(), None);
                }
                break;
            }
            cursor = parent_of.get(&current).cloned().flatten();
        }
    }

    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (branch, parent) in &parent_of {
        if let Some(parent) = parent {
            children.entry(parent.clone()).or_default().push(branch.clone());
        }
    }
    let creation = |branch: &str| forks.get(branch).map(|origin| origin.created_at);
    for siblings in children.values_mut() {
        siblings.sort_by(|a, b| (creation(a), a).cmp(&(creation(b), b)));
    }

    let first_worktree = |branch: &str| {
        worktrees
            .get(branch)
            .and_then(|indices| indices.first().copied())
            .unwrap_or(usize::MAX)
    };
    let mut roots: Vec<String> = parent_of
        .iter()
        .filter(|(branch, parent)| parent.is_none() && *branch != default_branch)
        .map(|(branch, _)| branch.clone())
        .collect();
    roots.sort_by(|a, b| {
        (!exists(a), first_worktree(a), a).cmp(&(!exists(b), first_worktree(b), b))
    });
    if parent_of.contains_key(default_branch) {
        roots.insert(0, default_branch.to_string());
    }

    let mut rows = Vec::new();
    let context = Walk {
        default_branch,
        exists: &exists,
        worktrees: &worktrees,
        children: &children,
    };
    for root in &roots {
        context.emit(root, None, Vec::new(), 0, true, &mut rows);
    }

    for (index, entry) in entries.iter().enumerate() {
        if entry.branch.is_none() {
            let short = entry
                .head_sha
                .as_deref()
                .map(|sha| sha.chars().take(7).collect())
                .unwrap_or_default();
            rows.push(TreeRow {
                node: TreeNode::Detached(short),
                worktree: Some(index),
                parent: None,
                parent_deleted: false,
                guides: Vec::new(),
                depth: 0,
                last: true,
            });
        }
    }
    rows
}

struct Walk<'a> {
    default_branch: &'a str,
    exists: &'a dyn Fn(&str) -> bool,
    worktrees: &'a HashMap<&'a str, Vec<usize>>,
    children: &'a BTreeMap<String, Vec<String>>,
}

impl Walk<'_> {
    fn emit(
        &self,
        branch: &str,
        parent: Option<&str>,
        guides: Vec<bool>,
        depth: usize,
        last: bool,
        rows: &mut Vec<TreeRow>,
    ) {
        let node = if (self.exists)(branch) || branch == self.default_branch {
            TreeNode::Branch {
                name: branch.to_string(),
                is_default: branch == self.default_branch,
            }
        } else {
            TreeNode::DeletedBranch(branch.to_string())
        };
        let parent_deleted = parent.is_some_and(|parent| {
            parent != self.default_branch && !(self.exists)(parent)
        });
        let slots: Vec<Option<usize>> = match self.worktrees.get(branch) {
            Some(indices) => indices.iter().copied().map(Some).collect(),
            None => vec![None],
        };
        for worktree in slots {
            rows.push(TreeRow {
                node: node.clone(),
                worktree,
                parent: parent.map(str::to_string),
                parent_deleted,
                guides: guides.clone(),
                depth,
                last,
            });
        }

        let Some(kids) = self.children.get(branch) else {
            return;
        };
        let mut child_guides = guides;
        if depth >= 1 {
            child_guides.push(!last);
        }
        for (position, child) in kids.iter().enumerate() {
            let child_last = position + 1 == kids.len();
            self.emit(child, Some(branch), child_guides.clone(), depth + 1, child_last, rows);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::fork_origin::ForkOrigin;

    fn entry(path: &str, branch: Option<&str>, is_main: bool) -> WorktreeEntry {
        WorktreeEntry {
            path: PathBuf::from(path),
            branch: branch.map(str::to_string),
            head_sha: Some("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678".to_string()),
            is_main,
            is_current: false,
        }
    }

    fn branches(names: &[&str]) -> BTreeMap<String, String> {
        names
            .iter()
            .map(|name| (name.to_string(), "0".repeat(40)))
            .collect()
    }

    fn forked(store: &mut ForkOriginStore, branch: &str, parent: &str, created_at: u64) {
        store.insert(
            branch,
            ForkOrigin {
                base_branch: parent.to_string(),
                base_sha: "0".repeat(40),
                created_at,
            },
        );
    }

    /// Each row as `indent + connector + label [@worktree]`, for readable
    /// assertions on the whole tree.
    fn outline(rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|row| {
                let mut line = String::new();
                for guide in &row.guides {
                    line.push_str(if *guide { "│  " } else { "   " });
                }
                if row.depth > 0 {
                    line.push_str(match (row.last, row.parent_deleted) {
                        (true, false) => "└─ ",
                        (false, false) => "├─ ",
                        (true, true) => "└┄ ",
                        (false, true) => "├┄ ",
                    });
                }
                match &row.node {
                    TreeNode::Branch { name, .. } => line.push_str(name),
                    TreeNode::DeletedBranch(name) => line.push_str(&format!("{name} (deleted)")),
                    TreeNode::Detached(sha) => line.push_str(&format!("detached @ {sha}")),
                }
                if let Some(index) = row.worktree {
                    line.push_str(&format!(" @{index}"));
                }
                line
            })
            .collect()
    }

    /// The spec's example table, row for row.
    #[test]
    fn the_spec_example_builds_the_spec_tree() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/fix-wt-ux", Some("fix/wt-ux"), false),
            entry("/r/feat-theme", Some("feat/theme"), false),
            entry("/r/feat-dark-fixes", Some("feat/dark-fixes"), false),
            entry("/r/old-cleanup", Some("chore/old-cleanup"), false),
            entry("/r/spike-parser", Some("spike/parser"), false),
            entry("/r/bisect", None, false),
        ];
        let local = branches(&[
            "main",
            "fix/wt-ux",
            "feat/theme",
            "feat/dark-fixes",
            "chore/old-cleanup",
            "spike/parser",
        ]);
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "fix/wt-ux", "main", 1);
        forked(&mut forks, "feat/theme", "main", 2);
        forked(&mut forks, "feat/dark-fixes", "feat/theme", 3);
        forked(&mut forks, "chore/old-cleanup", "main", 4);
        forked(&mut forks, "spike/parser", "experiments", 5);

        let rows = build_tree(&entries, "main", &local, &forks);
        assert_eq!(
            outline(&rows),
            [
                "main @0",
                "├─ fix/wt-ux @1",
                "├─ feat/theme @2",
                "│  └─ feat/dark-fixes @3",
                "└─ chore/old-cleanup @4",
                "experiments (deleted)",
                "└┄ spike/parser @5",
                "detached @ a1b2c3d @6",
            ]
        );
        assert_eq!(rows[3].parent.as_deref(), Some("feat/theme"));
        assert_eq!(rows[1].parent.as_deref(), Some("main"));
        assert!(rows[6].parent_deleted);
        assert_eq!(rows[5].node, TreeNode::DeletedBranch("experiments".into()));
    }

    #[test]
    fn a_parent_without_a_worktree_still_gets_a_row() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/dark", Some("feat/dark"), false),
        ];
        let local = branches(&["main", "feat/theme", "feat/dark"]);
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "feat/theme", "main", 1);
        forked(&mut forks, "feat/dark", "feat/theme", 2);

        let rows = build_tree(&entries, "main", &local, &forks);
        assert_eq!(outline(&rows), ["main @0", "└─ feat/theme", "   └─ feat/dark @1"]);
    }

    #[test]
    fn the_default_branch_gets_a_row_when_the_base_checkout_is_elsewhere() {
        let entries = vec![
            entry("/r/base", Some("feat/x"), true),
            entry("/r/y", Some("feat/y"), false),
        ];
        let local = branches(&["main", "feat/x", "feat/y"]);
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "feat/x", "main", 1);

        let rows = build_tree(&entries, "main", &local, &forks);
        assert_eq!(outline(&rows), ["main", "└─ feat/x @0", "feat/y @1"]);
        assert_eq!(
            rows[0].node,
            TreeNode::Branch { name: "main".into(), is_default: true }
        );
    }

    #[test]
    fn branches_without_records_sit_at_the_root_in_worktree_order() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/b", Some("zeta"), false),
            entry("/r/a", Some("alpha"), false),
        ];
        let rows = build_tree(&entries, "main", &branches(&["main", "zeta", "alpha"]), &ForkOriginStore::default());
        assert_eq!(outline(&rows), ["main @0", "zeta @1", "alpha @2"]);
        assert!(rows.iter().all(|row| row.depth == 0 && row.parent.is_none()));
    }

    #[test]
    fn siblings_follow_creation_time_then_name() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/c", Some("c"), false),
            entry("/r/b", Some("b"), false),
            entry("/r/a", Some("a"), false),
        ];
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "c", "main", 1);
        forked(&mut forks, "b", "main", 2);
        forked(&mut forks, "a", "main", 2);
        let rows = build_tree(&entries, "main", &branches(&["main", "a", "b", "c"]), &forks);
        assert_eq!(outline(&rows), ["main @0", "├─ c @1", "├─ a @3", "└─ b @2"]);
    }

    #[test]
    fn guides_continue_only_past_ancestors_with_later_siblings() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/1", Some("one"), false),
            entry("/r/2", Some("one-a"), false),
            entry("/r/3", Some("one-a-i"), false),
            entry("/r/4", Some("two"), false),
            entry("/r/5", Some("two-a"), false),
            entry("/r/6", Some("two-a-i"), false),
        ];
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "one", "main", 1);
        forked(&mut forks, "one-a", "one", 2);
        forked(&mut forks, "one-a-i", "one-a", 3);
        forked(&mut forks, "two", "main", 4);
        forked(&mut forks, "two-a", "two", 5);
        forked(&mut forks, "two-a-i", "two-a", 6);
        let local = branches(&["main", "one", "one-a", "one-a-i", "two", "two-a", "two-a-i"]);

        let rows = build_tree(&entries, "main", &local, &forks);
        assert_eq!(
            outline(&rows),
            [
                "main @0",
                "├─ one @1",
                "│  └─ one-a @2",
                "│     └─ one-a-i @3",
                "└─ two @4",
                "   └─ two-a @5",
                "      └─ two-a-i @6",
            ]
        );
    }

    #[test]
    fn a_record_cycle_cannot_hide_branches() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/a", Some("a"), false),
            entry("/r/b", Some("b"), false),
        ];
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "a", "b", 1);
        forked(&mut forks, "b", "a", 2);
        let rows = build_tree(&entries, "main", &branches(&["main", "a", "b"]), &forks);

        let named: HashSet<&str> = rows.iter().filter_map(TreeRow::branch).collect();
        assert_eq!(named, HashSet::from(["main", "a", "b"]));
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn a_record_naming_the_branch_itself_is_ignored() {
        let entries = vec![entry("/r/base", Some("main"), true), entry("/r/a", Some("a"), false)];
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "a", "a", 1);
        let rows = build_tree(&entries, "main", &branches(&["main", "a"]), &forks);
        assert_eq!(outline(&rows), ["main @0", "a @1"]);
    }

    #[test]
    fn two_worktrees_on_one_branch_give_two_rows() {
        let entries = vec![
            entry("/r/base", Some("main"), true),
            entry("/r/a1", Some("a"), false),
            entry("/r/a2", Some("a"), false),
        ];
        let mut forks = ForkOriginStore::default();
        forked(&mut forks, "a", "main", 1);
        let rows = build_tree(&entries, "main", &branches(&["main", "a"]), &forks);
        assert_eq!(outline(&rows), ["main @0", "└─ a @1", "└─ a @2"]);
    }

    #[test]
    fn detached_worktrees_come_last_at_the_root() {
        let mut detached = entry("/r/d", None, false);
        detached.head_sha = Some("ffffffffeeeeeee".to_string());
        let entries = vec![detached, entry("/r/base", Some("main"), true)];
        let rows = build_tree(&entries, "main", &branches(&["main"]), &ForkOriginStore::default());
        assert_eq!(outline(&rows), ["main @1", "detached @ fffffff @0"]);
    }

    #[test]
    fn ref_tips_parse_local_and_remote_branches_and_skip_symbolic_heads() {
        let output = "\
1111111111111111111111111111111111111111 refs/heads/main
2222222222222222222222222222222222222222 refs/heads/feat/theme
3333333333333333333333333333333333333333 refs/remotes/origin/HEAD
3333333333333333333333333333333333333333 refs/remotes/origin/main
malformed
";
        let tips = RefTips::parse(output);
        assert_eq!(tips.local("main"), Some("1111111111111111111111111111111111111111"));
        assert_eq!(tips.local("feat/theme"), Some("2222222222222222222222222222222222222222"));
        assert_eq!(tips.remote("origin/main"), Some("3333333333333333333333333333333333333333"));
        assert_eq!(tips.remote("origin/HEAD"), None);
        assert_eq!(tips.local.len(), 2);
        assert_eq!(tips.remote.len(), 1);
    }

    #[test]
    fn merge_state_reads_ahead_first() {
        let state = |ahead, behind, is_clean| Comparison { ahead, behind, is_clean }.merge_state();
        assert_eq!(state(0, 5, false), MergeState::AlreadyIn);
        assert_eq!(state(0, 0, true), MergeState::AlreadyIn);
        assert_eq!(state(2, 0, true), MergeState::Clean);
        assert_eq!(state(2, 3, true), MergeState::Clean);
        assert_eq!(state(2, 3, false), MergeState::Conflicts);
    }

    #[test]
    fn caption_states_cover_every_direction() {
        let caption = |ahead, behind| Caption {
            local: "main".into(),
            remote: "origin/main".into(),
            ahead,
            behind,
        }
        .state();
        assert_eq!(caption(0, 0), CaptionState::InSync);
        assert_eq!(caption(0, 7), CaptionState::Behind(7));
        assert_eq!(caption(3, 0), CaptionState::Ahead(3));
        assert_eq!(caption(3, 7), CaptionState::Diverged { ahead: 3, behind: 7 });
    }
}

/// `list_worktrees` against real repositories with a local bare `origin`.
#[cfg(test)]
mod repo_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::fork_origin::{ForkOrigin, fork_origin_path, record};
    use crate::git::recorder;
    use crate::remove::test_support::TestRepo;
    use crate::worktree::{WorktreeList, list_worktrees};

    struct DirGuard(PathBuf);

    impl DirGuard {
        fn enter(dir: &Path) -> Self {
            let old = std::env::current_dir().expect("cwd");
            std::env::set_current_dir(dir).expect("enter repo");
            Self(old)
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    /// Removes the repository's cache and fork store when dropped.
    struct StoreCleanup(Vec<PathBuf>);

    impl Drop for StoreCleanup {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn stores(repo: &TestRepo) -> StoreCleanup {
        let paths = vec![
            crate::cache::cache_path(&repo.path()).unwrap(),
            fork_origin_path(&repo.path()).unwrap(),
        ];
        for path in &paths {
            let _ = fs::remove_file(path);
        }
        StoreCleanup(paths)
    }

    fn fork(repo: &TestRepo, branch: &str, parent: &str, created_at: u64) {
        let origin = ForkOrigin {
            base_branch: parent.to_string(),
            base_sha: repo.sha(parent),
            created_at,
        };
        record(&fork_origin_path(&repo.path()).unwrap(), branch, origin).unwrap();
    }

    fn comparisons(list: &WorktreeList, branch: &str) -> BranchComparisons {
        *list
            .comparisons
            .get(branch)
            .unwrap_or_else(|| panic!("no comparisons for {branch}: {:?}", list.comparisons))
    }

    fn target_state(list: &WorktreeList, branch: &str) -> MergeState {
        comparisons(list, branch).target.expect("target comparison").merge_state()
    }

    fn caption_state(list: &WorktreeList) -> Option<CaptionState> {
        list.caption.as_ref().map(Caption::state)
    }

    #[test]
    #[serial_test::serial]
    fn the_caption_and_target_follow_origin_in_every_direction() {
        let repo = TestRepo::with_origin();
        let _stores = stores(&repo);
        let _guard = DirGuard::enter(&repo.path());
        let feature = repo.add_worktree("fix/x", "fix-x", "main");
        repo.commit_in(&feature, "fix.txt");

        // In sync: the local tip is the target.
        let list = list_worktrees().unwrap();
        assert_eq!(caption_state(&list), Some(CaptionState::InSync));
        let target = list.target.clone().unwrap();
        assert_eq!((target.reference.as_str(), target.diverged), ("main", false));
        assert_eq!(list.caption.as_ref().unwrap().remote, "origin/main");

        // PR-driven: origin moved on; the column compares against origin.
        repo.push_commit_to_origin("main", "upstream.txt");
        repo.git(&["fetch", "-q", "origin"]);
        let list = list_worktrees().unwrap();
        assert_eq!(caption_state(&list), Some(CaptionState::Behind(1)));
        assert_eq!(list.target.as_ref().unwrap().reference, "origin/main");
        assert_eq!(list.target.as_ref().unwrap().sha, repo.sha("origin/main"));
        assert_eq!(comparisons(&list, "fix/x").target.unwrap().behind, 1);

        // Diverged: origin is the target and says so.
        repo.commit("local.txt");
        let list = list_worktrees().unwrap();
        assert_eq!(caption_state(&list), Some(CaptionState::Diverged { ahead: 1, behind: 1 }));
        assert!(list.target.as_ref().unwrap().diverged);
        assert_eq!(list.target.as_ref().unwrap().reference, "origin/main");

        // Gitflow-style: local ahead; the local tip is the target.
        repo.git(&["reset", "-q", "--hard", "origin/main"]);
        repo.commit("gitflow.txt");
        let list = list_worktrees().unwrap();
        assert_eq!(caption_state(&list), Some(CaptionState::Ahead(1)));
        assert_eq!(list.target.as_ref().unwrap().reference, "main");
    }

    #[test]
    #[serial_test::serial]
    fn the_target_column_reports_already_in_clean_and_conflicts() {
        let repo = TestRepo::new();
        let _stores = stores(&repo);
        let _guard = DirGuard::enter(&repo.path());

        let merged = repo.add_worktree("chore/merged", "merged", "main");
        repo.commit_in(&merged, "merged.txt");
        repo.git(&["merge", "-q", "--ff-only", "chore/merged"]);

        let clean = repo.add_worktree("feat/clean", "clean", "main");
        repo.commit_in(&clean, "clean.txt");

        let conflicting = repo.add_worktree("feat/conflict", "conflict", "main");
        fs::write(conflicting.join("README.md"), "theirs\n").unwrap();
        repo.git_in(&conflicting, &["commit", "-q", "-am", "theirs"]);
        fs::write(repo.path().join("README.md"), "ours\n").unwrap();
        repo.git(&["commit", "-q", "-am", "ours"]);

        let list = list_worktrees().unwrap();
        assert_eq!(target_state(&list, "chore/merged"), MergeState::AlreadyIn);
        assert_eq!(target_state(&list, "feat/clean"), MergeState::Clean);
        assert_eq!(target_state(&list, "feat/conflict"), MergeState::Conflicts);
        assert!(!list.comparisons.contains_key("main"), "the default row has no comparison");
        assert_eq!(list.caption, None, "no remote, no caption");
        for branch in ["chore/merged", "feat/clean", "feat/conflict"] {
            assert_eq!(comparisons(&list, branch).parent, ParentComparison::NotApplicable);
        }
    }

    #[test]
    #[serial_test::serial]
    fn the_parent_column_and_tree_follow_the_fork_records_and_prune_stale_ones() {
        let repo = TestRepo::new();
        let _stores = stores(&repo);
        let _guard = DirGuard::enter(&repo.path());

        let theme = repo.add_worktree("feat/theme", "feat-theme", "main");
        fork(&repo, "feat/theme", "main", 1);
        repo.commit_in(&theme, "theme.txt");
        let dark = repo.add_worktree("feat/dark", "feat-dark", "feat/theme");
        fork(&repo, "feat/dark", "feat/theme", 2);
        fs::write(dark.join("theme.txt"), "dark\n").unwrap();
        repo.git_in(&dark, &["commit", "-q", "-am", "dark"]);
        fs::write(theme.join("theme.txt"), "light\n").unwrap();
        repo.git_in(&theme, &["commit", "-q", "-am", "light"]);

        repo.git(&["branch", "experiments"]);
        let spike = repo.add_worktree("spike/parser", "spike-parser", "experiments");
        fork(&repo, "spike/parser", "experiments", 3);
        repo.commit_in(&spike, "spike.txt");
        repo.git(&["branch", "-D", "experiments"]);
        fork(&repo, "gone", "main", 4);

        let list = list_worktrees().unwrap();

        assert_eq!(comparisons(&list, "feat/theme").parent, ParentComparison::NotApplicable);
        match comparisons(&list, "feat/dark").parent {
            ParentComparison::Compared(Some(comparison)) => {
                assert_eq!(comparison.merge_state(), MergeState::Conflicts);
            }
            other => panic!("feat/dark should compare with feat/theme, got {other:?}"),
        }
        assert_eq!(comparisons(&list, "spike/parser").parent, ParentComparison::Deleted);

        let labels: Vec<(TreeNode, usize, Option<String>)> = list
            .tree
            .iter()
            .map(|row| (row.node.clone(), row.depth, row.parent.clone()))
            .collect();
        let branch = |name: &str, is_default: bool| TreeNode::Branch { name: name.into(), is_default };
        assert_eq!(
            labels,
            [
                (branch("main", true), 0, None),
                (branch("feat/theme", false), 1, Some("main".into())),
                (branch("feat/dark", false), 2, Some("feat/theme".into())),
                (TreeNode::DeletedBranch("experiments".into()), 0, None),
                (branch("spike/parser", false), 1, Some("experiments".into())),
            ]
        );

        // The deleted branch's own record was pruned; the orphan's was kept.
        let store = crate::fork_origin::ForkOriginStore::load_from(&fork_origin_path(&repo.path()).unwrap());
        assert!(store.get("gone").is_none());
        assert_eq!(store.get("spike/parser").unwrap().base_branch, "experiments");
        assert_eq!(store.len(), 3);
    }

    #[test]
    #[serial_test::serial]
    fn one_cache_serves_the_caption_and_both_target_columns() {
        let repo = TestRepo::with_origin();
        let _stores = stores(&repo);
        let _guard = DirGuard::enter(&repo.path());
        let theme = repo.add_worktree("feat/theme", "feat-theme", "main");
        fork(&repo, "feat/theme", "main", 1);
        repo.commit_in(&theme, "theme.txt");
        let dark = repo.add_worktree("feat/dark", "feat-dark", "feat/theme");
        fork(&repo, "feat/dark", "feat/theme", 2);
        repo.commit_in(&dark, "dark.txt");
        repo.push_commit_to_origin("main", "upstream.txt");
        repo.git(&["fetch", "-q", "origin"]);

        recorder::start_recording();
        let cold = list_worktrees().unwrap();
        let cold_calls = recorder::finish_recording();
        let rev_lists = |calls: &[Vec<String>]| {
            recorder::count_matching(calls, |args| args.first().map(String::as_str) == Some("rev-list"))
        };
        // Caption, two target comparisons, and one parent comparison.
        assert_eq!(rev_lists(&cold_calls), 4, "got {cold_calls:?}");

        recorder::start_recording();
        let warm = list_worktrees().unwrap();
        let warm_calls = recorder::finish_recording();
        assert_eq!(rev_lists(&warm_calls), 0, "got {warm_calls:?}");
        assert_eq!(
            recorder::count_matching(&warm_calls, |args| args.first().map(String::as_str) == Some("merge-base")),
            0,
            "the target is chosen from the cached caption, got {warm_calls:?}"
        );
        assert_eq!(warm.caption, cold.caption);
        assert_eq!(warm.comparisons, cold.comparisons);
        assert_eq!(warm.target, cold.target);
    }
}

