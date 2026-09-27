# Provenance: bun-1.3.3/workspace-edited-duplicate-key

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Duplicated the `"packages/alpha"` key (with its whole object) inside `workspaces`.

## Expected result

Duplicate membership key -> treated as malformed (serde_json-style last-wins would hide it).
