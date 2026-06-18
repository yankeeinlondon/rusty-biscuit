use crate::raster::{rasterize_svg_to_png, rasterize_svg_to_png_bytes};

/// Read the PNG IHDR chunk and return (width, height).
fn png_dims(bytes: &[u8]) -> (u32, u32) {
    assert!(bytes.len() >= 24, "PNG too short");
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (width, height)
}

const SQUARE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
    <rect width="100" height="50" fill="red"/>
</svg>"#;

#[test]
fn rasterize_at_width_produces_exact_pixel_width() {
    let png = rasterize_svg_to_png(SQUARE_SVG, 800).unwrap();
    let (w, h) = png_dims(&png);
    assert_eq!(w, 800, "PNG width should match requested target");
    // Height derived from aspect ratio (SVG is 2:1)
    assert_eq!(h, 400, "PNG height should preserve aspect ratio");
}

#[test]
fn rasterize_at_width_preserves_aspect_ratio_for_irregular_sizes() {
    let png = rasterize_svg_to_png(SQUARE_SVG, 350).unwrap();
    let (w, h) = png_dims(&png);
    assert_eq!(w, 350);
    assert_eq!(h, 175);
}

#[test]
fn rasterize_at_width_handles_minimum_target() {
    // Even at width=1, we should produce a valid 1×1 PNG (height clamps to 1).
    let png = rasterize_svg_to_png(SQUARE_SVG, 1).unwrap();
    let (w, h) = png_dims(&png);
    assert_eq!(w, 1);
    assert!(h >= 1, "height must be at least 1 pixel");
}

#[test]
fn rasterize_at_scale_still_works_for_legacy_callers() {
    let png = rasterize_svg_to_png_bytes(SQUARE_SVG, 2).unwrap();
    let (w, h) = png_dims(&png);
    // scale=2 → native (100×50) × 2 = 200×100
    assert_eq!(w, 200);
    assert_eq!(h, 100);
}

#[test]
fn rasterize_at_width_and_at_scale_agree_when_target_matches_scale() {
    // At width=200, target equals the scale=2 output for a 100-wide native SVG.
    let by_width = rasterize_svg_to_png(SQUARE_SVG, 200).unwrap();
    let by_scale = rasterize_svg_to_png_bytes(SQUARE_SVG, 2).unwrap();
    let (w_w, w_h) = png_dims(&by_width);
    let (s_w, s_h) = png_dims(&by_scale);
    assert_eq!((w_w, w_h), (s_w, s_h));
}
