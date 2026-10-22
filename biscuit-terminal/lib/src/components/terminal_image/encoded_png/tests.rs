//! The encoded-PNG input: validation before anything is reserved, and bytes
//! that reach the Kitty and iTerm2 encoders unchanged on every dispatch path.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageError, RgbaImage};

use super::super::{ImageWidth, TerminalImage, TerminalImageError};
use super::{EncodedPng, encode_png};
use crate::discovery::detection::{ImageSupport, TerminalApp};
use crate::discovery::fonts::CellSize;
use crate::terminal::Terminal;

/// 80x48 pixels: 10 columns by exactly 3 rows of 8x16 cells.
fn pattern() -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_fn(80, 48, |x, y| {
        image::Rgba([(x * 3) as u8, (y * 5) as u8, ((x + y) * 7) as u8, 255])
    }))
}

/// The pattern encoded with settings this crate never uses, so any
/// re-encoding on the way to the terminal changes the bytes.
fn fast_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    pattern()
        .write_with_encoder(PngEncoder::new_with_quality(
            &mut bytes,
            CompressionType::Fast,
            FilterType::NoFilter,
        ))
        .unwrap();
    bytes
}

fn terminal(app: TerminalApp, support: ImageSupport) -> Terminal {
    Terminal::builder()
        .app(app)
        .is_tty(true)
        .image_support(support)
        .width(80)
        .cell_size(CellSize {
            width: 8,
            height: 16,
        })
        .build()
}

/// The base64 payload of every Kitty chunk, decoded.
fn kitty_payload(output: &str) -> Vec<u8> {
    let joined: String = output
        .split("\x1b_G")
        .skip(1)
        .map(|chunk| {
            let body = &chunk[chunk.find(';').unwrap() + 1..];
            &body[..body.find("\x1b\\").unwrap()]
        })
        .collect();
    BASE64.decode(joined).unwrap()
}

/// The base64 payload of the iTerm2 `File=` sequence, decoded.
fn iterm2_payload(output: &str) -> Vec<u8> {
    let start = output.find("size=auto:").unwrap() + "size=auto:".len();
    let end = start + output[start..].find('\x07').unwrap();
    BASE64.decode(&output[start..end]).unwrap()
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    let mut typed = kind.to_vec();
    typed.extend_from_slice(data);
    out.extend_from_slice(&typed);
    out.extend_from_slice(&crc32(&typed).to_be_bytes());
    out
}

/// A well-formed PNG header claiming `width` x `height` RGBA pixels, followed
/// by an empty image-data chunk: enough to read the size, never decodable.
fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut ihdr = width.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend(chunk(b"IHDR", &ihdr));
    bytes.extend(chunk(b"IDAT", &[]));
    bytes.extend(chunk(b"IEND", &[]));
    bytes
}

#[test]
fn the_fixture_would_change_if_it_were_re_encoded() {
    assert_ne!(encode_png(&pattern()).unwrap(), fast_png());
}

#[test]
fn the_pixel_size_comes_from_the_header() {
    let png = EncodedPng::new(fast_png()).unwrap();
    assert_eq!((png.width(), png.height()), (80, 48));
    assert_eq!(png.bytes(), fast_png().as_slice());
}

#[test]
fn bytes_that_are_not_a_png_are_rejected() {
    for bytes in [Vec::new(), b"GIF89a".to_vec(), fast_png()[..20].to_vec()] {
        assert!(
            matches!(EncodedPng::new(bytes.clone()), Err(TerminalImageError::ImageLoadError(_))),
            "{} bytes",
            bytes.len()
        );
    }
}

#[test]
fn more_bytes_than_the_file_size_limit_are_rejected_before_reading_the_header() {
    let mut bytes = fast_png();
    bytes.resize(10 * 1024 * 1024 + 1, 0);
    assert!(matches!(
        EncodedPng::new(bytes),
        Err(TerminalImageError::FileTooLarge {
            size: 10_485_761,
            max_size: 10_485_760
        })
    ));
}

#[test]
fn dimensions_beyond_the_allocation_limit_are_rejected_without_decoding() {
    // The control: the hand-built header itself reads fine.
    let small = EncodedPng::new(png_header(4, 4)).unwrap();
    assert_eq!((small.width(), small.height()), (4, 4));

    // 30000 x 30000 RGBA is 3.6 GB, over the 512 MiB decode limit.
    assert!(matches!(
        EncodedPng::new(png_header(30_000, 30_000)),
        Err(TerminalImageError::ImageLoadError(ImageError::Limits(_)))
    ));
}

#[test]
fn kitty_receives_the_bytes_unchanged_and_reserves_the_padded_rows() {
    let image = TerminalImage::from_encoded_png(EncodedPng::new(fast_png()).unwrap())
        .with_width(ImageWidth::Characters(10));

    for app in [TerminalApp::Kitty, TerminalApp::Wezterm] {
        let (sequence, rows, raw_rows) = image
            .render_kitty_for_terminal(&terminal(app.clone(), ImageSupport::Kitty))
            .unwrap();
        assert_eq!(kitty_payload(&sequence), fast_png(), "{app:?}");
        assert_eq!((rows, raw_rows), (3, 3.0), "{app:?}");
        assert!(sequence.contains("c=10"), "{app:?}");
    }
    let wezterm = image
        .render_kitty_for_terminal(&terminal(TerminalApp::Wezterm, ImageSupport::Kitty))
        .unwrap()
        .0;
    assert!(wezterm.contains("c=10,r=3,"));

    let optimistic = image.render_as_kitty(80).unwrap();
    assert_eq!(kitty_payload(&optimistic), fast_png());
}

#[test]
fn iterm2_receives_the_bytes_unchanged() {
    let image = TerminalImage::from_encoded_png(EncodedPng::new(fast_png()).unwrap())
        .with_width(ImageWidth::Characters(10));

    let (sequence, rows, _) = image
        .render_iterm2_for_terminal(&terminal(TerminalApp::ITerm2, ImageSupport::ITerm))
        .unwrap();
    assert_eq!(iterm2_payload(&sequence), fast_png());
    assert_eq!(rows, 3);
    assert!(sequence.contains("width=10;"));
    // No file: the protocol's name falls back to a generic one.
    assert!(sequence.contains(&format!("name={};", BASE64.encode("image.png"))));
}

#[test]
fn the_inline_path_sends_the_same_bytes_as_the_standalone_path() {
    let image = TerminalImage::from_encoded_png(EncodedPng::new(fast_png()).unwrap())
        .with_width(ImageWidth::Characters(10));

    let (kitty, kitty_rows) = image
        .render_inline(&terminal(TerminalApp::Kitty, ImageSupport::Kitty))
        .unwrap()
        .into_overlay();
    assert_eq!(kitty_payload(&kitty), fast_png());
    let (iterm2, iterm2_rows) = image
        .render_inline(&terminal(TerminalApp::ITerm2, ImageSupport::ITerm))
        .unwrap()
        .into_overlay();
    assert_eq!(iterm2_payload(&iterm2), fast_png());
    assert_eq!((kitty_rows, iterm2_rows), (3, 3));
}

#[test]
fn an_actual_resize_decodes_and_encodes_at_the_new_width() {
    let image = TerminalImage::from_encoded_png(EncodedPng::new(fast_png()).unwrap())
        .with_width(ImageWidth::Characters(5));

    let resized = iterm2_payload(&image.render_as_iterm2(80).unwrap());
    assert_ne!(resized, fast_png());
    let decoded = image::load_from_memory(&resized).unwrap();
    let cell = crate::discovery::fonts::cell_size().unwrap_or(CellSize::FALLBACK);
    assert_eq!(decoded.width(), 5 * cell.width.max(1));
}

#[test]
fn pixels_decode_from_the_encoded_bytes() {
    let image = TerminalImage::from_encoded_png(EncodedPng::new(fast_png()).unwrap());
    assert_eq!(image.load_image().unwrap().to_rgba8(), pattern().to_rgba8());
}

#[test]
fn encoding_keeps_the_pixels_and_beats_the_image_crate_default() {
    let mut default = std::io::Cursor::new(Vec::new());
    pattern().write_to(&mut default, image::ImageFormat::Png).unwrap();

    let ours = encode_png(&pattern()).unwrap();
    assert_eq!(image::load_from_memory(&ours).unwrap().to_rgba8(), pattern().to_rgba8());
    assert!(
        ours.len() < default.get_ref().len(),
        "{} >= {}",
        ours.len(),
        default.get_ref().len()
    );
}
