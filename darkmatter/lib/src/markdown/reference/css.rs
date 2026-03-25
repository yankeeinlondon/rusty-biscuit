//! CSS import and font-face source extraction.
//!
//! Extracts `@import` and `@font-face { src: url(...) }` references
//! from CSS content found within `<style>` blocks.

use regex::Regex;
use lazy_static::lazy_static;

use crate::markdown::compose::ComposeSource;
use super::types::{
    ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSyntax,
    classify_target, make_reference_id,
};

lazy_static! {
    /// Matches `@import url("...")` or `@import url('...')` or `@import url(...)`.
    static ref CSS_IMPORT_URL: Regex = Regex::new(
        r#"@import\s+url\(\s*["']?([^"')]+)["']?\s*\)"#
    ).unwrap();

    /// Matches `@import "..."` or `@import '...'`.
    static ref CSS_IMPORT_STRING: Regex = Regex::new(
        r#"@import\s+["']([^"']+)["']"#
    ).unwrap();

    /// Matches `src: url("...")` or `src: url('...')` inside @font-face blocks.
    static ref FONT_FACE_SRC_URL: Regex = Regex::new(
        r#"src:\s*[^;]*url\(\s*["']?([^"')]+)["']?\s*\)"#
    ).unwrap();
}

/// Extract `@import` URLs from CSS content.
///
/// The `base_line` parameter offsets line numbers so provenance maps
/// back to the original markdown document's `<style>` block position.
pub(crate) fn extract_css_imports(
    css_content: &str,
    source: &ComposeSource,
    base_line: usize,
) -> Vec<ReferenceRecord> {
    let mut records = Vec::new();

    // @import url("...")
    for captures in CSS_IMPORT_URL.captures_iter(css_content) {
        if let Some(url_match) = captures.get(1) {
            let url = url_match.as_str();
            let line = base_line + css_content[..url_match.start()].matches('\n').count();
            let span_start = url_match.start();

            records.push(ReferenceRecord {
                id: make_reference_id(source, line, span_start),
                kind: ReferenceKind::CssImport,
                target: classify_target(url),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line,
                    span: span_start..url_match.end(),
                    syntax: ReferenceSyntax::CssAtImport,
                },
                attributes: serde_json::Map::new(),
            });
        }
    }

    // @import "..." (without url() wrapper)
    for captures in CSS_IMPORT_STRING.captures_iter(css_content) {
        if let Some(url_match) = captures.get(1) {
            let url = url_match.as_str();
            // Skip if already captured by the url() pattern
            if records.iter().any(|r| {
                r.target.raw() == Some(url) && r.origin.syntax == ReferenceSyntax::CssAtImport
            }) {
                continue;
            }

            let line = base_line + css_content[..url_match.start()].matches('\n').count();
            let span_start = url_match.start();

            records.push(ReferenceRecord {
                id: make_reference_id(source, line, span_start),
                kind: ReferenceKind::CssImport,
                target: classify_target(url),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line,
                    span: span_start..url_match.end(),
                    syntax: ReferenceSyntax::CssAtImport,
                },
                attributes: serde_json::Map::new(),
            });
        }
    }

    records
}

/// Extract `@font-face { src: url(...) }` references from CSS content.
pub(crate) fn extract_font_face_sources(
    css_content: &str,
    source: &ComposeSource,
    base_line: usize,
) -> Vec<ReferenceRecord> {
    let mut records = Vec::new();

    // Find @font-face blocks and extract src: url() within them
    let mut in_font_face = false;
    let mut brace_depth = 0;
    let mut font_face_start = 0;

    let bytes = css_content.as_bytes();
    for i in 0..bytes.len() {
        if css_content[i..].starts_with("@font-face") {
            in_font_face = true;
            font_face_start = i;
        }

        if in_font_face {
            if bytes[i] == b'{' {
                brace_depth += 1;
            } else if bytes[i] == b'}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    // Extract the font-face block content
                    let block = &css_content[font_face_start..=i];
                    for captures in FONT_FACE_SRC_URL.captures_iter(block) {
                        if let Some(url_match) = captures.get(1) {
                            let url = url_match.as_str();
                            let abs_start = font_face_start + url_match.start();
                            let line = base_line
                                + css_content[..abs_start].matches('\n').count();

                            records.push(ReferenceRecord {
                                id: make_reference_id(source, line, abs_start),
                                kind: ReferenceKind::FontImport,
                                target: classify_target(url),
                                origin: ReferenceOrigin {
                                    source: source.clone(),
                                    line,
                                    span: abs_start..font_face_start + url_match.end(),
                                    syntax: ReferenceSyntax::CssFontFaceSrc,
                                },
                                attributes: serde_json::Map::new(),
                            });
                        }
                    }
                    in_font_face = false;
                }
            }
        }
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::ReferenceTarget;

    #[test]
    fn import_url_with_quotes() {
        let css = r#"@import url("reset.css");"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::CssImport);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "reset.css"
        ));
    }

    #[test]
    fn import_string_syntax() {
        let css = r#"@import "styles/main.css";"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "styles/main.css"
        ));
    }

    #[test]
    fn import_remote_url() {
        let css = r#"@import "https://fonts.googleapis.com/css2?family=Inter";"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::RemoteUrl { .. }
        ));
    }

    #[test]
    fn font_face_src() {
        let css = r#"@font-face {
  font-family: "MyFont";
  src: url("font.woff2") format("woff2");
}"#;
        let records = extract_font_face_sources(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::FontImport);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "font.woff2"
        ));
    }

    #[test]
    fn no_font_face_outside_block() {
        let css = r#"body { src: url("not-a-font.css"); }"#;
        let records = extract_font_face_sources(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn malformed_css_no_crash() {
        let css = r#"@import url(;
@font-face { src: url(}"#;
        let imports = extract_css_imports(css, &ComposeSource::Unknown, 1);
        let fonts = extract_font_face_sources(css, &ComposeSource::Unknown, 1);
        // Should not crash, may or may not find matches
        let _ = imports;
        let _ = fonts;
    }

    #[test]
    fn base_line_offset() {
        let css = "@import \"reset.css\";";
        let records = extract_css_imports(css, &ComposeSource::Unknown, 10);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].origin.line, 10);
    }
}
