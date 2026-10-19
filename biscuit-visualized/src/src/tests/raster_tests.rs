use crate::raster::{rasterize_svg_to_png, rasterize_svg_to_png_bytes};
use crate::tests::decode_png_straight;

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

#[test]
fn rasterize_at_width_rounds_height_for_non_integer_aspect_ratio() {
    // 333×77 at 1000 px wide is 231.23 px tall: the height must stay within
    // one pixel of the exact ratio, never stretch or squash the drawing.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="333" height="77" viewBox="0 0 333 77">
        <rect width="333" height="77" fill="blue"/>
    </svg>"#;
    let png = rasterize_svg_to_png(svg, 1000).unwrap();
    let (w, h) = png_dims(&png);
    assert_eq!(w, 1000);
    let exact = 1000.0 * 77.0 / 333.0;
    assert!(
        (h as f64 - exact).abs() <= 1.0,
        "height {h} should be within one pixel of {exact}"
    );
}

#[test]
fn rasterize_leaves_uncovered_pixels_transparent_and_covered_pixels_opaque() {
    // Only the left half is painted; the right half has no background.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
        <rect width="50" height="50" fill="rgb(255,0,0)"/>
    </svg>"#;
    for png in [
        rasterize_svg_to_png(svg, 200).unwrap(),
        rasterize_svg_to_png_bytes(svg, 2).unwrap(),
    ] {
        let (w, _h, pixels) = decode_png_straight(&png);
        let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
        assert_eq!(at(40, 40), [255, 0, 0, 255], "painted half must be opaque red");
        assert_eq!(at(160, 40)[3], 0, "unpainted half must stay fully transparent");
    }
}

#[test]
fn rasterize_partially_transparent_shape_keeps_its_straight_color() {
    // A half-opaque fill must decode to its own color at ~50% alpha. A
    // premultiplied value leaking through would read as a darker color.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20">
        <rect width="20" height="20" fill="rgb(200,100,50)" fill-opacity="0.5"/>
    </svg>"#;
    let png = rasterize_svg_to_png(svg, 20).unwrap();
    let (w, _h, pixels) = decode_png_straight(&png);
    let [r, g, b, a] = pixels[(10 * w + 10) as usize];
    assert!((127..=128).contains(&a), "alpha {a} should be half of 255");
    for (actual, expected) in [(r, 200u8), (g, 100), (b, 50)] {
        assert!(
            actual.abs_diff(expected) <= 2,
            "channel {actual} should be within rounding of {expected}"
        );
    }
}

#[test]
fn rasterize_rejects_unparseable_svg() {
    assert!(rasterize_svg_to_png("<not-an-svg>", 100).is_err());
    assert!(rasterize_svg_to_png_bytes("<not-an-svg>", 1).is_err());
}
