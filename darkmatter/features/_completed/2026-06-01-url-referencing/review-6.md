---
ready: true
agent: codex
model: ""
---

# Review 6: URL Referencing

## Findings

### High: cached local transclusions can bypass remote freshness revalidation

The spec requires changed remote dependencies to invalidate parents through the
remote artifact content hash, and freshness controls such as `--remote-refresh`
and stale TTLs must affect composed output. That is covered for a root document
that directly uses `::file <url>` from stdin ([darkmatter/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/cli.rs:561)), but I do not see coverage for the important cached-local-child case:

```md
# root.md
::file child.md

# child.md
::file https://allowed.example/remote.md
```

The implementation can accept the persistent compose cache for `child.md`
before any remote fetch/revalidation runs for the child's nested URL. Root eager
discovery only scans the root content ([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:495)), so it does not register remote URLs inside local children. The nested discovery/register path only runs inside the remote-file compose path after that remote body has been fetched ([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1930)); it does not run when a local child is served from `get_or_compute_compose`.

For local child transclusion, the cache checks persistent compose state before recomputing ([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:2203), [darkmatter/lib/src/markdown/compose/cache/runtime.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/runtime.rs:297)). During validation, a `RemoteUrl` dependency is considered current by reading the existing remote manifest's `content_hash` only ([darkmatter/lib/src/markdown/compose/cache/runtime.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/runtime.rs:1019)). It does not consult `RemoteFetchRuntime::content_hash`, which would block on the run's in-flight revalidation slot ([darkmatter/lib/src/markdown/compose/remote_fetch.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote_fetch.rs:410)), and in this path no slot exists anyway.

The likely user-visible result is stale composed output across runs:

1. Compose `root.md` with `--cache-root`; `child.md` and the remote artifact are cached.
2. The remote body changes or `--remote-refresh` is passed.
3. Compose `root.md` again; the `child.md` compose artifact can validate against the old remote manifest and return the old rendered child without fetching the remote URL.

Fix by ensuring remote dependencies recorded in compose manifests are revalidated under the active `RemoteReadConfig` before the parent compose artifact is accepted, or by discovering/registering nested remote dependencies for local cached children and making dependency validation wait on the corresponding remote slot. Add a Level 1 regression test with a file-backed root, local child, cache root, and a remote response that changes under `--remote-refresh` or expired TTL.

## Test Rigor

I ran the targeted Level 1 remote suite:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

Result: pass. `darkmatter` ran 102 remote-filtered tests, `darkmatter-cli` ran 8 remote CLI tests, and the filtered `biscuit-file` targets reported no failures.

Level 1 is the correct tier for URL classification, fetch policy, remote transclusion, expression-function reads, cache freshness, and CLI flags. The spec does not define terminal rendering fidelity, keyboard input, paste, IME, mouse, or modifier-key behavior for this feature, so no Level 2 or Level 3 coverage is required for production readiness.

The remaining gap is still Level 1: a cross-run integration test for persistent compose-cache invalidation when a local transcluded child depends on a remote URL. Because this gap affects user-visible composed output under documented freshness controls, the feature is not ready for production.

## Recommendation

Do not mark this production-ready until the local-child persistent-cache path revalidates remote dependencies and has a regression test proving `--remote-refresh` or stale TTL updates the final composed output.

## Resolution

The finding is addressed.

### Root cause

`RunLocalCache::resolve_dependency_closure_hash`
([darkmatter/lib/src/markdown/compose/cache/runtime.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/runtime.rs)) treated a
`RemoteUrl` dependency as current by reading the persisted
`RemoteUrlManifest.content_hash` directly, never consulting the run's
`RemoteFetchRuntime`. When a local child served from the persistent compose
cache carried a nested remote dependency, no fetch or revalidation ran for that
URL, so a changed remote body (or `--remote-refresh` / expired TTL) could not
invalidate the parent — stale composed output across runs.

### Fix

Dependency validation now revalidates remote dependencies under the active
`RemoteReadConfig` before the parent compose artifact is accepted:

- `RunLocalCache` carries the run's `RemoteFetchRuntime`
  (`with_remote_fetch`), wired in from `PipelineRuntime::with_remote_fetch`.
  Both share the same `Arc`-backed slot map, so single-flight dedups across the
  validation and recompute paths.
- The `RemoteUrl` arm of `resolve_dependency_closure_hash` recovers the URL
  from the persisted manifest, calls `register_and_fetch` (a no-op if already
  in flight this run), then blocks on `content_hash`. A changed body yields a
  new hash → the parent is recomputed against the fresh remote content; a
  Strict revalidation failure yields `None` → the dependency is treated as
  invalid so the failure surfaces (or stale is served under `Fallback`).

Because validation recurses through `ComposeDocumentCore` dependencies, nested
remote URLs are revalidated at any depth, not just for a directly-referenced
remote root.

### Regression test

`cached_local_child_revalidates_nested_remote_under_refresh`
(`darkmatter/lib/src/markdown/compose/mod.rs`, `remote_transclusion_tests`) is a
Level 1 test with a file-backed root, a local `::file ./child.md`, a
`--cache-root`, and a wiremock remote whose body changes between runs. Run 1
populates the caches (remote → `v1`); run 2 passes `--remote-refresh` with the
remote now serving `v2` and asserts the composed output contains `v2` and not
`v1`. It fails before the fix (stale `v1`) and passes after.

### Verification

The reviewer's exact command now passes:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

`darkmatter` runs 103 remote tests (102 prior + the new regression), 0 failed;
`darkmatter-cli` runs 8 remote CLI tests, 0 failed; `biscuit-file` reports 0
failures. The full `darkmatter` lib suite (3684 tests) passes, with clean
`clippy`.
