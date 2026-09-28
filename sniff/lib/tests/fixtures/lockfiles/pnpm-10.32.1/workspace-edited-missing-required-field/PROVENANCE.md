# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `pnpm-lock.yaml` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Removed the entire `importers:` section (the key and every entry beneath it, plus the blank line before it). `lockfileVersion` and `settings` remain.

## Expected result

The document parses and has a known version but no `importers` membership section; the parser must report a missing required field rather than an empty member set.
