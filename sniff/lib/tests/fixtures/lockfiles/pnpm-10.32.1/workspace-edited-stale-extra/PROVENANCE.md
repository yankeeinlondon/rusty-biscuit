# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `pnpm-lock.yaml` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Added an importer entry `packages/gamma: {}` (with a preceding blank line) after `packages/beta: {}` under `importers`.

## Expected result

The lockfile records a member (`packages/gamma`) that the manifests no longer declare: a stale-extra membership drift. Lockfile members: `.tools/hidden`, `packages/alpha`, `packages/beta`, `packages/gamma`; manifest members: `.tools/hidden`, `packages/alpha`, `packages/beta`.
