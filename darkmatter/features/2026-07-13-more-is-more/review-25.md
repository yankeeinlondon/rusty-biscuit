---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T11:33:08-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: false
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-25.md
previous: 2026-07-13-more-is-more/review-24.md
---

# Review 25

## Summary

The feature is **not production ready**. Review 24's credential-disclosure and
Azure DevOps response-contract findings are resolved: ambiguous discovery now
probes anonymously, retries with an exact-host/provider credential only after a
provider signature is present, and identifies Azure DevOps Server from the
documented on-premises connection-data fields. The replacement authentication
flow is incomplete, however. It cannot use the new host-bound credential when
an authentication boundary returns an ordinary unsigned `401`/`403`, even
though authenticated ambiguous-host probes are part of the specification. The
canonical Sniff Level-1 gate also remains incomplete because the host still has
two corrupt linked-worktree registrations.

## Review-24 Disposition

- **Resolved:** candidate discovery requests carry no global or unrelated
  provider token. The new Wiremock test configures every global token and
  verifies that only the GitLab host-bound token reaches the signed GitLab
  retry.
- **Resolved:** Azure DevOps Server detection now requires non-empty
  `instanceId` and `deploymentId` plus `deploymentType: OnPremises`, retains no
  invented version, and rejects hosted or malformed identities.
- **Partially resolved:** a signed authentication challenge is attributed to
  the correct provider and distinguishes missing, invalid, and forbidden
  credentials. An ordinary unsigned authentication challenge cannot enter that
  flow.
- **Unresolved:** `cd sniff && just test` still fails because of the host's
  corrupt `/private/tmp/dmbench/before` and `/private/tmp/dmbench/after`
  registrations.

## Findings

### High: Authenticated ambiguous-host discovery requires an identifying error body

The anonymous probe parses the response body before considering `401` or `403`
and simply skips the candidate when the body has no provider signature
([remote_observation.rs:453](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L453),
[remote_observation.rs:466](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L466)).
Consequently, the host-bound credential lookup and authenticated retry are
reachable only when an unauthenticated error response already contains the same
provider-identifying JSON fields as a successful identity response. A generic
reverse proxy or server authentication boundary returning an ordinary JSON or
HTML `401` cannot be discovered, regardless of whether the user configured the
exact `SNIFF_{PROVIDER}_{ENCODED_HOST}_TOKEN` credential.

The regression test encodes this limitation as expected behavior: its GitLab
fixture returns `version` and `revision` in the `401`, while the generic `401`
fixture expects `UnsupportedProvider` after five anonymous requests
([remote_observation.rs:362](../../../sniff/lib/tests/remote_observation.rs#L362),
[remote_observation.rs:437](../../../sniff/lib/tests/remote_observation.rs#L437)).
This does not satisfy the test strategy's required “authenticated probes
without credential disclosure” coverage and leaves AC21/AC27 incomplete for
authenticated self-hosted servers. The new host-bound credential source is
safe but cannot act as the provider-selection signal needed when the challenge
itself is unsigned.

Recommendation: when anonymous probing yields only unsigned authentication
challenges, use an explicit provider selection or the presence of exactly one
host-bound provider credential as the provider identity, then retry only that
candidate. Reject multiple configured candidates as ambiguous and never fall
back to global provider tokens. Add a Wiremock Level-1 matrix whose anonymous
responses contain no provider fields and verify successful, missing, invalid,
forbidden, multiple-candidate, and redaction outcomes.

Strongest verification present: **Level 1/Wiremock proves the safe signed-401
path and explicitly proves that an unsigned 401 remains unidentified; there is
no successful Level-1 authenticated probe for a normal unsigned challenge**.

GitNexus reports **HIGH** upstream impact for `probe_self_hosted_provider`
(nine affected symbols, two direct callers, three modules) and
`host_bound_provider_token` (39 affected symbols, two direct callers, four
modules). The latter reaches every focused PR and CI/CD query path.

### High: The canonical Sniff Level-1 acceptance gate is still incomplete

A fresh `cd sniff && just test` stopped after 1,307 of 1,631 selected tests:
1,305 passed, two failed, three were skipped, and 324 were not run. The failures
were `test_detect_with_base_dir` and `test_skip_os_with_filesystem_only`; each
exhausted its retries while opening the host's registered
`/private/tmp/dmbench/before` or `/private/tmp/dmbench/after` checkout and
received `NotARepository(MissingHead)`. This is host contamination rather than
evidence of a new feature regression, but AC29 explicitly requires the full
Sniff Level-1 suite to pass, so the acceptance gate remains unproven.

Recommendation: prune or repair those registrations, or run the canonical gate
in a clean CI user environment and record a complete result. Keep the existing
corrupt-worktree error behavior; masking the corruption is not an acceptable
test fix.

Strongest verification present: **Level 1 focused discovery regressions pass
(four of four), Sniff lint passes, and the full Darkmatter consumer gate passes;
the full Sniff Level-1 gate is incomplete**.

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
| 16 | Scoped suites and three-OS compile | L1 on 3 OSes | macOS/Linux pass; Windows GNU target pass, native MSVC absent | Pass under prior non-blocking ruling |
| 17 | First/last index functions | L1 | L1 | Pass |
| 18 | Object/array literal grammar and evaluation | L1 | L1 | Pass |
| 19 | Shared preferred-remote selection | L1 | L1 | Pass |
| 20 | Live branch observation and non-mutation | L1 + Wiremock | L1 + Wiremock | Pass |
| 21 | Six vendor values and bounded ambiguous-host probe | L1 + Wiremock | Anonymous and signed-401 fixtures; unsigned authenticated discovery fails | **Fail: authenticated probe incomplete** |
| 22 | Exact/paginated PR API, capabilities, no ignored filters | L1 + Wiremock | L1 + Wiremock | Pass |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock | Pass |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 + Wiremock | Pass for valid discovery paths |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 + Wiremock | Pass for valid discovery paths |
| 26 | Three-surface policy/cache/error parity and no credential disclosure | L1 + Wiremock | L1 header isolation and host-bound query reuse | Pass |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | Signed auth is focused; unsigned auth collapses to unsupported provider | **Fail at ambiguous authenticated discovery** |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | Focused macOS tests pass; full Sniff gate incomplete; prior Linux/Windows GNU evidence | **Gap** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies`. Sniff owns the changed remote-discovery and
credential symbols; Darkmatter, Darkmatter CLI, and DMLS are the feature
consumers. GitNexus impact results for the affected symbols are recorded above.

Fresh macOS gates:

- Focused Sniff nextest selection for the host-bound credential path, Azure
  identity, all six server flavors, and production-path self-managed GitLab —
  pass: four tests.
- `cd sniff && just test` — incomplete: 1,305 passed, two failed because of the
  host's corrupt linked worktrees, three skipped, and 324 not run.
- `cd sniff && just lint` — pass.
- `cd darkmatter && just test` — pass: 5,937 Darkmatter tests, 561 CLI tests,
  and 633 DMLS tests.
- `cd darkmatter && just lint` — pass for all three packages.
- `git diff --check` — run after review/frontmatter edits.

Recorded cross-platform evidence remains:

- Linux AArch64 — Sniff and Darkmatter build, full Level 1, and lint pass.
- Windows GNU target — Sniff with/without `remote`, Sniff CLI, Darkmatter,
  Darkmatter CLI, and DMLS compile checks pass.
- Native Windows/MSVC — no passing result; treated as non-blocking by the
  explicit prior review decision.

## Production Readiness

**Not ready for production.** Authenticated discovery of an ambiguous
self-hosted server fails whenever its anonymous authentication challenge lacks
provider-identifying fields, leaving AC21 and AC27 incomplete. AC29's canonical
Sniff Level-1 gate also remains incomplete on this host.
