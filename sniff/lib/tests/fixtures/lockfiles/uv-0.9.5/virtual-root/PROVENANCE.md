# Provenance: uv-0.9.5/virtual-root

- Tool: uv 0.9.5 (d5f39331a 2025-10-21)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write root pyproject.toml with only [tool.uv.workspace] members = ["packages/alpha", "packages/beta"]
# write packages/{alpha,beta}/pyproject.toml
uv lock
```

## Trimmed

- Nothing.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `packages/alpha`
  - `packages/beta`
- Recorded by the lockfile:
- `[manifest].members` = `["alpha", "beta"]` (no root entry; the root has no `[project]` and no `[[package]]`)
  - `[[package]]` alpha/beta with `source = { editable = "packages/alpha" | "packages/beta" }`
