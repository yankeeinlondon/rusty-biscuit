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

`filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES` is the one public list of
package manifest file names; use it instead of copying the names.

`RepoInfo.monorepo_standards` lists detected standards with resolved binaries
and confidence. `RepoInfo.monorepo_layers` lists membership layers. Every layer
has one authority, optional orchestrators, provenance, a required `lockfile`
observation, and paths into the canonical `RepoInfo.packages` catalog.
Standalone Poetry, PDM, and Composer lockfiles sit beside the layers in
`RepoInfo.standalone_lockfiles`. See the lockfile pipeline in
[architecture.md](architecture.md#lockfile-observation-pipeline).

The removed `MonorepoTool`, `workspace_tools`, `discovery_sources`, and
`lockfile_match` surfaces must not return. CLI labels use each standard's
stable `spec().label`.

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

### Blocking open-PR listing

`remote::blocking::open_pull_requests` (and `_with`) lists a repository's open
PRs as `PrSummary` through the same `run_with_deadline` helper and error
contract. Both entry points page through the focused client's private
`pr_list_rows`, which uses `NotFound::Error`; keep new PR list walks on it.

- Filter server-side by the open state (GitHub/Gitea `open`, GitLab `opened`,
  Bitbucket `OPEN`) and re-check locally with the same per-provider state rule.
- A GitLab fork MR names its source project only by ID. Resolve each distinct
  fork once with `projects/{id}`; a 404 there leaves `source_repo: None`
  instead of failing the list.
- Callers match a PR to a local branch on source repository *and* branch.

### Blocking branch head

`remote::blocking::branch_head` (and `_with`) returns `BranchHead { sha }`
through the same `run_with_deadline` helper, from the focused client's
`branch_head` request (GitHub singular `git/ref/heads/{branch}`, since the
plural `refs` prefix-matches and can answer an array; Gitea/Forgejo
`branches/{branch}`; GitLab `repository/branches/{branch}`; Bitbucket
`refs/branches/{branch}`).

- The branch goes through `path_segment`, so `/`, space, `?`, `#`, and
  Unicode stay inside one segment. GitLab rejects a raw `/`.
- It uses `NotFound::Error`: a 404 is `NotFoundOrNotPermitted`, never absence.
- The SHA must be 40 or 64 lowercase hex digits, or the lookup is `Other`.
- There is no anonymous retry on any blocking path (the schematic-backed
  provider modules have one; the focused client does not). A rejected or
  insufficient token is reported, and the caller decides on a fallback.

### Blocking credentials classification

`focused.rs` splits a 403 while the headers and body are still available;
`blocking::classify` sees only the `SniffError` plus the `key` that
`run_with_deadline` resolved from `FocusedProviderClient::credential_key`.

- 429, or a 403 with `x-ratelimit-remaining: 0` or a "rate limit" body, is
  `SniffError::RateLimited` → `RateLimited { authenticated, key }`.
- A 403 with a token whose body names a missing permission or scope is
  `RemoteForbidden` carrying `INSUFFICIENT_CREDENTIALS_MESSAGE` →
  `CredentialsInsufficient { key }`. `classify` keys on that exact text; any
  other 403 carries a different message and becomes `NotFoundOrNotPermitted`.
- 401 is `CredentialsRequired { key: None }` without a token and
  `CredentialsRejected { key }` with one.
- `credentials::provider_token_variables` is the one list of candidate names:
  `provider_token` sends the first set one and `credential_env` reports them in
  that order. It differs from schematic's `env_auth` (GitHub order, and
  Bitbucket's `BITBUCKET_TOKEN` versus username/app password).
- No `PrUnavailable` message may carry a token: `client_for_url` never echoes
  the remote URL, whose userinfo may hold one.
