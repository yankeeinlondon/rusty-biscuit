# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Removed the entire top-level `packages` object (key and value, plus the preceding comma). `name`, `version`, `lockfileVersion`, and `requires` remain.

## Expected result

The document parses and declares `lockfileVersion: 3`, but has no `packages` membership section; the parser must report a missing required field rather than an empty member set.
