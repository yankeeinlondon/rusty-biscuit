# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Changed `"lockfileVersion": 3` to `"lockfileVersion": 99`.

## Expected result

The document parses, but the lockfile version is one no current npm writes; the parser must report an unknown/unsupported lockfile version rather than trusting the membership data.
