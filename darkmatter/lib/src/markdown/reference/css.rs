//! CSS import and font-face source extraction using `cssparser`.
//!
//! Uses the `cssparser` tokenizer for spec-compliant extraction of
//! `@import` and `@font-face { src: url(...) }` references from CSS
//! content found within `<style>` blocks.

use cssparser::{Parser, ParserInput, Token};

use super::types::{
    ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSyntax, classify_target,
    make_reference_id,
};
use crate::markdown::compose::ComposeSource;

/// Extract `@import` URLs from CSS content.
///
/// The `base_line` parameter offsets line numbers so provenance maps
/// back to the original markdown document's `<style>` block position.
///
/// Handles all valid CSS `@import` forms:
/// - `@import url("path");`
/// - `@import url('path');`
/// - `@import url(path);`
/// - `@import "path";`
/// - `@import 'path';`
///
/// Correctly skips `@import` inside comments.
pub(crate) fn extract_css_imports(
    css_content: &str,
    source: &ComposeSource,
    base_line: usize,
) -> Vec<ReferenceRecord> {
    let mut records = Vec::new();
    let mut input = ParserInput::new(css_content);
    let mut parser = Parser::new(&mut input);

    while let Ok(token) = parser.next_including_whitespace_and_comments().cloned() {
        if let Token::AtKeyword(ref name) = token
            && name.eq_ignore_ascii_case("import")
        {
            let position = parser.position();
            let byte_offset = position.byte_index();

            // The import URL can be a string or url() function
            if let Some(url) = try_extract_import_url(&mut parser) {
                let line = base_line + css_content[..byte_offset].matches('\n').count();
                let span_start = byte_offset;
                let span_end = parser.position().byte_index();

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, span_start),
                    kind: ReferenceKind::CssImport,
                    target: classify_target(&url),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span: span_start..span_end,
                        syntax: ReferenceSyntax::CssAtImport,
                    },
                    attributes: serde_json::Map::new(),
                });
            }

            // Skip remaining tokens until semicolon or end
            skip_until_semicolon(&mut parser);
        }
    }

    records
}

/// Extract `@font-face { src: url(...) }` references from CSS content.
///
/// Correctly identifies `@font-face` blocks and only extracts `url()`
/// values from `src:` declarations within those blocks.
pub(crate) fn extract_font_face_sources(
    css_content: &str,
    source: &ComposeSource,
    base_line: usize,
) -> Vec<ReferenceRecord> {
    let mut records = Vec::new();
    let mut input = ParserInput::new(css_content);
    let mut parser = Parser::new(&mut input);

    while let Ok(token) = parser.next_including_whitespace_and_comments().cloned() {
        if let Token::AtKeyword(ref name) = token
            && name.eq_ignore_ascii_case("font-face")
        {
            // Skip whitespace/comments until the opening brace
            while let Ok(next) = parser.next_including_whitespace_and_comments().cloned() {
                match next {
                    Token::WhiteSpace(_) | Token::Comment(_) => continue,
                    Token::CurlyBracketBlock => {
                        // Now we can parse the nested block
                        let _ = parser.parse_nested_block(|block_parser| {
                            extract_font_face_urls(
                                block_parser,
                                css_content,
                                source,
                                base_line,
                                &mut records,
                            );
                            Ok::<(), cssparser::ParseError<'_, ()>>(())
                        });
                        break;
                    }
                    _ => break, // Unexpected token, skip this at-rule
                }
            }
        }
    }

    records
}

/// Try to extract a URL from after `@import`.
fn try_extract_import_url(parser: &mut Parser) -> Option<String> {
    // Skip whitespace
    while let Ok(token) = parser.next_including_whitespace_and_comments().cloned() {
        match token {
            Token::WhiteSpace(_) | Token::Comment(_) => continue,
            Token::QuotedString(ref s) => return Some(s.to_string()),
            Token::UnquotedUrl(ref s) => return Some(s.to_string()),
            Token::Function(ref name) if name.eq_ignore_ascii_case("url") => {
                // Parse inside url()
                let result: Result<String, cssparser::ParseError<'_, ()>> = parser
                    .parse_nested_block(|p| {
                        while let Ok(inner) = p.next_including_whitespace_and_comments().cloned() {
                            match inner {
                                Token::WhiteSpace(_) | Token::Comment(_) => continue,
                                Token::QuotedString(ref s) => return Ok(s.to_string()),
                                // Inside url(), bare text is also valid
                                _ => {}
                            }
                        }
                        Err(p.new_custom_error(()))
                    });
                return result.ok();
            }
            _ => return None,
        }
    }
    None
}

/// Extract url() values from src: declarations within a @font-face block.
fn extract_font_face_urls<'i>(
    parser: &mut Parser<'i, '_>,
    css_content: &str,
    source: &ComposeSource,
    base_line: usize,
    records: &mut Vec<ReferenceRecord>,
) {
    let mut in_src = false;

    while let Ok(token) = parser.next_including_whitespace_and_comments().cloned() {
        match token {
            Token::Ident(ref name) if name.eq_ignore_ascii_case("src") => {
                in_src = false;
                // Look for the colon
                while let Ok(next) = parser.next_including_whitespace_and_comments().cloned() {
                    match next {
                        Token::WhiteSpace(_) | Token::Comment(_) => continue,
                        Token::Colon => {
                            in_src = true;
                            break;
                        }
                        _ => break,
                    }
                }
            }
            Token::Semicolon | Token::CurlyBracketBlock => {
                in_src = false;
            }
            Token::Function(ref name) if in_src && name.eq_ignore_ascii_case("url") => {
                let url_byte_start = parser.position().byte_index();
                let result: Result<String, cssparser::ParseError<'_, ()>> = parser
                    .parse_nested_block(|p| {
                        while let Ok(inner) = p.next_including_whitespace_and_comments().cloned() {
                            match inner {
                                Token::WhiteSpace(_) | Token::Comment(_) => continue,
                                Token::QuotedString(ref s) => return Ok(s.to_string()),
                                _ => {}
                            }
                        }
                        Err(p.new_custom_error(()))
                    });

                if let Ok(url) = result {
                    let line = base_line + css_content[..url_byte_start].matches('\n').count();
                    let span_end = parser.position().byte_index();

                    records.push(ReferenceRecord {
                        id: make_reference_id(source, line, url_byte_start),
                        kind: ReferenceKind::FontImport,
                        target: classify_target(&url),
                        origin: ReferenceOrigin {
                            source: source.clone(),
                            line,
                            span: url_byte_start..span_end,
                            syntax: ReferenceSyntax::CssFontFaceSrc,
                        },
                        attributes: serde_json::Map::new(),
                    });
                }
            }
            Token::UnquotedUrl(ref url) if in_src => {
                let byte_offset = parser.position().byte_index();
                let line = base_line + css_content[..byte_offset].matches('\n').count();

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, byte_offset),
                    kind: ReferenceKind::FontImport,
                    target: classify_target(url),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span: byte_offset..byte_offset + url.len(),
                        syntax: ReferenceSyntax::CssFontFaceSrc,
                    },
                    attributes: serde_json::Map::new(),
                });
            }
            _ => {}
        }
    }
}

/// Skip tokens until a semicolon or end of input.
fn skip_until_semicolon(parser: &mut Parser) {
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        if matches!(token, Token::Semicolon) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::ReferenceTarget;
    use super::*;

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
        let css = r#"@import "reset.css";"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 10);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].origin.line, 10);
    }

    // ── Additional cssparser tests ──────────────────────────────────

    #[test]
    fn import_inside_comment_is_skipped() {
        let css = r#"/* @import "should-not-match.css"; */
@import "real.css";"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "real.css"
        ));
    }

    #[test]
    fn import_with_media_query() {
        let css = r#"@import url("print.css") print;"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "print.css"
        ));
    }

    #[test]
    fn multiple_imports() {
        let css = r#"@import "a.css";
@import url("b.css");
@import 'c.css';"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn multiple_font_face_src_urls() {
        let css = r#"@font-face {
  font-family: "MyFont";
  src: url("font.woff2") format("woff2"),
       url("font.woff") format("woff");
}"#;
        let records = extract_font_face_sources(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn import_url_single_quotes() {
        let css = r#"@import url('styles.css');"#;
        let records = extract_css_imports(css, &ComposeSource::Unknown, 1);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            ReferenceTarget::LocalPath { raw } if raw == "styles.css"
        ));
    }
}
