# Provenance: cargo-1.98.1/workspace-edited-malformed-trailing

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `Cargo.lock` differs from `workspace`.

## Exact edit

Appended the line `}}} not-valid` after the final newline.

## Expected result

TOML parse error: the lockfile is malformed.
