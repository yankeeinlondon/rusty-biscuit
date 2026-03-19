use crate::artifact::{OutputFormat, RenderRequest};

#[test]
fn default_render_request() {
    let request = RenderRequest::default();

    assert_eq!(request.format, OutputFormat::Png);
    assert_eq!(request.scale, 2);
    assert!(!request.transparent_background);
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
