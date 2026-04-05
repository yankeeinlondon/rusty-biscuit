//! PDF backend implementations.

use tracing::{debug, warn};

use super::types::{PdfConfig, PdfError};
#[cfg(feature = "lopdf")]
use super::types::{PdfToc, PdfTocItem};

/// Extract text from PDF bytes using pdf-extract.
#[cfg(feature = "extract")]
pub fn extract_text(bytes: &[u8], config: &PdfConfig) -> Result<String, PdfError> {
    let text =
        pdf_extract::extract_text_from_mem(bytes).map_err(|e| PdfError::Parse(e.to_string()))?;

    let text = if config.normalize_text {
        let normalized = normalize_text(&text);
        debug!(input_len = text.len(), output_len = normalized.len(), "PDF text extracted (normalized)");
        normalized
    } else {
        debug!(output_len = text.len(), "PDF text extracted");
        text
    };

    Ok(text)
}

/// Normalize extracted text by collapsing whitespace and de-hyphenating.
///
/// This function:
/// - Collapses runs of whitespace into single spaces
/// - Removes leading/trailing whitespace
/// - De-hyphenates words split across lines (hyphen immediately before
///   a newline, followed by whitespace on the next line)
pub fn normalize_text(text: &str) -> String {
    // First pass: de-hyphenate line breaks.
    // A hyphen at the end of a line (before \n or \r\n) indicates a word split.
    let dehyphenated = {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '-' {
                match chars.peek() {
                    Some('\n') => {
                        chars.next(); // consume \n
                        // Skip leading whitespace on next line
                        while let Some(&ws) = chars.peek() {
                            if ws.is_whitespace() && ws != '\n' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    Some('\r') => {
                        chars.next(); // consume \r
                        if chars.peek() == Some(&'\n') {
                            chars.next(); // consume \n
                        }
                        while let Some(&ws) = chars.peek() {
                            if ws.is_whitespace() && ws != '\n' {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    };

    // Second pass: collapse whitespace
    let mut result = String::with_capacity(dehyphenated.len());
    let mut last_was_whitespace = false;

    for ch in dehyphenated.chars() {
        if ch.is_whitespace() {
            if !last_was_whitespace && !result.is_empty() {
                result.push(' ');
            }
            last_was_whitespace = true;
        } else {
            result.push(ch);
            last_was_whitespace = false;
        }
    }

    result.trim().to_string()
}

/// Extract table of contents from PDF bytes using lopdf.
#[cfg(feature = "lopdf")]
pub fn extract_toc(bytes: &[u8]) -> Result<PdfToc, PdfError> {
    use lopdf::Document;

    let doc = Document::load_mem(bytes).map_err(|e| PdfError::Parse(e.to_string()))?;

    // Try to extract bookmarks/outlines
    let items = match extract_bookmarks(&doc) {
        Some(items) => {
            debug!(item_count = items.len(), "extracted PDF bookmarks");
            items
        }
        None => {
            warn!("could not extract PDF bookmarks — TOC will be empty");
            Vec::new()
        }
    };

    Ok(PdfToc { items })
}

/// Extract bookmarks from a lopdf Document.
#[cfg(feature = "lopdf")]
fn extract_bookmarks(doc: &lopdf::Document) -> Option<Vec<PdfTocItem>> {
    use lopdf::Object;

    // Get the catalog
    let catalog = doc.catalog().ok()?;

    // Get the outlines dictionary
    let outlines_ref = catalog.get(b"Outlines").ok()?;
    let outlines = doc.get_object(outlines_ref.as_reference().ok()?).ok()?;

    if let Object::Dictionary(dict) = outlines {
        // Get the first outline item
        let first_ref = dict.get(b"First").ok()?;
        let first_id = first_ref.as_reference().ok()?;

        return Some(extract_outline_items(doc, first_id));
    }

    None
}

/// Recursively extract outline items.
#[cfg(feature = "lopdf")]
fn extract_outline_items(doc: &lopdf::Document, item_id: lopdf::ObjectId) -> Vec<PdfTocItem> {
    use lopdf::Object;

    let mut items = Vec::new();
    let mut current_id = Some(item_id);

    while let Some(id) = current_id {
        let obj = match doc.get_object(id) {
            Ok(obj) => obj,
            Err(_) => break,
        };

        if let Object::Dictionary(dict) = obj {
            // Get the title
            let title = dict
                .get(b"Title")
                .ok()
                .and_then(|t| {
                    if let Object::String(s, _) = t {
                        String::from_utf8(s.clone()).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            // Get the destination page (simplified - just use 1 as fallback)
            let page_index = extract_page_number(doc, dict).unwrap_or(1);

            // Get children
            let children = dict
                .get(b"First")
                .ok()
                .and_then(|f| f.as_reference().ok())
                .map(|first_child| extract_outline_items(doc, first_child))
                .unwrap_or_default();

            items.push(PdfTocItem {
                title,
                page_index,
                children,
            });

            // Get next sibling
            current_id = dict.get(b"Next").ok().and_then(|n| n.as_reference().ok());
        } else {
            break;
        }
    }

    items
}

/// Extract page number from outline destination.
#[cfg(feature = "lopdf")]
fn extract_page_number(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Option<usize> {
    use lopdf::Object;

    // Try to get the destination
    let dest = dict.get(b"Dest").ok()?;

    match dest {
        Object::Array(arr) => {
            // First element should be the page reference
            let page_ref = arr.first()?.as_reference().ok()?;
            // Find the page index
            for (i, page_id) in doc.page_iter().enumerate() {
                if page_id == page_ref {
                    return Some(i + 1); // 1-indexed
                }
            }
            None
        }
        Object::Reference(r) => {
            // Named destination - look it up
            if let Ok(Object::Array(arr)) = doc.get_object(*r) {
                let page_ref = arr.first()?.as_reference().ok()?;
                for (i, page_id) in doc.page_iter().enumerate() {
                    if page_id == page_ref {
                        return Some(i + 1);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // normalize_text tests
    // ==========================================================================

    #[test]
    fn test_normalize_text_collapses_whitespace() {
        // Multiple spaces become single space
        assert_eq!(normalize_text("hello   world"), "hello world");

        // Multiple different whitespace characters
        assert_eq!(normalize_text("hello \t\n  world"), "hello world");

        // Leading and trailing whitespace removed
        assert_eq!(normalize_text("  hello   world  "), "hello world");
    }

    #[test]
    fn test_normalize_text_dehyphenates() {
        // Standard hyphenation pattern: hyphen at end of line
        assert_eq!(normalize_text("hyphen-\nated"), "hyphenated");

        // Multiple hyphenated words across lines
        assert_eq!(
            normalize_text("un-\nfortunate and mis-\nplaced"),
            "unfortunate and misplaced"
        );
    }

    #[test]
    fn test_normalize_text_preserves_dash_space_in_prose() {
        assert_eq!(
            normalize_text("list items - first"),
            "list items - first"
        );
    }

    #[test]
    fn test_normalize_text_dehyphenates_line_breaks_only() {
        let input = "hyphen-\nated word and some - dash";
        let result = normalize_text(input);
        assert_eq!(result, "hyphenated word and some - dash");
    }

    #[test]
    fn test_normalize_text_preserves_content() {
        // Normal text should be preserved
        assert_eq!(normalize_text("hello world"), "hello world");

        // Single word
        assert_eq!(normalize_text("hello"), "hello");

        // Hyphens in words (not at line breaks) should be preserved
        assert_eq!(normalize_text("well-known"), "well-known");
    }

    #[test]
    fn test_normalize_text_empty() {
        assert_eq!(normalize_text(""), "");
        assert_eq!(normalize_text("   "), "");
    }

    #[test]
    fn test_normalize_text_complex() {
        let input = "  This is a test   with   multiple\n\n  \tspaces\n\
                     and some hyphen-\nated words that span-\nned\n\
                     across     lines.  ";
        let expected = "This is a test with multiple spaces and some hyphenated words that spanned across lines.";
        assert_eq!(normalize_text(input), expected);
    }

    #[test]
    fn test_normalize_text_preserves_non_breaking_hyphens() {
        // Hyphens followed by non-space should be preserved
        assert_eq!(normalize_text("self-contained"), "self-contained");
        assert_eq!(normalize_text("pre-existing"), "pre-existing");
        assert_eq!(normalize_text("re-use"), "re-use");
    }

    #[test]
    fn test_normalize_text_unicode() {
        // Unicode characters should be preserved
        assert_eq!(normalize_text("café   résumé"), "café résumé");

        // Unicode whitespace should be normalized
        assert_eq!(normalize_text("hello\u{00A0}world"), "hello world"); // non-breaking space
    }
}
