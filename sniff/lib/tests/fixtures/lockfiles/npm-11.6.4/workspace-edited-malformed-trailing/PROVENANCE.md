# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Appended the text `\n}}} not-valid\n` after the closing brace of the otherwise unchanged document.

## Expected result

The lockfile fails to parse as JSON (trailing characters); the parser must report a malformed lockfile rather than any membership result.
