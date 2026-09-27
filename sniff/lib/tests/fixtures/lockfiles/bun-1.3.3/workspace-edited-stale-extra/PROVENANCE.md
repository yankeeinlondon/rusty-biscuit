# Provenance: bun-1.3.3/workspace-edited-stale-extra

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Added `"packages/gamma": {"name": "gamma", "version": "1.0.0"}` as the last key of `workspaces`, and `"gamma": ["gamma@workspace:packages/gamma"]` to `packages`, in Bun's formatting.

## Expected result

Lockfile `workspaces` keys (root `""` excluded): `.tools/hidden`, `packages/alpha`, `packages/beta`, `packages/gamma` -> one stale extra member (`packages/gamma`).
