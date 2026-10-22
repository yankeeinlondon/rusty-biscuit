//! Already-encoded PNG input for [`TerminalImage`](super::TerminalImage).

use std::borrow::Cow;
use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::error::{LimitError, LimitErrorKind};
use image::{DynamicImage, ImageError, ImageFormat, ImageReader, Limits};

use super::TerminalImageError;
use crate::components::image_options::DEFAULT_MAX_FILE_SIZE;

/// PNG bytes that are sent to Kitty and iTerm2 exactly as given, with the
/// pixel size read from their header.
///
/// Use it for an image the program produced itself, such as a diagram
/// rasterized at its display size, instead of writing a file for
/// [`TerminalImage::new`](super::TerminalImage::new) to decode and encode again.
/// It is not a way to load caller-supplied files: those keep going through the
/// path-based constructors and their validation.
///
/// Construction enforces the same limits a file meets before anything is
/// decoded or any rows are reserved: at most the default
/// [`max_file_size`](crate::components::image_options::TerminalImageOptions::max_file_size)
/// of bytes, a readable PNG header, and pixel dimensions within the `image`
/// crate's default allocation limit.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::{EncodedPng, TerminalImageError};
///
/// let not_png = EncodedPng::new(b"GIF89a".to_vec());
/// assert!(matches!(not_png, Err(TerminalImageError::ImageLoadError(_))));
/// ```
#[derive(Clone)]
pub struct EncodedPng {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

impl EncodedPng {
    /// Validates `bytes` as a PNG and records its pixel size.
    ///
    /// ## Errors
    ///
    /// - [`TerminalImageError::FileTooLarge`] above the default 10 MiB limit.
    /// - [`TerminalImageError::ImageLoadError`] when the header is not a PNG
    ///   header, or its dimensions exceed the decode allocation limit.
    pub fn new(bytes: Vec<u8>) -> Result<Self, TerminalImageError> {
        let size = bytes.len() as u64;
        if size > DEFAULT_MAX_FILE_SIZE {
            return Err(TerminalImageError::FileTooLarge {
                size,
                max_size: DEFAULT_MAX_FILE_SIZE,
            });
        }

        let (width, height) =
            ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png).into_dimensions()?;
        let limits = Limits::default();
        limits.check_dimensions(width, height)?;
        let decoded_bytes = u64::from(width) * u64::from(height) * 4;
        if limits.max_alloc.is_some_and(|max| decoded_bytes > max) {
            return Err(ImageError::Limits(LimitError::from_kind(
                LimitErrorKind::InsufficientMemory,
            ))
            .into());
        }

        Ok(Self {
            bytes,
            width,
            height,
        })
    }

    #[allow(missing_docs)]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[allow(missing_docs)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[allow(missing_docs)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Decodes the pixels, for output that cannot carry PNG bytes.
    pub(super) fn decode(&self) -> Result<DynamicImage, TerminalImageError> {
        let mut reader = ImageReader::with_format(Cursor::new(&self.bytes), ImageFormat::Png);
        reader.limits(Limits::default());
        Ok(reader.decode()?)
    }
}

impl std::fmt::Debug for EncodedPng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodedPng")
            .field("len", &self.bytes.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

/// An image's pixels as a protocol path needs them: the caller's PNG bytes
/// untouched, or a decoded file that must be encoded first.
pub(super) enum ImagePixels<'a> {
    Encoded(&'a EncodedPng),
    Decoded(DynamicImage),
}

impl<'a> ImagePixels<'a> {
    /// [`png`](Self::png), keeping a borrow of the caller's bytes alive
    /// past the pixels themselves.
    pub(super) fn into_png(self) -> Result<Cow<'a, [u8]>, TerminalImageError> {
        match self {
            Self::Encoded(png) => Ok(Cow::Borrowed(png.bytes())),
            Self::Decoded(img) => encode_png(&img).map(Cow::Owned),
        }
    }

    /// The pixels decoded, for encoders that need them (Sixel).
    pub(super) fn into_image(self) -> Result<DynamicImage, TerminalImageError> {
        match self {
            Self::Encoded(png) => png.decode(),
            Self::Decoded(img) => Ok(img),
        }
    }

    /// Pixel width and height.
    pub(super) fn size(&self) -> (u32, u32) {
        match self {
            Self::Encoded(png) => (png.width, png.height),
            Self::Decoded(img) => (img.width(), img.height()),
        }
    }

    /// The PNG to send: encoded input as-is, decoded files freshly encoded.
    pub(super) fn png(&self) -> Result<Cow<'_, [u8]>, TerminalImageError> {
        match self {
            Self::Encoded(png) => Ok(Cow::Borrowed(png.bytes())),
            Self::Decoded(img) => encode_png(img).map(Cow::Owned),
        }
    }

    /// The pixels resized to exactly `width` x `height` and encoded.
    pub(super) fn resized_png(&self, width: u32, height: u32) -> Result<Vec<u8>, TerminalImageError> {
        let resized = match self {
            Self::Encoded(png) => png.decode()?,
            Self::Decoded(img) => img.clone(),
        }
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        encode_png(&resized)
    }
}

/// Encodes with zlib's default level and adaptive filtering; the `image`
/// crate's default (`write_to`) favors speed and can exceed the raw pixel size.
pub(super) fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, TerminalImageError> {
    let mut buffer = Vec::new();
    img.write_with_encoder(PngEncoder::new_with_quality(
        &mut buffer,
        CompressionType::Default,
        FilterType::Adaptive,
    ))
    .map_err(|e| TerminalImageError::EncodingError {
        message: format!("PNG encoding failed: {e}"),
    })?;
    Ok(buffer)
}

#[cfg(test)]
mod tests;
