# Library API

The `biscuit-icon` library exposes one central handle, `Icon`, plus the
domain enums, the Iconify client, the SQLite cache, and the styling model.
Everything re-exports from the crate root.

```rust
pub use body::IconBody;
pub use cache::SetInfo;
pub use error::IconError;
pub use glyph::Glyph;
pub use icon::{Icon, Source};
pub use style::{Flip, Rotate, Style};
```

## `Icon`

`Icon` is a renderable handle around an `IconBody` plus accumulated `Style`,
an optional `Glyph` (Unicode + Nerd Font codepoints), the icon's id, and the
provenance (`Source`).

### Constructors

There are three layers; pick the most appropriate one:

1. **Enum-first (infallible, compiled in).** Every curated variant in a
   domain enum has an `icon()` method via the `DomainIcon` trait:

   ```rust
   use biscuit_icon::domain::{DomainIcon, Os};
   let finder = Os::Finder.icon();
   ```

   This is the canonical API. It is zero-cost, always available, and
   pinned to the reviewed, vendored body.

2. **String convenience (fallible, compiled in).** A `FromStr` impl on each
   domain enum drives a fallible constructor on `Icon` (one per set):

   ```rust
   use biscuit_icon::Icon;
   let happy = Icon::emoji("happy")?;       // Result<Icon, IconError>
   let finder = Icon::os("finder")?;
   ```

   The full list of constructors: `Icon::os`, `Icon::emoji`, `Icon::arrow`,
   `Icon::data`, `Icon::file`, `Icon::hardware`, `Icon::timing`, `Icon::button`,
   `Icon::control`, `Icon::network`, `Icon::dev_ops`, `Icon::actors`,
   `Icon::nav`, `Icon::sport`, `Icon::brand`, `Icon::social`. An unknown
   name returns `IconError::UnknownDomainIcon { set, name }`.

3. **Network / cache lookup (async).** `Icon::iconify("prefix:name")` looks
   up any of the 200,000+ Iconify icons:

   ```rust
   use biscuit_icon::Icon;
   let home = Icon::iconify("mdi:home").await?;
   ```

   The flow is:

   1. Parse the identifier (returns `IconError::InvalidIdentifier` when it
      is not in `prefix:name` form, or contains characters outside ASCII
      alphanumeric, `-`, or `_`).
   2. Open the default cache and read the body synchronously, off the async
      runtime thread via `tokio::task::spawn_blocking`.
   3. On a cache miss, fetch from the public Iconify API
      (`GET {base}/{prefix}.json?icons={name}`).
   4. On success, persist the body to the cache (again via
      `spawn_blocking`) and return the `Icon`.

   For tests, `Icon::iconify_with(id, &cache, &client)` takes an explicit
   `IconCache` and `IconifyClient` (use `IconifyClient::with_base` to point
   at a `wiremock` server).

### Builder methods

| Method | Signature | Effect |
|--------|-----------|--------|
| `color` | `fn color(self, c: impl Into<String>) -> Self` | Sets the inline `style="color: …"`, driving `currentColor` for monochrome icons. The value is XML-attribute-escaped. |
| `width` | `fn width(self, w: impl Into<String>) -> Self` | SVG `width` attribute (default `"1em"`). |
| `height` | `fn height(self, h: impl Into<String>) -> Self` | SVG `height` attribute (default `"1em"`). |
| `flip` | `fn flip(self, f: Flip) -> Self` | Horizontal / vertical / both-axis flip. Implemented via a `translate(...) scale(...)` transform that accounts for non-zero view-box origins. |
| `rotate` | `fn rotate(self, r: Rotate) -> Self` | `R90` / `R180` / `R270` rotation. The viewBox is swapped for 90/270 rotations of non-square bodies, and the rotation pivots around the viewBox center. |
| `view_box` | `fn view_box(self, on: bool) -> Self` | When `true`, prepends a transparent `<rect>` spanning the viewBox (matches Iconify's "Background" bounding box). |
| `nerd_font` | `fn nerd_font(self, on: bool) -> Self` | When `true`, prefers the Nerd Font glyph during terminal rendering (no effect on the assembled SVG). |

All builders are `#[must_use]` and chainable; finish with `.svg()` for the
assembled markup, `.body()` for the raw `IconBody`, or feed the `Icon` to
the renderable tree.

### Raw accessors

- `id() -> &str` — the icon identifier. For network icons this is the
  `prefix:name` you passed in; for embedded domain icons it is the upstream
  Iconify id (e.g. `"hugeicons:apple-finder"`).
- `source() -> Source` — `Embedded` or `Network`.
- `body() -> &IconBody` — the raw body (markup + view-box).
- `unicode_char() -> Option<char>` — the Unicode codepoint, if the curated
  variant defines one.
- `nerd_font_char() -> Option<char>` — the Nerd Font codepoint, if the
  curated variant defines one.
- `svg() -> String` — the assembled, styled `<svg>` string.
- `css() -> String` — the styled SVG percent-encoded as a CSS `url('data:image/svg+xml,...')` data URI.

## `Source`

```rust
pub enum Source {
    Embedded,  // a compiled-in curated domain icon
    Network,   // a network-fetched / cache-resident Iconify icon
}
```

The source is determined at construction time and does not change. Use it
to differentiate "always-available offline" icons from ones that depend on
the network being reachable at least once.

## `IconBody`

The raw Iconify body plus the geometry needed to wrap it in a complete
`<svg>`:

```rust
pub struct IconBody {
    pub body: String,   // inner SVG markup (paths, groups) — no surrounding <svg>
    pub width: u32,    // intrinsic width of the icon's coordinate system
    pub height: u32,   // intrinsic height
    pub left: i32,     // X-origin of the view box (default 0)
    pub top: i32,      // Y-origin of the view box (default 0)
}
```

Two constructors:

- `IconBody::new(body, width, height)` — origin defaults to `(0, 0)`.
- `IconBody::with_origin(body, width, height, left, top)` — explicit origin
  for icons whose viewBox is offset (preserved through cache round-trips).

`IconBody::view_box()` returns the formatted `"left top width height"`
string. Both width/height and left/top default to `0`; the Iconify client
uses the icon-level width/height when the per-icon override is absent.

## `Style`

```rust
pub struct Style {
    pub color: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub flip: Option<Flip>,
    pub rotate: Option<Rotate>,
    pub view_box: bool,  // default false
}
```

`Style::default()` is the "let the SVG defaults apply" state. `Style::assemble`
turns a body + style into a complete `<svg>` string with XML-attribute
escaping on user-supplied color and dimensions.

`Style` also implements `assemble(&IconBody) -> String`. Rotation
`R90`/`R270` swaps the viewBox width and height; flip uses a
`translate(...) scale(...)` transform whose translate compensates for
non-zero view-box origins so the icon does not get pushed off-canvas.

### `Flip` / `Rotate` enums

```rust
pub enum Flip { Horizontal, Vertical, Both }
pub enum Rotate { R90, R180, R270 }
```

Both implement `TryFrom<&str>`:

- `Flip::try_from("horizontal" | "vertical" | "both")`
- `Rotate::try_from("90" | "180" | "270")`

Invalid values return `IconError::InvalidIdentifier(format!("invalid flip/rotate: {value}"))`.

## `Glyph`

The character representation of an icon (only the curated subset defines
one):

```rust
pub struct Glyph {
    pub unicode: Option<char>,  // plain Unicode codepoint
    pub nerd_font: Option<char>,// Nerd Font private-use codepoint
}
```

Three constructors: `Glyph::unicode(c)`, `Glyph::nerd(c)`, `Glyph::both(unicode, nerd_font)`.

`Icon::unicode_char()` and `Icon::nerd_font_char()` are the user-facing
projections (return `None` when the variant defines no glyph or the icon
came from the network).

## `IconError`

```rust
pub enum IconError {
    UnknownDomainIcon { set: &'static str, name: String },
    InvalidIdentifier(String),
    NotFound(String),
    Fetch(String),
    Cache(String),
}
```

| Variant | Source | Notes |
|---------|--------|-------|
| `UnknownDomainIcon` | `Icon::os(...)` and friends, on an unknown variant name | `set` is the static domain-set name (e.g. `"os"`). |
| `InvalidIdentifier` | `parse_id`, `Icon::iconify`, the `Style::try_from` impls | The string is included verbatim for diagnostics. |
| `NotFound` | `IconifyClient::fetch_body` | The upstream returned a successful response that did not contain the requested icon. |
| `Fetch` | `IconifyClient::fetch_body` / `fetch_collections` / `search_icons` | Wraps transport / HTTP / parse errors. |
| `Cache` | Any `IconCache` operation | Wraps the underlying SQLite error message. |

`pub type Result<T> = std::result::Result<T, IconError>;` is the crate-wide
alias.

## Domain Enums

There is one enum per curated set: `Os`, `Emoji`, `Arrow`, `Data`, `File`,
`Hardware`, `Timing`, `Button`, `Control`, `Network`, `DevOps`, `Actors`,
`Nav`, `Sport`, `Brand`, `Social`. Each derives
`strum::{EnumString, EnumIter, Display}` plus `Copy, Clone, Debug,
PartialEq, Eq, Hash`, and implements the `DomainIcon` trait.

`biscuit_icon::domain::all_iconify_ids()` returns every curated Iconify
identifier across all domain sets (used by the asset pipeline and the
shell completion engine). `biscuit_icon::domain::icon_for_id(id)` resolves
an Iconify identifier to its `Icon`, if one is curated.

For the full list of curated variants, see [domain-icons.md](domain-icons.md).

## Module Layout

```
biscuit-icon/lib/src/
├── lib.rs              public re-exports
├── icon.rs             Icon, Source, builder, string-convenience ctors
├── style.rs            Style, Flip, Rotate, local <svg> assembly
├── body.rs             IconBody
├── glyph.rs            Glyph (Unicode + Nerd Font)
├── domain/             one module per set, generated.rs, DomainIcon trait
│   ├── mod.rs          all_iconify_ids, icon_for_id
│   ├── os.rs, emoji.rs, ...
│   └── generated.rs    include! enum→body mapping
├── iconify/            async HTTP client + API types
│   ├── mod.rs
│   └── client.rs       IconifyClient, parse_id, CollectionInfo, License
├── cache/              on-disk SQLite cache
│   ├── mod.rs
│   └── store.rs        IconCache, SetInfo, schema + migrations
├── catalog.rs          unified offline catalog (built-in ∪ cache)
├── render.rs           TreeRenderable impl (terminal + browser + markdown)
└── error.rs            IconError, Result
```

## Cargo Features

| Feature | Default | Effect |
|---------|---------|--------|
| `image` | off | Pulls in `biscuit-visualized`/`resvg`; enables the image rung of the terminal degradation ladder (rasterize the assembled SVG and emit it via the terminal's image protocol). When off, the ladder stops at the text-identifier fallback for glyph-less icons. |

`icon` (`biscuit-icon-cli`) supports the same `image` feature; its L2
tests are gated behind `--features image` (`just test-l2`).
