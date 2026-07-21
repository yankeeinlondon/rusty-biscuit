---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T10:13:31-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: codex/default
next: 2026-07-13-more-is-more/review-25.md
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-24.md
previous: 2026-07-13-more-is-more/review-23.md
---

# Review 24

## Summary

The feature is **not production ready**. Review 23's corrupt-worktree and
successful public SSH/SCP findings are resolved, and the provider probe now
attempts all six required server families. The expanded probe introduces a
critical credential-boundary violation, however: before it knows the remote's
provider, it sends each configured provider token to a different path on that
same unidentified host. Azure DevOps Server discovery also relies on a response
field that is absent from Microsoft's published `ConnectionData` contract, so
the required Azure server flavor is not actually supported by valid evidence.
Finally, the canonical Sniff Level-1 gate could not complete on this host due to
two existing corrupt linked-worktree registrations.

## Review-23 Disposition

- **Resolved:** the shared worktree-open helper distinguishes an absent stale
  checkout from an existing corrupt checkout, and both full detection and
  public worktree listing have regression tests for each case.
- **Resolved:** successful public ambiguous-host discovery now starts from real
  disposable repositories configured with SSH and SCP remotes and exercises
  `remote_vendor_at` and `FocusedProviderClient::discover`, including HTTPS
  origin synthesis, SSH-port omission, version retention, capability
  derivation, and API-base construction.
- **Partially resolved:** the bounded probe now has candidate routes for all six
  required server families and authenticated-probe tests, but its credential
  selection is unsafe and its Azure signature is incompatible with the
  published API contract.
- **Accepted evidence under the prior non-blocking ruling:** Windows GNU target
  compilation passes for Sniff, Sniff CLI, Darkmatter, Darkmatter CLI, and DMLS;
  native Windows/MSVC CI evidence remains absent.

## Findings

### Critical: Ambiguous discovery discloses unrelated provider credentials

`probe_self_hosted_provider` does not know which provider owns an ambiguous
host when it loops over GitHub, GitLab, Gitea, Bitbucket, and Azure candidate
paths. Nevertheless, each iteration loads that provider family's global environment
token and attaches it to the request
([remote_observation.rs:416](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L416),
[remote_observation.rs:435](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L435)).
An exact-host-allowed server therefore receives a configured secret for every
candidate family: GitHub/Gitea/Bitbucket bearer tokens, a GitLab private token, and an
Azure basic-auth token on separate routes. Exact-host network authorization is
not authorization to disclose credentials belonging to unrelated providers.
This violates AC26's credential-safe policy boundary and the specification's
explicit requirement for authenticated probes without credential disclosure.

The new authentication test removes the GitHub variables before setting a
GitLab token and asserts only that the GitLab token is absent from formatted
errors
([remote_observation.rs:276](../../../sniff/lib/tests/remote_observation.rs#L276)).
It does not inspect all received request headers with multiple provider tokens
configured, so it masks the cross-candidate disclosure. The same loop retains
the first `401`/`403` as its focused error; a generic authenticating reverse
proxy can consequently misreport missing GitHub credentials for a GitLab or
Azure server.

Recommendation: probe provider signatures anonymously first. Attach a
credential only after provider identity is established and only when that
credential is bound to the exact host and provider. If a server cannot expose
an anonymous identity signature, require an explicit provider selection or a
host-bound credential source instead of trying global tokens. Add a Wiremock
Level-1 matrix with all provider tokens set that asserts every candidate route
receives no unrelated credential, then verify provider-specific authenticated
retry and accurate authentication-error attribution.

Strongest verification present: **Level 1 proves one GitLab token reaches the
GitLab route and is redacted from errors; there is no Level-1 header-isolation
test, and inspection shows cross-provider disclosure**.

GitNexus reports **HIGH** upstream impact for `provider_token` (15 affected
symbols, two direct callers, four modules) and `probe_self_hosted_provider`
(nine affected symbols, two direct callers, three modules).

### High: Azure DevOps Server detection uses a non-contract response field

The Azure signature requires both `instanceId` and a `deploymentVersion` string
containing `azure devops server`, then treats that string as the version
([remote_observation.rs:561](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L561)).
Microsoft's published `ConnectionData` contract includes `instanceId`,
`deploymentId`, and `deploymentType` (`Hosted` or `OnPremises`), but it does not
define `deploymentVersion`: [Microsoft ConnectionData API](https://learn.microsoft.com/en-us/javascript/api/azure-devops-extension-api/connectiondata).
Microsoft's own captured connection-data example likewise contains deployment
identity and type but no version field: [azure-pipelines-vscode issue
538](https://github.com/microsoft/azure-pipelines-vscode/issues/538).

The Wiremock fixture invents `"deploymentVersion": "Azure DevOps Server
2022.2"`
([remote_observation.rs:214](../../../sniff/lib/tests/remote_observation.rs#L214)),
so it proves behavior against the implementation's assumption rather than the
service contract. A conforming on-premises response will be rejected as
unidentified, leaving AC21's Azure DevOps Server flavor unimplemented.

Recommendation: identify Azure DevOps Server from documented on-premises
deployment identity, including `deploymentType`, and obtain a version from a
documented endpoint/header if capability selection needs one. Otherwise make
the discovery version optional for this flavor. Replace the fabricated fixture
with the published response shape and include hosted/on-premises distinction
and missing/malformed identity cases.

Strongest verification present: **Level 1 with Wiremock against a fabricated
response shape; no valid contract-level fixture**.

GitNexus reports **HIGH** upstream impact for `discovery_from_signature` (eight
affected symbols, one direct caller, four modules).

### High: The canonical Sniff Level-1 acceptance gate is incomplete

A fresh `cd sniff && just test` stopped after 1,309 of 1,628 selected tests: two
tests failed while opening the host's existing registered linked worktrees at
`/private/tmp/dmbench/before` and `/private/tmp/dmbench/after`, both with
`NotARepository(MissingHead)`. The result was 1,307 passed, two failed, three
skipped, and 319 not run. This is consistent with the corrected contract that
existing corrupt checkouts must surface as errors, but it means AC29's full
focused Level-1 acceptance gate has not been demonstrated for this iteration.

Recommendation: prune or repair those host registrations, or run the canonical
gate in a clean CI user environment, and record a complete passing result. Do
not restore the previous corruption-masking behavior to make this host pass.

Strongest verification present: **Level 1 focused regressions pass (six of six),
but the full Sniff Level-1 gate is incomplete**.

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
| 21 | Six vendor values and bounded ambiguous-host probe | L1 + Wiremock | Five valid signature fixtures plus fabricated Azure fixture; public SSH/SCP integration | **Fail: Azure contract mismatch** |
| 22 | Exact/paginated PR API, capabilities, no ignored filters | L1 + Wiremock | L1 + Wiremock, including public SSH/SCP integration | Pass |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock | Pass |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 + Wiremock, including version thresholds | Pass for valid discovery paths |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 + Wiremock, including public SSH/SCP integration | Pass |
| 26 | Three-surface policy/cache/error parity and no credential disclosure | L1 + Wiremock | L1 misses header isolation; implementation sends unrelated tokens | **Fail: credential disclosure** |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 + Wiremock; ambiguous auth attribution is wrong | **Fail at ambiguous discovery** |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | Focused macOS tests pass; full Sniff gate incomplete; prior Linux/Windows GNU evidence | **Gap** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies`. Sniff is directly changed; Darkmatter is the
provider-expression and error-projection consumer. GitNexus impact results for
the affected discovery symbols are recorded in the findings.

Fresh macOS gates:

- Focused Sniff nextest selection for ambiguous discovery, authentication,
  public SSH/SCP discovery, stale worktrees, and corrupt worktrees — pass: six
  tests.
- `cd sniff && just test` — incomplete: 1,307 passed, two failed due to the
  host's corrupt registered worktrees, three skipped, 319 not run.
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

**Not ready for production.** AC21 fails for Azure DevOps Server, AC26 has a
critical cross-provider credential-disclosure defect, AC27 can misattribute
authentication failures, and AC29's canonical full Sniff Level-1 gate is not
green for this iteration.
