# Provenance: bun-1.3.3/workspace-edited-missing

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Removed the `"packages/beta"` key from `workspaces` and the `"@fixture/beta"` entry from `packages`.

## Expected result

Lockfile members: `.tools/hidden`, `packages/alpha`. Manifest member `packages/beta` is missing from the lockfile.
