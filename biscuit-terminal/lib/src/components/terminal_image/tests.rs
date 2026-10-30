use super::*;
use crate::discovery::detection::TerminalApp;
use crate::discovery::fonts::CellSize;
use crate::utils::layout::Alignment;
use serial_test::serial;
use std::io::Write;
use std::path::Path;

// Helper to create a minimal valid PNG using the image crate
fn create_test_png() -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    use std::io::Cursor;

    // Create a 2x2 red image
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(2, 2, |_x, _y| Rgb([255u8, 0u8, 0u8]));

    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png).unwrap();
    buffer.into_inner()
}

fn create_temp_test_image() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();
    (dir, file_path)
}

// Error type tests
#[test]
fn test_error_file_not_found_message() {
    let err = TerminalImageError::FileNotFound {
        path: "/nonexistent/file.png".to_string(),
    };
    assert!(err.to_string().contains("File not found"));
    assert!(err.to_string().contains("/nonexistent/file.png"));
}

#[test]
fn test_error_invalid_path_message() {
    let err = TerminalImageError::InvalidPath {
        path: "bad/path".to_string(),
        reason: "Permission denied".to_string(),
    };
    assert!(err.to_string().contains("Invalid path"));
    assert!(err.to_string().contains("Permission denied"));
}

#[test]
fn test_error_invalid_width_spec_message() {
    let err = TerminalImageError::InvalidWidthSpec {
        spec: "abc".to_string(),
    };
    assert!(err.to_string().contains("Invalid width specification"));
    assert!(err.to_string().contains("abc"));
}

#[test]
fn test_error_encoding_message() {
    let err = TerminalImageError::EncodingError {
        message: "PNG encode failed".to_string(),
    };
    assert!(err.to_string().contains("Encoding error"));
    assert!(err.to_string().contains("PNG encode failed"));
}

#[test]
fn test_error_unsupported_terminal_message() {
    let err = TerminalImageError::UnsupportedTerminal;
    assert!(err.to_string().contains("does not support image"));
}

// Width parsing tests
#[test]
fn test_parse_width_spec_empty() {
    let result = parse_width_spec("").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
}

#[test]
fn test_parse_width_spec_whitespace() {
    let result = parse_width_spec("   ").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
}

#[test]
fn test_parse_width_spec_fill() {
    assert!(matches!(
        parse_width_spec("fill").unwrap(),
        ImageWidth::Fill
    ));
    assert!(matches!(
        parse_width_spec("FILL").unwrap(),
        ImageWidth::Fill
    ));
    assert!(matches!(
        parse_width_spec("Fill").unwrap(),
        ImageWidth::Fill
    ));
}

#[test]
fn test_parse_width_spec_percentage() {
    let result = parse_width_spec("50%").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));

    let result = parse_width_spec("100%").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 1.0).abs() < 0.001));

    let result = parse_width_spec("25%").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.25).abs() < 0.001));
}

#[test]
fn test_parse_width_spec_percentage_with_spaces() {
    let result = parse_width_spec(" 50% ").unwrap();
    assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
}

#[test]
fn test_parse_width_spec_characters() {
    assert!(matches!(
        parse_width_spec("80").unwrap(),
        ImageWidth::Characters(80)
    ));
    assert!(matches!(
        parse_width_spec("25").unwrap(),
        ImageWidth::Characters(25)
    ));
    assert!(matches!(
        parse_width_spec("1").unwrap(),
        ImageWidth::Characters(1)
    ));
}

#[test]
fn test_parse_width_spec_characters_with_ch_suffix() {
    assert!(matches!(
        parse_width_spec("40ch").unwrap(),
        ImageWidth::Characters(40)
    ));
    assert!(matches!(
        parse_width_spec("100ch").unwrap(),
        ImageWidth::Characters(100)
    ));
    assert!(matches!(
        parse_width_spec(" 25ch ").unwrap(),
        ImageWidth::Characters(25)
    ));
    // 0ch should be invalid
    assert!(parse_width_spec("0ch").is_err());
}

#[test]
fn test_parse_width_spec_invalid() {
    assert!(parse_width_spec("abc").is_err());
    assert!(parse_width_spec("50px").is_err());
    assert!(parse_width_spec("-10").is_err());
    assert!(parse_width_spec("0").is_err());
    assert!(parse_width_spec("150%").is_err());
}

// Filepath parsing tests
#[test]
fn test_parse_filepath_and_width_simple() {
    let (path, width) = parse_filepath_and_width("image.png").unwrap();
    assert_eq!(path, "image.png");
    assert!(width.is_none());
}

#[test]
fn test_parse_filepath_and_width_with_percentage() {
    let (path, width) = parse_filepath_and_width("image.png|50%").unwrap();
    assert_eq!(path, "image.png");
    assert_eq!(width, Some("50%".to_string()));
}

#[test]
fn test_parse_filepath_and_width_with_characters() {
    let (path, width) = parse_filepath_and_width("photo.jpg|80").unwrap();
    assert_eq!(path, "photo.jpg");
    assert_eq!(width, Some("80".to_string()));
}

#[test]
fn test_parse_filepath_and_width_with_spaces() {
    let (path, width) = parse_filepath_and_width("image.png | 50%").unwrap();
    assert_eq!(path, "image.png");
    assert_eq!(width, Some("50%".to_string()));
}

#[test]
fn test_parse_filepath_and_width_with_fill() {
    let (path, width) = parse_filepath_and_width("image.png|fill").unwrap();
    assert_eq!(path, "image.png");
    assert_eq!(width, Some("fill".to_string()));
}

#[test]
fn test_parse_filepath_and_width_empty_path() {
    assert!(parse_filepath_and_width("").is_err());
    assert!(parse_filepath_and_width("|50%").is_err());
}

#[test]
fn test_parse_filepath_and_width_multiple_pipes() {
    // Only splits on first pipe
    let (path, width) = parse_filepath_and_width("file|50|extra").unwrap();
    assert_eq!(path, "file");
    assert_eq!(width, Some("50|extra".to_string()));
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_parse_width_spec_never_panics(s in ".*") {
        let _ = parse_width_spec(&s);
    }

    #[test]
    fn prop_parse_filepath_and_width_never_panics(s in ".*") {
        let _ = parse_filepath_and_width(&s);
    }

    #[test]
    fn prop_parse_width_spec_valid_percentages(p in 1..=100u32) {
        let s = format!("{}%", p);
        let result = parse_width_spec(&s).unwrap();
        let expected_float = (p as f32) / 100.0;
        assert!(matches!(result, ImageWidth::Percent(val) if (val - expected_float).abs() < 0.001));
    }

    #[test]
    fn prop_parse_width_spec_valid_characters(c in 1..=1000u32) {
        let s = format!("{}", c);
        assert!(matches!(parse_width_spec(&s).unwrap(), ImageWidth::Characters(val) if val == c));

        let s_ch = format!("{}ch", c);
        assert!(matches!(parse_width_spec(&s_ch).unwrap(), ImageWidth::Characters(val) if val == c));
    }
}

// Image loading tests
#[test]
fn test_terminal_image_new_file_not_found() {
    let result = TerminalImage::new(Path::new("/nonexistent/image.png"));
    assert!(matches!(
        result,
        Err(TerminalImageError::FileNotFound { .. })
    ));
}

#[test]
fn test_terminal_image_new_with_valid_file() {
    // Create a temp file
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(&create_test_png()).unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    assert!(img.filename.contains("test.png"));
    assert_eq!(img.relative, file_path.to_string_lossy());
}

#[test]
fn test_terminal_image_from_spec_simple() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::from_spec(&file_path.to_string_lossy()).unwrap();
    assert!(matches!(img.width, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
}

#[test]
fn test_terminal_image_from_spec_with_width() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let spec = format!("{}|75%", file_path.display());
    let img = TerminalImage::from_spec(&spec).unwrap();
    assert!(matches!(img.width, ImageWidth::Percent(p) if (p - 0.75).abs() < 0.001));
    assert_eq!(img.width_raw, Some("|75%".to_string()));
}

#[test]
fn test_terminal_image_load_image() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    let loaded = img.load_image().unwrap();
    assert_eq!(loaded.width(), 2);
    assert_eq!(loaded.height(), 2);
}

#[test]
#[cfg(feature = "image")]
fn test_terminal_image_load_svg() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.svg");
    std::fs::write(
        &file_path,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
            <rect width="100" height="50" fill="red"/>
        </svg>"#,
    )
    .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    assert!(img.is_svg());
    let loaded = img.load_image().unwrap();
    assert_eq!(loaded.width(), 100);
    assert_eq!(loaded.height(), 50);
}

#[test]
#[cfg(feature = "image")]
fn test_terminal_image_load_svg_demultiplies_partial_alpha() {
    // A half-opaque shape over nothing: the loaded image must carry the
    // shape's own color at ~50% alpha. Premultiplied channels leaking
    // through would halve every channel and draw a dark fringe.
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("half.svg");
    std::fs::write(
        &file_path,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10">
            <rect width="10" height="10" fill="rgb(200,100,50)" fill-opacity="0.5"/>
        </svg>"#,
    )
    .unwrap();

    let loaded = TerminalImage::new(&file_path)
        .unwrap()
        .load_image()
        .unwrap()
        .to_rgba8();
    assert_eq!((loaded.width(), loaded.height()), (20, 10));
    let [r, g, b, a] = loaded.get_pixel(5, 5).0;
    assert!((127..=128).contains(&a), "alpha {a} should be half of 255");
    for (actual, expected) in [(r, 200u8), (g, 100), (b, 50)] {
        assert!(
            actual.abs_diff(expected) <= 2,
            "channel {actual} should be within rounding of {expected}"
        );
    }
    assert_eq!(loaded.get_pixel(15, 5).0[3], 0, "unpainted area stays transparent");
}

#[test]
fn test_terminal_image_svg_invalid_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("bad.svg");
    std::fs::write(&file_path, "not valid svg").unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    assert!(img.load_image().is_err());
}

#[test]
fn test_terminal_image_is_svg_detection() {
    let img_svg = TerminalImage {
        filename: "/tmp/test.svg".to_string(),
        ..Default::default()
    };
    assert!(img_svg.is_svg());

    let img_svg_upper = TerminalImage {
        filename: "/tmp/test.SVG".to_string(),
        ..Default::default()
    };
    assert!(img_svg_upper.is_svg());

    let img_png = TerminalImage {
        filename: "/tmp/test.png".to_string(),
        ..Default::default()
    };
    assert!(!img_png.is_svg());
}

#[test]
fn test_terminal_image_encode_as_png() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let term_img = TerminalImage::new(&file_path).unwrap();
    let loaded = term_img.load_image().unwrap();
    let png_bytes = term_img.encode_as_png(&loaded).unwrap();

    // PNG files start with specific magic bytes
    assert!(png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
}

#[test]
fn test_terminal_image_encode_as_base64() {
    let term_img = TerminalImage::default();
    let data = b"Hello, World!";
    let encoded = term_img.encode_as_base64(data);
    assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
}

#[test]
fn test_terminal_image_generate_alt_text_default() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("my-photo.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    let alt = img.generate_alt_text();
    assert_eq!(alt, "[Image: my-photo.png]");
}

#[test]
fn test_terminal_image_generate_alt_text_custom() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_alt_text("A beautiful sunset");
    let alt = img.generate_alt_text();
    assert_eq!(alt, "A beautiful sunset");
}

#[test]
fn test_terminal_image_builder_methods() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_alt_text("Test image")
        .with_width(ImageWidth::Characters(40))
        .with_margins(2, 2);

    assert_eq!(img.alt_text, Some("Test image".to_string()));
    assert_eq!(img.width, ImageWidth::Characters(40));

    // Verify margins via resolve_dimensions
    let dims = img.resolve_dimensions(80);
    assert_eq!(dims.left_margin, 2);
    assert_eq!(dims.right_margin, 2);
}

// Protocol rendering tests
#[test]
fn test_render_kitty_small_data() {
    let term_img = TerminalImage::default();
    let png_data = create_test_png();
    let result = term_img.render_kitty(&png_data, 100, 100);

    // Should start with Kitty escape sequence
    assert!(result.starts_with("\x1b_Gq=2,i="), "quiet, with an image ID: {result:?}");
    // Should contain format and action parameters
    assert!(result.contains("f=100")); // PNG format
    assert!(result.contains("a=T")); // Transmit and display
    assert!(result.contains("t=d")); // Direct transmission
    // Should end with string terminator
    assert!(result.ends_with("\x1b\\"));
}

#[test]
fn test_render_kitty_chunking() {
    let term_img = TerminalImage::default();
    // Create a larger payload that requires chunking (>4096 bytes base64)
    let large_data = vec![0u8; 4000]; // Will be ~5333 bytes base64
    let result = term_img.render_kitty(&large_data, 100, 100);

    // Should have multiple escape sequences due to chunking
    let escape_count = result.matches("\x1b_G").count();
    assert!(escape_count >= 2, "Expected chunking for large data");

    // First chunk should have m=1 (more), last should have m=0
    assert!(result.contains("m=1"));
    assert!(result.contains("m=0"));
    // Only the first chunk names the image.
    assert_eq!(result.matches("i=").count(), 1, "{result:?}");
    assert!(result.split("\x1b_G").skip(2).all(|chunk| chunk.starts_with("m=")));
}

#[test]
fn test_render_iterm2() {
    let term_img = TerminalImage::default();
    let png_data = create_test_png();
    let result = term_img.render_iterm2(&png_data, "40", "test.png");

    // Should start with iTerm2 inline image escape
    assert!(result.starts_with("\x1b]1337;File="));
    // Should contain inline=1
    assert!(result.contains("inline=1"));
    // Should contain width
    assert!(result.contains("width=40"));
    // Should end with BEL
    assert!(result.ends_with("\x07"));
    // Filename should be base64 encoded
    let expected_filename_b64 = base64::engine::general_purpose::STANDARD.encode("test.png");
    assert!(result.contains(&format!("name={}", expected_filename_b64)));
}

#[test]
fn test_render_to_terminal_kitty_uses_save_restore_and_explicit_row_advance() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();
    let term = Terminal::builder()
        .app(TerminalApp::Kitty)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Use render_kitty_for_terminal to avoid cursor_position() hanging
    // when stdin is piped (DSR query blocks on read).
    let (seq, height, _) = term_img.render_kitty_for_terminal(&term).unwrap();
    let output = format!("{}\x1b[{}B\r", seq, height);

    assert!(output.contains("\x1b_G"));
    assert!(output.contains("\x1b[s"));
    assert!(output.contains("\x1b[u"));
    assert!(output.contains("B\r"));
    assert!(!output.contains("\x1b[1A"));
}

#[test]
fn test_render_to_terminal_iterm2_uses_save_restore_and_explicit_row_advance() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();
    let term = Terminal::builder()
        .app(TerminalApp::ITerm2)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::ITerm)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Use render_iterm2_for_terminal to avoid cursor_position() hang.
    let (seq, height, _) = term_img.render_iterm2_for_terminal(&term).unwrap();
    let output = format!("{}\x1b[{}B\r", seq, height);

    assert!(output.contains("\x1b]1337;File="));
    assert!(output.contains("\x1b[s"));
    assert!(output.contains("\x1b[u"));
    assert!(output.contains("B\r"));
    assert!(!output.contains("\x1b[1A"));
}

#[test]
fn test_render_to_terminal_wezterm_uses_explicit_row_sizing_and_row_advance() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();
    let term = Terminal::builder()
        .app(TerminalApp::Wezterm)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Use render_kitty_for_terminal to avoid cursor_position() hang.
    let (seq, height, _) = term_img.render_kitty_for_terminal(&term).unwrap();
    let output = format!("{}\x1b[{}B\r", seq, height);

    assert!(output.contains("\x1b_G"));
    assert!(output.contains("\x1b[s"));
    assert!(output.contains("\x1b[u"));
    assert!(output.contains("c="));
    assert!(output.contains(",r="));
    assert!(output.contains("B\r"));
    assert!(output.ends_with('\r'));
}

#[test]
fn test_render_to_terminal_iterm2_prefers_native_protocol_when_kitty_advertised() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();
    let iterm2 = Terminal::builder()
        .app(TerminalApp::ITerm2)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // iTerm2 with Kitty advertised should still use iterm2 protocol.
    // render_to_terminal dispatches to render_iterm2_for_terminal for
    // iTerm2 regardless of Kitty support.
    let (iterm2_out, _, _) = term_img.render_iterm2_for_terminal(&iterm2).unwrap();

    assert!(iterm2_out.contains("\x1b]1337;File="));
    assert!(!iterm2_out.contains("\x1b_G"));
}

#[test]
fn test_render_to_terminal_warp_matches_kitty_cursor_management_strategy() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();

    let kitty = Terminal::builder()
        .app(TerminalApp::Kitty)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();
    let warp = Terminal::builder()
        .app(TerminalApp::Warp)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Use render_kitty_for_terminal directly to avoid cursor_position()
    // which can hang when stdin is piped (DSR query blocks on read).
    let (kitty_seq, kitty_height, _) = term_img.render_kitty_for_terminal(&kitty).unwrap();
    let kitty_out = format!("{}\x1b[{}B\r", kitty_seq, kitty_height);
    let (warp_seq, _, warp_raw) = term_img.render_kitty_for_terminal(&warp).unwrap();
    let warp_rows = (warp_raw.floor() as u32).max(1);
    let warp_out = format!("{}\x1b[{}B\r", warp_seq, warp_rows);

    assert!(kitty_out.contains("\x1b[s"));
    assert!(kitty_out.contains("\x1b[u"));
    assert!(kitty_out.contains("B\r"));
    assert!(!kitty_out.contains("\x1b[1A"));
    assert!(warp_out.contains("\x1b[s"));
    assert!(warp_out.contains("\x1b[u"));
    assert!(warp_out.contains("B\r"));
    assert!(!warp_out.contains("\x1b[1A"));
}

#[test]
fn test_render_to_terminal_iterm2_uses_ceil_rounding() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();

    let iterm2 = Terminal::builder()
        .app(TerminalApp::ITerm2)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::ITerm)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Verify ceil rounding from a single render_iterm2_for_terminal call.
    let (_, height_cells, raw_height) = term_img.render_iterm2_for_terminal(&iterm2).unwrap();
    assert_eq!(
        height_cells,
        (raw_height.ceil() as u32).max(1),
        "iTerm2 should use ceil: raw_height={raw_height:.3}, height_cells={height_cells}"
    );
}

#[test]
fn test_render_to_terminal_wezterm_uses_ceil_rounding() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();

    let wezterm = Terminal::builder()
        .app(TerminalApp::Wezterm)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Verify ceil rounding directly from a single render_kitty_for_terminal
    // call. Avoids comparing two render_to_terminal calls which each query
    // cell_size() independently and can get different values when a real
    // terminal is attached.
    let (_, height_cells, raw_height) = term_img.render_kitty_for_terminal(&wezterm).unwrap();
    assert_eq!(
        height_cells,
        (raw_height.ceil() as u32).max(1),
        "WezTerm should use ceil: raw_height={raw_height:.3}, height_cells={height_cells}"
    );
}

#[test]
fn test_render_to_terminal_warp_uses_floor_rounding() {
    let (_dir, file_path) = create_temp_test_image();
    let term_img = TerminalImage::new(&file_path).unwrap();

    let kitty = Terminal::builder()
        .app(TerminalApp::Kitty)
        .is_tty(true)
        .image_support(crate::discovery::detection::ImageSupport::Kitty)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build();

    // Get raw_height from the terminal's cached cell_size via render_kitty_for_terminal.
    let (_, height_cells, raw_height) = term_img.render_kitty_for_terminal(&kitty).unwrap();
    let ceil_rows = (raw_height.ceil() as u32).max(1);
    let floor_rows = (raw_height.floor() as u32).max(1);

    // Kitty uses ceil
    assert_eq!(height_cells, ceil_rows);
    // Warp uses floor — verify it differs from ceil when raw_height is fractional
    assert!(floor_rows <= ceil_rows);
    if raw_height.fract() != 0.0 {
        assert_eq!(
            floor_rows + 1,
            ceil_rows,
            "For fractional raw_height={raw_height:.3}, floor ({floor_rows}) + 1 should equal ceil ({ceil_rows})"
        );
    } else {
        assert_eq!(
            floor_rows, ceil_rows,
            "For integer raw_height={raw_height:.3}, floor should equal ceil"
        );
    }
}

// Dimension calculation tests
#[test]
fn test_calculate_display_dimensions_fill() {
    let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Fill, 100);
    // 100 chars * 8 pixels = 800, should use original since no upscale
    assert_eq!(w, 800);
    // Aspect ratio preserved: 800 * (600/800) = 600
    assert_eq!(h, 600);
}

#[test]
fn test_calculate_display_dimensions_percent() {
    let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Percent(0.5), 100);
    // 50% of 100 chars * 8 pixels = 400
    assert_eq!(w, 400);
    // Aspect ratio: 400 * (600/800) = 300
    assert_eq!(h, 300);
}

#[test]
fn test_calculate_display_dimensions_characters() {
    let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Characters(50), 100);
    // 50 chars * 8 pixels = 400
    assert_eq!(w, 400);
    // Aspect ratio: 400 * (600/800) = 300
    assert_eq!(h, 300);
}

#[test]
fn test_calculate_display_dimensions_no_upscale() {
    // Image smaller than target - should not upscale
    let (w, h) = calculate_display_dimensions(100, 100, &ImageWidth::Fill, 100);
    assert_eq!(w, 100); // Don't upscale beyond original
    assert_eq!(h, 100);
}

#[test]
fn test_calculate_display_dimensions_minimum_size() {
    // Very small percentage
    let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Percent(0.001), 10);
    // Should be at least 1x1
    assert!(w >= 1);
    assert!(h >= 1);
}

// Integration tests for render_as_* methods
// These tests query the terminal for cell size - must run serially to avoid /dev/tty conflicts

#[test]
#[serial]
fn test_render_as_kitty_produces_valid_output() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Percent(0.5));

    let result = img.render_as_kitty(80).unwrap();

    // Should be a valid Kitty escape sequence
    assert!(result.starts_with("\x1b_Gq=2,i="), "quiet, with an image ID: {result:?}");
    assert!(result.contains("\x1b\\"));
    assert!(result.contains("f=100")); // PNG format
}

#[test]
#[serial]
fn test_render_as_iterm2_produces_valid_output() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Characters(40));

    let result = img.render_as_iterm2(80).unwrap();

    // Should be a valid iTerm2 escape sequence
    assert!(result.starts_with("\x1b]1337;File="));
    assert!(result.contains("\x07"));
    assert!(result.contains("inline=1"));
}

#[test]
#[serial]
fn test_render_as_kitty_with_zero_term_width_uses_default() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();

    // Should not panic with zero width, uses 80 as default
    let result = img.render_as_kitty(0);
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_render_as_iterm2_with_zero_term_width_uses_default() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();

    // Should not panic with zero width, uses 80 as default
    let result = img.render_as_iterm2(0);
    assert!(result.is_ok());
}

#[test]
fn test_from_spec_with_invalid_width_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let spec = format!("{}|invalid", file_path.display());
    let result = TerminalImage::from_spec(&spec);
    assert!(result.is_err());
}

#[test]
fn test_from_spec_with_percentage_over_100_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let spec = format!("{}|150%", file_path.display());
    let result = TerminalImage::from_spec(&spec);
    assert!(result.is_err());
}

// Security error type tests
#[test]
fn test_error_path_traversal_blocked_message() {
    let err = TerminalImageError::PathTraversalBlocked {
        path: "/etc/passwd".to_string(),
    };
    assert!(err.to_string().contains("Path traversal blocked"));
    assert!(err.to_string().contains("/etc/passwd"));
}

#[test]
fn test_error_file_too_large_message() {
    let err = TerminalImageError::FileTooLarge {
        size: 20_000_000,
        max_size: 10_000_000,
    };
    assert!(err.to_string().contains("File too large"));
    assert!(err.to_string().contains("20000000"));
    assert!(err.to_string().contains("10000000"));
}

#[test]
fn test_error_remote_url_blocked_message() {
    let err = TerminalImageError::RemoteUrlBlocked {
        url: "https://example.com/image.png".to_string(),
    };
    assert!(err.to_string().contains("Remote URLs not allowed"));
    assert!(err.to_string().contains("https://example.com"));
}

#[test]
fn test_error_viuer_error_message() {
    let err = TerminalImageError::ViuerError {
        message: "Protocol not supported".to_string(),
    };
    assert!(err.to_string().contains("viuer rendering error"));
    assert!(err.to_string().contains("Protocol not supported"));
}

// Security validation tests
#[test]
fn test_validate_not_remote_url_allows_local_path() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    assert!(img.validate_not_remote_url(false).is_ok());
}

#[test]
fn test_validate_not_remote_url_blocks_http() {
    let img = TerminalImage {
        filename: "http://example.com/image.png".to_string(),
        ..Default::default()
    };

    let result = img.validate_not_remote_url(false);
    assert!(matches!(
        result,
        Err(TerminalImageError::RemoteUrlBlocked { .. })
    ));
}

#[test]
fn test_validate_not_remote_url_blocks_https() {
    let img = TerminalImage {
        filename: "https://example.com/image.png".to_string(),
        ..Default::default()
    };

    let result = img.validate_not_remote_url(false);
    assert!(matches!(
        result,
        Err(TerminalImageError::RemoteUrlBlocked { .. })
    ));
}

#[test]
fn test_validate_not_remote_url_allows_when_permitted() {
    let img = TerminalImage {
        filename: "https://example.com/image.png".to_string(),
        ..Default::default()
    };

    // When allow_remote is true, should succeed
    assert!(img.validate_not_remote_url(true).is_ok());
}

#[test]
fn test_validate_not_remote_url_case_insensitive() {
    let img = TerminalImage {
        filename: "HTTPS://EXAMPLE.COM/IMAGE.PNG".to_string(),
        ..Default::default()
    };

    let result = img.validate_not_remote_url(false);
    assert!(matches!(
        result,
        Err(TerminalImageError::RemoteUrlBlocked { .. })
    ));
}

#[test]
fn test_validate_path_traversal_allows_within_base() {
    let dir = tempfile::tempdir().unwrap();
    let sub_dir = dir.path().join("images");
    std::fs::create_dir(&sub_dir).unwrap();

    let file_path = sub_dir.join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    let base_path = Some(dir.path().to_path_buf());

    assert!(img.validate_path_traversal(&base_path).is_ok());
}

#[test]
fn test_validate_path_traversal_blocks_escape() {
    let dir = tempfile::tempdir().unwrap();
    let sibling_dir = tempfile::tempdir().unwrap();

    let file_path = sibling_dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    let base_path = Some(dir.path().to_path_buf());

    let result = img.validate_path_traversal(&base_path);
    assert!(matches!(
        result,
        Err(TerminalImageError::PathTraversalBlocked { .. })
    ));
}

#[test]
fn test_validate_path_traversal_allows_no_base() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();

    // No base path means all paths are allowed
    assert!(img.validate_path_traversal(&None).is_ok());
}

#[test]
fn test_validate_file_size_allows_small_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let png_data = create_test_png();
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&png_data)
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();

    // Allow files up to 1MB (our test PNG is tiny)
    assert!(img.validate_file_size(1024 * 1024).is_ok());
}

#[test]
fn test_validate_file_size_blocks_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let png_data = create_test_png();
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&png_data)
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();

    // Set max size to 10 bytes (our PNG is larger)
    let result = img.validate_file_size(10);
    assert!(matches!(
        result,
        Err(TerminalImageError::FileTooLarge { .. })
    ));
}

#[test]
fn test_validate_file_size_exact_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    let png_data = create_test_png();
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&png_data)
        .unwrap();

    let img = TerminalImage::new(&file_path).unwrap();
    let file_size = std::fs::metadata(&file_path).unwrap().len();

    // Exact limit should pass
    assert!(img.validate_file_size(file_size).is_ok());

    // One byte less should fail
    let result = img.validate_file_size(file_size - 1);
    assert!(matches!(
        result,
        Err(TerminalImageError::FileTooLarge { .. })
    ));
}

// resolve_dimensions tests
#[test]
fn test_resolve_dimensions_fill() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Fill);
    let dims = img.resolve_dimensions(80);

    assert_eq!(dims.available_width, 80);
    assert_eq!(dims.image_width, 80);
    assert_eq!(dims.x_offset, 0);
}

#[test]
fn test_resolve_dimensions_percent() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Percent(0.5));
    let dims = img.resolve_dimensions(80);

    assert_eq!(dims.available_width, 80);
    assert_eq!(dims.image_width, 40);
}

#[test]
fn test_resolve_dimensions_with_margins() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_margins(5, 5)
        .with_width(ImageWidth::Fill);
    let dims = img.resolve_dimensions(80);

    assert_eq!(dims.left_margin, 5);
    assert_eq!(dims.right_margin, 5);
    assert_eq!(dims.available_width, 70); // 80 - 5 - 5
    assert_eq!(dims.image_width, 70);
    assert_eq!(dims.x_offset, 5);
}

#[test]
fn test_resolve_dimensions_center_alignment() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Characters(40))
        .alignment(Alignment::Center);
    let dims = img.resolve_dimensions(80);

    assert_eq!(dims.image_width, 40);
    // Centered: (80 - 40) / 2 = 20
    assert_eq!(dims.x_offset, 20);
}

#[test]
fn test_resolve_dimensions_right_alignment() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.png");
    std::fs::File::create(&file_path)
        .unwrap()
        .write_all(&create_test_png())
        .unwrap();

    let img = TerminalImage::new(&file_path)
        .unwrap()
        .with_width(ImageWidth::Characters(30))
        .alignment(Alignment::Right);
    let dims = img.resolve_dimensions(80);

    assert_eq!(dims.image_width, 30);
    // Right-aligned: 80 - 30 = 50
    assert_eq!(dims.x_offset, 50);
}

// ---------------------------------------------------------------------------
// ImageWidth::Scale
// ---------------------------------------------------------------------------

const CELL_8X16: CellSize = CellSize {
    width: 8,
    height: 16,
};

#[test]
fn scaled_columns_follow_the_cell_height_rule() {
    // 16-unit text on a 16 px line: one unit per pixel, 400 / 8 = 50 columns.
    assert_eq!(ImageWidth::scaled_columns(1.0, 400.0, CELL_8X16), 50);
    // A 10x20 cell draws 1.25 px per unit: 500 px / 10 = 50 columns, the same
    // visual size on a larger font.
    let cell_10x20 = CellSize {
        width: 10,
        height: 20,
    };
    assert_eq!(ImageWidth::scaled_columns(1.0, 400.0, cell_10x20), 50);
    // 125% rounds up: 400 × 1.25 / 8 = 62.5.
    assert_eq!(ImageWidth::scaled_columns(1.25, 400.0, CELL_8X16), 63);
    // A high-DPI cell (16x32) keeps the columns and doubles the pixels.
    let retina = CellSize {
        width: 16,
        height: 32,
    };
    assert_eq!(ImageWidth::scaled_columns(1.0, 400.0, retina), 50);
}

#[test]
fn scaled_columns_never_drop_below_one() {
    assert_eq!(ImageWidth::scaled_columns(1.0, 0.0, CELL_8X16), 1);
    assert_eq!(ImageWidth::scaled_columns(-1.0, 400.0, CELL_8X16), 1);
    assert_eq!(ImageWidth::scaled_columns(f32::NAN, 400.0, CELL_8X16), 1);
    assert_eq!(ImageWidth::scaled_columns(1.0, f32::NAN, CELL_8X16), 1);
}

#[test]
fn scaled_columns_treat_a_zero_cell_as_the_fallback() {
    let zero = CellSize {
        width: 0,
        height: 16,
    };
    assert_eq!(
        ImageWidth::scaled_columns(1.0, 400.0, zero),
        ImageWidth::scaled_columns(1.0, 400.0, CellSize::FALLBACK)
    );
    assert_eq!(CellSize::FALLBACK, CELL_8X16);
}

#[test]
fn scale_resolves_from_the_natural_width_and_clamps_to_the_available_columns() {
    let layout = Layout::default();
    let dims = TerminalImage::resolve_scaled_dimensions_for(
        &ImageWidth::Scale(1.0),
        &layout,
        120,
        Some(400.0),
        Some(CELL_8X16),
    );
    assert_eq!(dims.image_width, 50);

    // Wider than the terminal: capped at the available columns.
    let dims = TerminalImage::resolve_scaled_dimensions_for(
        &ImageWidth::Scale(1.0),
        &layout,
        40,
        Some(400.0),
        Some(CELL_8X16),
    );
    assert_eq!(dims.image_width, 40);

    // Margins shrink the cap.
    let mut margined = Layout::default();
    margined.margin.left = TargetValue::universal(Length::ch(5));
    margined.margin.right = TargetValue::universal(Length::ch(5));
    let dims = TerminalImage::resolve_scaled_dimensions_for(
        &ImageWidth::Scale(1.0),
        &margined,
        40,
        Some(400.0),
        Some(CELL_8X16),
    );
    assert_eq!((dims.available_width, dims.image_width), (30, 30));
}

#[test]
fn scale_without_a_cell_size_uses_the_8x16_fallback() {
    let dims = TerminalImage::resolve_scaled_dimensions_for(
        &ImageWidth::Scale(1.25),
        &Layout::default(),
        200,
        Some(400.0),
        None,
    );
    assert_eq!(dims.image_width, 63);
}

#[test]
fn scale_without_a_natural_width_fills_the_available_columns() {
    let dims = TerminalImage::resolve_dimensions_for(&ImageWidth::Scale(1.0), &Layout::default(), 90);
    assert_eq!(dims.image_width, dims.available_width);
}

#[test]
fn other_widths_ignore_the_natural_width() {
    for width in [
        ImageWidth::Fill,
        ImageWidth::Percent(0.5),
        ImageWidth::Characters(25),
    ] {
        let plain = TerminalImage::resolve_dimensions_for(&width, &Layout::default(), 100);
        let scaled = TerminalImage::resolve_scaled_dimensions_for(
            &width,
            &Layout::default(),
            100,
            Some(4000.0),
            Some(CELL_8X16),
        );
        assert_eq!(plain.image_width, scaled.image_width, "{width:?}");
    }
}

#[test]
fn legacy_display_dimensions_scale_the_pixel_width() {
    assert_eq!(
        calculate_display_dimensions(200, 100, &ImageWidth::Scale(0.5), 80),
        (100, 50)
    );
}

/// Whole rows stay whole: the `f32` product put 13 rows at `13.000001` and
/// reserved 14, a blank row under a Kitty image (seen at L2 in `wt list`).
#[test]
fn covered_rows_are_exact_on_whole_rows() {
    use super::cursor::covered_rows;
    assert_eq!(covered_rows(56, (784, 325), (14, 25)), 13);
    assert_eq!(covered_rows(20, (280, 25), (14, 25)), 1);
    assert_eq!(covered_rows(56, (784, 326), (14, 25)), 14);
    // Drawn at half its pixel width, so half its height: 200 px in 20 px rows.
    assert_eq!(covered_rows(50, (1000, 400), (10, 20)), 10);
}

// Route-driven dispatch: `render_with_route` picks the encoder from the
// terminal's graphics route, or the alt-text fallback with the route's reason.
mod route_dispatch {
    use super::*;
    use crate::discovery::detection::{
        GraphicsRoute, GraphicsTransport, ImageSupport, RouteConfidence, RouteInputs,
        RouteReasonKind, resolve_graphics_route,
    };

    fn terminal_with_route(app: TerminalApp, route: GraphicsRoute) -> Terminal {
        Terminal::builder()
            .app(app)
            .is_tty(true)
            .graphics_route(route)
            .width(80)
            .height(24)
            .cell_size(CellSize {
                width: 8,
                height: 16,
            })
            .build()
    }

    fn route_for(vars: &[(&str, &str)]) -> GraphicsRoute {
        let inputs = vars
            .iter()
            .fold(RouteInputs::new(true), |inputs, (k, v)| inputs.with_var(k, *v));
        resolve_graphics_route(&inputs)
    }

    #[test]
    fn a_valid_image_without_a_pixel_route_falls_back_to_its_alt_text_with_the_reason() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap();
        let route = route_for(&[("TERM_PROGRAM", "Apple_Terminal")]);
        let mut term = terminal_with_route(TerminalApp::AppleTerminal, route.clone());
        // Without truecolor, so `render` cannot draw half-blocks either.
        term.color_depth = crate::discovery::detection::ColorDepth::Enhanced;

        let output = image.render_with_route(&term).unwrap();

        assert_eq!(
            output,
            ImageOutput::Fallback {
                text: "[Image: test.png]".to_string(),
                reason: route.reason.clone(),
            }
        );
        assert_eq!(image.render(&term), "[Image: test.png]");
    }

    #[test]
    fn a_user_disabled_route_falls_back_and_keeps_the_user_disabled_reason() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap().with_alt_text("a red square");
        let route = route_for(&[("TERM_PROGRAM", "WezTerm"), ("BISCUIT_GRAPHICS", "none")]);
        let term = terminal_with_route(TerminalApp::Wezterm, route);

        match image.render_with_route(&term).unwrap() {
            ImageOutput::Fallback { text, reason } => {
                assert_eq!(text, "a red square", "explicit alt text wins over the file name");
                assert_eq!(reason.kind, RouteReasonKind::UserDisabled);
            }
            other => panic!("expected a fallback, got {other:?}"),
        }
    }

    fn windows_ssh_iterm2_route() -> GraphicsRoute {
        let mut inputs = RouteInputs::new(true)
            .with_var("SSH_CONNECTION", "1 2 3 4")
            .with_var("LC_TERMINAL", "WezTerm");
        inputs.is_windows = true;
        let route = resolve_graphics_route(&inputs);
        assert!(route.needs_cursor_resync, "{route:?}");
        route
    }

    /// `output` with `" \r"` inserted right after the image sequence's
    /// closing restore-cursor.
    fn with_resync_after_image(output: &str) -> String {
        let end = output.rfind("\x1b[u").expect("restore cursor") + "\x1b[u".len();
        format!("{} \r{}", &output[..end], &output[end..])
    }

    /// On the Windows SSH server the console host drops a CR it thinks is a
    /// no-op, so the iTerm2 image is followed by a space and a CR before the
    /// usual tail. Everything else is the bytes the route sent before.
    #[test]
    fn a_windows_ssh_iterm2_image_is_followed_by_a_space_and_a_carriage_return() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap().with_width(ImageWidth::Characters(10));
        let route = windows_ssh_iterm2_route();
        let mut unflagged_route = route.clone();
        unflagged_route.needs_cursor_resync = false;
        let term = terminal_with_route(TerminalApp::Wezterm, route);
        let unflagged = terminal_with_route(TerminalApp::Wezterm, unflagged_route);

        let ImageOutput::Pixels(output) = image.render_with_route(&term).unwrap() else {
            panic!("the Windows SSH iTerm2 route draws pixels");
        };
        let ImageOutput::Pixels(before) = image.render_with_route(&unflagged).unwrap() else {
            panic!("the unflagged route draws pixels");
        };

        let (sequence, rows, _) = image.render_iterm2_for_terminal(&term).unwrap();
        assert!(sequence.ends_with("\x07\x1b[u"), "{sequence:?}");
        assert!(
            output.starts_with(&format!("{sequence} \r\x1b[{rows}B\r")),
            "image, space, CR, then the existing tail: {:?}",
            &output[sequence.len()..]
        );
        assert_eq!(output, with_resync_after_image(&before));
        assert!(!before.contains(" \r"), "{before:?}");
    }

    /// A direct iTerm2 route (macOS iTerm2) gets no resync.
    #[test]
    fn a_direct_iterm2_image_gets_no_space_and_carriage_return() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap().with_width(ImageWidth::Characters(10));
        let route = route_for(&[("TERM_PROGRAM", "iTerm.app")]);
        assert!(!route.needs_cursor_resync);
        let term = terminal_with_route(TerminalApp::ITerm2, route);

        let ImageOutput::Pixels(output) = image.render_with_route(&term).unwrap() else {
            panic!("iTerm2 draws pixels");
        };
        let (sequence, rows, _) = image.render_iterm2_for_terminal(&term).unwrap();
        assert!(output.starts_with(&format!("{sequence}\x1b[{rows}B\r")), "{output:?}");
        let (inline, _) = image.render_inline(&term).unwrap().into_overlay();
        assert!(inline.ends_with("\x07\x1b[u"), "{inline:?}");
    }

    /// The inline overlay on the Windows SSH route carries the same space and
    /// CR right after the image, and starts the standalone output.
    #[test]
    fn inline_and_standalone_windows_ssh_images_share_the_resync() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap().with_width(ImageWidth::Characters(10));
        let route = windows_ssh_iterm2_route();
        let mut unflagged_route = route.clone();
        unflagged_route.needs_cursor_resync = false;
        let term = terminal_with_route(TerminalApp::Wezterm, route);
        let unflagged = terminal_with_route(TerminalApp::Wezterm, unflagged_route);

        let (inline, rows) = image.render_inline(&term).unwrap().into_overlay();
        let (inline_before, rows_before) = image.render_inline(&unflagged).unwrap().into_overlay();
        assert_eq!(rows, rows_before);
        assert_eq!(inline, format!("{inline_before} \r"));

        let ImageOutput::Pixels(standalone) = image.render_with_route(&term).unwrap() else {
            panic!("the Windows SSH iTerm2 route draws pixels");
        };
        assert!(standalone.starts_with(&inline), "inline {inline:?} vs standalone {standalone:?}");
    }

    /// A 4-column image on an 8x16 cell is 32 pixels wide; the 2x2 fixture
    /// is square, so 32 pixels tall: two rows.
    fn sixel_image(file_path: &Path) -> TerminalImage {
        TerminalImage::new(file_path).unwrap().with_width(ImageWidth::Characters(4))
    }

    #[test]
    fn a_sixel_route_reserves_its_rows_then_draws_and_moves_below_the_image() {
        let (_dir, file_path) = create_temp_test_image();
        let route = route_for(&[("BISCUIT_GRAPHICS", "sixel")]);
        let term = terminal_with_route(TerminalApp::Other("xterm".into()), route);

        let ImageOutput::Pixels(output) = sixel_image(&file_path).render_with_route(&term).unwrap() else {
            panic!("expected pixels");
        };

        // Index twice (scrolling at the viewport bottom), back up, save the
        // cursor, draw, restore it, then move below the two rows.
        let (prefix, rest) = output.split_at(output.find("\x1bP").expect("a DCS"));
        assert_eq!(prefix, "\x1bD\x1bD\x1b[2A\x1b[s");
        assert!(rest.starts_with("\x1bP9;1;0q\"1;1;32;32"), "{rest:?}");
        assert!(rest.ends_with("\x1b\\\x1b[u\x1b[2B\r"), "{rest:?}");
        assert!(!output.contains("\x1b[?"), "no private mode is set or reset: {output:?}");
    }

    #[test]
    fn a_sixel_image_with_a_left_offset_moves_right_after_saving_the_cursor() {
        let (_dir, file_path) = create_temp_test_image();
        let route = route_for(&[("BISCUIT_GRAPHICS", "sixel")]);
        let term = terminal_with_route(TerminalApp::Other("xterm".into()), route);
        let image = sixel_image(&file_path).with_margins(10, 0);

        let output = image.render_with_route(&term).unwrap().into_text();

        assert!(output.contains("\x1b[s\x1b[10C\x1bP"), "{output:?}");
    }

    #[test]
    fn a_sixel_image_leaves_transparency_unset_and_blends_edges_on_the_background() {
        let png = |alpha: u8| {
            let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, alpha]));
            let mut buffer = std::io::Cursor::new(Vec::new());
            img.write_to(&mut buffer, image::ImageFormat::Png).unwrap();
            TerminalImage::from_encoded_png(EncodedPng::new(buffer.into_inner()).unwrap())
                .with_width(ImageWidth::Characters(2))
        };
        let route = route_for(&[("BISCUIT_GRAPHICS", "sixel")]);
        let mut term = terminal_with_route(TerminalApp::Other("xterm".into()), route);
        term.background_color = Some(crate::discovery::osc_queries::RgbValue { r: 0, g: 128, b: 255 });

        use crate::discovery::osc_queries::ColorSource;
        for source in [ColorSource::Measured, ColorSource::Estimated] {
            term.background_source = source;
            let transparent = png(0).render_with_route(&term).unwrap().into_text();
            assert!(!transparent.contains("#0;2;"), "{source:?}: no color is painted: {transparent:?}");
            let edge = png(128).render_with_route(&term).unwrap().into_text();
            assert!(edge.contains("#0;2;50;25;50"), "{source:?}: blended onto the background: {edge:?}");
        }

        term.background_color = None;
        term.color_mode = crate::discovery::detection::ColorMode::Light;
        let estimated = png(0).render_with_route(&term).unwrap().into_text();
        assert!(!estimated.contains("#0;2;"), "the light estimate is never painted: {estimated:?}");
    }

    #[test]
    fn a_sixel_inline_image_is_an_overlay_that_returns_the_cursor() {
        let (_dir, file_path) = create_temp_test_image();
        let route = route_for(&[("BISCUIT_GRAPHICS", "sixel")]);
        let term = terminal_with_route(TerminalApp::Other("xterm".into()), route);

        let (sequence, rows) = sixel_image(&file_path).render_inline(&term).unwrap().into_overlay();

        assert_eq!(rows, 2);
        assert!(sequence.starts_with("\x1bD\x1bD\x1b[2A\x1b[s\x1bP9;1;0q"), "{sequence:?}");
        assert!(sequence.ends_with("\x1b\\\x1b[u"), "{sequence:?}");
    }

    #[test]
    fn a_sixel_route_through_tmux_passthrough_wraps_only_the_image() {
        let (_dir, file_path) = create_temp_test_image();
        let route = route_for(&[
            ("BISCUIT_GRAPHICS", "sixel"),
            ("BISCUIT_GRAPHICS_TRANSPORT", "tmux"),
        ]);
        assert_eq!(route.transport, GraphicsTransport::Tmux);
        let term = terminal_with_route(TerminalApp::Other("xterm".into()), route);

        let output = sixel_image(&file_path).render_with_route(&term).unwrap().into_text();

        assert!(
            output.starts_with("\x1bD\x1bD\x1b[2A\x1b[s\x1bPtmux;\x1b\x1bP9;1;0q"),
            "{output:?}"
        );
        assert!(output.ends_with("\x1b\x1b\\\x1b\\\x1b[u\x1b[2B\r"), "{output:?}");
    }

    #[test]
    fn a_corrupt_image_stays_an_error_without_a_pixel_route() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("broken.png");
        std::fs::write(&file_path, b"\x89PNG\r\n\x1a\nnot really a png").unwrap();
        let image = TerminalImage::new(&file_path).unwrap();
        let term = terminal_with_route(TerminalApp::AppleTerminal, ImageSupport::None.into());

        assert!(image.render_with_route(&term).is_err());
        assert_eq!(image.render(&term), "", "a load error renders nothing");
    }

    #[test]
    fn fallback_text_cannot_carry_terminal_controls() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path)
            .unwrap()
            .with_alt_text("evil\x1b]0;title\x07\x1b[31mred\n");
        let term = terminal_with_route(TerminalApp::AppleTerminal, ImageSupport::None.into());

        let ImageOutput::Fallback { text, .. } = image.render_with_route(&term).unwrap() else {
            panic!("expected a fallback");
        };
        assert_eq!(text, "evil]0;title[31mred");
    }

    // The pixel cases assert the encoder choice: rendering pixels queries
    // the live cursor position, which tests cannot answer.
    use super::protocol::{Encoder, encoder_for};

    #[test]
    fn a_known_kitty_route_to_iterm2_uses_iterm2s_format() {
        let term = terminal_with_route(TerminalApp::ITerm2, ImageSupport::Kitty.into());
        assert_eq!(encoder_for(&term), Encoder::Iterm2);
        let elsewhere = terminal_with_route(TerminalApp::Kitty, ImageSupport::Kitty.into());
        assert_eq!(encoder_for(&elsewhere), Encoder::Kitty);
    }

    #[test]
    fn a_forced_kitty_route_to_iterm2_keeps_kitty() {
        let route = route_for(&[("TERM_PROGRAM", "iTerm.app"), ("BISCUIT_GRAPHICS", "kitty")]);
        assert_eq!(route.confidence, RouteConfidence::Forced);
        let term = terminal_with_route(TerminalApp::ITerm2, route);
        assert_eq!(encoder_for(&term), Encoder::Kitty);
    }

    /// A Kitty route through a declared tmux (`BISCUIT_GRAPHICS_TRANSPORT=tmux`).
    fn tmux_terminal(format: &str) -> Terminal {
        let route = route_for(&[
            ("SSH_CONNECTION", "10.0.0.1 50000 10.0.0.2 22"),
            ("BISCUIT_GRAPHICS_TRANSPORT", "tmux"),
            ("BISCUIT_GRAPHICS", format),
        ]);
        assert_eq!(route.transport, crate::discovery::detection::GraphicsTransport::Tmux);
        terminal_with_route(TerminalApp::Other("tmux".into()), route)
    }

    /// Every APC in `text`, each of which must sit in its own tmux envelope.
    fn wrapped_kitty_chunks(text: &str) -> Vec<&str> {
        text.split("\x1bPtmux;").skip(1).collect()
    }

    #[test]
    fn a_tmux_kitty_route_wraps_each_chunk_with_an_id_and_explicit_rows() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap().with_width(ImageWidth::Characters(10));
        let term = tmux_terminal("kitty");
        assert_eq!(encoder_for(&term), Encoder::Kitty);

        let (sequence, rows) = image.render_inline(&term).unwrap().into_overlay();

        assert!(sequence.starts_with("\x1b[s"), "cursor save stays unwrapped: {sequence:?}");
        assert!(sequence.ends_with("\x1b[u"), "cursor restore stays unwrapped: {sequence:?}");
        let chunks = wrapped_kitty_chunks(&sequence);
        assert!(!chunks.is_empty(), "{sequence:?}");
        for chunk in &chunks {
            assert!(chunk.starts_with("\x1b\x1b_G"), "ESC doubled once: {chunk:?}");
            assert!(chunk.contains("\x1b\x1b\\\x1b\\"), "inner ST doubled, then the envelope's: {chunk:?}");
        }
        let first = chunks[0];
        assert!(first.contains("q=2,"), "quiet mode: {first:?}");
        let id: u32 = first
            .split("i=")
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .and_then(|id| id.parse().ok())
            .expect("the upload carries an image ID");
        assert_ne!(id, 0);
        assert!(first.contains(&format!(",c=10,r={rows},")), "{first:?}");

        let (again, _) = image.render_inline(&term).unwrap().into_overlay();
        assert!(!again.contains(&format!("i={id},")), "every upload gets a fresh ID");
    }

    #[test]
    fn a_direct_kitty_route_is_not_wrapped() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap();
        let term = terminal_with_route(TerminalApp::Kitty, ImageSupport::Kitty.into());
        let (sequence, _) = image.render_inline(&term).unwrap().into_overlay();
        assert!(!sequence.contains("tmux;"), "{sequence:?}");
    }

    /// A PNG of `side`x`side` noise, which does not compress: at 64 pixels
    /// on a side its base64 needs several 4096-byte Kitty chunks.
    fn create_noise_test_image(side: u32) -> (tempfile::TempDir, std::path::PathBuf) {
        use image::{ImageBuffer, Rgb};
        let mut state: u32 = 0x1234_5678;
        let img = ImageBuffer::from_fn(side, side, |_, _| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let [r, g, b, _] = state.to_le_bytes();
            Rgb([r, g, b])
        });
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noise.png");
        img.save(&path).unwrap();
        (dir, path)
    }

    /// The parameters of the single Kitty upload in `output`, which must be
    /// quiet and carry a nonzero image ID on its first chunk only, spread
    /// over more than one chunk. Returns the parameters and the ID.
    fn numbered_upload(output: &str) -> (String, u32) {
        assert!(!output.contains("tmux;"), "a direct upload: {output:?}");
        let chunks: Vec<&str> = output.split("\x1b_G").skip(1).collect();
        assert!(chunks.len() > 1, "a multi-chunk upload: {} chunk(s)", chunks.len());
        let params = &chunks[0][..chunks[0].find(';').expect("parameters end at ';'")];
        assert!(params.starts_with("q=2,i="), "quiet, then the ID: {params:?}");
        let id: u32 = params["q=2,i=".len()..]
            .split(',')
            .next()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("a numeric ID: {params:?}"));
        assert_ne!(id, 0, "{params:?}");
        assert!(params.ends_with(",m=1"), "{params:?}");
        for chunk in &chunks[1..] {
            assert!(chunk.starts_with("m="), "a continuation carries only m=: {chunk:?}");
            assert!(!chunk[..chunk.find(';').unwrap()].contains("i="), "{chunk:?}");
        }
        (params.to_string(), id)
    }

    fn direct_kitty_image(file_path: &Path) -> TerminalImage {
        TerminalImage::new(file_path).unwrap().with_width(ImageWidth::Characters(4))
    }

    fn standalone_upload(image: &TerminalImage, term: &Terminal) -> (String, u32) {
        let ImageOutput::Pixels(output) = image.render_with_route(term).unwrap() else {
            panic!("a direct Kitty route draws pixels");
        };
        numbered_upload(&output)
    }

    fn two_column_upload(image: TerminalImage, term: &Terminal) -> (String, u32) {
        use crate::components::two_column::TwoColumn;
        numbered_upload(&TwoColumn::new(image, "Caption").render(term))
    }

    /// Kitty sizes by width alone; each standalone upload is quiet and
    /// carries its own nonzero ID on the first chunk.
    #[test]
    fn a_standalone_direct_kitty_upload_is_numbered_and_sized_by_width() {
        let (_dir, file_path) = create_noise_test_image(64);
        let image = direct_kitty_image(&file_path);
        let term = terminal_with_route(TerminalApp::Kitty, ImageSupport::Kitty.into());

        let (params, id) = standalone_upload(&image, &term);
        assert!(params.contains(",f=100,a=T,t=d,c=4,m=1"), "width only: {params:?}");
        let (_, again) = standalone_upload(&image, &term);
        assert_ne!(again, id, "every upload gets a fresh ID");
    }

    #[test]
    fn an_inline_direct_kitty_upload_is_numbered_and_sized_by_width() {
        let (_dir, file_path) = create_noise_test_image(64);
        let term = terminal_with_route(TerminalApp::Kitty, ImageSupport::Kitty.into());

        let (params, id) = two_column_upload(direct_kitty_image(&file_path), &term);
        assert!(params.contains(",f=100,a=T,t=d,c=4,m=1"), "width only: {params:?}");
        let (_, again) = two_column_upload(direct_kitty_image(&file_path), &term);
        assert_ne!(again, id, "every upload gets a fresh ID");
    }

    /// WezTerm gets explicit rows; the 64-pixel square on 8x16 cells is
    /// four columns by two rows.
    #[test]
    fn a_standalone_direct_wezterm_upload_is_numbered_with_explicit_cells() {
        let (_dir, file_path) = create_noise_test_image(64);
        let image = direct_kitty_image(&file_path);
        let term = terminal_with_route(TerminalApp::Wezterm, ImageSupport::Kitty.into());

        let (params, id) = standalone_upload(&image, &term);
        assert!(params.contains(",f=100,a=T,t=d,c=4,r=2,m=1"), "explicit cells: {params:?}");
        let (_, again) = standalone_upload(&image, &term);
        assert_ne!(again, id, "every upload gets a fresh ID");
    }

    #[test]
    fn an_inline_direct_wezterm_upload_is_numbered_with_explicit_cells() {
        let (_dir, file_path) = create_noise_test_image(64);
        let term = terminal_with_route(TerminalApp::Wezterm, ImageSupport::Kitty.into());

        let (params, id) = two_column_upload(direct_kitty_image(&file_path), &term);
        assert!(params.contains(",f=100,a=T,t=d,c=4,r=2,m=1"), "explicit cells: {params:?}");
        let (_, again) = two_column_upload(direct_kitty_image(&file_path), &term);
        assert_ne!(again, id, "every upload gets a fresh ID");
    }

    #[test]
    fn a_tmux_iterm2_route_wraps_the_file_message_once() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path).unwrap();
        let term = tmux_terminal("iterm2");
        assert_eq!(encoder_for(&term), Encoder::Iterm2);

        let (sequence, _) = image.render_inline(&term).unwrap().into_overlay();

        assert_eq!(sequence.matches("\x1bPtmux;").count(), 1, "{sequence:?}");
        assert!(sequence.contains("\x1bPtmux;\x1b\x1b]1337;File="), "{sequence:?}");
        assert!(sequence.contains("\x07\x1b\\\x1b[u"), "BEL ends the OSC, then the envelope: {sequence:?}");
    }

    /// An iTerm2 message whose tmux envelope is exactly `wrapped` bytes.
    fn iterm2_message_wrapping_to(wrapped: usize) -> String {
        let head = "\x1b]1337;File=inline=1:";
        // The closing BEL, the head's ESC doubled (the message's only ESC),
        // and the 9-byte envelope.
        let overhead = head.len() + 1 + 1 + 9;
        format!("{head}{}\x07", "A".repeat(wrapped - overhead))
    }

    #[test]
    fn the_tmux_message_budget_holds_at_its_boundary() {
        use super::tmux::TMUX_MESSAGE_BUDGET;
        let term = tmux_terminal("iterm2");
        for (size, fits) in [
            (TMUX_MESSAGE_BUDGET - 1, true),
            (TMUX_MESSAGE_BUDGET, true),
            (TMUX_MESSAGE_BUDGET + 1, false),
        ] {
            let message = iterm2_message_wrapping_to(size);
            match super::protocol::for_transport(&term, "iterm2", message) {
                Ok(wrapped) => {
                    assert!(fits, "{size} bytes must be refused");
                    assert_eq!(wrapped.len(), size, "the envelope is exactly {size} bytes");
                }
                Err(reason) => {
                    assert!(!fits, "{size} bytes must fit: {reason:?}");
                    assert_eq!(reason.kind, RouteReasonKind::BlockedTransport);
                    assert!(reason.message.contains(&size.to_string()), "{}", reason.message);
                    assert!(reason.message.contains("1048576-byte"), "{}", reason.message);
                }
            }
        }
    }

    #[test]
    fn the_budget_only_applies_through_tmux() {
        use super::tmux::TMUX_MESSAGE_BUDGET;
        let direct = terminal_with_route(TerminalApp::ITerm2, ImageSupport::ITerm.into());
        let message = iterm2_message_wrapping_to(TMUX_MESSAGE_BUDGET + 1);
        assert_eq!(
            super::protocol::for_transport(&direct, "iterm2", message.clone()),
            Ok(message)
        );
    }

    /// A PNG of noise, which does not compress, so its base64 iTerm2 message
    /// is over tmux's 1 MiB limit.
    fn create_oversized_test_image() -> (tempfile::TempDir, std::path::PathBuf) {
        create_noise_test_image(640)
    }

    #[test]
    fn an_oversized_tmux_iterm2_image_falls_back_before_reserving_rows() {
        let (_dir, file_path) = create_oversized_test_image();
        let image = TerminalImage::new(&file_path).unwrap();
        let mut term = tmux_terminal("iterm2");
        // Without truecolor, so the inline path cannot draw half-blocks.
        term.color_depth = crate::discovery::detection::ColorDepth::Enhanced;

        let ImageOutput::Fallback { text, reason } = image.render_with_route(&term).unwrap() else {
            panic!("an image over the tmux limit must fall back");
        };
        assert_eq!(text, "[Image: noise.png]", "alt text only: no escapes, no reserved rows");
        assert_eq!(reason.kind, RouteReasonKind::BlockedTransport);
        assert!(reason.message.contains("tmux passthrough"), "{}", reason.message);
        assert!(matches!(
            image.render_inline(&term),
            Err(TerminalImageError::UnsupportedTerminal)
        ));
    }

    /// A Zellij pane that answered the probe, without (then with) a measured
    /// cell size.
    #[test]
    fn a_route_needing_measured_cells_falls_back_without_a_cell_size() {
        use crate::discovery::detection::{KittyReply, ProbeReply, ProbeResult};
        let mut inputs = RouteInputs::new(true).with_var("ZELLIJ", "0");
        inputs.zellij_version = Some(crate::discovery::detection::ToolVersion::new(0, 45, 0));
        inputs.probe.direct = Some(ProbeResult::Replied(ProbeReply {
            kitty: KittyReply::Accepted,
            da1: None,
        }));
        let route = resolve_graphics_route(&inputs);
        assert!(route.needs_measured_cells, "{route:?}");

        let reason = super::protocol::unmeasured_cells_reason(&route, None)
            .expect("no measured cell size means no pixels");
        assert!(reason.message.contains("cell size"), "{}", reason.message);
        let cell = CellSize {
            width: 8,
            height: 16,
        };
        assert_eq!(super::protocol::unmeasured_cells_reason(&route, Some(cell)), None);
        let unaffected: GraphicsRoute = ImageSupport::Kitty.into();
        assert_eq!(super::protocol::unmeasured_cells_reason(&unaffected, None), None);

        let measured = terminal_with_route(TerminalApp::Other("zellij".into()), route);
        assert_eq!(encoder_for(&measured), Encoder::Kitty);
    }

    /// `route_has_pixels` answers as `render_with_route` draws: no for a
    /// route without a pixel format, yes for each pixel format and for a
    /// Zellij route with a measured cell size. (An unmeasured cell size is
    /// the encoder's own test above; `Terminal::cell_size` would measure the
    /// host here.)
    #[test]
    fn route_has_pixels_matches_the_routes_encoder() {
        use crate::discovery::detection::{KittyReply, ProbeReply, ProbeResult};
        let plain = route_for(&[("TERM_PROGRAM", "Apple_Terminal")]);
        assert!(!TerminalImage::route_has_pixels(&terminal_with_route(TerminalApp::AppleTerminal, plain)));
        for support in [ImageSupport::Kitty, ImageSupport::ITerm, ImageSupport::Sixel] {
            let term = terminal_with_route(TerminalApp::Kitty, support.clone().into());
            assert!(TerminalImage::route_has_pixels(&term), "{support:?}");
        }

        let mut inputs = RouteInputs::new(true).with_var("ZELLIJ", "0");
        inputs.zellij_version = Some(crate::discovery::detection::ToolVersion::new(0, 45, 0));
        inputs.probe.direct = Some(ProbeResult::Replied(ProbeReply {
            kitty: KittyReply::Accepted,
            da1: None,
        }));
        let route = resolve_graphics_route(&inputs);
        assert!(route.needs_measured_cells, "{route:?}");
        let zellij = terminal_with_route(TerminalApp::Other("zellij".into()), route);
        assert!(TerminalImage::route_has_pixels(&zellij), "a measured cell size");
    }

    #[test]
    fn an_iterm2_route_uses_the_iterm2_format_on_any_app() {
        let route = route_for(&[("TERM_PROGRAM", "mintty")]);
        let term = terminal_with_route(TerminalApp::Other("mintty".into()), route);
        assert_eq!(encoder_for(&term), Encoder::Iterm2);
    }
}

/// Kitty through tmux with Unicode placeholders: a `U=1` upload in tmux
/// envelopes, then the placeholder grid as ordinary, unwrapped text.
mod placeholder_dispatch {
    use super::*;
    use crate::discovery::detection::{
        GraphicsPlacement, GraphicsTransport, Passthrough, RouteInputs, TmuxClient, TmuxFacts,
        ToolVersion, resolve_graphics_route,
    };

    /// A terminal inside tmux 3.4 with passthrough on and one kitty client,
    /// `width` columns wide, its route resolved from those facts.
    fn kitty_in_tmux(width: u32, termfeatures: &str) -> Terminal {
        let mut inputs = RouteInputs::new(true)
            .with_var("TMUX", "/tmp/tmux-501/default,1234,0")
            .with_var("TMUX_PANE", "%3")
            .with_var("TERM", "tmux-256color");
        inputs.tmux = TmuxFacts {
            version: Some(ToolVersion::new(3, 4, 0)),
            passthrough: Passthrough::On,
            clients: Some(vec![TmuxClient {
                termname: "xterm-kitty".into(),
                termtype: "kitty(0.49.2)".into(),
                utf8: true,
                termfeatures: termfeatures.into(),
            }]),
        };
        let route = resolve_graphics_route(&inputs);
        assert_eq!(route.transport, GraphicsTransport::Tmux, "{route:?}");
        Terminal::builder()
            .app(TerminalApp::Other("tmux".into()))
            .is_tty(true)
            .graphics_route(route)
            .width(width)
            .height(24)
            .cell_size(CellSize {
                width: 8,
                height: 16,
            })
            .build()
    }

    /// The `key=` value from the first Kitty chunk's parameters.
    fn param(first_chunk: &str, key: &str) -> u32 {
        first_chunk
            .split(&format!(",{key}="))
            .nth(1)
            .and_then(|rest| rest.split([',', ';']).next())
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("no {key}= in {first_chunk:?}"))
    }

    /// Splits `text` after its last tmux envelope: the envelopes, then the rest.
    fn split_after_envelopes(text: &str) -> (&str, &str) {
        let end = text.rfind("\x1b\\").expect("an envelope") + 2;
        text.split_at(end)
    }

    fn id_color(id: u32) -> String {
        format!("\x1b[38;2;{};{};{}m", id >> 16, (id >> 8) & 0xFF, id & 0xFF)
    }

    #[test]
    fn the_standalone_path_uploads_a_virtual_placement_then_prints_the_grid() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(10));
        let term = kitty_in_tmux(80, "RGB,title");
        assert_eq!(term.graphics_route.placement, GraphicsPlacement::Placeholders);

        let ImageOutput::Pixels(output) = image.render_with_route(&term).unwrap() else {
            panic!("a placeholder route draws pixels");
        };

        let (upload, grid) = split_after_envelopes(&output);
        let chunks: Vec<&str> = upload.split("\x1bPtmux;").skip(1).collect();
        assert!(!chunks.is_empty(), "{upload:?}");
        for chunk in &chunks {
            assert!(chunk.starts_with("\x1b\x1b_G"), "each chunk wrapped once: {chunk:?}");
        }
        let first = chunks[0];
        assert!(first.contains("q=2,"), "quiet: {first:?}");
        assert!(first.contains(",a=T,U=1,"), "virtual placement: {first:?}");
        let id = param(first, "i");
        assert!((1..=0xFF_FFFF).contains(&id), "a 24-bit nonzero ID: {id}");
        assert_eq!(param(first, "c"), 10);
        let rows = param(first, "r");
        assert!(rows > 0);

        assert!(!grid.contains("tmux;"), "the grid is never wrapped: {grid:?}");
        assert!(!grid.contains("\x1b[s") && !grid.contains("\x1b[u"), "no cursor overlay: {grid:?}");
        let lines: Vec<&str> = grid.split_terminator("\r\n").collect();
        assert_eq!(lines.len(), rows as usize, "{grid:?}");
        assert!(grid.ends_with("\x1b[39m\r\n"), "fg reset, cursor below the image: {grid:?}");
        for (row, line) in lines.iter().enumerate() {
            assert!(line.starts_with(&id_color(id)), "row {row}: {line:?}");
            assert!(line.ends_with("\x1b[39m"), "row {row}: {line:?}");
            assert_eq!(line.matches('\u{10EEEE}').count(), 10, "row {row}: {line:?}");
        }

        let ImageOutput::Pixels(again) = image.render_with_route(&term).unwrap() else {
            panic!("a placeholder route draws pixels");
        };
        let again_first = split_after_envelopes(&again).0.split("\x1bPtmux;").nth(1).unwrap();
        assert_ne!(param(again_first, "i"), id, "every upload gets a fresh ID");
    }

    #[test]
    fn the_inline_path_returns_one_text_line_per_image_row() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(6));
        let term = kitty_in_tmux(40, "RGB");

        let InlineImage::Lines(lines) = image.render_inline(&term).unwrap() else {
            panic!("placeholders render as lines of text");
        };

        let (upload, first_row) = split_after_envelopes(&lines[0]);
        assert!(upload.starts_with("\x1bPtmux;\x1b\x1b_G"), "{upload:?}");
        assert!(upload.contains(",U=1,"), "{upload:?}");
        let first_chunk = upload.split("\x1bPtmux;").nth(1).unwrap();
        let id = param(first_chunk, "i");
        assert!((1..=0xFF_FFFF).contains(&id), "a 24-bit nonzero ID: {id}");
        assert!(first_row.starts_with(&id_color(id)), "the grid draws that ID: {first_row:?}");
        let rows = param(first_chunk, "r");
        assert_eq!(lines.len(), rows as usize);
        for line in std::iter::once(first_row).chain(lines[1..].iter().map(String::as_str)) {
            assert!(!line.contains("tmux;"), "{line:?}");
            assert_eq!(line.matches('\u{10EEEE}').count(), 6, "{line:?}");
            assert!(line.ends_with("\x1b[39m"), "{line:?}");
            assert_eq!(crate::utils::block_constraint::visible_width(line), 6, "{line:?}");
        }
        assert_eq!(
            crate::utils::block_constraint::visible_width(&lines[0]),
            6,
            "the wrapped upload takes no columns: {:?}",
            lines[0]
        );
    }

    #[test]
    fn without_rgb_the_same_client_gets_an_ordinary_overlay() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(10));
        let term = kitty_in_tmux(80, "title");
        assert_eq!(term.graphics_route.placement, GraphicsPlacement::Ordinary);

        let (sequence, _) = image.render_inline(&term).unwrap().into_overlay();
        assert!(!sequence.contains("U=1"), "{sequence:?}");
        assert!(!sequence.contains('\u{10EEEE}'), "{sequence:?}");
        let ImageOutput::Pixels(output) = image.render_with_route(&term).unwrap() else {
            panic!("an ordinary tmux route draws pixels");
        };
        assert!(output.starts_with("\x1b[s"), "{output:?}");
        assert!(!output.contains("U=1"), "{output:?}");
        let first = output.split("\x1bPtmux;").nth(1).expect("a wrapped upload");
        assert!(first.contains("q=2,"), "quiet: {first:?}");
        assert_ne!(param(first, "i"), 0, "{first:?}");
    }

    #[test]
    fn a_grid_wider_than_the_diacritic_table_places_the_image_ordinarily() {
        let (_dir, file_path) = create_temp_test_image();
        let image = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(300));
        let term = kitty_in_tmux(400, "RGB");
        assert_eq!(term.graphics_route.placement, GraphicsPlacement::Placeholders);

        let ImageOutput::Pixels(output) = image.render_with_route(&term).unwrap() else {
            panic!("the ordinary placement still draws pixels");
        };
        assert!(output.starts_with("\x1b[s"), "ordinary overlay: {output:?}");
        assert!(!output.contains("U=1"), "{output:?}");
        assert!(output.contains(",c=300,"), "{output:?}");
        let (sequence, _) = image.render_inline(&term).unwrap().into_overlay();
        assert!(!sequence.contains("U=1"), "{sequence:?}");
    }
}
