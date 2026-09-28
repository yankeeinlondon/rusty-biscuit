# Provenance: cargo-1.98.1/workspace-edited-stale-extra

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `Cargo.lock` differs from `workspace`.

## Exact edit

Inserted before `hidden-tool`:

```toml
[[package]]
name = "gamma"
version = "0.1.0"
```

## Expected result

Lockfile records a sourceless path package `gamma` that no manifest declares (standing in for
`packages/gamma`; Cargo.lock has no paths). Expected: stale lockfile, extra path package.
