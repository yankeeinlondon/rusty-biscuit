---
title: biscuit-icon — Library & CLI Design Spec
status: draft
last_updated: 2026-06-07
---

# biscuit-icon — Design Spec

`biscuit-icon` provides SVG (and, for a curated subset, character-based) icons to
the rusty-biscuit monorepo. It combines a compile-time, offline catalog of curated
**domain icons** with an on-demand **network lookup** of any of the 200,000+
[Iconify](https://iconify.design/) icons, cached locally. Rendering is delivered
through the shared `renderable` multi-target tree (terminal, browser/HTML, markdown).

This spec covers the full vision. Implementation is sequenced into three phases
(see [Phasing](#phasing)); each phase is independently shippable and testable.

## Goals

- A small, always-available, **offline** catalog of curated domain icons embedded
  in the binary, accessed type-safely via enums.
- On-demand retrieval of any Iconify icon at runtime, cached to a local SQLite DB
  so repeat lookups and completions stay offline.
- Uniform local styling (color, size, rotate, flip, view box) for both embedded
  and network-fetched icons — no per-style network round-trips.
- First-class rendering through the `renderable` tree with a terminal degradation
  ladder and an inline-SVG browser target.
- A `icon` CLI for listing sets/icons and providing dynamic shell completions.

## Non-Goals

- No compile-time network fetching in normal library builds (reproducible builds
  are a hard requirement; the `iconify` crate is a **dev-only** asset tool).
- No SVG sanitization of arbitrary user input (embedded/fetched bodies are trusted
  build/curated input; raw-SVG insertion is the caller's trust boundary).
- No accessibility inference (decorative vs meaningful is the caller's concern).
- No icon-set license management beyond preserving attribution metadata.

## Key Design Decisions

| # | Decision | Choice |
|---|----------|--------|
| 1 | Domain icon embedding | **Vendor SVGs, embed offline.** A dev-only tool (the `iconify` crate / Iconify API) fetches curated icon bodies once into committed, reviewed assets. Normal builds never touch the network. |
| 2 | Rendering boundary | **Implement the `renderable` tree.** `Icon` composes into the shared multi-target IR; terminal degrades glyph → (optional) image → text; browser emits inline SVG. |
| 3 | Cache backend | **SQLite via `rusqlite` (bundled).** Single file under the user cache dir; indexed for completion/listing queries. |
| 4 | Styling model | **Local SVG assembly from a stored icon body.** Store the Iconify body + viewBox/dimensions; assemble the styled `<svg>` locally. No API round-trip; uniform for embedded and cached icons. |
| 5 | Scope/phasing | **One spec, phased plan** (offline library → network cache → CLI). |
| 6 | Domain API shape | **Enum-first, with a string convenience layer.** Enums are canonical/compile-time-safe; fallible string lookup is offered on top via `strum` `FromStr`. |

### Resolved defaults

- **Cache expiry:** none. Cached icons persist until an explicit `icon cache clear`.
  Refreshes are deliberate, matching the reproducibility stance.
- **Terminal render order:** glyph-first (Nerd Font → Unicode → image → text).
- **`resvg` image rendering:** gated behind an optional `image` cargo feature
  (off by default). Default terminal ladder is glyph → text; with `image` enabled
  it becomes glyph → image → text.
- **Offline listings:** `sets`/`icons` list from the built-in domain catalog plus
  whatever is in the local cache when offline; the full Iconify catalog is reached
  via the API only when online (and is cached thereafter).

## Package Layout

```
biscuit-icon/
├── lib/            crate: biscuit-icon          (library)
├── cli/            crate: biscuit-icon-cli      (binary: icon)
├── assets/icons/   committed SVG bodies (vendored, reviewed)
├── docs/
├── justfile        (shared recipes from /just)
└── README.md
```

Both crates are added to the root `Cargo.toml` workspace members. Edition 2024.

### Dependencies

**Library (`biscuit-icon`):**

- `serde` + `serde_json` — Iconify API payloads, cache rows.
- `thiserror` — `IconError`.
- `strum` + `strum_macros` — enum ↔ string, iteration.
- `rusqlite` (`features = ["bundled"]`) — local cache.
- `reqwest` (0.12) — async Iconify client.
- `tokio` — async runtime for the network path.
- `dirs` (v6) — locate the user cache directory.
- `tracing` — structured logging.
- `renderable` — multi-target render tree.
- `biscuit-terminal` — terminal capability detection + terminal rendering.
- `resvg` — **optional**, behind the `image` feature (SVG → raster for image-protocol terminals).

**CLI (`biscuit-icon-cli`, binary `icon`):**

- `clap` (`features = ["derive", "env", "unstable-ext"]`).
- `clap_complete` (`features = ["unstable-dynamic"]`).
- `color-eyre` — CLI error reporting.
- `tracing-subscriber` — `--debug`/`RUST_LOG`.
- `biscuit-terminal`, `renderable` — styled output.

**Dev / asset tooling only (not a normal dependency):**

- `iconify` crate — used by the offline asset-population recipe to fetch curated
  icon bodies. Never compiled into the shipped library or CLI.

### Cargo features (library)

| Feature | Default | Effect |
|---------|---------|--------|
| `image` | off | Pulls in `resvg`; enables image-protocol terminal rendering of glyph-less icons. |

## Core Types & Public API

The central handle is `Icon`. Domain icons are **enum-first** with a string
convenience layer.

```rust
// Enum-first, compile-time safe (canonical)
let finder: Icon = Os::Finder.icon();          // or Icon::from(Os::Finder)

// String convenience (fallible; strum FromStr under the hood)
let happy: Icon = Icon::emoji("happy")?;       // Result<Icon, IconError>

// Network/cache lookup (async)
let home: Icon = Icon::iconify("mdi:home").await?;
```

### Domain enums

One enum per curated set, each deriving `strum::{EnumString, EnumIter, Display}`
and `Debug, Clone, Copy, PartialEq, Eq, Hash`:

`Os`, `Emoji`, `Arrow`, `Data`, `File`, `Hardware`, `Timing`, `Button`, `Control`,
`Network`, `DevOps`, `Actors`, `Nav`, `Sport`, `Brand`, `Social`.

A `DomainIcon` trait unifies them:

```rust
pub trait DomainIcon: Copy {
    /// The curated icon's body assembled into an `Icon`.
    fn icon(self) -> Icon;
    /// The upstream Iconify identifier, e.g. "hugeicons:apple-finder".
    fn iconify_id(self) -> &'static str;
    /// Character representation, if this icon has one.
    fn glyph(self) -> Option<Glyph>;
}
```

String convenience constructors map a set to a fallible lookup (built on the set's
`FromStr`): `Icon::os(&str)`, `Icon::emoji(&str)`, `Icon::file(&str)`, etc.,
returning `Result<Icon, IconError>` (`IconError::UnknownDomainIcon`).

### `Icon`

`Icon` internally holds:

- `body: IconBody` — raw Iconify path markup + `viewBox` + intrinsic width/height.
- `glyph: Option<Glyph>` — Unicode + Nerd Font codepoints; present only for the
  curated subset that defines them.
- `source: Source` — `Embedded` (compiled-in domain icon) vs `Network` (fetched/cached).
- `style: Style` — accumulated builder options.

### Styling (builder)

Styling mutates `Style` and is applied via **local SVG assembly** — no API
round-trip. Mirrors the Iconify option surface:

| Option | Type | Meaning |
|--------|------|---------|
| `color` | `&str` | Any CSS color; applied via `fill`/`currentColor` on the assembled SVG. |
| `width` | `&str` | SVG width (default `1em`). |
| `height` | `&str` | SVG height (default `1em`). |
| `flip` | `&str` | `horizontal`, `vertical`, or `both` (via transform). |
| `rotate` | `&str` | `90`, `180`, or `270` (via transform). |
| `view_box` | `bool` | Adds a transparent bounding-box rect. |

```rust
let svg: String = Os::Finder.icon()
    .color("#d97706").width("24").height("24")
    .rotate("90").flip("horizontal").view_box(true)
    .svg();
```

> **Constraint (not derivable from types):** `color` reliably affects only
> monochrome (`currentColor`) icons. Multicolor icons with fixed path colors are
> unaffected — matching every researched Iconify crate. Documented on the builder.

### Raw accessors

- `fn svg(&self) -> String` — assembled, styled SVG markup.
- `fn unicode_char(&self) -> Option<char>`
- `fn nerd_font_char(&self) -> Option<char>`
- `fn body(&self) -> &IconBody`

## Offline Domain Assets & Dev Pipeline

The ~150 curated icons (enumerated in the README) are vendored as reviewed,
committed assets. A **dev-only** tool — a `just` recipe wrapping a small binary
that uses the `iconify` crate / Iconify API — performs a one-time fetch and writes:

- `assets/icons/<set>/<name>.svg` — icon **bodies + viewBox**, committed and
  reviewable.
- A generated mapping file (consumed via `include!`/`include_str!`) binding each
  enum variant to its body.

The **character representations** (Unicode + Nerd Font codepoints) for the subset
are a hand-curated table in source — never fetched.

Normal `cargo build` never touches the network. Refreshing icons is an explicit,
reviewed `just` action, treated like a dependency update (review the diff; upstream
artwork/licensing may change).

> **Reproducibility:** committed bodies pin the embedded bytes. The `iconify` crate
> and Iconify API appear only in the dev recipe, never in the library/CLI dependency
> graph.

## Network Lookup & SQLite Cache

`Icon::iconify("mdi:home").await` flow:

1. Parse the `prefix:name` identifier (`IconError::InvalidIdentifier` on malformed).
2. **Cache-first:** synchronous `rusqlite` lookup in
   `~/.cache/biscuit-icon/icons.db` (located via `dirs`). On hit, build `Icon` from
   the stored body.
3. **Miss:** async `reqwest` GET to the Iconify API; on success store the row and
   build `Icon`; on failure return `IconError::Fetch`/`IconError::NotFound`.

### Cache schema (SQLite)

```sql
CREATE TABLE IF NOT EXISTS icons (
    prefix      TEXT NOT NULL,
    name        TEXT NOT NULL,
    body        TEXT NOT NULL,   -- raw Iconify <path>… body
    view_box    TEXT NOT NULL,
    width       INTEGER,
    height      INTEGER,
    fetched_at  TEXT NOT NULL,   -- RFC3339
    PRIMARY KEY (prefix, name)
);
CREATE INDEX IF NOT EXISTS idx_icons_name ON icons(name);

CREATE TABLE IF NOT EXISTS sets (
    prefix      TEXT PRIMARY KEY,
    title       TEXT,
    license     TEXT,
    fetched_at  TEXT NOT NULL
);
```

- Indexed on `prefix` (PK) and `name` so the CLI can serve prefix/substring queries
  for completions and offline listings.
- **No TTL/expiry.** Entries persist until `icon cache clear`. The single file is
  inspectable and clearable.

## Rendering — `renderable` Tree Integration

`Icon` implements `renderable::TreeRenderable`, composing into the shared
multi-target IR.

- **Browser/HTML target** → inline assembled `<svg>`.
- **Markdown target** → inline SVG (Markdown permits HTML); text fallback otherwise.
- **Terminal target** → degradation ladder:
  1. **Nerd Font glyph** — if the icon has one *and* Nerd Font mode is enabled.
     Nerd Font presence is not reliably detectable, so this is config/flag/env-gated,
     never auto-sniffed.
  2. **Unicode glyph** — if available.
  3. **Image-protocol render** — only with the `image` feature: rasterize the SVG
     via `resvg`, emit via `biscuit-terminal` image rendering, when the terminal
     advertises an image protocol. The richest fallback for glyph-less icons.
  4. **Text fallback** — the icon identifier, when nothing else applies.

Glyph-first ordering keeps the common terminal path cheap and avoids pulling `resvg`
into the hot path; image rendering is an opt-in enhancement for glyph-less icons.

## CLI (`icon`)

`clap` derive. `icons` is the **default** subcommand, so `icon mdi:home` ≡
`icon icons mdi:home`.

| Command | Behavior |
|---------|----------|
| `icon sets [filter]` | List Iconify set names, optionally filtered by `filter`. |
| `icon icons [filter] [--from <csv>]` | List matching icons **rendered visually** (glyph or image per the render ladder) beside their names. `--from fa,mdi` limits to those sets. Default command. |
| `icon completions` | Dynamic shell completions: always knows built-in set/icon names; also queries the cache DB for cached icon names. |
| `icon cache clear` | Clear the local cache DB. |

**Listing boundary:** `sets`/`icons` serve the built-in domain catalog plus local
cache offline; when a filter is provided, the full Iconify catalog is reached via
the API and merged with offline results (results cached afterward). An empty filter
intentionally lists only offline icons.

**Errors:** the library uses `thiserror` (`IconError`). The CLI catches and renders
human-readable, Prose-styled errors (`<red><b>Error:</b></red>`, deduplicated cause
chain) per repo convention. `--verbose` is styled user output only; `--debug`/
`RUST_LOG` drive raw `tracing`.

## Module Structure (library)

```
lib/src/
├── lib.rs
├── icon.rs            Icon, Source, builder
├── style.rs           Style options + local-assembly logic
├── svg.rs             IconBody, assemble styled <svg>
├── glyph.rs           Glyph (Unicode + Nerd Font)
├── domain/            one module per set + DomainIcon trait
│   ├── mod.rs
│   ├── os.rs, emoji.rs, arrow.rs, …
│   └── generated.rs   include! enum→body mapping
├── iconify/           async client, API types, catalog
├── cache/             rusqlite store, schema, queries
├── render.rs          TreeRenderable impl + terminal degradation
└── error.rs           IconError (thiserror)
```

`IconError` variants (initial): `UnknownDomainIcon`, `InvalidIdentifier`,
`NotFound`, `Fetch`, `Cache`.

## Error Handling

- Library: `IconError` via `thiserror`, one variant per failure mode; network and
  cache errors wrap their sources.
- CLI: `color-eyre` at the boundary, rendered as styled Prose; no raw chains,
  locations, or backtraces in normal (`--verbose`) output.

## Testing Strategy

Follows the monorepo L1/L2/L3 taxonomy and canonical `just` recipes.

- **Domain / styling / SVG assembly (L1):** body→SVG assembly, style application
  (color/size/rotate/flip/view_box), glyph mapping, `strum` round-trips, the
  string-convenience constructors.
- **Cache (L1):** `rusqlite` in a temp dir — get/put, name/prefix queries,
  completion queries, `cache clear`.
- **Iconify client (L1):** `wiremock` for the HTTP contract; no live API in tests.
- **Renderable:** snapshot the tree output per target; terminal degradation ladder
  unit-tested per capability (glyph present/absent, `image` on/off, image-protocol
  advertised or not).
- **CLI (L2/L3):** `assert_cmd` + `predicates` — default-command equivalence,
  `--from` filtering, completions (built-in + cached), error rendering.

## Phasing

A single plan sequenced into three independently shippable phases.

### Phase 1 — Offline library

- Package scaffold (`lib`, workspace member, `justfile`).
- Domain enums + `DomainIcon` trait + string-convenience constructors.
- Offline asset pipeline (dev recipe) + committed bodies + generated mapping.
- `IconBody`, `Style`, local SVG assembly, raw accessors.
- `Glyph` (Unicode + Nerd Font) for the curated subset.
- `renderable` `TreeRenderable` impl: browser inline SVG; terminal glyph → text
  ladder (image path stubbed behind the `image` feature).
- L1 tests.

### Phase 2 — Network cache

- `rusqlite` cache (schema, store, queries) under the user cache dir.
- Async `iconify` client (`reqwest`), API types, `Icon::iconify`.
- Cache-first lookup; set/icon metadata storage.
- Optional `image` feature: `resvg` rasterization in the terminal ladder.
- L1 tests (`wiremock` + temp-dir cache).

### Phase 3 — CLI

- `biscuit-icon-cli` crate, binary `icon`.
- `sets`, `icons` (default, visually rendered), `completions`, `cache clear`.
- Offline-listing boundary; online full-catalog listing.
- Styled Prose error rendering; `--verbose` / `--debug` separation.
- L2/L3 tests.

## Open Questions

None blocking. Future considerations (out of scope for v1):

- A blocking (non-async) convenience API over `Icon::iconify`.
- Self-hosted / proxied Iconify endpoint configuration (`ICONIFY_URL` analogue).
- `icon` `remove`/`update`-style cache maintenance beyond `clear`.
