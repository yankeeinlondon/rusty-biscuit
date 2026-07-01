//! Bespoke style types and application for sub-spec #7.
//!
//! Covers `style.page.stylesheet`, `style.page.meta`, `style.page.code.theme`,
//! `style.hyperlinks.*`, `style.hyperlinks.local-style`, and
//! `style.images.local-style`.

use std::path::Path;

use biscuit_terminal::utils::{UnicodeWidthChar, UnicodeWidthStr};
use renderable::layout::{Alignment, Length, Width};
#[cfg(test)]
use renderable::layout::TargetValue;

use crate::layout::{DarkmatterPage, PageRenderError};
use crate::markdown::highlighting::ThemePair;
use crate::style::apply::StyleApplyError;
use crate::style::schema::{CommonStyle, StyleFrontmatter};
#[cfg(test)]
use crate::style::schema::WidthOrMode;

/// A resolved page stylesheet, either inlined from disk or referenced remotely.
#[derive(Debug, Clone, PartialEq)]
pub enum PageStylesheet {
    /// CSS read from a local file, with the source path preserved for
    /// diagnostics.
    Inline {
        /// Path the CSS was read from.
        source: std::path::PathBuf,
        /// The file contents.
        css: String,
    },
    /// A remote stylesheet URL emitted as `<link rel="stylesheet">`.
    Remote {
        /// Href attribute value.
        href: String,
    },
}

/// A collection of HTML `<meta>` tags derived from `style.page.meta`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageMeta {
    /// Ordered meta tag entries.
    pub tags: Vec<MetaTag>,
}

/// A single HTML `<meta>` tag.
#[derive(Debug, Clone, PartialEq)]
pub enum MetaTag {
    /// `<meta charset="...">`
    Charset(String),
    /// `<meta name="..." content="...">`
    Name {
        /// The `name` attribute.
        name: String,
        /// The `content` attribute.
        content: String,
    },
    /// `<meta property="..." content="...">` (Open Graph, etc.)
    Property {
        /// The `property` attribute.
        property: String,
        /// The `content` attribute.
        content: String,
    },
}

/// Field-level CLI overrides for bespoke style knobs.
///
/// Each `true` value means the corresponding frontmatter field must be ignored
/// because a CLI flag already claimed it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BespokeStyleOverrides {
    /// `--code-theme` was supplied; skip `style.page.code.theme`.
    pub code_theme: bool,
}

/// Apply parsed bespoke style onto a [`DarkmatterPage`] builder.
///
/// This is the integration point for sub-spec #7 knobs that do not fit the
/// standard page/component layout and color maps.
///
/// ## Phase 2
///
/// Wires `style.page.stylesheet` and `style.page.meta` into the page builder.
/// Local stylesheets are read from disk and inlined; remote stylesheets are
/// stored as href references. Meta tags are parsed from the opaque JSON object
/// into typed [`MetaTag`] entries.
///
/// ## Phase 3
///
/// Wires `style.page.code.theme` into the page builder, with CLI override
/// precedence. Returns [`StyleApplyError::InvalidCodeTheme`] for unknown theme
/// names.
pub fn apply_bespoke_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: BespokeStyleOverrides,
    source_path: Option<&Path>,
) -> Result<DarkmatterPage, StyleApplyError> {
    let mut page = page;

    // Page-level knobs
    if let Some(page_style) = style.page.as_ref() {
        // Stylesheet
        if let Some(sheet) = page_style.stylesheet.as_deref() {
            let stylesheet = resolve_stylesheet(sheet, source_path)?;
            page = page.with_stylesheet(stylesheet);
        }

        // Meta tags
        if let Some(meta_value) = page_style.meta.as_ref() {
            let meta = parse_page_meta(meta_value)?;
            if !meta.tags.is_empty() {
                page = page.with_page_meta(meta);
            }
        }

        // Code theme — CLI takes precedence over frontmatter.
        if !overrides.code_theme
            && let Some(theme_name) = page_style.code.as_ref().and_then(|c| c.theme.as_deref())
        {
            let theme = ThemePair::try_from(theme_name).map_err(|_| StyleApplyError::InvalidCodeTheme {
                value: theme_name.to_string(),
            })?;
            page = page.with_page_code_theme(theme);
        }
    }

    // Hyperlinks
    if let Some(hyperlinks) = style.hyperlinks.as_ref() {
        // Conflict detection for width/max-width in the common bucket.
        if hyperlinks.common.width.is_some() && hyperlinks.common.max_width.is_some() {
            return Err(StyleApplyError::ComponentWidthConflict { bucket: "hyperlinks" });
        }

        // Conflict detection for width/max-width in the local-style bucket.
        if let Some(local) = hyperlinks.local_style.as_ref()
            && local.width.is_some()
            && local.max_width.is_some()
        {
            return Err(StyleApplyError::ComponentWidthConflict {
                bucket: "hyperlinks.local-style",
            });
        }

        page = page.with_hyperlink_style(hyperlinks.common.clone());

        let merged_local = hyperlinks.local_style.as_ref().map(|local| {
            merge_common_style(&hyperlinks.common, local)
        });
        if let Some(local) = merged_local {
            page = page.with_local_hyperlink_style(local);
        }
    }

    // Images local-style
    if let Some(images) = style.images.as_ref() {
        // Conflict detection for width/max-width in the local-style bucket.
        if let Some(local) = images.local_style.as_ref()
            && local.width.is_some()
            && local.max_width.is_some()
        {
            return Err(StyleApplyError::ComponentWidthConflict {
                bucket: "images.local-style",
            });
        }

        let merged_local = images.local_style.as_ref().map(|local| {
            merge_common_style(&images.common, local)
        });
        if let Some(local) = merged_local {
            page = page.with_local_image_style(local);
        }
    }

    Ok(page)
}

/// Resolve a stylesheet string into a [`PageStylesheet`].
///
/// ## Errors
///
/// - [`StyleApplyError::EmptyStylesheet`] when the value is empty.
/// - [`StyleApplyError::UnsupportedStylesheetScheme`] when the value uses
///   `file://`.
/// - [`StyleApplyError::StylesheetNotFound`] when a local path does not exist.
/// - [`StyleApplyError::StylesheetRead`] when a local path cannot be read.
fn resolve_stylesheet(
    value: &str,
    source_path: Option<&Path>,
) -> Result<PageStylesheet, StyleApplyError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(StyleApplyError::EmptyStylesheet);
    }

    // Reject file:// scheme.
    if trimmed.starts_with("file://") {
        return Err(StyleApplyError::UnsupportedStylesheetScheme {
            scheme: "file://".into(),
        });
    }

    // Remote: http(s) URLs.
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(PageStylesheet::Remote {
            href: trimmed.to_string(),
        });
    }

    // Local path: resolve relative to source document or CWD.
    let path = std::path::PathBuf::from(trimmed);
    let resolved = if path.is_absolute() {
        path
    } else if let Some(source) = source_path
        && let Some(parent) = source.parent()
    {
        parent.join(path)
    } else {
        std::env::current_dir()
            .map_err(|e| StyleApplyError::StylesheetRead {
                path: trimmed.to_string(),
                reason: e.to_string(),
            })?
            .join(path)
    };

    if !resolved.exists() {
        return Err(StyleApplyError::StylesheetNotFound {
            path: resolved.display().to_string(),
        });
    }

    let css = std::fs::read_to_string(&resolved).map_err(|e| StyleApplyError::StylesheetRead {
        path: resolved.display().to_string(),
        reason: e.to_string(),
    })?;

    Ok(PageStylesheet::Inline {
        source: resolved,
        css,
    })
}

/// Parse `style.page.meta` into [`PageMeta`].
///
/// ## Errors
///
/// - [`StyleApplyError::InvalidMetaShape`] when the value is not an object or
///   a nested value has an unexpected shape.
fn parse_page_meta(value: &serde_json::Value) -> Result<PageMeta, StyleApplyError> {
    let serde_json::Value::Object(map) = value else {
        return Err(StyleApplyError::InvalidMetaShape {
            path: "page.meta".into(),
            reason: format!("expected object, got {}", json_type_name(value)),
        });
    };

    let mut tags = Vec::new();

    for (key, val) in map {
        let tag = meta_entry(key, val)?;
        tags.push(tag);
    }

    Ok(PageMeta { tags })
}

/// Convert a single meta key-value pair into a [`MetaTag`].
fn meta_entry(key: &str, value: &serde_json::Value) -> Result<MetaTag, StyleApplyError> {
    if key == "charset" {
        let content = meta_scalar(value, "style.page.meta.charset")?;
        return Ok(MetaTag::Charset(content));
    }

    let content = if key == "keywords" {
        match value {
            serde_json::Value::Array(arr) => {
                let parts: Vec<String> = arr
                    .iter()
                    .map(|v| meta_scalar(v, "style.page.meta.keywords"))
                    .collect::<Result<Vec<_>, _>>()?;
                parts.join(", ")
            }
            other => {
                return Err(StyleApplyError::InvalidMetaShape {
                    path: "page.meta.keywords".into(),
                    reason: format!("expected array, got {}", json_type_name(other)),
                });
            }
        }
    } else {
        meta_scalar(value, &format!("style.page.meta.{key}"))?
    };

    if key.starts_with("og:") {
        Ok(MetaTag::Property {
            property: key.to_string(),
            content,
        })
    } else {
        Ok(MetaTag::Name {
            name: key.to_string(),
            content,
        })
    }
}

/// Lower a JSON scalar (string, number, bool) to a string for meta content.
fn meta_scalar(value: &serde_json::Value, path: &str) -> Result<String, StyleApplyError> {
    match value {
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        other => Err(StyleApplyError::InvalidMetaShape {
            path: path.strip_prefix("style.").unwrap_or(path).to_string(),
            reason: format!("expected string/number/bool, got {}", json_type_name(other)),
        }),
    }
}

/// Returns `true` when the URL is a local reference.
///
/// Local hyperlinks are `LinkType::File`: relative paths, absolute paths,
/// anchors, and `file://` URLs. HTTP(S), including localhost, are remote.
pub fn is_local_hyperlink(url: &str) -> bool {
    let trimmed = url.trim();
    !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
}

/// Returns `true` when the image source is a local reference.
///
/// A local image has a primary `src` or `srcset` candidate that is not HTTP(S)
/// and is not a `data:` URL.
pub fn is_local_image(src: &str) -> bool {
    let trimmed = src.trim();
    !(trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:"))
}

/// Merge `local` over `base` field-by-field.
///
/// For every field that `local` sets, the local value wins. Unset fields fall
/// back to `base`. This is the rule for `style.hyperlinks.local-style` and
/// `style.images.local-style`.
pub fn merge_common_style(base: &CommonStyle, local: &CommonStyle) -> CommonStyle {
    CommonStyle {
        width: local.width.clone().or_else(|| base.width.clone()),
        max_width: local.max_width.clone().or_else(|| base.max_width.clone()),
        alignment: local.alignment.or(base.alignment),
        color: local.color.clone().or_else(|| base.color.clone()),
        bg_color: local.bg_color.clone().or_else(|| base.bg_color.clone()),
        margin: local.margin.clone().or_else(|| base.margin.clone()),
        padding: local.padding.clone().or_else(|| base.padding.clone()),
        border: local.border.clone().or_else(|| base.border.clone()),
        emphasis: local.emphasis.clone().or_else(|| base.emphasis.clone()),
        word_wrap: local.word_wrap.or(base.word_wrap),
    }
}

/// Validate that no `Length::Css(_)` value appears in the bespoke style buckets
/// for fields that would affect terminal layout.
///
/// HTML accepts arbitrary CSS lengths for inline link/image style, but terminal
/// layout works in cells. Per sub-spec #7 design decision #3, CSS lengths on
/// `style.hyperlinks.{width,max-width}`,
/// `style.hyperlinks.local-style.{width,max-width}`, and
/// `style.images.local-style.{width,max-width}` must be rejected with
/// [`PageRenderError::InvalidInlineCssLength`] when the terminal renderer would
/// otherwise have to honor them.
///
/// ## Errors
///
/// Returns [`PageRenderError::InvalidInlineCssLength`] when a `Length::Css(_)`
/// is set on a width/max-width field of any of the three buckets.
pub fn validate_terminal_inline_lengths(page: &DarkmatterPage) -> Result<(), PageRenderError> {
    check_inline_css(page.hyperlink_style(), "hyperlinks")?;
    check_inline_css(page.local_hyperlink_style(), "hyperlinks.local-style")?;
    check_inline_css(page.local_image_style(), "images.local-style")?;
    Ok(())
}

fn check_inline_css(
    style: Option<&CommonStyle>,
    bucket: &'static str,
) -> Result<(), PageRenderError> {
    let Some(style) = style else {
        return Ok(());
    };
    if let Some(width) = style.width.as_ref()
        && let Width::Fixed(renderable::layout::TargetValue::Universal(Length::Css(_))) = width.as_width()
    {
        return Err(PageRenderError::InvalidInlineCssLength {
            bucket,
            field: "width",
        });
    }
    if matches!(style.max_width, Some(Length::Css(_))) {
        return Err(PageRenderError::InvalidInlineCssLength {
            bucket,
            field: "max-width",
        });
    }
    Ok(())
}

/// Resolve a [`Length`] to a terminal cell width against `base_width`.
///
/// `Length::Css(_)` returns `None` because the caller is expected to have
/// already rejected CSS lengths via [`validate_terminal_inline_lengths`].
fn length_to_cells(length: &Length, base_width: u16) -> Option<u16> {
    match length {
        Length::Zero => Some(0),
        Length::Ch(n) => Some(u16::try_from(*n).unwrap_or(u16::MAX)),
        Length::Percent(p) => {
            if !p.is_finite() || !(0.0..=100.0).contains(p) {
                return None;
            }
            Some(((f32::from(base_width) * (p / 100.0)).round() as u16).min(base_width))
        }
        Length::Css(_) => None,
    }
}

/// Lay out an inline text fragment (a hyperlink display text or an image
/// fallback alt) inside a width-bound box.
///
/// Resolves [`CommonStyle`]'s `width`/`max-width`/`alignment` against
/// `base_width` (typically the terminal effective width):
///
/// - When `width` is set, truncates or pads to that exact cell count.
/// - When only `max-width` is set, truncates to that cell count if the text
///   is wider; otherwise returns the text unchanged.
/// - When neither is set, returns the text unchanged.
///
/// Padding is added per `alignment` (default `Left`). Truncation appends `…`
/// when the target width is at least 2 cells.
///
/// Returns the input text unchanged when `common` is `None` or when no
/// width-affecting field is set.
pub fn apply_inline_text_layout(
    text: &str,
    common: Option<&CommonStyle>,
    base_width: u16,
) -> String {
    let Some(common) = common else {
        return text.to_string();
    };

    let visible = UnicodeWidthStr::width(text) as u16;
    let target_width = if let Some(width) = common.width.as_ref() {
        match width.as_width() {
            Width::Fixed(renderable::layout::TargetValue::Universal(len)) => {
                length_to_cells(len, base_width)
            }
            _ => None,
        }
    } else if let Some(max) = common.max_width.as_ref() {
        match length_to_cells(max, base_width) {
            Some(max_cells) if visible > max_cells => Some(max_cells),
            _ => None,
        }
    } else {
        None
    };

    let Some(target) = target_width else {
        return text.to_string();
    };

    let alignment = common.alignment.unwrap_or(Alignment::Left);

    let bounded = if visible > target {
        truncate_to_cells(text, target)
    } else {
        text.to_string()
    };

    let bounded_width = UnicodeWidthStr::width(bounded.as_str()) as u16;
    let pad = target.saturating_sub(bounded_width) as usize;

    match alignment {
        Alignment::Left => format!("{}{}", bounded, " ".repeat(pad)),
        Alignment::Right => format!("{}{}", " ".repeat(pad), bounded),
        Alignment::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), bounded, " ".repeat(right))
        }
    }
}

/// Truncate `text` so its display width fits in `max_cells`, appending `…`
/// when there is at least 2 cells of room.
fn truncate_to_cells(text: &str, max_cells: u16) -> String {
    let max = max_cells as usize;
    if max == 0 {
        return String::new();
    }
    let reserve_ellipsis = max >= 2;
    let limit = if reserve_ellipsis { max - 1 } else { max };

    let mut acc = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > limit {
            break;
        }
        acc.push(ch);
        used += cw;
    }
    if reserve_ellipsis && UnicodeWidthStr::width(text) > max {
        acc.push('…');
    }
    acc
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_stylesheet_variants_are_constructible() {
        let inline = PageStylesheet::Inline {
            source: std::path::PathBuf::from("./style.css"),
            css: "body { color: red; }".into(),
        };
        assert!(matches!(inline, PageStylesheet::Inline { .. }));

        let remote = PageStylesheet::Remote {
            href: "https://example.com/style.css".into(),
        };
        assert!(matches!(remote, PageStylesheet::Remote { .. }));
    }

    #[test]
    fn meta_tag_variants_are_constructible() {
        let charset = MetaTag::Charset("utf-8".into());
        assert!(matches!(charset, MetaTag::Charset(_)));

        let name = MetaTag::Name {
            name: "description".into(),
            content: "A page".into(),
        };
        assert!(matches!(name, MetaTag::Name { .. }));

        let property = MetaTag::Property {
            property: "og:title".into(),
            content: "Title".into(),
        };
        assert!(matches!(property, MetaTag::Property { .. }));
    }

    #[test]
    fn page_meta_default_is_empty() {
        let meta = PageMeta::default();
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn bespoke_style_overrides_default_is_false() {
        let overrides = BespokeStyleOverrides::default();
        assert!(!overrides.code_theme);
    }

    #[test]
    fn apply_bespoke_style_returns_page_unchanged_when_no_page_block() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::StyleFrontmatter;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter::default();
        let result = apply_bespoke_style(
            page.clone(),
            &style,
            BespokeStyleOverrides::default(),
            None,
        )
        .unwrap();

        assert_eq!(result.terminal_width(), page.terminal_width());
        assert!(result.stylesheet().is_none());
        assert!(result.page_meta().is_none());
    }

    // ---------- Phase 2: stylesheet tests ----------

    #[test]
    fn resolve_stylesheet_empty_returns_error() {
        let err = resolve_stylesheet("", None).unwrap_err();
        assert_eq!(err, StyleApplyError::EmptyStylesheet);
    }

    #[test]
    fn resolve_stylesheet_file_scheme_returns_error() {
        let err = resolve_stylesheet("file:///etc/passwd", None).unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::UnsupportedStylesheetScheme {
                scheme: "file://".into(),
            }
        );
    }

    #[test]
    fn resolve_stylesheet_remote_https() {
        let sheet = resolve_stylesheet("https://example.com/style.css", None).unwrap();
        assert_eq!(
            sheet,
            PageStylesheet::Remote {
                href: "https://example.com/style.css".into(),
            }
        );
    }

    #[test]
    fn resolve_stylesheet_remote_http() {
        let sheet = resolve_stylesheet("http://example.com/style.css", None).unwrap();
        assert_eq!(
            sheet,
            PageStylesheet::Remote {
                href: "http://example.com/style.css".into(),
            }
        );
    }

    #[test]
    fn resolve_stylesheet_local_missing_returns_error() {
        let err = resolve_stylesheet("/nonexistent/path/to/style.css", None).unwrap_err();
        assert!(matches!(err, StyleApplyError::StylesheetNotFound { .. }));
    }

    #[test]
    fn resolve_stylesheet_local_relative_reads_file() {
        // Write a temp CSS file and resolve it relative to a fake source path.
        let dir = tempfile::tempdir().unwrap();
        let css_path = dir.path().join("style.css");
        std::fs::write(&css_path, "body { color: red; }").unwrap();

        let source = dir.path().join("doc.md");
        let sheet = resolve_stylesheet("style.css", Some(&source)).unwrap();
        assert_eq!(
            sheet,
            PageStylesheet::Inline {
                source: css_path,
                css: "body { color: red; }".into(),
            }
        );
    }

    #[test]
    fn resolve_stylesheet_local_absolute_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let css_path = dir.path().join("style.css");
        std::fs::write(&css_path, "h1 { font-size: 2em; }").unwrap();

        let sheet = resolve_stylesheet(css_path.to_str().unwrap(), None).unwrap();
        assert_eq!(
            sheet,
            PageStylesheet::Inline {
                source: css_path,
                css: "h1 { font-size: 2em; }".into(),
            }
        );
    }

    // ---------- Phase 2: meta tests ----------

    #[test]
    fn parse_meta_description() {
        let meta = parse_page_meta(&serde_json::json!({
            "description": "A test page"
        }))
        .unwrap();
        assert_eq!(meta.tags.len(), 1);
        assert_eq!(
            meta.tags[0],
            MetaTag::Name {
                name: "description".into(),
                content: "A test page".into(),
            }
        );
    }

    #[test]
    fn parse_meta_keywords_array() {
        let meta = parse_page_meta(&serde_json::json!({
            "keywords": ["rust", "markdown"]
        }))
        .unwrap();
        assert_eq!(meta.tags.len(), 1);
        assert_eq!(
            meta.tags[0],
            MetaTag::Name {
                name: "keywords".into(),
                content: "rust, markdown".into(),
            }
        );
    }

    #[test]
    fn parse_meta_og_property() {
        let meta = parse_page_meta(&serde_json::json!({
            "og:title": "My Title"
        }))
        .unwrap();
        assert_eq!(meta.tags.len(), 1);
        assert_eq!(
            meta.tags[0],
            MetaTag::Property {
                property: "og:title".into(),
                content: "My Title".into(),
            }
        );
    }

    #[test]
    fn parse_meta_charset() {
        let meta = parse_page_meta(&serde_json::json!({
            "charset": "utf-8"
        }))
        .unwrap();
        assert_eq!(meta.tags.len(), 1);
        assert_eq!(meta.tags[0], MetaTag::Charset("utf-8".into()));
    }

    #[test]
    fn parse_meta_number_and_bool() {
        let meta = parse_page_meta(&serde_json::json!({
            "viewport": 123,
            "refresh": true
        }))
        .unwrap();
        assert_eq!(meta.tags.len(), 2);
        assert!(meta.tags.contains(&MetaTag::Name {
            name: "viewport".into(),
            content: "123".into(),
        }));
        assert!(meta.tags.contains(&MetaTag::Name {
            name: "refresh".into(),
            content: "true".into(),
        }));
    }

    #[test]
    fn parse_meta_non_object_returns_error() {
        let err = parse_page_meta(&serde_json::json!("description")).unwrap_err();
        let StyleApplyError::InvalidMetaShape { path, .. } = err else {
            panic!("expected InvalidMetaShape");
        };
        assert_eq!(path, "page.meta");
    }

    #[test]
    fn parse_meta_nested_object_returns_error() {
        let err = parse_page_meta(&serde_json::json!({
            "author": {"name": "Ken"}
        }))
        .unwrap_err();
        let StyleApplyError::InvalidMetaShape { path, .. } = err else {
            panic!("expected InvalidMetaShape");
        };
        assert_eq!(path, "page.meta.author");
    }

    #[test]
    fn parse_meta_keywords_non_array_returns_error() {
        let err = parse_page_meta(&serde_json::json!({
            "keywords": "rust"
        }))
        .unwrap_err();
        let StyleApplyError::InvalidMetaShape { path, .. } = err else {
            panic!("expected InvalidMetaShape");
        };
        assert_eq!(path, "page.meta.keywords");
    }

    #[test]
    fn apply_bespoke_style_with_stylesheet_and_meta() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{PageStyle, StyleFrontmatter};

        let dir = tempfile::tempdir().unwrap();
        let css_path = dir.path().join("style.css");
        std::fs::write(&css_path, "body { margin: 0; }").unwrap();
        let source = dir.path().join("doc.md");

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            page: Some(PageStyle {
                stylesheet: Some("style.css".into()),
                meta: Some(serde_json::json!({
                    "description": "Test"
                })),
                ..PageStyle::default()
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), Some(&source))
            .unwrap();

        assert!(page.stylesheet().is_some());
        assert!(page.page_meta().is_some());
    }

    // ---------- Phase 3: code theme tests ----------

    #[test]
    fn apply_bespoke_style_with_code_theme() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{PageStyle, StyleFrontmatter};
        use crate::markdown::highlighting::ThemePair;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            page: Some(PageStyle {
                code: Some(crate::style::schema::CodeStyle {
                    theme: Some("dracula".into()),
                }),
                ..PageStyle::default()
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();

        assert_eq!(page.page_code_theme(), Some(&ThemePair::Dracula));
    }

    #[test]
    fn apply_bespoke_style_code_theme_cli_override_skips_frontmatter() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{PageStyle, StyleFrontmatter};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            page: Some(PageStyle {
                code: Some(crate::style::schema::CodeStyle {
                    theme: Some("dracula".into()),
                }),
                ..PageStyle::default()
            }),
            ..StyleFrontmatter::default()
        };

        let overrides = BespokeStyleOverrides { code_theme: true };

        let page = apply_bespoke_style(page, &style, overrides, None).unwrap();
        assert!(page.page_code_theme().is_none(), "CLI override should skip frontmatter code theme");
    }

    #[test]
    fn apply_bespoke_style_invalid_code_theme_returns_error() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{PageStyle, StyleFrontmatter};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            page: Some(PageStyle {
                code: Some(crate::style::schema::CodeStyle {
                    theme: Some("not-a-real-theme".into()),
                }),
                ..PageStyle::default()
            }),
            ..StyleFrontmatter::default()
        };

        let err = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::InvalidCodeTheme {
                value: "not-a-real-theme".into(),
            }
        );
    }

    // ---------- Phase 4: hyperlink tests ----------

    #[test]
    fn is_local_hyperlink_detects_local_vs_remote() {
        assert!(is_local_hyperlink("./file.md"));
        assert!(is_local_hyperlink("/absolute/path"));
        assert!(is_local_hyperlink("#anchor"));
        assert!(is_local_hyperlink("file:///etc/passwd"));
        assert!(!is_local_hyperlink("http://example.com"));
        assert!(!is_local_hyperlink("https://example.com"));
        assert!(!is_local_hyperlink("https://localhost:8080"));
    }

    #[test]
    fn merge_common_style_field_by_field() {
        use renderable::layout::Alignment;
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let base = CommonStyle {
            alignment: Some(Alignment::Left),
            color: Some(StyleColor {
                color: Color::Tailwind(Tailwind::Red500),
                opacity: None,
            }),
            ..CommonStyle::default()
        };
        let local = CommonStyle {
            alignment: Some(Alignment::Right),
            bg_color: Some(StyleColor {
                color: Color::Tailwind(Tailwind::Blue500),
                opacity: None,
            }),
            ..CommonStyle::default()
        };

        let merged = merge_common_style(&base, &local);
        assert_eq!(merged.alignment, Some(Alignment::Right), "local alignment wins");
        assert_eq!(
            merged.color,
            Some(StyleColor {
                color: Color::Tailwind(Tailwind::Red500),
                opacity: None,
            }),
            "base color falls through when local is unset"
        );
        assert!(
            merged.bg_color.is_some(),
            "local bg-color is present"
        );
    }

    #[test]
    fn apply_bespoke_style_hyperlink_width_conflict() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::Length;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    max_width: Some(Length::Percent(50.0)),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let err = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict { bucket: "hyperlinks" }
        );
    }

    #[test]
    fn apply_bespoke_style_hyperlink_local_style_width_conflict() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::Length;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    max_width: Some(Length::Percent(50.0)),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let err = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict {
                bucket: "hyperlinks.local-style",
            }
        );
    }

    #[test]
    fn apply_bespoke_style_hyperlink_style_stored() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::{Alignment, Length};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    alignment: Some(Alignment::Center),
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    ..CommonStyle::default()
                },
                local_style: Some(Box::new(CommonStyle {
                    alignment: Some(Alignment::Right),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();

        assert_eq!(
            page.hyperlink_style().map(|s| s.alignment),
            Some(Some(Alignment::Center))
        );
        assert_eq!(
            page.local_hyperlink_style().map(|s| s.alignment),
            Some(Some(Alignment::Right))
        );
    }

    #[test]
    fn hyperlink_color_injected_into_html() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[link](https://example.com)".into();
        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("color:rgb(251, 44, 54)"),
            "HTML should contain red hyperlink color. html={}",
            html
        );
    }

    #[test]
    fn local_hyperlink_color_overrides_global_in_html() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                },
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Blue500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[local](./file.md) and [remote](https://example.com)".into();
        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("color:rgb(43, 127, 255)"),
            "local link should be blue. html={}",
            html
        );
        assert!(
            html.contains("color:rgb(251, 44, 54)"),
            "remote link should be red. html={}",
            html
        );
    }

    #[test]
    fn hyperlink_per_link_inline_css_wins_over_frontmatter() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        // Per-link inline `color: green` declares `color`, so frontmatter's
        // `color: red-500` must NOT replace it. Frontmatter still contributes
        // properties the per-link style does not set (here: `background-color`).
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    bg_color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Blue500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown =
            "[x](https://example.com \"style='color: green'\")".into();
        let html = page.render_to_browser(&md).unwrap();

        assert!(
            html.contains("color:green"),
            "per-link `color: green` must survive. html={}",
            html
        );
        assert!(
            !html.contains("color:rgb(251, 44, 54)"),
            "frontmatter red must NOT overwrite the per-link color. html={}",
            html
        );
        assert!(
            html.contains("background-color:rgb(43, 127, 255)"),
            "frontmatter `background-color` must fill the unset property. html={}",
            html
        );
    }

    #[test]
    fn hyperlink_color_injected_into_terminal() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[link](https://example.com)".into();
        let output = page.render(&md).unwrap();
        // Terminal output should contain SGR color codes for red (38;2;239;68;68)
        assert!(
            output.contains("38;2;251;44;54"),
            "Terminal output should contain red SGR for hyperlinks. output={}",
            output
        );
    }

    #[test]
    fn local_hyperlink_color_overrides_global_in_terminal() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                },
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Blue500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[local](./file.md)".into();
        let output = page.render(&md).unwrap();
        // Local link should be blue (38;2;59;130;246)
        assert!(
            output.contains("38;2;43;127;255"),
            "Terminal output should contain blue SGR for local hyperlinks. output={}",
            output
        );
    }

    // ---------- Phase 5: local image style tests ----------

    #[test]
    fn is_local_image_detects_local_vs_remote() {
        assert!(is_local_image("./photo.png"));
        assert!(is_local_image("/absolute/path.png"));
        assert!(is_local_image("images/cat.jpg"));
        assert!(!is_local_image("http://example.com/img.png"));
        assert!(!is_local_image("https://example.com/img.png"));
        assert!(!is_local_image("data:image/png;base64,abc123"));
    }

    #[test]
    fn apply_bespoke_style_image_local_style_width_conflict() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::layout::Length;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    max_width: Some(Length::Percent(50.0)),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let err = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap_err();
        assert_eq!(
            err,
            StyleApplyError::ComponentWidthConflict {
                bucket: "images.local-style",
            }
        );
    }

    #[test]
    fn apply_bespoke_style_image_local_style_stored() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::layout::{Alignment, Length};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle {
                    alignment: Some(Alignment::Center),
                    ..CommonStyle::default()
                },
                local_style: Some(Box::new(CommonStyle {
                    alignment: Some(Alignment::Right),
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();

        assert_eq!(
            page.local_image_style().map(|s| s.alignment),
            Some(Some(Alignment::Right))
        );
        assert_eq!(
            page.local_image_style().map(|s| s.width.clone()),
            Some(Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))))
        );
    }

    #[test]
    fn local_image_color_injected_into_html() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![alt](./local.png)".into();
        let html = page.render_to_browser(&md).unwrap();
        assert!(
            html.contains("color:rgb(251, 44, 54)"),
            "HTML should contain red local image color. html={}",
            html
        );
    }

    #[test]
    fn remote_image_does_not_get_local_style_in_html() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![alt](https://example.com/remote.png)".into();
        let html = page.render_to_browser(&md).unwrap();
        assert!(
            !html.contains("color:rgb(251, 44, 54)"),
            "HTML should NOT contain red local image color for remote image. html={}",
            html
        );
    }

    #[test]
    fn local_image_color_injected_into_terminal() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![alt](./local.png)".into();
        let output = page.render(&md).unwrap();
        // Terminal fallback should contain SGR color codes for red
        assert!(
            output.contains("38;2;251;44;54"),
            "Terminal output should contain red SGR for local image fallback. output={}",
            output
        );
    }

    // ---------- Phase 6 (review-7): width/max-width/alignment + CSS rejection ----------

    #[test]
    fn hyperlink_width_pads_terminal_display_text() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::{Alignment, Length};

        // A minimal frame (`margin-left`) deliberately selects the optimistic
        // profile so OSC8 is available. Under the review-5 capability model the
        // hyperlink *width* is a local text-layout attr that applies regardless
        // of profile, while OSC8 availability follows the deliberate frame (or an
        // OSC8-capable terminal) — never the hyperlink-style match itself.
        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_margin_left(1);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(10))))),
                    alignment: Some(Alignment::Left),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[hi](https://example.com)".into();
        let output = page.render(&md).unwrap();

        // Width 10 + Left alignment over text "hi" (2 cells) must pad with
        // 8 trailing spaces, and the framed page's optimistic profile emits the
        // padded label inside an OSC8 hyperlink.
        assert!(
            output.contains("hi        ") && output.contains("]8;;https://example.com"),
            "padded display text inside an OSC8 hyperlink not present. output={:?}",
            output
        );
    }

    #[test]
    fn hyperlink_max_width_truncates_terminal_display_text() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::Length;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    max_width: Some(Length::Ch(5)),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[abcdefghij](https://example.com)".into();
        let output = page.render(&md).unwrap();

        // max-width 5 over "abcdefghij" (10 cells) must truncate to "abcd…"
        // (4 cells of label + 1 cell ellipsis).
        assert!(
            output.contains("abcd…"),
            "max-width truncation missing. output={:?}",
            output
        );
        assert!(
            !output.contains("abcdefghij"),
            "full label leaked past truncation. output={:?}",
            output
        );
    }

    #[test]
    fn hyperlink_alignment_center_pads_both_sides() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::{Alignment, Length};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(8))))),
                    alignment: Some(Alignment::Center),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[ab](https://example.com)".into();
        let output = page.render(&md).unwrap();

        // Width 8, "ab" (2 cells), Center => 3 left + ab + 3 right.
        // Fallback URL form (`text [url]`) follows the padded label.
        assert!(
            output.contains("   ab   "),
            "center-padded display text missing. output={:?}",
            output
        );
    }

    #[test]
    fn hyperlink_css_length_rejected_for_terminal() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::{DarkmatterPage, PageRenderError};
        use crate::markdown::Markdown;
        use crate::style::schema::{HyperlinkStyle, StyleFrontmatter};
        use renderable::layout::Length;
        use renderable::stylesheet::CssSizing;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            hyperlinks: Some(HyperlinkStyle {
                common: CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Css(CssSizing::px(320.0)))))),
                    ..CommonStyle::default()
                },
                local_style: None,
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "[x](https://example.com)".into();
        let err = page.render(&md).unwrap_err();
        assert_eq!(
            err,
            PageRenderError::InvalidInlineCssLength {
                bucket: "hyperlinks",
                field: "width",
            }
        );
    }

    #[test]
    fn local_image_css_length_rejected_for_terminal() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::{DarkmatterPage, PageRenderError};
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::layout::Length;
        use renderable::stylesheet::CssSizing;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    max_width: Some(Length::Css(CssSizing::px(120.0))),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![alt](./local.png)".into();
        let err = page.render(&md).unwrap_err();
        assert_eq!(
            err,
            PageRenderError::InvalidInlineCssLength {
                bucket: "images.local-style",
                field: "max-width",
            }
        );
    }

    #[test]
    fn local_image_alignment_pads_terminal_fallback() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::layout::{Alignment, Length};

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Ch(40))))),
                    alignment: Some(Alignment::Right),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![a](./local.png)".into();
        let output = page.render(&md).unwrap();

        // The tree path shapes the *complete* placeholder (width 40, Right):
        // `▉ IMAGE[a]` is right-aligned within the 40-cell field, so the padding
        // precedes the placeholder and the alt inside the brackets is untouched.
        let fallback_line = output
            .lines()
            .find(|l| l.contains("▉ IMAGE["))
            .unwrap_or_else(|| panic!("fallback line missing in output={output:?}"));
        let inner = fallback_line
            .split_once("▉ IMAGE[")
            .and_then(|(_, rest)| rest.split_once(']'))
            .map(|(inner, _)| inner)
            .unwrap_or("");
        assert_eq!(
            inner, "a",
            "alt inside the brackets must be untouched: {fallback_line:?}"
        );
        let leading = fallback_line.chars().take_while(|c| *c == ' ').count();
        let field_width = fallback_line.trim_end().chars().count();
        assert!(
            leading >= 28 && field_width == 40,
            "placeholder should be right-aligned within the 40-cell field, got {leading} leading, width {field_width}: {fallback_line:?}"
        );
    }

    #[test]
    fn apply_inline_text_layout_passthrough_when_no_style() {
        assert_eq!(apply_inline_text_layout("hello", None, 80), "hello");
        assert_eq!(
            apply_inline_text_layout("hello", Some(&CommonStyle::default()), 80),
            "hello"
        );
    }

    #[test]
    fn apply_inline_text_layout_percent_width() {
        use renderable::layout::Length;
        let style = CommonStyle {
            width: Some(WidthOrMode::Width(Width::Fixed(TargetValue::universal(Length::Percent(50.0))))),
            ..CommonStyle::default()
        };
        let out = apply_inline_text_layout("ab", Some(&style), 20);
        // 50% of 20 = 10 cells, Left default => "ab        "
        assert_eq!(out.chars().count(), 10);
        assert!(out.starts_with("ab"));
        assert!(out.ends_with("        "));
    }

    #[test]
    fn remote_image_terminal_fallback_unchanged() {
        use biscuit_terminal::terminal::Terminal;
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::style::schema::{ImageStyle, StyleFrontmatter};
        use renderable::color::{Color, Tailwind};
        use crate::style::color::StyleColor;

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term);
        let style = StyleFrontmatter {
            images: Some(ImageStyle {
                common: CommonStyle::default(),
                local_style: Some(Box::new(CommonStyle {
                    color: Some(StyleColor {
                        color: Color::Tailwind(Tailwind::Red500),
                        opacity: None,
                    }),
                    ..CommonStyle::default()
                })),
            }),
            ..StyleFrontmatter::default()
        };

        let page = apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
            .unwrap();
        let md: Markdown = "![alt](https://example.com/remote.png)".into();
        let output = page.render(&md).unwrap();
        // Remote image fallback should NOT contain red SGR
        assert!(
            !output.contains("38;2;251;44;54"),
            "Terminal output should NOT contain red SGR for remote image. output={}",
            output
        );
    }
}
