//! Shared lowering helpers used by more than one tree renderer.
//!
//! The browser renderer and the MarkdownPlus path of the Markdown renderer
//! both emit the same semantic HTML for a few render-tree widgets. These
//! helpers keep that HTML shape defined once so the two targets cannot drift.

use crate::color::{Color, Tailwind};
use crate::style::PaintColor;
use crate::stylesheet::CssColor;
use crate::tree::{ColumnWidthKind, ColumnsHints, ProgressHints};

/// HTML-escapes text for placement inside an HTML attribute or text node.
pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Lowers a [`Color`] to a CSS color string (a `#rrggbb` hex value).
///
/// Returns `None` for colors with no fixed RGB value (`DefaultForeground`,
/// `DefaultBackground`, `Reset`).
pub(crate) fn color_to_css(color: Color) -> Option<String> {
    color
        .to_rgb()
        .map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Lowers a [`PaintColor`] to a validated [`CssColor`].
///
/// ## Returns
///
/// - A fixed RGB color with opaque alpha → [`CssColor::rgb`].
/// - A fixed RGB color with non-opaque alpha → [`CssColor::rgba`] using the
///   normalized CSS alpha.
/// - The Tailwind `transparent` / `current` / `inherit` keywords → the
///   matching CSS keyword, ignoring any stored alpha.
/// - The terminal default / reset colors (`DefaultForeground`,
///   `DefaultBackground`, `Reset`) → `None` (no CSS declaration).
pub(crate) fn paint_to_css_color(paint: PaintColor) -> Option<CssColor> {
    match paint.color {
        Color::Tailwind(Tailwind::Transparent) => Some(CssColor::Transparent),
        Color::Tailwind(Tailwind::Current) => Some(CssColor::CurrentColor),
        Color::Tailwind(Tailwind::Inherit) => Some(CssColor::Inherit),
        Color::DefaultForeground | Color::DefaultBackground | Color::Reset => None,
        _ => {
            let (r, g, b) = paint.color.to_rgb()?;
            if paint.opacity.is_opaque() {
                Some(CssColor::rgb(r, g, b))
            } else {
                // `as_css_alpha` is always within `0.0..=1.0`, so `rgba`
                // cannot reject it.
                CssColor::rgba(r, g, b, paint.opacity.as_css_alpha()).ok()
            }
        }
    }
}

/// Builds the semantic progress-widget HTML emitted by the browser and
/// MarkdownPlus renderers.
///
/// The outer element carries `role="progressbar"` plus ARIA value attributes
/// and an accessible label derived from the component label. Color slots
/// lower to inline `background-color`; non-default glyphs and brackets are
/// preserved as `data-*` attributes so the HTML surface keeps the information.
///
/// `outer_style` is appended to the outer element's `style` attribute (the
/// browser uses it to lower node `Layout`; MarkdownPlus passes an empty
/// string because Markdown ignores layout).
pub(crate) fn progress_html(
    hints: &ProgressHints,
    fallback_text: &str,
    outer_style: &str,
) -> String {
    let value = hints.value.clamp(0.0, 1.0);
    let percentage = (value * 100.0).round() as u32;

    // The label is everything before a trailing numeric percentage token
    // (`"Loading 60%"` → `"Loading"`). The token is stripped by *shape* — a
    // run of digits followed by `%` — rather than by the clamped percentage,
    // so the label stays stable even when the component value clamps to a
    // different percentage than the one written in the fallback text.
    let label = {
        let trimmed = fallback_text.trim_end();
        let stripped = trimmed
            .strip_suffix('%')
            .map(|head| head.trim_end_matches(|c: char| c.is_ascii_digit()))
            .unwrap_or(trimmed)
            .trim_end();
        (stripped != trimmed && !stripped.is_empty()).then_some(stripped)
    };

    let defaults = ProgressHints::default();
    let mut data_attrs = String::new();
    let mut push_data = |name: &str, value: char, default: char| {
        if value != default {
            data_attrs.push_str(&format!(r#" {name}="{}""#, escape_html(&value.to_string())));
        }
    };
    push_data("data-fill-char", hints.fill_char, defaults.fill_char);
    push_data("data-empty-char", hints.empty_char, defaults.empty_char);
    push_data(
        "data-left-bracket",
        hints.left_bracket,
        defaults.left_bracket,
    );
    push_data(
        "data-right-bracket",
        hints.right_bracket,
        defaults.right_bracket,
    );
    // `bracket_color` has no semantic CSS slot in the rendered widget (the
    // brackets sit inside text, not on a dedicated span), so it is preserved
    // as a `data-bracket-color` attribute alongside the bracket *glyphs*.
    // Consumers that want to repaint the brackets can pick it up there.
    if let Some(css) = hints.bracket_color.and_then(color_to_css) {
        data_attrs.push_str(&format!(r#" data-bracket-color="{}""#, escape_html(&css)));
    }

    let aria_label = label
        .map(|label| format!(r#" aria-label="{}""#, escape_html(label)))
        .unwrap_or_default();
    // `outer_style` is a CSS declaration string, not HTML text, so it is not
    // HTML-escaped — layout/`Style` lowering never produces `<`, `>`, or `"`.
    let outer_style_attr = if outer_style.is_empty() {
        String::new()
    } else {
        format!(r#" style="{outer_style}""#)
    };

    // The filled span carries width and (optional) color in a single `style`
    // attribute — emitting a second `style` would be invalid HTML.
    let filled_style = {
        let mut decls = vec![format!("width:{percentage}%")];
        if let Some(css) = hints.filled_color.and_then(color_to_css) {
            decls.push(format!("background-color:{css}"));
        }
        format!(r#" style="{}""#, decls.join(";"))
    };
    let track_style = {
        let mut decls = vec![format!("width:{}ch", hints.bar_width)];
        if let Some(css) = hints.empty_color.and_then(color_to_css) {
            decls.push(format!("background-color:{css}"));
        }
        format!(r#" style="{}""#, decls.join(";"))
    };

    let label_html = label
        .map(|label| {
            format!(
                r#"<span class="progress-label">{}</span>"#,
                escape_html(label)
            )
        })
        .unwrap_or_default();

    format!(
        concat!(
            r#"<span class="progress" role="progressbar" aria-valuemin="0" "#,
            r#"aria-valuemax="100" aria-valuenow="{now}"{label_attr}{data}{style}>"#,
            r#"{label_html}"#,
            r#"<span class="progress-track"{track}>"#,
            r#"<span class="progress-filled"{filled}></span>"#,
            r#"</span>"#,
            r#"<span class="progress-percentage">{pct}%</span>"#,
            r#"</span>"#,
        ),
        now = percentage,
        label_attr = aria_label,
        data = data_attrs,
        style = outer_style_attr,
        label_html = label_html,
        track = track_style,
        filled = filled_style,
        pct = percentage,
    )
}

/// The inline CSS declarations for the left column of a two-column layout.
///
/// A [`ColumnWidthKind::Fixed`] left column is pinned to `flex: 0 0 {n}ch`
/// with a matching `max-width`; a [`ColumnWidthKind::Percent`] left column is
/// pinned to `flex: 0 0 {p}%` with the fraction clamped to `0.0..=1.0`.
pub(crate) fn left_column_css(width: ColumnWidthKind) -> String {
    match width {
        ColumnWidthKind::Fixed(n) => format!("flex:0 0 {n}ch;max-width:{n}ch"),
        ColumnWidthKind::Percent(p) => {
            let pct = p.clamp(0.0, 1.0) * 100.0;
            format!("flex:0 0 {pct}%")
        }
    }
}

/// The inline CSS for the right column of a two-column layout: it consumes
/// the remaining width.
pub(crate) fn right_column_css() -> &'static str {
    "flex:1 1 0"
}

/// The inline CSS for the outer two-column flex container.
///
/// `extra` carries any additional declarations (the browser appends layout
/// CSS); it is joined after the `display`/`gap` declarations without
/// overwriting them.
pub(crate) fn columns_container_css(hints: &ColumnsHints, extra: &str) -> String {
    let mut css = format!("display:flex;gap:{}ch", hints.gap);
    if !extra.is_empty() {
        css.push(';');
        css.push_str(extra);
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{BasicColor, RgbColor};
    use crate::style::Opacity;

    fn red() -> Color {
        Color::Rgb(RgbColor::new(255, 0, 0, BasicColor::Red))
    }

    #[test]
    fn paint_to_css_color_opaque_rgb() {
        let css = paint_to_css_color(PaintColor::new(red())).unwrap();
        assert_eq!(css, CssColor::Rgb(255, 0, 0));
    }

    #[test]
    fn paint_to_css_color_alpha_rgba() {
        let paint = PaintColor::new(red()).with_opacity(Opacity::from_percent(50).unwrap());
        let css = paint_to_css_color(paint).unwrap();
        match css {
            CssColor::Rgba(255, 0, 0, alpha) => {
                assert!((alpha - 128.0 / 255.0).abs() < 1e-4);
            }
            other => panic!("expected rgba, got {other:?}"),
        }
    }

    #[test]
    fn paint_to_css_color_keywords_ignore_alpha() {
        let transparent = PaintColor::new(Color::Tailwind(Tailwind::Transparent))
            .with_opacity(Opacity::from_percent(50).unwrap());
        assert_eq!(paint_to_css_color(transparent), Some(CssColor::Transparent));
        assert_eq!(
            paint_to_css_color(PaintColor::new(Color::Tailwind(Tailwind::Current))),
            Some(CssColor::CurrentColor)
        );
        assert_eq!(
            paint_to_css_color(PaintColor::new(Color::Tailwind(Tailwind::Inherit))),
            Some(CssColor::Inherit)
        );
    }

    #[test]
    fn paint_to_css_color_terminal_defaults_have_no_declaration() {
        assert_eq!(paint_to_css_color(PaintColor::new(Color::DefaultForeground)), None);
        assert_eq!(paint_to_css_color(PaintColor::new(Color::DefaultBackground)), None);
        assert_eq!(paint_to_css_color(PaintColor::new(Color::Reset)), None);
    }
}
