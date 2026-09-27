# Provenance: yarn-4.18.1/workspace-edited-unknown-version

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Changed `__metadata.version` from `10` to `99` (kept as an integer, the type Yarn writes).

## Expected result

Unsupported/unknown lockfile version; the parser declines rather than guessing.
