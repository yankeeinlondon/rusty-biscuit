//! Core types for TOML handling.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Source tracking for TOML content.
#[derive(Debug, Clone)]
pub enum TomlSource {
    /// Content loaded from a file path.
    Path(PathBuf),
    /// Content provided inline (not from a file).
    Inline,
}

/// Error type for TOML operations.
#[derive(Debug, Error)]
pub enum TomlError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

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

    /// Schema validation error.
    #[error("Schema validation error: {0}")]
    Schema(String),

    /// Schema feature not enabled.
    #[error("Schema support requires the 'schema' feature")]
    SchemaFeatureDisabled,
}

/// Validation issue severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Error that must be fixed.
    Error,
    /// Warning that should be reviewed.
    Warning,
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity of the issue.
    pub level: ValidationLevel,
    /// Human-readable description.
    pub message: String,
    /// Optional location in the source.
    pub path: Option<String>,
}

/// Report from validation operations.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// List of issues found during validation.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Returns true if validation passed with no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(|i| i.level == ValidationLevel::Error)
    }

    /// Returns only the error-level issues.
    #[must_use]
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.level == ValidationLevel::Error)
            .collect()
    }

    /// Returns only the warning-level issues.
    #[must_use]
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.level == ValidationLevel::Warning)
            .collect()
    }
}

/// TOML document with parsing, conversion, and validation capabilities.
///
/// The `Toml` struct provides a unified interface for working with TOML files:
/// - Parse from files, strings, or readers
/// - Convert to JSON or YAML formats
/// - Validate syntax and optionally against a JSON Schema
#[derive(Debug, Clone)]
pub struct Toml {
    source: TomlSource,
    raw: String,
    value: toml::Value,
}

impl Toml {
    /// Parse a TOML file from the given path.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid TOML.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, TomlError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)?;
        let value: toml::Value = toml::from_str(&raw)?;

        Ok(Self {
            source: TomlSource::Path(path.to_path_buf()),
            raw,
            value,
        })
    }

    /// Parse TOML from a string.
    ///
    /// ## Errors
    ///
    /// Returns an error if the string contains invalid TOML.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: impl AsRef<str>) -> Result<Self, TomlError> {
        let raw = input.as_ref().to_string();
        let value: toml::Value = toml::from_str(&raw)?;

        Ok(Self {
            source: TomlSource::Inline,
            raw,
            value,
        })
    }

    /// Parse TOML from a reader.
    ///
    /// ## Errors
    ///
    /// Returns an error if reading fails or the content is invalid TOML.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, TomlError> {
        let mut raw = String::new();
        reader.read_to_string(&mut raw)?;
        let value: toml::Value = toml::from_str(&raw)?;

        Ok(Self {
            source: TomlSource::Inline,
            raw,
            value,
        })
    }

    /// Returns a reference to the parsed TOML value.
    #[must_use]
    pub fn value(&self) -> &toml::Value {
        &self.value
    }

    /// Returns the raw TOML string.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the source of this TOML document.
    #[must_use]
    pub fn source(&self) -> &TomlSource {
        &self.source
    }

    /// Convert to a JSON string.
    ///
    /// TOML datetime values are serialized as RFC 3339 strings.
    ///
    /// ## Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn as_json(&self) -> Result<String, TomlError> {
        let json_value = self.as_json_value()?;
        let json = serde_json::to_string_pretty(&json_value)?;
        Ok(json)
    }

    /// Convert to a `serde_json::Value`.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails.
    pub fn as_json_value(&self) -> Result<serde_json::Value, TomlError> {
        let json_value = toml_to_json(&self.value);
        Ok(json_value)
    }

    /// Convert to a YAML string.
    ///
    /// ## Errors
    ///
    /// Returns an error if YAML serialization fails or the feature is disabled.
    #[cfg(feature = "yaml")]
    pub fn as_yaml(&self) -> Result<String, TomlError> {
        let json_value = self.as_json_value()?;
        let yaml = serde_yaml_ng::to_string(&json_value)?;
        Ok(yaml)
    }

    /// Convert to a YAML string.
    #[cfg(not(feature = "yaml"))]
    pub fn as_yaml(&self) -> Result<String, TomlError> {
        Err(TomlError::YamlFeatureDisabled)
    }

    /// Convert to a `serde_yaml_ng::Value`.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails or the feature is disabled.
    #[cfg(feature = "yaml")]
    pub fn as_yaml_value(&self) -> Result<serde_yaml_ng::Value, TomlError> {
        let json_value = self.as_json_value()?;
        // Convert JSON value to YAML value via serialization round-trip
        let yaml_str = serde_yaml_ng::to_string(&json_value)?;
        let yaml_value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_str)?;
        Ok(yaml_value)
    }

    /// Convert to a `serde_yaml_ng::Value`.
    #[cfg(not(feature = "yaml"))]
    pub fn as_yaml_value(&self) -> Result<(), TomlError> {
        Err(TomlError::YamlFeatureDisabled)
    }

    /// Validate the TOML document.
    ///
    /// This performs syntactic validation (already done during parsing)
    /// and returns an empty report if valid.
    #[must_use]
    pub fn validate(&self) -> ValidationReport {
        // Parsing already validates syntax
        ValidationReport::default()
    }

    /// Validate against a JSON Schema.
    ///
    /// ## Errors
    ///
    /// Returns an error if the schema feature is not enabled or validation fails.
    #[cfg(feature = "schema")]
    pub fn validate_with_schema(
        &self,
        _schema: &serde_json::Value,
    ) -> Result<ValidationReport, TomlError> {
        // TODO: Implement schema validation in Phase 2
        Err(TomlError::Schema(
            "Schema validation not yet implemented".to_string(),
        ))
    }

    /// Validate against a JSON Schema.
    #[cfg(not(feature = "schema"))]
    pub fn validate_with_schema(
        &self,
        _schema: &serde_json::Value,
    ) -> Result<ValidationReport, TomlError> {
        Err(TomlError::SchemaFeatureDisabled)
    }
}

/// Convert a TOML value to a JSON value.
///
/// TOML datetime types are converted to RFC 3339 strings.
fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let obj = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

// TryFrom implementations

impl TryFrom<&str> for Toml {
    type Error = TomlError;

    /// Interpret the string as a file path and load the TOML file.
    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<String> for Toml {
    type Error = TomlError;

    /// Interpret the string as a file path and load the TOML file.
    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&Path> for Toml {
    type Error = TomlError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<PathBuf> for Toml {
    type Error = TomlError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&OsStr> for Toml {
    type Error = TomlError;

    fn try_from(path: &OsStr) -> Result<Self, Self::Error> {
        Self::new(Path::new(path))
    }
}

impl TryFrom<OsString> for Toml {
    type Error = TomlError;

    fn try_from(path: OsString) -> Result<Self, Self::Error> {
        Self::new(Path::new(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ==========================================================================
    // Construction and Parsing Tests
    // ==========================================================================

    #[test]
    fn test_parse_simple_toml() {
        let toml = Toml::from_str(
            r#"
            [package]
            name = "test"
            version = "0.1.0"
        "#,
        )
        .unwrap();

        assert!(matches!(toml.source(), TomlSource::Inline));
        assert!(toml.value().is_table());
    }

    #[test]
    fn test_parse_from_reader() {
        let input = r#"
            name = "reader-test"
            value = 123
        "#;
        let cursor = Cursor::new(input);
        let toml = Toml::from_reader(cursor).unwrap();

        assert!(matches!(toml.source(), TomlSource::Inline));
        assert_eq!(toml.raw(), input);
        assert!(toml.value().is_table());
    }

    #[test]
    fn test_raw_content_preserved() {
        let input = r#"# A comment
name = "test"
"#;
        let toml = Toml::from_str(input).unwrap();
        assert_eq!(toml.raw(), input);
    }

    // ==========================================================================
    // Invalid TOML Error Tests
    // ==========================================================================

    #[test]
    fn test_invalid_toml_unclosed_bracket() {
        let result = Toml::from_str("invalid = [");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Parse(_)));
    }

    #[test]
    fn test_invalid_toml_duplicate_keys() {
        let result = Toml::from_str(
            r#"
            name = "first"
            name = "second"
        "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_bad_datetime() {
        let result = Toml::from_str(r#"date = 2024-13-45"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_missing_value() {
        let result = Toml::from_str("key = ");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_unclosed_string() {
        let result = Toml::from_str(r#"name = "unclosed"#);
        assert!(result.is_err());
    }

    // ==========================================================================
    // JSON Conversion Tests
    // ==========================================================================

    #[test]
    fn test_as_json_basic_types() {
        let toml = Toml::from_str(
            r#"
            name = "test"
            count = 42
            enabled = true
            ratio = 3.14
        "#,
        )
        .unwrap();

        let json = toml.as_json().unwrap();
        assert!(json.contains(r#""name": "test""#));
        assert!(json.contains(r#""count": 42"#));
        assert!(json.contains(r#""enabled": true"#));
        assert!(json.contains(r#""ratio": 3.14"#));
    }

    #[test]
    fn test_as_json_nested_tables() {
        let toml = Toml::from_str(
            r#"
            [server]
            host = "localhost"
            port = 8080

            [server.ssl]
            enabled = true
            cert = "/path/to/cert"
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["server"]["host"], "localhost");
        assert_eq!(json_value["server"]["port"], 8080);
        assert_eq!(json_value["server"]["ssl"]["enabled"], true);
        assert_eq!(json_value["server"]["ssl"]["cert"], "/path/to/cert");
    }

    #[test]
    fn test_as_json_arrays() {
        let toml = Toml::from_str(
            r#"
            tags = ["rust", "toml", "json"]
            numbers = [1, 2, 3, 4, 5]
            mixed_nested = [[1, 2], [3, 4]]
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["tags"][0], "rust");
        assert_eq!(json_value["tags"][1], "toml");
        assert_eq!(json_value["tags"][2], "json");
        assert_eq!(json_value["numbers"][2], 3);
        assert_eq!(json_value["mixed_nested"][0][1], 2);
    }

    #[test]
    fn test_as_json_array_of_tables() {
        let toml = Toml::from_str(
            r#"
            [[products]]
            name = "Widget"
            price = 9.99

            [[products]]
            name = "Gadget"
            price = 19.99
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["products"][0]["name"], "Widget");
        assert_eq!(json_value["products"][0]["price"], 9.99);
        assert_eq!(json_value["products"][1]["name"], "Gadget");
        assert_eq!(json_value["products"][1]["price"], 19.99);
    }

    #[test]
    fn test_as_json_inline_tables() {
        let toml = Toml::from_str(
            r#"
            point = { x = 10, y = 20 }
            person = { name = "Alice", age = 30 }
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["point"]["x"], 10);
        assert_eq!(json_value["point"]["y"], 20);
        assert_eq!(json_value["person"]["name"], "Alice");
        assert_eq!(json_value["person"]["age"], 30);
    }

    #[test]
    fn test_as_json_negative_numbers() {
        let toml = Toml::from_str(
            r#"
            negative_int = -42
            negative_float = -3.14
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["negative_int"], -42);
        assert_eq!(json_value["negative_float"], -3.14);
    }

    #[test]
    fn test_as_json_special_floats() {
        let toml = Toml::from_str(
            r#"
            inf_pos = inf
            inf_neg = -inf
            not_a_number = nan
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        // Non-finite floats become null in JSON
        assert!(json_value["inf_pos"].is_null());
        assert!(json_value["inf_neg"].is_null());
        assert!(json_value["not_a_number"].is_null());
    }

    #[test]
    fn test_as_json_empty_structures() {
        // Note: In TOML, items after [table] belong to that table
        // So we put empty_array at root level first
        let toml = Toml::from_str(
            r#"
            empty_array = []

            [empty_table]
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert!(json_value["empty_table"].is_object());
        assert!(json_value["empty_table"].as_object().unwrap().is_empty());
        assert!(json_value["empty_array"].is_array());
        assert!(json_value["empty_array"].as_array().unwrap().is_empty());
    }

    // ==========================================================================
    // DateTime Conversion Tests (RFC 3339)
    // ==========================================================================

    #[test]
    fn test_datetime_offset_datetime() {
        let toml = Toml::from_str(
            r#"
            created = 2024-01-15T10:30:00Z
            updated = 2024-06-20T14:45:30-05:00
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let created = json_value["created"].as_str().unwrap();
        let updated = json_value["updated"].as_str().unwrap();

        // Verify RFC 3339 format
        assert!(created.contains("2024-01-15"));
        assert!(created.contains("10:30:00"));
        assert!(updated.contains("2024-06-20"));
        assert!(updated.contains("14:45:30"));
        assert!(updated.contains("-05:00"));
    }

    #[test]
    fn test_datetime_local_datetime() {
        let toml = Toml::from_str(
            r#"
            local_dt = 2024-03-25T08:15:00
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let local = json_value["local_dt"].as_str().unwrap();
        assert!(local.contains("2024-03-25"));
        assert!(local.contains("08:15:00"));
    }

    #[test]
    fn test_datetime_local_date() {
        let toml = Toml::from_str(
            r#"
            birth_date = 1990-12-25
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let date = json_value["birth_date"].as_str().unwrap();
        assert_eq!(date, "1990-12-25");
    }

    #[test]
    fn test_datetime_local_time() {
        let toml = Toml::from_str(
            r#"
            alarm_time = 07:30:00
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let time = json_value["alarm_time"].as_str().unwrap();
        assert_eq!(time, "07:30:00");
    }

    #[test]
    fn test_datetime_with_subseconds() {
        let toml = Toml::from_str(
            r#"
            precise = 2024-01-15T10:30:00.123456Z
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let precise = json_value["precise"].as_str().unwrap();
        assert!(precise.contains("123456"));
    }

    // ==========================================================================
    // YAML Conversion Tests
    // ==========================================================================

    #[cfg(feature = "yaml")]
    #[test]
    fn test_as_yaml_basic() {
        let toml = Toml::from_str(
            r#"
            name = "test"
            version = "1.0.0"
        "#,
        )
        .unwrap();

        let yaml = toml.as_yaml().unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: '1.0.0'") || yaml.contains("version: 1.0.0"));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_as_yaml_value() {
        let toml = Toml::from_str(
            r#"
            [database]
            host = "localhost"
            port = 5432
        "#,
        )
        .unwrap();

        let yaml_value = toml.as_yaml_value().unwrap();
        assert!(yaml_value.is_mapping());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_as_yaml_nested() {
        let toml = Toml::from_str(
            r#"
            [server]
            host = "0.0.0.0"
            port = 8080

            [server.tls]
            enabled = true
        "#,
        )
        .unwrap();

        let yaml = toml.as_yaml().unwrap();
        assert!(yaml.contains("server:"));
        assert!(yaml.contains("host:"));
        assert!(yaml.contains("tls:"));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_as_yaml_arrays() {
        let toml = Toml::from_str(
            r#"
            tags = ["alpha", "beta", "gamma"]
        "#,
        )
        .unwrap();

        let yaml = toml.as_yaml().unwrap();
        assert!(yaml.contains("tags:"));
        assert!(yaml.contains("- alpha"));
        assert!(yaml.contains("- beta"));
        assert!(yaml.contains("- gamma"));
    }

    // ==========================================================================
    // Validation Tests
    // ==========================================================================

    #[test]
    fn test_validation_report_default() {
        let report = ValidationReport::default();
        assert!(report.is_valid());
        assert!(report.errors().is_empty());
        assert!(report.warnings().is_empty());
    }

    #[test]
    fn test_validation_with_issues() {
        let report = ValidationReport {
            issues: vec![
                ValidationIssue {
                    level: ValidationLevel::Error,
                    message: "Missing required field".to_string(),
                    path: Some("$.name".to_string()),
                },
                ValidationIssue {
                    level: ValidationLevel::Warning,
                    message: "Deprecated field".to_string(),
                    path: Some("$.old_field".to_string()),
                },
            ],
        };

        assert!(!report.is_valid());
        assert_eq!(report.errors().len(), 1);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn test_validate_valid_toml() {
        let toml = Toml::from_str(
            r#"
            [package]
            name = "valid"
            version = "1.0.0"
        "#,
        )
        .unwrap();

        let report = toml.validate();
        assert!(report.is_valid());
    }

    // ==========================================================================
    // TryFrom Implementation Tests
    // ==========================================================================

    #[test]
    fn test_try_from_str_nonexistent_file() {
        let result = Toml::try_from("/nonexistent/path/to/file.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    #[test]
    fn test_try_from_string_nonexistent_file() {
        let path = String::from("/nonexistent/path/to/file.toml");
        let result = Toml::try_from(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    #[test]
    fn test_try_from_path_nonexistent() {
        let path = Path::new("/nonexistent/path/to/file.toml");
        let result = Toml::try_from(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    #[test]
    fn test_try_from_pathbuf_nonexistent() {
        let path = PathBuf::from("/nonexistent/path/to/file.toml");
        let result = Toml::try_from(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    #[test]
    fn test_try_from_osstr_nonexistent() {
        let path = OsStr::new("/nonexistent/path/to/file.toml");
        let result = Toml::try_from(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    #[test]
    fn test_try_from_osstring_nonexistent() {
        let path = OsString::from("/nonexistent/path/to/file.toml");
        let result = Toml::try_from(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TomlError::Io(_)));
    }

    // ==========================================================================
    // TryFrom with Actual Files (using tempfile)
    // ==========================================================================

    #[test]
    fn test_try_from_str_real_file() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_toml_str.toml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"name = "from_str""#).unwrap();
        drop(file);

        let result = Toml::try_from(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let toml = result.unwrap();
        assert!(matches!(toml.source(), TomlSource::Path(_)));

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_try_from_pathbuf_real_file() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_toml_pathbuf.toml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"name = "from_pathbuf""#).unwrap();
        drop(file);

        let result = Toml::try_from(file_path.clone());
        assert!(result.is_ok());
        let toml = result.unwrap();
        let json = toml.as_json_value().unwrap();
        assert_eq!(json["name"], "from_pathbuf");

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_try_from_path_real_file() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_toml_path.toml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"name = "from_path""#).unwrap();
        drop(file);

        let result = Toml::try_from(file_path.as_path());
        assert!(result.is_ok());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_try_from_osstring_real_file() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_toml_osstring.toml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"name = "from_osstring""#).unwrap();
        drop(file);

        let os_path = OsString::from(file_path.as_os_str());
        let result = Toml::try_from(os_path);
        assert!(result.is_ok());

        std::fs::remove_file(&file_path).ok();
    }

    // ==========================================================================
    // Edge Cases and Complex Structures
    // ==========================================================================

    #[test]
    fn test_dotted_keys() {
        let toml = Toml::from_str(
            r#"
            physical.color = "orange"
            physical.shape = "round"
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["physical"]["color"], "orange");
        assert_eq!(json_value["physical"]["shape"], "round");
    }

    #[test]
    fn test_multiline_strings() {
        let toml = Toml::from_str(
            r#"
            bio = """
            Line one
            Line two
            Line three"""
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        let bio = json_value["bio"].as_str().unwrap();
        assert!(bio.contains("Line one"));
        assert!(bio.contains("Line two"));
        assert!(bio.contains("Line three"));
    }

    #[test]
    fn test_literal_strings() {
        let toml = Toml::from_str(
            r#"
            path = 'C:\Users\name\Documents'
            regex = '\\d+\\.\\d+'
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["path"], r"C:\Users\name\Documents");
        assert_eq!(json_value["regex"], r"\\d+\\.\\d+");
    }

    #[test]
    fn test_hex_octal_binary_integers() {
        let toml = Toml::from_str(
            r#"
            hex = 0xDEADBEEF
            octal = 0o755
            binary = 0b11010110
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["hex"], 0xDEADBEEF_i64);
        assert_eq!(json_value["octal"], 0o755_i64);
        assert_eq!(json_value["binary"], 0b11010110_i64);
    }

    #[test]
    fn test_underscore_in_numbers() {
        let toml = Toml::from_str(
            r#"
            large = 1_000_000
            precise = 3.141_592_653
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(json_value["large"], 1_000_000);
        // Float comparison with tolerance
        let precise = json_value["precise"].as_f64().unwrap();
        assert!((precise - 3.141592653).abs() < 1e-9);
    }

    #[test]
    fn test_unicode_strings() {
        let toml = Toml::from_str(
            r#"
            greeting = "Hello, \u4e16\u754c!"
            emoji = "Rust "
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert!(json_value["greeting"].as_str().unwrap().contains("Hello"));
        assert!(json_value["emoji"].as_str().unwrap().contains("Rust"));
    }

    #[test]
    fn test_source_path_tracking() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_source_tracking.toml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, r#"name = "tracked""#).unwrap();
        drop(file);

        let toml = Toml::new(&file_path).unwrap();
        match toml.source() {
            TomlSource::Path(p) => assert_eq!(p, &file_path),
            TomlSource::Inline => panic!("Expected Path source"),
        }

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_deeply_nested_structure() {
        let toml = Toml::from_str(
            r#"
            [level1]
            [level1.level2]
            [level1.level2.level3]
            [level1.level2.level3.level4]
            value = "deep"
        "#,
        )
        .unwrap();

        let json_value = toml.as_json_value().unwrap();
        assert_eq!(
            json_value["level1"]["level2"]["level3"]["level4"]["value"],
            "deep"
        );
    }
}
