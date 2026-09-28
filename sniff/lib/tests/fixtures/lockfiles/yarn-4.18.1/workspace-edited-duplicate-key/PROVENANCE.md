# Provenance: yarn-4.18.1/workspace-edited-duplicate-key

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Duplicated the entire `"alpha@workspace:packages/alpha"` entry, so the mapping key appears twice consecutively.

## Expected result

Duplicate mapping key -> treated as malformed (a YAML-strict parser rejects duplicate keys; Yarn's own syml parser would silently keep one).
