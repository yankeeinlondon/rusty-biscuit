---
date: 2026-06-13
agent: "${env.AGENT}"
reviewed: true
status: "ready for planning and implementation"
---

## Problem Statement

`wt list` is noticeably slow, especially in the `rusty-biscuit` monorepo (48
workspace members, many linked worktrees). The dominant cost **owned by this
package** is excessive `git` subprocess spawning: a single `wt list -v` on a
non-main worktree issues roughly **30+** `git` invocations, many of them
redundant and many serial when they could be parallel.

Each `git` invocation carries ~30–80 ms of pure fork/exec + config-load
overhead on macOS *before* any real work. At 30 spawns that is 1–2.5 s of
unavoidable overhead alone, on top of the actual graph traversal and (out of
scope here) the biscuit-terminal image rasterization.

This spec covers only the costs **owned by the worktree package area**. The
Mermaid→SVG→image rasterization path lives in `biscuit-terminal` /
`biscuit-visualized` and is intentionally out of scope (tracked separately).

Reader's note: this review keeps the existing public UX contract intact: status
rows always render, verbose text still renders on non-image terminals, and the
graph remains an opportunistic image-only enhancement. The implementation should
therefore optimize data gathering and subprocess orchestration without changing
what `wt list` means.

## Current-State Analysis

Trace of `wt list -v` from a non-main worktree, with file:line references and
subprocess counts. "Parallel" means dispatched across `std::thread::scope` in
`list_worktrees`; "serial" means one-after-another on the main thread.

### A. `list_worktrees` (`worktree/lib/src/worktree.rs:132`) — reasonably parallel

| Call                                                    | Where       | Count        | Mode     |
| ------------------------------------------------------- | ----------- | ------------ | -------- |
| `git worktree list --porcelain`                         | worktree.rs:133 | 1         | serial   |
| `git symbolic-ref …origin/HEAD` (`default_branch`)      | worktree.rs:59  | 1         | serial   |
| `git status --porcelain` per worktree (`dirty_status`)  | worktree.rs:253 | N         | parallel |
| `git rev-list --count` per non-main (`ahead_behind`)    | worktree.rs:302 | N-1       | parallel |
| `git merge-tree --write-tree` (only if diverged)        | worktree.rs:322 | 0..N-1    | parallel |

This block is already well-structured: per-worktree work is parallelized and
`dirty_status` runs concurrently on a sub-thread. `merge-tree` is correctly
gated behind `ahead>0 && behind>0`. **No change required here** except
surfacing the `default_branch` result to downstream callers (see B).

### B. Graph generation (`worktree/cli/src/commands/list.rs:27` → `git_graph.rs`) — redundant + serial

| Issue | Detail |
| --- | --- |
| **Graph data computed even when no image can render** | `graph_instructions(&statuses)` runs unconditionally (list.rs:27). The `fits` check (list.rs:38) tests terminal *width* only; `image_terminal` and its `ImageSupport` detection are built afterward (list.rs:44). On any non-image terminal (plain SSH, piped output, unlisted emulator) **all** graph git calls run and are then thrown away. |
| **`default_branch()` called 3× per run** | `list_worktrees` (worktree.rs:135), `graph_instructions` (list.rs:58), `render_verbose` (list.rs:93). Each spawns `git symbolic-ref` (+ possible `git rev-parse` fallbacks). Result is identical across the three calls. |
| **`merge-base` called 3× for the same branch pair** | `worktree_graph` (`worktree/cli/src/commands/git_graph.rs:226`), `merge_base_commit` (`git_graph.rs:14`), `branch_commits_detail` (`git_graph.rs:21`). In verbose non-main mode all three execute for the identical `<default, current>` pair. |
| **`short_sha()` spawns a subprocess to truncate a string** | `git_graph.rs:218`. `git rev-parse --short <sha>` is only used to match the `%h` shape emitted by `git log --format=%h`; the fallback already does pure truncation. Called once per branch in `base_graph` (`git_graph.rs:283`). |
| **`base_graph` per-branch queries are serial** | `git_graph.rs:275-295` iterates branches sequentially; each iteration runs `git merge-base` + `git rev-parse --short` + `git log` (3 serial subprocesses × N branches). `list_worktrees` already demonstrates the `thread::scope` pattern this should follow. |
| **Graph + verbose query overlapping data independently** | `graph_instructions` (list.rs:27) and `render_verbose` (list.rs:51) run back-to-back but each issue their own `git log` / `git merge-base` for overlapping commit sets. A single gather pass would collapse ~4 calls into shared structure. |

### C. Minor

- `ancestor_commits` / `commits_since` reverse output in Rust (git_graph.rs:193, 205); `git log --reverse` does this for free.
- `ancestor_commits` and `commits_since` are near-identical helpers (unifying is a maintainability win, not a speedup).
- `parse_commit_lines` also reverses verbose commit details in memory; it should
  be brought under the same oldest-first query contract when the log helpers are
  consolidated.

## Requirements

### R1 — Skip the entire graph path when images cannot render

`run()` must detect image support **before** calling `graph_instructions`, and
skip graph generation, width sizing, and `MermaidDiagram` rendering entirely
when `ImageSupport::None`. Detection uses the existing cheap env-based logic
(`detect_image_support_from_env`, list.rs:155).

- No graph-only `git` call in `git_graph.rs` may execute when the terminal
  cannot render images.
- `--verbose` is not graph-only. It must still render textual commit details on
  non-image terminals, using only the verbose data it needs.
- Behavior is unchanged on image-capable terminals.
- Width parsing should happen before graph generation, but width fitting should
  be checked before building Mermaid instructions. For character widths, skip
  graph data when `terminal.width() < MIN_GRAPH_TERMINAL_WIDTH`; for percentage
  and fill widths, rely on image support and existing sizing behavior.

### R2 — Compute `default_branch` exactly once per `wt list`

The default-branch name resolved in `list_worktrees` must be surfaced to
`graph_instructions` and `render_verbose` rather than re-derived. Exactly one
`git symbolic-ref` (or fallback) call per run, regardless of `--verbose`.

Implementation decision: change the library surface to return a small list
snapshot, for example:

```rust
pub struct WorktreeList {
    pub default_branch: String,
    pub statuses: Vec<WorktreeStatus>,
}
```

`list_worktrees()` may either return `WorktreeList` directly, or a new
`list_worktrees_with_default()` can be added while preserving `list_worktrees()`
as a compatibility wrapper. The preferred implementation is the direct return
type change because this package is pre-1.0, the CLI is the primary consumer,
and the new type makes the invariant explicit.

### R3 — Compute each `merge-base` exactly once per branch pair

`worktree_graph`, `merge_base_commit`, and `branch_commits_detail` must share a
single `git merge-base` result for a given `<default, branch>` pair within one
run.

Implementation decision: introduce an internal graph-data gather step that owns
the per-run cache, rather than adding global state. A sketch of the shape:

```rust
struct BranchGraphData {
    branch: String,
    merge_base_full: String,
    merge_base_short: String,
    default_context: Vec<String>,
    default_after_base: Vec<String>,
    branch_after_base: Vec<String>,
    merge_base_detail: Option<CommitDetail>,
    branch_details: Vec<CommitDetail>,
}
```

The exact names are flexible, but the key contract is not: graph and verbose
rendering must consume the same gathered values for the current branch instead
of each helper querying git independently.

### R4 — Eliminate the `short_sha()` subprocess

SHA shortening must be done in-process. Fetch full SHAs for identity checks,
compare full SHAs internally, and derive display IDs by truncating in Rust so
the per-branch truncation subprocess disappears entirely.

- Note: `git rev-parse --short` selects a length based on uniqueness. The new
  approach must preserve the existing branch-placement behavior in
  `base_graph` (`git_graph.rs:285`), but it should do so by comparing full SHAs
  instead of comparing a merge-base abbreviation against `%h`-formatted main
  commits.

Reader's note: avoid `git rev-parse --short` and do not introduce non-portable
shortening assumptions. The recommended fix is to stop comparing abbreviated
SHAs: fetch graph commit lists with full hashes (`%H`) for identity and derive a
display ID in Rust with a shared `display_sha(&full_sha)` helper. Mermaid commit
IDs can remain short-looking, but branch placement must use full-SHA equality.
This removes the subprocess and avoids relying on git's repository-dependent
abbreviation length for correctness.

### R5 — Parallelize `base_graph` per-branch queries

The per-branch `BranchData` collection in `base_graph` (merge-base +
commits_since, per branch) must run concurrently across branches, following the
`std::thread::scope` pattern already used in `list_worktrees`.

- Preserve deterministic output. Collect branch results concurrently, then sort
  by `(merge_base_idx, branch name)` before rendering so branch order does not
  depend on thread scheduling.
- Errors for one branch should continue to degrade that branch only. A single
  missing merge-base or failed log query must not suppress the whole graph if
  other branches have usable data.

### R6 — Share gathered commit data between graph and verbose rendering

When both the graph and verbose paths need commit data for the current branch,
gather it once into a shared structure consumed by both, rather than each path
re-issuing `git log` / `git merge-base`.

- If image rendering is unavailable but `--verbose` is set, gather only the
  current branch's verbose data. Do not gather base-graph branch data in this
  case.
- If image rendering is available and `--verbose` is set on a feature branch,
  gather the current branch once with both graph commit IDs and verbose commit
  details.
- If image rendering is available and the current checkout is the base branch,
  gather base-graph branch data only; verbose mode remains a no-op on base per
  the existing docs.

### R7 — (Minor) Prefer `git log --reverse` over in-process reversal

`ancestor_commits`, `commits_since`, `commit_details`, and
`commit_details_since` should pass `--reverse` to `git log` where they require
oldest-first output and drop the in-memory `.reverse()`.

### R8 — Add a worktree performance testing contract

This package currently has no `worktree/docs/performance-testing.md`, which is
an implicit opt-out under the repo testing strategy. Because this spec is
explicitly performance-driven, implementation must add that document and include
at least these H2 sections:

- `## List Status Collection`
- `## Graph Data Collection`
- `## Verbose Commit Details`

The document should state that rasterization is excluded from worktree-owned
benchmarks. If Criterion benches are added in the same implementation, wire them
through the existing package `just bench` path; otherwise, document the intended
bench surfaces so a follow-up can add them without rediscovering scope.

### R9 — Add subprocess-count regression coverage

The optimization depends on subprocess orchestration, not just wall-clock time.
Add test-only instrumentation around the `git_command` / `git_command_in`
boundary, or a narrow fake-git integration test, so tests can assert:

- no graph-only git commands run when image support is unavailable;
- one default-branch resolution is used per list snapshot;
- one merge-base is used for the current branch when graph and verbose data are
  both needed.

The test hook must be compiled only for tests or hidden behind an internal
feature; production CLI output and public API should not expose debug counters.

## Non-Goals

- **Out of scope:** Mermaid/SVG rasterization performance in `biscuit-terminal`
  / `biscuit-visualized`. This is the single largest wall-clock cost *when a
  graph actually renders*, but it is not owned by this package and will be
  addressed separately.
- **Out of scope:** Changing the semantics of the "dirty" badge (e.g. whether
  untracked `??` files count as dirty). `dirty_status` already uses
  `core.untrackedCache=true` and breaks early on the first source file; the
  untracked-walk cost is inherent unless semantics change. A future decision
  may offer a `--untracked-files=no` fast path, but that is a behavior change,
  not covered here.
- **Out of scope:** Replacing per-call `git` subprocess spawning with a
  long-lived `git` process or a git2 library binding. Larger architectural
  change; not justified by these wins alone.

## Acceptance Criteria

1. On a non-image-capable terminal, plain `wt list` issues **zero** graph-data
   `git` calls from `git_graph.rs` (verifiable by tracing/git strace or a debug
   counter).
2. `wt list -v` resolves `default_branch` exactly **once** and each
   `merge-base` for the current branch exactly **once**.
3. `short_sha()` no longer invokes a `git` subprocess.
4. In `base_graph`, branches are queried concurrently; wall-clock for the
   per-branch graph-data phase scales with the slowest branch query batch, not
   the branch count, while rendered branch order remains deterministic.
5. No change to user-visible output on image-capable terminals: identical table,
   identical graph, identical verbose section.
6. Existing unit tests (`parse_worktree_list`, `DirtyFiles`, `porcelain_path`,
   etc.) pass unchanged; the helpers that change signature gain updated tests.
7. On a warm cache in the `rusty-biscuit` monorepo, `wt list` (non-image
   terminal) and `wt list -v` (image terminal) meet the repo-wide 1-second SLA
   defined in `sniff/fixes/_completed/2026-04-21-performance/spec.md`, excluding
   the biscuit-terminal rasterization step.
8. `wt list -v` on a non-image terminal still prints the verbose textual commit
   section for a non-main worktree.
9. `worktree/docs/performance-testing.md` exists and describes the worktree-owned
   benchmark scope introduced by this spec.

## Open Questions

No blocking design questions remain after review. The spec now makes the
important decisions locally: preserve verbose output on non-image terminals,
surface the default branch through a list snapshot, use per-run graph data
instead of global caches, and compare full SHAs internally while deriving short
display IDs in process.
