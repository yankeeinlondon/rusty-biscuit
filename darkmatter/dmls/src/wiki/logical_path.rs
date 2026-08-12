//! Logical-path canonicalization for wiki-link resolution (R-8 "Normalization"
//! and "Extension Handling").
//!
//! A document's *canonical logical path* is its root-relative path after
//! separator normalization (always `/`), Markdown-extension elision on the
//! final segment, and Unicode NFC normalization. Wiki-link targets are
//! canonicalized the same way — but percent-decoded *before* NFC, exactly once
//! (R-8 cross-platform gotcha 7) — so `[[My%20Note]]` and `[[My Note]]`
//! resolve to the same document regardless of host filesystem case or Unicode
//! form.

use std::path::Path;

use unicode_normalization::UnicodeNormalization;

/// The two recognized Markdown extensions, longest first so `.markdown` is
/// tried before `.md`.
const MARKDOWN_EXTENSIONS: [&str; 2] = [".markdown", ".md"];

/// Normalizes `text` to Unicode NFC.
pub fn nfc(text: &str) -> String {
    text.nfc().collect()
}

/// Strips a trailing `.md`/`.markdown` (case-insensitively) from a final path
/// segment, leaving other extensions intact.
///
/// Only the final Markdown extension is elided: `note.md.md` → `note.md`.
pub fn elide_markdown_extension(segment: &str) -> &str {
    for extension in MARKDOWN_EXTENSIONS {
        if segment.len() > extension.len()
            && segment[segment.len() - extension.len()..].eq_ignore_ascii_case(extension)
        {
            return &segment[..segment.len() - extension.len()];
        }
    }
    segment
}

/// Whether `segment` still carries a Markdown extension after one elision pass
/// — the visually confusing `note.md.md` case (R-8 extension rule 5).
pub fn has_markdown_extension(segment: &str) -> bool {
    MARKDOWN_EXTENSIONS.iter().any(|extension| {
        segment.len() > extension.len()
            && segment[segment.len() - extension.len()..].eq_ignore_ascii_case(extension)
    })
}

/// The canonical logical segments of a document `path`, relative to `root` when
/// `root` is an ancestor.
///
/// When `root` is `None` or not an ancestor, the path's own components (minus
/// the filesystem root/prefix) are used, so basename and suffix matching still
/// work off the absolute-path tail. The final segment is Markdown-extension
/// elided; every segment is NFC-normalized.
pub fn canonical_segments(path: &Path, root: Option<&Path>) -> Vec<String> {
    use std::path::Component;

    let relative = root.and_then(|root| path.strip_prefix(root).ok());
    let source = relative.unwrap_or(path);

    let mut segments: Vec<String> = source
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(nfc(&segment.to_string_lossy())),
            _ => None,
        })
        .collect();
    if let Some(last) = segments.last_mut() {
        *last = elide_markdown_extension(last).to_string();
    }
    segments
}

/// The case-fold-and-NFC key of a canonical path, for portability-collision
/// detection (R-8 normalization rule 6): two documents whose logical paths
/// differ only by case or Unicode form collide here.
pub fn portability_key(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| nfc(&segment.to_lowercase()))
        .collect::<Vec<_>>()
        .join("/")
}

/// Percent-decodes `text` once, leaving malformed `%` escapes literal.
///
/// ## Returns
///
/// The decoded string and whether any malformed escape was left literal (R-8:
/// `wiki.invalid-percent-escape` is informational, never fatal).
pub fn decode_percent_once(text: &str) -> (String, bool) {
    let bytes = text.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut invalid = false;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 < bytes.len()
                && let (Some(high), Some(low)) =
                    (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push(high << 4 | low);
                index += 3;
                continue;
            }
            invalid = true;
            decoded.push(b'%');
            index += 1;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    (String::from_utf8_lossy(&decoded).into_owned(), invalid)
}

/// The value of an ASCII hex digit byte, or `None`.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_elide_only_final_markdown_extension() {
        assert_eq!(elide_markdown_extension("note.md"), "note");
        assert_eq!(elide_markdown_extension("Guide.MARKDOWN"), "Guide");
        assert_eq!(elide_markdown_extension("note.md.md"), "note.md");
        assert_eq!(elide_markdown_extension("image.png"), "image.png");
        assert_eq!(elide_markdown_extension("plain"), "plain");
    }

    #[test]
    fn test_canonical_segments_relative_to_root() {
        let path = PathBuf::from("/ws/root-a/notes/folder/Target.md");
        let root = PathBuf::from("/ws/root-a");
        assert_eq!(
            canonical_segments(&path, Some(&root)),
            vec!["notes", "folder", "Target"]
        );
    }

    #[test]
    fn test_canonical_segments_without_root_uses_tail() {
        let path = PathBuf::from("/w/a.md");
        assert_eq!(canonical_segments(&path, None), vec!["w", "a"]);
    }

    #[test]
    fn test_portability_key_folds_case() {
        let case = canonical_segments(Path::new("/w/Case.md"), None);
        let lower = canonical_segments(Path::new("/w/case.md"), None);
        assert_ne!(case, lower); // matching stays case-sensitive
        assert_eq!(portability_key(&case), portability_key(&lower)); // but they collide
    }

    #[test]
    fn test_decode_percent_valid_and_invalid() {
        assert_eq!(decode_percent_once("My%20Note"), ("My Note".to_string(), false));
        assert_eq!(decode_percent_once("bad%2"), ("bad%2".to_string(), true));
        assert_eq!(decode_percent_once("bad%zz"), ("bad%zz".to_string(), true));
        assert_eq!(decode_percent_once("plain"), ("plain".to_string(), false));
    }
}
