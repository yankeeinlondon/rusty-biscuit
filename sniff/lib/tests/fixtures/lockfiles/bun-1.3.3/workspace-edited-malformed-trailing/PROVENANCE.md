# Provenance: bun-1.3.3/workspace-edited-malformed-trailing

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Appended `\n}}} not-valid\n` after the closing `}` of the document.

## Expected result

Parse error (malformed lockfile); no membership comparison.
