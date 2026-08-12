---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T01:49:58-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-22.md
previous: 2026-07-13-more-is-more/review-21.md
next: 2026-07-13-more-is-more/review-23.md
---

# Review 22

## Summary

The feature is **not production ready**. Review 21's version-aware Gitea/Forgejo capability work, encoded-identity hardening, and Linux verification are resolved. SSH/SCP transport normalization, however, was added only to the focused provider client: `remote_vendor()` still sends the raw non-HTTP Git URL to the probe and fails before discovery. The successful public SSH/SCP discovery path is also covered only as two private halves rather than one end-to-end Level-1 test. In addition, the required full Sniff Level-1 suite is reproducibly red on macOS, and the required Windows result remains absent.

## Review-21 Disposition

- **Resolved:** self-hosted discovery retains the reported Gitea/Forgejo version, derives operation-specific capabilities, and rejects unsupported exact/list CI job operations before provider I/O.
- **Resolved:** canonical provider references reject decoded structural delimiters, controls, backslashes, and dot segments; request construction independently encodes identity segments.
- **Resolved for Linux:** native Linux AArch64 build, full Level-1 test, and lint gates passed for Sniff and Darkmatter.
- **Partially resolved:** `FocusedProviderClient::discover` normalizes neutral-host SSH/SCP remotes to a host-only HTTPS origin, but `remote_vendor_at` does not, and successful public discovery is not exercised end to end.
- **Still open:** no passing Windows compile result exists.

## Findings

### High: `remote_vendor()` still rejects ambiguous SSH/SCP remotes before discovery

AC21 requires ambiguous self-hosted remotes to undergo the allowlisted bounded provider probe ([spec.md:1690](spec.md#L1690)). `FocusedProviderClient::discover` now calls `discovery_remote` before probing, correctly turning SSH/SCP remotes into `https://{host}/` without carrying the SSH port ([focused.rs:84](../../../sniff/lib/src/remote/focused.rs#L84), [focused.rs:723](../../../sniff/lib/src/remote/focused.rs#L723)). `remote_vendor_at`, however, still passes `resolved.fetch_url` directly to `probe_self_hosted_provider` ([remote_observation.rs:78](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L78)). The probe accepts only HTTP(S), so `remote_vendor()` on `ssh://git@git.example/...` or `git@git.example:group/project.git` returns `UnsupportedRemoteCapability` instead of `gitlab`, `gitea`, or `forgejo`.

The new tests do not expose this split. One unit test checks only the private URL transformation and another injects a manufactured discovery result directly into the private client constructor ([focused.rs:1385](../../../sniff/lib/src/remote/focused.rs#L1385), [focused.rs:1398](../../../sniff/lib/src/remote/focused.rs#L1398)). The public integration test reaches only deny-by-default policy failure; it never performs a successful probe or client construction through `FocusedProviderClient::discover` ([focused_provider.rs:2174](../../../sniff/lib/tests/focused_provider.rs#L2174)). Thus the production success path is not verified as one operation, and `remote_vendor_at` has no SSH/SCP test at all.

Recommendation: move Git-transport-to-provider-origin selection into one shared Sniff helper used by both `remote_vendor_at` and `FocusedProviderClient::discover`. Add Wiremock-backed Level-1 tests that invoke the public functions from real configured SSH and SCP remotes and prove successful GitLab, Gitea, and Forgejo discovery, host policy, omission of the SSH port, version retention, and final API-base selection.

Strongest verification present: **Level 1 for the two private halves and for public policy denial; no Level-1 success test for the public SSH/SCP path, and no coverage for `remote_vendor()` over SSH/SCP**.

### High: The required full Sniff Level-1 gate is reproducibly red on macOS

AC16 and AC29 require the relevant full Level-1 suites to pass ([spec.md:1666](spec.md#L1666), [spec.md:1709](spec.md#L1709)). A fresh `cd sniff && just test` run failed after all retries in `test_detect_with_base_dir` and `test_skip_os_with_filesystem_only`; 1,330 tests passed, two failed, three were skipped, and 285 were canceled. A focused rerun of only those two tests also failed every retry.

Both tests ask detection to start from the active process directory ([lib.rs:427](../../../sniff/lib/src/lib.rs#L427), [lib.rs:457](../../../sniff/lib/src/lib.rs#L457)), but Git discovery attempts to open stale registered worktrees under `/private/tmp/dmbench/{base,before,after}` and returns `NotARepository(MissingHead)`. The implementation log encountered the same failure and explicitly declined to count that run as evidence ([log.md:1010](log.md#L1010)). Whatever its origin, a reproducible failure in the package's canonical full L1 recipe cannot satisfy the acceptance gate.

Recommendation: make repository discovery remain anchored to the requested path and tolerate unrelated missing linked-worktree registrations. Add a disposable-repository regression fixture containing stale linked-worktree metadata, then require `just test` to pass without relying on retries or an externally cleaned Git worktree list.

Strongest verification present: **focused Level 1 passes, but the canonical full Level-1 suite fails on macOS**.

### High: Required Windows compile evidence remains absent

DECISION: this is considered non-blocking for "production ready" qualification

AC16 and AC29 explicitly require passing compile checks on macOS, Windows, and Linux ([spec.md:1669](spec.md#L1669), [spec.md:1712](spec.md#L1712)). Review 21's implementation produced native Linux AArch64 build/test/lint evidence and this review produced fresh macOS build/lint evidence, but Windows was deliberately deferred and not attempted ([log.md:1015](log.md#L1015)). CI configuration or a deferred platform decision is not a passing result.

Recommendation: run the existing Windows CI matrix against the exact reviewed source and retain a green result for Sniff with `remote`, Darkmatter, DMLS, and any downstream package selected by the public API impact scope.

Strongest verification present: **Level 1/build evidence on Linux and macOS; no passing Windows compile result**.

## Verification Matrix

All feature behavior is process-local filesystem/Git/parser/network/Markdown/LSP behavior. None depends on terminal rendering or an OS input encoder, so Level 2 and Level 3 are not applicable.

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
| 16 | Scoped suites and three-OS compile | L1 on 3 OSes | Linux pass; macOS build/lint pass but L1 fails; no Windows result | **Fail** |
| 17 | First/last index functions | L1 | L1 | Pass |
| 18 | Object/array literal grammar and evaluation | L1 | L1 | Pass |
| 19 | Shared preferred-remote selection | L1 | L1 | Pass |
| 20 | Live branch observation and non-mutation | L1 + Wiremock | L1 + Wiremock | Pass |
| 21 | Vendor values and bounded ambiguous-host probe | L1 + Wiremock | L1 + Wiremock over HTTP; no SSH/SCP path | **Fail: `remote_vendor()` rejects ambiguous SSH/SCP remotes** |
| 22 | Exact/paginated PR API, capabilities, no ignored filters | L1 + Wiremock | L1 + Wiremock over HTTP; split-only SSH/SCP proof | **Gap: public SSH/SCP success path is not exercised end to end** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 + Wiremock over HTTP | Pass for exercised transports |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 + Wiremock, including version thresholds | Pass |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 + Wiremock over HTTP; split-only SSH/SCP proof | **Gap: public SSH/SCP success path is not exercised end to end** |
| 26 | Three-surface policy/cache/error parity and no passive I/O | L1 + Wiremock | L1 + Wiremock | Pass |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 + Wiremock | Pass |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | Linux pass; Darkmatter macOS pass; Sniff macOS fail; no Windows result | **Fail** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and `sniff repo package-dependencies`. Sniff is directly changed; Darkmatter is the expression/error-projection consumer. GitNexus reports low risk for `probe_self_hosted_provider` (three impacted symbols, two direct callers), low risk for `validated_repository_identity` (seven impacted symbols), and low aggregate risk with no affected execution flows for the current worktree changes.

Fresh macOS gates:

- `cd sniff && just build` — pass.
- `cd sniff && just test` — **fail**: 1,330 passed, two failed after retries, three skipped, 285 canceled.
- focused rerun of the two failed Sniff tests — **fail**: both failed all four attempts.
- `cd sniff && just lint` — pass.
- `cd darkmatter && just build` — pass for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `cd darkmatter && just test` — pass: 5,937 Darkmatter tests, 561 CLI tests, and 633 DMLS tests.
- `cd darkmatter && just lint` — pass for all three packages.
- `git diff --check` — pass after review-frontmatter edits.
- `md schema validate` — unavailable because the shared `schemas/feature-review.yaml` is itself rejected: its root mixes the tagged-schema form with keys unsupported by the baseline-schema parser. This is a pre-existing schema-authoring defect, not a validation failure in this review's frontmatter.

Recorded cross-platform evidence from the implementation log:

- Linux AArch64 — Sniff and Darkmatter build, full L1, and lint pass.
- Windows — no passing result.

## Production Readiness

**Not ready for production.** AC21 is functionally incomplete for ambiguous SSH/SCP remotes, AC22/25 lack the appropriate end-to-end Level-1 proof for successful SSH/SCP provider discovery, and AC16/29 fail because the canonical Sniff macOS suite is red and Windows evidence is absent.
