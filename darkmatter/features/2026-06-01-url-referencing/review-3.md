---
ready: false
agent: codex
model: ""
---

# Review 3: URL Referencing

## Findings

### Critical: fetch policy can be bypassed through HTTP redirects

`biscuit-file` enforces the allowlist only against the originally requested URL before calling `request.send()` ([biscuit-file/lib/src/file_reference/fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/src/file_reference/fetch.rs:131)). Because the shared `reqwest::Client` is supplied by callers and `RemoteFetchRuntime` builds it with `reqwest::Client::new()` ([darkmatter/lib/src/markdown/compose/remote_fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote_fetch.rs:185)), the default redirect behavior can follow a redirect from an allowed host to an unallowed host. That violates the spec's safety requirement that all remote access is gated by the shared `FetchPolicy` and that there is no unguarded fetch path ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-06-01-url-referencing/spec.md:180)).

Fix by making the primitive enforce policy for every effective request target. The simplest safe default is to build clients used by this feature with redirects disabled and return 3xx as an explicit error or structured redirect result. If redirects are supported, each hop must be checked against scheme and host policy before the request is issued. Add L1 wiremock tests for allowed-host `302 Location: http://blocked-host/...` and for redirects to unsupported schemes.

### High: default freshness does not match the spec's stale-on-failure behavior

The spec labels the default as "TTL -> conditional -> stale-on-failure" and says network failure should fall back to the stale cached copy ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-06-01-url-referencing/spec.md:154)). The implementation defaults to `RemoteFreshnessMode::Strict` ([darkmatter/lib/src/markdown/compose/remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:44)), and the CLI default is also `strict` ([darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs:193)). Stale-on-failure only happens when users explicitly pass `--remote-freshness fallback`.

This is a user-observable reliability gap for CI/offline builds, which the spec explicitly calls out. Either change the default to `Fallback`, or update the spec/docs if strict failure is the intended product behavior. The latter would be a spec change, not an implementation completion.

### Medium: expression-function remote URLs only work for literal URLs found by the scanner

Discovery for remote expression arguments is a best-effort line scanner that only registers URLs immediately following the function's opening parenthesis, optionally after a quote ([darkmatter/lib/src/markdown/compose/remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:220)). At point of use, `fetch_remote_text()` errors if the evaluated URL was not pre-registered ([darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:46)).

That means `{{ markdown_title(remote_doc) }}` where `remote_doc` is a frontmatter or state string URL will fail as "not registered" even though the function receives a valid URL argument. The spec describes URL-typed expression-function arguments generally, not only string literals ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-06-01-url-referencing/spec.md:119)). Register-and-fetch at point of use when an evaluated string is an HTTP(S) URL and no slot exists, or parse expression ASTs during discovery and document the literal-only limitation if dynamic URL arguments are intentionally out of scope.

## Test Rigor

The current tests are primarily Level 1, which is appropriate for most of this feature: mocked HTTP servers exercise CLI composition, policy denial, expression reads, refresh, fallback mode, duplicate single-flight fetches, and cache behavior without relying on terminal encoder behavior. The remote link styling requirement has an existing Level 2 terminal test for rendered hyperlink color behavior, which is the right level for terminal rendering.

Missing Level 1 coverage:

- Redirect escape from an allowed host to an unallowed host.
- Default stale-on-failure behavior, as specified.
- Dynamic/evaluated URL arguments in read-side expression functions, or an explicit test proving the intended literal-only contract.

No Level 3 coverage is required for this feature because the spec does not assert OS keyboard, mouse, paste, or IME behavior.

## Recommendation

Do not ship this as production-ready until the redirect policy bypass is closed and the default freshness behavior is reconciled with the spec. After that, the remaining expression-discovery issue can be resolved either by supporting evaluated URL arguments at point of use or by narrowing the public contract and tests to literal URL arguments only.
