# Provenance: cargo-1.98.1/workspace-edited-missing-required-field

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `Cargo.lock` differs from `workspace`.

## Exact edit

Deleted every `[[package]]` table; only the two header comments and `version = 4` remain.

## Expected result

The package array (Cargo's only membership evidence) is absent while the manifest declares members: malformed or unusable lockfile.
