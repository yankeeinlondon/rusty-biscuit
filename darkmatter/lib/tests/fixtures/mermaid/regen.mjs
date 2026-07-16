#!/usr/bin/env node
// Re-vendor the pinned Mermaid ESM-min runtime used by the Browser-tier
// `browser_mermaid_real_engine_renders_and_themes` test. Build-time network
// only; the resulting fixtures make the test itself network-free (loopback).
//
// Usage:
//   node regen.mjs            # uses DEFAULT_VERSION below
//   node regen.mjs 11.6.0     # explicit version (must match MERMAID_VERSION)
//
// Vendors ONLY `dist/mermaid.esm.min.mjs` plus its complete relative import
// closure under `dist/chunks/mermaid.esm.min/`. That is exactly what the
// bootstrap's `import()` pulls in; source maps, type decls, and the non-min /
// UMD builds are never fetched at runtime and are intentionally skipped.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Keep in sync with `darkmatter::mermaid::MERMAID_VERSION`.
const DEFAULT_VERSION = "11.6.0";
const version = process.argv[2] || DEFAULT_VERSION;

const here = path.dirname(fileURLToPath(import.meta.url));
const outRoot = path.join(here, version, "dist");
const dataApi = `https://data.jsdelivr.com/v1/packages/npm/mermaid@${version}`;
const cdnBase = `https://cdn.jsdelivr.net/npm/mermaid@${version}`;

function flatten(nodes, prefix, out) {
  for (const n of nodes) {
    const p = `${prefix}/${n.name}`;
    if (n.type === "directory") flatten(n.files, p, out);
    else out.push(p);
  }
}

const listing = await (await fetch(dataApi)).json();
if (!listing.files) {
  throw new Error(`jsDelivr listing for mermaid@${version} returned no files`);
}
const all = [];
flatten(listing.files, "", all);

// The ESM-min entry and its `chunks/mermaid.esm.min/` closure only.
const want = all.filter(
  (p) =>
    p === "/dist/mermaid.esm.min.mjs" ||
    (p.startsWith("/dist/chunks/mermaid.esm.min/") &&
      p.endsWith(".mjs") &&
      !p.endsWith(".map")),
);

fs.rmSync(path.join(here, version, "dist"), { recursive: true, force: true });

let bytes = 0;
for (const rel of want) {
  const res = await fetch(cdnBase + rel);
  if (!res.ok) throw new Error(`fetch ${rel} -> HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  // rel starts with "/dist/"; strip that segment, we re-root under outRoot.
  const dest = path.join(outRoot, rel.slice("/dist/".length));
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, buf);
  bytes += buf.length;
}

// Repo root ignores `**/dist/`; re-include this fixture tree.
fs.writeFileSync(
  path.join(here, version, ".gitignore"),
  "# Vendored Mermaid runtime (test fixture, not build output).\n" +
    "# Repo root ignores **/dist/; re-include it here.\n" +
    "!dist/\n!dist/**\n",
);

console.log(
  `vendored mermaid@${version}: ${want.length} files, ` +
    `${(bytes / 1048576).toFixed(2)} MB -> ${path.relative(process.cwd(), outRoot)}`,
);
