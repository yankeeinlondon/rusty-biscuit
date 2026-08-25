use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when querying terminal colors via OSC sequences.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OscQueryError {
    /// Standard output is not connected to a TTY.
    #[error("not connected to a TTY")]
    NotTty,

    /// Running in a CI environment where terminal queries are not supported.
    #[error("running in CI environment")]
    CiEnvironment,

    /// The terminal does not support this OSC query.
    #[error("terminal does not support OSC {0} queries")]
    Unsupported(u8),

    /// The query timed out waiting for a response.
    #[error("OSC query timed out after {0:?}")]
    Timeout(Duration),

    /// Failed to parse the terminal's response.
    #[error("failed to parse OSC response: {0}")]
    ParseError(String),

    /// An I/O error occurred during the query.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Running inside a terminal multiplexer that may not pass through OSC queries.
    #[error("running inside multiplexer ({0}), OSC queries may not work")]
    Multiplexer(String),
}

/// Default timeout for an actual terminal OSC round trip.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// RGB color with 8-bit components.
///
/// Represents a color in the sRGB color space with values from 0-255
/// for each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbValue {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl RgbValue {
    /// Create a new RGB color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Calculate relative luminance using the sRGB luminance formula.
    ///
    /// Returns a value between 0.0 (black) and 1.0 (white).
    ///
    /// This uses the ITU-R BT.709 coefficients for sRGB:
    /// - Red: 0.2126
    /// - Green: 0.7152
    /// - Blue: 0.0722
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::discovery::osc_queries::RgbValue;
    ///
    /// let black = RgbValue::new(0, 0, 0);
    /// assert!((black.luminance() - 0.0).abs() < 0.01);
    ///
    /// let white = RgbValue::new(255, 255, 255);
    /// assert!((white.luminance() - 1.0).abs() < 0.01);
    /// ```
    pub fn luminance(&self) -> f64 {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Check if this color is considered "light" (luminance > 0.5).
    pub fn is_light(&self) -> bool {
        self.luminance() > 0.5
    }

    /// Check if this color is considered "dark" (luminance <= 0.5).
    pub fn is_dark(&self) -> bool {
        self.luminance() <= 0.5
    }
}

impl std::fmt::Display for RgbValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_luminance_black() {
        let black = RgbValue::new(0, 0, 0);
        assert!((black.luminance() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_white() {
        let white = RgbValue::new(255, 255, 255);
        assert!((white.luminance() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_gray() {
        let gray = RgbValue::new(128, 128, 128);
        assert!(gray.luminance() > 0.2 && gray.luminance() < 0.8);
    }

    #[test]
    fn test_rgb_luminance_red() {
        let red = RgbValue::new(255, 0, 0);
        assert!((red.luminance() - 0.2126).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_green() {
        let green = RgbValue::new(0, 255, 0);
        assert!((green.luminance() - 0.7152).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_blue() {
        let blue = RgbValue::new(0, 0, 255);
        assert!((blue.luminance() - 0.0722).abs() < 0.01);
    }

    #[test]
    fn test_rgb_is_light_dark() {
        let black = RgbValue::new(0, 0, 0);
        assert!(black.is_dark());
        assert!(!black.is_light());

        let white = RgbValue::new(255, 255, 255);
        assert!(white.is_light());
        assert!(!white.is_dark());
    }

    #[test]
    fn test_rgb_display() {
        let color = RgbValue::new(100, 150, 200);
        assert_eq!(color.to_string(), "rgb(100, 150, 200)");
    }

    #[test]
    fn test_default_timeout_value() {
        assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(1));
    }

    #[test]
    fn test_osc_query_error_display() {
        let errors = [
            OscQueryError::NotTty,
            OscQueryError::CiEnvironment,
            OscQueryError::Unsupported(11),
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::ParseError("test".into()),
            OscQueryError::IoError("test".into()),
            OscQueryError::Multiplexer("tmux".into()),
        ];

        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty(), "Error {:?} should have a message", err);
        }
    }

    #[test]
    fn test_osc_query_error_variants() {
        let err = OscQueryError::NotTty;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotTty"));
    }

    #[test]
    fn test_osc_query_error_clone() {
        let err = OscQueryError::NotTty;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_osc_query_error_eq() {
        assert_eq!(OscQueryError::NotTty, OscQueryError::NotTty);
        assert_ne!(OscQueryError::NotTty, OscQueryError::CiEnvironment);
        assert_eq!(
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::Timeout(Duration::from_millis(100))
        );
        assert_ne!(
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::Timeout(Duration::from_millis(200))
        );
    }

    #[test]
    fn test_rgb_value_serialization() {
        let color = RgbValue::new(100, 150, 200);
        let json = serde_json::to_string(&color).unwrap();
        let deserialized: RgbValue = serde_json::from_str(&json).unwrap();
        assert_eq!(color, deserialized);
    }

    #[test]
    fn test_rgb_value_const_new() {
        const COLOR: RgbValue = RgbValue::new(255, 128, 0);
        assert_eq!(COLOR.r, 255);
        assert_eq!(COLOR.g, 128);
        assert_eq!(COLOR.b, 0);
    }
}
