---
phases: 5
created: 2026-05-24
start_phase: 1
source_files_during_phase_1:
  - worktree/lib/src/worktree.rs
  - worktree/justfile
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - worktree/lib/src/worktree.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - worktree/cli/src/args.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/commands/mod.rs
  - worktree/cli/src/commands/remove.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - worktree/cli/src/commands/dirty_tree.rs
  - worktree/cli/src/commands/mod.rs
  - worktree/cli/Cargo.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - worktree/cli/src/commands/remove.rs
docs_updated_during_phase_5:
  - worktree/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - worktree
  - worktree-cli
---

# `wt remove` Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `wt remove <name>` subcommand that safely removes a worktree, with `sniff`-driven dirtiness analysis, a hierarchical view of uncommitted files, tiered `--force` semantics, and optional `--branch` soft-delete.

**Architecture:** Lib layer (`worktree`) gains pure functions: `list_dirty_files` (returns relative `PathBuf`s + a `DirtyStatus` summary using `sniff::filesystem::path_kind::is_source_code_path`), `remove_worktree_with_options(path, force)`, and `delete_branch(name)` (soft delete via `git branch -d`). CLI layer (`worktree-cli`) adds a `Remove` subcommand whose handler implements the spec's confirmation matrix, renders dirty files as a hierarchical tree (custom `dirty_tree` module using `biscuit_terminal::components::filesystem::tree_chars` + `Prose`), drives confirmations with `inquire::Confirm`, then orchestrates removal and optional branch cleanup.

**Tech Stack:** Rust 2024, `clap` (derive + `ArgAction::Count` for `-f`/`-ff`), `inquire::Confirm`, `biscuit-terminal` (`Prose`, `tree_chars`), `sniff::filesystem::path_kind`, `worktree` lib, `git` subprocess (already wired via `git_command`/`git_command_in`).

**Spec reference:** `worktree/features/2026-04011-remove-cmd/spec.md`

---

## Phase 1: Library — Dirty-File Enumeration

**Goal:** Surface the list of uncommitted files in a worktree (relative paths) plus a source-code classification, reusing the existing `dirty_status` logic.

### Task 1.1: Add `DirtyFiles` struct and `list_dirty_files` function

**Files:**
- Modify: `worktree/lib/src/worktree.rs` (add new public type + function near the existing `dirty_status` function, around line 192)

- [ ] **Step 1: Write the failing test**

Append to the existing `#[cfg(test)] mod tests` block in `worktree/lib/src/worktree.rs`:

```rust
#[test]
fn classify_dirty_lines_clean() {
    let dirty = DirtyFiles::from_porcelain("");
    assert!(dirty.paths.is_empty());
    assert!(!dirty.has_source);
    assert_eq!(dirty.status(), DirtyStatus::Clean);
}

#[test]
fn classify_dirty_lines_non_source_only() {
    let porcelain = " M README.md\n?? notes.txt\n";
    let dirty = DirtyFiles::from_porcelain(porcelain);
    assert_eq!(dirty.paths.len(), 2);
    assert!(!dirty.has_source);
    assert_eq!(dirty.status(), DirtyStatus::DirtyNonSource);
}

#[test]
fn classify_dirty_lines_source_present() {
    let porcelain = " M README.md\n M src/lib.rs\n";
    let dirty = DirtyFiles::from_porcelain(porcelain);
    assert_eq!(dirty.paths.len(), 2);
    assert!(dirty.has_source);
    assert_eq!(dirty.status(), DirtyStatus::DirtySource);
}

#[test]
fn classify_dirty_lines_rename() {
    let porcelain = "R  old/foo.rs -> new/foo.rs\n";
    let dirty = DirtyFiles::from_porcelain(porcelain);
    assert_eq!(dirty.paths.len(), 1);
    assert_eq!(dirty.paths[0], std::path::PathBuf::from("new/foo.rs"));
    assert!(dirty.has_source);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p worktree --lib classify_dirty_lines`
Expected: FAIL with `cannot find type DirtyFiles` / `function from_porcelain`.

- [ ] **Step 3: Implement `DirtyFiles` and `from_porcelain`**

Insert into `worktree/lib/src/worktree.rs` directly above `pub fn dirty_status(...)` (around line 192):

```rust
/// A snapshot of the uncommitted files in a worktree, classified by content kind.
///
/// `paths` are repository-relative (as emitted by `git status --porcelain`); for
/// renames the new path is recorded. `has_source` is true if at least one path
/// classifies as source code under
/// [`sniff::filesystem::path_kind::is_source_code_path`].
#[derive(Debug, Clone, Default)]
pub struct DirtyFiles {
    pub paths: Vec<PathBuf>,
    pub has_source: bool,
}

impl DirtyFiles {
    /// Classify a worktree's `git status --porcelain` output.
    pub fn from_porcelain(porcelain: &str) -> Self {
        let mut paths = Vec::new();
        let mut has_source = false;
        for line in porcelain.lines() {
            let Some(file_path) = porcelain_path(line) else {
                continue;
            };
            let p = PathBuf::from(file_path);
            if !has_source && sniff::filesystem::path_kind::is_source_code_path(&p) {
                has_source = true;
            }
            paths.push(p);
        }
        Self { paths, has_source }
    }

    /// Folded summary equivalent to [`dirty_status`].
    pub fn status(&self) -> DirtyStatus {
        if self.paths.is_empty() {
            DirtyStatus::Clean
        } else if self.has_source {
            DirtyStatus::DirtySource
        } else {
            DirtyStatus::DirtyNonSource
        }
    }
}

/// List uncommitted files for the worktree rooted at `path`.
///
/// Returns an empty [`DirtyFiles`] (clean) if git fails, mirroring
/// [`dirty_status`]'s degraded-mode behavior so callers stay robust.
pub fn list_dirty_files(path: &Path) -> DirtyFiles {
    let Ok(output) = git_command_in(
        path,
        &["-c", "core.untrackedCache=true", "status", "--porcelain"],
    ) else {
        return DirtyFiles::default();
    };
    DirtyFiles::from_porcelain(&output)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p worktree --lib classify_dirty_lines`
Expected: PASS, 4 tests.

- [ ] **Step 5: Run full lib test suite to ensure no regressions**

Run: `cargo test -p worktree --lib`
Expected: PASS, including the four new tests.

- [ ] **Step 6: Commit**

```bash
git add worktree/lib/src/worktree.rs
git commit -m "feat(worktree): add DirtyFiles + list_dirty_files for per-path dirtiness"
```

---

## Phase 2: Library — Removal + Branch Cleanup APIs

**Goal:** Replace the always-force `remove_worktree` with a force-aware API and add a soft branch-delete helper.

### Task 2.1: Add force-aware `remove_worktree` and `delete_branch`

**Files:**
- Modify: `worktree/lib/src/worktree.rs` (replace the existing `remove_worktree` at lines 374-378; add `delete_branch` below it)

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `worktree/lib/src/worktree.rs`:

```rust
#[test]
fn delete_branch_outcome_variants_construct() {
    // Smoke-test that the outcome enum is constructable + matchable.
    let merged = DeleteBranchOutcome::Deleted;
    let preserved = DeleteBranchOutcome::Preserved {
        reason: "not fully merged".into(),
    };
    assert!(matches!(merged, DeleteBranchOutcome::Deleted));
    assert!(matches!(preserved, DeleteBranchOutcome::Preserved { .. }));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p worktree --lib delete_branch_outcome_variants_construct`
Expected: FAIL — `DeleteBranchOutcome` not defined.

- [ ] **Step 3: Replace `remove_worktree` with a force-aware version and add `delete_branch`**

In `worktree/lib/src/worktree.rs`, replace the existing function (lines 374-378):

```rust
/// Remove a worktree by path (used in tests).
pub fn remove_worktree(path: &std::path::Path) -> Result<(), WorktreeError> {
    git_command(&["worktree", "remove", "--force", &path.display().to_string()])?;
    Ok(())
}
```

with:

```rust
/// Remove a worktree by absolute path.
///
/// When `force` is true, `git worktree remove --force` is used (drops any
/// uncommitted changes). When false, git's own safety check applies and the
/// command fails if the worktree has uncommitted changes or is locked.
pub fn remove_worktree(path: &std::path::Path, force: bool) -> Result<(), WorktreeError> {
    let path_str = path.display().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    git_command(&args)?;
    Ok(())
}

/// Outcome of a soft branch delete attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteBranchOutcome {
    /// `git branch -d <branch>` succeeded.
    Deleted,
    /// `git branch -d` refused (e.g. not merged). The branch was preserved.
    Preserved { reason: String },
}

/// Attempt a soft delete of `branch` (`git branch -d`).
///
/// Soft delete fails if the branch is not fully merged into its upstream or
/// `HEAD`; in that case we report a [`DeleteBranchOutcome::Preserved`] with
/// git's stderr as the reason rather than escalating to `-D`.
pub fn delete_branch(branch: &str) -> DeleteBranchOutcome {
    match git_command(&["branch", "-d", branch]) {
        Ok(_) => DeleteBranchOutcome::Deleted,
        Err(WorktreeError::GitCommand(reason)) => DeleteBranchOutcome::Preserved { reason },
        Err(e) => DeleteBranchOutcome::Preserved {
            reason: e.to_string(),
        },
    }
}
```

- [ ] **Step 4: Update the one existing caller in this crate**

Search for any in-crate callers of `remove_worktree`:

Run: `cargo check -p worktree`
Expected: any compile errors point to internal call sites. Update each call to pass `true` as the second arg (preserves prior behavior).

If the only call site is the function itself (no other callers in `worktree` crate), no changes needed beyond verifying compilation.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p worktree --lib`
Expected: PASS including `delete_branch_outcome_variants_construct`.

- [ ] **Step 6: Commit**

```bash
git add worktree/lib/src/worktree.rs
git commit -m "feat(worktree): force-aware remove_worktree + delete_branch helper"
```

---

## Phase 3: CLI — Subcommand & Argument Parsing

**Goal:** Wire the new `Remove` subcommand into `clap`, including tiered `-f`/`-ff` counting and `--branch`/`-b`.

### Task 3.1: Add `Remove` variant to `Commands` enum

**Files:**
- Modify: `worktree/cli/src/args.rs` (extend `Commands` enum at lines 31-56; update `AFTER_HELP` at line 65)

- [ ] **Step 1: Extend the `Commands` enum**

In `worktree/cli/src/args.rs`, inside `pub enum Commands { ... }`, add after the `Go` variant (after line 55):

```rust
    /// Remove a worktree (and optionally its branch)
    Remove {
        /// Worktree name to remove
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,

        /// Force removal. Pass twice (-ff) to skip all confirmation.
        ///
        /// - `-f` / `--force`: skip confirmation when safe (clean, or <10 files
        ///   with no source code). Confirm when uncommitted source/many files exist.
        /// - `-ff`: remove immediately regardless of state.
        #[arg(long, short = 'f', action = clap::ArgAction::Count)]
        force: u8,

        /// Also attempt a soft delete (`git branch -d`) of the worktree's branch
        #[arg(long, short = 'b')]
        branch: bool,
    },
```

- [ ] **Step 2: Update `AFTER_HELP`**

Replace the `AFTER_HELP` constant (line 65) to add the new examples. The block becomes:

```rust
const AFTER_HELP: &str = "\
Examples:
  wt                    List all worktrees (default)
  wt list               List all worktrees with status
  wt create feature/x   Create a new worktree for branch feature/x
  wt create fix/y --stay Create without changing directory
  wt go feature-x       Navigate to a worktree
  wt go base            Navigate back to the base checkout
  wt remove feature-x   Remove a worktree (prompts on dirty files)
  wt remove feature-x -f Remove (skip confirm when safe)
  wt remove feature-x -ff Remove immediately, no confirmation
  wt remove feature-x -b Remove worktree AND soft-delete its branch

Shell Integration (cd wrapper + completions):
  source <(wt --completions bash)              Add to ~/.bashrc
  source <(wt --completions zsh)               Add to ~/.zshrc
  source (wt --completions fish | psub)        Add to config.fish";
```

- [ ] **Step 3: Verify the build compiles (handler wiring still pending)**

Run: `cargo check -p worktree-cli`
Expected: FAIL with non-exhaustive match in `main.rs` (`Commands::Remove { .. }` not covered). This is expected — Task 3.2 wires the handler.

- [ ] **Step 4: Commit**

```bash
git add worktree/cli/src/args.rs
git commit -m "feat(worktree-cli): add Remove subcommand definition"
```

### Task 3.2: Wire the dispatch in `main.rs`

**Files:**
- Modify: `worktree/cli/src/main.rs` (extend `match cli.command` at lines 33-37)
- Modify: `worktree/cli/src/commands/mod.rs` (add `mod remove;` + `pub use`)
- Create: `worktree/cli/src/commands/remove.rs` (stub only — full body lands in Phase 4)

- [ ] **Step 1: Create the stub handler**

Create `worktree/cli/src/commands/remove.rs` with:

```rust
use worktree::WorktreeError;

pub fn run(name: &str, force: u8, branch: bool) -> Result<(), WorktreeError> {
    let _ = (name, force, branch);
    Err(WorktreeError::GitCommand(
        "wt remove: not yet implemented".into(),
    ))
}
```

- [ ] **Step 2: Register the module**

In `worktree/cli/src/commands/mod.rs`, after the existing `mod` lines (line 4):

```rust
mod create;
mod git_graph;
mod go;
mod list;
mod remove;

pub use create::run as create;
pub use go::run as go;
pub use list::run as list;
pub use remove::run as remove;
```

- [ ] **Step 3: Add dispatch in `main.rs`**

In `worktree/cli/src/main.rs`, replace the `match` (lines 33-37) with:

```rust
    match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(width, verbose),
        Commands::Create { branch, stay } => commands::create(&branch, stay),
        Commands::Go { name, .. } => commands::go(&name),
        Commands::Remove {
            name,
            force,
            branch,
        } => commands::remove(&name, force, branch),
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p worktree-cli`
Expected: SUCCESS, no warnings about unused variants.

- [ ] **Step 5: Verify clap registers the subcommand**

Run: `cargo run -p worktree-cli -- remove --help`
Expected: help text lists `<NAME>`, `-f, --force` (with "Pass twice (-ff) to skip..." description), and `-b, --branch`.

Run: `cargo run -p worktree-cli -- remove some-name`
Expected: error exits with stderr `Error: failed to execute git command: wt remove: not yet implemented` (proving dispatch reaches the stub).

- [ ] **Step 6: Commit**

```bash
git add worktree/cli/src/main.rs worktree/cli/src/commands/mod.rs worktree/cli/src/commands/remove.rs
git commit -m "feat(worktree-cli): wire Remove subcommand dispatch (stub handler)"
```

---

## Phase 4: CLI — Dirty File Tree Renderer

**Goal:** Build a self-contained helper that takes `Vec<PathBuf>` of repository-relative paths and renders them as a tree (using `biscuit_terminal::components::filesystem::tree_chars` + `Prose` markup) for the confirmation prompt.

### Task 4.1: Build the `dirty_tree` module

**Files:**
- Create: `worktree/cli/src/commands/dirty_tree.rs`
- Modify: `worktree/cli/src/commands/mod.rs` (add `mod dirty_tree;`)

- [ ] **Step 1: Write the module skeleton + failing tests**

Create `worktree/cli/src/commands/dirty_tree.rs`:

```rust
//! Render a list of repository-relative paths as a hierarchical tree.
//!
//! Output uses the same box-drawing characters as the biscuit-terminal
//! filesystem component (`├── `, `└── `, `│   `, four-space indent) and emits
//! Prose-flavored markup so the caller can colorize via `Prose::new(...).render(...)`.
//!
//! Source-code files (per `sniff::filesystem::path_kind::is_source_code_path`)
//! are wrapped in `<red>...</red>`; other files in `<yellow>...</yellow>`;
//! directories in `<dim>...</dim>`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::filesystem::tree_chars;

#[derive(Debug, Default)]
struct Node {
    children: BTreeMap<String, Node>,
    is_file: bool,
}

impl Node {
    fn insert(&mut self, parts: &[String]) {
        let Some((head, tail)) = parts.split_first() else {
            self.is_file = true;
            return;
        };
        self.children.entry(head.clone()).or_default().insert(tail);
    }
}

/// Build the markup for a tree-rendering of `paths`, all interpreted as
/// repository-relative entries.
pub fn render_markup(paths: &[PathBuf]) -> String {
    let mut root = Node::default();
    for path in paths {
        let parts: Vec<String> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            continue;
        }
        root.insert(&parts);
    }

    let mut out = String::new();
    render_children(&root, &mut out, "", &PathBuf::new());
    out
}

fn render_children(node: &Node, out: &mut String, prefix: &str, base: &Path) {
    let total = node.children.len();
    for (idx, (name, child)) in node.children.iter().enumerate() {
        let is_last = idx + 1 == total;
        let connector = if is_last {
            tree_chars::LAST_BRANCH
        } else {
            tree_chars::BRANCH
        };
        let child_path = base.join(name);
        let label = format_label(name, child.is_file && child.children.is_empty(), &child_path);
        out.push_str(prefix);
        out.push_str(connector);
        out.push_str(&label);
        out.push('\n');

        let next_prefix = format!(
            "{}{}",
            prefix,
            if is_last { tree_chars::INDENT } else { tree_chars::VERTICAL }
        );
        render_children(child, out, &next_prefix, &child_path);
    }
}

fn format_label(name: &str, is_file: bool, full_path: &Path) -> String {
    if !is_file {
        return format!("<dim>{name}/</dim>");
    }
    if sniff::filesystem::path_kind::is_source_code_path(full_path) {
        format!("<red>{name}</red>")
    } else {
        format!("<yellow>{name}</yellow>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_paths_render_empty() {
        assert_eq!(render_markup(&[]), "");
    }

    #[test]
    fn single_file_at_root() {
        let out = render_markup(&[PathBuf::from("README.md")]);
        assert_eq!(out, "└── <yellow>README.md</yellow>\n");
    }

    #[test]
    fn nested_files_group_by_directory() {
        let paths = vec![
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("docs/intro.md"),
        ];
        let out = render_markup(&paths);
        // docs/ comes before src/ (BTreeMap is alphabetical)
        assert!(out.contains("<dim>docs/</dim>"));
        assert!(out.contains("<dim>src/</dim>"));
        assert!(out.contains("<red>lib.rs</red>"));
        assert!(out.contains("<red>main.rs</red>"));
        assert!(out.contains("<yellow>intro.md</yellow>"));
        // The first directory at root uses BRANCH; the last uses LAST_BRANCH.
        assert!(out.starts_with("├── <dim>docs/</dim>\n"));
        assert!(out.contains("└── <dim>src/</dim>\n"));
    }

    #[test]
    fn deep_nesting_uses_vertical_continuation() {
        let paths = vec![
            PathBuf::from("a/b/c.rs"),
            PathBuf::from("a/d.rs"),
        ];
        let out = render_markup(&paths);
        // Expect a vertical continuation under `a/` for the non-last child line.
        assert!(out.contains("│   "));
    }
}
```

- [ ] **Step 2: Register the module**

In `worktree/cli/src/commands/mod.rs`, add `mod dirty_tree;` to the module list (private — only `remove` consumes it):

```rust
mod create;
mod dirty_tree;
mod git_graph;
mod go;
mod list;
mod remove;

pub use create::run as create;
pub use go::run as go;
pub use list::run as list;
pub use remove::run as remove;
```

- [ ] **Step 3: Run the new tests**

Run: `cargo test -p worktree-cli --lib dirty_tree`
Expected: PASS, 4 tests.

`worktree-cli` is a binary crate; tests live alongside the source via `#[cfg(test)]`. If `cargo test -p worktree-cli --lib` reports "no library targets", use `cargo test -p worktree-cli --bin wt dirty_tree` instead.

- [ ] **Step 4: Commit**

```bash
git add worktree/cli/src/commands/dirty_tree.rs worktree/cli/src/commands/mod.rs
git commit -m "feat(worktree-cli): add dirty_tree markup renderer for uncommitted files"
```

---

## Phase 5: CLI — Confirmation Flow & End-to-End Wiring

**Goal:** Implement the full `remove` handler: dirtiness gating, tiered force semantics, FileTree-style display, `inquire::Confirm` prompts, removal, and optional branch cleanup.

### Task 5.1: Implement the `remove` handler

**Files:**
- Modify: `worktree/cli/src/commands/remove.rs` (replace the stub from Task 3.2)

- [ ] **Step 1: Replace the stub with the full handler**

Overwrite `worktree/cli/src/commands/remove.rs` with:

```rust
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{Confirm, InquireError};
use worktree::WorktreeError;
use worktree::worktree::{
    DeleteBranchOutcome, DirtyFiles, delete_branch, find_worktree, list_dirty_files,
    remove_worktree,
};

use super::dirty_tree;

/// Threshold (per spec): below this count, a non-source-only dirtiness still
/// allows `-f` to skip the confirmation prompt.
const FORCE_BYPASS_FILE_LIMIT: usize = 10;

pub fn run(name: &str, force: u8, delete_branch_flag: bool) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();

    let entry = find_worktree(name)?;

    // Disallow removing the main checkout — git itself rejects this, but
    // surface a friendlier error before we run any prompts.
    if entry.is_main {
        let msg = format!(
            "<red><b>Error:</b></red> refusing to remove the main checkout \
            <blue>{}</blue>. Use plain <dim>git</dim> for that.",
            name
        );
        eprintln!("{}", Prose::new(msg).render(&terminal));
        return Err(WorktreeError::GitCommand(
            "cannot remove the main worktree".into(),
        ));
    }

    let dirty = list_dirty_files(&entry.path);
    let display_name = entry.branch.clone().unwrap_or_else(|| name.to_string());

    // Decide whether to prompt, based on `force` count + dirty state.
    let should_prompt = decide_prompt(force, &dirty);

    if should_prompt {
        render_dirty_summary(&terminal, &display_name, &dirty);
        let prompt_msg = build_prompt_message(&display_name, &dirty);
        if !confirm(&prompt_msg)? {
            let cancelled = format!("<dim>Cancelled. Worktree <blue>{display_name}</blue> was not removed.</dim>");
            eprintln!("{}", Prose::new(cancelled).render(&terminal));
            return Ok(());
        }
    }

    // Anything past this point removes the worktree. Force the underlying git
    // call when the user passed any -f or when dirty files exist (so git
    // doesn't reject the removal in confirmed flows).
    let force_git = force > 0 || dirty.status() != worktree::worktree::DirtyStatus::Clean;
    remove_worktree(&entry.path, force_git)?;

    let removed = format!(
        "\n<green>Removed worktree</green> <bold>{display_name}</bold> at <dim>{}</dim>",
        entry.path.display()
    );
    eprintln!("{}", Prose::new(removed).render(&terminal));

    if delete_branch_flag {
        if let Some(branch) = entry.branch.as_deref() {
            match delete_branch(branch) {
                DeleteBranchOutcome::Deleted => {
                    let msg = format!("<green>Deleted branch</green> <bold>{branch}</bold>");
                    eprintln!("{}", Prose::new(msg).render(&terminal));
                }
                DeleteBranchOutcome::Preserved { reason } => {
                    let msg = format!(
                        "<yellow><b>Warning:</b></yellow> branch <bold>{branch}</bold> was preserved: <dim>{reason}</dim>\n  \
                        <dim>Run <i>git branch -D {branch}</i> if you want to force-delete it.</dim>"
                    );
                    eprintln!("{}", Prose::new(msg).render(&terminal));
                }
            }
        } else {
            let msg = format!(
                "<yellow><b>Warning:</b></yellow> no branch associated with <bold>{display_name}</bold>; skipping branch cleanup."
            );
            eprintln!("{}", Prose::new(msg).render(&terminal));
        }
    }

    Ok(())
}

/// Decide whether to show the confirmation prompt, per the spec matrix.
///
/// `force` is the count of `-f` flags:
/// - 0 (no flag): prompt whenever dirty (any kind).
/// - 1 (`-f` / `--force`): skip prompt when clean OR (non-source AND < 10 files).
/// - 2+ (`-ff`): never prompt.
fn decide_prompt(force: u8, dirty: &DirtyFiles) -> bool {
    if force >= 2 {
        return false;
    }
    if dirty.paths.is_empty() {
        // Clean worktree: any force level skips prompt; no flag also skips.
        return false;
    }
    if force == 0 {
        return true;
    }
    // force == 1: bypass when no source files AND under the file-count threshold.
    if !dirty.has_source && dirty.paths.len() < FORCE_BYPASS_FILE_LIMIT {
        return false;
    }
    true
}

fn render_dirty_summary(terminal: &Terminal, display_name: &str, dirty: &DirtyFiles) {
    if dirty.paths.is_empty() {
        return;
    }
    let header = format!(
        "\n<b>Worktree <blue>{display_name}</blue> has {} uncommitted file(s):</b>",
        dirty.paths.len()
    );
    eprintln!("{}", Prose::new(header).render(terminal));
    let tree = dirty_tree::render_markup(&dirty.paths);
    eprint!("{}", Prose::new(tree).render(terminal));
}

fn build_prompt_message(display_name: &str, dirty: &DirtyFiles) -> String {
    let count = dirty.paths.len();
    if dirty.has_source {
        format!(
            "- the <blue>{display_name}</blue> worktree has source code files in it which \
            have not been committed to <b>git</b>! Are you sure you want to remove this \
            worktree? All file changes will be lost."
        )
    } else if count > 0 {
        format!(
            "- the <blue>{display_name}</blue> has {count} files which have not been \
            committed to <b>git</b>! Are you sure you want to remove this worktree? \
            All file changes will be lost."
        )
    } else {
        format!("- remove worktree <blue>{display_name}</blue>?")
    }
}

fn confirm(message_markup: &str) -> Result<bool, WorktreeError> {
    let terminal = Terminal::default();
    let rendered = Prose::new(message_markup.to_string()).render(&terminal);
    Confirm::new(&rendered)
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

fn map_inquire_err(e: InquireError) -> WorktreeError {
    match e {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            WorktreeError::Cancelled
        }
        e => WorktreeError::Io(std::io::Error::other(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dirty_with(paths: Vec<&str>, has_source: bool) -> DirtyFiles {
        DirtyFiles {
            paths: paths.into_iter().map(PathBuf::from).collect(),
            has_source,
        }
    }

    #[test]
    fn no_force_clean_does_not_prompt() {
        assert!(!decide_prompt(0, &DirtyFiles::default()));
    }

    #[test]
    fn no_force_dirty_prompts() {
        let d = dirty_with(vec!["README.md"], false);
        assert!(decide_prompt(0, &d));
    }

    #[test]
    fn force_one_clean_skips() {
        assert!(!decide_prompt(1, &DirtyFiles::default()));
    }

    #[test]
    fn force_one_few_non_source_skips() {
        let d = dirty_with(vec!["a.txt", "b.md"], false);
        assert!(!decide_prompt(1, &d));
    }

    #[test]
    fn force_one_many_non_source_prompts() {
        let paths: Vec<&str> = (0..FORCE_BYPASS_FILE_LIMIT)
            .map(|_| "x.txt")
            .collect();
        let d = dirty_with(paths, false);
        assert!(decide_prompt(1, &d));
    }

    #[test]
    fn force_one_source_prompts() {
        let d = dirty_with(vec!["src/lib.rs"], true);
        assert!(decide_prompt(1, &d));
    }

    #[test]
    fn force_two_never_prompts() {
        let d = dirty_with(vec!["src/lib.rs", "a.md", "b.md"], true);
        assert!(!decide_prompt(2, &d));
    }

    #[test]
    fn prompt_message_source_variant() {
        let d = dirty_with(vec!["src/lib.rs"], true);
        let msg = build_prompt_message("feat-x", &d);
        assert!(msg.contains("source code files"));
        assert!(msg.contains("<blue>feat-x</blue>"));
    }

    #[test]
    fn prompt_message_non_source_variant() {
        let d = dirty_with(vec!["a.md", "b.md"], false);
        let msg = build_prompt_message("feat-x", &d);
        assert!(msg.contains("has 2 files"));
        assert!(msg.contains("<blue>feat-x</blue>"));
    }
}
```

- [ ] **Step 2: Build the CLI**

Run: `cargo build -p worktree-cli`
Expected: SUCCESS, no warnings.

- [ ] **Step 3: Run the new unit tests**

Run: `cargo test -p worktree-cli decide_prompt`
Expected: PASS, 7 `decide_prompt_*` tests.

Run: `cargo test -p worktree-cli prompt_message`
Expected: PASS, 2 tests.

- [ ] **Step 4: Run the full workspace test for the two crates**

Run: `cargo test -p worktree -p worktree-cli`
Expected: PASS for everything (no regressions in existing list/create/go tests).

- [ ] **Step 5: Commit**

```bash
git add worktree/cli/src/commands/remove.rs
git commit -m "feat(worktree-cli): implement wt remove with confirmation matrix + branch cleanup"
```

### Task 5.2: Manual verification

**Files:** *(no source changes)*

- [ ] **Step 1: Create a throwaway worktree for testing**

```bash
cargo run -p worktree-cli -- create feat/remove-cmd-smoke --stay
```

Expected: output reports "Created worktree feat/remove-cmd-smoke at ...".

- [ ] **Step 2: Verify the clean-removal path (no prompt, no force)**

```bash
cargo run -p worktree-cli -- remove feat/remove-cmd-smoke
```

Expected: no prompt; prints "Removed worktree feat/remove-cmd-smoke at ...". The local branch `feat/remove-cmd-smoke` should still exist (`git branch --list feat/remove-cmd-smoke` returns it).

- [ ] **Step 3: Verify the dirty + prompt path**

```bash
cargo run -p worktree-cli -- create feat/remove-cmd-dirty --stay
# Inside the new worktree, create both source and non-source dirty files:
WT_DIR="$(cargo run -q -p worktree-cli -- go feat/remove-cmd-dirty | sed 's/^cd://')"
( cd "$WT_DIR" && touch dirty.txt && mkdir -p src && echo 'fn x(){}' > src/dirty.rs )
cargo run -p worktree-cli -- remove feat/remove-cmd-dirty
```

Expected:
- Hierarchical view of `dirty.txt` (yellow) and `src/dirty.rs` (red under `src/`).
- Confirmation prompt mentions "source code files" (because `src/dirty.rs` triggers source classification).
- Answer `n` -> "Cancelled..." message; worktree still present.
- Re-run and answer `y` -> "Removed worktree..." message.

- [ ] **Step 4: Verify `-ff` bypasses prompt even when dirty**

```bash
cargo run -p worktree-cli -- create feat/remove-cmd-ff --stay
WT_DIR="$(cargo run -q -p worktree-cli -- go feat/remove-cmd-ff | sed 's/^cd://')"
( cd "$WT_DIR" && echo 'fn x(){}' > src.rs )
cargo run -p worktree-cli -- remove feat/remove-cmd-ff -ff
```

Expected: no prompt; worktree removed; branch preserved.

- [ ] **Step 5: Verify `-b` branch-delete (success path)**

```bash
cargo run -p worktree-cli -- create feat/remove-cmd-branch --stay
cargo run -p worktree-cli -- remove feat/remove-cmd-branch -b
```

Expected: removal message, then either:
- "Deleted branch feat/remove-cmd-branch" (if branch is merged / has no unique commits), or
- "Warning: branch ... was preserved: ..." with a hint about `git branch -D`.

Confirm the outcome matches what `git branch --list feat/remove-cmd-branch` reports.

- [ ] **Step 6: Verify `-b` preserves an un-mergeable branch**

```bash
cargo run -p worktree-cli -- create feat/remove-cmd-unmerged --stay
WT_DIR="$(cargo run -q -p worktree-cli -- go feat/remove-cmd-unmerged | sed 's/^cd://')"
( cd "$WT_DIR" && git commit --allow-empty -m "diverge" )
cargo run -p worktree-cli -- remove feat/remove-cmd-unmerged -b
```

Expected: worktree removed; "Warning: branch feat/remove-cmd-unmerged was preserved: ..." with a `git branch -D` hint. `git branch --list feat/remove-cmd-unmerged` still returns the branch.

- [ ] **Step 7: Clean up leftover branches from manual verification**

```bash
for b in feat/remove-cmd-smoke feat/remove-cmd-ff feat/remove-cmd-branch feat/remove-cmd-unmerged; do
  git branch -D "$b" 2>/dev/null || true
done
```

- [ ] **Step 8: Commit only if any docs/notes were updated (none expected here)**

If everything works as expected, no commit is needed for this verification task. If you discovered a bug and fixed it in remove.rs / dirty_tree.rs / lib, commit with a clear `fix:` message and re-run the relevant manual verification steps.

### Task 5.3: Documentation drift

**Files:**
- Modify: `worktree/README.md` (add a `wt remove` section)

- [ ] **Step 1: Update the README**

Open `worktree/README.md` and find the section listing commands (likely near the top). Add a `wt remove` entry that documents:
- the signature: `wt remove <name> [-f | -ff] [-b]`
- the safety semantics (when prompts fire, the file-count + source-code rule)
- the branch-cleanup soft-delete behavior

Keep the prose terse — match the existing style of `wt create` / `wt go` sections. If those sections do not yet exist, add a short bullet list of available commands instead.

- [ ] **Step 2: Verify README renders cleanly**

Run: `md hash worktree/README.md`
Expected: command succeeds (only update the README's `hash:` frontmatter line if the README already has one).

- [ ] **Step 3: Final workspace verification**

Run in parallel:
- `cargo test -p worktree -p worktree-cli`
- `cargo clippy -p worktree -p worktree-cli -- -D warnings`

Expected: both PASS with no warnings.

- [ ] **Step 4: Commit**

```bash
git add worktree/README.md
git commit -m "docs(worktree): document wt remove command"
```

---

## Self-Review Notes

- **Spec coverage:**
    - CLI signature → Task 3.1.
    - Uncommitted detection via `sniff` + `repo` semantics → Task 1.1 (`DirtyFiles` uses `is_source_code_path`).
    - `FileTree`-style hierarchical view → Task 4.1 (`dirty_tree` renders box-drawing tree using `tree_chars` from biscuit-terminal; the spec's "FileTree" wording is honored by using biscuit-terminal's tree-drawing primitives, since darkmatter's `FileTree` is Markdown-reference-specific and the biscuit-terminal `FileSystem` component walks real directories rather than an arbitrary path list).
    - Safety dialogs → Task 5.1 (`render_dirty_summary` + `confirm`).
    - `--force` / `-f` tiered semantics (clean → skip; < 10 non-source → skip; else prompt) → Task 5.1 (`decide_prompt`).
    - `-ff` immediate removal → Task 5.1 (`force >= 2` branch).
    - Confirmation message wording (both source and non-source variants) → Task 5.1 (`build_prompt_message`).
    - `--branch` / `-b` soft delete via `git branch -d` with warning on failure → Tasks 2.1 (`delete_branch`) + 5.1 (handler).

- **Type / signature consistency:** `remove_worktree(path, force: bool)`, `list_dirty_files(path) -> DirtyFiles`, `delete_branch(branch) -> DeleteBranchOutcome`, `Commands::Remove { name: String, force: u8, branch: bool }` — all referenced consistently across Phases 2, 3, 5.

- **No placeholders:** every step has either runnable code or an exact command + expected output.
