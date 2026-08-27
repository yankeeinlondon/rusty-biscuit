---
name: biscuit-icon
description: >
  Expert knowledge for the biscuit-icon Rust library and `icon` CLI — curated
  offline domain icons plus on-demand Iconify (https://iconify.design/) lookup,
  cached in a local SQLite database. Use when working in the `biscuit-icon/`
  package area, using `biscuit_icon::Icon` or any of the domain enums
  (`Os`, `Emoji`, `File`, `Brand`, ...), assembling styled `<svg>` from an
  `IconBody`, calling `Icon::iconify` for network/cache lookup, rendering
  icons through the `renderable` tree (terminal ladder, browser inline SVG,
  markdown), or using the `icon` CLI for listing sets/icons and dynamic
  shell completions.
---

# biscuit-icon

`biscuit-icon` combines a compile-time, offline catalog of curated **domain
icons** with an on-demand **network lookup** of any of the 200,000+ [Iconify]
icons (cached locally in SQLite). Rendering is delivered through the shared
`renderable` multi-target tree (terminal, browser/HTML, markdown).

The two crates:

- **`biscuit-icon`** (library) — the `Icon` handle, domain enums, the Iconify
  client, the SQLite cache, and the `renderable` integration.
- **`biscuit-icon-cli`** (binary `icon`) — a small CLI for listing sets/icons,
  rendering icons, providing dynamic shell completions, and clearing the cache.

[Iconify]: https://iconify.design/

## Package Structure

| Crate                  | Path                | Binary | Purpose                              |
|------------------------|---------------------|--------|--------------------------------------|
| `biscuit-icon`         | `biscuit-icon/lib`  | —      | Library: `Icon`, domain enums, cache |
| `biscuit-icon-cli`     | `biscuit-icon/cli`  | `icon` | CLI: list, render, completions, cache maintenance |
| Vendored bodies        | `biscuit-icon/assets/icons/` | — | Reviewed, committed SVG bodies for the curated subset |

Canonical recipes (run from the package area root):

```bash
just build           # build both crates
just test            # L1 tests
just test-l2         # CLI integration tests (uses `--features image`)
just lint            # clippy
just doctest         # cargo doc --no-deps
just install         # build --release + install `icon` to ~/.cargo/bin
just cli -- mdi:home # run the CLI from source (`cargo run -p biscuit-icon-cli --`)
just docs            # cargo doc -p biscuit-icon --open
just populate-assets # dev-only: refresh curated domain bodies from Iconify
```

## Start Here

- For **listing/looking up icons in a terminal or script**, use the `icon` CLI
  — see [cli.md](cli.md).
- For **embedding icons in a Rust app** (browser/markdown/terminal output),
  see [library.md](library.md) for the `Icon` builder, the domain enums, and
  the network/cache lookup API.
- For **styling options** (color, size, flip, rotate, view-box), the local
  `<svg>` assembly, and the `Flip` / `Rotate` enums, see
  [library.md § Style](library.md#style).
- For **render-tree projection** (browser inline SVG, terminal degradation
  ladder, markdown), see [rendering.md](rendering.md).
- For the **local SQLite cache** (`~/.cache/biscuit-icon/icons.db`),
  schema, and migrations, see [cache.md](cache.md).
- For the **curated domain icon subset** (16 enums, ~150 variants, with
  Unicode + Nerd Font codepoints for a subset), see [domain-icons.md](domain-icons.md).
- For the **Iconify HTTP API surface** used by the library (endpoints,
  query params, identifier syntax), see [iconify.md](iconify.md).

## Responsibility Split

| Need                                                    | Owner        |
|---------------------------------------------------------|--------------|
| Curated offline domain icons (enum-first API)           | `biscuit-icon` |
| On-demand Iconify network lookup                        | `biscuit-icon::iconify` |
| Local SQLite cache of bodies + set metadata            | `biscuit-icon::cache` |
| Styled `<svg>` assembly (color, flip, rotate, view-box) | `biscuit-icon::style` |
| Browser / markdown / terminal render-tree integration   | `biscuit-icon` (via `renderable` + `biscuit-terminal`) |
| Terminal ladder (Nerd Font → Unicode → image → text)    | `biscuit-icon` (via `biscuit-terminal`) |
| `icon show`, `icon sets`, `icon domain`, `icon cache list`/`cache clear`, `icon completions` | `biscuit-icon-cli` |
| `mdi:home` URL routing on icon-sets.iconify.design      | Iconify      |

## Core Public Types (Library)

```rust
use biscuit_icon::{Icon, Source, IconError, IconBody, Glyph, Style, Flip, Rotate};
use biscuit_icon::domain::{Os, Emoji, File, Brand, /* ... */};
```

- `Icon` — the central handle (body + style + glyph + source + id). Use the
  builder methods (`.color()`, `.width()`, `.flip()`, `.rotate()`, ...) and
  finish with `.svg()` to get the assembled `<svg>`. Implements
  `biscuit_terminal::TerminalRenderable` and `renderable::TreeRenderable`.
- `Source` — `Embedded` (compiled-in domain icon) vs `Network` (fetched /
  cache-resident Iconify icon).
- `IconBody` — raw `<path>` body + intrinsic width / height + view-box
  origin (`left`, `top`); supports non-zero origins preserved through
  cache round-trips.
- `Style` — accumulated presentation options; all default to "unset" so
  unset options mean "let the SVG default apply".
- `Flip`, `Rotate` — typed enums for the corresponding `Style` options.
- `Glyph` — optional `unicode` char and optional Nerd Font `nerd_font` char.
- `IconError` — `UnknownDomainIcon`, `InvalidIdentifier`, `NotFound`,
  `Fetch`, `Cache`.

## Quick Examples

### Library — embed an icon

```rust
use biscuit_icon::Icon;
use biscuit_icon::domain::{DomainIcon, Os};

let finder = Os::Finder.icon(); // enum-first, infallible, compiled in

// String convenience (fallible; IconError::UnknownDomainIcon on miss)
let happy = Icon::emoji("happy")?;

// Network/cache lookup
let home = Icon::iconify("mdi:home").await?;

// Style and emit
let svg = finder.color("#d97706").width("24").height("24").svg();
```

### Library — render through the tree

```rust
use biscuit_icon::Icon;
use biscuit_icon::domain::DomainIcon;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();
let out = biscuit_icon::domain::Emoji::Happy.icon().render(&term);
// → the Unicode grinning-face glyph in a TTY, the id in non-TTY
```

### CLI — list, search, complete

```bash
# list every Iconify set (offline + online), as a striped 4-column table
icon sets

# list icons matching "home" (offline catalog ∪ online search, deduplicated)
icon home

# limit to specific sets
icon --from mdi,lucide home

# direct lookup & render
icon mdi:home
icon --nerd mdi:github

# clear the local SQLite cache
icon cache clear
```

## Iconify at a Glance

- The full docs root is <https://iconify.design/docs>; the queries reference
  is at <https://iconify.design/docs/api/queries.html>.
- Public API host: <https://api.iconify.design> (over 275k icons across 200+
  open-source sets). The library targets this host by default; tests inject
  a custom base via `IconifyClient::with_base`.
- A browseable alternative is <https://icones.js.org/> (by Anthony Fu).
- Browse per-set galleries at `https://icon-sets.iconify.design/{prefix}/`;
  the set directory is at <https://icon-sets.iconify.design> and supports
  filters for `?palette=`, `?commercial=`, `?attribution=`, `?grid=`,
  `?category=`, and `?tag=`.
- Icon sets expose rich **metadata** — name, author, license, category, tags,
  palette flag — and icons within sets expose **categories**, **themes**
  (prefix/suffix variations), and optional **character maps** (for icon-font
  imports). See [iconify.md § Iconify Platform Metadata](iconify.md#iconify-platform-metadata).

For endpoint-by-endpoint coverage (the three endpoints the library uses,
plus their query parameters and response shapes), see [iconify.md](iconify.md).

## Tech Stack

### Library

- `renderable` (path) — multi-target render tree (`TreeRenderable`).
- `biscuit-terminal` (path) — terminal rendering and the
  `TerminalRenderable` ladder (glyph → image → text).
- `biscuit-visualized` (path, gated by the `image` cargo feature) — `resvg`
  rasterization for image-protocol terminals.
- `rusqlite` (bundled) — local SQLite cache.
- `reqwest` 0.12 — async Iconify HTTP client.
- `tokio` — async runtime; `spawn_blocking` for synchronous SQLite I/O.
- `strum` / `strum_macros` — enum ↔ string for the domain enums.
- `serde` / `serde_json` — Iconify API payloads and `IconBody` (de)serialization.
- `thiserror` — `IconError`.
- `dirs` — default cache directory resolution.
- `tracing` — structured diagnostics.

### CLI

- `clap` (derive / env / unstable-ext) — argument parsing.
- `clap_complete` (unstable-dynamic) — dynamic value completion + AOT
  script generation for `icon completions <shell>`.
- `color-eyre` — error reporting in `main`.
- `tracing-subscriber` — `--debug` / `RUST_LOG` on stderr.

### Cargo features

| Feature | Default | Effect |
|---------|---------|--------|
| `image` (library + CLI) | off | Pulls in `biscuit-visualized`/`resvg`; enables image-protocol terminal rendering of glyph-less icons. When off, the terminal ladder stops at the text identifier fallback. |

## Testing

Follows the monorepo L1/L2/L3 taxonomy.

- L1 (library): domain / style / SVG assembly, `strum` round-trips, the
  string-convenience constructors, the Iconify client (via `wiremock` for
  HTTP contract), the SQLite cache (via `tempfile`).
- L2 (CLI): real-terminal rendering behind the internal `terminal-tests`
  feature. `just test-l2` enables it with `image`; local L1 omits the target.
- L3: not applicable.

## Open Files in This Skill

| Topic | File |
|-------|------|
| Library API (`Icon`, `Style`, `Source`, builders, error type) | [library.md](library.md) |
| `icon` CLI (subcommands, flags, env vars, completion behavior) | [cli.md](cli.md) |
| Iconify HTTP API surface used by the library (endpoints, queries, identifiers) | [iconify.md](iconify.md) |
| Render-tree integration (terminal ladder, browser, markdown) | [rendering.md](rendering.md) |
| SQLite cache (location, schema, migrations) | [cache.md](cache.md) |
| Curated domain icon subset (16 enums, ~150 variants) | [domain-icons.md](domain-icons.md) |
