//! Core types for JSON5 handling.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Source tracking for JSON5 content.
#[derive(Debug, Clone)]
pub enum Json5Source {
    /// Content loaded from a file path.
    Path(PathBuf),
    /// Content provided inline (not from a file).
    Inline,
}

/// Error type for JSON5 operations.
#[derive(Debug, Error)]
pub enum Json5Error {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON5 parsing error.
    #[error("JSON5 parse error: {0}")]
    Parse(String),

    /// JSON serialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML serialization error.
    #[cfg(feature = "yaml")]
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// YAML feature not enabled.
    #[cfg(not(feature = "yaml"))]
    #[error("YAML support requires the 'yaml' feature")]
    YamlFeatureDisabled,

    /// TOML serialization error.
    #[cfg(feature = "toml")]
    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),

    /// TOML feature not enabled.
    #[cfg(not(feature = "toml"))]
    #[error("TOML support requires the 'toml' feature")]
    TomlFeatureDisabled,
}

/// JSON5 document with parsing and conversion capabilities.
///
/// The `Json5` struct provides a unified interface for working with JSON5 files:
/// - Parse from files or strings
/// - Convert to JSON, YAML, or TOML formats
///
/// Internally, JSON5 is parsed into a `serde_json::Value` since JSON5 is a
/// superset of JSON with the same data model.
///
/// ## Examples
///
/// ```rust,ignore
/// use biscuit_file::Json5;
///
/// let j = Json5::from_str(r#"{ key: "value", /* comment */ }"#)?;
/// let json = j.as_json()?;
/// ```
#[derive(Debug, Clone)]
pub struct Json5 {
    source: Json5Source,
    raw: String,
    value: serde_json::Value,
}

impl Json5 {
    /// Parse a JSON5 file from the given path.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSON5.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Json5Error> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)?;
        let value: serde_json::Value =
            json_five::from_str(&raw).map_err(|e| Json5Error::Parse(e.to_string()))?;

        Ok(Self {
            source: Json5Source::Path(path.to_path_buf()),
            raw,
            value,
        })
    }

    /// Parse JSON5 from a string.
    ///
    /// ## Errors
    ///
    /// Returns an error if the string contains invalid JSON5.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: impl AsRef<str>) -> Result<Self, Json5Error> {
        let raw = input.as_ref().to_string();
        let value: serde_json::Value =
            json_five::from_str(&raw).map_err(|e| Json5Error::Parse(e.to_string()))?;

        Ok(Self {
            source: Json5Source::Inline,
            raw,
            value,
        })
    }

    /// Parse JSON5 from raw bytes.
    ///
    /// ## Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8 or contain invalid JSON5.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Json5Error> {
        let raw = std::str::from_utf8(bytes)
            .map_err(|e| Json5Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?
            .to_string();
        let value: serde_json::Value =
            json_five::from_str(&raw).map_err(|e| Json5Error::Parse(e.to_string()))?;

        Ok(Self {
            source: Json5Source::Inline,
            raw,
            value,
        })
    }

    /// Returns a reference to the parsed value.
    #[must_use]
    pub fn value(&self) -> &serde_json::Value {
        &self.value
    }

    /// Returns the raw JSON5 string.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the source of this JSON5 document.
    #[must_use]
    pub fn source(&self) -> &Json5Source {
        &self.source
    }

    /// Convert to a pretty-printed JSON string.
    ///
    /// ## Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn as_json(&self) -> Result<String, Json5Error> {
        let json = serde_json::to_string_pretty(&self.value)?;
        Ok(json)
    }

    /// Convert to a compact single-line JSON string.
    ///
    /// ## Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn as_json_compact(&self) -> Result<String, Json5Error> {
        let json = serde_json::to_string(&self.value)?;
        Ok(json)
    }

    /// Returns a reference to the underlying `serde_json::Value`.
    #[must_use]
    pub fn as_json_value(&self) -> &serde_json::Value {
        &self.value
    }

    /// Convert to a pretty-printed JSON5 string with idiomatic syntax.
    ///
    /// Outputs unquoted keys (when valid identifiers), single-quoted strings,
    /// and trailing commas.
    #[must_use]
    pub fn as_json5(&self) -> String {
        super::format::to_json5_pretty(&self.value)
    }

    /// Convert to a compact single-line JSON5 string.
    ///
    /// Same as [`as_json5`](Self::as_json5) but without indentation, newlines,
    /// or trailing commas.
    #[must_use]
    pub fn as_json5_compact(&self) -> String {
        super::format::to_json5_compact(&self.value)
    }

    /// Convert to a YAML string.
    ///
    /// ## Errors
    ///
    /// Returns an error if YAML serialization fails or the feature is disabled.
    #[cfg(feature = "yaml")]
    pub fn as_yaml(&self) -> Result<String, Json5Error> {
        let yaml = serde_yaml_ng::to_string(&self.value)?;
        Ok(yaml)
    }

    /// Convert to a YAML string.
    #[cfg(not(feature = "yaml"))]
    pub fn as_yaml(&self) -> Result<String, Json5Error> {
        Err(Json5Error::YamlFeatureDisabled)
    }

    /// Convert to a pretty-printed TOML string.
    ///
    /// ## Errors
    ///
    /// Returns an error if TOML serialization fails or the feature is disabled.
    /// JSON5 values containing types unsupported by TOML (e.g., null) will fail.
    #[cfg(feature = "toml")]
    pub fn as_toml(&self) -> Result<String, Json5Error> {
        // Convert serde_json::Value → toml::Value via serde round-trip
        let toml_value: toml::Value = serde_json::from_value(self.value.clone())
            .map_err(|e| Json5Error::Parse(format!("cannot convert to TOML: {e}")))?;
        let toml_str = toml::to_string_pretty(&toml_value)?;
        Ok(toml_str)
    }

    /// Convert to a pretty-printed TOML string.
    #[cfg(not(feature = "toml"))]
    pub fn as_toml(&self) -> Result<String, Json5Error> {
        Err(Json5Error::TomlFeatureDisabled)
    }
}

impl TryFrom<PathBuf> for Json5 {
    type Error = Json5Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&Path> for Json5 {
    type Error = Json5Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&str> for Json5 {
    type Error = Json5Error;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────

    #[test]
    fn parse_basic_json5() {
        let input = r#"{
            // a comment
            key: "value",
            num: 42,
        }"#;
        let j = Json5::from_str(input).unwrap();
        assert_eq!(j.value()["key"], "value");
        assert_eq!(j.value()["num"], 42);
    }

    #[test]
    fn parse_single_quoted_strings() {
        let j = Json5::from_str("{ name: 'hello' }").unwrap();
        assert_eq!(j.value()["name"], "hello");
    }

    #[test]
    fn parse_trailing_commas() {
        let j = Json5::from_str(r#"{ a: 1, b: 2, }"#).unwrap();
        assert_eq!(j.value()["a"], 1);
        assert_eq!(j.value()["b"], 2);
    }

    #[test]
    fn parse_hex_numbers() {
        let j = Json5::from_str("{ hex: 0xFF }").unwrap();
        assert_eq!(j.value()["hex"], 255);
    }

    #[test]
    fn parse_multiline_comment() {
        let input = r#"{
            /* this is a
               multi-line comment */
            key: "value",
        }"#;
        let j = Json5::from_str(input).unwrap();
        assert_eq!(j.value()["key"], "value");
    }

    #[test]
    fn parse_unquoted_keys() {
        let j = Json5::from_str("{ unquoted: true }").unwrap();
        assert_eq!(j.value()["unquoted"], true);
    }

    #[test]
    fn parse_standard_json_is_valid() {
        let j = Json5::from_str(r#"{"key": "value", "num": 42}"#).unwrap();
        assert_eq!(j.value()["key"], "value");
    }

    #[test]
    fn raw_preserved() {
        let input = "{ key: 'value' }";
        let j = Json5::from_str(input).unwrap();
        assert_eq!(j.raw(), input);
    }

    #[test]
    fn from_bytes_valid() {
        let j = Json5::from_bytes(b"{ key: 'value' }").unwrap();
        assert_eq!(j.value()["key"], "value");
    }

    #[test]
    fn from_bytes_invalid_utf8() {
        let result = Json5::from_bytes(&[0xFF, 0xFE]);
        assert!(result.is_err());
    }

    // ── Error handling ───────────────────────────────────────────────

    #[test]
    fn invalid_json5_errors() {
        let result = Json5::from_str("{ key: }");
        assert!(result.is_err());
    }

    #[test]
    fn empty_input_errors() {
        let result = Json5::from_str("");
        assert!(result.is_err());
    }

    // ── Conversion: JSON ─────────────────────────────────────────────

    #[test]
    fn as_json_pretty() {
        let j = Json5::from_str(r#"{ name: "example", version: "1.0" }"#).unwrap();
        let json = j.as_json().unwrap();
        assert!(json.contains(r#""name": "example""#));
        assert!(json.contains(r#""version": "1.0""#));
    }

    #[test]
    fn as_json_value_ref() {
        let j = Json5::from_str("{ key: 42 }").unwrap();
        assert_eq!(j.as_json_value()["key"], 42);
    }

    #[test]
    fn as_json_nested() {
        let input = r#"{
            outer: {
                inner: [1, 2, 3],
            },
        }"#;
        let j = Json5::from_str(input).unwrap();
        let json = j.as_json().unwrap();
        assert!(json.contains(r#""inner""#));
    }

    // ── Conversion: JSON5 ────────────────────────────────────────────

    #[test]
    fn as_json5_roundtrip() {
        let input = r#"{ name: "test", count: 5 }"#;
        let j = Json5::from_str(input).unwrap();
        let output = j.as_json5();
        // Should be parseable back
        let j2 = Json5::from_str(&output).unwrap();
        assert_eq!(j.value(), j2.value());
    }

    #[test]
    fn as_json5_unquoted_keys() {
        let j = Json5::from_str(r#"{ "name": "Bob", "age": 32 }"#).unwrap();
        let output = j.as_json5();
        assert!(output.contains("name: 'Bob'"));
        assert!(output.contains("age: 32"));
        assert!(!output.contains("\"name\""));
    }

    // ── Conversion: YAML ─────────────────────────────────────────────

    #[cfg(feature = "yaml")]
    #[test]
    fn as_yaml_basic() {
        let j = Json5::from_str(r#"{ name: "example", tags: ["a", "b"] }"#).unwrap();
        let yaml = j.as_yaml().unwrap();
        assert!(yaml.contains("name: example"));
    }

    // ── Conversion: TOML ─────────────────────────────────────────────

    #[cfg(feature = "toml")]
    #[test]
    fn as_toml_basic() {
        let j = Json5::from_str(r#"{ name: "example", version: "1.0" }"#).unwrap();
        let toml = j.as_toml().unwrap();
        assert!(toml.contains("name = \"example\""));
        assert!(toml.contains("version = \"1.0\""));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn as_toml_with_null_fails() {
        let j = Json5::from_str(r#"{ key: null }"#).unwrap();
        assert!(j.as_toml().is_err());
    }

    // ── TryFrom ──────────────────────────────────────────────────────

    #[test]
    fn try_from_nonexistent_path() {
        let result = Json5::try_from(Path::new("/nonexistent/file.json5"));
        assert!(result.is_err());
    }

    // ── Source tracking ──────────────────────────────────────────────

    #[test]
    fn source_inline() {
        let j = Json5::from_str("{ key: 1 }").unwrap();
        assert!(matches!(j.source(), Json5Source::Inline));
    }
}
