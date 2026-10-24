pub mod png;
pub use png::{rasterize_svg, rasterize_svg_to_png, rasterize_svg_to_png_bytes, RasterError};

use std::sync::Arc;

use resvg::usvg;

use crate::fonts::{FontChain, FontQuery, default_chain};

/// Returns the sorted, deduplicated font family names the rasterizer can
/// draw by name: every installed family plus the bundled `Inter`.
///
/// Reads the same font database the rasterizer draws with, so every
/// returned name is renderable. The bundled Noto Emoji (Linux builds) is a
/// fallback face only and is not listed.
pub fn available_font_families() -> Vec<String> {
    default_chain().family_names()
}

/// usvg options whose font database is `chain`'s and whose `FontResolver`
/// asks `chain`, so every `<text>` is drawn in the face the chain resolves
/// and missing characters fall back in the chain's order.
pub(crate) fn usvg_options(chain: &Arc<FontChain>) -> usvg::Options<'static> {
    let mut options = usvg::Options {
        fontdb: chain.drawing_database(),
        ..usvg::Options::default()
    };
    let select_chain = Arc::clone(chain);
    let fallback_chain = Arc::clone(chain);
    options.font_resolver = usvg::FontResolver {
        select_font: Box::new(move |font, _db| select_chain.resolve(&FontQuery::from_usvg(font))),
        select_fallback: Box::new(move |c, already_tried, _db| {
            fallback_chain.fallback(c, already_tried)
        }),
    };
    options
}
