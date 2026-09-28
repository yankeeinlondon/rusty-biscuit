# Provenance: yarn-4.18.1/workspace-edited-malformed-trailing

Hand-edited from `workspace` (`yarn-4.18.1/workspace`), date 2026-09-26. Only `yarn.lock` was edited; manifests are identical to the real fixture.

## Edit

Appended `\n}}} not-valid\n` after the final (valid) entry.

## Expected result

Parse error (malformed lockfile); no membership comparison.
