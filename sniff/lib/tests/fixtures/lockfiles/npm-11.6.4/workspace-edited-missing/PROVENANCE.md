# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Removed both entries npm writes for the member `packages/beta`: `packages["node_modules/@fixture/beta"]` (the link) and `packages["packages/beta"]` (the member itself). `packages/alpha`'s `dependencies` still names `@fixture/beta`.

## Expected result

The lockfile omits the current member `packages/beta`: a missing-member drift. Lockfile members: `.tools/hidden`, `packages/alpha`; manifest members: `.tools/hidden`, `packages/alpha`, `packages/beta`.
