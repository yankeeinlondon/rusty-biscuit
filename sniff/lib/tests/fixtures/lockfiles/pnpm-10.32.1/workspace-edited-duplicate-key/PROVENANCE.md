# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `pnpm-lock.yaml` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Duplicated the importer entry `packages/beta: {}` (a second identical key directly after the first, separated by a blank line) under `importers`.

## Expected result

YAML forbids duplicate mapping keys; a strict parser must reject the document (or report a duplicate importer) rather than silently collapsing the two entries.
