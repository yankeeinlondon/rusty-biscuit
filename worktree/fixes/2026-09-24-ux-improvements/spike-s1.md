---
spike: S1
date: 2026-09-24
---

# Spike S1: PR identity fields across providers

`kind` is omitted because the repository has no `schemas/kind.yaml`. `schemas/` holds only `feature-review.yaml`, `memory.yaml`, and `suggestion-review.yaml`.

## Summary

- Every provider exposes the source repository identity, a source head SHA, and enough state to tell open, merged, and closed-unmerged apart. Only GitHub and GitLab let a single list call filter by branch across merged PRs server-side. Bitbucket Cloud can do the same with `q=` plus a repeated `state=` parameter. Upstream Gitea has no branch filter on the list endpoint. Forgejo adds `head=`.
- **Bitbucket Cloud abbreviates the source head.** The schema pattern is `[0-9a-f]{7,}?` (7 or more characters), and real payloads carry 12 characters. The other three return full object IDs: 40 characters for SHA-1, and 64 for SHA-256 repositories on Gitea and GitLab.
- **The unsafe case is Gitea's single-PR endpoint.** `GET /pulls/{n}` reports the **live branch tip** in `head.sha` while the head branch still exists, even after the PR is merged. The list endpoint reads the frozen `refs/pull/N/head`. A branch that gains commits after its merge would falsely match through the single-PR endpoint.
- **sniff today:** `PullRequestInfo` carries neither the source repository nor the head SHA, although every provider already deserializes a head SHA and GitLab also deserializes `source_project_id`. The Stage-1 path reads the first page only. The focused client turns a list-endpoint 404 into an **empty list**, which is exactly the GitHub private-repository case. Details and the required changes follow.

## Field-mapping table

| Provider | Source repo identity | Source head SHA (length) | Definitive state | Per-branch server filter |
|---|---|---|---|---|
| GitHub | `head.repo.full_name`, compared with `base.repo.full_name`. `head.repo` is `null` once the fork is deleted. `head.label` is `owner:branch`. | `head.sha` (40) | `state` is `open`/`closed`. Merged means `merged_at != null`, which the list endpoint returns. `merged` (bool) is returned by the single-PR endpoint only. | `head=owner:branch` with `state=all`, which covers merged PRs. `base=` is also available. |
| GitLab | `source_project_id` compared with `target_project_id`. The payload has no path, so resolving one takes `GET /projects/:id` (or compare IDs after one `GET /projects/:encoded_path` for the local fork). | `sha` (40, or 64 on SHA-256) in both list and single responses. `diff_refs.head_sha` is single-MR only. | `state` is one of `opened`, `closed`, `locked`, `merged`, returned by both list and single endpoints. | `source_branch=` with `state=all` (the default), which covers merged. `target_branch=` is also available. Pair it with the source project check, because MRs from different forks can share a branch name. |
| Gitea | `head.repo.full_name` or `head.repo_id`, compared with `base.repo_id`. `head.repo` is `null` and `repo_id` is -1 when the head repo is gone. | `head.sha` (40, or 64). **The list endpoint reads `refs/pull/N/head`. The single endpoint reads the live branch tip when the branch exists** (see caveats). | `state` is `open`/`closed`. Merged means `merged: true` or `merged_at` set, and both endpoints return it. | List endpoint: `base_branch=` only, no head filter. Use `GET /repos/{o}/{r}/pulls/{base}/{owner[/repo]:branch}`, which returns one PR by base, head repo, and head branch. It is unordered, may be old, and requires knowing the base. |
| Forgejo | Same shape as Gitea | Same as Gitea (fork of Gitea 1.22). Not separately verified. | Same as Gitea | The list endpoint adds `head=` (the head branch name) and `base=` |
| Bitbucket Cloud | `source.repository.full_name` (and `uuid`), compared with `destination.repository.full_name` | `source.commit.hash`, **abbreviated to 12 characters in practice**. The schema only guarantees 7 or more. | `state` is one of `OPEN`, `MERGED`, `DECLINED`, `SUPERSEDED`, returned by the list endpoint. No `merged_at` exists. | `q=source.branch.name="x"` **plus** repeated `state=OPEN&state=MERGED`. Without `state`, the endpoint returns OPEN only. |
| Bitbucket Server/DC | `fromRef.repository.project.key`/`slug` (`/rest/api/1.0`) | `fromRef.latestCommit` (40) | `state` is `OPEN`/`MERGED`/`DECLINED` | `at=refs/heads/x&direction=OUTGOING&state=ALL` |

Bitbucket Server/DC is **not supported by sniff**. `BitbucketRemote::with_base_url` (`sniff/lib/src/remote/bitbucket.rs:63-67`) only swaps the base URL but keeps the Cloud 2.0 payload shapes, and the 1.0 API uses `fromRef`/`toRef`. Treat it as "no PR evidence".

**Pagination defaults:**

| Provider | Default page size | Maximum | How to page |
|---|---|---|---|
| GitHub | `per_page` 30 | 100 | `Link` header |
| GitLab | `per_page` 20 | 100 | Page parameters |
| Gitea/Forgejo | `limit` = `[api] DEFAULT_PAGING_NUM` (30) | `MAX_RESPONSE_ITEMS` (50) | `limit`; **the server ignores `per_page`** |
| Bitbucket | `pagelen` 10 | 50 on the PR endpoint (not verified against current docs) | Follow `next` |

A per-branch filter keeps the result to one page in practice. Only the unfiltered Gitea list (the "repo's open PRs" entry point) needs real pagination.

## Merged-PR head-SHA caveats

1. **GitHub:** `head.sha` stays frozen at the head that was merged. Pushes after the merge do not change it. After "Rebase and merge" or "Squash", `head.sha` still equals the user's local tip even though those commits are not reachable from the default branch. That is exactly the case PR evidence exists for.
2. **GitLab:** `sha` stays frozen at the MR's last head. **A server-side rebase rewrites the source branch before merging.** That covers the Rebase button and fast-forward-only projects. `sha` then names the rebased commit, the pre-rebase local tip no longer matches, and the check falls through to the non-Safe tiers. That outcome is conservative and correct.
3. **Gitea single-PR endpoint (unsafe):** `ToAPIPullRequest` sets `head.sha` from the head branch's current commit whenever the branch exists (`services/convert/pull.go`, Gitea main, around lines 195-225). Only a deleted branch falls back to `refs/pull/N/head`. The list endpoint (`ToAPIPullRequests`, around line 448) always reads `refs/pull/N/head`, which Gitea stops updating at merge. **Take head SHA evidence for merged Gitea/Forgejo PRs from the list endpoint only.** Otherwise, for a merged PR, cross-check `head.sha` against `GET /repos/{o}/{r}/git/refs/pull/{n}/head`.
4. **Gitea branch name after deletion:** in both converters, `head.ref` becomes `refs/pull/N/head` once the head branch is deleted. `head.label` always carries the branch name. sniff maps `head.ref`, so for a merged PR whose branch was deleted it reports the wrong `source_branch`.
5. **Bitbucket:** `source.commit.hash` is the PR's recorded source commit, abbreviated. Atlassian has declined to change this ([community thread](https://community.atlassian.com/forums/Bitbucket-questions/Hash-format-in-Pull-Request-event-payloads-Webhooks/qaq-p/2142250)). For merged PRs sniff's Stage-1 path derives `merged_at` from `updated_on` (`bitbucket.rs:510`), and the focused path leaves it `None` (`focused.rs:774`). State must come from `state == "MERGED"`, not from `merged_at`.
6. **All providers:** a deleted fork (`head.repo` null on GitHub and Gitea, an unresolvable `source_project_id` on GitLab, a null `source.repository` on Bitbucket) gives no identity. Treat that as no Safe evidence.

## Truncated SHA recommendation (R8)

Confirm R8, with three amendments:

1. **Base the comparison on the local object ID length, not the literal 40.** Gitea and GitLab can serve SHA-256 repositories, where full IDs are 64 characters. The rule becomes "provider SHA shorter than the local tip's full ID".
2. **Enforce a floor of 7 hex characters,** which is Bitbucket's schema minimum. Reject anything shorter, and anything that is not lowercase hex, as "no Safe evidence".
3. **Define "unique prefix match" as follows:**
   - the local tip starts with the provider prefix, **and**
   - `git rev-parse --disambiguate=<prefix>` (or `git cat-file -e` combined with an ambiguity check) resolves to exactly one object in the local repository.

   Comparing only against the tip leaves a theoretical collision in which the PR's real head is a different object sharing a 12-character prefix. The uniqueness check closes that locally without a network call.

Rejected alternative: resolving the short hash through Bitbucket's `GET /2.0/repositories/{ws}/{repo}/commit/{hash}`. That costs one extra call per PR and a second failure mode, and a local check answers the same question.

## sniff's current mapping and error types

**Record (`sniff/lib/src/remote/types.rs:259`):** `PullRequestInfo` has `state` (a free-form string), `source_branch`, `target_branch`, `merged_at`, and `html_url`, but **no source repository and no head SHA**.

**Stage-1 providers (`RemoteRepoProvider::list_pull_requests`):**

| Provider | Mapping | Already deserialized but dropped | Notes |
|---|---|---|---|
| GitHub | `github.rs:426-440`, taking `state`, `head.ref_name`, and `merged_at` | `head.sha` and `head.repo.full_name` (`schematic/definitions/src/github/types/pull_requests.rs:10-21`) | The request type has no `head` parameter (`schematic/schema/src/github/requests.rs:281-296`). Its doc says "default: 100", but GitHub's default is 30. The Stage-1 list sends no `per_page`, so it reads one page of 30. |
| GitLab | `gitlab.rs:413-430` | `sha`, `source_project_id`, `target_project_id` (`schematic/definitions/src/gitlab/types/merge_requests.rs:31-47`) | The request has no `source_branch` field (`schematic/schema/src/gitlab/requests.rs:452-460`). One page (20). |
| Gitea | `gitea.rs:466-501`, taking `head.ref` as the source branch | `head.sha` (`schematic/definitions/src/gitea/types.rs:211-212`) | `has_merged` (`types.rs:248`) has no `rename = "merged"`, so it is **always `None`**. It is unused, but the merged bool is not captured. `head.label`, `head.repo`, and `repo_id` are not deserialized. |
| Bitbucket | `bitbucket.rs:488-530` | `source.commit.hash` and `source.repository.full_name` (`schematic/definitions/src/bitbucket/types/pull_requests.rs:111-150`, `repos.rs:207-210`) | The All and Closed states leave `state` unset (`bitbucket.rs:462-465`). The comment there says this returns all PRs, but the API returns **OPEN only**. **That is comment drift and a real bug.** First page only (`bitbucket.rs:487`). |

**Focused client (`sniff/lib/src/remote/focused.rs`):**
- `normalize_pr` (`focused.rs:722-790`) is a JSON-probing normalizer with 3s connect and 5s total timeouts. It already maps `source_branch` from `head.ref` or `source.branch.name` (`:746`). It maps neither the source repository nor the head SHA.
- `query_pull_requests` (`:244-292`) paginates up to 20 pages and applies `source_branch` **locally** (`:1333-1336`). It sends no server-side branch filter.
- For Gitea and Forgejo it sends `per_page` (`:1282-1291`), which Gitea ignores. The page returns about 30 rows, which is fewer than `PAGE_SIZE=100`, so the walk stops as though exhausted. **Gitea PR queries are silently truncated at one page.**

**Can auth or permission failure be told apart from an empty list?**
- **Stage-1 path: yes, but loosely typed.** GitHub maps 401 to `InvalidCredentials` (`github.rs:215-224`), 403 to `RateLimited` if the body contains "rate limit" and to `RemoteApi{403}` otherwise (`:225-239`), and 404 to `RemoteApi{404,"Not found"}` (`:244-248`). GitLab, Gitea, and Bitbucket map 401 to `MissingCredentials` even when a token was sent (`gitlab.rs:225`, `gitea.rs:271`, `bitbucket.rs:187`), and handle 403 and 404 the same way as GitHub. Nothing returns `Ok(vec![])` on error. A GitHub private-repository 404 cannot be told apart from "repo does not exist", but it is still an error.
- **Focused path: typed, except for 404.** `get_json` (`focused.rs:571-596`) maps 401 to `MissingCredentials` or `InvalidCredentials` depending on whether a token was present, 403 to `RemoteForbidden` (`error.rs:215`), 429 to `RateLimited`, and 3xx to `RemoteUnreachable`. **It maps 404 to `Ok(None)`.** `query_pull_requests` treats `None` as "exhausted" (`:258-260`) and returns an **empty page**. **A GitHub private repository queried without access, and Gitea or Bitbucket private-repository 404s, therefore look like "no PRs".** This violates the plan's rule that auth or permission failures are `Unavailable`, never `None`.

## Required changes for Phase 2 ("sniff: PR for one branch")

1. **Extend the record** rather than adding a parallel type. Add `source_repo: Option<String>`, holding `full_name`, the GitLab project path, or the Bitbucket `full_name`, plus `source_repo_is_target: Option<bool>` and `source_head_sha: Option<String>`. The SHA must be stored exactly as received, with no padding. Add the fields to `PullRequestInfo` and to both mappers (Stage-1 and `normalize_pr`). Record the SHA's abbreviation implicitly through its length, and document that R8 treats any value shorter than the local ID length as a prefix.
2. **Build `blocking::pull_request_for_branch` on `FocusedProviderClient`,** not on Stage-1. The focused client already has host-bound credentials, redirect blocking, and typed 401/403 handling. The `deadline` argument has to replace the hard-coded 5s timeout (`focused.rs:533-534`).
3. **Add a list-scoped 404 policy.** A 404 on a *list* endpoint must surface as an error, mapped to `PrUnavailable` as "not found or not permitted". Only the exact-lookup `get_pull_request` keeps "404 means `None`".
4. **Use server-side branch filters:**
   - GitHub: `head={source_owner}:{branch}&state=all`.
   - GitLab: `source_branch={branch}&state=all`, then keep MRs whose `source_project_id` equals the resolved source project ID. That takes one `GET /projects/{encoded}` for the source repository.
   - Bitbucket: `q=source.branch.name="{branch}"&state=OPEN&state=MERGED`, then compare `source.repository.full_name`.
   - Forgejo: `head={branch}&state=all`.
   - Gitea: paginate `state=all` with `limit=50`, then filter locally on `head.label` and `head.repo.full_name`.
5. **Choose between multiple matches by preferring OPEN over MERGED.** If several merged PRs match, choose the one whose head SHA matches the local tip. If none match, return the most recent. Never let `closed`-unmerged, `DECLINED`, or `SUPERSEDED` PRs grant evidence.
6. **Derive state per provider:**

   | Provider | Merged when | Open when |
   |---|---|---|
   | GitHub, Gitea, Forgejo | `merged_at != null`, or Gitea `merged == true` | `state == "open"` |
   | GitLab | `state == "merged"` | `state == "opened"` |
   | Bitbucket | `state == "MERGED"` | `state == "OPEN"` |

   Expose the result as an enum (`Open`, `Merged`), not a string.
7. **Take the Gitea head SHA from the list payload only** (caveat 3), and read the Gitea branch name from `head.label` (caveat 4).
8. **Drift fixes found in passing** (separate commits, per Rule 3):
   - Stage-1 Bitbucket All/Closed returns OPEN only (`bitbucket.rs:462-465`).
   - Gitea `has_merged` has no serde rename.
   - Focused Gitea pagination sends `per_page` instead of `limit`.
   - The GitHub request docs claim a default of 100.
9. **Tests (wiremock), per provider:** add a Bitbucket 12-character hash case, a Gitea merged PR with a moved branch, and a GitHub list 404 that must yield `Unavailable`, in addition to the plan's list.

Sources: [GitHub pulls REST](https://docs.github.com/en/rest/pulls/pulls?apiVersion=2022-11-28); [GitLab merge requests API](https://docs.gitlab.com/api/merge_requests/); [GitLab REST pagination](https://docs.gitlab.com/api/rest/#pagination); Gitea swagger (`https://gitea.com/swagger.v1.json`, 1.27-dev) and source ([`routers/api/v1/repo/pull.go`](https://github.com/go-gitea/gitea/blob/main/routers/api/v1/repo/pull.go), [`services/convert/pull.go`](https://github.com/go-gitea/gitea/blob/main/services/convert/pull.go), [`routers/api/v1/utils/page.go`](https://github.com/go-gitea/gitea/blob/main/routers/api/v1/utils/page.go)); Forgejo swagger (`https://codeberg.org/swagger.v1.json`); Bitbucket Cloud OpenAPI (`https://dac-static.atlassian.com/cloud/bitbucket/swagger.v3.json`: the `pullrequest_endpoint.commit.hash` pattern `[0-9a-f]{7,}?`, OPEN-only default, repeatable `state`); [Atlassian KB on filtering merged PRs by branch](https://support.atlassian.com/bitbucket-cloud/kb/bitbucket-cloud-how-to-programmatically-list-merged-pull-requests-from-a-source-branch-during-branch-comparison/).
