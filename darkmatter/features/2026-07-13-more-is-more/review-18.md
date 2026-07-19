---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T10:01:12-07:00
spec: 2026-07-13-more-is-more/spec.md
log: darkmatter/features/2026-07-13-more-is-more/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-18.md
previous: 2026-07-13-more-is-more/review-17.md
---

# Review 18

## Summary

The feature is **not production ready**. Review 17's missing AC17–30 surface now exists, and the local filesystem/Git, expression-literal, catalog-enum, and downstream `claudine` work is materially complete. The new provider-query implementation is not yet faithful to the ratified contract, however: it can send invalid provider state tokens, stop before the requested global order is knowable, silently treat bounded truncation as a complete no-match result, accept or reject canonical query inputs incorrectly, and discard provider-exposed CI/CD fields. Required Level-1 verification also stops at component boundaries and has no Windows or Linux compile evidence.

## Findings

### Critical: Provider list adapters do not implement complete, correctly ordered canonical queries

The specification permits local emulation only over a complete bounded result domain, requires pagination until the requested limit or provider exhaustion, and defaults both list functions to newest-first ([spec.md:1290](spec.md#L1290), [spec.md:1801](spec.md#L1801)). The implementation instead stops PR collection as soon as `limit` matches are seen and sorts only afterward ([focused.rs:153](../../../sniff/lib/src/remote/focused.rs#L153), [focused.rs:179](../../../sniff/lib/src/remote/focused.rs#L179)). CI/CD direct and parent traversal likewise return immediately at `limit` or a hard inspection cap, then sort the partial subset ([focused.rs:245](../../../sniff/lib/src/remote/focused.rs#L245), [focused.rs:256](../../../sniff/lib/src/remote/focused.rs#L256), [focused.rs:291](../../../sniff/lib/src/remote/focused.rs#L291), [focused.rs:311](../../../sniff/lib/src/remote/focused.rs#L311)). If 20 pages contain no match, PR code exits the loop with `exhausted == false` but returns neither an error/warning nor a continuation token because `normalized.len() != limit` ([focused.rs:180](../../../sniff/lib/src/remote/focused.rs#L180)). That is a silent incomplete-domain `[]`, which the specification explicitly forbids.

The provider-side state projection is also not an adapter: it forwards the canonical state token almost verbatim ([focused.rs:408](../../../sniff/lib/src/remote/focused.rs#L408)). Consequently the default canonical `open` is sent to GitLab, whose API requires `opened`; canonical `merged` is sent to GitHub, whose list endpoint accepts only `open`, `closed`, or `all`; and Bitbucket receives invented values such as `ALL` or `CLOSED` rather than the service's repeated `OPEN`/`MERGED`/`DECLINED` states. These provider vocabularies are confirmed by the [GitLab merge-request API](https://docs.gitlab.com/api/merge_requests/), [GitHub pull-request API](https://docs.github.com/en/rest/pulls/pulls?apiVersion=latest#list-pull-requests), and [Bitbucket pull-request API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/#api-repositories-workspace-repo-slug-pullrequests-get).

The current Level-1 pagination test checks only a two-page GitHub author filter and does not assert provider state parameters, requested global ascending/descending order, exhaustion, or cap behavior ([focused_provider.rs:169](../../../sniff/lib/tests/focused_provider.rs#L169)). The capability test treats an empty successful response as proof that a field is “honored,” so it cannot detect incorrect filtering or ordering ([focused_provider.rs:221](../../../sniff/lib/tests/focused_provider.rs#L221)).

Recommendation: introduce flavor-specific request projections, traverse an authoritative complete domain for local filters/sorts, and return an explicit focused capability/incomplete-result error whenever a safety cap prevents exact completion. Add discriminating Wiremock fixtures for every provider flavor, state mapping, default/explicit order, filters beyond page one, and every cap/exhaustion edge.

Strongest verification present: **Level 1 component integration, insufficient for the specified behavior**. No Level 2 or Level 3 test is applicable.

### High: Canonical query validation and defaults disagree with the public contract

The specification requires unknown keys, wrong types, invalid enum values, and invalid datetimes to fail before network access; PR state is limited to `open`, `closed`, and `merged`; and CI/CD `parent` accepts either an integer or string ([spec.md:1270](spec.md#L1270), [spec.md:1290](spec.md#L1290), [spec.md:1336](spec.md#L1336)). Several inputs violate those rules:

- `remote` is removed with an `as_str()` chain, so `remote: 42` or an empty value is silently converted to “preferred remote” instead of rejected ([pull_requests.rs:60](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L60), [cicd.rs:96](../../lib/src/markdown/compose/expression/functions/cicd.rs#L96)).
- `PullRequestState` publicly deserializes `draft` and `all`, so those values pass `pr_list()` even though `draft` is a separate boolean and `state` is closed to three values ([types.rs:137](../../../sniff/lib/src/remote/types.rs#L137), [types.rs:189](../../../sniff/lib/src/remote/types.rs#L189)).
- `CiCdJobQuery.parent` is `Option<String>`, so the specified integer form is rejected during deserialization ([types.rs:426](../../../sniff/lib/src/remote/types.rs#L426)).
- Datetime fields are unvalidated strings and ranges are compared lexically ([pull_requests.rs:72](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L72), [cicd.rs:108](../../lib/src/markdown/compose/expression/functions/cicd.rs#L108)). A single invalid datetime is accepted and reaches the network.
- Both query structs derive `Default` with `descending == false` ([types.rs:258](../../../sniff/lib/src/remote/types.rs#L258), [types.rs:426](../../../sniff/lib/src/remote/types.rs#L426)). `cicd_list({})` therefore sorts oldest-first ([focused.rs:224](../../../sniff/lib/src/remote/focused.rs#L224)), while the numeric overload alone forces newest-first ([cicd.rs:91](../../lib/src/markdown/compose/expression/functions/cicd.rs#L91)).

Recommendation: parse the authored canonical vocabulary into dedicated Darkmatter input types, validate RFC 3339/ISO 8601 datetimes before repository or client resolution, make newest-first an explicit default, and translate into provider-neutral Sniff queries only after validation succeeds.

Strongest verification present: **Level 1 unit tests**, but their invalid-input tables omit every case above ([pull_requests.rs:162](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L162), [cicd.rs:198](../../lib/src/markdown/compose/expression/functions/cicd.rs#L198)).

### High: CI/CD normalization drops fields the structured-record contract promises to retain

The specification requires available parent identity, branch/ref, commit, timestamps, canonical URL, and runner metadata in Sniff's structured job record ([spec.md:1318](spec.md#L1318)). `normalize_job()` only reads top-level `head_branch`/`ref`, `head_sha`/`sha`, a narrow timestamp set, and nested `runner.name` ([focused.rs:485](../../../sniff/lib/src/remote/focused.rs#L485)). It therefore loses common provider data such as GitLab's nested `commit.id`, `finished_at`, and `runner.description`; GitHub/Gitea parent-run branch/commit/trigger and top-level `runner_name`; and Bitbucket `started_on`/`completed_on`. Parent normalization retains only ID, name, and URL, so metadata available solely on a workflow run cannot reach its jobs ([focused.rs:801](../../../sniff/lib/src/remote/focused.rs#L801)).

The provider test fixtures are intentionally skeletal and assert only name, normalized status, and non-empty parent identity ([focused_provider.rs:117](../../../sniff/lib/tests/focused_provider.rs#L117)); they do not verify the promised fields for any flavor.

Recommendation: add flavor-specific response projections (or typed wire DTOs) rather than one key-probing normalizer, then test realistic exact and list payloads for every supported provider/API flavor.

Strongest verification present: **Level 1 Wiremock**, but no discriminating field-retention verification.

### High: The provider expression surface has no end-to-end Level-1 network verification

AC23, AC25, and AC26 promise that actual `pr*`/`cicd*` expressions behave identically across frontmatter, body, and `$()` surfaces, honor exact-host policy, preserve focused errors, and single-flight identical requests ([spec.md:1691](spec.md#L1691), [spec.md:1698](spec.md#L1698), [spec.md:1701](spec.md#L1701)). Sniff has Wiremock component tests and Darkmatter has private parser/formatter unit tests, but no test composes a document containing a provider function against Wiremock. The cross-surface cache test invokes `cached_provider_query` with a manufactured closure rather than a provider expression ([options.rs:2200](../../lib/src/markdown/compose/context/options.rs#L2200)); the real one-request frontmatter/body integration exercises the unrelated remote `frontmatter(url)` function ([transclusion.rs:1198](../../lib/src/markdown/compose/tests/transclusion.rs#L1198)).

Recommendation: add Darkmatter L1 integration fixtures that compose each exact/list function through body, frontmatter, and `$()` against Wiremock, assert rendered values/errors, and assert the server receives exactly one request for identical normalized calls.

Strongest verification present: **Level 1 component tests only; no Level-1 end-to-end test of the user-facing provider expressions**. This is a high-severity verification gap under the review's rigor rules. Level 2 and Level 3 are not applicable.

### High: Required Windows and Linux compile verification is absent

AC16 and AC29 explicitly require macOS, Windows, and Linux compile checks ([spec.md:1667](spec.md#L1667), [spec.md:1711](spec.md#L1711)). The implementation record states that only macOS was verified and that Windows/Linux were not run ([log.md:265](log.md#L265)). Portable dependencies make those targets plausible, but they do not constitute compile evidence.

Recommendation: add the affected `sniff`, `darkmatter`, `darkmatter-cli`, `dmls`, and downstream `claudine` packages to real Windows and Linux CI jobs and record those results before production approval.

Strongest verification present: **Level 1 on macOS only**.

### Medium: “Markdown-escaped” provider text still permits Markdown formatting injection

Both formatters collapse whitespace and escape only backslashes and square brackets ([pull_requests.rs:111](../../lib/src/markdown/compose/expression/functions/pull_requests.rs#L111), [cicd.rs:145](../../lib/src/markdown/compose/expression/functions/cicd.rs#L145)). Provider titles such as `**urgent**`, `` `code` ``, or `_name_` retain active Markdown despite the specification's Markdown-escaped requirement ([spec.md:1374](spec.md#L1374)). Tests cover only square brackets.

Recommendation: use one shared CommonMark-aware plain-text escaping helper for both formatters and cover every punctuation class that can alter inline parsing.

### Medium: DMLS hover and authored docs do not expose or link the query vocabulary

The specification says function hover links to the authored query vocabulary ([spec.md:1441](spec.md#L1441)). DMLS copies only the short catalog description into completion documentation ([expressions.rs:408](../../dmls/src/overlay/expressions.rs#L408)), while the generated expression documentation contains only terse table rows for `pr_list` and `cicd_list` and no vocabulary section or link ([darkmatter-expressions.md:422](../../docs/topics/darkmatter-expressions.md#L422)). Users therefore cannot discover valid keys, defaults, bounds, or status values from either promised surface.

Recommendation: author one canonical query-vocabulary section, link it from the catalog descriptions, and assert the link reaches DMLS hover/completion and generated docs.

### Medium: Each provider expression creates a thread and Tokio runtime instead of using the run-local executor

The design calls for a runtime-owned executor following `RemoteFetchRuntime` ([spec.md:1056](spec.md#L1056)). `provider::run` instead spawns a new OS thread and builds a new current-thread Tokio runtime for every cache miss ([provider.rs:24](../../lib/src/markdown/compose/expression/functions/provider.rs#L24)). This adds avoidable thread/runtime construction overhead and divorces provider work from the shared executor lifecycle even though concurrency and memoization state are run-local.

Recommendation: execute provider futures on one executor owned by the shared run runtime and retain the synchronous bridge at that boundary.

## Verification Matrix

| Requirement group | Required level | Strongest present | Result |
|---|---:|---:|---|
| AC1–16: context, Git capture, and conflict prediction | Level 1 | Level 1 | Implemented with discriminating local tests; scoped merge-prediction rerun passed 10/10 |
| AC17–19: index functions, literals, and preferred remote | Level 1 | Level 1 | Implemented with focused parser/runtime/resolver tests |
| AC20–21: live branch observation and vendor probing | Level 1 with Wiremock for network behavior | Level 1 with Wiremock | Core paths are present; no Level 2/3 requirement |
| AC22–25: PR/CI exact and list APIs/functions | Level 1 with Wiremock | Level 1 component tests | **Fail**: adapter semantics, validation, ordering, and record retention do not meet the contract |
| AC26–28: cross-surface policy/cache/errors/catalog/DMLS/docs | Level 1 with Wiremock for provider expressions | Level 1 mechanism/component tests | **Gap**: no composed provider-expression network test; vocabulary link/docs missing |
| AC29: scoped suites and three-OS compile checks | Level 1 on macOS, Windows, and Linux | macOS only | **Gap**: Windows/Linux evidence absent |
| AC30: closed enum returns | Level 1 | Level 1 | Implemented with parser/projection/catalog tests |

No requirement in this feature depends on terminal glyph widths, SGR styling, scrolling, paste/IME/mouse behavior, hotkeys, or the terminal emulator's keyboard encoder. Level 2 and Level 3 tests are therefore not required.

## Checks Run

Repository scope was established with `sniff repo packages`, `sniff repo package-areas`, and package-dependency discovery. GitNexus reports CRITICAL upstream risk for `ResolvedRemote` (26 impacted symbols, four direct callers) and MEDIUM risk for `merge_conflicts_with_branch_at`, so the affected scope is `sniff`, `sniff-cli`, `darkmatter`, `darkmatter-cli`, `dmls`, and downstream `claudine`.

Fresh macOS verification produced these results:

```text
cd sniff && just test
  sniff library: advanced to the sniff-cli phase after the 1,565-test library run
  sniff-cli: interrupted during a cold build after the non-interactive command ceiling

cargo nextest run -p sniff --features remote \
  -E 'binary(focused_provider) | binary(remote_observation) | binary(merge_conflict_prediction)'
  merge_conflict_prediction: 10/10 passed
  Wiremock binaries: could not bind localhost in the managed sandbox (PermissionDenied)

cargo check -p sniff --features remote -p darkmatter -p claudine --tests
  interrupted during a cold check build after the command ceiling; no compiler diagnostic emitted
```

The interrupted commands are **not** counted as passes or product failures. Historical cycle-17 logs report green macOS area tests/lints and a green `claudine --tests` check; this review treats those as historical evidence only. The critical/high findings are established by direct source-contract mismatches and missing discriminating assertions, not by the sandbox's inability to bind a new Wiremock server.

`biscuit-file` parsed all requested lifecycle frontmatter values exactly, and `git diff --check` passed. `md schema validate` could not validate the review because the shared `schemas/feature-review.yaml` definition itself mixes tagged-schema `kind`/`types` with unsupported top-level `$schema`/`description` keys. That pre-existing schema-authoring defect is outside this feature's implementation scope.

## Production Readiness

**Not ready for production.** The formerly absent AC17–30 implementation is now substantial, but the core PR/CI list contract is observably incorrect, required canonical inputs and structured fields are mishandled, provider expressions lack end-to-end Level-1 verification, and the required Windows/Linux compile checks have not occurred.
