//! Canonical string conversions for horizontal-rule schema enums.
//!
//! These map the typed `style.hr` schema enums
//! ([`HrKind`]/[`HrWeight`]/[`HrAlignment`]) back to the kebab-/lower-case
//! strings the render-tree `ThematicBreakAttrs` carry, so page-level HR
//! defaults projected onto the tree agree with the attribute-block path.

use crate::style::schema::hr::{HrAlignment, HrKind, HrWeight};

/// Convert an [`HrKind`] to its canonical kebab-case string.
pub(crate) fn hr_kind_to_string(kind: HrKind) -> &'static str {
    match kind {
        HrKind::Dashes => "dashes",
        HrKind::Dots => "dots",
        HrKind::Waves => "waves",
        HrKind::LineStar => "line-star",
        HrKind::LineCircle => "line-circle",
        HrKind::InsetLine => "inset-line",
        HrKind::CurtainRod => "curtain-rod",
    }
}

/// Convert an [`HrWeight`] to its canonical lowercase string.
pub(crate) fn hr_weight_to_string(weight: HrWeight) -> &'static str {
    match weight {
        HrWeight::Thin => "thin",
        HrWeight::Medium => "medium",
        HrWeight::Thick => "thick",
    }
}

/// Convert an [`HrAlignment`] to its canonical lowercase string.
pub(crate) fn hr_alignment_to_string(alignment: HrAlignment) -> &'static str {
    match alignment {
        HrAlignment::Full => "full",
        HrAlignment::Left => "left",
        HrAlignment::Center => "center",
        HrAlignment::Right => "right",
    }
}
