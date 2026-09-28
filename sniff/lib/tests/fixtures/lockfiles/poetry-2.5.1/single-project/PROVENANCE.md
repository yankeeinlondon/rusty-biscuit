# Provenance: poetry-2.5.1/single-project

- Tool: Poetry (version 2.5.1) via `uvx poetry`
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write pyproject.toml ([project] fixture-root, [tool.poetry] package-mode = false,
#   [tool.poetry.dependencies] local-lib = { path = "local-lib" }) and local-lib/pyproject.toml (hatchling)
uvx poetry lock --no-interaction
```

## Trimmed

- `local-lib/local_lib/__init__.py` (a stub not needed by the lock). Poetry's virtualenv was created in `~/Library/Caches/pypoetry/virtualenvs`, outside the project.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- none (Poetry has no workspace members)
- Recorded by the lockfile:
- `[metadata].lock-version = "2.1"`; `local-lib` appears as `[[package]]` with `[package.source] type = "directory"`, `url = "local-lib"`. No membership data.
