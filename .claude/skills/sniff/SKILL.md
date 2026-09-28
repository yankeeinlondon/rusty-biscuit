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
`remote::blocking::open_pull_requests` lists a repository's open PRs. A list
404 in either is an error, not "no PR".

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
