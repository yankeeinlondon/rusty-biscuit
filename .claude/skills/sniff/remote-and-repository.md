# Sniff Repository and Remote Semantics

Use this reference for monorepo topology, aggregate output, worktrees, Git
conflicts, and provider queries.

## Contents

- [Topology model](#topology-model)
- [Aggregate projection](#aggregate-projection)
- [Worktrees](#worktrees)
- [Conflicts and branches](#conflicts-and-branches)
- [Remote snapshots](#remote-snapshots)
- [Focused providers](#focused-providers)

## Topology model

`RepoInfo.monorepo_standards` lists detected standards with resolved binaries
and confidence. `RepoInfo.monorepo_layers` lists membership layers. Every layer
has one authority, optional orchestrators, provenance, and paths into the
canonical `RepoInfo.packages` catalog.

The removed `MonorepoTool`, `workspace_tools`, and `discovery_sources` surfaces
must not return. CLI labels use each standard's stable `spec().label`.

## Aggregate projection

`sniff repo --json` projects one captured request into:

- Top-level repository identity.
- CWD-relative facts under `context`.
- One worktree array and one branch array.
- `dirty`, `staged`, `unstaged`, and `untracked` scope buckets containing
  files, source code, documentation, packages, and package areas.
- A lean aggregate Git status; focused commands retain richer detail.

The aggregate excludes network-primary and parameterized commands and performs
no network request.

## Worktrees

Ordinary aggregate projection reuses worktree metadata and opens zero linked
repositories. Focused inspection may open a registered target to validate it:

- Current/main/linked valid targets are reported.
- An absent registered target is stale and omitted.
- A path that exists but is a corrupt repository is an error.
- Ahead/behind work follows the focused detail request and does not widen the
  aggregate path.

## Conflicts and branches

`merge_conflicts_at` observes actual non-zero live-index stages.
`merge_conflicts_with_branch_at` predicts a committed-tip merge in memory.
Prediction uses exact local tips, captured attributes, and safe configuration;
it never fetches, executes external drivers, runs hooks, or reads the live
worktree state as merge input.

Branch discovery uses locally known refs by default. Refresh requires explicit
opt-in.

## Remote snapshots

`RemoteRepoSnapshot` captures provider metadata, default branch, and tree once
per `fetch_report`. Document and CI/CD projections consume the snapshot rather
than refetching the same evidence.

`RemoteTree::available == false` means the fetch failed and absence cannot be
concluded. Truncated provider trees must continue through provider-specific
bounds; continuation work uses its own counter.

## Focused providers

`FocusedProviderClient` handles exact/list pull-request and CI/CD job queries.
It preserves pagination bounds, host policy, provider flavor/version,
credential scope, and typed errors.

Ambiguous self-hosted hosts are probed anonymously first. Authentication retry
is allowed only after provider identity is established, or when exactly one
host-bound provider credential can disambiguate the challenge. Never send a
global provider token to an unidentified host.

Unsupported provider/version operations fail before provider I/O. Do not
convert malformed, missing, authorization, rate-limit, capability, or transport
errors into empty results.
