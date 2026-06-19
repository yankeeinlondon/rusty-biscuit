//! Terminal-facing layer over the shared [`renderable::stylesheet`] module.
//!
//! The type-safe CSS declaration model — [`CssStyle`], [`CssProp`],
//! [`CssColor`], [`CssSizing`], and friends — lives in the render-target
//! agnostic `renderable` crate and is re-exported here so darkmatter's
//! `darkmatter::render::stylesheet::*` API path stays stable.
//!
//! This module adds the two pieces of behavior that are inherently
//! terminal-specific and therefore cannot live in a leaf crate:
//!
//! - [`TerminalCss`] — an extension trait on [`CssStyle`] that renders a
//!   declaration block with ANSI styling (via `biscuit-terminal`'s `Prose`).
//! - [`StylesheetBlockError`] — a darkmatter-local newtype wrapper around the
//!   foreign [`StylesheetError`] that implements `biscuit-terminal`'s
//!   [`BlockError`] contract, rendering a parse/validation failure as a
//!   `StatusBlock`. The wrapper exists to sidestep the orphan rule: neither
//!   `BlockError` nor `StylesheetError` is owned by darkmatter.
//!
//! [`BlockError`]: biscuit_terminal::errors::BlockError

use std::error::Error as StdError;
use std::fmt::{self, Display};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

pub use renderable::stylesheet::{
    CssColor, CssColorProp, CssCustomProp, CssIntegerProp, CssProp, CssRaw, CssSizing,
    CssSizingMulti, CssSizingMultiProp, CssSizingProp, CssStyle, CssTypedProperty, CssUnit,
    CssValue, CssValueKind, IntoCssValue, StylesheetError,
};

/// Terminal-rendering extension for [`CssStyle`].
///
/// `renderable` is a leaf crate and intentionally does not depend on
/// `biscuit-terminal`, so the ANSI-styled rendering of a declaration block is
/// implemented here, in darkmatter, for the foreign [`CssStyle`] type.
/// darkmatter owns this trait, so the impl for a foreign type is permitted.
pub trait TerminalCss {
    /// Renders the style for terminal output, with ANSI styling only when the
    /// target is a TTY.
    ///
    /// When `terminal.is_tty` is `false`, the output matches
    /// [`CssStyle::to_css`] verbatim. When it is `true`, each declaration is
    /// colorized by category:
    ///
    /// | Element            | Style                          |
    /// |--------------------|--------------------------------|
    /// | Property name      | bold blue (`rgb 97,175,239`)   |
    /// | `:` and `;`        | gray (`rgb 160,160,160`)       |
    /// | Sizing values      | teal (`rgb 86,182,194`)        |
    /// | Color values       | green (`rgb 152,195,121`)      |
    /// | Integer values     | amber (`rgb 229,192,123`)      |
    /// | Raw values         | light gray (`rgb 220,220,220`) |
    ///
    /// ## Returns
    ///
    /// The styled (or plain) declaration block, joined by `\n`. Returns an
    /// empty string when the style has no declarations.
    fn to_terminal_string(&self, terminal: &Terminal) -> String;

    /// Renders the style with ANSI styling and writes it to stdout.
    ///
    /// Equivalent to calling [`TerminalCss::to_terminal_string`] and printing
    /// the result via `println!`. When the style is empty, nothing is printed.
    fn to_terminal(&self, terminal: &Terminal) {
        let output = self.to_terminal_string(terminal);
        if !output.is_empty() {
            println!("{output}");
        }
    }
}

impl TerminalCss for CssStyle {
    fn to_terminal_string(&self, terminal: &Terminal) -> String {
        if self.is_empty() {
            return String::new();
        }

        // The declaration model lives in `renderable`, which does not expose
        // per-declaration accessors. Re-parsing the plain CSS form gives the
        // declaration list back without reaching into private state.
        let plain = self.to_css();
        let colorize = terminal.is_tty;

        if !colorize {
            return plain;
        }

        plain
            .lines()
            .map(|line| style_declaration_line(terminal, line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Colorizes a single `property: value;` line for terminal output.
///
/// The line is split on the first `:` (property name) and the trailing `;`
/// (terminator). The value's category is inferred via [`CssProp::expected_kind`]
/// so the value text receives the right color.
fn style_declaration_line(terminal: &Terminal, line: &str) -> String {
    let line = line.trim_end();
    let Some((name, rest)) = line.split_once(':') else {
        return line.to_string();
    };

    let name = name.trim();
    let mut value_text = rest.trim();
    let has_semicolon = value_text.ends_with(';');
    if has_semicolon {
        value_text = value_text[..value_text.len() - 1].trim_end();
    }

    let kind = CssProp::from_css_name(name)
        .ok()
        .and_then(|prop| prop.expected_kind())
        .unwrap_or(CssValueKind::Raw);

    let styled_name = prose_style_text(terminal, "<bold><rgb 97,175,239>{text}</rgb></bold>", name);
    let styled_colon = Prose::new("<rgb 160,160,160>:</rgb>").render(terminal);
    let value_template = match kind {
        CssValueKind::Sizing | CssValueKind::SizingMulti => "<rgb 86,182,194>{text}</rgb>",
        CssValueKind::Color => "<rgb 152,195,121>{text}</rgb>",
        CssValueKind::Integer => "<rgb 229,192,123>{text}</rgb>",
        CssValueKind::Raw => "<rgb 220,220,220>{text}</rgb>",
    };
    let styled_value = prose_style_text(terminal, value_template, value_text);
    let styled_semicolon = Prose::new("<rgb 160,160,160>;</rgb>").render(terminal);

    format!("{styled_name}{styled_colon} {styled_value}{styled_semicolon}")
}

/// Renders `text` inside a [`Prose`] template without letting Prose interpret
/// any of `text`'s characters as inline markdown markup.
///
/// `template` must contain a literal `{text}` token. The token is replaced
/// with a unique placeholder, Prose renders the resulting source through the
/// terminal-aware styler, and the placeholder is swapped back to the original
/// `text` in the final string. This keeps CSS values like `_my-token_` or
/// `**important**` from being mangled by Prose's markdown subset.
fn prose_style_text(term: &Terminal, template: &str, text: &str) -> String {
    let placeholder = unique_placeholder(text);
    let source = template.replace("{text}", &placeholder);
    let rendered = Prose::new(source).render(term);
    rendered.replace(&placeholder, text)
}

/// Generates a placeholder string guaranteed not to appear in `text`.
///
/// Avoids characters [`Prose`] interprets as inline markdown (notably `_`,
/// which pairs as italics/bold). The placeholder must survive
/// [`Prose::render`] unchanged so the caller can swap it for the original
/// `text` afterwards. Numeric collisions are avoided by incrementing `idx`
/// until the candidate is absent from `text`.
fn unique_placeholder(text: &str) -> String {
    let mut idx = 0usize;
    loop {
        let candidate = format!("XDMSTYLESHEETTEXT{idx}XEND");
        if !text.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

/// darkmatter-local [`BlockError`] wrapper for the foreign [`StylesheetError`].
///
/// [`StylesheetError`] is defined in the `renderable` crate and
/// [`BlockError`](biscuit_terminal::errors::BlockError) is defined in
/// `biscuit-terminal`; neither is owned by darkmatter, so darkmatter cannot
/// `impl BlockError for StylesheetError` directly (orphan rule). This newtype
/// is the local type that carries the impl.
///
/// Construct it from any [`StylesheetError`] via [`From`], and render it as a
/// Block Style Error through the trait methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StylesheetBlockError(pub StylesheetError);

impl From<StylesheetError> for StylesheetBlockError {
    fn from(error: StylesheetError) -> Self {
        Self(error)
    }
}

impl Display for StylesheetBlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl StdError for StylesheetBlockError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

/// Renders [`StylesheetError`] into a styled
/// [`biscuit_terminal::components::status_block::StatusBlock`] for CLI display.
///
/// Each variant maps to a `StatusState::Error` block with three parts:
///
/// - **Header** — `"StylesheetError"` plus a short subtitle.
/// - **Body** — the offending input rendered with `<dim>` labels and `<cyan>` values.
/// - **Hint** — a one-line fix suggestion, often including a worked example.
///
/// For [`StylesheetError::PropertyValueTypeMismatch`] the hint is contextual:
/// it tries to resolve the property name back to a [`CssProp`] and looks up a
/// canonical example value via `examples_for_property`; otherwise it falls
/// back to a generic example from `example_for_kind`.
impl biscuit_terminal::errors::BlockError for StylesheetBlockError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match &self.0 {
            StylesheetError::InvalidDeclaration { declaration } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("StylesheetError", "invalid declaration"))
                    .body(format!("<dim>Declaration:</dim> <cyan>{declaration}</cyan>"))
                    .hint("Each declaration must be of the form <cyan>property: value;</cyan>.")
            }

            StylesheetError::InvalidPropertyName { name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid property name"))
                .body(format!("<dim>Property:</dim> <cyan>{name}</cyan>"))
                .hint(
                    "CSS property names must start with a letter (or `--` for custom properties) and contain only letters, digits, and hyphens.",
                ),

            StylesheetError::PropertyValueTypeMismatch {
                property,
                expected,
                actual,
                value,
            } => {
                let example = CssProp::from_css_name(property)
                    .ok()
                    .map(|prop| examples_for_property(&prop))
                    .unwrap_or(example_for_kind(*expected));
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "StylesheetError",
                        "property/value type mismatch",
                    ))
                    .body(format!(
                        "<dim>Property:</dim> <cyan>{property}</cyan>\n<dim>Expected:</dim> <b>{expected}</b>\n<dim>Actual:</dim> {actual}\n<dim>Value:</dim> <cyan>{value}</cyan>"
                    ))
                    .hint(format!(
                        "Use a <cyan>{expected}</cyan> value (e.g., <cyan>{example}</cyan>)."
                    ))
            }

            StylesheetError::InvalidSizing { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid sizing value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted sizing tokens: <cyan>0</cyan>, <cyan>42px</cyan>, <cyan>1.5rem</cyan>, <cyan>50%</cyan>, <cyan>auto</cyan>, <cyan>min-content</cyan>, or a <cyan>calc(...)</cyan> expression.",
                ),

            StylesheetError::InvalidSizingMulti { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "StylesheetError",
                    "invalid multi-sizing value",
                ))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Use 1 to 4 sizing tokens separated by spaces, e.g. <cyan>8px 16px</cyan> or <cyan>4px 8px 12px 16px</cyan>.",
                ),

            StylesheetError::InvalidColor { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid color value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted color tokens: <cyan>#rgb</cyan>, <cyan>#rrggbb</cyan>, <cyan>rgb(r,g,b)</cyan>, <cyan>rgba(r,g,b,a)</cyan>, a named color, or one of <cyan>transparent</cyan> / <cyan>currentColor</cyan>.",
                ),

            StylesheetError::InvalidInteger { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid integer value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint("Use a whole number, e.g. <cyan>0</cyan>, <cyan>1</cyan>, or <cyan>-3</cyan>."),

            StylesheetError::InvalidRawValue { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid raw value"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint("Raw CSS values must not be empty and must not contain <cyan>;</cyan>."),

            StylesheetError::InvalidCustomProperty { name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("StylesheetError", "invalid custom property"))
                .body(format!("<dim>Property:</dim> <cyan>{name}</cyan>"))
                .hint(
                    "Custom properties must start with <cyan>--</cyan> followed by a non-empty identifier (letters, digits, hyphens, underscores).",
                ),
        }
    }
}

/// Returns a canonical example string for the given value kind.
///
/// Used by [`StylesheetBlockError`]'s status block rendering to suggest a
/// valid value when a category mismatch occurs. The returned slice is short
/// and representative — it is **not** an exhaustive description of the kind's
/// accepted syntax.
fn example_for_kind(kind: CssValueKind) -> &'static str {
    match kind {
        CssValueKind::Sizing => "12px",
        CssValueKind::SizingMulti => "8px 16px",
        CssValueKind::Color => "#336699",
        CssValueKind::Integer => "40",
        CssValueKind::Raw => "var(--token)",
    }
}

/// Returns a property-specific example value used in error messages.
///
/// Falls back to `example_for_kind` for properties without a hand-tuned
/// example, and to `"var(--token)"` when the property has no known
/// [`CssValueKind`] (i.e. [`CssProp::Other`]).
fn examples_for_property(prop: &CssProp) -> &'static str {
    match prop {
        CssProp::FontSize => "16px",
        CssProp::Width | CssProp::MinWidth | CssProp::MaxWidth => "320px",
        CssProp::Height | CssProp::MinHeight | CssProp::MaxHeight => "240px",
        CssProp::Margin | CssProp::Padding => "8px 16px",
        CssProp::BorderRadius => "12px 12px 0 0",
        CssProp::Color
        | CssProp::BackgroundColor
        | CssProp::BorderColor
        | CssProp::OutlineColor
        | CssProp::TextDecorationColor => "#336699",
        CssProp::ZIndex => "10",
        CssProp::Order | CssProp::FlexGrow | CssProp::FlexShrink => "1",
        _ => prop
            .expected_kind()
            .map(example_for_kind)
            .unwrap_or("var(--token)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    #[test]
    fn typed_add_builds_valid_style() {
        let style = CssStyle::new()
            .add(CssSizingProp::TopMargin, CssSizing::px(12.0))
            .add(
                CssSizingMultiProp::Margin,
                CssSizingMulti::from((CssSizing::px(8.0), CssSizing::px(16.0))),
            )
            .add(
                CssColorProp::Color,
                CssColor::hex("#336699").expect("hex color should be valid"),
            )
            .add(CssIntegerProp::ZIndex, 40);

        assert_eq!(
            style.to_css(),
            "margin-top: 12px;\nmargin: 8px 16px;\ncolor: #336699;\nz-index: 40;"
        );
    }

    #[test]
    fn terminal_rendering_contains_ansi_when_tty() {
        use biscuit_terminal::discovery::detection::ColorDepth;

        let style = CssStyle::new().add(CssColorProp::Color, CssColor::rgb(1, 2, 3));
        let terminal = Terminal::builder()
            .width(80)
            .is_tty(true)
            .color_depth(ColorDepth::TrueColor)
            .build();

        let rendered = style.to_terminal_string(&terminal);
        assert!(rendered.contains("\u{1b}["));
        assert!(rendered.contains("color"));
        assert!(rendered.contains("rgb(1, 2, 3)"));
    }

    #[test]
    fn terminal_rendering_is_plain_when_not_tty() {
        let style = CssStyle::new().add(CssSizingProp::Width, CssSizing::px(80.0));
        let terminal = Terminal::builder().width(80).is_tty(false).build();

        // Non-TTY targets receive plain CSS, matching `to_css`.
        assert_eq!(style.to_terminal_string(&terminal), style.to_css());
    }

    #[test]
    fn stylesheet_block_error_renders_status_block() {
        let err = StylesheetBlockError(StylesheetError::InvalidInteger {
            value: "1.5".into(),
        });
        let out = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
        assert!(out.contains("StylesheetError"));
        assert!(out.contains("invalid integer value"));
        assert!(out.contains("1.5"));
    }
}
