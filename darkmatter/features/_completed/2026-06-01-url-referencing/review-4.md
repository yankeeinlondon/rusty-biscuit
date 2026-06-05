---
ready: true
agent: codex
model: ""
---

# Review 4: URL Referencing

## Findings

### Critical: the public fetch primitive still permits redirect policy bypasses with ordinary clients

The redirect fix only holds when the caller supplies `policy_client()` ([biscuit-file/lib/src/file_reference/fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/src/file_reference/fetch.rs:116)). The public primitive still accepts any `reqwest::Client` ([biscuit-file/lib/src/file_reference/fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/src/file_reference/fetch.rs:157)), and the existing fetch integration helper still uses `Client::new()` for most tests ([biscuit-file/lib/tests/fetch_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/tests/fetch_integration.rs:9)). With reqwest's default redirect behavior, `request.send()` follows a 3xx before `reject_redirect()` sees the response, so the primitive cannot enforce the allowlist for every effective request target. The doc comment currently assumes "the feature's clients do not follow redirects" ([biscuit-file/lib/src/file_reference/fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/src/file_reference/fetch.rs:152)), but the type signature does not enforce that.

This violates the spec's safety requirement that all remote access flows through the single `biscuit-file` primitive where `FetchPolicy` is enforced. Fix the primitive API so bypassing redirect policy is impossible: either do not accept an arbitrary client, wrap the client in a policy-enforced newtype, or install a custom reqwest redirect policy that checks each hop before issuing the redirected request. Also remove the fallback from `policy_client()` to `Client::new()` ([biscuit-file/lib/src/file_reference/fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/src/file_reference/fetch.rs:120)), because that fallback silently weakens the safety boundary if client construction ever fails.

Required verification: Level 1 wiremock coverage must call `fetch()` with an ordinary/default client and prove a redirect from an allowed origin to a disallowed reachable target is blocked before the second request is issued. The current redirect tests only cover `policy_client()` ([biscuit-file/lib/tests/fetch_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-file/lib/tests/fetch_integration.rs:196)), so they do not verify the public primitive's stated contract.

### Medium: remote concurrency configuration is incomplete versus the spec

The spec requires a concurrency cap with a default around 16 and says it is overridable via `ComposeOptions`, a CLI flag, and an env var ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-06-01-url-referencing/spec.md:125)). The implementation defaults to 4 in both `RemoteReadConfig` and the CLI ([darkmatter/lib/src/markdown/compose/remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:84), [darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs:181)), and I found no env-var override path.

This is not a safety issue, but it is a designed behavior gap and can materially affect remote-heavy composes. Either implement the env override and reconcile the default with the spec, or update the spec to explicitly accept `4` and remove the env-var requirement. Add Level 1 tests for precedence across default, env var, CLI flag, and `ComposeOptions`.

### Medium: invalid `--remote-freshness` values silently become `strict`

`build_remote_read_config()` maps `"optimistic"` and `"fallback"` explicitly, but every other string becomes `RemoteFreshnessMode::Strict` ([darkmatter/cli/src/commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands.rs:388)). A typo like `--remote-freshness fallbak` therefore changes behavior instead of failing fast. The argument is modeled as a free-form `String` ([darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs:193)) even though clap can enforce a closed value set.

This is a user-facing ergonomics and reliability issue for the freshness modes defined by the spec. Use a `ValueEnum` for the mode or return a CLI error for unknown values. Add a Level 1 CLI test that an invalid mode exits non-zero and prints the accepted values.

## Test Rigor

Most of this feature is correctly testable at Level 1: URL classification, policy denial, fetch behavior, cache freshness, stale fallback, CLI composition, and expression-function reads do not require a real terminal emulator or OS keyboard injection. No Level 3 coverage is required because the spec does not define keyboard, mouse, paste, IME, or modifier-key behavior.

Coverage is still insufficient for production readiness because the highest-risk safety requirement, policy enforcement across redirects, is only verified for the helper-built client, not for the public primitive signature callers actually receive. The concurrency env override has no matching Level 1 coverage because the behavior is missing. Invalid freshness-mode handling also lacks Level 1 CLI coverage.

## Recommendation

Do not mark this production-ready yet. The previous iteration fixed the darkmatter call sites, but the shared `biscuit-file` primitive still exposes a redirect-following footgun that contradicts the safety model. After the fetch API is made policy-safe by construction, resolve the concurrency configuration drift and make freshness mode parsing explicit.

## Resolution

All three findings have been addressed.

### Critical: fetch API is now policy-safe by construction

The redirect bypass is closed at the type level. `biscuit-file` now exposes a
`PolicyClient` newtype that wraps a `reqwest::Client` built with
`redirect::Policy::none()`; its only constructor, `PolicyClient::new()`, returns
`Result<Self, FetchError>` and surfaces a new `FetchError::ClientBuild` on
backend-init failure. The old `policy_client()` free function — and its silent
`unwrap_or_else(|_| reqwest::Client::new())` fallback — is removed.

`fetch`, `post`, `fetch_blocking`, and `post_blocking` now accept `&PolicyClient`
instead of `&reqwest::Client`. There is no longer any way to hand the primitive
a redirect-following client, so a 3xx can never be followed to an unallowed host
regardless of which caller invokes it: the contract the doc comment previously
only *assumed* is now enforced by the signature. A redirect still surfaces as
`FetchError::RedirectBlocked` via `reject_redirect`.

Both darkmatter consumers were updated: `RemoteFetchRuntime` stores
`Option<PolicyClient>` (a build failure fails every fetch slot rather than
falling back to an unsafe client, mirroring the existing runtime-build-failure
path), and the side-effect engine's `http_post` propagates a build failure as
`EffectError::Network`.

The required Level 1 wiremock coverage was retained and re-pointed at
`PolicyClient` (`policy_client_blocks_redirect_to_unallowed_host`,
`policy_client_blocks_redirect_to_unsupported_scheme` in
`biscuit-file/lib/tests/fetch_integration.rs`); the redirect target host is never
mounted, so a blocked 3xx proves no second request is issued. The reviewer's
"call `fetch()` with an ordinary/default client" scenario is no longer
expressible — passing a bare `reqwest::Client` is now a compile error, which is
the stronger guarantee.

### Medium: remote concurrency reconciled with the spec

`DEFAULT_REMOTE_CONCURRENCY` is now `16` (matching the spec), used by both
`RemoteReadConfig::default()` and the CLI. A `DARKMATTER_REMOTE_CONCURRENCY`
environment override was added (named after the existing
`DARKMATTER_SCHEMA_CACHE_SIZE` convention). The CLI `--remote-concurrency` flag
is now `Option<usize>` so an explicit value is distinguishable from the default,
and `resolve_remote_concurrency()` applies the precedence
**CLI flag > env var > default** (with `0` promoted to `1`). A programmatic
`RemoteReadConfig` (the `ComposeOptions` path) sits above all of these.

New Level 1 tests in `remote.rs` cover the full precedence chain — default,
env-over-default, CLI-over-env, invalid/zero env fallback, zero-CLI clamp, and
the authoritative `ComposeOptions` value — using a pure resolver core so no
process-environment mutation is needed.

### Medium: invalid `--remote-freshness` values now fail fast

`--remote-freshness` is now a clap `ValueEnum` (`RemoteFreshness`), so an
unrecognized value exits non-zero and prints the accepted set instead of
silently collapsing to `Strict`. `build_remote_read_config` maps the enum
exhaustively onto `RemoteFreshnessMode`.

New Level 1 CLI test `test_compose_invalid_remote_freshness_fails_fast` asserts
that `--remote-freshness fallbak` exits non-zero and lists `optimistic`,
`strict`, and `fallback`.

`md compose` docs were updated for the new concurrency default and env var. All
`biscuit-file` (`--features fetch`), `darkmatter`, and `darkmatter-cli`
remote/effects/CLI tests pass with clean clippy.
