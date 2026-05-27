---
last_updated: 2026-03-23
sources:
  - "worktree/README.md"
---

# Worktree -- Implementation Plan

## Context

Worktree (`wt`) is a CLI that simplifies working with git worktrees. It provides three core commands: `list` (default), `create <branch>`, and `go <worktree|base>`. The implementation is split into a library (business logic, git orchestration, directory resolution) and a CLI (argument parsing, terminal rendering via biscuit-terminal).

---

## Phase 1: Library Foundation -- Error Types, Config, and Git Detection

**Goal**: Establish the library's error handling, configuration resolution (`WT` env / `~/.worktree.json`), and git detection (presence of `git`, repo name, relative path).

### Step 1.1: Define error types (`lib/src/error.rs`)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("git is not installed or not found on PATH")]
    GitNotFound,

    #[error("not inside a git repository")]
    NotInGitRepo,

    #[error("base directory not configured: set the WT environment variable or create ~/.worktree.json")]
    BaseDirectoryNotConfigured,

    #[error("base directory '{0}' does not exist")]
    BaseDirectoryNotFound(String),

    #[error("base directory '{0}' is itself a git repository -- it must be a plain directory")]
    BaseDirectoryIsGitRepo(String),

    #[error("worktree '{0}' already exists")]
    WorktreeAlreadyExists(String),

    #[error("worktree '{0}' not found")]
    WorktreeNotFound(String),

    #[error("failed to execute git command: {0}")]
    GitCommand(String),

    #[error("failed to parse git output: {0}")]
    GitParse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
```

### Step 1.2: Configuration resolution (`lib/src/config.rs`)

Resolve the base directory for worktree storage:

1. Check `WT` environment variable
2. Fall back to `~/.worktree.json` (`{ "base_dir": "/path/to/dir" }`)
3. Validate: directory must exist and must NOT be a git repo

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorktreeConfig {
    pub base_dir: String,
}

/// Resolves the base directory from WT env var or ~/.worktree.json
pub fn resolve_base_dir() -> Result<PathBuf, WorktreeError> { ... }
```

### Step 1.3: Git detection (`lib/src/git.rs`)

Detect git binary on PATH, determine if CWD is inside a git repo, and extract repo metadata:

```rust
pub struct RepoInfo {
    /// Name of the repository (directory name of the root)
    pub name: String,
    /// Absolute path to the repository root
    pub root: PathBuf,
    /// Relative path from repo root to CWD
    pub relative_path: PathBuf,
}

/// Verify git is installed
pub fn ensure_git() -> Result<(), WorktreeError> { ... }

/// Get repository info for the current working directory
pub fn repo_info() -> Result<RepoInfo, WorktreeError> { ... }
```

Use `which` crate for git detection and `std::process::Command` for `git rev-parse` calls.

### Step 1.4: Wire up `lib/src/lib.rs`

```rust
pub mod config;
pub mod error;
pub mod git;

pub use error::WorktreeError;
```

### Step 1.5: Add dependencies to `lib/Cargo.toml`

```toml
[dependencies]
dirs = "6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
which = "8.0"
```

### Step 1.6: Tests

- `config::resolve_base_dir` -- test WT env override, JSON file fallback, missing config error, invalid directory error, directory-is-git-repo error
- `git::ensure_git` -- test success (git exists on CI/dev machines)
- `git::repo_info` -- test from within this monorepo (name = `rusty-biscuit`, relative_path includes `worktree/`)

---

## Phase 2: `wt list` -- Worktree Listing

**Goal**: Implement the default command that lists all worktrees with status indicators.

### Step 2.1: Git worktree parsing (`lib/src/worktree.rs`)

Parse `git worktree list --porcelain` output:

```rust
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// Branch name (or HEAD commit if detached)
    pub branch: Option<String>,
    /// Whether this is the main/base checkout
    pub is_main: bool,
    /// Whether the worktree is the one the user is currently in
    pub is_current: bool,
}
```

### Step 2.2: Status detection (`lib/src/status.rs`)

For each worktree, determine:

- **Clean vs Conflict**: Can the worktree branch be merged into main/master without conflicts? Use `git merge-tree --write-tree` (available since git 2.38) or `git merge --no-commit --no-ff` dry-run approach.
- **Ahead/Behind**: Use `git rev-list --left-right --count main...branch` to get commit counts.

```rust
#[derive(Debug, Clone)]
pub struct WorktreeStatus {
    pub entry: WorktreeEntry,
    /// Whether the branch can merge cleanly into main
    pub is_clean: bool,
    /// Commits ahead of main
    pub ahead: usize,
    /// Commits behind main
    pub behind: usize,
}

/// Get status for all worktrees
pub fn list_worktrees() -> Result<Vec<WorktreeStatus>, WorktreeError> { ... }

/// Detect the default branch name (main or master)
pub fn default_branch() -> Result<String, WorktreeError> { ... }
```

### Step 2.3: Tests

- Parse porcelain output from `git worktree list --porcelain` (use captured fixtures)
- Ahead/behind parsing
- Default branch detection

---

## Phase 3: `wt create <branch>` -- Worktree Creation

**Goal**: Create a new worktree with automatic directory placement and optional CWD change.

### Step 3.1: Branch name dasherization (`lib/src/util.rs`)

Convert branch names to filesystem-safe directory names:

- `feature/my-thing` → `feature-my-thing`
- `fix/JIRA-123/description` → `fix-jira-123-description`

```rust
/// Convert a branch name to a dasherized directory name
pub fn dasherize(branch: &str) -> String { ... }
```

### Step 3.2: Create worktree (`lib/src/worktree.rs`)

```rust
pub struct CreateResult {
    /// Path to the new worktree
    pub worktree_path: PathBuf,
    /// Path the user should cd to (preserving relative position)
    pub target_cwd: PathBuf,
    /// Branch name
    pub branch: String,
}

/// Create a new worktree at {base_dir}/{repo_name}/{dasherized_branch}/
pub fn create_worktree(branch: &str) -> Result<CreateResult, WorktreeError> { ... }
```

The implementation:
1. Call `resolve_base_dir()` to get base
2. Call `repo_info()` to get repo name and relative path
3. Compute target path: `{base}/{repo_name}/{dasherize(branch)}/`
4. Execute `git worktree add {target_path} -b {branch}` (or without `-b` if branch exists)
5. Compute `target_cwd` = `{target_path}/{relative_path}`
6. Return `CreateResult`

### Step 3.3: Tests

- `dasherize` -- various branch naming patterns
- `create_worktree` -- integration test using a temp git repo (via `tempfile`)

---

## Phase 4: `wt go <name>` -- Worktree Navigation

**Goal**: Navigate to a named worktree or back to the base checkout.

### Step 4.1: Resolve worktree by name (`lib/src/worktree.rs`)

```rust
/// Find a worktree by name (matches against branch name or dasherized directory name)
pub fn find_worktree(name: &str) -> Result<WorktreeEntry, WorktreeError> { ... }
```

Match against:
- The branch name
- The dasherized directory name
- The literal string `"base"` for the main checkout

### Step 4.2: Output the target directory

Since a subprocess cannot change the parent shell's CWD, the CLI will print the path and rely on a shell function/alias to `cd` to it. The standard pattern:

```bash
# Shell function (installed by user)
wt() {
    local result
    result="$(command wt "$@")" || return $?
    if [[ "$result" == "cd:"* ]]; then
        cd "${result#cd:}"
    else
        echo "$result"
    fi
}
```

The library returns the path; the CLI prefixes it with `cd:` when a directory change is needed.

### Step 4.3: Tests

- `find_worktree` -- match by branch, by directory name, by "base"
- Error on not found

---

## Phase 5: CLI Implementation

**Goal**: Build the `wt` binary with clap, shell completions, and biscuit-terminal rendering.

### Step 5.1: Update `cli/Cargo.toml`

```toml
[package]
name = "worktree-cli"
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-only"

[[bin]]
name = "wt"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive", "wrap_help"] }
clap_complete = { version = "4", features = ["unstable-dynamic"] }
biscuit-terminal = { path = "../../biscuit-terminal/lib" }
darkmatter = { path = "../../darkmatter/lib" }
worktree = { path = "../lib" }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

### Step 5.2: Argument parsing (`cli/src/args.rs`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "wt",
    version,
    about = "A simple CLI for working with git worktrees",
    disable_help_subcommand = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all worktrees with status indicators
    List,

    /// Create a new worktree from a branch name
    Create {
        /// Branch name for the new worktree
        branch: String,

        /// Create the worktree but don't change into it
        #[arg(long)]
        stay: bool,
    },

    /// Navigate to a worktree or the base checkout
    Go {
        /// Worktree name or "base" for the main checkout
        name: String,
    },
}
```

When `command` is `None`, default to `Commands::List`.

### Step 5.3: Command handlers (`cli/src/commands.rs`)

```rust
mod list;
mod create;
mod go;

pub use list::run as list;
pub use create::run as create;
pub use go::run as go;
```

Each handler calls the library, then renders output using biscuit-terminal components:

- **list**: Use `UnorderedList` or a custom layout showing worktree name, branch, clean/conflict indicator, ahead/behind counts. Highlight the current worktree.
- **create**: Show success message with the new worktree path. Output `cd:{path}` when `--stay` is not set.
- **go**: Output `cd:{path}` for the target worktree.

### Step 5.4: Entry point (`cli/src/main.rs`)

```rust
mod args;
mod commands;

use args::{Cli, Commands};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(),
        Commands::Create { branch, stay } => commands::create(&branch, stay),
        Commands::Go { name } => commands::go(&name),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

### Step 5.5: Shell completions

Use `clap_complete` with dynamic completions for the `go` subcommand. The completer should list available worktree names by calling the library's `list_worktrees()`.

### Step 5.6: Tests

- CLI integration tests with `assert_cmd`:
  - `wt --help` exits 0, contains expected text
  - `wt list` runs without error (when inside a git repo)
  - `wt create` with missing branch arg shows error
  - `wt go` with missing name arg shows error

---

## Phase 6: Shell Integration and Documentation

**Goal**: Provide shell wrapper functions and finalize documentation.

### Step 6.1: Shell wrapper

Create `worktree/shell/wt.sh` (bash/zsh) and optionally `wt.fish`:

```bash
# Add to ~/.bashrc or ~/.zshrc
wt() {
    local output
    output="$(command wt "$@")" || { echo "$output"; return $?; }
    if [[ "$output" == "cd:"* ]]; then
        builtin cd "${output#cd:}" || return $?
    else
        [[ -n "$output" ]] && echo "$output"
    fi
}
```

### Step 6.2: Library README (`lib/README.md`)

Document the public API, error types, configuration, and usage examples.

### Step 6.3: CLI README (`cli/README.md`)

Document all commands, flags, installation, and shell integration setup.

### Step 6.4: `just.md`

Create a one-liner help file for the justfile default recipe.

---

## Dependency Summary

### Library (`worktree`)

| Crate | Purpose |
|-------|---------|
| `thiserror` | Error type derivation |
| `serde` + `serde_json` | Parse `~/.worktree.json` config |
| `which` | Detect `git` on PATH |
| `dirs` | Resolve `~` in config paths |

### CLI (`worktree-cli`)

| Crate | Purpose |
|-------|---------|
| `clap` + `clap_complete` | Argument parsing and shell completions |
| `biscuit-terminal` | Terminal rendering components |
| `darkmatter` | Markdown rendering (help text) |
| `worktree` | Core library |

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `assert_cmd` + `predicates` | CLI integration tests |
| `tempfile` | Temporary git repos for integration tests |

---

## Implementation Order

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6
 config      list       create       go          CLI         docs
 git det.    status     dasherize    resolve     rendering   shell fn
 errors      parsing    placement    navigate    completions README
```

Phases 1-4 are library-only. Phase 5 builds the CLI on top. Phase 6 is documentation and polish. Each phase should be testable and committable independently.

---

## Design Decisions

1. **Shell command execution over `git2`**: The README specifies using the host's git program via shell commands. This keeps the binary small and avoids `libgit2` linking complexity. The `which` crate detects git presence.

2. **`cd:` protocol for directory changes**: Since a child process cannot change the parent shell's CWD, the CLI outputs `cd:{path}` which a shell wrapper function intercepts. This is the standard pattern used by tools like `zoxide`.

3. **Synchronous execution**: No `tokio` needed -- all operations are local git commands that complete quickly. This keeps the dependency tree lean.

4. **`list` as default command**: Using `Option<Commands>` in clap and defaulting to `List` when `None`.
