---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-21T13:23:59-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: false
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-27.md
previous: 2026-07-13-more-is-more/review-26.md
---

# Review 27

## Summary

The feature is **production ready**. Review 26's provider-authentication finding
is resolved: one provider-aware helper now applies the correct credential scheme
to both blocking discovery and asynchronous focused queries
([credentials.rs:37](../../../sniff/lib/src/credentials.rs#L37),
[remote_observation.rs:483](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L483),
[remote_observation.rs:580](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L580),
[focused.rs:545](../../../sniff/lib/src/remote/focused.rs#L545)). Private Gitea
and Forgejo requests use `Authorization: token <pat>`, GitLab uses
`PRIVATE-TOKEN`, Azure DevOps Server uses Basic authentication, and GitHub
Enterprise/Bitbucket Data Center retain Bearer authentication.

No new correctness, completeness, ergonomics, performance, safety, or
requirement-to-test-level finding was identified.

## Review-26 Disposition

- **Resolved:** signed and unsigned ambiguous-host discovery retries use the
  exact provider authentication scheme for all six supported self-hosted
  flavors. The Wiremock matrix also verifies that global provider tokens do not
  reach host-bound requests and that invalid credentials are not disclosed
  ([remote_observation.rs:615](../../../sniff/lib/tests/remote_observation.rs#L615)).
- **Resolved:** private Gitea and Forgejo pull-request queries use API-key
  authentication. Supported Gitea 1.25 exact and list job queries use the same
  scheme ([focused_provider.rs:2063](../../../sniff/lib/tests/focused_provider.rs#L2063)).
- **Resolved:** the focused GitLab production-path fixture now requires the
  provider's `PRIVATE-TOKEN` header, guarding the other non-Bearer branch of the
  shared helper.

## Findings

None.

## Verification Matrix

All specified behavior is process-local filesystem/Git/parser/network/Markdown/
LSP behavior. Provider traffic is exercised against Wiremock rather than a live
service. No requirement concerns terminal rendering, terminal input encoding,
or OS keyboard/mouse delivery, so Level 2 and Level 3 are not applicable.

| AC | User-observable contract | Required | Strongest present | Result |
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
| 16 | Scoped suites and three-OS compile | L1 on 3 OSes | Prior macOS/Linux and Windows GNU evidence | Pass under the prior non-blocking platform ruling |
| 17 | First/last index functions | L1 | L1 | Pass |
| 18 | Object/array literal grammar and evaluation | L1 | L1 | Pass |
| 19 | Shared preferred-remote selection | L1 | L1 | Pass |
| 20 | Live branch observation and non-mutation | L1 + Wiremock | L1 + Wiremock | Pass |
| 21 | Vendor values and bounded authenticated discovery | L1 + Wiremock | Six-flavor signed/unsigned authentication matrix | Pass |
| 22 | Exact/paginated PR API and capabilities | L1 + Wiremock | L1 + authenticated private-provider Wiremock | Pass |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock | Pass |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 + authenticated Gitea Wiremock | Pass |
| 25 | `cicd*` overloads, pagination, projection | L1 + Wiremock | L1 + Wiremock | Pass |
| 26 | Three-surface policy/cache/credential parity | L1 + Wiremock | L1 + Wiremock | Pass |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 + Wiremock | Pass |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | Full isolated L1 plus focused current-tree L1 and prior cross-platform evidence | Pass |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies`. Sniff owns the changed credential and request
implementation; Darkmatter, Darkmatter CLI, and DMLS are the specified direct
consumers. GitNexus confirms that `authenticate_provider_request` has exactly
the two intended callers: `probe_self_hosted_provider` and
`FocusedProviderClient::get_json`; the latter serves exact/list PR and CI/CD
paths.

Fresh macOS checks:

- Targeted Review-26 regressions: 3 passed.
- Complete affected Sniff integration binaries (`remote_observation` and
  `focused_provider`): 65 passed.
- Darkmatter provider-runtime L1 selection: 29 passed; one test needed its
  configured retry after a transient nextest handle-leak result, then passed.
- `cd sniff && just lint`: passed for Sniff, Sniff CLI, and checked dependencies,
  including compilation of the downstream Darkmatter library.
- `cd sniff && just test`: 1,304 tests passed before the known host-only stale
  linked-worktree registrations under `/private/tmp/dmbench/{before,after}`
  caused four cwd-sensitive discovery tests to fail and fail-fast canceled the
  remainder. The affected provider tests passed in this run. Review 26's
  implementation evidence records the same HEAD passing all 1,634 Sniff library
  tests in a disposable clone with independent Git metadata; Review 25 records
  a complete isolated Sniff library and CLI gate.

The in-tree failure is not attributed to this feature: it is reproduced by
ambient linked-worktree metadata outside the product/test patch, while the
independent-repository gate and every affected current-tree test pass. No host
Git metadata was changed during this review.

## Production Readiness

**Ready for production.** Every acceptance criterion has verification at its
appropriate level, Review 26's authenticated private-Gitea/Forgejo failure is
closed on both discovery and focused query paths, and no remaining finding was
identified.
