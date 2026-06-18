# Iconify API

`biscuit-icon` speaks the [public Iconify JSON API]. The library uses
three endpoints — collection body lookup, collection listing, and search
— and the public hosts are mirrored on GitHub, NPM, and Docker for
self-hosting.

[public Iconify JSON API]: https://iconify.design/docs/api/queries.html

## Hosts

| Purpose | URL |
|---------|-----|
| Public API (default)            | `https://api.iconify.design` |
| Set directory (browse by prefix) | `https://icon-sets.iconify.design` |
| Per-set gallery                  | `https://icon-sets.iconify.design/{prefix}/` |
| Documentation root               | `https://iconify.design/docs` |
| Queries reference                | `https://iconify.design/docs/api/queries.html` |
| Self-hosting (Docker)            | `https://hub.docker.com/r/iconify/api` |
| Self-hosting (NPM)               | `https://www.npmjs.com/package/@iconify/api` |
| Source (self-host customization) | `https://github.com/iconify/api/` |
| Browse alternative (Anthony Fu)  | `https://icones.js.org/` |

The library's `IconifyClient::new()` targets
`https://api.iconify.design`; `IconifyClient::with_base(url)` lets tests
point at a `wiremock` server, and the CLI honors the `ICONIFY_BASE_URL`
environment variable for self-hosted instances.

## Endpoints Used by the Library

### `GET /{prefix}.json?icons={name}` — collection body

Fetch a single icon's body, optionally restricted to specific names
within the collection. The library uses this for `Icon::iconify` and
`IconifyClient::fetch_body`.

Example:

```
GET https://api.iconify.design/mdi.json?icons=home
```

Response shape (the library deserializes the `icons` map into
`IconEntry` and the top-level `width` / `height` are used as the
collection defaults):

```json
{
  "prefix": "mdi",
  "width": 24,
  "height": 24,
  "icons": {
    "home": {
      "body": "<path d=\"...\"/>",
      "left": 0,
      "top": 0,
      "width": 24,
      "height": 24
    }
  }
}
```

Per-icon `width` / `height` / `left` / `top` override the collection
defaults; missing fields default to `16` for width/height and `0` for
left/top. A successful response that does not contain the requested
`name` is mapped to `IconError::NotFound`. Any non-2xx HTTP status
returns `IconError::Fetch`.

### `GET /collections` — collection listing

Enumerate every collection with its human title, optional license
metadata, and the upstream icon count. The library uses this for
`IconifyClient::fetch_collections` (drives `icon sets`).

Example:

```
GET https://api.iconify.design/collections
```

Response shape (the library decodes the top-level object as a
`BTreeMap<String, CollectionMeta>` so the returned `Vec<CollectionInfo>`
is sorted by prefix):

```json
{
  "mdi": {
    "name": "Material Design Icons",
    "total": 5000,
    "license": {
      "title": "Apache License 2.0",
      "spdx": "Apache-2.0",
      "url": "https://github.com/Templarian/MaterialDesign/blob/master/LICENSE"
    }
  },
  "lucide": {
    "name": "Lucide",
    "total": 0
  },
  "hero": {
    "name": "Hero Icons"
  }
}
```

The library maps this into:

```rust
pub struct CollectionInfo {
    pub prefix: String,
    pub title: String,
    pub license: Option<License>,
    pub total: Option<usize>,  // None when missing (not zero)
}
pub struct License {
    pub title: String,
    pub spdx: String,
    pub url: Option<String>,
}
```

> A missing `total` is **unknown** and is not coerced to `0`. The CLI
> renders unknown totals as `Unknown` in the `sets` table — see
> [cli.md](cli.md).

### `GET /search?query=…&limit=…&start=…[&prefix=…|&prefixes=…]` — search

Full-text search over the icon catalog. The library uses this for
`IconifyClient::search_icons`, which the CLI drives from
`icon <filter>` and `icon --from <csv> <filter>`.

Query parameters (all optional except `query`):

| Param      | Type   | Notes |
|------------|--------|-------|
| `query`    | string | Required. An empty query is not supported and returns an error. |
| `limit`    | int    | Page size; the library pages in batches of 100. |
| `start`    | int    | Offset into the result list (incremented by the previous batch's `icons.length`). |
| `prefix`   | string | Restrict to a single prefix (sent when the caller passes exactly one). |
| `prefixes` | string | Comma-separated restriction (sent when the caller passes two or more). |

Response shape:

```json
{
  "icons": ["mdi:home", "lucide:home", "..."],
  "total": 2
}
```

The library returns `(Vec<String>, usize)` — the (potentially
truncated) hits and the API's reported total so the CLI can show "…
N more result(s) available online" when applicable.

## Identifier Syntax

An Iconify identifier is `prefix:name`. The library's `parse_id` is
strict about the syntax:

- Exactly one `:` separating the two parts.
- Both parts are non-empty.
- Both parts contain only ASCII alphanumerics, `-`, or `_`.
- The name does not contain an additional `:`.

Anything else returns `IconError::InvalidIdentifier`. The string is
included verbatim in the error for diagnostics, but it is not
echoed to the rendered icon, and user-supplied values do not pass
through the SVG assembler (XML-attribute escaping is applied to
`Style::color`, `width`, and `height`).

## Icon Body Format

An `IconBody` from the API looks like:

```rust
pub struct IconBody {
    pub body: String,   // inner SVG markup (paths, groups) — no surrounding <svg>
    pub width: u32,    // intrinsic width of the icon's coordinate system
    pub height: u32,
    pub left: i32,     // X-origin of the view box (default 0)
    pub top: i32,      // Y-origin of the view box (default 0)
}
```

Non-zero `left` / `top` origins are preserved through cache
round-trips and threaded through the `Style::assemble` SVG output
and the flip / rotate transforms. The `viewBox` is rendered as
`"{left} {top} {width} {height}"`.

## Self-hosting

You can host the Iconify API yourself — the public service is
free-to-use but not free-to-run, and the maintainers ask heavy users
to self-host or sponsor. Three distribution channels:

- **GitHub** — clone and customize: <https://github.com/iconify/api/>.
- **NPM** — embed without running a server: `@iconify/api` at
  <https://www.npmjs.com/package/@iconify/api>.
- **Docker** — quick deployment: <https://hub.docker.com/r/iconify/api>.

Point the CLI at your instance with `ICONIFY_BASE_URL=https://your-host`.
The library itself is configured at construction time via
`IconifyClient::with_base(url)`.

## Ways to Use Icons (Reference)

For completeness — the broader use cases Iconify supports, not all of
which `biscuit-icon` directly addresses:

1. **SVG + CSS** — the preferred solution in browsers. Reduces HTML
   size, caches icons in CSS, and gives CSS full power over the
   icons. See <https://iconify.design/docs/usage/svg-css/>. Limited
   framework support.
2. **SVG in CSS** — use icons as background or mask images in CSS,
   with `<span>` elements in HTML. See
   <https://iconify.design/docs/usage/css/>.
3. **SVG in HTML** — embed `<svg>` directly. See
   <https://iconify.design/docs/usage/svg/>.

`biscuit-icon` itself produces the third form: the assembled
`<svg>` from `Icon::svg()`. CSS and SVG+CSS are caller concerns
that layer on top.

## Iconify Platform Metadata

The Iconify platform exposes rich metadata at both the **icon set** level and the **individual icon** level. This metadata is available through the API and is cached locally in the SQLite database.

### Icon Set Metadata (IconifyInfo)

Each icon set has associated metadata exposed via `/collections` and included in search responses:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable icon set name |
| `author.name` | string | Author's name |
| `author.url` | string | Link to the icon set's website/repository (optional) |
| `license.title` | string | Human-readable license name |
| `license.spdx` | string | SPDX license identifier |
| `license.url` | string | Link to license file (optional) |
| `total` | number | Total icon count in the set |
| `version` | string | Version string of the icon set |
| `samples` | string[] | Sample icon names to display as previews |
| `height` | number \| number[] | Icon grid height; can be an array for mixed heights |
| `displayHeight` | number | Height to use when showing samples (16–24) |
| `category` | string | Category for filtering sets (e.g., "General", "Emoji", "Logos") |
| `tags` | string[] | Tags for filtering icon sets |
| `palette` | boolean | `true` if icons use hardcoded colors; `false` if they use `currentColor` |

Example from `/collections`:

```json
{
  "mdi": {
    "name": "Material Design Icons",
    "total": 5000,
    "license": {
      "title": "Apache License 2.0",
      "spdx": "Apache-2.0",
      "url": "https://github.com/Templarian/MaterialDesign/blob/master/LICENSE"
    }
  }
}
```

### Icon-Level Metadata

#### Categories

Each icon can belong to one or more categories. Categories are stored in the `categories` object within an icon set, mapping category names to arrays of icon names:

```json
{
  "categories": {
    "Accessibility": ["accessible-icon"],
    "Audio & Video": ["youtube"],
    "Communication": ["bluetooth", "bluetooth-b"],
    "Currency": ["bitcoin", "btc", "ethereum"]
  }
}
```

Not all icon sets include category data. The Iconify browse UI (`icon-sets.iconify.design`) exposes filters for `?category=` and `?tag=` on per-set galleries.

#### Themes: Prefixes and Suffixes

Themes organize icon variations that share a common prefix or suffix. This is common in icon sets like Material Design Icons (baseline-, outline-, round-, sharp-, twotone-) or Ant Design (filled, outlined, twotone).

**Prefixes** — the prefix appears at the start of the icon name:

```json
{
  "prefixes": {
    "baseline": "Baseline",
    "outline": "Outline",
    "round": "Round",
    "sharp": "Sharp",
    "twotone": "Two-Tone"
  }
}
```

Icons like `baseline-home` belong to "Baseline"; `outline-home` belongs to "Outline".

**Suffixes** — the suffix appears at the end of the icon name:

```json
{
  "suffixes": {
    "filled": "Filled",
    "outlined": "Outlined",
    "twotone": "TwoTone"
  }
}
```

Icons like `home-filled` belong to "Filled"; `home-outlined` belongs to "Outlined".

A default theme with an empty string key catches icons that don't match other themes:

```json
{
  "suffixes": {
    "": "Filled",
    "outline": "Outline",
    "negative": "Negative"
  }
}
```

#### Characters Map (Icon Fonts)

The `chars` map links Unicode code points (in hex) to icon names, used for icons imported from icon fonts:

```json
{
  "chars": {
    "e007": "firefox-browser",
    "e013": "ideal",
    "e01a": "microblog"
  }
}
```

### Search and Keywords API

#### `/search` — Full-Text Icon Search

Searches icons across the catalog. Returns matching icon names (with prefix), total count, and `collections` info for the sets in the results.

```json
{
  "icons": ["mdi:home", "lucide:home", "..."],
  "total": 64,
  "limit": 64,
  "start": 0,
  "collections": {
    "mdi": { "name": "Material Design Icons", "total": 5000, "category": "General", ... }
  }
}
```

Query parameters: `query` (required), `limit` (32–999, default 64), `start`, `prefix`, `prefixes`, `category`.

#### `/keywords` — Autocomplete Suggestions

Returns keyword matches for building autocomplete UIs. Two modes:

- `prefix` — matches keywords that **start with** the input (e.g., `hom` → `home`, `home2`, `homebrew`)
- `keyword` — matches keywords that **start or end with** the input (e.g., `home` → `home`, `esphome`, `petsathome`)

```json
{
  "prefix": "hom",
  "exists": false,
  "matches": ["home", "home2", "home3", "homebrew", "homebridge", ...]
}
```

Keyword requirements: letters a–z, numbers, and `-` only; at least 2 characters; `-` splits on the last segment only.

### Cached Metadata in SQLite

The local SQLite cache (`~/.cache/biscuit-icon/icons.db`) stores icon set metadata including:

- `icon_sets` table — set prefix, title, license, total count, category, palette flag
- `icons` table — per-icon entries with name, width, height, body, and parent set

Icon-level categories, themes (prefixes/suffixes), and the characters map are available from the full Iconify JSON API responses but are **not currently cached** in `biscuit-icon`'s SQLite schema — only the icon body and basic set metadata are persisted locally.

## Iconify Documentation Quick Links

- API overview — <https://iconify.design/docs/api/queries.html>
- API hosting — <https://iconify.design/docs/api/hosting.html>
- SVG generation — <https://iconify.design/docs/api/svg.html>
- CSS generation — <https://iconify.design/docs/api/css.html>
- Icon components — <https://iconify.design/docs/icon-components/>
- Icon sets directory — <https://icon-sets.iconify.design>
- Per-set galleries — `https://icon-sets.iconify.design/{prefix}/`
- Browse alternative — <https://icones.js.org/>
- IconifyInfo type — <https://iconify.design/docs/types/iconify-info.html>
- IconifyJSON metadata — <https://iconify.design/docs/types/iconify-json-metadata.html>
- Search API — <https://iconify.design/docs/api/search.html>
- Keywords API — <https://iconify.design/docs/api/keywords.html>
