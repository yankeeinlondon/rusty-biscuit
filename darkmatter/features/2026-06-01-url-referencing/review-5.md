---
ready: false
agent: codex
model: ""
---

# Review 5: URL Referencing

## Findings

### High: targeted remote test suite is failing after the concurrency default change

The implementation now defines the remote concurrency default as `16`, matching the spec's "default ~16" requirement ([darkmatter/lib/src/markdown/compose/remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:73)). However, the `ComposeOptions` regression test still asserts the old value of `4` ([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:2596)).

I ran:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

Result: `darkmatter` failed with:

```text
markdown::compose::types::tests::remote_read_config_defaults_to_deny_all
assertion `left == right` failed
  left: 16
 right: 4
```

This is likely a stale test expectation rather than broken runtime behavior, but production readiness requires a green targeted suite. Update the assertion to the canonical default constant or to `16`, then rerun the same command.

Required verification: Level 1 unit/integration. No Level 2 or Level 3 coverage is needed for this requirement because it is configuration behavior, not terminal rendering or keyboard input behavior.

### Low: `Optimistic` freshness docs still describe TTL-scoped behavior while the implementation ignores TTL

The implementation intentionally serves any cached artifact in `Optimistic` mode, even when the entry is stale ([darkmatter/lib/src/markdown/compose/cache/remote_cache.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/remote_cache.rs:128)). The persistent-cache tests also pin this behavior. But both the library enum and CLI help still say "Serve cache without revalidation when within TTL" ([darkmatter/lib/src/markdown/compose/remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:85), [darkmatter/cli/src/args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/args.rs:38)).

That wording is misleading for users choosing a freshness policy. Change it to make the risk explicit, for example: "Serve any cached artifact without revalidation, even when stale." The existing Level 1 tests already verify the behavior; this only needs doc/help text cleanup.

## Test Rigor

This feature is mostly backend composition, HTTP policy, caching, and CLI argument behavior. Level 1 coverage is the appropriate tier for URL classification, deny-all defaults, redirect blocking, remote `::file`/`::code`, read-side expression functions, cache TTL/revalidation, stale fallback, concurrency caps, and CLI flags. The implementation has good Level 1 coverage in those areas, including local HTTP server tests for remote reads and fetch-policy behavior.

No Level 2 or Level 3 tests are required by the spec because it does not define terminal rendering fidelity, PTY behavior, OS keyboard input, paste, IME, mouse, or modifier-key behavior.

The feature is not production-ready yet because the targeted Level 1 remote test suite is red.

## Recommendation

Do not mark this ready for production until the stale concurrency assertion is fixed and the targeted remote test command passes. The remaining freshness wording issue is low risk but should be cleaned up in the same iteration because it affects CLI/user-facing semantics.
