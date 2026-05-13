pub mod png;
pub use png::{rasterize_svg, rasterize_svg_to_png, rasterize_svg_to_png_bytes, RasterError};

use std::sync::OnceLock;

use resvg::usvg::fontdb::Database;

fn system_font_database() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();

    DB.get_or_init(|| {
        let mut db = Database::new();
        db.load_system_fonts();
        db
    })
}

/// Returns a sorted, deduplicated list of available system font family names.
///
/// Uses the same fontdb that resvg uses for rasterization, so the returned
/// names are guaranteed to be renderable.
pub fn available_font_families() -> Vec<String> {
    use std::collections::BTreeSet;

    let families: BTreeSet<String> = system_font_database()
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

pub(crate) fn cloned_system_font_database() -> Database {
    system_font_database().clone()
}
