# Smart Image Design Document

This document provides the technical design for the smart image feature of the `composition` library. See [Smart Image Requirements](../archive/smart-image-requirements.md) for functional requirements.

## Overview

The smart image system generates optimized image variants for web delivery using HTML's `<srcset>` functionality. It produces multiple sizes and formats (AVIF, WebP, JPEG/PNG fallbacks) to ensure optimal loading across devices and browsers.

## Dependencies

```toml
[dependencies]
# Image processing
image = { version = "0.25", default-features = false, features = [
    "png", "jpeg", "webp", "avif", "gif"
] }
kamadak-exif = "0.5"

# Parallel processing
rayon = "1.8"

# Hashing
xxhash-rust = { version = "0.8", features = ["xxh3"] }

# Caching
surrealdb = { version = "2", features = ["kv-rocksdb"] }

# Error handling
thiserror = "2"

# Logging
tracing = "0.1"
```

## Core Types

### Configuration

```rust
use std::path::PathBuf;

/// Breakpoint configuration following Tailwind CSS conventions.
/// Image widths are derived from these values.
#[derive(Debug, Clone)]
pub struct Breakpoints {
    /// Default: 640px
    pub sm: u32,
    /// Default: 768px
    pub md: u32,
    /// Default: 1024px
    pub lg: u32,
    /// Default: 1280px
    pub xl: u32,
    /// Default: 1536px
    pub xxl: u32,
}

impl Default for Breakpoints {
    fn default() -> Self {
        Self {
            sm: 640,
            md: 768,
            lg: 1024,
            xl: 1280,
            xxl: 1536,
        }
    }
}

impl Breakpoints {
    /// Derive image widths from breakpoints.
    /// - micro: sm / 2
    /// - xs: sm
    /// - sm, md, lg, xl, 2xl: breakpoint * 2 (for retina displays)
    pub fn to_image_widths(&self) -> ImageWidths {
        ImageWidths {
            micro: self.sm / 2,
            xs: self.sm,
            sm: self.sm * 2,
            md: self.md * 2,
            lg: self.lg * 2,
            xl: self.xl * 2,
            xxl: self.xxl * 2,
        }
    }
}

/// Target image widths in pixels.
#[derive(Debug, Clone)]
pub struct ImageWidths {
    /// 320px default (sm / 2)
    pub micro: u32,
    /// 640px default (sm)
    pub xs: u32,
    /// 1280px default (sm * 2)
    pub sm: u32,
    /// 1536px default (md * 2)
    pub md: u32,
    /// 2048px default (lg * 2)
    pub lg: u32,
    /// 2560px default (xl * 2)
    pub xl: u32,
    /// 3072px default (2xl * 2)
    pub xxl: u32,
}

impl ImageWidths {
    /// Returns widths as a sorted vector for iteration.
    pub fn as_vec(&self) -> Vec<(ImageSize, u32)> {
        vec![
            (ImageSize::Micro, self.micro),
            (ImageSize::Xs, self.xs),
            (ImageSize::Sm, self.sm),
            (ImageSize::Md, self.md),
            (ImageSize::Lg, self.lg),
            (ImageSize::Xl, self.xl),
            (ImageSize::Xxl, self.xxl),
        ]
    }
}

/// Named size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageSize {
    Micro,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl ImageSize {
    pub fn suffix(&self) -> &'static str {
        match self {
            ImageSize::Micro => "-micro",
            ImageSize::Xs => "-xs",
            ImageSize::Sm => "-sm",
            ImageSize::Md => "-md",
            ImageSize::Lg => "-lg",
            ImageSize::Xl => "-xl",
            ImageSize::Xxl => "-2xl",
        }
    }
}
```

### Input/Output Types

```rust
use std::path::PathBuf;

/// Source of an image to be processed.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// Local file path.
    Local(PathBuf),
    /// Remote URL (will be fetched and cached).
    Remote(String),
}

impl ImageSource {
    /// Compute the resource hash (hash of identifier, not content).
    pub fn resource_hash(&self) -> u64 {
        use xxhash_rust::xxh3::xxh3_64;
        match self {
            ImageSource::Local(path) => xxh3_64(path.to_string_lossy().as_bytes()),
            ImageSource::Remote(url) => xxh3_64(url.as_bytes()),
        }
    }

    /// Format resource hash as hex string for filenames.
    pub fn resource_hash_hex(&self) -> String {
        format!("{:016x}", self.resource_hash())
    }
}

/// Input to the smart_images function.
#[derive(Debug, Clone)]
pub struct SmartImageInput {
    /// The source image (local file or remote URL).
    pub source: ImageSource,
    /// Breakpoints for this image (from document frontmatter).
    pub breakpoints: Breakpoints,
}

/// Result of processing a single image.
#[derive(Debug, Clone)]
pub struct SmartImageOutput {
    /// The resource hash (base filename).
    pub hash: String,
    /// Generated variant files, keyed by (size, format).
    pub variants: Vec<ImageVariant>,
    /// Optional blur placeholder image path.
    pub blur_placeholder: Option<PathBuf>,
    /// Extracted metadata (if any).
    pub metadata: Option<ImageMetadata>,
}

/// A single generated image variant.
#[derive(Debug, Clone)]
pub struct ImageVariant {
    /// Size variant.
    pub size: ImageSize,
    /// Output format.
    pub format: OutputFormat,
    /// Output file path.
    pub path: PathBuf,
    /// Actual width after processing (may differ from target if source was smaller).
    pub width: u32,
    /// Actual height after processing.
    pub height: u32,
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    Avif,
    WebP,
    Jpeg,
    Png,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Avif => "avif",
            OutputFormat::WebP => "webp",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
        }
    }

    /// Determine fallback format based on source.
    pub fn fallback_for_source(has_transparency: bool) -> Self {
        if has_transparency {
            OutputFormat::Png
        } else {
            OutputFormat::Jpeg
        }
    }
}

/// Extracted image metadata.
#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    /// Camera make.
    pub make: Option<String>,
    /// Camera model.
    pub model: Option<String>,
    /// Date/time original.
    pub date_time: Option<String>,
    /// GPS latitude.
    pub latitude: Option<f64>,
    /// GPS longitude.
    pub longitude: Option<f64>,
    /// Image description (potential alt text).
    pub description: Option<String>,
    /// Original width.
    pub original_width: u32,
    /// Original height.
    pub original_height: u32,
}
```

### Error Types

```rust
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum SmartImageError {
    #[error("Failed to open image: {path}")]
    OpenFailed {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("Failed to save image: {path}")]
    SaveFailed {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("Unsupported image format: {extension}")]
    UnsupportedFormat { extension: String },

    #[error("Failed to fetch remote image: {url}")]
    FetchFailed {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Cache operation failed")]
    CacheFailed {
        #[source]
        source: surrealdb::Error,
    },

    #[error("Failed to read EXIF metadata")]
    ExifFailed {
        #[source]
        source: kamadak_exif::Error,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SmartImageError>;
```

## Image Cache Schema

The cache uses SurrealDB with an embedded RocksDB backend for persistence.

### SurrealDB Schema

```sql
-- Table: image_cache
-- Stores cache entries keyed by resource hash

DEFINE TABLE image_cache SCHEMAFULL;

-- Resource hash (hash of file path or URL)
DEFINE FIELD resource_hash ON image_cache TYPE string;
-- Content hash (hash of actual image bytes)
DEFINE FIELD content_hash ON image_cache TYPE string;
-- Timestamp when cache entry was created
DEFINE FIELD created_at ON image_cache TYPE datetime DEFAULT time::now();
-- Source type: "local" or "remote"
DEFINE FIELD source_type ON image_cache TYPE string;
-- Original source path/URL for debugging
DEFINE FIELD source ON image_cache TYPE string;
-- Whether transparency was detected
DEFINE FIELD has_transparency ON image_cache TYPE bool;
-- Original dimensions
DEFINE FIELD original_width ON image_cache TYPE int;
DEFINE FIELD original_height ON image_cache TYPE int;

-- Index for fast lookup by resource hash
DEFINE INDEX idx_resource_hash ON image_cache FIELDS resource_hash UNIQUE;
```

### Rust Cache Types

```rust
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageCacheEntry {
    pub id: RecordId,
    pub resource_hash: String,
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub source_type: String,
    pub source: String,
    pub has_transparency: bool,
    pub original_width: u32,
    pub original_height: u32,
}

#[derive(Debug, Serialize)]
pub struct NewCacheEntry {
    pub resource_hash: String,
    pub content_hash: String,
    pub source_type: String,
    pub source: String,
    pub has_transparency: bool,
    pub original_width: u32,
    pub original_height: u32,
}
```

### Cache Operations

```rust
use surrealdb::{Surreal, engine::local::RocksDb};

pub struct ImageCache {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl ImageCache {
    /// Initialize cache with embedded RocksDB.
    pub async fn new(db_path: &Path) -> Result<Self> {
        let db = Surreal::new::<RocksDb>(db_path).await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        db.use_ns("composition").use_db("cache").await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        Ok(Self { db })
    }

    /// Check if a valid cache entry exists.
    /// Returns None if not found or if content hash doesn't match.
    pub async fn get(&self, resource_hash: &str, content_hash: &str) -> Result<Option<ImageCacheEntry>> {
        let mut response = self.db
            .query("SELECT * FROM image_cache WHERE resource_hash = $rh AND content_hash = $ch")
            .bind(("rh", resource_hash))
            .bind(("ch", content_hash))
            .await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        let entry: Option<ImageCacheEntry> = response.take(0)
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        Ok(entry)
    }

    /// Check if cache entry exists (by resource hash only).
    /// Used to detect stale entries.
    pub async fn get_by_resource(&self, resource_hash: &str) -> Result<Option<ImageCacheEntry>> {
        let mut response = self.db
            .query("SELECT * FROM image_cache WHERE resource_hash = $rh")
            .bind(("rh", resource_hash))
            .await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        Ok(response.take(0).ok().flatten())
    }

    /// Insert or update cache entry.
    pub async fn upsert(&self, entry: NewCacheEntry) -> Result<()> {
        self.db
            .query(r#"
                DELETE FROM image_cache WHERE resource_hash = $resource_hash;
                CREATE image_cache CONTENT $entry;
            "#)
            .bind(("resource_hash", &entry.resource_hash))
            .bind(("entry", &entry))
            .await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        Ok(())
    }

    /// Remove stale cache entry.
    pub async fn remove(&self, resource_hash: &str) -> Result<()> {
        self.db
            .query("DELETE FROM image_cache WHERE resource_hash = $rh")
            .bind(("rh", resource_hash))
            .await
            .map_err(|e| SmartImageError::CacheFailed { source: e })?;

        Ok(())
    }
}
```

## Image Processing Pipeline

### Main Entry Point

```rust
use rayon::prelude::*;
use std::path::Path;
use tracing::{info, warn, instrument};

/// Process multiple images in parallel.
#[instrument(skip(inputs, cache))]
pub fn smart_images(
    inputs: Vec<SmartImageInput>,
    output_dir: &Path,
    cache: &ImageCache,
) -> Vec<Result<SmartImageOutput>> {
    let images_dir = output_dir.join("images");
    std::fs::create_dir_all(&images_dir).ok();

    info!(count = inputs.len(), "Processing images");

    inputs
        .into_par_iter()
        .map(|input| process_single_image(input, &images_dir, cache))
        .collect()
}

/// Process a single image source.
fn process_single_image(
    input: SmartImageInput,
    output_dir: &Path,
    cache: &ImageCache,
) -> Result<SmartImageOutput> {
    let resource_hash = input.source.resource_hash_hex();

    // Step 1: Load source image bytes
    let (bytes, source_str) = load_source_bytes(&input.source)?;

    // Step 2: Compute content hash
    let content_hash = compute_content_hash(&bytes);

    // Step 3: Check cache (async block in sync context via tokio::runtime::Handle)
    let handle = tokio::runtime::Handle::current();
    let cached = handle.block_on(async {
        cache.get(&resource_hash, &content_hash).await
    })?;

    if let Some(entry) = cached {
        info!(hash = %resource_hash, "Cache hit - skipping processing");
        return load_cached_output(&resource_hash, &entry, output_dir);
    }

    // Step 4: Check for stale cache and remove
    let stale = handle.block_on(async {
        cache.get_by_resource(&resource_hash).await
    })?;
    if stale.is_some() {
        warn!(hash = %resource_hash, "Stale cache entry - removing");
        handle.block_on(async {
            cache.remove(&resource_hash).await
        })?;
    }

    // Step 5: Decode image
    let img = image::load_from_memory(&bytes)
        .map_err(|e| SmartImageError::OpenFailed {
            path: PathBuf::from(&source_str),
            source: e,
        })?;

    // Step 6: Detect transparency
    let has_transparency = detect_transparency(&img);

    // Step 7: Extract metadata (before stripping)
    let metadata = extract_metadata(&bytes);

    // Step 8: Generate all variants
    let widths = input.breakpoints.to_image_widths();
    let variants = generate_variants(&img, &resource_hash, output_dir, &widths, has_transparency)?;

    // Step 9: Generate blur placeholder (optional)
    let blur_placeholder = generate_blur_placeholder(&img, &resource_hash, output_dir)?;

    // Step 10: Update cache
    let cache_entry = NewCacheEntry {
        resource_hash: resource_hash.clone(),
        content_hash,
        source_type: match &input.source {
            ImageSource::Local(_) => "local".to_string(),
            ImageSource::Remote(_) => "remote".to_string(),
        },
        source: source_str,
        has_transparency,
        original_width: img.width(),
        original_height: img.height(),
    };
    handle.block_on(async {
        cache.upsert(cache_entry).await
    })?;

    Ok(SmartImageOutput {
        hash: resource_hash,
        variants,
        blur_placeholder,
        metadata,
    })
}
```

### Helper Functions

```rust
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::FilterType};
use xxhash_rust::xxh3::xxh3_64;

/// Load bytes from local file or remote URL.
fn load_source_bytes(source: &ImageSource) -> Result<(Vec<u8>, String)> {
    match source {
        ImageSource::Local(path) => {
            let bytes = std::fs::read(path)?;
            Ok((bytes, path.to_string_lossy().to_string()))
        }
        ImageSource::Remote(url) => {
            // Note: Actual implementation would use reqwest
            // This is synchronous; consider async in production
            unimplemented!("Remote fetching requires async HTTP client")
        }
    }
}

/// Compute XXH3-64 hash of image content.
fn compute_content_hash(bytes: &[u8]) -> String {
    format!("{:016x}", xxh3_64(bytes))
}

/// Detect if image has transparency (alpha channel with non-opaque pixels).
fn detect_transparency(img: &DynamicImage) -> bool {
    match img {
        DynamicImage::ImageRgba8(rgba) => {
            rgba.pixels().any(|p| p.0[3] < 255)
        }
        DynamicImage::ImageRgba16(rgba) => {
            rgba.pixels().any(|p| p.0[3] < 65535)
        }
        _ => false,
    }
}

/// Generate all size/format variants.
fn generate_variants(
    img: &DynamicImage,
    hash: &str,
    output_dir: &Path,
    widths: &ImageWidths,
    has_transparency: bool,
) -> Result<Vec<ImageVariant>> {
    let original_width = img.width();
    let original_height = img.height();

    // Determine output formats
    let fallback = OutputFormat::fallback_for_source(has_transparency);
    let formats = vec![OutputFormat::Avif, OutputFormat::WebP, fallback];

    let mut variants = Vec::new();

    for (size, target_width) in widths.as_vec() {
        // Never upsample: use original dimensions if smaller
        let (actual_width, actual_height) = if target_width >= original_width {
            (original_width, original_height)
        } else {
            let scale = target_width as f64 / original_width as f64;
            let height = (original_height as f64 * scale).round() as u32;
            (target_width, height)
        };

        // Resize (or clone if no resize needed)
        let resized = if actual_width == original_width {
            img.clone()
        } else {
            img.resize(actual_width, actual_height, FilterType::Lanczos3)
        };

        // Generate each format
        for format in &formats {
            let filename = format!("{}{}.{}", hash, size.suffix(), format.extension());
            let path = output_dir.join(&filename);

            save_image(&resized, &path, *format, has_transparency)?;

            variants.push(ImageVariant {
                size,
                format: *format,
                path,
                width: actual_width,
                height: actual_height,
            });
        }
    }

    Ok(variants)
}

/// Save image in specified format.
fn save_image(
    img: &DynamicImage,
    path: &Path,
    format: OutputFormat,
    has_transparency: bool,
) -> Result<()> {
    let img_format = match format {
        OutputFormat::Avif => ImageFormat::Avif,
        OutputFormat::WebP => ImageFormat::WebP,
        OutputFormat::Jpeg => ImageFormat::Jpeg,
        OutputFormat::Png => ImageFormat::Png,
    };

    // For JPEG, convert to RGB (drop alpha)
    if format == OutputFormat::Jpeg && has_transparency {
        let rgb = img.to_rgb8();
        rgb.save_with_format(path, img_format)
            .map_err(|e| SmartImageError::SaveFailed { path: path.to_owned(), source: e })?;
    } else {
        img.save_with_format(path, img_format)
            .map_err(|e| SmartImageError::SaveFailed { path: path.to_owned(), source: e })?;
    }

    Ok(())
}

/// Generate 32x32 blurred placeholder image.
fn generate_blur_placeholder(
    img: &DynamicImage,
    hash: &str,
    output_dir: &Path,
) -> Result<Option<PathBuf>> {
    // Scale down to 32x32
    let tiny = img.thumbnail(32, 32);

    // Apply blur (sigma ~2.0)
    let blurred = tiny.blur(2.0);

    let path = output_dir.join(format!("{}-blur.jpg", hash));

    blurred.to_rgb8()
        .save_with_format(&path, ImageFormat::Jpeg)
        .map_err(|e| SmartImageError::SaveFailed { path: path.clone(), source: e })?;

    Ok(Some(path))
}
```

### Metadata Extraction

```rust
use kamadak_exif::{Reader, Tag, In};
use std::io::{BufReader, Cursor};

/// Extract EXIF metadata from image bytes.
fn extract_metadata(bytes: &[u8]) -> Option<ImageMetadata> {
    let mut cursor = Cursor::new(bytes);
    let exif = Reader::new().read_from_container(&mut cursor).ok()?;

    let get_string = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY)
            .map(|f| f.display_value().to_string())
    };

    let get_gps = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY)
            .and_then(|f| {
                // GPS parsing is complex; simplified here
                None::<f64>
            })
    };

    // Get original dimensions from EXIF or return None
    let width = exif.get_field(Tag::ImageWidth, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(0);
    let height = exif.get_field(Tag::ImageLength, In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(0);

    Some(ImageMetadata {
        make: get_string(Tag::Make),
        model: get_string(Tag::Model),
        date_time: get_string(Tag::DateTimeOriginal),
        latitude: get_gps(Tag::GPSLatitude),
        longitude: get_gps(Tag::GPSLongitude),
        description: get_string(Tag::ImageDescription),
        original_width: width,
        original_height: height,
    })
}
```

## Thread Pool Configuration

```rust
use rayon::ThreadPoolBuilder;

/// Initialize Rayon thread pool for image processing.
/// Limits threads to avoid memory exhaustion with large images.
pub fn init_image_thread_pool(max_threads: Option<usize>) -> Result<(), rayon::ThreadPoolBuildError> {
    let num_threads = max_threads.unwrap_or_else(|| {
        // Default: use available cores, but cap at 8 for memory safety
        std::thread::available_parallelism()
            .map(|n| n.get().min(8))
            .unwrap_or(4)
    });

    ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(|i| format!("smart-image-{}", i))
        .build_global()
}
```

## Generated File Structure

For an input image that hashes to `a1b2c3d4e5f67890`:

```
{{output_dir}}/images/
├── a1b2c3d4e5f67890-micro.avif
├── a1b2c3d4e5f67890-micro.webp
├── a1b2c3d4e5f67890-micro.jpg
├── a1b2c3d4e5f67890-xs.avif
├── a1b2c3d4e5f67890-xs.webp
├── a1b2c3d4e5f67890-xs.jpg
├── a1b2c3d4e5f67890-sm.avif
├── a1b2c3d4e5f67890-sm.webp
├── a1b2c3d4e5f67890-sm.jpg
├── a1b2c3d4e5f67890-md.avif
├── a1b2c3d4e5f67890-md.webp
├── a1b2c3d4e5f67890-md.jpg
├── a1b2c3d4e5f67890-lg.avif
├── a1b2c3d4e5f67890-lg.webp
├── a1b2c3d4e5f67890-lg.jpg
├── a1b2c3d4e5f67890-xl.avif
├── a1b2c3d4e5f67890-xl.webp
├── a1b2c3d4e5f67890-xl.jpg
├── a1b2c3d4e5f67890-2xl.avif
├── a1b2c3d4e5f67890-2xl.webp
├── a1b2c3d4e5f67890-2xl.jpg
└── a1b2c3d4e5f67890-blur.jpg
```

For PNG inputs with transparency, `.jpg` becomes `.png`.

## HTML Output Generation

### The `sizes` Attribute Problem

The `srcset` attribute tells the browser what image widths are available, but the browser also needs to know **how large the image will be rendered** to pick the right source. This is the role of the `sizes` attribute.

**Without `sizes`**: The browser defaults to `100vw` (full viewport width), causing it to download images far larger than necessary when images don't span the full viewport.

**The fundamental challenge**: Images are preloaded before CSS/JavaScript is parsed, so the browser doesn't know the actual layout size when it makes the image request. The `sizes` attribute provides hints based on viewport width.

### Responsive Image Priority Order

The `<picture>` element enables a two-level selection process:

1. **Format Selection** (via `<source type="...">`) - Browser picks the first `<source>` with a MIME type it supports (AVIF → WebP → fallback)
2. **Size Selection** (via `srcset` + `sizes`) - Within the selected source, browser picks the best width based on viewport and device pixel ratio

### The `sizes="auto"` Progressive Enhancement

For **lazy-loaded images** (`loading="lazy"`), browsers can use the actual rendered size since layout is complete before the image loads. The HTML specification now supports `sizes="auto"`:

| Browser | `sizes="auto"` Support |
|---------|------------------------|
| Chrome/Edge | 126+ (July 2024) |
| Opera | 112+ |
| Firefox | Not yet (positive stance) |
| Safari | Not yet (positive stance) |

**Strategy**: Use `sizes="auto, <fallback>"` syntax - browsers that support it use the actual rendered size; others fall back to the viewport-based hints.

### Container-Based Sizing (Future)

Currently, `sizes` can only reference **viewport width** (`vw`), not container width. There's an [active proposal (WHATWG #10182)](https://github.com/whatwg/html/issues/10182) for a `container` attribute on `<source>` elements, but it's not yet implemented due to the preload timing challenge.

**Current workaround**: Since we use `loading="lazy"`, the `sizes="auto"` feature provides container-aware sizing in supported browsers.

### Configuration Types

```rust
/// How the image is expected to render relative to its container.
#[derive(Debug, Clone)]
pub enum ImageLayout {
    /// Full width of container/viewport (default).
    FullWidth,
    /// Fixed pixel width.
    Fixed(u32),
    /// Percentage of container (maps to viewport % in sizes).
    Percentage(f32),
    /// Custom sizes attribute value.
    Custom(String),
}

/// Options for HTML generation.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Alt text for the image.
    pub alt: String,
    /// Expected layout of the image.
    pub layout: ImageLayout,
    /// Custom breakpoints for sizes attribute (optional override).
    pub size_hints: Option<Vec<(u32, String)>>,
    /// CSS class to add to the img element.
    pub class: Option<String>,
    /// Whether to use sizes="auto" (requires loading="lazy").
    pub use_auto_sizes: bool,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            alt: String::new(),
            layout: ImageLayout::FullWidth,
            size_hints: None,
            class: None,
            use_auto_sizes: true, // Progressive enhancement
        }
    }
}
```

### HTML Generation Implementation

```rust
/// Generate HTML picture element with proper srcset and sizes.
pub fn generate_picture_html(output: &SmartImageOutput, options: &HtmlOptions) -> String {
    let mut html = String::from("<picture>\n");

    // Generate sizes attribute value
    let sizes_value = generate_sizes_attribute(options);

    // Format priority: AVIF first (best compression), then WebP, then fallback
    // Within each format, srcset lists all available widths
    for format in [OutputFormat::Avif, OutputFormat::WebP] {
        let srcset = build_srcset(&output.variants, format);
        if !srcset.is_empty() {
            html.push_str(&format!(
                r#"  <source type="image/{}" srcset="{}" sizes="{}">"#,
                format.extension(),
                srcset,
                sizes_value
            ));
            html.push('\n');
        }
    }

    // Fallback img element (JPEG or PNG)
    let fallback_format = output.variants
        .iter()
        .find(|v| matches!(v.format, OutputFormat::Jpeg | OutputFormat::Png))
        .map(|v| v.format)
        .unwrap_or(OutputFormat::Jpeg);

    let fallback_srcset = build_srcset(&output.variants, fallback_format);
    let fallback_src = output.variants
        .iter()
        .filter(|v| v.format == fallback_format)
        .max_by_key(|v| v.width)
        .map(|v| v.path.display().to_string())
        .unwrap_or_default();

    let class_attr = options.class.as_ref()
        .map(|c| format!(r#" class="{}""#, c))
        .unwrap_or_default();

    html.push_str(&format!(
        r#"  <img src="{}" srcset="{}" sizes="{}" alt="{}"{} loading="lazy" decoding="async">"#,
        fallback_src,
        fallback_srcset,
        sizes_value,
        html_escape(&options.alt),
        class_attr
    ));
    html.push_str("\n</picture>");

    html
}

/// Build srcset string for a specific format, sorted by width ascending.
fn build_srcset(variants: &[ImageVariant], format: OutputFormat) -> String {
    let mut matching: Vec<_> = variants
        .iter()
        .filter(|v| v.format == format)
        .collect();

    // Sort by width ascending for proper srcset ordering
    matching.sort_by_key(|v| v.width);

    matching
        .iter()
        .map(|v| format!("{} {}w", v.path.display(), v.width))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate the sizes attribute based on layout options.
fn generate_sizes_attribute(options: &HtmlOptions) -> String {
    // If custom size hints are provided, use them
    if let Some(hints) = &options.size_hints {
        let parts: Vec<String> = hints
            .iter()
            .map(|(breakpoint, size)| format!("(max-width: {}px) {}", breakpoint, size))
            .collect();
        return parts.join(", ");
    }

    // Build sizes based on layout type
    let base_sizes = match &options.layout {
        ImageLayout::FullWidth => "100vw".to_string(),
        ImageLayout::Fixed(px) => format!("{}px", px),
        ImageLayout::Percentage(pct) => format!("{}vw", (pct * 100.0) as u32),
        ImageLayout::Custom(custom) => custom.clone(),
    };

    // Prepend "auto" for lazy-loaded images if enabled (progressive enhancement)
    // Browsers that support sizes="auto" will use actual rendered size;
    // others fall back to the viewport-based value
    if options.use_auto_sizes {
        format!("auto, {}", base_sizes)
    } else {
        base_sizes
    }
}

/// Escape HTML special characters in alt text.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

### Example Output

For an image with default options:

```html
<picture>
  <source type="image/avif" srcset="a1b2c3d4-micro.avif 320w, a1b2c3d4-xs.avif 640w, a1b2c3d4-sm.avif 1280w, a1b2c3d4-md.avif 1536w, a1b2c3d4-lg.avif 2048w, a1b2c3d4-xl.avif 2560w, a1b2c3d4-2xl.avif 3072w" sizes="auto, 100vw">
  <source type="image/webp" srcset="a1b2c3d4-micro.webp 320w, a1b2c3d4-xs.webp 640w, a1b2c3d4-sm.webp 1280w, a1b2c3d4-md.webp 1536w, a1b2c3d4-lg.webp 2048w, a1b2c3d4-xl.webp 2560w, a1b2c3d4-2xl.webp 3072w" sizes="auto, 100vw">
  <img src="a1b2c3d4-2xl.jpg" srcset="a1b2c3d4-micro.jpg 320w, a1b2c3d4-xs.jpg 640w, a1b2c3d4-sm.jpg 1280w, a1b2c3d4-md.jpg 1536w, a1b2c3d4-lg.jpg 2048w, a1b2c3d4-xl.jpg 2560w, a1b2c3d4-2xl.jpg 3072w" sizes="auto, 100vw" alt="A beautiful sunset" loading="lazy" decoding="async">
</picture>
```

For a sidebar image (30% width):

```rust
let options = HtmlOptions {
    alt: "Profile photo".to_string(),
    layout: ImageLayout::Percentage(0.30),
    ..Default::default()
};
```

```html
<picture>
  <source type="image/avif" srcset="..." sizes="auto, 30vw">
  <source type="image/webp" srcset="..." sizes="auto, 30vw">
  <img src="..." srcset="..." sizes="auto, 30vw" alt="Profile photo" loading="lazy" decoding="async">
</picture>
```

### How Browser Selection Works

1. **Format selection**: Browser evaluates `<source>` elements top-to-bottom, picks first with supported `type`
2. **Size calculation**: Browser parses `sizes` - if `auto` is supported and image is lazy-loaded, uses actual rendered width; otherwise uses viewport-based calculation
3. **Source selection**: Browser picks from `srcset` the image closest to (but not smaller than) the calculated size × device pixel ratio

## Format Support Matrix

| Input Format | Supported | Output Formats | Notes |
|--------------|-----------|----------------|-------|
| JPEG | Yes | AVIF, WebP, JPEG | Primary use case |
| PNG | Yes | AVIF, WebP, PNG | Preserves transparency |
| WebP | Yes | AVIF, WebP, JPEG/PNG | Based on alpha |
| GIF | Partial | GIF only | No format conversion; resize only if possible |
| AVIF | Yes | AVIF, WebP, JPEG/PNG | Based on alpha |
| PSD | No | - | Out of scope |

## Performance Considerations

1. **Parallel Processing**: Rayon's `par_iter()` distributes image processing across CPU cores
2. **Memory Limits**: Thread pool capped at 8 threads to prevent memory exhaustion with large images
3. **Caching**: SurrealDB cache prevents reprocessing unchanged images
4. **Hash Performance**: XXH3-64 provides ~31 GB/s hashing for fast cache key generation
5. **Lazy Loading**: Generated HTML includes `loading="lazy"` for deferred image loading
6. **Optimal Size Selection**: `sizes="auto"` enables container-aware image selection in modern browsers, preventing oversized downloads
7. **Format Priority**: AVIF sources listed first provide ~50% smaller files than JPEG in supporting browsers

## Browser Compatibility Notes

### `sizes="auto"` Polyfill

For broader browser support, consider the [Shopify autosizes polyfill](https://github.com/ACP-CODE/autosizes):

```html
<script src="https://unpkg.com/autosizes@1.0.0/dist/autosizes.min.js" defer></script>
```

This extends `sizes="auto"` support to Safari 14.1+ and Firefox 84+ (browsers with `PerformancePaintTiming` API).

### Fallback Behavior

| Browser State | Behavior |
|---------------|----------|
| Supports `sizes="auto"` | Uses actual rendered container width |
| No support, lazy image | Uses viewport-based `sizes` fallback |
| No support, eager image | Uses viewport-based `sizes` fallback |

The `auto, 100vw` syntax ensures graceful degradation - unsupported browsers ignore `auto` and use the fallback value.

## Future Enhancements

1. **Remote Image Fetching**: Implement async HTTP client for remote URLs
2. **GIF Support**: Investigate animated GIF resize support
3. **PSD Support**: Evaluate `psd` crate for Photoshop file input
4. **Metadata Writing**: Inject custom metadata into output images
5. **SIMD Optimization**: Enable AVX2/NEON via RUSTFLAGS for maximum decode performance
6. **Container Queries**: When [WHATWG #10182](https://github.com/whatwg/html/issues/10182) is implemented, add `container` attribute support for art direction based on container size

## References

- [MDN: Responsive Images](https://developer.mozilla.org/en-US/docs/Web/HTML/Guides/Responsive_images)
- [web.dev: Responsive Images](https://web.dev/learn/design/responsive-images)
- [Cloud Four: Container Queries and Responsive Images](https://cloudfour.com/thinks/on-container-queries-responsive-images-and-jpeg-xl/)
- [Stefan Judis: Should responsive images work with container queries?](https://www.stefanjudis.com/notes/should-responsive-images-work-with-container-queries/)
- [WordPress 6.7: Auto Sizes](https://make.wordpress.org/core/2024/10/18/auto-sizes-for-lazy-loaded-images-in-wordpress-6-7/)
