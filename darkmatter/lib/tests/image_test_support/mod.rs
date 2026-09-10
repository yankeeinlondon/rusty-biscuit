/// Encodes a square opaque PNG filled with one RGB color.
pub fn write_solid_png(path: &std::path::Path, size: u32, rgb: [u8; 3]) {
    let image = image::RgbImage::from_pixel(size, size, image::Rgb(rgb));
    image
        .save_with_format(path, image::ImageFormat::Png)
        .expect("encode probe PNG");
}

/// Counts near-target, non-black, and total pixels in a PNG capture.
pub fn classify_pixels(png: &[u8], target: [u8; 3], tolerance: i32) -> (u64, u64, u64) {
    let image = image::load_from_memory(png).expect("decode screen capture");
    let rgb = image.to_rgb8();
    let mut near_target = 0u64;
    let mut non_black = 0u64;
    let total = u64::from(rgb.width()) * u64::from(rgb.height());
    for pixel in rgb.pixels() {
        let [red, green, blue] = pixel.0;
        let near = (i32::from(red) - i32::from(target[0])).abs() <= tolerance
            && (i32::from(green) - i32::from(target[1])).abs() <= tolerance
            && (i32::from(blue) - i32::from(target[2])).abs() <= tolerance;
        if near {
            near_target += 1;
        }
        if red > 30 || green > 30 || blue > 30 {
            non_black += 1;
        }
    }
    (near_target, non_black, total)
}
