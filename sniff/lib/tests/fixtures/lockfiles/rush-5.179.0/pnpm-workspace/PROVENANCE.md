# PROVENANCE

- Tool: Rush 5.179.0 (`@microsoft/rush`, latest 5.x; run as `npx -y @microsoft/rush@5.179.0`)
- Package manager installed by Rush: pnpm 9.15.9 (`rush.json` `pnpmVersion`, the value `rush init` wrote)
- Lockfile: `common/config/rush/pnpm-lock.yaml`, `lockfileVersion: '9.0'`
- Host: macOS 27.2 (build 26B5091g), `Darwin 27.2.0 arm64` (Apple Silicon); Node.js v24.21.0 (the `rush init` template requires `nodeSupportedVersionRange: ">=24.11.1 <25.0.0"`; `rush init` itself ran under v22.20.0)
- Date: 2026-09-26
- Generated in scratch `/tmp/lockfile-fixtures/rush-5.179.0/pnpm-workspace/`, a throwaway Git repository outside any real repository

## Commands (in order)

1. `git init -q -b main .`
2. `npx -y @microsoft/rush@5.179.0 init`
3. Edited `rush.json`: replaced the comment-only `projects` array (template lines 338 to the end) with three projects: `alpha` at `packages/alpha`, `@fixture/beta` at `packages/beta`, `hidden-tool` at `.tools/hidden`. Nothing else in `rush.json` was changed.
4. Wrote `packages/alpha/package.json` (`alpha`, depends on `@fixture/beta` at `workspace:*`), `packages/beta/package.json` (`@fixture/beta`), `.tools/hidden/package.json` (`hidden-tool`).
5. `git add -A` and a local, unsigned commit in the scratch repository only
6. `CI=1 npx -y @microsoft/rush@5.179.0 update` with Node.js v24.21.0 first on `PATH`

Rush has no `local-lib` equivalent here; no project has a `file:`/`link:` dependency.

## Settings in effect

- `common/config/rush/pnpm-config.json`: `"useWorkspaces": true` (template default)
- `common/config/rush/subspaces.json`: `"subspacesEnabled": false`, `"subspaceNames": []` (template default), so there is one lockfile at `common/config/rush/pnpm-lock.yaml`

## Importer keys, exactly as written

```
.
../../.tools/hidden
../../packages/alpha
../../packages/beta
```

Base directory: `common/temp`. Confirmed three ways: Rush ran `pnpm install` in `common/temp` and then copied `common/temp/pnpm-lock.yaml` to `common/config/rush/pnpm-lock.yaml`; the generated `common/temp/pnpm-workspace.yaml` lists the same three `../../…` paths; and the `.` importer (`{}`) is the synthetic `common/temp` root, not the repository root. Resolve keys against `<repo>/common/temp`, NOT against the directory holding the lockfile (`common/config/rush`), where `../../packages/alpha` would wrongly resolve to `common/packages/alpha`.

Dependency links inside an importer are relative to that importer, not to `common/temp`: `../../packages/alpha` records `@fixture/beta` as `version: link:../beta`.

## Trimmed

- `.git/` (scratch repository)
- `common/temp/` (install workspace, including `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `node_modules/`, `pnpm-local` symlink)
- `common/scripts/` (`install-run*.js` bootstrap scripts)
- `common/git-hooks/`, `.github/`, `.gitattributes`, `.gitignore`
- From `common/config/rush/`: `.npmrc`, `.npmrc-publish`, `artifactory.json`, `build-cache.json`, `cobuild.json`, `command-line.json`, `common-versions.json`, `custom-tips.json`, `experiments.json`, `rush-plugins.json`, `version-policies.json` (comment-only templates)
- Kept in `common/config/rush/`: `pnpm-lock.yaml`, `pnpm-config.json`, `subspaces.json`, `repo-state.json` (written by `rush update`), `.pnpmfile.cjs` (the lockfile's `pnpmfileChecksum` covers it)

## Expected membership

Manifest-declared members (`rush.json` `projects[].projectFolder`, relative to the repository root):

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`

Lockfile members (`importers` keys, excluding `.`, resolved against `common/temp` and re-expressed relative to the repository root):

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`
