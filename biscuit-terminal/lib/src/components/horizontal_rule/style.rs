/// Defines the visual style of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum RuleStyle {
    /// Simple dashed line: ---
    Dashes,
    /// Dotted line: ···
    Dots,
    /// Wavy line using Unicode characters.
    ///
    /// ## Notes
    ///
    /// Waves has no heavy Unicode variant — `RuleWeight::Thick` produces the
    /// same body in the terminal as `RuleWeight::Medium`. Weight affects only
    /// browser rendering (stroke width) for this style. ASCII fallback (`~`)
    /// also has no heavy variant.
    Waves,
    /// Line with star symbols: * * *
    LineStar,
    /// Line with circle symbols: ○ ○ ○
    LineCircle,
    /// Inset line with border effect
    InsetLine,
    /// Curtain rod style with decorative ends
    CurtainRod,
}

/// Defines the alignment of a horizontal rule within the available width.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RuleAlignment {
    /// Span the full available width
    Full,
    /// Centered with equal margins on both sides
    Centered,
    /// Aligned to the left edge
    Left,
    /// Aligned to the right edge
    Right,
}

/// Defines the visual weight (thickness) of a horizontal rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RuleWeight {
    /// Thin line (2px stroke in browser, single-line chars in terminal).
    Thin,
    /// Medium line (4px stroke in browser, single-line chars in terminal).
    Medium,
    /// Thick line (8px stroke in browser, heavy/double chars in terminal).
    Thick,
}
