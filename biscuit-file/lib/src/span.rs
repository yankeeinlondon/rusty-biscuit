//! Shared byte-offset source-span vocabulary.
//!
//! A [`SourceSpan`] is a half-open byte-offset range into the exact source
//! text handed to a parser. Offsets are 0-indexed and end-exclusive; no
//! line-ending normalization is applied, so spans remain valid against the
//! caller's original document (including CRLF content and multibyte UTF-8).
//!
//! This is the single span type shared by every crate in the workspace that
//! produces or consumes source ranges (YAML diagnostics, Markdown parse
//! products). Keeping it a transparent alias of [`Range<usize>`] means two
//! crates can exchange spans with no conversion and can never drift apart.

use std::ops::Range;

/// Byte-offset range into a source document (0-indexed, end-exclusive).
pub type SourceSpan = Range<usize>;

/// Serde adapter that serializes a [`SourceSpan`] as a `{"start", "end"}`
/// object. Use with `#[serde(with = "biscuit_file::span_serde")]` on
/// `SourceSpan` fields; [`Range`] itself has no serde implementation.
///
/// Deserialization rejects inverted ranges (`start > end`).
pub mod span_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::SourceSpan;

    #[derive(Serialize, Deserialize)]
    struct SpanObject {
        start: usize,
        end: usize,
    }

    /// Serialize `span` as `{"start": .., "end": ..}`.
    pub fn serialize<S: Serializer>(span: &SourceSpan, serializer: S) -> Result<S::Ok, S::Error> {
        SpanObject {
            start: span.start,
            end: span.end,
        }
        .serialize(serializer)
    }

    /// Deserialize a `{"start": .., "end": ..}` object back into a span.
    ///
    /// ## Errors
    ///
    /// Returns an error when `start > end`.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SourceSpan, D::Error> {
        let object = SpanObject::deserialize(deserializer)?;
        if object.start > object.end {
            return Err(serde::de::Error::custom(format!(
                "invalid span: start {} is past end {}",
                object.start, object.end
            )));
        }
        Ok(object.start..object.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_is_byte_offsets_not_chars() {
        // 'é' is two bytes; a span over it must use byte offsets.
        let source = "abécd";
        let span: SourceSpan = 2..4;
        assert_eq!(&source[span], "é");
    }

    #[test]
    fn test_span_multibyte_tail() {
        let source = "key: 日本語";
        // "日本語" is 9 bytes starting at byte 5.
        let span: SourceSpan = 5..14;
        assert_eq!(&source[span.clone()], "日本語");
        assert_eq!(span.len(), 9);
    }

    #[test]
    fn test_span_crlf_byte_offsets() {
        // CRLF is two bytes; offsets must account for the '\r'.
        let source = "a: 1\r\nb: 2\r\n";
        let b_value: SourceSpan = 9..10;
        assert_eq!(&source[b_value], "2");
        let crlf: SourceSpan = 4..6;
        assert_eq!(&source[crlf], "\r\n");
    }

    #[test]
    fn test_span_zero_length_is_valid_insertion_point() {
        let source = "abc";
        let insertion: SourceSpan = 3..3;
        assert_eq!(&source[insertion], "");
    }

    // Helper exercising the `span_serde` adapter through a derived struct the
    // same way diagnostic types do.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct SpanWrapper {
        #[serde(with = "crate::span::span_serde")]
        span: SourceSpan,
    }

    #[test]
    fn test_span_serde_serializes_start_end_object() {
        let wrapper = SpanWrapper { span: 3..9 };
        let json = serde_json::to_string(&wrapper).unwrap();
        assert_eq!(json, r#"{"span":{"start":3,"end":9}}"#);
    }

    #[test]
    fn test_span_serde_rejects_inverted_range() {
        let result = serde_json::from_str::<SpanWrapper>(r#"{"span":{"start":9,"end":3}}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_span_serde_deserializes_object() {
        let wrapper: SpanWrapper = serde_json::from_str(r#"{"span":{"start":11,"end":24}}"#).unwrap();
        assert_eq!(wrapper.span, 11..24);
    }

    #[test]
    fn test_span_serde_round_trip() {
        let wrapper = SpanWrapper { span: 0..0 };
        let json = serde_json::to_string(&wrapper).unwrap();
        let back: SpanWrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(back.span, 0..0);
    }
}
