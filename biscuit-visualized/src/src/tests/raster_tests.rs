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

mod requests {
    use crate::artifact::{Background, RenderRequest};
    use crate::raster::png::rasterize_svg_for_request;

    use super::{SQUARE_SVG, png_dims};

    /// A 100x50 SVG whose bottom row of pixels is drawn, so the end of the
    /// drawing is visible in the raster.
    const BANDED_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
        <rect width="100" height="50" fill="#0000ff"/>
    </svg>"##;

    fn rgba(png: &[u8]) -> image::RgbaImage {
        image::load_from_memory(png).unwrap().to_rgba8()
    }

    #[test]
    fn the_height_is_padded_up_to_whole_rows() {
        let request = RenderRequest::default().with_target_width(30).with_row_height(16);
        // 30 px wide is 15 px tall: one 16 px row.
        assert_eq!(png_dims(&rasterize_svg_for_request(SQUARE_SVG, &request).unwrap()), (30, 16));
        // 64 px wide is 32 px tall: already two whole rows, no padding.
        let exact = request.clone().with_target_width(64);
        assert_eq!(png_dims(&rasterize_svg_for_request(SQUARE_SVG, &exact).unwrap()), (64, 32));
        // Without a row height the drawing's own height stands.
        let unpadded = RenderRequest::default().with_target_width(30);
        assert_eq!(png_dims(&rasterize_svg_for_request(SQUARE_SVG, &unpadded).unwrap()), (30, 15));
    }

    /// Padding is added below the drawing, which keeps its aspect ratio:
    /// the drawn rows equal the unpadded raster's, nothing is stretched.
    #[test]
    fn padding_sits_below_an_unstretched_drawing() {
        let unpadded = rgba(
            &rasterize_svg_for_request(BANDED_SVG, &RenderRequest::default().with_target_width(30))
                .unwrap(),
        );
        for (background, padding) in [
            (Background::Transparent, image::Rgba([0, 0, 0, 0])),
            (Background::Theme, image::Rgba([0, 0, 0, 0])),
            (Background::Solid { r: 10, g: 20, b: 30 }, image::Rgba([10, 20, 30, 255])),
        ] {
            let request = RenderRequest::default()
                .with_target_width(30)
                .with_row_height(16)
                .with_background(background);
            let padded = rgba(&rasterize_svg_for_request(BANDED_SVG, &request).unwrap());

            assert_eq!(padded.dimensions(), (30, 16), "{background:?}");
            for x in 0..30 {
                for y in 0..15 {
                    assert_eq!(padded.get_pixel(x, y), unpadded.get_pixel(x, y), "{background:?}");
                }
                assert_eq!(*padded.get_pixel(x, 15), padding, "{background:?}");
            }
        }
    }

    /// The rasterizer's encoding keeps every pixel of tiny-skia's own encoder
    /// and is smaller.
    #[test]
    fn the_encoding_is_lossless_and_smaller_than_tiny_skias() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120" viewBox="0 0 200 120">
            <defs><linearGradient id="g"><stop offset="0" stop-color="#123"/><stop offset="1" stop-color="#fa0" stop-opacity="0.4"/></linearGradient></defs>
            <rect width="200" height="120" fill="url(#g)"/>
            <circle cx="60" cy="60" r="40" fill="#0a8" fill-opacity="0.7"/>
            <path d="M10 110 L190 10" stroke="#fff" stroke-width="3"/>
        </svg>"##;
        let ours = rasterize_svg_for_request(svg, &RenderRequest::default().with_target_width(400)).unwrap();

        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(400, 240).unwrap();
        resvg::render(&tree, resvg::usvg::Transform::from_scale(2.0, 2.0), &mut pixmap.as_mut());
        let theirs = pixmap.encode_png().unwrap();

        assert_eq!(rgba(&ours), rgba(&theirs));
        assert!(ours.len() < theirs.len(), "{} >= {}", ours.len(), theirs.len());
    }
}
