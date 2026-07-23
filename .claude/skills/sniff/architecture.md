# Sniff Request Architecture

Use this reference when changing detection plans, shared filesystem work,
subprocesses, host capabilities, or request cost.

## Contents

- [Request planning](#request-planning)
- [Filesystem observation](#filesystem-observation)
- [Package topology](#package-topology)
- [Programs and subprocesses](#programs-and-subprocesses)
- [Network defaults](#network-defaults)

## Request planning

Top-level OS, hardware, network, and filesystem domains run in scoped workers
through `detect_with_plan`. Requests choose what evidence is needed before work
starts. A detector must not widen another domain simply because a Git handle or
repository root is available.

`DetectionPlan::default()` enables all domains at safe defaults. In particular,
it uses full OS identity with NTP disabled, so Tier-1 detection performs no
implicit network probe.

## Filesystem observation

Full repository detection builds one request-scoped
`FilesystemSystemView`. It retains selected evidence rather than `DirEntry`
objects or file bodies. `RepoEvidence::from_view` is the bridge into repository
detection.

Walk scope comes from consumers:

- Formatting-only requests probe `.editorconfig` directly and start no walker.
- Structure-only repository requests start no shared descendant walker.
- Inventory-only requests walk the resolved package root.
- Full repository and repository-wide docs requests walk the repository root.

Inventory accepts at most `MAX_FILES`. An inventory-only walk stops globally at
the cap. A combined walk continues for active manifest/docs consumers while the
inventory projection reports `truncated` and `limit`. A truncated subset is not
stable across runs; complete output is sorted and deterministic.

## Package topology

`RepoInfo.packages` is the canonical catalog. `MonorepoLayer.packages` stores
repo-relative references to that catalog rather than duplicate package data.

Each layer has one membership authority and zero or more task orchestrators.
An orchestrator alone is not a monorepo. Package `standard` and `provenance`
identify the membership authority and discovery method.

Structure requests collect membership and minimum identity only. Use a focused
request for selected manifest facts and full mode for inventory-backed
enrichment.

## Programs and subprocesses

Program categories share an executable index. Prefer eager builders for bulk
lookup. `_only` builders exclude platform fallback layers, which changes
Windows App Paths behavior.

All subprocesses go through `process::run_with_timeout` or
`process::run_command_with_timeout`. These helpers own deadlines, concurrent
pipe draining, process-tree termination, and reaping on macOS, Linux, and
Windows. Preserve caller cwd and environment when using the builder form.

Batch service enrichment with bounded chunks. A failed or timed-out chunk
degrades only its own services; it must not discard healthy chunks.

## Network defaults

WAN IP is opt-in through the network request. It reuses one blocking client and
queries fallback endpoints sequentially, stopping after success. Strictly parse
the body as `IpAddr`; do not include a response body in errors or counters.

Remote Git refresh is explicit. Ordinary branch, aggregate repository, and
conflict-prediction paths use locally known refs and perform no fetch.
