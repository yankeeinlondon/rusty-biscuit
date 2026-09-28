# Provenance: uv-0.9.5/workspace

- Tool: uv 0.9.5 (d5f39331a 2025-10-21)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
mkdir -p packages/alpha packages/beta .tools/hidden local-lib
# write pyproject.toml files (root fixture-root, alpha, beta, hidden-tool, local-lib)
uv lock
```

## Trimmed

- Nothing. `uv lock` created no `.venv`. The CPython interpreter used was /opt/homebrew/opt/python@3.12 (3.12.14).

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `.tools/hidden`
  - `packages/alpha`
  - `packages/beta`
- Recorded by the lockfile:
- `[manifest].members` = `["alpha", "beta", "fixture-root", "hidden-tool"]` (package NAMES, sorted, root included)
  - member paths appear only as `[[package]].source = { editable = "<path>" }`: `.tools/hidden`, `packages/alpha`, `packages/beta`; root is `source = { virtual = "." }`
  - `local-lib` (non-member path source) is `source = { directory = "local-lib" }` and is absent from `[manifest].members`

## Notes

Root has `[project]` but no `[build-system]`, so uv records it as `virtual = "."`. Members with a build
backend are `editable`. The version field is top-level `version = 1` with `revision = 3`.
