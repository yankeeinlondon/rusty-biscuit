# Artifacts & Caching

## OutputFormat

```rust
pub enum OutputFormat {
    Svg,
    Png,
}
```

## RenderRequest

Controls how a diagram is rendered:

```rust
pub struct RenderRequest {
    pub format: OutputFormat,        // Default: Png
    pub scale: u32,                  // Default: 2; ignored when target_width is Some
    pub target_width: Option<u32>,   // Exact PNG width in pixels
    pub row_height: Option<u32>,     // Pad PNG height to whole rows; ignored for SVG
    pub background: Background,      // Default: Background::Theme
}

pub enum Background {
    Theme,                     // theme's background color (default)
    Transparent,               // nothing painted
    Solid { r: u8, g: u8, b: u8 },
}
```

`Background` applies to SVG and PNG alike, and also paints the `row_height` padding (transparent unless `Solid`). `Background::solid_hex()` returns `Some("#rrggbb")` for `Solid`.

### Construction

```rust
use biscuit_visualized::artifact::{Background, OutputFormat, RenderRequest};

// SVG with defaults
let req = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };

// PNG at a terminal's display size, transparent background
let req = RenderRequest::default()
    .with_target_width(540)
    .with_row_height(18)
    .with_background(Background::Transparent);
```

## RenderedArtifact

Returned by all `render()` methods:

```rust
pub struct RenderedArtifact {
    pub path: PathBuf,           // Absolute path to the rendered file
    pub format: OutputFormat,    // Svg or Png
    pub cache_hit: bool,         // true if served from cache
    pub alt_text: String,        // Description for accessibility
}
```

The `path` points to either a cache directory entry or a temp file.

## FileCache

Content-addressed file cache using xxHash for key generation.

### Cache Directory Layout

```
$TMPDIR/biscuit-visualized/v1/
├── mermaid/
│   ├── svg/
│   │   └── <hash>.svg
│   └── png/
│       └── <hash>.png
└── graph/
    ├── svg/
    │   └── <hash>.svg
    └── png/
        └── <hash>.png
```

### VisualizationKind

```rust
pub enum VisualizationKind {
    Mermaid,
    Graph,
}
```

### Cache Key Generation

The cache key is an xxHash of a composite string containing:

1. **Key schema prefix** (`v2`) — bumped when what a key covers changes; `v2` added background, row padding, and PNG encoding, so every artifact stored under `v1|` is a miss
2. **Visualization kind** (mermaid or graph)
3. **Source content** (the diagram/graph source text)
4. **Options JSON** (theme, config, orientation, etc., plus a `request` entry from `RenderRequest::cache_identity()`: scale, target width, row height, background, and for PNG the encoder settings `png@0.18:compression=balanced,filter=adaptive`)
5. **Backend ID** (renderer version identifier)
6. **Output format** (svg or png)

This ensures that any change to inputs, configuration, geometry, background, encoder, or renderer version produces a different cache key. The key schema prefix is independent of the `v1` directory name: the directory stays `.../biscuit-visualized/v1/`.

### Mermaid cache key

`MermaidDiagram::cache_key_for` (`mermaid/render.rs`) is checked **after** the source rewrites and before any Merman SVG request:

- **Source:** the rewritten source (`\n` rewrite, gitGraph `commitSpacing` directive), so the written spacing is part of the key.
- **Backend id:** `MERMAID_BACKEND` = `merman@6e6b1e6554e8+bv4` (`merman@<12-char rev>+bv<n>`). Bump the rev with the Merman pin; bump `+bv<n>` whenever the rewrites, post-process, accents, contrast pass, or background change output.
- **Options JSON:** `design` = xxHash of `site config | compiled design JSON | Palette Debug` (font stack, `MermaidConfig`, any palette variables, the family design, and every palette color, so one changed palette color changes the key); `fonts` = `FontChain::cache_identity(&theme_font_stacks())`; `title`; `request` = `RenderRequest::cache_identity()` (scale, target width, row height, background, PNG encoder).
- **Font identity** (`fonts/chain.rs`, memoized per chain and stack list, so once per process for the default chain): a font-set hash over every reachable face, by **contents** for the faces resolved for the theme stack (`"trebuchet ms", verdana, arial, sans-serif`, regular and bold) and the generic families plus the bundled Inter / Linux Noto Emoji, and by **path + size + mtime + face index** for every other font file; plus, per query, the resolved face's identity and its fallback order. So `MERMAID_FONT`, installed fonts, or fallback order changes miss the cache; a file replaced in place with the same size and mtime is not detected.
- A tagged gitGraph's `layout_json` pass runs before the check, so its cache hits still cost one Merman layout.

### FileCache API

```rust
use biscuit_visualized::cache::FileCache;

let cache = FileCache::new();

// Check for cached artifact
if let Some(path) = cache.get(VisualizationKind::Mermaid, &key, OutputFormat::Svg) {
    // Use cached file at `path`
}

// Store a new artifact
let path = cache.store(
    VisualizationKind::Graph,
    &key,
    OutputFormat::Png,
    &png_bytes,
)?;

// Cache management
let bytes = cache.size_bytes();
let count = cache.entry_count();
cache.clear()?;  // Remove all cached files
```

### Cache Lifecycle

- Cache lives in `$TMPDIR`, so it is cleared on OS reboot or temp cleanup
- No automatic eviction or size limits
- `cache.clear()` removes all entries across all kinds and formats
- Cache keys include the backend ID, so renderer upgrades automatically invalidate stale entries
- `store` writes to a temp file in the cache directory and renames it into place, so a concurrent render of the same key never reads a half-written file (every writer of a key writes the same bytes, so the last rename wins harmlessly)
- `$BISCUIT_VISUALIZED_CACHE_DIR` replaces `$TMPDIR` as the cache root (`$BISCUIT_VISUALIZED_CACHE_DIR/biscuit-visualized/v1/`)

## Rendering Flow

The typical render flow for both Mermaid and Graph diagrams:

1. **Compute cache key** from source + options + backend + format (Mermaid: after the source rewrites)
2. **Check cache** — if hit, return `RenderedArtifact` with `cache_hit: true`
3. **Render SVG** — Mermaid via Merman (under the time budget), graphs via layout-rs
4. **Post-process SVG** — Mermaid: solid gradient strokes, `sans-serif` default, palette accents, the text contrast pass (HTML static edits for SVG with `MermaidTarget::HtmlStatic`), then the background rect; graphs: apply colors and fonts, trim padding
5. **If PNG requested**: rasterize SVG via resvg at the target width (or scale), pad to `row_height`, and encode with the `png` crate
6. **Store in cache** and return `RenderedArtifact` with `cache_hit: false`

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/src/artifact.rs` | `OutputFormat`, `RenderRequest`, `RenderedArtifact` |
| `biscuit-visualized/src/src/cache/mod.rs` | `VisualizationKind` enum |
| `biscuit-visualized/src/src/cache/file_cache.rs` | `FileCache` implementation, `MERMAID_BACKEND`, `GRAPH_BACKEND`, `RASTERIZER` |
| `biscuit-visualized/src/src/mermaid/render.rs` | Mermaid cache key (`cache_key_for`) |
| `biscuit-visualized/src/src/fonts/chain.rs` | `FontChain::cache_identity`, `FaceIdentity` |
