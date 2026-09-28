# Provenance: yarn-4.18.1/workspace-edited-missing

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Deleted the whole `"@fixture/beta@workspace:^, @fixture/beta@workspace:packages/beta"` entry (5 lines plus its blank separator). `alpha`'s `dependencies` still names `@fixture/beta`.

## Expected result

Lockfile members: `.tools/hidden`, `packages/alpha`. Manifest member `packages/beta` is missing from the lockfile.
