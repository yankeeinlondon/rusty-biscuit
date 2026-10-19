mod artifact_tests;
mod cache_tests;
#[cfg(feature = "image")]
mod gitgraph_tests;
#[cfg(feature = "image")]
mod graph_tests;
#[cfg(feature = "image")]
mod mermaid_tests;
#[cfg(feature = "image")]
mod raster_tests;

#[cfg(feature = "image")]
use tempfile::TempDir;

#[cfg(feature = "image")]
use crate::cache::file_cache::CACHE_DIR_ENV;

/// Redirects the process-global render cache into a fresh temp directory.
///
/// `GraphDiagram::render` / `MermaidDiagram::render` build a [`FileCache`] via
/// `FileCache::new`, which reads [`CACHE_DIR_ENV`]. Pointing that env var at a
/// per-test directory isolates each render test from the shared
/// `$TMPDIR/biscuit-visualized/v1/` location. Callers must be `#[serial]`:
/// the env var is process-global, so concurrent threads would clobber it.
///
/// The returned [`TempDir`] must outlive the render calls; dropping it deletes
/// the directory.
///
/// [`FileCache`]: crate::cache::FileCache
#[cfg(feature = "image")]
fn isolated_cache_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var(CACHE_DIR_ENV, dir.path());
    dir
}

/// Decodes a rendered PNG into straight (non-premultiplied) RGBA pixels,
/// returned as `(width, height, pixels)` in row-major order.
#[cfg(feature = "image")]
fn decode_png_straight(png: &[u8]) -> (u32, u32, Vec<[u8; 4]>) {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).expect("PNG should decode");
    let pixels = pixmap
        .pixels()
        .iter()
        .map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    (pixmap.width(), pixmap.height(), pixels)
}

/// Returns the user-unit size an SVG document declares, as the rasterizer
/// sees it.
#[cfg(feature = "image")]
fn svg_size(svg: &str) -> (f32, f32) {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .expect("SVG should parse");
    (tree.size().width(), tree.size().height())
}
