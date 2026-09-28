# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `pnpm-lock.yaml` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Changed `lockfileVersion: '9.0'` to `lockfileVersion: '99.0'`.

## Expected result

The document parses, but the lockfile version is one no current pnpm writes; the parser must report an unknown/unsupported lockfile version rather than trusting the membership data.
