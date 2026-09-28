# Provenance: composer-2.10.3/single-project

- Tool: Composer version 2.10.3 2026-08-27 13:34:23
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write composer.json (name fixture/fixture-root, "require": {})
COMPOSER_NO_INTERACTION=1 composer update --no-install --no-interaction
```

## Trimmed

- Nothing (`--no-install` created no `vendor/`).

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- none (Composer has no workspace members)
- Recorded by the lockfile:
- JSON with `packages: []`, `packages-dev: []`; no lockfile format version key at all (closest are
    `content-hash` and `plugin-api-version` = "2.9.0", which is the Composer plugin API, not a format
    version). A lock is written even with zero dependencies.
