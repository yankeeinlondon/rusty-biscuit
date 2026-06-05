//! The single inheritance resolver threaded by every render fold.

use crate::style::Style;

/// The effective text-appearance carried into a child during a fold.
///
/// Only [`Style::color`] and [`Style::emphasis`] inherit; the box-painting
/// fields ([`Style::background`], [`Style::border`]) and geometry never do.
/// Every render fold threads one of these instead of reconstructing the
/// color/emphasis push-down by hand, so the inheritance rule lives in exactly
/// one place.
#[derive(Debug, Clone, Default)]
pub struct InheritedStyle {
    /// Only `color` + `emphasis` are ever populated.
    inherited: Style,
}

impl InheritedStyle {
    /// The root context: nothing inherited yet.
    #[must_use]
    pub fn root() -> Self {
        Self {
            inherited: Style::default(),
        }
    }

    /// Enter a node carrying `node_style`.
    ///
    /// ## Returns
    ///
    /// A pair of the context to thread into the node's children and the full
    /// effective [`Style`] to apply to the node itself (the node's own
    /// box-painting layered over the inherited text appearance). The child
    /// context carries only the inheriting fields forward.
    #[must_use]
    pub fn enter(&self, node_style: Option<&Style>) -> (InheritedStyle, Style) {
        let effective = match node_style {
            Some(s) => s.inherited_from(&self.inherited),
            None => self.inherited.clone(),
        };
        let child = InheritedStyle {
            inherited: Style {
                color: effective.color.clone(),
                emphasis: effective.emphasis,
                ..Style::default()
            },
        };
        (child, effective)
    }

    /// The text appearance accumulated so far, for renderers that must merge a
    /// derived style (e.g. a heading's intrinsic emphasis) over the inherited
    /// context before folding a subtree.
    #[must_use]
    pub fn effective(&self) -> &Style {
        &self.inherited
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Color, Tailwind};
    use crate::layout::TargetValue;
    use crate::style::{Border, PerMode, Style, TextEmphasis};

    fn red() -> Style {
        Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                Tailwind::Red500,
            )))),
            ..Style::default()
        }
    }

    #[test]
    fn enter_threads_color_and_emphasis_only() {
        let root = InheritedStyle::root();
        // A node sets bold + red; the child context inherits both, box-painting is cleared.
        let mut styled = red();
        styled.emphasis = TextEmphasis {
            bold: true,
            ..Default::default()
        };
        styled.border = Some(Border::default());
        let (child_ctx, effective) = root.enter(Some(&styled));
        assert!(effective.color.is_some());
        assert!(effective.emphasis.bold);
        // border is applied to THIS node's effective style but does not inherit:
        let (_, grandchild_effective) = child_ctx.enter(None);
        assert!(grandchild_effective.color.is_some()); // color inherited
        assert!(grandchild_effective.emphasis.bold); // emphasis inherited
        assert!(grandchild_effective.border.is_none()); // box-painting did NOT inherit
    }
}
