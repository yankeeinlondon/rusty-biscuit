# Provenance: yarn-4.18.1/workspace

- Tool: Yarn Berry 4.18.1 (latest 4.x on npm on 2026-09-26), run through corepack 0.x bundled with Node v22.20.0
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 Darwin Kernel Version 27.2.0: Sun Sep 13 19:46:18 PDT 2026; root:xnu-13432.40.162~92/RELEASE_ARM64_T6041 arm64); Node v22.20.0
- Date: 2026-09-26
- Generated in `/tmp/lockfile-fixtures/yarn-4.18.1/workspace/`, then copied here.

## Setup

- `package.json` (name `fixture-root`, `packageManager: yarn@4.18.1`) declares `workspaces: ["packages/*", ".tools/hidden"]`
  and depends on `local-lib` via `link:./local-lib` and `portal-lib` via `portal:./portal-lib`. Neither is a member.
- `packages/alpha` depends on `@fixture/beta` with `workspace:^`.
- `.yarnrc.yml`: `nodeLinker: node-modules` (so no `.pnp.cjs`), `enableTelemetry: false`.

## Commands (in order)

1. Wrote the manifests and `.yarnrc.yml` by hand.
2. `COREPACK_ENABLE_DOWNLOAD_PROMPT=0 CI=1 corepack yarn@4.18.1 install` with an empty `yarn.lock`: failed with
   YN0028 ("The lockfile would have been modified by this install, which is explicitly forbidden") because `CI=1`
   makes installs immutable. Nothing was written.
3. `rm -f yarn.lock`
4. `COREPACK_ENABLE_DOWNLOAD_PROMPT=0 YARN_ENABLE_IMMUTABLE_INSTALLS=false corepack yarn@4.18.1 install </dev/null` -> succeeded.

## Trimmed

- `node_modules/`
- `.yarn/` (contained only `install-state.gz`)

## Expected membership

Declared by the manifest (root excluded, sorted):

```
.tools/hidden
packages/alpha
packages/beta
```

Recorded by the lockfile (root excluded, sorted):

```
.tools/hidden
packages/alpha
packages/beta
```

Membership is not a dedicated section: every top-level entry whose `resolution` is `<name>@workspace:<path>`
is a workspace, and `fixture-root@workspace:.` is the root. `local-lib` (`link:`) and `portal-lib` (`portal:`)
also have `linkType: soft` but `languageName: node` and a non-`workspace:` resolution; they are not members.
