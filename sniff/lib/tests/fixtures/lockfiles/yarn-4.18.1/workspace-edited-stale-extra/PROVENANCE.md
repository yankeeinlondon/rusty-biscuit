# Provenance: yarn-4.18.1/workspace-edited-stale-extra

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Inserted a `"gamma@workspace:packages/gamma"` entry (resolution `gamma@workspace:packages/gamma`, `languageName: unknown`, `linkType: soft`) before the `hidden-tool` entry, in sorted position.

## Expected result

Lockfile members: `.tools/hidden`, `packages/alpha`, `packages/beta`, `packages/gamma`. Manifest members lack `packages/gamma` -> one stale extra member (`packages/gamma`).
