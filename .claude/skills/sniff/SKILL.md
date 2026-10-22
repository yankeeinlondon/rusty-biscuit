---
name: sniff
description: Expert guidance for the cross-platform `sniff` Rust library and CLI. Use when discovering host hardware, OS, network, programs, services, filesystems, Git repositories, worktrees, monorepo packages, remote providers, or when changing Sniff request tiers, observation reuse, work counters, CLI output, and macOS/Linux/Windows behavior.
---

# Sniff

Sniff observes hosts and repositories through the `sniff` library and `sniff`
CLI. Its central contract is request-scoped discovery: observe expensive state
once, project it to multiple outputs, and make cost visible through stable work
counters.

## Platform contract

Supported targets are macOS, Linux, and Windows. WSL follows the Linux compile
and runtime path; it does not cross into native Windows detectors.

- Gate Unix- and OS-specific imports with `cfg`.
- Build `PATH` with `std::env::join_paths`; never hard-code `:` or `;`.
- Use bounded subprocess helpers from `process`; never poll a child with piped
  output that is not being drained.
- Windows audio detection uses PowerShell CIM, not `wmic`.
- `current_user_id()` returns the effective Unix UID or Windows process-token
  SID without subprocess, registry, environment-variable, or network fallback.
  It is on-demand and absent from normal host inventory and caches.
- `os::processes_working_in(dir)` lists processes whose current directory is
  `dir` or below, with each one's command line, parent, and start time (one
  `sysinfo` refresh of cwd and command line; caller and its ancestors
  excluded; matched on both the given and canonical spelling). Open files are
  not visible, and a hidden cwd (another user's) is skipped. On-demand too.

Read [architecture.md](architecture.md) before changing request costs, shared
walks, Git/worktree behavior, subprocess policy, or remote snapshots.

## API tiers

### Convenience

```rust
use sniff::detect;

let result = detect()?;
```

`DetectionPlan::default()` requests all domains at safe defaults. It does not
enable the optional NTP network probe.

### Request plan

```rust
use sniff::{detect_with_plan, request::*};

let plan = DetectionPlan::new()
    .os(OsRequest::summary())
    .hardware(HardwareRequest::summary())
    .without_network()
    .filesystem(
        FilesystemRequest::new()
            .git(GitRequest::identity())
            .repo(RepoRequest::structure())
            .without_docs(),
    );

let result = detect_with_plan(plan)?;
```

### Focused modules

Use module-level APIs when a caller needs one bounded fact, such as
`GitRepo::discover`, `current_user_id`, or a focused provider query. Do not use
full host detection as a convenience wrapper around one fact.

`GitRepo::changed_paths(ChangedBase)` is the focused path-only changed-file
API: one status observation plus (when the base differs from `HEAD`) one
direct tip-vs-tip tree comparison, returning deduplicated, byte-sorted native
repository-relative paths. Local only — no fetch, line stats, patches, rename
similarity, history, or inventory. It rejects bare repositories
(`SniffError::BareRepository`), unresolved/non-commit references
(`UnresolvedReference`/`NonCommitReference`), and any unmerged index entry
(`MergeConflicts`, naming every conflicted path). `ChangedBase::Omitted` makes
an unborn `HEAD` valid (pure working-tree status); `Reference(token)` peels
annotated tags and, against an unborn `HEAD`, compares the base with the empty
tree — `Reference("HEAD")` on unborn is an error, keeping explicit `HEAD`
distinct from omission. See "Changed Paths" in
[architecture.md](architecture.md) and
[sniff-library-architecture.md](../../../sniff/docs/sniff-library-architecture.md).

The returned paths include deletions; callers map them onto `repo_root()`,
check current existence, and match native filesystem identities before
reading source. A staged deletion recreated on disk is still a candidate.
File-type replacements count as modifications; rich/path status, fast dirty
checks, and detailed counts share `status::classify_worktree_status`. Preserve
the fast dirty reader's early stop when changing that classification.
Do not use `file_changes()` here: it enriches status with line counts and
lossy display paths. Preserve native paths through matching, and reject
conflicts anywhere rather than narrowing conflict checks to a package.
Tree-hugger's `--changed` is a consumer with optional features disabled.
All live index boundaries use `open::guard_index_read` / `read_index` to contain
upstream decoder panics: cache sizing skips failures, while `status_iter` and
rich `StatusContext::new` propagate typed Git errors. Committed diff caches also
read the live index for attributes: use guarded `open::tree_diff_cache` for
commit files, history, structured commits, and tip comparisons. Keep absent/valid empty
indexes valid and metadata-only consumers independent of readable index state.
`changed_paths_tests::index_reads` covers every shared consumer; CLI
`changed::index_corruption` partitions the full prefix/report matrix by shape.

`filesystem::repo::{PackageNameSource, PackageOwnershipLookup}` support callers
that need declared identity and repeated file ownership. Check `name_source ==
Manifest`; a nonempty `name` alone may be a relative-path fallback. Legacy
serialized packages default to `PathFallback` (omitted on serialization).
Build the borrowing lookup once per `RepoInfo`, then call `package_for_path`
with absolute native paths. Deepest component boundary wins before checking
name usability, duplicate aliases use catalog order, and paths outside the root
return `None`. Root packages have an empty `relative` string. Use the structure
API with root-package fallback for standalone projects; do not add another
manifest reader. JSON duplicate names preserve existing last-wins behavior.

`sniff::filesystem::query::query_path_usage` is the focused "which processes
use this path" query: one shared budget that stops scheduling work (never a
return-time guarantee), one tree walk per query, identity-first matching,
and library-owned `outcome`/coverage. OS backends implement the crate-private
`UsageBackend` trait in `filesystem/query/backend.rs`; tests drive the query
through a fake backend in `filesystem/query/tests.rs`. The Linux/WSL2 backend
(`filesystem/query/linux.rs`) reads a configurable proc root: its tests build a
synthetic `/proc` from symlinks (`stat`/`readlink` treat them like magic links)
so the `fdinfo` matrix runs on every Unix host, while live controlled children
(`linux_tests::live`) run on Linux only. The macOS backend
(`filesystem/query/macos.rs`) is generic over a small `Source` trait of
`libproc` calls: a scripted source runs its classification tests on every
Unix host, and only `macos::ffi` and `macos_tests::live` are macOS-only.
The Windows backend (`filesystem/query/win32.rs`, module `win32` so it does not
shadow the `windows` crate) reports `open_handles` through a file-side scan and `loaded_modules`: modules are
path-only observations (read in one pass over each process's loader list,
`Source::loader_paths`, falling back to `GetModuleFileNameExW` per module;
never by the kernel's mapped-file name, which names a hard-linked image by
whichever link was mapped first), so its scripted-`Source` tests run on every host,
and only `win32::ffi`, `win32_tests::spellings`, and `win32_tests::live` are
Windows-only. It keeps each process handle from enumeration through
enrichment (closing it early when nothing is in scope) and reads `details`
through it, never by PID. The `win32_paths` module reuses regular-file/directory entries from the one tree
walk, queries file-side holders on at most eight scoped, joined workers, and
propagates the request performance collector. `open_handles` coverage counts
paths; `path_holder_inspections` counts scheduled path probes. No foreign
handle is duplicated. Previously enumerated holders retain their process
handles; a newly opened holder must be confirmed by another file-side query.
Unconfirmed evidence stays identity-uncertain. Delete-pending paths are gaps;
cwd and subscription mechanisms stay unsupported (see the `os` skill).
Every backend shares the per-mechanism `Tally` in `backend.rs`. Contract:
`sniff/docs/topics/filesystem-query.md`.

**A library that walks a tree before asking who uses it should supply its own
index** (`TreeIndex::builder(root_identity)` + `entry(relative, kind,
identity)` → `PathUsageOptions::with_tree_index(Arc)`) instead of letting the
query walk again; `wt remove` passes its deletion inventory this way. The
walk and the builder produce the same root-relative `TreeIndex` (root = empty
path), and paths are joined onto `root.resolved` only for matches and the
Windows probe list (`TreeIndex::probe_entries`, built once per scan in
`win32.rs`). Root verification and the recheck still run; a different root
identity, a file root, or `target_only` is `InvalidOption`. The report's
`index_provenance` (`walked`/`supplied`/`unknown`) is required at schema 2;
`PathUsageReport` deserializes through a private `RawReport`
(`try_from`) so absent-at-schema-1 reads as `unknown`, never `walked`. Keep
that loader when adding report fields. Tests: the "caller-supplied tree index"
section of `query/tests.rs`.

`filesystem::space::available_space(path)` is the focused free-space reading
(nearest existing ancestor, aliases resolved by canonicalization, `f_bavail` /
`GetDiskFreeSpaceExW` caller quota). It never calls `hardware::storage`
(device inventory that spawns `diskutil`). Its `SpaceSource` trait is the
test seam. Contract: `sniff/docs/topics/available-space.md`.

`SniffConfig` remains a compatibility surface, but new code should use request
plans.

## Request-cost tiers

| Request | Cost/contract |
|---|---|
| `GitRequest::identity()` | Repository identity only; no working-tree status walk |
| `minimal()` / `summary()` | Branch and dirty flag; no commits or worktrees |
| `full()` | Commits, per-file stats, worktrees; no unified diffs or network |
| `deep()` | Adds unified diffs, branch detail, containment, and explicit remote refresh |
| `RepoRequest::structure()` | Membership and minimum package identity only; no lockfile corroboration |
| `RepoRequest::focused(...)` | Selected manifest-backed facts without full inventory or lockfile corroboration |
| `RepoRequest::full()` | Inventory-backed enrichment, lockfile corroboration, and repository-wide observations |

`RepoRequest::with_lockfile_provenance(bool)` sets layer lockfile
corroboration on any tier; a serialized request without the field keeps it.
Every `MonorepoLayer` carries a required `lockfile` observation (it replaced
`lockfile_match`); only an exact `match` upgrades provenance. Read the
lockfile pipeline in [architecture.md](architecture.md#lockfile-observation-pipeline)
before touching `filesystem/repo/lockfile/` or parsing JSON with comments.
`GitMetadataRequest` narrows legacy coarse requests; it never widens them.
`metadata: None` derives legacy behavior and is required for serialized-plan
compatibility.

## Aggregate versus focused semantics

Aggregate output is a projection of one captured observation. It must not call
focused discovery repeatedly.

- Bare `sniff repo --json` returns consolidated identity, context, worktrees,
  branches, and dirty/staged/unstaged/untracked scope buckets.
- Network-primary (`remote`, `pr`) and parameterized (`hash`) commands are not
  part of the aggregate, and aggregate reporting makes no network requests.
- Focused commands retain richer contracts and may request details that the
  aggregate deliberately omits.
- Repo-level version collapses to a string only when all discovered packages
  expose one distinct version; otherwise it is null.

Worktree projection has two distinct paths:

- Aggregate Git/worktree output uses captured metadata and performs zero linked
  repository opens in the ordinary path.
- Focused worktree inspection may validate registered targets. It omits a
  stale registration (directory or `.git` file absent, as `git worktree list`
  marks prunable) but reports a corrupt repository whose `.git` file exists.

Conflict prediction operates on captured committed tips in memory. It must not
fetch, run hooks or subprocesses, consult the live index, or mutate the
worktree. Actual live-index conflicts and predicted branch conflicts remain
separate APIs.

## Remote authority

`resolve_remote_at` is the selection authority for configured remotes.
Classification stays local when possible; ambiguous self-hosted HTTP(S) hosts
require exact-host consent before probing.

`RemoteRepoSnapshot` captures metadata, default branch, and tree once per
report. Providers project documents and CI/CD from that snapshot. A downstream
provider that overrides `list_documents_with` must also override
`list_documents` to avoid default-method recursion.

`FocusedProviderClient` performs bounded pull-request and CI/CD queries. It
preserves provider/repository identity, pagination bounds, host policy,
credential scope, and typed malformed/missing/auth/rate-limit/capability/
transport errors. Never collapse a focused provider error to an empty list.
`remote::blocking::pull_request_for_branch` is its deadline-bound blocking
entry point for "which PR came from this branch", and
`remote::blocking::open_pull_requests` lists a repository's open PRs, and
`remote::blocking::branch_head` reads one branch's head commit. A 404 in any of
them is an error, not "no PR" or "no branch". Credentials failures are split
into `CredentialsRequired`, `CredentialsRejected`, `CredentialsInsufficient`,
and `RateLimited { authenticated, key }`; `key` is a variable name, never a
value. `credential_env` lists the variables those lookups read. A successful
`branch_head` or `open_pull_requests` carries `RequestCredentials` (anonymous,
keyed with variable names, or unknown), recorded per request by the sending
client, host-bound overrides included; never re-derive it from the
environment.

Read [remote-and-repository.md](remote-and-repository.md) for topology,
worktree, aggregate, and remote details.

## Performance invariants

Judge optimization by work removed, not wall-clock timing. Stable counters live
in `sniff/lib/src/performance/counters.rs`; an absent counter means zero.

- Instrument one chokepoint per unit of work.
- Check `performance::is_collecting()` before clocks or formatted names.
- Propagate `WorkerCollector` into every spawned, Rayon, or parallel-walker
  worker; otherwise work silently disappears from the report.
- Attribute a high aggregate counter to call site and path before optimizing.
- Compare counters only against a compatible phase, OS, runner class, and
  request shape.

Read [performance.md](performance.md) before adding counters, parallel work,
filesystem walks, Git diff/blob work, or interpreting archived baselines.

## Topic routing

| Topic | File |
|---|---|
| Request architecture, shared work, subprocess policy | [architecture.md](architecture.md) |
| Repository topology, worktrees, aggregate and remote contracts | [remote-and-repository.md](remote-and-repository.md) |
| Work counters and baseline gotchas | [performance.md](performance.md) |
| CLI catalog and output modes | [cli.md](cli.md) |
| Program catalog, lookup, and install CLI | [programs.md](programs.md) |
| Add or change a detector/category/install implementation | [extending.md](extending.md) |
| Service detection | [services.md](services.md) |
| Scoped addresses, default gateways, ICMP probes | [network.md](network.md) |
| Fixtures, CLI test harnesses, suite audits | [testing.md](testing.md) |
| Adding detection capabilities | [extending.md](extending.md) |

Load only the reference matching the active subsystem.

## Testing and verification

Use the Sniff package-area recipes:

```sh
cd sniff
just build
just test
just lint
```

`just test` enables the `remote` feature. A test behind `remote` or `network`
that is run without that feature does not compile or execute and is not valid
coverage. Cross-platform compile guards must enable the same feature.

`just lint` here runs clippy without `--all-targets` or `-D warnings`, so it
passes over warnings in `#[cfg(test)]` modules and test targets that CI's
shared `_lint` (`--all-targets -D warnings`, no features) rejects. Before
handoff, also run `cargo clippy -p sniff --all-targets -- -D warnings` and the
same for `sniff-cli`.

Read [testing.md](testing.md) for the CLI `test-fixtures` feature, suite
audits, regression-input rules, real-tool lockfile fixtures, and the L1/L2
CLI harnesses.
