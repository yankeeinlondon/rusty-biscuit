use crate::artifact::{Background, OutputFormat, RenderRequest};

#[test]
fn default_render_request() {
    let request = RenderRequest::default();

    assert_eq!(request.format, OutputFormat::Png);
    assert_eq!(request.scale, 2);
    assert_eq!(request.background, Background::Theme);
    assert_eq!(request.row_height, None);
}

#[test]
fn output_format_extension() {
    assert_eq!(OutputFormat::Svg.extension(), "svg");
    assert_eq!(OutputFormat::Png.extension(), "png");
}

#[test]
fn output_format_equality() {
    assert_eq!(OutputFormat::Svg, OutputFormat::Svg);
    assert_eq!(OutputFormat::Png, OutputFormat::Png);
    assert_ne!(OutputFormat::Svg, OutputFormat::Png);
}

#[test]
fn only_a_solid_background_has_a_color() {
    assert_eq!(
        Background::Solid { r: 0x12, g: 0xab, b: 0x0f }.solid_hex().as_deref(),
        Some("#12ab0f")
    );
    assert_eq!(Background::Theme.solid_hex(), None);
    assert_eq!(Background::Transparent.solid_hex(), None);
}

/// Everything that changes an artifact's bytes is in its cache identity.
#[test]
fn the_cache_identity_covers_background_geometry_and_encoding() {
    let base = RenderRequest::default().with_target_width(240);
    let variants = [
        base.clone(),
        base.clone().with_background(Background::Transparent),
        base.clone().with_background(Background::Solid { r: 0, g: 0, b: 0 }),
        base.clone().with_background(Background::Solid { r: 255, g: 255, b: 255 }),
        base.clone().with_row_height(16),
        base.clone().with_row_height(20),
        base.clone().with_target_width(241),
        RenderRequest { scale: 3, ..RenderRequest::default() },
    ];
    let identities: Vec<_> = variants.iter().map(RenderRequest::cache_identity).collect();
    for (i, a) in identities.iter().enumerate() {
        for b in &identities[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert_eq!(base.cache_identity(), base.clone().cache_identity());

    assert_eq!(
        base.cache_identity()["png_encoding"],
        crate::artifact::PNG_ENCODING
    );
    let svg = RenderRequest { format: OutputFormat::Svg, ..base };
    assert!(svg.cache_identity()["png_encoding"].is_null());
}
