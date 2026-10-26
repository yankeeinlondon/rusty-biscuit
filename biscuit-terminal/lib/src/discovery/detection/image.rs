use serde::{Deserialize, Serialize};

use super::route::{GraphicsRoute, RouteConfidence, RouteReasonKind, graphics_route};

/// The pixel format (if any) a terminal's graphics route uses.
///
/// This is a projection of [`GraphicsRoute`]: read
/// [`Terminal::image_support`](crate::terminal::Terminal::image_support) or
/// [`GraphicsRoute::image_support`]. A text fallback such as half-blocks is
/// not image support.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageSupport {
    None,
    /// The [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
    Kitty,
    /// iTerm2's `OSC 1337;File=` inline image format.
    ITerm,
    /// DEC Sixel.
    Sixel,
}

/// Detailed result of image support detection, including the reason for the decision.
///
/// A projection of [`GraphicsRoute`]; prefer the route, which also carries
/// transport, placement, and a structured reason.
#[derive(Debug, Clone)]
pub struct ImageSupportResult {
    /// The detected image support level
    pub support: ImageSupport,
    /// Human-readable reason for the detection result
    pub reason: String,
    /// The detection method used
    pub method: DetectionMethod,
}

/// The method used to detect image support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Direct TTY capability check
    TtyCheck,
    /// Detection via viuer library probing. No longer produced; kept so
    /// existing matches still compile.
    Viuer,
    /// Heuristic based on environment variables
    EnvHeuristic,
    /// Known terminal application lookup
    KnownTerminal,
}

impl std::fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TtyCheck => write!(f, "tty_check"),
            Self::Viuer => write!(f, "viuer"),
            Self::EnvHeuristic => write!(f, "env_heuristic"),
            Self::KnownTerminal => write!(f, "known_terminal"),
        }
    }
}

impl From<&GraphicsRoute> for ImageSupportResult {
    fn from(route: &GraphicsRoute) -> Self {
        let method = if route.reason.kind == RouteReasonKind::NotATerminal {
            DetectionMethod::TtyCheck
        } else if route.confidence == RouteConfidence::Known {
            DetectionMethod::KnownTerminal
        } else {
            DetectionMethod::EnvHeuristic
        };
        Self {
            support: route.image_support(),
            reason: route.reason.to_string(),
            method,
        }
    }
}

/// The image support this process's graphics route gives.
///
/// Resolves the route from stdout and the environment on every call;
/// [`Terminal::new`](crate::terminal::Terminal::new) resolves once and keeps
/// it, so prefer the terminal's route when you have one.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{image_support, ImageSupport};
///
/// match image_support() {
///     ImageSupport::Kitty => println!("Kitty graphics protocol"),
///     ImageSupport::ITerm => println!("iTerm2 image format"),
///     ImageSupport::Sixel => println!("Sixel"),
///     ImageSupport::None => println!("No image support"),
/// }
/// ```
pub fn image_support() -> ImageSupport {
    graphics_route().image_support()
}

/// [`image_support`] with the route's reason and how it was decided.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::image_support_with_reason;
///
/// let result = image_support_with_reason();
/// println!("Support: {:?}", result.support);
/// println!("Reason: {}", result.reason);
/// println!("Method: {}", result.method);
/// ```
pub fn image_support_with_reason() -> ImageSupportResult {
    ImageSupportResult::from(&graphics_route())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_support_eq() {
        assert_eq!(ImageSupport::None, ImageSupport::None);
        assert_eq!(ImageSupport::Kitty, ImageSupport::Kitty);
        assert_eq!(ImageSupport::ITerm, ImageSupport::ITerm);
        assert_ne!(ImageSupport::None, ImageSupport::Kitty);
        assert_ne!(ImageSupport::Kitty, ImageSupport::ITerm);
    }

    #[test]
    fn test_image_support_debug() {
        let debug_none = format!("{:?}", ImageSupport::None);
        assert!(debug_none.contains("None"));

        let debug_kitty = format!("{:?}", ImageSupport::Kitty);
        assert!(debug_kitty.contains("Kitty"));

        let debug_iterm = format!("{:?}", ImageSupport::ITerm);
        assert!(debug_iterm.contains("ITerm"));
    }

    #[test]
    fn test_image_support_clone() {
        let support = ImageSupport::Kitty;
        let cloned = support.clone();
        assert_eq!(support, cloned);
    }

    #[test]
    fn test_image_support_result_fields() {
        let result = ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: "test reason".to_string(),
            method: DetectionMethod::TtyCheck,
        };

        assert_eq!(result.support, ImageSupport::Kitty);
        assert_eq!(result.reason, "test reason");
        assert_eq!(result.method, DetectionMethod::TtyCheck);
    }

    #[test]
    fn test_image_support_result_debug() {
        let result = ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "viuer detected iTerm2".to_string(),
            method: DetectionMethod::Viuer,
        };

        let debug = format!("{:?}", result);
        assert!(debug.contains("ITerm"));
        assert!(debug.contains("viuer"));
    }

    #[test]
    fn test_image_support_result_clone() {
        let result = ImageSupportResult {
            support: ImageSupport::None,
            reason: "not a tty".to_string(),
            method: DetectionMethod::TtyCheck,
        };

        let cloned = result.clone();
        assert_eq!(cloned.support, result.support);
        assert_eq!(cloned.reason, result.reason);
        assert_eq!(cloned.method, result.method);
    }

    #[test]
    fn test_image_support_returns_support_field() {
        let simple = image_support();
        let detailed = image_support_with_reason();
        assert_eq!(simple, detailed.support);
    }

    #[test]
    fn test_image_support_with_reason_has_non_empty_fields() {
        let result = image_support_with_reason();

        assert!(!result.reason.is_empty(), "Reason should not be empty");

        let valid_methods = [
            DetectionMethod::TtyCheck,
            DetectionMethod::Viuer,
            DetectionMethod::EnvHeuristic,
            DetectionMethod::KnownTerminal,
        ];
        assert!(
            valid_methods.contains(&result.method),
            "Method '{:?}' should be one of {:?}",
            result.method,
            valid_methods
        );
    }

    #[test]
    fn test_image_support_detection_does_not_panic() {
        let _ = image_support_with_reason();
    }
}
