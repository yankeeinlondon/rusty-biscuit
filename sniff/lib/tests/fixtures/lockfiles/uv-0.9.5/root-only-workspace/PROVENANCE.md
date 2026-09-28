# Provenance: `uv-0.9.5/root-only-workspace`

- **Tool:** uv 0.9.5 (d5f39331a 2025-10-21)
- **Host:** macOS 27.2 (arm64)
- **Date:** 2026-09-26
- **Generated in:** `/tmp/uv-s3/a`, outside the repository

## Commands

```sh
# pyproject.toml: [project] fixture-root 0.1.0 plus [tool.uv.workspace] members = []
uv lock -q
```

## Trimmed

Nothing: `uv lock` creates no `.venv`. No file content was edited.

## Expected membership

- **Manifest members** (root excluded): none. The workspace's only member is the root.
- **Lockfile records:** no `[manifest]` table. uv omits `[manifest]` when the root
  is the only workspace member; the root package appears as `source = { virtual = "." }`.
- **Meaning for Sniff:** an absent `[manifest]` means "the locked member set is the root
  alone", which is empty once the root is excluded. Both sets are empty and were established
  from valid, complete records, so this is a `match`.
- **Lockfile version:** `version = 1`, `revision = 3`
