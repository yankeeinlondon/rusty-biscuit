---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T09:10:43-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-23.md
previous: 2026-07-13-more-is-more/review-22.md
next: 2026-07-13-more-is-more/review-24.md
---

# Review 23

## Summary

The feature is **not production ready**. Review 22's SSH/SCP normalization and
red macOS Sniff suite are resolved, and fresh Sniff and Darkmatter Level-1 and
lint gates pass. The provider-discovery authority is still materially narrower
than the specification, however: it probes only anonymous GitLab and
Gitea/Forgejo version endpoints, omitting three required enterprise/server
families and authenticated probing. Review 22's successful public SSH/SCP probe
test was also not added. Finally, the stale-worktree repair introduced a
high-impact regression by silently treating an existing but corrupt registered
checkout as stale.

## Review-22 Disposition

- **Resolved:** `remote_vendor_at` and `FocusedProviderClient::discover` now use
  one SSH/SCP-to-HTTPS discovery-origin helper, and SSH ports are not reused as
  HTTP ports.
- **Resolved:** the canonical Sniff Level-1 suite is green on macOS with the
  host's stale linked-worktree registrations still present.
- **Not resolved:** successful ambiguous-host SSH/SCP discovery remains tested
  as separate private/HTTP pieces plus public policy denial, not as one public
  successful operation.
- **Accepted evidence under the prior non-blocking ruling:** Windows GNU target
  compilation passes for Sniff, Sniff CLI, Darkmatter, Darkmatter CLI, and DMLS;
  native Windows/MSVC CI evidence remains absent.

## Findings

### High: Ambiguous-provider discovery omits required server flavors and credentials

The specification requires ambiguous-host discovery to distinguish GitHub
Enterprise, GitLab self-managed, Gitea, Forgejo, Bitbucket Data Center, and
Azure DevOps Server, and its test strategy explicitly requires every server
flavor plus authenticated probes ([spec.md:1184](spec.md#L1184),
[spec.md:1595](spec.md#L1595)). The production probe contains only two routes:
GitLab `/api/v4/version` and Gitea `/api/v1/version`
([remote_observation.rs:294](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L294)).
No GitHub Enterprise, Bitbucket Data Center, or Azure DevOps Server signature is
attempted, so a neutral custom hostname for any of those required flavors ends
as `UnsupportedProvider`.

The same path constructs a bare `PolicyClient` and sends no provider credential.
An authorized private GitLab whose version endpoint requires authentication
therefore returns an authentication error even when the appropriate credential
is configured. This is especially visible now that the probe is also the
version authority for focused self-hosted clients. GitLab's official Metadata
API example sends `PRIVATE-TOKEN` to `GET /api/v4/version`:
<https://docs.gitlab.com/api/metadata/>.

Recommendation: make the bounded probe a provider-signature table covering all
required server flavors, with conflict detection rather than first-success
guessing. After exact-host policy approval, resolve candidate credentials
through Sniff's existing provider credential authority and attach them only to
the matching candidate request. Add Wiremock Level-1 cases for all six server
flavors, conflicting/unidentified signatures, anonymous success, required
authentication, valid/invalid credentials, and proof that secrets never enter
errors or logs.

Strongest verification present: **Level 1 with Wiremock for anonymous
GitLab/Gitea/Forgejo only; no behavior or test for three required server
flavors, and no authenticated-probe test**.

### High: Existing corrupt linked worktrees are silently discarded as stale

The new shared helper maps every `gix::open::Error::NotARepository` to `Ok(None)`
([open.rs:73](../../../sniff/lib/src/filesystem/git/open.rs#L73)). That error does
not prove the registered target was deleted: an existing checkout whose `.git`
file is missing or corrupt produces the same class. The implementation removed
the previous `worktree_trusted_open_failure_is_propagated` test, which deleted
only the linked checkout's `.git` file and required an error. Both full Git
detection and public worktree listing now silently omit that corruption.

This contradicts the helper's own statement that corruption remains an error
and widens the Review-22 remedy beyond its intended case of an absent stale
target. GitNexus reports **HIGH** upstream impact for
`trusted_open_registered_worktree` (34 affected symbols, two direct callers,
one CLI execution flow) and `get_worktrees` (20 affected symbols, six direct
callers).

Recommendation: classify staleness from the registered checkout target's
absence before opening it. If the target exists, propagate `NotARepository` as
corruption. Restore the deleted existing-directory/missing-`.git` regression and
add it for both `GitRepo::worktrees`/full detection and `list_worktrees`, while
retaining the new absent-target fixtures.

Strongest verification present: **Level 1 for an absent checkout and malformed
registry metadata; no Level-1 preservation test for an existing corrupt
checkout**.

### High: Successful public ambiguous SSH/SCP discovery is still unverified

Review 22 required a successful public operation starting from a real configured
SSH/SCP remote. The new public tests stop at policy denial for neutral hosts
([focused_provider.rs:2174](../../../sniff/lib/tests/focused_provider.rs#L2174),
[remote_observation.rs:182](../../../sniff/lib/tests/remote_observation.rs#L182)).
The only public success test uses `gitlab.com`, which is classified locally and
does not execute the ambiguous-host probe. Gitea/Forgejo success is still proven
by injecting a manufactured private discovery result into
`from_discovered_flavor`, while the Wiremock probe begins from an HTTP remote.

Consequently no test proves that `remote_vendor_at` or
`FocusedProviderClient::discover` takes a real neutral-host SSH/SCP
`ResolvedRemote`, synthesizes the correct HTTPS origin, completes a provider
probe, retains the version, derives capabilities, and constructs the final API
base as one public operation. This is the exact integration seam that failed in
Reviews 21 and 22.

Recommendation: add a hermetic TLS-capable provider fixture or a narrowly
injectable test resolver below the production discovery authority, then call
the public APIs from disposable repositories configured with SSH URL and SCP
remotes. Cover GitLab, Gitea, and Forgejo successful discovery, SSH-port
omission, host policy, version retention, capability derivation, and final API
base selection. Do not weaken production HTTPS behavior to make the fixture
easier.

Strongest verification present: **Level 1 for public policy denial and
deterministic-host success, plus private split tests; no Level-1 successful
public ambiguous SSH/SCP operation**.

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
| 21 | Vendor values and bounded ambiguous-host probe | L1 + Wiremock | Anonymous GitLab/Gitea/Forgejo only; split SSH/SCP proof | **Fail** |
| 22 | Exact/paginated PR API, capabilities, no ignored filters | L1 + Wiremock | L1 + Wiremock over HTTP; split SSH/SCP proof | **Gap** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock | Pass for exercised discovery paths |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 + Wiremock, including version thresholds | Pass for implemented flavors |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 + Wiremock over HTTP; split SSH/SCP proof | **Gap** |
| 26 | Three-surface policy/cache/error parity and no passive I/O | L1 + Wiremock | L1 + Wiremock; no authenticated probe | **Fail at provider discovery** |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 + Wiremock | Pass for exercised paths |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | macOS/Linux pass; Windows GNU target pass, native MSVC absent | Pass under prior non-blocking ruling |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies`. Sniff is directly changed; Darkmatter is the
provider-expression and error-projection consumer. GitNexus reports low risk for
`remote_vendor_at` and `FocusedProviderClient::discover`, but high risk for the
new worktree-open helper and `get_worktrees` as described above.

Fresh macOS gates:

- `cd sniff && just test` — pass: 1,621 Sniff tests and 769 Sniff CLI tests.
- `cd sniff && just lint` — pass.
- `cd darkmatter && just test` — pass: 5,937 Darkmatter tests, 561 CLI tests,
  and 633 DMLS tests.
- `cd darkmatter && just lint` — pass for all three packages.
- `git diff --check` — run after review/frontmatter edits.

Recorded cross-platform evidence:

- Linux AArch64 — Sniff and Darkmatter build, full L1, and lint pass.
- Windows GNU target — Sniff with/without `remote`, Sniff CLI, Darkmatter,
  Darkmatter CLI, and DMLS compile checks pass.
- Native Windows/MSVC — no passing result; treated as non-blocking by the
  explicit prior review decision.

## Production Readiness

**Not ready for production.** AC21 and AC26 are functionally incomplete for
required enterprise/server and authenticated ambiguous-host discovery; AC22 and
AC25 still lack the successful public SSH/SCP Level-1 proof required by Review
22; and the stale-worktree fix introduces a high-impact corruption-masking
regression in existing Sniff behavior.
