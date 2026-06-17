//! The raw Iconify icon body plus geometry needed to assemble an `<svg>`.

use serde::{Deserialize, Serialize};

/// An Iconify icon body (the inner markup, e.g. `<path .../>`) and the
/// geometry required to wrap it in a complete `<svg>` element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconBody {
    /// Inner SVG markup (paths, groups), without the surrounding `<svg>`.
    pub body: String,
    /// Intrinsic width of the icon's coordinate system.
    pub width: u32,
    /// Intrinsic height of the icon's coordinate system.
    pub height: u32,
    /// X-origin of the view box (default 0).
    #[serde(default)]
    pub left: i32,
    /// Y-origin of the view box (default 0).
    #[serde(default)]
    pub top: i32,
}

impl IconBody {
    /// Builds a body with an explicit coordinate system.
    #[must_use]
    pub fn new(body: impl Into<String>, width: u32, height: u32) -> Self {
        Self { body: body.into(), width, height, left: 0, top: 0 }
    }

    /// Builds a body with a non-zero view-box origin.
    #[must_use]
    pub fn with_origin(body: impl Into<String>, width: u32, height: u32, left: i32, top: i32) -> Self {
        Self { body: body.into(), width, height, left, top }
    }

    /// The `viewBox` string, `"{left} {top} {width} {height}"`.
    #[must_use]
    pub fn view_box(&self) -> String {
        format!("{} {} {} {}", self.left, self.top, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_box_uses_intrinsic_dimensions() {
        let body = IconBody::new("<path d=\"M0 0h24v24H0z\"/>", 24, 24);
        assert_eq!(body.view_box(), "0 0 24 24");
    }
}
