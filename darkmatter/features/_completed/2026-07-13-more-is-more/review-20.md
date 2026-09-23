---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T15:59:40-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-20.md
previous: 2026-07-13-more-is-more/review-19.md
next: 2026-07-13-more-is-more/review-21.md
---

# Review 20

## Summary

The feature is **not production ready**. The implementation since review 19 correctly closes the focused-provider error-parity finding: provider failures now retain a typed classification through memoization and are authoring-fatal in frontmatter, body interpolation, and `$()` evaluation. The shared run-local executor also remains a sound performance improvement over per-call runtimes.

The other review-19 findings were not implemented. Production provider construction still cannot represent ordinary self-hosted endpoints, authored query objects still expose internal fields and silently approximate advertised filters, canonical API URLs are not parsed according to the contract, and the required Windows/Linux result evidence is absent. Provider-supplied link destinations also remain unvalidated, and the DMLS vocabulary link is still relative to an undefined hover-document base.

## Review-19 Disposition

- **Resolved:** focused provider failures use `ExpressionError::Provider`, survive the run-local cache as typed errors, and abort all three expression surfaces ([error.rs:268](../../lib/src/markdown/compose/expression/error.rs#L268), [resolve_ctx.rs:14](../../lib/src/markdown/compose/expression/resolve_ctx.rs#L14), [provider_network.rs:689](../../lib/src/markdown/compose/tests/provider_network.rs#L689)).
- **Still open:** the six findings below. The implementation lines underlying those findings remain unchanged since review 19.

## Findings

### High: Production provider construction still excludes ordinary self-hosted servers

The supported scope includes self-managed GitLab and Gitea/Forgejo installations selected from detected server family/version ([spec.md:914](spec.md#L914)). `ResolvedRemote` still derives `ApiFlavor` from hostname patterns and stores only `host_str()`, discarding the configured scheme and port ([remote_resolver.rs:126](../../../sniff/lib/src/filesystem/git/remote_resolver.rs#L126), [remote_resolver.rs:139](../../../sniff/lib/src/filesystem/git/remote_resolver.rs#L139)). A neutral enterprise host therefore resolves as `SelfHosted`, while only hostnames containing recognizable vendor tokens become GitLab/Gitea/Forgejo ([types.rs:419](../../../sniff/lib/src/filesystem/git/types.rs#L419)). Client construction then synthesizes an HTTPS API base from that incomplete host and rejects unsupported/unknown flavors ([focused.rs:892](../../../sniff/lib/src/remote/focused.rs#L892)). The bounded discovery used by `remote_vendor()` is not fed back into the focused client.

The end-to-end Darkmatter fixture explicitly bypasses both failures with a test-only flavor/API-base override and documents why the production constructor cannot address its loopback endpoint ([provider.rs:63](../../lib/src/markdown/compose/expression/functions/provider.rs#L63)). Those tests verify composition and projection, not the production resolution path.

Recommendation: make `ResolvedRemote` retain a normalized endpoint origin (scheme, host, and port), detected API flavor/version, and derived API base. Reuse the allowlisted discovery result when constructing focused clients. Add Level-1 production-path fixtures for neutral-host self-managed GitLab, Gitea, and Forgejo endpoints, including a non-default port.

Strongest verification present: **Level 1 with a test-only constructor override; insufficient for the production constructor**.

### High: The authored query boundary still accepts internal fields and approximates canonical filters

The v1 authoring contract exposes only its documented provider-neutral keys and forbids ignored fields or approximations ([spec.md:1268](spec.md#L1268), [spec.md:1805](spec.md#L1805)). Darkmatter still removes `direction`, injects `descending`, and deserializes the remaining authored object directly into Sniff's transport/query structs ([pull_requests.rs:56](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L56), [cicd.rs:88](../../lib/src/markdown/compose/expression/functions/cicd.rs#L88)). Those structs publicly deserialize `descending` and `cursor`, so authors can supply both undocumented keys; `cursor` is not consumed by the focused query implementation ([types.rs:317](../../../sniff/lib/src/remote/types.rs#L317), [types.rs:572](../../../sniff/lib/src/remote/types.rs#L572)).

The semantic gaps also remain:

- capabilities advertise `stage` for every CI flavor, while job-query validation is flavor-independent; providers without stage data return no matches instead of an unsupported-filter error ([focused.rs:124](../../../sniff/lib/src/remote/focused.rs#L124), [focused.rs:922](../../../sniff/lib/src/remote/focused.rs#L922));
- `workflow` matches only parent name or native ID, not the promised definition path ([focused.rs:1042](../../../sniff/lib/src/remote/focused.rs#L1042)); and
- `sort: "provider-default"` skips key sorting but is still reversed by the default `descending: true`, so it does not preserve provider order ([focused.rs:1026](../../../sniff/lib/src/remote/focused.rs#L1026)).

Recommendation: introduce authored request DTOs containing exactly the catalog vocabulary, then translate to separate internal paging/query types. Validate each canonical field against the selected API flavor before I/O and retain every field needed for exact matching. Add negative authored-expression tests for `descending`/`cursor`, flavor-specific `stage`, workflow path/definition ID, and both directions of `provider-default`.

Strongest verification present: **Level 1 component tests for genuinely unknown keys and selected filters; no authored-boundary verification for the accepted internal keys or the approximated cases**.

### High: Canonical provider API URLs are still rejected or misclassified

The contract defines canonical provider URLs as web **or API** URLs ([spec.md:911](spec.md#L911), [spec.md:1797](spec.md#L1797)). The parser remains keyed only to web-route markers such as `/pull/`, `/-/merge_requests/`, `/pull-requests/`, and `/pulls/` ([focused.rs:753](../../../sniff/lib/src/remote/focused.rs#L753), [focused.rs:809](../../../sniff/lib/src/remote/focused.rs#L809)). As a result:

- GitHub `/repos/acme/project/pulls/7` is treated as a Gitea-style `/pulls/` route and receives the wrong namespace/flavor;
- GitLab `/api/v4/projects/group%2Fproject/merge_requests/8` is rejected and its encoded project identity is never decoded; and
- Bitbucket `/2.0/repositories/acme/project/pullrequests/10` is rejected.

The focused URL test covers only canonical web URLs ([focused_provider.rs:114](../../../sniff/lib/tests/focused_provider.rs#L114)).

Recommendation: implement flavor-specific web/API parsers that retain scheme/port, decode provider-specific repository identity exactly once, and reject cross-flavor route shapes. Add positive and malformed API-URL fixtures for every supported provider and for both PR and job references.

Strongest verification present: **Level 1 for web URLs only; the API-URL half of the input contract has no verification**.

### High: Required Windows and Linux result evidence is still absent

AC16 and AC29 require macOS, Windows, and Linux compile checks to pass ([spec.md:1667](spec.md#L1667), [spec.md:1711](spec.md#L1711)). The repository now has a real three-OS Sniff matrix with `--all-targets --features remote` ([test.yml:56](../../../.github/workflows/test.yml#L56), [test.yml:102](../../../.github/workflows/test.yml#L102)) and downstream Claudine compile configuration, although its Windows leg remains advisory ([claudine-tests.yml:128](../../../.github/workflows/claudine-tests.yml#L128)). Configuration is not a passing result. The implementation log explicitly records that the Windows/Linux evidence does not exist and no run was triggered ([log.md:546](log.md#L546), [log.md:579](log.md#L579)); the current commits are not contained by any local remote-tracking branch.

Recommendation: capture green Sniff, Darkmatter/DMLS, and downstream Claudine results on all three OSes. Resolve any Windows failure rather than treating an advisory job as production evidence.

Strongest verification present: **Level 1 on macOS; Windows/Linux are configured but unverified**. This is a high-severity rigor gap.

### High: Provider-supplied link destinations bypass validation and escaping

The compact projection promises a canonical provider web link ([spec.md:1452](spec.md#L1452)). Provider response fields are copied directly into `web_url` without scheme/origin validation ([focused.rs:509](../../../sniff/lib/src/remote/focused.rs#L509), [focused.rs:555](../../../sniff/lib/src/remote/focused.rs#L555)), and both Darkmatter formatters interpolate that value directly into a Markdown destination ([pull_requests.rs:75](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L75), [cicd.rs:107](../../lib/src/markdown/compose/expression/functions/cicd.rs#L107)). A malformed destination containing `)` breaks deterministic Markdown; a cross-host or non-HTTP scheme can create a misleading or unsafe link in downstream renderers.

The hostile-provider tests exercise link-label/title text, not malformed, cross-host, or non-HTTP destinations. This user-visible projection requirement therefore has no Level-1 proof at its actual trust boundary.

Recommendation: parse and normalize response URLs in Sniff, require HTTP(S), enforce the resolved provider/repository origin policy, and serialize destinations safely. Add Level-1 fixtures for delimiter-bearing, cross-host, and non-HTTP URLs through exact and list formatting paths.

Strongest verification present: **none for hostile link destinations; Level 1 is required**.

### Medium: The DMLS vocabulary link still has no resolvable editor base

The catalog authors `darkmatter-expressions.md#provider-query-vocabulary` ([expression-functions.yaml:1548](../../docs/schemas/expression-functions.yaml#L1548)), and DMLS copies the description unchanged into completion/hover Markdown ([expressions.rs:408](../../dmls/src/overlay/expressions.rs#L408)). The active Markdown document may be anywhere in the workspace, so this sibling-relative target does not identify `darkmatter/docs/topics/darkmatter-expressions.md`. The test proves only that the raw substring reaches both response surfaces ([expressions.rs:711](../../dmls/src/overlay/expressions.rs#L711)); it does not prove navigation resolves.

Recommendation: rewrite the authored documentation target to an absolute workspace/file URI at the LSP response boundary, or embed the compact vocabulary in hover content. Add an LSP-response test that resolves the emitted target and anchor from a document outside `darkmatter/docs/topics/`.

Strongest verification present: **Level 1 substring propagation, not resolvable navigation**.

## Verification Matrix

All feature requirements are in-process filesystem/Git/parser/network/Markdown/LSP behavior. None depends on a real terminal's glyph widths, SGR interpretation, scrolling, terminal input encoder, paste/IME/mouse path, or OS keyboard events; Level 2 and Level 3 are not applicable.

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
| 21 | Vendor values and bounded ambiguous-host probe | L1 + Wiremock | L1 + Wiremock | Pass for `remote_vendor`; detected flavor is not reused by provider queries |
| 22 | Exact/paginated PR API, capabilities, no ignored filter | L1 + Wiremock | L1 component | **Fail: self-hosted/API-URL/internal-key/filter gaps** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 end-to-end | **Fail: production resolution, API URL, and destination trust boundary** |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 component | **Fail: self-hosted/API-URL/filter capability gaps** |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 end-to-end | **Fail: same production/query/destination gaps** |
| 26 | Three-surface policy/cache/error parity and no passive I/O | L1 + Wiremock | L1 end-to-end | Pass |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 component/end-to-end | Pass for focused failures; malformed destination validation remains absent |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | **Gap: emitted vocabulary link is not reliably navigable** |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | L1 macOS | **Gap: Windows/Linux results absent** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages`, `sniff repo package-areas`, and `sniff repo package-dependencies`. The directly affected package areas are Sniff and Darkmatter (including `darkmatter`, `darkmatter-cli`, and `dmls`), with Claudine as a downstream consumer. GitNexus reports **CRITICAL** upstream risk for `ResolvedRemote` (41 impacted symbols, four direct dependents, five modules) and **HIGH** risk for `PullRequestQuery` (16 impacted symbols, ten direct dependents, three modules).

A focused Darkmatter nextest run for the new provider-failure classification and all-surface tests was stopped with exit 130 after exceeding the non-interactive command ceiling during a cold dependency build. It emitted no compiler or test diagnostic and is not counted as a pass. Review-19's implementation log records green full macOS Sniff and Darkmatter area tests/lints before the new parity commit; current checked-in Level-1 tests cover every focused failure class on all three expression surfaces, but this review does not present the interrupted run as fresh execution evidence.

The readiness decision does not depend on that interruption: the high findings above are direct current-source/spec mismatches, and the Windows/Linux evidence gap is explicitly recorded by the implementation itself.

`md get` and `bf` parsed every requested lifecycle property with the exact value shown in frontmatter, and `git diff --check` passed. `md schema validate` cannot validate the review because the shared `schemas/feature-review.yaml` is invalid for the current standalone-schema loader: tagged schema documents permit only `kind` and `types`, while that file also declares `description` and `$schema`. This pre-existing schema-authoring defect is outside the reviewed feature.

## Production Readiness

**Not ready for production.** The provider-error parity fix is correct and materially improves the implementation, but AC22–25 still fail at production remote resolution, query, API-reference, and URL-trust boundaries, while AC16/29 still lack the required Windows/Linux results.
