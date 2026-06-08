---
status: ready for planning and implementation
reviewed: true
---

# Migrating sniff from `git2` to `gitoxide` (`gix`)

**Date:** 2026-06-06
**Status:** Reviewed and ready for planning and implementation
**Scope:** `sniff/lib` and `sniff/cli` production source. Test/bench fixture
builders are addressed where the migration forces a change.
**Companion inputs:** `current-usage.md` (factual `git2` inventory + review
findings R1–R12 in the same directory).

**Review note:** This inline review makes the following design decisions explicit:

- `sniff-cli` will not depend directly on `gix`; Git discovery and remote/ref
  queries currently implemented in the CLI move behind library APIs.
- The migration pins `gix` to `=0.84.0` for planning. Changing that version is a
  separate dependency update that must revalidate features, APIs, and this spec.
- Repository opening preserves the current ownership-validation behavior by
  rejecting untrusted repositories and surfacing that failure distinctly from
  "not a repository."
- sniff remains SHA-1-only for behavior parity with `git2` 0.20. SHA-256
  repository support is not introduced by this migration.
- Criterion regressions are decided by before/after runs on the same host and
  toolchain. Committed timing tables are audit records, not portable thresholds.

This spec defines a behavior-preserving migration of sniff's **read-only** git
usage from the C-backed `git2` (libgit2) bindings to the pure-Rust `gix` crate.
It bakes in the review findings (R1–R12) where they reduce migration surface or
codify intended semantics, and it gates the migration on **equal-or-better
performance** proven by Criterion benchmarks with a saved git2 baseline.

The single write path — an out-of-process `git fetch` subprocess — is an explicit
**non-goal** and stays untouched (R10).

---

## 1. Goals, Non-Goals, Constraints

### 1.1 Goals

- **Behavior-preserving.** Every `sniff` and `sniff repo …` output that depends
  on git must be byte-identical (modulo the deliberate correctness fixes below)
  to the current git2-backed output for representative repositories.
- **Read-only.** All production git access remains read-only. No object DB or ref
  writes are introduced.
- **Cross-platform.** macOS, Linux, Windows must all build and pass parity tests.
  `gix` is pure Rust, which removes the vendored-libgit2 C build (a cross-compile
  and static-link win) but introduces a `gix-sec` trust evaluation on open (see
  §4) and a bytes-first ref/path model (see §3) that must be handled on all three
  OSes.
- **Preserve the library/CLI boundary.** Git backend types and operations live in
  `sniff/lib`. `sniff/cli` may request repository roots, preferred remotes,
  branch history, and commit URLs through public library types, but must not
  import `gix` or expose backend-specific errors.
- **Equal-or-better performance.** The hot paths (discovery, status, revwalk,
  diff, ancestry, worktree fan-out) must show **no statistically significant
  regression** versus the saved git2 baseline. Several paths are expected to
  *improve* (commit-graph, lazy `Id` decode — R2).
- **Deliberate review fixes folded in.** R1 (discover-once handle threading), R2
  (lazy revwalk + commit-graph), R6 (per-operation error policy), R7 (explicit
  non-UTF-8 policy) are implemented as part of the migration rather than deferred.

### 1.2 Non-Goals

- **Do NOT port `git fetch` to `gix` networking** (R10/R11). The fetch stays a
  `std::process::Command::new("git") … fetch --quiet --prune` subprocess so it
  continues to inherit the user's credential helpers, SSH agent, proxy, and
  `url.*.insteadOf` rules. `gix` networking is opt-in, requires a TLS-backend
  choice, and `gix-credentials` does not transparently match a user's full `git`
  config. The migration of the *surrounding* read code (remote enumeration,
  `workdir`) is independent of the fetch and leaves `fetch_single_remote`
  (`lib/src/filesystem/git/remote_refresh.rs:350-360`) unchanged.
- **No new git capabilities.** No blame, no merge driver changes beyond the
  existing conflict probe, no clone/push.
- **No behavioral feature additions.** R8/R9/R12 robustness improvements are
  optional follow-ups noted at the relevant phase, not required exit criteria.
- **No SHA-256 repository support.** The current `git2` 0.20 implementation is
  SHA-1-only. Enabling and validating `gix` SHA-256 support would be a separate
  public behavior change with fixture, serialization, and CLI compatibility
  work.

### 1.3 Constraints

- **Pin `gix` exactly.** Use `=0.84.0`. `gix` is pre-1.0 ("initial development /
  usable" per `crate-status.md`), so even minor updates may change signatures or
  feature composition. A later version bump must be reviewed as a separate
  dependency change; do not silently choose "latest" during implementation.
  Treat snippets here as API shape until checked against 0.84.0.
- **Toolchain floor.** `gix` requires Rust 1.82, while Rust edition 2024 requires
  Rust 1.85. The effective floor for both sniff crates is therefore Rust 1.85.
  Verify the workspace build and CI images provide Rust 1.85 or newer. This
  migration does not independently introduce a lower `rust-version` contract.
- **`default-features = false` + minimal allowlist.** Library authors over-compile
  `gix`. Start from no default features and add only what the inventory proves is
  used. The exact allowlist is derived in §2.2.
- **Thread-local repository handle.** `gix::Repository` is always `!Sync`.
  With `default-features = false`, the `parallel` feature makes the relevant
  repository handles `Send`; shared access still requires conversion through
  `ThreadSafeRepository`. The worktree fan-out (Rayon) therefore needs the
  deliberate `into_sync()` / `to_thread_local()` design in §2.3.
- **Object cache is OFF by default** in `gix` and the wrong size *hurts*
  (`gitoxide.md` §Caching). Any path that decodes the same objects repeatedly must
  size the cache via `compute_object_cache_size_for_tree_diffs` or
  `object_cache_size_if_unset`.

---

## 2. Dependency Changes

### 2.1 Manifest edits

**`sniff/lib/Cargo.toml`** — replace line 17:

```toml
# remove:
git2 = "0.20.3"

# add:
gix = { version = "=0.84.0", default-features = false, features = [
    "sha1",         # preserve the current SHA-1-only repository contract
    "revision",     # rev_walk, rev_parse, merge-base, ahead/behind graph ops
    "status",       # repo.status / is_dirty / is_pristine (index↔worktree↔HEAD)
    "blob-diff",    # tree↔tree and index↔worktree blob/line diffs
    "dirwalk",      # untracked-file discovery for status (recurse-untracked)
    "excludes",     # .gitignore evaluation needed by dirwalk
    "parallel",     # makes handles Send and enables threaded algorithms
] }
```

**`sniff/cli/Cargo.toml`** — remove the runtime `git2` dependency and do not add
`gix`:

```toml
# remove from [dependencies]:
git2 = "0.20"

# retain in [dev-dependencies] for fixture construction:
git2 = { version = "0.20", default-features = false }
```

The CLI currently performs repository discovery, preferred-remote selection,
remote lookup, commit URL construction, and branch-history queries directly.
Move those operations into `sniff/lib` as backend-neutral queries on `GitRepo`
or small public result types. The CLI remains responsible only for argument
handling, provider selection, and rendering.

### 2.2 Feature allowlist derivation

Each enabled feature maps to a proven inventory operation. **No network feature is
enabled** (R10/R11): sniff uses no libgit2/`gix` networking; fetch stays a
subprocess.

| `gix` feature | Justifying inventory operation(s) | Inventory ref |
|---|---|---|
| `sha1` | current repository object format; preserves `git2` 0.20 behavior | §1.2 |
| `revision` | revwalk (`rev_walk`), `rev_parse_single`, `graph_ahead_behind`, `graph_descendant_of`, merge-base | §6.1, §6.3 |
| `status` | `statuses` / `StatusOptions`, `is_empty`/`count`, blast-radius status | §5.1 |
| `blob-diff` | `diff_tree_to_tree`, `diff_tree_to_index`, `diff_index_to_workdir`, `Diff::print(Patch)`, `Diff::deltas` | §5.1–5.3 |
| `dirwalk` | `include_untracked(true)` + `recurse_untracked_dirs(true)` on every status walk | §5.1 |
| `excludes` | `.gitignore` evaluation required by `dirwalk` to classify untracked | §5.1 |
| `parallel` | worktree fan-out under Rayon + top-level scoped threads need `Sync` repo via `into_sync()` | §6.4, §11 |

Features **deliberately NOT enabled** (and why):

- No `blame` — sniff has no blame call site.
- No `merge` *unless* §5/R5 chooses to keep an in-memory 3-way merge. See
  §3 (worktrees) and §8 Phase 6: if the R5 short-circuit removes the merge for
  merged branches and an early-abort conflict mode is unavailable, `merge` is
  added then; otherwise it stays off. **Default: off**, added only if the
  conflict-probe port requires it.
- No `blocking-network-client` / `async-network-client` / any HTTPS transport /
  `credentials` — fetch stays a subprocess (R10).
- No `worktree-archive` / `worktree-stream` / `mailmap` / `attributes` (beyond
  what `excludes`/`dirwalk` pull) — unused.
- No `sha256` — this migration preserves the current SHA-1-only contract.

### 2.3 `!Sync` / `ThreadSafeRepository` decision

`gix::Repository` is a thread-local, `!Sync` handle. With default features
disabled, `parallel` is required to make the relevant handles `Send`.
`ThreadSafeRepository` is the shared container and is converted to a
thread-local `Repository` in each worker. sniff has two multi-threaded contexts:

1. **Top-level domain concurrency** (`detect_with_plan` scoped threads) — the git
   work runs inside the filesystem domain's thread; the handle is created and used
   on one thread, so `Send` (always available) suffices there.
2. **Worktree fan-out** (`get_worktrees`, Rayon) — needs to share a base-repo
   handle across worker threads. Under git2 this is worked around by opening a
   fresh `Repository` per thread (the documented `!Sync` workaround,
   `remote_refresh.rs:567,602`). With `gix` + `parallel`, open the base **once**,
   call `repo.into_sync()` → `ThreadSafeRepository`, and call `to_thread_local()`
   per Rayon worker (R4). This removes the per-worktree base re-open.

**Decision:** enable `parallel` in `sniff/lib`. It makes the relevant handles
`Send` and turns on `gix`'s internal multi-threaded algorithms. For worktree
fan-out, convert the opened `Repository` with `into_sync()` and create one
thread-local handle per Rayon worker with `to_thread_local()`. The CLI has no
backend dependency.

### 2.4 Test/dev fixture builders

`lib/benches/support/builder.rs` and several `#[cfg(test)]` modules use git2
**write** APIs (`init`/`commit`/`Index`/`Signature`/`Time`/worktree/`reference`)
to build fixtures (inventory §10). Two options:

- **Option A (recommended): keep git2 as a `[dev-dependencies]` fixture builder.**
  Production code uses `gix`; fixtures keep using git2's mature write API. This
  decouples fixture migration from production migration and lets the **git2
  baseline benches and the gix benches read the same on-disk fixtures**, which is
  exactly what a fair before/after comparison needs. Add
  `git2 = { version = "0.20", default-features = false }` to
  `sniff/lib` and `sniff/cli` `[dev-dependencies]` (it is already there for the
  CLI). Production `[dependencies].git2` is removed.
- **Option B: port fixtures to `gix` write APIs** (`index`,
  `worktree-mutation`, `tree-editor`). More churn, no correctness benefit,
  and complicates baseline comparison. **Rejected** for this migration.

Under Option A, the CLI keeps only its git2 dev-dependency for fixtures. It has no
runtime or dev dependency on `gix`.

---

## 3. Per-Operation Migration Mapping

All examples assume an opened handle (`let repo = gix::discover(path)?;` for entry
points, or a threaded `&gix::Repository` after R1). API is pre-1.0 — confirm
signatures against the pinned version. The `R/W` column is read-only throughout
except the flagged subprocess fetch.

| # | git2 operation (inventory ref) | `gix` equivalent | Gotchas / levers |
|---|---|---|---|
| 1 | `Repository::discover` (§4, R1) | `gix::discover(path)` → `Repository` | Walks parents like git2. Adds `gix-sec` trust eval on open (see §4). `gix::discover::Error` is its own type (R6). |
| 2 | `Repository::open(path)` (§6.4) | `gix::open(path)` | No parent walk. Use for known worktree paths. |
| 3 | `Repository::workdir` (14×) | `repo.workdir()` → `Option<&Path>` | git2 trailing-separator quirk (CLI `mod.rs:662`) — re-verify the CLI `repo root` output; `gix` may not add a trailing separator. Behavior-parity test required. |
| 4 | `Repository::path` (§6.4) | `repo.path()` → `&Path` (the `.git` dir/gitdir) | — |
| 5 | `Repository::commondir` (§4.1, §6.2, §6.4) | `repo.common_dir()` → `&Path` | Used for worktree base resolution. |
| 6 | `Repository::is_worktree` (6×) | `repo.worktree().is_some()` is the linked-worktree check; `repo.is_bare()`/`repo.work_dir()` differentiate | Confirm the exact predicate against pinned `gix`: `gix` distinguishes "this handle is a linked worktree" via the worktree platform. Parity-test `in_worktree()`. |
| 7 | `Repository::head` (14×) → `Reference::shorthand`/`is_branch`/`target`/`peel_to_commit` | `repo.head()?` → `gix::Head`; `repo.head_ref()?`, `repo.head_id()?`, `repo.head_name()?`; `reference.peel_to_id_in_place()` / `.into_fully_peeled_id()` | `head_id()` is the cheap "HEAD oid" path. Detached HEAD: `head()?.is_detached()`. Branch shorthand from `head_name()` (`Option<FullName>` → strip `refs/heads/`). |
| 8 | `Repository::head_detached` (§6.4) | `repo.head()?.is_detached()` | — |
| 9 | `Repository::find_reference(name)` (3×) | `repo.find_reference(name)?` | Returns `gix::Reference`. Name is `&BStr`-compatible. |
| 10 | `Repository::references` (full scan, §6.2) | `repo.references()?.all()?` (iterator of `Result<Reference>`) | Cheap: packed-refs + loose, **no decode**. Peeling is where cost lives (R9 — size object cache). |
| 11 | `Repository::references_glob(prefix)` (3×) | `repo.references()?.prefixed(prefix.into())?` | `prefixed` takes a `&BStr` prefix (e.g. `b"refs/remotes/origin/"`). |
| 12 | `Repository::find_branch(name, Local)` (3×) | `repo.find_reference("refs/heads/<name>")?` or `repo.try_find_reference(...)?` | `gix` has no `Branch` type; branches are references under `refs/heads/`. |
| 13 | `Repository::branches(Local)` (§6.3) | `repo.references()?.local_branches()?` (iterator) | **Do not** use `repo.branch_names()` — that reads `branch.*` config, not the ref store (`gitoxide.md` §branch gotcha). |
| 14 | `Repository::find_commit(oid)` (8×, R2) | `id.object()?.into_commit()` or `repo.find_commit(id)?` | **Decode lazily.** Only call where the body is consumed. For time/parents-only gates use the commit-graph (row 17). |
| 15 | `Repository::revwalk` + `push_head`/`push`/`hide`/`set_sorting`/`take`/`count` (6 walks) | `repo.rev_walk(tips)` → `revision::walk::Platform`; `.all()?` yields cheap `Id`s; `.first_parent()`, sorting via platform options; `.count()` for tallies | The walk yields **IDs**, not commits (R2 upside). `Sort::TIME`→ default newest-first; confirm sort knobs map (`gix` uses `Sorting::ByCommitTimeNewestFirst` etc.). `hide` = pass tips to exclude / use a revspec range. |
| 16 | `git2::Sort::{TIME, TOPOLOGICAL}` (§6.1) | `gix::revision::walk::Sorting::{ByCommitTimeNewestFirst, Topological}` (confirm names) | The `recent_commits.rs:371` early-`break` on time cutoff requires the time-newest-first ordering — preserve it. |
| 17 | (no git2 equivalent) commit-graph (R2) | `repo.commit_graph_if_enabled()?` → `Option<gix::commitgraph::Graph>` | **Primary `gix` upside.** Read commit time + parents without decoding the object. Apply to the two timestamp-only gates (`recent_commits.rs:385`, `remote_refresh.rs:443`). |
| 18 | `Repository::revparse_single(spec)` (3×) | `repo.rev_parse_single(spec)?` → `Id` | Cheap (graph/ref lookups). |
| 19 | `Repository::diff_tree_to_tree(a, b)` (§5.3, 2×) | `repo.diff_tree_to_tree(&tree_a, &tree_b, options)?` or a `diff::resource_cache()` for repeated diffs across a walk (R3) | Reuse one `resource_cache` across a range walk (R3). Sizing: `compute_object_cache_size_for_tree_diffs(&index)`. |
| 20 | `Repository::diff_tree_to_index` (§5.1) | tree↔index diff via the diff platform with the HEAD tree and `repo.index()` | Used for `git diff --cached`. Confirm the exact `gix` index-diff entry on pinned version. |
| 21 | `Repository::diff_index_to_workdir` (§5.1) | index↔worktree changes are produced by the **status** platform (`repo.status(...)`); the worktree diff lines come from the status change set | `gix` folds index↔worktree into `status`. The current two-diff aggregation (`status.rs:62-65`) must be re-expressed against `repo.status(...)` + `diff_tree_to_index` (see §5.1 redesign note). |
| 22 | `Diff::print(DiffFormat::Patch, cb)` (§5.2) | iterate the diff platform's per-change `UnifiedDiff` / line interner; `gix-diff` emits hunks/lines you format yourself | No single `print(Patch, closure)`. Build the patch text from the change iterator. Preserve the **single-pass** property (`diff.rs` doc): one walk drives both stats and patch text. |
| 23 | `Diff::deltas` + `DiffDelta::{new_file,old_file,status}` (§5.3) | the change iterator yields per-path `Change` with `location` (BStr path) + a kind (Added/Deleted/Modified/Rewrite) | Path is `BStr` (R7). Map kinds → `DeltaKind` (`discovery.rs:325`). |
| 24 | `git2::Delta::{Added,Deleted,Renamed,Copied}` (§5.3) | `gix` change kinds; rename/copy only if rename tracking enabled | **Rename detection is OFF** in sniff (default) — tests assert rename→delete+add (`status.rs:598`). Keep tracking off so the delete+add pairing is preserved. |
| 25 | `DiffLine::{origin,content}` (§5.2) | line tokens from `gix-diff` (`+`/`-`/context) + byte content | Content is bytes (R7 — `to_string_lossy`). |
| 26 | `StatusOptions::new().include_untracked(true).recurse_untracked_dirs(true)` (4×) | `repo.status(gix::progress::Discard)?` → configure the `status::Platform`: untracked via `UntrackedFiles::Files` + dirwalk recursion | **`status()` requires a progress arg** — pass `gix::progress::Discard`. Needs `status` + `dirwalk`/`excludes` features. Loaded index: `repo.index_or_empty()` to tolerate no index. |
| 27 | `repo.statuses(...).is_empty()/iter().count()` (4×) | iterate `platform.into_iter()?` (yields `status::Item`); for the dirty *flag* use `repo.is_dirty()?` (fast short-circuit) | **Use `is_dirty()` for `summary()`** (branch + dirty flag only) — it bails on first change. Full file changes use the iterator. This is the summary-vs-full split the bench matrix gates (§5). |
| 28 | `Status::is_index_*/is_wt_*` flag methods (4 sites) | match on `status::Item` / `status::index_worktree::Item` variants (Added/Modified/Deleted/Untracked, with index vs worktree side) | Map the variant set onto sniff's staged/unstaged/untracked categorization. Parity-test the per-category counts. |
| 29 | `Repository::index` + `Index::conflicts` + `IndexConflict`/`IndexEntry::path` (§5.1) | `repo.index()?` → `gix::index::File`; iterate entries, detect conflict stages (stage 1/2/3) | `gix` exposes index entries with stage; "has conflicts" = any entry at stage > 0. Path is `BStr` (R7 — currently coerced to `""` on non-UTF-8, `status.rs:398`). |
| 30 | `Repository::config` + `Config::get_string`/`get_bool` (12 keys) (§6.3) | `repo.config_snapshot()` → `snapshot.string(key)` / `.boolean(key)` | All production opens reject untrusted repositories (§4), so config output cannot silently differ because sensitive values were filtered. |
| 31 | `Config::add_file(path, ConfigLevel::ProgramData, false)` (§6.3) | layer an extra config source via `gix::config` open options, or read the ProgramData/CLT gitconfig directly and merge | `gix` config layering differs from libgit2's `add_file`. Confirm the `gix-config` API for adding a system-level file to the in-memory snapshot; if unavailable, read the file via `gix-config` directly and merge keys in sniff. Parity-test the 12 keys on macOS + Windows. |
| 32 | `Repository::remotes` (5×) | `repo.remote_names()` → set of `BString` | Cheap config read. |
| 33 | `Repository::find_remote(name)` + `Remote::url(Direction)` (6×) | `repo.find_remote(name)?` → `remote.url(gix::remote::Direction::Fetch)` → `Option<&gix::Url>` | URL is a `gix::Url`; `.to_bstring()` / display for the string form. |
| 34 | `Repository::graph_ahead_behind(a, b)` (3×) | `repo`'s revwalk-based count: walk `a` hiding `b` and vice versa, or use `gix`'s ahead/behind helper if present in pinned version | Confirm whether pinned `gix` exposes a direct ahead/behind; otherwise compute via two bounded `rev_walk` counts. Gated by `wants_repo_metadata()` (`types.rs:697`) — preserve the gate. |
| 35 | `Repository::graph_descendant_of(a, b)` (2×) | merge-base test: `b` is an ancestor of `a` iff `merge_base(a,b) == b`; or an ancestry `rev_walk` from `a` hiding nothing, checking for `b` | `gix` `revision` feature provides merge-base. Use commit-graph for the ancestry walk (R2). |
| 36 | `Repository::merge_commits(a, b, None)` + `Index::has_conflicts` (§6.4, R5) | `repo.merge_commits(...)` (feature `merge`) **only if** the R5 short-circuit cannot avoid it; check `gix-merge` for an early-abort/"first-conflict" mode | **R5 first:** if `graph_descendant_of(base, wt)` is already true, set `has_conflicts = false` without merging. Only unmerged branches reach the merge. Add `merge` feature only if this path survives. |
| 37 | `Repository::worktrees` (2×) | `repo.worktrees()?` → list of worktree proxies | — |
| 38 | `Repository::find_worktree(name)` + `Worktree::path` (2×) | from `repo.worktrees()?` find by name → `.base()` / path accessor | Confirm the path accessor name on pinned version. |
| 39 | `git2::Oid` (`from_str`/`to_string`/`Eq`/HashMap key) (many) | `gix::ObjectId` (`from_hex`/`to_hex`/`Hash`/`Eq`) | Use as a HashMap key without assuming a fixed hex width in parsing or formatting code. **R3:** stop the `Oid→String→Oid` round-trip and pass `ObjectId` directly. SHA-256 repositories remain unsupported by policy (§1.2). |
| 40 | `git2::Error` `#[from]` (error.rs:15) | per-operation `gix` errors (`gix::discover::Error`, `gix::status::Error`, `gix::revision::walk::Error`, …); 0.50+ `gix-error`/`Exn` model | **Cannot be ported one-to-one** (R6). See §4. |

### Cross-cutting gotchas to honor (from `gitoxide.md`)

- **status needs a progress arg** — always pass `gix::progress::Discard`.
- **Object cache OFF by default** — for repeated object access (range-walk body
  decode, peel-per-ref decorations) call `repo.object_cache_size_if_unset(size)`
  with `size` from `compute_object_cache_size_for_tree_diffs(&index)`.
- **Lazy decode** — revwalk yields `Id`s; only `.object()` the ones rendered.
- **Trust model** — open with `bail_if_untrusted()` to preserve libgit2 ownership
  validation, and map that failure separately from repository absence (see §4).
- **Bytes-first** — ref names and paths are `BStr`/`BString`; UTF-8 is the
  caller's conversion (R7).
- **Integrity checks differ** — `git2` verifies hashes/objects strictly by
  default; `gix` currently does less. Note in §8 risks.

---

## 4. Error Handling Strategy

The `#[from] git2::Error` blanket (`error.rs:15`) is the **most migration-coupled
finding** (R6). `gix` has no unified error — each operation returns its own enum.

### 4.1 New `SniffError` shape

Replace the single `Git(#[from] git2::Error)` variant with a shape that
preserves the *failing operation* and distinguishes "absent → None" from "real
error → surface":

```rust
// lib/src/error.rs — replace the Git variant
/// Git operation failed. Wraps the underlying gix error as a boxed source
/// so the per-operation type is preserved in the cause chain without
/// enumerating every gix error enum in SniffError.
#[error("Git error during {operation}: {source}")]
Git {
    /// The conceptual operation that failed (e.g. "discover", "status",
    /// "revwalk", "diff", "config"). Drives debuggability without a
    /// per-enum variant explosion.
    operation: &'static str,
    #[source]
    source: Box<dyn std::error::Error + Send + Sync + 'static>,
},
```

Rationale: a `Box<dyn Error>` source keeps the concrete `gix` error in the cause
chain (so `color-eyre`/`Display` still show it) while a single `operation: &str`
tag gives the actionable "which op" context the blanket `#[from]` erased. This
avoids a 10-variant enum churn for a read-only consumer and keeps `?` ergonomic
via small `map_err` helpers.

Provide thin helpers so call sites stay terse:

```rust
impl SniffError {
    pub(crate) fn git(operation: &'static str, e: impl std::error::Error + Send + Sync + 'static) -> Self {
        SniffError::Git { operation, source: Box::new(e) }
    }
}
// usage: repo.status(Discard).map_err(|e| SniffError::git("status", e))?;
```

### 4.2 Absent-repo vs real-error policy (R6)

Codify the policy the inventory found applied inconsistently:

- **Discovery (`gix::discover`).** Distinguish three outcomes instead of mapping
  everything to `NotARepository`:
  - **Genuine "not a repo"** (`gix::discover::Error` upward-search exhausted) →
    `Ok(None)` (the existing `GitRepo::discover` contract, `types.rs:510`).
  - **Trust / ownership failure** (`gix-sec` untrusted, the analogue of git2
    `ErrorCode::Owner` / `safe.directory`) → surface a distinct
    `SniffError::Git { operation: "discover", .. }` (do **not** report as "not a
    repository"). This is the R6 fix: a CI/container ownership mismatch must not
    masquerade as "no repo".
  - **Permission / I/O / corrupt `.git`** → surface as `SniffError::Git`.
  - Centralize opening in helpers that use
    `ThreadSafeRepository::discover_opts`/`open_opts` with a trust mapping whose
    open options set `bail_if_untrusted(true)`. Convert to a thread-local
    repository after the trust check, then match the resulting error variants
    rather than using `map_err(|_| NotARepository)`.
- **Optional reads (HEAD, upstream, branch shorthand).** Keep the
  downgrade-to-`None` for the *legitimately optional* cases (detached HEAD, no
  upstream, unborn branch). Match the "not found" variant specifically. On
  fallible public library APIs, permission, I/O, and corruption errors must
  return `SniffError::Git`; do not warn and then convert them to `None`. Existing
  infallible convenience accessors may retain `Option` behavior only by
  delegating to a new fallible query and documenting that errors are suppressed.
- **Trust strictness.** Use `bail_if_untrusted()` for every production
  `discover`/`open`, including known worktree paths. This preserves libgit2's
  ownership-validation contract and prevents sensitive config differences from
  creating backend-dependent output. A trust failure is a real error, not
  repository absence.

### 4.3 CLI layer

The CLI receives backend-neutral library results. Only a genuine `Ok(None)` emits
"Not a git repository"; trust, permission, I/O, and corruption failures render
the library error. No CLI function accepts `gix::Repository`,
`gix::ObjectId`, or a `gix` error type.

---

## 5. Performance Benchmarks (STRICT — hard requirement)

The migration is **gated** on equal-or-better performance proven by Criterion
with a saved git2 baseline. sniff already has a mature Criterion harness — this
section **extends** it, it does not reinvent it.

### 5.1 Where benches live and how they wire into `just bench`

- Bench entry point: `sniff/lib/benches/perf.rs` (`harness = false`, registered in
  `lib/Cargo.toml:67-69`).
- Domain cases live under `sniff/lib/benches/cases/`; the git cases are
  `cases/git.rs` (already present, registered in `perf.rs:38`).
- Fixture builders: `benches/support/builder.rs` (git2-based — kept as a
  dev-dependency per §2.4) + `benches/support/fixtures.rs` (`TempDir` wrappers).
- Runner: `just bench` → `cargo bench -p sniff --features network --bench perf`
  (`sniff/justfile:139-141`). The CI subset is `lib/benches/ci-bench-ids.txt`
  filtered by `just bench-ci` (`justfile:164-168`).

**New git micro-benches** go in a dedicated module to isolate the operations the
migration touches (the existing `cases/git.rs` measures *end-to-end*
`detect_git_with_request`, which is the right integration-level guard but too
coarse to localize a regression). Add `cases/git_ops.rs`, register it in
`perf.rs`, and add its bench IDs to `ci-bench-ids.txt`.

These micro-benches must run against fixtures with identical content and shape.
Because production code will be `gix` after the port, the comparison is temporal:
save the git2 baseline before the port and compare after each phase on the same
host, toolchain, build profile, and checkout. The micro-benches call sniff's
public Git APIs, exactly like the existing `cases/git.rs`. Fixture construction,
repository discovery, and commit-graph generation must occur outside the timed
iteration.

### 5.2 Benchmark matrix (one target per hot path)

Each row is a Criterion benchmark. Group names follow the existing
`configure_slow_group` convention; IDs are parameterized where a size sweep
matters (`BenchmarkId::new(name, param)`).

| Bench ID | Hot path | sniff entry point | Fixture | Parameters | Notes |
|---|---|---|---|---|---|
| `git_ops/discover` | Repo discovery (R1) | `GitRepo::discover` (`types.rs:512`) | `small_git_repo`, `large_monorepo` | repo shape | Measures parent-walk + open + trust eval. |
| `git_ops/status_dirty_flag` | Status summary (dirty flag only) | `get_repo_status_counts` (`status.rs:308`) via `GitRequest::summary()` | `git_repo_with_dirty_files(N)` | N ∈ {10,100,(1000)} | Should map to `is_dirty()` short-circuit (row 27). |
| `git_ops/status_file_changes` | Status full file changes | `get_repo_status_with_changes` (`status.rs:36`) via `GitRequest::full()` | `git_repo_with_dirty_files(N)` | N ∈ {10,100,(1000)} | Full index↔worktree↔HEAD + per-file stats. |
| `git_ops/revwalk_recent_gated` | Time-gated revwalk (R2) | `get_recent_commits_*` time path (`recent_commits.rs:368`) | `large_monorepo` (deep history) | since-cutoff | Should use commit-graph for the time gate (no body decode). |
| `git_ops/revwalk_recent_full` | Full-decode revwalk | recent commits with body render (`discovery.rs:144`) | `large_monorepo` | count=10 | Bodies decoded; object cache sized. |
| `git_ops/diff_commit_files` | Single-pass diff per commit (R3) | `get_commit_files` (`discovery.rs:390`) | `large_monorepo` | commits=10 | Reuse one `resource_cache` across the walk. |
| `git_ops/ancestry_containment` | Ancestry-walk containment | `populate_recent_commit_remotes` (`remote_refresh.rs:390`) | `git_repo_with_fake_remotes(20, R)` | R ∈ {1,5,10,25} | Existing `git_deep_remote` group already covers the integration form; this isolates the ancestry walk. |
| `git_ops/worktree_fanout` | Worktree fan-out (R4) | `get_worktrees` (`remote_refresh.rs:554`) | new `git_repo_with_worktrees(W)` fixture | W ∈ {1,4,8} | `ThreadSafeRepository` + `to_thread_local()` vs per-thread open. |
| `git_ops/config_read` | Config reads | `get_git_config` (`remote_refresh.rs:20`) | `small_git_repo` | — | 12 keys + ProgramData layering parity. |
| `git_ops/refs_enumerate` | Refs/branches enumeration (R9) | `collect_ref_decorations` (`discovery.rs:37`) + `get_local_branches` | `git_repo_with_fake_remotes(20, 25)` | — | Full refdb scan + peel-per-ref; object cache helps. |

The existing end-to-end groups (`git_dirty_scaling`, `git_deep_remote` in
`cases/git.rs`) stay as integration guards. The `git_ops/*` group is the
per-operation localizer.

### 5.3 Fixture strategy (reproducible, cross-platform)

- **Reuse the existing deterministic builders** (`builder.rs`): fixed signature
  (`Signature::new("Bench Runner", … Time::new(0,0))`), fixed file layouts, fixed
  commit messages → byte-stable repos across runs and machines.
- **Known-shape fixtures already exist:** `small_git_repo` (~10 files, 5 commits),
  `large_monorepo` (60 Rust + 30 JS pkgs, 21 commits — the deep-history fixture),
  `git_repo_with_dirty_files(N)`, `git_repo_with_fake_remotes(C, R)`.
- **Add `build_git_repo_with_worktrees(root, W)`** to `builder.rs` for the
  worktree fan-out bench (no fixture currently creates linked worktrees in the
  bench tree; `worktree.rs` tests do — lift that logic into the builder).
- **Large-history fixture for commit-graph wins:** `large_monorepo` has only 21
  commits — too shallow to demonstrate the R2 commit-graph upside. Add
  `build_deep_history_repo(root, commits)` with `commits ∈ {1000, (10000 gated)}`
  linear commits, and **write a commit-graph file** in the builder (run
  `git commit-graph write` via subprocess, or `gix`'s commit-graph writer in the
  fixture builder) so the gated revwalk bench actually exercises the graph. Gate
  the 10k row behind `SNIFF_BENCH_DEEP_DIRTY`-style env (mirror `cases/git.rs:30`).
  Include separate graph-present and graph-absent benchmark IDs; the migration
  must not regress repositories that have no commit-graph. If fixture generation
  uses the `git` executable, check availability once before registering that
  benchmark and report a clear skip rather than failing unrelated benches.
- **Cross-platform concerns:** (1) line endings — builders write `\n` explicitly;
  ensure no `core.autocrlf` interference by setting it off in fixtures on Windows.
  (2) file mode bits — irrelevant for read-only status parity but note Windows
  has no exec bit. (3) path separators — assert on normalized paths in parity
  tests. (4) `TempDir` under antivirus on Windows can add status latency — accept
  higher variance there (document in pass criteria).

### 5.4 Comparison methodology & pass criteria (STRICT)

Use Criterion's saved-baseline workflow (`criterion.md` §baselines:
`--save-baseline` / `--baseline`).

**Step 1 — Lock the git2 baseline BEFORE any production port (Phase 0).**
With production code still on git2, run the full git bench set and save a named
baseline, and **commit the numbers** for auditability:

```bash
# from sniff/
just bench -- --save-baseline git2 '^git_ops|^git_dirty_scaling|^git_deep_remote'
```

Save the resulting `time:` estimates (per bench ID) into a committed file
`sniff/features/2026-06-06-gitoxide/baselines/git2.md` (a table of
`bench_id → [lower estimate upper] ns`) together with OS, CPU, Rust version,
git commit, and relevant power-mode notes. This file is an audit record, not a
portable pass/fail threshold. The Criterion on-disk baseline
(`target/criterion/**/git2/`) is valid only for comparison on the machine that
created it.

**Step 2 — After each migration phase, compare against the baseline.**

```bash
just bench -- --baseline git2 '^git_ops|^git_dirty_scaling|^git_deep_remote'
```

Criterion prints a `change:` line with a p-value per bench.

If the on-disk baseline is unavailable or the host/toolchain changed, check out
the Phase-0 commit in a separate worktree, regenerate the git2 baseline on the
current host, then rerun the gix commit on that same host. Never judge a
regression by comparing committed absolute timings from different machines.

**Pass criteria (hard gate):**

1. **No statistically significant regression.** For every bench ID, Criterion's
   `change:` must NOT report "Performance has regressed" at the default
   `significance_level = 0.05`. A `change:` within the noise threshold
   (`noise_threshold = 0.02`, i.e. ±2%) or marked "No change detected" passes.
2. **Acceptable variance documented.** Benches whose baseline `time:` confidence
   interval is wider than ±10% (typically `worktree_fanout` and Windows
   `TempDir`-backed status) are flagged "high variance" in the baseline table and
   judged on the *median* estimate, not the bound, with a relaxed ±15% no-regress
   band. Document each such bench explicitly.
3. **Expected improvements asserted as non-regressions, reported as wins.** The
   R2 paths (`revwalk_recent_gated`, on the deep-history fixture) are *expected*
   to improve via commit-graph. They must at minimum not regress; record the
   observed improvement in the phase exit note. (Do not fail the gate if an
   expected win does not materialize — only fail on regression — but investigate.)
4. **Run conditions.** Benches run in release (Criterion default), on AC power,
   Low Power Mode off (macOS) / `cpupower governor performance` (Linux), per
   `criterion.md` best-practices checklist. CI runs the `bench-ci` subset as a
   smoke/performance-history signal. It is not a replacement for the same-host
   Phase-0 comparison unless CI explicitly benchmarks both commits on the same
   pinned runner.

**Perf levers to reach the targets** (apply during the relevant phase, not
speculatively):

- **Commit-graph** for `revwalk_recent_gated` and `ancestry_containment`:
  `repo.commit_graph_if_enabled()` for time/parent gates (R2).
- **Object cache sizing** for `revwalk_recent_full`, `diff_commit_files`,
  `refs_enumerate`: `repo.object_cache_size_if_unset(compute_object_cache_size_for_tree_diffs(&index))`.
- **`parallel` feature** for `worktree_fanout` (and `gix`'s internal parallel
  status) — already in the allowlist (§2.2).
- **One `resource_cache` reused across the range walk** for `diff_commit_files`
  (R3).

### 5.5 CI wiring

Add the new `git_ops/*` IDs that should gate (e.g.
`git_ops/status_file_changes/100`, `git_ops/revwalk_recent_gated`,
`git_ops/worktree_fanout/4`) to `lib/benches/ci-bench-ids.txt` so `just bench-ci`
(and `sniff-performance.yml`) exercises them. Keep the high-N rows opt-in.

---

## 6. Test Strategy

Correctness is gated by **L1 behavior-parity tests**; performance by §5 benches.
This split follows the monorepo taxonomy (`rust-testing` skill): in-process git
fixtures are L1 (no real terminal/device).

### 6.1 L1 behavior-parity tests

For each migrated operation cluster, add (or extend) an L1 test that asserts the
`gix`-backed output **equals the recorded git2 output** for the deterministic
fixtures:

- **Golden values from the git2 era.** Before porting an operation (in its phase),
  capture the current git2 output for the deterministic fixtures (commit
  hashes are stable because `Signature` uses `Time::new(0,0)`), and assert the
  gix output matches. Because fixtures are byte-deterministic, golden assertions
  (commit SHAs, branch names, ahead/behind counts, ordered file-change lists,
  status category counts) are reproducible.
- **Cover the inventory's asserted invariants:** rename→delete+add pairing
  (`status.rs:598`), counts-only vs detailed vs full-changes equivalence, the
  `wants_repo_metadata()` gate (summary skips per-branch ahead/behind), worktree
  ahead/behind/merged numbers, conflict detection (stage>0).
- **R7 explicit-policy tests:** add a fixture with a non-UTF-8 path/ref (where the
  OS allows) and assert the chosen policy (lossy conversion, not silent drop). On
  Windows, where non-UTF-8 paths are restricted, gate the test (`#[cfg(unix)]`).
- **Object-format policy:** add a SHA-1 fixture assertion and, when the installed
  `git` supports creating one, a SHA-256 repository test that asserts the
  documented unsupported/error outcome rather than a panic or misparsed
  fixed-width object ID.
- **R6 error-policy tests:** assert genuine not-a-repo → `Ok(None)`; assert a
  trust/ownership failure surfaces a `SniffError::Git { operation: "discover", .. }`
  (simulate via a fixture with mismatched ownership where feasible, else unit-test
  the error mapping with a constructed `gix::discover::Error`).
- **CLI parity:** `cli/tests/cli.rs` already exercises `sniff repo …` against git2
  fixtures (inventory §10). These become the end-to-end parity guard — they must
  pass unchanged after the port (their fixtures stay git2-built per §2.4, the
  production reader becomes gix). Re-verify the `repo root` trailing-separator
  output (row 3 gotcha) explicitly.

### 6.2 Doctests

Any `///` example that constructs a `git2::Repository` or names a git2 type must
be updated to the `gix` equivalent and pass `just doctest`
(`cargo test --doc`). Audit `GitRepo` and `filesystem::git` public docs. Update
the `sniff` skill `GitRepo` description ("libgit2 handle") to "gix handle".

### 6.3 Level placement

- All git parity tests: **L1** (default tier, no prefix).
- The deep-history / large-fixture parity tests that exceed ~5s: prefix `slow_`
  so `sanity` skips them (`rust-testing` taxonomy).
- No L2/L3/browser involvement — git is in-process.

### 6.4 `require_level!`

Not needed for L1 git tests (they never need a real resource). The existing
`cli/tests/cli.rs` integration tests keep their current gating.

---

## 7. Phased Execution Plan

Ordered, reviewable phases. Each phase lists scope, files touched, and exit
criteria (tests + bench). Phases 0–1 reduce surface and lock the contract;
2–7 migrate operation clusters; Final removes git2 from production.

### Phase 0 — Lock baselines + R1 discover-once refactor (no gix yet)

- **Scope:**
  - Add the `git_ops/*` micro-benches (`cases/git_ops.rs`) and the new fixtures
    (`build_git_repo_with_worktrees`, `build_deep_history_repo` + commit-graph) to
    `builder.rs`/`fixtures.rs`. Still git2-backed production.
  - **Save the git2 baseline** (§5.4 Step 1) and commit `baselines/git2.md`.
  - **R1:** thread a single opened handle. Make `determine_shared_walk_root`
    (`mod.rs:362`) return/share the opened `GitRepo`; convert the free functions
    in `mod.rs`, `identity.rs`, `docs.rs`, `just.rs`, `blast_radius.rs`, and the
    five `recent_commits.rs` entry points to accept `&GitRepo`/`&Repository`,
    keeping `discover`-from-path only at true library entry points. CLI commands
    call those library entry points rather than opening repositories directly.
    Pure git2 cleanup.
  - **R6 (error-semantics prep):** preserve the discover error before the migration
    (`map_err(|e| { debug!(error=%e); … })`) and branch on `e.code()` for
    `Owner`/`NotFound` so the *intended* tri-state is locked as testable behavior
    while still on git2.
- **Files:** `benches/cases/git_ops.rs` (new), `benches/perf.rs`,
  `benches/support/builder.rs`, `benches/support/fixtures.rs`,
  `benches/ci-bench-ids.txt`, `lib/src/filesystem/mod.rs`,
  `repo/identity.rs`, `docs.rs`, `just.rs`, `blast_radius.rs`,
  `git/recent_commits.rs`, `git/types.rs`, CLI command modules.
- **Exit criteria:** `just test` green; `just bench -- --save-baseline git2 …`
  produces and commits the baseline; `discover` call-site count drops from 16
  toward the true entry points; no behavior change in `cli/tests/cli.rs`.
- **Rollback:** Phase 0 is pure git2 refactor + bench scaffolding — revert the
  commit; baseline file is additive.

### Phase 1 — Add `gix`, migrate discovery + error type

- **Scope:** add `gix` deps + allowlist (§2.1/§2.2); remove production
  `git2` dep from the library and CLI, add git2 as `[dev-dependencies]` (§2.4).
  Move direct CLI Git operations behind backend-neutral library APIs: repository
  root discovery, preferred/selected remote URL, commit URL inputs, and
  branch-history lookup. Implement the new
  `SniffError::Git { operation, source }` (§4.1) and the discovery tri-state
  (§4.2). Migrate `GitRepo::discover`/`open`, `workdir`, `path`, `common_dir`,
  `is_worktree`, `head`/`head_id`/branch shorthand, `repo_root`. Decide
  `into_sync()` plumbing for the handle (§2.3).
- **Files:** `lib/Cargo.toml`, `cli/Cargo.toml`, `lib/src/error.rs`,
  `lib/src/filesystem/git/types.rs`, `lib/src/filesystem/git/worktree.rs`
  (discover paths), new or extended library remote/query APIs, CLI discover
  sites (`mod.rs`, `repo.rs`, `remote.rs`).
- **Exit criteria:** workspace builds on macOS/Linux/Windows;
  `git_ops/discover` bench `--baseline git2` shows no regression; L1 discover +
  error-policy parity tests pass; `repo root` trailing-separator parity verified;
  `rg 'git2::|gix::|use git2|use gix' sniff/cli/src` returns no matches.
- **Rollback:** revert the dep swap + discovery commit; production returns to
  git2 (baseline still valid).

### Phase 2 — Status (summary dirty-flag + full file changes)

- **Scope:** migrate `status.rs` — `repo.status(Discard)` platform with
  `UntrackedFiles::Files` + dirwalk recursion; `is_dirty()` for `summary()`
  (row 27); status-item variant mapping (row 28); index conflicts via stage>0
  (row 29). Re-express the `diff_index_to_workdir` worktree-diff via the status
  change set (row 21) and `diff_tree_to_index` for `--cached`. Apply R7 lossy path
  policy for conflict paths.
- **Files:** `lib/src/filesystem/git/status.rs`,
  `lib/src/filesystem/blast_radius.rs` (its `statuses` call, §3 inventory).
- **Exit criteria:** `git_ops/status_dirty_flag` and `git_ops/status_file_changes`
  no regression; rename→delete+add and per-category-count parity tests pass;
  blast-radius parity test passes.
- **Rollback:** status is a self-contained module — revert to the git2 status
  impl while the rest stays gix (the `GitRepo` handle exposes `&Repository`; a
  temporary git2 shim is not possible once the dep is gix-only, so rollback here
  means reverting Phases 1–2 together).

### Phase 3 — Diff (single-pass patch + per-commit changed files)

- **Scope:** migrate `diff.rs` (`aggregate_diff` → gix change iterator + unified
  diff, single-pass, rows 22/23/25) and `discovery.rs` commit-diff
  (`get_commit_files`, `diff_tree_to_tree`, rows 19/24). Apply R3: change
  `get_commit_files` to take `ObjectId`/`&Commit` (no string round-trip) and reuse
  one `resource_cache` across the range walk. Keep rename detection OFF (row 24).
- **Files:** `lib/src/filesystem/git/diff.rs`,
  `lib/src/filesystem/git/discovery.rs`.
- **Exit criteria:** `git_ops/diff_commit_files` no regression (ideally improved
  via `resource_cache`); ordered file-change-list parity tests pass; `DeltaKind`
  mapping parity.
- **Rollback:** revert Phases 1–3.

### Phase 4 — Revwalk / recent-commits (commit-graph + lazy decode)

- **Scope:** migrate all 6 revwalks (`discovery.rs`, `recent_commits.rs`,
  `remote_refresh.rs` ancestry) to `repo.rev_walk(tips).all()` yielding `Id`s
  (R2). Apply commit-graph (row 17) to the two timestamp-only gates
  (`recent_commits.rs:385`, `remote_refresh.rs:443`); decode bodies only where
  rendered. Preserve TIME-newest-first ordering for the early-`break`
  (`recent_commits.rs:371`). Size the object cache for body-decode walks.
  Migrate `revparse_single`, `graph_descendant_of` (merge-base), `find_commit`.
- **Files:** `lib/src/filesystem/git/discovery.rs`,
  `lib/src/filesystem/git/recent_commits.rs`,
  `lib/src/filesystem/git/remote_refresh.rs` (ancestry portion).
- **Exit criteria:** `git_ops/revwalk_recent_gated` (deep-history fixture) no
  regression and ideally improved; `git_ops/revwalk_recent_full` no regression;
  `HashNotReachable` parity; recent-commits ordering + cutoff parity.
- **Rollback:** revert Phases 1–4.

### Phase 5 — Refs, branches, tracking, remotes, config

- **Scope:** migrate `collect_ref_decorations` (row 10/11, R9), `get_local_branches`
  (`local_branches()`, row 13), `get_tracking_status`/`graph_ahead_behind`
  (row 34, preserve `wants_repo_metadata()` gate), `get_remotes`/`find_remote`/URL
  (rows 32/33), `get_remote_default_branch`/`get_remote_branches`,
  `get_git_config` (rows 30/31 incl. ProgramData layering). Apply R7 to ref names.
  Optional: R8 (memoize remotes), R12 (consult `origin/HEAD` before `main`/`master`).
- **Files:** `lib/src/filesystem/git/discovery.rs`,
  `lib/src/filesystem/git/remote_refresh.rs`, `lib/src/filesystem/repo/identity.rs`,
  CLI `remote.rs`.
- **Exit criteria:** `git_ops/config_read` and `git_ops/refs_enumerate` no
  regression; config 12-key parity on macOS + Windows; branch/remote-URL/
  ahead-behind parity.
- **Rollback:** revert Phases 1–5.

### Phase 6 — Worktrees (ThreadSafeRepository fan-out) + conflict probe

- **Scope:** migrate `get_worktrees` (`remote_refresh.rs:554`) and `worktree.rs`
  listing. Apply R4: open base once → `into_sync()` → `to_thread_local()` per
  Rayon worker (remove dead `_base_repo` and per-iteration base open). Apply R5:
  short-circuit `has_conflicts = false` when `graph_descendant_of(base, wt)`;
  only unmerged branches reach a merge. If the merge is still needed and
  `gix-merge` lacks an early-abort mode, add the `merge` feature (§2.2) and port
  `merge_commits` + conflict check; otherwise leave `merge` off.
- **Files:** `lib/src/filesystem/git/remote_refresh.rs`,
  `lib/src/filesystem/git/worktree.rs`, possibly `lib/Cargo.toml` (`merge`).
- **Exit criteria:** `git_ops/worktree_fanout` no regression (ideally improved by
  removing per-worktree base re-open); worktree ahead/behind/merged/conflict
  parity tests pass; `get_current_worktree_name` / `list_worktrees` CLI parity.
- **Rollback:** revert Phases 1–6.

### Phase 7 (Final) — Remove git2 from production, confirm, document

- **Scope:** confirm no production `git2::` symbols remain (grep). Run the full
  bench set `--baseline git2` and confirm every ID passes the §5.4 gate. Record
  observed wins. Update docs.
- **Files:** READMEs (`sniff/lib/README.md`, `sniff/cli/README.md`),
  `sniff/docs/sniff-library-architecture.md` (cost model — commit-graph notes),
  `docs/dependencies.md` + `sniff/docs/dependencies.md` (git2→gix), the `sniff`
  skill (`GitRepo` "gix handle" + shared-work notes), `lib/benches/README.md`.
- **Exit criteria:** `rg 'git2::|gix::|use git2|use gix' sniff/cli/src` returns
  no matches, and `rg 'git2::|use git2' sniff/lib/src` returns only
  `#[cfg(test)]` fixture hits; full
  `just bench -- --baseline git2 …` green; `just test`, `just lint`,
  `just doctest` green; dependency docs updated.
- **Rollback:** the dep swap is the last revertable unit; production git2 is gone,
  so post-Final rollback means reverting the whole feature branch.

---

## 8. Risks & Rollback

| Risk | Likelihood | Impact | Mitigation / rollback |
|---|---|---|---|
| **`gix` pre-1.0 API churn** — pinned signatures drift on the next bump | High | Medium | Pin `=0.84.0` exactly (§1.3); isolate gix calls behind sniff's `GitRepo`; review any version bump separately. |
| **Status semantics differ** — `gix` folds index↔worktree into `status`; untracked/conflict classification may not map 1:1 to git2's bitflags | Medium | High | Phase 2 parity tests against git2 goldens (rename→delete+add, per-category counts). Block the phase on parity, not just "compiles". |
| **Integrity checks weaker than git2** — `gix` does less strict hash/object verification by default (`gitoxide.md` §6) | Low | Medium | sniff is read-only inventory, not a security tool; acceptable. Note in README. If strictness is later required, evaluate `gix` verification options. |
| **Trust model surprises** — default `gix` behavior may omit sensitive config in untrusted repos | Medium | Medium | §4.2: use `bail_if_untrusted()` on every open to preserve libgit2 ownership validation; test trusted, untrusted, and missing repositories separately. |
| **SHA-256 behavior changes accidentally** — `ObjectId` supports multiple hash kinds while current sniff is SHA-1-only | Low | Medium | Enable only `sha1`; avoid fixed-width parsing assumptions; test the documented unsupported outcome for a SHA-256 repository. |
| **commit-graph absent on real user repos** — the R2 win needs a written commit-graph; most repos don't have one | Medium | Low (perf only) | `commit_graph_if_enabled()` falls back to object DB when absent — never a correctness issue. The win is opportunistic; the gate only requires no-regression. Bench fixture writes a graph to *prove* the lever works. |
| **Worktree `!Sync` port regresses** — `into_sync()`/`to_thread_local()` misuse could serialize or deadlock the Rayon fan-out | Medium | Medium | Phase 6 `git_ops/worktree_fanout` bench guards throughput; parity tests guard correctness. Model the port on the R4 recommendation exactly. |
| **Non-UTF-8 path/ref behavior change** — `gix` is BStr-first; the old `Option<&str>`→skip policy is implicit | Low | Low | R7: make the lossy policy explicit with a Phase-2/5 `#[cfg(unix)]` parity test. |
| **Cross-platform build/feature gaps** — a feature needed only on one OS | Low | Medium | Phase 1 builds on all three OSes as an exit criterion; CI matrix runs the bench-ci subset per OS. |
| **Config `add_file`/ProgramData layering has no direct gix analogue** (row 31) | Medium | Medium | Phase 5: if `gix-config` lacks add-file layering, read the system gitconfig directly and merge keys in sniff; parity-test the 12 keys on macOS + Windows. |
| **Bench variance hides a real regression** | Low | High | §5.4 uses same-host Criterion comparisons at p<0.05, not absolute timings from another machine; high-variance benches use a documented relaxed band; CI is only authoritative when it benchmarks both commits on the same pinned runner. |

**Per-phase rollback** is linear: each phase is one (or a few) commits on the
feature branch; reverting a phase reverts to the previous phase's state. The
**point of no return** for production is Phase 1 (git2 leaves
`[dependencies]`), so Phases 2–6 roll back to "Phase 1 state", not to git2. The
git2 baseline file and bench scaffolding (Phase 0) are additive and survive any
rollback, so the performance gate remains usable throughout.

---

## Appendix A — Touched Production Files (consolidated)

| File | Operation clusters | Phase(s) |
|---|---|---|
| `lib/Cargo.toml`, `cli/Cargo.toml` | deps, features; CLI backend-dependency removal | 1, (6 if `merge`) |
| `lib/src/error.rs` | error type (R6) | 1 |
| `lib/src/filesystem/git/types.rs` | discover/handle, R1, R8 | 0,1 |
| `lib/src/filesystem/git/status.rs` | status, conflicts, R7 | 2 |
| `lib/src/filesystem/blast_radius.rs` | status | 2 |
| `lib/src/filesystem/git/diff.rs` | diff aggregation | 3 |
| `lib/src/filesystem/git/discovery.rs` | diff, revwalk, refs, R2/R3/R9 | 3,4,5 |
| `lib/src/filesystem/git/recent_commits.rs` | revwalk, R2, R6 | 0,4 |
| `lib/src/filesystem/git/remote_refresh.rs` | remotes/config/worktree/ancestry, R4/R5/R10/R12 | 4,5,6 |
| `lib/src/filesystem/git/worktree.rs` | worktree listing | 1,6 |
| `lib/src/filesystem/repo/identity.rs` | remote basename, R1 | 0,5 |
| `lib/src/filesystem/{mod.rs,docs.rs,just.rs}` | discover threading (R1) | 0 |
| `cli/src/commands/{mod.rs,repo.rs,remote.rs}` | replace direct backend access with library queries; error rendering (R6) | 0,1,5 |
| `lib/benches/cases/git_ops.rs` (new), `perf.rs`, `support/{builder,fixtures}.rs`, `ci-bench-ids.txt` | bench matrix + fixtures | 0 |

## Appendix B — Out-of-Scope (explicitly unchanged)

- `lib/src/filesystem/git/remote_refresh.rs:350-360` (`fetch_single_remote`) — the
  `git fetch` subprocess stays as-is (R10). Optionally add a non-zero-exit
  `warn!` (R10 caveat) — independent of the migration.
- All `#[cfg(test)]` / `tests/` / bench fixture *builders* keep using git2 write
  APIs (§2.4 Option A).
