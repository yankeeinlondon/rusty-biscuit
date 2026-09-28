# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Duplicated the `"packages/beta"` key inside the `packages` object (a second identical entry directly after the first).

## Expected result

JSON (RFC 8259) says object names SHOULD be unique and most parsers (including serde_json into a map) silently keep the last value. The parser must detect the duplicate member key and report it rather than silently collapsing it.
