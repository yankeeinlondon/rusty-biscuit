# Provenance: uv-0.9.5/workspace-edited-stale-extra

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `uv.lock` differs from `workspace`.

## Exact edit

Added `"gamma"` to `[manifest].members` (after `"fixture-root"`) and inserted a package table before
`hidden-tool`:

```toml
[[package]]
name = "gamma"
version = "0.1.0"
source = { editable = "packages/gamma" }
```

## Expected result

Lockfile records a member (`packages/gamma`, package `gamma`) that the manifests do not declare: stale lockfile, extra member.
