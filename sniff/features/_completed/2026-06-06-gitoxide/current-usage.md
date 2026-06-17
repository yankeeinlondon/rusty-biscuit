# `git2` Current-Usage Inventory — sniff package area

**Date:** 2026-06-06
**Scope:** `sniff/lib` and `sniff/cli` (production source only; `#[cfg(test)]` and
`tests/`/`benches/` usage is catalogued separately at the end and excluded from the
migration-relevant counts).
**Purpose:** Factual inventory of every `git2` (libgit2 bindings) API used across the
sniff package area, to feed a planned migration to the pure-Rust `gitoxide` (`gix`)
crate. Sniff's interest in git is read-only with one exception (an out-of-process
`git fetch`, flagged below). This document does not propose or critique the migration —
it records what exists.

---

## 1. Summary Table — git2 API surface → call sites → conceptual operation

Counts are **production call sites** (non-test). File:line anchors for each appear in the
per-module sections. Where a method is wrapped in a closure-based accessor chain
(`.map_err(...).ok()`), the underlying git2 call is counted once.

| git2 type / method | Prod call sites | Conceptual git operation | R/W |
|---|---|---|---|
| `git2::Repository::discover` | 16 | `git rev-parse --show-toplevel` (repo discovery, walks parents) | R |
| `git2::Repository::open` | 4 | open a known repo / worktree path | R |
| `git2::Repository::init` | 0 (test-only) | `git init` | W (tests) |
| `Repository::workdir` | 14 | resolve working-tree root | R |
| `Repository::path` | 1 | `.git` dir path | R |
| `Repository::commondir` | 3 | shared `.git` of a worktree | R |
| `Repository::is_worktree` | 6 | detect linked worktree | R |
| `Repository::head` | 14 | resolve `HEAD` ref | R |
| `Repository::head_detached` | 1 | detached-HEAD check | R |
| `Repository::set_head_detached` | 0 (test-only) | detach HEAD | W (tests) |
| `Repository::find_reference` | 3 | look up a ref by full name | R |
| `Repository::references` | 1 | enumerate all refs | R |
| `Repository::references_glob` | 3 | enumerate refs by glob | R |
| `Repository::find_branch` | 3 | look up a branch ref | R |
| `Repository::branches` | 1 | `git branch` (enumerate local) | R |
| `Repository::find_commit` | 8 | object lookup → commit | R |
| `Repository::revwalk` | 6 | `git log` / `git rev-list` walker | R |
| `Repository::revparse_single` | 3 | `git rev-parse <spec>` | R |
| `Repository::diff_tree_to_tree` | 2 | `git diff <treeA> <treeB>` | R |
| `Repository::diff_tree_to_index` | 1 | `git diff --cached` | R |
| `Repository::diff_index_to_workdir` | 1 | `git diff` (worktree) | R |
| `Repository::statuses` | 4 | `git status` | R |
| `Repository::index` | 1 | read `.git/index` | R |
| `Repository::config` | 1 | `git config` (merged view) | R |
| `Repository::remotes` | 5 | `git remote` (list names) | R |
| `Repository::find_remote` | 6 | load a remote's config | R |
| `Repository::graph_ahead_behind` | 3 | `git rev-list --left-right --count` | R |
| `Repository::graph_descendant_of` | 2 | ancestry test | R |
| `Repository::merge_commits` | 1 | in-memory 3-way merge (conflict probe) | R (no writes) |
| `Repository::worktrees` | 2 | list linked worktree names | R |
| `Repository::find_worktree` | 2 | load a worktree handle | R |
| `Repository::reference` | 0 (test-only) | create a ref | W (tests) |
| `Repository::commit` | 0 (test-only) | `git commit` | W (tests) |
| `Repository::worktree` (create) | 0 (test-only) | `git worktree add` | W (tests) |
| `Reference` (`name`/`shorthand`/`is_branch`/`target`/`symbolic_target`/`peel_to_commit`/`peel_to_tree`/`get`/`into_reference`) | many | ref inspection / peeling | R |
| `Branch` (`name`/`upstream`/`get`/`into_reference`) | several | branch inspection | R |
| `Commit` (`id`/`author`/`message`/`time`/`tree`/`parent`/`as_object`) | many | commit field reads | R |
| `Signature` (`name`) via `commit.author()` | several | author read | R |
| `git2::Oid` (`from_str`, `to_string`, `Eq`) + `HashMap<Oid, _>` keys | many | object id handling/caching | R |
| `git2::Sort` (`TIME`, `TOPOLOGICAL`) | 4 | revwalk sort order | R |
| `Revwalk` (`push`, `push_head`, `hide`, `set_sorting`, `take`, `count`, `Iterator`) | 6 walks | history traversal config | R |
| `git2::Diff::print` + `git2::DiffFormat::Patch` | 1 | patch text emission | R |
| `git2::Diff::deltas` | 2 | per-file delta enumeration | R |
| `DiffDelta` (`new_file`/`old_file`/`status`) + `DiffFile::path` | several | delta path/kind reads | R |
| `git2::Delta` enum (`Added`/`Deleted`/`Renamed`/`Copied`) | 1 site (5 variants) | map delta → `DeltaKind` | R |
| `DiffLine` (`origin`/`content`) | 1 | line-level diff text | R |
| `git2::StatusOptions` (`new`/`include_untracked`/`recurse_untracked_dirs`) | 4 | configure status walk | R |
| `Statuses`/`StatusEntry` (`iter`/`status`/`path`/`is_empty`/`count`) | 4 | status results | R |
| `git2::Status` flag methods (`is_index_new`/`is_index_modified`/`is_index_deleted`/`is_wt_modified`/`is_wt_deleted`/`is_wt_new`) | 4 sites | status bit checks | R |
| `Index::conflicts` + `IndexConflict`/`IndexEntry::path` | 1 | unmerged-file detection | R |
| `Config` (`get_string`/`get_bool`/`add_file`) + `git2::ConfigLevel::ProgramData` | 1 | read git config keys | R (add_file is in-memory) |
| `Remote::url` | 6 | remote URL read | R |
| `git2::Error` | type | error variant wrapped by `SniffError::Git` | — |

**Distinct production git2 surface (types/methods):** ~60 distinct items across **~73
production call sites**, plus the `git2::Error` `#[from]` conversion in `error.rs`.

**Write / non-read-only operations flagged:** exactly **one** in production code — an
*out-of-process* `git fetch` via `std::process::Command` (not a libgit2 call), which
mutates the object DB and remote-tracking refs. See [§7](#7-remotefetch--remote_refreshrs).
`Repository::merge_commits` ([§6](#6-worktrees)) performs an **in-memory** 3-way merge
(no repo writes). All `Repository::init`/`commit`/`reference`/`worktree`/`set_head_detached`
calls are confined to `#[cfg(test)]` modules and test crates.

---

## 2. Dependency Declaration

| Manifest | Line | Declaration | Features |
|---|---|---|---|
| `sniff/lib/Cargo.toml` | [`17`](../../lib/Cargo.toml#L17) | `git2 = "0.20.3"` | **default features** (no explicit `features`/`default-features`) |
| `sniff/cli/Cargo.toml` | [`28`](../../cli/Cargo.toml#L28) | `git2 = "0.20"` | default features (runtime dep) |
| `sniff/cli/Cargo.toml` | [`44`](../../cli/Cargo.toml#L44) | `git2 = "0.20"` | default features (`[dev-dependencies]`, for `tests/cli.rs`) |

Notes:
- No feature flags are enabled. Per `git2.md`, **0.20** still ships `ssh`/`https`/`cred`
  as default features (the de-defaulting happened in **0.21**). Sniff does **not** use
  libgit2 networking (no `Remote::fetch`/`connect`/`Cred`), so these defaults are unused
  at the call-site level even though they are linked.
- `git2 0.20.3` corresponds to `libgit2-sys` tracking **libgit2 1.9.2** (per the version
  history in `git2.md`).
- The CLI pins `0.20` (caret) while the lib pins `0.20.3`; Cargo unifies to a single
  resolved version in the workspace.

---

## 3. Error Handling — how `git2::Error` flows through sniff

**Single conversion point:** [`lib/src/error.rs:15`](../../lib/src/error.rs#L15)

```rust
/// Git operation failed.
#[error("Git error: {0}")]
Git(#[from] git2::Error),
```

- `SniffError` (a `thiserror` enum) owns a `Git(git2::Error)` variant with a `#[from]`,
  so any `git2::Result<T>` propagates into `crate::Result<T>` via `?`.
- `crate::Result<T> = std::result::Result<T, SniffError>` ([`error.rs:167`](../../lib/src/error.rs#L167)).

**Where the `#[from]` is actually exercised (production):**
- `lib/src/filesystem/git/status.rs` — `repo.statuses(...)?` ([`status.rs:41`](../../lib/src/filesystem/git/status.rs#L41)), and `aggregate_diff(...)?` which propagates `diff.print(...)?`.
- `lib/src/filesystem/git/diff.rs` — `diff.print(...)?` ([`diff.rs:33`](../../lib/src/filesystem/git/diff.rs#L33)) returns `crate::Result<()>`, so the `git2::Error` from `print` becomes `SniffError::Git`.
- `lib/src/filesystem/blast_radius.rs` — `repo.statuses(...)?` ([`blast_radius.rs:175`](../../lib/src/filesystem/blast_radius.rs#L175)).
- `lib/src/filesystem/git/recent_commits.rs` — explicit `.map_err(SniffError::Git)` / `SniffError::Git(e)` at [`recent_commits.rs:285`](../../lib/src/filesystem/git/recent_commits.rs#L285), [`289`](../../lib/src/filesystem/git/recent_commits.rs#L289), [`296`](../../lib/src/filesystem/git/recent_commits.rs#L296) (revparse, peel, HEAD peel).

**Where `git2::Error` is deliberately swallowed (not surfaced as `SniffError`):**
A pervasive pattern across the git modules converts `git2::Error` to `Option`/empty
collections after a `tracing::debug!`. Examples:
- `remote_refresh.rs` returns `Result<usize, git2::Error>` internally from
  `push_relevant_ahead` ([`remote_refresh.rs:234`](../../lib/src/filesystem/git/remote_refresh.rs#L234)) but the caller logs and `continue`s; it never reaches `SniffError`.
- `discovery.rs`, `types.rs`, `worktree.rs` use `.map_err(|e| { debug!(...); e }).ok()`
  to downgrade errors to `None`. These are read-only lookups where "absent" is a valid
  result (no HEAD, detached HEAD, missing upstream, etc.).
- `worktree.rs` public functions return `Result<_, Box<dyn Error>>` and map
  `Repository::discover` failure to `Ok(None)` rather than an error ([`worktree.rs:47-50`](../../lib/src/filesystem/git/worktree.rs#L47)).

**CLI layer:** `cli/src/commands/*` does **not** use `SniffError::Git`. It converts
`Repository::discover` failures directly into `Box<dyn std::error::Error>` strings via
`.map_err(|e| format!("Not a git repository: {}", e))` (e.g.
[`mod.rs:193`](../../cli/src/commands/mod.rs#L193), [`repo.rs:46`](../../cli/src/commands/repo.rs#L46), [`remote.rs:73`](../../cli/src/commands/remote.rs#L73)).

---

## 4. Discovery & Repo Handle

### 4.1 `GitRepo` wrapper — `lib/src/filesystem/git/types.rs`

The central read-only handle. Wraps a `git2::Repository` plus cached ref decorations and
config.

| Item | file:line | git2 surface | Operation | R/W |
|---|---|---|---|---|
| struct field `repo: Repository` | [`types.rs:477`](../../lib/src/filesystem/git/types.rs#L477) | `git2::Repository` | owned handle | R |
| field `ref_decorations: RefCell<Option<HashMap<git2::Oid, …>>>` | [`types.rs:480`](../../lib/src/filesystem/git/types.rs#L480) | `git2::Oid` as HashMap key | **containment/decoration cache** | R |
| `ref_decorations()` return type | [`types.rs:497`](../../lib/src/filesystem/git/types.rs#L497) | `git2::Oid` | cached map accessor | R |
| `GitRepo::discover` | [`types.rs:513`](../../lib/src/filesystem/git/types.rs#L513) | `Repository::discover` | repo discovery | R |
| ↳ `repo.workdir()` | [`types.rs:521`](../../lib/src/filesystem/git/types.rs#L521) | `Repository::workdir` | resolve root | R |
| `current_branch()` | [`types.rs:539`](../../lib/src/filesystem/git/types.rs#L539) | `Repository::head` → `Reference::shorthand` | current branch | R |
| `in_worktree()` | [`types.rs:553`](../../lib/src/filesystem/git/types.rs#L553) | `Repository::is_worktree` | worktree check | R |
| `base_repo_root()` | [`types.rs:558-559`](../../lib/src/filesystem/git/types.rs#L558) | `is_worktree` + `commondir().parent()` | base repo root | R |
| `detect_with_request()` `is_worktree` | [`types.rs:756`](../../lib/src/filesystem/git/types.rs#L756) | `Repository::is_worktree` | populate `GitInfo.in_worktree` | R |

`detect_with_request` ([`types.rs:654`](../../lib/src/filesystem/git/types.rs#L654)) is the
fan-out hub. It conditionally calls (all delegating to other modules):
`refresh_remote_tracking_refs` (write — see §7), `get_recent_commits`, status counts/changes,
`get_remotes`, `get_worktrees`, `get_git_config`, `get_local_branches`,
`get_tracking_status`, `populate_recent_commit_remotes`. The gating note at
[`types.rs:697-702`](../../lib/src/filesystem/git/types.rs#L697) is the documented
performance hook: `wants_repo_metadata()` gates the per-branch `graph_ahead_behind` walk so
a `summary().include_file_changes` request skips it.

### 4.2 Other discovery sites (return `workdir`)

| file:line | Function | git2 surface |
|---|---|---|
| [`lib/src/filesystem/mod.rs:374`](../../lib/src/filesystem/mod.rs#L374) | `discover_repo_root` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/docs.rs:111`](../../lib/src/filesystem/docs.rs#L111) | `RepoDocuments::new` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/docs.rs:270`](../../lib/src/filesystem/docs.rs#L270) | `detect_blast_radius_docs` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/just.rs:111-112`](../../lib/src/filesystem/just.rs#L111) | `find_scope_root` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/repo/identity.rs:72`](../../lib/src/filesystem/repo/identity.rs#L72) | `detect_repo_identity` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/blast_radius.rs:73`](../../lib/src/filesystem/blast_radius.rs#L73) | `collect_changed_paths` | `Repository::discover` + `workdir` |
| [`lib/src/filesystem/git/recent_commits.rs:202`](../../lib/src/filesystem/git/recent_commits.rs#L202), [`230`](../../lib/src/filesystem/git/recent_commits.rs#L230), [`251`](../../lib/src/filesystem/git/recent_commits.rs#L251), [`275`](../../lib/src/filesystem/git/recent_commits.rs#L275), [`335`](../../lib/src/filesystem/git/recent_commits.rs#L335) | 5 `get_recent_commits_*` entry points | `Repository::discover` + `workdir` |

CLI discovery sites are listed in [§9](#9-cli-consumers).

---

## 5. Status, Diff, and Conflict Detection

### 5.1 Status — `lib/src/filesystem/git/status.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`status.rs:7`](../../lib/src/filesystem/git/status.rs#L7) | import | `Repository`, `StatusOptions` | — | — |
| [`status.rs:36-39`](../../lib/src/filesystem/git/status.rs#L36) | `get_repo_status_with_changes` | `StatusOptions::new` + `include_untracked(true)` + `recurse_untracked_dirs(true)` | configure `git status` | R |
| [`status.rs:41`](../../lib/src/filesystem/git/status.rs#L41) | same | `repo.statuses(Some(&mut opts))?` | `git status` walk | R |
| [`status.rs:43`,`58`](../../lib/src/filesystem/git/status.rs#L43) | same | `Statuses::is_empty` / `iter().count()` | result inspection | R |
| [`status.rs:60`](../../lib/src/filesystem/git/status.rs#L60) | same | `repo.head().and_then(peel_to_tree)` | HEAD tree | R |
| [`status.rs:62-64`](../../lib/src/filesystem/git/status.rs#L62) | same | `repo.diff_tree_to_index(Some(tree), None, None)` | `git diff --cached` | R |
| [`status.rs:65`](../../lib/src/filesystem/git/status.rs#L65) | same | `repo.diff_index_to_workdir(None, None)` | `git diff` (worktree) | R |
| [`status.rs:89-95`](../../lib/src/filesystem/git/status.rs#L89) | same | `StatusEntry::status` / `path`; `Status::is_index_new`/`is_index_modified`/`is_index_deleted`/`is_wt_modified`/`is_wt_deleted`/`is_wt_new` | status bit decode | R |
| [`status.rs:196`](../../lib/src/filesystem/git/status.rs#L196) | same | `get_commit_refs(repo)` (HEAD + upstream SHAs) | ref read | R |
| [`status.rs:199`](../../lib/src/filesystem/git/status.rs#L199) | same | `repo.workdir()` | absolute paths | R |
| [`status.rs:308-316`](../../lib/src/filesystem/git/status.rs#L308) | `get_repo_status_counts` | `StatusOptions` (same flags) + `repo.statuses(...)` | counts-only `git status` | R |
| [`status.rs:340-348`](../../lib/src/filesystem/git/status.rs#L340) | `get_repo_status_counts_detailed` | `StatusOptions` + `repo.statuses(...)` | per-category counts | R |
| [`status.rs:377`](../../lib/src/filesystem/git/status.rs#L377) | `detect_merge_conflicts` | `repo.index()` | read `.git/index` | R |
| [`status.rs:382`](../../lib/src/filesystem/git/status.rs#L382) | same | `Index::conflicts()` | unmerged entries | R |
| [`status.rs:392-398`](../../lib/src/filesystem/git/status.rs#L392) | same | `IndexConflict::{our,their,ancestor}` + `IndexEntry::path` | conflict path read | R |

**Status layers (perf model, from the sniff skill):** three selectable depths —
`get_repo_status_counts` (bool + total), `get_repo_status_counts_detailed`
(staged/unstaged/untracked), and `get_repo_status_with_changes` (full `FileChange` list,
optionally with unified diffs). All three share the same `StatusOptions`
(`include_untracked + recurse_untracked_dirs`); rename detection is **off** (the default),
which the tests at [`status.rs:598`](../../lib/src/filesystem/git/status.rs#L598) assert
(rename → delete+add pair).

### 5.2 Diff aggregation — `lib/src/filesystem/git/diff.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`diff.rs:28-33`](../../lib/src/filesystem/git/diff.rs#L28) | `aggregate_diff` | `&git2::Diff`; `diff.print(git2::DiffFormat::Patch, closure)` | single-pass patch walk | R |
| [`diff.rs:34-37`](../../lib/src/filesystem/git/diff.rs#L34) | closure | `DiffDelta::new_file().path()` / `old_file().path()` | per-line path attribution | R |
| [`diff.rs:43`,`53`](../../lib/src/filesystem/git/diff.rs#L43) | closure | `DiffLine::origin` | `+`/`-`/` ` classification | R |
| [`diff.rs:56`](../../lib/src/filesystem/git/diff.rs#L56) | closure | `DiffLine::content` | patch text bytes | R |

**Perf note (documented in source):** the module-level doc comment
([`diff.rs:3-6`](../../lib/src/filesystem/git/diff.rs#L3)) and `status.rs` doc
([`status.rs:201-203`](../../lib/src/filesystem/git/status.rs#L201)) record that the
two repo-wide diffs (`diff_tree_to_index` + `diff_index_to_workdir`) are each walked
**exactly once** via `aggregate_diff`, replacing an earlier
`O(dirty_files × diff_setup_cost)` per-file-pathspec loop. `DiffFormat::Patch` with a
closure is the single API that drives both line-stat accumulation and per-file patch text.

### 5.3 Commit-diff (changed-files) — `lib/src/filesystem/git/discovery.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`discovery.rs:325-331`](../../lib/src/filesystem/git/discovery.rs#L325) | `DeltaKind::from_delta` | `git2::Delta::{Added,Deleted,Renamed,Copied}` (+ `_` → Modified) | delta → enum | R |
| [`discovery.rs:391`](../../lib/src/filesystem/git/discovery.rs#L391) | `get_commit_files` | `git2::Oid::from_str` | parse SHA | R |
| [`discovery.rs:394`](../../lib/src/filesystem/git/discovery.rs#L394) | same | `repo.find_commit(oid)` | commit lookup | R |
| [`discovery.rs:397`](../../lib/src/filesystem/git/discovery.rs#L397) | same | `Commit::tree` | commit tree | R |
| [`discovery.rs:401-409`](../../lib/src/filesystem/git/discovery.rs#L401) | same | `Commit::parent(0)` → `Commit::tree` | first-parent tree | R |
| [`discovery.rs:417`](../../lib/src/filesystem/git/discovery.rs#L417) | same | `repo.diff_tree_to_tree(parent, Some(tree), None)` | `git diff <parent> <commit>` | R |
| [`discovery.rs:428-435`](../../lib/src/filesystem/git/discovery.rs#L428) | same | `Diff::deltas` + `DiffDelta::new_file().path()` + `DiffDelta::status` | per-file delta enumeration | R |
| [`discovery.rs:498-513`](../../lib/src/filesystem/git/discovery.rs#L498) | `get_commits_for_path_with_decorations` | `diff_tree_to_tree` + `Diff::deltas` + `new_file/old_file().path()` | path-touch test per commit | R |

---

## 6. Log / Recent-Commits, Refs, Branches, Worktrees

### 6.1 Revwalk / log — `discovery.rs` + `recent_commits.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`discovery.rs:132-136`](../../lib/src/filesystem/git/discovery.rs#L132) | `get_recent_commits_with_decorations` | `repo.revwalk()` + `Revwalk::push_head` | `git log` from HEAD | R |
| [`discovery.rs:144-160`](../../lib/src/filesystem/git/discovery.rs#L144) | same | `Revwalk::take`; `find_commit`; `Commit::{id,message,author,time}`; `Signature::name` | per-commit reads | R |
| [`discovery.rs:459-462`](../../lib/src/filesystem/git/discovery.rs#L459) | `get_commits_for_path_with_decorations` | `revwalk()` + `push_head` + `Iterator` | path-filtered log | R |
| [`discovery.rs:558-577`](../../lib/src/filesystem/git/discovery.rs#L558) | `get_commits_for_branch` | `find_branch` → `Branch::into_reference().target()`, fallback `revparse_single` → `peel_to_commit`; `revwalk()` + `Revwalk::push(oid)` + `take` | `git log <branch>` | R |
| [`recent_commits.rs:368-374`](../../lib/src/filesystem/git/recent_commits.rs#L368) | `collect_commits_in_range` | `revwalk()`; `Revwalk::set_sorting(git2::Sort::TIME)`; `push_head`; `Iterator` | time-bounded `git log` | R |
| [`recent_commits.rs:389`](../../lib/src/filesystem/git/recent_commits.rs#L389) | same | `Commit::time().seconds()` | range cutoff | R |
| [`recent_commits.rs:460-467`](../../lib/src/filesystem/git/recent_commits.rs#L460) | `collect_commits_from_hash_to_head` | `revwalk()`; `set_sorting(Sort::TOPOLOGICAL \| Sort::TIME)`; `push_head` | `git log <hash>..HEAD` | R |
| [`recent_commits.rs:550-554`](../../lib/src/filesystem/git/recent_commits.rs#L550) | `collect_commits_by_count` | `revwalk()`; `set_sorting(Sort::TIME)`; `push_head` | last-N `git log` | R |
| [`recent_commits.rs:282-290`](../../lib/src/filesystem/git/recent_commits.rs#L282) | `get_recent_commits_by_hash` | `revparse_single` → `peel_to_commit` → `Commit::id` | `git rev-parse <hash>` | R |
| [`recent_commits.rs:293-297`](../../lib/src/filesystem/git/recent_commits.rs#L293) | same | `repo.head().and_then(peel_to_commit)` → `Commit::id` | HEAD oid | R |
| [`recent_commits.rs:299-303`](../../lib/src/filesystem/git/recent_commits.rs#L299) | same | `repo.graph_descendant_of(head_oid, target_oid)` | ancestry validation (→ `HashNotReachable`) | R |

**Sort usage:** `git2::Sort::TIME` at [`recent_commits.rs:373`](../../lib/src/filesystem/git/recent_commits.rs#L373) and [`553`](../../lib/src/filesystem/git/recent_commits.rs#L553); `Sort::TOPOLOGICAL | Sort::TIME` at [`recent_commits.rs:464`](../../lib/src/filesystem/git/recent_commits.rs#L464). The source comment at [`recent_commits.rs:371-372`](../../lib/src/filesystem/git/recent_commits.rs#L371) documents that pure TIME sort makes the early `break` on the range cutoff safe.

### 6.2 Ref decoration & resolution — `discovery.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`discovery.rs:37-38`](../../lib/src/filesystem/git/discovery.rs#L37) | `collect_ref_decorations` | returns `HashMap<git2::Oid, Vec<RefDecoration>>` | per-commit ref map | R |
| [`discovery.rs:41-54`](../../lib/src/filesystem/git/discovery.rs#L41) | same | `repo.head()` → `Reference::{is_branch,shorthand}` | active branch | R |
| [`discovery.rs:57`](../../lib/src/filesystem/git/discovery.rs#L57) | same | `repo.references()` | enumerate **all** refs | R |
| [`discovery.rs:62-70`](../../lib/src/filesystem/git/discovery.rs#L62) | same | `Reference::name`; `Reference::peel_to_commit`; `Commit::id` | classify `refs/heads`/`refs/remotes`/`refs/tags` | R |
| [`discovery.rs:170-193`](../../lib/src/filesystem/git/discovery.rs#L170) | `get_commit_refs` | `repo.head()` → `peel_to_commit` → `id`; `get_upstream_commit` | HEAD + upstream SHA | R |
| [`discovery.rs:197-237`](../../lib/src/filesystem/git/discovery.rs#L197) | `get_upstream_commit` | `head()`; `Reference::is_branch`/`shorthand`; `find_branch(name, BranchType::Local)`; `Branch::upstream`; `Branch::get().peel_to_commit` | `@{upstream}` resolution | R |
| [`discovery.rs:245-289`](../../lib/src/filesystem/git/discovery.rs#L245) | `resolve_base_branch` | `is_worktree`; `commondir().parent()`; `Repository::open`; `head()`/`shorthand`/`peel_to_commit`; `find_reference("refs/heads/main\|master")` | base-branch + oid (worktree-aware) | R |
| [`discovery.rs:342-378`](../../lib/src/filesystem/git/discovery.rs#L342) | `get_commit_by_sha_with_decorations` | `revparse_single` → `peel_to_commit`; `Commit::{id,message,author,time}` | `git show <sha>` metadata | R |

### 6.3 Remotes, branches, tracking — `remote_refresh.rs`

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`remote_refresh.rs:20`](../../lib/src/filesystem/git/remote_refresh.rs#L20) | `get_git_config` | `repo.config()` | merged `git config` | R |
| [`remote_refresh.rs:33`,`43`](../../lib/src/filesystem/git/remote_refresh.rs#L33) | same | `Config::add_file(path, git2::ConfigLevel::ProgramData, false)` | add macOS/Windows system gitconfig (in-memory layer) | R (no disk write) |
| [`remote_refresh.rs:48-59`](../../lib/src/filesystem/git/remote_refresh.rs#L48) | same | `Config::get_string` / `get_bool` (12 keys) | read user/gpg/delta config | R |
| [`remote_refresh.rs:75-90`](../../lib/src/filesystem/git/remote_refresh.rs#L75) | `get_local_branches` | `head()` → `peel_to_commit` → `id` | HEAD oid baseline | R |
| [`remote_refresh.rs:92`](../../lib/src/filesystem/git/remote_refresh.rs#L92) | same | `repo.branches(Some(git2::BranchType::Local))` | `git branch` (local) | R |
| [`remote_refresh.rs:95-111`](../../lib/src/filesystem/git/remote_refresh.rs#L95) | same | `Branch::name`; `Branch::get().peel_to_commit().id()` | name + short hash | R |
| [`remote_refresh.rs:127`](../../lib/src/filesystem/git/remote_refresh.rs#L127) | same | `repo.graph_ahead_behind(c.id(), head_id)` | per-branch ahead/behind | R |
| [`remote_refresh.rs:173`](../../lib/src/filesystem/git/remote_refresh.rs#L173) | `get_tracking_status` | `find_branch(name, BranchType::Local)` | local branch | R |
| [`remote_refresh.rs:177`](../../lib/src/filesystem/git/remote_refresh.rs#L177) | same | `Branch::get().peel_to_commit` | local tip | R |
| [`remote_refresh.rs:181`](../../lib/src/filesystem/git/remote_refresh.rs#L181) | same | `repo.remotes()` | remote names | R |
| [`remote_refresh.rs:187-192`](../../lib/src/filesystem/git/remote_refresh.rs#L187) | same | `find_reference("refs/remotes/<r>/<b>")` → `peel_to_commit` | remote tip | R |
| [`remote_refresh.rs:196`](../../lib/src/filesystem/git/remote_refresh.rs#L196) | same | `repo.graph_ahead_behind(local, remote)` | behind count | R |
| [`remote_refresh.rs:230-248`](../../lib/src/filesystem/git/remote_refresh.rs#L230) | `push_relevant_ahead` | `git2::Oid` arg; `repo.revwalk()`; `Revwalk::push(local)`; `references_glob("refs/remotes/<r>/*")`; `Reference::target`; `Revwalk::hide`; `Revwalk::count` | push-relevant ahead = `rev-list local --not refs/remotes/<r>/*` | R |
| [`remote_refresh.rs:256-269`](../../lib/src/filesystem/git/remote_refresh.rs#L256) | `get_remotes` | `repo.remotes()`; `repo.find_remote(name)`; `Remote::url` | `git remote -v` | R |
| [`remote_refresh.rs:497-508`](../../lib/src/filesystem/git/remote_refresh.rs#L497) | `get_remote_default_branch` | `find_reference("refs/remotes/<r>/HEAD")`; `Reference::symbolic_target` | remote default branch | R |
| [`remote_refresh.rs:515-530`](../../lib/src/filesystem/git/remote_refresh.rs#L515) | `get_remote_branches` | `references_glob("refs/remotes/<r>/*")`; `Reference::name` | remote branch list | R |

**Ancestry-walk containment cache (perf, documented):** `populate_recent_commit_remotes`
([`remote_refresh.rs:390-465`](../../lib/src/filesystem/git/remote_refresh.rs#L390)) builds
a `HashMap<git2::Oid, Vec<String>>` ([`remote_refresh.rs:420`](../../lib/src/filesystem/git/remote_refresh.rs#L420))
by walking ancestry **once per remote tip** instead of `graph_descendant_of` per
(commit × branch). git2 surface: `git2::Oid::from_str` ([`411`](../../lib/src/filesystem/git/remote_refresh.rs#L411), [`452`](../../lib/src/filesystem/git/remote_refresh.rs#L452)),
`find_commit` + `Commit::time().seconds()` for the early-stop heuristic ([`414`,`443`](../../lib/src/filesystem/git/remote_refresh.rs#L414)),
`revwalk()` + `push(tip_oid)` + `Iterator` ([`423-430`](../../lib/src/filesystem/git/remote_refresh.rs#L423)).
`remote_branch_tips` ([`468-491`](../../lib/src/filesystem/git/remote_refresh.rs#L468))
feeds it via `references_glob("refs/remotes/*")` + `Reference::{name,peel_to_commit}` →
`(remote_name, git2::Oid)`.

### 6.4 Worktrees — `remote_refresh.rs` + `worktree.rs`

`get_worktrees` (`remote_refresh.rs`) is the **worktree fan-out** path called out in the
skill. It opens a fresh `git2::Repository` per worktree because `Repository` is `!Sync`,
then fans out per-worktree analysis with Rayon.

| file:line | git2 surface | Operation | R/W |
|---|---|---|---|
| [`remote_refresh.rs:557`](../../lib/src/filesystem/git/remote_refresh.rs#L557) | `repo.worktrees()` | list worktree names | R |
| [`remote_refresh.rs:564`](../../lib/src/filesystem/git/remote_refresh.rs#L564) | `resolve_base_branch(repo)` | base branch + oid | R |
| [`remote_refresh.rs:569`](../../lib/src/filesystem/git/remote_refresh.rs#L569) | `repo.path()` | base `.git` path | R |
| [`remote_refresh.rs:574-575`](../../lib/src/filesystem/git/remote_refresh.rs#L574) | `repo.find_worktree(name)` + `Worktree::path` | per-worktree path | R |
| [`remote_refresh.rs:581`,`603`](../../lib/src/filesystem/git/remote_refresh.rs#L581) | `Repository::open(base_repo_path)` | per-thread base handle (`!Sync` workaround — documented at [`567`,`602`](../../lib/src/filesystem/git/remote_refresh.rs#L567)) | R |
| [`remote_refresh.rs:586`](../../lib/src/filesystem/git/remote_refresh.rs#L586) | `Repository::open(worktree_path)` | per-worktree handle | R |
| [`remote_refresh.rs:589-598`](../../lib/src/filesystem/git/remote_refresh.rs#L589) | `head()`; `Reference::shorthand`; `peel_to_commit`; `Commit::id` | branch + HEAD sha | R |
| [`remote_refresh.rs:610`](../../lib/src/filesystem/git/remote_refresh.rs#L610) | `base_repo.graph_ahead_behind(wt, base)` | ahead/behind vs base | R |
| [`remote_refresh.rs:620`](../../lib/src/filesystem/git/remote_refresh.rs#L620) | `base_repo.graph_descendant_of(base, wt)` | merged? test | R |
| [`remote_refresh.rs:630-633`](../../lib/src/filesystem/git/remote_refresh.rs#L630) | `find_commit(base_id)`; `base_repo.merge_commits(wt, base, None)`; `Index::has_conflicts` | **in-memory 3-way merge** conflict probe | R (no repo writes) |
| [`remote_refresh.rs:638`](../../lib/src/filesystem/git/remote_refresh.rs#L638) | `get_repo_status_counts(&worktree_repo)` | dirty + changed-file count | R |

`worktree.rs` (the lighter, CLI-facing worktree listing):

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`worktree.rs:47`](../../lib/src/filesystem/git/worktree.rs#L47) | `get_current_worktree_name` | `Repository::discover`; `is_worktree`; `workdir` | current worktree basename | R |
| [`worktree.rs:77`](../../lib/src/filesystem/git/worktree.rs#L77) | `get_current_worktree_info` | `discover`; `is_worktree`; `workdir` | name + abs path | R |
| [`worktree.rs:128`](../../lib/src/filesystem/git/worktree.rs#L128) | `list_worktrees` | `discover` | repo handle | R |
| [`worktree.rs:135-137`](../../lib/src/filesystem/git/worktree.rs#L135) | same | `is_worktree`; `commondir().parent()`; `Repository::open(common_dir)` | re-open base for full enumeration | R |
| [`worktree.rs:154`](../../lib/src/filesystem/git/worktree.rs#L154) | same | `repo.workdir()` | main worktree path/name | R |
| [`worktree.rs:176`](../../lib/src/filesystem/git/worktree.rs#L176) | same | `repo.worktrees()` | linked worktree names | R |
| [`worktree.rs:185`](../../lib/src/filesystem/git/worktree.rs#L185) | same | `repo.find_worktree(name)` + `Worktree::path` | per-worktree path | R |
| [`worktree.rs:194`](../../lib/src/filesystem/git/worktree.rs#L194) | same | `Repository::open(path)` | open worktree for HEAD | R |
| [`worktree.rs:215-225`](../../lib/src/filesystem/git/worktree.rs#L215) | `resolve_branch_and_detached` | `repo.head_detached()`; `repo.head()`; `Reference::shorthand` | branch + detached state | R |

---

## 7. Remote / Fetch — `remote_refresh.rs`

**The one write path in production.** It does **not** call `Remote::fetch` (libgit2);
it shells out to the user's `git` binary.

| file:line | Function | Mechanism | Effect | R/W |
|---|---|---|---|---|
| [`remote_refresh.rs:303-347`](../../lib/src/filesystem/git/remote_refresh.rs#L303) | `refresh_remote_tracking_refs` | `repo.workdir()` + `repo.remotes()` (git2, read), then `std::thread::scope` fan-out | orchestrates parallel fetch | mixed |
| [`remote_refresh.rs:350-360`](../../lib/src/filesystem/git/remote_refresh.rs#L350) | `fetch_single_remote` | `std::process::Command::new("git")` with `args(["fetch","--quiet","--prune", remote])`, `env("GIT_TERMINAL_PROMPT","0")` | **writes object DB + updates `refs/remotes/*`** | **W (out-of-process)** |

Details relevant to migration:
- **Not a git2 API call.** The fetch is an external `git` subprocess; libgit2 is only used
  to enumerate remotes and resolve `workdir`. A `gix` migration of the surrounding read
  code does not change the fetch mechanism unless explicitly re-platformed.
- **`GIT_TERMINAL_PROMPT=0`** ([`remote_refresh.rs:353`](../../lib/src/filesystem/git/remote_refresh.rs#L353))
  is set to prevent interactive credential hangs — the skill's "parallel remote fetch …
  `GIT_TERMINAL_PROMPT=0` is preserved" note.
- **Bounded parallelism:** single remote → serial path ([`319-322`](../../lib/src/filesystem/git/remote_refresh.rs#L319));
  multiple → `max_concurrency.clamp(1,3)` ([`324`](../../lib/src/filesystem/git/remote_refresh.rs#L324)) chunked across scoped threads.
- This is triggered only when `GitRequest.refresh_remote_tracking` is true (deep mode),
  via `detect_with_request` ([`types.rs:658-660`](../../lib/src/filesystem/git/types.rs#L658)).

---

## 8. Repo Identity & Misc Read Sites

`lib/src/filesystem/repo/identity.rs`:

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`identity.rs:72`](../../lib/src/filesystem/repo/identity.rs#L72) | `detect_repo_identity` | `Repository::discover` + `workdir` | repo root | R |
| [`identity.rs:103`](../../lib/src/filesystem/repo/identity.rs#L103) | `resolve_name` (takes `&git2::Repository`) | passes repo to `remote_basename` | — | R |
| [`identity.rs:147-158`](../../lib/src/filesystem/repo/identity.rs#L147) | `remote_basename` | `find_remote("origin")`; `repo.remotes()`; `find_remote(name)`; `Remote::url` | derive name from remote URL | R |

`lib/src/filesystem/mod.rs`, `docs.rs`, `just.rs`, `blast_radius.rs` — discovery + status
already covered in §4/§5.

---

## 9. CLI Consumers

`cli/src/commands/mod.rs`:

| file:line | Context | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`mod.rs:192-197`](../../cli/src/commands/mod.rs#L192) | `Docs` command | `Repository::discover` + `workdir` | repo root | R |
| [`mod.rs:414-419`](../../cli/src/commands/mod.rs#L414) | blast-radius text render | `Repository::discover` + `workdir` | repo root | R |
| [`mod.rs:486-488`](../../cli/src/commands/mod.rs#L486) | `RepoAction::Remote` (no arg) | `Repository::discover` + `resolve_origin_or_first_remote` | default remote | R |
| [`mod.rs:527-531`](../../cli/src/commands/mod.rs#L527) | `RepoAction::Hash` | `Repository::discover`; `get_commit_by_sha`; `get_commit_files` | `git show <sha>` | R |
| [`mod.rs:642-664`](../../cli/src/commands/mod.rs#L642) | `RepoAction::Root` | `Repository::discover`; `repo.workdir()` (comment notes git2 trailing-separator quirk at [`662`](../../cli/src/commands/mod.rs#L662)) | print repo root | R |
| [`mod.rs:680-682`](../../cli/src/commands/mod.rs#L680) | `RepoAction::HasMergeConflict` | `Repository::discover`; `detect_merge_conflicts` | conflict check | R |
| [`mod.rs:1101-1103`](../../cli/src/commands/mod.rs#L1101) | git-status path scoping | `Repository::discover`; `get_commits_for_path` | scoped log | R |
| [`mod.rs:1176-1178`](../../cli/src/commands/mod.rs#L1176) | git-status `--branch` | `Repository::discover`; `get_commits_for_branch` | branch log | R |

`cli/src/commands/repo.rs`:

| file:line | Context | git2 surface | R/W |
|---|---|---|---|
| [`repo.rs:46-48`](../../cli/src/commands/repo.rs#L46) | `repo packages` root resolution | `Repository::discover` + `workdir` | R |
| [`repo.rs:134-136`](../../cli/src/commands/repo.rs#L134) | `repo package-areas` root resolution | `Repository::discover` + `workdir` | R |

`cli/src/commands/remote.rs`:

| file:line | Function | git2 surface | Operation | R/W |
|---|---|---|---|---|
| [`remote.rs:72`](../../cli/src/commands/remote.rs#L72) | `handle_pr_command` | `Repository::discover` | repo handle | R |
| [`remote.rs:167-180`](../../cli/src/commands/remote.rs#L167) | `resolve_origin_or_first_remote(&git2::Repository)` | `find_remote("origin")` + `Remote::url`; `repo.remotes()`; `find_remote(name)` + `url` | preferred remote URL | R |
| [`remote.rs:215-217`](../../cli/src/commands/remote.rs#L215) | `resolve_remote_name` | `Repository::discover`; `find_remote(name)`; `Remote::url` | named remote URL | R |
| [`remote.rs:221-223`](../../cli/src/commands/remote.rs#L221) | `commit_url_from_repo(&git2::Repository, sha)` | `find_remote("origin")`; `Remote::url` | build commit browser URL | R |

---

## 10. Test-Only git2 Usage (out of migration scope, recorded for completeness)

These do **not** count toward the production inventory but use `git2` write APIs
(`init`, `commit`, `reference`, `worktree`, `set_head_detached`, `Signature`,
`Index::add_path`/`write`/`write_tree`/`remove_path`, `CheckoutBuilder`, `IndexAddOption`,
`Time`). They build fixtures.

| File | Lines | Nature |
|---|---|---|
| `lib/src/filesystem/git/types.rs` | [`1157-1172`](../../lib/src/filesystem/git/types.rs#L1157) | `#[cfg(test)] setup_repo` (init/commit) |
| `lib/src/filesystem/git/status.rs` | [`414-447`](../../lib/src/filesystem/git/status.rs#L414), 532, 643 | test fixtures (init/commit/index) |
| `lib/src/filesystem/git/worktree.rs` | [`243-258`](../../lib/src/filesystem/git/worktree.rs#L243), 386-409 | test fixtures (init/commit/worktree/set_head_detached) |
| `lib/src/filesystem/git/remote_refresh.rs` | [`666-835`](../../lib/src/filesystem/git/remote_refresh.rs#L666) | test fixtures (init/commit/`Repository::reference` fake remotes) |
| `lib/src/filesystem/docs.rs` | [`1561`](../../lib/src/filesystem/docs.rs#L1561) | `Repository::init` in `#[cfg(test)]` |
| `lib/src/filesystem/repo/identity.rs` | [`258`,`276`,`294`](../../lib/src/filesystem/repo/identity.rs#L258) | `Repository::init` in `#[cfg(test)]` |
| `lib/tests/fixtures.rs` | 1 | `use git2::Repository` (integration fixtures) |
| `lib/tests/bench_fixtures.rs` | 16 | `git2::{Repository, StatusOptions}` |
| `lib/tests/integration.rs` | 720, 1142, 1441, 1558, 1570, 1674, 1816, 1933–1976 | fixtures: `Signature`, `Oid`, `CheckoutBuilder`, `IndexAddOption`, `Time`, `commit` |
| `lib/benches/support/builder.rs` | 1, 14, 73, 80 | `IndexAddOption`, `Repository`, `Signature`, `Time`, `Commit` |
| `cli/tests/cli.rs` | 1519, 1548, 1569, 2571, 2617, 2632, 2651, 2677, 2960, 4287, 4325, 4781, 5173, 5179, 5231, 5269 | `Repository::{init,open}`, `IndexAddOption` |

---

## 11. Cross-Cutting Observations (factual)

- **`Repository` is `!Sync`** and sniff respects this explicitly: the worktree fan-out
  (`get_worktrees`) and remote fetch orchestration open a **fresh handle per thread**
  rather than sharing `&Repository` (documented at
  [`remote_refresh.rs:567`](../../lib/src/filesystem/git/remote_refresh.rs#L567) and
  [`602`](../../lib/src/filesystem/git/remote_refresh.rs#L602)).
- **Oid-keyed caching** is used in two places: the per-instance ref-decoration cache
  (`RefCell<Option<HashMap<git2::Oid, Vec<RefDecoration>>>>` in `GitRepo`) and the
  per-call ancestry-containment map (`HashMap<git2::Oid, Vec<String>>` in
  `populate_recent_commit_remotes`).
- **No libgit2 networking** is used anywhere in production: no `Remote::fetch`/`connect`,
  no `Cred`/`RemoteCallbacks`/`FetchOptions`. The only fetch is the external `git`
  subprocess in §7.
- **Rename detection is off** everywhere (default `StatusOptions`/`DiffOptions`); staged
  renames surface as delete+add pairs (asserted by tests).
- **Diff is always read** via `Diff::print(DiffFormat::Patch, …)` (line text + stats in one
  pass) or `Diff::deltas` (path/kind only). No `DiffStats`, no `Patch::from_diff`.
- **Config** is read-only at the disk level; `Config::add_file(..., ConfigLevel::ProgramData, …)`
  layers an extra system gitconfig into the in-memory merged view (macOS Command Line Tools
  / Git-for-Windows paths) but does not persist.

---

# Usage Review: Patterns to Improve

This section critiques the `git2` usage catalogued above. It is a code review, not
a migration spec. Findings are tagged for whether they should be fixed **independent
of**, **before**, or **during** the planned `gix` migration. Line numbers were
confirmed against source at review time (2026-06-06).

## R1 — Repeated `Repository::discover` for the same logical path — High

**Locations:**
- `lib/src/filesystem/mod.rs:373-376` (`discover_repo_root`)
- `lib/src/filesystem/repo/identity.rs:72` (`detect_repo_identity`)
- `lib/src/filesystem/docs.rs:111`, `docs.rs:270`
- `lib/src/filesystem/just.rs:111-112` (`find_scope_root`)
- `lib/src/filesystem/blast_radius.rs:73`
- `lib/src/filesystem/git/recent_commits.rs:202, 230, 251, 275, 335`
- `lib/src/filesystem/git/types.rs:513` (`GitRepo::discover`)
- CLI: `cli/src/commands/mod.rs:192, 414, 486, 527, 642, 680, 1101, 1176`;
  `cli/src/commands/repo.rs:46, 134`; `cli/src/commands/remote.rs:72, 215`

**Problem.** There are 16 production `Repository::discover` call sites. `discover`
is the single most expensive cheap-looking call here: it `stat`s its way up the
directory tree from `path` to the filesystem root looking for a `.git`, then opens
the repo (loads config, refdb, odb handles). In a single `sniff` invocation that
renders, say, repo identity + docs + blast-radius + git status, the same toplevel
is discovered and re-opened several times across modules — each independently
walking parents and re-initializing libgit2 handles. The library already has the
right abstraction (`GitRepo`, which discovers once and threads `&Repository` to
every helper), but the free functions in `mod.rs`, `identity.rs`, `docs.rs`,
`just.rs`, `blast_radius.rs`, and the five `recent_commits.rs` entry points each
re-discover instead of accepting a borrowed handle. The filesystem staging layer
(`determine_shared_walk_root` → `discover_repo_root`, `mod.rs:362/373`) discovers
the root, then hands a `PathBuf` downstream, and the git stage discovers *again*
from that path.

**Recommendation (independent of migration, but high-value before it).** Thread a
single opened handle. Two concrete options: (a) have `determine_shared_walk_root`
return the opened `GitRepo` (or its root + a shared handle) and pass it into the
git/repo/docs stages, or (b) make the public free functions take
`repo: &GitRepo` / `&Repository` and keep the `discover`-from-path variants only
at the true entry points (CLI commands, Tier-1 `detect`). Doing this *before* the
migration shrinks the surface that has to be ported and makes the win measurable
against the current baseline.

**Migration relevance.** `gix::discover` is comparably priced (parent walk +
repo open) and `gix` adds a **trust evaluation** on open (`gix-sec`), so redundant
opens cost *more* relatively under `gix`, not less. Worse, `gix::Repository` is
`!Sync` without the `parallel` feature and is **not cheaply `Clone`** in the
`git2` sense — the idiomatic pattern is `repo.into_sync()` →
`ThreadSafeRepository` shared via `to_thread_local()`. Consolidating to one handle
now means the migration ports *one* discovery/threading decision instead of 16.

## R2 — `find_commit` (full object decode) inside revwalk loops that only need IDs/metadata — Medium

**Locations:**
- `lib/src/filesystem/git/discovery.rs:148, 476, 587` (recent-commit walks)
- `lib/src/filesystem/git/recent_commits.rs:385, 478, 568` (range/count/hash walks)
- `lib/src/filesystem/git/remote_refresh.rs:413, 443` (containment ancestry walk)

**Problem.** Every revwalk in the codebase follows the `for oid in revwalk { let
commit = repo.find_commit(oid)?; … }` shape. `find_commit` decompresses and parses
the full commit object (zlib inflate + header/tree/parent/author parse). That is
necessary where the body is actually consumed (message/author/time → `CommitInfo`,
`recent_commits.rs:407, 494, 581`). But two of these walks decode the full commit
only to read **`commit.time().seconds()`**:
- `recent_commits.rs:385-392` decodes every commit just to compare its timestamp
  against the `since` cutoff, then (for in-range commits) decodes again-worth of
  work for the body. The timestamp gate is the only reason most commits are
  touched at all.
- `remote_refresh.rs:443-447` decodes every ancestor commit in the containment
  walk purely to read its time for the early-stop heuristic.

Neither walk uses the **commit-graph** (`.git/objects/info/commit-graph`), which
stores commit time + parents in a flat, mmap-friendly index precisely so you can
get generation/time without inflating the object. On a large monorepo with a deep
history (the stated target environment), the timestamp-only decodes are pure waste.

**Recommendation (independent; revisit during migration).** Under `git2` the
lever is limited: `git2` exposes no public commit-graph reader, so the realistic
win is (a) ensure libgit2's object cache is sized (`opts::set_cache_max_size`,
available in 0.21 — sniff is on 0.20.3) so the re-decode in `collect_commits_in_range`
is cheap, and (b) where only time is needed for a gate, accept that `git2` will
decode anyway. This is more of a "note the cost" finding for `git2`.

**Migration relevance.** This is a **primary `gix` upside**. `gix` has first-class
commit-graph support (`repo.commit_graph_if_enabled()`) and the revwalk yields
cheap `Id`s — you call `.object()` only on the commits you actually render. The
two timestamp-only gates above map directly onto commit-graph lookups (no object
decode). Flag these three walks in the migration spec as the places to (1) enable
the commit-graph and (2) split "gate on time/parents" (graph) from "render body"
(decode). Also set `repo.object_cache_size_if_unset(...)` for the walks that *do*
decode bodies repeatedly across stages.

## R3 — `get_commit_files` re-opens the diff machinery per commit and is called per-commit inside a walk — Medium

**Locations:**
- `lib/src/filesystem/git/discovery.rs:390-438` (`get_commit_files`)
- callers: `recent_commits.rs:398, 485, 575` (once **per commit** in the walk)
- `discovery.rs:498-514` (`get_commits_for_path_with_decorations`, inline variant)

**Problem.** `get_commit_files` takes a **`&str` SHA**, then re-parses it with
`Oid::from_str` (`discovery.rs:391`) and re-looks-up the commit with
`find_commit` (`discovery.rs:394`) — even though the caller in
`collect_commits_in_range` already holds the decoded `Commit` and the `Oid`
(`recent_commits.rs:385, 397`). So for every commit in range we: decode the commit
(caller), stringify the oid, re-parse the string back to an oid, re-look-up the
same commit, fetch its tree, fetch parent(0)'s tree, and run a full
`diff_tree_to_tree`. The string round-trip (`Oid` → `String` → `Oid`) is gratuitous,
and the redundant `find_commit` doubles the decode.

**Recommendation (independent of migration).** Change `get_commit_files` to accept
`&Commit` (or `Oid` + the already-open repo) instead of `&str`, eliminating the
`to_string`/`from_str`/`find_commit` round-trip. The diff-per-commit cost is
inherent to "what files did this commit touch", but the object re-decode is not.
This is a pure-`git2` cleanup worth doing now; it also makes the call sites smaller
to port.

**Migration relevance.** `gix` diff is `repo.diff_tree_to_tree(...)` /
`diff::resource_cache()`; the same "pass the already-resolved id/commit, don't
stringify" discipline applies and is *more* important because `gix` rewards
reusing a `resource_cache` across many tree diffs. Note the per-commit diff in a
range walk is exactly the workload `gix`'s caches target — call it out as a
"reuse one resource cache across the walk" opportunity in the spec.

## R4 — Worktree fan-out opens 2–3 `Repository` handles per worktree under Rayon — Medium

**Locations:**
- `lib/src/filesystem/git/remote_refresh.rs:554-657` (`get_worktrees`)
- specifically the dead `_base_repo` at `:581` and per-iteration
  `Repository::open(&base_repo_path)` at `:603`, plus
  `Repository::open(worktree_path)` at `:586`

**Problem.** The `!Sync` workaround is real and correctly documented, but the
current shape is wasteful. Line 581 opens `_base_repo` "to keep structures warm"
and immediately leaves it unused (`let _base_repo = …`), while line 603 opens a
**fresh** base-repo handle *inside every Rayon iteration*. Opening a repo is not
free (config + refdb + odb init), so each worktree pays: one `Repository::open`
for the worktree (`:586`) + one `Repository::open` for the base (`:603`). The
`_base_repo` at 581 provides no warming benefit across threads in `git2` (libgit2's
caches are per-`Repository`, and the handle is never shared), so it is a pure
wasted open. For a checkout with many worktrees (the stated hot path) this is
`2N` repo opens where `N+1` would do if base access were structured per-thread via
Rayon's `map_init`.

**Recommendation (independent of migration).** Remove the unused `_base_repo`
(`:581`) — it is misleading and does nothing. If base-repo warmth matters, use
`par_iter().map_init(|| Repository::open(&base_repo_path).ok(), |base, item| …)`
so each Rayon worker thread opens the base **once** and reuses it across the
worktrees it processes, instead of once per worktree. Confirm the comment at
`:600-602` against the new structure (it currently describes the per-iteration
open).

**Migration relevance.** This is the canonical `!Sync` fan-out the inventory
flags, and it is where `gix`'s `ThreadSafeRepository` changes the calculus: open
the base **once**, `into_sync()`, then `to_thread_local()` per Rayon worker — no
re-discovery, no per-worktree base re-open. The migration should treat
`get_worktrees` as the model conversion for the `!Sync` pattern; fixing the
`map_init` shape now makes the `gix` port a near-mechanical substitution.

## R5 — In-memory 3-way merge per worktree for a boolean "would this conflict?" — Medium

**Locations:**
- `lib/src/filesystem/git/remote_refresh.rs:625-636` (`merge_commits` + `has_conflicts`)

**Problem.** For each worktree, the code runs a full `merge_commits(wt, base, None)`
(`:632`) and then only reads `index.has_conflicts()` (`:634`) — a boolean. A
3-way merge materializes a merged index (tree merge across all entries, blob-level
3-way on conflicting paths); throwing all of it away for one bit is expensive,
especially multiplied across worktrees inside the Rayon fan-out. It also runs
unconditionally even when `merged == true` (`:614-623`), where a fully-merged
branch can never conflict — the merge is provably redundant in that case.

**Recommendation (independent of migration).** Short-circuit: if `merged` (i.e.
`graph_descendant_of(base, wt)`) is already true, set `has_conflicts = false`
without merging. That alone removes the merge for every already-merged worktree.
If conflict detection on unmerged branches is still wanted, keep `merge_commits`
but only on that path. (libgit2 has no cheaper "merge-tree --quiet"; the
short-circuit is the realistic `git2` win.)

**Migration relevance.** `gix` exposes `repo.merge_commits(...)` / `merge_trees`
with configurable favor/abort behavior; if a "stop at first conflict" mode is
available in the pinned `gix` version it is a better fit than full-index
materialization. Note this in the spec as a place to check `gix-merge` for an
early-abort option rather than porting the full merge verbatim.

## R6 — Over-broad `git2::Error` mapping that erases the failing operation — Medium

**Locations:**
- `lib/src/error.rs:15` (`Git(#[from] git2::Error)` — single bucket)
- `lib/src/filesystem/git/recent_commits.rs:202, 230, 251, 275, 335`
  (`map_err(|_| SniffError::NotARepository(path))` — discards the real error)
- CLI: `cli/src/commands/mod.rs:650, 681`; `repo.rs:46`; `remote.rs:73`
  (`format!("Not a git repository: {}", e)`)

**Problem.** Two opposite smells. (1) `recent_commits.rs` maps *every*
`Repository::discover` failure to `NotARepository` with `map_err(|_| …)`,
swallowing the underlying `git2::Error`. `discover` can fail for reasons other
than "no repo" — `ErrorCode::Owner` (the libgit2 1.5+ `safe.directory` ownership
check, very real in CI/containers, see `git2.md` cross-cutting notes), permission
denied, or a corrupt `.git`. Reporting all of these as "not a repository" sends
users down the wrong debugging path. (2) The single `Git(git2::Error)` variant is
fine as a catch-all, but combined with the pervasive
`.map_err(|e| { debug!(...); e }).ok()` downgrade-to-`None` pattern (e.g.
`types.rs:541`, `discovery.rs:43`, the whole of `remote_refresh.rs`), genuine
errors (corrupt refdb, I/O failure) are indistinguishable from the legitimate
"absent" cases (detached HEAD, no upstream) at the call site. The "absent" cases
are correct to swallow; the I/O/corruption cases arguably are not.

**Recommendation (before migration, low effort).** For the discover sites, at
minimum preserve the error: `map_err(|e| { debug!(error=%e); SniffError::NotARepository(..) })`,
or branch on `e.code()` so `ErrorCode::Owner`/`NotFound` map distinctly. Keep the
`.ok()` downgrades for the genuinely-optional reads (HEAD/upstream), but consider
matching on `ErrorCode::NotFound` specifically there so that a non-`NotFound`
error is logged at `warn!` rather than silently `debug!`-and-dropped. Doing this
before the migration locks in the *intended* error semantics as testable behavior.

**Migration relevance.** `gix` does **not** have a single `git2::Error`; each
operation returns its own error enum (`gix::discover::Error`, `gix::status::Error`,
`gix::revision::walk::Error`, …), and 0.50+ introduced the `gix-error`/`Exn`
exception-tree model. The `#[from] git2::Error` blanket conversion **cannot** be
ported one-to-one — every `?` site will need a per-operation `SniffError` variant
or a `map_err`. Deciding now which failures are "absent → None" vs "real error →
surface" turns that forced rewrite into a deliberate redesign instead of a
mechanical re-bucketing. This is the single most migration-coupled finding.

## R7 — Non-UTF-8 ref/path handling silently drops data — Low (correctness)

**Locations:**
- `lib/src/filesystem/git/discovery.rs:62` (`reference.name()` → `Option<&str>`,
  `None` skips the ref)
- `lib/src/filesystem/git/status.rs:398` (`std::str::from_utf8(&entry.path).unwrap_or_default()`
  → conflicted path becomes `""` on non-UTF-8)
- `lib/src/filesystem/git/diff.rs:34-37` (path via `DiffFile::path()`, which is
  `None` for non-UTF-8 paths in `git2` → line silently dropped)
- `current_branch`/`shorthand` sites broadly (`types.rs:547`, `worktree.rs:223`, …)

**Problem.** `git2` 0.20 returns `Option<&str>` / `Option<&Path>` for names and
paths, yielding `None` on non-UTF-8 — and sniff treats `None` as "skip"
(`discovery.rs:62-64`) or coerces to empty string (`status.rs:398`). On a repo
with a non-UTF-8 ref name or filename (legal in git), a conflicted file shows as
an empty path, and a non-UTF-8 ref is omitted from decorations entirely. This is
latent and rare, but it is a silent-wrong-answer, not an error.

**Recommendation (independent of migration).** Where correctness matters
(conflict paths, file changes), prefer the `*_bytes` accessors `git2` offers
(`Reference::name_bytes`, `DiffFile::path_bytes`, the raw `IndexEntry::path` you
already have at `status.rs:398`) and convert with `to_string_lossy()` rather than
dropping. Low priority because sniff's repos are overwhelmingly UTF-8, but worth a
deliberate decision rather than an accidental `unwrap_or_default()`.

**Migration relevance.** `gix` is **bytes-first** (`BStr`/`BString` everywhere);
ref names and paths are byte strings by default and UTF-8 is the caller's
conversion choice. The migration is a natural moment to make the lossy-vs-skip
decision explicit, since `gix` forces you to handle the bytes rather than handing
you an `Option<&str>`. Note in the spec that the current "`None` → skip" behavior
is an implicit policy that must be re-expressed against `BStr`.

## R8 — `org_and_repo` / metadata helpers re-enumerate remotes instead of reusing computed state — Low

**Locations:**
- `lib/src/filesystem/git/types.rs:566-572` (`org_and_repo` calls
  `get_remotes(&self.repo, false)`)
- vs `detect_with_request` `types.rs:704-708` already computing `remotes` and
  `types.rs:745-748` deriving `(org, repo)` from them

**Problem.** `GitRepo::org_and_repo()` independently calls `get_remotes` (which
iterates `repo.remotes()` + `find_remote` per name + reads URLs) every time it is
invoked, and `detect_with_request` separately computes the same `remotes` vector
and derives `(org, repo)` from it. A caller that wants both the full `GitInfo` and
later calls `org_and_repo()` pays the remote enumeration twice. Remote enumeration
is config-file I/O (cheap individually) but it is uncached on the `GitRepo` handle,
unlike `config_cache` (`types.rs:482`) and `ref_decorations` (`types.rs:480`)
which *are* memoized.

**Recommendation (independent of migration).** Either memoize the remote list on
`GitRepo` the way `config` and `ref_decorations` are, or have `org_and_repo`
accept/reuse an already-computed `&[RemoteInfo]`. Minor, but it closes the one
gap in the otherwise-consistent "compute once, cache on the handle" design.

**Migration relevance.** Neutral — `gix`'s `repo.remote_names()` / `find_remote`
is likewise cheap config I/O. Caching strategy ports unchanged; just fewer call
sites to convert if consolidated first.

## R9 — `references()` full-refdb scan to build decorations, recomputed per non-cached path — Low

**Locations:**
- `lib/src/filesystem/git/discovery.rs:57-94` (`collect_ref_decorations`)
- cached path: `types.rs:495-504` (`GitRepo::ref_decorations`, memoized) — good
- uncached paths: `discovery.rs:581` (`get_commits_for_branch`), `discovery.rs:467`
  and `recent_commits.rs` flows that pass `None` and fall back to
  `collect_ref_decorations(repo)` (`discovery.rs:142, 369`)

**Problem.** `collect_ref_decorations` enumerates **all** refs (`repo.references()`,
`discovery.rs:57`) and `peel_to_commit`s each one (`:67`) — a full refdb walk plus
an object decode per ref. The `GitRepo` handle memoizes this correctly. But the
free-function flows (`get_commits_for_branch` at `:581`, and any caller passing
`ref_decorations: None`) rebuild the entire decoration map from scratch on every
call. The CLI `git-status --branch` path (`cli/src/commands/mod.rs:1176` →
`get_commits_for_branch`) thus does a full refdb scan that the `GitRepo` handle
would have cached.

**Recommendation (independent of migration).** Route the free-function commit
walks through the `GitRepo` handle (ties into R1) so they share the memoized
decoration map, or pass the cached map down. Low severity because these are
single-shot CLI paths, but it is avoidable O(all-refs × peel) work.

**Migration relevance.** `gix` ref iteration (`repo.references()?.all()`) is
cheap (packed-refs + loose, no decode) and peeling is where the cost is; the
memoization pattern ports directly. The `peel_to_commit`-per-ref decode is another
spot where a `gix` object cache helps. Consolidating onto one handle (R1) makes
the cached-decoration design the default rather than the exception.

## R10 — Out-of-process `git fetch` shell-out: correct choice, with caveats to preserve — Low (informational)

**Locations:**
- `lib/src/filesystem/git/remote_refresh.rs:350-360` (`fetch_single_remote`)
- orchestration `:303-347` (`refresh_remote_tracking_refs`)

**Problem / assessment.** This is the one write path and it deliberately avoids
libgit2 networking. Given sniff declares `git2` with default features but uses
**no** `Cred`/`RemoteCallbacks`/`FetchOptions`, shelling out to the user's `git`
is the *right* call: it inherits the user's credential helpers, SSH agent, proxy
config, and `~/.gitconfig` `url.*.insteadOf` rules for free — none of which
libgit2 would pick up without substantial callback code. The `GIT_TERMINAL_PROMPT=0`
guard (`:353`) and bounded concurrency (`:324`, clamp 1–3) are both correct. Two
minor caveats: the result `status()` is mapped to a `warn!` but a non-zero exit
(auth failure, network down) is otherwise swallowed (`:351-359` ignores
`ExitStatus::success()`), so a failed fetch silently yields stale tracking data;
and `Command::new("git")` assumes `git` is on `PATH`.

**Recommendation (independent of migration).** Optionally inspect
`status.success()` and `debug!`/`warn!` on non-zero exit so a failed fetch is
observable (the data staleness is otherwise invisible). Keep the shell-out.

**Migration relevance.** **Do not** port this to `gix` networking. `gix` network
features are opt-in, require choosing a TLS backend, and credential-helper
integration (`gix-credentials`) does not match a user's full `git` config as
transparently as the subprocess does. The migration of the *surrounding read code*
(remote enumeration, `workdir`) is independent of this fetch and should leave it
untouched. Flag explicitly in the spec: "fetch stays a subprocess."

## R11 — `default-features = true` on `git2` pulls in unused `ssh`/`https`/`cred` — Low

**Locations:**
- `sniff/lib/Cargo.toml:17` (`git2 = "0.20.3"`), `sniff/cli/Cargo.toml:28, 44`

**Problem.** On `git2` 0.20, `ssh`/`https`/`cred` are still default features (they
de-default in 0.21). Sniff uses no libgit2 networking (confirmed: no `Remote::fetch`,
`Cred`, `RemoteCallbacks`), so these features pull in libssh2 → OpenSSL and the
HTTPS transport for nothing — extra build time, larger static link, and a bigger
CVE surface (the OpenSSL/libssh2 chain) for code that is never exercised.

**Recommendation (independent of migration; do now).** Set
`git2 = { version = "0.20.3", default-features = false }` in both manifests, and
bump toward 0.21 where de-defaulting is the upstream default (0.21 also fixes the
non-UTF-8 branch-name panics relevant to R7 and adds `set_cache_max_size` relevant
to R2). Verify the build still links (it should — no networking is used).

**Migration relevance.** Directly informs the `gix` feature-flag choice: sniff's
`gix` dependency should start from `default-features = false` and add only
`status`, `blame`(if used), `revision`, `dirwalk`/`excludes`, `blob-diff`, and
**no** network features — mirroring the "read-only, no networking" profile this
finding establishes. The de-featuring exercise now produces the exact allowlist
the migration needs.

## R12 — Detached-HEAD and base-branch fallbacks are heuristic, not always correct — Low (robustness)

**Locations:**
- `lib/src/filesystem/git/discovery.rs:245-293` (`resolve_base_branch`)
- `lib/src/filesystem/git/discovery.rs:207` (`get_upstream_commit` early-returns
  on detached HEAD — correct)

**Problem.** `resolve_base_branch` falls back to literal `"main"` then `"master"`
(`:277`) and finally `("main".to_string(), None)` (`:292`) when HEAD is detached
or unavailable. For worktree ahead/behind this can silently compare against the
wrong base (e.g. a repo whose default is `develop` or `trunk`), producing
plausible-but-wrong ahead/behind/merged numbers rather than an "unknown". The
remote default branch is actually resolvable (`get_remote_default_branch`,
`remote_refresh.rs:497`, reads `refs/remotes/<r>/HEAD`) but is not consulted in
the fallback chain.

**Recommendation (independent of migration).** Before the `"main"/"master"`
literal guess, consult `refs/remotes/origin/HEAD` (the function already exists)
to learn the real default branch. Where even that is absent, consider surfacing
`base_branch: None`/"unknown" rather than a confident-but-wrong `"main"`.

**Migration relevance.** Neutral — `gix` resolves the same symbolic ref via
`find_reference("refs/remotes/origin/HEAD")`; the logic ports directly. Fix the
heuristic in whichever crate is current when touched.

## Prioritized Summary

| ID  | Severity | One-line | Migration-coupled? |
|-----|----------|----------|--------------------|
| R1  | High     | 16 redundant `Repository::discover` calls; thread one handle instead | Yes (handle-threading is the core `gix` `!Sync`/trust decision) |
| R6  | Medium   | Blanket `git2::Error` mapping + lossy `map_err(\|_\| NotARepository)` erase failing op | **Yes (highest)** — `gix` has no unified error; forces per-op redesign |
| R2  | Medium   | `find_commit` decodes full objects in walks needing only ID/time; no commit-graph | Yes (primary `gix` upside: commit-graph + lazy `Id`) |
| R4  | Medium   | Worktree fan-out: dead `_base_repo` + per-iteration base re-open under Rayon | Yes (model case for `ThreadSafeRepository` port) |
| R3  | Medium   | `get_commit_files(&str)` round-trips Oid→String→Oid + re-`find_commit` per walked commit | Partial (reuse a `gix` resource_cache) |
| R5  | Medium   | Full 3-way `merge_commits` per worktree for one `has_conflicts` bool; runs even when merged | Partial (check `gix-merge` early-abort) |
| R7  | Low      | Non-UTF-8 ref/path → silent skip or empty string; use `*_bytes` accessors | Yes (`gix` is `BStr`-first; forces explicit policy) |
| R8  | Low      | `org_and_repo` re-enumerates remotes; not memoized like config/decorations | No |
| R9  | Low      | Uncached `collect_ref_decorations` does full refdb scan + peel-per-ref on free-fn paths | Partial (object cache helps; ties to R1) |
| R10 | Low      | `git fetch` subprocess is correct; preserve it + observe non-zero exit | Yes (spec must say "fetch stays subprocess") |
| R11 | Low      | `git2` default features pull unused ssh/https/cred; set `default-features = false` | Yes (defines the `gix` read-only feature allowlist) |
| R12 | Low      | Base-branch fallback guesses `main`/`master` instead of reading `origin/HEAD` | No |
