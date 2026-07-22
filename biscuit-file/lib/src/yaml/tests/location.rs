//! Tests for the structured parse-location projection (acceptance C-5).
//!
//! Pins the byte/line/column projection of `serde_yaml_ng::Error::location()`
//! — including the flagship reserved-indicator input, multibyte content, and
//! CRLF offsets — and proves the `YamlError` display text and conversion
//! behavior are byte-identical to the pre-change contract.

use super::super::types::{Yaml, YamlError};
use super::super::YamlLocation;

/// Parse `input`, expecting failure, and return the projected location.
fn parse_location(input: &str) -> YamlLocation {
    let error = Yaml::from_str(input).unwrap_err();
    error
        .location()
        .unwrap_or_else(|| panic!("expected a structured location for {input:?}"))
}

#[test]
fn test_flagship_reserved_indicator_location() {
    // The original failing input from the spec: `title: @daily-report`.
    // The '@' is byte 7 of the line ("title: " is 7 bytes).
    let location = parse_location("title: @daily-report");
    assert_eq!(location.byte, 7);
    assert_eq!(location.line, 1);
    assert_eq!(location.column, 8);
}

#[test]
fn test_flow_sequence_location_points_into_source() {
    // Unterminated flow sequence: the reported byte indexes the source.
    let input = "key: [unclosed";
    let location = parse_location(input);
    assert_eq!(location.line, 2);
    assert_eq!(location.column, 1);
    assert_eq!(location.byte, input.len());
}

#[test]
fn test_multibyte_location_byte_index_is_a_byte_offset() {
    // 'é' is two bytes; '@' is byte 4 ("é", ":", " " precede it). The parser's
    // column is a character column (4th character), so byte and column
    // diverge here — both are projected faithfully.
    let input = "é: @bad";
    let location = parse_location(input);
    assert_eq!(location.byte, 4);
    assert_eq!(location.line, 1);
    assert_eq!(location.column, 4);
    assert_eq!(&input[location.byte..], "@bad");
}

#[test]
fn test_multibyte_key_location() {
    // "clé" is 4 bytes; '@' is byte 6, the 6th character.
    let input = "clé: @bad\r\n";
    let location = parse_location(input);
    assert_eq!(location.byte, 6);
    assert_eq!(location.line, 1);
    assert_eq!(location.column, 6);
    assert_eq!(input.as_bytes()[location.byte], b'@');
}

#[test]
fn test_crlf_offsets_count_cr_and_lf_bytes() {
    // "a: b\r\n" is 6 bytes; the error line starts at byte 6 and '@' is byte 9.
    let input = "a: b\r\nc: @bad";
    let location = parse_location(input);
    assert_eq!(location.byte, 9);
    assert_eq!(location.line, 2);
    assert_eq!(location.column, 4);
    assert_eq!(input.as_bytes()[location.byte], b'@');
}

#[test]
fn test_location_matches_underlying_parser_location() {
    // The projection must be a lossless copy of `serde_yaml_ng`'s location.
    let input = "n:\r\n  bad: [";
    let raw = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(input).unwrap_err();
    let raw_location = raw.location().unwrap();
    let location = parse_location(input);
    assert_eq!(location.byte, raw_location.index());
    assert_eq!(location.line, raw_location.line());
    assert_eq!(location.column, raw_location.column());
}

#[test]
fn test_location_is_none_for_non_parse_variants() {
    assert_eq!(YamlError::JsonConversion("x".to_string()).location(), None);
    assert_eq!(YamlError::SchemaInvalid("x".to_string()).location(), None);
    assert_eq!(YamlError::CycleDetected("x".to_string()).location(), None);
    assert_eq!(YamlError::MaxDepthExceeded(3).location(), None);
}

#[test]
fn test_parse_location_is_some_through_yaml_error() {
    let error = Yaml::from_str("key: [invalid").unwrap_err();
    assert!(matches!(error, YamlError::Parse(_)));
    assert!(error.location().is_some());
}

// ===== YamlError display snapshots (pre-change contract, byte-identical) =====

#[test]
fn test_display_flagship_parse_error_unchanged() {
    let error = Yaml::from_str("title: @daily-report").unwrap_err();
    assert_eq!(
        error.to_string(),
        "YAML parse error: found character that cannot start any token at line 1 \
         column 8, while scanning for the next token"
    );
}

#[test]
fn test_display_flow_sequence_error_unchanged() {
    let error = Yaml::from_str("key: [unclosed").unwrap_err();
    assert_eq!(
        error.to_string(),
        "YAML parse error: did not find expected ',' or ']' at line 2 column 1, \
         while parsing a flow sequence at line 1 column 6"
    );
}

#[test]
fn test_display_non_parse_variants_unchanged() {
    assert_eq!(
        YamlError::JsonConversion("bad".to_string()).to_string(),
        "JSON conversion error: bad"
    );
    assert_eq!(
        YamlError::TomlConversion("bad".to_string()).to_string(),
        "TOML conversion error: bad"
    );
    assert_eq!(
        YamlError::SchemaInvalid("bad".to_string()).to_string(),
        "Schema invalid: bad"
    );
    assert_eq!(
        YamlError::SchemaFeatureDisabled.to_string(),
        "Schema support requires the 'schema' feature"
    );
    assert_eq!(
        YamlError::CycleDetected("a.b".to_string()).to_string(),
        "Cycle detected in YAML at path: a.b"
    );
    assert_eq!(
        YamlError::MaxDepthExceeded(7).to_string(),
        "Max depth exceeded: 7"
    );
}

#[test]
fn test_from_serde_error_conversion_preserved() {
    // The `#[from]` conversion path still wraps the parser error verbatim.
    let raw = serde_yaml_ng::from_str::<serde_yaml_ng::Value>("title: @daily-report")
        .unwrap_err();
    let expected = format!("YAML parse error: {raw}");
    let converted: YamlError = raw.into();
    assert_eq!(converted.to_string(), expected);
    assert!(matches!(converted, YamlError::Parse(_)));
}
