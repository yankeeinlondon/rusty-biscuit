# Provenance: bun-1.3.3/workspace-edited-missing-required-field

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Removed the entire top-level `"workspaces"` key and its object. `packages` (which still contains `workspace:` entries) left intact.

## Expected result

Membership section absent -> the lockfile cannot corroborate membership (missing required field), not "zero members".
