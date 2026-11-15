---
spike: S14b
spec: 2026-11-06-publish-more-channels
question: Are the planned names still free on crates.io, PyPI, and npm under `@yankeeinlondon`?
answer: yes — all ten names are unregistered on crates.io, PyPI, and under @yankeeinlondon on npm as of 2026-11-07 16:01 UTC
confidence: high (for "unregistered now"); not determined for PyPI name-similarity rejection at upload time
date: 2026-11-07
---

## Answer

Every name checked is free on all three registries today. Nothing was
reserved; names can be taken at any time until each first publish.

| Name | crates.io | PyPI | npm `@yankeeinlondon/<name>` |
|---|---|---|---|
| darkmatter-cli | free | free | free |
| darkmatter-lsp | free | free | free |
| sniff-detection-cli | free | free | free |
| git-wt-cli | free | free | free |
| tree-hugger-cli | free | free | free |
| claudine-cli | free | free | free |
| biscuit-file-cli | free | free | free |
| biscuit-hash-cli | free | free | free |
| biscuit-hash (library) | free | free | free |
| biscuit-file (library) | free | free | free |

The `@yankeeinlondon` npm scope exists and already holds packages
(`@yankeeinlondon/happy-wrapper`, `…/gray-matter`, `…/ask`, and others), so the
scope is owned and usable; whether the publishing token belongs to an account
with write access to it was not checked (no npm login on this host).

## Evidence

Read-only HTTP GETs, 2026-11-07 16:01 UTC. A 404 means "no such package".

```text
crates.io : GET https://crates.io/api/v1/crates/<name>      (with a User-Agent)
PyPI      : GET https://pypi.org/pypi/<name>/json
npm       : GET https://registry.npmjs.org/@yankeeinlondon%2f<name>
```

All thirty lookups returned `404`.

Controls, to show a 404 is not a broken query: crates.io `serde`,
`darkmatter`, `sniff` → 200; PyPI `requests` → 200; npm `@types/node` → 200.
crates.io treats `-` and `_` as the same name; `biscuit_hash` → 404 too.

npm scope ownership: `GET https://registry.npmjs.org/-/org/yankeeinlondon/package`
returned a package map (`"@yankeeinlondon/happy-wrapper":"write"`, …), so the
scope is an existing org/user scope. The npm website pages
(`/~yankeeinlondon`, `/org/yankeeinlondon`) returned 403 to curl and were not
used.

Side observation, unscoped npm (not asked, for context only):
`darkmatter-cli`, `tree-hugger-cli`, `claudine-cli` → 404; `sniff-cli`,
`biscuit-cli`, `wt-cli` → 200 (taken). This is why the scope matters.

## What this changes in the spec

- **Spikes needed → "crates.io name availability"**: settled for today. The
  names are free; the risk that they are taken before the first publish
  remains, and only a first publish (the spec's manual, confirmed step)
  removes it.
- **Outcome table**: `npm i -g @yankeeinlondon/darkmatter-cli` and
  `uv tool install darkmatter-cli` are both available as written.

## Surprises

- crates.io already has unrelated crates named `darkmatter` and `sniff`
  (matching `release-plz.toml`'s note). The `-cli`/`-lsp` names chosen here
  avoid them, but a crates.io publish of the *libraries* `darkmatter` or
  `sniff` is impossible under those names. Only `biscuit-hash` and
  `biscuit-file` were asked about and are free.
- Not determined: PyPI can reject an upload whose normalized name is
  "too similar" to an existing project. A JSON-API 404 does not rule that
  out; only an upload (or TestPyPI) would.
