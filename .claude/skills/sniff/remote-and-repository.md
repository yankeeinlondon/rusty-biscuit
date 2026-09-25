# Sniff Repository and Remote Semantics

Use this reference for monorepo topology, aggregate output, worktrees, Git
conflicts, and provider queries.

## Contents

- [Topology model](#topology-model)
- [Aggregate projection](#aggregate-projection)
- [Worktrees](#worktrees)
- [Recent commits](#recent-commits)
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

### The empty top-level area

`Package.package_area` is `""` for a package directly under the repository
root, and `area_for_dir` is `""` at the root. There is no `"root"` sentinel; a
real directory named `root` is an ordinary area. Two consequences:

- Never `root.join(package_area)` without skipping `""` first: the join yields
  the root itself, so an area-containment scan claims every path for the
  first top-level package. `package_area_for_dir` already skips it.
- The `(root)` label exists only at CLI render time (`area_display_label`);
  JSON and library values stay `""`. `sniff repo area` prints an empty line and
  exits 0 at the root; `package-area` treats `""` as no result.

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

`GitInfo.current_worktree` names the observed linked worktree (directory
basename; `None` in the main checkout) for every `GitRequest` preset, from the
open handle and without enumerating worktrees. Evidence builders use it rather
than widening a summary request with `include_worktrees`.

Ordinary aggregate projection reuses worktree metadata and opens zero linked
repositories. Focused inspection may open a registered target to validate it:

- Current/main/linked valid targets are reported.
- A registered target whose directory or `.git` file is absent is stale and
  omitted, matching what `git worktree list` marks as prunable.
- A target whose `.git` file exists but does not open as a repository is
  corrupt and is an error.
- Ahead/behind work follows the focused detail request and does not widen the
  aggregate path.

## Recent commits

`RecentCommits::collect(&repo, &options)` gathers the set;
`RecentCommits::plain_blocks(&options)` owns the per-commit plain block. The
set-level `to_plain` and `sniff repo recent-commits --plain` are those blocks
joined by one blank line. Every collected commit has a block, so a consumer
that omits no-file commits (Darkmatter's `ctx.recent_commits`) filters on
`commit.files.is_empty()` itself, and takes the blocks rather than splitting
the set output on blank lines, which the verbose layout also uses inside a
block.

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

### Blocking PR-for-branch lookup

`remote::blocking::pull_request_for_branch` (and `_with`, for a prebuilt
client) runs a focused branch query on its own current-thread runtime. Never
call it from inside a Tokio runtime. The `deadline` replaces the client's 5 s
per-request timeout for every request in the lookup. Keep new blocking entry
points on the shared `run_with_deadline` helper.

- Filter by branch server-side (GitHub `head=owner:branch`, GitLab
  `source_branch`, Bitbucket `q=` plus `state=OPEN&state=MERGED`, Forgejo
  `head`). Upstream Gitea has no head filter, so page it with `limit`, because
  Gitea ignores `per_page`.
- Always re-check the branch and source repository locally. GitLab compares
  `source_project_id`, which costs one `projects/{path}` lookup for a fork.
- Derive state per provider (GitLab `opened`/`merged`, Bitbucket
  `OPEN`/`MERGED`, and `merged_at` or Gitea `merged` elsewhere).
  Closed-unmerged, DECLINED, and SUPERSEDED PRs never count.
- Take the Gitea head SHA and branch from the list payload (`head.label`),
  never from the single-PR endpoint, which reports the live branch tip.
- Store the head SHA as received. Bitbucket's 12-character value is a prefix.
- A list 404 is `PrUnavailable::NotFoundOrNotPermitted`, because providers hide
  private repositories behind 404. Only the exact lookup `get_pull_request`
  keeps "404 means absent". `query_pull_requests` still treats a list 404 as
  exhaustion.
