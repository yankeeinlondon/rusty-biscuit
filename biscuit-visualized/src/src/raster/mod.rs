pub mod png;
pub use png::{rasterize_svg, rasterize_svg_to_png_bytes, RasterError};

/// Returns a sorted, deduplicated list of available system font family names.
///
/// Uses the same fontdb that resvg uses for rasterization, so the returned
/// names are guaranteed to be renderable.
pub fn available_font_families() -> Vec<String> {
    use std::collections::BTreeSet;

    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_system_fonts();

    let families: BTreeSet<String> = db
        .faces()
        .flat_map(|face| {
            face.families
                .iter()
                .map(|(name, _lang)| name.clone())
                .collect::<Vec<_>>()
        })
        .collect();

    families.into_iter().collect()
}
