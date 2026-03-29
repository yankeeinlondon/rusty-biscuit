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
    pub format: OutputFormat,
    pub scale: f32,              // Default: 1.0 (only affects PNG)
    pub transparent_background: bool,  // Default: false
}
```

### Construction

```rust
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

// SVG with defaults
let req = RenderRequest::new(OutputFormat::Svg);

// PNG at 2x scale with transparent background
let req = RenderRequest::new(OutputFormat::Png)
    .with_scale(2.0)
    .with_transparent_background(true);
```

## RenderedArtifact

Returned by all `render()` methods:

```rust
pub struct RenderedArtifact {
    pub path: PathBuf,           // Absolute path to the rendered file
    pub format: OutputFormat,    // Svg or Png
    pub cache_hit: bool,         // true if served from cache
    pub alt_text: Option<String>, // Description for accessibility
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

1. **Version prefix** (`v1`) — for cache schema evolution
2. **Visualization kind** (mermaid or graph)
3. **Source content** (the diagram/graph source text)
4. **Options JSON** (theme, config, orientation, etc.)
5. **Backend ID** (renderer version identifier)
6. **Output format** (svg or png)

This ensures that any change to inputs, configuration, or renderer version produces a different cache key.

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

## Rendering Flow

The typical render flow for both Mermaid and Graph diagrams:

1. **Compute cache key** from source + options + backend + format
2. **Check cache** — if hit, return `RenderedArtifact` with `cache_hit: true`
3. **Render SVG** — via mermaid-rs-renderer or layout-rs
4. **Post-process SVG** — fix alignment, apply colors, trim padding
5. **If PNG requested**: rasterize SVG via resvg at the specified scale
6. **Store in cache** and return `RenderedArtifact` with `cache_hit: false`

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/artifact.rs` | `OutputFormat`, `RenderRequest`, `RenderedArtifact` |
| `biscuit-visualized/src/cache/mod.rs` | `VisualizationKind` enum |
| `biscuit-visualized/src/cache/file_cache.rs` | `FileCache` implementation |
