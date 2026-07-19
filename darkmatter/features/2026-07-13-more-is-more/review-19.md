---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T13:08:17-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: false
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-19.md
previous: 2026-07-13-more-is-more/review-18.md
---

# Review 19

## Summary

The feature is **not production ready**. The review-18 implementation materially improved the provider adapters: canonical defaults and datetime validation, provider-state projection, bounded-domain failure, provider-specific CI/CD projection, composed Wiremock fixtures, shared provider execution, CommonMark escaping, catalog vocabulary documentation, and cross-platform CI configuration are now present. The remaining gaps are narrower but affect ratified behavior: provider failures still differ by compose surface, ordinary self-hosted installations cannot reach the production provider client, internal query fields remain authorable while some advertised filters are silently approximated, and canonical API URLs are not parsed as promised. The required Windows/Linux result evidence is also still absent.

## Findings

### High: Focused provider failures still have different frontmatter and body semantics

AC26 requires frontmatter/body availability parity and AC27 requires focused provider errors to remain actionable rather than becoming neutral output ([spec.md:1701](spec.md#L1701)). The new end-to-end test documents the opposite behavior: a failing `pr(123)` aborts composition in frontmatter, while the identical body call succeeds, leaves the unevaluated `{{ pr(123) }}` text in place, and records only a warning ([provider_network.rs:601](../../lib/src/markdown/compose/tests/provider_network.rs#L601)). The test explicitly calls this “a real asymmetry against AC26's frontmatter/body parity claim” ([provider_network.rs:607](../../lib/src/markdown/compose/tests/provider_network.rs#L607)). Pinning known behavior is useful, but it does not satisfy the specification.

Recommendation: preserve a dedicated focused-provider error classification through expression resolution and apply one authoring-fatality rule on frontmatter, body, and `$()` surfaces. Replace the current asymmetry test with parity cases for not-found, denied host, authentication, rate limit, unsupported capability, incomplete domain, and transport failure.

Strongest verification present: **Level 1 end-to-end, proving the specified behavior is not implemented**. Level 2 and Level 3 are not applicable.

### High: Production provider construction cannot support the promised self-hosted servers

The provider scope includes self-managed GitLab and Gitea/Forgejo selected from the detected server family/version ([spec.md:914](spec.md#L914)). `ResolvedRemote` instead derives `ApiFlavor` solely from hostname patterns and stores only `host_str()`, losing the configured scheme and port ([remote_resolver.rs:126](../../../sniff/lib/src/filesystem/git/remote_resolver.rs#L126), [remote_resolver.rs:139](../../../sniff/lib/src/filesystem/git/remote_resolver.rs#L139)). An ordinary host such as `git.company.com` therefore becomes `SelfHosted`/`Unknown`, while only names containing `gitea` or `forgejo` are classified as those flavors ([types.rs:419](../../../sniff/lib/src/filesystem/git/types.rs#L419)). Client construction then hard-codes HTTPS and standard API paths from that incomplete host value and rejects `Unknown` ([focused.rs:892](../../../sniff/lib/src/remote/focused.rs#L892)). The bounded vendor probe used by `remote_vendor()` does not update the resolved remote consumed by `pr*` or `cicd*`.

Darkmatter's test-only transport confirms the production limitation: its documentation says a real constructor cannot express a loopback scheme/port or select a flavor for a neutral hostname, so tests overwrite both API base and flavor ([provider.rs:63](../../lib/src/markdown/compose/expression/functions/provider.rs#L63)). Those tests exercise HTTP projection but bypass the broken production resolution path.

Recommendation: make the resolved remote carry the normalized endpoint origin (including scheme and non-default port), detected API flavor/version, and derived API base. Reuse the allowlisted server-discovery result for provider client construction. Add Level-1 tests that pass through production-equivalent classification and endpoint derivation for self-managed GitLab, Gitea, and Forgejo, including a non-default port.

Strongest verification present: **Level 1 component and end-to-end tests with a test-only classification/API-base override; insufficient for the production constructor**.

### High: The authored query boundary accepts internal fields and silently approximates advertised filters

The specification exposes only the canonical keys and forbids provider-native escape hatches, ignored fields, and approximations ([spec.md:1268](spec.md#L1268), [spec.md:1805](spec.md#L1805)). Darkmatter removes `direction`, inserts the internal `descending` field, and then deserializes the caller's remaining object directly into Sniff's transport/query type ([pull_requests.rs:56](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L56), [cicd.rs:88](../../lib/src/markdown/compose/expression/functions/cicd.rs#L88)). Because those public Serde structs contain `descending` and `cursor`, authors can supply either key even though neither belongs to the v1 vocabulary; `cursor` is subsequently ignored ([types.rs:317](../../../sniff/lib/src/remote/types.rs#L317), [types.rs:575](../../../sniff/lib/src/remote/types.rs#L575)). `deny_unknown_fields` does not help when internal fields are part of the deserialized type.

There are also semantic approximations behind accepted canonical fields:

- `capabilities()` advertises `stage` for every supported CI flavor ([focused.rs:94](../../../sniff/lib/src/remote/focused.rs#L94)), but capability validation is not flavor-aware ([focused.rs:922](../../../sniff/lib/src/remote/focused.rs#L922)). Providers without a stage value therefore return `[]` rather than an explicit unsupported-filter error.
- `workflow` is matched only against parent name or native ID ([focused.rs:1042](../../../sniff/lib/src/remote/focused.rs#L1042)), although the contract accepts a workflow definition ID, name, or path. The projection does not retain a definition path.
- `sort: "provider-default"` deliberately skips sorting, but the default `descending: true` still reverses the provider's order ([focused.rs:1026](../../../sniff/lib/src/remote/focused.rs#L1026)). That is neither the provider default nor the documented behavior.

Recommendation: introduce authored request DTOs that contain exactly the catalog vocabulary, then translate them into separate internal paging/query types. Validate every canonical filter against the selected flavor before I/O and retain every field needed for exact local matching. Add negative tests for authored `descending`/`cursor`, flavor-specific `stage`, workflow path/definition ID, and both directions of `provider-default`.

Strongest verification present: **Level 1 unit/component tests, but the invalid-key cases use only a genuinely unknown key and omit the accepted internal keys and flavor-specific mismatches**.

### High: Canonical API URLs are not recognized as exact references

The specification defines a provider URL as a canonical web **or API** URL and requires `pr()`/`cicd()` to accept those exact forms ([spec.md:911](spec.md#L911), [spec.md:1797](spec.md#L1797)). The parser is shape-based only for web routes: GitHub `/pull/`, GitLab `/-/merge_requests/`, Bitbucket `/pull-requests/`, Gitea/Forgejo `/pulls/`, and corresponding web job routes ([focused.rs:753](../../../sniff/lib/src/remote/focused.rs#L753), [focused.rs:809](../../../sniff/lib/src/remote/focused.rs#L809)). Consequently canonical API examples such as GitHub `/repos/acme/project/pulls/7`, GitLab `/api/v4/projects/group%2Fproject/merge_requests/8`, or Bitbucket `/2.0/repositories/acme/project/pullrequests/10` are rejected or decoded into the wrong namespace/repository. Percent-encoded GitLab project paths are not decoded.

The focused URL test covers only web URLs ([focused_provider.rs:114](../../../sniff/lib/tests/focused_provider.rs#L114)), so its Level-1 pass does not verify the promised API half of the input domain.

Recommendation: use host/flavor-specific parsers for both canonical web and API route families, percent-decode repository identity where required, retain scheme/port, and add positive and malformed API-URL cases for every supported provider and both PR/job references.

Strongest verification present: **Level 1 for web URLs only; the API-URL requirement has no verification**.

### High: Required Windows and Linux result evidence still does not exist

AC16 and AC29 require macOS, Windows, and Linux compile checks to pass ([spec.md:1667](spec.md#L1667), [spec.md:1711](spec.md#L1711)). Review 18 added a real three-OS Sniff matrix, including `--all-targets --features remote` and provider-test execution ([test.yml:56](../../../.github/workflows/test.yml#L56)). It also added downstream Claudine macOS/Windows checks, although the Windows leg is advisory via `continue-on-error` ([claudine-tests.yml:128](../../../.github/workflows/claudine-tests.yml#L128)). The implementation record accurately states that these are configurations for future evidence and that no Windows/Linux run was triggered ([log.md:546](log.md#L546)). Configuration is not a passing result.

Recommendation: capture a green run of the affected Sniff, Darkmatter/DMLS, and downstream Claudine scope on all three OSes. A Windows failure must be resolved rather than hidden by the advisory job before this feature is called production ready.

Strongest verification present: **Level 1 on macOS; Windows/Linux are configured but unverified**. This is a high-severity rigor gap.

### Medium: DMLS emits a document-relative link that does not identify the authored vocabulary file

The catalog now contains a Markdown link to `darkmatter-expressions.md#provider-query-vocabulary` ([expression-functions.yaml:1548](../../docs/schemas/expression-functions.yaml#L1548)), and DMLS copies that description verbatim into hover/completion ([expressions.rs:408](../../dmls/src/overlay/expressions.rs#L408)). In an editor, however, the active document can live anywhere; the relative target is not anchored to `darkmatter/docs/topics/`. The test asserts only that the raw substring reaches both surfaces and assumes Markdown rendering makes it navigable ([expressions.rs:711](../../dmls/src/overlay/expressions.rs#L711)). A relative link beside an arbitrary prompt does not locate the authored documentation.

Recommendation: resolve the catalog documentation target to an absolute workspace/file URI appropriate for the LSP client, or embed the compact vocabulary in the hover. Add a test at the LSP response boundary that verifies the emitted target resolves to the generated topic and anchor.

### Medium: Provider-supplied link destinations bypass the text-escaping boundary

The new shared text escaper protects titles, labels, and metadata, but both formatters interpolate `web_url` directly into the Markdown destination ([pull_requests.rs:75](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L75), [cicd.rs:107](../../lib/src/markdown/compose/expression/functions/cicd.rs#L107)). Provider projections accept response URL fields without validating that they are canonical HTTP(S) URLs for the selected provider/repository. A malformed destination containing `)` can break the promised deterministic projection, and a non-HTTP scheme can become an unsafe link in downstream renderers.

Recommendation: parse and validate normalized record URLs at the provider boundary, require an allowed HTTP(S) origin consistent with the resolved provider policy, and serialize the destination safely. Extend the hostile-provider Level-1 fixture beyond title text to malformed, cross-host, and non-HTTP URL values.

## Verification Matrix

All user-observable requirements in this feature are in-process filesystem/Git/parser/network/rendered-Markdown behavior. None depends on a real terminal's glyph layout, SGR handling, scrolling, input encoder, paste/IME/mouse path, or OS keyboard events; Level 2 and Level 3 are therefore not applicable.

| AC | User-observable contract | Required | Strongest present | Review result |
|---:|---|---:|---:|---|
| 1 | Generated Git context schema/catalog | L1 | L1 | Pass |
| 2 | Demand-driven shared Git capture | L1 | L1 | Pass |
| 3 | Attached-branch/null behavior | L1 | L1 | Pass |
| 4 | Linked-worktree/null behavior | L1 | L1 | Pass |
| 5 | Conflict-path capture and falsy empty array | L1 | L1 | Pass |
| 6 | Shared read-only conflict API | L1 | L1 | Pass |
| 7 | In-memory, non-persisting merge probe | L1 | L1 | Pass |
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
| 21 | Vendor values and bounded ambiguous-host probe | L1 + Wiremock | L1 + Wiremock | Pass for `remote_vendor`; its detected flavor is not reused by provider queries |
| 22 | Exact/paginated PR API, capabilities, no ignored filter | L1 + Wiremock | L1 component | **Fail: API URLs/internal keys/filters/self-hosted path** |
| 23 | `pr*` overloads, errors, formatting, empty result | L1 + Wiremock | L1 end-to-end | **Fail: body error parity and URL boundary** |
| 24 | Provider-aware exact/paginated CI job model | L1 + Wiremock | L1 component | **Fail: self-hosted/API URL/filter capability gaps** |
| 25 | `cicd*` overloads, errors, pagination, projection | L1 + Wiremock | L1 end-to-end | **Fail: same query/error gaps as PR surface** |
| 26 | Three-surface policy/cache/error parity and no passive I/O | L1 + Wiremock | L1 end-to-end | **Fail: test proves frontmatter/body failure asymmetry** |
| 27 | Focused error distinctions and actionable conversion | L1 + Wiremock | L1 component/end-to-end | **Fail on body conversion** |
| 28 | Catalog, DMLS, docs, aliases, overload agreement | L1 | L1 | **Gap: emitted relative vocabulary link is not reliably navigable** |
| 29 | Full focused L1 plus three-OS compile | L1 on 3 OSes | L1 macOS | **Gap: Windows/Linux results absent** |
| 30 | Closed return enums across parser/catalog/docs/DMLS | L1 | L1 | Pass |

## Checks Run

Scope discovery used `sniff repo packages` and `sniff repo package-areas`: the affected packages are `sniff`, `sniff-cli`, `darkmatter`, `darkmatter-cli`, `dmls`, and downstream `claudine`/`claudine-cli`. GitNexus reports **CRITICAL** upstream risk for `ResolvedRemote` (41 impacted symbols, four direct callers, five modules), and MEDIUM risk for the PR and CI query types. That blast radius reinforces the need for downstream and cross-platform gates before changing the resolution model.

Fresh macOS checks in this review:

```text
cargo nextest run -p sniff --features remote <five non-socket focused-provider cases>
  5/5 passed

cargo nextest run -p dmls <catalog vocabulary and remote-enum cases>
  2/2 passed

cargo nextest run -p sniff --features remote -E 'binary(focused_provider)'
  network-backed cases could not bind a loopback Wiremock port in the managed
  sandbox (PermissionDenied); this is an environment blocker, not an assertion
  failure

cargo nextest run -p darkmatter <focused more-is-more/provider cases>
  stopped during a cold dependency build at the non-interactive command ceiling;
  no compiler or test diagnostic was emitted
```

The implementation log records passing full macOS `just test` and `just lint` gates for Sniff and Darkmatter after the review-18 fixes. This review treats those as historical evidence, not as fresh results. The readiness decision does not depend on either interrupted command: the high findings are direct source/spec mismatches, one of which is explicitly demonstrated by the checked-in Level-1 test.

`md get` parsed every requested lifecycle property with the exact value shown in frontmatter, and `git diff --check` passed. Reference/schema validation cannot run because the repository's `schemas/feature-review.yaml` is itself invalid for the current Darkmatter schema loader: tagged schemas support only `kind` and `types`, while this shared file also defines top-level `description` and `$schema`. That pre-existing schema-authoring defect is outside this feature review.

## Production Readiness

**Not ready for production.** Review 18's core adapter hardening is real, but AC22–27 remain observably incomplete at the production resolution/query/error boundaries, and AC16/29 still lack the required Windows/Linux result evidence.
