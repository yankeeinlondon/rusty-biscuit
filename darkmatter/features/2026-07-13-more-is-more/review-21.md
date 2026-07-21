---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T20:25:02-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-21.md
previous: 2026-07-13-more-is-more/review-20.md
---

# Review 21

## Summary

The feature is **not production ready**. Review 20's authored-query, ordinary canonical URL, provider-link, and DMLS-navigation findings are substantially resolved, and fresh macOS build, Level-1 test, and lint gates pass. Production self-hosted discovery remains incomplete for ordinary SSH/SCP Git remotes and discards the detected Gitea/Forgejo version before capability selection. Canonical provider URL parsing also accepts percent-encoded URL delimiters that later alter the API request target. Finally, the spec's required passing Windows and Linux evidence still does not exist.

## Review-20 Disposition

- **Resolved:** authored request DTOs reject internal `descending`/`cursor` fields, validate flavor-specific filters, and preserve provider-default ordering.
- **Resolved with a new edge-case finding below:** flavor-selected web/API URL grammars now cover all supported providers, retain scheme/port, and reject tested malformed/cross-flavor shapes.
- **Resolved:** provider-supplied links are normalized and constrained to trusted HTTP(S) sites before Markdown destination escaping.
- **Resolved:** DMLS rewrites the vocabulary target to an absolute file URI and verifies resolution from an unrelated document.
- **Partially resolved:** neutral-host HTTP(S) production discovery works, but SSH/SCP remotes and version-aware capability decisions do not.
- **Still open:** passing Windows and Linux results required by AC16 and AC29.

## Findings

### High: Neutral-host SSH/SCP remotes cannot enter production provider discovery

The spec requires one allowlisted bounded probe for an ambiguous self-hosted URL ([spec.md:1184](spec.md#L1184)) and includes self-managed providers in the supported surface ([spec.md:915](spec.md#L915)). `FocusedProviderClient::discover` passes the configured Git fetch URL directly to `probe_self_hosted_flavor` ([focused.rs:64](../../../sniff/lib/src/remote/focused.rs#L64)). That probe parses only HTTP(S) URLs and returns `UnsupportedRemoteCapability("provider detection", "non-HTTP Git transport")` for every SSH/SCP form ([remote_observation.rs:261](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L261)). A common neutral-host remote such as `git@git.example:group/project.git` therefore cannot use `pr*` or `cicd*`, even though `ResolvedRemote.endpoint` already carries its host identity.

The new production tests use loopback HTTP clone URLs ([focused_provider.rs:1754](../../../sniff/lib/tests/focused_provider.rs#L1754), [provider_network.rs:415](../../lib/src/markdown/compose/tests/provider_network.rs#L415)). Existing SSH fixtures use `with_api_base`, bypassing production discovery ([focused_provider.rs:83](../../../sniff/lib/tests/focused_provider.rs#L83), [focused_provider.rs:104](../../../sniff/lib/tests/focused_provider.rs#L104)).

Recommendation: derive a policy-checked HTTPS discovery origin from `ResolvedRemote.endpoint.host` for SSH/SCP remotes instead of treating the Git transport as the provider transport. Do not reinterpret an SSH port as an HTTP port. Add production-path Level-1 tests for neutral-host SSH and SCP remotes against GitLab, Gitea, and Forgejo.

Strongest verification present: **Level 1 for neutral-host HTTP remotes only; the production SSH/SCP path has no passing test**.

### High: Gitea/Forgejo capability selection ignores the detected server version

The supported scope is expressly version-dependent for Gitea/Forgejo ([spec.md:915](spec.md#L915)); capability decisions must use detected family and version ([spec.md:1028](spec.md#L1028)), and unsupported-version errors must name the provider/flavor/version ([spec.md:1432](spec.md#L1432)). The discovery probe reads `/api/v1/version`, but returns only `ApiFlavor` ([remote_observation.rs:261](../../../sniff/lib/src/filesystem/git/remote_observation.rs#L261)). `ResolvedRemote` has no version or capability field ([remote_resolver.rs:73](../../../sniff/lib/src/filesystem/git/remote_resolver.rs#L73)), and `capabilities()` advertises PR and CI jobs for every Gitea and Forgejo server solely from the flavor enum ([focused.rs:146](../../../sniff/lib/src/remote/focused.rs#L146)).

On a server version without the required Actions/job endpoint, the client still issues the request and maps every 404 to `Ok(None)` ([focused.rs:456](../../../sniff/lib/src/remote/focused.rs#L456), [focused.rs:483](../../../sniff/lib/src/remote/focused.rs#L483)). That can turn an unsupported capability into a neutral exact result or empty list, contrary to the error contract. The discovery tests distinguish Gitea from Forgejo using version response text, but do not test capability thresholds ([focused_provider.rs:1785](../../../sniff/lib/tests/focused_provider.rs#L1785)).

Recommendation: return and retain a structured discovery result containing server family, parsed version, and derived capabilities. Reject unsupported operations and filters before network I/O. Test versions immediately below and above each supported endpoint threshold, including exact and list expressions, and assert an actionable unsupported-version error instead of `null`/`[]`.

Strongest verification present: **Level 1 for family detection, but none for version-dependent capabilities or error projection**.

### High: Encoded URL delimiters can retarget accepted canonical references

Canonical provider references must either resolve to the correct repository-scoped identity or fail as malformed ([spec.md:1798](spec.md#L1798)). For flat provider routes, `identity()` percent-decodes each path segment and rejects only empty strings or decoded `/` characters ([provider_url.rs:355](../../../sniff/lib/src/remote/provider_url.rs#L355)). It accepts decoded `?`, `#`, and `\\`. The resulting namespace/repository is later interpolated unescaped by `repo_path()` ([focused.rs:1060](../../../sniff/lib/src/remote/focused.rs#L1060)) and passed to `Url::join` ([focused.rs:456](../../../sniff/lib/src/remote/focused.rs#L456)).

For example, `https://api.github.com/repos/acme/project%3Ffoo/pulls/7` is accepted as repository `project?foo`; the subsequent relative path `repos/acme/project?foo/pulls/7` targets `https://api.github.com/repos/acme/project?foo/pulls/7`, where the intended endpoint suffix became query text. The malformed-reference table tests encoded `/`, but not other reserved delimiters ([focused_provider.rs:334](../../../sniff/lib/tests/focused_provider.rs#L334)).

Recommendation: validate decoded provider identities against each provider's permitted segment grammar and reject URL delimiters, backslashes, controls, and dot-segment ambiguity. Independently percent-encode every identity segment when constructing request paths. Add positive and negative tests for `%3F`, `%23`, `%5C`, encoded controls, and Unicode identities across exact and list paths.

Strongest verification present: **Level 1 for selected malformed routes; no verification for encoded reserved delimiters reaching request construction**.

### High: Required Windows and Linux passing evidence remains absent

AC16 requires macOS, Windows, and Linux compile checks ([spec.md:1669](spec.md#L1669)); AC29 requires the focused suites and three-OS compile evidence ([spec.md:1712](spec.md#L1712)). This review obtained fresh native macOS build, test, and lint passes. The implementation log records that Linux Docker execution was permission-blocked and produced no result, while Windows cross-compilation failed in host tooling/dependency build scripts and likewise produced no passing result ([log.md:830](log.md#L830), [log.md:855](log.md#L855)). CI configuration is present, but no run containing the reviewed HEAD is available as evidence.

Recommendation: run the existing three-OS CI matrices on the reviewed commit and retain green Sniff, Darkmatter/DMLS, and downstream Claudine results. A soft-failing Windows job is useful visibility, but it does not satisfy a required passing result.

Strongest verification present: **Level 1 on macOS; no passing Windows or Linux result**. This is a high-severity rigor gap.

**DECISION:** Use Docker to pass Linux results but defer Windows for now.

## Verification Matrix

All feature requirements are in-process filesystem/Git/parser/network/Markdown/LSP behavior. None depends on terminal-emulator rendering or its input encoder, so Level 2 and Level 3 are not applicable.

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
| 16 | Scoped suites and three-OS compile | L1 on 3 OSes | L1 macOS | **Gap: Windows/Linux results absent** |
| 17 | First/last index functions | L1 | L1 | Pass |
| 18 | Object/array literal grammar and evaluation | L1 | L1 | Pass |
| 19 | Shared preferred-remote selection | L1 | L1 | Pass |
| 20 | Live branch observation and non-mutation | L1 + Wiremock | L1 + Wiremock | Pass |
| 21 | Vendor values and bounded ambiguous-host probe | L1 + Wiremock | L1 + Wiremock over HTTP | **Fail: SSH/SCP ambiguous-host discovery is unsupported** |
| 22 | Exact/paginated PR API, capabilities, no ignored filters | L1 + Wiremock | L1 component | **Fail: SSH discovery, version capabilities, encoded identities** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 end-to-end over HTTP | **Fail: production SSH and malformed-reference paths** |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 component | **Fail: version-gated job capabilities and encoded identities** |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 end-to-end over HTTP | **Fail: production SSH and unsupported-version paths** |
| 26 | Three-surface policy/cache/error parity and no passive I/O | L1 + Wiremock | L1 end-to-end | Pass for supported endpoints |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 component/end-to-end | **Fail: unsupported versions can collapse to `null`/`[]`** |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | Pass |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | L1 macOS | **Gap: Windows/Linux results absent** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and `sniff repo package-dependencies`. The directly affected areas are Sniff and Darkmatter (including `darkmatter`, `darkmatter-cli`, and `dmls`), with Claudine identified as a downstream package-area consumer. GitNexus impact analysis reports **CRITICAL** risk for `ResolvedRemote` (58 impacted symbols, four direct dependents), **HIGH** risk for `parse_provider_url` (eight impacted symbols, two direct dependents), and **MEDIUM** risk for `trusted_web_link` (32 impacted symbols, four direct dependents). The branch-wide compare-to-main report is CRITICAL but includes a large unrelated stacked-branch history, so readiness was assessed against the feature's recorded package scope and current source.

Fresh macOS gates:

- `cd sniff && just build` — pass.
- `cd sniff && just test` — pass: 1,608 Sniff library tests and 769 Sniff CLI tests.
- `cd sniff && just lint` — pass.
- `cd darkmatter && just build` — pass for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `cd darkmatter && just test` — pass: 5,915 Darkmatter tests, 561 CLI tests, and 605 DMLS tests.
- `cd darkmatter && just lint` — pass for all three packages.

These are Level-1 results. No Level-2 or Level-3 run was needed for this feature's user-observable contracts.

## Production Readiness

**Not ready for production.** AC21–25 and AC27 remain incomplete at self-hosted transport/version and malformed-reference boundaries, and AC16/29 still lack the required passing Windows and Linux evidence.
