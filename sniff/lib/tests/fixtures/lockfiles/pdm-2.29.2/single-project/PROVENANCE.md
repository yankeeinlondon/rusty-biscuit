# Provenance: pdm-2.29.2/single-project

- Tool: PDM, version 2.29.2 via `uvx pdm`
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write pyproject.toml ([project] fixture-root, dependencies = ["local-lib @ file:///${PROJECT_ROOT}/local-lib"],
#   [tool.pdm] distribution = false) and local-lib/pyproject.toml (hatchling)
uvx pdm lock
```

## Trimmed

- `.venv/` and `.pdm-python` (PDM created a virtualenv during `lock`), and `local-lib/local_lib/__init__.py`.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- none (PDM has no workspace members)
- Recorded by the lockfile:
- `[metadata].lock_version = "4.5.1"`; `local-lib` appears as `[[package]]` with `path = "./local-lib"`. No membership data.
