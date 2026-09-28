# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `pnpm-lock.yaml` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Removed the importer entry `packages/beta: {}` (and its preceding blank line) from `importers`. The `link:../beta` version in `packages/alpha`'s dependency was left unchanged.

## Expected result

The lockfile omits the current member `packages/beta`: a missing-member drift. Lockfile members: `.tools/hidden`, `packages/alpha`; manifest members: `.tools/hidden`, `packages/alpha`, `packages/beta`.
