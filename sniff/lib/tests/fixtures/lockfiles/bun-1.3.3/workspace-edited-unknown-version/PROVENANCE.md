# Provenance: bun-1.3.3/workspace-edited-unknown-version

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Changed `"lockfileVersion": 1` to `"lockfileVersion": 99`. `configVersion` left at 1.

## Expected result

Unsupported/unknown lockfile version; the parser declines rather than guessing.
