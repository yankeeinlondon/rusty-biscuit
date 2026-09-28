---
area: sniff
status: unscheduled
created: 2026-09-26
owner: Ken Snyder <ken@ken.net>
origin: Phase 5 passive corpus pass of 2026-09-26-lockfile-corroboration
related:
    - 2026-09-26-lockfile-corroboration
packages:
    - sniff
---

# `package.json` workspace patterns starting with `./` match no members

## Problem

A `package.json` whose `workspaces` entries start with `./` (for example
`"./pk/*"`) resolves to no members. The same tree with `"pk/*"` resolves both
members. npm, Yarn, and Bun all accept the `./` spelling.

Reproduction on a disposable directory:

```sh
mkdir -p pk/a pk/b
echo '{"name":"r","private":true,"workspaces":["./pk/*"]}' > package.json
echo '{"name":"a","version":"1.0.0"}' > pk/a/package.json
echo '{"name":"b","version":"1.0.0"}' > pk/b/package.json
sniff --base . repo structure --json   # is_monorepo: false, layer packages: []
```

With `"pk/*"`, the same command reports `is_monorepo: true` and the layer
packages `pk/a` and `pk/b`.

The corpus pass found this on a real checkout (`typescript-go`, which declares
`"./_packages/*"`). Its npm layer reports lockfile `mismatch` with both
packages as `extra`. The lockfile is right; the manifest side is empty.

## Why this is separate

`2026-09-26-lockfile-corroboration` compares the lockfile with the members
manifest discovery already found. It does not change discovery. The mismatch
it reports here is truthful given what discovery produced.

## Scope

- Normalize a leading `./` (and `.` components generally) in `package.json`
  workspace patterns before glob expansion. Check whether the pnpm, Cargo,
  and uv glob paths share the same gap.
- Add a regression test with the exact pattern above, through the public
  detection API, asserting the layer members and the lockfile observation.
