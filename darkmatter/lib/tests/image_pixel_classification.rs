#[path = "image_test_support/mod.rs"]
mod image_test_support;

use image_test_support::{classify_pixels, write_solid_png};

#[test]
fn pixel_classification_distinguishes_magenta_from_black() {
    const MAGENTA: [u8; 3] = [255, 0, 255];
    let directory = tempfile::tempdir().expect("temporary image directory");

    let magenta_path = directory.path().join("magenta.png");
    write_solid_png(&magenta_path, 64, MAGENTA);
    let (near, non_black, total) = classify_pixels(
        &std::fs::read(&magenta_path).expect("read magenta PNG"),
        MAGENTA,
        60,
    );
    assert_eq!(total, 64 * 64);
    assert_eq!(near, total, "every magenta pixel is near the target");
    assert_eq!(non_black, total, "every magenta pixel is non-black");

    let black_path = directory.path().join("black.png");
    write_solid_png(&black_path, 64, [0, 0, 0]);
    let (near, non_black, total) = classify_pixels(
        &std::fs::read(&black_path).expect("read black PNG"),
        MAGENTA,
        60,
    );
    assert_eq!(total, 64 * 64);
    assert_eq!(near, 0, "black has no near-magenta pixels");
    assert_eq!(non_black, 0, "black is the blocked-capture signature");
}
