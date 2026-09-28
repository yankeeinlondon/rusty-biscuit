# Provenance: uv-0.9.5/workspace-edited-malformed-trailing

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `uv.lock` differs from `workspace`.

## Exact edit

Appended the line `}}} not-valid` after the final newline.

## Expected result

TOML parse error: the lockfile is malformed.
