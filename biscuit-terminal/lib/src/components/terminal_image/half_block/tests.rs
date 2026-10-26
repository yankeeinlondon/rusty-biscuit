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

fn solid(width: u32, height: u32, color: [u8; 4]) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba(color)))
}

#[test]
fn a_fixed_fixture_encodes_to_golden_lines() {
    // Upper pixel in the foreground, lower in the background; the
    // transparent pixel uses the terminal's default background (SGR 49).
    let lines = encode_half_blocks(&quad(), (2, 2), BLACK);
    assert_eq!(
        lines,
        vec!["\x1b[38;2;255;0;0;48;2;0;0;255m▀\x1b[38;2;0;255;0;49m▀\x1b[0m".to_string()]
    );
}

#[test]
fn a_transparent_pixel_is_never_painted_with_the_background() {
    // The background is a guess; whatever it is, a transparent pixel shows
    // the terminal's own background.
    for background in [BLACK, WHITE] {
        let lines = encode_half_blocks(&quad(), (2, 2), background);
        assert!(lines[0].ends_with("\x1b[38;2;0;255;0;49m▀\x1b[0m"), "{lines:?}");
    }
}

#[test]
fn a_transparent_upper_pixel_draws_the_lower_half_block() {
    let mut img = RgbaImage::new(1, 2);
    img.put_pixel(0, 0, Rgba([9, 9, 9, 0]));
    img.put_pixel(0, 1, Rgba([10, 20, 30, 255]));
    let lines = encode_half_blocks(&DynamicImage::ImageRgba8(img), (1, 2), WHITE);
    assert_eq!(lines, vec!["\x1b[38;2;10;20;30;49m▄\x1b[0m".to_string()]);
}

#[test]
fn a_fully_transparent_cell_is_a_space_on_the_default_background() {
    let lines = encode_half_blocks(&solid(2, 2, [255, 0, 0, 0]), (2, 2), WHITE);
    assert_eq!(lines, vec!["\x1b[49m  \x1b[0m".to_string()]);
}

#[test]
fn partial_transparency_is_blended_onto_the_background() {
    let half = encode_half_blocks(&solid(1, 2, [255, 0, 0, 128]), (1, 2), BLACK);
    assert_eq!(half, vec!["\x1b[38;2;128;0;0;48;2;128;0;0m▀\x1b[0m".to_string()]);
    let on_white = encode_half_blocks(&solid(1, 2, [255, 0, 0, 128]), (1, 2), WHITE);
    assert_eq!(on_white, vec!["\x1b[38;2;255;127;127;48;2;255;127;127m▀\x1b[0m".to_string()]);
}

#[test]
fn an_odd_last_pixel_row_has_the_default_background_below() {
    let lines = encode_half_blocks(&solid(1, 3, [10, 20, 30, 255]), (1, 3), WHITE);
    assert_eq!(
        lines,
        vec![
            "\x1b[38;2;10;20;30;48;2;10;20;30m▀\x1b[0m".to_string(),
            "\x1b[38;2;10;20;30;49m▀\x1b[0m".to_string(),
        ]
    );
}

#[test]
fn a_repeated_color_is_written_once_per_run() {
    let lines = encode_half_blocks(&solid(3, 2, [1, 2, 3, 255]), (3, 2), BLACK);
    assert_eq!(lines, vec!["\x1b[38;2;1;2;3;48;2;1;2;3m▀▀▀\x1b[0m".to_string()]);
}

#[test]
fn the_image_is_resized_to_the_requested_cells() {
    let lines = encode_half_blocks(&solid(40, 40, [9, 9, 9, 255]), (5, 6), BLACK);
    assert_eq!(lines.len(), 3, "6 pixel rows are 3 text rows");
    for line in &lines {
        assert_eq!(line.matches('▀').count(), 5, "{line:?}");
    }
}

#[test]
fn pixel_rows_keep_the_aspect_ratio() {
    let fallback = CellSize::FALLBACK;
    assert_eq!((fallback.width, fallback.height), (8, 16), "the cases below assume an 8x16 cell");
    // A square image on a 1:2 cell: one half-block pixel per column.
    assert_eq!(half_block_pixel_rows(10, (100, 100), fallback), 10);
    // A 4:1 image: 10 columns show 2.5 pixel rows, rounded to 3.
    assert_eq!(half_block_pixel_rows(10, (200, 50), fallback), 3);
    // A measured, wider cell (10x20 is still 1:2).
    assert_eq!(half_block_pixel_rows(10, (100, 100), CellSize { width: 10, height: 20 }), 10);
    // A 1:3 cell makes each half-block pixel taller, so fewer rows.
    assert_eq!(half_block_pixel_rows(12, (100, 100), CellSize { width: 8, height: 24 }), 8);
    // Never zero, even for a very wide image.
    assert_eq!(half_block_pixel_rows(1, (10_000, 1), fallback), 1);
}

mod permission {
    use super::*;

    fn term(is_tty: bool, depth: ColorDepth, unicode: bool) -> Terminal {
        Terminal::builder()
            .is_tty(is_tty)
            .color_depth(depth)
            .supports_unicode(unicode)
            .build()
    }

    fn reason(kind: RouteReasonKind) -> RouteReason {
        RouteReason {
            kind,
            message: String::new(),
            fix: None,
        }
    }

    #[test]
    fn needs_a_truecolor_unicode_terminal() {
        let unknown = reason(RouteReasonKind::Unknown);
        assert!(half_blocks_permitted(&term(true, ColorDepth::TrueColor, true), &unknown));
        for depth in [ColorDepth::Enhanced, ColorDepth::Basic, ColorDepth::Minimal, ColorDepth::None] {
            assert!(!half_blocks_permitted(&term(true, depth, true), &unknown), "{depth:?}");
        }
        assert!(!half_blocks_permitted(&term(true, ColorDepth::TrueColor, false), &unknown), "no Unicode");
        assert!(!half_blocks_permitted(&term(false, ColorDepth::TrueColor, true), &unknown), "not a tty");
    }

    #[test]
    fn follows_the_reason_the_pixel_route_failed() {
        let capable = term(true, ColorDepth::TrueColor, true);
        for kind in [
            RouteReasonKind::NoSupport,
            RouteReasonKind::Unknown,
            RouteReasonKind::BlockedTransport,
            RouteReasonKind::Supported,
        ] {
            assert!(half_blocks_permitted(&capable, &reason(kind)), "{kind:?}");
        }
        for kind in [
            RouteReasonKind::UserDisabled,
            RouteReasonKind::InvalidInput,
            RouteReasonKind::NotATerminal,
        ] {
            assert!(!half_blocks_permitted(&capable, &reason(kind)), "{kind:?}");
        }
    }
}

/// [`TerminalImage::render_with_half_blocks`] and the inline path, end to end
/// from an image file and a resolved route.
mod rendering {
    use super::*;
    use crate::components::renderable::TerminalRenderable;
    use crate::components::terminal_image::{ImageOutput, ImageWidth, InlineImage};
    use crate::discovery::detection::{
        GraphicsRoute, RouteInputs, TerminalApp, resolve_graphics_route,
    };
    use crate::utils::layout::{Alignment, Layout};

    fn route_for(output_is_tty: bool, vars: &[(&str, &str)]) -> GraphicsRoute {
        let inputs = vars
            .iter()
            .fold(RouteInputs::new(output_is_tty), |inputs, (k, v)| inputs.with_var(k, *v));
        resolve_graphics_route(&inputs)
    }

    /// Terminal.app: no pixel route.
    fn apple_route() -> GraphicsRoute {
        route_for(true, &[("TERM_PROGRAM", "Apple_Terminal")])
    }

    fn terminal(route: GraphicsRoute, depth: ColorDepth, unicode: bool) -> Terminal {
        Terminal::builder()
            .app(TerminalApp::AppleTerminal)
            .is_tty(true)
            .graphics_route(route)
            .color_depth(depth)
            .supports_unicode(unicode)
            .color_mode(crate::discovery::detection::ColorMode::Dark)
            .width(80)
            .height(24)
            .cell_size(CellSize { width: 8, height: 16 })
            .build()
    }

    /// A 4x2 PNG: red on the left half, blue on the right.
    fn image_file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let img = RgbaImage::from_fn(4, 2, |x, _| {
            if x < 2 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) }
        });
        let path = dir.join(name);
        img.save(&path).unwrap();
        path
    }

    /// `text` with every SGR sequence removed; any other control left behind
    /// would show here.
    fn without_sgr(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if d == 'm' {
                        break;
                    }
                    assert!(d.is_ascii_digit() || d == ';', "non-SGR escape in {text:?}");
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    #[test]
    fn a_truecolor_unicode_terminal_without_pixels_draws_half_blocks_with_the_route_reason() {
        let dir = tempfile::tempdir().unwrap();
        let image = TerminalImage::new(&image_file(dir.path(), "flag.png"))
            .unwrap()
            .with_width(ImageWidth::Characters(4));
        let route = apple_route();
        let term = terminal(route.clone(), ColorDepth::TrueColor, true);

        let output = image.render_with_half_blocks(&term).unwrap();

        let ImageOutput::HalfBlocks { text, reason } = &output else {
            panic!("expected half-blocks, got {output:?}");
        };
        assert_eq!(*reason, route.reason, "the route's reason is kept so hints still apply");
        // 4 columns of a 2:1 image on a 1:2 cell: 2 pixel rows, 1 text row.
        assert_eq!(text, "\x1b[38;2;255;0;0;48;2;255;0;0m▀▀\x1b[38;2;0;0;255;48;2;0;0;255m▀▀\x1b[0m");
        assert_eq!(without_sgr(text), "▀▀▀▀");
        assert_eq!(image.render(&term), *text, "TerminalRenderable draws the same half-blocks");
        // Diagrams' entry point is unchanged: alt text, never half-blocks.
        assert!(matches!(image.render_with_route(&term).unwrap(), ImageOutput::Fallback { .. }));
    }

    #[test]
    fn rows_and_layout_offset_follow_the_resolved_width() {
        let dir = tempfile::tempdir().unwrap();
        let mut image = TerminalImage::new(&image_file(dir.path(), "flag.png"))
            .unwrap()
            .with_width(ImageWidth::Characters(8));
        image.layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let term = terminal(apple_route(), ColorDepth::TrueColor, true);

        let ImageOutput::HalfBlocks { text, .. } = image.render_with_half_blocks(&term).unwrap() else {
            panic!("expected half-blocks");
        };
        let lines: Vec<String> = text.lines().map(without_sgr).collect();
        // 8 columns of a 2:1 image: 4 pixel rows, 2 text rows, centered in 80.
        assert_eq!(lines.len(), 2, "{lines:?}");
        for line in &lines {
            assert_eq!(line, &format!("{}{}", " ".repeat(36), "▀".repeat(8)));
        }
    }

    #[test]
    fn without_truecolor_or_unicode_the_alt_text_is_printed() {
        let dir = tempfile::tempdir().unwrap();
        let path = image_file(dir.path(), "flag.png");
        for (depth, unicode) in [
            (ColorDepth::Enhanced, true),
            (ColorDepth::Basic, true),
            (ColorDepth::None, true),
            (ColorDepth::TrueColor, false),
        ] {
            let term = terminal(apple_route(), depth, unicode);
            let output = TerminalImage::new(&path).unwrap().render_with_half_blocks(&term).unwrap();
            assert_eq!(
                output,
                ImageOutput::Fallback {
                    text: "[Image: flag.png]".to_string(),
                    reason: apple_route().reason,
                },
                "{depth:?}, unicode {unicode}"
            );
            let with_alt = TerminalImage::new(&path).unwrap().with_alt_text("a flag");
            assert_eq!(with_alt.render(&term), "a flag", "explicit alt text wins over the file name");
        }
    }

    #[test]
    fn piped_output_gets_plain_text_even_when_color_is_forced() {
        let dir = tempfile::tempdir().unwrap();
        let image = TerminalImage::new(&image_file(dir.path(), "flag.png")).unwrap();
        let route = route_for(false, &[("TERM_PROGRAM", "Apple_Terminal")]);
        // `FORCE_COLOR` marks the terminal a tty with truecolor; the route
        // still knows stdout is a pipe.
        let forced = terminal(route.clone(), ColorDepth::TrueColor, true);
        let mut piped = forced.clone();
        piped.is_tty = false;

        for term in [&forced, &piped] {
            let output = image.render_with_half_blocks(term).unwrap();
            let ImageOutput::Fallback { text, reason } = output else {
                panic!("expected plain text, got {output:?}");
            };
            assert_eq!(text, "[Image: flag.png]");
            assert_eq!(reason.kind, RouteReasonKind::NotATerminal);
        }
    }

    #[test]
    fn disabled_or_invalid_graphics_settings_are_not_bypassed() {
        let dir = tempfile::tempdir().unwrap();
        let image = TerminalImage::new(&image_file(dir.path(), "flag.png")).unwrap();
        for (value, kind) in [
            ("none", RouteReasonKind::UserDisabled),
            ("bogus", RouteReasonKind::InvalidInput),
        ] {
            let route = route_for(true, &[("TERM_PROGRAM", "Apple_Terminal"), ("BISCUIT_GRAPHICS", value)]);
            let term = terminal(route, ColorDepth::TrueColor, true);
            let output = image.render_with_half_blocks(&term).unwrap();
            let ImageOutput::Fallback { text, reason } = output else {
                panic!("BISCUIT_GRAPHICS={value} drew {output:?}");
            };
            assert_eq!(text, "[Image: flag.png]");
            assert_eq!(reason.kind, kind, "BISCUIT_GRAPHICS={value}");
            assert!(matches!(
                image.render_inline(&term),
                Err(TerminalImageError::UnsupportedTerminal)
            ));
        }
    }

    #[test]
    fn a_pixel_route_is_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let image = TerminalImage::new(&image_file(dir.path(), "flag.png")).unwrap();
        let route = route_for(true, &[("TERM_PROGRAM", "WezTerm")]);
        assert!(route.has_pixels());
        let mut term = terminal(route, ColorDepth::TrueColor, true);
        term.app = TerminalApp::Wezterm;

        let ImageOutput::Pixels(output) = image.render_with_half_blocks(&term).unwrap() else {
            panic!("a pixel route draws pixels");
        };
        let ImageOutput::Pixels(direct) = image.render_with_route(&term).unwrap() else {
            panic!("a pixel route draws pixels");
        };
        // Every upload has a fresh image ID; everything else is identical.
        assert_eq!(without_kitty_id(&output), without_kitty_id(&direct));
    }

    /// `output` with its Kitty image ID's digits removed.
    fn without_kitty_id(output: &str) -> String {
        let start = output.find("i=").expect("a Kitty image ID") + "i=".len();
        let digits = output[start..].find(',').expect("more parameters follow the ID");
        format!("{}{}", &output[..start], &output[start + digits..])
    }

    #[test]
    fn a_corrupt_image_is_an_error_not_half_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\nnot really a png").unwrap();
        let image = TerminalImage::new(&path).unwrap();
        let term = terminal(apple_route(), ColorDepth::TrueColor, true);

        assert!(image.render_with_half_blocks(&term).is_err());
        assert!(image.render_inline(&term).is_err());
        assert_eq!(image.render(&term), "", "a load error renders as nothing");
    }

    /// Alt text and file names print as literal text: embedded ESC, OSC, CR,
    /// and BEL cannot reach the terminal, and half-blocks carry no text.
    #[test]
    fn alt_text_is_sanitized_and_never_part_of_half_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let path = image_file(dir.path(), "flag.png");
        let hostile = "evil\x1b]8;;https://x.test\x07link\r\x1b[2Jend\x1b\\";
        let image = TerminalImage::new(&path).unwrap().with_alt_text(hostile);

        let plain = terminal(apple_route(), ColorDepth::Enhanced, true);
        let ImageOutput::Fallback { text, .. } = image.render_with_half_blocks(&plain).unwrap() else {
            panic!("expected alt text");
        };
        assert_eq!(text, "evil]8;;https://x.testlink[2Jend\\");
        assert_eq!(image.render(&plain), text);

        let capable = terminal(apple_route(), ColorDepth::TrueColor, true);
        let ImageOutput::HalfBlocks { text, .. } = image.render_with_half_blocks(&capable).unwrap() else {
            panic!("expected half-blocks");
        };
        assert!(!text.contains("evil"), "{text:?}");
        let _ = without_sgr(&text);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_name_with_controls_is_sanitized() {
        let dir = tempfile::tempdir().unwrap();
        let path = image_file(dir.path(), "a\x1b[31mb\rc.png");
        let image = TerminalImage::new(&path).unwrap();
        let term = terminal(apple_route(), ColorDepth::Enhanced, true);

        assert_eq!(image.render(&term), "[Image: a[31mbc.png]");
    }

    #[test]
    fn inline_without_pixels_draws_half_block_lines() {
        let dir = tempfile::tempdir().unwrap();
        let image = TerminalImage::new(&image_file(dir.path(), "flag.png"))
            .unwrap()
            .with_width(ImageWidth::Characters(8));
        let mut column = terminal(apple_route(), ColorDepth::TrueColor, true);
        column.fixed_width = Some(20);

        let InlineImage::Lines(lines) = image.render_inline(&column).unwrap() else {
            panic!("expected lines");
        };
        assert_eq!(lines.len(), 2, "{lines:?}");
        for line in &lines {
            assert_eq!(without_sgr(line), "▀".repeat(8));
        }

        let without = terminal(apple_route(), ColorDepth::Enhanced, true);
        assert!(matches!(
            image.render_inline(&without),
            Err(TerminalImageError::UnsupportedTerminal)
        ));
    }

    /// Through the public component: an image column of a `TwoColumn`
    /// draws half-block rows beside the text column.
    #[test]
    fn a_two_column_image_draws_half_blocks_beside_its_text() {
        use crate::components::two_column::TwoColumn;

        let dir = tempfile::tempdir().unwrap();
        let path = image_file(dir.path(), "flag.png");
        let image = || TerminalImage::new(&path).unwrap().with_width(ImageWidth::Fill);
        let mut term = terminal(apple_route(), ColorDepth::TrueColor, true);
        term.fixed_width = Some(40);

        let rendered = TwoColumn::new(image(), "Caption").render(&term);
        let lines: Vec<String> = rendered.lines().map(without_sgr).collect();
        assert!(lines.len() >= 2, "{lines:?}");
        assert!(lines[0].contains('▀') && lines[0].contains("Caption"), "{lines:?}");
        assert!(lines.iter().all(|line| !line.contains("[Image")), "{lines:?}");

        term.color_depth = ColorDepth::Enhanced;
        let rendered = TwoColumn::new(image(), "Caption").render(&term);
        assert!(!rendered.contains('▀'), "{rendered:?}");
        assert!(rendered.contains("[Image: flag.png]"), "{rendered:?}");
    }

    /// A pixel route refused after encoding (an image over tmux's message
    /// limit) still draws half-blocks, with the size-limit reason.
    #[test]
    fn an_image_over_the_tmux_limit_draws_half_blocks() {
        let mut state: u32 = 0x1234_5678;
        let noise = image::RgbImage::from_fn(640, 640, |_, _| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let [r, g, b, _] = state.to_le_bytes();
            image::Rgb([r, g, b])
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noise.png");
        noise.save(&path).unwrap();
        let image = TerminalImage::new(&path).unwrap().with_width(ImageWidth::Characters(10));
        let route = route_for(
            true,
            &[
                ("SSH_CONNECTION", "10.0.0.1 50000 10.0.0.2 22"),
                ("BISCUIT_GRAPHICS_TRANSPORT", "tmux"),
                ("BISCUIT_GRAPHICS", "iterm2"),
            ],
        );
        let term = terminal(route, ColorDepth::TrueColor, true);

        let output = image.render_with_half_blocks(&term).unwrap();
        let ImageOutput::HalfBlocks { text, reason } = output else {
            panic!("expected half-blocks, got {output:?}");
        };
        assert_eq!(reason.kind, RouteReasonKind::BlockedTransport);
        assert_eq!(text.lines().count(), 5, "10 square columns: 10 pixel rows");
        assert!(!text.contains("\x1bPtmux;"), "no passthrough envelope");
        assert!(matches!(image.render_inline(&term).unwrap(), InlineImage::Lines(_)));
    }
}
