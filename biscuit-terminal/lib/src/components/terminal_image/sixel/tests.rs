use super::*;
use image::{Rgba, RgbaImage};

const BLACK: RgbValue = RgbValue { r: 0, g: 0, b: 0 };
const WHITE: RgbValue = RgbValue {
    r: 255,
    g: 255,
    b: 255,
};

/// A 2x2 fixture: red, green / blue, and a fully transparent pixel.
fn quad() -> DynamicImage {
    let mut img = RgbaImage::new(2, 2);
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
    img.put_pixel(0, 1, Rgba([0, 0, 255, 255]));
    img.put_pixel(1, 1, Rgba([200, 200, 200, 0]));
    DynamicImage::ImageRgba8(img)
}

/// A gradient with far more than 256 colors.
fn gradient() -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_fn(40, 30, |x, y| {
        Rgba([(x * 6) as u8, (y * 8) as u8, ((x + y) * 3) as u8, 255])
    }))
}

/// The palette entries (`#n;2;r;g;b`) a Sixel string defines, in percent.
fn palette(sixel: &str) -> Vec<(u32, u32, u32)> {
    sixel
        .split('#')
        .skip(1)
        .filter_map(|entry| {
            let fields: Vec<&str> = entry.split(';').collect();
            (fields.len() == 5 && fields[1] == "2").then(|| {
                let pct = |f: &str| f.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().unwrap();
                (pct(fields[2]), pct(fields[3]), pct(fields[4]))
            })
        })
        .collect()
}

/// The RGBA pixel at (`x`, `y`) of a decoded Sixel string; alpha 0 means no
/// color set a bit there.
fn decoded_pixel(sixel: &str, x: usize, y: usize) -> [u8; 4] {
    let image = SixelImage::decode(sixel.as_bytes()).expect("decodes");
    let at = (y * image.width + x) * 4;
    image.pixels[at..at + 4].try_into().unwrap()
}

#[test]
fn a_fixed_fixture_encodes_to_golden_bytes() {
    // Palette: red, blue, green; the transparent pixel sets no bit in any
    // color. One band, each color's column bits then `$`.
    let sixel = encode_sixel(&quad(), (2, 2), 2, BLACK).unwrap();
    assert_eq!(
        sixel,
        "\x1bP9;1;0q\"1;1;2;2#0;2;100;0;0#1;2;0;100;0#2;2;0;0;100#0@?$#1?@$#2A?$-\x1b\\"
    );
}

#[test]
fn encoding_is_deterministic_and_the_palette_is_bounded() {
    let first = encode_sixel(&gradient(), (40, 30), 32, BLACK).unwrap();
    let second = encode_sixel(&gradient(), (40, 30), 32, BLACK).unwrap();
    assert_eq!(first, second);
    let colors = palette(&first).len();
    assert!(colors > 2 && colors <= usize::from(SIXEL_PALETTE_COLORS), "{colors} colors");
}

#[test]
fn a_two_color_image_gains_no_dithering_colors() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_fn(12, 12, |x, _| {
        if x < 6 { Rgba([255, 255, 255, 255]) } else { Rgba([0, 0, 0, 255]) }
    }));
    let sixel = encode_sixel(&img, (12, 12), 12, BLACK).unwrap();
    let mut colors = palette(&sixel);
    colors.sort_unstable();
    colors.dedup();
    assert_eq!(colors, vec![(0, 0, 0), (100, 100, 100)]);
}

#[test]
fn a_transparent_pixel_sets_no_bits_and_opaque_pixels_are_unchanged() {
    for background in [BLACK, WHITE] {
        let sixel = encode_sixel(&quad(), (2, 2), 2, background).unwrap();
        assert_eq!(decoded_pixel(&sixel, 1, 1)[3], 0, "transparent pixel left unset: {sixel:?}");
        assert_eq!(decoded_pixel(&sixel, 0, 0), [255, 0, 0, 255]);
        assert_eq!(decoded_pixel(&sixel, 1, 0), [0, 255, 0, 255]);
        assert_eq!(decoded_pixel(&sixel, 0, 1), [0, 0, 255, 255]);
        let level = u32::from(background.r) * 100 / 255;
        assert!(
            !palette(&sixel).contains(&(level, level, level)),
            "the background is never painted: {sixel:?}"
        );
    }
}

#[test]
fn partial_transparency_is_blended_onto_the_background() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 128])));
    let on_white = encode_sixel(&img, (1, 1), 1, WHITE).unwrap();
    assert_eq!(palette(&on_white), vec![(100, 50, 50)], "{on_white:?}");
    assert_eq!(decoded_pixel(&on_white, 0, 0)[3], 255, "drawn opaque");
    let on_black = encode_sixel(&img, (1, 1), 1, BLACK).unwrap();
    assert_eq!(palette(&on_black), vec![(50, 0, 0)], "{on_black:?}");
}

#[test]
fn half_transparent_pixels_blend_with_the_background() {
    assert_eq!(blend(255, 0, 255), 255);
    assert_eq!(blend(255, 0, 0), 0);
    assert_eq!(blend(255, 0, 128), 128);
    assert_eq!(blend(0, 255, 128), 127);
    assert_eq!(blend(10, 10, 77), 10);
}

#[test]
fn the_raster_is_padded_to_whole_rows_with_unset_pixels() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 3, Rgba([255, 0, 0, 255])));
    let sixel = encode_sixel(&img, (4, 3), 8, WHITE).unwrap();
    assert!(sixel.contains("\"1;1;4;8"), "raster attributes name the padded size: {sixel:?}");
    assert_eq!(palette(&sixel), vec![(100, 0, 0)], "the padding paints no color: {sixel:?}");
    assert_eq!(decoded_pixel(&sixel, 0, 2), [255, 0, 0, 255]);
    assert_eq!(decoded_pixel(&sixel, 0, 3)[3], 0, "{sixel:?}");
    assert_eq!(decoded_pixel(&sixel, 3, 7)[3], 0, "{sixel:?}");
}

#[test]
fn the_image_is_resized_to_the_display_size() {
    let sixel = encode_sixel(&gradient(), (20, 15), 16, BLACK).unwrap();
    assert!(sixel.contains("\"1;1;20;16"), "{sixel:?}");
}

#[test]
fn the_sequence_is_one_square_pixel_dcs_and_sets_no_mode() {
    let sixel = encode_sixel(&gradient(), (40, 30), 32, BLACK).unwrap();
    assert!(sixel.starts_with("\x1bP9;1;0q"), "square pixels, transparent P2: {sixel:?}");
    assert!(sixel.ends_with("\x1b\\"));
    assert_eq!(sixel.matches('\x1b').count(), 2, "one DCS, nothing else");
}

#[test]
fn scaled_height_rounds_and_is_never_zero() {
    assert_eq!(scaled_height(80, (40, 30)), 60);
    assert_eq!(scaled_height(10, (3, 1)), 3);
    assert_eq!(scaled_height(10, (6, 1)), 2);
    assert_eq!(scaled_height(1, (1000, 1)), 1);
    assert_eq!(scaled_height(5, (0, 4)), 20, "a zero width counts as one pixel");
}

mod background {
    use super::*;
    use crate::discovery::detection::ColorMode;

    fn terminal(background: Option<RgbValue>, source: ColorSource, mode: ColorMode) -> Terminal {
        Terminal {
            background_color: background,
            background_source: source,
            color_mode: mode,
            ..Terminal::default()
        }
    }

    #[test]
    fn a_measured_background_is_used_as_is() {
        let measured = RgbValue { r: 30, g: 40, b: 50 };
        for mode in [ColorMode::Light, ColorMode::Dark, ColorMode::Unknown] {
            assert_eq!(
                SixelBackground::for_terminal(&terminal(Some(measured), ColorSource::Measured, mode)),
                SixelBackground {
                    color: measured,
                    source: BackgroundSource::Measured
                }
            );
        }
    }

    /// The live defect: under tmux the per-app default `#1e1e1e` reached
    /// `background_color` with no OSC 11 reply and was labelled measured.
    #[test]
    fn an_unreported_background_is_an_estimate_not_a_measurement() {
        let guess = RgbValue { r: 30, g: 30, b: 30 };
        assert_eq!(
            SixelBackground::for_terminal(&terminal(Some(guess), ColorSource::Estimated, ColorMode::Dark)),
            SixelBackground {
                color: guess,
                source: BackgroundSource::Estimated
            }
        );
    }

    #[test]
    fn without_any_color_the_color_mode_decides_the_estimate() {
        let estimate = |mode| SixelBackground::for_terminal(&terminal(None, ColorSource::Estimated, mode));
        assert_eq!(
            estimate(ColorMode::Light),
            SixelBackground {
                color: WHITE,
                source: BackgroundSource::Estimated
            }
        );
        for mode in [ColorMode::Dark, ColorMode::Unknown] {
            assert_eq!(
                estimate(mode),
                SixelBackground {
                    color: BLACK,
                    source: BackgroundSource::Estimated
                }
            );
        }
    }
}
