# Provenance: bun-1.3.3/workspace-edited-comments-trailing-commas

Hand-edited from `workspace` (`bun-1.3.3/workspace`), date 2026-09-26. Only `bun.lock` was edited; manifests are identical to the real fixture.

## Edit

Valid-JSONC edit, membership unchanged: added a `//` comment before the opening `{`, a `/* */` comment before `lockfileVersion`, a `//` comment line before the `".tools/hidden"` key, an inline `/* */` comment after its `{`, a trailing comma inside the last `packages` array (`{},]`), a trailing comma after the `packages` object (`},`), a multi-line `/* */` comment before the final `}`, and a `//` comment after it. (Bun already writes trailing commas after every last object member.)

## Expected result

Parses as JSONC; membership identical to `workspace`: `.tools/hidden`, `packages/alpha`, `packages/beta`.
