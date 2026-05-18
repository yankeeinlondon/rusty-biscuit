//! Shared, target-neutral text-styling leaf primitives.
//!
//! [`TextEmphasis`] is the single source of truth for text weight and
//! decoration intent — bold, dim, italic, underline, strikethrough, and
//! blink. It is reused by `biscuit-terminal`'s `Prose` styling and is
//! intended to back the render-tree `Style` primitive, so terminal SGR
//! emission and browser CSS / semantic-HTML emission are written once here
//! and never drift between targets.
//!
//! Capability-aware degradation (e.g. downgrading a double underline on a
//! terminal that lacks it) is deliberately *not* done here — that decision
//! belongs to the target emitter that knows the terminal's capabilities.

/// The underline variant a styled region requests.
///
/// This records intent only. Capability-aware degradation is the target
/// emitter's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    /// A straight single underline.
    Straight,
    /// A double underline.
    Double,
    /// A curly (wavy) underline.
    Curly,
    /// A dotted underline.
    Dotted,
    /// A dashed underline.
    Dashed,
}

impl UnderlineStyle {
    /// The opening SGR escape for this underline variant.
    ///
    /// This is the *un-degraded* code; a terminal emitter may substitute a
    /// simpler variant when the terminal lacks support for it.
    pub fn sgr_open(self) -> &'static str {
        match self {
            Self::Straight => "\x1b[4m",
            Self::Double => "\x1b[4:2m",
            Self::Curly => "\x1b[4:3m",
            Self::Dotted => "\x1b[4:4m",
            Self::Dashed => "\x1b[4:5m",
        }
    }

    /// The CSS `text-decoration` declaration(s) for this underline variant.
    pub fn css_declaration(self) -> &'static str {
        match self {
            Self::Straight => "text-decoration: underline",
            Self::Double => "text-decoration: underline; text-decoration-style: double",
            Self::Curly => "text-decoration: underline; text-decoration-style: wavy",
            Self::Dotted => "text-decoration: underline; text-decoration-style: dotted",
            Self::Dashed => "text-decoration: underline; text-decoration-style: dashed",
        }
    }
}

/// An independent SGR attribute group within [`TextEmphasis`].
///
/// Each layer maps to one SGR attribute group, so a terminal emitter can
/// restore a parent span's value per layer instead of issuing a blanket
/// reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmphasisLayer {
    /// Bold / dim — the SGR font-weight group.
    Weight,
    /// Italic.
    Italic,
    /// Underline (any variant).
    Underline,
    /// Strikethrough.
    Strikethrough,
    /// Blink.
    Blink,
}

impl EmphasisLayer {
    /// The SGR code that clears this layer back to the terminal default.
    pub fn sgr_reset(self) -> &'static str {
        match self {
            Self::Weight => "\x1b[22m",
            Self::Italic => "\x1b[23m",
            Self::Underline => "\x1b[24m",
            Self::Strikethrough => "\x1b[29m",
            Self::Blink => "\x1b[25m",
        }
    }
}

/// Target-neutral text-emphasis leaf: weight and decoration intent.
///
/// Shared across render targets — `biscuit-terminal`'s `Prose` embeds it,
/// and the render-tree `Style` primitive is intended to as well. Terminal
/// SGR emission ([`sgr_ops`](Self::sgr_ops)) and browser semantic-HTML / CSS
/// emission ([`html_wrappers`](Self::html_wrappers)) are defined here so the
/// two targets never drift apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextEmphasis {
    /// Bold weight.
    pub bold: bool,
    /// Dim weight.
    pub dim: bool,
    /// Italic.
    pub italic: bool,
    /// Strikethrough.
    pub strikethrough: bool,
    /// Blink.
    pub blink: bool,
    /// Underline variant, if any.
    pub underline: Option<UnderlineStyle>,
}

impl TextEmphasis {
    /// `true` when no emphasis attribute is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The non-underline SGR open codes for this emphasis, in nesting order.
    ///
    /// Underline is intentionally excluded: its opening escape depends on a
    /// capability-aware degradation decision that only the terminal emitter
    /// can make. Use the [`underline`](Self::underline) field together with
    /// [`UnderlineStyle::sgr_open`] for the underline layer.
    pub fn sgr_ops(&self) -> Vec<(EmphasisLayer, &'static str)> {
        let mut ops = Vec::new();
        if self.bold {
            ops.push((EmphasisLayer::Weight, "\x1b[1m"));
        }
        if self.dim {
            ops.push((EmphasisLayer::Weight, "\x1b[2m"));
        }
        if self.italic {
            ops.push((EmphasisLayer::Italic, "\x1b[3m"));
        }
        if self.blink {
            ops.push((EmphasisLayer::Blink, "\x1b[5m"));
        }
        if self.strikethrough {
            ops.push((EmphasisLayer::Strikethrough, "\x1b[9m"));
        }
        ops
    }

    /// The HTML wrapper pairs (`open`, `close`) for this emphasis, in nesting
    /// order.
    ///
    /// Semantic styles use semantic HTML (`<strong>`, `<em>`, `<s>`);
    /// presentational styles use `<span style="…">`.
    pub fn html_wrappers(&self) -> Vec<(String, &'static str)> {
        let mut wrappers: Vec<(String, &'static str)> = Vec::new();
        if self.bold {
            wrappers.push(("<strong>".to_string(), "</strong>"));
        }
        if self.italic {
            wrappers.push(("<em>".to_string(), "</em>"));
        }
        if self.strikethrough {
            wrappers.push(("<s>".to_string(), "</s>"));
        }
        if let Some(underline) = self.underline {
            wrappers.push((
                format!("<span style=\"{}\">", underline.css_declaration()),
                "</span>",
            ));
        }
        if self.dim {
            wrappers.push(("<span style=\"opacity: 0.6\">".to_string(), "</span>"));
        }
        if self.blink {
            wrappers.push((
                "<span style=\"text-decoration: blink\">".to_string(),
                "</span>",
            ));
        }
        wrappers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(TextEmphasis::default().is_empty());
    }

    #[test]
    fn bold_is_not_empty() {
        let e = TextEmphasis {
            bold: true,
            ..Default::default()
        };
        assert!(!e.is_empty());
    }

    #[test]
    fn sgr_ops_excludes_underline() {
        let e = TextEmphasis {
            bold: true,
            underline: Some(UnderlineStyle::Double),
            ..Default::default()
        };
        let ops = e.sgr_ops();
        assert_eq!(ops, vec![(EmphasisLayer::Weight, "\x1b[1m")]);
    }

    #[test]
    fn html_wrappers_use_semantic_tags() {
        let e = TextEmphasis {
            bold: true,
            italic: true,
            ..Default::default()
        };
        let wrappers = e.html_wrappers();
        assert_eq!(wrappers[0].0, "<strong>");
        assert_eq!(wrappers[1].0, "<em>");
    }

    #[test]
    fn underline_css_for_curly_is_wavy() {
        assert_eq!(
            UnderlineStyle::Curly.css_declaration(),
            "text-decoration: underline; text-decoration-style: wavy"
        );
    }

    #[test]
    fn underline_sgr_open_matches_variant() {
        assert_eq!(UnderlineStyle::Straight.sgr_open(), "\x1b[4m");
        assert_eq!(UnderlineStyle::Double.sgr_open(), "\x1b[4:2m");
    }
}
