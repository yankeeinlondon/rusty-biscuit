# Vendored Mermaid distribution (Browser-tier test fixture)

These files are the **exact** minified ESM runtime of Mermaid, pinned to the
version in `darkmatter::mermaid::MERMAID_VERSION` (`11.6.0`). They exist so the
Browser-tier test `browser_mermaid_real_engine_renders_and_themes`
(`darkmatter/lib/tests/browser_render.rs`) can run the **real** pinned engine
against a **loopback-only** static server — proving Mermaid 11.6.0 actually
exports the API shape the bootstrap calls, accepts the resolver's
`themeVariables`, parses the diagram, and produces a themed SVG. The handwritten
stub tests in the same file cannot prove any of that.

The test **never touches the network at runtime**: it serves these files from a
`127.0.0.1:0` server and redirects the two CDN specifiers via an import map.

## Layout

```
mermaid/
  README.md          <- this file
  regen.mjs          <- reproducible re-vendoring script (build-time network)
  11.6.0/
    .gitignore       <- re-includes dist/ (the repo root ignores **/dist/)
    dist/
      mermaid.esm.min.mjs                    <- the ESM entry the bootstrap imports
      chunks/mermaid.esm.min/*.mjs           <- its complete import closure
```

Only the **ESM-min entry** (`dist/mermaid.esm.min.mjs`) and its relative chunk
imports (`dist/chunks/mermaid.esm.min/*.mjs`) are vendored — that is the exact,
self-contained closure the bootstrap's `import()` pulls in. Source maps
(`*.map`, ~43 MB), type declarations (`*.d.ts`), the non-minified chunk tree,
and the UMD/`.core` builds are **not** vendored: none are fetched at runtime.

- Files: 67
- Size: ~2.5 MB

Size trade-off: committing ~2.5 MB keeps the test reproducible for every
checkout with zero runtime network. Vendoring the whole upstream `dist/` tree
would be ~61 MB (mostly source maps that never load), which is not worth the
repo weight; the entry's import closure is complete on its own (verified: zero
missing relative imports).

## Regenerating

Requires network. Run from anywhere with Node 18+ (uses global `fetch`):

```sh
node darkmatter/lib/tests/fixtures/mermaid/regen.mjs
```

To bump the pinned version, first update `MERMAID_VERSION` in
`darkmatter/lib/src/mermaid/feature.rs`, then run:

```sh
node darkmatter/lib/tests/fixtures/mermaid/regen.mjs <new-version>
```

The script enumerates the package's `dist/` tree via the jsDelivr data API,
keeps only the ESM-min entry plus its `chunks/mermaid.esm.min/` closure, and
writes them under `<version>/dist/…`. If the version changes, delete the old
`<version>/` directory by hand.
