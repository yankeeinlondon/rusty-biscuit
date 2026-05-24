//! Image width specification, parsing, and dimension calculation helpers.

use super::TerminalImageError;

/// Width specification for image rendering.
///
/// Controls how wide an image displays in the terminal. The width is resolved
/// against the available terminal space (terminal width minus left and right margins).
///
/// ## Variants
///
/// - **`Fill`**: Use all available space. The image expands to fill the entire
///   available width calculated from `terminal_width - left_margin - right_margin`.
///
/// - **`Percent(f32)`**: Use a percentage of available space. Values range from 0.0 to 1.0
///   (internally, the percentage is specified as 0-100 in width spec strings like `"50%"`).
///
/// - **`Characters(u32)`**: Fixed width in terminal character cells. The image is scaled
///   to occupy exactly this many columns.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::ImageWidth;
///
/// // Fill available space
/// let fill = ImageWidth::Fill;
///
/// // 50% of available width (default)
/// let half = ImageWidth::Percent(0.5);
///
/// // Fixed 80 character width
/// let fixed = ImageWidth::Characters(80);
/// ```
///
/// ## Width Specification Syntax
///
/// When parsing from strings (e.g., `"image.png|50%"`), these formats are supported:
/// - `"fill"` → `ImageWidth::Fill`
/// - `"50%"` → `ImageWidth::Percent(0.5)`
/// - `"80"` → `ImageWidth::Characters(80)`
/// - `"40ch"` → `ImageWidth::Characters(40)`
#[derive(Debug, Clone, PartialEq)]
pub enum ImageWidth {
    /// Fill available space (using margins as offsets).
    Fill,
    /// Use a percentage of the available space where
    /// available space is the number of characters - (left_margin + right_margin).
    Percent(f32),
    /// A fixed width based on character width.
    Characters(u32),
}

impl Default for ImageWidth {
    fn default() -> Self {
        ImageWidth::Percent(0.5)
    }
}

/// Calculate display dimensions while preserving aspect ratio.
///
/// ## Arguments
///
/// * `img_width` - Original image width in pixels
/// * `img_height` - Original image height in pixels
/// * `target_width` - Target width specification
/// * `term_width` - Terminal width in characters
///
/// ## Returns
///
/// Tuple of (width, height) in pixels for display.
pub fn calculate_display_dimensions(
    img_width: u32,
    img_height: u32,
    target_width: &ImageWidth,
    term_width: u32,
) -> (u32, u32) {
    // Assume roughly 2:1 pixel aspect ratio for terminal cells
    // (characters are typically ~twice as tall as wide)
    let cell_pixel_width = 8u32;

    let target_pixels = match target_width {
        ImageWidth::Fill => term_width * cell_pixel_width,
        ImageWidth::Percent(pct) => ((term_width as f32) * pct * (cell_pixel_width as f32)) as u32,
        ImageWidth::Characters(chars) => chars * cell_pixel_width,
    };

    // Calculate height preserving aspect ratio
    let aspect_ratio = img_height as f32 / img_width as f32;
    let display_width = target_pixels.min(img_width); // Don't upscale
    let display_height = (display_width as f32 * aspect_ratio) as u32;

    (display_width.max(1), display_height.max(1))
}

/// Parse a width specification string.
///
/// ## Supported formats
///
/// - Empty or whitespace: Default to 50% (`ImageWidth::Percent(0.5)`)
/// - `"fill"`: `ImageWidth::Fill`
/// - Number with `%` suffix: `ImageWidth::Percent(value / 100.0)`
/// - Number with `ch` suffix: `ImageWidth::Characters(value)` (e.g., "40ch")
/// - Bare number: `ImageWidth::Characters(value)`
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::{parse_width_spec, ImageWidth};
///
/// assert!(matches!(parse_width_spec("50%").unwrap(), ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
/// assert!(matches!(parse_width_spec("fill").unwrap(), ImageWidth::Fill));
/// assert!(matches!(parse_width_spec("80").unwrap(), ImageWidth::Characters(80)));
/// assert!(matches!(parse_width_spec("40ch").unwrap(), ImageWidth::Characters(40)));
/// ```
///
/// ## Errors
///
/// Returns `TerminalImageError::InvalidWidthSpec` for invalid specifications.
pub fn parse_width_spec(spec: &str) -> Result<ImageWidth, TerminalImageError> {
    let trimmed = spec.trim();

    // Empty or whitespace defaults to 50%
    if trimmed.is_empty() {
        return Ok(ImageWidth::Percent(0.5));
    }

    // Handle "fill" keyword
    if trimmed.eq_ignore_ascii_case("fill") {
        return Ok(ImageWidth::Fill);
    }

    // Handle percentage (e.g., "50%")
    if let Some(pct_str) = trimmed.strip_suffix('%') {
        let pct_val: f32 =
            pct_str
                .trim()
                .parse()
                .map_err(|_| TerminalImageError::InvalidWidthSpec {
                    spec: spec.to_string(),
                })?;

        // Validate percentage range (0-100)
        if !(0.0..=100.0).contains(&pct_val) {
            return Err(TerminalImageError::InvalidWidthSpec {
                spec: spec.to_string(),
            });
        }

        return Ok(ImageWidth::Percent(pct_val / 100.0));
    }

    // Handle character width with "ch" suffix (e.g., "40ch")
    if let Some(char_str) = trimmed.strip_suffix("ch") {
        let char_val: u32 =
            char_str
                .trim()
                .parse()
                .map_err(|_| TerminalImageError::InvalidWidthSpec {
                    spec: spec.to_string(),
                })?;

        if char_val == 0 {
            return Err(TerminalImageError::InvalidWidthSpec {
                spec: spec.to_string(),
            });
        }

        return Ok(ImageWidth::Characters(char_val));
    }

    // Handle bare number (characters)
    let char_val: u32 = trimmed
        .parse()
        .map_err(|_| TerminalImageError::InvalidWidthSpec {
            spec: spec.to_string(),
        })?;

    // Validate that width is positive
    if char_val == 0 {
        return Err(TerminalImageError::InvalidWidthSpec {
            spec: spec.to_string(),
        });
    }

    Ok(ImageWidth::Characters(char_val))
}

/// Parse a filepath string that may include a width specification.
///
/// Splits on the `|` delimiter and returns the filepath and optional width spec.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::parse_filepath_and_width;
///
/// let (path, width) = parse_filepath_and_width("image.png|50%").unwrap();
/// assert_eq!(path, "image.png");
/// assert_eq!(width, Some("50%".to_string()));
///
/// let (path, width) = parse_filepath_and_width("image.png").unwrap();
/// assert_eq!(path, "image.png");
/// assert!(width.is_none());
/// ```
pub fn parse_filepath_and_width(
    input: &str,
) -> Result<(String, Option<String>), TerminalImageError> {
    let parts: Vec<&str> = input.splitn(2, '|').collect();

    let filepath = parts[0].trim().to_string();

    if filepath.is_empty() {
        return Err(TerminalImageError::InvalidPath {
            path: input.to_string(),
            reason: "Empty filepath".to_string(),
        });
    }

    let width_spec = parts.get(1).map(|s| s.trim().to_string());

    Ok((filepath, width_spec))
}
