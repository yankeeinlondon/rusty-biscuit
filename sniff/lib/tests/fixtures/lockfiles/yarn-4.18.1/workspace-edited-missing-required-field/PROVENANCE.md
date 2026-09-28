# Provenance: yarn-4.18.1/workspace-edited-missing-required-field

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Yarn has no dedicated membership section; membership is the set of entries whose resolution uses the `workspace:` protocol. Deleted every such entry (`@fixture/beta`, `alpha`, `fixture-root` (root), `hidden-tool`), leaving `__metadata` and the `local-lib`/`portal-lib` entries.

## Expected result

No workspace entries at all, not even the root `fixture-root@workspace:.` -> membership data absent (a Berry lockfile always records the root workspace), so the lockfile cannot corroborate membership.
