# Provenance: uv-0.9.5/single-project

- Tool: uv 0.9.5 (d5f39331a 2025-10-21)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write pyproject.toml ([project] fixture-root, no [tool.uv.workspace])
uv lock
```

## Trimmed

- Nothing.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- none (not a workspace)
- Recorded by the lockfile:
- no `[manifest]` table at all; the only package is `fixture-root` with `source = { virtual = "." }`

## Notes

A lock with no `[manifest]` table is valid: uv omits it when the project is not a multi-member workspace.
