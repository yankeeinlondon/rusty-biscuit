---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T12:39:34-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-26.md
previous: 2026-07-13-more-is-more/review-25.md
---

# Review 26

## Summary

The feature is **not production ready**. Review 25's unsigned-authentication
fallback now selects exactly one exact-host/provider credential without exposing
global tokens, and the canonical Sniff Level-1 gate has passed in an isolated
repository. The fallback is only correct for the GitLab path covered by its new
test, however. Gitea and Forgejo API-key tokens require the `token` authorization
scheme, while discovery and every focused follow-up query send them as Bearer
tokens. Authenticated `remote_vendor`, pull-request, and supported Gitea CI/CD
operations therefore fail against private Gitea/Forgejo installations.

## Review-25 Disposition

- **Resolved:** when all anonymous identity probes return unsigned authentication
  challenges, exactly one configured host-bound provider credential selects one
  retry route; multiple configured candidates are rejected and global tokens are
  not sent.
- **Resolved:** the canonical `cd sniff && just test` Level-1 gate passed in the
  clean-environment run recorded by the implementation log: 1,632 Sniff tests and
  769 Sniff CLI tests passed.
- **Partially resolved:** the unsigned-challenge behavior is verified for GitLab,
  including success, missing/invalid/forbidden credentials, ambiguity, and secret
  redaction. The provider-specific authentication branches are not covered, and
  the shipped Gitea/Forgejo branch uses the wrong scheme.

## Findings

### High: Host-bound Gitea/Forgejo API-key credentials are sent as Bearer tokens

Both authenticated discovery paths special-case GitLab and Azure DevOps, then
send every other provider credential with `bearer_auth`
([remote_observation.rs:483](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L483),
[remote_observation.rs:580](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L580)).
The focused client created after discovery also sends every host-bound token as
Bearer regardless of provider
([focused.rs:535](../../../sniff/lib/src/remote/focused.rs#L535),
[focused.rs:545](../../../sniff/lib/src/remote/focused.rs#L545)). This contradicts
the repository's existing Gitea contract: `GITEA_TOKEN` is a personal API key
and must be sent as `Authorization: token <pat>`, not Bearer
([gitea.rs:27](../../../sniff/lib/src/remote/gitea.rs#L27),
[gitea.rs:35](../../../sniff/lib/src/remote/gitea.rs#L35)); the generated-client
definition already models the same `token ` prefix
([gitea/mod.rs:118](../../../schematic/definitions/src/gitea/mod.rs#L118),
[gitea/mod.rs:162](../../../schematic/definitions/src/gitea/mod.rs#L162)). The
official [Gitea API](https://docs.gitea.com/next/development/api-usage) and
[Forgejo API](https://forgejo.org/docs/v15.0/user/api-usage/) documentation
specifies that historical API-key scheme as well.

Consequently, a private Gitea installation that returns an unsigned `401` from
`/api/v1/version` receives `Authorization: Bearer <GITEA_TOKEN>` on retry and
rejects a valid API key. Even when discovery succeeds anonymously, subsequent
`pr`, `pr_list`, and Gitea 1.25+ `cicd`/`cicd_list` requests repeat the wrong
Bearer scheme. Forgejo access tokens have the same API-key form; Bearer is a
distinct OAuth/JWT credential form and the host-bound environment contract does
not identify the supplied token as OAuth.

The tests do not exercise this boundary. The new unsigned-challenge matrix sets
only the GitLab host-bound credential and asserts `PRIVATE-TOKEN`
([remote_observation.rs:455](../../../sniff/lib/tests/remote_observation.rs#L455)).
The production-path Gitea/Forgejo test returns `200` without requiring any
authorization header, so it cannot detect a wrong credential scheme
([focused_provider.rs:2023](../../../sniff/lib/tests/focused_provider.rs#L2023)).
The six-flavor test likewise covers anonymous signatures only.

Recommendation: introduce one provider-aware request-authentication helper and
reuse it for signed discovery retries, unsigned discovery retries, and focused
queries. Preserve GitLab's supported token form and Azure DevOps Basic auth;
emit `Authorization: token <pat>` for Gitea/Forgejo API-key credentials. Add a
Level-1 Wiremock matrix that requires the exact header for GitHub Enterprise,
GitLab self-managed, Gitea, Forgejo, Bitbucket Data Center, and Azure DevOps
Server, plus authenticated private Gitea/Forgejo pull-request and supported
Gitea job queries. Each case must also prove global-token isolation and secret
redaction.

Strongest verification present: **Level 1/Wiremock verifies the GitLab
host-bound path and anonymous discovery for all six server flavors, but no
Level-1 test authenticates Gitea/Forgejo discovery or a private focused query**.
The verification level is appropriate; the provider/authentication matrix is
incomplete.

GitNexus reports **HIGH** upstream impact for
`probe_self_hosted_provider` (ten affected symbols, two direct callers, three
modules) and `host_bound_provider_token` (39 affected symbols, two direct
callers, four modules). `FocusedProviderClient::get_json` has **MEDIUM** impact
(41 affected symbols, five direct callers, two modules). The affected paths
reach `remote_vendor` and the focused PR/CI query surface.

## Verification Matrix

All feature behavior is process-local filesystem/Git/parser/network/Markdown/LSP
behavior. Level 2 and Level 3 are not applicable.

| AC | User-observable contract | Required | Strongest present | Review result |
|---:|---|---:|---:|---|
| 1 | Generated Git context schema/catalog | L1 | L1 | Pass |
| 2 | Demand-driven shared Git capture | L1 | L1 | Pass |
| 3 | Attached-branch/null behavior | L1 | L1 | Pass |
| 4 | Linked-worktree/null behavior | L1 | L1 | Pass |
| 5 | Conflict-path capture and falsy empty array | L1 | L1 | Pass |
| 6 | Shared read-only conflict API | L1 | L1 | Pass |
| 7 | Hermetic in-memory merge probe | L1 | L1 | Pass |
| 8 | Git/git2 conflict-oracle parity | L1 | L1 | Pass |
| 9 | `predict_conflicts` direction and paths | L1 | L1 | Pass |
| 10 | Clean/edge/error outcomes | L1 | L1 | Pass |
| 11 | Independence from live index/worktree dirt | L1 | L1 | Pass |
| 12 | Caller anchoring and three-surface parity | L1 | L1 | Pass |
| 13 | No repository mutation/subprocess/network | L1 | L1 | Pass |
| 14 | Function catalog and passive editor metadata | L1 | L1 | Pass |
| 15 | Generated documentation/skill consistency | L1 | L1 | Pass |
| 16 | Scoped suites and three-OS compile | L1 on 3 OSes | Prior macOS/Linux and Windows GNU evidence | Pass under prior non-blocking ruling |
| 17 | First/last index functions | L1 | L1 | Pass |
| 18 | Object/array literal grammar and evaluation | L1 | L1 | Pass |
| 19 | Shared preferred-remote selection | L1 | L1 | Pass |
| 20 | Live branch observation and non-mutation | L1 + Wiremock | L1 + Wiremock | Pass |
| 21 | Six vendor values and bounded authenticated probe | L1 + Wiremock | GitLab authenticated; six flavors anonymous | **Fail for Gitea/Forgejo API-key authentication** |
| 22 | Exact/paginated PR API and capabilities | L1 + Wiremock | Public Gitea/Forgejo; authenticated GitLab | **Fail for private Gitea/Forgejo** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock except private Gitea/Forgejo | **Fail for authenticated Gitea/Forgejo** |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | Public fixtures | **Fail for authenticated supported Gitea jobs** |
| 25 | `cicd*` overloads, pagination, projection | L1 + Wiremock | Public fixtures | **Fail for authenticated supported Gitea jobs** |
| 26 | Three-surface policy/cache/credential parity | L1 + Wiremock | GitLab host-bound path | **Fail for Gitea/Forgejo credential use** |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | Wrong-scheme `401` becomes invalid credentials | **Fail: valid API keys are rejected** |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | Full gates recorded; provider-auth matrix incomplete | **Gap** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies`. Sniff owns the affected credential and
provider-discovery implementation; Darkmatter, Darkmatter CLI, and DMLS are the
specified consumers.

Fresh macOS focused Level-1 checks:

- Three `remote_observation` tests passed: signed GitLab host-bound discovery,
  unsigned GitLab host-bound discovery, and anonymous six-flavor discovery.
- Two `focused_provider` tests passed: the authenticated self-managed GitLab
  production path and the anonymous Gitea/Forgejo production path.
- These passing selections reproduce the coverage gap: no selected or existing
  production-path test requires Gitea/Forgejo API-key authentication.

Recorded implementation evidence retained from Review 25:

- Clean-environment `cd sniff && just test` — pass: 1,632 Sniff tests and 769
  Sniff CLI tests.
- `cd sniff && just lint` — pass.
- Prior full Darkmatter library, CLI, and DMLS Level-1/lint gates — pass.
- Linux AArch64 and Windows GNU compile evidence remains recorded; native
  Windows/MSVC remains subject to the prior explicit non-blocking ruling.

## Production Readiness

**Not ready for production.** A valid host-bound Gitea/Forgejo API-key
credential is encoded with the wrong authorization scheme in both discovery and
focused provider requests. This leaves AC21 through AC27 and the provider-auth
portion of AC29 incomplete despite the passing GitLab-focused tests.
